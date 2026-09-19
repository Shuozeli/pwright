use super::*;
use crate::ffi_budget::{FfiLimits, FfiMetrics};
use std::{
    future::Future,
    panic::{AssertUnwindSafe, catch_unwind},
    pin::Pin,
    sync::{Mutex, TryLockError},
    task::{Context, Poll, Wake, Waker},
};

const BASE_BYTES: u64 = 128;

fn budget() -> FfiBudget {
    FfiBudget::new(FfiLimits {
        connects: 1,
        commands: 2,
        closes: 1,
        watches: 1,
        reserved_bytes: 8192,
        future_bytes: [4096; 4],
    })
    .unwrap()
}

fn poll_now<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

fn lock_available<T>(lock: &Mutex<T>) -> bool {
    match lock.try_lock() {
        Ok(_) | Err(TryLockError::Poisoned(_)) => true,
        Err(TryLockError::WouldBlock) => false,
    }
}

fn running_metrics(future_bytes: usize) -> FfiMetrics {
    FfiMetrics {
        pending: 1,
        commands: 1,
        running: 1,
        reserved_bytes: BASE_BYTES + u64::try_from(future_bytes).unwrap(),
        ..FfiMetrics::default()
    }
}

struct ObservedFuture {
    budget: FfiBudget,
    drops: Arc<Mutex<Vec<FfiMetrics>>>,
    panic_on_poll: bool,
}

impl Future for ObservedFuture {
    type Output = Result<(), HostError>;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        assert!(!self.panic_on_poll, "injected native future panic");
        Poll::Pending
    }
}

impl Drop for ObservedFuture {
    fn drop(&mut self) {
        self.drops
            .lock()
            .unwrap()
            .push(self.budget.metrics().unwrap());
    }
}

