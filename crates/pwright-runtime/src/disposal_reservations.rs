//! Optional latest-state disposal preparation with pre-admitted storage and,
//! when selected, outbox capacity. In-process users need no control outbox.
//!
//! This is storage admission, not an object-incarnation or native-retirement
//! receipt. A composition supplies the authoritative object ID at preparation.
//! No lifecycle gate, graph snapshot, worker or transport is held while waiting.
//!
//! ```
//! use std::num::NonZeroUsize;
//! use pwright_core::{ObjectId, ObjectKindTag, ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope};
//! use pwright_runtime::{ControlledDispatcher, NoDeadlineScheduler, RuntimeLimits,
//!     disposal_reservations::{DisposalReservationProfile, ControlOutboxStorageProfile}};
//!
//! let limits = RuntimeLimits::new(4, 2).with_control_messages(8);
//! let selected = DisposalReservationProfile::new(
//!     NonZeroUsize::new(4).unwrap(), NonZeroUsize::new(1).unwrap(),
//!     ResourceScope::extension(1, 1),
//! )?.with_deferred_commit(NonZeroUsize::new(2).unwrap())?;
//! let outbox = ControlOutboxStorageProfile::new(
//!     NonZeroUsize::new(8).unwrap(), ResourceScope::extension(1, 2),
//! )?;
//! let plan = limits.resource_requirements().combine_concurrent(selected.requirements())
//!     .combine_concurrent(outbox.requirements());
//! let mut budget = ResourceLimits::deny_all(plan.scopes, plan.leases);
//! for class in ResourceClass::ALL {
//!     let amount = plan.class(class);
//!     budget = budget.with_class(class, amount.global, amount.per_scope);
//! }
//! let resources = ProductionResourceLedger::new(budget, plan)?;
//! let runtime = ControlledDispatcher::new((), (), NoDeadlineScheduler, limits)
//!     .with_production_resources(&resources)?;
//! outbox.install(&runtime.lifecycle_handle(), &resources)?;
//! runtime.register_object(ObjectId::new(1), None, ObjectKindTag::new(1))?;
//! runtime.pop_control_message()?;
//! let pool = selected.bind(&runtime.lifecycle_handle(), &resources)?;
//! let reservation = pool.reserve()?;
//! // No exclusive gate is held: a child may arrive during the native wait.
//! runtime.register_object(ObjectId::new(2), Some(ObjectId::new(1)), ObjectKindTag::new(1))?;
//! runtime.pop_control_message()?;
//! // A real composition validates its native retirement receipt separately.
//! let prepared = reservation.prepare(&runtime.lifecycle_handle(), ObjectId::new(1))?;
//! assert_eq!(prepared.disposal_order(), &[ObjectId::new(2), ObjectId::new(1)]);
//! let ((), mut committed) = prepared.commit_deferred_with_publication(None, || {
//!     // Commit independently preflighted local state here. No runtime reentry,
//!     // native wait, JavaScript delivery or user waker is allowed in this scope.
//!     Ok::<(), std::convert::Infallible>(())
//! })??;
//! assert_eq!(runtime.live_object_count()?, 0);
//! // Release enclosing topology/route/handle locks before notifying callers.
//! committed.notify();
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::num::NonZeroUsize;
use std::sync::{Weak, atomic::AtomicUsize};

use super::*;

