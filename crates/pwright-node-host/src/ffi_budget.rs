//! Admission and reclamation across a foreign runtime's asynchronous boundary.
//! No foreign values or runtime handles are stored here.

use std::collections::BTreeMap;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use pwright_core::{
    ProductionResourceLedger, ResourceClass, ResourceLease, ResourceLedger, ResourceLimits,
    ResourceRequest, ResourceRequirements, ResourceScope,
};
use tokio_util::sync::CancellationToken;
use tokio_util::task::{TaskTracker, task_tracker::TaskTrackerToken};

use crate::HostError;

mod publication_registry;
pub use publication_registry::PublicationRegistryStorage;
#[cfg(feature = "retained-file-input")]
pub use publication_registry::{PublicationPhaseBudget, PublicationPhaseStorage};
#[cfg(feature = "retained-file-input")]
mod preparation;
#[cfg(feature = "retained-file-input")]
pub use preparation::{FfiPreparation, FfiRevoker};

#[derive(Clone, Copy, Debug)]
pub enum FfiLane {
    Connect,
    Command,
    Close,
    Watch,
}

impl FfiLane {
    const fn index(self) -> usize {
        match self {
            Self::Connect => 0,
            Self::Command => 1,
            Self::Close => 2,
            Self::Watch => 3,
        }
    }
}

#[derive(Clone, Copy)]
pub struct FfiLimits {
    pub connects: usize,
    pub commands: usize,
    pub closes: usize,
    pub watches: usize,
    pub reserved_bytes: u64,
    /// Maximum boxed future body for Connect, Command, Close, Watch respectively.
    pub future_bytes: [usize; 4],
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct FfiMetrics {
    pub pending: u32,
    pub queued: u32,
    pub queued_commands: u32,
    pub running: u32,
    pub reserved_bytes: u64,
    pub connects: u32,
    pub commands: u32,
    pub closes: u32,
    pub watches: u32,
}

trait Reclaim: Send + Sync {
    fn reclaim(&self);
    fn is_active(&self) -> bool;
}

struct Registry {
    // Admission authority is sealed under this lock before cancellation wakes
    // arbitrary Rust code. The cancellation token is notification, not the
    // sole close gate during the interval before those wakeups are delivered.
    closed: bool,
    next_id: u64,
    entries: BTreeMap<u64, Arc<dyn Reclaim>>,
}

struct BudgetInner {
    resources: ResourceLedger,
    limits: [usize; 4],
    future_bytes: [usize; 4],
    active: [AtomicUsize; 4],
    queued: AtomicUsize,
    queued_commands: AtomicUsize,
    registry: Mutex<Registry>,
    stopped: CancellationToken,
    tasks: TaskTracker,
}

#[derive(Clone)]
pub struct FfiBudget(Arc<BudgetInner>);

impl FfiBudget {
    pub fn new(limits: FfiLimits) -> Result<Self, HostError> {
        let lanes = [
            limits.connects,
            limits.commands,
            limits.closes,
            limits.watches,
        ];
        let capacity = lanes
            .iter()
            .try_fold(0usize, |total, value| total.checked_add(*value))
            .ok_or_else(|| HostError::ResourceExhausted("FFI capacity overflow".into()))?;
        let units = u64::try_from(capacity).map_err(HostError::backend)?;
        let leases = capacity
            .checked_mul(2)
            .ok_or_else(|| HostError::ResourceExhausted("FFI lease capacity overflow".into()))?;
        let requirements = ResourceRequirements::none()
            .with_class(ResourceClass::PendingCall, units, units)
            .with_class(
                ResourceClass::BackendStateBytes,
                limits.reserved_bytes,
                limits.reserved_bytes,
            )
            .with_metadata(1, leases);
        let resource_limits = ResourceLimits::deny_all(1, leases)
            .with_class(ResourceClass::PendingCall, units, units)
            .with_class(
                ResourceClass::BackendStateBytes,
                limits.reserved_bytes,
                limits.reserved_bytes,
            );
        let resources = ProductionResourceLedger::new(resource_limits, requirements)
            .and_then(|ledger| ledger.claim(requirements))
            .map_err(HostError::backend)?;
        Ok(Self(Arc::new(BudgetInner {
            resources,
            limits: lanes,
            future_bytes: limits.future_bytes,
            active: std::array::from_fn(|_| AtomicUsize::new(0)),
            queued: AtomicUsize::new(0),
            queued_commands: AtomicUsize::new(0),
            registry: Mutex::new(Registry {
                closed: false,
                next_id: 1,
                entries: BTreeMap::new(),
            }),
            stopped: CancellationToken::new(),
            tasks: TaskTracker::new(),
        })))
    }