#[test]
fn unpolled_run_drops_actual_future_before_its_permit_and_work_token() {
    // Arrange
    let budget = budget();
    let drops = Arc::new(Mutex::new(Vec::new()));
    let mut execution = budget
        .prepare::<()>(FfiLane::Command, BASE_BYTES)
        .unwrap()
        .start(&budget)
        .unwrap();
    let future = ObservedFuture {
        budget: budget.clone(),
        drops: drops.clone(),
        panic_on_poll: false,
    };
    let future_bytes = std::mem::size_of_val(&future);
    execution.reserve_future(future_bytes).unwrap();
    let run = execution.run(future);

    // Act
    drop(run);

    // Assert
    assert_eq!(*drops.lock().unwrap(), [running_metrics(future_bytes)]);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn partially_polled_run_drops_actual_future_before_its_permit_and_work_token() {
    // Arrange
    let budget = budget();
    let drops = Arc::new(Mutex::new(Vec::new()));
    let mut execution = budget
        .prepare::<()>(FfiLane::Command, BASE_BYTES)
        .unwrap()
        .start(&budget)
        .unwrap();
    let future = ObservedFuture {
        budget: budget.clone(),
        drops: drops.clone(),
        panic_on_poll: false,
    };
    let future_bytes = std::mem::size_of_val(&future);
    execution.reserve_future(future_bytes).unwrap();
    let mut run = Box::pin(execution.run(future));
    assert!(poll_now(run.as_mut()).is_pending());

    // Act
    drop(run);

    // Assert
    assert_eq!(*drops.lock().unwrap(), [running_metrics(future_bytes)]);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn panicking_native_poll_drops_future_before_releasing_its_admission() {
    // Arrange
    let budget = budget();
    let drops = Arc::new(Mutex::new(Vec::new()));
    let mut execution = budget
        .prepare::<()>(FfiLane::Command, BASE_BYTES)
        .unwrap()
        .start(&budget)
        .unwrap();
    let future = ObservedFuture {
        budget: budget.clone(),
        drops: drops.clone(),
        panic_on_poll: true,
    };
    let future_bytes = std::mem::size_of_val(&future);
    execution.reserve_future(future_bytes).unwrap();
    let mut run = Box::pin(execution.run(future));

    // Act
    let result = catch_unwind(AssertUnwindSafe(|| poll_now(run.as_mut())));
    drop(run);

    // Assert
    assert!(result.is_err());
    assert_eq!(*drops.lock().unwrap(), [running_metrics(future_bytes)]);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[derive(Debug)]
struct LockObservation {
    registry_available: bool,
    entry_available: bool,
    metrics: FfiMetrics,
}

struct ReentrantWake {
    budget: FfiBudget,
    entry: Weak<Entry<()>>,
    observations: Mutex<Vec<LockObservation>>,
}

impl Wake for ReentrantWake {
    fn wake(self: Arc<Self>) {
        let registry_available = lock_available(&self.budget.0.registry);
        let entry_available = self
            .entry
            .upgrade()
            .is_some_and(|entry| lock_available(&entry.state));
        self.observations.lock().unwrap().push(LockObservation {
            registry_available,
            entry_available,
            metrics: self.budget.metrics().unwrap(),
        });
    }
}

#[test]
fn running_revoke_wakes_outside_locks_and_keeps_storage_until_future_drop() {
    // Arrange
    let budget = budget();
    let drops = Arc::new(Mutex::new(Vec::new()));
    let preparation = budget.prepare::<()>(FfiLane::Command, BASE_BYTES).unwrap();
    let revoker = preparation.revoker();
    let wake = Arc::new(ReentrantWake {
        budget: budget.clone(),
        entry: Arc::downgrade(&preparation.ticket.entry),
        observations: Mutex::new(Vec::new()),
    });
    let mut execution = preparation.start(&budget).unwrap();
    let future = ObservedFuture {
        budget: budget.clone(),
        drops: drops.clone(),
        panic_on_poll: false,
    };
    let future_bytes = std::mem::size_of_val(&future);
    execution.reserve_future(future_bytes).unwrap();
    let mut run = Box::pin(execution.run(future));
    let waker = Waker::from(wake.clone());
    assert!(
        run.as_mut()
            .poll(&mut Context::from_waker(&waker))
            .is_pending()
    );

    // Act
    let revoked = revoker.revoke();

    // Assert: cancellation wakes but cannot destroy storage on the revoker's
    // stack; the future still owns the body until its next poll or Drop.
    assert!(revoked);
    assert!(drops.lock().unwrap().is_empty());
    assert_eq!(budget.metrics().unwrap(), running_metrics(future_bytes));
    let observations = wake.observations.lock().unwrap();
    assert!(!observations.is_empty());
    for observation in observations.iter() {
        assert!(observation.registry_available);
        assert!(observation.entry_available);
        assert_eq!(observation.metrics, running_metrics(future_bytes));
    }
    drop(observations);
    let Poll::Ready(ticket) = poll_now(run.as_mut()) else {
        panic!("revoked execution must settle on the next poll");
    };
    assert_eq!(*drops.lock().unwrap(), [running_metrics(future_bytes)]);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    drop(ticket);
}

#[derive(Debug)]
struct PayloadObservation {
    locks: LockObservation,
    reentered_admission: bool,
}

struct Payload {
    budget: FfiBudget,
    entry: Weak<Entry<Payload>>,
    drops: Arc<Mutex<Vec<PayloadObservation>>>,
    panic_on_drop: bool,
}

impl Drop for Payload {
    fn drop(&mut self) {
        let registry_available = lock_available(&self.budget.0.registry);
        let entry_available = self
            .entry
            .upgrade()
            .is_some_and(|entry| lock_available(&entry.state));
        let metrics = self.budget.metrics().unwrap();
        // Try locks first so a regression is reported instead of deadlocking
        // this deterministic test. On the correct path, actually reenter.
        let reentered_admission = registry_available
            && entry_available
            && self
                .budget
                .prepare::<()>(FfiLane::Command, 1)
                .map(drop)
                .is_ok();
        self.drops.lock().unwrap().push(PayloadObservation {
            locks: LockObservation {
                registry_available,
                entry_available,
                metrics,
            },
            reentered_admission,
        });
        assert!(!self.panic_on_drop, "injected payload destructor panic");
    }
}

#[tokio::test]
async fn queued_revoke_drops_payload_outside_locks_while_all_bytes_are_charged() {
    // Arrange
    let budget = budget();
    let drops = Arc::new(Mutex::new(Vec::new()));
    let preparation = budget
        .prepare::<Payload>(FfiLane::Command, BASE_BYTES)
        .unwrap();
    let revoker = preparation.revoker();
    let payload = Payload {
        budget: budget.clone(),
        entry: Arc::downgrade(&preparation.ticket.entry),
        drops: drops.clone(),
        panic_on_drop: false,
    };
    let mut execution = preparation.start(&budget).unwrap();
    let future = async { Ok(payload) };
    let future_bytes = std::mem::size_of_val(&future);
    execution.reserve_future(future_bytes).unwrap();
    let ticket = execution.run(future).await;

    // Act
    let revoked = revoker.revoke();

    // Assert
    assert!(revoked);
    let observations = drops.lock().unwrap();
    assert_eq!(observations.len(), 1);
    let observation = &observations[0];
    assert!(observation.locks.registry_available);
    assert!(observation.locks.entry_available);
    assert!(observation.reentered_admission);
    assert_eq!(
        observation.locks.metrics,
        FfiMetrics {
            running: 0,
            queued: 1,
            queued_commands: 1,
            ..running_metrics(future_bytes)
        }
    );
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    drop(ticket);
}

#[tokio::test]
async fn panicking_parked_inspection_does_not_poison_global_shutdown() {
    // Arrange
    let budget = budget();
    let drops = Arc::new(Mutex::new(Vec::new()));
    let preparation = budget
        .prepare::<Payload>(FfiLane::Command, BASE_BYTES)
        .unwrap();
    let payload = Payload {
        budget: budget.clone(),
        entry: Arc::downgrade(&preparation.ticket.entry),
        drops: drops.clone(),
        panic_on_drop: false,
    };
    let parked = preparation
        .start(&budget)
        .unwrap()
        .run(async { Ok(payload) })
        .await
        .park(&budget)
        .unwrap();

    // Act
    let inspected = catch_unwind(AssertUnwindSafe(|| {
        parked.inspect_parked(&budget, |_| panic!("injected pure inspection panic"))
    }));
    let mut shutdown = Box::pin(budget.shutdown());
    let shutdown_result = poll_now(shutdown.as_mut());

    // Assert
    assert!(inspected.is_err());
    assert!(matches!(shutdown_result, Poll::Ready(Ok(()))));
    let observations = drops.lock().unwrap();
    assert_eq!(observations.len(), 1);
    assert!(observations[0].locks.registry_available);
    assert!(observations[0].locks.entry_available);
    assert!(!observations[0].reentered_admission);
    assert_eq!(
        observations[0].locks.metrics,
        FfiMetrics {
            running: 0,
            ..running_metrics(0)
        }
    );
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    drop(parked);
}

#[tokio::test]
async fn panicking_queued_payload_drop_still_unwinds_its_permit() {
    // Arrange
    let budget = budget();
    let drops = Arc::new(Mutex::new(Vec::new()));
    let preparation = budget
        .prepare::<Payload>(FfiLane::Command, BASE_BYTES)
        .unwrap();
    let revoker = preparation.revoker();
    let payload = Payload {
        budget: budget.clone(),
        entry: Arc::downgrade(&preparation.ticket.entry),
        drops: drops.clone(),
        panic_on_drop: true,
    };
    let ticket = preparation
        .start(&budget)
        .unwrap()
        .run(async { Ok(payload) })
        .await;

    // Act
    let revoked = catch_unwind(AssertUnwindSafe(|| revoker.revoke()));

    // Assert
    assert!(revoked.is_err());
    let observations = drops.lock().unwrap();
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].locks.metrics.reserved_bytes, BASE_BYTES);
    assert!(observations[0].reentered_admission);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    drop(ticket);
}

struct RevokeOnReady {
    revoker: FfiRevoker,
    payload: Option<Payload>,
}

impl Future for RevokeOnReady {
    type Output = Result<Payload, HostError>;

    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        self.revoker.revoke();
        Poll::Ready(Ok(self.payload.take().unwrap()))
    }
}

