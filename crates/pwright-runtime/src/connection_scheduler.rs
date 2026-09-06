use std::collections::{BTreeSet, VecDeque};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, OnceLock};
use std::task::{Context, Poll, Waker};

use pwright_core::{
    CallId, ResourceClass, ResourceError, ResourceLease, ResourceLedger, ResourceRequest,
    ResourceScope,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConnectionSchedulerLimits {
    pub active_commands: usize,
    pub waiting_calls: usize,
}

impl ConnectionSchedulerLimits {
    pub const fn new(active_commands: usize, waiting_calls: usize) -> Self {
        Self {
            active_commands,
            waiting_calls,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ConnectionSchedulerMetrics {
    pub active_commands: usize,
    pub waiting_calls: usize,
    pub peak_active_commands: usize,
    pub peak_waiting_calls: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConnectionSchedulerError {
    DuplicateCall { call_id: CallId },
    ActiveCommandCapacityExceeded { limit: usize },
    WaitingCallCapacityExceeded { limit: usize },
    MissingCall { call_id: CallId },
    InconsistentState { call_id: CallId },
    ResourceLedgerAlreadyInstalled,
    ResourceLedgerRequiresEmptyState,
    Resource(ResourceError),
    StatePoisoned,
}

impl Display for ConnectionSchedulerError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateCall { call_id } => {
                write!(
                    formatter,
                    "call `{call_id}` is already connection-scheduled"
                )
            }
            Self::ActiveCommandCapacityExceeded { limit } => {
                write!(formatter, "active command limit of {limit} was reached")
            }
            Self::WaitingCallCapacityExceeded { limit } => {
                write!(
                    formatter,
                    "connection waiting call limit of {limit} was reached"
                )
            }
            Self::MissingCall { call_id } => {
                write!(formatter, "call `{call_id}` is not connection-scheduled")
            }
            Self::InconsistentState { call_id } => write!(
                formatter,
                "call `{call_id}` is missing from connection admission state"
            ),
            Self::ResourceLedgerAlreadyInstalled => {
                formatter.write_str("connection scheduler resource ledger is already installed")
            }
            Self::ResourceLedgerRequiresEmptyState => formatter.write_str(
                "connection scheduler resource ledger must be installed before call admission",
            ),
            Self::Resource(error) => Display::fmt(error, formatter),
            Self::StatePoisoned => formatter.write_str("connection scheduler lock was poisoned"),
        }
    }
}

impl Error for ConnectionSchedulerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resource(error) => Some(error),
            Self::DuplicateCall { .. }
            | Self::ActiveCommandCapacityExceeded { .. }
            | Self::WaitingCallCapacityExceeded { .. }
            | Self::MissingCall { .. }
            | Self::InconsistentState { .. }
            | Self::ResourceLedgerAlreadyInstalled
            | Self::ResourceLedgerRequiresEmptyState
            | Self::StatePoisoned => None,
        }
    }
}

impl From<ResourceError> for ConnectionSchedulerError {
    fn from(error: ResourceError) -> Self {
        Self::Resource(error)
    }
}

/// Bounded FIFO admission for aggregate backend commands on one connection.
///
/// This scheduler creates no tasks. Dropping a queued acquisition removes it;
/// dropping or releasing a permit wakes exactly the oldest waiter.
#[derive(Clone)]
pub struct ConnectionScheduler {
    shared: Arc<Mutex<ConnectionSchedulerState>>,
}

impl ConnectionScheduler {
    pub fn new(limits: ConnectionSchedulerLimits) -> Self {
        Self {
            shared: Arc::new(Mutex::new(ConnectionSchedulerState::new(limits))),
        }
    }

    pub(crate) fn install_resource_ledger(
        &self,
        ledger: ResourceLedger,
    ) -> Result<(), ConnectionSchedulerError> {
        let mut state = self
            .shared
            .lock()
            .map_err(|_| ConnectionSchedulerError::StatePoisoned)?;
        if state.resource_ledger.is_some() {
            return Err(ConnectionSchedulerError::ResourceLedgerAlreadyInstalled);
        }
        if !state.calls.is_empty() || !state.active_resources.is_empty() {
            return Err(ConnectionSchedulerError::ResourceLedgerRequiresEmptyState);
        }
        state.resource_ledger = Some(ledger);
        Ok(())
    }

