use super::*;
use crate::ffi_budget::{FfiLimits, FfiMetrics};
use std::{
    future::Future,
    pin::Pin,
    sync::atomic::AtomicUsize,
    task::{Context, Poll, Waker},
};

fn budget() -> FfiBudget {
    FfiBudget::new(FfiLimits {
        connects: 1,
        commands: 2,
        closes: 1,
        watches: 1,
        reserved_bytes: 64,
        future_bytes: [32; 4],
    })
    .unwrap()
}

fn poll_now<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

struct Owned {
    value: u8,
    drops: Arc<AtomicUsize>,
}
impl Drop for Owned {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn park_resume_reuses_exact_entry_and_permit_without_retyping_or_extra_admission() {
    // Arrange
    let budget = budget();
    let drops = Arc::new(AtomicUsize::new(0));
    let input = Owned {
        value: 7,
        drops: drops.clone(),
    };
    let preparation = budget.prepare::<Owned>(FfiLane::Command, 16).unwrap();
    let entry = Arc::downgrade(&preparation.ticket.entry);
    let mut execution = preparation.start(&budget).unwrap();
    execution.reserve_future(8).unwrap();
    let ticket = execution.run(async { Ok(input) }).await;
    assert_eq!(budget.metrics().unwrap().queued_commands, 1);
    assert_eq!(budget.metrics().unwrap().reserved_bytes, 24);

    // Act
    let parked = ticket.park(&budget).unwrap();

