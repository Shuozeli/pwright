use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use pwright_browser_model::{
    BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND, BROWSER_OBJECT_KIND_CATALOG,
    FRAME_OBJECT_KIND, PAGE_OBJECT_KIND,
};
use pwright_cap_handle_evaluation::{
    EvaluateHandle, EvaluateHandleParams, EvaluationNumber, EvaluationValue, EvaluationValueLimits,
    GetPropertiesOptions, GetPropertyParams, HandleEvaluate, HandleEvaluateHandle,
    HandleEvaluateParams, HandleEvaluationValue, HandleGetProperties, HandleGetProperty,
    HandleJsonValue, JsonValueOptions,
};
use pwright_cap_handle_lifecycle::HandleDispose;
use pwright_cdp::CdpClient;
use pwright_codec_control::ControlCodec;
use pwright_composition_chromium_handle_evaluation::{
    CdpElementHandleAdoptionProductionProfile, CdpEvaluationLimits, CdpEvaluationProductionProfile,
    CdpHandleEvaluationLimits, CdpHandlePropertyLimits, ChromiumHandleEvaluationBackendProfile,
    ChromiumHandleEvaluationBindings, ChromiumHandleEvaluationCommandService,
    ChromiumHandleEvaluationInstallation, ChromiumHandleEvaluationProfile,
    ChromiumHandleEvaluationRouteError, ChromiumHandleEvaluationRouteResolver,
    ChromiumHandleEvaluationRouteSpec, ChromiumHandleLifecycleProfile, ChromiumHandleRouteProfile,
    ContinuableHandleEvaluationService, FrameLimits, HandleEvaluationCodecLimits,
    HandleLifecycleCodecLimits,
};
use pwright_core::{
    AtomicObjectIdSource, CallId, CallMetadata, Command, CommandResult, ControlMessage,
    FailureKind, ObjectId, ProductionResourceLedger, ProtocolVersion, ResourceClass,
    ResourceLimits, ResourceRequirements, ResourceScope, Service,
};
use pwright_fake::FakeCdpClient;
use pwright_plugin_ordered_handle_delivery::OrderedHandleEvaluationHandler;
use pwright_runtime::{
    CleanupQueue, ControlledDispatcher, NoDeadlineScheduler, ObjectTopologyMode, RuntimeLimits,
};
use pwright_runtime_control::{OrderedRuntimeControl, RuntimeControlDriver};
use pwright_server_wire::{ConcurrentServerFrameHandler, ServerFrameDisposition};
use pwright_transport::{FrameRoute, FrameTransport, VersionPolicy, WireProtocol};
use pwright_transport_memory::MemoryFrameTransport;

type TestRuntime = ControlledDispatcher<(), (), NoDeadlineScheduler, CleanupQueue>;

mod acquisition_cleanup;
#[cfg(feature = "native-creation")]
mod native_creation;
mod property_creation;

