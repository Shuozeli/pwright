use super::*;
use std::{
    future::Future,
    task::{Context, Waker},
};

#[tokio::test]
async fn delayed_result_consumer_retains_call_capacity_without_an_additional_lease() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let released = Arc::new(AtomicUsize::new(0));
    let admission = owners.admit().unwrap();
    let admitted = resources.metrics().unwrap();
    let caller = admission.call.clone();
    let native = admission.call.clone();
    assert_eq!(resources.metrics().unwrap(), admitted);
    let (reply, received) = tokio::sync::oneshot::channel::<PendingHandle>();
    let mut consumer = Box::pin(async move {
        let _caller = caller;
        received.await.unwrap()
    });
    let mut context = Context::from_waker(Waker::noop());
    assert!(consumer.as_mut().poll(&mut context).is_pending());
    let (initializer, ticket, owner) = ticket(2, admission, &cleanup, released.clone());
    let pending = owners.retain(initializer, ticket, owner).unwrap();
    let remaining = (1..MAX_CALLS)
        .map(|_| owners.admit_call().unwrap())
        .collect::<Vec<_>>();

    // Act: publication settles, but its consumer is deliberately not repolled.
    assert!(reply.send(pending).is_ok());
    drop(native);
    let excess = owners.admit_call();

    // Assert
    assert!(matches!(excess, Err(HostError::ResourceExhausted(_))));
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), MAX_CALLS);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        STATE_BYTES + OWNER_BYTES + CALL_BYTES * MAX_CALLS as u64
    );
    let pending = consumer.await;
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), MAX_CALLS - 1);
    drop(remaining);
    pending.dispose().await.unwrap();
    drop(pending);
    assert_eq!(cleanup.shutdown().await.metrics.completed, 1);
    assert_eq!(released.load(Ordering::Acquire), 1);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        STATE_BYTES
    );
}

#[tokio::test]
async fn cancelled_consumer_keeps_the_started_continuation_admitted() {
    // Arrange
    let (owners, resources) = owners();
    let admission = owners.admit().unwrap();
    let native = admission.call.clone();
    let caller = admission.call.clone();
    let (resume, blocked) = tokio::sync::oneshot::channel::<()>();
    let mut consumer = Box::pin(async move {
        let _caller = caller;
        blocked.await.unwrap();
    });
    let mut context = Context::from_waker(Waker::noop());
    assert!(consumer.as_mut().poll(&mut context).is_pending());

    // Act: cancellation drops the caller, while pre-admitted native work survives.
    drop(consumer);
    drop(admission);

    // Assert
    assert!(resume.is_closed());
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 1);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        STATE_BYTES + CALL_BYTES
    );
    drop(native);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 0);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        STATE_BYTES
    );
}

#[tokio::test]
async fn lost_result_receiver_releases_the_shared_call_and_cleans_the_result_once() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let released = Arc::new(AtomicUsize::new(0));
    let admission = owners.admit().unwrap();
    let native = admission.call.clone();
    let caller = admission.call.clone();
    let (reply, received) = tokio::sync::oneshot::channel::<PendingHandle>();
    let consumer = async move {
        let _caller = caller;
        received.await.unwrap()
    };
    drop(consumer);
    let (initializer, ticket, owner) = ticket(2, admission, &cleanup, released.clone());
    let pending = owners.retain(initializer, ticket, owner).unwrap();

    // Act
    let undelivered = reply.send(pending).err().unwrap();
    drop(undelivered);
    drop(native);
    let report = cleanup.shutdown().await;

    // Assert
    assert_eq!(released.load(Ordering::Acquire), 1);
    assert_eq!(report.metrics.completed, 1);
    assert_eq!(report.metrics.failed, 0);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 0);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        STATE_BYTES
    );
}
