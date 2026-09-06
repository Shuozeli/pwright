use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::{CallId, CallMetadata, CancellationId, CancellationReason, ObjectId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PendingCallState {
    Running,
    CancellationRequested(CancellationReason),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PendingCall {
    call_id: CallId,
    object_id: ObjectId,
    metadata: CallMetadata,
    state: PendingCallState,
}

impl PendingCall {
    pub const fn call_id(&self) -> CallId {
        self.call_id
    }

    pub const fn object_id(&self) -> ObjectId {
        self.object_id
    }

    pub const fn metadata(&self) -> CallMetadata {
        self.metadata
    }

    pub const fn state(&self) -> PendingCallState {
        self.state
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CancellationRequestOutcome {
    Requested,
    AlreadyRequested { original_reason: CancellationReason },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PendingCallError {
    DuplicateCall {
        call_id: CallId,
    },
    MissingCall {
        call_id: CallId,
    },
    CapacityExceeded {
        limit: usize,
    },
    CancellationNotRegistered {
        call_id: CallId,
    },
    CancellationIdMismatch {
        call_id: CallId,
        expected: CancellationId,
        received: CancellationId,
    },
}

impl Display for PendingCallError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateCall { call_id } => {
                write!(formatter, "call `{call_id}` is already pending")
            }
            Self::MissingCall { call_id } => write!(formatter, "call `{call_id}` is not pending"),
            Self::CapacityExceeded { limit } => {
                write!(formatter, "pending call limit of {limit} was reached")
            }
            Self::CancellationNotRegistered { call_id } => {
                write!(formatter, "call `{call_id}` has no cancellation identity")
            }
            Self::CancellationIdMismatch {
                call_id,
                expected,
                received,
            } => write!(
                formatter,
                "call `{call_id}` cancellation identity mismatch: expected `{expected}`, received `{received}`"
            ),
        }
    }
}

impl Error for PendingCallError {}

/// Tracks live calls with an explicit upper bound and cancellation identity.
#[derive(Debug)]
pub struct PendingCallTable {
    limit: usize,
    peak_len: usize,
    calls: BTreeMap<CallId, PendingCall>,
}

impl PendingCallTable {
    pub const fn new(limit: usize) -> Self {
        Self {
            limit,
            peak_len: 0,
            calls: BTreeMap::new(),
        }
    }

    pub const fn limit(&self) -> usize {
        self.limit
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

    pub fn get(&self, call_id: CallId) -> Option<&PendingCall> {
        self.calls.get(&call_id)
    }

    /// Iterates in deterministic call-ID order.
    pub fn iter(&self) -> impl Iterator<Item = &PendingCall> {
        self.calls.values()
    }

    pub fn insert(
        &mut self,
        call_id: CallId,
        object_id: ObjectId,
        metadata: CallMetadata,
    ) -> Result<(), PendingCallError> {
        self.validate_insert(call_id)?;

        self.calls.insert(
            call_id,
            PendingCall {
                call_id,
                object_id,
                metadata,
                state: PendingCallState::Running,
            },
        );
        self.peak_len = self.peak_len.max(self.calls.len());
        Ok(())
    }

    /// Checks call admission without changing the table.
    pub fn validate_insert(&self, call_id: CallId) -> Result<(), PendingCallError> {
        if self.calls.contains_key(&call_id) {
            return Err(PendingCallError::DuplicateCall { call_id });
        }
        if self.calls.len() >= self.limit {
            return Err(PendingCallError::CapacityExceeded { limit: self.limit });
        }
        Ok(())
    }

    pub fn request_cancellation(
        &mut self,
        call_id: CallId,
        cancellation: CancellationId,
        reason: CancellationReason,
    ) -> Result<CancellationRequestOutcome, PendingCallError> {
        let Some(call) = self.calls.get_mut(&call_id) else {
            return Err(PendingCallError::MissingCall { call_id });
        };
        let Some(expected) = call.metadata.cancellation else {
            return Err(PendingCallError::CancellationNotRegistered { call_id });
        };
        if expected != cancellation {
            return Err(PendingCallError::CancellationIdMismatch {
                call_id,
                expected,
                received: cancellation,
            });
        }

        match call.state {
            PendingCallState::Running => {
                call.state = PendingCallState::CancellationRequested(reason);
                Ok(CancellationRequestOutcome::Requested)
            }
            PendingCallState::CancellationRequested(original_reason) => {
                Ok(CancellationRequestOutcome::AlreadyRequested { original_reason })
            }
        }
    }

    /// Records cancellation from a trusted runtime source such as a deadline
    /// or parent disposal. Unlike an external cancel control, this path does
    /// not require a caller-provided cancellation identity.
    pub fn request_runtime_cancellation(
        &mut self,
        call_id: CallId,
        reason: CancellationReason,
    ) -> Result<CancellationRequestOutcome, PendingCallError> {
        let Some(call) = self.calls.get_mut(&call_id) else {
            return Err(PendingCallError::MissingCall { call_id });
        };

        match call.state {
            PendingCallState::Running => {
                call.state = PendingCallState::CancellationRequested(reason);
                Ok(CancellationRequestOutcome::Requested)
            }
            PendingCallState::CancellationRequested(original_reason) => {
                Ok(CancellationRequestOutcome::AlreadyRequested { original_reason })
            }
        }
    }

    pub fn complete(&mut self, call_id: CallId) -> Result<PendingCall, PendingCallError> {
        self.calls
            .remove(&call_id)
            .ok_or(PendingCallError::MissingCall { call_id })
    }

    /// Drains every pending call in deterministic call-ID order.
    pub fn drain(&mut self) -> Vec<PendingCall> {
        let calls = std::mem::take(&mut self.calls);
        calls.into_values().collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::{CallId, CallMetadata, CancellationId, CancellationReason, ObjectId};

    use super::{
        CancellationRequestOutcome, PendingCall, PendingCallError, PendingCallState,
        PendingCallTable,
    };

    fn cancellable_metadata(cancellation: u64) -> CallMetadata {
        CallMetadata::new(None, Some(CancellationId::new(cancellation)))
    }

    #[test]
    fn insert_enforces_the_pending_call_limit() {
        // Arrange
        let mut calls = PendingCallTable::new(1);
        calls
            .insert(CallId::new(1), ObjectId::new(10), CallMetadata::default())
            .expect("first call should fit");

        // Act
        let error = calls
            .insert(CallId::new(2), ObjectId::new(10), CallMetadata::default())
            .expect_err("second call should exceed the limit");

        // Assert
        assert_eq!(error, PendingCallError::CapacityExceeded { limit: 1 });
        assert_eq!(calls.len(), 1);
        assert_eq!(calls.peak_len(), 1);
    }

    #[test]
    fn insert_rejects_a_duplicate_call_without_replacing_it() {
        // Arrange
        let mut calls = PendingCallTable::new(2);
        calls
            .insert(CallId::new(1), ObjectId::new(10), CallMetadata::default())
            .expect("first call should register");

        // Act
        let error = calls
            .insert(CallId::new(1), ObjectId::new(11), CallMetadata::default())
            .expect_err("duplicate call should fail");

        // Assert
        assert_eq!(
            error,
            PendingCallError::DuplicateCall {
                call_id: CallId::new(1),
            }
        );
        assert_eq!(
            calls.get(CallId::new(1)).map(|call| call.object_id()),
            Some(ObjectId::new(10))
        );
    }

    #[test]
    fn cancellation_fails_when_the_call_has_no_cancellation_identity() {
        // Arrange
        let mut calls = PendingCallTable::new(1);
        calls
            .insert(CallId::new(1), ObjectId::new(10), CallMetadata::default())
            .expect("call should register");

        // Act
        let error = calls
            .request_cancellation(
                CallId::new(1),
                CancellationId::new(7),
                CancellationReason::Caller,
            )
            .expect_err("uncancellable call should fail cancellation");

        // Assert
        assert_eq!(
            error,
            PendingCallError::CancellationNotRegistered {
                call_id: CallId::new(1),
            }
        );
        assert_eq!(
            calls.get(CallId::new(1)).map(|call| call.state()),
            Some(PendingCallState::Running)
        );
    }

    #[test]
    fn cancellation_requires_the_registered_identity() {
        // Arrange
        let mut calls = PendingCallTable::new(1);
        calls
            .insert(CallId::new(1), ObjectId::new(10), cancellable_metadata(7))
            .expect("call should register");

        // Act
        let error = calls
            .request_cancellation(
                CallId::new(1),
                CancellationId::new(8),
                CancellationReason::Caller,
            )
            .expect_err("wrong cancellation identity should fail");

        // Assert
        assert_eq!(
            error,
            PendingCallError::CancellationIdMismatch {
                call_id: CallId::new(1),
                expected: CancellationId::new(7),
                received: CancellationId::new(8),
            }
        );
        assert_eq!(
            calls.get(CallId::new(1)).map(|call| call.state()),
            Some(PendingCallState::Running)
        );
    }

    #[test]
    fn repeated_cancellation_preserves_the_first_reason() {
        // Arrange
        let mut calls = PendingCallTable::new(1);
        calls
            .insert(CallId::new(1), ObjectId::new(10), cancellable_metadata(7))
            .expect("call should register");
        calls
            .request_cancellation(
                CallId::new(1),
                CancellationId::new(7),
                CancellationReason::DeadlineExceeded,
            )
            .expect("first cancellation should register");

        // Act
        let outcome = calls
            .request_cancellation(
                CallId::new(1),
                CancellationId::new(7),
                CancellationReason::TransportClosed,
            )
            .expect("repeated cancellation should be idempotent");

        // Assert
        assert_eq!(
            outcome,
            CancellationRequestOutcome::AlreadyRequested {
                original_reason: CancellationReason::DeadlineExceeded,
            }
        );
        assert_eq!(
            calls.get(CallId::new(1)).map(|call| call.state()),
            Some(PendingCallState::CancellationRequested(
                CancellationReason::DeadlineExceeded
            ))
        );
    }

    #[test]
    fn runtime_cancellation_does_not_require_an_external_identity() {
        // Arrange
        let mut calls = PendingCallTable::new(1);
        calls
            .insert(CallId::new(1), ObjectId::new(10), CallMetadata::default())
            .expect("call should register");

        // Act
        let outcome = calls
            .request_runtime_cancellation(CallId::new(1), CancellationReason::DeadlineExceeded)
            .expect("runtime deadline should cancel the call");

        // Assert
        assert_eq!(outcome, CancellationRequestOutcome::Requested);
        assert_eq!(
            calls.get(CallId::new(1)).map(|call| call.state()),
            Some(PendingCallState::CancellationRequested(
                CancellationReason::DeadlineExceeded
            ))
        );
    }

    #[test]
    fn complete_removes_the_call_and_returns_its_terminal_context() {
        // Arrange
        let mut calls = PendingCallTable::new(1);
        calls
            .insert(CallId::new(4), ObjectId::new(12), cancellable_metadata(9))
            .expect("call should register");

        // Act
        let completed = calls
            .complete(CallId::new(4))
            .expect("pending call should complete");

        // Assert
        assert_eq!(completed.call_id(), CallId::new(4));
        assert_eq!(completed.object_id(), ObjectId::new(12));
        assert!(calls.is_empty());
    }

    #[test]
    fn drain_returns_all_calls_in_deterministic_id_order() {
        // Arrange
        let mut calls = PendingCallTable::new(2);
        calls
            .insert(CallId::new(9), ObjectId::new(10), CallMetadata::default())
            .expect("later call should register");
        calls
            .insert(CallId::new(3), ObjectId::new(10), CallMetadata::default())
            .expect("earlier call should register");

        // Act
        let drained = calls.drain();

        // Assert
        assert_eq!(
            drained
                .iter()
                .map(|call| call.call_id())
                .collect::<Vec<_>>(),
            vec![CallId::new(3), CallId::new(9)]
        );
        assert!(calls.is_empty());
    }

    #[test]
    fn iter_exposes_calls_in_deterministic_id_order() {
        // Arrange
        let mut calls = PendingCallTable::new(2);
        calls
            .insert(CallId::new(9), ObjectId::new(10), CallMetadata::default())
            .expect("later call should register");
        calls
            .insert(CallId::new(3), ObjectId::new(11), CallMetadata::default())
            .expect("earlier call should register");

        // Act
        let call_ids = calls.iter().map(PendingCall::call_id).collect::<Vec<_>>();

        // Assert
        assert_eq!(call_ids, vec![CallId::new(3), CallId::new(9)]);
        assert_eq!(calls.len(), 2);
    }
}