pub const POOL_BYTES: u64 = 512;
pub const RESERVATION_BYTES: u64 = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutboxMode {
    Required,
    Absent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisposalReservationProfile {
    objects: usize,
    pending: usize,
    scope: ResourceScope,
    bytes: u64,
    requirements: ResourceRequirements,
    deferred_calls: Option<usize>,
    outbox: OutboxMode,
}

impl DisposalReservationProfile {
    /// `objects` must equal the runtime's whole live-object limit, including
    /// descendants that may be created after admission. Outbox EventItem and
    /// control-lease capacity come from the runtime's existing queue budget;
    /// this profile adds only retained reservation storage and its metadata.
    pub fn new(
        objects: NonZeroUsize,
        pending: NonZeroUsize,
        scope: ResourceScope,
    ) -> Result<Self, DisposalReservationError> {
        Self::build(objects, pending, scope, OutboxMode::Required)
    }

    /// Select an in-process runtime whose control outbox is disabled. Reserves
    /// bounded disposal-order storage and a pending slot, but no EventItem or
    /// control-lease arrays. Admission can coexist with an ordinary prepared
    /// lifecycle transaction because there is no uncommitted outbox to overbook.
    /// Preparation/commit still require a caller-serialized lifecycle boundary.
    pub fn new_in_process(
        objects: NonZeroUsize,
        pending: NonZeroUsize,
        scope: ResourceScope,
    ) -> Result<Self, DisposalReservationError> {
        Self::build(objects, pending, scope, OutboxMode::Absent)
    }

    fn build(
        objects: NonZeroUsize,
        pending: NonZeroUsize,
        scope: ResourceScope,
        outbox: OutboxMode,
    ) -> Result<Self, DisposalReservationError> {
        if !matches!(scope, ResourceScope::Extension { .. }) {
            return Err(DisposalReservationError::Invalid(
                "an extension scope is required",
            ));
        }
        let overflow = || DisposalReservationError::Invalid("reservation capacity overflow");
        let control_bytes = match outbox {
            OutboxMode::Required => std::mem::size_of::<ResourceLease>()
                .checked_add(std::mem::size_of::<pwright_core::ResourceRequest>())
                .ok_or_else(overflow)?,
            OutboxMode::Absent => 0,
        };
        let per_object = control_bytes
            // Child-first order grows as a Vec; charge its doubling headroom.
            .checked_add(2 * std::mem::size_of::<ObjectId>())
            .ok_or_else(overflow)?;
        let bytes = objects
            .get()
            .checked_mul(per_object)
            .and_then(|value| u64::try_from(value).ok())
            .and_then(|value| value.checked_add(RESERVATION_BYTES))
            .ok_or_else(overflow)?;
        let total = bytes
            .checked_mul(u64::try_from(pending.get()).map_err(|_| overflow())?)
            .and_then(|value| value.checked_add(POOL_BYTES))
            .ok_or_else(overflow)?;
        let leases = pending.get().checked_add(1).ok_or_else(overflow)?;
        objects
            .get()
            .checked_mul(pending.get())
            .ok_or_else(overflow)?;
        Ok(Self {
            objects: objects.get(),
            pending: pending.get(),
            scope,
            bytes,
            requirements: ResourceRequirements::none()
                .with_class(ResourceClass::BackendStateBytes, total, total)
                .with_metadata(1, leases),
            deferred_calls: None,
            outbox,
        })
    }

    /// Select post-commit notification ownership before any native work. The
    /// pending-call bound must match the runtime, including calls arriving while
    /// a reservation waits. Adds the deferred commit's collection payload peak
    /// and retained records/wakers; long-lived runtime/outbox backing storage,
    /// allocator overhead, arbitrary waker-owned heaps and caller-supplied typed
    /// diagnostic storage remain separate.
    pub fn with_deferred_commit(
        mut self,
        pending_calls: NonZeroUsize,
    ) -> Result<Self, DisposalReservationError> {
        if self.deferred_calls.is_some() {
            return Err(DisposalReservationError::Invalid(
                "deferred commit already selected",
            ));
        }
        let extra = deferred::commit_storage_bytes(self.objects, pending_calls.get())?;
        let overflow = || DisposalReservationError::Invalid("deferred commit capacity overflow");
        self.bytes = self.bytes.checked_add(extra).ok_or_else(overflow)?;
        let total = self
            .bytes
            .checked_mul(self.pending as u64)
            .and_then(|bytes| bytes.checked_add(POOL_BYTES))
            .ok_or_else(overflow)?;
        self.requirements = ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, total, total)
            .with_metadata(1, self.pending + 1);
        self.deferred_calls = Some(pending_calls.get());
        Ok(self)
    }

    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }

    pub fn bind(
        self,
        handle: &ControlledLifecycleHandle,
        resources: &ProductionResourceLedger,
    ) -> Result<DisposalReservations, DisposalReservationError> {
        {
            let state = handle
                .store
                .state
                .lock()
                .map_err(|_| RuntimeStateError::StatePoisoned)?;
            if state.transport_closed {
                return Err(RuntimeStateError::TransportClosed.into());
            }
            if state.objects.limit() != self.objects {
                return Err(DisposalReservationError::Invalid(
                    "runtime object limit differs from reservation profile",
                ));
            }
            if self
                .deferred_calls
                .is_some_and(|limit| limit != state.pending_calls.limit())
            {
                return Err(DisposalReservationError::Invalid(
                    "runtime pending-call limit differs from deferred commit profile",
                ));
            }
            match (self.outbox, state.control_messages.as_ref()) {
                (OutboxMode::Required, None) => {
                    return Err(DisposalReservationError::Invalid(
                        "a controlled outbox is required",
                    ));
                }
                (OutboxMode::Absent, Some(_)) => {
                    return Err(DisposalReservationError::Invalid(
                        "in-process disposal reservations require a disabled control outbox",
                    ));
                }
                (OutboxMode::Required, Some(_)) | (OutboxMode::Absent, None) => {}
            }
            if state.resource_ledger.is_none() {
                return Err(DisposalReservationError::Invalid(
                    "a budgeted runtime is required",
                ));
            }
            if let Some(queue) = &state.control_messages
                && self.objects * self.pending > queue.capacity()
            {
                return Err(DisposalReservationError::Invalid(
                    "outbox cannot hold all declared disposal reservations",
                ));
            }
        }
        let ledger = resources
            .claim(self.requirements)
            .map_err(RuntimeStateError::from)?;
        let lease = ledger
            .reserve(self.scope, ResourceClass::BackendStateBytes, POOL_BYTES)
            .map_err(RuntimeStateError::from)?;
        Ok(DisposalReservations {
            state: Arc::new(Pool {
                store: Arc::downgrade(&handle.store),
                ledger,
                profile: self,
                pending: AtomicUsize::new(0),
                _lease: lease,
            }),
        })
    }
}

