use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::atomic::{AtomicU64, Ordering};

use pwright_core::CallId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CallIdSourceError;

impl Display for CallIdSourceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("call ID source is exhausted")
    }
}

impl Error for CallIdSourceError {}

pub trait CallIdSource: Send + Sync + 'static {
    fn next_call_id(&self) -> Result<CallId, CallIdSourceError>;
}

/// Monotonic, connection-local call IDs without random or global state.
pub struct AtomicCallIdSource {
    next: AtomicU64,
}

impl AtomicCallIdSource {
    pub const fn new(first: u64) -> Self {
        Self {
            next: AtomicU64::new(first),
        }
    }
}

impl CallIdSource for AtomicCallIdSource {
    fn next_call_id(&self) -> Result<CallId, CallIdSourceError> {
        self.next
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map(CallId::new)
            .map_err(|_| CallIdSourceError)
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::CallId;

    use super::{AtomicCallIdSource, CallIdSource, CallIdSourceError};

    #[test]
    fn atomic_source_issues_monotonic_connection_local_ids() {
        // Arrange
        let source = AtomicCallIdSource::new(7);

        // Act
        let first = source.next_call_id();
        let second = source.next_call_id();

        // Assert
        assert_eq!(first, Ok(CallId::new(7)));
        assert_eq!(second, Ok(CallId::new(8)));
    }

    #[test]
    fn atomic_source_reports_exhaustion_without_wrapping() {
        // Arrange
        let source = AtomicCallIdSource::new(u64::MAX);

        // Act
        let first = source.next_call_id();
        let second = source.next_call_id();

        // Assert
        assert_eq!(first, Err(CallIdSourceError));
        assert_eq!(second, Err(CallIdSourceError));
    }
}
