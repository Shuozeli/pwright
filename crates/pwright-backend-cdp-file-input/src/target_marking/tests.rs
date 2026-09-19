use super::*;
use pwright_backend_cdp_locator_resolution::{
    CdpLocatorResolutionService, ResolutionProfile, ResolutionRequest, RetainedElementRequest,
};
use pwright_core::{AtomicObjectIdSource, ResourceLimits, Service};
use pwright_fake::FakeCdpClient;
use std::{
    pin::Pin,
    task::{Context, Poll, Waker},
};
use tokio::sync::Notify;

fn ledger(needs: ResourceRequirements) -> ProductionResourceLedger {
    let mut limits = ResourceLimits::deny_all(needs.scopes, needs.leases);
    for class in ResourceClass::ALL {
        let bound = needs.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    ProductionResourceLedger::new(limits, needs).unwrap()
}
fn fixture() -> (
    CdpTargetMarkingService,
    ProductionResourceLedger,
    Arc<FakeCdpClient>,
) {
    let profile = TargetMarkingProfile::new(ResourceScope::extension(182, 1), 1, 1).unwrap();
    let resources = ledger(profile.requirements());
    let service = CdpTargetMarkingService::new(
        profile,
        &resources,
        Arc::new(AtomicObjectIdSource::new(1).unwrap()),
    )
    .unwrap();
    let client = Arc::new(FakeCdpClient::from_html(""));
    client.set_evaluate_response(state_response());
    (service, resources, client)
}
fn state_response() -> Value {
    json!({"result":{"type":"object","objectId":"marker-state"}})
}
fn world(client: &Arc<FakeCdpClient>) -> MarkerWorld {
    MarkerWorld {
        client: client.clone(),
        utility_world: "utility-exact".into(),
        destruction: None,
    }
}
fn poll_now<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}
fn current(resources: &ProductionResourceLedger, class: ResourceClass) -> u64 {
    resources.metrics().unwrap().class(class).current
}
async fn target(client: &Arc<FakeCdpClient>, utility: &str, ordinary: bool) -> ResolvedElement {
    let profile = ResolutionProfile::new(ResourceScope::extension(182, 2), 2)
        .unwrap()
        .with_retained_reference()
        .unwrap();
    let resources = ledger(profile.requirements());
    let resolver = CdpLocatorResolutionService::new(
        profile,
        &resources,
        Arc::new(AtomicObjectIdSource::new(1).unwrap()),
    )
    .unwrap();
    if ordinary {
        client.set_evaluate_response(
            json!({"result":{"type":"object","subtype":"node","objectId":"adopted-target"}}),
        );
        resolver
            .call(ResolutionRequest {
                client: client.clone(),
                utility_world: utility.into(),
                selector: pwright_selector_model::Selector::css("input").unwrap(),
                destruction: None,
            })
            .await
            .unwrap()
            .unwrap()
    } else {
        client.set_object_backend_node("persistent-source", 73);
        client.set_adopted_node_response(
            73,
            19,
            json!({"object":{"type":"object","subtype":"node","objectId":"adopted-target"}}),
        );
        client.set_evaluate_response(json!({"result":{"type":"boolean","value":true}}));
        resolver
            .call(RetainedElementRequest {
                source_client: client.clone(),
                source_object_id: "persistent-source".into(),
                client: client.clone(),
                utility_world: utility.into(),
                utility_execution_context_id: 19,
                destruction: None,
            })
            .await
            .unwrap()
    }
}

