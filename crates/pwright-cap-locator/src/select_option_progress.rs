//! Invocation-owned, one-bit progress for opt-in select-option timeouts.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// The last completed renderer retry either observed a disabled option or it
/// did not. Clones share only this invocation's single bounded bit.
#[derive(Clone, Debug, Default)]
#[doc(hidden)]
pub struct SelectOptionProgress(Arc<AtomicBool>);

impl SelectOptionProgress {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_retry_option_not_enabled(&self, option_not_enabled: bool) {
        self.0.store(option_not_enabled, Ordering::Release);
    }

    pub fn option_not_enabled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clones_observe_only_their_invocations_last_retry() {
        // Arrange
        let first = SelectOptionProgress::new();
        let first_backend = first.clone();
        let second = SelectOptionProgress::new();

        // Act
        first_backend.record_retry_option_not_enabled(true);

        // Assert
        assert!(first.option_not_enabled());
        assert!(!second.option_not_enabled());
    }

    #[test]
    fn later_retry_clears_stale_disabled_option_progress() {
        // Arrange
        let progress = SelectOptionProgress::new();
        progress.record_retry_option_not_enabled(true);

        // Act
        progress.record_retry_option_not_enabled(false);

        // Assert
        assert!(!progress.option_not_enabled());
    }
}