#[tokio::test]
async fn success_returned_after_revoke_is_destroyed_before_its_running_permit() {
    // Arrange
    let budget = budget();
    let drops = Arc::new(Mutex::new(Vec::new()));
    let preparation = budget
        .prepare::<Payload>(FfiLane::Command, BASE_BYTES)
        .unwrap();
    let future = RevokeOnReady {
        revoker: preparation.revoker(),
        payload: Some(Payload {
            budget: budget.clone(),
            entry: Arc::downgrade(&preparation.ticket.entry),
            drops: drops.clone(),
            panic_on_drop: false,
        }),
    };
    let future_bytes = std::mem::size_of_val(&future);
    let mut execution = preparation.start(&budget).unwrap();
    execution.reserve_future(future_bytes).unwrap();

    // Act
    let ticket = execution.run(future).await;

    // Assert
    let observations = drops.lock().unwrap();
    assert_eq!(observations.len(), 1);
    assert!(observations[0].locks.registry_available);
    assert!(observations[0].locks.entry_available);
    assert!(observations[0].reentered_admission);
    assert_eq!(observations[0].locks.metrics, running_metrics(future_bytes));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    assert!(matches!(ticket.park(&budget), Err(HostError::TargetClosed)));
}

fn seal_before_cancellation(budget: &FfiBudget) {
    budget.0.registry.lock().unwrap().closed = true;
    assert!(!budget.0.stopped.is_cancelled());
}

