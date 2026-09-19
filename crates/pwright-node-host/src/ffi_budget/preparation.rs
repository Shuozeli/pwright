//! Revocable admission before and between execution phases. No foreign roots.

use super::{BudgetInner, Entry, FfiBudget, FfiExecution, FfiLane, FfiTicket, HostError};
use std::sync::{Arc, Weak, atomic::Ordering};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Initial,
    Running,
    Queued,
    Parked,
}

trait Revoke: Send + Sync {
    fn revoke(&self) -> bool;
}

/// Weak, thread-safe cancellation authority for one admitted entry. This owns
/// neither its budget nor its result, and carries no session identity itself.
/// Running work keeps its permit until its future is dropped; idle ownership
/// is reclaimed immediately. The embedding must separately bound/register any
/// session hooks and must release foreign roots on their runtime thread.
#[derive(Clone)]
pub struct FfiRevoker(Weak<dyn Revoke>);

impl FfiRevoker {
    /// Returns true only for the first revocation of a still-admitted entry.
    pub fn revoke(&self) -> bool {
        self.0.upgrade().is_some_and(|entry| entry.revoke())
    }
}

impl<T: Send> Revoke for Entry<T> {
    fn revoke(&self) -> bool {
        let (outcome, permit) = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if state.revoked || state.permit.is_none() {
                return false;
            }
            state.revoked = true;
            if state.phase == Phase::Running {
                (None, None)
            } else {
                (state.outcome.take(), state.permit.take())
            }
        };
        // Cancellation wakes arbitrary Rust futures. Neither wakeups nor owner
        // destructors may run under the entry (or budget registry) lock.
        self.cancelled.cancel();
        drop(outcome);
        drop(permit);
        true
    }
}

/// Move-only storage/slot admission that does not count as running work.
///
/// Holding this value cannot delay [`FfiBudget::shutdown`]: shutdown reclaims
/// its permit and any parked result even when the owner never resumes. A held
/// revoked receipt owns only an empty cell and weak budget identity. A parked
/// result stays in the budget's original entry, not in the foreign runtime.
///
/// This proves only exact budget origin, not session identity. An embedding
/// must independently bind/revoke session authority and bound any foreign
/// roots before allocating them. Foreign roots must be released on their own
/// runtime thread before budget shutdown; none are retained by this type.
#[must_use = "start or drop the reserved preparation"]
pub struct FfiPreparation<T: Send> {
    ticket: FfiTicket<T>,
    origin: Weak<BudgetInner>,
}

impl FfiBudget {
    /// Reserve the same lane and byte allowance as `admit`, but no work token.
    /// Call before allocating bounded foreign-conversion storage or roots.
    ///
    /// Arbitrary foreign waiting must finish before `FfiPreparation::start`;
    /// only promotion registers work that shutdown must join.
    pub fn prepare<T: Send + 'static>(
        &self,
        lane: FfiLane,
        reserved_bytes: u64,
    ) -> Result<FfiPreparation<T>, HostError> {
        let mut registry = self.0.registry.lock().map_err(HostError::backend)?;
        let ticket = self.admit_ticket(&mut registry, lane, reserved_bytes)?;
        Ok(FfiPreparation {
            ticket,
            origin: Arc::downgrade(&self.0),
        })
    }
}

impl<T: Send> FfiPreparation<T> {
    /// Consume this receipt into existing execution without admitting another
    /// slot or byte lease. A foreign budget or revoked receipt fails closed.
    ///
    /// Lock order matches admission/reclamation: registry, then entry. The
    /// close check and task-token registration share shutdown's registry lock,
    /// so no token can appear after a closed-and-empty tracker observation.
    /// After promotion, immediately run or drop the execution; holding its
    /// work token across another arbitrary foreign await would delay shutdown.
    pub fn start(self, budget: &FfiBudget) -> Result<FfiExecution<T>, HostError> {
        let Self { ticket, origin } = self;
        if !Weak::ptr_eq(&origin, &Arc::downgrade(&budget.0)) {
            return Err(HostError::InvalidArgument(
                "FFI preparation belongs to another budget".into(),
            ));
        }
        let registry = budget.0.registry.lock().map_err(HostError::backend)?;
        if registry.closed || budget.0.stopped.is_cancelled() {
            return Err(HostError::TargetClosed);
        }
        let mut state = ticket.entry.state.lock().map_err(HostError::backend)?;
        if state.revoked || state.permit.is_none() {
            return Err(HostError::TargetClosed);
        }
        if state.phase != Phase::Initial || state.outcome.is_some() {
            return Err(phase_error());
        }
        let running = budget.0.tasks.token();
        state.phase = Phase::Running;
        drop(state);
        Ok(FfiExecution {
            ticket,
            running,
            stopped: budget.0.stopped.clone(),
        })
    }