struct Pool {
    store: Weak<RuntimeStore>,
    ledger: ResourceLedger,
    profile: DisposalReservationProfile,
    pending: AtomicUsize,
    _lease: ResourceLease,
}

#[derive(Clone)]
pub struct DisposalReservations {
    state: Arc<Pool>,
}

impl DisposalReservations {
    pub fn pending(&self) -> usize {
        self.state.pending.load(Ordering::Acquire)
    }

    pub fn reserve(&self) -> Result<ReservedDisposalCapacity, DisposalReservationError> {
        let profile = self.state.profile;
        let store = self
            .state
            .store
            .upgrade()
            .ok_or(RuntimeStateError::TransportClosed)?;
        let mut state = store
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        // Transport termination is authoritative, even if another reservation
        // still occupies every pending slot or resource capacity is exhausted.
        if state.transport_closed {
            return Err(RuntimeStateError::TransportClosed.into());
        }
        self.state
            .pending
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |pending| {
                (pending < profile.pending).then(|| pending + 1)
            })
            .map_err(|_| DisposalReservationError::SlotsExhausted {
                limit: profile.pending,
            })?;
        let permit = Permit(Arc::clone(&self.state));
        let memory = self
            .state
            .ledger
            .reserve(
                profile.scope,
                ResourceClass::BackendStateBytes,
                profile.bytes,
            )
            .map_err(RuntimeStateError::from)?;
        let (controls, slots) = match profile.outbox {
            OutboxMode::Required => {
                // Ordinary prepared transactions own outbox capacity implicitly
                // through their gate. Never overbook those uncommitted items.
                state.ensure_no_lifecycle_transaction()?;
                state.ensure_control_capacity(profile.objects)?;
                let ledger =
                    state
                        .resource_ledger
                        .as_ref()
                        .ok_or(DisposalReservationError::Invalid(
                            "runtime lost its resource ledger",
                        ))?;
                let mut controls = VecDeque::with_capacity(profile.objects);
                for _ in 0..profile.objects {
                    controls.push_back(
                        ledger
                            .reserve(profile.scope, ResourceClass::EventItem, 1)
                            .map_err(RuntimeStateError::from)?,
                    );
                }
                (Some(controls), profile.objects)
            }
            OutboxMode::Absent => (None, 0),
        };
        // Nothing fallible follows this logical admission. On earlier failure
        // locals release leases before the permit, without a recursive store lock.
        state.reserved_control_slots += slots;
        drop(state);
        Ok(ReservedDisposalCapacity {
            controls,
            memory,
            queue: QueueReservation {
                store: Arc::downgrade(&store),
                slots,
            },
            permit,
        })
    }
}

