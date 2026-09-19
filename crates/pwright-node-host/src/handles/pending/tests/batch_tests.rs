use super::*;
use crate::handles::NamedPendingHandle;
use std::{
    future::Future,
    task::{Context, Waker},
};

fn bytes(resources: &ProductionResourceLedger) -> u64 {
    resources
        .metrics()
        .unwrap()
        .class(ResourceClass::BackendStateBytes)
        .current
}

fn siblings(
    owners: &Arc<Owners>,
    cleanup: &TokioCleanupExecutor,
    released: &[Arc<AtomicUsize>],
) -> Vec<PendingHandle> {
    let mut batch = owners.admit_batch().unwrap();
    assert!(released.len() <= batch.entries.len());
    batch.entries.truncate(released.len());
    let BatchAdmission {
        entries,
        call,
        work,
    } = batch;
    let handles = entries
        .into_iter()
        .zip(released)
        .enumerate()
        .map(|(index, (admission, released))| {
            let (initializer, ticket, owner) =
                ticket(index as u64 + 2, admission, cleanup, released.clone());
            owners.retain(initializer, ticket, owner).unwrap()
        })
        .collect();
    drop((call, work));
    handles
}

#[test]
fn all_batch_entries_share_one_call_but_have_distinct_owner_allowances() {
    // Arrange
    let (owners, resources) = owners();

    // Act
    let batch = owners.admit_batch().unwrap();

    // Assert
    assert_eq!(batch.entries.len(), MAX_HANDLES);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 1);
    assert_eq!(owners.capacity.owners.load(Ordering::Acquire), MAX_HANDLES);
    for entry in &batch.entries {
        assert!(Arc::ptr_eq(&entry.call, &batch.call));
    }
    for pair in batch.entries.windows(2) {
        assert!(!Arc::ptr_eq(&pair[0].owner, &pair[1].owner));
    }
    assert_eq!(
        bytes(&resources),
        STATE_BYTES + OWNER_BYTES * MAX_HANDLES as u64 + CALL_BYTES
    );
    assert!(
        owners
            .state
            .lock()
            .unwrap()
            .tickets
            .iter()
            .all(Option::is_none)
    );
    drop(batch);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 0);
    assert_eq!(owners.capacity.owners.load(Ordering::Acquire), 0);
    assert_eq!(bytes(&resources), STATE_BYTES);
}

#[test]
fn batch_reserves_only_available_owners_without_overflowing_existing_admission() {
    // Arrange
    let (owners, resources) = owners();
    let existing = [owners.admit().unwrap(), owners.admit().unwrap()];
    let baseline = bytes(&resources);

    // Act
    let batch = owners.admit_batch().unwrap();
    let overflow = owners.admit();

    // Assert
    assert_eq!(batch.entries.len(), MAX_HANDLES - existing.len());
    assert_eq!(owners.capacity.owners.load(Ordering::Acquire), MAX_HANDLES);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 3);
    assert!(matches!(overflow, Err(HostError::ResourceExhausted(_))));
    assert_eq!(
        bytes(&resources),
        STATE_BYTES + OWNER_BYTES * MAX_HANDLES as u64 + CALL_BYTES * 3
    );
    drop(batch);
    assert_eq!(bytes(&resources), baseline);
    drop(existing);
    assert_eq!(bytes(&resources), STATE_BYTES);
}

#[test]
fn batch_call_saturation_fails_before_reserving_any_owner_or_ticket() {
    // Arrange
    let (owners, resources) = owners();
    let calls = (0..MAX_CALLS)
        .map(|_| owners.admit_call().unwrap())
        .collect::<Vec<_>>();
    let baseline = resources.metrics().unwrap();

    // Act
    let overflow = owners.admit_batch();

    // Assert: this admission layer cannot submit any native cleanup work.
    assert!(matches!(overflow, Err(HostError::ResourceExhausted(_))));
    assert_eq!(resources.metrics().unwrap(), baseline);
    assert_eq!(owners.capacity.owners.load(Ordering::Acquire), 0);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), MAX_CALLS);
    assert!(
        owners
            .state
            .lock()
            .unwrap()
            .tickets
            .iter()
            .all(Option::is_none)
    );
    drop(calls);
    assert_eq!(bytes(&resources), STATE_BYTES);
}

#[test]
fn truncating_unused_batch_entries_returns_owner_capacity_but_retains_its_call() {
    // Arrange
    let (owners, resources) = owners();
    let mut batch = owners.admit_batch().unwrap();
    let call = batch.call.clone();

    // Act
    batch.entries.truncate(2);

    // Assert
    assert_eq!(owners.capacity.owners.load(Ordering::Acquire), 2);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 1);
    assert_eq!(
        bytes(&resources),
        STATE_BYTES + OWNER_BYTES * 2 + CALL_BYTES
    );
    let replacement = owners.admit_batch().unwrap();
    assert_eq!(replacement.entries.len(), MAX_HANDLES - 2);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 2);
    drop(replacement);
    drop(batch);
    // A name/result consumer can share the same allowance without a new lease.
    assert_eq!(owners.capacity.owners.load(Ordering::Acquire), 0);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 1);
    assert_eq!(bytes(&resources), STATE_BYTES + CALL_BYTES);
    drop(call);
    assert_eq!(bytes(&resources), STATE_BYTES);
}

