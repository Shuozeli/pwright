//! Bounded, session-revocable ownership of not-yet-delivered Handle results.
//! The returned descriptor never holds a native service. Closing the owner
//! submits all accepted cleanup, even while callers retain descriptors.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use pwright_cap_handle_evaluation::HandleInitializer;
use pwright_composition_chromium_handle_evaluation::ChromiumHandleEvaluationRouteIdentity;
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceLedger,
    ResourceRequirements, ResourceScope,
};
use pwright_runtime::{CleanupFailure, CleanupJob, CleanupReservation};
use pwright_runtime_tokio::TokioCleanupReservation;

use super::{MAX_CALLS, MAX_HANDLES, MAX_NATIVE_ID_BYTES};
use crate::{HostError, evaluation};

const SCOPE: ResourceScope = ResourceScope::extension(154, 1);
const STATE_BYTES: u64 = 16 * 1024;
// Descriptor, accepted cleanup, watch backing, the <=1024-byte immutable source
// world ID, and the independently retained bounded completion diagnostic must
// all remain covered through disposal. Source identity owns no native service.
const OWNER_BYTES: u64 = 16 * 1024;
const CALL_BYTES: u64 = evaluation::FFI_ALLOWANCE;
// A malformed acquisition can carry distinct result and exception receivers.
// Both remain owned until the hardened acquisition compensates them.
const READ_NATIVE_HANDLES: u64 = 2;

pub(crate) fn requirements() -> ResourceRequirements {
    let bytes = STATE_BYTES + OWNER_BYTES * MAX_HANDLES as u64 + CALL_BYTES * MAX_CALLS as u64;
    ResourceRequirements::none()
        .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
        .with_class(
            ResourceClass::RemoteHandle,
            MAX_CALLS as u64 * READ_NATIVE_HANDLES,
            MAX_CALLS as u64 * READ_NATIVE_HANDLES,
        )
        .with_metadata(1, 1 + MAX_HANDLES + 2 * MAX_CALLS)
}

struct Capacity {
    owners: AtomicUsize,
    calls: AtomicUsize,
    ledger: ResourceLedger,
    _state: ResourceLease,
}

enum Kind {
    Owner,
    Call,
}
pub(crate) struct Allowance {
    capacity: Arc<Capacity>,
    kind: Kind,
    _lease: ResourceLease,
}
impl Drop for Allowance {
    fn drop(&mut self) {
        match self.kind {
            Kind::Owner => &self.capacity.owners,
            Kind::Call => &self.capacity.calls,
        }
        .fetch_sub(1, Ordering::AcqRel);
    }
}

impl Capacity {
    fn admit(self: &Arc<Self>, kind: Kind) -> Result<Allowance, HostError> {
        let (counter, maximum, bytes) = match kind {
            Kind::Owner => (&self.owners, MAX_HANDLES, OWNER_BYTES),
            Kind::Call => (&self.calls, MAX_CALLS, CALL_BYTES),
        };
        counter
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < maximum).then(|| count + 1)
            })
            .map_err(|_| {
                HostError::ResourceExhausted("Handle result ownership capacity exceeded".into())
            })?;
        match self
            .ledger
            .reserve(SCOPE, ResourceClass::BackendStateBytes, bytes)
        {
            Ok(lease) => Ok(Allowance {
                capacity: self.clone(),
                kind,
                _lease: lease,
            }),
            Err(error) => {
                counter.fetch_sub(1, Ordering::AcqRel);
                Err(HostError::backend(error))
            }
        }
    }
}

pub(crate) struct Admission {
    pub owner: Arc<Allowance>,
    // One slot/lease shared by execution, publication and its suspended caller.
    // Cloning the guard retains existing admission; it never creates a new one.
    pub call: Arc<Allowance>,
    pub work: tokio_util::task::task_tracker::TaskTrackerToken,
}

/// Every possible result is admitted before native enumeration. Unused owners
/// are trimmed downward; all siblings share exactly one call allowance.
pub(crate) struct BatchAdmission {
    pub entries: Vec<Admission>,
    pub call: Arc<Allowance>,
    pub work: tokio_util::task::task_tracker::TaskTrackerToken,
}

/// A read that temporarily acquires a native object must remain tracked through
/// its cleanup, even when the public caller and the persistent Handle disappear.
pub(crate) struct ReadAdmission {
    pub call: Arc<Allowance>,
    pub work: tokio_util::task::task_tracker::TaskTrackerToken,
    pub native: ResourceLease,
}

/// Caller/continuation storage and shutdown tracking for a retained upload.
/// Its independently owned resolver admits temporary native objects. This
/// guard does not pin the persistent source against explicit disposal.
#[cfg(feature = "retained-file-input")]
pub(crate) struct TrackedCallAdmission {
    pub call: Arc<Allowance>,
    pub work: tokio_util::task::task_tracker::TaskTrackerToken,
}

