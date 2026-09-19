//! Helper-only shutdown/admission proofs with real tokens and task tracking.
//! No assembled HostSession, transport, native owner or browser is exercised.
use super::*;
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex, mpsc},
    task::{Context, Poll, Wake, Waker},
    thread,
    time::Duration,
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

const CHANNEL_DEADLINE: Duration = Duration::from_secs(3);

#[derive(Debug, PartialEq, Eq)]
struct WakeObservation {
    state_unlocked: bool,
    stopped: bool,
    tracker_closed: bool,
}

struct LockObservingWake {
    state: Arc<Mutex<()>>,
    stop: CancellationToken,
    operations: TaskTracker,
    observations: Arc<Mutex<Vec<WakeObservation>>>,
}

impl LockObservingWake {
    fn observe(&self) {
        // try_lock avoids a test deadlock if production wakes under the lock.
        let state_unlocked = self.state.try_lock().is_ok();
        self.observations.lock().unwrap().push(WakeObservation {
            state_unlocked,
            stopped: self.stop.is_cancelled(),
            tracker_closed: self.operations.is_closed(),
        });
    }
}

impl Wake for LockObservingWake {
    fn wake(self: Arc<Self>) {
        self.observe();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.observe();
    }
}

fn poll_with<F: Future + ?Sized>(future: Pin<&mut F>, waker: &Waker) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(waker))
}

#[test]
fn stop_waker_runs_without_state_lock_before_tracker_close() {
    // Arrange
    let state = Arc::new(Mutex::new(()));
    let stop = CancellationToken::new();
    let operations = TaskTracker::new();
    let observations = Arc::new(Mutex::new(Vec::new()));
    let waker = Waker::from(Arc::new(LockObservingWake {
        state: state.clone(),
        stop: stop.clone(),
        operations: operations.clone(),
        observations: observations.clone(),
    }));
    let mut cancelled = std::pin::pin!(stop.cancelled());
    assert!(poll_with(cancelled.as_mut(), &waker).is_pending());

    // Act
    let returned = cancel_then_seal(&state, &stop, &operations, |()| {
        assert!(stop.is_cancelled());
        assert!(!operations.is_closed());
        assert!(
            state.try_lock().is_err(),
            "pure take executes under the admission barrier"
        );
        17
    })
    .unwrap();

    // Assert
    assert_eq!(returned, 17);
    assert_eq!(
        *observations.lock().unwrap(),
        [WakeObservation {
            state_unlocked: true,
            stopped: true,
            tracker_closed: false,
        }]
    );
    assert!(operations.is_closed());
    assert!(poll_with(cancelled.as_mut(), Waker::noop()).is_ready());
}

#[test]
fn empty_tracker_close_waker_runs_without_state_lock() {
    // Arrange
    let state = Arc::new(Mutex::new(()));
    let stop = CancellationToken::new();
    let operations = TaskTracker::new();
    let observations = Arc::new(Mutex::new(Vec::new()));
    let waker = Waker::from(Arc::new(LockObservingWake {
        state: state.clone(),
        stop: stop.clone(),
        operations: operations.clone(),
        observations: observations.clone(),
    }));
    let mut waiting = std::pin::pin!(operations.wait());
    assert!(poll_with(waiting.as_mut(), &waker).is_pending());

    // Act
    cancel_then_seal(&state, &stop, &operations, |()| ()).unwrap();

    // Assert
    assert_eq!(
        *observations.lock().unwrap(),
        [WakeObservation {
            state_unlocked: true,
            stopped: true,
            tracker_closed: true,
        }]
    );
    assert!(poll_with(waiting.as_mut(), Waker::noop()).is_ready());
}

struct SignalWake(mpsc::Sender<()>);

impl Wake for SignalWake {
    fn wake(self: Arc<Self>) {
        let _ = self.0.send(());
    }

    fn wake_by_ref(self: &Arc<Self>) {
        let _ = self.0.send(());
    }
}

#[test]
fn admission_that_read_stop_under_state_publishes_before_shutdown_can_seal() {
    // Arrange
    let state = Arc::new(Mutex::new(()));
    let stop = CancellationToken::new();
    let operations = TaskTracker::new();
    let (read_tx, read_rx) = mpsc::channel();
    let (publish_tx, publish_rx) = mpsc::channel();
    let (token_tx, token_rx) = mpsc::channel();
    let admission_state = state.clone();
    let admission_stop = stop.clone();
    let admission_operations = operations.clone();
    let admission = thread::spawn(move || {
        let guard = admission_state.lock().unwrap();
        assert!(!admission_stop.is_cancelled());
        read_tx.send(()).unwrap();
        publish_rx.recv_timeout(CHANNEL_DEADLINE).unwrap();
        // The real admission protocol publishes its work token while it still
        // owns the same lock used as shutdown's post-cancellation barrier.
        token_tx.send(admission_operations.token()).unwrap();
        drop(guard);
    });
    read_rx.recv_timeout(CHANNEL_DEADLINE).unwrap();
    let (cancel_tx, cancel_rx) = mpsc::channel();
    let waker = Waker::from(Arc::new(SignalWake(cancel_tx)));
    let mut cancelled = std::pin::pin!(stop.cancelled());
    assert!(poll_with(cancelled.as_mut(), &waker).is_pending());
    let shutdown_state = state.clone();
    let shutdown_stop = stop.clone();
    let shutdown_operations = operations.clone();
    let (returned_tx, returned_rx) = mpsc::channel();

    // Act: cancellation occurs while the admitted reader still owns state.
    let shutdown = thread::spawn(move || {
        let value = cancel_then_seal(
            &shutdown_state,
            &shutdown_stop,
            &shutdown_operations,
            |()| 29,
        );
        returned_tx.send(value).unwrap();
    });
    cancel_rx.recv_timeout(CHANNEL_DEADLINE).unwrap();
    let returned_before_publication = returned_rx.try_recv();
    let closed_before_publication = operations.is_closed();
    publish_tx.send(()).unwrap();
    let token = token_rx.recv_timeout(CHANNEL_DEADLINE).unwrap();
    let returned = returned_rx.recv_timeout(CHANNEL_DEADLINE).unwrap();
    admission.join().unwrap();
    shutdown.join().unwrap();

    // Assert: close cannot report an empty tracker between the read and token.
    assert!(matches!(
        returned_before_publication,
        Err(mpsc::TryRecvError::Empty)
    ));
    assert!(!closed_before_publication);
    assert_eq!(returned.unwrap(), 29);
    assert!(operations.is_closed());
    assert_eq!(operations.len(), 1);
    let mut waiting = std::pin::pin!(operations.wait());
    assert!(poll_with(waiting.as_mut(), Waker::noop()).is_pending());
    drop(token);
    assert!(poll_with(waiting.as_mut(), Waker::noop()).is_ready());
    assert!(operations.is_empty());
}

#[test]
fn later_admission_observes_cancellation_under_the_state_barrier() {
    // Arrange
    let state = Mutex::new(());
    let stop = CancellationToken::new();
    let operations = TaskTracker::new();
    cancel_then_seal(&state, &stop, &operations, |()| ()).unwrap();

    // Act: mirror only the admission stop-check/token critical section.
    let later = {
        let _guard = state.lock().unwrap();
        if stop.is_cancelled() {
            None
        } else {
            Some(operations.token())
        }
    };

    // Assert
    assert!(later.is_none());
    assert!(operations.is_closed());
    assert!(operations.is_empty());
    let mut waiting = std::pin::pin!(operations.wait());
    assert!(poll_with(waiting.as_mut(), Waker::noop()).is_ready());
}