    // Assert: the result remains inside the same entry while no task runs.
    assert!(Weak::ptr_eq(&entry, &Arc::downgrade(&parked.ticket.entry)));
    assert_eq!(
        parked.inspect_parked(&budget, |value| value.value).unwrap(),
        7
    );
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert_eq!(
        budget.metrics().unwrap(),
        FfiMetrics {
            pending: 1,
            commands: 1,
            reserved_bytes: 16,
            ..FfiMetrics::default()
        }
    );
    let (mut value, mut execution) = parked.resume(&budget.clone()).unwrap();
    assert_eq!(budget.metrics().unwrap().running, 1);
    assert_eq!(budget.metrics().unwrap().pending, 1);
    execution.reserve_future(12).unwrap();
    value.value += 1;
    let ticket = execution.run(async { Ok(value) }).await;
    assert!(Weak::ptr_eq(&entry, &Arc::downgrade(&ticket.entry)));
    assert_eq!(budget.metrics().unwrap().reserved_bytes, 28);
    let observed = ticket.deliver(|result| result.unwrap().value).unwrap();
    assert_eq!(observed, 8);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[tokio::test]
async fn failed_phase_cannot_park_and_forwards_original_error() {
    // Arrange
    let budget = budget();
    let execution = budget
        .prepare::<()>(FfiLane::Command, 16)
        .unwrap()
        .start(&budget)
        .unwrap();
    let ticket = execution.run(async { Err(HostError::Timeout(37)) }).await;

    // Act
    let result = ticket.park(&budget);

    // Assert
    assert!(matches!(result, Err(HostError::Timeout(37))));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn an_unsettled_ticket_cannot_be_parked() {
    // Arrange: internal defensive boundary, not a public extraction API.
    let budget = budget();
    let FfiPreparation { ticket, .. } = budget.prepare::<()>(FfiLane::Command, 16).unwrap();

    // Act
    let result = ticket.park(&budget);

    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn initial_preparation_has_no_parked_result_and_cannot_resume() {
    // Arrange
    let budget = budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, 16).unwrap();
    assert!(matches!(
        preparation.inspect_parked(&budget, |_| ()),
        Err(HostError::InvalidArgument(_))
    ));

    // Act
    let result = preparation.resume(&budget);

    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[tokio::test]
async fn parked_preparation_cannot_restart_and_overwrite_its_owned_result() {
    // Arrange
    let budget = budget();
    let drops = Arc::new(AtomicUsize::new(0));
    let input = Owned {
        value: 0,
        drops: drops.clone(),
    };
    let execution = budget
        .prepare::<Owned>(FfiLane::Command, 16)
        .unwrap()
        .start(&budget)
        .unwrap();
    let preparation = execution
        .run(async { Ok(input) })
        .await
        .park(&budget)
        .unwrap();

    // Act
    let result = preparation.start(&budget);

    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[tokio::test]
async fn foreign_park_consumes_only_origin_ticket_and_drops_its_result() {
    // Arrange
    let origin = budget();
    let foreign = budget();
    let drops = Arc::new(AtomicUsize::new(0));
    let input = Owned {
        value: 0,
        drops: drops.clone(),
    };
    let execution = origin
        .prepare::<Owned>(FfiLane::Command, 16)
        .unwrap()
        .start(&origin)
        .unwrap();
    let ticket = execution.run(async { Ok(input) }).await;

    // Act
    let result = ticket.park(&foreign);

    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(origin.metrics().unwrap(), FfiMetrics::default());
    assert_eq!(foreign.metrics().unwrap(), FfiMetrics::default());
}

#[tokio::test]
async fn foreign_inspection_leaves_parked_result_but_foreign_resume_reclaims_origin() {
    // Arrange
    let origin = budget();
    let foreign = budget();
    let execution = origin
        .prepare::<u8>(FfiLane::Command, 16)
        .unwrap()
        .start(&origin)
        .unwrap();
    let parked = execution.run(async { Ok(7) }).await.park(&origin).unwrap();
    let before = origin.metrics().unwrap();
    assert!(matches!(
        parked.inspect_parked(&foreign, |_| panic!("foreign callback must not execute")),
        Err(HostError::InvalidArgument(_))
    ));
    assert_eq!(origin.metrics().unwrap(), before);

    // Act
    let result = parked.resume(&foreign);

    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert_eq!(origin.metrics().unwrap(), FfiMetrics::default());
    assert_eq!(foreign.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn weak_revoker_reclaims_initial_storage_without_owning_budget_or_entry() {
    fn assert_send_sync<T: Send + Sync>() {}
    // Arrange
    assert_send_sync::<FfiRevoker>();
    let budget = budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, 16).unwrap();
    let revoker = preparation.revoker();
    let weak_budget = Arc::downgrade(&budget.0);

    // Act
    let revoked = revoker.revoke();

    // Assert
    assert!(revoked);
    assert!(!revoker.revoke());
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    assert!(matches!(
        preparation.start(&budget),
        Err(HostError::TargetClosed)
    ));
    drop(budget);
    assert!(weak_budget.upgrade().is_none());
    assert!(revoker.0.upgrade().is_none());
}

#[tokio::test]
async fn weak_revoker_reclaims_queued_result_and_exact_queue_counts() {
    // Arrange
    let budget = budget();
    let drops = Arc::new(AtomicUsize::new(0));
    let preparation = budget.prepare::<Owned>(FfiLane::Command, 16).unwrap();
    let revoker = preparation.revoker();
    let execution = preparation.start(&budget).unwrap();
    let input = Owned {
        value: 0,
        drops: drops.clone(),
    };
    let ticket = execution.run(async { Ok(input) }).await;
    assert_eq!(budget.metrics().unwrap().queued_commands, 1);

    // Act
    let revoked = revoker.revoke();

    // Assert
    assert!(revoked);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    assert!(matches!(ticket.park(&budget), Err(HostError::TargetClosed)));
}

#[tokio::test]
async fn weak_revoker_reclaims_parked_owner_while_preparation_remains_held() {
    // Arrange
    let budget = budget();
    let drops = Arc::new(AtomicUsize::new(0));
    let preparation = budget.prepare::<Owned>(FfiLane::Command, 16).unwrap();
    let revoker = preparation.revoker();
    let input = Owned {
        value: 0,
        drops: drops.clone(),
    };
    let parked = preparation
        .start(&budget)
        .unwrap()
        .run(async { Ok(input) })
        .await
        .park(&budget)
        .unwrap();

    // Act
    let revoked = revoker.revoke();

    // Assert
    assert!(revoked);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    assert!(matches!(
        parked.inspect_parked(&budget, |_| ()),
        Err(HostError::TargetClosed)
    ));
    assert!(matches!(
        parked.resume(&budget),
        Err(HostError::TargetClosed)
    ));
}

#[tokio::test]
async fn shutdown_reclaims_parked_owner_without_waiting_for_foreign_resume() {
    // Arrange
    let budget = budget();
    let drops = Arc::new(AtomicUsize::new(0));
    let input = Owned {
        value: 0,
        drops: drops.clone(),
    };
    let parked = budget
        .prepare::<Owned>(FfiLane::Command, 16)
        .unwrap()
        .start(&budget)
        .unwrap()
        .run(async { Ok(input) })
        .await
        .park(&budget)
        .unwrap();
    let mut shutdown = Box::pin(budget.shutdown());

    // Act: a single deterministic poll, not a timer-based eventual assertion.
    let result = poll_now(shutdown.as_mut());

    // Assert
    assert!(matches!(result, Poll::Ready(Ok(()))));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    assert!(matches!(
        parked.resume(&budget),
        Err(HostError::TargetClosed)
    ));
}

#[tokio::test]
async fn resumed_phase_has_a_new_independent_future_body_size_check() {
    // Arrange
    let budget = budget();
    let mut execution = budget
        .prepare::<u8>(FfiLane::Command, 16)
        .unwrap()
        .start(&budget)
        .unwrap();
    execution.reserve_future(8).unwrap();
    let parked = execution.run(async { Ok(7) }).await.park(&budget).unwrap();
    let (value, mut execution) = parked.resume(&budget).unwrap();

    // Act
    let result = execution.reserve_future(33);

    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(value, 7);
    assert_eq!(budget.metrics().unwrap().reserved_bytes, 16);
    assert_eq!(budget.metrics().unwrap().running, 1);
    drop(execution);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}