pub(crate) struct Ticket {
    pub reservation: TokioCleanupReservation,
    pub job: CleanupJob,
}
impl Ticket {
    fn submit(self) {
        self.reservation.submit(self.job);
    }
}

struct OwnedTicket {
    object: ObjectId,
    ticket: Ticket,
}

struct State {
    closed: bool,
    tickets: [Option<OwnedTicket>; MAX_HANDLES],
}
pub(crate) struct Owners {
    state: Mutex<State>,
    capacity: Arc<Capacity>,
    work: tokio_util::task::TaskTracker,
}

/// Fixed-size preparation metadata, not permission to use a native receiver.
///
/// The weak origin proves exact session provenance without keeping its native
/// services, cleanup work or resource leases alive. Object and Frame IDs are
/// copied values; neither is a current-world or current-DOM ownership proof.
/// Callers must budget retained metadata/weak backing, and submission must
/// independently revalidate the actual Handle's liveness and native route.
#[cfg(feature = "retained-file-input")]
#[derive(Clone)]
pub struct HandleSourceIdentity {
    owners: std::sync::Weak<Owners>,
    target: ObjectId,
    creation_frame: ObjectId,
    creation_page: Option<ObjectId>,
    is_element: bool,
}

#[cfg(feature = "retained-file-input")]
impl HandleSourceIdentity {
    pub const fn target(&self) -> ObjectId {
        self.target
    }

    /// Creation Frame only; a DOM node may subsequently belong to another Frame.
    pub const fn creation_frame_id(&self) -> ObjectId {
        self.creation_frame
    }

    /// Immutable creation-world Page proof, not a current DOM owner query.
    pub const fn creation_page_id(&self) -> Option<ObjectId> {
        self.creation_page
    }

    pub(crate) fn belongs_to(&self, owners: &std::sync::Weak<Owners>) -> bool {
        std::sync::Weak::ptr_eq(&self.owners, owners)
    }

    #[cfg(test)]
    pub(crate) fn timeout_policy_fixture(
        owners: &Arc<Owners>,
        creation_page: Option<ObjectId>,
    ) -> Self {
        Self {
            owners: Arc::downgrade(owners),
            target: ObjectId::new(3),
            creation_frame: ObjectId::new(2),
            creation_page,
            is_element: false,
        }
    }

    pub const fn is_element(&self) -> bool {
        self.is_element
    }
}

impl Owners {
    pub fn new(resources: &ProductionResourceLedger) -> Result<Arc<Self>, HostError> {
        if std::mem::size_of::<Self>() + std::mem::size_of::<Capacity>() + 128
            > STATE_BYTES as usize
            || std::mem::size_of::<PendingHandle>()
                + std::mem::size_of::<Ticket>()
                + MAX_NATIVE_ID_BYTES
                + 8192
                + 512
                > OWNER_BYTES as usize
        {
            return Err(HostError::ResourceExhausted(
                "Handle owner storage exceeds its budget".into(),
            ));
        }
        let ledger = resources
            .claim(requirements())
            .map_err(HostError::backend)?;
        let state = ledger
            .reserve(SCOPE, ResourceClass::BackendStateBytes, STATE_BYTES)
            .map_err(HostError::backend)?;
        Ok(Arc::new(Self {
            work: tokio_util::task::TaskTracker::new(),
            state: Mutex::new(State {
                closed: false,
                tickets: std::array::from_fn(|_| None),
            }),
            capacity: Arc::new(Capacity {
                owners: AtomicUsize::new(0),
                calls: AtomicUsize::new(0),
                ledger,
                _state: state,
            }),
        }))
    }