    /// Reserve before copying foreign arguments or spawning any task. The
    /// byte allowance covers bounded inputs, result/error and conversion
    /// overlap; it is an admission ceiling, not measured process RSS.
    pub fn admit<T: Send + 'static>(
        &self,
        lane: FfiLane,
        reserved_bytes: u64,
    ) -> Result<FfiExecution<T>, HostError> {
        let mut registry = self.0.registry.lock().map_err(HostError::backend)?;
        let ticket = self.admit_ticket(&mut registry, lane, reserved_bytes)?;
        #[cfg(feature = "retained-file-input")]
        {
            ticket.entry.state.lock().map_err(HostError::backend)?.phase =
                preparation::Phase::Running;
        }
        // Register work under shutdown's lock, including the existing eager
        // admission path. Preparations explicitly defer only this registration.
        Ok(FfiExecution {
            ticket,
            running: self.0.tasks.token(),
            stopped: self.0.stopped.clone(),
        })
    }

    /// Caller holds the registry lock through any subsequent work registration.
    fn admit_ticket<T: Send + 'static>(
        &self,
        registry: &mut Registry,
        lane: FfiLane,
        reserved_bytes: u64,
    ) -> Result<FfiTicket<T>, HostError> {
        if registry.closed || self.0.stopped.is_cancelled() {
            return Err(HostError::TargetClosed);
        }
        registry.entries.retain(|_, entry| entry.is_active());
        let index = lane.index();
        if self.0.active[index].load(Ordering::Acquire) >= self.0.limits[index] {
            return Err(HostError::ResourceExhausted(format!(
                "FFI {lane:?} pending-result capacity exceeded ({})",
                self.0.limits[index]
            )));
        }
        let id = registry.next_id;
        registry.next_id = id
            .checked_add(1)
            .ok_or_else(|| HostError::ResourceExhausted("FFI ticket IDs exhausted".into()))?;
        let mut requests = vec![ResourceRequest::new(ResourceClass::PendingCall, 1)];
        if reserved_bytes != 0 {
            requests.push(ResourceRequest::new(
                ResourceClass::BackendStateBytes,
                reserved_bytes,
            ));
        }
        let lease = self
            .0
            .resources
            .reserve_many(ResourceScope::extension(122, 1), &requests)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        self.0.active[index].fetch_add(1, Ordering::Release);
        let entry = Arc::new(Entry {
            state: Mutex::new(EntryState {
                permit: Some(Permit {
                    budget: self.clone(),
                    lane,
                    queued: false,
                    _lease: lease,
                    additional: Vec::new(),
                    future: None,
                }),
                outcome: None,
                #[cfg(feature = "retained-file-input")]
                phase: preparation::Phase::Initial,
                #[cfg(feature = "retained-file-input")]
                revoked: false,
            }),
            #[cfg(feature = "retained-file-input")]
            cancelled: CancellationToken::new(),
        });
        registry.entries.insert(id, entry.clone());
        Ok(FfiTicket { entry })
    }

    pub fn metrics(&self) -> Result<FfiMetrics, HostError> {
        let metrics = self.0.resources.metrics().map_err(HostError::backend)?;
        let [connects, commands, closes, watches] =
            self.0.active.each_ref().map(|count| {
                u32::try_from(count.load(Ordering::Acquire)).map_err(HostError::backend)
            });
        Ok(FfiMetrics {
            pending: u32::try_from(metrics.class(ResourceClass::PendingCall).current)
                .map_err(HostError::backend)?,
            queued: u32::try_from(self.0.queued.load(Ordering::Acquire))
                .map_err(HostError::backend)?,
            queued_commands: u32::try_from(self.0.queued_commands.load(Ordering::Acquire))
                .map_err(HostError::backend)?,
            running: u32::try_from(self.0.tasks.len()).map_err(HostError::backend)?,
            reserved_bytes: metrics.class(ResourceClass::BackendStateBytes).current,
            connects: connects?,
            commands: commands?,
            closes: closes?,
            watches: watches?,
        })
    }

    /// Cancel and join Rust execution, then reclaim queued values without
    /// calling into the foreign runtime. Its callbacks retain only empty
    /// ticket cells afterward; no callback delivery is needed for reclamation.
    pub async fn shutdown(&self) -> Result<(), HostError> {
        {
            let mut registry = self.0.registry.lock().map_err(HostError::backend)?;
            registry.closed = true;
        }
        // Both operations can synchronously wake arbitrary Rust code. The
        // sealed registry rejects admission/promotion even before cancellation
        // is observed; neither wakeup may run while its lock is held.
        self.0.stopped.cancel();
        self.0.tasks.close();
        self.0.tasks.wait().await;
        let entries =
            std::mem::take(&mut self.0.registry.lock().map_err(HostError::backend)?.entries);
        for entry in entries.into_values() {
            entry.reclaim();
        }
        Ok(())
    }
}

