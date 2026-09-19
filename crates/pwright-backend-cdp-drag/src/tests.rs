use super::*;
use pwright_core::ResourceLimits;
use pwright_fake::FakeCdpClient;
use std::task::Waker;

fn setup(
    capacity: usize,
) -> (
    DragListenerFactory,
    ProductionResourceLedger,
    Arc<FakeCdpClient>,
) {
    let required = DragListenerFactory::requirements(capacity).unwrap();
    let bytes = required.class(ResourceClass::BackendStateBytes);
    let resources = ProductionResourceLedger::new(
        ResourceLimits::deny_all(required.scopes, required.leases).with_class(
            ResourceClass::BackendStateBytes,
            bytes.global,
            bytes.per_scope,
        ),
        required,
    )
    .unwrap();
    let factory = DragListenerFactory::new(
        "test_page",
        ResourceScope::extension(14, 60),
        capacity,
        &resources,
    )
    .unwrap();
    let client = Arc::new(FakeCdpClient::from_html(""));
    client.set_evaluate_response(json!({"result": {"value": true}}));
    (factory, resources, client)
}
fn pending<F: Future + Unpin>(future: &mut F) {
    assert!(
        Pin::new(future)
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
}

#[tokio::test]
async fn unpolled_retirement_sends_nothing_and_retains_admission_until_owner_drops() {
    // Arrange
    let (factory, resources, client) = setup(1);
    let mut listener = factory.start(client.clone(), "world-1").unwrap();
    // Act
    listener.retire().await.unwrap();
    // Assert
    assert!(listener.is_retired());
    assert!(client.calls().is_empty());
    assert_eq!(factory.active_listeners(), 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 2);
    drop(listener);
    assert_eq!(factory.active_listeners(), 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    drop(factory);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn prepare_and_observe_use_same_exact_context_and_token_then_retire() {
    // Arrange
    let (factory, _, client) = setup(1);
    let mut listener = factory.start(client.clone(), "unique-world").unwrap();
    client.set_evaluate_responses([
        json!({"result":{"value":true}}),
        json!({"result":{"value":false}}),
    ]);
    // Act
    listener.prepare().await.unwrap();
    let started = listener.observe().await.unwrap();
    // Assert
    assert!(!started);
    assert!(listener.is_retired());
    let calls = client.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].1[0]["uniqueContextId"], "unique-world");
    assert_eq!(calls[1].1[0]["uniqueContextId"], "unique-world");
    assert_eq!(calls[0].1[0]["arguments"][0], calls[1].1[0]["arguments"][0]);
    assert_eq!(calls[1].1[0]["arguments"][1]["value"], true);
    assert_eq!(calls[0].1[0]["awaitPromise"], true);
    listener.retire().await.unwrap();
    assert_eq!(client.calls().len(), 2);
}

#[tokio::test]
async fn cancellation_during_setup_drains_it_then_removes_without_observing() {
    // Arrange
    let (factory, _, client) = setup(1);
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_evaluate_gate(Some(gate.clone()));
    let mut listener = factory.start(client.clone(), "world-1").unwrap();
    let mut preparing = Box::pin(listener.prepare());
    pending(&mut preparing);
    drop(preparing);
    // Act
    let mut retirement = Box::pin(listener.retire());
    pending(&mut retirement);
    drop(retirement);
    // Assert
    assert!(listener.prepare().await.is_err());
    assert!(listener.observe().await.is_err());
    assert!(!listener.is_retired());
    assert_eq!(client.calls().len(), 1);
    client.set_evaluate_gate(None);
    gate.notify_one();
    listener.retire().await.unwrap();
    assert!(listener.is_retired());
    let calls = client.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[1].1[0]["arguments"][1]["value"], false);
}

#[tokio::test]
async fn failed_setup_still_requires_cleanup_of_possibly_installed_listener() {
    // Arrange
    let (factory, _, client) = setup(1);
    client.set_evaluate_errors(["reply lost after script ran".into()]);
    let mut listener = factory.start(client.clone(), "world-1").unwrap();
    // Act
    let result = listener.prepare().await.unwrap_err();
    // Assert
    assert!(result.to_string().contains("reply lost"));
    assert!(!listener.is_retired());
    listener.retire().await.unwrap();
    assert!(listener.is_retired());
    assert_eq!(client.calls().len(), 2);
}

#[tokio::test]
async fn cleanup_failure_retains_dirty_owner_and_requires_explicit_retry() {
    // Arrange
    let (factory, resources, client) = setup(1);
    let mut listener = factory.start(client.clone(), "world-1").unwrap();
    listener.prepare().await.unwrap();
    client.set_evaluate_errors(["cleanup reply failed".into()]);
    // Act
    let error = listener.retire().await.unwrap_err();
    // Assert
    assert!(error.to_string().contains("cleanup reply failed"));
    assert!(!listener.is_retired());
    assert_eq!(factory.active_listeners(), 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 2);
    assert_eq!(client.calls().len(), 2);
    listener.retire().await.unwrap();
    assert_eq!(client.calls().len(), 3);
    assert!(listener.is_retired());
}