#[test]
fn zero_result_batch_still_consumes_one_shared_call_slot() {
    // Arrange
    let (owners, resources) = owners();
    let mut batch = owners.admit_batch().unwrap();
    batch.entries.clear();
    let calls = (1..MAX_CALLS)
        .map(|_| owners.admit_call().unwrap())
        .collect::<Vec<_>>();

    // Act
    let overflow = owners.admit_batch();

    // Assert
    assert!(batch.entries.is_empty());
    assert!(matches!(overflow, Err(HostError::ResourceExhausted(_))));
    assert_eq!(owners.capacity.owners.load(Ordering::Acquire), 0);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), MAX_CALLS);
    assert_eq!(
        bytes(&resources),
        STATE_BYTES + CALL_BYTES * MAX_CALLS as u64
    );
    drop(batch);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), MAX_CALLS - 1);
    drop(calls);
    assert_eq!(bytes(&resources), STATE_BYTES);
}

#[test]
fn full_owner_capacity_can_admit_an_empty_batch_without_creating_another_owner() {
    // Arrange
    let (owners, resources) = owners();
    let full = owners.admit_batch().unwrap();

    // Act
    let empty = owners.admit_batch().unwrap();

    // Assert
    assert_eq!(full.entries.len(), MAX_HANDLES);
    assert!(empty.entries.is_empty());
    assert_eq!(owners.capacity.owners.load(Ordering::Acquire), MAX_HANDLES);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 2);
    assert_eq!(
        bytes(&resources),
        STATE_BYTES + OWNER_BYTES * MAX_HANDLES as u64 + CALL_BYTES * 2
    );
    drop((empty, full));
    assert_eq!(bytes(&resources), STATE_BYTES);
}

#[tokio::test]
async fn siblings_with_one_creation_call_dispose_by_independent_object_identity() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(2);
    let released = [Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0))];
    let pending = siblings(&owners, &cleanup, &released);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 0);
    assert_ne!(
        pending[0].initializer().object_id,
        pending[1].initializer().object_id
    );

    // Act
    pending[0].dispose().await.unwrap();

    // Assert
    assert_eq!(released[0].load(Ordering::Acquire), 1);
    assert_eq!(released[1].load(Ordering::Acquire), 0);
    assert!(
        matches!(owners.admit_read(&pending[0]), Err(HostError::InvalidArgument(message)) if message.contains("disposed"))
    );
    drop(owners.admit_read(&pending[1]).unwrap());
    assert_eq!(bytes(&resources), STATE_BYTES + OWNER_BYTES * 2);
    pending[0].dispose().await.unwrap();
    assert_eq!(released[0].load(Ordering::Acquire), 1);
    pending[1].dispose().await.unwrap();
    assert_eq!(released[1].load(Ordering::Acquire), 1);
    drop(pending);
    owners.close();
    owners.wait().await;
    let report = cleanup.shutdown().await;
    assert_eq!(report.metrics.completed, 2);
    assert_eq!(report.metrics.reserved, 0);
    assert_eq!(bytes(&resources), STATE_BYTES);
}

#[tokio::test]
async fn sibling_arguments_reject_foreign_owners_before_consuming_call_capacity() {
    // Arrange
    let (owner, resources) = owners();
    let (foreign, foreign_resources) = owners();
    let cleanup = TokioCleanupExecutor::new(4);
    let released = [Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0))];
    let pending = siblings(&owner, &cleanup, &released);
    let foreign_released = [Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0))];
    let foreign_pending = siblings(&foreign, &cleanup, &foreign_released);
    assert_eq!(
        pending[0].initializer().object_id,
        foreign_pending[0].initializer().object_id
    );
    let baseline = resources.metrics().unwrap();
    let foreign_baseline = foreign_resources.metrics().unwrap();

    // Act
    let rejected = foreign.check_arguments(pending.iter());
    let rejected_read = foreign.admit_read(&pending[1]);

    // Assert
    assert!(
        matches!(rejected, Err(HostError::InvalidArgument(message)) if message.contains("another session"))
    );
    assert!(
        matches!(rejected_read, Err(HostError::InvalidArgument(message)) if message.contains("another session"))
    );
    assert_eq!(resources.metrics().unwrap(), baseline);
    assert_eq!(foreign_resources.metrics().unwrap(), foreign_baseline);
    assert_eq!(foreign.capacity.calls.load(Ordering::Acquire), 0);
    owner.check_arguments(pending.iter()).unwrap();
    owner.close();
    foreign.close();
    owner.wait().await;
    foreign.wait().await;
    assert_eq!(cleanup.shutdown().await.metrics.completed, 4);
    drop(pending);
    drop(foreign_pending);
    assert_eq!(bytes(&resources), STATE_BYTES);
    assert_eq!(bytes(&foreign_resources), STATE_BYTES);
}