struct Permit {
    budget: FfiBudget,
    lane: FfiLane,
    queued: bool,
    _lease: ResourceLease,
    additional: Vec<ResourceLease>,
    future: Option<ResourceLease>,
}

impl Drop for Permit {
    fn drop(&mut self) {
        self.budget.0.active[self.lane.index()].fetch_sub(1, Ordering::Release);
        if self.queued {
            self.budget.0.queued.fetch_sub(1, Ordering::Release);
            if matches!(self.lane, FfiLane::Command) {
                self.budget
                    .0
                    .queued_commands
                    .fetch_sub(1, Ordering::Release);
            }
        }
    }
}

struct EntryState<T> {
    permit: Option<Permit>,
    outcome: Option<Result<T, HostError>>,
    #[cfg(feature = "retained-file-input")]
    phase: preparation::Phase,
    #[cfg(feature = "retained-file-input")]
    revoked: bool,
}
struct Entry<T> {
    state: Mutex<EntryState<T>>,
    #[cfg(feature = "retained-file-input")]
    cancelled: CancellationToken,
}

impl<T: Send> Reclaim for Entry<T> {
    fn reclaim(&self) {
        // Cleanup only, and no user destructor is invoked under the cell lock.
        let (outcome, permit) = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            (state.outcome.take(), state.permit.take())
        };
        drop(outcome);
        drop(permit);
    }

    fn is_active(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .permit
            .is_some()
    }
}

pub struct FfiExecution<T: Send> {
    ticket: FfiTicket<T>,
    running: TaskTrackerToken,
    stopped: CancellationToken,
}

impl<T: Send> FfiExecution<T> {
    /// A typed handle that can charge a result-dependent allocation after the
    /// foreign call has produced its bounded native value. The added lease is
    /// retained by the original ticket through foreign-runtime conversion.
    pub fn reservation(&self) -> FfiReservation<T> {
        FfiReservation {
            entry: Arc::clone(&self.ticket.entry),
        }
    }

    /// Charge the actual inline body before boxing it at the FFI boundary.
    /// Its reservation stays through settlement (conservative after execution).
    pub fn reserve_future(&mut self, bytes: usize) -> Result<(), HostError> {
        let mut state = self.ticket.entry.state.lock().map_err(HostError::backend)?;
        #[cfg(feature = "retained-file-input")]
        if state.revoked {
            return Err(HostError::TargetClosed);
        }
        let permit = state.permit.as_mut().ok_or(HostError::TargetClosed)?;
        let maximum = permit.budget.0.future_bytes[permit.lane.index()];
        if bytes > maximum {
            return Err(HostError::ResourceExhausted(format!(
                "FFI {:?} future needs {bytes} bytes, exceeds {maximum}",
                permit.lane
            )));
        }
        if permit.future.is_some() {
            return Err(HostError::InvalidArgument(
                "FFI future allocation was already reserved".into(),
            ));
        }
        if bytes > 0 {
            permit.future = Some(
                permit
                    .budget
                    .0
                    .resources
                    .reserve(
                        ResourceScope::extension(122, 1),
                        ResourceClass::BackendStateBytes,
                        u64::try_from(bytes).map_err(HostError::backend)?,
                    )
                    .map_err(|error| HostError::ResourceExhausted(error.to_string()))?,
            );
        }
        Ok(())
    }

    pub fn run(
        self,
        future: impl Future<Output = Result<T, HostError>>,
    ) -> impl Future<Output = FfiTicket<T>> {
        ExecutionWork {
            future,
            execution: self,
        }
        .run()
    }

