use std::collections::VecDeque;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

/// An owned backend-cleanup operation that can outlive the dropped command
/// future which created it.
pub type CleanupJob = Pin<Box<dyn Future<Output = Result<(), CleanupFailure>> + Send + 'static>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CleanupFailure {
    message: String,
}

impl CleanupFailure {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for CleanupFailure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for CleanupFailure {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CleanupSubmitError {
    CapacityExceeded { limit: usize },
    Unavailable,
    StatePoisoned,
}

impl Display for CleanupSubmitError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded { limit } => {
                write!(formatter, "cleanup queue limit of {limit} was reached")
            }
            Self::Unavailable => formatter.write_str("cleanup executor is unavailable"),
            Self::StatePoisoned => formatter.write_str("cleanup executor state was poisoned"),
        }
    }
}

impl Error for CleanupSubmitError {}

/// Accepts owned cleanup jobs without prescribing how they are executed.
///
/// Implementations must impose a hard bound and return an error on saturation;
/// silently spawning or growing an unbounded queue violates this contract.
pub trait CleanupExecutor: Send + Sync + 'static {
    fn try_submit(&self, job: CleanupJob) -> Result<(), CleanupSubmitError>;
}

/// One pre-admitted cleanup slot that cannot be stolen by later submissions.
pub trait CleanupReservation: Send + 'static {
    fn submit(self, job: CleanupJob);
}

/// Reserves bounded continuation ownership before an asynchronous operation
/// starts an irreversible backend side effect.
pub trait ReservedCleanupExecutor: CleanupExecutor {
    type Reservation: CleanupReservation;

    fn try_reserve(&self) -> Result<Self::Reservation, CleanupSubmitError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CleanupQueueMetrics {
    pub capacity: usize,
    pub reserved: usize,
    pub queued: usize,
    pub peak_queued: usize,
    pub accepted: u64,
    pub rejected: u64,
    pub completed: u64,
    pub failed: u64,
}

#[derive(Clone)]
pub struct CleanupQueue {
    shared: Arc<Mutex<CleanupQueueState>>,
}

struct CleanupQueueState {
    capacity: usize,
    reserved: usize,
    jobs: VecDeque<CleanupJob>,
    peak_queued: usize,
    accepted: u64,
    rejected: u64,
    completed: u64,
    failed: u64,
}

impl CleanupQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            shared: Arc::new(Mutex::new(CleanupQueueState {
                capacity,
                reserved: 0,
                jobs: VecDeque::new(),
                peak_queued: 0,
                accepted: 0,
                rejected: 0,
                completed: 0,
                failed: 0,
            })),
        }
    }

    /// Removes the oldest submitted job for an external executor to poll.
    pub fn try_take(&self) -> Result<Option<CleanupJob>, CleanupSubmitError> {
        Ok(self
            .shared
            .lock()
            .map_err(|_| CleanupSubmitError::StatePoisoned)?
            .jobs
            .pop_front())
    }

    pub fn metrics(&self) -> Result<CleanupQueueMetrics, CleanupSubmitError> {
        let state = self
            .shared
            .lock()
            .map_err(|_| CleanupSubmitError::StatePoisoned)?;
        Ok(CleanupQueueMetrics {
            capacity: state.capacity,
            reserved: state.reserved,
            queued: state.jobs.len(),
            peak_queued: state.peak_queued,
            accepted: state.accepted,
            rejected: state.rejected,
            completed: state.completed,
            failed: state.failed,
        })
    }
}

impl CleanupExecutor for CleanupQueue {
    fn try_submit(&self, job: CleanupJob) -> Result<(), CleanupSubmitError> {
        let mut state = self
            .shared
            .lock()
            .map_err(|_| CleanupSubmitError::StatePoisoned)?;
        if state.jobs.len().saturating_add(state.reserved) >= state.capacity {
            state.rejected = state.rejected.saturating_add(1);
            return Err(CleanupSubmitError::CapacityExceeded {
                limit: state.capacity,
            });
        }

        let tracked = tracked_cleanup_job(job, Arc::clone(&self.shared));
        state.jobs.push_back(tracked);
        state.accepted = state.accepted.saturating_add(1);
        state.peak_queued = state.peak_queued.max(state.jobs.len());
        Ok(())
    }
}

