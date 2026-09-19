use super::*;
use pwright_core::{AtomicObjectIdSource, ResourceLimits};
use pwright_fake::FakeCdpClient;
use serde_json::json;

fn fixture() -> (
    CdpLocatorResolutionService,
    Arc<FakeCdpClient>,
    ProductionResourceLedger,
) {
    let profile = ResolutionProfile::new(ResourceScope::extension(86, 1), 1).unwrap();
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
    (service, Arc::new(FakeCdpClient::from_html("")), resources)
}
fn request(client: Arc<FakeCdpClient>) -> ResolutionRequest {
    ResolutionRequest {
        client,
        utility_world: "utility-1".into(),
        selector: Selector::css("button").unwrap(),
        destruction: None,
    }
}
fn node() -> Value {
    json!({"result":{"type":"object","subtype":"node","objectId":"element-1"}})
}

struct Destruction(AtomicBool);
impl ObjectGroupDestructionProof for Destruction {
    fn all_group_worlds_destroyed(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

struct ReleaseFence {
    pending: AtomicBool,
    retired: AtomicBool,
    waiter: std::sync::Mutex<Option<std::task::Waker>>,
}
impl ObjectGroupDestructionProof for ReleaseFence {
    fn all_group_worlds_destroyed(&self) -> bool {
        false
    }
    fn all_group_objects_retired(&self) -> bool {
        self.retired.load(Ordering::Acquire)
    }
    fn poll_release_ready(&self, cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
        let mut waiter = self.waiter.lock().unwrap();
        if self.pending.load(Ordering::Acquire) {
            *waiter = Some(cx.waker().clone());
            std::task::Poll::Pending
        } else {
            std::task::Poll::Ready(())
        }
    }
}
async fn release_racing_retirement(succeeded: bool) {
    // Arrange
    let (service, client, resources) = fixture();
    client.set_evaluate_response(node());
    let proof = Arc::new(ReleaseFence {
        pending: AtomicBool::new(false),
        retired: AtomicBool::new(false),
        waiter: std::sync::Mutex::new(None),
    });
    let mut input = request(client.clone());
    input.destruction = Some(proof.clone());
    let owner = service.call(input).await.unwrap().unwrap();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_release_object_group_gate(Some(gate.clone()));
    client.set_release_object_error(Some("Session closed".into()));
    let mut release = Box::pin(owner.release());
    assert!(
        release
            .as_mut()
            .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
            .is_pending()
    );
    proof.pending.store(true, Ordering::Release);
    gate.notify_waiters();
    assert!(
        release
            .as_mut()
            .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
            .is_pending()
    );
    assert!(service.is_cleanup_healthy());
    assert!(resources.metrics().unwrap().active_leases > 0);
    // Act: the acknowledgement, not the earlier failed command, decides proof.
    proof.retired.store(succeeded, Ordering::Release);
    proof.pending.store(false, Ordering::Release);
    let waiter = proof.waiter.lock().unwrap().take();
    if let Some(waiter) = waiter {
        waiter.wake();
    }
    let result = release.await;
    // Assert
    assert_eq!(result.is_ok(), succeeded);
    assert_eq!(service.is_cleanup_healthy(), succeeded);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    assert_eq!(
        client.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
    );
}
#[tokio::test]
async fn failed_release_waits_for_the_successful_retirement_acknowledgement() {
    release_racing_retirement(true).await;
}
#[tokio::test]
async fn failed_release_remains_an_error_after_failed_retirement() {
    release_racing_retirement(false).await;
}

struct SessionRetirement(AtomicBool);
impl ObjectGroupDestructionProof for SessionRetirement {
    fn all_group_worlds_destroyed(&self) -> bool {
        false
    }
    fn all_group_objects_retired(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[tokio::test]
async fn explicit_object_retirement_does_not_require_a_false_world_destruction_claim() {
    // Arrange
    let (service, client, resources) = fixture();
    client.set_evaluate_response(node());
    let proof = Arc::new(SessionRetirement(AtomicBool::new(false)));
    let mut input = request(client.clone());
    input.destruction = Some(proof.clone());
    let owner = service.call(input).await.unwrap().unwrap();
    client.set_release_object_error(Some("Session closed".into()));
    // Act
    proof.0.store(true, Ordering::Release);
    let result = owner.release().await;
    // Assert
    assert!(result.is_ok());
    assert!(!proof.all_group_worlds_destroyed());
    assert!(service.is_cleanup_healthy());
    assert_eq!(client.method_names(), ["Runtime.callFunctionOn"]);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn unacknowledged_object_retirement_keeps_release_failure_visible() {
    // Arrange
    let (service, client, _) = fixture();
    client.set_evaluate_response(node());
    let mut input = request(client.clone());
    input.destruction = Some(Arc::new(SessionRetirement(AtomicBool::new(false))));
    let owner = service.call(input).await.unwrap().unwrap();
    client.set_release_object_error(Some("Session closed".into()));
    // Act
    let result = owner.release().await;
    // Assert
    assert!(matches!(result, Err(ResolutionError::Cleanup { .. })));
    assert!(!service.is_cleanup_healthy());
}

#[tokio::test]
async fn destruction_observed_during_native_release_discharges_its_failure() {
    // Arrange
    let (service, client, resources) = fixture();
    client.set_evaluate_response(node());
    let proof = Arc::new(Destruction(AtomicBool::new(false)));
    let mut input = request(client.clone());
    input.destruction = Some(proof.clone());
    let owner = service.call(input).await.unwrap().unwrap();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_release_object_group_gate(Some(gate.clone()));
    client.set_release_object_error(Some("Session closed".into()));
    let mut release = Box::pin(owner.release());
    assert!(
        release
            .as_mut()
            .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
            .is_pending()
    );
    // Act
    proof.0.store(true, Ordering::Release);
    gate.notify_one();
    let result = release.await;
    // Assert
    assert!(result.is_ok());
    assert!(service.is_cleanup_healthy());
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    assert_eq!(
        client.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
    );
}

#[tokio::test]
async fn confirmed_destruction_retires_an_owner_without_sending_native_release() {
    // Arrange
    let (service, client, resources) = fixture();
    client.set_evaluate_response(node());
    let proof = Arc::new(Destruction(AtomicBool::new(false)));
    let mut input = request(client.clone());
    input.destruction = Some(proof.clone());
    let owner = service.call(input).await.unwrap().unwrap();
    proof.0.store(true, Ordering::Release);
    client.set_release_object_error(Some("Session closed".into()));
    // Act
    let result = owner.release().await;
    // Assert
    assert!(result.is_ok());
    assert_eq!(client.method_names(), ["Runtime.callFunctionOn"]);
    assert!(service.is_cleanup_healthy());
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn unconfirmed_destruction_does_not_mask_a_closed_session_release_failure() {
    // Arrange
    let (service, client, _) = fixture();
    client.set_evaluate_response(node());
    let mut input = request(client.clone());
    input.destruction = Some(Arc::new(Destruction(AtomicBool::new(false))));
    let owner = service.call(input).await.unwrap().unwrap();
    client.set_release_object_error(Some("Session closed".into()));
    // Act
    let result = owner.release().await;
    // Assert
    assert!(matches!(result, Err(ResolutionError::Cleanup { .. })));
    assert_eq!(
        client.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
    );
    assert!(!service.is_cleanup_healthy());
}

#[tokio::test]
async fn dropping_an_owner_requires_proof_before_it_can_keep_admission_healthy() {
    for destroyed in [false, true] {
        // Arrange
        let (service, client, resources) = fixture();
        client.set_evaluate_response(node());
        let proof = Arc::new(Destruction(AtomicBool::new(false)));
        let mut input = request(client.clone());
        input.destruction = Some(proof.clone());
        let owner = service.call(input).await.unwrap().unwrap();
        proof.0.store(destroyed, Ordering::Release);
        // Act
        drop(owner);
        // Assert
        assert_eq!(service.is_cleanup_healthy(), destroyed);
        assert_eq!(resources.metrics().unwrap().active_leases, 0);
        assert_eq!(client.method_names(), ["Runtime.callFunctionOn"]);
    }
}

#[tokio::test]
async fn strict_match_retains_only_owner_budget_until_group_release() {
    // Arrange
    let (service, client, resources) = fixture();
    client.set_evaluate_response(node());
    // Act
    let element = service
        .call(request(client.clone()))
        .await
        .unwrap()
        .unwrap();
    let retained = resources.metrics().unwrap();
    let id = element.object_id().to_owned();
    element.release().await.unwrap();
    // Assert
    assert_eq!(id, "element-1");
    assert_eq!(retained.active_leases, 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    let calls = client.calls();
    assert_eq!(calls[0].1[0]["uniqueContextId"], "utility-1");
    assert_eq!(calls[0].1[0]["returnByValue"], false);
    assert_eq!(calls[0].1[0]["awaitPromise"], false);
    assert_eq!(calls[0].1[0]["objectGroup"], "pwright.locator.1");
    assert_eq!(calls[1].0, "Runtime.releaseObjectGroup");
    assert_eq!(calls[1].1[0], "pwright.locator.1");
}

#[tokio::test]
async fn empty_query_returns_no_element_and_releases_its_group() {
    // Arrange
    let (service, client, resources) = fixture();
    // Act
    let result = service.call(request(client.clone())).await.unwrap();
    // Assert
    assert!(result.is_none());
    assert_eq!(
        client.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
    );
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn multiple_matches_report_strictness_without_choosing_an_element() {
    // Arrange
    let (service, client, resources) = fixture();
    client.set_evaluate_response(json!({"result":{"type":"number","value":3}}));
    // Act
    let result = service.call(request(client.clone())).await;
    // Assert
    assert!(matches!(
        result,
        Err(ResolutionError::StrictViolation { matches: 3 })
    ));
    assert_eq!(
        client.method_names().last().unwrap(),
        "Runtime.releaseObjectGroup"
    );
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn held_element_exhausts_admission_before_a_second_query_is_dispatched() {
    // Arrange
    let (service, client, resources) = fixture();
    client.set_evaluate_response(node());
    let element = service
        .call(request(client.clone()))
        .await
        .unwrap()
        .unwrap();
    // Act
    let result = service.clone().call(request(client.clone())).await;
    // Assert
    assert!(matches!(result, Err(ResolutionError::Resource(_))));
    assert_eq!(client.calls().len(), 1);
    element.release().await.unwrap();
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn unpolled_attempt_releases_prepared_capacity_without_native_work() {
    // Arrange
    let (service, client, resources) = fixture();
    let future = service.call(request(client.clone()));
    assert_eq!(resources.metrics().unwrap().active_leases, 2);
    // Act
    drop(future);
    // Assert
    assert!(client.calls().is_empty());
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn malformed_remote_objects_and_exceptions_are_released_without_trusting_ids() {
    for response in [
        json!({"result":{"type":"object","subtype":"node","objectId":"x".repeat(NATIVE_ID_BYTES + 1)}}),
        json!({"result":{"type":"object","objectId":"wrong-kind"}}),
        json!({"exceptionDetails":{"exception":{"objectId":"exception"}},"result":{"type":"undefined"}}),
        json!({"result":{"type":"number","value":1}}),
    ] {
        // Arrange
        let (service, client, resources) = fixture();
        client.set_evaluate_response(response);
        // Act
        let result = service.call(request(client.clone())).await;
        // Assert
        assert!(matches!(result, Err(ResolutionError::Protocol(_))));
        assert_eq!(
            client.method_names(),
            ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
        );
        assert_eq!(resources.metrics().unwrap().active_leases, 0);
    }
}

#[tokio::test]
async fn failed_native_query_still_releases_the_preallocated_group() {
    // Arrange
    let (service, client, resources) = fixture();
    client.set_evaluate_errors(["Execution context was destroyed".into()]);
    // Act
    let result = service.call(request(client.clone())).await;
    // Assert
    assert!(matches!(result, Err(ResolutionError::Backend(_))));
    assert_eq!(
        client.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
    );
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn cleanup_failure_retains_the_primary_strictness_failure() {
    // Arrange
    let (service, client, resources) = fixture();
    client.set_evaluate_response(json!({"result":{"type":"number","value":2}}));
    client.set_release_object_error(Some("release failed".into()));
    // Act
    let result = service.call(request(client.clone())).await;
    // Assert
    assert!(
        matches!(result, Err(ResolutionError::Cleanup { primary: Some(primary), .. }) if matches!(*primary, ResolutionError::StrictViolation { matches: 2 }))
    );
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    assert!(!service.is_cleanup_healthy());
    let next = service.call(request(client.clone())).await;
    assert!(matches!(next, Err(ResolutionError::Admission(_))));
    assert_eq!(client.calls().len(), 2);
}

#[tokio::test]
async fn rejected_world_does_not_allocate_or_dispatch() {
    // Arrange
    let (service, client, resources) = fixture();
    let mut invalid = request(client.clone());
    invalid.utility_world = "".into();
    // Act
    let result = service.call(invalid).await;
    // Assert
    assert!(matches!(result, Err(ResolutionError::Setup(_))));
    assert!(client.calls().is_empty());
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn failed_owner_release_closes_admission_for_all_service_clones() {
    // Arrange
    let (service, client, resources) = fixture();
    let other = service.clone();
    client.set_evaluate_response(node());
    let owner = service
        .call(request(client.clone()))
        .await
        .unwrap()
        .unwrap();
    client.set_release_object_error(Some("release failed".into()));
    // Act
    let release = owner.release().await;
    let next = other.call(request(client.clone())).await;
    // Assert
    assert!(matches!(
        release,
        Err(ResolutionError::Cleanup { primary: None, .. })
    ));
    assert!(!other.is_cleanup_healthy());
    assert!(matches!(next, Err(ResolutionError::Admission(_))));
    assert_eq!(client.calls().len(), 2);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn installation_rejects_an_undeclared_plan_and_zero_capacity() {
    // Arrange
    let resources = ProductionResourceLedger::new(
        ResourceLimits::deny_all(0, 0),
        pwright_core::ResourceRequirements::none(),
    )
    .unwrap();
    let profile = ResolutionProfile::new(ResourceScope::extension(86, 1), 1).unwrap();
    // Act
    let install = CdpLocatorResolutionService::new(
        profile,
        &resources,
        Arc::new(AtomicObjectIdSource::new(1).unwrap()),
    );
    let empty = ResolutionProfile::new(ResourceScope::extension(86, 1), 0);
    // Assert
    assert!(matches!(install, Err(ResolutionError::Production(_))));
    assert!(matches!(empty, Err(ResolutionError::Setup(_))));
}

#[tokio::test]
async fn exhausted_identity_source_does_not_dispatch_or_retain_reservations() {
    // Arrange
    let (_, client, resources) = fixture();
    let profile = ResolutionProfile::new(ResourceScope::extension(86, 1), 1).unwrap();
    let service = CdpLocatorResolutionService::new(
        profile,
        &resources,
        Arc::new(AtomicObjectIdSource::new(u64::MAX).unwrap()),
    )
    .unwrap();
    // Act
    let result = service.call(request(client.clone())).await;
    // Assert
    assert!(matches!(
        result,
        Err(ResolutionError::Setup("object-group IDs exhausted"))
    ));
    assert!(client.calls().is_empty());
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn abandoning_a_raw_element_poisoned_the_installation_without_spawning_cleanup() {
    // Arrange
    let (service, client, resources) = fixture();
    client.set_evaluate_response(node());
    let element = service
        .call(request(client.clone()))
        .await
        .unwrap()
        .unwrap();
    // Act
    drop(element);
    let next = service.call(request(client.clone())).await;
    // Assert
    assert!(!service.is_cleanup_healthy());
    assert!(matches!(next, Err(ResolutionError::Admission(_))));
    assert_eq!(client.calls().len(), 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn selector_limit_rejects_before_compilation_or_cdp() {
    // Arrange
    let (service, client, resources) = fixture();
    let mut invalid = request(client.clone());
    invalid.selector = Selector::css("x".repeat(MAX_SELECTOR_STRING_BYTES + 1)).unwrap();
    // Act
    let result = service.call(invalid).await;
    // Assert
    assert!(matches!(result, Err(ResolutionError::Selector(_))));
    assert!(client.calls().is_empty());
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn completed_groups_are_never_reused_by_cloned_services() {
    // Arrange
    let (service, client, _) = fixture();
    service.call(request(client.clone())).await.unwrap();
    // Act
    service.clone().call(request(client.clone())).await.unwrap();
    // Assert
    let calls = client.calls();
    assert_eq!(calls[0].1[0]["objectGroup"], "pwright.locator.1");
    assert_eq!(calls[2].1[0]["objectGroup"], "pwright.locator.2");
}
