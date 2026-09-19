//! Thread-affine, explicitly revocable staging. Values never enter Rust tasks.
use super::PreparationProgress;
use pwright_node_host::{
    HostError,
    ffi_budget::{
        FfiBudget, FfiLane, FfiPreparation, FfiRevoker, PublicationPhaseBudget,
        PublicationPhaseStorage, PublicationRegistryStorage,
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

const SESSIONS: usize = 16;
const CALLS: usize = 32;
const ADMIN_BYTES: usize = 16 * 1024;
pub(super) const CALL_BYTES: u64 = 1024 * 1024;

struct Entry<T> {
    id: u64,
    session: u64,
    busy: bool,
    value: Option<T>,
    preparation: Option<FfiPreparation<PreparationProgress>>,
    running: Option<FfiRevoker>,
}
struct State<T> {
    stopped: bool,
    next_id: u64,
    scopes: [Option<Weak<ScopeInner<T>>>; SESSIONS],
    entries: [Option<Entry<T>>; CALLS],
    phase_budget: PublicationPhaseBudget,
    _storage: PublicationRegistryStorage,
}
pub(super) struct Registry<T>(Rc<RefCell<State<T>>>);
impl<T> Clone for Registry<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
struct ScopeInner<T> {
    id: u64,
    closed: Cell<bool>,
    next_protocol_call: Cell<u64>,
    registry: Weak<RefCell<State<T>>>,
}
pub(super) struct Scope<T>(Rc<ScopeInner<T>>);
impl<T> Clone for Scope<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
/// Only a weak, unforgeable session identity and never-reused local key escape.
pub(super) struct Ticket<T> {
    owner: Weak<ScopeInner<T>>,
    id: u64,
}

/// Exclusive phase authority, not a payload, root, FFI permit or session owner.
/// User code may run while this guard is held, but never inside `inspect`.
/// Revocation removes the registry entry even while a phase is outstanding.
/// Separate synchronous scratch stays charged through reentrant revocation;
/// never retain this guard while awaiting a language Promise.
#[must_use = "replace the phase payload or drop to abort the preparation"]
pub(super) struct Phase<T> {
    owner: Weak<ScopeInner<T>>,
    id: u64,
    finished: bool,
    _scratch: PublicationPhaseStorage,
}

/// Node-thread authority spanning exactly one promoted native phase. It owns no
/// payload or FFI permit; dropping it revokes the exact registry entry.
#[cfg(feature = "retained-target-marking")]
#[must_use = "park the promoted FFI ticket or drop to abort the preparation"]
pub(super) struct NativePhase<T> {
    owner: Weak<ScopeInner<T>>,
    id: u64,
    finished: bool,
}

fn closed() -> HostError {
    HostError::TargetClosed
}
fn capacity() -> HostError {
    HostError::ResourceExhausted("Node upload preparation capacity exceeded".into())
}
fn next<T>(state: &mut State<T>) -> Result<u64, HostError> {
    let id = state.next_id.checked_add(1).ok_or_else(capacity)?;
    state.next_id = id;
    Ok(id)
}

impl<T> Registry<T> {
    pub(super) fn new() -> Result<Self, HostError> {
        if std::mem::size_of::<State<T>>()
            + 2 * std::mem::size_of::<usize>()
            + SESSIONS * (std::mem::size_of::<ScopeInner<T>>() + 2 * std::mem::size_of::<usize>())
            + 4096
            > ADMIN_BYTES
        {
            return Err(capacity());
        }
        let storage = PublicationRegistryStorage::reserve(ADMIN_BYTES)?;
        Ok(Self(Rc::new(RefCell::new(State {
            stopped: false,
            next_id: 0,
            scopes: std::array::from_fn(|_| None),
            entries: std::array::from_fn(|_| None),
            phase_budget: PublicationPhaseBudget::new()?,
            _storage: storage,
        }))))
    }

    pub(super) fn scope(&self) -> Result<Scope<T>, HostError> {
        let mut state = self.0.borrow_mut();
        if state.stopped {
            return Err(closed());
        }
        let index = state
            .scopes
            .iter()
            .position(|slot| {
                slot.as_ref()
                    .and_then(Weak::upgrade)
                    .is_none_or(|scope| scope.closed.get())
            })
            .ok_or_else(capacity)?;
        let scope = Rc::new(ScopeInner {
            id: next(&mut state)?,
            closed: Cell::new(false),
            next_protocol_call: Cell::new(0),
            registry: Rc::downgrade(&self.0),
        });
        state.scopes[index] = Some(Rc::downgrade(&scope));
        Ok(Scope(scope))
    }

    /// Seal every scope before any payload/reference destruction can reenter.
    pub(super) fn seal(&self) {
        let mut state = self.0.borrow_mut();
        state.stopped = true;
        for scope in state.scopes.iter().flatten().filter_map(Weak::upgrade) {
            scope.closed.set(true);
        }
    }

    pub(super) fn shutdown(&self) {
        self.seal();
        let entries = {
            let mut state = self.0.borrow_mut();
            std::mem::replace(&mut state.entries, std::array::from_fn(|_| None))
        };
        // Node references and their permits drop with no RefCell borrow held.
        drop(entries);
    }

    pub(super) fn metrics(&self) -> (u32, u32) {
        let state = self.0.borrow();
        let pending = state.entries.iter().flatten().count() as u32;
        let populated = state
            .entries
            .iter()
            .flatten()
            .filter(|entry| entry.value.is_some())
            .count() as u32;
        (pending, populated)
    }

    /// Count direct roots using only pure, non-reentrant Rust inspection.
    /// No language callback or NAPI operation may run inside `count`.
    pub(super) fn rooted_values(&self, mut count: impl FnMut(&T) -> u32) -> Result<u32, HostError> {
        let state = self.0.borrow();
        state
            .entries
            .iter()
            .flatten()
            .filter_map(|entry| entry.value.as_ref())
            .try_fold(0u32, |total, value| {
                total.checked_add(count(value)).ok_or_else(capacity)
            })
    }
}

impl<T> Scope<T> {
    pub(super) fn ensure_live(&self) -> Result<(), HostError> {
        if self.0.closed.get() {
            return Err(closed());
        }
        let registry = self.0.registry.upgrade().ok_or_else(closed)?;
        if registry.borrow().stopped {
            return Err(closed());
        }
        Ok(())
    }

    /// Allocate one connection/session-wide public protocol identity. This is
    /// deliberately independent from preparation IDs and native runtime call
    /// IDs. Gaps after a failed later phase are valid protocol behavior.
    #[cfg(feature = "retained-target-marking")]
    pub(super) fn next_protocol_call(&self) -> Result<u64, HostError> {
        self.ensure_live()?;
        let id = self
            .0
            .next_protocol_call
            .get()
            .checked_add(1)
            .ok_or_else(|| {
                HostError::ResourceExhausted("Protocol call ID space exhausted".into())
            })?;
        self.0.next_protocol_call.set(id);
        Ok(id)
    }

    pub(super) fn admit(&self, budget: &FfiBudget) -> Result<Ticket<T>, HostError> {
        if self.0.closed.get() {
            return Err(closed());
        }
        let registry = self.0.registry.upgrade().ok_or_else(closed)?;
        let mut state = registry.borrow_mut();
        if state.stopped {
            return Err(closed());
        }
        let index = state
            .entries
            .iter()
            .position(Option::is_none)
            .ok_or_else(capacity)?;
        let id = next(&mut state)?;
        let preparation = budget.prepare(FfiLane::Command, CALL_BYTES)?;
        state.entries[index] = Some(Entry {
            id,
            session: self.0.id,
            busy: false,
            value: None,
            preparation: Some(preparation),
            running: None,
        });
        Ok(Ticket {
            owner: Rc::downgrade(&self.0),
            id,
        })
    }

    pub(super) fn close(&self) {
        self.0.close();
    }

    /// Only synchronous non-reentrant Rust inspection is allowed in this scope.
    pub(super) fn inspect<R>(
        &self,
        ticket: &Ticket<T>,
        read: impl FnOnce(&T) -> R,
    ) -> Result<R, HostError> {
        if !Weak::ptr_eq(&ticket.owner, &Rc::downgrade(&self.0)) {
            return Err(HostError::InvalidArgument(
                "Upload preparation belongs to another session".into(),
            ));
        }
        if self.0.closed.get() {
            return Err(closed());
        }
        let registry = self.0.registry.upgrade().ok_or_else(closed)?;
        let state = registry.borrow();
        if state.stopped {
            return Err(closed());
        }
        let value = state
            .entries
            .iter()
            .flatten()
            .find(|entry| entry.id == ticket.id)
            .and_then(|entry| entry.value.as_ref())
            .ok_or_else(closed)?;
        Ok(read(value))
    }
}

impl<T> ScopeInner<T> {
    fn close(&self) {
        self.closed.set(true);
        let Some(registry) = self.registry.upgrade() else {
            return;
        };
        let removed: [Option<Entry<T>>; CALLS] = {
            let mut state = registry.borrow_mut();
            std::array::from_fn(|index| {
                if state.entries[index]
                    .as_ref()
                    .is_some_and(|entry| entry.session == self.id)
                {
                    state.entries[index].take()
                } else {
                    None
                }
            })
        };
        drop(removed);
    }
}
impl<T> Drop for ScopeInner<T> {
    fn drop(&mut self) {
        self.close();
    }
}

impl<T> Ticket<T> {
    /// Check out before invoking any user code. The payload stays revocable in
    /// the registry, and no borrow or strong owner escapes this operation.
    pub(super) fn phase(&self) -> Result<Phase<T>, HostError> {
        self.checkout_phase(true)
    }

    /// Admit synchronous initial root construction before any NAPI allocation.
    /// Unlike `phase`, this requires an empty, not-yet-populated entry.
    pub(super) fn creation_phase(&self) -> Result<Phase<T>, HostError> {
        self.checkout_phase(false)
    }

    fn checkout_phase(&self, populated: bool) -> Result<Phase<T>, HostError> {
        let scope = self.owner.upgrade().ok_or_else(closed)?;
        if scope.closed.get() {
            return Err(closed());
        }
        let registry = scope.registry.upgrade().ok_or_else(closed)?;
        let mut state = registry.borrow_mut();
        if state.stopped {
            return Err(closed());
        }
        let State {
            entries,
            phase_budget,
            ..
        } = &mut *state;
        let entry = entries
            .iter_mut()
            .flatten()
            .find(|entry| entry.id == self.id && entry.session == scope.id)
            .ok_or_else(closed)?;
        if entry.busy {
            return Err(HostError::InvalidArgument(
                "Upload preparation already has an active phase".into(),
            ));
        }
        if entry.value.is_some() != populated {
            return if populated {
                Err(closed())
            } else {
                Err(HostError::InvalidArgument(
                    "Upload preparation was already populated".into(),
                ))
            };
        }
        // This allowance cannot be revoked with the registry's FFI permit.
        // Reserve before marking busy, so refusal leaves the ticket reusable.
        let scratch = phase_budget.reserve()?;
        entry.busy = true;
        Ok(Phase {
            owner: self.owner.clone(),
            id: self.id,
            finished: false,
            _scratch: scratch,
        })
    }

    /// Promote the exact parked admission after all arbitrary language awaits.
    /// `read` is pure Rust and no registry borrow crosses native execution.
    #[cfg(feature = "retained-target-marking")]
    pub(super) fn begin_native<R>(
        &self,
        read: impl FnOnce(&T) -> Result<R, HostError>,
    ) -> Result<(NativePhase<T>, R, FfiPreparation<PreparationProgress>), HostError> {
        let scope = self.owner.upgrade().ok_or_else(closed)?;
        if scope.closed.get() {
            return Err(closed());
        }
        let registry = scope.registry.upgrade().ok_or_else(closed)?;
        let mut state = registry.borrow_mut();
        if state.stopped {
            return Err(closed());
        }
        let entry = state
            .entries
            .iter_mut()
            .flatten()
            .find(|entry| entry.id == self.id && entry.session == scope.id)
            .ok_or_else(closed)?;
        if entry.busy {
            return Err(HostError::InvalidArgument(
                "Upload preparation already has an active phase".into(),
            ));
        }
        let value = entry.value.as_ref().ok_or_else(closed)?;
        let snapshot = read(value)?;
        let preparation = entry.preparation.take().ok_or_else(|| {
            HostError::InvalidArgument("Upload preparation has no parked FFI admission".into())
        })?;
        entry.running = Some(preparation.revoker());
        entry.busy = true;
        Ok((
            NativePhase {
                owner: self.owner.clone(),
                id: self.id,
                finished: false,
            },
            snapshot,
            preparation,
        ))
    }

    // Fixture-only population: production must admit creation scratch before
    // constructing a payload, then commit through creation_phase().replace().
    #[cfg(test)]
    pub(super) fn populate(&self, value: T) -> Result<(), HostError> {
        let scope = self.owner.upgrade().ok_or_else(closed)?;
        if scope.closed.get() {
            return Err(closed());
        }
        let registry = scope.registry.upgrade().ok_or_else(closed)?;
        let mut state = registry.borrow_mut();
        if state.stopped {
            return Err(closed());
        }
        let entry = state
            .entries
            .iter_mut()
            .flatten()
            .find(|entry| entry.id == self.id)
            .ok_or_else(closed)?;
        if entry.busy || entry.value.is_some() {
            return Err(HostError::InvalidArgument(
                "Upload preparation was already populated".into(),
            ));
        }
        entry.value = Some(value);
        Ok(())
    }

    pub(super) fn abort(&self) -> bool {
        abort(&self.owner, self.id)
    }

    pub(super) fn is_pending(&self) -> bool {
        let Some(scope) = self.owner.upgrade() else {
            return false;
        };
        if scope.closed.get() {
            return false;
        }
        let Some(registry) = scope.registry.upgrade() else {
            return false;
        };
        let state = registry.borrow();
        !state.stopped
            && state
                .entries
                .iter()
                .flatten()
                .any(|entry| entry.id == self.id)
    }
}

impl<T> Drop for Entry<T> {
    fn drop(&mut self) {
        if let Some(running) = self.running.take() {
            running.revoke();
        }
    }
}
impl<T> Drop for Ticket<T> {
    fn drop(&mut self) {
        self.abort();
    }
}

impl<T> Phase<T> {
    /// Only pure, synchronous, non-reentrant Rust inspection is permitted. In
    /// particular, do not call user JavaScript or copy foreign roots here.
    pub(super) fn inspect<R>(&self, read: impl FnOnce(&T) -> R) -> Result<R, HostError> {
        let scope = self.owner.upgrade().ok_or_else(closed)?;
        if scope.closed.get() {
            return Err(closed());
        }
        let registry = scope.registry.upgrade().ok_or_else(closed)?;
        let state = registry.borrow();
        if state.stopped {
            return Err(closed());
        }
        let value = state
            .entries
            .iter()
            .flatten()
            .find(|entry| entry.id == self.id && entry.session == scope.id && entry.busy)
            .and_then(|entry| entry.value.as_ref())
            .ok_or_else(closed)?;
        Ok(read(value))
    }

    /// Commit one replacement without exposing the old payload. Phase scratch
    /// covers bounded administrative roots and their synchronous overlap, not
    /// arbitrary `T` allocations or the reachable V8 object graph.
    /// Both rejected and replaced values drop outside every registry borrow.
    pub(super) fn replace(mut self, value: T) -> Result<(), HostError> {
        let scope = self.owner.upgrade().ok_or_else(closed)?;
        if scope.closed.get() {
            return Err(closed());
        }
        let registry = scope.registry.upgrade().ok_or_else(closed)?;
        let old = {
            let mut state = registry.borrow_mut();
            if state.stopped {
                return Err(closed());
            }
            let entry = state
                .entries
                .iter_mut()
                .flatten()
                .find(|entry| entry.id == self.id && entry.session == scope.id && entry.busy)
                .ok_or_else(closed)?;
            let old = entry.value.replace(value);
            entry.busy = false;
            old
        };
        // A destructor may reenter and begin the next phase or revoke the
        // preparation. This completed guard must never abort that later work.
        self.finished = true;
        drop(old);
        Ok(())
    }

    /// Atomically commit a pure terminal state transition and remove the exact
    /// entry. The parked FFI result and payload drop after the registry borrow,
    /// releasing their permits without encoding success as cancellation.
    pub(super) fn finish_terminal(
        mut self,
        commit: impl FnOnce(&mut T) -> Result<(), HostError>,
    ) -> Result<(), HostError> {
        let scope = self.owner.upgrade().ok_or_else(closed)?;
        if scope.closed.get() {
            return Err(closed());
        }
        let registry = scope.registry.upgrade().ok_or_else(closed)?;
        let removed = {
            let mut state = registry.borrow_mut();
            if state.stopped {
                return Err(closed());
            }
            let index = state
                .entries
                .iter()
                .position(|entry| {
                    entry.as_ref().is_some_and(|entry| {
                        entry.id == self.id && entry.session == scope.id && entry.busy
                    })
                })
                .ok_or_else(closed)?;
            let value = state.entries[index]
                .as_mut()
                .and_then(|entry| entry.value.as_mut())
                .ok_or_else(closed)?;
            commit(value)?;
            state.entries[index].take()
        };
        self.finished = true;
        drop(removed);
        Ok(())
    }
}

impl<T> Drop for Phase<T> {
    fn drop(&mut self) {
        if !self.finished {
            abort(&self.owner, self.id);
        }
    }
}

#[cfg(feature = "retained-target-marking")]
impl<T> NativePhase<T> {
    /// Reinstall the SAME FFI admission before any publication callback can
    /// reenter Node. Returned phase scratch remains live across publication.
    pub(super) fn park(
        mut self,
        preparation: FfiPreparation<PreparationProgress>,
    ) -> Result<Phase<T>, HostError> {
        let scope = self.owner.upgrade().ok_or_else(closed)?;
        if scope.closed.get() {
            return Err(closed());
        }
        let registry = scope.registry.upgrade().ok_or_else(closed)?;
        let scratch = {
            let mut state = registry.borrow_mut();
            if state.stopped {
                return Err(closed());
            }
            let State {
                entries,
                phase_budget,
                ..
            } = &mut *state;
            // Reserve before changing ownership so refusal leaves Drop able to
            // revoke the still-running entry through its registered revoker.
            let scratch = phase_budget.reserve()?;
            let entry = entries
                .iter_mut()
                .flatten()
                .find(|entry| entry.id == self.id && entry.session == scope.id && entry.busy)
                .ok_or_else(closed)?;
            if entry.preparation.is_some() || entry.running.is_none() {
                return Err(HostError::InvalidArgument(
                    "Upload preparation native phase is out of order".into(),
                ));
            }
            entry.preparation = Some(preparation);
            entry.running = None;
            scratch
        };
        self.finished = true;
        Ok(Phase {
            owner: self.owner.clone(),
            id: self.id,
            finished: false,
            _scratch: scratch,
        })
    }
}

#[cfg(feature = "retained-target-marking")]
impl<T> Drop for NativePhase<T> {
    fn drop(&mut self) {
        if !self.finished {
            abort(&self.owner, self.id);
        }
    }
}

fn abort<T>(owner: &Weak<ScopeInner<T>>, id: u64) -> bool {
    let Some(scope) = owner.upgrade() else {
        return false;
    };
    let Some(registry) = scope.registry.upgrade() else {
        return false;
    };
    let removed = {
        let mut state = registry.borrow_mut();
        state
            .entries
            .iter_mut()
            .find(|entry| {
                entry
                    .as_ref()
                    .is_some_and(|entry| entry.id == id && entry.session == scope.id)
            })
            .and_then(Option::take)
    };
    let existed = removed.is_some();
    drop(removed);
    existed
}

#[cfg(test)]
mod tests;
