use super::*;
use pwright_core::{
    AtomicObjectIdSource, ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope,
};
use pwright_fake::FakeCdpClient;
use pwright_runtime::CleanupQueue;
use serde_json::json;
use std::task::Waker;

type FixtureService = ContinuableResolutionService<CdpLocatorResolutionService, CleanupQueue>;

fn fixture(queue: CleanupQueue) -> (FixtureService, Arc<FakeCdpClient>, ProductionResourceLedger) {
    let profile = ResolutionProfile::new(ResourceScope::extension(168, 1), 2)
        .unwrap()
        .with_retained_reference()
        .unwrap();
    let requirements = profile.requirements();
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let bound = requirements.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    let resources = ProductionResourceLedger::new(limits, requirements).unwrap();
    let raw = CdpLocatorResolutionService::new(
        profile,
        &resources,
        Arc::new(AtomicObjectIdSource::new(1).unwrap()),
    )
    .unwrap();
    let client = Arc::new(FakeCdpClient::from_html(""));
    client.set_object_backend_node("source", 17);
    client.set_adopted_node_response(
        17,
        23,
        json!({"object":{"type":"object","subtype":"node","objectId":"adopted"}}),
    );
    client.set_evaluate_response(json!({"result":{"type":"boolean","value":true}}));
    (
        ContinuableResolutionService::new(Arc::new(raw), queue),
        client,
        resources,
    )
}

fn request(client: Arc<FakeCdpClient>) -> RetainedElementRequest {
    RetainedElementRequest {
        source_client: client.clone(),
        source_object_id: "source".into(),
        client,
        utility_world: "utility-23".into(),
        utility_execution_context_id: 23,
        destruction: None,
    }
}

fn poll<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

#[tokio::test]
async fn missing_continuation_rejects_retained_adoption_before_io_and_admission() {
    // Arrange
    let (service, client, resources) = fixture(CleanupQueue::new(0));
    // Act
    let result = service.call(request(client.clone())).await;
    // Assert
    assert!(matches!(result, Err(ResolutionError::Admission(_))));
    assert!(client.calls().is_empty());
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn unpolled_adoption_releases_route_and_reservation_without_native_work() {
    // Arrange
    let queue = CleanupQueue::new(1);
    let (service, client, resources) = fixture(queue.clone());
    let route = Arc::new(17_u32);
    let weak = Arc::downgrade(&route);
    let future = service.call_retained_retaining(request(client.clone()), Some(route));
    assert_eq!(queue.metrics().unwrap().reserved, 1);
    // Act
    drop(future);
    // Assert
    assert!(weak.upgrade().is_none());
    assert!(client.calls().is_empty());
    assert_eq!(queue.metrics().unwrap().reserved, 0);
    assert_eq!(queue.metrics().unwrap().queued, 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn cancelled_acquisition_keeps_route_and_native_budget_through_group_ack() {
    // Arrange
    let queue = CleanupQueue::new(1);
    let (service, client, resources) = fixture(queue.clone());
    let route = Arc::new(17_u32);
    let weak = Arc::downgrade(&route);
    let acquisition_gate = Arc::new(tokio::sync::Notify::new());
    let release_gate = Arc::new(tokio::sync::Notify::new());
    client.set_adoption_gate(Some(acquisition_gate.clone()));
    client.set_release_object_group_gate(Some(release_gate.clone()));
    let mut future =
        Box::pin(service.call_retained_retaining(request(client.clone()), Some(route)));
    assert!(poll(future.as_mut()).is_pending());
    // Act
    drop(future);
    let mut cleanup = queue.try_take().unwrap().unwrap();
    acquisition_gate.notify_waiters();
    assert!(poll(cleanup.as_mut()).is_pending());
    // Assert: native group, source route and cleanup work outlive the caller.
    assert!(weak.upgrade().is_some());
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    assert_eq!(
        client.method_names().last().unwrap(),
        "Runtime.releaseObjectGroup"
    );
    release_gate.notify_waiters();
    cleanup.await.unwrap();
    assert!(weak.upgrade().is_none());
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    assert_eq!(queue.metrics().unwrap().completed, 1);
    assert_eq!(
        client.method_names(),
        [
            "DOM.describeNode",
            "DOM.resolveNode",
            "Runtime.callFunctionOn",
            "Runtime.releaseObjectGroup"
        ]
    );
}

#[tokio::test]
async fn completed_adoption_returns_real_owned_element_and_releases_its_group_on_drop() {
    // Arrange
    let queue = CleanupQueue::new(1);
    let (service, client, resources) = fixture(queue.clone());
    let owner = service.call(request(client.clone())).await.unwrap();
    let group = owner.element().object_group().to_owned();
    assert_eq!(owner.element().object_id(), "adopted");
    assert_eq!(owner.element().utility_world(), "utility-23");
    // Act
    drop(owner);
    queue.try_take().unwrap().unwrap().await.unwrap();
    // Assert
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    assert_eq!(queue.metrics().unwrap().completed, 1);
    assert_eq!(client.calls().last().unwrap().1, [json!(group)]);
    assert!(
        !client
            .method_names()
            .iter()
            .any(|name| name == "Runtime.releaseObject")
    );
}

#[tokio::test]
async fn abandoned_bad_response_preserves_failed_group_cleanup_for_the_executor() {
    // Arrange
    let queue = CleanupQueue::new(1);
    let (service, client, resources) = fixture(queue.clone());
    client.set_adopted_node_response(17, 23, json!({"object":{"type":"object"}}));
    client.set_release_object_error(Some("group refused".into()));
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_adoption_gate(Some(gate.clone()));
    let mut future = Box::pin(service.call(request(client.clone())));
    assert!(poll(future.as_mut()).is_pending());
    // Act
    drop(future);
    gate.notify_waiters();
    let result = queue.try_take().unwrap().unwrap().await;
    // Assert
    assert!(result.is_err());
    assert_eq!(queue.metrics().unwrap().failed, 1);
    assert!(!service.inner.is_cleanup_healthy());
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    assert_eq!(
        client.method_names(),
        [
            "DOM.describeNode",
            "DOM.resolveNode",
            "Runtime.releaseObjectGroup"
        ]
    );
}

#[tokio::test]
async fn cancelled_explicit_release_continues_the_same_native_group_request() {
    // Arrange
    let queue = CleanupQueue::new(1);
    let (service, client, resources) = fixture(queue.clone());
    let route = Arc::new(17_u32);
    let weak = Arc::downgrade(&route);
    let owner = service
        .call_retained_retaining(request(client.clone()), Some(route))
        .await
        .unwrap();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_release_object_group_gate(Some(gate.clone()));
    let mut release = Box::pin(owner.release());
    assert!(poll(release.as_mut()).is_pending());
    // Act
    drop(release);
    let cleanup = queue.try_take().unwrap().unwrap();
    // Assert
    assert!(weak.upgrade().is_some());
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    gate.notify_waiters();
    cleanup.await.unwrap();
    assert!(weak.upgrade().is_none());
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    assert_eq!(
        client
            .method_names()
            .iter()
            .filter(|name| *name == "Runtime.releaseObjectGroup")
            .count(),
        1
    );
}
