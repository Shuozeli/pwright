use super::*;
use pwright_core::{
    AtomicObjectIdSource, ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope,
};
use pwright_fake::FakeCdpClient;
use pwright_runtime::CleanupQueue;
use pwright_selector_model::Selector;
use serde_json::json;
use std::task::Waker;

mod route_retention;

type FixtureService = ContinuableResolutionService<CdpLocatorResolutionService, CleanupQueue>;
fn fixture(queue: CleanupQueue) -> (FixtureService, Arc<FakeCdpClient>, ProductionResourceLedger) {
    let profile = ResolutionProfile::new(ResourceScope::extension(87, 1), 1).unwrap();
    let required = profile.requirements();
    let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
    for class in ResourceClass::ALL {
        let bound = required.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    let resources = ProductionResourceLedger::new(limits, required).unwrap();
    let raw = CdpLocatorResolutionService::new(
        profile,
        &resources,
        Arc::new(AtomicObjectIdSource::new(1).unwrap()),
    )
    .unwrap();
    let client = Arc::new(FakeCdpClient::from_html(""));
    client.set_evaluate_response(
        json!({"result":{"type":"object","subtype":"node","objectId":"one"}}),
    );
    (
        ContinuableResolutionService::new(Arc::new(raw), queue),
        client,
        resources,
    )
}
fn request(client: Arc<FakeCdpClient>) -> ResolutionRequest {
    ResolutionRequest {
        client,
        utility_world: "utility".into(),
        selector: Selector::css("button").unwrap(),
        destruction: None,
    }
}
fn poll<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

#[tokio::test]
async fn unavailable_cleanup_capacity_rejects_before_native_admission() {
    // Arrange
    let (service, client, resources) = fixture(CleanupQueue::new(0));
    // Act
    let result = service.call(request(client.clone())).await;
    // Assert
    assert!(matches!(
        result,
        Err(ResolutionError::Admission(
            "cleanup continuation not admitted"
        ))
    ));
    assert!(client.calls().is_empty());
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn dropping_unpolled_resolution_returns_continuation_without_starting_work() {
    // Arrange
    let queue = CleanupQueue::new(1);
    let (service, client, resources) = fixture(queue.clone());
    let future = service.call(request(client.clone()));
    assert_eq!(queue.metrics().unwrap().reserved, 1);
    // Act
    drop(future);
    // Assert
    assert!(client.calls().is_empty());
    assert_eq!(queue.metrics().unwrap().reserved, 0);
    assert_eq!(queue.metrics().unwrap().queued, 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn dropping_started_resolution_finishes_once_then_releases_the_group() {
    // Arrange
    let queue = CleanupQueue::new(1);
    let (service, client, resources) = fixture(queue.clone());
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_evaluate_gate(Some(gate.clone()));
    let mut future = Box::pin(service.call(request(client.clone())));
    assert!(poll(future.as_mut()).is_pending());
    // Act
    drop(future);
    gate.notify_waiters();
    queue.try_take().unwrap().unwrap().await.unwrap();
    // Assert
    assert_eq!(
        client.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
    );
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    assert_eq!(queue.metrics().unwrap().completed, 1);
}

#[tokio::test]
async fn dropping_a_resolved_owner_uses_its_original_cleanup_reservation() {
    // Arrange
    let queue = CleanupQueue::new(1);
    let (service, client, resources) = fixture(queue.clone());
    let element = service
        .call(request(client.clone()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    // Act
    drop(element);
    queue.try_take().unwrap().unwrap().await.unwrap();
    // Assert
    assert_eq!(
        client.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
    );
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    assert_eq!(queue.metrics().unwrap().reserved, 0);
}

#[tokio::test]
async fn unpolled_explicit_release_transfers_cleanup_instead_of_losing_ownership() {
    // Arrange
    let queue = CleanupQueue::new(1);
    let (service, client, resources) = fixture(queue.clone());
    let element = service
        .call(request(client.clone()))
        .await
        .unwrap()
        .unwrap();
    // Act
    drop(element.release());
    queue.try_take().unwrap().unwrap().await.unwrap();
    // Assert
    assert_eq!(
        client.method_names().last().unwrap(),
        "Runtime.releaseObjectGroup"
    );
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn cleanup_failures_after_cancelled_query_remain_visible_to_the_executor() {
    // Arrange
    let queue = CleanupQueue::new(1);
    let (service, client, resources) = fixture(queue.clone());
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_evaluate_gate(Some(gate.clone()));
    client.set_evaluate_errors(["query failed".into()]);
    client.set_release_object_error(Some("cleanup failed".into()));
    let mut future = Box::pin(service.call(request(client.clone())));
    assert!(poll(future.as_mut()).is_pending());
    // Act
    drop(future);
    gate.notify_waiters();
    let result = queue.try_take().unwrap().unwrap().await;
    // Assert
    assert!(result.is_err());
    assert_eq!(queue.metrics().unwrap().failed, 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn awaited_release_returns_its_slot_without_enqueuing_extra_work() {
    // Arrange
    let queue = CleanupQueue::new(1);
    let (service, client, resources) = fixture(queue.clone());
    let element = service
        .call(request(client.clone()))
        .await
        .unwrap()
        .unwrap();
    // Act
    element.release().await.unwrap();
    // Assert
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    assert_eq!(queue.metrics().unwrap().reserved, 0);
    assert_eq!(queue.metrics().unwrap().queued, 0);
    assert_eq!(
        client.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
    );
}

#[tokio::test]
async fn dropping_started_release_continues_the_same_request_without_reissuing_it() {
    // Arrange
    let queue = CleanupQueue::new(1);
    let (service, client, resources) = fixture(queue.clone());
    let element = service
        .call(request(client.clone()))
        .await
        .unwrap()
        .unwrap();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_release_object_group_gate(Some(gate.clone()));
    let mut release = Box::pin(element.release());
    assert!(poll(release.as_mut()).is_pending());
    // Act
    drop(release);
    gate.notify_waiters();
    queue.try_take().unwrap().unwrap().await.unwrap();
    // Assert
    assert_eq!(
        client.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
    );
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    assert_eq!(queue.metrics().unwrap().completed, 1);
}

#[tokio::test]
async fn abandoned_cleanup_job_poisoned_the_raw_service_instead_of_reopening_capacity() {
    // Arrange
    let queue = CleanupQueue::new(1);
    let (service, client, resources) = fixture(queue.clone());
    let element = service
        .call(request(client.clone()))
        .await
        .unwrap()
        .unwrap();
    drop(element);
    let job = queue.try_take().unwrap().unwrap();
    // Act
    drop(job);
    let next = service.call(request(client.clone())).await;
    // Assert
    assert!(matches!(next, Err(ResolutionError::Admission(_))));
    assert_eq!(client.calls().len(), 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn discarded_pending_query_continuation_also_closes_admission() {
    // Arrange
    let queue = CleanupQueue::new(1);
    let (service, client, resources) = fixture(queue.clone());
    client.set_evaluate_gate(Some(Arc::new(tokio::sync::Notify::new())));
    let mut future = Box::pin(service.call(request(client.clone())));
    assert!(poll(future.as_mut()).is_pending());
    drop(future);
    let job = queue.try_take().unwrap().unwrap();
    // Act
    drop(job);
    let next = service.call(request(client.clone())).await;
    // Assert
    assert!(matches!(next, Err(ResolutionError::Admission(_))));
    assert_eq!(client.calls().len(), 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}