    pub fn admit(self: &Arc<Self>) -> Result<Admission, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        let owner = Arc::new(self.capacity.admit(Kind::Owner)?);
        let call = Arc::new(self.capacity.admit(Kind::Call)?);
        Ok(Admission {
            owner,
            call,
            work: self.work.token(),
        })
    }

    pub fn admit_read(self: &Arc<Self>, handle: &PendingHandle) -> Result<Allowance, HostError> {
        self.check_arguments(std::iter::once(handle))?;
        self.admit_call()
    }

    pub fn admit_tracked_read(
        self: &Arc<Self>,
        handle: &PendingHandle,
    ) -> Result<ReadAdmission, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        self.check_handle(&state, handle)?;
        // Admission and work registration share close()'s lock. A token cannot
        // appear after shutdown has observed the tracker as closed and empty.
        Ok(ReadAdmission {
            call: Arc::new(self.capacity.admit(Kind::Call)?),
            work: self.work.token(),
            native: self
                .capacity
                .ledger
                .reserve(SCOPE, ResourceClass::RemoteHandle, READ_NATIVE_HANDLES)
                .map_err(HostError::backend)?,
        })
    }

    #[cfg(feature = "retained-file-input")]
    pub(crate) fn admit_tracked_call(
        self: &Arc<Self>,
        handle: &PendingHandle,
    ) -> Result<TrackedCallAdmission, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        self.check_handle(&state, handle)?;
        // Match close()'s admission boundary: validation, the shared call slot
        // and tracked work must all precede a closed-and-empty observation.
        Ok(TrackedCallAdmission {
            call: Arc::new(self.capacity.admit(Kind::Call)?),
            work: self.work.token(),
        })
    }

    pub fn admit_batch(self: &Arc<Self>) -> Result<BatchAdmission, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        let call = Arc::new(self.capacity.admit(Kind::Call)?);
        let available = MAX_HANDLES - self.capacity.owners.load(Ordering::Acquire);
        let mut entries = Vec::with_capacity(available);
        for _ in 0..available {
            entries.push(Admission {
                owner: Arc::new(self.capacity.admit(Kind::Owner)?),
                call: call.clone(),
                work: self.work.token(),
            });
        }
        Ok(BatchAdmission {
            entries,
            call,
            work: self.work.token(),
        })
    }

    pub fn admit_call(self: &Arc<Self>) -> Result<Allowance, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        self.capacity.admit(Kind::Call)
    }

    pub fn check_arguments<'a>(
        self: &Arc<Self>,
        handles: impl IntoIterator<Item = &'a PendingHandle>,
    ) -> Result<(), HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        for handle in handles {
            self.check_handle(&state, handle)?;
        }
        Ok(())
    }

    #[cfg(feature = "retained-file-input")]
    pub(crate) fn check_file_input_source(
        self: &Arc<Self>,
        source: &HandleSourceIdentity,
    ) -> Result<(), HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        if !std::sync::Weak::ptr_eq(&source.owners, &Arc::downgrade(self)) {
            return Err(HostError::InvalidArgument(
                "Handle belongs to another session".into(),
            ));
        }
        if !source.is_element {
            return Err(HostError::InvalidArgument(
                "setInputFiles requires an ElementHandle".into(),
            ));
        }
        // Deliberately do not consult tickets: an explicit dispose must not
        // skip user-visible preparation getters after an ownerFrame override.
        Ok(())
    }

    /// Check channel registration immediately before dispatch, not preparation
    /// provenance or permission to use a native receiver. Navigation and Frame
    /// detachment do not themselves unregister a retained Handle dispatcher.
    /// The later native operation must still validate its actual route/world.
    #[cfg(feature = "retained-file-input")]
    pub(crate) fn check_file_input_dispatch_source(
        self: &Arc<Self>,
        source: &HandleSourceIdentity,
    ) -> Result<(), HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        if !std::sync::Weak::ptr_eq(&source.owners, &Arc::downgrade(self)) {
            return Err(HostError::InvalidArgument(
                "Handle belongs to another session".into(),
            ));
        }
        if !source.is_element {
            return Err(HostError::InvalidArgument(
                "setInputFiles requires an ElementHandle".into(),
            ));
        }
        if !state
            .tickets
            .iter()
            .flatten()
            .any(|ticket| ticket.object == source.target)
        {
            return Err(HostError::TargetClosed);
        }
        Ok(())
    }

    fn check_handle(
        self: &Arc<Self>,
        state: &State,
        handle: &PendingHandle,
    ) -> Result<(), HostError> {
        if !Arc::ptr_eq(self, &handle.owners) {
            return Err(HostError::InvalidArgument(
                "Handle belongs to another session".into(),
            ));
        }
        if !state
            .tickets
            .iter()
            .flatten()
            .any(|ticket| ticket.object == handle.initializer.object_id)
        {
            return Err(HostError::InvalidArgument("JSHandle is disposed".into()));
        }
        Ok(())
    }

    /// Synchronous ownership transfer: no await can separate creation from
    /// either a registered cleanup ticket or submission of that same ticket.
    pub fn retain(
        self: &Arc<Self>,
        initializer: HandleInitializer,
        mut ticket: Ticket,
        owner: Arc<Allowance>,
    ) -> Result<PendingHandle, HostError> {
        let object = initializer.object_id;
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(error) => {
                ticket.submit();
                return Err(HostError::backend(error));
            }
        };
        if state.closed {
            drop(state);
            ticket.submit();
            return Err(HostError::TargetClosed);
        }
        let Some(slot) = state.tickets.iter_mut().find(|slot| slot.is_none()) else {
            drop(state);
            ticket.submit();
            return Err(HostError::ResourceExhausted(
                "Handle result ticket capacity exceeded".into(),
            ));
        };
        // The completion belongs to the accepted cleanup job, not to a caller
        // waiting for disposal. Cancelling that caller cannot cancel release.
        let (completion, released) = tokio::sync::watch::channel(None);
        let job = ticket.job;
        ticket.job = Box::pin(async move {
            let result = job.await;
            completion.send_replace(Some(result.clone()));
            result
        });
        *slot = Some(OwnedTicket { object, ticket });
        Ok(PendingHandle {
            initializer,
            source_world: None,
            owners: self.clone(),
            _owner: owner,
            released,
        })
    }

    fn abandon(&self, object: ObjectId) {
        // Poison recovery is cleanup-only. Taking a ticket is exclusive and
        // submit runs outside the lock; shutdown and drop cannot submit twice.
        let ticket = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .tickets
            .iter_mut()
            .find(|slot| slot.as_ref().is_some_and(|ticket| ticket.object == object))
            .and_then(Option::take);
        if let Some(ticket) = ticket {
            ticket.ticket.submit();
        }
    }

    pub fn close(&self) {
        let tickets = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.closed = true;
            std::mem::replace(&mut state.tickets, std::array::from_fn(|_| None))
        };
        // The state flag already prevents new tokens. Tracker closure can
        // synchronously wake a waiter that reenters owner admission, so it
        // must happen after releasing the owner lock, before draining tickets.
        self.work.close();
        for ticket in tickets.into_iter().flatten() {
            ticket.ticket.submit();
        }
    }

    pub async fn wait(&self) {
        self.work.wait().await;
    }
}
impl Drop for Owners {
    fn drop(&mut self) {
        self.close();
    }
}

