use super::*;
use std::{
    pin::Pin,
    sync::TryLockError,
    task::{Context, Poll, Wake, Waker},
};

const BASE_BYTES: u64 = 16;

fn budget() -> FfiBudget {
    FfiBudget::new(FfiLimits {
        connects: 1,
        commands: 2,
        closes: 1,
        watches: 1,
        reserved_bytes: 256,
        future_bytes: [128; 4],
    })
    .unwrap()
}

fn poll_now<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

#[derive(Debug)]
struct ShutdownWakeObservation {
    registry_available: bool,
    admission_rejected_closed: Option<bool>,
}

struct ShutdownWake {
    budget: FfiBudget,
    observations: Mutex<Vec<ShutdownWakeObservation>>,
}

impl Wake for ShutdownWake {
    fn wake(self: Arc<Self>) {
        let registry_available = match self.budget.0.registry.try_lock() {
            Ok(_) | Err(TryLockError::Poisoned(_)) => true,
            Err(TryLockError::WouldBlock) => false,
        };
        // A broken implementation is recorded without hanging the test. On
        // the unlocked path this exercises real admission from a wakeup.
        let admission_rejected_closed = registry_available.then(|| {
            matches!(
                self.budget.admit::<()>(FfiLane::Command, 1),
                Err(HostError::TargetClosed)
            )
        });
        self.observations
            .lock()
            .unwrap()
            .push(ShutdownWakeObservation {
                registry_available,
                admission_rejected_closed,
            });
    }
}

#[tokio::test]
async fn shutdown_wakes_outside_registry_lock_with_admission_already_closed() {
    // Arrange
    let budget = budget();
    let execution = budget.admit::<()>(FfiLane::Command, BASE_BYTES).unwrap();
    let wake = Arc::new(ShutdownWake {
        budget: budget.clone(),
        observations: Mutex::new(Vec::new()),
    });
    let waker = Waker::from(wake.clone());
    let mut run = Box::pin(execution.run(std::future::pending()));
    assert!(
        run.as_mut()
            .poll(&mut Context::from_waker(&waker))
            .is_pending()
    );
    let mut shutdown = Box::pin(budget.shutdown());

    // Act
    let before_settlement = poll_now(shutdown.as_mut());

    // Assert: finish owned work before checking observations, so the red case
    // also leaves no live execution or outstanding shutdown future.
    assert!(before_settlement.is_pending());
    let Poll::Ready(ticket) = poll_now(run.as_mut()) else {
        panic!("shutdown cancellation must settle on the next poll");
    };
    assert!(matches!(poll_now(shutdown.as_mut()), Poll::Ready(Ok(()))));
    drop(ticket);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    let observations = wake.observations.lock().unwrap();
    assert!(!observations.is_empty());
    for observation in observations.iter() {
        assert!(observation.registry_available);
        assert_eq!(observation.admission_rejected_closed, Some(true));
    }
}

#[test]
fn sealed_registry_rejects_eager_admission_before_cancellation_is_broadcast() {
    // Arrange: model shutdown's interval after sealing and before wakeups.
    let budget = budget();
    budget.0.registry.lock().unwrap().closed = true;
    assert!(!budget.0.stopped.is_cancelled());

    // Act
    let result = budget.admit::<()>(FfiLane::Command, BASE_BYTES);

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

struct ObservedFuture {
    budget: FfiBudget,
    drops: Arc<Mutex<Vec<FfiMetrics>>>,
}

impl Future for ObservedFuture {
    type Output = Result<(), HostError>;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
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

fn running_metrics(future_bytes: usize) -> FfiMetrics {
    FfiMetrics {
        pending: 1,
        commands: 1,
        running: 1,
        reserved_bytes: BASE_BYTES + u64::try_from(future_bytes).unwrap(),
        ..FfiMetrics::default()
    }
}

#[test]
fn unpolled_eager_run_drops_future_before_releasing_bytes_and_work_token() {
    // Arrange
    let budget = budget();
    let drops = Arc::new(Mutex::new(Vec::new()));
    let future = ObservedFuture {
        budget: budget.clone(),
        drops: drops.clone(),
    };
    let future_bytes = std::mem::size_of_val(&future);
    let mut execution = budget.admit::<()>(FfiLane::Command, BASE_BYTES).unwrap();
    execution.reserve_future(future_bytes).unwrap();
    let run = execution.run(future);

    // Act
    drop(run);

    // Assert
    assert_eq!(*drops.lock().unwrap(), [running_metrics(future_bytes)]);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[tokio::test]
async fn shutdown_drops_eager_native_future_while_bytes_and_work_remain_charged() {
    // Arrange
    let budget = budget();
    let drops = Arc::new(Mutex::new(Vec::new()));
    let future = ObservedFuture {
        budget: budget.clone(),
        drops: drops.clone(),
    };
    let future_bytes = std::mem::size_of_val(&future);
    let mut execution = budget.admit::<()>(FfiLane::Command, BASE_BYTES).unwrap();
    execution.reserve_future(future_bytes).unwrap();
    let mut run = Box::pin(execution.run(future));
    assert!(poll_now(run.as_mut()).is_pending());
    let mut shutdown = Box::pin(budget.shutdown());

    // Act
    let before_settlement = poll_now(shutdown.as_mut());

    // Assert
    assert!(before_settlement.is_pending());
    assert!(drops.lock().unwrap().is_empty());
    let Poll::Ready(ticket) = poll_now(run.as_mut()) else {
        panic!("shutdown cancellation must settle on the next poll");
    };
    assert_eq!(*drops.lock().unwrap(), [running_metrics(future_bytes)]);
    assert!(matches!(poll_now(shutdown.as_mut()), Poll::Ready(Ok(()))));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    drop(ticket);
}
