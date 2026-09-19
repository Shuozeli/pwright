#![cfg(feature = "retained-reference")]

use std::sync::Arc;

use pwright_browser_model::{
    BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND, BROWSER_OBJECT_KIND_CATALOG,
    FRAME_OBJECT_KIND, PAGE_OBJECT_KIND,
};
use pwright_cap_handle_evaluation::{EvaluateHandle, EvaluateHandleParams, EvaluationValueLimits};
use pwright_cap_handle_lifecycle::HandleDispose;
use pwright_cdp::CdpClient;
use pwright_composition_chromium_handle_evaluation::{
    CdpEvaluationLimits, CdpEvaluationProductionProfile, CdpHandlePropertyLimits,
    ChromiumHandleEvaluationBackendProfile, ChromiumHandleEvaluationBindings,
    ChromiumHandleEvaluationCommandService, ChromiumHandleEvaluationInstallation,
    ChromiumHandleEvaluationProfile, ChromiumHandleLifecycleProfile, ChromiumHandleRouteProfile,
    FrameLimits, HandleEvaluationCodecLimits, HandleLifecycleCodecLimits,
};
use pwright_core::{
    AtomicObjectIdSource, CallId, CallMetadata, Command, FailureKind, ObjectId,
    ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope, Service,
};
use pwright_fake::FakeCdpClient;
use pwright_runtime::{
    CleanupQueue, ControlledDispatcher, NoDeadlineScheduler, ObjectTopologyMode, RuntimeLimits,
};
use serde_json::{Value, json};

type Runtime = ControlledDispatcher<(), (), NoDeadlineScheduler, CleanupQueue>;

struct Fixture {
    runtime: Arc<Runtime>,
    installation: ChromiumHandleEvaluationInstallation<CleanupQueue, CleanupQueue>,
    client: Arc<FakeCdpClient>,
    resources: ProductionResourceLedger,
    profile: ChromiumHandleEvaluationProfile,
}