    pub fn acquire(
        &self,
        call_id: CallId,
    ) -> Result<ConnectionPermitFuture, ConnectionSchedulerError> {
        let mut state = self
            .shared
            .lock()
            .map_err(|_| ConnectionSchedulerError::StatePoisoned)?;
        if state.calls.contains(&call_id) {
            return Err(ConnectionSchedulerError::DuplicateCall { call_id });
        }
        if state.limits.active_commands == 0 {
            return Err(ConnectionSchedulerError::ActiveCommandCapacityExceeded { limit: 0 });
        }

        let signal = Arc::new(AdmissionSignal::new());
        if state.active.len() < state.limits.active_commands {
            let resource = state.reserve_active(call_id)?;
            state.active.insert(call_id);
            if let Some(resource) = resource {
                let previous = state.active_resources.insert(call_id, resource);
                debug_assert!(previous.is_none());
            }
            state.peak_active_commands = state.peak_active_commands.max(state.active.len());
            signal.complete(Ok(()));
        } else {
            if state.waiting.len() >= state.limits.waiting_calls {
                return Err(ConnectionSchedulerError::WaitingCallCapacityExceeded {
                    limit: state.limits.waiting_calls,
                });
            }
            state.waiting.push_back(WaitingCall {
                call_id,
                signal: Arc::clone(&signal),
            });
            state.peak_waiting_calls = state.peak_waiting_calls.max(state.waiting.len());
        }
        state.calls.insert(call_id);

        Ok(ConnectionPermitFuture {
            scheduler: self.clone(),
            call_id,
            signal,
            completed: false,
        })
    }

    pub fn metrics(&self) -> Result<ConnectionSchedulerMetrics, ConnectionSchedulerError> {
        let state = self
            .shared
            .lock()
            .map_err(|_| ConnectionSchedulerError::StatePoisoned)?;
        Ok(ConnectionSchedulerMetrics {
            active_commands: state.active.len(),
            waiting_calls: state.waiting.len(),
            peak_active_commands: state.peak_active_commands,
            peak_waiting_calls: state.peak_waiting_calls,
        })
    }