#[tokio::test]
async fn abandoned_observation_drains_the_started_cleanup_instead_of_resubmitting() {
    // Arrange
    let (factory, _, client) = setup(1);
    let mut listener = factory.start(client.clone(), "world-1").unwrap();
    listener.prepare().await.unwrap();
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_evaluate_gate(Some(gate.clone()));
    let mut observation = Box::pin(listener.observe());
    pending(&mut observation);
    drop(observation);
    // Act
    let mut retirement = Box::pin(listener.retire());
    pending(&mut retirement);
    gate.notify_one();
    retirement.await.unwrap();
    // Assert
    assert!(listener.is_retired());
    assert_eq!(client.calls().len(), 2);
}

#[test]
fn admission_rejects_capacity_and_invalid_context_without_native_effects() {
    // Arrange
    let (factory, resources, client) = setup(1);
    let owner = factory.start(client.clone(), "world-1").unwrap();
    // Act
    let full = factory.start(client.clone(), "world-2").err().unwrap();
    let invalid = factory
        .start(client.clone(), &"x".repeat(129))
        .err()
        .unwrap();
    // Assert
    assert!(full.to_string().contains("capacity exceeded"));
    assert!(invalid.to_string().contains("unique context"));
    assert_eq!(factory.active_listeners(), 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 2);
    assert!(client.calls().is_empty());
    drop(owner);
}

#[tokio::test]
async fn tokens_are_not_reused_when_slots_are_reclaimed() {
    // Arrange
    let (factory, _, client) = setup(1);
    let mut first = factory.start(client.clone(), "world-1").unwrap();
    first.prepare().await.unwrap();
    first.retire().await.unwrap();
    drop(first);
    // Act
    let mut second = factory.start(client.clone(), "world-1").unwrap();
    second.prepare().await.unwrap();
    second.retire().await.unwrap();
    // Assert
    let calls = client.calls();
    assert_ne!(calls[0].1[0]["arguments"][0], calls[2].1[0]["arguments"][0]);
}

#[test]
fn factory_remains_admitted_while_an_escaped_listener_owner_exists() {
    // Arrange
    let (factory, resources, client) = setup(1);
    let owner = factory.start(client, "world-1").unwrap();
    // Act
    drop(factory);
    // Assert
    assert_eq!(resources.metrics().unwrap().active_leases, 2);
    drop(owner);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn destruction_ack_requires_exact_unique_world_and_drained_native_call() {
    // Arrange
    let (factory, _, client) = setup(1);
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_evaluate_gate(Some(gate.clone()));
    let mut listener = factory.start(client.clone(), "world-1").unwrap();
    let mut preparation = Box::pin(listener.prepare());
    pending(&mut preparation);
    drop(preparation);
    // Act
    let early = listener.acknowledge_destroyed_world("world-1");
    // Assert
    assert!(early.is_err());
    client.set_evaluate_errors(["world destroyed".into()]);
    gate.notify_one();
    assert!(listener.prepare().await.is_err());
    assert!(listener.acknowledge_destroyed_world("world-2").is_err());
    listener.acknowledge_destroyed_world("world-1").unwrap();
    assert!(listener.is_retired());
}

#[tokio::test]
async fn malformed_cleanup_response_is_not_retirement_proof() {
    // Arrange
    let (factory, _, client) = setup(1);
    let mut listener = factory.start(client.clone(), "world-1").unwrap();
    listener.prepare().await.unwrap();
    client.set_evaluate_response(json!({"result":{"value":null}}));
    // Act
    let result = listener.retire().await;
    // Assert
    assert!(result.is_err());
    assert!(!listener.is_retired());
    assert_eq!(factory.active_listeners(), 1);
}

#[test]
fn invalid_profiles_and_exhausted_identifiers_fail_before_allocation_or_io() {
    // Arrange
    let (factory, _, client) = setup(1);
    factory.0.state.lock().unwrap().next = u64::MAX;
    // Act
    let exhausted = factory.start(client.clone(), "world-1").err().unwrap();
    // Assert
    assert!(DragListenerFactory::requirements(0).is_err());
    assert!(DragListenerFactory::requirements(33).is_err());
    assert!(exhausted.to_string().contains("identifiers exhausted"));
    assert_eq!(factory.active_listeners(), 0);
    assert!(client.calls().is_empty());
}

#[tokio::test]
async fn explicit_no_install_receipt_never_cleans_another_owners_token() {
    // Arrange
    let (factory, _, client) = setup(1);
    client.set_evaluate_response(json!({"result":{"value":false}}));
    let mut owner = factory.start(client.clone(), "world-1").unwrap();
    // Act
    let rejected = owner.prepare().await;
    // Assert
    assert!(rejected.is_err());
    assert!(owner.is_retired());
    owner.retire().await.unwrap();
    assert_eq!(client.calls().len(), 1);
}
