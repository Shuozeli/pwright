//! Optional, one-shot cancellation ownership transfer to a live ancestor.
//! Original command metadata, deadline, pending slot and target permit do not move.
use super::*;
use std::sync::Weak;

// Charge the whole selected signal, including its lease and Arc counters. The
// lease follows the final signal reference, not merely PendingCall completion.
pub(super) const SIGNAL_BYTES: u64 =
    (std::mem::size_of::<CancellationSignal>() + 2 * std::mem::size_of::<usize>()) as u64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifetimeHandoffError {
    Invalid(&'static str),
    TargetClosed,
    TransportClosed,
    Cancelled(CancellationReason),
    StatePoisoned,
}
impl Display for LifetimeHandoffError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => f.write_str(message),
            Self::TargetClosed => f.write_str("lifetime handoff object is closed"),
            Self::TransportClosed => f.write_str("lifetime handoff transport is closed"),
            Self::Cancelled(reason) => write!(f, "lifetime handoff call cancelled: {reason:?}"),
            Self::StatePoisoned => f.write_str("lifetime handoff state poisoned"),
        }
    }
}
impl Error for LifetimeHandoffError {}
impl LifetimeHandoffError {
    pub fn operation_error<D>(self) -> OperationError<D> {
        match self {
            Self::Cancelled(reason) => cancellation_error(reason, None),
            error => OperationError::new(
                match error {
                    Self::TargetClosed => FailureKind::TargetClosed,
                    Self::TransportClosed => FailureKind::TransportClosed,
                    _ => FailureKind::Backend,
                },
                error.to_string(),
                None,
            ),
        }
    }
}

