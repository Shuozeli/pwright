use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::ObjectId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectIdSourceError {
    ZeroFirstId,
    Exhausted,
}

impl Display for ObjectIdSourceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroFirstId => formatter.write_str("object ID source must start above zero"),
            Self::Exhausted => formatter.write_str("object ID source is exhausted"),
        }
    }
}

impl Error for ObjectIdSourceError {}

/// Supplies connection-local object identities without coupling allocation to
/// a runtime, backend, or transport.
pub trait ObjectIdSource: Send + Sync + 'static {
    fn next_object_id(&self) -> Result<ObjectId, ObjectIdSourceError>;
}

/// Monotonic object IDs. Allocated IDs are intentionally never reused, even
/// when the operation that requested one later rolls back.
pub struct AtomicObjectIdSource {
    next: AtomicU64,
}

impl AtomicObjectIdSource {
    pub const fn new(first: u64) -> Result<Self, ObjectIdSourceError> {
        if first == 0 {
            return Err(ObjectIdSourceError::ZeroFirstId);
        }
        Ok(Self {
            next: AtomicU64::new(first),
        })
    }
}

impl ObjectIdSource for AtomicObjectIdSource {
    fn next_object_id(&self) -> Result<ObjectId, ObjectIdSourceError> {
        self.next
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map(ObjectId::new)
            .map_err(|_| ObjectIdSourceError::Exhausted)
    }
}

#[cfg(test)]
mod tests {
    use crate::ObjectId;

    use super::{AtomicObjectIdSource, ObjectIdSource, ObjectIdSourceError};

    #[test]
    fn atomic_source_issues_monotonic_nonzero_ids() {
        // Arrange
        let source = AtomicObjectIdSource::new(2).expect("nonzero start should be valid");

        // Act
        let first = source.next_object_id();
        let second = source.next_object_id();

        // Assert
        assert_eq!(first, Ok(ObjectId::new(2)));
        assert_eq!(second, Ok(ObjectId::new(3)));
    }

    #[test]
    fn atomic_source_rejects_zero_as_the_first_identity() {
        // Arrange / Act
        let result = AtomicObjectIdSource::new(0);

        // Assert
        assert!(matches!(result, Err(ObjectIdSourceError::ZeroFirstId)));
    }

    #[test]
    fn atomic_source_reports_exhaustion_without_wrapping_or_reuse() {
        // Arrange
        let source = AtomicObjectIdSource::new(u64::MAX).expect("maximum is a nonzero start");

        // Act
        let first = source.next_object_id();
        let second = source.next_object_id();

        // Assert
        assert_eq!(first, Err(ObjectIdSourceError::Exhausted));
        assert_eq!(second, Err(ObjectIdSourceError::Exhausted));
    }
}