#[tokio::test]
async fn shutdown_releases_all_siblings_but_keeps_descriptor_storage_until_each_drop() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(MAX_HANDLES);
    let released = (0..MAX_HANDLES)
        .map(|_| Arc::new(AtomicUsize::new(0)))
        .collect::<Vec<_>>();
    let mut pending = siblings(&owners, &cleanup, &released);

    // Act
    owners.close();
    owners.wait().await;
    let report = cleanup.shutdown().await;

    // Assert
    assert_eq!(report.metrics.completed, MAX_HANDLES as u64);
    assert_eq!(report.metrics.reserved, 0);
    assert_eq!(report.metrics.failed, 0);
    assert!(
        released
            .iter()
            .all(|count| count.load(Ordering::Acquire) == 1)
    );
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 0);
    assert_eq!(owners.capacity.owners.load(Ordering::Acquire), MAX_HANDLES);
    assert_eq!(
        bytes(&resources),
        STATE_BYTES + OWNER_BYTES * MAX_HANDLES as u64
    );
    assert!(matches!(owners.admit_batch(), Err(HostError::TargetClosed)));
    for handle in &pending {
        handle.dispose().await.unwrap();
    }
    drop(pending.pop());
    assert_eq!(
        bytes(&resources),
        STATE_BYTES + OWNER_BYTES * (MAX_HANDLES - 1) as u64
    );
    drop(pending);
    assert_eq!(bytes(&resources), STATE_BYTES);
    drop(owners);
    assert_eq!(bytes(&resources), 0);
    assert!(
        released
            .iter()
            .all(|count| count.load(Ordering::Acquire) == 1)
    );
}

#[test]
fn closed_owner_rejects_batch_admission_without_changing_the_ledger() {
    // Arrange
    let (owners, resources) = owners();
    owners.close();
    let baseline = resources.metrics().unwrap();

    // Act
    let rejected = owners.admit_batch();

    // Assert
    assert!(matches!(rejected, Err(HostError::TargetClosed)));
    assert_eq!(resources.metrics().unwrap(), baseline);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 0);
    assert_eq!(owners.capacity.owners.load(Ordering::Acquire), 0);
}

#[tokio::test]
async fn empty_batch_work_token_keeps_shutdown_waiting_until_the_batch_is_dropped() {
    // Arrange
    let (owners, resources) = owners();
    let mut batch = owners.admit_batch().unwrap();
    batch.entries.clear();
    owners.close();
    let mut waiting = Box::pin(owners.wait());
    let mut context = Context::from_waker(Waker::noop());
    assert!(waiting.as_mut().poll(&mut context).is_pending());

    // Act
    drop(batch);
    waiting.await;

    // Assert
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 0);
    assert_eq!(owners.capacity.owners.load(Ordering::Acquire), 0);
    assert_eq!(bytes(&resources), STATE_BYTES);
}

#[tokio::test]
async fn named_siblings_retain_the_shared_call_until_the_last_name_is_consumed() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(2);
    let released = Arc::new(AtomicUsize::new(0));
    let mut batch = owners.admit_batch().unwrap();
    batch.entries.truncate(2);
    let BatchAdmission {
        entries,
        call,
        work,
    } = batch;
    let admitted = resources.metrics().unwrap();
    let mut named = entries
        .into_iter()
        .enumerate()
        .map(|(index, admission)| {
            let (initializer, ticket, owner) =
                ticket(index as u64 + 2, admission, &cleanup, released.clone());
            NamedPendingHandle {
                name: "name".repeat(4096),
                handle: owners.retain(initializer, ticket, owner).unwrap(),
                _storage: call.clone(),
            }
        })
        .collect::<Vec<_>>();
    drop((call, work));
    assert_eq!(resources.metrics().unwrap(), admitted);
    let other_calls = (1..MAX_CALLS)
        .map(|_| owners.admit_call().unwrap())
        .collect::<Vec<_>>();

    // Act: consume one name while its sibling still waits for N-API conversion.
    let first = named.remove(0).into_handle();

    // Assert
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), MAX_CALLS);
    assert!(matches!(
        owners.admit_call(),
        Err(HostError::ResourceExhausted(_))
    ));
    assert_eq!(named[0].name().len(), 16 * 1024);
    assert_eq!(
        bytes(&resources),
        STATE_BYTES + OWNER_BYTES * 2 + CALL_BYTES * MAX_CALLS as u64
    );
    let last = named.pop().unwrap().into_handle();
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), MAX_CALLS - 1);
    drop(other_calls);
    first.dispose().await.unwrap();
    last.dispose().await.unwrap();
    drop((first, last, named));
    owners.close();
    owners.wait().await;
    assert_eq!(cleanup.shutdown().await.metrics.completed, 2);
    assert_eq!(released.load(Ordering::Acquire), 2);
    assert_eq!(bytes(&resources), STATE_BYTES);
}