#[test]
fn state_and_call_admissions_have_independent_capacity_and_explicit_resources() {
    // Arrange
    let (service, resources, client) = fixture();
    // Act
    let creation = service.create(world(&client)).unwrap();
    let call = service.admit().unwrap();
    let extra_state = service.create(world(&client));
    let extra_call = service.admit();
    // Assert
    assert!(matches!(extra_state, Err(TargetMarkingError::Capacity)));
    assert!(matches!(extra_call, Err(TargetMarkingError::Capacity)));
    assert_eq!(current(&resources, ResourceClass::RemoteHandle), 4);
    assert_eq!(
        current(&resources, ResourceClass::BackendStateBytes),
        ADMIN_BYTES + STATE_BYTES + CALL_BYTES
    );
    assert_eq!(resources.metrics().unwrap().active_leases, 3);
    assert!(client.calls().is_empty());
    drop(creation);
    drop(call);
    assert!(service.is_cleanup_healthy());
    assert_eq!(current(&resources, ResourceClass::RemoteHandle), 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    drop(service);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn unpolled_creation_reclaims_capacity_without_native_work() {
    // Arrange
    let (service, resources, client) = fixture();
    let creation = service.create(world(&client)).unwrap();
    // Act
    drop(creation);
    // Assert
    assert!(client.calls().is_empty());
    assert!(service.is_cleanup_healthy());
    assert_eq!(current(&resources, ResourceClass::RemoteHandle), 0);
    assert!(service.create(world(&client)).is_ok());
}

#[tokio::test]
async fn creation_uses_private_group_exact_world_and_keeps_init_allowance_until_release() {
    // Arrange
    let (service, resources, client) = fixture();
    // Act
    let state = service.create(world(&client)).unwrap().await.unwrap();
    // Assert
    let calls = client.calls();
    assert_eq!(calls.len(), 1);
    let params = &calls[0].1[0];
    assert_eq!(params["uniqueContextId"], "utility-exact");
    assert_eq!(params["functionDeclaration"], CREATE_SCRIPT);
    assert_eq!(params["objectGroup"], "pwright.target-marker.1");
    assert_eq!(params["arguments"], json!([]));
    assert_eq!(params["returnByValue"], false);
    assert_eq!(params["awaitPromise"], false);
    assert_eq!(params["userGesture"], true);
    assert_eq!(state.object_id(), "marker-state");
    assert_eq!(current(&resources, ResourceClass::RemoteHandle), 2);
    state.release().await.unwrap();
    assert_eq!(client.calls()[1].0, "Runtime.releaseObjectGroup");
    assert_eq!(client.calls()[1].1, vec![json!("pwright.target-marker.1")]);
    assert!(service.is_cleanup_healthy());
    assert_eq!(current(&resources, ResourceClass::RemoteHandle), 0);
}

#[tokio::test]
async fn malformed_initial_result_is_compensated_by_its_group_before_error() {
    for response in [
        json!({}),
        json!({"result":{"type":"undefined"}}),
        json!({"result":{"type":"object","subtype":"array","objectId":"unknown"}}),
        json!({"result":{"type":"object","objectId":""}}),
        json!({"result":{"type":"object","objectId":"x".repeat(MAX_NATIVE_ID+1)}}),
    ] {
        // Arrange
        let (service, resources, client) = fixture();
        client.set_evaluate_response(response);
        // Act
        let result = service.create(world(&client)).unwrap().await;
        // Assert
        assert!(matches!(result, Err(TargetMarkingError::Protocol(_))));
        assert_eq!(client.calls().len(), 2);
        assert_eq!(client.calls()[1].0, "Runtime.releaseObjectGroup");
        assert!(service.is_cleanup_healthy());
        assert_eq!(current(&resources, ResourceClass::RemoteHandle), 0);
    }
}

#[tokio::test]
async fn initial_exception_cleans_both_raw_owners_by_group_and_bounds_message() {
    // Arrange
    let (service, resources, client) = fixture();
    client.set_evaluate_response(json!({"result":{"type":"object","objectId":"result-owner"},
        "exceptionDetails":{"exception":{"objectId":"exception-owner","description":"界".repeat(2000)}}}));
    // Act
    let result = service.create(world(&client)).unwrap().await;
    // Assert
    let Err(TargetMarkingError::JavaScript(message)) = result else {
        panic!("expected JavaScript error")
    };
    assert!(message.len() <= DIAGNOSTIC_BYTES);
    assert_eq!(client.calls().len(), 2);
    assert_eq!(client.calls()[1].0, "Runtime.releaseObjectGroup");
    assert!(service.is_cleanup_healthy());
    assert_eq!(current(&resources, ResourceClass::RemoteHandle), 0);
}

#[tokio::test]
async fn initial_transport_failure_keeps_bounded_typed_protocol_error_after_cleanup() {
    // Arrange
    let (service, resources, client) = fixture();
    client.set_evaluate_errors(["界".repeat(2000)]);
    // Act
    let result = service.create(world(&client)).unwrap().await;
    // Assert
    let Err(TargetMarkingError::Backend(CdpError::Protocol { code, message })) = result else {
        panic!("typed protocol failure lost")
    };
    assert_eq!(code, -32000);
    assert!(message.len() <= DIAGNOSTIC_BYTES);
    assert!(message.capacity() <= DIAGNOSTIC_BYTES);
    assert!(service.is_cleanup_healthy());
    assert_eq!(current(&resources, ResourceClass::RemoteHandle), 0);
}

#[tokio::test]
async fn failed_initial_compensation_preserves_primary_and_poisons_future_admission() {
    // Arrange
    let (service, resources, client) = fixture();
    client.set_evaluate_response(json!({"exceptionDetails":{"text":"primary"}}));
    client.set_release_object_error(Some("cleanup".into()));
    // Act
    let result = service.create(world(&client)).unwrap().await;
    // Assert
    let Err(TargetMarkingError::Cleanup {
        primary: Some(primary),
        source: CdpError::Protocol { .. },
    }) = result
    else {
        panic!("cleanup classification lost")
    };
    assert!(matches!(*primary, TargetMarkingError::JavaScript(_)));
    assert!(!service.is_cleanup_healthy());
    assert!(matches!(
        service.admit(),
        Err(TargetMarkingError::Unhealthy)
    ));
    assert_eq!(current(&resources, ResourceClass::RemoteHandle), 0);
}

#[test]
fn dropped_started_creation_poisons_before_capacity_reuse() {
    // Arrange
    let (service, resources, client) = fixture();
    client.set_evaluate_gate(Some(Arc::new(Notify::new())));
    let mut creation = Box::pin(service.create(world(&client)).unwrap());
    assert!(poll_now(creation.as_mut()).is_pending());
    // Act
    drop(creation);
    // Assert
    assert_eq!(client.calls().len(), 1);
    assert!(!service.is_cleanup_healthy());
    assert!(matches!(
        service.create(world(&client)),
        Err(TargetMarkingError::Unhealthy)
    ));
    assert_eq!(current(&resources, ResourceClass::RemoteHandle), 0);
}

#[tokio::test]
async fn dropped_ready_state_poisons_without_claiming_native_release() {
    // Arrange
    let (service, resources, client) = fixture();
    let state = service.create(world(&client)).unwrap().await.unwrap();
    // Act
    drop(state);
    // Assert
    assert_eq!(client.calls().len(), 1);
    assert!(!service.is_cleanup_healthy());
    assert_eq!(current(&resources, ResourceClass::RemoteHandle), 0);
}

#[tokio::test]
async fn release_waits_for_ack_while_native_budget_and_state_slot_remain_charged() {
    // Arrange
    let (service, resources, client) = fixture();
    let state = service.create(world(&client)).unwrap().await.unwrap();
    let gate = Arc::new(Notify::new());
    client.set_release_object_group_gate(Some(gate.clone()));
    let mut release = Box::pin(state.release());
    // Act
    let first = poll_now(release.as_mut());
    // Assert
    assert!(first.is_pending());
    assert_eq!(current(&resources, ResourceClass::RemoteHandle), 2);
    assert_eq!(
        current(&resources, ResourceClass::BackendStateBytes),
        ADMIN_BYTES + STATE_BYTES
    );
    assert!(matches!(
        service.create(world(&client)),
        Err(TargetMarkingError::Capacity)
    ));
    gate.notify_one();
    release.await.unwrap();
    assert_eq!(current(&resources, ResourceClass::RemoteHandle), 0);
    assert!(service.is_cleanup_healthy());
}

struct Proof(AtomicBool);
impl ObjectGroupDestructionProof for Proof {
    fn all_group_worlds_destroyed(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[tokio::test]
async fn exact_retirement_receipt_discharges_group_without_native_release() {
    // Arrange
    let (service, resources, client) = fixture();
    let proof = Arc::new(Proof(AtomicBool::new(false)));
    let mut request = world(&client);
    request.destruction = Some(proof.clone());
    let state = service.create(request).unwrap().await.unwrap();
    proof.0.store(true, Ordering::Release);
    // Act
    state.release().await.unwrap();
    // Assert
    assert_eq!(client.calls().len(), 1);
    assert!(service.is_cleanup_healthy());
    assert_eq!(current(&resources, ResourceClass::RemoteHandle), 0);
}

#[tokio::test]
async fn exact_retirement_during_failed_release_discharges_after_settlement() {
    // Arrange
    let (service, resources, client) = fixture();
    let proof = Arc::new(Proof(AtomicBool::new(false)));
    let mut request = world(&client);
    request.destruction = Some(proof.clone());
    let state = service.create(request).unwrap().await.unwrap();
    client.set_release_object_error(Some("session detached is not itself proof".into()));
    let gate = Arc::new(Notify::new());
    client.set_release_object_group_gate(Some(gate.clone()));
    let mut release = Box::pin(state.release());
    assert!(poll_now(release.as_mut()).is_pending());
    // Act
    proof.0.store(true, Ordering::Release);
    gate.notify_one();
    let result = release.await;
    // Assert
    assert!(result.is_ok());
    assert!(service.is_cleanup_healthy());
    assert_eq!(current(&resources, ResourceClass::RemoteHandle), 0);
}

#[tokio::test]
async fn marking_uses_target_group_and_retains_state_after_call_cleanup() {
    // Arrange
    let (service, resources, client) = fixture();
    let state = service.create(world(&client)).unwrap().await.unwrap();
    let element = target(&client, "utility-exact", false).await;
    let target_group = element.object_group().to_owned();
    client.set_evaluate_response(json!({"result":{"type":"undefined"}}));
    let mut admission = service.admit().unwrap();
    // Act
    let result = admission.mark(&state, &element, "call@42").await;
    // Assert
    assert!(result.is_ok());
    let calls = client.calls();
    let params = &calls.last().unwrap().1[0];
    assert_eq!(params["uniqueContextId"], "utility-exact");
    assert_eq!(params["functionDeclaration"], MARK_SCRIPT);
    assert_eq!(params["objectGroup"], target_group);
    assert_ne!(target_group, state.object_group());
    assert_eq!(params["returnByValue"], true);
    assert_eq!(params["awaitPromise"], true);
    assert_eq!(params["userGesture"], true);
    assert_eq!(
        params["arguments"],
        json!([{"objectId":"marker-state"},{"objectId":"adopted-target"},{"value":"call@42"}])
    );
    assert_eq!(current(&resources, ResourceClass::RemoteHandle), 4);
    element.release().await.unwrap();
    admission.complete_cleanup();
    drop(admission);
    assert_eq!(current(&resources, ResourceClass::RemoteHandle), 2);
    assert!(service.is_cleanup_healthy());
    state.release().await.unwrap();
}

#[tokio::test]
async fn empty_call_id_consumes_one_shot_without_browser_marking() {
    // Arrange
    let (service, _, client) = fixture();
    let state = service.create(world(&client)).unwrap().await.unwrap();
    let element = target(&client, "utility-exact", false).await;
    let before = client.calls().len();
    let mut admission = service.admit().unwrap();
    // Act
    let result = admission.mark(&state, &element, "").await;
    // Assert
    assert!(result.is_ok());
    assert_eq!(client.calls().len(), before);
    assert!(matches!(
        admission.mark(&state, &element, "call@2").await,
        Err(TargetMarkingError::AlreadyUsed)
    ));
    element.release().await.unwrap();
    drop(admission);
    state.release().await.unwrap();
    assert!(service.is_cleanup_healthy());
}

#[tokio::test]
async fn foreign_client_wrong_world_and_ordinary_group_reject_before_mark_io() {
    for variant in 0..3 {
        // Arrange
        let (service, _, client) = fixture();
        let state = service.create(world(&client)).unwrap().await.unwrap();
        let peer = if variant == 0 {
            Arc::new(FakeCdpClient::from_html(""))
        } else {
            client.clone()
        };
        let utility = if variant == 1 {
            "different-world"
        } else {
            "utility-exact"
        };
        let element = target(&peer, utility, variant == 2).await;
        let before = client.calls().len();
        let mut admission = service.admit().unwrap();
        // Act
        let result = admission.mark(&state, &element, "call@1").await;
        // Assert
        assert!(matches!(result, Err(TargetMarkingError::InvalidTarget)));
        assert_eq!(client.calls().len(), before);
        element.release().await.unwrap();
        drop(admission);
        state.release().await.unwrap();
        assert!(service.is_cleanup_healthy());
    }
}

#[tokio::test]
async fn discarded_unpolled_mark_is_one_shot_but_creates_no_native_owner() {
    // Arrange
    let (service, _, client) = fixture();
    let state = service.create(world(&client)).unwrap().await.unwrap();
    let element = target(&client, "utility-exact", false).await;
    let before = client.calls().len();
    let mut admission = service.admit().unwrap();
    // Act
    drop(admission.mark(&state, &element, "call@1"));
    // Assert
    assert_eq!(client.calls().len(), before);
    assert!(matches!(
        admission.mark(&state, &element, "call@1").await,
        Err(TargetMarkingError::AlreadyUsed)
    ));
    element.release().await.unwrap();
    drop(admission);
    state.release().await.unwrap();
    assert!(service.is_cleanup_healthy());
}

#[tokio::test]
async fn marking_raw_exception_and_malformed_success_leave_cleanup_with_target_owner() {
    for response in [
        json!({"result":{"type":"object","objectId":"result"},"exceptionDetails":{"exception":{"objectId":"exception","description":"marker-threw"}}}),
        json!({"result":{"type":"number","value":0}}),
        json!({"result":{"type":"undefined","objectId":"unexpected"}}),
        json!({"result":{"type":"undefined","value":null}}),
        json!({}),
    ] {
        // Arrange
        let (service, resources, client) = fixture();
        let state = service.create(world(&client)).unwrap().await.unwrap();
        let element = target(&client, "utility-exact", false).await;
        let before = client.calls().len();
        client.set_evaluate_response(response);
        let mut admission = service.admit().unwrap();
        // Act
        let result = admission.mark(&state, &element, "call@1").await;
        // Assert
        assert!(matches!(
            result,
            Err(TargetMarkingError::JavaScript(_)) | Err(TargetMarkingError::Protocol(_))
        ));
        assert_eq!(client.calls().len(), before + 1);
        assert_eq!(current(&resources, ResourceClass::RemoteHandle), 4);
        element.release().await.unwrap();
        admission.complete_cleanup();
        drop(admission);
        assert!(service.is_cleanup_healthy());
        state.release().await.unwrap();
    }
}

#[tokio::test]
async fn omitted_target_cleanup_receipt_poisons_call_admission_even_after_success() {
    // Arrange
    let (service, _, client) = fixture();
    let state = service.create(world(&client)).unwrap().await.unwrap();
    let element = target(&client, "utility-exact", false).await;
    client.set_evaluate_response(json!({"result":{"type":"undefined"}}));
    let mut admission = service.admit().unwrap();
    admission.mark(&state, &element, "call@1").await.unwrap();
    // Act
    drop(admission);
    // Assert
    assert!(!service.is_cleanup_healthy());
    assert!(matches!(
        service.admit(),
        Err(TargetMarkingError::Unhealthy)
    ));
    element.release().await.unwrap();
    state.release().await.unwrap();
}

#[tokio::test]
async fn mark_call_allowance_remains_charged_through_target_release_ack() {
    // Arrange
    let (service, resources, client) = fixture();
    let state = service.create(world(&client)).unwrap().await.unwrap();
    let element = target(&client, "utility-exact", false).await;
    client.set_evaluate_response(json!({"result":{"type":"undefined"}}));
    let mut admission = service.admit().unwrap();
    admission.mark(&state, &element, "call@1").await.unwrap();
    let gate = Arc::new(Notify::new());
    client.set_release_object_group_gate(Some(gate.clone()));
    let mut release = Box::pin(element.release());
    // Act
    let first = poll_now(release.as_mut());
    // Assert
    assert!(first.is_pending());
    assert_eq!(current(&resources, ResourceClass::RemoteHandle), 4);
    assert!(matches!(service.admit(), Err(TargetMarkingError::Capacity)));
    gate.notify_one();
    release.await.unwrap();
    admission.complete_cleanup();
    drop(admission);
    assert_eq!(current(&resources, ResourceClass::RemoteHandle), 2);
    client.set_release_object_group_gate(None);
    state.release().await.unwrap();
}

#[tokio::test]
async fn service_drop_does_not_release_administration_before_last_native_owner() {
    // Arrange
    let (service, resources, client) = fixture();
    let state = service.create(world(&client)).unwrap().await.unwrap();
    // Act
    drop(service);
    // Assert
    assert_eq!(
        current(&resources, ResourceClass::BackendStateBytes),
        ADMIN_BYTES + STATE_BYTES
    );
    assert_eq!(resources.metrics().unwrap().active_leases, 2);
    state.release().await.unwrap();
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn invalid_world_profiles_and_exhausted_ids_refuse_without_native_io() {
    // Arrange
    let (mut service, resources, client) = fixture();
    service.ids = Arc::new(AtomicObjectIdSource::new(u64::MAX).unwrap());
    let mut invalid = world(&client);
    invalid.utility_world = "x".repeat(MAX_NATIVE_ID + 1).into();
    // Act
    let bad_world = service.create(invalid);
    let exhausted = service.create(world(&client));
    // Assert
    assert!(matches!(bad_world, Err(TargetMarkingError::InvalidWorld)));
    assert!(matches!(
        exhausted,
        Err(TargetMarkingError::IdentifierExhausted)
    ));
    assert!(TargetMarkingProfile::new(ResourceScope::Connection, 0, 1).is_err());
    assert!(TargetMarkingProfile::new(ResourceScope::Connection, usize::MAX, 1).is_err());
    assert!(client.calls().is_empty());
    assert_eq!(current(&resources, ResourceClass::RemoteHandle), 0);
}
