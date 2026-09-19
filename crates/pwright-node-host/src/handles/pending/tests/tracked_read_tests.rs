use super::*;
use crate::handles::creation::{Creation, map_leaf_error};
use pwright_cap_handle_evaluation::HandleEvaluationErrorDetails;
use pwright_core::{FailureKind, OperationError};
use std::{
    future::Future,
    task::{Context, Waker},
};

fn retained(
    owners: &Arc<Owners>,
    cleanup: &TokioCleanupExecutor,
    released: &Arc<AtomicUsize>,
) -> PendingHandle {
    let (initializer, ticket, owner) =
        ticket(2, owners.admit().unwrap(), cleanup, released.clone());
    owners.retain(initializer, ticket, owner).unwrap()
}

fn bytes(resources: &ProductionResourceLedger) -> u64 {
    resources
        .metrics()
        .unwrap()
        .class(ResourceClass::BackendStateBytes)
        .current
}

#[tokio::test]
async fn tracked_read_shares_the_eight_call_limit_without_admitting_another_handle() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let released = Arc::new(AtomicUsize::new(0));
    let pending = retained(&owners, &cleanup, &released);
    let ordinary = (1..MAX_CALLS)
        .map(|_| owners.admit_call().unwrap())
        .collect::<Vec<_>>();

    // Act
    let read = owners.admit_tracked_read(&pending).unwrap();
    let admitted = resources.metrics().unwrap();
    let caller = read.call.clone();
    let overflow_read = owners.admit_tracked_read(&pending);
    let overflow_ordinary = owners.admit_call();

    // Assert
    assert!(matches!(
        overflow_read,
        Err(HostError::ResourceExhausted(_))
    ));
    assert!(matches!(
        overflow_ordinary,
        Err(HostError::ResourceExhausted(_))
    ));
    assert_eq!(resources.metrics().unwrap(), admitted);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), MAX_CALLS);
    assert_eq!(owners.capacity.owners.load(Ordering::Acquire), 1);
    assert_eq!(admitted.class(ResourceClass::RemoteHandle).current, 2);
    assert_eq!(
        bytes(&resources),
        STATE_BYTES + OWNER_BYTES + CALL_BYTES * MAX_CALLS as u64
    );
    drop(read);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), MAX_CALLS);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::RemoteHandle)
            .current,
        0
    );
    drop(caller);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), MAX_CALLS - 1);
    drop(ordinary);
    pending.dispose().await.unwrap();
    drop(pending);
    owners.close();
    owners.wait().await;
    let report = cleanup.shutdown().await;
    assert_eq!(released.load(Ordering::Acquire), 1);
    assert_eq!(report.metrics.completed, 1);
    assert_eq!(report.metrics.failed, 0);
    assert_eq!(report.metrics.reserved, 0);
    assert_eq!(bytes(&resources), STATE_BYTES);
}

#[tokio::test]
async fn close_waits_for_tracked_work_after_explicit_cleanup_and_descriptor_drop() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let released = Arc::new(AtomicUsize::new(0));
    let pending = retained(&owners, &cleanup, &released);
    let ReadAdmission { call, work, native } = owners.admit_tracked_read(&pending).unwrap();
    pending.dispose().await.unwrap();
    drop(pending);
    drop(call);
    assert_eq!(released.load(Ordering::Acquire), 1);
    assert_eq!(bytes(&resources), STATE_BYTES);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::RemoteHandle)
            .current,
        2
    );

    // Act
    owners.close();
    let mut waiting = Box::pin(owners.wait());
    let mut context = Context::from_waker(Waker::noop());
    let while_tracked = waiting.as_mut().poll(&mut context);
    drop(native);
    drop(work);
    let after_completion = waiting.as_mut().poll(&mut context);

    // Assert
    assert!(while_tracked.is_pending());
    assert!(after_completion.is_ready());
    assert_eq!(owners.capacity.owners.load(Ordering::Acquire), 0);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 0);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::RemoteHandle)
            .current,
        0
    );
    let report = cleanup.shutdown().await;
    assert_eq!(report.metrics.completed, 1);
    assert_eq!(report.metrics.failed, 0);
    assert_eq!(report.metrics.reserved, 0);
}