#[test]
fn sealed_registry_rejects_preparation_before_cancellation_is_broadcast() {
    // Arrange
    let budget = budget();
    seal_before_cancellation(&budget);

    // Act
    let result = budget.prepare::<()>(FfiLane::Command, BASE_BYTES);

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn sealed_registry_rejects_start_without_registering_late_work() {
    // Arrange
    let budget = budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, BASE_BYTES).unwrap();
    seal_before_cancellation(&budget);

    // Act
    let result = preparation.start(&budget);

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[tokio::test]
async fn sealed_registry_rejects_resume_without_extracting_a_parked_owner() {
    // Arrange
    let budget = budget();
    let drops = Arc::new(Mutex::new(Vec::new()));
    let preparation = budget
        .prepare::<Payload>(FfiLane::Command, BASE_BYTES)
        .unwrap();
    let payload = Payload {
        budget: budget.clone(),
        entry: Arc::downgrade(&preparation.ticket.entry),
        drops: drops.clone(),
        panic_on_drop: false,
    };
    let parked = preparation
        .start(&budget)
        .unwrap()
        .run(async { Ok(payload) })
        .await
        .park(&budget)
        .unwrap();
    seal_before_cancellation(&budget);

    // Act
    let result = parked.resume(&budget);

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    let observations = drops.lock().unwrap();
    assert_eq!(observations.len(), 1);
    assert!(observations[0].locks.registry_available);
    assert!(observations[0].locks.entry_available);
    assert!(!observations[0].reentered_admission);
    assert_eq!(observations[0].locks.metrics.running, 0);
    assert_eq!(observations[0].locks.metrics.reserved_bytes, BASE_BYTES);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[tokio::test]
async fn sealed_registry_rejects_inspection_without_invoking_its_callback() {
    // Arrange
    let budget = budget();
    let parked = budget
        .prepare::<u8>(FfiLane::Command, BASE_BYTES)
        .unwrap()
        .start(&budget)
        .unwrap()
        .run(async { Ok(7) })
        .await
        .park(&budget)
        .unwrap();
    seal_before_cancellation(&budget);

    // Act
    let result = parked.inspect_parked(&budget, |_| panic!("sealed callback must not run"));

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(budget.metrics().unwrap().pending, 1);
    assert_eq!(budget.metrics().unwrap().running, 0);
    drop(parked);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[tokio::test]
async fn sealed_registry_rejects_parking_and_reclaims_the_queued_result() {
    // Arrange
    let budget = budget();
    let mut execution = budget
        .prepare::<u8>(FfiLane::Command, BASE_BYTES)
        .unwrap()
        .start(&budget)
        .unwrap();
    execution.reserve_future(8).unwrap();
    let ticket = execution.run(async { Ok(7) }).await;
    seal_before_cancellation(&budget);

    // Act
    let result = ticket.park(&budget);

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}
