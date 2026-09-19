use super::*;
use pwright_core::{AtomicObjectIdSource, ResourceLimits};
use pwright_fake::FakeCdpClient;
use serde_json::json;
use std::task::{Context, Poll, Waker};
use tokio::sync::Notify;

fn fixture() -> (
    CdpLocatorResolutionService,
    Arc<FakeCdpClient>,
    ProductionResourceLedger,
) {
    let profile = ResolutionProfile::new(ResourceScope::extension(168, 1), 2)
        .unwrap()
        .with_retained_reference()
        .unwrap();
    let requirements = profile.requirements();
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let required = requirements.class(class);
        limits = limits.with_class(class, required.global, required.per_scope);
    }
    let resources = ProductionResourceLedger::new(limits, requirements).unwrap();
    let service = CdpLocatorResolutionService::new(
        profile,
        &resources,
        Arc::new(AtomicObjectIdSource::new(1).unwrap()),
    )
    .unwrap();
    let client = Arc::new(FakeCdpClient::from_html(""));
    client.set_object_backend_node("source-element", 73);
    client.set_adopted_node_response(73, 19, adopted());
    client.set_evaluate_response(json!({"result":{"type":"boolean", "value":true}}));
    (service, client, resources)
}

fn adopted() -> Value {
    json!({"object":{"type":"object", "subtype":"node", "objectId":"adopted-element"}})
}

#[test]
fn retained_scratch_is_explicit_idempotent_and_does_not_add_leases_or_selector_bytes() {
    // Arrange
    let base = ResolutionProfile::new(ResourceScope::extension(168, 2), 5).unwrap();
    let ordinary = base.requirements();
    // Act
    let selected = base.with_retained_reference().unwrap();
    let repeated = selected.with_retained_reference().unwrap().requirements();
    // Assert
    assert_eq!(selected.requirements(), repeated);
    assert_eq!(
        repeated.class(ResourceClass::BackendStateBytes).global,
        ordinary.class(ResourceClass::BackendStateBytes).global + 2 * ATTEMPT_BYTES
    );
    assert_eq!(repeated.leases, ordinary.leases);
    assert_eq!(repeated.scopes, ordinary.scopes);
    assert_eq!(
        repeated.class(ResourceClass::RemoteHandle),
        ordinary.class(ResourceClass::RemoteHandle)
    );
    assert_eq!(
        repeated.class(ResourceClass::SelectorBytes),
        ordinary.class(ResourceClass::SelectorBytes)
    );
}

#[test]
fn retained_profile_rejects_fewer_than_two_native_owner_units() {
    // Arrange
    let base = ResolutionProfile::new(ResourceScope::extension(168, 2), 1).unwrap();
    // Act
    let result = base.with_retained_reference();
    // Assert
    assert!(matches!(result, Err(ResolutionError::Setup(_))));
}

#[tokio::test]
async fn compiled_retained_feature_does_not_implicitly_select_its_resource_profile() {
    // Arrange
    let (mut service, client, resources) = fixture();
    // A plain profile remains selector-only even in a feature-enabled binary.
    service.profile = ResolutionProfile::new(ResourceScope::extension(168, 1), 2).unwrap();
    // Act
    let result = service.call(request(&client)).await;
    // Assert
    assert!(matches!(result, Err(ResolutionError::Setup(_))));
    assert!(client.calls().is_empty());
    assert_balanced(&resources);
}

fn request(client: &Arc<FakeCdpClient>) -> RetainedElementRequest {
    RetainedElementRequest {
        source_client: client.clone(),
        source_object_id: "source-element".into(),
        client: client.clone(),
        utility_world: "exact-utility-world".into(),
        utility_execution_context_id: 19,
        destruction: None,
    }
}

#[tokio::test]
async fn retained_owner_reports_settlement_release_without_new_admission() {
    // Arrange
    let (service, client, resources) = fixture();
    let owner = service.call(request(&client)).await.unwrap();
    let before = resources.metrics().unwrap();

    // Act
    let selected = owner.uses_settlement_release();

    // Assert
    assert!(selected);
    assert_eq!(resources.metrics().unwrap(), before);
    owner.release().await.unwrap();
    assert_balanced(&resources);
}