#[tokio::test]
async fn handle_world_identity_is_the_creation_world_not_the_current_frame_world() {
    // Arrange
    let fixture = fixture(2);
    let service = fixture.installation.raw_service();
    fixture.primary.set_evaluate_response(serde_json::json!({
        "result": {"type": "object", "objectId": "target-1", "description": "target"}
    }));
    let handle = service
        .call(Command::<EvaluateHandle>::new(
            CallId::new(1),
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::expression("({})"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    fixture
        .installation
        .routes()
        .unbind(ObjectId::new(4))
        .unwrap();
    fixture
        .installation
        .routes()
        .bind(
            ObjectId::new(4),
            ObjectId::new(3),
            fixture.primary.clone(),
            201,
            "replacement-world",
        )
        .unwrap();
    let before = fixture.primary.calls().len();

    // Act
    let identity = service
        .handle_world_identity(handle.object_id)
        .unwrap()
        .unwrap();

    // Assert
    assert_eq!(identity.page_id(), ObjectId::new(3));
    assert_eq!(identity.execution_context_id(), 101);
    assert_eq!(identity.unique_context_id(), "world-1");
    assert_eq!(fixture.primary.calls().len(), before);
}
type TestInstallation = ChromiumHandleEvaluationInstallation<CleanupQueue, CleanupQueue>;

struct Fixture {
    runtime: Arc<TestRuntime>,
    installation: TestInstallation,
    primary: Arc<FakeCdpClient>,
    secondary: Arc<FakeCdpClient>,
    resources: ProductionResourceLedger,
    adoption_continuation: CleanupQueue,
}

#[test]
fn frame_route_rejects_non_positive_execution_context_before_state_change() {
    // Arrange
    let fixture = fixture(2);
    let routes = fixture.installation.routes();
    let before = routes.len().unwrap();
    let client: Arc<dyn CdpClient> = fixture.primary.clone();

    // Act
    let error = routes
        .bind(
            ObjectId::new(8),
            ObjectId::new(3),
            client,
            0,
            "invalid-world",
        )
        .expect_err("zero execution-context ID must be rejected");

    // Assert
    assert_eq!(
        error,
        ChromiumHandleEvaluationRouteError::InvalidExecutionContextId {
            execution_context_id: 0,
        }
    );
    assert_eq!(routes.len().unwrap(), before);
}

#[test]
fn complete_frame_route_snapshot_replaces_every_route_atomically() {
    // Arrange
    let fixture = fixture(2);
    let routes = fixture.installation.routes();
    let primary: Arc<dyn CdpClient> = fixture.primary.clone();
    let secondary: Arc<dyn CdpClient> = fixture.secondary.clone();
    let replacement = [
        ChromiumHandleEvaluationRouteSpec::new(
            ObjectId::new(4),
            ObjectId::new(3),
            primary,
            201,
            "replacement-main",
        ),
        ChromiumHandleEvaluationRouteSpec::new(
            ObjectId::new(6),
            ObjectId::new(5),
            secondary,
            202,
            "replacement-oopif",
        ),
    ];

    // Act
    routes
        .replace_all(replacement)
        .expect("valid complete route snapshot should replace the old set");

    // Assert
    assert_eq!(routes.len().unwrap(), 2);
    assert!(routes.contains(ObjectId::new(4)).unwrap());
    assert!(routes.contains(ObjectId::new(6)).unwrap());
    assert!(!routes.contains(ObjectId::new(7)).unwrap());
}

#[test]
fn invalid_complete_frame_route_snapshot_preserves_previous_routes() {
    // Arrange
    let fixture = fixture(2);
    let routes = fixture.installation.routes();
    let before = [ObjectId::new(4), ObjectId::new(6), ObjectId::new(7)];
    let primary: Arc<dyn CdpClient> = fixture.primary.clone();
    let duplicate: Arc<dyn CdpClient> = fixture.primary.clone();
    let invalid = [
        ChromiumHandleEvaluationRouteSpec::new(
            ObjectId::new(8),
            ObjectId::new(3),
            primary,
            201,
            "replacement-main",
        ),
        ChromiumHandleEvaluationRouteSpec::new(
            ObjectId::new(8),
            ObjectId::new(3),
            duplicate,
            202,
            "duplicate-frame",
        ),
    ];

    // Act
    let error = routes
        .replace_all(invalid)
        .expect_err("duplicate Frame must reject the entire candidate snapshot");

    // Assert
    assert_eq!(
        error,
        ChromiumHandleEvaluationRouteError::Duplicate {
            frame_id: ObjectId::new(8),
        }
    );
    assert_eq!(routes.len().unwrap(), before.len());
    for frame_id in before {
        assert!(routes.contains(frame_id).unwrap());
    }
    assert!(!routes.contains(ObjectId::new(8)).unwrap());
}

#[tokio::test]
async fn command_service_accepts_an_injected_live_frame_route_resolver() {
    // Arrange
    let fixture = fixture(2);
    let frame_routes: Arc<dyn ChromiumHandleEvaluationRouteResolver> =
        Arc::new(fixture.installation.routes());
    let service = ChromiumHandleEvaluationCommandService::new_with_route_resolver(
        fixture.runtime.lifecycle_handle(),
        Arc::new(AtomicObjectIdSource::new(20).expect("ID source should build")),
        frame_routes,
        fixture.installation.lifecycle().routes(),
        profile(2).backend(),
    );
    fixture.primary.set_evaluate_response(serde_json::json!({
        "result": {"type": "object", "objectId": "resolver-result", "description": "DIV"}
    }));

    // Act
    let result = service
        .call(Command::<EvaluateHandle>::new(
            CallId::new(1),
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::expression("document.createElement('div')"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .expect("injected route resolver should reach the exact backend");

    // Assert
    assert_eq!(result.object_id, ObjectId::new(20));
    assert_eq!(result.parent_id, ObjectId::new(4));
    assert!(
        fixture
            .installation
            .lifecycle()
            .routes()
            .contains(result.object_id)
            .unwrap()
    );
    assert_eq!(
        fixture.primary.method_names(),
        vec!["Runtime.callFunctionOn"]
    );
}

fn exact_limits(requirements: ResourceRequirements) -> ResourceLimits {
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let required = requirements.class(class);
        limits = limits.with_class(class, required.global, required.per_scope);
    }
    limits
}

fn profile(max_handles: usize) -> ChromiumHandleEvaluationProfile {
    let values = EvaluationValueLimits::new(8, 64, 64, 2048, 2048)
        .expect("evaluation value limits should validate");
    let frames = FrameLimits::new(32 * 1024, 0, 0, 32 * 1024);
    let evaluation = CdpEvaluationProductionProfile::new(
        ResourceScope::extension(98, 1),
        CdpEvaluationLimits::new(2048, 32 * 1024, 1024, values),
        2,
    )
    .with_context_routes(3, 64);
    let lifecycle = ChromiumHandleLifecycleProfile::new(
        ChromiumHandleRouteProfile::new(ResourceScope::extension(98, 2), max_handles, 64, 128),
        HandleLifecycleCodecLimits::new(1024),
        frames,
        frames,
    );
    ChromiumHandleEvaluationProfile::new(
        ChromiumHandleEvaluationBackendProfile::new(
            evaluation,
            64,
            128,
            CdpHandlePropertyLimits::new(4, 64).expect("property limits should validate"),
        ),
        HandleEvaluationCodecLimits::new(2048, 1024, values)
            .expect("Handle evaluation codec limits should validate"),
        frames,
        frames,
        lifecycle,
    )
}

fn control_protocol() -> WireProtocol<ControlCodec> {
    WireProtocol::new(
        ControlCodec,
        VersionPolicy::exact(ProtocolVersion::new(1, 0)),
        ProtocolVersion::new(1, 0),
        FrameLimits::new(128, 0, 0, 128),
    )
    .expect("control protocol should build")
}

fn fixture(max_handles: usize) -> Fixture {
    fixture_with_optional_adoption(max_handles, false)
}

fn fixture_with_adoption(max_handles: usize) -> Fixture {
    fixture_with_optional_adoption(max_handles, true)
}

fn fixture_with_optional_adoption(max_handles: usize, adopt_elements: bool) -> Fixture {
    let profile = profile(max_handles);
    let adoption_profile = CdpElementHandleAdoptionProductionProfile::new(
        ResourceScope::extension(98, 3),
        CdpHandleEvaluationLimits::new(
            profile.backend().evaluation().limits(),
            profile.backend().native_id_bytes(),
            profile.backend().preview_bytes(),
        ),
        2,
        4,
    );
    let runtime_limits = RuntimeLimits::new(32, 4)
        .with_active_commands(4)
        .with_control_messages(32);
    let handle_requirements = if adopt_elements {
        profile.requirements_with_element_adoption(adoption_profile)
    } else {
        profile.requirements()
    };
    let requirements = runtime_limits
        .resource_requirements()
        .combine_concurrent(handle_requirements);
    let resources = ProductionResourceLedger::new(exact_limits(requirements), requirements)
        .expect("exact Handle-evaluation resources should build");
    let runtime = Arc::new(
        ControlledDispatcher::with_object_kinds_and_topology(
            (),
            (),
            NoDeadlineScheduler,
            runtime_limits,
            BROWSER_OBJECT_KIND_CATALOG,
            ObjectTopologyMode::Complete,
        )
        .expect("browser object catalog should validate")
        .with_production_resources(&resources)
        .expect("runtime resources should fit"),
    );
    for (object_id, parent, kind) in [
        (ObjectId::new(1), None, BROWSER_OBJECT_KIND.tag),
        (
            ObjectId::new(2),
            Some(ObjectId::new(1)),
            BROWSER_CONTEXT_OBJECT_KIND.tag,
        ),
        (
            ObjectId::new(3),
            Some(ObjectId::new(2)),
            PAGE_OBJECT_KIND.tag,
        ),
        (
            ObjectId::new(4),
            Some(ObjectId::new(3)),
            FRAME_OBJECT_KIND.tag,
        ),
        (
            ObjectId::new(5),
            Some(ObjectId::new(2)),
            PAGE_OBJECT_KIND.tag,
        ),
        (
            ObjectId::new(6),
            Some(ObjectId::new(5)),
            FRAME_OBJECT_KIND.tag,
        ),
        (
            ObjectId::new(7),
            Some(ObjectId::new(3)),
            FRAME_OBJECT_KIND.tag,
        ),
    ] {
        runtime
            .register_object(object_id, parent, kind)
            .expect("test object should register");
    }
    while runtime
        .pop_control_message()
        .expect("control queue should load")
        .is_some()
    {}
    let bindings = ChromiumHandleEvaluationBindings {
        runtime: runtime.lifecycle_handle(),
        object_ids: Arc::new(AtomicObjectIdSource::new(10).expect("ID source should build")),
        creation_continuation: CleanupQueue::new(4),
        lifecycle_continuation: CleanupQueue::new(4),
    };
    let adoption_continuation = CleanupQueue::new(2);
    let installation = if adopt_elements {
        ChromiumHandleEvaluationInstallation::install_with_element_adoption(
            bindings,
            profile,
            adoption_profile,
            adoption_continuation.clone(),
            &resources,
        )
    } else {
        ChromiumHandleEvaluationInstallation::install(bindings, profile, &resources)
    }
    .expect("Handle evaluation should install");
    let primary = Arc::new(FakeCdpClient::from_html(""));
    let secondary = Arc::new(FakeCdpClient::from_html(""));
    let primary_client: Arc<dyn CdpClient> = primary.clone();
    let secondary_client: Arc<dyn CdpClient> = secondary.clone();
    installation
        .routes()
        .bind(
            ObjectId::new(4),
            ObjectId::new(3),
            primary_client,
            101,
            "world-1",
        )
        .expect("primary Frame route should bind");
    installation
        .routes()
        .bind(
            ObjectId::new(6),
            ObjectId::new(5),
            secondary_client,
            102,
            "world-2",
        )
        .expect("secondary Frame route should bind");
    let primary_client: Arc<dyn CdpClient> = primary.clone();
    installation
        .routes()
        .bind(
            ObjectId::new(7),
            ObjectId::new(3),
            primary_client,
            103,
            "world-child",
        )
        .expect("child Frame route should bind");
    Fixture {
        runtime,
        installation,
        primary,
        secondary,
        resources,
        adoption_continuation,
    }
}

#[tokio::test]
async fn six_commands_share_one_lifecycle_registry_and_dispose_every_result() {
    // Arrange
    let fixture = fixture(5);
    let service = fixture.installation.raw_service();
    fixture.primary.set_evaluate_responses([
        serde_json::json!({
            "result": {"type": "object", "objectId": "target-1", "description": "target"}
        }),
        serde_json::json!({
            "result": {"value": {"ok": true, "value": {"t": "str", "v": "value"}}}
        }),
        serde_json::json!({
            "result": {"type": "object", "objectId": "child-2", "description": "child"}
        }),
        serde_json::json!({
            "result": {"value": {"ok": true, "value": {"t": "num", "v": 7.0}}}
        }),
        serde_json::json!({
            "result": {"type": "object", "objectId": "property-snapshot-3", "description": "Object"}
        }),
        serde_json::json!({
            "result": [{"name": "value", "enumerable": true, "value": {
                "type": "object", "objectId": "property-3", "description": "property"
            }}]
        }),
        serde_json::json!({
            "result": {"type": "object", "objectId": "snapshot-4", "description": "Object"}
        }),
        serde_json::json!({
            "result": [
                {"name": "one", "enumerable": true, "value": {"type": "number", "value": 1}},
                {"name": "two", "enumerable": true, "value": {
                    "type": "object", "objectId": "property-5", "description": "nested"
                }}
            ]
        }),
    ]);

    // Act
    let target = service
        .call(Command::<EvaluateHandle>::new(
            CallId::new(1),
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::expression("({ value: 7 })"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    let value = service
        .call(Command::<HandleEvaluate>::new(
            CallId::new(2),
            target.object_id,
            CallMetadata::default(),
            HandleEvaluateParams::function_without_argument("target => target.value"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    let child = service
        .call(Command::<HandleEvaluateHandle>::new(
            CallId::new(3),
            target.object_id,
            CallMetadata::default(),
            HandleEvaluateParams::function_without_argument("target => target.child"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    let json = service
        .call(Command::<HandleJsonValue>::new(
            CallId::new(4),
            target.object_id,
            CallMetadata::default(),
            JsonValueOptions,
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    let property = service
        .call(Command::<HandleGetProperty>::new(
            CallId::new(5),
            target.object_id,
            CallMetadata::default(),
            GetPropertyParams::new("value"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    let properties = service
        .call(Command::<HandleGetProperties>::new(
            CallId::new(6),
            target.object_id,
            CallMetadata::default(),
            GetPropertiesOptions,
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    let all_ids = [target.object_id, child.object_id, property.object_id]
        .into_iter()
        .chain(
            properties
                .properties
                .iter()
                .map(|entry| entry.handle.object_id),
        )
        .collect::<Vec<_>>();
    for (index, object_id) in all_ids.iter().enumerate() {
        fixture
            .installation
            .lifecycle()
            .dispose_service()
            .call(Command::<HandleDispose>::new(
                CallId::new(20 + index as u64),
                *object_id,
                CallMetadata::default(),
                (),
            ))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();
    }

    // Assert
    assert_eq!(target.object_id, ObjectId::new(10));
    assert_eq!(value.value, EvaluationValue::String("value".to_owned()));
    assert_eq!(child.object_id, ObjectId::new(11));
    assert_eq!(
        json.value,
        EvaluationValue::Number(EvaluationNumber::new(7.0).unwrap())
    );
    assert_eq!(property.object_id, ObjectId::new(12));
    assert_eq!(properties.properties.len(), 2);
    assert_eq!(properties.properties[0].name, "one");
    assert_eq!(properties.properties[1].name, "two");
    assert_eq!(
        all_ids,
        [
            ObjectId::new(10),
            ObjectId::new(11),
            ObjectId::new(12),
            ObjectId::new(13),
            ObjectId::new(14),
        ]
    );
    assert_eq!(
        fixture
            .installation
            .lifecycle()
            .routes()
            .metrics()
            .unwrap()
            .current,
        0
    );
    assert!(
        all_ids
            .iter()
            .all(|id| fixture.runtime.is_object_live(*id) == Ok(false))
    );
    assert!(fixture.secondary.calls().is_empty());
}

#[tokio::test]
async fn independent_leaf_composes_with_ordered_remote_delivery_plugin() {
    // Arrange
    let fixture = fixture(1);
    fixture.primary.set_evaluate_response(serde_json::json!({
        "result": {"type": "object", "objectId": "remote-1", "description": "body"}
    }));
    let (server_transport, client_transport) = MemoryFrameTransport::pair(4);
    let server_transport = Arc::new(server_transport);
    let handler = OrderedHandleEvaluationHandler::new(
        fixture.installation.protocol(),
        fixture.installation.raw_service(),
        fixture.installation.raw_service(),
        fixture.installation.lifecycle().raw_dispose_service(),
        CleanupQueue::new(1),
        OrderedRuntimeControl::new(
            Arc::clone(&fixture.runtime),
            RuntimeControlDriver::new(control_protocol(), Arc::clone(&server_transport)),
            1,
        )
        .expect("ordered control should build"),
    );
    let call_id = CallId::new(31);
    let command = Command::<EvaluateHandle>::new(
        call_id,
        ObjectId::new(4),
        CallMetadata::default(),
        EvaluateHandleParams::expression("document.body"),
    );

    // Act
    let disposition = handler
        .handle_concurrently(
            fixture
                .installation
                .protocol()
                .encode(&command)
                .expect("command should encode"),
            server_transport,
        )
        .await
        .expect("remote delivery should complete");
    let create = client_transport
        .receive()
        .await
        .expect("control receive should succeed")
        .expect("Create should be sent");
    let result = client_transport
        .receive()
        .await
        .expect("result receive should succeed")
        .expect("Result should be sent");

    // Assert
    assert!(matches!(disposition, ServerFrameDisposition::Handled(_)));
    assert_eq!(create.route(), FrameRoute::Control);
    assert!(matches!(
        control_protocol()
            .decode::<ControlMessage>(create)
            .expect("Create should decode"),
        ControlMessage::Create {
            object_id,
            parent: Some(parent),
            ..
        } if object_id == ObjectId::new(10) && parent == ObjectId::new(4)
    ));
    assert_eq!(result.route(), FrameRoute::Result { call_id });
    assert_eq!(
        fixture
            .installation
            .lifecycle()
            .routes()
            .metrics()
            .expect("route metrics should load")
            .current,
        1
    );
}

#[tokio::test]
async fn cross_frame_handle_argument_is_rejected_before_cdp_io() {
    // Arrange
    let fixture = fixture(2);
    let service = fixture.installation.raw_service();
    fixture.primary.set_evaluate_response(serde_json::json!({
        "result": {"type": "object", "objectId": "target-1", "description": "target"}
    }));
    let target = service
        .call(Command::<EvaluateHandle>::new(
            CallId::new(1),
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::expression("({})"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    let calls_before = fixture.secondary.calls().len();

    // Act
    let error = service
        .call(Command::<EvaluateHandle>::new(
            CallId::new(2),
            ObjectId::new(6),
            CallMetadata::default(),
            EvaluateHandleParams::function(
                "value => value",
                HandleEvaluationValue::Handle(target.object_id),
            ),
        ))
        .await
        .unwrap()
        .into_outcome()
        .expect_err("cross-Frame Handle should fail");

    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert_eq!(fixture.secondary.calls().len(), calls_before);
    assert_eq!(
        fixture
            .installation
            .lifecycle()
            .routes()
            .metrics()
            .unwrap()
            .current,
        1
    );
}

#[tokio::test]
async fn opted_in_public_command_adopts_child_frame_element_and_releases_temporary_owner() {
    // Arrange a live child-Frame ElementHandle in the same Page/CDP session as
    // the target Frame, plus the exact Chromium describe/resolve response.
    let fixture = fixture_with_adoption(3);
    let service = fixture.installation.raw_service();
    fixture.primary.set_evaluate_responses([
        serde_json::json!({
            "result": {
                "type": "object",
                "subtype": "node",
                "objectId": "source-node",
                "description": "main"
            }
        }),
        serde_json::json!({
            "result": {"type": "string", "value": "MAIN", "description": "MAIN"}
        }),
    ]);
    fixture.primary.set_object_backend_node("source-node", 77);
    fixture.primary.set_adopted_node_response(
        77,
        101,
        serde_json::json!({
            "object": {
                "type": "object",
                "subtype": "node",
                "objectId": "adopted-node",
                "description": "main"
            }
        }),
    );
    let source = service
        .call(Command::<EvaluateHandle>::new(
            CallId::new(1),
            ObjectId::new(7),
            CallMetadata::default(),
            EvaluateHandleParams::expression("document.querySelector('main')"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    while fixture.runtime.pop_control_message().unwrap().is_some() {}
    let before = fixture.resources.metrics().unwrap();

    // Act through the normal public Frame.evaluateHandle service.
    let output = service
        .call(Command::<EvaluateHandle>::new(
            CallId::new(2),
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::function(
                "node => node.textContent",
                HandleEvaluationValue::Handle(source.object_id),
            ),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    fixture
        .installation
        .lifecycle()
        .raw_dispose_service()
        .call(Command::<HandleDispose>::new(
            CallId::new(3),
            output.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    while fixture.runtime.pop_control_message().unwrap().is_some() {}

    // Assert the native argument came from DOM adoption, that temporary
    // object was released before command completion, and no cleanup permit or
    // production lease remained stranded.
    assert_eq!(output.preview, "MAIN");
    let calls = fixture.primary.calls();
    assert_eq!(
        calls
            .iter()
            .map(|(method, _)| method.as_str())
            .collect::<Vec<_>>(),
        [
            "Runtime.callFunctionOn",
            "DOM.describeNode",
            "DOM.resolveNode",
            "Runtime.callFunctionOn",
            "Runtime.releaseObject",
        ]
    );
    assert_eq!(calls[3].1[0]["arguments"][1]["objectId"], "adopted-node");
    assert_eq!(calls[4].1[0], "adopted-node");
    let resource_metrics = fixture.resources.metrics().unwrap();
    assert_eq!(resource_metrics.active_leases, before.active_leases);
    assert_eq!(
        resource_metrics.class(ResourceClass::RemoteHandle).current,
        before.class(ResourceClass::RemoteHandle).current
    );
    assert!(
        resource_metrics.class(ResourceClass::RemoteHandle).peak
            >= before.class(ResourceClass::RemoteHandle).current + 2
    );
    let cleanup_metrics = fixture.adoption_continuation.metrics().unwrap();
    assert_eq!(cleanup_metrics.reserved, 0);
    assert_eq!(cleanup_metrics.queued, 0);
}

#[tokio::test]
async fn opted_in_public_command_rejects_cross_world_js_handle_before_dom_io() {
    // Arrange a JSHandle in the child Frame while adoption is installed.
    let fixture = fixture_with_adoption(2);
    let service = fixture.installation.raw_service();
    fixture.primary.set_evaluate_response(serde_json::json!({
        "result": {
            "type": "object",
            "objectId": "source-js",
            "description": "Object"
        }
    }));
    let source = service
        .call(Command::<EvaluateHandle>::new(
            CallId::new(1),
            ObjectId::new(7),
            CallMetadata::default(),
            EvaluateHandleParams::expression("({ value: 1 })"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    let calls_before = fixture.primary.calls().len();

    // Act
    let error = service
        .call(Command::<EvaluateHandle>::new(
            CallId::new(2),
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::function(
                "value => value.value",
                HandleEvaluationValue::Handle(source.object_id),
            ),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap_err();

    // Assert Playwright's Element-only rule holds before describe/resolve or
    // target Runtime I/O, even when the optional adopter is installed.
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert_eq!(fixture.primary.calls().len(), calls_before);
    assert_eq!(fixture.adoption_continuation.metrics().unwrap().reserved, 0);
}

#[tokio::test]
async fn opted_in_public_command_rejects_cross_session_child_element_before_adoption_io() {
    // Arrange an ElementHandle whose child Frame is modeled in the same Page
    // but routed through another CDP target session, as with an OOPIF.
    let fixture = fixture_with_adoption(2);
    let routes = fixture.installation.routes();
    assert!(routes.unbind(ObjectId::new(7)).unwrap());
    let secondary: Arc<dyn CdpClient> = fixture.secondary.clone();
    routes
        .bind(
            ObjectId::new(7),
            ObjectId::new(3),
            secondary,
            103,
            "oopif-world",
        )
        .unwrap();
    fixture.secondary.set_evaluate_response(serde_json::json!({
        "result": {
            "type": "object",
            "subtype": "node",
            "objectId": "oopif-node",
            "description": "main"
        }
    }));
    let service = fixture.installation.raw_service();
    let source = service
        .call(Command::<EvaluateHandle>::new(
            CallId::new(1),
            ObjectId::new(7),
            CallMetadata::default(),
            EvaluateHandleParams::expression("document.querySelector('main')"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    let primary_calls = fixture.primary.calls().len();
    let secondary_calls = fixture.secondary.calls().len();

    // Act
    let error = service
        .call(Command::<EvaluateHandle>::new(
            CallId::new(2),
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::function(
                "node => node.textContent",
                HandleEvaluationValue::Handle(source.object_id),
            ),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap_err();

    // Assert the CDP-client identity check rejects before describe/resolve on
    // either session or a target-world Runtime call.
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert_eq!(fixture.primary.calls().len(), primary_calls);
    assert_eq!(fixture.secondary.calls().len(), secondary_calls);
    assert_eq!(fixture.adoption_continuation.metrics().unwrap().reserved, 0);
}

#[tokio::test]
async fn get_properties_batch_capacity_failure_releases_every_native_value() {
    // Arrange
    let fixture = fixture(1);
    let service = fixture.installation.raw_service();
    fixture.primary.set_evaluate_response(serde_json::json!({
        "result": {"type": "object", "objectId": "target-1", "description": "target"}
    }));
    let target = service
        .call(Command::<EvaluateHandle>::new(
            CallId::new(1),
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::expression("({ one: {}, two: {} })"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    fixture.primary.set_evaluate_responses([
        serde_json::json!({"result": {"type": "object", "objectId": "snapshot-2"}}),
        serde_json::json!({
            "result": [
                {"name": "one", "enumerable": true, "value": {"type": "object", "objectId": "value-3"}},
                {"name": "two", "enumerable": true, "value": {"type": "object", "objectId": "value-4"}}
            ]
        }),
    ]);

    // Act
    let error = service
        .call(Command::<HandleGetProperties>::new(
            CallId::new(2),
            target.object_id,
            CallMetadata::default(),
            GetPropertiesOptions,
        ))
        .await
        .unwrap()
        .into_outcome()
        .expect_err("complete property batch should exceed route capacity");

    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert_eq!(
        fixture
            .installation
            .lifecycle()
            .routes()
            .metrics()
            .unwrap()
            .current,
        1
    );
    assert!(fixture.runtime.is_object_live(target.object_id).unwrap());
    let calls = fixture.primary.calls();
    assert_eq!(calls[calls.len() - 3].1[0], serde_json::json!("snapshot-2"));
    assert_eq!(calls[calls.len() - 2].1[0], serde_json::json!("value-3"));
    assert_eq!(calls[calls.len() - 1].1[0], serde_json::json!("value-4"));
}

#[tokio::test]
async fn missing_creation_continuation_rejects_before_cdp_io() {
    // Arrange
    let fixture = fixture(1);
    let service = ContinuableHandleEvaluationService::new(
        fixture.installation.raw_service(),
        fixture.installation.lifecycle().dispose_service(),
        CleanupQueue::new(0),
    );

    // Act
    let error = service
        .call(Command::<EvaluateHandle>::new(
            CallId::new(1),
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::expression("({ rejected: true })"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .expect_err("missing continuation should reject creation");

    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert!(fixture.primary.calls().is_empty());
    assert_eq!(
        fixture
            .installation
            .lifecycle()
            .routes()
            .metrics()
            .unwrap()
            .current,
        0
    );
}

#[tokio::test]
async fn same_session_world_replacement_rejects_an_old_handle_before_cdp_io() {
    // Arrange
    let fixture = fixture(1);
    fixture.primary.set_evaluate_response(serde_json::json!({
        "result": {"type": "object", "objectId": "old-world", "description": "old"}
    }));
    let service = fixture.installation.raw_service();
    let old_handle = service
        .call(Command::<EvaluateHandle>::new(
            CallId::new(1),
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::expression("({ old: true })"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    let calls_before_rejection = fixture.primary.calls().len();
    let routes = fixture.installation.routes();
    routes.unbind(ObjectId::new(4)).unwrap();
    let same_session: Arc<dyn CdpClient> = fixture.primary.clone();
    routes
        .bind(
            ObjectId::new(4),
            ObjectId::new(3),
            same_session,
            104,
            "world-replaced-same-session",
        )
        .unwrap();

    // Act
    let error = service
        .call(Command::<HandleEvaluate>::new(
            CallId::new(2),
            old_handle.object_id,
            CallMetadata::default(),
            HandleEvaluateParams::function_without_argument("target => target.old"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .expect_err("an old-world Handle must not cross into a replacement world");

    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert_eq!(fixture.primary.calls().len(), calls_before_rejection);
}

#[tokio::test]
async fn world_replacement_during_cdp_compensates_old_native_result() {
    // Arrange
    let fixture = fixture(1);
    let gate = Arc::new(tokio::sync::Notify::new());
    fixture.primary.set_evaluate_gate(Some(Arc::clone(&gate)));
    fixture.primary.set_evaluate_response(serde_json::json!({
        "result": {"type": "object", "objectId": "old-world-1", "description": "old"}
    }));
    let service = fixture.installation.raw_service();
    let operation = tokio::spawn(async move {
        service
            .call(Command::<EvaluateHandle>::new(
                CallId::new(1),
                ObjectId::new(4),
                CallMetadata::default(),
                EvaluateHandleParams::expression("({ old: true })"),
            ))
            .await
            .unwrap()
            .into_outcome()
    });
    for _ in 0..100 {
        if !fixture.primary.calls().is_empty() {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert_eq!(fixture.primary.method_names(), ["Runtime.callFunctionOn"]);

    // Act
    fixture
        .installation
        .routes()
        .unbind(ObjectId::new(4))
        .unwrap();
    let replacement: Arc<dyn CdpClient> = fixture.secondary.clone();
    fixture
        .installation
        .routes()
        .bind(
            ObjectId::new(4),
            ObjectId::new(3),
            replacement,
            103,
            "world-replaced",
        )
        .unwrap();
    fixture.primary.set_evaluate_gate(None);
    gate.notify_waiters();
    let error = operation
        .await
        .unwrap()
        .expect_err("old-world result must not publish");

    // Assert
    assert_eq!(error.kind(), FailureKind::TargetClosed);
    assert_eq!(
        fixture.primary.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObject"]
    );
    assert_eq!(
        fixture
            .installation
            .lifecycle()
            .routes()
            .metrics()
            .unwrap()
            .current,
        0
    );
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(10)), Ok(false));
}

fn poll_now<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    let mut context = Context::from_waker(Waker::noop());
    future.poll(&mut context)
}

#[tokio::test]
async fn dropped_started_creation_finishes_and_disposes_unobserved_handle() {
    // Arrange
    let fixture = fixture(1);
    let gate = Arc::new(tokio::sync::Notify::new());
    fixture.primary.set_evaluate_gate(Some(Arc::clone(&gate)));
    fixture.primary.set_evaluate_response(serde_json::json!({
        "result": {"type": "object", "objectId": "abandoned-1", "description": "abandoned"}
    }));
    let continuation = CleanupQueue::new(1);
    let service = ContinuableHandleEvaluationService::new(
        fixture.installation.raw_service(),
        fixture.installation.lifecycle().dispose_service(),
        continuation.clone(),
    );
    let mut abandoned = Box::pin(service.call(Command::<EvaluateHandle>::new(
        CallId::new(1),
        ObjectId::new(4),
        CallMetadata::default(),
        EvaluateHandleParams::expression("({ abandoned: true })"),
    )));

    // Act
    assert!(poll_now(abandoned.as_mut()).is_pending());
    drop(abandoned);
    fixture.primary.set_evaluate_gate(None);
    gate.notify_waiters();
    let mut cleanup = continuation
        .try_take()
        .expect("continuation queue should be readable")
        .expect("started creation should transfer its future");
    let cleanup_result = poll_now(cleanup.as_mut());

    // Assert
    assert!(matches!(cleanup_result, Poll::Ready(Ok(()))));
    assert_eq!(
        fixture.primary.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObject"]
    );
    assert_eq!(
        fixture
            .installation
            .lifecycle()
            .routes()
            .metrics()
            .unwrap()
            .current,
        0
    );
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(10)), Ok(false));
}

/// Inject a native release failure only for selected portable identities.
/// Creation, route ownership, disposal and the continuation are real services.
struct SelectiveReleaseFailure<Inner> {
    inner: Arc<Inner>,
    client: Arc<FakeCdpClient>,
    failed_ids: Vec<ObjectId>,
}

impl<Inner> Service<Command<HandleDispose>> for SelectiveReleaseFailure<Inner>
where
    Inner: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>,
{
    type Response = CommandResult<HandleDispose>;
    type Error = Infallible;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Infallible>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, command: Command<HandleDispose>) -> Self::Future<'_> {
        Box::pin(async move {
            self.client.set_release_object_error(
                self.failed_ids
                    .contains(&command.object_id())
                    .then(|| "injected native release refusal".to_owned()),
            );
            self.inner.call(command).await
        })
    }
}

async fn assert_abandoned_batch_cleanup(failed_ids: Vec<ObjectId>) {
    // Arrange
    let fixture = fixture(4);
    let raw = fixture.installation.raw_service();
    fixture.primary.set_evaluate_response(serde_json::json!({
        "result": {"type": "object", "objectId": "batch-target"}
    }));
    let target = raw
        .call(Command::<EvaluateHandle>::new(
            CallId::new(1),
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::expression("({one: {}, two: {}, three: {}})"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    fixture.primary.set_evaluate_responses([
        serde_json::json!({"result": {"type": "object", "objectId": "snapshot"}}),
        serde_json::json!({"result": [
            {"name": "one", "enumerable": true, "value": {"type": "object", "objectId": "one"}},
            {"name": "two", "enumerable": true, "value": {"type": "object", "objectId": "two"}},
            {"name": "three", "enumerable": true, "value": {"type": "object", "objectId": "three"}}
        ]}),
    ]);
    let gate = Arc::new(tokio::sync::Notify::new());
    fixture.primary.set_evaluate_gate(Some(Arc::clone(&gate)));
    let continuation = CleanupQueue::new(1);
    let service = ContinuableHandleEvaluationService::new(
        raw,
        Arc::new(SelectiveReleaseFailure {
            inner: fixture.installation.lifecycle().raw_dispose_service(),
            client: Arc::clone(&fixture.primary),
            failed_ids: failed_ids.clone(),
        }),
        continuation.clone(),
    );
    let mut abandoned = Box::pin(service.call(Command::<HandleGetProperties>::new(
        CallId::new(2),
        target.object_id,
        CallMetadata::default(),
        GetPropertiesOptions,
    )));
    assert!(poll_now(abandoned.as_mut()).is_pending());
    drop(abandoned);
    fixture.primary.set_evaluate_gate(None);
    gate.notify_waiters();
    let cleanup = continuation
        .try_take()
        .unwrap()
        .expect("started batch owns cleanup");

    // Act
    let outcome = cleanup.await;

    // Assert
    assert_eq!(outcome.is_err(), !failed_ids.is_empty());
    if let Err(error) = outcome {
        assert_eq!(
            error.message(),
            "abandoned Handle evaluation result could not be disposed"
        );
    }
    let released = fixture
        .primary
        .calls()
        .into_iter()
        .filter(|(method, _)| method == "Runtime.releaseObject")
        .map(|(_, params)| params[0].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(released, ["snapshot", "one", "two", "three"]);
    let routes = fixture.installation.lifecycle().routes();
    assert_eq!(routes.metrics().unwrap().current, 1 + failed_ids.len());
    for id in [ObjectId::new(11), ObjectId::new(12), ObjectId::new(13)] {
        assert_eq!(routes.contains(id).unwrap(), failed_ids.contains(&id));
        assert_eq!(
            fixture.runtime.is_object_live(id).unwrap(),
            failed_ids.contains(&id)
        );
    }
    assert_eq!(
        fixture
            .resources
            .metrics()
            .unwrap()
            .class(ResourceClass::RemoteHandle)
            .current,
        (1 + failed_ids.len()) as u64
    );
    assert_eq!(continuation.metrics().unwrap().reserved, 0);
    assert_eq!(continuation.metrics().unwrap().queued, 0);

    // Release the still-visible target and failed routes through the same public
    // disposer; failed native releases must remain available for explicit retry.
    fixture.primary.set_release_object_error(None);
    let dispose = fixture.installation.lifecycle().raw_dispose_service();
    for id in std::iter::once(target.object_id).chain(failed_ids) {
        dispose
            .call(Command::<HandleDispose>::new(
                CallId::new(3),
                id,
                CallMetadata::default(),
                (),
            ))
            .await
            .unwrap()
            .into_outcome()
            .unwrap();
    }
    assert_eq!(routes.metrics().unwrap().current, 0);
    assert_eq!(
        fixture
            .resources
            .metrics()
            .unwrap()
            .class(ResourceClass::RemoteHandle)
            .current,
        0
    );
}

#[tokio::test]
async fn abandoned_batch_first_release_failure_does_not_skip_later_handles() {
    assert_abandoned_batch_cleanup(vec![ObjectId::new(11)]).await;
}

#[tokio::test]
async fn abandoned_batch_middle_release_failure_does_not_skip_last_handle() {
    assert_abandoned_batch_cleanup(vec![ObjectId::new(12)]).await;
}

#[tokio::test]
async fn abandoned_batch_multiple_release_failures_still_attempt_every_handle() {
    assert_abandoned_batch_cleanup(vec![ObjectId::new(11), ObjectId::new(12)]).await;
}

#[tokio::test]
async fn abandoned_batch_success_disposes_every_handle() {
    assert_abandoned_batch_cleanup(Vec::new()).await;
}