/// Caller-budgeted token. A weak store reference prevents a retained backend
/// continuation from creating a runtime/cleanup ownership cycle. The original
/// signal Arc fences call-ID reuse. The selected signal and its one-shot owner
/// have a last-reference byte lease when resources are installed; there is no
/// new registry. Token/provider storage is separately admitted by the caller.
pub struct CallLifetimeHandoff {
    store: Weak<RuntimeStore>,
    signal: Arc<CancellationSignal>,
    call: CallId,
    source: ObjectId,
    ancestor: ObjectId,
}
fn validate(
    state: &RuntimeState,
    call: CallId,
    source: ObjectId,
    ancestor: ObjectId,
) -> Result<&Arc<CancellationSignal>, LifetimeHandoffError> {
    if state.transport_closed {
        return Err(LifetimeHandoffError::TransportClosed);
    }
    let pending = state
        .pending_calls
        .get(call)
        .ok_or(LifetimeHandoffError::Invalid(
            "handoff call is no longer pending",
        ))?;
    if pending.object_id() != source {
        return Err(LifetimeHandoffError::Invalid(
            "handoff source differs from original command target",
        ));
    }
    if let PendingCallState::CancellationRequested(reason) = pending.state() {
        return Err(LifetimeHandoffError::Cancelled(reason));
    }
    if !state.backend_started.contains(&call) {
        return Err(LifetimeHandoffError::Invalid(
            "handoff backend has not started",
        ));
    }
    let signal = state
        .cancellation_signals
        .get(&call)
        .ok_or(LifetimeHandoffError::Invalid(
            "handoff cancellation signal missing",
        ))?;
    if signal.lifetime_owner.get().is_some() {
        return Err(LifetimeHandoffError::Invalid(
            "handoff is already committed",
        ));
    }
    let source_record = state
        .objects
        .get(source)
        .ok_or(LifetimeHandoffError::TargetClosed)?;
    if state.objects.get(ancestor).is_none() {
        return Err(LifetimeHandoffError::TargetClosed);
    }
    let mut parent = source_record.parent();
    while let Some(id) = parent {
        if id == ancestor {
            return Ok(signal);
        }
        parent = state.objects.get(id).and_then(|object| object.parent());
    }
    Err(LifetimeHandoffError::Invalid(
        "handoff destination is not a live strict ancestor",
    ))
}
impl CallLifetimeHandoff {
    /// No allocation, native work, await, deadline reset or object resurrection.
    /// Lifecycle disposal and this transition linearize under the same store lock.
    pub fn commit(&self) -> Result<(), LifetimeHandoffError> {
        let store = self
            .store
            .upgrade()
            .ok_or(LifetimeHandoffError::TransportClosed)?;
        let state = store
            .state
            .lock()
            .map_err(|_| LifetimeHandoffError::StatePoisoned)?;
        let signal = validate(&state, self.call, self.source, self.ancestor)?;
        if !Arc::ptr_eq(signal, &self.signal) {
            return Err(LifetimeHandoffError::Invalid(
                "handoff belongs to an earlier call incarnation",
            ));
        }
        signal
            .lifetime_owner
            .set(self.ancestor)
            .map_err(|_| LifetimeHandoffError::Invalid("handoff is already committed"))
    }
}
impl<S, Cancellation, Deadlines, Cleanup>
    ControlledDispatcher<S, Cancellation, Deadlines, Cleanup>
{
    /// Prepare only after backend admission, before the operation's phase boundary.
    /// The caller budgets token/provider storage and must commit at that boundary.
    pub fn prepare_lifetime_handoff(
        &self,
        call: CallId,
        source: ObjectId,
        ancestor: ObjectId,
    ) -> Result<CallLifetimeHandoff, LifetimeHandoffError> {
        let state = self
            .store
            .state
            .lock()
            .map_err(|_| LifetimeHandoffError::StatePoisoned)?;
        let signal = validate(&state, call, source, ancestor)?.clone();
        Ok(CallLifetimeHandoff {
            store: Arc::downgrade(&self.store),
            signal,
            call,
            source,
            ancestor,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_core::ResourceLimits;
    type Runtime = ControlledDispatcher<(), (), ()>;
    fn fixture() -> (Runtime, PendingCallGuard, Arc<CancellationSignal>) {
        let runtime = Runtime::new((), (), (), RuntimeLimits::new(8, 8));
        runtime
            .store
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .unwrap();
        runtime
            .store
            .register_object(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                ObjectKindTag::new(2),
            )
            .unwrap();
        runtime
            .store
            .register_object(ObjectId::new(3), None, ObjectKindTag::new(1))
            .unwrap();
        let (guard, signal) = runtime
            .store
            .begin_call::<()>(
                CallId::new(1),
                ObjectId::new(2),
                CallMetadata::new(Some(Duration::from_secs(5)), Some(CancellationId::new(8))),
            )
            .unwrap();
        runtime
            .store
            .mark_backend_started::<()>(CallId::new(1))
            .unwrap();
        (runtime, guard, signal)
    }
    fn token(runtime: &Runtime) -> CallLifetimeHandoff {
        runtime
            .prepare_lifetime_handoff(CallId::new(1), ObjectId::new(2), ObjectId::new(1))
            .unwrap()
    }
    #[test]
    fn committed_handoff_survives_source_disposal_but_ancestor_close_cancels() {
        // Arrange
        let (runtime, guard, signal) = fixture();
        let handoff = token(&runtime);
        // Act
        handoff.commit().unwrap();
        runtime.dispose_object(ObjectId::new(2)).unwrap();
        // Assert
        assert!(signal.termination.get().is_none());
        let state = runtime.store.state.lock().unwrap();
        let call = state.pending_calls.get(CallId::new(1)).unwrap();
        assert_eq!(call.object_id(), ObjectId::new(2));
        assert_eq!(call.metadata().timeout, Some(Duration::from_secs(5)));
        assert_eq!(state.call_lifecycle_owner(call), ObjectId::new(1));
        assert!(state.objects.get(ObjectId::new(2)).is_none());
        drop(state);
        runtime.dispose_object(ObjectId::new(1)).unwrap();
        assert_eq!(
            signal.termination.get(),
            Some(&CallTermination::Cancellation(
                CancellationReason::ParentClosed
            ))
        );
        guard.complete::<()>().unwrap();
        assert_eq!(runtime.pending_call_count().unwrap(), 0);
    }
    #[test]
    fn disposal_before_commit_wins_without_resurrecting_the_call() {
        // Arrange
        let (runtime, _guard, signal) = fixture();
        let handoff = token(&runtime);
        runtime.dispose_object(ObjectId::new(2)).unwrap();
        // Act
        let result = handoff.commit();
        // Assert
        assert_eq!(
            result,
            Err(LifetimeHandoffError::Cancelled(
                CancellationReason::ParentClosed
            ))
        );
        assert!(signal.lifetime_owner.get().is_none());
    }
    #[test]
    fn explicit_cancellation_before_commit_cannot_be_overridden() {
        // Arrange
        let (runtime, _guard, signal) = fixture();
        let handoff = token(&runtime);
        runtime
            .store
            .cancel_call(
                CallId::new(1),
                CancellationId::new(8),
                CancellationReason::Caller,
            )
            .unwrap();
        // Act
        let result = handoff.commit();
        // Assert
        assert_eq!(
            result,
            Err(LifetimeHandoffError::Cancelled(CancellationReason::Caller))
        );
        assert!(signal.lifetime_owner.get().is_none());
    }
    #[test]
    fn unrelated_destination_is_rejected_before_selection() {
        // Arrange
        let (runtime, _guard, signal) = fixture();
        // Act
        let result =
            runtime.prepare_lifetime_handoff(CallId::new(1), ObjectId::new(2), ObjectId::new(3));
        // Assert
        assert!(matches!(result, Err(LifetimeHandoffError::Invalid(_))));
        assert!(signal.lifetime_owner.get().is_none());
    }
    #[test]
    fn token_cannot_transfer_a_reused_call_id() {
        // Arrange
        let (runtime, guard, _) = fixture();
        let handoff = token(&runtime);
        guard.complete::<()>().unwrap();
        let (_next, signal) = runtime
            .store
            .begin_call::<()>(CallId::new(1), ObjectId::new(2), CallMetadata::default())
            .unwrap();
        runtime
            .store
            .mark_backend_started::<()>(CallId::new(1))
            .unwrap();
        // Act
        let result = handoff.commit();
        // Assert
        assert!(matches!(result, Err(LifetimeHandoffError::Invalid(_))));
        assert!(signal.lifetime_owner.get().is_none());
    }
    #[test]
    fn token_rechecks_ancestor_after_source_adoption() {
        // Arrange
        let (runtime, _guard, signal) = fixture();
        let handoff = token(&runtime);
        runtime
            .store
            .adopt_object(ObjectId::new(2), ObjectId::new(3))
            .unwrap();
        // Act
        let result = handoff.commit();
        // Assert
        assert!(matches!(result, Err(LifetimeHandoffError::Invalid(_))));
        assert!(signal.lifetime_owner.get().is_none());
    }
    #[test]
    fn second_commit_is_rejected() {
        // Arrange
        let (runtime, _guard, _) = fixture();
        let handoff = token(&runtime);
        handoff.commit().unwrap();
        // Act
        let result = handoff.commit();
        // Assert
        assert!(matches!(result, Err(LifetimeHandoffError::Invalid(_))));
    }
    #[test]
    fn transport_close_still_cancels_transferred_call() {
        // Arrange
        let (runtime, _guard, signal) = fixture();
        token(&runtime).commit().unwrap();
        // Act
        runtime.store.close_transport_and_cancel_calls().unwrap();
        // Assert
        assert_eq!(
            signal.termination.get(),
            Some(&CallTermination::Cancellation(
                CancellationReason::TransportClosed
            ))
        );
    }

    #[test]
    fn prepared_disposal_rechecks_the_committed_lifetime_owner() {
        // Arrange
        let (runtime, _guard, signal) = fixture();
        let handoff = token(&runtime);
        let disposal = runtime.prepare_object_disposal(ObjectId::new(2)).unwrap();
        handoff.commit().unwrap();
        // Act
        disposal.commit().unwrap();
        // Assert
        assert!(signal.termination.get().is_none());
        assert!(!runtime.is_object_live(ObjectId::new(2)).unwrap());
        runtime
            .prepare_object_disposal(ObjectId::new(1))
            .unwrap()
            .commit()
            .unwrap();
        assert!(signal.termination.get().is_some());
    }

    #[test]
    fn lifecycle_batches_and_idle_checks_follow_the_new_owner() {
        for cancel_pending in [false, true] {
            // Arrange
            let (runtime, _guard, signal) = fixture();
            token(&runtime).commit().unwrap();
            // Act
            runtime
                .store
                .apply_lifecycle_batch(
                    &[ControlMessage::Dispose {
                        object_id: ObjectId::new(2),
                    }],
                    runtime.object_kinds,
                    runtime.topology,
                    cancel_pending,
                )
                .unwrap();
            // Assert
            assert!(signal.termination.get().is_none());
            assert!(matches!(
                runtime.store.ensure_subtree_idle(ObjectId::new(1)),
                Err(RuntimeStateError::ObjectHasPendingCalls {
                    pending_calls: 1,
                    ..
                })
            ));
            let disposal = runtime.store.apply_lifecycle_batch(
                &[ControlMessage::Dispose {
                    object_id: ObjectId::new(1),
                }],
                runtime.object_kinds,
                runtime.topology,
                cancel_pending,
            );
            assert_eq!(disposal.is_ok(), cancel_pending);
            assert_eq!(signal.termination.get().is_some(), cancel_pending);
        }
    }

    #[test]
    fn idle_disposal_releases_source_but_refuses_busy_ancestor() {
        // Arrange
        let (runtime, _guard, signal) = fixture();
        token(&runtime).commit().unwrap();
        runtime.store.ensure_subtree_idle(ObjectId::new(2)).unwrap();
        // Act
        runtime
            .store
            .dispose_object_if_idle(ObjectId::new(2))
            .unwrap();
        // Assert
        assert!(signal.termination.get().is_none());
        assert!(matches!(
            runtime.store.dispose_object_if_idle(ObjectId::new(1)),
            Err(RuntimeStateError::ObjectHasPendingCalls {
                pending_calls: 1,
                ..
            })
        ));
    }

    #[test]
    fn typed_interruption_preserves_ancestor_failure_without_interrupting_source() {
        // Arrange
        let (runtime, _guard, signal) = fixture();
        token(&runtime).commit().unwrap();
        let failure = RuntimeCallFailure::new(FailureKind::BrowserCrashed, "Page crashed");
        // Act
        let source = runtime
            .interrupt_subtree_calls(ObjectId::new(2), failure.clone())
            .unwrap();
        let ancestor = runtime
            .interrupt_subtree_calls(ObjectId::new(1), failure)
            .unwrap();
        // Assert
        assert_eq!((source, ancestor), (0, 1));
        assert_eq!(
            signal
                .termination
                .get()
                .unwrap()
                .operation_error::<()>(None)
                .kind(),
            FailureKind::BrowserCrashed
        );
    }

    #[test]
    fn selected_signal_bytes_remain_leased_until_the_last_token_is_dropped() {
        // Arrange
        let limits = RuntimeLimits::new(2, 1);
        let requirements = limits.resource_requirements();
        let mut bounds = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let required = requirements.class(class);
            bounds = bounds.with_class(class, required.global, required.per_scope);
        }
        let ledger = ProductionResourceLedger::new(bounds, requirements).unwrap();
        let runtime = Runtime::new((), (), (), limits)
            .with_production_resources(&ledger)
            .unwrap();
        runtime
            .store
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .unwrap();
        runtime
            .store
            .register_object(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                ObjectKindTag::new(2),
            )
            .unwrap();
        let (guard, signal) = runtime
            .store
            .begin_call::<()>(CallId::new(1), ObjectId::new(2), CallMetadata::default())
            .unwrap();
        runtime
            .store
            .mark_backend_started::<()>(CallId::new(1))
            .unwrap();
        let handoff = token(&runtime);
        drop(signal);
        // Act
        guard.complete::<()>().unwrap();
        // Assert
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::PendingCall)
                .current,
            0
        );
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::BackendStateBytes)
                .current,
            SIGNAL_BYTES
        );
        let rejected = runtime.store.begin_call::<()>(
            CallId::new(2),
            ObjectId::new(2),
            CallMetadata::default(),
        );
        assert_eq!(
            rejected.err().unwrap().kind(),
            FailureKind::ResourceExhausted
        );
        assert_eq!(runtime.pending_call_count().unwrap(), 0);
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::PendingCall)
                .current,
            0
        );
        drop(handoff);
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::BackendStateBytes)
                .current,
            0
        );
        let (next, signal) = runtime
            .store
            .begin_call::<()>(CallId::new(2), ObjectId::new(2), CallMetadata::default())
            .unwrap();
        next.complete::<()>().unwrap();
        drop(signal);
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::BackendStateBytes)
                .current,
            0
        );
        eprintln!("lifetime-handoff signal allowance: {SIGNAL_BYTES} bytes per pending call");
    }
}