#[tokio::test]
async fn selector_owner_does_not_implicitly_gain_settlement_release() {
    // Arrange: even a retained-capable service can produce ordinary owners.
    let (service, client, resources) = fixture();
    client.set_evaluate_response(json!({"result": {
        "type": "object", "subtype": "node", "objectId": "selected-node"
    }}));
    let owner = service
        .call(ResolutionRequest {
            client,
            utility_world: "exact-utility-world".into(),
            selector: pwright_selector_model::Selector::css("input").unwrap(),
            destruction: None,
        })
        .await
        .unwrap()
        .unwrap();
    let before = resources.metrics().unwrap();

    // Act
    let selected = owner.uses_settlement_release();

    // Assert
    assert!(!selected);
    assert_eq!(resources.metrics().unwrap(), before);
    owner.release().await.unwrap();
    // Ordinary resolution legitimately used selector scratch earlier; unlike
    // retained adoption, its historical selector high-water mark is not zero.
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

fn poll_now<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

fn assert_admitted(resources: &ProductionResourceLedger) {
    let metrics = resources.metrics().unwrap();
    assert_eq!(metrics.active_leases, 1);
    assert_eq!(metrics.class(ResourceClass::RemoteHandle).current, 2);
    assert_eq!(
        metrics.class(ResourceClass::BackendStateBytes).current,
        2 * OWNER_BYTES
    );
    assert_eq!(metrics.class(ResourceClass::SelectorBytes).current, 0);
    assert_eq!(metrics.class(ResourceClass::SelectorBytes).peak, 0);
}

fn assert_balanced(resources: &ProductionResourceLedger) {
    let metrics = resources.metrics().unwrap();
    assert_eq!(metrics.active_leases, 0);
    assert_eq!(metrics.class(ResourceClass::RemoteHandle).current, 0);
    assert_eq!(metrics.class(ResourceClass::BackendStateBytes).current, 0);
    assert_eq!(metrics.class(ResourceClass::SelectorBytes).peak, 0);
}

fn assert_group_only_cleanup(client: &FakeCdpClient) {
    let releases = client
        .calls()
        .into_iter()
        .filter(|(method, _)| method.starts_with("Runtime.release"))
        .collect::<Vec<_>>();
    assert_eq!(
        releases,
        [(
            "Runtime.releaseObjectGroup".into(),
            vec![json!("pwright.retained.1")]
        )]
    );
}

struct Destruction(AtomicBool);

impl ObjectGroupDestructionProof for Destruction {
    fn all_group_worlds_destroyed(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[tokio::test]
async fn exact_native_adoption_uses_no_selector_and_releases_only_its_group() {
    // Arrange
    let (service, client, resources) = fixture();

    // Act
    let owner = service.call(request(&client)).await.unwrap();
    let id = owner.object_id().to_owned();
    let world = owner.utility_world().to_owned();
    let group = owner.object_group().to_owned();
    let same_client = Arc::ptr_eq(&owner.client(), &request(&client).client);
    assert_admitted(&resources);
    owner.release().await.unwrap();

    // Assert
    assert_eq!(id, "adopted-element");
    assert_eq!(world, "exact-utility-world");
    assert_eq!(group, "pwright.retained.1");
    assert!(same_client);
    let calls = client.calls();
    assert_eq!(
        client.method_names(),
        [
            "DOM.describeNode",
            "DOM.resolveNode",
            "Runtime.callFunctionOn",
            "Runtime.releaseObjectGroup"
        ]
    );
    assert_eq!(calls[0].1, [json!({"objectId":"source-element"})]);
    assert_eq!(
        calls[1].1,
        [json!({"backendNodeId":73, "executionContextId":19, "objectGroup":group})]
    );
    assert_eq!(
        calls[2].1,
        [json!({
            "uniqueContextId":"exact-utility-world", "functionDeclaration":VALIDATE_WORLD,
            "arguments":[{"objectId":"adopted-element"}], "objectGroup":group,
            "returnByValue":true, "awaitPromise":true
        })]
    );
    assert_group_only_cleanup(&client);
    assert!(service.is_cleanup_healthy());
    assert_balanced(&resources);
}

#[tokio::test]
async fn retained_admission_is_synchronous_two_native_units_without_selector_storage() {
    // Arrange
    let (service, client, resources) = fixture();

    // Act
    let operation = service.call(request(&client));
    let admitted = resources.metrics().unwrap();
    let rejected = service.call(request(&client)).await;
    drop(operation);

    // Assert
    assert_eq!(admitted.class(ResourceClass::RemoteHandle).current, 2);
    assert_eq!(
        admitted.class(ResourceClass::BackendStateBytes).current,
        2 * OWNER_BYTES + ATTEMPT_BYTES
    );
    assert_eq!(admitted.class(ResourceClass::SelectorBytes).current, 0);
    assert!(matches!(rejected, Err(ResolutionError::Resource(_))));
    assert!(client.calls().is_empty());
    assert!(service.is_cleanup_healthy());
    assert_balanced(&resources);
}

#[tokio::test]
async fn source_session_and_native_identity_validation_reject_before_any_io() {
    for invalid in 0..7 {
        // Arrange
        let (service, client, resources) = fixture();
        let other = Arc::new(FakeCdpClient::from_html(""));
        let mut input = request(&client);
        match invalid {
            0 => input.source_client = other.clone(),
            1 => input.source_object_id = "".into(),
            2 => input.source_object_id = "é".repeat(NATIVE_ID_BYTES).into(),
            3 => input.utility_world = "".into(),
            4 => input.utility_world = "é".repeat(NATIVE_ID_BYTES).into(),
            5 => input.utility_execution_context_id = 0,
            _ => input.utility_execution_context_id = -1,
        }

        // Act
        let result = service.call(input).await;

        // Assert
        assert!(matches!(result, Err(ResolutionError::Setup(_))));
        assert!(client.calls().is_empty());
        assert!(other.calls().is_empty());
        assert!(service.is_cleanup_healthy());
        assert_balanced(&resources);
    }
}

#[tokio::test]
async fn invalid_described_backend_node_never_starts_native_group_acquisition() {
    // Arrange
    let (service, client, resources) = fixture();
    client.set_object_backend_node("source-element", 0);

    // Act
    let result = service.call(request(&client)).await;

    // Assert
    assert!(matches!(result, Err(ResolutionError::Protocol(_))));
    assert_eq!(client.method_names(), ["DOM.describeNode"]);
    assert!(service.is_cleanup_healthy());
    assert_balanced(&resources);
}

#[tokio::test]
async fn pending_resolve_and_probe_keep_admission_until_group_cleanup() {
    for during_resolve in [true, false] {
        // Arrange
        let (service, client, resources) = fixture();
        let gate = Arc::new(Notify::new());
        if during_resolve {
            client.set_adoption_gate(Some(gate.clone()));
        } else {
            client.set_evaluate_gate(Some(gate.clone()));
        }

        // Act
        let mut operation = service.call(request(&client));
        let pending = poll_now(operation.as_mut());
        let blocked = resources.metrics().unwrap();
        let rejected = service.call(request(&client)).await;
        gate.notify_one();
        let owner = operation.await.unwrap();
        let acquired = resources.metrics().unwrap();
        owner.release().await.unwrap();

        // Assert
        assert!(pending.is_pending());
        assert_eq!(blocked.class(ResourceClass::RemoteHandle).current, 2);
        assert_eq!(blocked.class(ResourceClass::SelectorBytes).peak, 0);
        assert_eq!(acquired.class(ResourceClass::RemoteHandle).current, 2);
        assert!(matches!(rejected, Err(ResolutionError::Resource(_))));
        assert_group_only_cleanup(&client);
        assert!(service.is_cleanup_healthy());
        assert_balanced(&resources);
    }
}

#[tokio::test]
async fn pending_successful_owner_release_keeps_both_native_units_until_ack() {
    // Arrange
    let (service, client, resources) = fixture();
    let owner = service.call(request(&client)).await.unwrap();
    let gate = Arc::new(Notify::new());
    client.set_release_object_group_gate(Some(gate.clone()));

    // Act
    let mut release = Box::pin(owner.release());
    let pending = poll_now(release.as_mut());
    let blocked = resources.metrics().unwrap();
    let rejected = service.call(request(&client)).await;
    gate.notify_one();
    let result = release.await;

    // Assert
    assert!(pending.is_pending());
    assert_eq!(blocked.class(ResourceClass::RemoteHandle).current, 2);
    assert_eq!(blocked.active_leases, 1);
    assert!(matches!(rejected, Err(ResolutionError::Resource(_))));
    assert!(result.is_ok());
    assert_group_only_cleanup(&client);
    assert!(service.is_cleanup_healthy());
    assert_balanced(&resources);
}

#[tokio::test]
async fn malformed_missing_oversized_or_source_alias_native_ids_are_group_compensated() {
    for response in [
        json!({}),
        json!({"object":null}),
        json!({"object":{"type":"object", "subtype":"node"}}),
        json!({"object":{"type":"object", "subtype":"node", "objectId":""}}),
        json!({"object":{"type":"object", "subtype":"node", "objectId":17}}),
        json!({"object":{"type":"object", "subtype":"node", "objectId":"é".repeat(NATIVE_ID_BYTES)}}),
        json!({"object":{"type":"object", "subtype":"node", "objectId":"source-element"}}),
        json!({"object":{"type":"object", "objectId":"non-node-owner"}}),
        json!({"object":{"type":"object", "subtype":"node", "objectId":"adopted-element"}, "exceptionDetails":{"exception":{"objectId":"exception-owner"}}}),
    ] {
        // Arrange
        let (service, client, resources) = fixture();
        client.set_adopted_node_response(73, 19, response);

        // Act
        let result = service.call(request(&client)).await;

        // Assert
        assert!(matches!(result, Err(ResolutionError::Protocol(_))));
        assert_eq!(
            client.method_names(),
            [
                "DOM.describeNode",
                "DOM.resolveNode",
                "Runtime.releaseObjectGroup"
            ]
        );
        assert_group_only_cleanup(&client);
        assert!(service.is_cleanup_healthy());
        assert_balanced(&resources);
    }
}

#[tokio::test]
async fn malformed_adoption_keeps_owner_admission_while_group_compensation_is_blocked() {
    // Arrange
    let (service, client, resources) = fixture();
    client.set_adopted_node_response(73, 19, json!({"object":{"objectId":"untyped-owner"}}));
    let gate = Arc::new(Notify::new());
    client.set_release_object_group_gate(Some(gate.clone()));

    // Act
    let mut operation = service.call(request(&client));
    let pending = poll_now(operation.as_mut());
    let blocked = resources.metrics().unwrap();
    gate.notify_one();
    let result = operation.await;

    // Assert
    assert!(pending.is_pending());
    assert_eq!(blocked.class(ResourceClass::RemoteHandle).current, 2);
    assert_eq!(
        blocked.class(ResourceClass::BackendStateBytes).current,
        2 * OWNER_BYTES + ATTEMPT_BYTES
    );
    assert!(matches!(result, Err(ResolutionError::Protocol(_))));
    assert_group_only_cleanup(&client);
    assert!(service.is_cleanup_healthy());
    assert_balanced(&resources);
}

#[tokio::test]
async fn failed_unique_world_probe_ignores_old_destruction_proof_until_actual_group_ack() {
    // Arrange
    let (service, client, resources) = fixture();
    let proof = Arc::new(Destruction(AtomicBool::new(true)));
    let mut input = request(&client);
    input.destruction = Some(proof);
    client.set_evaluate_errors(["Cannot find context with specified id".into()]);
    let gate = Arc::new(Notify::new());
    client.set_release_object_group_gate(Some(gate.clone()));

    // Act
    let mut operation = service.call(input);
    let pending = poll_now(operation.as_mut());
    let blocked = resources.metrics().unwrap();
    gate.notify_one();
    let result = operation.await;

    // Assert
    assert!(
        pending.is_pending(),
        "an old-world proof must not skip the new group's acknowledgement"
    );
    assert_eq!(blocked.class(ResourceClass::RemoteHandle).current, 2);
    assert!(matches!(result, Err(ResolutionError::Backend(_))));
    assert_group_only_cleanup(&client);
    assert!(service.is_cleanup_healthy());
    assert_balanced(&resources);
}

#[tokio::test]
async fn failed_probe_and_release_ignore_old_world_proof_and_poison_future_admission() {
    // Arrange
    let (service, client, resources) = fixture();
    let mut input = request(&client);
    input.destruction = Some(Arc::new(Destruction(AtomicBool::new(true))));
    client.set_evaluate_errors(["Cannot find context with specified id".into()]);
    client.set_release_object_error(Some("Session closed".into()));

    // Act
    let result = service.call(input).await;
    let calls_after_failure = client.calls();
    let rejected = service.call(request(&client)).await;

    // Assert
    assert!(matches!(
        result,
        Err(ResolutionError::Cleanup {
            primary: Some(_),
            ..
        })
    ));
    assert!(!service.is_cleanup_healthy());
    assert!(matches!(rejected, Err(ResolutionError::Admission(_))));
    assert_eq!(client.calls(), calls_after_failure);
    assert_group_only_cleanup(&client);
    assert_balanced(&resources);
}

#[tokio::test]
async fn malformed_probe_results_cannot_activate_a_preexisting_destruction_proof() {
    for response in [
        json!({"result":{"type":"boolean", "value":false}}),
        json!({"result":{"value":true}}),
        json!({"result":{"type":"boolean", "value":true, "objectId":"probe-owner"}}),
        json!({"result":{"type":"boolean", "value":true}, "exceptionDetails":{"exception":{"objectId":"probe-exception"}}}),
    ] {
        // Arrange
        let (service, client, resources) = fixture();
        let mut input = request(&client);
        input.destruction = Some(Arc::new(Destruction(AtomicBool::new(true))));
        client.set_evaluate_response(response);

        // Act
        let result = service.call(input).await;

        // Assert
        assert!(matches!(result, Err(ResolutionError::Protocol(_))));
        assert_group_only_cleanup(&client);
        assert!(service.is_cleanup_healthy());
        assert_balanced(&resources);
    }
}

#[tokio::test]
async fn validated_world_accepts_later_authoritative_destruction_proof() {
    // Arrange
    let (service, client, resources) = fixture();
    let proof = Arc::new(Destruction(AtomicBool::new(false)));
    let mut input = request(&client);
    input.destruction = Some(proof.clone());
    let owner = service.call(input).await.unwrap();
    proof.0.store(true, Ordering::Release);
    client.set_release_object_error(Some("this release must not be sent".into()));

    // Act
    let result = owner.release().await;

    // Assert
    assert!(result.is_ok());
    assert_eq!(
        client.method_names(),
        [
            "DOM.describeNode",
            "DOM.resolveNode",
            "Runtime.callFunctionOn"
        ]
    );
    assert!(service.is_cleanup_healthy());
    assert_balanced(&resources);
}

#[tokio::test]
async fn successful_owner_release_failure_without_proof_poisons_service() {
    // Arrange
    let (service, client, resources) = fixture();
    let owner = service.call(request(&client)).await.unwrap();
    client.set_release_object_error(Some("release failed".into()));

    // Act
    let result = owner.release().await;

    // Assert
    assert!(matches!(
        result,
        Err(ResolutionError::Cleanup { primary: None, .. })
    ));
    assert!(!service.is_cleanup_healthy());
    assert_group_only_cleanup(&client);
    assert_balanced(&resources);
}

#[tokio::test]
async fn dropping_started_numeric_resolve_cannot_use_unvalidated_old_world_proof() {
    // Arrange
    let (service, client, resources) = fixture();
    let mut input = request(&client);
    input.destruction = Some(Arc::new(Destruction(AtomicBool::new(true))));
    client.set_adoption_gate(Some(Arc::new(Notify::new())));
    let mut operation = service.call(input);
    assert!(poll_now(operation.as_mut()).is_pending());

    // Act: a raw caller violates the required continuation contract.
    drop(operation);

    // Assert
    assert!(!service.is_cleanup_healthy());
    assert_eq!(
        client.method_names(),
        ["DOM.describeNode", "DOM.resolveNode"]
    );
    assert_balanced(&resources);
}