struct Permit(Arc<Pool>);
impl Drop for Permit {
    fn drop(&mut self) {
        self.0.pending.fetch_sub(1, Ordering::AcqRel);
    }
}

struct QueueReservation {
    store: Weak<RuntimeStore>,
    slots: usize,
}
impl Drop for QueueReservation {
    fn drop(&mut self) {
        if self.slots == 0 {
            return;
        }
        if let Some(store) = self.store.upgrade()
            && let Ok(mut state) = store.state.lock()
        {
            state.reserved_control_slots -= self.slots;
        }
    }
}

/// Drop releases event leases, queue slots and the pending permit in that order.
#[must_use = "retain until preparation or drop to release reserved capacity"]
pub struct ReservedDisposalCapacity {
    controls: Option<VecDeque<ResourceLease>>,
    memory: ResourceLease,
    queue: QueueReservation,
    permit: Permit,
}

/// A busy lifecycle boundary preserves admission without holding a transaction.
/// The caller parks `Deferred` and retries only local preparation at its next
/// serialized boundary; this value does not authorize repeating native work.
#[must_use = "commit prepared disposal or retain deferred capacity"]
pub enum DisposalPreparation {
    Prepared(ReservedPreparedDisposal),
    Deferred(ReservedDisposalCapacity),
}

impl ReservedDisposalCapacity {
    /// Check deferred-commit admission before native work without acquiring a
    /// lifecycle transaction or consuming the reservation. This validates the
    /// exact runtime and its transport now, not its state after native I/O.
    pub fn validate_deferred_commit(
        &self,
        handle: &ControlledLifecycleHandle,
    ) -> Result<(), DisposalReservationError> {
        if !Weak::ptr_eq(&self.queue.store, &Arc::downgrade(&handle.store)) {
            return Err(DisposalReservationError::Invalid(
                "disposal reservation belongs to another runtime",
            ));
        }
        let state = handle
            .store
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        if state.transport_closed {
            return Err(RuntimeStateError::TransportClosed.into());
        }
        if self.permit.0.profile.deferred_calls.is_none() {
            return Err(DisposalReservationError::Invalid(
                "deferred commit was not admitted",
            ));
        }
        Ok(())
    }

    /// The composition validates native identity/retirement separately. Unknown
    /// IDs retain ordinary runtime no-op semantics, not a native-success claim.
    /// Preparation consumes the reservation on failure; schedule this short
    /// phase only when the shared lifecycle gate is available.
    pub fn prepare(
        self,
        handle: &ControlledLifecycleHandle,
        object_id: ObjectId,
    ) -> Result<ReservedPreparedDisposal, DisposalReservationError> {
        match self.prepare_or_defer(handle, object_id)? {
            DisposalPreparation::Prepared(prepared) => Ok(prepared),
            DisposalPreparation::Deferred(_) => {
                Err(RuntimeStateError::LifecycleTransactionInProgress.into())
            }
        }
    }