#[tokio::test]
async fn eight_tracked_reads_admit_sixteen_temporary_objects_not_persistent_handles() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let released = Arc::new(AtomicUsize::new(0));
    let pending = retained(&owners, &cleanup, &released);

    // Act
    let reads = (0..MAX_CALLS)
        .map(|_| owners.admit_tracked_read(&pending).unwrap())
        .collect::<Vec<_>>();
    let admitted = resources.metrics().unwrap();
    let overflow = owners.admit_tracked_read(&pending);

    // Assert
    assert!(matches!(overflow, Err(HostError::ResourceExhausted(_))));
    assert_eq!(resources.metrics().unwrap(), admitted);
    assert_eq!(owners.capacity.owners.load(Ordering::Acquire), 1);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), MAX_CALLS);
    assert_eq!(
        admitted.class(ResourceClass::RemoteHandle).current,
        2 * MAX_CALLS as u64
    );
    drop(reads);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 0);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::RemoteHandle)
            .current,
        0
    );
    pending.dispose().await.unwrap();
    drop(pending);
    owners.close();
    owners.wait().await;
    assert_eq!(cleanup.shutdown().await.metrics.completed, 1);
    assert_eq!(released.load(Ordering::Acquire), 1);
    assert_eq!(bytes(&resources), STATE_BYTES);
}

#[tokio::test]
async fn foreign_tracked_read_rejects_equal_numeric_identity_without_admission() {
    // Arrange
    let (owner, resources) = owners();
    let (foreign, foreign_resources) = owners();
    let cleanup = TokioCleanupExecutor::new(2);
    let released = Arc::new(AtomicUsize::new(0));
    let local = retained(&owner, &cleanup, &released);
    let other = retained(&foreign, &cleanup, &released);
    assert_eq!(local.initializer().object_id, other.initializer().object_id);
    let baseline = resources.metrics().unwrap();
    let foreign_baseline = foreign_resources.metrics().unwrap();

    // Act
    let result = owner.admit_tracked_read(&other);

    // Assert
    assert!(
        matches!(result, Err(HostError::InvalidArgument(message)) if message == "Handle belongs to another session")
    );
    assert_eq!(resources.metrics().unwrap(), baseline);
    assert_eq!(foreign_resources.metrics().unwrap(), foreign_baseline);
    assert_eq!(owner.capacity.calls.load(Ordering::Acquire), 0);
    local.dispose().await.unwrap();
    other.dispose().await.unwrap();
    drop((local, other));
    owner.close();
    foreign.close();
    owner.wait().await;
    foreign.wait().await;
    assert_eq!(cleanup.shutdown().await.metrics.completed, 2);
    assert_eq!(released.load(Ordering::Acquire), 2);
    assert_eq!(bytes(&resources), STATE_BYTES);
    assert_eq!(bytes(&foreign_resources), STATE_BYTES);
}

#[tokio::test]
async fn disposed_tracked_read_rejects_while_descriptor_is_held_without_admission() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let released = Arc::new(AtomicUsize::new(0));
    let pending = retained(&owners, &cleanup, &released);
    pending.dispose().await.unwrap();
    let baseline = resources.metrics().unwrap();

    // Act
    let result = owners.admit_tracked_read(&pending);

    // Assert
    assert!(
        matches!(result, Err(HostError::InvalidArgument(message)) if message == "JSHandle is disposed")
    );
    assert_eq!(resources.metrics().unwrap(), baseline);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 0);
    assert_eq!(released.load(Ordering::Acquire), 1);
    drop(pending);
    owners.close();
    owners.wait().await;
    assert_eq!(cleanup.shutdown().await.metrics.completed, 1);
    assert_eq!(bytes(&resources), STATE_BYTES);
}

#[tokio::test]
async fn closed_owners_reject_tracked_read_without_reviving_work_or_admission() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let released = Arc::new(AtomicUsize::new(0));
    let pending = retained(&owners, &cleanup, &released);
    owners.close();
    owners.wait().await;
    let baseline = resources.metrics().unwrap();

    // Act
    let result = owners.admit_tracked_read(&pending);

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(resources.metrics().unwrap(), baseline);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 0);
    let mut waiting = Box::pin(owners.wait());
    assert!(
        waiting
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_ready()
    );
    assert_eq!(released.load(Ordering::Acquire), 1);
    pending.dispose().await.unwrap();
    drop(pending);
    assert_eq!(cleanup.shutdown().await.metrics.completed, 1);
    assert_eq!(bytes(&resources), STATE_BYTES);
}