    /// Execute a bounded synchronous foreign-runtime read while retaining the
    /// same admission, shutdown tracking and conversion-time ownership used by
    /// asynchronous calls. The returned ticket must be delivered by the
    /// caller while its foreign runtime is still available.
    pub fn run_sync(self, operation: impl FnOnce() -> Result<T, HostError>) -> FfiTicket<T> {
        let outcome = if self.stopped.is_cancelled() {
            Err(HostError::TargetClosed)
        } else {
            operation()
        };
        self.settle(outcome)
    }

    fn settle(self, outcome: Result<T, HostError>) -> FfiTicket<T> {
        let Self {
            ticket,
            running,
            stopped: _,
        } = self;
        let (discarded, permit) = {
            let mut state = ticket
                .entry
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            #[cfg(feature = "retained-file-input")]
            let revoked = state.revoked;
            #[cfg(not(feature = "retained-file-input"))]
            let revoked = false;
            if let Some(permit) = state.permit.as_mut().filter(|_| !revoked) {
                permit.queued = true;
                permit.budget.0.queued.fetch_add(1, Ordering::Release);
                if matches!(permit.lane, FfiLane::Command) {
                    permit
                        .budget
                        .0
                        .queued_commands
                        .fetch_add(1, Ordering::Release);
                }
                #[cfg(feature = "retained-file-input")]
                {
                    state.phase = preparation::Phase::Queued;
                }
                state.outcome = Some(outcome);
                (None, None)
            } else {
                (Some(outcome), state.permit.take())
            }
        };
        // Native owner Drop may enqueue cleanup or reenter admission. Keep its
        // byte allowance through that Drop and never invoke it under the lock.
        drop(discarded);
        drop(permit);
        drop(running);
        ticket
    }
}

pub struct FfiReservation<T: Send> {
    entry: Arc<Entry<T>>,
}

impl<T: Send> Clone for FfiReservation<T> {
    fn clone(&self) -> Self {
        Self {
            entry: Arc::clone(&self.entry),
        }
    }
}

impl<T: Send> FfiReservation<T> {
    pub fn reserve_additional(&self, bytes: u64) -> Result<(), HostError> {
        if bytes == 0 {
            return Ok(());
        }
        let mut state = self.entry.state.lock().map_err(HostError::backend)?;
        #[cfg(feature = "retained-file-input")]
        if state.revoked {
            return Err(HostError::TargetClosed);
        }
        let permit = state.permit.as_mut().ok_or(HostError::TargetClosed)?;
        let lease = permit
            .budget
            .0
            .resources
            .reserve(
                ResourceScope::extension(122, 1),
                ResourceClass::BackendStateBytes,
                bytes,
            )
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        permit.additional.push(lease);
        Ok(())
    }
}

// Field order covers even an unpolled run future: its single captured owner
// destroys the user's future before releasing that future's admission/token.
struct ExecutionWork<T: Send, F> {
    future: F,
    execution: FfiExecution<T>,
}

impl<T: Send, F: Future<Output = Result<T, HostError>>> ExecutionWork<T, F> {
    async fn run(self) -> FfiTicket<T> {
        let Self { future, execution } = self;
        #[cfg(feature = "retained-file-input")]
        let outcome = {
            // This lexical holder owns the actual future, not just a Pin
            // reference. Drop it before exposing a settled result or releasing
            // a cancelled phase's permit. A revoker never drops Running storage.
            tokio::pin!(future);
            tokio::select! {
                biased;
                () = execution.stopped.cancelled() => Err(HostError::TargetClosed),
                () = execution.ticket.entry.cancelled.cancelled() => Err(HostError::TargetClosed),
                result = &mut future => result,
            }
        };
        #[cfg(not(feature = "retained-file-input"))]
        let outcome = tokio::select! {
            biased;
            () = execution.stopped.cancelled() => Err(HostError::TargetClosed),
            result = future => result,
        };
        execution.settle(outcome)
    }
}

pub struct FfiTicket<T: Send> {
    entry: Arc<Entry<T>>,
}

