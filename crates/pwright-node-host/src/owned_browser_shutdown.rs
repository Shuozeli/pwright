//! One-flight, cancellation-safe shutdown for an owned browser process.
//!
//! The first caller transfers the unique process authority into a Tokio task.
//! Later callers await the same retained outcome rather than mistaking an
//! already-taken lease for completed shutdown.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, watch};

type ShutdownFuture<Outcome> = Pin<Box<dyn Future<Output = Outcome> + Send + 'static>>;

pub(super) struct OneFlightShutdown<Outcome> {
    pending: Mutex<Option<ShutdownFuture<Outcome>>>,
    completed: watch::Sender<Option<Arc<Outcome>>>,
}

impl<Outcome: Send + Sync + 'static> OneFlightShutdown<Outcome> {
    pub(super) fn new(future: impl Future<Output = Outcome> + Send + 'static) -> Self {
        let (completed, _) = watch::channel(None);
        Self {
            pending: Mutex::new(Some(Box::pin(future))),
            completed,
        }
    }

    pub(super) fn completed(&self) -> Option<Arc<Outcome>> {
        self.completed.borrow().clone()
    }

    pub(super) async fn run(&self, timeout: Duration) -> Result<Arc<Outcome>, &'static str> {
        if let Some(outcome) = self.completed() {
            return Ok(outcome);
        }
        let mut receiver = self.completed.subscribe();
        {
            let mut pending = self.pending.lock().await;
            if let Some(future) = pending.take() {
                let completed = self.completed.clone();
                // The task owns the process lease before any caller can cancel
                // its wait. A dropped caller cannot discard a started shutdown.
                tokio::spawn(async move {
                    completed.send_replace(Some(Arc::new(future.await)));
                });
            }
        }
        tokio::time::timeout(timeout, async move {
            loop {
                if let Some(outcome) = receiver.borrow_and_update().clone() {
                    return Ok(outcome);
                }
                receiver
                    .changed()
                    .await
                    .map_err(|_| "owned Chromium shutdown receipt channel closed")?;
            }
        })
        .await
        .map_err(|_| "owned Chromium shutdown did not produce a receipt before the wait deadline")?
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use tokio::sync::Notify;

    use super::OneFlightShutdown;

    #[tokio::test]
    async fn concurrent_callers_wait_for_one_retained_completion() {
        // Arrange
        let starts = Arc::new(AtomicUsize::new(0));
        let started = Arc::new(Notify::new());
        let finish = Arc::new(Notify::new());
        let shutdown = Arc::new(OneFlightShutdown::new({
            let starts = Arc::clone(&starts);
            let started = Arc::clone(&started);
            let finish = Arc::clone(&finish);
            async move {
                starts.fetch_add(1, Ordering::SeqCst);
                started.notify_one();
                finish.notified().await;
                17_u32
            }
        }));

        // Act
        let first = tokio::spawn({
            let shutdown = Arc::clone(&shutdown);
            async move { shutdown.run(Duration::from_secs(1)).await }
        });
        started.notified().await;
        let second = tokio::spawn({
            let shutdown = Arc::clone(&shutdown);
            async move { shutdown.run(Duration::from_secs(1)).await }
        });
        tokio::task::yield_now().await;

        // Assert
        assert!(!first.is_finished());
        assert!(!second.is_finished());
        assert_eq!(starts.load(Ordering::SeqCst), 1);
        finish.notify_waiters();
        assert_eq!(*first.await.unwrap().unwrap(), 17);
        assert_eq!(*second.await.unwrap().unwrap(), 17);
        assert_eq!(*shutdown.completed().unwrap(), 17);
    }

    #[tokio::test]
    async fn cancelling_first_waiter_does_not_cancel_the_owned_shutdown() {
        // Arrange
        let starts = Arc::new(AtomicUsize::new(0));
        let started = Arc::new(Notify::new());
        let finish = Arc::new(Notify::new());
        let shutdown = Arc::new(OneFlightShutdown::new({
            let starts = Arc::clone(&starts);
            let started = Arc::clone(&started);
            let finish = Arc::clone(&finish);
            async move {
                starts.fetch_add(1, Ordering::SeqCst);
                started.notify_one();
                finish.notified().await;
                23_u32
            }
        }));

        // Act
        let first = tokio::spawn({
            let shutdown = Arc::clone(&shutdown);
            async move { shutdown.run(Duration::from_secs(1)).await }
        });
        started.notified().await;
        first.abort();
        let second = tokio::spawn({
            let shutdown = Arc::clone(&shutdown);
            async move { shutdown.run(Duration::from_secs(1)).await }
        });
        tokio::task::yield_now().await;

        // Assert
        assert!(!second.is_finished());
        assert_eq!(starts.load(Ordering::SeqCst), 1);
        finish.notify_waiters();
        assert_eq!(*second.await.unwrap().unwrap(), 23);
        assert_eq!(*shutdown.completed().unwrap(), 23);
    }
}
