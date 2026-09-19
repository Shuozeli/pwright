use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::num::NonZeroUsize;
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
/// `Drop`, which wakes exactly the next waiter for the same target. The default
/// is serial; an explicit per-target concurrency selection grants a bounded set
/// of permits while retaining FIFO order for excess calls.
#[derive(Clone)]
pub struct TargetScheduler {
    shared: Arc<Mutex<TargetSchedulerState>>,
}

impl TargetScheduler {
    pub fn new(limits: TargetSchedulerLimits) -> Self {
        Self::with_concurrency(limits, NonZeroUsize::MIN)
    }

    pub fn with_concurrency(limits: TargetSchedulerLimits, maximum: NonZeroUsize) -> Self {
        Self {
            shared: Arc::new(Mutex::new(TargetSchedulerState::new(limits, maximum))),
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
        if let Some(lane) = state.lanes.get(&target) {
            let can_grant = lane.active.len() < state.target_concurrency.get();
            let waiting_limit = state.limits.waiting_calls;
            if !can_grant && state.waiting_calls >= waiting_limit {
                return Err(TargetSchedulerError::WaitingCallCapacityExceeded {
                    limit: waiting_limit,
                });
            }
            let lane = state
                .lanes
                .get_mut(&target)
                .expect("target lane existence was checked");
            if can_grant {
                debug_assert!(lane.waiting.is_empty());
                lane.active.insert(call_id);
                signal.grant();
            } else {
                lane.waiting.push_back(WaitingCall {
                    call_id,
                    signal: Arc::clone(&signal),
                });
                state.waiting_calls += 1;
                state.peak_waiting_calls = state.peak_waiting_calls.max(state.waiting_calls);
            }
        } else {
            if state.lanes.len() >= state.limits.active_targets {
                return Err(TargetSchedulerError::ActiveTargetCapacityExceeded {
                    limit: state.limits.active_targets,
                });
            }
            let maximum = state.target_concurrency;
            state.lanes.insert(
                target,
                TargetLane {
                    active: ActiveCalls::new(call_id, maximum),
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
            if lane.active.remove(&call_id) {
                match lane.waiting.pop_front() {
                    Some(next) => {
                        lane.active.insert(next.call_id);
                        (Some(next.signal), false, true)
                    }
                    None => (None, lane.active.is_empty(), false),
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
    target_concurrency: NonZeroUsize,
    lanes: BTreeMap<ObjectId, TargetLane>,
    calls: BTreeMap<CallId, ObjectId>,
    waiting_calls: usize,
    peak_active_targets: usize,
    peak_waiting_calls: usize,
}

impl TargetSchedulerState {
    fn new(limits: TargetSchedulerLimits, target_concurrency: NonZeroUsize) -> Self {
        Self {
            limits,
            target_concurrency,
            lanes: BTreeMap::new(),
            calls: BTreeMap::new(),
            waiting_calls: 0,
            peak_active_targets: 0,
            peak_waiting_calls: 0,
        }
    }
}

struct TargetLane {
    active: ActiveCalls,
    waiting: VecDeque<WaitingCall>,
}

// Keep the serial default's active identity inline. Only the explicitly
// concurrent profile allocates a bounded set of active target identities.
enum ActiveCalls {
    Serial(Option<CallId>),
    Concurrent(BTreeSet<CallId>),
}

impl ActiveCalls {
    fn new(first: CallId, maximum: NonZeroUsize) -> Self {
        if maximum == NonZeroUsize::MIN {
            Self::Serial(Some(first))
        } else {
            Self::Concurrent(BTreeSet::from([first]))
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Serial(call) => usize::from(call.is_some()),
            Self::Concurrent(calls) => calls.len(),
        }
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn insert(&mut self, call_id: CallId) {
        match self {
            Self::Serial(slot) => {
                assert!(slot.is_none());
                *slot = Some(call_id);
            }
            Self::Concurrent(calls) => {
                assert!(calls.insert(call_id));
            }
        }
    }

    fn remove(&mut self, call_id: &CallId) -> bool {
        match self {
            Self::Serial(slot) if slot.as_ref() == Some(call_id) => {
                *slot = None;
                true
            }
            Self::Serial(_) => false,
            Self::Concurrent(calls) => calls.remove(call_id),
        }
    }
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

    fn concurrent(waiters: usize) -> TargetScheduler {
        TargetScheduler::with_concurrency(
            TargetSchedulerLimits::new(1, waiters),
            std::num::NonZeroUsize::new(2).unwrap(),
        )
    }

    fn take_permit(future: &mut super::TargetPermitFuture) -> super::TargetPermit {
        let wake = Arc::new(CountingWake {
            wakes: AtomicUsize::new(0),
        });
        match poll_with_waker(Pin::new(future), wake) {
            Poll::Ready(Ok(permit)) => permit,
            _ => panic!("the selected call must have target admission"),
        }
    }

    #[test]
    fn concurrent_target_promotes_waiters_in_fifo_order_without_releasing_siblings() {
        // Arrange
        let scheduler = concurrent(2);
        let mut first = scheduler.acquire(ObjectId::new(1), CallId::new(1)).unwrap();
        let mut second = scheduler.acquire(ObjectId::new(1), CallId::new(2)).unwrap();
        let mut third = scheduler.acquire(ObjectId::new(1), CallId::new(3)).unwrap();
        let mut fourth = scheduler.acquire(ObjectId::new(1), CallId::new(4)).unwrap();
        let first = take_permit(&mut first);
        let second = take_permit(&mut second);
        let wake = Arc::new(CountingWake {
            wakes: AtomicUsize::new(0),
        });
        assert!(poll_with_waker(Pin::new(&mut third), wake.clone()).is_pending());
        assert!(poll_with_waker(Pin::new(&mut fourth), wake.clone()).is_pending());
        // Act
        drop(first);
        // Assert
        assert_eq!(wake.wakes.load(Ordering::SeqCst), 1);
        let third = take_permit(&mut third);
        assert_eq!(third.call_id(), CallId::new(3));
        assert!(poll_with_waker(Pin::new(&mut fourth), wake).is_pending());
        assert_eq!(scheduler.metrics().unwrap().active_targets, 1);
        drop(third);
        let fourth = take_permit(&mut fourth);
        assert_eq!(fourth.call_id(), CallId::new(4));
        drop((second, fourth));
        assert_eq!(scheduler.metrics().unwrap().active_targets, 0);
        assert_eq!(scheduler.metrics().unwrap().waiting_calls, 0);
    }

    #[test]
    fn concurrent_target_rejects_work_beyond_its_permit_and_waiting_bounds() {
        // Arrange
        let scheduler = concurrent(0);
        let _first = scheduler.acquire(ObjectId::new(1), CallId::new(1)).unwrap();
        let _second = scheduler.acquire(ObjectId::new(1), CallId::new(2)).unwrap();
        // Act
        let result = scheduler.acquire(ObjectId::new(1), CallId::new(3));
        // Assert
        assert!(matches!(
            result,
            Err(TargetSchedulerError::WaitingCallCapacityExceeded { limit: 0 })
        ));
        assert_eq!(scheduler.metrics().unwrap().waiting_calls, 0);
    }

    #[test]
    fn concurrent_target_does_not_expand_the_target_count_limit() {
        // Arrange
        let scheduler = concurrent(0);
        let _first = scheduler.acquire(ObjectId::new(1), CallId::new(1)).unwrap();
        // Act
        let result = scheduler.acquire(ObjectId::new(2), CallId::new(2));
        // Assert
        assert!(matches!(
            result,
            Err(TargetSchedulerError::ActiveTargetCapacityExceeded { limit: 1 })
        ));
    }

    #[test]
    fn dropping_an_unpolled_concurrent_grant_promotes_one_waiter() {
        // Arrange
        let scheduler = concurrent(1);
        let first = scheduler.acquire(ObjectId::new(1), CallId::new(1)).unwrap();
        let second = scheduler.acquire(ObjectId::new(1), CallId::new(2)).unwrap();
        let mut third = scheduler.acquire(ObjectId::new(1), CallId::new(3)).unwrap();
        // Act
        drop(first);
        // Assert
        let third = take_permit(&mut third);
        assert_eq!(scheduler.metrics().unwrap().waiting_calls, 0);
        drop((second, third));
        assert_eq!(scheduler.metrics().unwrap().active_targets, 0);
    }

    #[test]
    fn dropping_a_concurrent_waiter_preserves_the_next_fifo_position() {
        // Arrange
        let scheduler = concurrent(2);
        let first = scheduler.acquire(ObjectId::new(1), CallId::new(1)).unwrap();
        let second = scheduler.acquire(ObjectId::new(1), CallId::new(2)).unwrap();
        let third = scheduler.acquire(ObjectId::new(1), CallId::new(3)).unwrap();
        let mut fourth = scheduler.acquire(ObjectId::new(1), CallId::new(4)).unwrap();
        // Act
        drop(third);
        // Assert
        assert_eq!(scheduler.metrics().unwrap().waiting_calls, 1);
        drop(first);
        let fourth = take_permit(&mut fourth);
        assert_eq!(fourth.call_id(), CallId::new(4));
        drop((second, fourth));
        assert_eq!(scheduler.metrics().unwrap().active_targets, 0);
    }

    #[test]
    fn concurrent_identity_cannot_be_reused_on_another_target() {
        // Arrange
        let scheduler = concurrent(1);
        let _first = scheduler.acquire(ObjectId::new(1), CallId::new(1)).unwrap();
        let _second = scheduler.acquire(ObjectId::new(1), CallId::new(2)).unwrap();
        // Act
        let result = scheduler.acquire(ObjectId::new(2), CallId::new(2));
        // Assert
        assert!(
            matches!(result, Err(TargetSchedulerError::DuplicateCall { call_id }) if call_id == CallId::new(2))
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
