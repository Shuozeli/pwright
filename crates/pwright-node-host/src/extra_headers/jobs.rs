//! Bounded work owned and polled by the existing topology task. Cooperating
//! with an await never cancels/restarts its in-progress topology transaction.
use super::Request;
use crate::HostError;
use futures_util::{FutureExt, StreamExt, future::BoxFuture, stream::FuturesUnordered};
use std::{future::poll_fn, task::Poll};

pub(super) const FUTURE_BYTES: usize = 64 * 1024;
pub(crate) struct Jobs {
    pending: FuturesUnordered<BoxFuture<'static, ()>>,
    completed: std::sync::Arc<tokio::sync::Notify>,
}
impl Jobs {
    /// The selected Installation body has already reserved queue/node storage.
    pub(crate) fn new() -> Self {
        Self {
            pending: FuturesUnordered::new(),
            completed: std::sync::Arc::new(tokio::sync::Notify::new()),
        }
    }
    pub(crate) fn wakeup(&self) -> std::sync::Arc<tokio::sync::Notify> {
        self.completed.clone()
    }
    pub(crate) fn push(&mut self, request: Request) -> Result<(), HostError> {
        self.insert(super::execute(request))
    }
    fn insert(
        &mut self,
        future: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), HostError> {
        if self.pending.len() == super::MAX_REQUESTS
            || std::mem::size_of_val(&future) > FUTURE_BYTES
        {
            return Err(HostError::ResourceExhausted(
                "header driver job storage exhausted".into(),
            ));
        }
        self.pending.push(future.boxed());
        Ok(())
    }
    pub(crate) async fn drive<F: Future>(&mut self, future: F) -> F::Output {
        let mut future = std::pin::pin!(future);
        poll_fn(|cx| {
            while let Poll::Ready(Some(())) = self.pending.poll_next_unpin(cx) {
                // Completed work can leave obsolete-session effects to clear.
                // Wake idle topology reconciliation even without another event.
                self.completed.notify_one();
            }
            future.as_mut().poll(cx)
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::sync::oneshot;

    #[test]
    fn production_header_job_body_fits_its_reserved_slot() {
        // Arrange
        fn bytes<F: Future>(_: fn(Request) -> F) -> usize {
            std::mem::size_of::<F>()
        }
        // Act
        let actual = bytes(super::super::execute);
        // Assert
        assert!(
            actual <= FUTURE_BYTES,
            "job body {actual} exceeds {FUTURE_BYTES}"
        );
    }

    #[tokio::test]
    async fn completed_job_wakes_idle_topology_reconciliation() {
        // Arrange
        let mut jobs = Jobs::new();
        let signal = jobs.wakeup();
        jobs.insert(async {}).unwrap();
        let mut idle = std::pin::pin!(jobs.drive(signal.notified()));
        // Act
        let ready = std::future::poll_fn(|cx| Poll::Ready(idle.as_mut().poll(cx).is_ready())).await;
        // Assert: no subsequent browser event or command is required.
        assert!(ready);
    }

    #[tokio::test]
    async fn cooperating_initialization_can_wait_for_native_capacity_without_stalling_jobs() {
        // Arrange
        let native = std::sync::Arc::new(tokio::sync::Semaphore::new(1));
        let permit = native.clone().acquire_owned().await.unwrap();
        let (receipt, received) = oneshot::channel();
        let mut jobs = Jobs::new();
        jobs.insert(async move {
            received.await.unwrap();
            drop(permit);
        })
        .unwrap();
        let mut initialization = std::pin::pin!(jobs.drive(native.acquire()));
        std::future::poll_fn(|cx| {
            assert!(initialization.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        // Act
        receipt.send(()).unwrap();
        let ready = initialization.await.unwrap();
        // Assert
        assert_eq!(native.available_permits(), 0);
        drop(ready);
        assert_eq!(native.available_permits(), 1);
    }

    struct Held(Arc<AtomicUsize>);
    impl Drop for Held {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::AcqRel);
        }
    }

    #[tokio::test]
    async fn cooperating_work_delivers_early_result_then_retains_compensation_until_drain() {
        // Arrange
        let mut jobs = Jobs::new();
        let active = Arc::new(AtomicUsize::new(1));
        let held = Held(active.clone());
        let (primary, primary_done) = oneshot::channel();
        let (compensation, compensation_done) = oneshot::channel();
        let (settled, settled_done) = oneshot::channel();
        jobs.insert(async move {
            let _held = held;
            let _ = primary.send(());
            let _ = compensation_done.await;
            let _ = settled.send(());
        })
        .unwrap();
        // Act
        jobs.drive(primary_done).await.unwrap();
        // Assert: primary is available even though compensation still holds work.
        assert_eq!(active.load(Ordering::Acquire), 1);
        assert_eq!(jobs.pending.len(), 1);
        compensation.send(()).unwrap();
        jobs.drive(settled_done).await.unwrap();
        assert_eq!(active.load(Ordering::Acquire), 0);
        assert!(jobs.pending.is_empty());
    }

    #[tokio::test]
    async fn job_drop_releases_pending_owners_and_capacity_rejects_before_allocating() {
        // Arrange
        let mut jobs = Jobs::new();
        let active = Arc::new(AtomicUsize::new(super::super::MAX_REQUESTS));
        for _ in 0..super::super::MAX_REQUESTS {
            let held = Held(active.clone());
            jobs.insert(async move {
                let _held = held;
                std::future::pending::<()>().await
            })
            .unwrap();
        }
        // Act
        let excess = jobs.insert(std::future::pending());
        drop(jobs);
        // Assert
        assert!(matches!(excess, Err(HostError::ResourceExhausted(_))));
        assert_eq!(active.load(Ordering::Acquire), 0);
    }
}
