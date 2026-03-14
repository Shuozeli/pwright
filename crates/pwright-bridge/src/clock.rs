//! Clock abstraction for dependency injection in time-dependent code.
//!
//! Production code uses `TokioClock` (real time).
//! Tests can use `FakeClock` to control time without real delays.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;

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
        self.now_ms() + timeout.as_millis() as u64
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
}

impl FakeClock {
    pub fn new() -> Self {
        Self {
            current_ms: AtomicU64::new(0),
        }
    }

    /// Advance time by the given duration.
    pub fn advance(&self, duration: Duration) {
        self.current_ms
            .fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
    }

    /// Set the current time to a specific value in milliseconds.
    pub fn set_ms(&self, ms: u64) {
        self.current_ms.store(ms, Ordering::Relaxed);
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

    async fn sleep(&self, _duration: Duration) {
        // No-op: fake clock doesn't actually sleep.
        // Tests advance time explicitly via `advance()`.
        tokio::task::yield_now().await;
    }
}

#[cfg(test)]
mod tests {
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
    async fn test_fake_clock_sleep_yields() {
        let clock = FakeClock::new();
        // Should not block
        clock.sleep(Duration::from_secs(999)).await;
        // Time didn't advance
        assert_eq!(clock.now_ms(), 0);
    }

    #[tokio::test]
    async fn test_tokio_clock_advances() {
        let clock = TokioClock::new();
        let t0 = clock.now_ms();
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(clock.now_ms() >= t0);
    }
}
