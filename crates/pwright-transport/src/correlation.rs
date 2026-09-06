use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use pwright_core::{CallId, MethodTag, ObjectId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CorrelatedCall {
    call_id: CallId,
    target: ObjectId,
    method: MethodTag,
}

impl CorrelatedCall {
    pub const fn call_id(self) -> CallId {
        self.call_id
    }

    pub const fn target(self) -> ObjectId {
        self.target
    }

    pub const fn method(self) -> MethodTag {
        self.method
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CorrelationError {
    DuplicateCall { call_id: CallId },
    UnknownResult { call_id: CallId },
    CapacityExceeded { limit: usize },
}

impl Display for CorrelationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateCall { call_id } => {
                write!(formatter, "call `{call_id}` is already awaiting a result")
            }
            Self::UnknownResult { call_id } => {
                write!(formatter, "result references unknown call `{call_id}`")
            }
            Self::CapacityExceeded { limit } => {
                write!(formatter, "wire correlation limit of {limit} was reached")
            }
        }
    }
}

impl Error for CorrelationError {}

/// Bounded client-side correlation for commands crossing a wire boundary.
pub struct CallCorrelator {
    limit: usize,
    peak_len: usize,
    calls: BTreeMap<CallId, CorrelatedCall>,
}

impl CallCorrelator {
    pub const fn new(limit: usize) -> Self {
        Self {
            limit,
            peak_len: 0,
            calls: BTreeMap::new(),
        }
    }

    pub fn register(
        &mut self,
        call_id: CallId,
        target: ObjectId,
        method: MethodTag,
    ) -> Result<(), CorrelationError> {
        if self.calls.contains_key(&call_id) {
            return Err(CorrelationError::DuplicateCall { call_id });
        }
        if self.calls.len() >= self.limit {
            return Err(CorrelationError::CapacityExceeded { limit: self.limit });
        }
        self.calls.insert(
            call_id,
            CorrelatedCall {
                call_id,
                target,
                method,
            },
        );
        self.peak_len = self.peak_len.max(self.calls.len());
        Ok(())
    }

    pub fn complete(&mut self, call_id: CallId) -> Result<CorrelatedCall, CorrelationError> {
        self.calls
            .remove(&call_id)
            .ok_or(CorrelationError::UnknownResult { call_id })
    }

    pub fn len(&self) -> usize {
        self.calls.len()
    }

    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }

    pub const fn peak_len(&self) -> usize {
        self.peak_len
    }

    pub fn drain(&mut self) -> Vec<CorrelatedCall> {
        std::mem::take(&mut self.calls).into_values().collect()
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::{CallId, MethodTag, ObjectId};

    use super::{CallCorrelator, CorrelationError};

    #[test]
    fn correlator_returns_the_registered_typed_context() {
        // Arrange
        let mut correlator = CallCorrelator::new(1);
        correlator
            .register(CallId::new(1), ObjectId::new(2), MethodTag::new(3))
            .expect("call should register");

        // Act
        let call = correlator
            .complete(CallId::new(1))
            .expect("known result should correlate");

        // Assert
        assert_eq!(call.call_id(), CallId::new(1));
        assert_eq!(call.target(), ObjectId::new(2));
        assert_eq!(call.method(), MethodTag::new(3));
        assert!(correlator.is_empty());
        assert_eq!(correlator.peak_len(), 1);
    }

    #[test]
    fn correlator_rejects_duplicate_and_capacity_exhaustion() {
        // Arrange
        let mut correlator = CallCorrelator::new(1);
        correlator
            .register(CallId::new(1), ObjectId::new(2), MethodTag::new(3))
            .expect("first call should register");

        // Act
        let duplicate = correlator
            .register(CallId::new(1), ObjectId::new(9), MethodTag::new(9))
            .expect_err("duplicate call should fail");
        let exhausted = correlator
            .register(CallId::new(2), ObjectId::new(2), MethodTag::new(3))
            .expect_err("second call should exceed capacity");

        // Assert
        assert_eq!(
            duplicate,
            CorrelationError::DuplicateCall {
                call_id: CallId::new(1)
            }
        );
        assert_eq!(exhausted, CorrelationError::CapacityExceeded { limit: 1 });
        assert_eq!(correlator.len(), 1);
    }

    #[test]
    fn correlator_rejects_unknown_result() {
        // Arrange
        let mut correlator = CallCorrelator::new(1);

        // Act
        let error = correlator
            .complete(CallId::new(99))
            .expect_err("unknown result should fail");

        // Assert
        assert_eq!(
            error,
            CorrelationError::UnknownResult {
                call_id: CallId::new(99)
            }
        );
    }

    #[test]
    fn transport_close_drain_is_deterministic_by_call_id() {
        // Arrange
        let mut correlator = CallCorrelator::new(3);
        for call_id in [3, 1, 2] {
            correlator
                .register(CallId::new(call_id), ObjectId::new(1), MethodTag::new(1))
                .expect("call should register");
        }

        // Act
        let drained = correlator.drain();

        // Assert
        assert_eq!(
            drained
                .into_iter()
                .map(|call| call.call_id())
                .collect::<Vec<_>>(),
            vec![CallId::new(1), CallId::new(2), CallId::new(3)]
        );
        assert!(correlator.is_empty());
    }
}