    fn release_call(&self, call_id: CallId) -> Result<(), ConnectionSchedulerError> {
        let mut state = self
            .shared
            .lock()
            .map_err(|_| ConnectionSchedulerError::StatePoisoned)?;
        if !state.calls.remove(&call_id) {
            return Err(ConnectionSchedulerError::MissingCall { call_id });
        }

        let mut promotion_error = None;
        let mut failed_waiters = VecDeque::new();
        let promoted = if state.active.remove(&call_id) {
            let mut resource = state.take_active_resource(call_id);
            match state.waiting.pop_front() {
                Some(next) => {
                    let admission = match &mut resource {
                        Some(resource) => resource
                            .try_replace(
                                ResourceScope::Call(next.call_id),
                                &[ResourceRequest::new(ResourceClass::ActiveCommand, 1)],
                            )
                            .map_err(ConnectionSchedulerError::Resource),
                        None => Ok(()),
                    };
                    match admission {
                        Ok(()) => {
                            state.active.insert(next.call_id);
                            if let Some(resource) = resource {
                                let previous =
                                    state.active_resources.insert(next.call_id, resource);
                                debug_assert!(previous.is_none());
                            }
                            Some((next.signal, Ok(())))
                        }
                        Err(error) => {
                            state.calls.remove(&next.call_id);
                            promotion_error = Some(error.clone());
                            failed_waiters = std::mem::take(&mut state.waiting);
                            for waiting in &failed_waiters {
                                state.calls.remove(&waiting.call_id);
                            }
                            Some((next.signal, Err(error)))
                        }
                    }
                }
                None => None,
            }
        } else {
            let position = state
                .waiting
                .iter()
                .position(|waiting| waiting.call_id == call_id)
                .ok_or(ConnectionSchedulerError::InconsistentState { call_id })?;
            state.waiting.remove(position);
            None
        };
        drop(state);
        if let Some((signal, outcome)) = promoted {
            signal.complete(outcome);
        }
        if let Some(error) = &promotion_error {
            for waiting in failed_waiters {
                waiting.signal.complete(Err(error.clone()));
            }
        }
        match promotion_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

struct ConnectionSchedulerState {
    limits: ConnectionSchedulerLimits,
    calls: BTreeSet<CallId>,
    active: BTreeSet<CallId>,
    active_resources: std::collections::BTreeMap<CallId, ResourceLease>,
    resource_ledger: Option<ResourceLedger>,
    waiting: VecDeque<WaitingCall>,
    peak_active_commands: usize,
    peak_waiting_calls: usize,
}

impl ConnectionSchedulerState {
    fn new(limits: ConnectionSchedulerLimits) -> Self {
        Self {
            limits,
            calls: BTreeSet::new(),
            active: BTreeSet::new(),
            active_resources: std::collections::BTreeMap::new(),
            resource_ledger: None,
            waiting: VecDeque::new(),
            peak_active_commands: 0,
            peak_waiting_calls: 0,
        }
    }

    fn reserve_active(
        &self,
        call_id: CallId,
    ) -> Result<Option<ResourceLease>, ConnectionSchedulerError> {
        self.resource_ledger
            .as_ref()
            .map(|ledger| {
                ledger.reserve(
                    ResourceScope::Call(call_id),
                    ResourceClass::ActiveCommand,
                    1,
                )
            })
            .transpose()
            .map_err(ConnectionSchedulerError::Resource)
    }

    fn take_active_resource(&mut self, call_id: CallId) -> Option<ResourceLease> {
        if self.resource_ledger.is_some() {
            Some(
                self.active_resources
                    .remove(&call_id)
                    .expect("every budgeted active command owns one resource lease"),
            )
        } else {
            debug_assert!(!self.active_resources.contains_key(&call_id));
            None
        }
    }
}

struct WaitingCall {
    call_id: CallId,
    signal: Arc<AdmissionSignal>,
}

struct AdmissionSignal {
    outcome: OnceLock<Result<(), ConnectionSchedulerError>>,
    waker: Mutex<Option<Waker>>,
}

impl AdmissionSignal {
    const fn new() -> Self {
        Self {
            outcome: OnceLock::new(),
            waker: Mutex::new(None),
        }
    }

    fn complete(&self, outcome: Result<(), ConnectionSchedulerError>) {
        if self.outcome.set(outcome).is_err() {
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

    fn poll(&self, task: &mut Context<'_>) -> Poll<Result<(), ConnectionSchedulerError>> {
        if let Some(outcome) = self.outcome.get() {
            return Poll::Ready(outcome.clone());
        }
        let mut waker = self
            .waker
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(outcome) = self.outcome.get() {
            return Poll::Ready(outcome.clone());
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

#[must_use = "an acquisition must be awaited or dropped to leave the connection queue"]
pub struct ConnectionPermitFuture {
    scheduler: ConnectionScheduler,
    call_id: CallId,
    signal: Arc<AdmissionSignal>,
    completed: bool,
}

impl Future for ConnectionPermitFuture {
    type Output = Result<ConnectionPermit, ConnectionSchedulerError>;

    fn poll(mut self: Pin<&mut Self>, task: &mut Context<'_>) -> Poll<Self::Output> {
        match self.signal.poll(task) {
            Poll::Ready(Ok(())) => {
                self.completed = true;
                Poll::Ready(Ok(ConnectionPermit {
                    scheduler: self.scheduler.clone(),
                    call_id: self.call_id,
                    released: false,
                }))
            }
            Poll::Ready(Err(error)) => {
                self.completed = true;
                Poll::Ready(Err(error))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for ConnectionPermitFuture {
    fn drop(&mut self) {
        if !self.completed {
            let _ = self.scheduler.release_call(self.call_id);
        }
    }
}

#[must_use = "dropping or releasing the permit admits the next connection call"]
pub struct ConnectionPermit {
    scheduler: ConnectionScheduler,
    call_id: CallId,
    released: bool,
}

impl ConnectionPermit {
    pub const fn call_id(&self) -> CallId {
        self.call_id
    }

    pub fn release(mut self) -> Result<(), ConnectionSchedulerError> {
        self.scheduler.release_call(self.call_id)?;
        self.released = true;
        Ok(())
    }
}

impl Drop for ConnectionPermit {
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

    use pwright_core::{
        CallId, ResourceClass, ResourceError, ResourceLedger, ResourceLimits, ResourceScope,
    };

    use super::{
        ConnectionScheduler, ConnectionSchedulerError, ConnectionSchedulerLimits,
        ConnectionSchedulerMetrics,
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

    fn active_ledger(global: u64, per_scope: u64, scopes: usize) -> ResourceLedger {
        ResourceLedger::new(ResourceLimits::deny_all(scopes, 8).with_class(
            ResourceClass::ActiveCommand,
            global,
            per_scope,
        ))
    }

    #[test]
    fn active_budget_excludes_waiters_and_moves_with_fifo_promotion() {
        // Arrange
        let ledger = active_ledger(1, 1, 1);
        let scheduler = ConnectionScheduler::new(ConnectionSchedulerLimits::new(1, 2));
        scheduler
            .install_resource_ledger(ledger.clone())
            .expect("ledger should install before admission");
        let mut first = Box::pin(
            scheduler
                .acquire(CallId::new(1))
                .expect("first call should receive the active slot"),
        );
        let mut second = Box::pin(
            scheduler
                .acquire(CallId::new(2))
                .expect("second call should wait"),
        );
        let wake = Arc::new(CountingWake {
            wakes: AtomicUsize::new(0),
        });
        let Poll::Ready(Ok(first_permit)) = poll_with_waker(first.as_mut(), Arc::clone(&wake))
        else {
            panic!("first call should be active");
        };
        assert!(poll_with_waker(second.as_mut(), Arc::clone(&wake)).is_pending());
        let before = ledger.metrics().expect("metrics should be available");

        // Act
        first_permit.release().expect("first permit should release");
        let Poll::Ready(Ok(second_permit)) = poll_with_waker(second.as_mut(), Arc::clone(&wake))
        else {
            panic!("oldest waiter should be promoted");
        };
        let promoted = ledger.metrics().expect("metrics should be available");
        let old_scope = ledger
            .scope_metrics(ResourceScope::Call(CallId::new(1)))
            .expect("old scope metrics should be available");
        let new_scope = ledger
            .scope_metrics(ResourceScope::Call(CallId::new(2)))
            .expect("new scope metrics should be available")
            .expect("promoted call should own the active lease");
        drop(second_permit);
        let released = ledger.metrics().expect("metrics should be available");

        // Assert
        assert_eq!(before.class(ResourceClass::ActiveCommand).current, 1);
        assert_eq!(before.active_leases, 1);
        assert_eq!(promoted.class(ResourceClass::ActiveCommand).current, 1);
        assert_eq!(promoted.class(ResourceClass::ActiveCommand).peak, 1);
        assert_eq!(promoted.live_scopes, 1);
        assert!(old_scope.is_none());
        assert_eq!(new_scope.units(ResourceClass::ActiveCommand), 1);
        assert_eq!(released.class(ResourceClass::ActiveCommand).current, 0);
        assert_eq!(released.active_leases, 0);
    }

    #[test]
    fn active_budget_rejects_before_scheduler_state_changes() {
        // Arrange
        let ledger = active_ledger(0, 0, 1);
        let scheduler = ConnectionScheduler::new(ConnectionSchedulerLimits::new(1, 1));
        scheduler
            .install_resource_ledger(ledger.clone())
            .expect("deny-all ledger should still install");

        // Act
        let error = match scheduler.acquire(CallId::new(1)) {
            Err(error) => error,
            Ok(_) => panic!("active command should exceed the shared budget"),
        };
        let scheduler_metrics = scheduler.metrics().expect("scheduler metrics should load");
        let resource_metrics = ledger.metrics().expect("resource metrics should load");

        // Assert
        assert_eq!(
            error,
            ConnectionSchedulerError::Resource(ResourceError::GlobalLimitExceeded {
                class: ResourceClass::ActiveCommand,
                current: 0,
                requested: 1,
                limit: 0,
            })
        );
        assert_eq!(scheduler_metrics.active_commands, 0);
        assert_eq!(scheduler_metrics.waiting_calls, 0);
        assert_eq!(
            resource_metrics.class(ResourceClass::ActiveCommand).current,
            0
        );
        assert_eq!(resource_metrics.rejected_reservations, 1);
    }

    #[test]
    fn dropping_unpolled_granted_acquisition_releases_active_budget() {
        // Arrange
        let ledger = active_ledger(1, 1, 1);
        let scheduler = ConnectionScheduler::new(ConnectionSchedulerLimits::new(1, 0));
        scheduler
            .install_resource_ledger(ledger.clone())
            .expect("ledger should install before admission");
        let acquisition = scheduler
            .acquire(CallId::new(1))
            .expect("call should be granted immediately");

        // Act
        drop(acquisition);
        let metrics = ledger.metrics().expect("metrics should be available");

        // Assert
        assert_eq!(metrics.class(ResourceClass::ActiveCommand).current, 0);
        assert_eq!(metrics.class(ResourceClass::ActiveCommand).peak, 1);
        assert_eq!(metrics.active_leases, 0);
        assert_eq!(
            scheduler.metrics(),
            Ok(ConnectionSchedulerMetrics {
                peak_active_commands: 1,
                ..ConnectionSchedulerMetrics::default()
            })
        );
    }

    #[test]
    fn failed_promotion_rejects_all_waiters_and_releases_the_old_slot() {
        // Arrange
        let ledger = active_ledger(3, 1, 4);
        let conflicting = ledger
            .reserve(
                ResourceScope::Call(CallId::new(2)),
                ResourceClass::ActiveCommand,
                1,
            )
            .expect("failure injection lease should fit");
        let scheduler = ConnectionScheduler::new(ConnectionSchedulerLimits::new(1, 2));
        scheduler
            .install_resource_ledger(ledger.clone())
            .expect("ledger should install before scheduler admission");
        let mut first = Box::pin(
            scheduler
                .acquire(CallId::new(1))
                .expect("first call should be active"),
        );
        let mut second = Box::pin(
            scheduler
                .acquire(CallId::new(2))
                .expect("second call should wait"),
        );
        let mut third = Box::pin(
            scheduler
                .acquire(CallId::new(3))
                .expect("third call should wait"),
        );
        let wake = Arc::new(CountingWake {
            wakes: AtomicUsize::new(0),
        });
        let Poll::Ready(Ok(first_permit)) = poll_with_waker(first.as_mut(), Arc::clone(&wake))
        else {
            panic!("first call should be active");
        };
        assert!(poll_with_waker(second.as_mut(), Arc::clone(&wake)).is_pending());
        assert!(poll_with_waker(third.as_mut(), Arc::clone(&wake)).is_pending());

        // Act
        let release_error = first_permit
            .release()
            .expect_err("conflicting target scope should reject promotion");
        let second_error = poll_with_waker(second.as_mut(), Arc::clone(&wake));
        let third_error = poll_with_waker(third.as_mut(), wake);
        let scheduler_metrics = scheduler.metrics().expect("scheduler metrics should load");
        let resource_metrics = ledger.metrics().expect("resource metrics should load");

        // Assert
        assert!(matches!(
            release_error,
            ConnectionSchedulerError::Resource(ResourceError::ScopeLimitExceeded {
                scope: ResourceScope::Call(call_id),
                class: ResourceClass::ActiveCommand,
                current: 1,
                requested: 1,
                limit: 1,
            }) if call_id == CallId::new(2)
        ));
        assert!(matches!(
            second_error,
            Poll::Ready(Err(ConnectionSchedulerError::Resource(_)))
        ));
        assert!(matches!(
            third_error,
            Poll::Ready(Err(ConnectionSchedulerError::Resource(_)))
        ));
        assert_eq!(scheduler_metrics.active_commands, 0);
        assert_eq!(scheduler_metrics.waiting_calls, 0);
        assert_eq!(
            resource_metrics.class(ResourceClass::ActiveCommand).current,
            1
        );
        drop(conflicting);
    }

    #[test]
    fn scheduler_admits_up_to_the_limit_and_promotes_fifo() {
        // Arrange
        let scheduler = ConnectionScheduler::new(ConnectionSchedulerLimits::new(2, 2));
        let mut first = Box::pin(
            scheduler
                .acquire(CallId::new(1))
                .expect("first should schedule"),
        );
        let mut second = Box::pin(
            scheduler
                .acquire(CallId::new(2))
                .expect("second should schedule"),
        );
        let mut third = Box::pin(
            scheduler
                .acquire(CallId::new(3))
                .expect("third should queue"),
        );
        let mut fourth = Box::pin(
            scheduler
                .acquire(CallId::new(4))
                .expect("fourth should queue"),
        );
        let wake = Arc::new(CountingWake {
            wakes: AtomicUsize::new(0),
        });
        let Poll::Ready(Ok(first_permit)) = poll_with_waker(first.as_mut(), Arc::clone(&wake))
        else {
            panic!("first call should be admitted");
        };
        let Poll::Ready(Ok(_second_permit)) = poll_with_waker(second.as_mut(), Arc::clone(&wake))
        else {
            panic!("second call should be admitted");
        };
        assert!(poll_with_waker(third.as_mut(), Arc::clone(&wake)).is_pending());
        assert!(poll_with_waker(fourth.as_mut(), Arc::clone(&wake)).is_pending());

        // Act
        first_permit.release().expect("permit should release");

        // Assert
        let Poll::Ready(Ok(third_permit)) = poll_with_waker(third.as_mut(), Arc::clone(&wake))
        else {
            panic!("oldest waiter should be promoted");
        };
        assert!(poll_with_waker(fourth.as_mut(), Arc::clone(&wake)).is_pending());
        assert_eq!(third_permit.call_id(), CallId::new(3));
        assert_eq!(
            scheduler.metrics(),
            Ok(ConnectionSchedulerMetrics {
                active_commands: 2,
                waiting_calls: 1,
                peak_active_commands: 2,
                peak_waiting_calls: 2,
            })
        );
    }

    #[test]
    fn scheduler_rejects_zero_active_capacity() {
        // Arrange
        let scheduler = ConnectionScheduler::new(ConnectionSchedulerLimits::new(0, 1));

        // Act
        let error = scheduler
            .acquire(CallId::new(1))
            .err()
            .expect("zero active capacity cannot make progress");

        // Assert
        assert_eq!(
            error,
            ConnectionSchedulerError::ActiveCommandCapacityExceeded { limit: 0 }
        );
    }

    #[test]
    fn scheduler_enforces_waiting_capacity() {
        // Arrange
        let scheduler = ConnectionScheduler::new(ConnectionSchedulerLimits::new(1, 1));
        let _active = scheduler
            .acquire(CallId::new(1))
            .expect("active call should schedule");
        let _waiting = scheduler
            .acquire(CallId::new(2))
            .expect("one waiter should fit");

        // Act
        let error = scheduler
            .acquire(CallId::new(3))
            .err()
            .expect("second waiter should exceed the bound");

        // Assert
        assert_eq!(
            error,
            ConnectionSchedulerError::WaitingCallCapacityExceeded { limit: 1 }
        );
    }

    #[test]
    fn scheduler_rejects_duplicate_call_identity() {
        // Arrange
        let scheduler = ConnectionScheduler::new(ConnectionSchedulerLimits::new(2, 0));
        let _first = scheduler
            .acquire(CallId::new(1))
            .expect("first call should schedule");

        // Act
        let error = scheduler
            .acquire(CallId::new(1))
            .err()
            .expect("duplicate call should fail");

        // Assert
        assert_eq!(
            error,
            ConnectionSchedulerError::DuplicateCall {
                call_id: CallId::new(1)
            }
        );
    }

    #[test]
    fn dropping_a_waiter_removes_it_without_waking_later_calls() {
        // Arrange
        let scheduler = ConnectionScheduler::new(ConnectionSchedulerLimits::new(1, 2));
        let mut active = Box::pin(
            scheduler
                .acquire(CallId::new(1))
                .expect("active should schedule"),
        );
        let waiting = scheduler
            .acquire(CallId::new(2))
            .expect("waiter should schedule");
        let mut last = Box::pin(
            scheduler
                .acquire(CallId::new(3))
                .expect("last should schedule"),
        );
        let wake = Arc::new(CountingWake {
            wakes: AtomicUsize::new(0),
        });
        let Poll::Ready(Ok(active_permit)) = poll_with_waker(active.as_mut(), Arc::clone(&wake))
        else {
            panic!("first call should be active");
        };
        assert!(poll_with_waker(last.as_mut(), Arc::clone(&wake)).is_pending());

        // Act
        drop(waiting);

        // Assert
        assert_eq!(wake.wakes.load(Ordering::SeqCst), 0);
        assert_eq!(
            scheduler
                .metrics()
                .expect("metrics should load")
                .waiting_calls,
            1
        );
        active_permit.release().expect("active should release");
        assert!(matches!(
            poll_with_waker(last.as_mut(), wake),
            Poll::Ready(Ok(_))
        ));
    }

    #[test]
    fn dropping_an_unpolled_granted_acquisition_releases_capacity() {
        // Arrange
        let scheduler = ConnectionScheduler::new(ConnectionSchedulerLimits::new(1, 1));
        let granted = scheduler
            .acquire(CallId::new(1))
            .expect("first call should be granted");
        let mut waiting = Box::pin(
            scheduler
                .acquire(CallId::new(2))
                .expect("second should wait"),
        );
        let wake = Arc::new(CountingWake {
            wakes: AtomicUsize::new(0),
        });
        assert!(poll_with_waker(waiting.as_mut(), Arc::clone(&wake)).is_pending());

        // Act
        drop(granted);

        // Assert
        assert_eq!(wake.wakes.load(Ordering::SeqCst), 1);
        assert!(matches!(
            poll_with_waker(waiting.as_mut(), wake),
            Poll::Ready(Ok(_))
        ));
    }
}
