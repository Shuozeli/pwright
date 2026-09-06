use std::collections::{BTreeMap, VecDeque};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, OnceLock};
use std::task::{Context, Poll, Waker};

use pwright_core::{CallId, ObjectId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TargetSchedulerLimits {
    pub active_targets: usize,
    pub waiting_calls: usize,
}

impl TargetSchedulerLimits {
    pub const fn new(active_targets: usize, waiting_calls: usize) -> Self {
        Self {
            active_targets,
            waiting_calls,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TargetSchedulerMetrics {
    pub active_targets: usize,
    pub waiting_calls: usize,
    pub peak_active_targets: usize,
    pub peak_waiting_calls: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetSchedulerError {
    DuplicateCall { call_id: CallId },
    ActiveTargetCapacityExceeded { limit: usize },
    WaitingCallCapacityExceeded { limit: usize },
    MissingCall { call_id: CallId },
    InconsistentLane { call_id: CallId, target: ObjectId },
    StatePoisoned,
}

impl Display for TargetSchedulerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateCall { call_id } => {
                write!(formatter, "call `{call_id}` is already scheduled")
            }
            Self::ActiveTargetCapacityExceeded { limit } => {
                write!(formatter, "active target limit of {limit} was reached")
            }
            Self::WaitingCallCapacityExceeded { limit } => {
                write!(formatter, "waiting call limit of {limit} was reached")
            }
            Self::MissingCall { call_id } => write!(formatter, "call `{call_id}` is not scheduled"),
            Self::InconsistentLane { call_id, target } => write!(
                formatter,
                "call `{call_id}` is missing from its target `{target}` lane"
            ),
            Self::StatePoisoned => formatter.write_str("target scheduler lock was poisoned"),
        }
    }
}

impl Error for TargetSchedulerError {}

/// Bounded FIFO admission for commands keyed by target object.
///
/// The scheduler creates no tasks. A permit is released explicitly or on
/// `Drop`, which wakes exactly the next waiter for the same target.
#[derive(Clone)]
pub struct TargetScheduler {
    shared: Arc<Mutex<TargetSchedulerState>>,
}

impl TargetScheduler {
    pub fn new(limits: TargetSchedulerLimits) -> Self {
        Self {
            shared: Arc::new(Mutex::new(TargetSchedulerState::new(limits))),
        }
    }

    pub fn acquire(
        &self,
        target: ObjectId,
        call_id: CallId,
    ) -> Result<TargetPermitFuture, TargetSchedulerError> {
        let mut state = self
            .shared
            .lock()
            .map_err(|_| TargetSchedulerError::StatePoisoned)?;
        if state.calls.contains_key(&call_id) {
            return Err(TargetSchedulerError::DuplicateCall { call_id });
        }

        let signal = Arc::new(AdmissionSignal::new());
        if state.lanes.contains_key(&target) {
            let waiting_limit = state.limits.waiting_calls;
            if state.waiting_calls >= waiting_limit {
                return Err(TargetSchedulerError::WaitingCallCapacityExceeded {
                    limit: waiting_limit,
                });
            }
            let lane = state
                .lanes
                .get_mut(&target)
                .expect("target lane existence was checked");
            lane.waiting.push_back(WaitingCall {
                call_id,
                signal: Arc::clone(&signal),
            });
            state.waiting_calls += 1;
            state.peak_waiting_calls = state.peak_waiting_calls.max(state.waiting_calls);
        } else {
            if state.lanes.len() >= state.limits.active_targets {
                return Err(TargetSchedulerError::ActiveTargetCapacityExceeded {
                    limit: state.limits.active_targets,
                });
            }
            state.lanes.insert(
                target,
                TargetLane {
                    active: call_id,
                    waiting: VecDeque::new(),
                },
            );
            state.peak_active_targets = state.peak_active_targets.max(state.lanes.len());
            signal.grant();
        }
        state.calls.insert(call_id, target);

        Ok(TargetPermitFuture {
            scheduler: self.clone(),
            target,
            call_id,
            signal,
            completed: false,
        })
    }

    pub fn metrics(&self) -> Result<TargetSchedulerMetrics, TargetSchedulerError> {
        let state = self
            .shared
            .lock()
            .map_err(|_| TargetSchedulerError::StatePoisoned)?;
        Ok(TargetSchedulerMetrics {
            active_targets: state.lanes.len(),
            waiting_calls: state.waiting_calls,
            peak_active_targets: state.peak_active_targets,
            peak_waiting_calls: state.peak_waiting_calls,
        })
    }

    fn release_call(&self, call_id: CallId) -> Result<(), TargetSchedulerError> {
        let mut state = self
            .shared
            .lock()
            .map_err(|_| TargetSchedulerError::StatePoisoned)?;
        let target = state
            .calls
            .remove(&call_id)
            .ok_or(TargetSchedulerError::MissingCall { call_id })?;

        let (promoted, remove_lane, removed_waiter) = {
            let lane = state
                .lanes
                .get_mut(&target)
                .ok_or(TargetSchedulerError::InconsistentLane { call_id, target })?;
            if lane.active == call_id {
                match lane.waiting.pop_front() {
                    Some(next) => {
                        lane.active = next.call_id;
                        (Some(next.signal), false, true)
                    }
                    None => (None, true, false),
                }
            } else {
                let position = lane
                    .waiting
                    .iter()
                    .position(|waiting| waiting.call_id == call_id)
                    .ok_or(TargetSchedulerError::InconsistentLane { call_id, target })?;
                lane.waiting.remove(position);
                (None, false, true)
            }
        };

        if removed_waiter {
            state.waiting_calls -= 1;
        }
        if remove_lane {
            state.lanes.remove(&target);
        }
        drop(state);
        if let Some(signal) = promoted {
            signal.grant();
        }
        Ok(())
    }
}

struct TargetSchedulerState {
    limits: TargetSchedulerLimits,
    lanes: BTreeMap<ObjectId, TargetLane>,
    calls: BTreeMap<CallId, ObjectId>,
    waiting_calls: usize,
    peak_active_targets: usize,
    peak_waiting_calls: usize,
}

impl TargetSchedulerState {
    fn new(limits: TargetSchedulerLimits) -> Self {
        Self {
            limits,
            lanes: BTreeMap::new(),
            calls: BTreeMap::new(),
            waiting_calls: 0,
            peak_active_targets: 0,
            peak_waiting_calls: 0,
        }
    }
}

struct TargetLane {
    active: CallId,
    waiting: VecDeque<WaitingCall>,
}

struct WaitingCall {
    call_id: CallId,
    signal: Arc<AdmissionSignal>,
}

struct AdmissionSignal {
    granted: OnceLock<()>,
    waker: Mutex<Option<Waker>>,
}

impl AdmissionSignal {
    const fn new() -> Self {
        Self {
            granted: OnceLock::new(),
            waker: Mutex::new(None),
        }
    }

    fn grant(&self) {
        if self.granted.set(()).is_err() {
            return;
        }
        let waker = self
            .waker
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    fn poll(&self, task: &mut Context<'_>) -> Poll<()> {
        if self.granted.get().is_some() {
            return Poll::Ready(());
        }
        let mut waker = self
            .waker
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.granted.get().is_some() {
            return Poll::Ready(());
        }
        if waker
            .as_ref()
            .is_none_or(|registered| !registered.will_wake(task.waker()))
        {
            *waker = Some(task.waker().clone());
        }
        Poll::Pending
    }
}

#[must_use = "an acquisition must be awaited or dropped to leave the target queue"]
pub struct TargetPermitFuture {
    scheduler: TargetScheduler,
    target: ObjectId,
    call_id: CallId,
    signal: Arc<AdmissionSignal>,
    completed: bool,
}

impl Future for TargetPermitFuture {
    type Output = Result<TargetPermit, TargetSchedulerError>;

    fn poll(mut self: Pin<&mut Self>, task: &mut Context<'_>) -> Poll<Self::Output> {
        match self.signal.poll(task) {
            Poll::Ready(()) => {
                self.completed = true;
                Poll::Ready(Ok(TargetPermit {
                    scheduler: self.scheduler.clone(),
                    target: self.target,
                    call_id: self.call_id,
                    released: false,
                }))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for TargetPermitFuture {
    fn drop(&mut self) {
        if !self.completed {
            let _ = self.scheduler.release_call(self.call_id);
        }
    }
}

#[must_use = "dropping or releasing the permit admits the next call for this target"]
pub struct TargetPermit {
    scheduler: TargetScheduler,
    target: ObjectId,
    call_id: CallId,
    released: bool,
}

impl TargetPermit {
    pub const fn target(&self) -> ObjectId {
        self.target
    }

    pub const fn call_id(&self) -> CallId {
        self.call_id
    }

    pub fn release(mut self) -> Result<(), TargetSchedulerError> {
        self.scheduler.release_call(self.call_id)?;
        self.released = true;
        Ok(())
    }
}

impl Drop for TargetPermit {
    fn drop(&mut self) {
        if !self.released {
            let _ = self.scheduler.release_call(self.call_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::{Context, Poll, Wake, Waker};

    use pwright_core::{CallId, ObjectId};

    use super::{
        TargetScheduler, TargetSchedulerError, TargetSchedulerLimits, TargetSchedulerMetrics,
    };

    struct CountingWake {
        wakes: AtomicUsize,
    }

    impl Wake for CountingWake {
        fn wake(self: Arc<Self>) {
            self.wakes.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn poll_with_waker<F>(future: Pin<&mut F>, wake: Arc<CountingWake>) -> Poll<F::Output>
    where
        F: Future,
    {
        let waker = Waker::from(wake);
        let mut context = Context::from_waker(&waker);
        future.poll(&mut context)
    }

    #[test]
    fn same_target_calls_are_admitted_in_fifo_order() {
        // Arrange
        let scheduler = TargetScheduler::new(TargetSchedulerLimits::new(1, 2));
        let mut first = Box::pin(
            scheduler
                .acquire(ObjectId::new(1), CallId::new(10))
                .expect("first call should schedule"),
        );
        let mut second = Box::pin(
            scheduler
                .acquire(ObjectId::new(1), CallId::new(11))
                .expect("second call should queue"),
        );
        let first_wake = Arc::new(CountingWake {
            wakes: AtomicUsize::new(0),
        });
        let second_wake = Arc::new(CountingWake {
            wakes: AtomicUsize::new(0),
        });
        let Poll::Ready(Ok(first_permit)) =
            poll_with_waker(first.as_mut(), Arc::clone(&first_wake))
        else {
            panic!("first call should be admitted");
        };
        assert!(poll_with_waker(second.as_mut(), Arc::clone(&second_wake)).is_pending());

        // Act
        first_permit.release().expect("first permit should release");

        // Assert
        assert_eq!(second_wake.wakes.load(Ordering::SeqCst), 1);
        let Poll::Ready(Ok(second_permit)) =
            poll_with_waker(second.as_mut(), Arc::clone(&second_wake))
        else {
            panic!("second call should be admitted next");
        };
        assert_eq!(second_permit.call_id(), CallId::new(11));
        assert_eq!(second_permit.target(), ObjectId::new(1));
        drop(second_permit);
        assert_eq!(
            scheduler.metrics(),
            Ok(TargetSchedulerMetrics {
                active_targets: 0,
                waiting_calls: 0,
                peak_active_targets: 1,
                peak_waiting_calls: 1,
            })
        );
    }

    #[test]
    fn independent_targets_are_admitted_concurrently() {
        // Arrange
        let scheduler = TargetScheduler::new(TargetSchedulerLimits::new(2, 0));
        let mut first = Box::pin(
            scheduler
                .acquire(ObjectId::new(1), CallId::new(10))
                .expect("first target should schedule"),
        );
        let mut second = Box::pin(
            scheduler
                .acquire(ObjectId::new(2), CallId::new(11))
                .expect("second target should schedule"),
        );
        let wake = Arc::new(CountingWake {
            wakes: AtomicUsize::new(0),
        });

        // Act
        let first_outcome = poll_with_waker(first.as_mut(), Arc::clone(&wake));
        let second_outcome = poll_with_waker(second.as_mut(), Arc::clone(&wake));

        // Assert
        assert!(matches!(first_outcome, Poll::Ready(Ok(_))));
        assert!(matches!(second_outcome, Poll::Ready(Ok(_))));
        assert_eq!(
            scheduler
                .metrics()
                .expect("metrics should load")
                .active_targets,
            2
        );
    }

    #[test]
    fn scheduler_rejects_waiters_beyond_the_hard_limit() {
        // Arrange
        let scheduler = TargetScheduler::new(TargetSchedulerLimits::new(1, 1));
        let _first = scheduler
            .acquire(ObjectId::new(1), CallId::new(10))
            .expect("active call should schedule");
        let _second = scheduler
            .acquire(ObjectId::new(1), CallId::new(11))
            .expect("one waiter should fit");

        // Act
        let error = match scheduler.acquire(ObjectId::new(1), CallId::new(12)) {
            Ok(_acquisition) => panic!("second waiter should exceed the limit"),
            Err(error) => error,
        };

        // Assert
        assert_eq!(
            error,
            TargetSchedulerError::WaitingCallCapacityExceeded { limit: 1 }
        );
        assert_eq!(
            scheduler
                .metrics()
                .expect("metrics should load")
                .waiting_calls,
            1
        );
    }

    #[test]
    fn scheduler_rejects_targets_beyond_the_hard_limit() {
        // Arrange
        let scheduler = TargetScheduler::new(TargetSchedulerLimits::new(1, 1));
        let _first = scheduler
            .acquire(ObjectId::new(1), CallId::new(10))
            .expect("first target should fit");

        // Act
        let error = match scheduler.acquire(ObjectId::new(2), CallId::new(11)) {
            Ok(_acquisition) => panic!("second active target should exceed the limit"),
            Err(error) => error,
        };

        // Assert
        assert_eq!(
            error,
            TargetSchedulerError::ActiveTargetCapacityExceeded { limit: 1 }
        );
        assert_eq!(
            scheduler
                .metrics()
                .expect("metrics should load")
                .active_targets,
            1
        );
    }

    #[test]
    fn scheduler_rejects_duplicate_call_identity_across_targets() {
        // Arrange
        let scheduler = TargetScheduler::new(TargetSchedulerLimits::new(2, 1));
        let _first = scheduler
            .acquire(ObjectId::new(1), CallId::new(10))
            .expect("first call should schedule");

        // Act
        let error = match scheduler.acquire(ObjectId::new(2), CallId::new(10)) {
            Ok(_acquisition) => panic!("duplicate call should not schedule"),
            Err(error) => error,
        };

        // Assert
        assert_eq!(
            error,
            TargetSchedulerError::DuplicateCall {
                call_id: CallId::new(10)
            }
        );
        assert_eq!(
            scheduler
                .metrics()
                .expect("metrics should load")
                .active_targets,
            1
        );
    }

    #[test]
    fn dropping_a_waiter_removes_it_without_disturbing_the_active_call() {
        // Arrange
        let scheduler = TargetScheduler::new(TargetSchedulerLimits::new(1, 1));
        let mut first = Box::pin(
            scheduler
                .acquire(ObjectId::new(1), CallId::new(10))
                .expect("active call should schedule"),
        );
        let wake = Arc::new(CountingWake {
            wakes: AtomicUsize::new(0),
        });
        let Poll::Ready(Ok(first_permit)) = poll_with_waker(first.as_mut(), Arc::clone(&wake))
        else {
            panic!("first call should be admitted");
        };
        let second = scheduler
            .acquire(ObjectId::new(1), CallId::new(11))
            .expect("waiter should schedule");

        // Act
        drop(second);

        // Assert
        assert_eq!(
            scheduler
                .metrics()
                .expect("metrics should load")
                .waiting_calls,
            0
        );
        assert_eq!(
            scheduler
                .metrics()
                .expect("metrics should load")
                .active_targets,
            1
        );
        drop(first_permit);
        assert_eq!(
            scheduler
                .metrics()
                .expect("metrics should load")
                .active_targets,
            0
        );
    }
}
