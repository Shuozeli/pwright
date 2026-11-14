//! Clock abstraction for dependency injection in time-dependent code.
//!
//! Production code uses `TokioClock` (real time).
//! Tests can use `FakeClock` to control time without real delays.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::Notify;

fn duration_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// Abstraction over time operations for testability.
#[async_trait]
pub trait Clock: Send + Sync {
    /// Current monotonic instant in milliseconds.
    fn now_ms(&self) -> u64;

    /// Sleep for the given duration.
    async fn sleep(&self, duration: Duration);

    /// Check if a deadline (in ms, from `now_ms`) has passed.
    fn is_past(&self, deadline_ms: u64) -> bool {
        self.now_ms() >= deadline_ms
    }

    /// Compute a deadline from now.
    fn deadline_from_now(&self, timeout: Duration) -> u64 {
        self.now_ms().saturating_add(duration_millis(timeout))
    }
}

/// Real clock using tokio time primitives.
pub struct TokioClock {
    epoch: tokio::time::Instant,
}

impl TokioClock {
    pub fn new() -> Self {
        Self {
            epoch: tokio::time::Instant::now(),
        }
    }
}

impl Default for TokioClock {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Clock for TokioClock {
    fn now_ms(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
    }

    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}

/// Fake clock for deterministic tests. Time only advances when `advance()` is called.
pub struct FakeClock {
    current_ms: AtomicU64,
    changed: Notify,
}

impl FakeClock {
    pub fn new() -> Self {
        Self {
            current_ms: AtomicU64::new(0),
            changed: Notify::new(),
        }
    }

    /// Advance time by the given duration.
    pub fn advance(&self, duration: Duration) {
        let increment = duration_millis(duration);
        let _previous =
            self.current_ms
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                    Some(current.saturating_add(increment))
                });
        self.changed.notify_waiters();
    }

    /// Set the current time to a specific value in milliseconds.
    pub fn set_ms(&self, ms: u64) {
        self.current_ms.store(ms, Ordering::Relaxed);
        self.changed.notify_waiters();
    }
}

impl Default for FakeClock {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Clock for FakeClock {
    fn now_ms(&self) -> u64 {
        self.current_ms.load(Ordering::Relaxed)
    }

    async fn sleep(&self, duration: Duration) {
        let deadline = self.deadline_from_now(duration);
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            // Register the waiter before observing time. Without `enable`, an
            // advance from another thread between the check and the first poll
            // could be lost because `notify_waiters` does not retain a permit.
            changed.as_mut().enable();
            if self.is_past(deadline) {
                return;
            }
            changed.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[tokio::test]
    async fn test_fake_clock_starts_at_zero() {
        let clock = FakeClock::new();
        assert_eq!(clock.now_ms(), 0);
    }

    #[tokio::test]
    async fn test_fake_clock_advance() {
        let clock = FakeClock::new();
        clock.advance(Duration::from_millis(500));
        assert_eq!(clock.now_ms(), 500);
        clock.advance(Duration::from_millis(200));
        assert_eq!(clock.now_ms(), 700);
    }

    #[tokio::test]
    async fn test_fake_clock_deadline() {
        let clock = FakeClock::new();
        let deadline = clock.deadline_from_now(Duration::from_millis(1000));
        assert_eq!(deadline, 1000);
        assert!(!clock.is_past(deadline));

        clock.advance(Duration::from_millis(999));
        assert!(!clock.is_past(deadline));

        clock.advance(Duration::from_millis(1));
        assert!(clock.is_past(deadline));
    }

    #[tokio::test]
    async fn fake_clock_sleep_waits_for_the_requested_manual_advance() {
        // Arrange
        let clock = Arc::new(FakeClock::new());
        let sleeper_clock = clock.clone();

        // Act
        let sleeper = tokio::spawn(async move {
            sleeper_clock.sleep(Duration::from_secs(1)).await;
        });
        tokio::task::yield_now().await;
        clock.advance(Duration::from_millis(999));
        tokio::task::yield_now().await;

        // Assert
        assert!(!sleeper.is_finished());
        clock.advance(Duration::from_millis(1));
        sleeper.await.expect("fake clock sleeper should finish");
    }

    #[test]
    fn deadline_saturates_instead_of_wrapping() {
        // Arrange
        let clock = FakeClock::new();
        clock.set_ms(u64::MAX - 5);

        // Act
        let deadline = clock.deadline_from_now(Duration::from_millis(10));

        // Assert
        assert_eq!(deadline, u64::MAX);
    }

    #[test]
    fn duration_larger_than_u64_milliseconds_saturates() {
        // Arrange
        let clock = FakeClock::new();
        let timeout = Duration::from_secs(u64::MAX);

        // Act
        let deadline = clock.deadline_from_now(timeout);

        // Assert
        assert_eq!(deadline, u64::MAX);
    }

    #[tokio::test]
    async fn test_tokio_clock_advances() {
        let clock = TokioClock::new();
        let t0 = clock.now_ms();
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(clock.now_ms() >= t0);
    }
}