#[tokio::test]
async fn unpolled_tracked_creation_drops_admission_without_starting_native_work() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(2);
    let released = Arc::new(AtomicUsize::new(0));
    let pending = retained(&owners, &cleanup, &released);
    let admission = owners.admit_tracked_read(&pending).unwrap();
    let started = Arc::new(AtomicUsize::new(0));
    let observed = started.clone();
    let creation = Creation::<Option<String>>::new(
        async move {
            let _admission = admission;
            observed.fetch_add(1, Ordering::AcqRel);
            Ok(None)
        },
        cleanup.try_reserve().unwrap(),
    );

    // Act
    drop(creation);

    // Assert
    assert_eq!(started.load(Ordering::Acquire), 0);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 0);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::RemoteHandle)
            .current,
        0
    );
    assert_eq!(bytes(&resources), STATE_BYTES + OWNER_BYTES);
    pending.dispose().await.unwrap();
    drop(pending);
    owners.close();
    owners.wait().await;
    let report = cleanup.shutdown().await;
    assert_eq!(released.load(Ordering::Acquire), 1);
    assert_eq!(report.metrics.completed, 1);
    assert_eq!(report.metrics.failed, 0);
    assert_eq!(report.metrics.reserved, 0);
    assert_eq!(bytes(&resources), STATE_BYTES);
}

#[tokio::test]
async fn cancelled_tracked_creation_retains_native_call_and_work_until_settlement() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(2);
    let released = Arc::new(AtomicUsize::new(0));
    let pending = retained(&owners, &cleanup, &released);
    let admission = owners.admit_tracked_read(&pending).unwrap();
    let started = Arc::new(AtomicUsize::new(0));
    let observed = started.clone();
    let (settle, settlement) = tokio::sync::oneshot::channel();
    let mut creation = Box::pin(Creation::<Option<String>>::new(
        async move {
            let _admission = admission;
            observed.fetch_add(1, Ordering::AcqRel);
            settlement.await.unwrap();
            Ok(Some("native-frame".into()))
        },
        cleanup.try_reserve().unwrap(),
    ));
    let mut context = Context::from_waker(Waker::noop());
    assert!(creation.as_mut().poll(&mut context).is_pending());

    // Act
    drop(creation);
    owners.close();
    pending.dispose().await.unwrap();
    drop(pending);
    let mut waiting = Box::pin(owners.wait());
    let before_settlement = waiting.as_mut().poll(&mut context);
    let retained = resources.metrics().unwrap();
    settle.send(()).unwrap();
    waiting.await;
    let report = cleanup.shutdown().await;

    // Assert
    assert!(before_settlement.is_pending());
    assert_eq!(retained.class(ResourceClass::RemoteHandle).current, 2);
    assert_eq!(
        retained.class(ResourceClass::BackendStateBytes).current,
        STATE_BYTES + CALL_BYTES
    );
    assert_eq!(started.load(Ordering::Acquire), 1);
    assert_eq!(released.load(Ordering::Acquire), 1);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 0);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::RemoteHandle)
            .current,
        0
    );
    assert_eq!(bytes(&resources), STATE_BYTES);
    assert_eq!(report.metrics.completed, 2);
    assert_eq!(report.metrics.failed, 0);
    assert_eq!(report.metrics.reserved, 0);
}

#[tokio::test]
async fn abandoned_tracked_creation_preserves_leaf_cleanup_failure_in_executor_health() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(2);
    let released = Arc::new(AtomicUsize::new(0));
    let pending = retained(&owners, &cleanup, &released);
    let admission = owners.admit_tracked_read(&pending).unwrap();
    let (settle, settlement) = tokio::sync::oneshot::channel();
    let mut creation = Box::pin(Creation::<Option<String>>::new(
        async move {
            let _admission = admission;
            settlement.await.unwrap();
            Err(map_leaf_error(OperationError::new(
                FailureKind::Cleanup,
                "temporary receiver cleanup failed",
                Some(HandleEvaluationErrorDetails::Cleanup {
                    primary: "ownerFrame lookup completed".into(),
                    cleanup: "native release refused".into(),
                }),
            )))
        },
        cleanup.try_reserve().unwrap(),
    ));
    assert!(
        creation
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );

    // Act
    drop(creation);
    owners.close();
    settle.send(()).unwrap();
    pending.dispose().await.unwrap();
    drop(pending);
    owners.wait().await;
    let report = cleanup.shutdown().await;

    // Assert
    assert_eq!(report.metrics.failed, 1);
    assert_eq!(report.metrics.timed_out, 0);
    assert_eq!(report.metrics.reserved, 0);
    assert_eq!(released.load(Ordering::Acquire), 1);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 0);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::RemoteHandle)
            .current,
        0
    );
    assert_eq!(bytes(&resources), STATE_BYTES);
}