impl<T: Send> FfiTicket<T> {
    /// The permit remains alive through conversion, including failure. Taking
    /// a ticket reclaimed during shutdown returns closed, never stale data.
    pub fn deliver<R>(
        self,
        convert: impl FnOnce(Result<T, HostError>) -> R,
    ) -> Result<R, HostError> {
        let (outcome, permit) = {
            let mut state = self.entry.state.lock().map_err(HostError::backend)?;
            (
                state.outcome.take().ok_or(HostError::TargetClosed)?,
                state.permit.take(),
            )
        };
        let result = convert(outcome);
        drop(permit);
        Ok(result)
    }
}

impl<T: Send> Drop for FfiTicket<T> {
    fn drop(&mut self) {
        self.entry.reclaim();
    }
}

#[cfg(test)]
mod shutdown_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn budget() -> FfiBudget {
        FfiBudget::new(FfiLimits {
            connects: 1,
            commands: 2,
            closes: 1,
            watches: 1,
            reserved_bytes: 32,
            future_bytes: [16; 4],
        })
        .unwrap()
    }

    #[tokio::test]
    async fn queued_results_remain_charged_through_conversion() {
        // Arrange
        let budget = budget();
        let ticket = budget
            .admit(FfiLane::Command, 16)
            .unwrap()
            .run(async { Ok("result") })
            .await;
        // Act
        let value = ticket
            .deliver(|value| {
                assert_eq!(budget.metrics().unwrap().pending, 1);
                assert_eq!(budget.metrics().unwrap().queued, 1);
                assert_eq!(budget.metrics().unwrap().reserved_bytes, 16);
                value.unwrap()
            })
            .unwrap();
        // Assert
        assert_eq!(value, "result");
        assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    }

    #[tokio::test]
    async fn boxed_future_bytes_stay_reserved_until_conversion() {
        // Arrange
        let budget = budget();
        let mut execution = budget.admit::<()>(FfiLane::Command, 8).unwrap();
        execution.reserve_future(16).unwrap();
        // Act
        let ticket = execution.run(async { Ok(()) }).await;
        // Assert
        assert_eq!(budget.metrics().unwrap().reserved_bytes, 24);
        ticket.deliver(|_| ()).unwrap();
        assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    }

    #[tokio::test]
    async fn conversion_failure_releases_the_still_charged_result() {
        // Arrange
        let budget = budget();
        let ticket = budget
            .admit(FfiLane::Command, 16)
            .unwrap()
            .run(async { Ok("result") })
            .await;
        // Act
        let converted = ticket
            .deliver(|_| {
                assert_eq!(budget.metrics().unwrap().reserved_bytes, 16);
                Err::<(), _>("foreign conversion failed")
            })
            .unwrap();
        // Assert
        assert_eq!(converted, Err("foreign conversion failed"));
        assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    }

    #[test]
    fn synchronous_reads_stay_charged_through_conversion() {
        // Arrange
        let budget = budget();
        let execution = budget.admit(FfiLane::Command, 8).unwrap();

        // Act
        let ticket = execution.run_sync(|| Ok("snapshot"));
        let value = ticket
            .deliver(|value| {
                assert_eq!(budget.metrics().unwrap().pending, 1);
                assert_eq!(budget.metrics().unwrap().queued_commands, 1);
                value.unwrap()
            })
            .unwrap();

        // Assert
        assert_eq!(value, "snapshot");
        assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    }

    #[test]
    fn future_byte_exhaustion_preserves_only_the_existing_reservation() {
        // Arrange
        let budget = budget();
        let mut execution = budget.admit::<()>(FfiLane::Command, 24).unwrap();
        // Act
        let rejected = execution.reserve_future(16);
        // Assert
        assert!(matches!(rejected, Err(HostError::ResourceExhausted(_))));
        assert_eq!(budget.metrics().unwrap().reserved_bytes, 24);
        drop(execution);
        assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    }

    #[test]
    fn oversized_future_is_rejected_before_boxing() {
        // Arrange
        let budget = budget();
        let mut execution = budget.admit::<()>(FfiLane::Command, 8).unwrap();
        // Act
        let rejected = execution.reserve_future(17);
        // Assert
        assert!(matches!(rejected, Err(HostError::ResourceExhausted(_))));
        drop(execution);
        assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    }

    #[test]
    fn result_dependent_bytes_stay_reserved_through_delivery() {
        // Arrange
        let budget = budget();
        let execution = budget.admit::<()>(FfiLane::Command, 8).unwrap();
        let reservation = execution.reservation();

        // Act
        reservation.reserve_additional(16).unwrap();
        let ticket = execution.run_sync(|| Ok(()));

        // Assert
        assert_eq!(budget.metrics().unwrap().reserved_bytes, 24);
        ticket.deliver(|result| result.unwrap()).unwrap();
        assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    }

    #[test]
    fn rejected_result_reservation_keeps_the_original_admission_intact() {
        // Arrange
        let budget = budget();
        let execution = budget.admit::<()>(FfiLane::Command, 24).unwrap();
        let reservation = execution.reservation();

        // Act
        let rejected = reservation.reserve_additional(9);

        // Assert
        assert!(matches!(rejected, Err(HostError::ResourceExhausted(_))));
        assert_eq!(budget.metrics().unwrap().reserved_bytes, 24);
        drop(execution);
        assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    }

    #[tokio::test]
    async fn queued_errors_cannot_bypass_admission() {
        // Arrange
        let budget = budget();
        let a = budget
            .admit::<()>(FfiLane::Command, 4)
            .unwrap()
            .run(async { Err(HostError::TargetClosed) })
            .await;
        let b = budget
            .admit::<()>(FfiLane::Command, 4)
            .unwrap()
            .run(async { Err(HostError::TargetClosed) })
            .await;
        // Act
        let rejected = budget.admit::<()>(FfiLane::Command, 4);
        // Assert
        assert!(matches!(rejected, Err(HostError::ResourceExhausted(_))));
        assert_eq!(budget.metrics().unwrap().queued, 2);
        drop((a, b));
        assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    }

    #[tokio::test]
    async fn notification_settlements_do_not_inflate_queued_command_counts() {
        // Arrange
        let budget = budget();
        let command = budget
            .admit(FfiLane::Command, 4)
            .unwrap()
            .run(async { Ok(()) })
            .await;
        // Act
        let watch = budget
            .admit(FfiLane::Watch, 4)
            .unwrap()
            .run(async { Ok(()) })
            .await;
        // Assert
        assert_eq!(budget.metrics().unwrap().queued, 2);
        assert_eq!(budget.metrics().unwrap().queued_commands, 1);
        drop((command, watch));
        assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    }

    #[test]
    fn byte_rejection_leaves_no_partial_slot_or_task() {
        // Arrange
        let budget = budget();
        // Act
        let rejected = budget.admit::<()>(FfiLane::Command, 33);
        // Assert
        assert!(matches!(rejected, Err(HostError::ResourceExhausted(_))));
        assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    }

    #[test]
    fn close_admission_is_independent_of_saturated_command_slots() {
        // Arrange
        let budget = budget();
        let a = budget.admit::<()>(FfiLane::Command, 1).unwrap();
        let b = budget.admit::<()>(FfiLane::Command, 1).unwrap();
        // Act
        let close = budget.admit::<()>(FfiLane::Close, 1);
        // Assert
        assert!(close.is_ok());
        drop((a, b, close));
        assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    }

    #[tokio::test]
    async fn shutdown_reclaims_values_even_when_foreign_tickets_never_deliver() {
        struct Dropped(Arc<AtomicUsize>);
        impl Drop for Dropped {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        // Arrange
        let budget = budget();
        let drops = Arc::new(AtomicUsize::new(0));
        let value = Dropped(drops.clone());
        let ticket = budget
            .admit(FfiLane::Command, 16)
            .unwrap()
            .run(async { Ok(value) })
            .await;
        // Act
        budget.shutdown().await.unwrap();
        // Assert
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
        assert!(matches!(
            ticket.deliver(|_| ()),
            Err(HostError::TargetClosed)
        ));
        assert!(matches!(
            budget.admit::<()>(FfiLane::Close, 1),
            Err(HostError::TargetClosed)
        ));
    }

    #[tokio::test]
    async fn shutdown_cancels_running_work_without_a_foreign_callback() {
        // Arrange
        let budget = budget();
        let execution = budget.admit::<()>(FfiLane::Command, 16).unwrap();
        let task = tokio::spawn(execution.run(std::future::pending()));
        // Act
        budget.shutdown().await.unwrap();
        // Assert
        let ticket = task.await.unwrap();
        assert!(matches!(
            ticket.deliver(|_| ()),
            Err(HostError::TargetClosed)
        ));
        assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    }
}
