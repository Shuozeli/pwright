//! Optional native ancestor authority, not a retirement/publication transaction.
//! Production installation derives connection and topology from one attachment.
use crate::{AttachedChromiumComposition, SharedChromiumTopology};
use pwright_backend_cdp::{CdpContextOwnership, ConnectionEpoch};
use pwright_backend_cdp_page_lifecycle::native_confirmation::NativeCloseInterruption;
use pwright_cdp::{CdpConnection, CdpSession};
use pwright_core::{
    FailureKind, ObjectId, OperationError, ProductionResourceLedger, ResourceClass, ResourceLease,
    ResourceLedger, ResourceRequirements, ResourceScope,
};
use std::{
    future::Future,
    num::NonZeroU16,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};

pub type NativeCloseAuthorityError = OperationError<()>;
type Error = NativeCloseAuthorityError;
const STATE_BYTES: u64 = 2048;
const INTENT_BYTES: u64 = 16384;
mod context;
#[cfg(feature = "native-close-scheduler")]
pub mod scheduler;
pub use context::{
    NativeAncestorRetirement, NativeContextCloseFuture, NativeContextCloseReceipt,
    PreparedNativeContextClose,
};

#[derive(Clone, Copy)]
pub struct NativeCloseAuthorityProfile {
    scope: ResourceScope,
    pending: NonZeroU16,
    id_bytes: NonZeroU16,
}
impl NativeCloseAuthorityProfile {
    pub fn new(
        scope: ResourceScope,
        pending: NonZeroU16,
        id_bytes: NonZeroU16,
    ) -> Result<Self, Error> {
        if id_bytes.get() > 1024 {
            return Err(fail(
                FailureKind::ProtocolViolation,
                "native close identity bound exceeds 1024",
            ));
        }
        Ok(Self {
            scope,
            pending,
            id_bytes,
        })
    }
    pub fn requirements(self) -> ResourceRequirements {
        let bytes = STATE_BYTES + INTENT_BYTES * u64::from(self.pending.get());
        ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, usize::from(self.pending.get()) + 1)
    }
}

struct State {
    connection: Arc<CdpConnection>,
    topology: SharedChromiumTopology,
    epoch: ConnectionEpoch,
    ledger: ResourceLedger,
    profile: NativeCloseAuthorityProfile,
    interruption: Arc<dyn NativeCloseInterruption>,
    pending: AtomicUsize,
    sequence: AtomicU64,
    _storage: ResourceLease,
}

/// Clones share one exact attachment domain, admission limit and generation.
#[derive(Clone)]
pub struct NativeCloseAuthority(Arc<State>);

impl AttachedChromiumComposition {
    /// The extension requirements must be declared before attach. Installation
    /// opens no socket and does not select native scheduling or Page confirmation.
    pub fn native_close_authority<I: NativeCloseInterruption + 'static>(
        &self,
        profile: NativeCloseAuthorityProfile,
        interruption: I,
    ) -> Result<NativeCloseAuthority, Error> {
        NativeCloseAuthority::bind(
            self.connection().clone(),
            self.shared_topology(),
            self.resources(),
            profile,
            interruption,
        )
    }
}