fn fixture(client: Arc<FakeCdpClient>) -> Fixture {
    let values = EvaluationValueLimits::new(8, 64, 64, 2048, 2048).unwrap();
    let frames = FrameLimits::new(32 * 1024, 0, 0, 32 * 1024);
    let profile = ChromiumHandleEvaluationProfile::new(
        ChromiumHandleEvaluationBackendProfile::new(
            CdpEvaluationProductionProfile::new(
                ResourceScope::extension(169, 1),
                CdpEvaluationLimits::new(2048, 32 * 1024, 1024, values),
                2,
            )
            .with_context_routes(1, 64),
            64,
            128,
            CdpHandlePropertyLimits::new(4, 64).unwrap(),
        ),
        HandleEvaluationCodecLimits::new(2048, 1024, values).unwrap(),
        frames,
        frames,
        ChromiumHandleLifecycleProfile::new(
            ChromiumHandleRouteProfile::new(ResourceScope::extension(169, 2), 4, 64, 128),
            HandleLifecycleCodecLimits::new(1024),
            frames,
            frames,
        ),
    );
    let runtime_limits = RuntimeLimits::new(16, 2)
        .with_active_commands(2)
        .with_control_messages(16);
    let required = runtime_limits
        .resource_requirements()
        .combine_concurrent(profile.requirements());
    let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
    for class in ResourceClass::ALL {
        let bound = required.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    let resources = ProductionResourceLedger::new(limits, required).unwrap();
    let runtime = Arc::new(
        ControlledDispatcher::with_object_kinds_and_topology(
            (),
            (),
            NoDeadlineScheduler,
            runtime_limits,
            BROWSER_OBJECT_KIND_CATALOG,
            ObjectTopologyMode::Complete,
        )
        .unwrap()
        .with_production_resources(&resources)
        .unwrap(),
    );
    for (id, parent, kind) in [
        (1, None, BROWSER_OBJECT_KIND.tag),
        (2, Some(1), BROWSER_CONTEXT_OBJECT_KIND.tag),
        (3, Some(2), PAGE_OBJECT_KIND.tag),
        (4, Some(3), FRAME_OBJECT_KIND.tag),
        (5, Some(2), PAGE_OBJECT_KIND.tag),
    ] {
        runtime
            .register_object(ObjectId::new(id), parent.map(ObjectId::new), kind)
            .unwrap();
    }
    while runtime.pop_control_message().unwrap().is_some() {}
    let installation = ChromiumHandleEvaluationInstallation::install(
        ChromiumHandleEvaluationBindings {
            runtime: runtime.lifecycle_handle(),
            object_ids: Arc::new(AtomicObjectIdSource::new(10).unwrap()),
            creation_continuation: CleanupQueue::new(2),
            lifecycle_continuation: CleanupQueue::new(2),
        },
        profile,
        &resources,
    )
    .unwrap();
    installation
        .routes()
        .bind(
            ObjectId::new(4),
            ObjectId::new(3),
            client.clone(),
            101,
            "creation-world",
        )
        .unwrap();
    Fixture {
        runtime,
        installation,
        client,
        resources,
        profile,
    }
}

fn fresh() -> Fixture {
    fixture(Arc::new(FakeCdpClient::from_html("")))
}

async fn create(fixture: &Fixture, remote: Value) -> ObjectId {
    fixture
        .client
        .set_evaluate_response(json!({"result":remote}));
    fixture
        .installation
        .raw_service()
        .call(Command::<EvaluateHandle>::new(
            CallId::new(1),
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::expression("document.body"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap()
        .object_id
}

async fn element(fixture: &Fixture, native_id: &str) -> ObjectId {
    create(
        fixture,
        json!({"type":"object", "subtype":"node", "objectId":native_id}),
    )
    .await
}

#[tokio::test]
async fn borrowing_and_validating_exact_element_share_metadata_without_io_or_admission() {
    // Arrange
    let fixture = fresh();
    let target = element(&fixture, "element-native").await;
    let service = fixture.installation.raw_service();
    let calls = fixture.client.calls();
    let metrics = fixture.resources.metrics().unwrap();

    // Act
    let source = service.borrow_element_source(target).unwrap();
    let again = service.borrow_element_source(target).unwrap();
    let validated = service.validate_element_source(target, &source);
    let world = source.world_identity();

    // Assert
    assert!(validated.is_ok());
    assert_eq!(source.target(), target);
    assert_eq!(source.object_id(), "element-native");
    assert_eq!(source.owner_frame(), ObjectId::new(4));
    assert_eq!(source.owner_page(), ObjectId::new(3));
    assert_eq!(source.creation_execution_context_id(), 101);
    assert_eq!(source.creation_unique_context_id(), "creation-world");
    assert_eq!(world.page_id(), ObjectId::new(3));
    assert_eq!(world.execution_context_id(), 101);
    assert_eq!(world.unique_context_id(), "creation-world");
    let client: Arc<dyn CdpClient> = fixture.client.clone();
    assert!(Arc::ptr_eq(&source.client(), &client));
    assert_eq!(source.object_id().as_ptr(), again.object_id().as_ptr());
    assert_eq!(
        source.creation_unique_context_id().as_ptr(),
        again.creation_unique_context_id().as_ptr()
    );
    assert_eq!(fixture.client.calls(), calls);
    assert_eq!(fixture.resources.metrics().unwrap(), metrics);
}

#[tokio::test]
async fn non_element_object_and_primitive_are_rejected_without_io() {
    for remote in [
        json!({"type":"object", "objectId":"plain-object"}),
        json!({"type":"number", "value":7}),
    ] {
        // Arrange
        let fixture = fresh();
        let target = create(&fixture, remote).await;
        let calls = fixture.client.calls();

        // Act
        let error = fixture
            .installation
            .raw_service()
            .borrow_element_source(target)
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert_eq!(fixture.client.calls(), calls);
    }
}

#[tokio::test]
async fn unknown_portable_target_fails_closed_without_io() {
    // Arrange
    let fixture = fresh();

    // Act
    let error = fixture
        .installation
        .raw_service()
        .borrow_element_source(ObjectId::new(999))
        .unwrap_err();

    // Assert
    assert_eq!(error.kind(), FailureKind::TargetClosed);
    assert!(fixture.client.calls().is_empty());
}

#[tokio::test]
async fn missing_native_route_rejects_previously_borrowed_source_even_if_runtime_is_live() {
    // Arrange
    let fixture = fresh();
    let target = element(&fixture, "element-native").await;
    let service = fixture.installation.raw_service();
    let source = service.borrow_element_source(target).unwrap();
    fixture
        .installation
        .lifecycle()
        .routes()
        .retire_connection()
        .unwrap();
    let calls = fixture.client.calls();

    // Act
    let error = service
        .validate_element_source(target, &source)
        .unwrap_err();

    // Assert
    assert!(fixture.runtime.is_object_live(target).unwrap());
    assert_eq!(error.kind(), FailureKind::TargetClosed);
    assert_eq!(fixture.client.calls(), calls);
}

#[tokio::test]
async fn descriptor_does_not_pin_explicit_disposal_or_grant_native_release() {
    // Arrange
    let fixture = fresh();
    let target = element(&fixture, "element-native").await;
    let service = fixture.installation.raw_service();
    let source = service.borrow_element_source(target).unwrap();
    fixture
        .installation
        .lifecycle()
        .dispose_service()
        .call(Command::<HandleDispose>::new(
            CallId::new(2),
            target,
            CallMetadata::default(),
            (),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    let calls = fixture.client.calls();

    // Act
    let error = service
        .validate_element_source(target, &source)
        .unwrap_err();
    drop(source);

    // Assert
    assert_eq!(error.kind(), FailureKind::TargetClosed);
    assert!(!fixture.runtime.is_object_live(target).unwrap());
    assert_eq!(fixture.client.calls(), calls);
    assert_eq!(
        calls
            .iter()
            .filter(|(method, _)| method == "Runtime.releaseObject")
            .count(),
        1
    );
}

#[tokio::test]
async fn retired_parent_runtime_rejects_source_even_when_native_routes_remain() {
    for parent in [ObjectId::new(4), ObjectId::new(3)] {
        // Arrange
        let fixture = fresh();
        let target = element(&fixture, "element-native").await;
        let service = fixture.installation.raw_service();
        let source = service.borrow_element_source(target).unwrap();
        fixture.runtime.dispose_object(parent).unwrap();
        let calls = fixture.client.calls();

        // Act
        let error = service
            .validate_element_source(target, &source)
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::TargetClosed);
        assert!(
            fixture
                .installation
                .lifecycle()
                .routes()
                .contains(target)
                .unwrap()
        );
        assert_eq!(fixture.client.calls(), calls);
    }
}

#[tokio::test]
async fn replaced_creation_page_client_numeric_or_unique_world_rejects_old_receipt() {
    for change in 0..4 {
        // Arrange
        let fixture = fresh();
        let target = element(&fixture, "element-native").await;
        let service = fixture.installation.raw_service();
        let source = service.borrow_element_source(target).unwrap();
        let routes = fixture.installation.routes();
        routes.unbind(ObjectId::new(4)).unwrap();
        let other = Arc::new(FakeCdpClient::from_html(""));
        let client: Arc<dyn CdpClient> = if change == 1 {
            other.clone()
        } else {
            fixture.client.clone()
        };
        routes
            .bind(
                ObjectId::new(4),
                if change == 0 {
                    ObjectId::new(5)
                } else {
                    ObjectId::new(3)
                },
                client,
                if change == 2 { 202 } else { 101 },
                if change == 3 {
                    "replacement-world"
                } else {
                    "creation-world"
                },
            )
            .unwrap();
        let calls = fixture.client.calls();

        // Act
        let error = service
            .validate_element_source(target, &source)
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert_eq!(fixture.client.calls(), calls);
        assert!(other.calls().is_empty());
    }
}

#[tokio::test]
async fn missing_creation_frame_route_rejects_receipt_without_io() {
    // Arrange
    let fixture = fresh();
    let target = element(&fixture, "element-native").await;
    let service = fixture.installation.raw_service();
    let source = service.borrow_element_source(target).unwrap();
    fixture
        .installation
        .routes()
        .unbind(ObjectId::new(4))
        .unwrap();
    let calls = fixture.client.calls();

    // Act
    let error = service
        .validate_element_source(target, &source)
        .unwrap_err();

    // Assert
    assert_eq!(error.kind(), FailureKind::TargetClosed);
    assert_eq!(fixture.client.calls(), calls);
}

#[tokio::test]
async fn foreign_receipt_with_identical_ids_world_and_client_is_not_interchangeable() {
    // Arrange two distinct route entries with identical observable fields.
    let client = Arc::new(FakeCdpClient::from_html(""));
    let first = fixture(client.clone());
    let second = fixture(client.clone());
    let first_id = element(&first, "element-native").await;
    let second_id = element(&second, "element-native").await;
    assert_eq!(first_id, second_id);
    let source = first
        .installation
        .raw_service()
        .borrow_element_source(first_id)
        .unwrap();
    let calls = client.calls();

    // Act
    let error = second
        .installation
        .raw_service()
        .validate_element_source(second_id, &source)
        .unwrap_err();

    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert_eq!(client.calls(), calls);
}

#[tokio::test]
async fn source_receipt_for_a_different_portable_target_is_rejected() {
    // Arrange
    let fixture = fresh();
    let first = element(&fixture, "first-native").await;
    let second = element(&fixture, "second-native").await;
    let service = fixture.installation.raw_service();
    let source = service.borrow_element_source(first).unwrap();
    let calls = fixture.client.calls();

    // Act
    let error = service
        .validate_element_source(second, &source)
        .unwrap_err();

    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert_eq!(fixture.client.calls(), calls);
}

#[tokio::test]
async fn rebinding_identical_creation_identity_keeps_the_same_source_valid() {
    // Arrange
    let fixture = fresh();
    let target = element(&fixture, "element-native").await;
    let service = fixture.installation.raw_service();
    let source = service.borrow_element_source(target).unwrap();
    let routes = fixture.installation.routes();
    routes.unbind(ObjectId::new(4)).unwrap();
    routes
        .bind(
            ObjectId::new(4),
            ObjectId::new(3),
            fixture.client.clone(),
            101,
            "creation-world",
        )
        .unwrap();
    let calls = fixture.client.calls();

    // Act
    let result = service.validate_element_source(target, &source);

    // Assert
    assert!(result.is_ok());
    assert_eq!(fixture.client.calls(), calls);
}

#[tokio::test]
async fn borrower_enforces_its_native_id_bound_before_exposing_a_source() {
    // Arrange a published route from a wider installation and a stricter reader.
    let fixture = fresh();
    let target = element(&fixture, "element-native").await;
    let backend = fixture.profile.backend();
    let service = ChromiumHandleEvaluationCommandService::new(
        fixture.runtime.lifecycle_handle(),
        Arc::new(AtomicObjectIdSource::new(100).unwrap()),
        fixture.installation.routes(),
        fixture.installation.lifecycle().routes(),
        ChromiumHandleEvaluationBackendProfile::new(
            backend.evaluation(),
            4,
            backend.preview_bytes(),
            backend.properties(),
        ),
    );
    let calls = fixture.client.calls();

    // Act
    let error = service.borrow_element_source(target).unwrap_err();

    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert_eq!(fixture.client.calls(), calls);
}
