use super::*;

#[test]
fn unpolled_attempt_returns_its_caller_route_owner_without_browser_work() {
    // Arrange
    let (service, client, _) = fixture(CleanupQueue::new(1));
    let owner = Arc::new(());
    let weak = Arc::downgrade(&owner);
    let future = service.call_retaining(request(client.clone()), Some(owner));
    // Act
    drop(future);
    // Assert
    assert!(weak.upgrade().is_none());
    assert!(client.calls().is_empty());
}

#[tokio::test]
async fn cancelled_started_query_retains_route_through_its_exact_cleanup_continuation() {
    // Arrange
    let queue = CleanupQueue::new(1);
    let (service, client, _) = fixture(queue.clone());
    let owner = Arc::new(());
    let weak = Arc::downgrade(&owner);
    let query = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    client.set_evaluate_gate(Some(query.clone()));
    client.set_release_object_group_gate(Some(release.clone()));
    let mut future = Box::pin(service.call_retaining(request(client.clone()), Some(owner)));
    assert!(poll(future.as_mut()).is_pending());
    // Act
    drop(future);
    let mut job = queue.try_take().unwrap().unwrap();
    query.notify_waiters();
    assert!(poll(job.as_mut()).is_pending());
    // Assert
    assert!(
        weak.upgrade().is_some(),
        "route must outlive query and pending release"
    );
    release.notify_waiters();
    job.await.unwrap();
    assert!(weak.upgrade().is_none());
    assert_eq!(
        client.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
    );
}

#[tokio::test]
async fn returned_owner_and_cancelled_explicit_release_retain_the_route_until_acknowledged() {
    // Arrange
    let queue = CleanupQueue::new(1);
    let (service, client, _) = fixture(queue.clone());
    let owner = Arc::new(());
    let weak = Arc::downgrade(&owner);
    let element = service
        .call_retaining(request(client.clone()), Some(owner))
        .await
        .unwrap()
        .unwrap();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_release_object_group_gate(Some(gate.clone()));
    let mut release = Box::pin(element.release());
    assert!(poll(release.as_mut()).is_pending());
    // Act
    drop(release);
    // Assert
    assert!(weak.upgrade().is_some());
    gate.notify_waiters();
    queue.try_take().unwrap().unwrap().await.unwrap();
    assert!(weak.upgrade().is_none());
}

#[tokio::test]
async fn dropping_selected_owner_retains_route_until_queued_release_is_polled() {
    // Arrange
    let queue = CleanupQueue::new(1);
    let (service, client, _) = fixture(queue.clone());
    let owner = Arc::new(());
    let weak = Arc::downgrade(&owner);
    let element = service
        .call_retaining(request(client), Some(owner))
        .await
        .unwrap()
        .unwrap();
    // Act
    drop(element);
    // Assert
    assert!(weak.upgrade().is_some());
    queue.try_take().unwrap().unwrap().await.unwrap();
    assert!(weak.upgrade().is_none());
}

#[tokio::test]
async fn empty_result_reclaims_route_before_completed_future_is_dropped() {
    // Arrange
    let (service, client, _) = fixture(CleanupQueue::new(1));
    client.set_evaluate_response(json!({"result":{"type":"number","value":0}}));
    let owner = Arc::new(());
    let weak = Arc::downgrade(&owner);
    let mut future = Box::pin(service.call_retaining(request(client), Some(owner)));
    // Act
    let result = future.as_mut().await;
    // Assert
    assert!(result.unwrap().is_none());
    assert!(weak.upgrade().is_none());
}