    /// Inspect a parked result synchronously without extracting its ownership.
    /// The callback must be pure, non-reentrant Rust: no NAPI, foreign callback,
    /// await or entry/budget operation. References cannot escape this callback.
    /// Drop the registry guard before inspection so a callback panic cannot
    /// poison the global shutdown/admission lock.
    pub fn inspect_parked<R>(
        &self,
        budget: &FfiBudget,
        read: impl FnOnce(&T) -> R,
    ) -> Result<R, HostError> {
        check_origin(&self.origin, budget)?;
        let registry = budget.0.registry.lock().map_err(HostError::backend)?;
        if registry.closed || budget.0.stopped.is_cancelled() {
            return Err(HostError::TargetClosed);
        }
        let state = self.ticket.entry.state.lock().map_err(HostError::backend)?;
        if state.revoked || state.permit.is_none() {
            return Err(HostError::TargetClosed);
        }
        if state.phase != Phase::Parked {
            return Err(phase_error());
        }
        let Some(Ok(value)) = &state.outcome else {
            return Err(phase_error());
        };
        drop(registry);
        Ok(read(value))
    }

    /// Move the parked result into a newly tracked native phase using the SAME
    /// permit. Immediately move BOTH returned values into the next native
    /// future, reserve that body's actual size, then run or drop it. Do not
    /// retain the execution over arbitrary foreign waiting, or drop it while
    /// independently keeping the extracted result beyond its byte admission.
    pub fn resume(self, budget: &FfiBudget) -> Result<(T, FfiExecution<T>), HostError> {
        let Self { ticket, origin } = self;
        check_origin(&origin, budget)?;
        let registry = budget.0.registry.lock().map_err(HostError::backend)?;
        if registry.closed || budget.0.stopped.is_cancelled() {
            return Err(HostError::TargetClosed);
        }
        let mut state = ticket.entry.state.lock().map_err(HostError::backend)?;
        if state.revoked || state.permit.is_none() {
            return Err(HostError::TargetClosed);
        }
        if state.phase != Phase::Parked || !matches!(state.outcome, Some(Ok(_))) {
            return Err(phase_error());
        }
        let running = budget.0.tasks.token();
        state.phase = Phase::Running;
        let value = match state.outcome.take() {
            Some(Ok(value)) => value,
            _ => unreachable!("checked parked result remains under the entry lock"),
        };
        drop(state);
        Ok((
            value,
            FfiExecution {
                ticket,
                running,
                stopped: budget.0.stopped.clone(),
            },
        ))
    }
}

impl<T: Send + 'static> FfiPreparation<T> {
    pub fn revoker(&self) -> FfiRevoker {
        let entry: Arc<dyn Revoke> = self.ticket.entry.clone();
        FfiRevoker(Arc::downgrade(&entry))
    }
}

impl<T: Send> FfiTicket<T> {
    /// Park a successful completed phase in its original entry. The native
    /// future has already been destroyed by `run`; its separate size lease can
    /// now return. The lane/base bytes remain held, without a running token.
    /// Failed, foreign or revoked tickets are consumed and reclaimed.
    pub fn park(self, budget: &FfiBudget) -> Result<FfiPreparation<T>, HostError> {
        let origin = {
            let state = self.entry.state.lock().map_err(HostError::backend)?;
            let permit = state.permit.as_ref().ok_or(HostError::TargetClosed)?;
            Arc::downgrade(&permit.budget.0)
        };
        check_origin(&origin, budget)?;
        let previous_future = {
            let registry = budget.0.registry.lock().map_err(HostError::backend)?;
            if registry.closed || budget.0.stopped.is_cancelled() {
                return Err(HostError::TargetClosed);
            }
            let mut state = self.entry.state.lock().map_err(HostError::backend)?;
            if state.revoked || state.permit.is_none() {
                return Err(HostError::TargetClosed);
            }
            if state.phase != Phase::Queued || state.outcome.is_none() {
                return Err(phase_error());
            }
            if matches!(state.outcome, Some(Err(_))) {
                let Some(Err(error)) = state.outcome.take() else {
                    unreachable!("checked failed result remains under the entry lock")
                };
                return Err(error);
            }
            state.phase = Phase::Parked;
            let permit = state.permit.as_mut().expect("checked active permit");
            permit.queued = false;
            permit.budget.0.queued.fetch_sub(1, Ordering::Release);
            if matches!(permit.lane, FfiLane::Command) {
                permit
                    .budget
                    .0
                    .queued_commands
                    .fetch_sub(1, Ordering::Release);
            }
            permit.future.take()
        };
        drop(previous_future);
        Ok(FfiPreparation {
            ticket: self,
            origin,
        })
    }
}

fn check_origin(origin: &Weak<BudgetInner>, budget: &FfiBudget) -> Result<(), HostError> {
    if Weak::ptr_eq(origin, &Arc::downgrade(&budget.0)) {
        Ok(())
    } else {
        Err(HostError::InvalidArgument(
            "FFI preparation belongs to another budget".into(),
        ))
    }
}

fn phase_error() -> HostError {
    HostError::InvalidArgument("FFI preparation is not at the required execution phase".into())
}

#[cfg(test)]
mod parking_tests;
#[cfg(test)]
mod staged_tests;
#[cfg(test)]
mod tests;