/// An owned, not-yet-published result. Dropping it schedules native release;
/// session shutdown also releases it without waiting for this Rust value or a
/// JS wrapper to be collected. This is the Rust owner, not a JS Handle itself.
#[must_use = "a pending Handle must remain owned until delivery or be dropped for cleanup"]
pub struct PendingHandle {
    initializer: HandleInitializer,
    // Bounded immutable metadata only: unlike a snapshot/lifetime, this cannot
    // retain a native client, world permit, route or shutdown work token.
    source_world: Option<ChromiumHandleEvaluationRouteIdentity>,
    owners: Arc<Owners>,
    _owner: Arc<Allowance>,
    released: tokio::sync::watch::Receiver<Option<Result<(), CleanupFailure>>>,
}

/// A completion-only receipt. It owns neither the descriptor nor a native
/// service, allowing a disposed language wrapper to relinquish its allowance.
#[derive(Clone)]
pub struct HandleRelease {
    released: tokio::sync::watch::Receiver<Option<Result<(), CleanupFailure>>>,
}

impl PendingHandle {
    /// Capture preparation authority without native I/O, allocation or liveness
    /// checks. This metadata can remain after the descriptor is disposed/dropped.
    #[cfg(feature = "retained-file-input")]
    pub fn source_identity(&self) -> HandleSourceIdentity {
        HandleSourceIdentity {
            owners: Arc::downgrade(&self.owners),
            target: self.initializer.object_id,
            creation_frame: self.initializer.parent_id,
            creation_page: self.source_world.as_ref().map(|world| world.page_id()),
            is_element: self.is_element(),
        }
    }

    pub(super) fn with_source_world(
        mut self,
        source_world: ChromiumHandleEvaluationRouteIdentity,
    ) -> Self {
        self.source_world = Some(source_world);
        self
    }

    pub(crate) fn source_world(&self) -> Option<&ChromiumHandleEvaluationRouteIdentity> {
        self.source_world.as_ref()
    }

    pub fn initializer(&self) -> &HandleInitializer {
        &self.initializer
    }

    pub fn is_element(&self) -> bool {
        self.initializer.object_kind == pwright_browser_model::ELEMENT_HANDLE_OBJECT_KIND.tag
    }

    /// Submit release exactly once and wait for its native/local completion.
    /// Repeated or concurrent calls observe the same result. Cancelling this
    /// wait does not cancel cleanup; session shutdown uses the same ticket.
    /// Descriptor storage stays charged until this owned value is dropped.
    pub async fn dispose(&self) -> Result<(), HostError> {
        self.request_dispose().wait().await
    }

    pub fn request_dispose(&self) -> HandleRelease {
        self.owners.abandon(self.initializer.object_id);
        HandleRelease {
            released: self.released.clone(),
        }
    }
}

impl HandleRelease {
    pub async fn wait(&self) -> Result<(), HostError> {
        let mut released = self.released.clone();
        loop {
            if let Some(result) = released.borrow_and_update().clone() {
                return result.map_err(HostError::backend);
            }
            released.changed().await.map_err(|_| {
                HostError::backend("Handle release ended without a completion acknowledgement")
            })?;
        }
    }
}
impl Drop for PendingHandle {
    fn drop(&mut self) {
        self.owners.abandon(self.initializer.object_id);
    }
}

#[cfg(test)]
mod tests;