pub struct CleanupQueueReservation {
    shared: Option<Arc<Mutex<CleanupQueueState>>>,
}

impl CleanupReservation for CleanupQueueReservation {
    fn submit(mut self, job: CleanupJob) {
        let shared = self
            .shared
            .take()
            .expect("cleanup reservation can be consumed only once");
        let tracked = tracked_cleanup_job(job, Arc::clone(&shared));
        let mut state = shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        debug_assert!(state.reserved > 0);
        state.reserved = state.reserved.saturating_sub(1);
        state.jobs.push_back(tracked);
        state.accepted = state.accepted.saturating_add(1);
        state.peak_queued = state.peak_queued.max(state.jobs.len());
    }
}

impl Drop for CleanupQueueReservation {
    fn drop(&mut self) {
        let Some(shared) = self.shared.take() else {
            return;
        };
        let mut state = shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        debug_assert!(state.reserved > 0);
        state.reserved = state.reserved.saturating_sub(1);
    }
}

impl ReservedCleanupExecutor for CleanupQueue {
    type Reservation = CleanupQueueReservation;

    fn try_reserve(&self) -> Result<Self::Reservation, CleanupSubmitError> {
        let mut state = self
            .shared
            .lock()
            .map_err(|_| CleanupSubmitError::StatePoisoned)?;
        if state.jobs.len().saturating_add(state.reserved) >= state.capacity {
            state.rejected = state.rejected.saturating_add(1);
            return Err(CleanupSubmitError::CapacityExceeded {
                limit: state.capacity,
            });
        }
        state.reserved = state.reserved.saturating_add(1);
        drop(state);
        Ok(CleanupQueueReservation {
            shared: Some(Arc::clone(&self.shared)),
        })
    }
}

fn tracked_cleanup_job(
    job: CleanupJob,
    completion_state: Arc<Mutex<CleanupQueueState>>,
) -> CleanupJob {
    Box::pin(async move {
        let outcome = job.await;
        let mut state = completion_state
            .lock()
            .map_err(|_| CleanupFailure::new("cleanup queue state was poisoned"))?;
        match outcome {
            Ok(()) => state.completed = state.completed.saturating_add(1),
            Err(_) => state.failed = state.failed.saturating_add(1),
        }
        outcome
    })
}

#[cfg(test)]
mod tests {
    use std::future::{pending, ready};
    use std::task::{Context, Poll, Waker};

    use super::{
        CleanupExecutor, CleanupFailure, CleanupQueue, CleanupQueueMetrics, CleanupReservation,
        CleanupSubmitError, ReservedCleanupExecutor,
    };

    #[test]
    fn queue_preserves_fifo_submission_order() {
        // Arrange
        let queue = CleanupQueue::new(2);
        queue
            .try_submit(Box::pin(ready(Ok(()))))
            .expect("first job should fit");
        queue
            .try_submit(Box::pin(pending()))
            .expect("second job should fit");

        // Act
        let first = queue.try_take().expect("queue should load");
        let second = queue.try_take().expect("queue should load");

        // Assert
        assert!(first.is_some());
        assert!(second.is_some());
        assert!(queue.try_take().expect("queue should load").is_none());
    }

    #[test]
    fn zero_capacity_rejects_without_growing() {
        // Arrange
        let queue = CleanupQueue::new(0);

        // Act
        let error = queue
            .try_submit(Box::pin(ready(Ok(()))))
            .expect_err("zero-capacity queue must reject");

        // Assert
        assert_eq!(error, CleanupSubmitError::CapacityExceeded { limit: 0 });
        assert_eq!(
            queue.metrics(),
            Ok(CleanupQueueMetrics {
                capacity: 0,
                reserved: 0,
                queued: 0,
                peak_queued: 0,
                accepted: 0,
                rejected: 1,
                completed: 0,
                failed: 0,
            })
        );
    }