    /// Unlike `prepare`, retains all admission when another lifecycle
    /// transaction is active. Other errors consume and release the reservation.
    /// No native identity, scheduling, notification or retry policy is supplied.
    pub fn prepare_or_defer(
        mut self,
        handle: &ControlledLifecycleHandle,
        object_id: ObjectId,
    ) -> Result<DisposalPreparation, DisposalReservationError> {
        match self.prepare_ready(handle, object_id) {
            Ok(transaction) => Ok(DisposalPreparation::Prepared(ReservedPreparedDisposal {
                transaction,
                _memory: self.memory,
                _permit: self.permit,
            })),
            Err(DisposalReservationError::Runtime(
                RuntimeStateError::LifecycleTransactionInProgress,
            )) => Ok(DisposalPreparation::Deferred(self)),
            Err(error) => Err(error),
        }
    }

    fn prepare_ready(
        &mut self,
        handle: &ControlledLifecycleHandle,
        object_id: ObjectId,
    ) -> Result<PreparedObjectDisposal, DisposalReservationError> {
        if !Weak::ptr_eq(&self.queue.store, &Arc::downgrade(&handle.store)) {
            return Err(DisposalReservationError::Invalid(
                "disposal reservation belongs to another runtime",
            ));
        }
        let mut state = handle
            .store
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        if state.transport_closed {
            return Err(RuntimeStateError::TransportClosed.into());
        }
        state.ensure_no_lifecycle_transaction()?;
        let disposal_order = state.objects.subtree_ids_child_first(object_id);
        debug_assert!(disposal_order.len() <= self.permit.0.profile.objects);
        let transaction_id = state.begin_lifecycle_transaction()?;
        if let Some(controls) = &mut self.controls {
            controls.truncate(disposal_order.len());
        }
        let reserved_control_slots = self.queue.slots;
        self.queue.slots = 0;
        drop(state);
        Ok(PreparedObjectDisposal {
            store: Arc::clone(&handle.store),
            transaction_id,
            object_id,
            disposal_order,
            control_resources: self.controls.take(),
            reserved_control_slots,
            finished: false,
        })
    }
}

/// Reuses ordinary runtime child-first disposal, call cancellation and typed
/// failure propagation. The ordinary profile covers waiting owners/outbox
/// leases; selecting `with_deferred_commit` additionally admits the deferred
/// runtime collection peak and retains admission with its returned records and
/// wakers. Neither profile covers an arbitrary native operation's future.
#[must_use = "commit or drop the short exclusive disposal"]
pub struct ReservedPreparedDisposal {
    transaction: PreparedObjectDisposal,
    _memory: ResourceLease,
    _permit: Permit,
}

impl ReservedPreparedDisposal {
    pub fn disposal_order(&self) -> &[ObjectId] {
        self.transaction.disposal_order()
    }
    pub fn commit(self) -> Result<Vec<ObjectRecord<ObjectKindTag>>, RuntimeStateError> {
        self.transaction.commit()
    }
    pub fn commit_with_failure(
        self,
        failure: RuntimeCallFailure,
    ) -> Result<Vec<ObjectRecord<ObjectKindTag>>, RuntimeStateError> {
        self.transaction.commit_with_failure(failure)
    }
}

#[derive(Debug)]
pub enum DisposalReservationError {
    Invalid(&'static str),
    SlotsExhausted { limit: usize },
    Runtime(RuntimeStateError),
}
impl From<RuntimeStateError> for DisposalReservationError {
    fn from(error: RuntimeStateError) -> Self {
        Self::Runtime(error)
    }
}
impl Display for DisposalReservationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => f.write_str(message),
            Self::SlotsExhausted { limit } => write!(
                f,
                "runtime disposal reservation capacity exceeded ({limit})"
            ),
            Self::Runtime(error) => Display::fmt(error, f),
        }
    }
}
impl Error for DisposalReservationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Runtime(error) => Some(error),
            Self::Invalid(_) | Self::SlotsExhausted { .. } => None,
        }
    }
}

mod deferred;
mod outbox_storage;
pub use outbox_storage::ControlOutboxStorageProfile;
#[cfg(test)]
mod tests;
pub use deferred::DeferredDisposalCommit;
