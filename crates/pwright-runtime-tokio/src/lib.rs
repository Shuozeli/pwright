//! Optional Tokio timer and bounded cleanup execution for `pwright-runtime`.
//!
//! Installing this crate opts into Tokio. The generic runtime and all portable
//! capability contracts remain executor-independent. Cleanup jobs may opt into
//! an explicit deadline; expiration drops the job so its own cancellation-safe
//! guard can escalate without this adapter knowing backend details. Application
//! roots may also close admission, drain accepted work, report lifecycle
//! events, and explicitly submit reproducible cleanup factories with a finite
//! retry policy.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::num::NonZeroUsize;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pwright_runtime::{
    CleanupExecutor, CleanupFailure, CleanupJob, CleanupReservation, CleanupSubmitError,
    DeadlineScheduler, ReservedCleanupExecutor,
};
use tokio::runtime::Handle;
use tokio::sync::{Notify, OwnedSemaphorePermit, Semaphore, TryAcquireError};

#[derive(Clone, Copy, Debug, Default)]
pub struct TokioDeadlineScheduler;

impl DeadlineScheduler for TokioDeadlineScheduler {
    type Sleep<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

    fn sleep(&self, budget: Duration) -> Option<Self::Sleep<'_>> {
        Some(Box::pin(tokio::time::sleep(budget)))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TokioCleanupMetrics {
    pub capacity: usize,
    pub reserved: usize,
    pub peak_reserved: usize,
    pub in_flight: usize,
    pub peak_in_flight: usize,
    pub accepted: u64,
    pub rejected: u64,
    pub attempts: u64,
    pub retries: u64,
    pub completed: u64,
    pub failed: u64,
    pub timed_out: u64,
    pub cancelled: u64,
    pub drain_timeouts: u64,
    pub reporter_panics: u64,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TokioCleanupJobId(u64);

impl TokioCleanupJobId {
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TokioCleanupEvent {
    JobAccepted {
        job_id: TokioCleanupJobId,
        max_attempts: NonZeroUsize,
    },
    RetryScheduled {
        job_id: TokioCleanupJobId,
        failed_attempt: NonZeroUsize,
        next_attempt: NonZeroUsize,
        delay: Duration,
        failure: CleanupFailure,
    },
    JobCompleted {
        job_id: TokioCleanupJobId,
        attempts: NonZeroUsize,
    },
    JobFailed {
        job_id: TokioCleanupJobId,
        attempts: NonZeroUsize,
        failure: CleanupFailure,
    },
    JobTimedOut {
        job_id: TokioCleanupJobId,
    },
    JobCancelled {
        job_id: TokioCleanupJobId,
    },
    SubmissionRejected {
        error: CleanupSubmitError,
    },
    ShutdownStarted,
    ShutdownDrained,
    ShutdownTimedOut {
        reserved: usize,
        in_flight: usize,
    },
}

/// Receives bounded cleanup lifecycle diagnostics synchronously.
///
/// Implementations must return quickly and must not block on asynchronous I/O.
/// An application that needs asynchronous reporting should perform its own
/// bounded, nonblocking admission from this callback. Reporter panics are
/// isolated and counted rather than allowed to orphan accepted cleanup work.
pub trait TokioCleanupReporter: Send + Sync + 'static {
    fn record(&self, event: TokioCleanupEvent);
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DiscardTokioCleanupEvents;

impl TokioCleanupReporter for DiscardTokioCleanupEvents {
    fn record(&self, _event: TokioCleanupEvent) {}
}

impl<Reporter> TokioCleanupReporter for Reporter
where
    Reporter: Fn(TokioCleanupEvent) + Send + Sync + 'static,
{
    fn record(&self, event: TokioCleanupEvent) {
        self(event);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TokioCleanupRetryPolicy {
    max_attempts: NonZeroUsize,
    delay: Duration,
}

impl TokioCleanupRetryPolicy {
    pub const fn new(max_attempts: NonZeroUsize, delay: Duration) -> Self {
        Self {
            max_attempts,
            delay,
        }
    }

    pub const fn max_attempts(self) -> NonZeroUsize {
        self.max_attempts
    }

    pub const fn delay(self) -> Duration {
        self.delay
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TokioCleanupShutdownReport {
    pub already_started: bool,
    pub metrics: TokioCleanupMetrics,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokioCleanupDrainOutcome {
    Drained(TokioCleanupShutdownReport),
    TimedOut(TokioCleanupShutdownReport),
}

#[derive(Clone)]
pub struct TokioCleanupExecutor {
    shared: Arc<TokioCleanupState>,
    job_timeout: Option<Duration>,
}

struct TokioCleanupState {
    capacity: usize,
    permits: Arc<Semaphore>,
    reserved: AtomicUsize,
    peak_reserved: AtomicUsize,
    in_flight: AtomicUsize,
    peak_in_flight: AtomicUsize,
    next_job_id: AtomicU64,
    accepted: AtomicU64,
    rejected: AtomicU64,
    attempts: AtomicU64,
    retries: AtomicU64,
    completed: AtomicU64,
    failed: AtomicU64,
    timed_out: AtomicU64,
    cancelled: AtomicU64,
    drain_timeouts: AtomicU64,
    reporter_panics: AtomicU64,
    gate: Mutex<TokioCleanupGate>,
    drained: Notify,
    reporter: Arc<dyn TokioCleanupReporter>,
}

struct TokioCleanupGate {
    accepting: bool,
    drained_reported: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokioCleanupPolicyError {
    ZeroJobTimeout,
    ZeroDrainTimeout,
}

impl Display for TokioCleanupPolicyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroJobTimeout => {
                formatter.write_str("Tokio cleanup job timeout must be nonzero")
            }
            Self::ZeroDrainTimeout => {
                formatter.write_str("Tokio cleanup drain timeout must be nonzero")
            }
        }
    }
}

impl Error for TokioCleanupPolicyError {}

struct TokioCleanupAdmission {
    handle: Handle,
    permit: OwnedSemaphorePermit,
}

pub struct TokioCleanupReservation {
    shared: Arc<TokioCleanupState>,
    job_timeout: Option<Duration>,
    admission: Option<TokioCleanupAdmission>,
}

struct TokioCleanupTaskGuard {
    shared: Arc<TokioCleanupState>,
    job_id: TokioCleanupJobId,
    permit: Option<OwnedSemaphorePermit>,
    terminal: bool,
}

struct ManagedCleanupOutcome {
    attempts: NonZeroUsize,
    result: Result<(), CleanupFailure>,
}

type ManagedCleanupJob = Pin<Box<dyn Future<Output = ManagedCleanupOutcome> + Send + 'static>>;

impl TokioCleanupExecutor {
    pub fn new(capacity: usize) -> Self {
        Self::with_reporter(capacity, DiscardTokioCleanupEvents)
    }

    pub fn with_reporter<Reporter>(capacity: usize, reporter: Reporter) -> Self
    where
        Reporter: TokioCleanupReporter,
    {
        Self {
            shared: Arc::new(TokioCleanupState {
                capacity,
                permits: Arc::new(Semaphore::new(capacity)),
                reserved: AtomicUsize::new(0),
                peak_reserved: AtomicUsize::new(0),
                in_flight: AtomicUsize::new(0),
                peak_in_flight: AtomicUsize::new(0),
                next_job_id: AtomicU64::new(1),
                accepted: AtomicU64::new(0),
                rejected: AtomicU64::new(0),
                attempts: AtomicU64::new(0),
                retries: AtomicU64::new(0),
                completed: AtomicU64::new(0),
                failed: AtomicU64::new(0),
                timed_out: AtomicU64::new(0),
                cancelled: AtomicU64::new(0),
                drain_timeouts: AtomicU64::new(0),
                reporter_panics: AtomicU64::new(0),
                gate: Mutex::new(TokioCleanupGate {
                    accepting: true,
                    drained_reported: false,
                }),
                drained: Notify::new(),
                reporter: Arc::new(reporter),
            }),
            job_timeout: None,
        }
    }

    /// Creates an executor whose every submitted job has the same explicit
    /// wall-clock bound. The Tokio runtime is still supplied by the caller.
    pub fn new_with_job_timeout(
        capacity: usize,
        job_timeout: Duration,
    ) -> Result<Self, TokioCleanupPolicyError> {
        if job_timeout.is_zero() {
            return Err(TokioCleanupPolicyError::ZeroJobTimeout);
        }
        Self::new(capacity).with_job_timeout(job_timeout)
    }

    pub fn with_job_timeout(
        mut self,
        job_timeout: Duration,
    ) -> Result<Self, TokioCleanupPolicyError> {
        if job_timeout.is_zero() {
            return Err(TokioCleanupPolicyError::ZeroJobTimeout);
        }
        self.job_timeout = Some(job_timeout);
        Ok(self)
    }

    pub const fn job_timeout(&self) -> Option<Duration> {
        self.job_timeout
    }

    pub fn metrics(&self) -> TokioCleanupMetrics {
        TokioCleanupMetrics {
            capacity: self.shared.capacity,
            reserved: self.shared.reserved.load(Ordering::Relaxed),
            peak_reserved: self.shared.peak_reserved.load(Ordering::Relaxed),
            in_flight: self.shared.in_flight.load(Ordering::Relaxed),
            peak_in_flight: self.shared.peak_in_flight.load(Ordering::Relaxed),
            accepted: self.shared.accepted.load(Ordering::Relaxed),
            rejected: self.shared.rejected.load(Ordering::Relaxed),
            attempts: self.shared.attempts.load(Ordering::Relaxed),
            retries: self.shared.retries.load(Ordering::Relaxed),
            completed: self.shared.completed.load(Ordering::Relaxed),
            failed: self.shared.failed.load(Ordering::Relaxed),
            timed_out: self.shared.timed_out.load(Ordering::Relaxed),
            cancelled: self.shared.cancelled.load(Ordering::Relaxed),
            drain_timeouts: self.shared.drain_timeouts.load(Ordering::Relaxed),
            reporter_panics: self.shared.reporter_panics.load(Ordering::Relaxed),
        }
    }

    pub fn try_submit_with_id(
        &self,
        job: CleanupJob,
    ) -> Result<TokioCleanupJobId, CleanupSubmitError> {
        let admission = self.try_admit_job()?;
        let job_id = self.next_job_id();
        let managed = single_attempt_job(Arc::clone(&self.shared), job);
        self.start(admission, job_id, NonZeroUsize::MIN, managed);
        Ok(job_id)
    }

    /// Submits an explicitly reproducible cleanup operation.
    ///
    /// The factory is called once per attempt and owns the decision that
    /// repeating the operation is safe. Ordinary `CleanupJob` continuations
    /// are never retried implicitly.
    pub fn try_submit_retrying<Factory>(
        &self,
        policy: TokioCleanupRetryPolicy,
        factory: Factory,
    ) -> Result<TokioCleanupJobId, CleanupSubmitError>
    where
        Factory: FnMut(NonZeroUsize) -> CleanupJob + Send + 'static,
    {
        let admission = self.try_admit_job()?;
        let job_id = self.next_job_id();
        let managed = retrying_job(Arc::clone(&self.shared), job_id, policy, factory);
        self.start(admission, job_id, policy.max_attempts(), managed);
        Ok(job_id)
    }

    /// Stops new cleanup admission and returns whether this call performed the
    /// one-way transition. Existing reservations remain valid and are drained.
    pub fn begin_shutdown(&self) -> bool {
        let (started, drained) = {
            let mut gate = self
                .shared
                .gate
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if !gate.accepting {
                (false, false)
            } else {
                gate.accepting = false;
                let drained = self.mark_drained_locked(&mut gate);
                (true, drained)
            }
        };
        if started {
            report_cleanup_event(&self.shared, TokioCleanupEvent::ShutdownStarted);
        }
        if drained {
            self.report_drained();
        }
        started
    }

    pub async fn shutdown(&self) -> TokioCleanupShutdownReport {
        let already_started = !self.begin_shutdown();
        self.wait_until_drained().await;
        TokioCleanupShutdownReport {
            already_started,
            metrics: self.metrics(),
        }
    }

    pub async fn shutdown_with_timeout(
        &self,
        timeout: Duration,
    ) -> Result<TokioCleanupDrainOutcome, TokioCleanupPolicyError> {
        if timeout.is_zero() {
            return Err(TokioCleanupPolicyError::ZeroDrainTimeout);
        }
        let already_started = !self.begin_shutdown();
        let drained = tokio::time::timeout(timeout, self.wait_until_drained())
            .await
            .is_ok();
        let report = TokioCleanupShutdownReport {
            already_started,
            metrics: self.metrics(),
        };
        if drained {
            Ok(TokioCleanupDrainOutcome::Drained(report))
        } else {
            self.shared.drain_timeouts.fetch_add(1, Ordering::Relaxed);
            let metrics = self.metrics();
            report_cleanup_event(
                &self.shared,
                TokioCleanupEvent::ShutdownTimedOut {
                    reserved: metrics.reserved,
                    in_flight: metrics.in_flight,
                },
            );
            Ok(TokioCleanupDrainOutcome::TimedOut(
                TokioCleanupShutdownReport {
                    already_started,
                    metrics,
                },
            ))
        }
    }

    fn try_admit_job(&self) -> Result<TokioCleanupAdmission, CleanupSubmitError> {
        let mut gate = self.lock_admission_gate()?;
        let admission = match self.try_acquire_admission() {
            Ok(admission) => admission,
            Err(error) => {
                drop(gate);
                self.reject(error);
                return Err(error);
            }
        };
        let in_flight = self.shared.in_flight.fetch_add(1, Ordering::Relaxed) + 1;
        self.shared
            .peak_in_flight
            .fetch_max(in_flight, Ordering::Relaxed);
        self.shared.accepted.fetch_add(1, Ordering::Relaxed);
        gate.drained_reported = false;
        Ok(admission)
    }

    fn try_admit_reservation(&self) -> Result<TokioCleanupAdmission, CleanupSubmitError> {
        let mut gate = self.lock_admission_gate()?;
        let admission = match self.try_acquire_admission() {
            Ok(admission) => admission,
            Err(error) => {
                drop(gate);
                self.reject(error);
                return Err(error);
            }
        };
        let reserved = self.shared.reserved.fetch_add(1, Ordering::Relaxed) + 1;
        self.shared
            .peak_reserved
            .fetch_max(reserved, Ordering::Relaxed);
        gate.drained_reported = false;
        Ok(admission)
    }

    fn lock_admission_gate(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, TokioCleanupGate>, CleanupSubmitError> {
        let gate = self.shared.gate.lock().map_err(|_| {
            self.reject(CleanupSubmitError::StatePoisoned);
            CleanupSubmitError::StatePoisoned
        })?;
        if !gate.accepting {
            drop(gate);
            self.reject(CleanupSubmitError::Unavailable);
            return Err(CleanupSubmitError::Unavailable);
        }
        Ok(gate)
    }

    fn try_acquire_admission(&self) -> Result<TokioCleanupAdmission, CleanupSubmitError> {
        let handle = match Handle::try_current() {
            Ok(handle) => handle,
            Err(_) => return Err(CleanupSubmitError::Unavailable),
        };
        let permit = match Arc::clone(&self.shared.permits).try_acquire_owned() {
            Ok(permit) => permit,
            Err(TryAcquireError::NoPermits) => {
                return Err(CleanupSubmitError::CapacityExceeded {
                    limit: self.shared.capacity,
                });
            }
            Err(TryAcquireError::Closed) => return Err(CleanupSubmitError::Unavailable),
        };
        Ok(TokioCleanupAdmission { handle, permit })
    }

    fn start(
        &self,
        admission: TokioCleanupAdmission,
        job_id: TokioCleanupJobId,
        max_attempts: NonZeroUsize,
        job: ManagedCleanupJob,
    ) {
        report_cleanup_event(
            &self.shared,
            TokioCleanupEvent::JobAccepted {
                job_id,
                max_attempts,
            },
        );
        start_cleanup_job(
            Arc::clone(&self.shared),
            self.job_timeout,
            admission,
            job_id,
            job,
        );
    }

    fn next_job_id(&self) -> TokioCleanupJobId {
        TokioCleanupJobId(self.shared.next_job_id.fetch_add(1, Ordering::Relaxed))
    }

    fn reject(&self, error: CleanupSubmitError) {
        self.shared.rejected.fetch_add(1, Ordering::Relaxed);
        report_cleanup_event(
            &self.shared,
            TokioCleanupEvent::SubmissionRejected { error },
        );
    }

    fn mark_drained_locked(&self, gate: &mut TokioCleanupGate) -> bool {
        if !gate.accepting
            && self.shared.reserved.load(Ordering::Relaxed) == 0
            && self.shared.in_flight.load(Ordering::Relaxed) == 0
            && !gate.drained_reported
        {
            gate.drained_reported = true;
            true
        } else {
            false
        }
    }

    fn report_drained(&self) {
        report_cleanup_event(&self.shared, TokioCleanupEvent::ShutdownDrained);
        self.shared.drained.notify_waiters();
    }

    async fn wait_until_drained(&self) {
        loop {
            let notified = self.shared.drained.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let drained = {
                let mut gate = self
                    .shared
                    .gate
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if self.shared.reserved.load(Ordering::Relaxed) == 0
                    && self.shared.in_flight.load(Ordering::Relaxed) == 0
                {
                    let report = self.mark_drained_locked(&mut gate);
                    drop(gate);
                    if report {
                        self.report_drained();
                    }
                    true
                } else {
                    false
                }
            };
            if drained {
                return;
            }
            notified.await;
        }
    }
}

impl CleanupExecutor for TokioCleanupExecutor {
    fn try_submit(&self, job: CleanupJob) -> Result<(), CleanupSubmitError> {
        self.try_submit_with_id(job).map(|_| ())
    }
}

impl ReservedCleanupExecutor for TokioCleanupExecutor {
    type Reservation = TokioCleanupReservation;

    fn try_reserve(&self) -> Result<Self::Reservation, CleanupSubmitError> {
        let admission = self.try_admit_reservation()?;
        Ok(TokioCleanupReservation {
            shared: Arc::clone(&self.shared),
            job_timeout: self.job_timeout,
            admission: Some(admission),
        })
    }
}

impl CleanupReservation for TokioCleanupReservation {
    fn submit(mut self, job: CleanupJob) {
        let admission = self
            .admission
            .take()
            .expect("Tokio cleanup reservation can be consumed only once");
        let job_id = TokioCleanupJobId(self.shared.next_job_id.fetch_add(1, Ordering::Relaxed));
        {
            let _gate = self
                .shared
                .gate
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            self.shared.reserved.fetch_sub(1, Ordering::Relaxed);
            let in_flight = self.shared.in_flight.fetch_add(1, Ordering::Relaxed) + 1;
            self.shared
                .peak_in_flight
                .fetch_max(in_flight, Ordering::Relaxed);
            self.shared.accepted.fetch_add(1, Ordering::Relaxed);
        }
        report_cleanup_event(
            &self.shared,
            TokioCleanupEvent::JobAccepted {
                job_id,
                max_attempts: NonZeroUsize::MIN,
            },
        );
        start_cleanup_job(
            Arc::clone(&self.shared),
            self.job_timeout,
            admission,
            job_id,
            single_attempt_job(Arc::clone(&self.shared), job),
        );
    }
}

impl Drop for TokioCleanupReservation {
    fn drop(&mut self) {
        if self.admission.is_some() {
            let drained = {
                let mut gate = self
                    .shared
                    .gate
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                self.shared.reserved.fetch_sub(1, Ordering::Relaxed);
                mark_shared_drained_locked(&self.shared, &mut gate)
            };
            self.admission = None;
            if drained {
                report_shared_drained(&self.shared);
            }
        }
    }
}

fn single_attempt_job(shared: Arc<TokioCleanupState>, job: CleanupJob) -> ManagedCleanupJob {
    Box::pin(async move {
        shared.attempts.fetch_add(1, Ordering::Relaxed);
        ManagedCleanupOutcome {
            attempts: NonZeroUsize::MIN,
            result: job.await,
        }
    })
}

fn retrying_job<Factory>(
    shared: Arc<TokioCleanupState>,
    job_id: TokioCleanupJobId,
    policy: TokioCleanupRetryPolicy,
    mut factory: Factory,
) -> ManagedCleanupJob
where
    Factory: FnMut(NonZeroUsize) -> CleanupJob + Send + 'static,
{
    Box::pin(async move {
        let mut attempt = NonZeroUsize::MIN;
        loop {
            shared.attempts.fetch_add(1, Ordering::Relaxed);
            match factory(attempt).await {
                Ok(()) => {
                    return ManagedCleanupOutcome {
                        attempts: attempt,
                        result: Ok(()),
                    };
                }
                Err(failure) if attempt < policy.max_attempts() => {
                    let next_attempt = NonZeroUsize::new(attempt.get() + 1)
                        .expect("a retry before max_attempts cannot overflow");
                    shared.retries.fetch_add(1, Ordering::Relaxed);
                    report_cleanup_event(
                        &shared,
                        TokioCleanupEvent::RetryScheduled {
                            job_id,
                            failed_attempt: attempt,
                            next_attempt,
                            delay: policy.delay(),
                            failure,
                        },
                    );
                    if !policy.delay().is_zero() {
                        tokio::time::sleep(policy.delay()).await;
                    }
                    attempt = next_attempt;
                }
                Err(failure) => {
                    return ManagedCleanupOutcome {
                        attempts: attempt,
                        result: Err(failure),
                    };
                }
            }
        }
    })
}

fn start_cleanup_job(
    shared: Arc<TokioCleanupState>,
    job_timeout: Option<Duration>,
    admission: TokioCleanupAdmission,
    job_id: TokioCleanupJobId,
    job: ManagedCleanupJob,
) {
    let mut guard = TokioCleanupTaskGuard {
        shared,
        job_id,
        permit: Some(admission.permit),
        terminal: false,
    };
    admission.handle.spawn(async move {
        match job_timeout {
            Some(timeout) => match tokio::time::timeout(timeout, job).await {
                Ok(outcome) => guard.finish(outcome),
                Err(_) => guard.timeout(),
            },
            None => guard.finish(job.await),
        }
    });
}

impl TokioCleanupTaskGuard {
    fn finish(&mut self, outcome: ManagedCleanupOutcome) {
        match outcome.result {
            Ok(()) => {
                self.shared.completed.fetch_add(1, Ordering::Relaxed);
                report_cleanup_event(
                    &self.shared,
                    TokioCleanupEvent::JobCompleted {
                        job_id: self.job_id,
                        attempts: outcome.attempts,
                    },
                );
            }
            Err(failure) => {
                self.shared.failed.fetch_add(1, Ordering::Relaxed);
                report_cleanup_event(
                    &self.shared,
                    TokioCleanupEvent::JobFailed {
                        job_id: self.job_id,
                        attempts: outcome.attempts,
                        failure,
                    },
                );
            }
        }
        self.complete();
    }

    fn timeout(&mut self) {
        self.shared.timed_out.fetch_add(1, Ordering::Relaxed);
        report_cleanup_event(
            &self.shared,
            TokioCleanupEvent::JobTimedOut {
                job_id: self.job_id,
            },
        );
        self.complete();
    }

    fn complete(&mut self) {
        self.terminal = true;
        let drained = {
            let mut gate = self
                .shared
                .gate
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            self.shared.in_flight.fetch_sub(1, Ordering::Relaxed);
            mark_shared_drained_locked(&self.shared, &mut gate)
        };
        self.permit = None;
        if drained {
            report_shared_drained(&self.shared);
        }
    }
}

impl Drop for TokioCleanupTaskGuard {
    fn drop(&mut self) {
        if self.terminal {
            return;
        }
        self.shared.cancelled.fetch_add(1, Ordering::Relaxed);
        report_cleanup_event(
            &self.shared,
            TokioCleanupEvent::JobCancelled {
                job_id: self.job_id,
            },
        );
        let drained = {
            let mut gate = self
                .shared
                .gate
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            self.shared.in_flight.fetch_sub(1, Ordering::Relaxed);
            mark_shared_drained_locked(&self.shared, &mut gate)
        };
        if drained {
            report_shared_drained(&self.shared);
        }
    }
}

fn mark_shared_drained_locked(shared: &TokioCleanupState, gate: &mut TokioCleanupGate) -> bool {
    if !gate.accepting
        && shared.reserved.load(Ordering::Relaxed) == 0
        && shared.in_flight.load(Ordering::Relaxed) == 0
        && !gate.drained_reported
    {
        gate.drained_reported = true;
        true
    } else {
        false
    }
}

fn report_shared_drained(shared: &TokioCleanupState) {
    report_cleanup_event(shared, TokioCleanupEvent::ShutdownDrained);
    shared.drained.notify_waiters();
}

fn report_cleanup_event(shared: &TokioCleanupState, event: TokioCleanupEvent) {
    if catch_unwind(AssertUnwindSafe(|| shared.reporter.record(event))).is_err() {
        shared.reporter_panics.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use std::future::{Pending, Ready, pending, ready};
    use std::num::NonZeroUsize;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Waker};
    use std::time::Duration;

    use pwright_core::{
        CallId, CallMetadata, CancellationReason, Command, CommandCancellation, CommandSpec,
        FailureKind, MethodTag, ObjectId, ObjectKindTag, OperationContext, OperationError, Service,
    };
    use pwright_runtime::{
        CleanupExecutor, CleanupFailure, CleanupReservation, CleanupSubmitError,
        ControlledDispatcher, ReservedCleanupExecutor, RuntimeLimits,
    };

    use super::{
        TokioCleanupDrainOutcome, TokioCleanupEvent, TokioCleanupExecutor, TokioCleanupMetrics,
        TokioCleanupPolicyError, TokioCleanupRetryPolicy, TokioDeadlineScheduler,
    };

    struct DropSignal(Arc<AtomicBool>);

    impl Drop for DropSignal {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    struct TestCommand;

    impl CommandSpec for TestCommand {
        const METHOD: MethodTag = MethodTag::new(1);
        type Params = ();
        type Result = ();
        type ErrorDetails = ();
    }

    struct PendingService;

    impl Service<()> for PendingService {
        type Response = ();
        type Error = OperationError<()>;
        type Future<'a> = Pending<Result<(), OperationError<()>>>;

        fn call(&self, _request: ()) -> Self::Future<'_> {
            pending()
        }
    }

    struct SuccessfulCancellation;

    impl CommandCancellation<TestCommand> for SuccessfulCancellation {
        type Future<'a> = Ready<Result<(), OperationError<()>>>;

        fn cancel(
            &self,
            _context: OperationContext,
            _reason: CancellationReason,
        ) -> Self::Future<'_> {
            ready(Ok(()))
        }
    }

    struct CountingCancellation {
        calls: Arc<AtomicUsize>,
    }

    impl CommandCancellation<TestCommand> for CountingCancellation {
        type Future<'a> = Ready<Result<(), OperationError<()>>>;

        fn cancel(
            &self,
            _context: OperationContext,
            _reason: CancellationReason,
        ) -> Self::Future<'_> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            ready(Ok(()))
        }
    }

    #[tokio::test(start_paused = true)]
    async fn tokio_scheduler_drives_a_controlled_dispatch_deadline() {
        // Arrange
        let dispatcher = ControlledDispatcher::new(
            PendingService,
            SuccessfulCancellation,
            TokioDeadlineScheduler,
            RuntimeLimits::new(1, 1),
        );
        let page_id = ObjectId::new(2);
        dispatcher
            .register_object(page_id, None, ObjectKindTag::new(3))
            .expect("target should register");
        let command = Command::<TestCommand>::new(
            CallId::new(5),
            page_id,
            CallMetadata::new(Some(Duration::from_secs(5)), None),
            (),
        );
        let started_at = tokio::time::Instant::now();

        // Act
        let result = dispatcher
            .call(command)
            .await
            .expect("in-process dispatch should be infallible");

        // Assert
        assert_eq!(
            result
                .outcome()
                .as_ref()
                .expect_err("pending operation should time out")
                .kind(),
            FailureKind::Timeout
        );
        assert_eq!(started_at.elapsed(), Duration::from_secs(5));
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[test]
    fn cleanup_executor_rejects_when_no_tokio_runtime_is_active() {
        // Arrange
        let executor = TokioCleanupExecutor::new(1);

        // Act
        let error = executor
            .try_submit(Box::pin(ready(Ok(()))))
            .expect_err("executor must not create a hidden runtime");

        // Assert
        assert_eq!(error, CleanupSubmitError::Unavailable);
        assert_eq!(
            executor.metrics(),
            TokioCleanupMetrics {
                capacity: 1,
                reserved: 0,
                peak_reserved: 0,
                in_flight: 0,
                peak_in_flight: 0,
                accepted: 0,
                rejected: 1,
                attempts: 0,
                retries: 0,
                completed: 0,
                failed: 0,
                timed_out: 0,
                cancelled: 0,
                drain_timeouts: 0,
                reporter_panics: 0,
            }
        );
    }

    #[tokio::test]
    async fn cleanup_executor_enforces_active_job_capacity() {
        // Arrange
        let executor = TokioCleanupExecutor::new(1);
        executor
            .try_submit(Box::pin(pending()))
            .expect("first cleanup should start");

        // Act
        let error = executor
            .try_submit(Box::pin(ready(Ok(()))))
            .expect_err("second cleanup must exceed the active-job limit");

        // Assert
        assert_eq!(error, CleanupSubmitError::CapacityExceeded { limit: 1 });
        assert_eq!(
            executor.metrics(),
            TokioCleanupMetrics {
                capacity: 1,
                reserved: 0,
                peak_reserved: 0,
                in_flight: 1,
                peak_in_flight: 1,
                accepted: 1,
                rejected: 1,
                attempts: 0,
                retries: 0,
                completed: 0,
                failed: 0,
                timed_out: 0,
                cancelled: 0,
                drain_timeouts: 0,
                reporter_panics: 0,
            }
        );
    }

    #[tokio::test]
    async fn cleanup_executor_records_success_and_failure() {
        // Arrange
        let executor = TokioCleanupExecutor::new(2);
        executor
            .try_submit(Box::pin(ready(Ok(()))))
            .expect("successful cleanup should start");
        executor
            .try_submit(Box::pin(ready(Err(CleanupFailure::new("failed")))))
            .expect("failed cleanup should start");

        // Act
        for _ in 0..10 {
            if executor.metrics().in_flight == 0 {
                break;
            }
            tokio::task::yield_now().await;
        }

        // Assert
        assert_eq!(
            executor.metrics(),
            TokioCleanupMetrics {
                capacity: 2,
                reserved: 0,
                peak_reserved: 0,
                in_flight: 0,
                peak_in_flight: 2,
                accepted: 2,
                rejected: 0,
                attempts: 2,
                retries: 0,
                completed: 1,
                failed: 1,
                timed_out: 0,
                cancelled: 0,
                drain_timeouts: 0,
                reporter_panics: 0,
            }
        );
    }

    #[test]
    fn cleanup_timeout_policy_rejects_a_zero_bound() {
        // Arrange / Act
        let result = TokioCleanupExecutor::new_with_job_timeout(1, Duration::ZERO);

        // Assert
        assert!(matches!(
            result,
            Err(TokioCleanupPolicyError::ZeroJobTimeout)
        ));
    }

    #[tokio::test]
    async fn reserved_cleanup_capacity_cannot_be_stolen_and_submits_without_rechecking() {
        // Arrange
        let executor = TokioCleanupExecutor::new(1);
        let reservation = executor
            .try_reserve()
            .expect("one cleanup slot should be reservable");
        assert_eq!(executor.metrics().reserved, 1);

        // Act
        let blocked = executor.try_submit(Box::pin(ready(Ok(()))));
        reservation.submit(Box::pin(ready(Ok(()))));
        for _ in 0..10 {
            if executor.metrics().in_flight == 0 {
                break;
            }
            tokio::task::yield_now().await;
        }

        // Assert
        assert_eq!(
            blocked,
            Err(CleanupSubmitError::CapacityExceeded { limit: 1 })
        );
        let metrics = executor.metrics();
        assert_eq!(metrics.reserved, 0);
        assert_eq!(metrics.peak_reserved, 1);
        assert_eq!(metrics.accepted, 1);
        assert_eq!(metrics.rejected, 1);
        assert_eq!(metrics.completed, 1);
    }

    #[tokio::test]
    async fn reserved_cleanup_drop_releases_its_executor_slot() {
        // Arrange
        let executor = TokioCleanupExecutor::new(1);
        let reservation = executor
            .try_reserve()
            .expect("one cleanup slot should be reservable");

        // Act
        drop(reservation);
        let submitted = executor.try_submit(Box::pin(ready(Ok(()))));
        for _ in 0..10 {
            if executor.metrics().in_flight == 0 {
                break;
            }
            tokio::task::yield_now().await;
        }

        // Assert
        assert!(submitted.is_ok());
        assert_eq!(executor.metrics().reserved, 0);
        assert_eq!(executor.metrics().completed, 1);
    }

    #[tokio::test(start_paused = true)]
    async fn cleanup_deadline_drops_a_stalled_job_and_releases_capacity() {
        // Arrange
        let executor = TokioCleanupExecutor::new_with_job_timeout(1, Duration::from_secs(5))
            .expect("nonzero timeout should validate");
        let dropped = Arc::new(AtomicBool::new(false));
        let dropped_by_job = Arc::clone(&dropped);
        executor
            .try_submit(Box::pin(async move {
                let _drop_signal = DropSignal(dropped_by_job);
                std::future::pending::<Result<(), CleanupFailure>>().await
            }))
            .expect("stalled cleanup should start");
        tokio::task::yield_now().await;
        assert!(!dropped.load(Ordering::SeqCst));

        // Act
        tokio::time::advance(Duration::from_secs(5)).await;
        for _ in 0..10 {
            if executor.metrics().in_flight == 0 {
                break;
            }
            tokio::task::yield_now().await;
        }
        let replacement = executor.try_submit(Box::pin(ready(Ok(()))));
        for _ in 0..10 {
            if executor.metrics().in_flight == 0 {
                break;
            }
            tokio::task::yield_now().await;
        }

        // Assert
        assert!(dropped.load(Ordering::SeqCst));
        assert!(replacement.is_ok());
        let metrics = executor.metrics();
        assert_eq!(metrics.in_flight, 0);
        assert_eq!(metrics.timed_out, 1);
        assert_eq!(metrics.cancelled, 0);
        assert_eq!(metrics.completed, 1);
        assert_eq!(metrics.failed, 0);
    }

    #[tokio::test]
    async fn shutdown_rejects_new_work_but_drains_a_preaccepted_reservation() {
        // Arrange
        let events = Arc::new(Mutex::new(Vec::new()));
        let reported = Arc::clone(&events);
        let executor = TokioCleanupExecutor::with_reporter(1, move |event| {
            reported.lock().expect("events should lock").push(event);
        });
        let reservation = executor
            .try_reserve()
            .expect("reservation should be accepted before shutdown");

        // Act
        assert!(executor.begin_shutdown());
        let late = executor.try_submit(Box::pin(ready(Ok(()))));
        let draining_executor = executor.clone();
        let drain = tokio::spawn(async move { draining_executor.shutdown().await });
        tokio::task::yield_now().await;
        assert!(!drain.is_finished());
        reservation.submit(Box::pin(ready(Ok(()))));
        let report = drain.await.expect("drain task should join");

        // Assert
        assert_eq!(late, Err(CleanupSubmitError::Unavailable));
        assert!(report.already_started);
        assert_eq!(report.metrics.reserved, 0);
        assert_eq!(report.metrics.in_flight, 0);
        assert_eq!(report.metrics.accepted, 1);
        assert_eq!(report.metrics.rejected, 1);
        assert_eq!(report.metrics.completed, 1);
        assert_eq!(report.metrics.attempts, 1);
        let events = events.lock().expect("events should lock");
        assert!(matches!(
            events.first(),
            Some(TokioCleanupEvent::ShutdownStarted)
        ));
        assert!(matches!(
            events.last(),
            Some(TokioCleanupEvent::ShutdownDrained)
        ));
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, TokioCleanupEvent::ShutdownDrained))
                .count(),
            1
        );
    }

    #[tokio::test(start_paused = true)]
    async fn explicit_retry_factory_reuses_one_slot_and_reports_each_retry() {
        // Arrange
        let events = Arc::new(Mutex::new(Vec::new()));
        let reported = Arc::clone(&events);
        let executor = TokioCleanupExecutor::with_reporter(1, move |event| {
            reported.lock().expect("events should lock").push(event);
        });
        let attempts = Arc::new(AtomicUsize::new(0));
        let factory_attempts = Arc::clone(&attempts);
        let policy = TokioCleanupRetryPolicy::new(
            NonZeroUsize::new(3).expect("three is nonzero"),
            Duration::from_secs(2),
        );
        let job_id = executor
            .try_submit_retrying(policy, move |attempt| {
                factory_attempts.fetch_add(1, Ordering::SeqCst);
                Box::pin(ready(if attempt.get() < 3 {
                    Err(CleanupFailure::new(format!(
                        "attempt {} failed",
                        attempt.get()
                    )))
                } else {
                    Ok(())
                }))
            })
            .expect("retryable job should be admitted");
        tokio::task::yield_now().await;
        assert_eq!(attempts.load(Ordering::SeqCst), 1);

        // Act
        tokio::time::advance(Duration::from_secs(2)).await;
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(2)).await;
        let report = executor.shutdown().await;

        // Assert
        assert!(!report.already_started);
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
        assert_eq!(report.metrics.peak_in_flight, 1);
        assert_eq!(report.metrics.attempts, 3);
        assert_eq!(report.metrics.retries, 2);
        assert_eq!(report.metrics.completed, 1);
        assert_eq!(report.metrics.failed, 0);
        let events = events.lock().expect("events should lock");
        assert!(matches!(
            events.first(),
            Some(TokioCleanupEvent::JobAccepted {
                job_id: accepted_id,
                max_attempts,
            }) if *accepted_id == job_id && max_attempts.get() == 3
        ));
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, TokioCleanupEvent::RetryScheduled { .. }))
                .count(),
            2
        );
        assert!(events.iter().any(|event| matches!(
            event,
            TokioCleanupEvent::JobCompleted {
                job_id: completed_id,
                attempts,
            } if *completed_id == job_id && attempts.get() == 3
        )));
    }

    #[tokio::test]
    async fn retry_factory_stops_at_its_finite_attempt_limit() {
        // Arrange
        let events = Arc::new(Mutex::new(Vec::new()));
        let reported = Arc::clone(&events);
        let executor = TokioCleanupExecutor::with_reporter(1, move |event| {
            reported.lock().expect("events should lock").push(event);
        });
        let policy = TokioCleanupRetryPolicy::new(
            NonZeroUsize::new(2).expect("two is nonzero"),
            Duration::ZERO,
        );
        let job_id = executor
            .try_submit_retrying(policy, |attempt| {
                Box::pin(ready(Err(CleanupFailure::new(format!(
                    "terminal attempt {}",
                    attempt.get()
                )))))
            })
            .expect("retryable job should be admitted");

        // Act
        let report = executor.shutdown().await;

        // Assert
        assert_eq!(report.metrics.attempts, 2);
        assert_eq!(report.metrics.retries, 1);
        assert_eq!(report.metrics.completed, 0);
        assert_eq!(report.metrics.failed, 1);
        assert!(
            events
                .lock()
                .expect("events should lock")
                .iter()
                .any(|event| matches!(
                    event,
                    TokioCleanupEvent::JobFailed {
                        job_id: failed_id,
                        attempts,
                        failure,
                    } if *failed_id == job_id
                        && attempts.get() == 2
                        && failure.message() == "terminal attempt 2"
                ))
        );
    }

    #[tokio::test(start_paused = true)]
    async fn drain_timeout_reports_without_cancelling_then_late_completion_drains_once() {
        // Arrange
        let events = Arc::new(Mutex::new(Vec::new()));
        let reported = Arc::clone(&events);
        let executor = TokioCleanupExecutor::with_reporter(1, move |event| {
            reported.lock().expect("events should lock").push(event);
        });
        let release = Arc::new(tokio::sync::Notify::new());
        let job_release = Arc::clone(&release);
        executor
            .try_submit(Box::pin(async move {
                job_release.notified().await;
                Ok(())
            }))
            .expect("cleanup should start");
        tokio::task::yield_now().await;
        let draining_executor = executor.clone();
        let drain = tokio::spawn(async move {
            draining_executor
                .shutdown_with_timeout(Duration::from_secs(5))
                .await
        });
        tokio::task::yield_now().await;

        // Act
        tokio::time::advance(Duration::from_secs(5)).await;
        let timed_out = drain
            .await
            .expect("drain task should join")
            .expect("timeout policy should validate");
        release.notify_one();
        let final_report = executor.shutdown().await;

        // Assert
        let TokioCleanupDrainOutcome::TimedOut(timeout_report) = timed_out else {
            panic!("pending cleanup should exceed the drain deadline");
        };
        assert!(!timeout_report.already_started);
        assert_eq!(timeout_report.metrics.in_flight, 1);
        assert_eq!(timeout_report.metrics.drain_timeouts, 1);
        assert!(final_report.already_started);
        assert_eq!(final_report.metrics.in_flight, 0);
        assert_eq!(final_report.metrics.completed, 1);
        assert_eq!(final_report.metrics.cancelled, 0);
        let events = events.lock().expect("events should lock");
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, TokioCleanupEvent::ShutdownTimedOut { .. }))
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, TokioCleanupEvent::ShutdownDrained))
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn zero_drain_timeout_does_not_close_admission() {
        // Arrange
        let executor = TokioCleanupExecutor::new(1);

        // Act
        let invalid = executor.shutdown_with_timeout(Duration::ZERO).await;
        let submitted = executor.try_submit(Box::pin(ready(Ok(()))));
        let report = executor.shutdown().await;

        // Assert
        assert_eq!(invalid, Err(TokioCleanupPolicyError::ZeroDrainTimeout));
        assert!(submitted.is_ok());
        assert_eq!(report.metrics.completed, 1);
    }

    #[tokio::test]
    async fn reporter_panic_is_counted_without_orphaning_the_cleanup_job() {
        // Arrange
        let report_calls = Arc::new(AtomicUsize::new(0));
        let callback_calls = Arc::clone(&report_calls);
        let executor = TokioCleanupExecutor::with_reporter(1, move |_event| {
            if callback_calls.fetch_add(1, Ordering::SeqCst) == 0 {
                panic!("application reporter failed");
            }
        });

        // Act
        executor
            .try_submit(Box::pin(ready(Ok(()))))
            .expect("reporter failure must not reject accepted cleanup");
        let report = executor.shutdown().await;

        // Assert
        assert_eq!(report.metrics.completed, 1);
        assert_eq!(report.metrics.in_flight, 0);
        assert_eq!(report.metrics.reporter_panics, 1);
        assert!(report_calls.load(Ordering::SeqCst) >= 4);
    }

    #[test]
    fn runtime_shutdown_drops_active_cleanup_and_records_cancellation() {
        // Arrange
        let executor = TokioCleanupExecutor::new(1);
        let dropped = Arc::new(AtomicBool::new(false));
        let dropped_by_job = Arc::clone(&dropped);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("test runtime should build");
        runtime.block_on(async {
            executor
                .try_submit(Box::pin(async move {
                    let _drop_signal = DropSignal(dropped_by_job);
                    std::future::pending::<Result<(), CleanupFailure>>().await
                }))
                .expect("cleanup should start on the caller runtime");
            tokio::task::yield_now().await;
        });
        assert_eq!(executor.metrics().in_flight, 1);

        // Act
        drop(runtime);
        let replacement_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("replacement test runtime should build");
        let replacement = replacement_runtime.block_on(async {
            let submitted = executor.try_submit(Box::pin(ready(Ok(()))));
            for _ in 0..10 {
                if executor.metrics().in_flight == 0 {
                    break;
                }
                tokio::task::yield_now().await;
            }
            submitted
        });

        // Assert
        assert!(dropped.load(Ordering::SeqCst));
        assert!(replacement.is_ok());
        let metrics = executor.metrics();
        assert_eq!(metrics.in_flight, 0);
        assert_eq!(metrics.cancelled, 1);
        assert_eq!(metrics.timed_out, 0);
        assert_eq!(metrics.completed, 1);
    }

    #[tokio::test]
    async fn dropped_dispatch_cleanup_runs_on_the_injected_tokio_executor() {
        // Arrange
        let cleanup_calls = Arc::new(AtomicUsize::new(0));
        let executor = TokioCleanupExecutor::new(1);
        let dispatcher = ControlledDispatcher::with_cleanup_executor(
            PendingService,
            CountingCancellation {
                calls: Arc::clone(&cleanup_calls),
            },
            TokioDeadlineScheduler,
            RuntimeLimits::new(1, 1),
            executor.clone(),
        );
        let page_id = ObjectId::new(2);
        dispatcher
            .register_object(page_id, None, ObjectKindTag::new(3))
            .expect("target should register");
        let command =
            Command::<TestCommand>::new(CallId::new(5), page_id, CallMetadata::default(), ());
        let mut dispatch = Box::pin(dispatcher.call(command));
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        assert!(matches!(
            dispatch.as_mut().poll(&mut context),
            Poll::Pending
        ));

        // Act
        drop(dispatch);
        for _ in 0..10 {
            if cleanup_calls.load(Ordering::SeqCst) == 1 {
                break;
            }
            tokio::task::yield_now().await;
        }

        // Assert
        assert_eq!(cleanup_calls.load(Ordering::SeqCst), 1);
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
        assert_eq!(executor.metrics().accepted, 1);
        assert_eq!(executor.metrics().completed, 1);
        assert_eq!(executor.metrics().rejected, 0);
    }
}