    #[test]
    fn queue_enforces_its_hard_capacity() {
        // Arrange
        let queue = CleanupQueue::new(1);
        queue
            .try_submit(Box::pin(ready(Ok(()))))
            .expect("first job should fit");

        // Act
        let error = queue
            .try_submit(Box::pin(ready(Ok(()))))
            .expect_err("second job should exceed capacity");

        // Assert
        assert_eq!(error, CleanupSubmitError::CapacityExceeded { limit: 1 });
        assert_eq!(
            queue.metrics(),
            Ok(CleanupQueueMetrics {
                capacity: 1,
                reserved: 0,
                queued: 1,
                peak_queued: 1,
                accepted: 1,
                rejected: 1,
                completed: 0,
                failed: 0,
            })
        );
    }

    #[test]
    fn reservation_cannot_be_stolen_and_drop_releases_its_capacity() {
        // Arrange
        let queue = CleanupQueue::new(1);
        let reservation = queue
            .try_reserve()
            .expect("first continuation should reserve capacity");

        // Act
        let blocked = queue.try_submit(Box::pin(ready(Ok(()))));
        let while_reserved = queue.metrics().expect("metrics should load");
        drop(reservation);
        let after_drop = queue.try_submit(Box::pin(ready(Ok(()))));

        // Assert
        assert_eq!(
            blocked,
            Err(CleanupSubmitError::CapacityExceeded { limit: 1 })
        );
        assert_eq!(while_reserved.reserved, 1);
        assert_eq!(while_reserved.queued, 0);
        assert!(after_drop.is_ok());
        assert_eq!(queue.metrics().expect("metrics should load").reserved, 0);
    }

    #[test]
    fn admitted_reservation_transfers_to_a_job_without_a_second_capacity_check() {
        // Arrange
        let queue = CleanupQueue::new(1);
        let reservation = queue
            .try_reserve()
            .expect("continuation should reserve capacity");

        // Act
        reservation.submit(Box::pin(pending()));
        let metrics = queue.metrics().expect("metrics should load");

        // Assert
        assert_eq!(metrics.reserved, 0);
        assert_eq!(metrics.queued, 1);
        assert_eq!(metrics.accepted, 1);
        assert!(queue.try_take().expect("queue should load").is_some());
    }

    #[test]
    fn completion_metrics_distinguish_success_and_failure() {
        // Arrange
        let queue = CleanupQueue::new(2);
        queue
            .try_submit(Box::pin(ready(Ok(()))))
            .expect("successful job should fit");
        queue
            .try_submit(Box::pin(ready(Err(CleanupFailure::new("failed")))))
            .expect("failed job should fit");

        // Act
        let mut success = queue
            .try_take()
            .expect("queue should load")
            .expect("first job should exist");
        let mut failed = queue
            .try_take()
            .expect("queue should load")
            .expect("second job should exist");
        let mut context = Context::from_waker(Waker::noop());
        let Poll::Ready(success) = success.as_mut().poll(&mut context) else {
            panic!("ready cleanup should complete");
        };
        let Poll::Ready(failure) = failed.as_mut().poll(&mut context) else {
            panic!("ready cleanup should complete");
        };

        // Assert
        success.expect("first job should succeed");
        let failure = failure.expect_err("second job should fail");
        assert_eq!(failure.message(), "failed");
        assert_eq!(
            queue.metrics(),
            Ok(CleanupQueueMetrics {
                capacity: 2,
                reserved: 0,
                queued: 0,
                peak_queued: 2,
                accepted: 2,
                rejected: 0,
                completed: 1,
                failed: 1,
            })
        );
    }
}