impl NativeCloseAuthority {
    fn bind<I: NativeCloseInterruption + 'static>(
        connection: Arc<CdpConnection>,
        topology: SharedChromiumTopology,
        resources: &ProductionResourceLedger,
        profile: NativeCloseAuthorityProfile,
        interruption: I,
    ) -> Result<Self, Error> {
        if std::mem::size_of::<State>()
            + std::mem::size_of::<I>()
            + 4 * std::mem::size_of::<usize>()
            > STATE_BYTES as usize
        {
            return Err(fail(
                FailureKind::ResourceExhausted,
                "native close authority installation exceeds admission",
            ));
        }
        let epoch = topology.lock().map_err(|_| poisoned())?.topology().epoch();
        let ledger = resources.claim(profile.requirements()).map_err(resource)?;
        let storage = ledger
            .reserve(profile.scope, ResourceClass::BackendStateBytes, STATE_BYTES)
            .map_err(resource)?;
        Ok(Self(Arc::new(State {
            connection,
            topology,
            epoch,
            ledger,
            profile,
            interruption: Arc::new(interruption),
            pending: AtomicUsize::new(0),
            sequence: AtomicU64::new(0),
            _storage: storage,
        })))
    }

    /// Captures current owned-Context ancestry; no command or mutation gate.
    /// The binding is sealed and shares the explicit retained-intent capacity.
    pub fn bind_page(&self, page: ObjectId) -> Result<NativePageCloseBinding, Error> {
        self.0.ensure_live()?;
        let topology = self.0.topology.lock().map_err(|_| poisoned())?;
        let topology = topology.topology();
        if topology.epoch() != self.0.epoch {
            return Err(fail(
                FailureKind::TargetClosed,
                "native close authority epoch changed",
            ));
        }
        let route = topology
            .page_session_routes()
            .find(|route| route.object_id() == page)
            .ok_or_else(|| {
                fail(
                    FailureKind::TargetClosed,
                    "native close Page route is absent",
                )
            })?;
        let context = topology
            .browser_context_route(route.browser_context_object_id())
            .ok_or_else(|| {
                fail(
                    FailureKind::TargetClosed,
                    "native close Context route is absent",
                )
            })?;
        if context.ownership() != CdpContextOwnership::Owned {
            return Err(fail(
                FailureKind::UnsupportedCapability,
                "ancestor close authority requires an owned Context",
            ));
        }
        for value in [
            route.target_id(),
            route.session_id(),
            context.browser_context_id(),
        ] {
            self.0.validate_id(value)?;
        }
        let ticket = Ticket::new(self.0.clone())?;
        let ordinal = self
            .0
            .sequence
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_add(1))
            .map_err(|_| {
                fail(
                    FailureKind::ResourceExhausted,
                    "native close binding generation exhausted",
                )
            })?
            + 1;
        Ok(NativePageCloseBinding {
            ticket,
            ordinal,
            epoch: topology.epoch(),
            page,
            context: context.object_id(),
            native_context: context.browser_context_id().into(),
            target: route.target_id().into(),
            session: route.session_id().into(),
        })
    }
}

/// Exact Page/Context ancestry from a single attached topology. Not native death.
pub struct NativePageCloseBinding {
    ticket: Ticket,
    ordinal: u64,
    epoch: ConnectionEpoch,
    page: ObjectId,
    context: ObjectId,
    native_context: Box<str>,
    target: Box<str>,
    session: Box<str>,
}
impl NativePageCloseBinding {
    pub fn page(&self) -> ObjectId {
        self.page
    }
    pub fn target_id(&self) -> &str {
        &self.target
    }
    pub fn session_id(&self) -> &str {
        &self.session
    }
    pub fn context(&self) -> ObjectId {
        self.context
    }
    pub fn epoch(&self) -> ConnectionEpoch {
        self.epoch
    }
}

struct Pending(Arc<State>);
impl Drop for Pending {
    fn drop(&mut self) {
        self.0.pending.fetch_sub(1, Ordering::AcqRel);
    }
}
struct Ticket {
    _storage: ResourceLease,
    pending: Pending,
}
impl Ticket {
    fn new(state: Arc<State>) -> Result<Self, Error> {
        state.ensure_live()?;
        state
            .pending
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < usize::from(state.profile.pending.get())).then_some(n + 1)
            })
            .map_err(|_| {
                fail(
                    FailureKind::ResourceExhausted,
                    "native close authority capacity exhausted",
                )
            })?;
        let pending = Pending(state);
        let storage = pending
            .0
            .ledger
            .reserve(
                pending.0.profile.scope,
                ResourceClass::BackendStateBytes,
                INTENT_BYTES,
            )
            .map_err(resource)?;
        Ok(Self {
            pending,
            _storage: storage,
        })
    }
}
impl State {
    fn ensure_live(&self) -> Result<(), Error> {
        if self.connection.is_closed() || self.interruption.is_interrupted() {
            return Err(fail(
                FailureKind::TargetClosed,
                "native close authority owner terminated",
            ));
        }
        Ok(())
    }
    fn validate_id(&self, value: &str) -> Result<(), Error> {
        if value.is_empty() || value.len() > usize::from(self.profile.id_bytes.get()) {
            return Err(fail(
                FailureKind::ProtocolViolation,
                "native close route identity exceeds its declared bound",
            ));
        }
        Ok(())
    }
}
fn fail(kind: FailureKind, text: &str) -> Error {
    OperationError::new(kind, text, None)
}
fn poisoned() -> Error {
    fail(FailureKind::Backend, "native close topology lock poisoned")
}
fn resource(_: impl std::fmt::Display) -> Error {
    fail(
        FailureKind::ResourceExhausted,
        "native close authority resource admission failed",
    )
}

const _: () =
    assert!(std::mem::size_of::<NativePageCloseBinding>() + 3 * 1024 < INTENT_BYTES as usize);

#[cfg(test)]
mod tests;
