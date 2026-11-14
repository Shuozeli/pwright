use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use pwright_backend_cdp::{
    CdpFrameAttachment, CdpFrameDetachReason, CdpTopologyEventError, CdpTopologyProductionProfile,
    CdpTopologyUpdate, CdpWorldAttachment, ConnectionEpoch,
};
use pwright_cdp::connection::CdpError;
use pwright_cdp::generated::{page, runtime, target};
use pwright_cdp::{
    CdpClient, CdpConnection, CdpConnectionProductionProfile, CdpEvent, CdpEventReceiver,
    CdpEventRecvError, CdpEventSubscriptionError, CdpEventTryRecvError, SessionFactory,
};
use pwright_codec_browser_events::BrowserEventsCodec;
use pwright_core::{
    ControlMessage, ObjectId, ObjectIdSource, ObjectIdSourceError, ProductionResourceError,
    ProductionResourceLedger, ResourceClass, ResourceError, ResourceLease, ResourceRequirements,
    ResourceScope,
};
use pwright_runtime::{ControlledLifecycleHandle, RuntimeStateError};
use pwright_runtime_control::{ControlMessageSource, OrderedRuntimeControl};
use pwright_server_browser_events::{
    BrowserDisconnectedPublisher, BrowserDisconnectedPublisherState, BrowserNativeDisconnectSignal,
};
use pwright_server_wire::{
    ConcurrentServerBackend, EncodedServerSupervisor, ServerControlFrameHandler,
    ServerSupervisorCloseReason, ServerTransportCloseError,
};
use pwright_transport::{FrameTransport, ShutdownFrameTransport, WireCodec};

use crate::{
    ChromiumCompositionError, ChromiumNetworkEventDriverError,
    ChromiumNetworkPageRegistrationHandle, ChromiumTopologyPageEventFenceError,
    ChromiumTopologyPageEventPublication, ChromiumTopologyPageEventPublisher,
    ChromiumTopologyRuntime, SharedChromiumTopology, transact_topology_update,
};

pub(super) const UTILITY_WORLD_NAME_PREFIX: &str = "__pwright_utility_";
const FRAME_AUTO_ATTACH_TARGET_TYPES: &[&str] = &["page", "iframe"];
const FRAME_AND_WORKER_AUTO_ATTACH_TARGET_TYPES: &[&str] = &["page", "iframe", "worker"];

/// One opt-in auxiliary browser-root CDP session whose child attachments are
/// owned outside the portable topology. The owner registers the root session
/// before attaching any child and clears it after its children are retired.
#[derive(Debug)]
pub struct ChromiumAuxiliarySessionParent {
    max_native_id_bytes: usize,
    session_id: Mutex<Option<Box<str>>>,
    public_browser_sessions: Mutex<BTreeSet<Box<str>>>,
}

const MAX_PUBLIC_BROWSER_SESSIONS: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChromiumAuxiliarySessionParentError {
    InvalidNativeIdLimit,
    EmptySessionId,
    SessionIdTooLong { actual: usize, limit: usize },
    AlreadyRegistered,
    PublicSessionAlreadyRegistered,
    PublicSessionCapacityExceeded,
    StateUnavailable,
}

impl Display for ChromiumAuxiliarySessionParentError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNativeIdLimit => {
                formatter.write_str("auxiliary CDP session ID limit must be positive")
            }
            Self::EmptySessionId => {
                formatter.write_str("auxiliary CDP session ID must not be empty")
            }
            Self::SessionIdTooLong { actual, limit } => write!(
                formatter,
                "auxiliary CDP session ID has {actual} bytes, exceeding limit {limit}"
            ),
            Self::AlreadyRegistered => {
                formatter.write_str("an auxiliary CDP parent session is already registered")
            }
            Self::PublicSessionAlreadyRegistered => {
                formatter.write_str("public browser CDP session ID is already registered")
            }
            Self::PublicSessionCapacityExceeded => {
                formatter.write_str("public browser CDP session capacity exceeded")
            }
            Self::StateUnavailable => {
                formatter.write_str("auxiliary CDP parent session state is unavailable")
            }
        }
    }
}

impl Error for ChromiumAuxiliarySessionParentError {}

impl ChromiumAuxiliarySessionParent {
    pub fn new(max_native_id_bytes: usize) -> Result<Self, ChromiumAuxiliarySessionParentError> {
        if max_native_id_bytes == 0 {
            return Err(ChromiumAuxiliarySessionParentError::InvalidNativeIdLimit);
        }
        Ok(Self {
            max_native_id_bytes,
            session_id: Mutex::new(None),
            public_browser_sessions: Mutex::new(BTreeSet::new()),
        })
    }

    /// Register one exact parent. Repeating the same ID is idempotent; a
    /// different parent requires clearing the old owner first.
    pub fn register(&self, session_id: &str) -> Result<(), ChromiumAuxiliarySessionParentError> {
        self.validate_id(session_id)?;
        let mut active = self
            .session_id
            .lock()
            .map_err(|_| ChromiumAuxiliarySessionParentError::StateUnavailable)?;
        match active.as_deref() {
            Some(current) if current == session_id => Ok(()),
            Some(_) => Err(ChromiumAuxiliarySessionParentError::AlreadyRegistered),
            None => {
                let public = self
                    .public_browser_sessions
                    .lock()
                    .map_err(|_| ChromiumAuxiliarySessionParentError::StateUnavailable)?;
                if public.contains(session_id) {
                    return Err(ChromiumAuxiliarySessionParentError::AlreadyRegistered);
                }
                *active = Some(session_id.into());
                Ok(())
            }
        }
    }

    /// Clear only the matching parent, so a late retirement cannot clear a
    /// replacement session.
    pub fn clear(&self, session_id: &str) -> Result<bool, ChromiumAuxiliarySessionParentError> {
        self.validate_id(session_id)?;
        let mut active = self
            .session_id
            .lock()
            .map_err(|_| ChromiumAuxiliarySessionParentError::StateUnavailable)?;
        if active.as_deref() == Some(session_id) {
            *active = None;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Public browser-root sessions are independent of the private Page
    /// attachment parent. Track only their exact native IDs so nested Target
    /// events cannot be mistaken for Pwright-owned topology attachments.
    pub fn register_public_browser_session(
        &self,
        session_id: &str,
    ) -> Result<(), ChromiumAuxiliarySessionParentError> {
        self.validate_id(session_id)?;
        let private = self
            .session_id
            .lock()
            .map_err(|_| ChromiumAuxiliarySessionParentError::StateUnavailable)?;
        if private.as_deref() == Some(session_id) {
            return Err(ChromiumAuxiliarySessionParentError::PublicSessionAlreadyRegistered);
        }
        let mut sessions = self
            .public_browser_sessions
            .lock()
            .map_err(|_| ChromiumAuxiliarySessionParentError::StateUnavailable)?;
        if sessions.contains(session_id) {
            return Err(ChromiumAuxiliarySessionParentError::PublicSessionAlreadyRegistered);
        }
        if sessions.len() >= MAX_PUBLIC_BROWSER_SESSIONS {
            return Err(ChromiumAuxiliarySessionParentError::PublicSessionCapacityExceeded);
        }
        sessions.insert(session_id.into());
        Ok(())
    }

    pub fn clear_public_browser_session(
        &self,
        session_id: &str,
    ) -> Result<bool, ChromiumAuxiliarySessionParentError> {
        self.validate_id(session_id)?;
        self.public_browser_sessions
            .lock()
            .map_err(|_| ChromiumAuxiliarySessionParentError::StateUnavailable)
            .map(|mut sessions| sessions.remove(session_id))
    }

    fn matches(&self, session_id: &str) -> Result<bool, ChromiumAuxiliarySessionParentError> {
        let active = self
            .session_id
            .lock()
            .map_err(|_| ChromiumAuxiliarySessionParentError::StateUnavailable)?;
        if active.as_deref() == Some(session_id) {
            return Ok(true);
        }
        drop(active);
        self.public_browser_sessions
            .lock()
            .map_err(|_| ChromiumAuxiliarySessionParentError::StateUnavailable)
            .map(|sessions| sessions.contains(session_id))
    }

    fn validate_id(&self, session_id: &str) -> Result<(), ChromiumAuxiliarySessionParentError> {
        if session_id.is_empty() {
            return Err(ChromiumAuxiliarySessionParentError::EmptySessionId);
        }
        if session_id.len() > self.max_native_id_bytes {
            return Err(ChromiumAuxiliarySessionParentError::SessionIdTooLong {
                actual: session_id.len(),
                limit: self.max_native_id_bytes,
            });
        }
        Ok(())
    }
}

mod discovered_session;
mod synchronized_session;

pub use pwright_plugin_chromium_frame_observer::{
    ChromiumCommittedFrameOwnership, ChromiumContextMembership, ChromiumFrameOwnership,
    ChromiumFrameOwnershipView, ChromiumFrameUrlObserver, ChromiumPageMembership,
};
pub use pwright_plugin_chromium_session_initialization::{
    ChromiumSessionInitializationFuture, ChromiumSessionInitializationTarget,
    ChromiumSessionInitializer, ChromiumSessionResumeFence, ChromiumSessionTargetKind,
};

/// Explicit opt-in for Playwright-style per-Page isolated utility worlds.
///
/// The world name is derived from the connection-local portable Page ID, so
/// page-authored contexts cannot impersonate one global fixed name. This
/// profile creates no task and retains no Rust-side world-name table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumUtilityWorldProfile {
    grant_universal_access: bool,
}

impl ChromiumUtilityWorldProfile {
    /// Matches Playwright's Chromium utility-world access policy.
    pub const fn playwright_compatible() -> Self {
        Self {
            grant_universal_access: true,
        }
    }

    /// Creates the isolated world without Chromium's universal-access grant.
    pub const fn restricted() -> Self {
        Self {
            grant_universal_access: false,
        }
    }

    pub const fn grant_universal_access(self) -> bool {
        self.grant_universal_access
    }

    fn world_name(self, page_object_id: ObjectId) -> String {
        format!("{UTILITY_WORLD_NAME_PREFIX}{:016x}", page_object_id.get())
    }
}

/// Fixed memory declaration for caller-driven Chromium topology events.
///
/// The bound covers the retained initialized-session table, one complete
/// session-route snapshot, normalized frame attachments, duplicate-detection
/// identities, one temporary target-local client, and a
/// conservative four-times expansion of the largest admitted event and
/// frame-tree response while they are deserialized into generated types.
/// Opt-in paired prerequisites reserve a second response expansion; concurrent
/// utility-world registration reserves a third. Recursive discovery startup
/// admits seven simultaneous owners across two fixed phases. Page, Runtime,
/// lifecycle, script-registration and recursive-attachment prerequisites
/// finish while the target is paused. Caller settings, utility-world creation
/// and debugger resume then overlap, while every result remains a readiness
/// fence for a network-navigating popup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumTopologyEventDriverProductionProfile {
    resource_scope: ResourceScope,
    max_sessions: usize,
    max_contexts: usize,
    max_native_id_bytes: usize,
    max_response_bytes: usize,
    max_event_bytes: usize,
    event_channel_capacity: usize,
    pending_command_capacity: usize,
    parallel_session_prerequisites: bool,
    parallel_utility_world_registration: bool,
    parallel_recursive_attachment: bool,
    maximum_retained_bytes: u64,
    requirements: ResourceRequirements,
}

impl ChromiumTopologyEventDriverProductionProfile {
    pub fn from_profiles(
        resource_scope: ResourceScope,
        connection: &CdpConnectionProductionProfile,
        topology: CdpTopologyProductionProfile,
    ) -> Result<Self, ChromiumTopologyEventDriverProfileError> {
        if !matches!(resource_scope, ResourceScope::Extension { .. }) {
            return Err(
                ChromiumTopologyEventDriverProfileError::InvalidResourceScope { resource_scope },
            );
        }
        let limits = topology.limits();
        let max_sessions = limits
            .pages
            .checked_add(limits.frames)
            .ok_or(ChromiumTopologyEventDriverProfileError::CapacityOverflow)?;
        let session_bytes = u64::try_from(max_sessions)
            .ok()
            .and_then(|count| count.checked_mul(12))
            .and_then(|slots| slots.checked_mul(u64::try_from(limits.native_id_bytes).ok()?))
            .ok_or(ChromiumTopologyEventDriverProfileError::CapacityOverflow)?;
        let context_bytes = u64::try_from(limits.contexts)
            .ok()
            .and_then(|count| count.checked_add(1))
            .and_then(|count| count.checked_mul(u64::try_from(limits.native_id_bytes).ok()?))
            .ok_or(ChromiumTopologyEventDriverProfileError::CapacityOverflow)?;
        let event_bytes = u64::try_from(connection.config().max_event_bytes)
            .ok()
            .and_then(|bytes| bytes.checked_mul(4))
            .ok_or(ChromiumTopologyEventDriverProfileError::CapacityOverflow)?;
        let response_bytes = u64::try_from(connection.config().max_response_bytes)
            .ok()
            .and_then(|bytes| bytes.checked_mul(4))
            .ok_or(ChromiumTopologyEventDriverProfileError::CapacityOverflow)?;
        let maximum_retained_bytes = session_bytes
            .checked_add(context_bytes)
            .and_then(|bytes| bytes.checked_add(event_bytes))
            .and_then(|bytes| bytes.checked_add(response_bytes))
            .ok_or(ChromiumTopologyEventDriverProfileError::CapacityOverflow)?;
        let requirements = if maximum_retained_bytes == 0 {
            ResourceRequirements::none()
        } else {
            ResourceRequirements::none()
                .with_class(
                    ResourceClass::BackendStateBytes,
                    maximum_retained_bytes,
                    maximum_retained_bytes,
                )
                .with_metadata(1, 1)
        };
        Ok(Self {
            resource_scope,
            max_sessions,
            max_contexts: limits.contexts,
            max_native_id_bytes: limits.native_id_bytes,
            max_response_bytes: connection.config().max_response_bytes,
            max_event_bytes: connection.config().max_event_bytes,
            event_channel_capacity: connection.config().event_channel_capacity,
            pending_command_capacity: connection.config().pending_command_capacity,
            parallel_session_prerequisites: false,
            parallel_utility_world_registration: false,
            parallel_recursive_attachment: false,
            maximum_retained_bytes,
            requirements,
        })
    }

    pub const fn resource_scope(self) -> ResourceScope {
        self.resource_scope
    }

    pub const fn max_sessions(self) -> usize {
        self.max_sessions
    }

    pub const fn max_native_id_bytes(self) -> usize {
        self.max_native_id_bytes
    }

    pub const fn max_contexts(self) -> usize {
        self.max_contexts
    }

    pub const fn max_event_bytes(self) -> usize {
        self.max_event_bytes
    }

    pub const fn event_channel_capacity(self) -> usize {
        self.event_channel_capacity
    }

    pub const fn max_response_bytes(self) -> usize {
        self.max_response_bytes
    }

    pub const fn maximum_retained_bytes(self) -> u64 {
        self.maximum_retained_bytes
    }

    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }

    /// Polls frame-tree acquisition and Runtime enable together, for one
    /// session at a time. No task is spawned; both must finish before readiness.
    pub fn with_parallel_session_prerequisites(
        mut self,
    ) -> Result<Self, ChromiumTopologyEventDriverProfileError> {
        if self.pending_command_capacity < 2 {
            return Err(
                ChromiumTopologyEventDriverProfileError::InsufficientPendingCommands {
                    capacity: self.pending_command_capacity,
                },
            );
        }
        if !self.parallel_session_prerequisites {
            self.maximum_retained_bytes = u64::try_from(self.max_response_bytes)
                .ok()
                .and_then(|bytes| bytes.checked_mul(4))
                .and_then(|bytes| self.maximum_retained_bytes.checked_add(bytes))
                .ok_or(ChromiumTopologyEventDriverProfileError::CapacityOverflow)?;
            self.requirements = self.requirements.with_class(
                ResourceClass::BackendStateBytes,
                self.maximum_retained_bytes,
                self.maximum_retained_bytes,
            );
            self.parallel_session_prerequisites = true;
        }
        Ok(self)
    }

    pub const fn parallel_session_prerequisites(self) -> bool {
        self.parallel_session_prerequisites
    }

    /// Adds empty utility-world script registration to the paired prerequisites.
    /// Page enable and Context settings still precede this join; world creation,
    /// recursive attachment, resume and publication still follow all replies.
    /// The serial and paired-only profiles retain their original behavior.
    pub fn with_parallel_utility_world_registration(
        self,
    ) -> Result<Self, ChromiumTopologyEventDriverProfileError> {
        if self.pending_command_capacity < 3 {
            return Err(
                ChromiumTopologyEventDriverProfileError::InsufficientRegistrationCommands {
                    capacity: self.pending_command_capacity,
                },
            );
        }
        let mut profile = self.with_parallel_session_prerequisites()?;
        if !profile.parallel_utility_world_registration {
            profile.maximum_retained_bytes = u64::try_from(profile.max_response_bytes)
                .ok()
                .and_then(|bytes| bytes.checked_mul(4))
                .and_then(|bytes| profile.maximum_retained_bytes.checked_add(bytes))
                .ok_or(ChromiumTopologyEventDriverProfileError::CapacityOverflow)?;
            profile.requirements = profile.requirements.with_class(
                ResourceClass::BackendStateBytes,
                profile.maximum_retained_bytes,
                profile.maximum_retained_bytes,
            );
            profile.parallel_utility_world_registration = true;
        }
        Ok(profile)
    }

    pub const fn parallel_utility_world_registration(self) -> bool {
        self.parallel_utility_world_registration
    }

    /// Uses two fixed-width discovery phases with at most seven pending CDP
    /// commands. Runtime and frame-tree prerequisites complete while paused so
    /// a short-lived popup cannot execute before console observation exists.
    /// Caller settings and utility-world creation are polled before debugger
    /// resume in the second phase. No cross-session concurrency or independent
    /// initialization task is added.
    pub fn with_parallel_recursive_attachment(
        self,
    ) -> Result<Self, ChromiumTopologyEventDriverProfileError> {
        if self.pending_command_capacity < 7 {
            return Err(
                ChromiumTopologyEventDriverProfileError::InsufficientAttachmentCommands {
                    capacity: self.pending_command_capacity,
                },
            );
        }
        let mut profile = self.with_parallel_utility_world_registration()?;
        if !profile.parallel_recursive_attachment {
            profile.maximum_retained_bytes = u64::try_from(profile.max_response_bytes)
                .ok()
                .and_then(|bytes| bytes.checked_mul(8))
                .and_then(|bytes| profile.maximum_retained_bytes.checked_add(bytes))
                .ok_or(ChromiumTopologyEventDriverProfileError::CapacityOverflow)?;
            profile.requirements = profile.requirements.with_class(
                ResourceClass::BackendStateBytes,
                profile.maximum_retained_bytes,
                profile.maximum_retained_bytes,
            );
            profile.parallel_recursive_attachment = true;
        }
        Ok(profile)
    }

    pub const fn parallel_recursive_attachment(self) -> bool {
        self.parallel_recursive_attachment
    }

    pub fn matches(
        self,
        connection: &CdpConnectionProductionProfile,
        topology: CdpTopologyProductionProfile,
    ) -> bool {
        let limits = topology.limits();
        limits.pages.checked_add(limits.frames) == Some(self.max_sessions)
            && limits.contexts == self.max_contexts
            && limits.native_id_bytes == self.max_native_id_bytes
            && connection.config().max_response_bytes == self.max_response_bytes
            && connection.config().max_event_bytes == self.max_event_bytes
            && connection.config().event_channel_capacity == self.event_channel_capacity
            && connection.config().pending_command_capacity == self.pending_command_capacity
    }
}

/// Optional browser-root target discovery. This marker retains no independent
/// variable-size state: discovered Context/Page/Frame identities are charged
/// to topology and frame-tree decoding is charged to the event driver.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChromiumPageDiscoveryProductionProfile {
    max_contexts: usize,
    max_pages: usize,
    max_frames: usize,
    max_native_id_bytes: usize,
    max_response_bytes: usize,
}

impl ChromiumPageDiscoveryProductionProfile {
    pub fn from_profiles(
        connection: &CdpConnectionProductionProfile,
        topology: CdpTopologyProductionProfile,
    ) -> Self {
        let limits = topology.limits();
        Self {
            max_contexts: limits.contexts,
            max_pages: limits.pages,
            max_frames: limits.frames,
            max_native_id_bytes: limits.native_id_bytes,
            max_response_bytes: connection.config().max_response_bytes,
        }
    }

    pub fn matches(
        self,
        connection: &CdpConnectionProductionProfile,
        topology: CdpTopologyProductionProfile,
    ) -> bool {
        let limits = topology.limits();
        self.max_pages == limits.pages
            && self.max_contexts == limits.contexts
            && self.max_frames == limits.frames
            && self.max_native_id_bytes == limits.native_id_bytes
            && self.max_response_bytes == connection.config().max_response_bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChromiumTopologyEventDriverProfileError {
    InvalidResourceScope { resource_scope: ResourceScope },
    InsufficientPendingCommands { capacity: usize },
    InsufficientRegistrationCommands { capacity: usize },
    InsufficientAttachmentCommands { capacity: usize },
    CapacityOverflow,
}

impl Display for ChromiumTopologyEventDriverProfileError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidResourceScope { resource_scope } => write!(
                formatter,
                "Chromium topology events require an extension resource scope, got `{resource_scope}`"
            ),
            Self::CapacityOverflow => {
                formatter.write_str("Chromium topology-event capacity calculation overflowed")
            }
            Self::InsufficientPendingCommands { capacity } => write!(
                formatter,
                "parallel session prerequisites require at least two pending CDP commands, got {capacity}"
            ),
            Self::InsufficientRegistrationCommands { capacity } => write!(
                formatter,
                "parallel utility-world registration requires at least three pending CDP commands, got {capacity}"
            ),
            Self::InsufficientAttachmentCommands { capacity } => write!(
                formatter,
                "parallel recursive attachment and debugger resume require at least seven pending CDP commands, got {capacity}"
            ),
        }
    }
}

impl Error for ChromiumTopologyEventDriverProfileError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChromiumTopologyEventDriverOutcome {
    Ignored,
    Updated(CdpTopologyUpdate),
}

#[derive(Debug)]
pub enum ChromiumTopologyEventDriverError {
    FrameEventObserver(Box<dyn Error + Send + Sync>),
    SessionInitialization(Box<dyn Error + Send + Sync>),
    SessionContextUnavailable,
    InitialSnapshotLimitExceeded {
        limit: usize,
    },
    AlreadyInstalled,
    AuxiliarySessionParent(ChromiumAuxiliarySessionParentError),
    EventSubscription(CdpEventSubscriptionError),
    EventStreamClosed,
    ResyncRequired {
        skipped: u64,
    },
    ResyncNotRequired,
    PageDiscoveryRequiredForResync,
    MissingEventSession {
        method: &'static str,
    },
    StaleEpoch {
        driver: ConnectionEpoch,
        current: ConnectionEpoch,
    },
    SessionCapacityExceeded {
        actual: usize,
        limit: usize,
    },
    ContextCapacityExceeded {
        actual: usize,
        limit: usize,
    },
    NativeIdTooLong {
        field: &'static str,
        actual: usize,
        limit: usize,
    },
    EmptyNativeId {
        field: &'static str,
    },
    FrameTreeRootHasParent {
        frame_id: String,
    },
    FrameTreeParentMismatch {
        frame_id: String,
        expected: String,
        actual: String,
    },
    DuplicateFrameTreeId {
        frame_id: String,
    },
    UtilityWorldPageUnavailable {
        page_target_id: String,
    },
    Decode {
        method: &'static str,
        error: serde_json::Error,
    },
    TopologyEvent(CdpTopologyEventError),
    ObjectIds(ObjectIdSourceError),
    Cdp(CdpError),
    Composition(ChromiumCompositionError),
    Resources(ProductionResourceError),
    Resource(ResourceError),
    Network(ChromiumNetworkEventDriverError),
    FrameUrlObserver(Box<dyn Error + Send + Sync>),
    PageDiscoveryCleanup {
        primary: String,
        cleanup: String,
    },
}

impl Display for ChromiumTopologyEventDriverError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::FrameEventObserver(error) => Display::fmt(error, formatter),
            Self::SessionInitialization(error) => Display::fmt(error, formatter),
            Self::SessionContextUnavailable => {
                formatter.write_str("session has no registered browser Context")
            }
            Self::InitialSnapshotLimitExceeded { limit } => write!(
                formatter,
                "initial Chromium topology exceeded the {limit}-event snapshot budget"
            ),
            Self::AlreadyInstalled => formatter
                .write_str("this Chromium attachment already owns its topology-event driver"),
            Self::AuxiliarySessionParent(error) => Display::fmt(error, formatter),
            Self::EventSubscription(error) => Display::fmt(error, formatter),
            Self::EventStreamClosed => formatter.write_str("CDP topology event stream closed"),
            Self::ResyncRequired { skipped } => write!(
                formatter,
                "CDP topology event consumer lagged by {skipped} events; a full topology resync is required"
            ),
            Self::ResyncNotRequired => formatter
                .write_str("CDP topology reconstruction requires a prior event-loss signal"),
            Self::PageDiscoveryRequiredForResync => formatter.write_str(
                "CDP topology reconstruction requires the Chromium Page-discovery profile",
            ),
            Self::MissingEventSession { method } => {
                write!(formatter, "CDP `{method}` event has no target session")
            }
            Self::StaleEpoch { driver, current } => write!(
                formatter,
                "topology-event driver epoch {} is stale; current topology epoch is {}",
                driver.get(),
                current.get()
            ),
            Self::SessionCapacityExceeded { actual, limit } => write!(
                formatter,
                "topology-event session count {actual} exceeds limit {limit}"
            ),
            Self::ContextCapacityExceeded { actual, limit } => write!(
                formatter,
                "CDP non-default browser-context count {actual} exceeds topology-event limit {limit}"
            ),
            Self::NativeIdTooLong {
                field,
                actual,
                limit,
            } => write!(
                formatter,
                "{field} has {actual} bytes, exceeding topology-event limit {limit}"
            ),
            Self::EmptyNativeId { field } => write!(formatter, "{field} must not be empty"),
            Self::FrameTreeRootHasParent { frame_id } => write!(
                formatter,
                "CDP frame tree root `{frame_id}` unexpectedly has a parent"
            ),
            Self::FrameTreeParentMismatch {
                frame_id,
                expected,
                actual,
            } => write!(
                formatter,
                "CDP frame `{frame_id}` reports parent `{actual}`, expected `{expected}` from the frame tree"
            ),
            Self::DuplicateFrameTreeId { frame_id } => {
                write!(formatter, "CDP frame tree repeats frame ID `{frame_id}`")
            }
            Self::UtilityWorldPageUnavailable { page_target_id } => write!(
                formatter,
                "cannot derive a Chromium utility-world name before Page target `{page_target_id}` is materialized"
            ),
            Self::Decode { method, error } => {
                write!(formatter, "could not decode CDP `{method}` event: {error}")
            }
            Self::TopologyEvent(error) => Display::fmt(error, formatter),
            Self::ObjectIds(error) => Display::fmt(error, formatter),
            Self::Cdp(error) => Display::fmt(error, formatter),
            Self::Composition(error) => Display::fmt(error, formatter),
            Self::Resources(error) => Display::fmt(error, formatter),
            Self::Resource(error) => Display::fmt(error, formatter),
            Self::Network(error) => Display::fmt(error, formatter),
            Self::FrameUrlObserver(error) => Display::fmt(error, formatter),
            Self::PageDiscoveryCleanup { primary, cleanup } => write!(
                formatter,
                "Page discovery failed ({primary}) and cleanup also failed: {cleanup}"
            ),
        }
    }
}

impl Error for ChromiumTopologyEventDriverError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::FrameEventObserver(error) => Some(error.as_ref()),
            Self::SessionInitialization(error) => Some(error.as_ref()),
            Self::AuxiliarySessionParent(error) => Some(error),
            Self::Decode { error, .. } => Some(error),
            Self::EventSubscription(error) => Some(error),
            Self::TopologyEvent(error) => Some(error),
            Self::ObjectIds(error) => Some(error),
            Self::Cdp(error) => Some(error),
            Self::Composition(error) => Some(error),
            Self::Resources(error) => Some(error),
            Self::Resource(error) => Some(error),
            Self::Network(error) => Some(error),
            Self::FrameUrlObserver(error) => Some(error.as_ref()),
            _ => None,
        }
    }
}

impl From<CdpTopologyEventError> for ChromiumTopologyEventDriverError {
    fn from(error: CdpTopologyEventError) -> Self {
        Self::TopologyEvent(error)
    }
}

impl From<ObjectIdSourceError> for ChromiumTopologyEventDriverError {
    fn from(error: ObjectIdSourceError) -> Self {
        Self::ObjectIds(error)
    }
}

impl From<CdpError> for ChromiumTopologyEventDriverError {
    fn from(error: CdpError) -> Self {
        Self::Cdp(error)
    }
}

impl From<CdpEventSubscriptionError> for ChromiumTopologyEventDriverError {
    fn from(error: CdpEventSubscriptionError) -> Self {
        Self::EventSubscription(error)
    }
}

impl From<ChromiumCompositionError> for ChromiumTopologyEventDriverError {
    fn from(error: ChromiumCompositionError) -> Self {
        Self::Composition(error)
    }
}

impl From<ProductionResourceError> for ChromiumTopologyEventDriverError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Resources(error)
    }
}

impl From<ResourceError> for ChromiumTopologyEventDriverError {
    fn from(error: ResourceError) -> Self {
        Self::Resource(error)
    }
}

impl From<ChromiumNetworkEventDriverError> for ChromiumTopologyEventDriverError {
    fn from(error: ChromiumNetworkEventDriverError) -> Self {
        Self::Network(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SessionRoute {
    session_id: String,
    target_id: String,
    page_target_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct InitializedSession {
    target_id: String,
    discovered_page: bool,
    resume_after_dialog: bool,
}

fn outcome_update(outcome: ChromiumTopologyEventDriverOutcome) -> CdpTopologyUpdate {
    match outcome {
        ChromiumTopologyEventDriverOutcome::Ignored => CdpTopologyUpdate::idempotent(),
        ChromiumTopologyEventDriverOutcome::Updated(update) => update,
    }
}

fn update_outcome(update: CdpTopologyUpdate) -> ChromiumTopologyEventDriverOutcome {
    if update.disposition() == pwright_backend_cdp::CdpTopologyDisposition::Idempotent
        && update.effects().is_empty()
    {
        ChromiumTopologyEventDriverOutcome::Ignored
    } else {
        ChromiumTopologyEventDriverOutcome::Updated(update)
    }
}

/// Explicit, caller-driven bridge from generated CDP events to portable
/// topology and runtime state.
///
/// This type never spawns. One call consumes at most one selected CDP event;
/// callers may sequence it with cancellation, deadlines, or their own
/// executor policy.
pub struct ChromiumTopologyEventDriver {
    receiver: CdpEventReceiver,
    browser_client: Arc<dyn CdpClient>,
    session_factory: Arc<dyn SessionFactory>,
    topology: SharedChromiumTopology,
    object_ids: Arc<dyn ObjectIdSource>,
    epoch: ConnectionEpoch,
    initialized_sessions: BTreeMap<String, InitializedSession>,
    nondefault_context_ids: BTreeSet<String>,
    default_native_context_id: Option<String>,
    browser_initialized: bool,
    resync_required: Option<u64>,
    page_discovery: Option<ChromiumPageDiscoveryProductionProfile>,
    utility_world: Option<ChromiumUtilityWorldProfile>,
    network_registration: Option<ChromiumNetworkPageRegistrationHandle>,
    frame_urls: Option<Arc<dyn ChromiumFrameUrlObserver>>,
    frame_events: Option<Arc<dyn crate::ChromiumFrameEventObserver>>,
    session_initializer: Option<Arc<dyn ChromiumSessionInitializer>>,
    auxiliary_session_parent: Option<Arc<ChromiumAuxiliarySessionParent>>,
    playwright_context_fallback: bool,
    profile: ChromiumTopologyEventDriverProductionProfile,
    _resource: Option<ResourceLease>,
}

impl Drop for ChromiumTopologyEventDriver {
    fn drop(&mut self) {
        self.seal_frame_events();
    }
}

impl ChromiumTopologyEventDriver {
    fn auto_attach_target_types(&self) -> &'static [&'static str] {
        if self
            .frame_events
            .as_ref()
            .is_some_and(|observer| observer.retains_worker_targets())
        {
            FRAME_AND_WORKER_AUTO_ATTACH_TARGET_TYPES
        } else {
            FRAME_AUTO_ATTACH_TARGET_TYPES
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_with_production_resources(
        receiver: impl Into<CdpEventReceiver>,
        browser_client: Arc<dyn CdpClient>,
        session_factory: Arc<dyn SessionFactory>,
        topology: SharedChromiumTopology,
        object_ids: Arc<dyn ObjectIdSource>,
        epoch: ConnectionEpoch,
        profile: ChromiumTopologyEventDriverProductionProfile,
        page_discovery: Option<ChromiumPageDiscoveryProductionProfile>,
        utility_world: Option<ChromiumUtilityWorldProfile>,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumTopologyEventDriverError> {
        let ledger = resources.claim(profile.requirements())?;
        let resource = if profile.maximum_retained_bytes() == 0 {
            None
        } else {
            Some(ledger.reserve(
                profile.resource_scope(),
                ResourceClass::BackendStateBytes,
                profile.maximum_retained_bytes(),
            )?)
        };
        Ok(Self {
            receiver: receiver.into(),
            browser_client,
            session_factory,
            topology,
            object_ids,
            epoch,
            initialized_sessions: BTreeMap::new(),
            nondefault_context_ids: BTreeSet::new(),
            default_native_context_id: None,
            browser_initialized: false,
            resync_required: None,
            page_discovery,
            utility_world,
            network_registration: None,
            frame_urls: None,
            frame_events: None,
            session_initializer: None,
            auxiliary_session_parent: None,
            playwright_context_fallback: false,
            profile,
            _resource: resource,
        })
    }

    pub const fn profile(&self) -> ChromiumTopologyEventDriverProductionProfile {
        self.profile
    }

    /// Exclude child attachments reported through one explicitly owned
    /// browser-root CDP session from portable Page discovery. The handle may
    /// register its exact parent ID after browser initialization.
    pub fn with_auxiliary_session_parent(
        mut self,
        parent: Arc<ChromiumAuxiliarySessionParent>,
    ) -> Result<Self, ChromiumTopologyEventDriverError> {
        if self.browser_initialized || self.auxiliary_session_parent.is_some() {
            return Err(ChromiumTopologyEventDriverError::AlreadyInstalled);
        }
        self.auxiliary_session_parent = Some(parent);
        Ok(self)
    }

    /// Install before any session initialization; rejection never replaces an
    /// existing observer. Sealed owners must be replaced with a new driver.
    pub fn with_frame_event_observer(
        mut self,
        observer: Arc<dyn crate::ChromiumFrameEventObserver>,
    ) -> Result<Self, ChromiumTopologyEventDriverError> {
        if self.browser_initialized
            || !self.initialized_sessions.is_empty()
            || self.frame_events.is_some()
        {
            return Err(ChromiumTopologyEventDriverError::AlreadyInstalled);
        }
        self.frame_events = Some(observer);
        Ok(self)
    }

    fn seal_frame_events(&self) {
        if let Some(observer) = &self.frame_events {
            observer.seal();
        }
    }

    async fn enable_frame_events(
        &self,
        client: &dyn CdpClient,
        page: ObjectId,
    ) -> Result<(), ChromiumTopologyEventDriverError> {
        let observes_page = self
            .frame_events
            .as_ref()
            .is_some_and(|observer| observer.observes_page(page));
        if observes_page {
            if let Some(observer) = &self.frame_events {
                let topology = self
                    .topology
                    .lock()
                    .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
                observer
                    .prepare_page(topology.topology(), page)
                    .map_err(ChromiumTopologyEventDriverError::FrameEventObserver)?;
            }
            tokio::try_join!(
                client.network_enable(),
                client.page_set_lifecycle_events_enabled(true)
            )?;
        }
        Ok(())
    }

    pub fn initialized_session_count(&self) -> usize {
        self.initialized_sessions.len()
    }

    pub const fn epoch(&self) -> ConnectionEpoch {
        self.epoch
    }

    /// Install before initializing the snapshot. URL changes share this
    /// driver's ordered cursor and topology lock, including initial discovery.
    pub fn with_frame_url_observer(mut self, observer: Arc<dyn ChromiumFrameUrlObserver>) -> Self {
        self.frame_urls = Some(observer);
        self
    }

    pub fn with_session_initializer(
        mut self,
        initializer: Arc<dyn ChromiumSessionInitializer>,
    ) -> Self {
        self.session_initializer = Some(initializer);
        self
    }

    /// Match Playwright's CDP attachment view: Pages in foreign, non-default
    /// contexts are attributed to this connection's default Context. This
    /// does not acquire the foreign Context or grant authority to dispose it.
    /// Strict foreign-context exclusion remains the default for other callers.
    pub fn with_playwright_context_fallback(mut self) -> Self {
        self.playwright_context_fallback = true;
        self
    }

    fn foreign_context_key(&self) -> Result<Option<String>, ChromiumTopologyEventDriverError> {
        if self.playwright_context_fallback {
            self.default_context_key()
        } else {
            Ok(None)
        }
    }

    /// Makes newly discovered Pages join the installed single-cursor network
    /// driver before their portable lifecycle becomes visible. The handle is
    /// non-owning, so dropping the network driver still terminates its lanes.
    pub fn with_network_registration(
        mut self,
        registration: ChromiumNetworkPageRegistrationHandle,
    ) -> Self {
        self.network_registration = Some(registration);
        self
    }

    /// Returns the number of known skipped events while authoritative
    /// reconstruction is required.
    pub const fn resync_required(&self) -> Option<u64> {
        self.resync_required
    }

    /// Initializes the default Context and drains the bounded initial target
    /// announcements without waiting for an unrelated future browser event.
    /// The browser command barrier in initialization orders existing targets.
    /// Callers must keep this driver for subsequent topology updates.
    pub async fn initialize_snapshot<Runtime>(
        &mut self,
        runtime: &Runtime,
        max_events: usize,
    ) -> Result<ChromiumTopologyEventDriverOutcome, ChromiumTopologyEventDriverError>
    where
        Runtime: ChromiumTopologyRuntime,
    {
        let mut attempt = crate::frame_event_observer::ObservationAttempt::new(&self.frame_events);
        let result = self.initialize_snapshot_inner(runtime, max_events).await;
        if result.is_ok() {
            attempt.completed();
        }
        result
    }

    async fn initialize_snapshot_inner<Runtime>(
        &mut self,
        runtime: &Runtime,
        max_events: usize,
    ) -> Result<ChromiumTopologyEventDriverOutcome, ChromiumTopologyEventDriverError>
    where
        Runtime: ChromiumTopologyRuntime,
    {
        if let Some(skipped) = self.resync_required {
            return Err(ChromiumTopologyEventDriverError::ResyncRequired { skipped });
        }
        let mut combined = self.initialize_browser(runtime).await?;
        combined = combined.combine(self.synchronize_sessions(runtime).await?);
        for processed in 0..=max_events {
            let event = match self.receiver.try_recv() {
                Ok(event) => event,
                Err(CdpEventTryRecvError::Empty) => {
                    // Lifecycle services can commit topology directly. Project
                    // their final state even when no subsequent event arrived.
                    self.refresh_frame_ownership()?;
                    return Ok(update_outcome(combined));
                }
                Err(CdpEventTryRecvError::Closed) => {
                    return Err(ChromiumTopologyEventDriverError::EventStreamClosed);
                }
                Err(CdpEventTryRecvError::Lagged(skipped)) => {
                    self.resync_required = Some(skipped);
                    return Err(ChromiumTopologyEventDriverError::ResyncRequired { skipped });
                }
            };
            if processed == max_events {
                self.resync_required = Some(1);
                return Err(
                    ChromiumTopologyEventDriverError::InitialSnapshotLimitExceeded {
                        limit: max_events,
                    },
                );
            }
            combined = combined.combine(outcome_update(self.apply_event(runtime, &event).await?));
            combined = combined.combine(self.synchronize_sessions(runtime).await?);
        }
        unreachable!("bounded snapshot loop returns after its final iteration")
    }

    pub async fn receive_once<Runtime>(
        &mut self,
        runtime: &Runtime,
    ) -> Result<ChromiumTopologyEventDriverOutcome, ChromiumTopologyEventDriverError>
    where
        Runtime: ChromiumTopologyRuntime,
    {
        self.receive_once_or_signal(runtime, std::future::pending())
            .await
    }

    /// A caller-owned signal can interrupt only the idle event receive, never
    /// an in-progress topology transaction or target initialization. Callers
    /// may then drain the bounded snapshot to establish a command-return fence.
    pub async fn receive_once_or_signal<Runtime>(
        &mut self,
        runtime: &Runtime,
        signal: impl Future<Output = ()>,
    ) -> Result<ChromiumTopologyEventDriverOutcome, ChromiumTopologyEventDriverError>
    where
        Runtime: ChromiumTopologyRuntime,
    {
        let mut attempt = crate::frame_event_observer::ObservationAttempt::new(&self.frame_events);
        let result = self.receive_once_or_signal_inner(runtime, signal).await;
        if result.is_ok() {
            attempt.completed();
        }
        result
    }

    async fn receive_once_or_signal_inner<Runtime>(
        &mut self,
        runtime: &Runtime,
        signal: impl Future<Output = ()>,
    ) -> Result<ChromiumTopologyEventDriverOutcome, ChromiumTopologyEventDriverError>
    where
        Runtime: ChromiumTopologyRuntime,
    {
        if let Some(skipped) = self.resync_required {
            return Err(ChromiumTopologyEventDriverError::ResyncRequired { skipped });
        }
        let mut combined = self.initialize_browser(runtime).await?;
        combined = combined.combine(self.synchronize_sessions(runtime).await?);
        let received = tokio::select! {
            biased;
            () = signal => return Ok(update_outcome(combined)),
            received = self.receiver.recv() => received,
        };
        let event = match received {
            Ok(event) => event,
            Err(CdpEventRecvError::Closed) => {
                return Err(ChromiumTopologyEventDriverError::EventStreamClosed);
            }
            Err(CdpEventRecvError::Lagged(skipped)) => {
                self.resync_required = Some(skipped);
                return Err(ChromiumTopologyEventDriverError::ResyncRequired { skipped });
            }
        };
        combined = combined.combine(outcome_update(self.apply_event(runtime, &event).await?));
        combined = combined.combine(self.synchronize_sessions(runtime).await?);
        Ok(update_outcome(combined))
    }

    /// Rebuilds the complete browser topology after this driver's bounded
    /// event receiver reports loss.
    ///
    /// The caller supplies a strictly newer epoch. Auto-attach is disabled,
    /// the event subscription is replaced, then auto-attach is re-enabled so
    /// Chromium republishes every existing related target. The ordering
    /// barrier bounds that snapshot; only after its queued events are applied
    /// are missing Context, Page, and Frame objects disposed. This method does
    /// not spawn and cannot be called speculatively without prior event loss.
    pub async fn resynchronize_after_event_loss<Runtime>(
        &mut self,
        runtime: &Runtime,
        new_epoch: ConnectionEpoch,
    ) -> Result<ChromiumTopologyEventDriverOutcome, ChromiumTopologyEventDriverError>
    where
        Runtime: ChromiumTopologyRuntime,
    {
        self.seal_frame_events();
        let skipped = self
            .resync_required
            .ok_or(ChromiumTopologyEventDriverError::ResyncNotRequired)?;
        if self.page_discovery.is_none() {
            return Err(ChromiumTopologyEventDriverError::PageDiscoveryRequiredForResync);
        }
        self.ensure_topology_epoch()?;

        self.browser_client
            .target_set_auto_attach_types(false, false, true, self.auto_attach_target_types())
            .await?;
        self.receiver.resubscribe();

        let mut combined =
            self.transact_update(runtime, |topology| topology.begin_reconnect(new_epoch))?;
        self.epoch = new_epoch;
        self.initialized_sessions.clear();
        self.nondefault_context_ids.clear();
        self.default_native_context_id = None;
        self.browser_initialized = false;

        combined = combined.combine(self.initialize_browser(runtime).await?);
        loop {
            let event = match self.receiver.try_recv() {
                Ok(event) => event,
                Err(CdpEventTryRecvError::Empty) => break,
                Err(CdpEventTryRecvError::Closed) => {
                    return Err(ChromiumTopologyEventDriverError::EventStreamClosed);
                }
                Err(CdpEventTryRecvError::Lagged(additional)) => {
                    let skipped = skipped.saturating_add(additional);
                    self.resync_required = Some(skipped);
                    return Err(ChromiumTopologyEventDriverError::ResyncRequired { skipped });
                }
            };
            combined = combined.combine(outcome_update(self.apply_event(runtime, &event).await?));
            combined = combined.combine(self.synchronize_sessions(runtime).await?);
        }
        combined = combined.combine(self.synchronize_sessions(runtime).await?);
        combined = combined.combine(
            self.transact_update(runtime, pwright_backend_cdp::CdpTopology::finish_reconnect)?,
        );
        if let Some(registration) = &self.network_registration {
            registration
                .reconcile_after_reconnect(runtime.page_services())
                .map_err(ChromiumTopologyEventDriverError::Network)?;
        }
        self.resync_required = None;
        Ok(update_outcome(combined))
    }

    async fn initialize_browser<Runtime>(
        &mut self,
        runtime: &Runtime,
    ) -> Result<CdpTopologyUpdate, ChromiumTopologyEventDriverError>
    where
        Runtime: ChromiumTopologyRuntime,
    {
        if self.page_discovery.is_none() || self.browser_initialized {
            return Ok(CdpTopologyUpdate::idempotent());
        }
        self.browser_client
            .target_set_auto_attach_types(true, true, true, self.auto_attach_target_types())
            .await?;
        self.browser_client.target_get_target_info().await?;
        self.refresh_nondefault_contexts().await?;
        let existing = {
            let topology = self
                .topology
                .lock()
                .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
            self.ensure_current_epoch(topology.topology().epoch())?;
            topology
                .topology()
                .default_context_route()
                .map(|route| route.object_id())
        };
        let object_id = match existing {
            Some(object_id) => object_id,
            None => self.object_ids.next_object_id()?,
        };
        let context_ids = self.nondefault_context_ids.clone();
        let epoch = self.epoch;
        let update = self.transact_update(runtime, move |topology| {
            let mut update = topology.register_default_context(epoch, object_id)?;
            for context_id in context_ids {
                update = update.combine(topology.observe_context(epoch, &context_id)?);
            }
            Ok(update)
        })?;
        self.browser_initialized = true;
        Ok(update)
    }

    async fn refresh_nondefault_contexts(
        &mut self,
    ) -> Result<(), ChromiumTopologyEventDriverError> {
        let context_ids = self.browser_client.target_get_browser_contexts().await?;
        if context_ids.len() > self.profile.max_contexts() {
            return Err(ChromiumTopologyEventDriverError::ContextCapacityExceeded {
                actual: context_ids.len(),
                limit: self.profile.max_contexts(),
            });
        }
        let mut next = BTreeSet::new();
        for context_id in context_ids {
            if context_id.is_empty() {
                return Err(ChromiumTopologyEventDriverError::EmptyNativeId {
                    field: "browser context ID",
                });
            }
            if context_id.len() > self.profile.max_native_id_bytes() {
                return Err(ChromiumTopologyEventDriverError::NativeIdTooLong {
                    field: "browser context ID",
                    actual: context_id.len(),
                    limit: self.profile.max_native_id_bytes(),
                });
            }
            next.insert(context_id.into_boxed_str().into_string());
        }
        self.nondefault_context_ids = next;
        Ok(())
    }

    async fn synchronize_sessions<Runtime>(
        &mut self,
        runtime: &Runtime,
    ) -> Result<CdpTopologyUpdate, ChromiumTopologyEventDriverError>
    where
        Runtime: ChromiumTopologyRuntime,
    {
        let mut combined = CdpTopologyUpdate::idempotent();
        let routes = self.session_routes()?;
        self.initialized_sessions.retain(|session_id, initialized| {
            routes.iter().any(|route| {
                route.session_id == *session_id && route.target_id == initialized.target_id
            })
        });
        for route in routes {
            // A preceding parent retirement may have removed another route
            // from this bounded snapshot. Never initialize that stale child.
            if !self.session_route_is_current(&route)? {
                continue;
            }
            if self
                .initialized_sessions
                .get(&route.session_id)
                .is_some_and(|initialized| initialized.target_id == route.target_id)
            {
                continue;
            }
            let frame_tree = match self.initialize_synchronized_session(&route).await {
                Ok(tree) => tree,
                Err(error) if discovered_session::missing_native_session(&error) => {
                    let retired =
                        self.retire_session(runtime, &route.session_id, Some(&route.target_id))?;
                    combined = combined.combine(outcome_update(retired));
                    continue;
                }
                Err(error) => return Err(error),
            };
            combined = combined.combine(self.initialize_frame_tree(runtime, &route, frame_tree)?);
            self.initialized_sessions.insert(
                route.session_id,
                InitializedSession {
                    target_id: route.target_id,
                    discovered_page: false,
                    resume_after_dialog: false,
                },
            );
        }
        Ok(combined)
    }

    fn utility_world_name_for_route(
        &self,
        route: &SessionRoute,
    ) -> Result<Option<String>, ChromiumTopologyEventDriverError> {
        let Some(profile) = self.utility_world else {
            return Ok(None);
        };
        let page_object_id = self
            .topology
            .lock()
            .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?
            .topology()
            .page_object_id(&route.page_target_id)
            .ok_or_else(
                || ChromiumTopologyEventDriverError::UtilityWorldPageUnavailable {
                    page_target_id: route.page_target_id.clone(),
                },
            )?;
        Ok(Some(profile.world_name(page_object_id)))
    }

    async fn initialize_context_session(
        &self,
        route: &SessionRoute,
        context: ObjectId,
        page: ObjectId,
        resume_fence: Option<&ChromiumSessionResumeFence>,
    ) -> Result<(), ChromiumTopologyEventDriverError> {
        let Some(initializer) = &self.session_initializer else {
            return Ok(());
        };
        let ownership = self
            .topology
            .lock()
            .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?
            .topology()
            .browser_context_route(context)
            .map(|context| context.ownership())
            .ok_or(ChromiumTopologyEventDriverError::SessionContextUnavailable)?;
        initializer
            .initialize_scoped(ChromiumSessionInitializationTarget {
                session_id: &route.session_id,
                target_id: &route.target_id,
                page_target_id: &route.page_target_id,
                page,
                context,
                ownership,
                kind: if route.target_id == route.page_target_id {
                    ChromiumSessionTargetKind::Page
                } else {
                    ChromiumSessionTargetKind::OutOfProcessFrame
                },
                resume_fence,
            })
            .await
            .map_err(ChromiumTopologyEventDriverError::SessionInitialization)
    }

    fn utility_world_name_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<String>, ChromiumTopologyEventDriverError> {
        let Some(route) = self.route_for_session(session_id)? else {
            return Ok(None);
        };
        self.utility_world_name_for_route(&route)
    }

    async fn initialize_session_prerequisites(
        &self,
        client: &dyn CdpClient,
        world_name: Option<&str>,
        attachment_started: bool,
    ) -> Result<page::FrameTree, ChromiumTopologyEventDriverError> {
        let parallel_registration = self.profile.parallel_utility_world_registration();
        let resume_with_prerequisites = self.resumes_with_utility_worlds();
        let frame_tree = if self.profile.parallel_recursive_attachment() {
            let prerequisites = async {
                let attachment = async {
                    if !attachment_started {
                        client
                            .target_set_auto_attach_types(
                                true,
                                self.page_discovery.is_some(),
                                true,
                                self.auto_attach_target_types(),
                            )
                            .await?;
                    }
                    Ok(())
                };
                crate::session_prerequisites::initialize_with_attachment(
                    client.page_get_frame_tree(),
                    client.runtime_enable(),
                    async {
                        if let Some(world_name) = world_name {
                            client
                                .page_add_script_on_new_document_in_world("", world_name)
                                .await?;
                        }
                        Ok(())
                    },
                    attachment,
                )
                .await
                .map_err(ChromiumTopologyEventDriverError::from)
            };
            if resume_with_prerequisites && !attachment_started {
                let (tree, ()) = tokio::try_join!(prerequisites, async {
                    client
                        .runtime_run_if_waiting_for_debugger()
                        .await
                        .map_err(ChromiumTopologyEventDriverError::from)
                })?;
                tree
            } else {
                prerequisites.await?
            }
        } else if parallel_registration && let Some(world_name) = world_name {
            crate::session_prerequisites::initialize_with_registration(
                client.page_get_frame_tree(),
                client.runtime_enable(),
                async {
                    client
                        .page_add_script_on_new_document_in_world("", world_name)
                        .await?;
                    Ok(())
                },
            )
            .await?
        } else {
            crate::session_prerequisites::initialize(
                self.profile.parallel_session_prerequisites(),
                client.page_get_frame_tree(),
                client.runtime_enable(),
            )
            .await?
        };
        Ok(frame_tree)
    }

    async fn finish_session_worlds(
        &self,
        client: &dyn CdpClient,
        frame_tree: page::FrameTree,
        world_name: Option<&str>,
        allow_root_parent: bool,
    ) -> Result<page::FrameTree, ChromiumTopologyEventDriverError> {
        let Some(world_name) = world_name else {
            return Ok(frame_tree);
        };
        let parallel_registration = self.profile.parallel_utility_world_registration();
        // Validate before the serial registration, retaining the default error
        // ordering. The opt-in registration is empty and independent of the tree;
        // no actual worlds or ready routes are created until validation succeeds.
        let frame_ids = self.validate_frame_tree_ids(&frame_tree, allow_root_parent)?;
        if !parallel_registration {
            client
                .page_add_script_on_new_document_in_world("", world_name)
                .await?;
        }
        let worlds = self.initialize_utility_worlds(
            client,
            frame_tree,
            frame_ids,
            world_name,
            allow_root_parent,
        );
        worlds.await
    }

    async fn initialize_session_worlds(
        &self,
        client: &dyn CdpClient,
        world_name: Option<&str>,
        allow_root_parent: bool,
        attachment_started: bool,
    ) -> Result<page::FrameTree, ChromiumTopologyEventDriverError> {
        let frame_tree = self
            .initialize_session_prerequisites(client, world_name, attachment_started)
            .await?;
        self.finish_session_worlds(client, frame_tree, world_name, allow_root_parent)
            .await
    }

    fn resumes_with_utility_worlds(&self) -> bool {
        self.profile.parallel_recursive_attachment()
            && self.page_discovery.is_some()
            && self.utility_world.is_some()
    }

    async fn initialize_utility_worlds(
        &self,
        client: &dyn CdpClient,
        frame_tree: page::FrameTree,
        frame_ids: Vec<String>,
        world_name: &str,
        allow_root_parent: bool,
    ) -> Result<page::FrameTree, ChromiumTopologyEventDriverError> {
        let mut failed = None;
        for frame_id in &frame_ids {
            if let Err(error) = client
                .page_create_isolated_world(
                    frame_id,
                    world_name,
                    self.utility_world
                        .expect("utility-world initialization requires its profile")
                        .grant_universal_access(),
                )
                .await
            {
                failed = Some((frame_id.clone(), error));
                break;
            }
        }
        let Some((failed_frame_id, creation_error)) = failed else {
            return Ok(frame_tree);
        };

        // `Page.getFrameTree` is the authoritative race check. Only a frame
        // proven absent may forgive the first create failure; every other CDP
        // error remains terminal instead of being hidden as a generic race.
        let refreshed = match client.page_get_frame_tree().await {
            Ok(frame_tree) => frame_tree,
            Err(refresh_error) => {
                return Err(ChromiumTopologyEventDriverError::Cdp(CdpError::Compound {
                    source: Box::new(creation_error),
                    system: Box::new(refresh_error),
                }));
            }
        };
        let refreshed_ids = self.validate_frame_tree_ids(&refreshed, allow_root_parent)?;
        if refreshed_ids.iter().any(|id| id == &failed_frame_id) {
            return Err(creation_error.into());
        }
        for frame_id in &refreshed_ids {
            client
                .page_create_isolated_world(
                    frame_id,
                    world_name,
                    self.utility_world
                        .expect("utility-world initialization requires its profile")
                        .grant_universal_access(),
                )
                .await?;
        }
        Ok(refreshed)
    }

    fn validate_frame_tree_ids(
        &self,
        frame_tree: &page::FrameTree,
        allow_root_parent: bool,
    ) -> Result<Vec<String>, ChromiumTopologyEventDriverError> {
        if !allow_root_parent && frame_tree.frame.parent_id.is_some() {
            return Err(ChromiumTopologyEventDriverError::FrameTreeRootHasParent {
                frame_id: frame_tree.frame.id.clone(),
            });
        }
        let mut stack = vec![(frame_tree, None::<&str>)];
        let mut seen = BTreeSet::new();
        let mut frame_ids = Vec::new();
        while let Some((node, tree_parent)) = stack.pop() {
            let frame_id = node.frame.id.as_str();
            if !seen.insert(frame_id) {
                return Err(ChromiumTopologyEventDriverError::DuplicateFrameTreeId {
                    frame_id: frame_id.to_owned(),
                });
            }
            if let (Some(expected), Some(actual)) = (tree_parent, node.frame.parent_id.as_deref())
                && expected != actual
            {
                return Err(ChromiumTopologyEventDriverError::FrameTreeParentMismatch {
                    frame_id: frame_id.to_owned(),
                    expected: expected.to_owned(),
                    actual: actual.to_owned(),
                });
            }
            frame_ids.push(frame_id.to_owned());
            if let Some(children) = node.child_frames.as_deref() {
                for child in children.iter().rev() {
                    stack.push((child, Some(frame_id)));
                }
            }
        }
        Ok(frame_ids)
    }

    fn initialize_frame_tree<Runtime>(
        &self,
        runtime: &Runtime,
        route: &SessionRoute,
        frame_tree: page::FrameTree,
    ) -> Result<CdpTopologyUpdate, ChromiumTopologyEventDriverError>
    where
        Runtime: ChromiumTopologyRuntime,
    {
        let page_session = route.target_id == route.page_target_id;
        let prepared = self.prepare_frame_tree(route, &frame_tree, None)?;
        let page_target_id = route.page_target_id.clone();
        let epoch = self.epoch;
        self.transact_update_and_observe(
            runtime,
            move |topology| {
                let mut update = CdpTopologyUpdate::idempotent();
                for (attachment, object_id) in prepared {
                    update = update.combine(topology.attach_frame(epoch, attachment, object_id)?);
                }
                if page_session {
                    update = update.combine(topology.mark_page_ready(epoch, &page_target_id)?);
                }
                Ok(update)
            },
            |topology| self.observe_frame_tree(topology, route, &frame_tree),
        )
    }

    fn prepare_frame_tree(
        &self,
        route: &SessionRoute,
        frame_tree: &page::FrameTree,
        supplied_main_frame_object_id: Option<pwright_core::ObjectId>,
    ) -> Result<Vec<(CdpFrameAttachment, pwright_core::ObjectId)>, ChromiumTopologyEventDriverError>
    {
        let page_session = route.target_id == route.page_target_id;
        if page_session && frame_tree.frame.parent_id.is_some() {
            return Err(ChromiumTopologyEventDriverError::FrameTreeRootHasParent {
                frame_id: frame_tree.frame.id.clone(),
            });
        }
        let mut stack = vec![(frame_tree, None::<&str>, true)];
        let mut seen = std::collections::BTreeSet::new();
        let mut attachments = Vec::new();
        while let Some((node, tree_parent, root)) = stack.pop() {
            let frame_id = node.frame.id.clone();
            if !seen.insert(frame_id.clone()) {
                return Err(ChromiumTopologyEventDriverError::DuplicateFrameTreeId { frame_id });
            }
            if let (Some(expected), Some(actual)) = (tree_parent, node.frame.parent_id.as_deref())
                && expected != actual
            {
                return Err(ChromiumTopologyEventDriverError::FrameTreeParentMismatch {
                    frame_id,
                    expected: expected.to_owned(),
                    actual: actual.to_owned(),
                });
            }
            if root && page_session {
                attachments.push(CdpFrameAttachment::main(
                    route.page_target_id.clone(),
                    node.frame.id.clone(),
                ));
            } else if !root {
                attachments.push(CdpFrameAttachment::local(
                    route.page_target_id.clone(),
                    node.frame.id.clone(),
                    tree_parent.expect("a non-root frame-tree item has a tree parent"),
                ));
            }
            if let Some(children) = &node.child_frames {
                for child in children.iter().rev() {
                    stack.push((child, Some(node.frame.id.as_str()), false));
                }
            }
        }

        let main_frame_object_id = if page_session && supplied_main_frame_object_id.is_none() {
            let topology = self
                .topology
                .lock()
                .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
            self.ensure_current_epoch(topology.topology().epoch())?;
            topology
                .topology()
                .main_frame_object_id(&route.page_target_id)
        } else {
            supplied_main_frame_object_id
        };
        let mut prepared = Vec::with_capacity(attachments.len());
        for (index, attachment) in attachments.into_iter().enumerate() {
            let existing = {
                let topology = self
                    .topology
                    .lock()
                    .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
                self.ensure_current_epoch(topology.topology().epoch())?;
                topology.topology().frame_object_id(&attachment.frame_id)
            };
            let object_id = if index == 0 && page_session {
                main_frame_object_id.ok_or(ChromiumCompositionError::TopologyStateUnavailable)?
            } else {
                match existing {
                    Some(object_id) => object_id,
                    None => self.object_ids.next_object_id()?,
                }
            };
            prepared.push((attachment, object_id));
        }
        Ok(prepared)
    }

    fn session_routes(&self) -> Result<Vec<SessionRoute>, ChromiumTopologyEventDriverError> {
        let topology = self
            .topology
            .lock()
            .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
        self.ensure_current_epoch(topology.topology().epoch())?;
        let routes = topology
            .topology()
            .target_session_routes()
            .map(|route| SessionRoute {
                session_id: route.session_id().to_owned(),
                target_id: route.target_id().to_owned(),
                page_target_id: route.page_target_id().to_owned(),
            })
            .collect::<Vec<_>>();
        self.validate_routes(&routes)?;
        Ok(routes)
    }

    fn validate_routes(
        &self,
        routes: &[SessionRoute],
    ) -> Result<(), ChromiumTopologyEventDriverError> {
        if routes.len() > self.profile.max_sessions {
            return Err(ChromiumTopologyEventDriverError::SessionCapacityExceeded {
                actual: routes.len(),
                limit: self.profile.max_sessions,
            });
        }
        for route in routes {
            for (field, value) in [
                ("session ID", route.session_id.as_str()),
                ("target ID", route.target_id.as_str()),
                ("Page target ID", route.page_target_id.as_str()),
            ] {
                if value.len() > self.profile.max_native_id_bytes {
                    return Err(ChromiumTopologyEventDriverError::NativeIdTooLong {
                        field,
                        actual: value.len(),
                        limit: self.profile.max_native_id_bytes,
                    });
                }
            }
        }
        Ok(())
    }

    async fn apply_event<Runtime>(
        &mut self,
        runtime: &Runtime,
        event: &CdpEvent,
    ) -> Result<ChromiumTopologyEventDriverOutcome, ChromiumTopologyEventDriverError>
    where
        Runtime: ChromiumTopologyRuntime,
    {
        let outcome = self.apply_topology_event(runtime, event).await?;
        if let Some(observer) = &self.frame_events {
            let topology = self
                .topology
                .lock()
                .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
            observer
                .event(topology.topology(), event)
                .map_err(ChromiumTopologyEventDriverError::FrameEventObserver)?;
        }
        Ok(outcome)
    }

    async fn apply_topology_event<Runtime>(
        &mut self,
        runtime: &Runtime,
        event: &CdpEvent,
    ) -> Result<ChromiumTopologyEventDriverOutcome, ChromiumTopologyEventDriverError>
    where
        Runtime: ChromiumTopologyRuntime,
    {
        self.ensure_topology_epoch()?;
        match event.method.as_str() {
            "Target.targetDestroyed" => {
                // This is a native destruction fact, unlike detachedFromTarget
                // or local route disposal. Do not infer Page/Frame/public close
                // events here, rebuild routes, or require the route to survive.
                let topology = self
                    .topology
                    .lock()
                    .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
                self.ensure_current_epoch(topology.topology().epoch())?;
                let routes = runtime.page_services().routes.lock().map_err(|_| {
                    ChromiumCompositionError::from(crate::CdpPageRouteError::StatePoisoned)
                })?;
                let Some(registry) = &routes.native_world_retirement else {
                    return Ok(ChromiumTopologyEventDriverOutcome::Ignored);
                };
                let params: target::TargetDestroyedEvent =
                    self.decode(event, "Target.targetDestroyed")?;
                registry
                    .mark(
                        self.epoch,
                        crate::native_world_retirement::Destruction::PageTarget(&params.target_id),
                    )
                    .map_err(ChromiumCompositionError::from)?;
                Ok(ChromiumTopologyEventDriverOutcome::Ignored)
            }
            "Page.frameAttached" => {
                let params: page::FrameAttachedEvent = self.decode(event, "Page.frameAttached")?;
                let Some(route) = self.route_for_event_session(event, "Page.frameAttached")? else {
                    return Ok(ChromiumTopologyEventDriverOutcome::Ignored);
                };
                let attachment =
                    CdpFrameAttachment::from_frame_attached(route.page_target_id, &params);
                self.attach_frame(runtime, attachment)
            }
            "Page.frameDetached" => {
                let params: page::FrameDetachedEvent = self.decode(event, "Page.frameDetached")?;
                if self
                    .route_for_event_session(event, "Page.frameDetached")?
                    .is_none()
                {
                    return Ok(ChromiumTopologyEventDriverOutcome::Ignored);
                }
                let reason = CdpFrameDetachReason::from_event(&params)?;
                self.transact(runtime, |topology| {
                    topology.detach_frame(self.epoch, &params.frame_id, reason)
                })
            }
            "Page.frameNavigated" => {
                let params: page::FrameNavigatedEvent =
                    self.decode(event, "Page.frameNavigated")?;
                let Some(route) = self.route_for_event_session(event, "Page.frameNavigated")?
                else {
                    return Ok(ChromiumTopologyEventDriverOutcome::Ignored);
                };
                if params.frame.parent_id.is_some() || route.target_id != route.page_target_id {
                    let update = self.transact_update_and_observe(
                        runtime,
                        |topology| {
                            topology.frame_document_committed(
                                self.epoch,
                                &route.session_id,
                                &params.frame.id,
                            )
                        },
                        |topology| {
                            self.observe_frame_url(
                                topology,
                                &route,
                                &params.frame.id,
                                &params.frame.url,
                                params.frame.url_fragment.as_deref(),
                                Some(params.frame.name.as_deref().unwrap_or("")),
                            )
                        },
                    )?;
                    return Ok(update_outcome(update));
                }
                let main_frame_object_id = {
                    let topology = self
                        .topology
                        .lock()
                        .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
                    topology
                        .topology()
                        .main_frame_object_id(&route.page_target_id)
                };
                let Some(main_frame_object_id) = main_frame_object_id else {
                    return Ok(ChromiumTopologyEventDriverOutcome::Ignored);
                };
                let attachment =
                    CdpFrameAttachment::main(route.page_target_id.clone(), params.frame.id.clone());
                let epoch = self.epoch;
                let update = self.transact_update_and_observe(
                    runtime,
                    |topology| {
                        let update =
                            topology.attach_frame(epoch, attachment, main_frame_object_id)?;
                        Ok(update.combine(topology.frame_document_committed(
                            epoch,
                            &route.session_id,
                            &params.frame.id,
                        )?))
                    },
                    |topology| {
                        self.observe_frame_url(
                            topology,
                            &route,
                            &params.frame.id,
                            &params.frame.url,
                            params.frame.url_fragment.as_deref(),
                            Some(params.frame.name.as_deref().unwrap_or("")),
                        )
                    },
                )?;
                Ok(update_outcome(update))
            }
            "Page.navigatedWithinDocument" if self.frame_urls.is_some() => {
                let params: page::NavigatedWithinDocumentEvent =
                    self.decode(event, "Page.navigatedWithinDocument")?;
                if let Some(route) =
                    self.route_for_event_session(event, "Page.navigatedWithinDocument")?
                {
                    self.observe_frame_event(&route, &params.frame_id, &params.url, None, None)?;
                }
                Ok(ChromiumTopologyEventDriverOutcome::Ignored)
            }
            "Runtime.executionContextCreated" => {
                let params: runtime::ExecutionContextCreatedEvent =
                    self.decode(event, "Runtime.executionContextCreated")?;
                let session_id = self.required_session(event, "Runtime.executionContextCreated")?;
                if self.route_for_session(session_id)?.is_none() {
                    return Ok(ChromiumTopologyEventDriverOutcome::Ignored);
                }
                let Some(attachment) = CdpWorldAttachment::from_execution_context(
                    session_id,
                    &params,
                    self.utility_world_name_for_session(session_id)?.as_deref(),
                )?
                else {
                    return Ok(ChromiumTopologyEventDriverOutcome::Ignored);
                };
                let epoch = self.epoch;
                self.transact(runtime, move |topology| {
                    topology.attach_world(epoch, attachment)
                })
            }
            "Runtime.executionContextDestroyed" => {
                let params: runtime::ExecutionContextDestroyedEvent =
                    self.decode(event, "Runtime.executionContextDestroyed")?;
                let session_id =
                    self.required_session(event, "Runtime.executionContextDestroyed")?;
                if self.route_for_session(session_id)?.is_none() {
                    return Ok(ChromiumTopologyEventDriverOutcome::Ignored);
                }
                self.transact_destruction(
                    runtime,
                    |topology| {
                        topology.destroy_world(self.epoch, session_id, params.execution_context_id)
                    },
                    crate::native_world_retirement::Destruction::World {
                        session: session_id,
                        unique: &params.execution_context_unique_id,
                    },
                )
            }
            "Runtime.executionContextsCleared" => {
                let session_id =
                    self.required_session(event, "Runtime.executionContextsCleared")?;
                if self.route_for_session(session_id)?.is_none() {
                    return Ok(ChromiumTopologyEventDriverOutcome::Ignored);
                }
                self.transact_destruction(
                    runtime,
                    |topology| topology.clear_worlds(self.epoch, session_id),
                    crate::native_world_retirement::Destruction::SessionWorlds(session_id),
                )
            }
            "Page.javascriptDialogClosed" => {
                let session_id = self.required_session(event, "Page.javascriptDialogClosed")?;
                if self
                    .initialized_sessions
                    .get(session_id)
                    .is_some_and(|initialized| initialized.resume_after_dialog)
                {
                    self.initialized_sessions.remove(session_id);
                }
                Ok(ChromiumTopologyEventDriverOutcome::Ignored)
            }
            "Target.attachedToTarget" => {
                if let (Some(parent), Some(event_parent)) =
                    (&self.auxiliary_session_parent, event.session_id.as_deref())
                    && parent
                        .matches(event_parent)
                        .map_err(ChromiumTopologyEventDriverError::AuxiliarySessionParent)?
                {
                    return Ok(ChromiumTopologyEventDriverOutcome::Ignored);
                }
                let params: target::AttachedToTargetEvent =
                    self.decode(event, "Target.attachedToTarget")?;
                // attachToBrowserTarget can publish its root attachment before
                // the command response provides the new public session ID.
                // Portable topology never owns browser-type targets.
                if event.session_id.is_none()
                    && self.auxiliary_session_parent.is_some()
                    && params.target_info.r#type == "browser"
                {
                    return Ok(ChromiumTopologyEventDriverOutcome::Ignored);
                }
                if params.target_info.r#type == "page"
                    && event.session_id.is_none()
                    && self.page_discovery.is_some()
                {
                    return self.attach_discovered_page(runtime, params).await;
                }
                if params.target_info.r#type == "iframe" {
                    let Some(parent) =
                        self.route_for_event_session(event, "Target.attachedToTarget")?
                    else {
                        self.browser_client
                            .target_detach_in_parent(
                                event.session_id.as_deref(),
                                &params.session_id,
                            )
                            .await?;
                        return Ok(ChromiumTopologyEventDriverOutcome::Ignored);
                    };
                    let attachment =
                        CdpFrameAttachment::from_attached_oopif(parent.page_target_id, &params)?;
                    return self.attach_frame(runtime, attachment);
                }
                let retain_worker = if params.target_info.r#type == "worker" {
                    match (&self.frame_events, event.session_id.as_deref()) {
                        (Some(observer), Some(parent)) if observer.retains_worker_targets() => {
                            let route = self.route_for_session(parent)?;
                            route.is_some() || observer.retains_worker_session(parent)
                        }
                        _ => false,
                    }
                } else {
                    false
                };
                if retain_worker {
                    let enable_network = self
                        .frame_events
                        .as_ref()
                        .is_some_and(|observer| observer.enables_worker_network());
                    let client = self.session_factory.create_session(
                        params.session_id.clone(),
                        params.target_info.target_id.clone(),
                    );
                    let initialize: Result<(), ChromiumTopologyEventDriverError> = async {
                        client.runtime_enable().await?;
                        client
                            .target_set_auto_attach_types(
                                true,
                                true,
                                true,
                                self.auto_attach_target_types(),
                            )
                            .await?;
                        if enable_network {
                            client.network_enable().await?;
                        }
                        client.runtime_run_if_waiting_for_debugger().await?;
                        Ok(())
                    }
                    .await;
                    match initialize {
                        Ok(()) => {}
                        Err(ChromiumTopologyEventDriverError::Cdp(CdpError::SessionDetached)) => {}
                        Err(ChromiumTopologyEventDriverError::Cdp(CdpError::Protocol {
                            code: -32001,
                            message,
                        })) if message == "Session with given id not found." => {}
                        Err(error) => return Err(error),
                    }
                    return Ok(ChromiumTopologyEventDriverOutcome::Ignored);
                }
                if self.page_discovery.is_some() {
                    self.browser_client
                        .target_detach_in_parent(event.session_id.as_deref(), &params.session_id)
                        .await?;
                }
                Ok(ChromiumTopologyEventDriverOutcome::Ignored)
            }
            "Target.detachedFromTarget" => {
                if let (Some(parent), Some(event_parent)) =
                    (&self.auxiliary_session_parent, event.session_id.as_deref())
                    && parent
                        .matches(event_parent)
                        .map_err(ChromiumTopologyEventDriverError::AuxiliarySessionParent)?
                {
                    return Ok(ChromiumTopologyEventDriverOutcome::Ignored);
                }
                let params: target::DetachedFromTargetEvent =
                    self.decode(event, "Target.detachedFromTarget")?;
                self.retire_session(runtime, &params.session_id, params.target_id.as_deref())
            }
            "Target.targetCrashed" => {
                let params: target::TargetCrashedEvent =
                    self.decode(event, "Target.targetCrashed")?;
                let is_page = {
                    let topology = self
                        .topology
                        .lock()
                        .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
                    topology
                        .topology()
                        .page_object_id(&params.target_id)
                        .is_some()
                };
                if !is_page {
                    return Ok(ChromiumTopologyEventDriverOutcome::Ignored);
                }
                let epoch = self.epoch;
                self.transact(runtime, move |topology| {
                    topology.crash_target(
                        epoch,
                        &params.target_id,
                        params.status,
                        params.error_code,
                    )
                })
            }
            _ => Ok(ChromiumTopologyEventDriverOutcome::Ignored),
        }
    }

    async fn attach_discovered_page<Runtime>(
        &mut self,
        runtime: &Runtime,
        event: target::AttachedToTargetEvent,
    ) -> Result<ChromiumTopologyEventDriverOutcome, ChromiumTopologyEventDriverError>
    where
        Runtime: ChromiumTopologyRuntime,
    {
        if event.target_info.subtype.is_some() {
            self.browser_client.target_detach(&event.session_id).await?;
            return Ok(ChromiumTopologyEventDriverOutcome::Ignored);
        }
        let synchronized_session = {
            let topology = self
                .topology
                .lock()
                .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
            self.ensure_current_epoch(topology.topology().epoch())?;
            topology
                .topology()
                .page_session_routes()
                .find(|route| route.target_id() == event.target_info.target_id)
                .map(|route| route.session_id().to_owned())
        };
        if let Some(session_id) = synchronized_session
            && self
                .initialized_sessions
                .get(&session_id)
                .is_some_and(|initialized| {
                    initialized.target_id == event.target_info.target_id
                        && !initialized.discovered_page
                })
        {
            // Explicit Page creation already initialized this exact target.
            // A repeated announcement of the same session is only a replay;
            // detaching it would close the live Page. A distinct auto-attach
            // session is redundant and must be detached before any selected
            // session capability is installed a second time.
            if session_id != event.session_id {
                self.browser_client.target_detach(&event.session_id).await?;
            }
            return Ok(ChromiumTopologyEventDriverOutcome::Ignored);
        }
        let reported_context_id = event.target_info.browser_context_id.clone();
        let registered_context_id = {
            let topology = self
                .topology
                .lock()
                .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
            self.ensure_current_epoch(topology.topology().epoch())?;
            reported_context_id.as_ref().and_then(|browser_context_id| {
                topology
                    .topology()
                    .context_object_id(browser_context_id)
                    .map(|_| browser_context_id.clone())
            })
        };
        let browser_context_id = if let Some(registered) = registered_context_id {
            Some(registered)
        } else if let Some(reported) = reported_context_id {
            self.validate_discovery_native_id("browser context ID", &reported)?;
            // Playwright attributes every unregistered native Context to this
            // connection's default view. A Context-list snapshot may predate an
            // attachment, or the Context may already be disposed while its
            // attach event waits. Do not infer default identity from that stale
            // list in this selected mode. Initialization verifies the session.
            if self.playwright_context_fallback
                || self.default_native_context_id.as_deref() == Some(reported.as_str())
            {
                self.default_context_key()?
            } else if self.nondefault_context_ids.contains(&reported) {
                self.foreign_context_key()?
            } else if self.default_native_context_id.is_none() {
                self.default_native_context_id = Some(reported.into_boxed_str().into_string());
                self.default_context_key()?
            } else {
                self.refresh_nondefault_contexts().await?;
                if self.nondefault_context_ids.contains(&reported) {
                    self.foreign_context_key()?
                } else {
                    match self.default_native_context_id.as_deref() {
                        Some(existing) if existing != reported => None,
                        Some(_) => self.default_context_key()?,
                        None => unreachable!("the missing default ID was handled before refresh"),
                    }
                }
            }
        } else {
            self.default_context_key()?
        };
        let Some(browser_context_id) = browser_context_id else {
            self.browser_client.target_detach(&event.session_id).await?;
            return Ok(ChromiumTopologyEventDriverOutcome::Ignored);
        };
        let (existing_ids, browser_context_object_id) = {
            let topology = self
                .topology
                .lock()
                .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
            let existing_ids = topology
                .topology()
                .page_object_id(&event.target_info.target_id)
                .zip(
                    topology
                        .topology()
                        .main_frame_object_id(&event.target_info.target_id),
                );
            let browser_context_object_id = topology
                .topology()
                .context_object_id(&browser_context_id)
                .ok_or(ChromiumCompositionError::TopologyStateUnavailable)?;
            (existing_ids, browser_context_object_id)
        };
        let is_new_page = existing_ids.is_none();
        let object_ids = match existing_ids {
            Some((page, main_frame)) => {
                pwright_backend_cdp::CdpPageObjectIds::new(page, main_frame)
            }
            None => pwright_backend_cdp::CdpPageObjectIds::new(
                self.object_ids.next_object_id()?,
                self.object_ids.next_object_id()?,
            ),
        };
        let page_object_id = object_ids.page;
        let mut target = pwright_backend_cdp::CdpPageTarget::new(
            event.target_info.target_id,
            event.session_id,
            browser_context_id,
        );
        if let Some(opener) = event.target_info.opener_id {
            target = target.with_opener(opener);
        }
        let route = SessionRoute {
            session_id: target.session_id.clone(),
            target_id: target.target_id.clone(),
            page_target_id: target.target_id.clone(),
        };
        let network_registration = self.network_registration.clone();
        let replace_network_session = if !is_new_page {
            match network_registration.as_ref().map(|registration| {
                registration.validate_page_session(object_ids.page, &route.session_id)
            }) {
                Some(Ok(())) | None => false,
                Some(Err(
                    ChromiumNetworkEventDriverError::SessionReplacementRequiresRetirement {
                        ..
                    },
                )) => true,
                Some(Err(error)) => {
                    let primary = ChromiumTopologyEventDriverError::Network(error);
                    return match self.browser_client.target_detach(&route.session_id).await {
                        Ok(()) => Err(primary),
                        Err(cleanup) => {
                            Err(ChromiumTopologyEventDriverError::PageDiscoveryCleanup {
                                primary: primary.to_string(),
                                cleanup: cleanup.to_string(),
                            })
                        }
                    };
                }
            }
        } else {
            false
        };
        // Match Playwright's internal Page lifecycle: make the Page and its
        // main-Frame/session route available to ordered observers before the
        // paused target can execute author script, but keep it Initializing so
        // no public Page/Popup event is emitted yet. This is selected only by
        // the recursive compatibility profile; the base composition retains
        // its historical all-at-once publication boundary.
        let provisional_page = is_new_page && self.resumes_with_utility_worlds();
        let client = self
            .session_factory
            .create_session(route.session_id.clone(), route.target_id.clone());
        if provisional_page {
            let provisional_target = target.clone();
            let provisional_ids = object_ids.clone();
            let epoch = self.epoch;
            let publish = || {
                self.transact_update(runtime, move |topology| {
                    topology.attach_page(epoch, provisional_target, provisional_ids)
                })
            };
            if let Some(registration) = &network_registration {
                // Reserve the optional network lane before the topology route
                // becomes observable. Native Network.enable is already queued
                // by discovered-session initialization before resume; `true`
                // prevents this reservation from issuing a duplicate command.
                let prepared = match registration
                    .prepare_page_registration(
                        page_object_id,
                        browser_context_object_id,
                        route.session_id.clone(),
                        Vec::new(),
                        Arc::clone(&client),
                        true,
                    )
                    .await
                {
                    Ok(prepared) => prepared,
                    Err(error) => {
                        let primary = ChromiumTopologyEventDriverError::Network(error);
                        return match self.browser_client.target_detach(&route.session_id).await {
                            Ok(()) => Err(primary),
                            Err(cleanup) => {
                                Err(ChromiumTopologyEventDriverError::PageDiscoveryCleanup {
                                    primary: primary.to_string(),
                                    cleanup: cleanup.to_string(),
                                })
                            }
                        };
                    }
                };
                let publication = registration.commit_page_registration(prepared, publish);
                let primary = match publication {
                    Ok(Ok(_)) => None,
                    Ok(Err(error)) => Some(error),
                    Err(error) => Some(ChromiumTopologyEventDriverError::Network(error)),
                };
                if let Some(primary) = primary {
                    let mut cleanup = registration
                        .rollback_page_registration(prepared)
                        .await
                        .err()
                        .map(|error| error.to_string());
                    if let Err(error) = self.browser_client.target_detach(&route.session_id).await {
                        let message = error.to_string();
                        cleanup = Some(match cleanup {
                            Some(existing) => {
                                format!("{existing}; session detach failed: {message}")
                            }
                            None => message,
                        });
                    }
                    return match cleanup {
                        Some(cleanup) => {
                            Err(ChromiumTopologyEventDriverError::PageDiscoveryCleanup {
                                primary: primary.to_string(),
                                cleanup,
                            })
                        }
                        None => Err(primary),
                    };
                }
            } else {
                publish()?;
            }
        }
        let initialized = self
            .initialize_discovered_session(
                client.as_ref(),
                &route,
                browser_context_object_id,
                page_object_id,
            )
            .await;
        let initialized = match initialized {
            Ok(initialized) => initialized,
            Err(primary) if provisional_page => {
                return Err(self
                    .rollback_provisional_discovered_page(
                        runtime,
                        &route,
                        page_object_id,
                        primary,
                        None,
                    )
                    .await);
            }
            Err(primary) => return Err(primary),
        };
        let Some((frame_tree, native_session_missing, resume_after_dialog)) = initialized else {
            return Ok(ChromiumTopologyEventDriverOutcome::Ignored);
        };
        let prepared_frames =
            match self.prepare_frame_tree(&route, &frame_tree, Some(object_ids.main_frame)) {
                Ok(prepared) => prepared,
                Err(primary) if provisional_page => {
                    return Err(self
                        .rollback_provisional_discovered_page(
                            runtime,
                            &route,
                            page_object_id,
                            primary,
                            None,
                        )
                        .await);
                }
                Err(primary) => return Err(primary),
            };
        let initial_network_frames = match self.validate_frame_tree_ids(&frame_tree, false) {
            Ok(frames) => frames,
            Err(primary) if provisional_page => {
                return Err(self
                    .rollback_provisional_discovered_page(
                        runtime,
                        &route,
                        page_object_id,
                        primary,
                        None,
                    )
                    .await);
            }
            Err(primary) => return Err(primary),
        };
        let prepared_network = if is_new_page && !provisional_page && !native_session_missing {
            if let Some(registration) = &network_registration {
                match registration
                    .prepare_page_registration(
                        page_object_id,
                        browser_context_object_id,
                        route.session_id.clone(),
                        // A newly discovered target may already have its first
                        // navigation request buffered ahead of Page publication.
                        // Seeding its frame tree here would let that request see
                        // topology from the future and break popup-first ordering.
                        Vec::new(),
                        Arc::clone(&client),
                        true,
                    )
                    .await
                {
                    Ok(prepared) => Some(prepared),
                    Err(error) => {
                        let primary = ChromiumTopologyEventDriverError::Network(error);
                        if provisional_page {
                            return Err(self
                                .rollback_provisional_discovered_page(
                                    runtime,
                                    &route,
                                    page_object_id,
                                    primary,
                                    None,
                                )
                                .await);
                        }
                        return match self.browser_client.target_detach(&route.session_id).await {
                            Ok(()) => Err(primary),
                            Err(cleanup) => {
                                Err(ChromiumTopologyEventDriverError::PageDiscoveryCleanup {
                                    primary: primary.to_string(),
                                    cleanup: cleanup.to_string(),
                                })
                            }
                        };
                    }
                }
            } else {
                None
            }
        } else {
            None
        };
        let prepared_network_replacement = if replace_network_session && !native_session_missing {
            let registration = network_registration
                .as_ref()
                .expect("network session replacement requires its registration handle");
            match registration
                .prepare_page_replacement(
                    page_object_id,
                    browser_context_object_id,
                    route.session_id.clone(),
                    initial_network_frames,
                    Arc::clone(&client),
                    true,
                )
                .await
            {
                Ok(prepared) => Some(prepared),
                Err(error) => {
                    let primary = ChromiumTopologyEventDriverError::Network(error);
                    return match self.browser_client.target_detach(&route.session_id).await {
                        Ok(()) => Err(primary),
                        Err(cleanup) => {
                            Err(ChromiumTopologyEventDriverError::PageDiscoveryCleanup {
                                primary: primary.to_string(),
                                cleanup: cleanup.to_string(),
                            })
                        }
                    };
                }
            }
        } else {
            None
        };
        let epoch = self.epoch;
        let target_id = target.target_id.clone();
        let publish = |network_cleanup: &[ControlMessage]| {
            let network_cleanup = network_cleanup.to_vec();
            self.transact_update_and_observe(
                runtime,
                move |topology| {
                    let mut update = if provisional_page {
                        CdpTopologyUpdate::idempotent()
                    } else {
                        topology.attach_page(epoch, target, object_ids)?
                    };
                    for (attachment, object_id) in prepared_frames {
                        update =
                            update.combine(topology.attach_frame(epoch, attachment, object_id)?);
                    }
                    if !network_cleanup.is_empty() {
                        update = update.combine(CdpTopologyUpdate::lifecycle(network_cleanup));
                    }
                    update = update.combine(topology.mark_page_ready(epoch, &target_id)?);
                    Ok(update)
                },
                |topology| self.observe_frame_tree(topology, &route, &frame_tree),
            )
        };
        let publication = match (
            network_registration.as_ref(),
            prepared_network,
            prepared_network_replacement,
        ) {
            (Some(registration), Some(prepared), None) => {
                match registration.commit_page_registration(prepared, || publish(&[])) {
                    Ok(publication) => publication,
                    Err(error) => Err(ChromiumTopologyEventDriverError::Network(error)),
                }
            }
            (Some(registration), None, Some(prepared)) => {
                match registration.commit_page_replacement(prepared, publish) {
                    Ok(publication) => publication,
                    Err(error) => Err(ChromiumTopologyEventDriverError::Network(error)),
                }
            }
            _ => publish(&[]),
        };
        let update = match publication {
            Ok(update) => update,
            Err(primary) => {
                let mut cleanup = None;
                if let (Some(registration), Some(prepared)) =
                    (network_registration.as_ref(), prepared_network)
                    && let Err(error) = registration.rollback_page_registration(prepared).await
                {
                    cleanup = Some(error.to_string());
                }
                if let (Some(registration), Some(prepared)) =
                    (network_registration.as_ref(), prepared_network_replacement)
                    && let Err(error) = registration.rollback_page_replacement(prepared).await
                {
                    cleanup = Some(match cleanup {
                        Some(existing) => {
                            format!("{existing}; replacement rollback failed: {error}")
                        }
                        None => error.to_string(),
                    });
                }
                if provisional_page {
                    return Err(self
                        .rollback_provisional_discovered_page(
                            runtime,
                            &route,
                            page_object_id,
                            primary,
                            cleanup,
                        )
                        .await);
                }
                if let Err(error) = self.browser_client.target_detach(&route.session_id).await {
                    let message = error.to_string();
                    cleanup = Some(match cleanup {
                        Some(existing) => format!("{existing}; session detach failed: {message}"),
                        None => message,
                    });
                }
                return match cleanup {
                    Some(cleanup) => Err(ChromiumTopologyEventDriverError::PageDiscoveryCleanup {
                        primary: primary.to_string(),
                        cleanup,
                    }),
                    None => Err(primary),
                };
            }
        };
        // Even a session that closed during initialization completed this
        // attach transaction. Keep it out of the same-turn synchronization
        // retry so already queued execution-context, console/dialog and detach
        // events retain their native order. The detach transaction removes the
        // route, and the following synchronization prunes this marker.
        self.initialized_sessions.insert(
            route.session_id,
            InitializedSession {
                target_id: route.target_id,
                discovered_page: true,
                resume_after_dialog,
            },
        );
        Ok(ChromiumTopologyEventDriverOutcome::Updated(update))
    }

    async fn rollback_provisional_discovered_page<Runtime>(
        &self,
        runtime: &Runtime,
        route: &SessionRoute,
        page: ObjectId,
        primary: ChromiumTopologyEventDriverError,
        initial_cleanup: Option<String>,
    ) -> ChromiumTopologyEventDriverError
    where
        Runtime: ChromiumTopologyRuntime,
    {
        let mut cleanup = initial_cleanup.into_iter().collect::<Vec<_>>();
        if let Err(error) = self.browser_client.target_detach(&route.session_id).await {
            cleanup.push(format!("session detach failed: {error}"));
        }
        let session_id = route.session_id.clone();
        let target_id = route.target_id.clone();
        let epoch = self.epoch;
        let publish = || {
            self.transact_update(runtime, move |topology| {
                topology.detach_target(epoch, &session_id, Some(&target_id))
            })
        };
        if let Some(registration) = &self.network_registration {
            match registration.commit_page_retirement(page, publish) {
                Ok(Ok((_update, _network_cleanup))) => {}
                Ok(Err(error)) => {
                    cleanup.push(format!("provisional topology rollback failed: {error}"));
                }
                Err(error) => {
                    cleanup.push(format!("provisional network rollback failed: {error}"));
                }
            }
        } else if let Err(error) = publish() {
            cleanup.push(format!("provisional topology rollback failed: {error}"));
        }
        if let Some(observer) = &self.frame_events {
            match self.topology.lock() {
                Ok(topology) => {
                    if let Err(error) = observer.discard_prepared_page(topology.topology(), page) {
                        cleanup.push(format!("provisional observer rollback failed: {error}"));
                    }
                }
                Err(_) => cleanup.push("provisional observer rollback state unavailable".into()),
            }
        }
        if let Err(error) = self.refresh_frame_ownership() {
            cleanup.push(format!("provisional ownership rollback failed: {error}"));
        }
        if cleanup.is_empty() {
            primary
        } else {
            ChromiumTopologyEventDriverError::PageDiscoveryCleanup {
                primary: primary.to_string(),
                cleanup: cleanup.join("; "),
            }
        }
    }

    fn default_context_key(&self) -> Result<Option<String>, ChromiumTopologyEventDriverError> {
        let topology = self
            .topology
            .lock()
            .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
        self.ensure_current_epoch(topology.topology().epoch())?;
        Ok(topology
            .topology()
            .default_context_route()
            .map(|route| route.browser_context_id().to_owned()))
    }

    fn validate_discovery_native_id(
        &self,
        field: &'static str,
        value: &str,
    ) -> Result<(), ChromiumTopologyEventDriverError> {
        if value.is_empty() {
            return Err(ChromiumTopologyEventDriverError::EmptyNativeId { field });
        }
        if value.len() > self.profile.max_native_id_bytes() {
            return Err(ChromiumTopologyEventDriverError::NativeIdTooLong {
                field,
                actual: value.len(),
                limit: self.profile.max_native_id_bytes(),
            });
        }
        Ok(())
    }

    fn attach_frame<Runtime>(
        &self,
        runtime: &Runtime,
        attachment: CdpFrameAttachment,
    ) -> Result<ChromiumTopologyEventDriverOutcome, ChromiumTopologyEventDriverError>
    where
        Runtime: ChromiumTopologyRuntime,
    {
        let object_id = {
            let topology = self
                .topology
                .lock()
                .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
            self.ensure_current_epoch(topology.topology().epoch())?;
            match topology.topology().frame_object_id(&attachment.frame_id) {
                Some(object_id) => object_id,
                None => self.object_ids.next_object_id()?,
            }
        };
        self.transact(runtime, move |topology| {
            topology.attach_frame(self.epoch, attachment, object_id)
        })
    }

    fn transact<Runtime>(
        &self,
        runtime: &Runtime,
        apply: impl FnOnce(
            &mut pwright_backend_cdp::CdpTopology,
        ) -> Result<CdpTopologyUpdate, pwright_backend_cdp::CdpTopologyError>,
    ) -> Result<ChromiumTopologyEventDriverOutcome, ChromiumTopologyEventDriverError>
    where
        Runtime: ChromiumTopologyRuntime,
    {
        Ok(ChromiumTopologyEventDriverOutcome::Updated(
            self.transact_update(runtime, apply)?,
        ))
    }

    fn transact_update<Runtime>(
        &self,
        runtime: &Runtime,
        apply: impl FnOnce(
            &mut pwright_backend_cdp::CdpTopology,
        ) -> Result<CdpTopologyUpdate, pwright_backend_cdp::CdpTopologyError>,
    ) -> Result<CdpTopologyUpdate, ChromiumTopologyEventDriverError>
    where
        Runtime: ChromiumTopologyRuntime,
    {
        self.transact_update_and_observe(runtime, apply, |_| Ok(()))
    }

    fn transact_destruction<Runtime>(
        &self,
        runtime: &Runtime,
        apply: impl FnOnce(
            &mut pwright_backend_cdp::CdpTopology,
        ) -> Result<CdpTopologyUpdate, pwright_backend_cdp::CdpTopologyError>,
        destruction: crate::native_world_retirement::Destruction<'_>,
    ) -> Result<ChromiumTopologyEventDriverOutcome, ChromiumTopologyEventDriverError>
    where
        Runtime: ChromiumTopologyRuntime,
    {
        let mut topology = self
            .topology
            .lock()
            .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
        self.ensure_current_epoch(topology.topology().epoch())?;
        let update = crate::transact_topology_update_with_destruction(
            &mut topology,
            runtime,
            apply,
            Some(destruction),
        )?;
        // These mutations only retire worlds; Frame URL ownership is unchanged.
        Ok(ChromiumTopologyEventDriverOutcome::Updated(update))
    }

    fn transact_update_and_observe<Runtime>(
        &self,
        runtime: &Runtime,
        apply: impl FnOnce(
            &mut pwright_backend_cdp::CdpTopology,
        ) -> Result<CdpTopologyUpdate, pwright_backend_cdp::CdpTopologyError>,
        observe: impl FnOnce(
            &pwright_backend_cdp::CdpTopology,
        ) -> Result<(), ChromiumTopologyEventDriverError>,
    ) -> Result<CdpTopologyUpdate, ChromiumTopologyEventDriverError>
    where
        Runtime: ChromiumTopologyRuntime,
    {
        let mut topology = self
            .topology
            .lock()
            .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
        self.ensure_current_epoch(topology.topology().epoch())?;
        let update = transact_topology_update(&mut topology, runtime, apply)?;
        if let Some(observer) = &self.frame_urls {
            observer
                .retain_live(&|id| topology.topology().frame_native_id(id).is_some())
                .map_err(ChromiumTopologyEventDriverError::FrameUrlObserver)?;
        }
        observe(topology.topology())?;
        if let Some(observer) = &self.frame_urls {
            observer
                .reconcile_committed(ChromiumCommittedFrameOwnership::new(
                    ChromiumFrameOwnershipView::new(topology.topology()),
                    update.effects(),
                ))
                .map_err(ChromiumTopologyEventDriverError::FrameUrlObserver)?;
        }
        Ok(update)
    }

    fn refresh_frame_ownership(&self) -> Result<(), ChromiumTopologyEventDriverError> {
        if let Some(observer) = &self.frame_events {
            let topology = self
                .topology
                .lock()
                .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
            self.ensure_current_epoch(topology.topology().epoch())?;
            observer
                .reconcile(topology.topology())
                .map_err(ChromiumTopologyEventDriverError::FrameEventObserver)?;
        }
        let Some(observer) = &self.frame_urls else {
            return Ok(());
        };
        let topology = self
            .topology
            .lock()
            .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
        self.ensure_current_epoch(topology.topology().epoch())?;
        observer
            .retain_live(&|id| topology.topology().frame_native_id(id).is_some())
            .map_err(ChromiumTopologyEventDriverError::FrameUrlObserver)?;
        observer
            .reconcile_ownership(ChromiumFrameOwnershipView::new(topology.topology()))
            .map_err(ChromiumTopologyEventDriverError::FrameUrlObserver)
    }

    fn observe_frame_event(
        &self,
        route: &SessionRoute,
        frame_id: &str,
        url: &str,
        fragment: Option<&str>,
        name: Option<&str>,
    ) -> Result<(), ChromiumTopologyEventDriverError> {
        let topology = self
            .topology
            .lock()
            .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
        self.ensure_current_epoch(topology.topology().epoch())?;
        self.observe_frame_url(topology.topology(), route, frame_id, url, fragment, name)
    }

    fn observe_frame_url(
        &self,
        topology: &pwright_backend_cdp::CdpTopology,
        route: &SessionRoute,
        frame_id: &str,
        url: &str,
        fragment: Option<&str>,
        name: Option<&str>,
    ) -> Result<(), ChromiumTopologyEventDriverError> {
        let Some(observer) = &self.frame_urls else {
            return Ok(());
        };
        let Some(frame) = topology.frame_object_id(frame_id) else {
            return Ok(());
        };
        if topology.page_object_id_for_frame(frame)
            != topology.page_object_id(&route.page_target_id)
            || topology.frame_session_id(frame) != Some(route.session_id.as_str())
        {
            return Ok(());
        }
        match name {
            Some(name) => observer.update_navigation(frame, url, fragment, name),
            None => observer.update(frame, url, fragment),
        }
        .map_err(ChromiumTopologyEventDriverError::FrameUrlObserver)
    }

    fn observe_frame_tree(
        &self,
        topology: &pwright_backend_cdp::CdpTopology,
        route: &SessionRoute,
        tree: &page::FrameTree,
    ) -> Result<(), ChromiumTopologyEventDriverError> {
        if self.frame_urls.is_none() && self.frame_events.is_none() {
            return Ok(());
        }
        let mut pending = vec![tree];
        while let Some(node) = pending.pop() {
            if let Some(observer) = &self.frame_events {
                observer
                    .initialize_frame(topology, &route.session_id, &node.frame)
                    .map_err(ChromiumTopologyEventDriverError::FrameEventObserver)?;
            }
            self.observe_frame_url(
                topology,
                route,
                &node.frame.id,
                &node.frame.url,
                node.frame.url_fragment.as_deref(),
                Some(node.frame.name.as_deref().unwrap_or("")),
            )?;
            if let Some(children) = &node.child_frames {
                pending.extend(children.iter().rev());
            }
        }
        Ok(())
    }

    fn route_for_event_session(
        &self,
        event: &CdpEvent,
        method: &'static str,
    ) -> Result<Option<SessionRoute>, ChromiumTopologyEventDriverError> {
        let session_id = self.required_session(event, method)?;
        self.route_for_session(session_id)
    }

    fn route_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionRoute>, ChromiumTopologyEventDriverError> {
        let topology = self
            .topology
            .lock()
            .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
        self.ensure_current_epoch(topology.topology().epoch())?;
        Ok(topology
            .topology()
            .target_session_route(session_id)
            .map(|route| SessionRoute {
                session_id: route.session_id().to_owned(),
                target_id: route.target_id().to_owned(),
                page_target_id: route.page_target_id().to_owned(),
            }))
    }

    fn required_session<'a>(
        &self,
        event: &'a CdpEvent,
        method: &'static str,
    ) -> Result<&'a str, ChromiumTopologyEventDriverError> {
        event
            .session_id
            .as_deref()
            .ok_or(ChromiumTopologyEventDriverError::MissingEventSession { method })
    }

    fn decode<T: serde::de::DeserializeOwned>(
        &self,
        event: &CdpEvent,
        method: &'static str,
    ) -> Result<T, ChromiumTopologyEventDriverError> {
        serde_json::from_value(event.params.clone())
            .map_err(|error| ChromiumTopologyEventDriverError::Decode { method, error })
    }

    fn ensure_topology_epoch(&self) -> Result<(), ChromiumTopologyEventDriverError> {
        let current = self
            .topology
            .lock()
            .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?
            .topology()
            .epoch();
        self.ensure_current_epoch(current)
    }

    fn ensure_current_epoch(
        &self,
        current: ConnectionEpoch,
    ) -> Result<(), ChromiumTopologyEventDriverError> {
        if current == self.epoch {
            Ok(())
        } else {
            Err(ChromiumTopologyEventDriverError::StaleEpoch {
                driver: self.epoch,
                current,
            })
        }
    }
}

type BoxedCdpConnectionClosed = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChromiumCdpLivenessBackendError {
    NativeConnectionClosed,
    ConnectionTerminated { reason: ServerSupervisorCloseReason },
}

impl Display for ChromiumCdpLivenessBackendError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NativeConnectionClosed => formatter.write_str(
                "the Chromium CDP transport closed; its Browser object graph is terminal",
            ),
            Self::ConnectionTerminated { reason } => write!(
                formatter,
                "the server connection terminated as {reason:?} before Chromium disconnected"
            ),
        }
    }
}

impl Error for ChromiumCdpLivenessBackendError {}

/// Task-free liveness lane for one concrete Chromium CDP connection.
///
/// Native transport closure is an error, not a reconnect request: the
/// supervisor terminates the current Pwright connection and its Browser object
/// graph. A separately attached browser creates a new graph and new event
/// view, matching Playwright's disconnect semantics.
pub struct ChromiumCdpLivenessBackend {
    closed: Option<BoxedCdpConnectionClosed>,
    native_connection_closed: bool,
    connection_termination: Option<ServerSupervisorCloseReason>,
}

impl ChromiumCdpLivenessBackend {
    pub fn new(connection: &CdpConnection) -> Self {
        Self::from_closed_future(Box::pin(connection.closed()))
    }

    fn from_closed_future(closed: BoxedCdpConnectionClosed) -> Self {
        Self {
            closed: Some(closed),
            native_connection_closed: false,
            connection_termination: None,
        }
    }

    pub const fn native_connection_closed(&self) -> bool {
        self.native_connection_closed
    }

    pub const fn connection_termination(&self) -> Option<ServerSupervisorCloseReason> {
        self.connection_termination
    }
}

impl ConcurrentServerBackend for ChromiumCdpLivenessBackend {
    type Event = std::convert::Infallible;
    type Error = ChromiumCdpLivenessBackendError;

    fn poll_drive_once(
        &mut self,
        context: &mut Context<'_>,
    ) -> Poll<Result<Self::Event, Self::Error>> {
        if let Some(reason) = self.connection_termination {
            return Poll::Ready(Err(ChromiumCdpLivenessBackendError::ConnectionTerminated {
                reason,
            }));
        }
        if self.native_connection_closed {
            return Poll::Ready(Err(ChromiumCdpLivenessBackendError::NativeConnectionClosed));
        }
        let closed = self
            .closed
            .as_mut()
            .expect("an active liveness backend retains its close future");
        if closed.as_mut().poll(context).is_pending() {
            return Poll::Pending;
        }
        self.closed = None;
        self.native_connection_closed = true;
        Poll::Ready(Err(ChromiumCdpLivenessBackendError::NativeConnectionClosed))
    }

    fn handle_connection_terminated(&mut self, reason: ServerSupervisorCloseReason) {
        if self.connection_termination.is_none() {
            self.closed = None;
            self.connection_termination = Some(reason);
        }
    }
}

#[derive(Debug)]
pub enum ChromiumEventfulCdpLivenessBackendError {
    Liveness(ChromiumCdpLivenessBackendError),
    Runtime(RuntimeStateError),
    PublicationStateUnavailable,
}

impl Display for ChromiumEventfulCdpLivenessBackendError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Liveness(error) => Display::fmt(error, formatter),
            Self::Runtime(error) => Display::fmt(error, formatter),
            Self::PublicationStateUnavailable => {
                formatter.write_str("Browser disconnected publication state is unavailable")
            }
        }
    }
}

impl Error for ChromiumEventfulCdpLivenessBackendError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Liveness(error) => Some(error),
            Self::Runtime(error) => Some(error),
            Self::PublicationStateUnavailable => None,
        }
    }
}

/// Native CDP liveness coordinated with the optional Browser terminal event.
///
/// An owned `Browser.close` prepares runtime disposal before it closes CDP.
/// When native closure becomes visible during that transaction, this lane
/// yields to the command. After commit it continues yielding until the shared
/// one-shot publisher reaches a terminal state. A spontaneous native closure
/// while the Browser is live remains immediately terminal so connection
/// cleanup can own disposal and event publication.
pub struct ChromiumEventfulCdpLivenessBackend<Transport> {
    inner: ChromiumCdpLivenessBackend,
    signal: BrowserNativeDisconnectSignal,
    lifecycle: ControlledLifecycleHandle,
    browser: ObjectId,
    disconnected: BrowserDisconnectedPublisher<BrowserEventsCodec, Arc<Transport>>,
}

impl<Transport> ChromiumEventfulCdpLivenessBackend<Transport> {
    pub fn new(
        inner: ChromiumCdpLivenessBackend,
        lifecycle: ControlledLifecycleHandle,
        browser: ObjectId,
        disconnected: BrowserDisconnectedPublisher<BrowserEventsCodec, Arc<Transport>>,
    ) -> Self {
        Self {
            inner,
            signal: BrowserNativeDisconnectSignal::default(),
            lifecycle,
            browser,
            disconnected,
        }
    }

    pub const fn inner(&self) -> &ChromiumCdpLivenessBackend {
        &self.inner
    }

    pub fn native_disconnect_signal(&self) -> BrowserNativeDisconnectSignal {
        self.signal.clone()
    }
}

impl<Transport> ConcurrentServerBackend for ChromiumEventfulCdpLivenessBackend<Transport>
where
    Transport: FrameTransport,
{
    type Event = std::convert::Infallible;
    type Error = ChromiumEventfulCdpLivenessBackendError;

    fn poll_drive_once(
        &mut self,
        context: &mut Context<'_>,
    ) -> Poll<Result<Self::Event, Self::Error>> {
        let error = match self.inner.poll_drive_once(context) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Ok(never)) => match never {},
            Poll::Ready(Err(error)) => error,
        };
        if !matches!(
            error,
            ChromiumCdpLivenessBackendError::NativeConnectionClosed
        ) {
            return Poll::Ready(Err(ChromiumEventfulCdpLivenessBackendError::Liveness(
                error,
            )));
        }
        self.signal.mark_closed();
        if self
            .lifecycle
            .lifecycle_transaction_active()
            .map_err(ChromiumEventfulCdpLivenessBackendError::Runtime)?
        {
            return Poll::Pending;
        }
        if !self
            .lifecycle
            .is_object_live(self.browser)
            .map_err(ChromiumEventfulCdpLivenessBackendError::Runtime)?
        {
            let publication = self.disconnected.publication_state().map_err(|_| {
                ChromiumEventfulCdpLivenessBackendError::PublicationStateUnavailable
            })?;
            if matches!(
                publication,
                BrowserDisconnectedPublisherState::Ready
                    | BrowserDisconnectedPublisherState::Publishing
            ) {
                return Poll::Pending;
            }
        }
        Poll::Ready(Err(ChromiumEventfulCdpLivenessBackendError::Liveness(
            error,
        )))
    }

    fn handle_connection_terminated(&mut self, reason: ServerSupervisorCloseReason) {
        self.inner.handle_connection_terminated(reason);
    }
}

/// Policy applied when the bounded CDP event reader proves that topology
/// events were lost.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChromiumTopologyEventLossPolicy {
    /// End the remote connection so the client rebuilds one unambiguous event
    /// view on its next connection.
    TerminateConnection,
    /// Re-enumerate the browser on the same CDP connection. This is valid only
    /// when no portable Page/popup discovery lane is installed, because replay
    /// cannot otherwise distinguish delivered callbacks from lost callbacks.
    ReconstructSameConnection {
        maximum_reconstructions: NonZeroUsize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChromiumTopologyConnectionPumpTermination {
    PreviousDriveFutureDropped,
    EventStreamClosed,
    EventLossRequiresReconnect { skipped: u64 },
    ReconstructionLimitExceeded { limit: usize },
    EpochExhausted,
    DriverFailed,
    PortableEventFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChromiumTopologyConnectionPumpConfigurationError {
    PortableDiscoveryEventsCannotReplay,
}

impl Display for ChromiumTopologyConnectionPumpConfigurationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::PortableDiscoveryEventsCannotReplay => formatter.write_str(
                "same-connection topology replay cannot be selected with portable Page/popup discovery events",
            ),
        }
    }
}

impl Error for ChromiumTopologyConnectionPumpConfigurationError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChromiumTopologyConnectionPumpOutcome {
    Ignored,
    Published {
        update: CdpTopologyUpdate,
        events: ChromiumTopologyPageEventPublication,
    },
    Reconstructed {
        skipped: u64,
        epoch: ConnectionEpoch,
        update: CdpTopologyUpdate,
        events: ChromiumTopologyPageEventPublication,
    },
}

#[derive(Debug)]
pub enum ChromiumTopologyConnectionPumpError<CodecError, TransportError> {
    AlreadyTerminated {
        reason: ChromiumTopologyConnectionPumpTermination,
    },
    PreviousDriveFutureDropped,
    EventLossRequiresReconnect {
        skipped: u64,
    },
    ReconstructionLimitExceeded {
        limit: usize,
    },
    EpochExhausted,
    Driver(ChromiumTopologyEventDriverError),
    PortableEvent(ChromiumTopologyPageEventFenceError<CodecError, TransportError>),
}

impl<CodecError, TransportError> Display
    for ChromiumTopologyConnectionPumpError<CodecError, TransportError>
where
    CodecError: Display,
    TransportError: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyTerminated { reason } => {
                write!(
                    formatter,
                    "Chromium topology connection pump terminated: {reason:?}"
                )
            }
            Self::PreviousDriveFutureDropped => formatter
                .write_str("the previous Chromium topology drive future was dropped after polling"),
            Self::EventLossRequiresReconnect { skipped } => write!(
                formatter,
                "CDP topology lost {skipped} event(s); a new remote connection is required"
            ),
            Self::ReconstructionLimitExceeded { limit } => write!(
                formatter,
                "same-connection topology reconstruction limit {limit} was exhausted"
            ),
            Self::EpochExhausted => {
                formatter.write_str("Chromium connection epoch space is exhausted")
            }
            Self::Driver(error) => Display::fmt(error, formatter),
            Self::PortableEvent(error) => Display::fmt(error, formatter),
        }
    }
}

impl<CodecError, TransportError> Error
    for ChromiumTopologyConnectionPumpError<CodecError, TransportError>
where
    CodecError: Error + 'static,
    TransportError: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Driver(error) => Some(error),
            Self::PortableEvent(error) => Some(error),
            Self::AlreadyTerminated { .. }
            | Self::PreviousDriveFutureDropped
            | Self::EventLossRequiresReconnect { .. }
            | Self::ReconstructionLimitExceeded { .. }
            | Self::EpochExhausted => None,
        }
    }
}

#[derive(Debug)]
pub enum ChromiumTopologyConnectionPumpShutdownError<ControlError, TransportError> {
    Active,
    Shutdown(ServerTransportCloseError<ControlError, TransportError>),
}

impl<ControlError, TransportError> Display
    for ChromiumTopologyConnectionPumpShutdownError<ControlError, TransportError>
where
    ControlError: Display,
    TransportError: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => formatter.write_str(
                "an active Chromium topology connection pump cannot terminate its supervisor",
            ),
            Self::Shutdown(error) => Display::fmt(error, formatter),
        }
    }
}

impl<ControlError, TransportError> Error
    for ChromiumTopologyConnectionPumpShutdownError<ControlError, TransportError>
where
    ControlError: Error + 'static,
    TransportError: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Active => None,
            Self::Shutdown(error) => Some(error),
        }
    }
}

/// Caller-owned topology side of a Chromium connection loop.
///
/// The pump creates no task and owns no transport reader beyond the CDP event
/// receiver already held by its driver. Once a returned future has been
/// polled, its caller must retain it through completion. A later call detects a
/// dropped in-flight turn and makes that connection terminal.
pub struct ChromiumTopologyConnectionPump {
    driver: ChromiumTopologyEventDriver,
    publisher: ChromiumTopologyPageEventPublisher,
    loss_policy: ChromiumTopologyEventLossPolicy,
    reconstructions: usize,
    driving: bool,
    terminal: Option<ChromiumTopologyConnectionPumpTermination>,
}

impl ChromiumTopologyConnectionPump {
    pub fn new(
        driver: ChromiumTopologyEventDriver,
        publisher: ChromiumTopologyPageEventPublisher,
        loss_policy: ChromiumTopologyEventLossPolicy,
    ) -> Result<Self, ChromiumTopologyConnectionPumpConfigurationError> {
        if !publisher.is_inert()
            && matches!(
                loss_policy,
                ChromiumTopologyEventLossPolicy::ReconstructSameConnection { .. }
            )
        {
            return Err(
                ChromiumTopologyConnectionPumpConfigurationError::PortableDiscoveryEventsCannotReplay,
            );
        }
        Ok(Self {
            driver,
            publisher,
            loss_policy,
            reconstructions: 0,
            driving: false,
            terminal: None,
        })
    }

    pub const fn loss_policy(&self) -> ChromiumTopologyEventLossPolicy {
        self.loss_policy
    }

    pub const fn reconstruction_count(&self) -> usize {
        self.reconstructions
    }

    pub const fn termination(&self) -> Option<ChromiumTopologyConnectionPumpTermination> {
        if self.driving {
            Some(ChromiumTopologyConnectionPumpTermination::PreviousDriveFutureDropped)
        } else {
            self.terminal
        }
    }

    pub const fn driver(&self) -> &ChromiumTopologyEventDriver {
        &self.driver
    }

    pub async fn drive_once<Runtime, Source, Codec, Transport>(
        &mut self,
        runtime: &Runtime,
        control: &OrderedRuntimeControl<Source, Codec, Transport>,
    ) -> Result<
        ChromiumTopologyConnectionPumpOutcome,
        ChromiumTopologyConnectionPumpError<Codec::Error, Transport::Error>,
    >
    where
        Runtime: ChromiumTopologyRuntime,
        Source: ControlMessageSource,
        Codec: WireCodec<ControlMessage>,
        Transport: FrameTransport,
    {
        if let Some(reason) = self.terminal {
            return Err(ChromiumTopologyConnectionPumpError::AlreadyTerminated { reason });
        }
        if self.driving {
            self.terminal =
                Some(ChromiumTopologyConnectionPumpTermination::PreviousDriveFutureDropped);
            return Err(ChromiumTopologyConnectionPumpError::PreviousDriveFutureDropped);
        }
        self.driving = true;
        let result = self.drive_once_inner(runtime, control).await;
        self.driving = false;
        result
    }

    async fn drive_once_inner<Runtime, Source, Codec, Transport>(
        &mut self,
        runtime: &Runtime,
        control: &OrderedRuntimeControl<Source, Codec, Transport>,
    ) -> Result<
        ChromiumTopologyConnectionPumpOutcome,
        ChromiumTopologyConnectionPumpError<Codec::Error, Transport::Error>,
    >
    where
        Runtime: ChromiumTopologyRuntime,
        Source: ControlMessageSource,
        Codec: WireCodec<ControlMessage>,
        Transport: FrameTransport,
    {
        match self.driver.receive_once(runtime).await {
            Ok(outcome) => self.publish_outcome(outcome, control, false, 0).await,
            Err(ChromiumTopologyEventDriverError::ResyncRequired { skipped }) => {
                self.handle_event_loss(runtime, control, skipped).await
            }
            Err(error) => {
                self.terminal = Some(
                    if matches!(error, ChromiumTopologyEventDriverError::EventStreamClosed) {
                        ChromiumTopologyConnectionPumpTermination::EventStreamClosed
                    } else {
                        ChromiumTopologyConnectionPumpTermination::DriverFailed
                    },
                );
                Err(ChromiumTopologyConnectionPumpError::Driver(error))
            }
        }
    }

    async fn handle_event_loss<Runtime, Source, Codec, Transport>(
        &mut self,
        runtime: &Runtime,
        control: &OrderedRuntimeControl<Source, Codec, Transport>,
        skipped: u64,
    ) -> Result<
        ChromiumTopologyConnectionPumpOutcome,
        ChromiumTopologyConnectionPumpError<Codec::Error, Transport::Error>,
    >
    where
        Runtime: ChromiumTopologyRuntime,
        Source: ControlMessageSource,
        Codec: WireCodec<ControlMessage>,
        Transport: FrameTransport,
    {
        let ChromiumTopologyEventLossPolicy::ReconstructSameConnection {
            maximum_reconstructions,
        } = self.loss_policy
        else {
            self.terminal = Some(
                ChromiumTopologyConnectionPumpTermination::EventLossRequiresReconnect { skipped },
            );
            return Err(
                ChromiumTopologyConnectionPumpError::EventLossRequiresReconnect { skipped },
            );
        };
        let limit = maximum_reconstructions.get();
        if self.reconstructions >= limit {
            self.terminal = Some(
                ChromiumTopologyConnectionPumpTermination::ReconstructionLimitExceeded { limit },
            );
            return Err(ChromiumTopologyConnectionPumpError::ReconstructionLimitExceeded { limit });
        }
        let Some(epoch) = self.driver.epoch().checked_next() else {
            self.terminal = Some(ChromiumTopologyConnectionPumpTermination::EpochExhausted);
            return Err(ChromiumTopologyConnectionPumpError::EpochExhausted);
        };
        self.reconstructions = self.reconstructions.saturating_add(1);
        match self
            .driver
            .resynchronize_after_event_loss(runtime, epoch)
            .await
        {
            Ok(outcome) => self.publish_outcome(outcome, control, true, skipped).await,
            Err(error) => {
                self.terminal = Some(ChromiumTopologyConnectionPumpTermination::DriverFailed);
                Err(ChromiumTopologyConnectionPumpError::Driver(error))
            }
        }
    }

    async fn publish_outcome<Source, Codec, Transport>(
        &mut self,
        outcome: ChromiumTopologyEventDriverOutcome,
        control: &OrderedRuntimeControl<Source, Codec, Transport>,
        reconstructed: bool,
        skipped: u64,
    ) -> Result<
        ChromiumTopologyConnectionPumpOutcome,
        ChromiumTopologyConnectionPumpError<Codec::Error, Transport::Error>,
    >
    where
        Source: ControlMessageSource,
        Codec: WireCodec<ControlMessage>,
        Transport: FrameTransport,
    {
        let update = match outcome {
            ChromiumTopologyEventDriverOutcome::Ignored if !reconstructed => {
                return Ok(ChromiumTopologyConnectionPumpOutcome::Ignored);
            }
            ChromiumTopologyEventDriverOutcome::Ignored => CdpTopologyUpdate::idempotent(),
            ChromiumTopologyEventDriverOutcome::Updated(update) => update,
        };
        let events = match self
            .publisher
            .publish_after_creation_fence(&update, control)
            .await
        {
            Ok(events) => events,
            Err(error) => {
                self.terminal =
                    Some(ChromiumTopologyConnectionPumpTermination::PortableEventFailed);
                return Err(ChromiumTopologyConnectionPumpError::PortableEvent(error));
            }
        };
        if reconstructed {
            Ok(ChromiumTopologyConnectionPumpOutcome::Reconstructed {
                skipped,
                epoch: self.driver.epoch(),
                update,
                events,
            })
        } else {
            Ok(ChromiumTopologyConnectionPumpOutcome::Published { update, events })
        }
    }

    /// Runs control cleanup and transport shutdown exactly once under the
    /// supervisor's classified backend-event terminal reason.
    pub async fn terminate_supervisor<Transport, Controls>(
        &self,
        supervisor: &mut EncodedServerSupervisor<Transport>,
        controls: &mut Controls,
    ) -> Result<
        ServerSupervisorCloseReason,
        ChromiumTopologyConnectionPumpShutdownError<Controls::Error, Transport::Error>,
    >
    where
        Transport: ShutdownFrameTransport,
        Controls: ServerControlFrameHandler,
    {
        if self.termination().is_none() {
            return Err(ChromiumTopologyConnectionPumpShutdownError::Active);
        }
        supervisor
            .terminate_with_controls_and_transport(
                ServerSupervisorCloseReason::BackendEventFailed,
                controls,
            )
            .await
            .map_err(ChromiumTopologyConnectionPumpShutdownError::Shutdown)
    }
}

type BoxedChromiumTopologyDrive<CodecError, TransportError> = Pin<
    Box<
        dyn Future<
                Output = (
                    ChromiumTopologyConnectionPump,
                    Result<
                        ChromiumTopologyConnectionPumpOutcome,
                        ChromiumTopologyConnectionPumpError<CodecError, TransportError>,
                    >,
                ),
            > + Send
            + 'static,
    >,
>;

#[derive(Debug)]
pub enum ChromiumTopologyServerBackendError<CodecError, TransportError> {
    ConnectionTerminated { reason: ServerSupervisorCloseReason },
    Pump(ChromiumTopologyConnectionPumpError<CodecError, TransportError>),
}

impl<CodecError, TransportError> Display
    for ChromiumTopologyServerBackendError<CodecError, TransportError>
where
    CodecError: Display,
    TransportError: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionTerminated { reason } => write!(
                formatter,
                "the server connection terminated as {reason:?} before another topology turn"
            ),
            Self::Pump(error) => Display::fmt(error, formatter),
        }
    }
}

impl<CodecError, TransportError> Error
    for ChromiumTopologyServerBackendError<CodecError, TransportError>
where
    CodecError: Error + 'static,
    TransportError: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ConnectionTerminated { .. } => None,
            Self::Pump(error) => Some(error),
        }
    }
}

/// Adapts the Chromium topology pump to the server supervisor's fourth,
/// backend lane. The lane owns an in-flight future rather than lending one
/// from the pump, so receive, command, and outbound progress cannot cancel a
/// partially applied topology turn.
pub struct ChromiumTopologyServerBackend<Runtime, Source, Codec, Transport>
where
    Codec: WireCodec<ControlMessage>,
    Transport: FrameTransport,
{
    runtime: Arc<Runtime>,
    control: OrderedRuntimeControl<Source, Codec, Transport>,
    pump: Option<ChromiumTopologyConnectionPump>,
    in_flight: Option<BoxedChromiumTopologyDrive<Codec::Error, Transport::Error>>,
    connection_termination: Option<ServerSupervisorCloseReason>,
}

impl<Runtime, Source, Codec, Transport>
    ChromiumTopologyServerBackend<Runtime, Source, Codec, Transport>
where
    Codec: WireCodec<ControlMessage>,
    Transport: FrameTransport,
{
    pub const fn new(
        pump: ChromiumTopologyConnectionPump,
        runtime: Arc<Runtime>,
        control: OrderedRuntimeControl<Source, Codec, Transport>,
    ) -> Self {
        Self {
            runtime,
            control,
            pump: Some(pump),
            in_flight: None,
            connection_termination: None,
        }
    }

    pub const fn is_driving(&self) -> bool {
        self.in_flight.is_some()
    }

    pub const fn pump(&self) -> Option<&ChromiumTopologyConnectionPump> {
        self.pump.as_ref()
    }

    pub const fn connection_termination(&self) -> Option<ServerSupervisorCloseReason> {
        self.connection_termination
    }
}

impl<Runtime, Source, Codec, Transport> ConcurrentServerBackend
    for ChromiumTopologyServerBackend<Runtime, Source, Codec, Transport>
where
    Runtime: ChromiumTopologyRuntime + Send + Sync + 'static,
    Source: ControlMessageSource + Send + Sync + 'static,
    Codec: WireCodec<ControlMessage> + Send + Sync + 'static,
    Codec::Error: Send,
    Transport: FrameTransport + Send + Sync + 'static,
    Transport::Error: Send,
{
    type Event = ChromiumTopologyConnectionPumpOutcome;
    type Error = ChromiumTopologyServerBackendError<Codec::Error, Transport::Error>;

    fn poll_drive_once(
        &mut self,
        context: &mut Context<'_>,
    ) -> Poll<Result<Self::Event, Self::Error>> {
        if let Some(reason) = self.connection_termination {
            return Poll::Ready(Err(
                ChromiumTopologyServerBackendError::ConnectionTerminated { reason },
            ));
        }
        if self.in_flight.is_none() {
            let mut pump = self
                .pump
                .take()
                .expect("a topology pump is absent only while its drive future is retained");
            let runtime = Arc::clone(&self.runtime);
            let control = self.control.clone();
            self.in_flight = Some(Box::pin(async move {
                let result = pump.drive_once(runtime.as_ref(), &control).await;
                (pump, result)
            }));
        }

        let output = {
            let drive = self
                .in_flight
                .as_mut()
                .expect("a topology drive must exist after initialization");
            match drive.as_mut().poll(context) {
                Poll::Ready(output) => output,
                Poll::Pending => return Poll::Pending,
            }
        };
        self.in_flight = None;
        self.pump = Some(output.0);
        Poll::Ready(output.1.map_err(ChromiumTopologyServerBackendError::Pump))
    }

    fn handle_connection_terminated(&mut self, reason: ServerSupervisorCloseReason) {
        if self.connection_termination.is_none() {
            self.in_flight = None;
            self.connection_termination = Some(reason);
        }
    }
}

#[cfg(test)]
mod tests {
    mod committed_observer;
    mod discovery_retirement;
    mod document_navigation;
    #[cfg(feature = "frame-lifecycle")]
    mod lifecycle_observation;
    #[cfg(feature = "frame-lifecycle")]
    mod lifecycle_registry;
    #[cfg(feature = "close-route-retirement")]
    mod native_page_worlds;
    use std::collections::BTreeMap;
    use std::convert::Infallible;
    use std::future::{Future, Ready, ready};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll, Waker};

    use pwright_backend_cdp::{
        CdpContextOwnership, CdpEvaluationLimits, CdpEvaluationProductionProfile,
        CdpFrameAttachment, CdpNetworkEventProductionProfile, CdpPageEventAdapter,
        CdpPageObjectIds, CdpPageTarget, CdpTopologyLimits, CdpTopologyProductionProfile,
        ConnectionEpoch, ProductionCdpTopology,
    };
    use pwright_browser_model::{
        BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND, FRAME_OBJECT_KIND, PAGE_OBJECT_KIND,
    };
    use pwright_cap_evaluation::EvaluationValueLimits;
    use pwright_cap_network_events::NETWORK_REQUEST_EVENT_TAG;
    use pwright_cap_page_events::{BrowserContextPage, PageLoad};
    use pwright_cdp::connection::{CdpError, ConnectionConfig};
    use pwright_cdp::generated::page;
    use pwright_cdp::{CdpClient, CdpConnectionProductionProfile, CdpEvent, SessionFactory};
    use pwright_codec_browser_events::BrowserEventsCodec;
    use pwright_codec_control::ControlCodec;
    use pwright_codec_page_events::PageEventsCodec;
    use pwright_core::{
        AtomicObjectIdSource, ObjectId, ObjectIdSource, ObjectKindTag, OverflowPolicy,
        ProductionResourceLedger, ProtocolVersion, ResourceClass, ResourceLimits,
        ResourceRequirements, ResourceScope,
    };
    use pwright_fake::FakeCdpClient;
    use pwright_network_model::NetworkLifecycleLimits;
    use pwright_runtime::{
        ControlledDispatcher, DirectDispatcher, NoDeadlineScheduler, RuntimeLimits,
    };
    use pwright_runtime_control::{
        OrderedRuntimeControl, RuntimeControlDriver, SubscriptionActivator,
    };
    use pwright_server_browser_events::BrowserDisconnectedPublisher;
    use pwright_server_events::TypedEventOutbound;
    use pwright_server_wire::{
        ConcurrentServerBackend, ConcurrentServerBackendExt, EncodedServerSupervisor,
        ServerBackendChainError, ServerControlFrameHandler, ServerFrameDisposition,
        ServerSupervisorCloseReason, ServerSupervisorLimits, ServerSupervisorPhase,
    };
    use pwright_transport::{
        FrameLimits, FrameTransport, ShutdownFrameTransport, WireFrame, WireProtocol,
    };
    use tokio::sync::broadcast;

    use super::{
        ChromiumAuxiliarySessionParent, ChromiumAuxiliarySessionParentError,
        ChromiumCdpLivenessBackend, ChromiumCdpLivenessBackendError,
        ChromiumEventfulCdpLivenessBackend, ChromiumEventfulCdpLivenessBackendError,
        ChromiumPageDiscoveryProductionProfile, ChromiumTopologyConnectionPump,
        ChromiumTopologyConnectionPumpConfigurationError, ChromiumTopologyConnectionPumpError,
        ChromiumTopologyConnectionPumpOutcome, ChromiumTopologyConnectionPumpTermination,
        ChromiumTopologyEventDriver, ChromiumTopologyEventDriverError,
        ChromiumTopologyEventDriverOutcome, ChromiumTopologyEventDriverProductionProfile,
        ChromiumTopologyEventLossPolicy, ChromiumTopologyServerBackend,
        ChromiumTopologyServerBackendError, ChromiumUtilityWorldProfile,
    };
    use crate::{
        CapabilityWireProfile, CdpPageRouteProductionProfile, CdpPageServices,
        ChromiumAttachProductionProfile, ChromiumNetworkEventDriver,
        ChromiumNetworkEventDriverShared, ChromiumNetworkEventDriverState,
        ChromiumNetworkEventsProductionProfile, ChromiumPageLoadServerBackend,
        ChromiumPageLoadServerBackendOutcome, ChromiumPageSubscriptionActivator,
        ChromiumTopologyPageEventPublisher, EvaluationInstallationProfile, SharedChromiumTopology,
    };

    #[derive(Default)]
    struct PumpTransport {
        sent: Mutex<Vec<WireFrame>>,
        shutdowns: AtomicUsize,
    }

    impl FrameTransport for PumpTransport {
        type Error = Infallible;
        type Send<'a> = Ready<Result<(), Self::Error>>;
        type Receive<'a> = Ready<Result<Option<WireFrame>, Self::Error>>;

        fn send(&self, frame: WireFrame) -> Self::Send<'_> {
            self.sent
                .lock()
                .expect("pump frames should be available")
                .push(frame);
            ready(Ok(()))
        }

        fn receive(&self) -> Self::Receive<'_> {
            ready(Ok(None))
        }
    }

    impl ShutdownFrameTransport for PumpTransport {
        type Shutdown<'a> = Ready<Result<(), Self::Error>>;

        fn shutdown(&self) -> Self::Shutdown<'_> {
            self.shutdowns.fetch_add(1, Ordering::SeqCst);
            ready(Ok(()))
        }
    }

    struct PumpCloseControl {
        reasons: Arc<Mutex<Vec<ServerSupervisorCloseReason>>>,
    }

    impl ServerControlFrameHandler for PumpCloseControl {
        type Event = ();
        type Error = Infallible;
        type Future<'a> = Ready<Result<ServerFrameDisposition<Self::Event>, Self::Error>>;
        type CloseFuture<'a> = Ready<Result<Option<Self::Event>, Self::Error>>;

        fn handle_control<'a>(&'a mut self, frame: WireFrame) -> Self::Future<'a> {
            ready(Ok(ServerFrameDisposition::Unhandled(frame)))
        }

        fn handle_transport_closed<'a>(&'a mut self) -> Self::CloseFuture<'a> {
            ready(Ok(None))
        }

        fn handle_connection_terminated<'a>(
            &'a mut self,
            reason: ServerSupervisorCloseReason,
        ) -> Self::CloseFuture<'a> {
            self.reasons
                .lock()
                .expect("pump close reasons should be available")
                .push(reason);
            ready(Ok(None))
        }
    }

    fn pump_control_protocol() -> WireProtocol<ControlCodec> {
        WireProtocol::new_exact_directional(
            ControlCodec,
            ProtocolVersion::new(1, 0),
            FrameLimits::new(64, 0, 0, 64),
            FrameLimits::new(64, 0, 0, 64),
        )
    }

    fn browser_event_protocol() -> WireProtocol<BrowserEventsCodec> {
        WireProtocol::new_exact_directional(
            BrowserEventsCodec,
            ProtocolVersion::new(1, 0),
            FrameLimits::new(17, 0, 0, 17),
            FrameLimits::new(17, 0, 0, 17),
        )
    }

    fn empty_pump_control()
    -> OrderedRuntimeControl<Arc<DirectDispatcher<()>>, ControlCodec, Arc<PumpTransport>> {
        OrderedRuntimeControl::new(
            Arc::new(DirectDispatcher::new(
                (),
                RuntimeLimits::new(1, 1).with_control_messages(1),
            )),
            RuntimeControlDriver::new(pump_control_protocol(), Arc::new(PumpTransport::default())),
            1,
        )
        .expect("pump control should build")
    }

    #[derive(Default)]
    struct RecordingSessionFactory {
        created: Mutex<Vec<(String, String)>>,
        clients: Mutex<BTreeMap<String, Arc<FakeCdpClient>>>,
        frame_trees: Mutex<BTreeMap<String, page::FrameTree>>,
    }

    impl RecordingSessionFactory {
        fn client(&self, target_id: &str) -> Arc<FakeCdpClient> {
            Arc::clone(
                self.clients
                    .lock()
                    .expect("test client table should be available")
                    .get(target_id)
                    .expect("target session should have been initialized"),
            )
        }

        fn set_frame_tree(&self, target_id: &str, frame_tree: page::FrameTree) {
            self.frame_trees
                .lock()
                .expect("frame-tree fixtures should be available")
                .insert(target_id.to_owned(), frame_tree);
        }
    }

    impl SessionFactory for RecordingSessionFactory {
        fn create_session(&self, session_id: String, target_id: String) -> Arc<dyn CdpClient> {
            self.created
                .lock()
                .expect("created-session log should be available")
                .push((session_id, target_id.clone()));
            if let Some(existing) = self
                .clients
                .lock()
                .expect("fake session clients should be available")
                .get(&target_id)
                .cloned()
            {
                return existing;
            }
            let client = Arc::new(FakeCdpClient::from_html(""));
            client.set_main_frame_id(target_id.clone());
            if let Some(frame_tree) = self
                .frame_trees
                .lock()
                .expect("frame-tree fixtures should be available")
                .get(&target_id)
                .cloned()
            {
                client.set_frame_tree(frame_tree);
            }
            self.clients
                .lock()
                .expect("test client table should be available")
                .insert(target_id, Arc::clone(&client));
            client
        }
    }

    struct DriverFixture {
        sender: broadcast::Sender<CdpEvent>,
        topology: SharedChromiumTopology,
        runtime: DirectDispatcher<CdpPageServices>,
        driver: ChromiumTopologyEventDriver,
        factory: Arc<RecordingSessionFactory>,
        browser: Arc<FakeCdpClient>,
        network_driver: Option<ChromiumNetworkEventDriver>,
    }

    struct RecordingInitializer {
        factory: Arc<RecordingSessionFactory>,
        calls: Mutex<Vec<(String, CdpContextOwnership, bool)>>,
        fail: bool,
    }

    struct GatedInitializer {
        factory: Arc<RecordingSessionFactory>,
        gate: Arc<tokio::sync::Notify>,
    }

    struct HeaderFenceInitializer {
        enqueue_gate: Arc<tokio::sync::Notify>,
        acknowledgement_gate: Arc<tokio::sync::Notify>,
        queued_header: Mutex<Option<(&'static str, &'static str)>>,
    }

    impl super::ChromiumSessionInitializer for HeaderFenceInitializer {
        fn initialize<'a>(
            &'a self,
            _session_id: &'a str,
            _target_id: &'a str,
            _ownership: CdpContextOwnership,
            _is_page: bool,
        ) -> super::ChromiumSessionInitializationFuture<'a> {
            panic!("header fence needs scoped attachment identity")
        }

        fn initialize_scoped<'a>(
            &'a self,
            target: super::ChromiumSessionInitializationTarget<'a>,
        ) -> super::ChromiumSessionInitializationFuture<'a> {
            Box::pin(async move {
                let fence = target.resume_fence.expect("discovered popup fence");
                fence.require_enqueue();
                self.enqueue_gate.notified().await;
                *self.queued_header.lock().unwrap() = Some(("foo", "bar"));
                fence.command_enqueued();
                self.acknowledgement_gate.notified().await;
                Ok(())
            })
        }
    }

    impl super::ChromiumSessionInitializer for GatedInitializer {
        fn initialize<'a>(
            &'a self,
            _session_id: &'a str,
            target_id: &'a str,
            _ownership: CdpContextOwnership,
            _is_page: bool,
        ) -> super::ChromiumSessionInitializationFuture<'a> {
            Box::pin(async move {
                assert!(
                    !self
                        .factory
                        .client(target_id)
                        .method_names()
                        .iter()
                        .any(|method| method == "Runtime.runIfWaitingForDebugger"),
                    "session settings must start before debugger resume"
                );
                self.gate.notified().await;
                Ok(())
            })
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct ScopedInitializationRecord {
        session: String,
        target: String,
        page_target: String,
        page: ObjectId,
        context: ObjectId,
        ownership: CdpContextOwnership,
        kind: super::ChromiumSessionTargetKind,
        page_was_published: bool,
    }

    struct ScopedInitializer {
        topology: SharedChromiumTopology,
        factory: Arc<RecordingSessionFactory>,
        calls: Mutex<Vec<ScopedInitializationRecord>>,
        fail: bool,
    }

    impl super::ChromiumSessionInitializer for ScopedInitializer {
        fn initialize<'a>(
            &'a self,
            _session_id: &'a str,
            _target_id: &'a str,
            _ownership: CdpContextOwnership,
            _is_page: bool,
        ) -> super::ChromiumSessionInitializationFuture<'a> {
            panic!("driver must use the identity-aware entry point")
        }

        fn initialize_scoped<'a>(
            &'a self,
            target: super::ChromiumSessionInitializationTarget<'a>,
        ) -> super::ChromiumSessionInitializationFuture<'a> {
            Box::pin(async move {
                assert!(
                    !self
                        .factory
                        .client(target.target_id)
                        .method_names()
                        .iter()
                        .any(|method| method == "Runtime.runIfWaitingForDebugger")
                );
                let published = self
                    .topology
                    .try_lock()
                    .expect("initializer must not hold topology lock")
                    .topology()
                    .page_object_id(target.page_target_id)
                    .is_some();
                self.calls.lock().unwrap().push(ScopedInitializationRecord {
                    session: target.session_id.to_owned(),
                    target: target.target_id.to_owned(),
                    page_target: target.page_target_id.to_owned(),
                    page: target.page,
                    context: target.context,
                    ownership: target.ownership,
                    kind: target.kind,
                    page_was_published: published,
                });
                if self.fail {
                    return Err(std::io::Error::other("scoped settings rejected").into());
                }
                Ok(())
            })
        }
    }

    fn scoped_initializer(fixture: &DriverFixture, fail: bool) -> Arc<ScopedInitializer> {
        Arc::new(ScopedInitializer {
            topology: fixture.topology.clone(),
            factory: fixture.factory.clone(),
            calls: Mutex::new(Vec::new()),
            fail,
        })
    }

    #[tokio::test]
    async fn scoped_initializer_receives_reserved_page_identity_before_publication() {
        // Arrange
        let mut fixture = discovery_fixture(8);
        let initializer = scoped_initializer(&fixture, false);
        fixture.driver = fixture.driver.with_session_initializer(initializer.clone());
        fixture
            .sender
            .send(attached_page_event("page-a", "session-a", None, None))
            .unwrap();
        // Act
        fixture.driver.receive_once(&fixture.runtime).await.unwrap();
        // Assert
        let topology = fixture.topology.lock().unwrap();
        let page = topology.topology().page_object_id("page-a").unwrap();
        let context = topology
            .topology()
            .default_context_route()
            .unwrap()
            .object_id();
        assert_eq!(
            *initializer.calls.lock().unwrap(),
            [ScopedInitializationRecord {
                session: "session-a".into(),
                target: "page-a".into(),
                page_target: "page-a".into(),
                page,
                context,
                ownership: CdpContextOwnership::Default,
                kind: super::ChromiumSessionTargetKind::Page,
                page_was_published: false,
            }]
        );
    }

    #[tokio::test]
    async fn scoped_initializer_failure_keeps_reserved_page_unpublished_and_paused() {
        // Arrange
        let mut fixture = discovery_fixture(8);
        let initializer = scoped_initializer(&fixture, true);
        fixture.driver = fixture.driver.with_session_initializer(initializer.clone());
        fixture
            .sender
            .send(attached_page_event("page-a", "session-a", None, None))
            .unwrap();
        // Act
        let result = fixture.driver.receive_once(&fixture.runtime).await;
        // Assert
        assert!(matches!(
            result,
            Err(ChromiumTopologyEventDriverError::SessionInitialization(_))
        ));
        assert_eq!(initializer.calls.lock().unwrap().len(), 1);
        assert!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .page_object_id("page-a")
                .is_none()
        );
        assert!(
            !fixture
                .factory
                .client("page-a")
                .method_names()
                .iter()
                .any(|method| method == "Runtime.runIfWaitingForDebugger")
        );
    }

    #[tokio::test]
    async fn scoped_oopif_initializer_inherits_owning_page_and_context_without_reinitializing_parent()
     {
        // Arrange
        let mut fixture = fixture(8);
        let initializer = scoped_initializer(&fixture, false);
        fixture.driver = fixture.driver.with_session_initializer(initializer.clone());
        fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 0)
            .await
            .unwrap();
        assert_eq!(initializer.calls.lock().unwrap().len(), 1);
        initializer.calls.lock().unwrap().clear();
        fixture.factory.set_frame_tree(
            "frame-oopif",
            frame_tree(serde_json::json!({
                "frame": frame_value("frame-oopif", Some("target-a"))
            })),
        );
        fixture
            .sender
            .send(CdpEvent::new(
                "Target.attachedToTarget".into(),
                serde_json::json!({
                    "sessionId": "session-oopif",
                    "targetInfo": { "targetId": "frame-oopif", "type": "iframe", "title": "",
                        "url": "https://other.example", "attached": true, "canAccessOpener": false,
                        "parentFrameId": "target-a" },
                    "waitingForDebugger": true
                }),
                Some("session-a".into()),
            ))
            .unwrap();
        // Act
        fixture.driver.receive_once(&fixture.runtime).await.unwrap();
        // Assert
        assert_eq!(
            *initializer.calls.lock().unwrap(),
            [ScopedInitializationRecord {
                session: "session-oopif".into(),
                target: "frame-oopif".into(),
                page_target: "target-a".into(),
                page: ObjectId::new(3),
                context: ObjectId::new(2),
                ownership: CdpContextOwnership::External,
                kind: super::ChromiumSessionTargetKind::OutOfProcessFrame,
                page_was_published: true,
            }]
        );
        assert_eq!(fixture.driver.initialized_session_count(), 2);
    }

    impl super::ChromiumSessionInitializer for RecordingInitializer {
        fn initialize<'a>(
            &'a self,
            _session_id: &'a str,
            target_id: &'a str,
            ownership: CdpContextOwnership,
            is_page: bool,
        ) -> super::ChromiumSessionInitializationFuture<'a> {
            Box::pin(async move {
                assert!(
                    !self
                        .factory
                        .client(target_id)
                        .method_names()
                        .iter()
                        .any(|method| method == "Runtime.runIfWaitingForDebugger"),
                    "settings must be applied before resuming the target"
                );
                self.calls
                    .lock()
                    .unwrap()
                    .push((target_id.to_owned(), ownership, is_page));
                if self.fail {
                    return Err(std::io::Error::other("fixture initializer failed").into());
                }
                Ok(())
            })
        }
    }

    #[tokio::test]
    async fn selected_session_initializer_runs_before_resume_and_page_ready() {
        // Arrange
        let mut fixture = discovery_fixture(8);
        fixture.factory.set_frame_tree(
            "page-a",
            frame_tree(serde_json::json!({
                "frame": frame_value("page-a", None),
            })),
        );
        let initializer = Arc::new(RecordingInitializer {
            factory: fixture.factory.clone(),
            calls: Mutex::new(Vec::new()),
            fail: false,
        });
        fixture.driver = fixture.driver.with_session_initializer(initializer.clone());
        fixture
            .sender
            .send(attached_page_event("page-a", "session-a", None, None))
            .unwrap();
        // Act
        fixture.driver.receive_once(&fixture.runtime).await.unwrap();
        // Assert
        assert_eq!(
            *initializer.calls.lock().unwrap(),
            [("page-a".to_owned(), CdpContextOwnership::Default, true)]
        );
        assert_eq!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .page_state("page-a"),
            Some(&pwright_backend_cdp::CdpPageState::Ready)
        );
    }

    #[tokio::test]
    async fn failed_session_initializer_prevents_resume_and_page_publication() {
        // Arrange
        let mut fixture = discovery_fixture(8);
        let initializer = Arc::new(RecordingInitializer {
            factory: fixture.factory.clone(),
            calls: Mutex::new(Vec::new()),
            fail: true,
        });
        fixture.driver = fixture.driver.with_session_initializer(initializer.clone());
        fixture
            .sender
            .send(attached_page_event("page-a", "session-a", None, None))
            .unwrap();
        // Act
        let result = fixture.driver.receive_once(&fixture.runtime).await;
        // Assert
        assert!(matches!(
            result,
            Err(ChromiumTopologyEventDriverError::SessionInitialization(_))
        ));
        assert!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .page_object_id("page-a")
                .is_none()
        );
        assert!(
            !initializer
                .factory
                .client("page-a")
                .method_names()
                .iter()
                .any(|method| method == "Runtime.runIfWaitingForDebugger")
        );
    }

    fn exact_limits(requirements: ResourceRequirements) -> ResourceLimits {
        let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let required = requirements.class(class);
            limits = limits.with_class(class, required.global, required.per_scope);
        }
        limits
    }

    fn profiles(
        event_channel_capacity: usize,
    ) -> (
        ChromiumAttachProductionProfile,
        ChromiumTopologyEventDriverProductionProfile,
    ) {
        profiles_with_runtime(event_channel_capacity, RuntimeLimits::new(16, 4))
    }

    fn profiles_with_runtime(
        event_channel_capacity: usize,
        runtime: RuntimeLimits,
    ) -> (
        ChromiumAttachProductionProfile,
        ChromiumTopologyEventDriverProductionProfile,
    ) {
        let connection = CdpConnectionProductionProfile::new(
            ResourceScope::Connection,
            ConnectionConfig {
                event_channel_capacity,
                max_command_bytes: 4096,
                max_response_bytes: 4096,
                max_event_bytes: 4096,
                max_websocket_bytes: 4096,
                ..ConnectionConfig::default()
            },
        );
        let topology = CdpTopologyProductionProfile::new(
            CdpTopologyLimits::new(16, 2, 2, 8, 8, 128),
            ResourceScope::extension(41, 1),
        )
        .expect("topology profile should validate");
        let page_routes =
            CdpPageRouteProductionProfile::from_topology(ResourceScope::extension(41, 2), topology);
        let topology_events = ChromiumTopologyEventDriverProductionProfile::from_profiles(
            ResourceScope::extension(41, 3),
            &connection,
            topology,
        )
        .expect("topology-event profile should validate");
        let values = EvaluationValueLimits::new(8, 64, 64, 2048, 2048)
            .expect("evaluation values should validate");
        let evaluation = EvaluationInstallationProfile::new(
            CdpEvaluationProductionProfile::new(
                ResourceScope::extension(41, 4),
                CdpEvaluationLimits::new(2048, 32 * 1024, 1024, values),
                2,
            ),
            pwright_codec_evaluation::EvaluationCodecLimits::new(2048, 1024, values)
                .expect("evaluation codec should validate"),
            CapabilityWireProfile::new(
                ProtocolVersion::new(1, 0),
                FrameLimits::new(4096, 0, 0, 4096),
                FrameLimits::new(4096, 0, 0, 4096),
            ),
        );
        let profile = ChromiumAttachProductionProfile::new(
            connection,
            topology,
            page_routes,
            runtime,
            ServerSupervisorLimits::new(2).expect("supervisor limits should validate"),
        )
        .with_evaluation(evaluation)
        .with_topology_events(topology_events);
        (profile, topology_events)
    }

    fn fixture(event_channel_capacity: usize) -> DriverFixture {
        fixture_with_utility_world(event_channel_capacity, None)
    }

    fn fixture_with_utility_world(
        event_channel_capacity: usize,
        utility_world: Option<ChromiumUtilityWorldProfile>,
    ) -> DriverFixture {
        fixture_with_registration(event_channel_capacity, utility_world, false)
    }

    fn fixture_with_registration(
        event_channel_capacity: usize,
        utility_world: Option<ChromiumUtilityWorldProfile>,
        parallel: bool,
    ) -> DriverFixture {
        fixture_with_startup(event_channel_capacity, utility_world, parallel, false)
    }

    fn fixture_with_startup(
        event_channel_capacity: usize,
        utility_world: Option<ChromiumUtilityWorldProfile>,
        parallel: bool,
        recursive: bool,
    ) -> DriverFixture {
        let (profile, topology_events) = profiles(event_channel_capacity);
        let topology_events = if recursive {
            topology_events
                .with_parallel_recursive_attachment()
                .unwrap()
        } else if parallel {
            topology_events
                .with_parallel_utility_world_registration()
                .unwrap()
        } else {
            topology_events
        };
        let profile = profile.with_topology_events(topology_events);
        let profile = match utility_world {
            Some(utility_world) => profile.with_utility_world(utility_world),
            None => profile,
        };
        let requirements = profile.requirements();
        let resources = ProductionResourceLedger::new(exact_limits(requirements), requirements)
            .expect("declared production plan should fit");
        let epoch = pwright_backend_cdp::ConnectionEpoch::new(1);
        let mut topology =
            ProductionCdpTopology::new(ObjectId::new(1), epoch, profile.topology(), &resources)
                .expect("topology should build");
        topology
            .register_context(
                epoch,
                "context-a",
                ObjectId::new(2),
                CdpContextOwnership::External,
            )
            .expect("context should register");
        topology
            .attach_page(
                epoch,
                CdpPageTarget::new("target-a", "session-a", "context-a"),
                CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
            )
            .expect("Page should attach");
        let topology = Arc::new(Mutex::new(topology));
        let factory = Arc::new(RecordingSessionFactory::default());
        let session_factory: Arc<dyn SessionFactory> = factory.clone();
        let services = CdpPageServices::new_with_production_resources(
            Arc::clone(&session_factory),
            &profile,
            &resources,
        )
        .expect("Page routes should build");
        services
            .synchronize(&topology.lock().expect("topology should be available"))
            .expect("initial Page route should publish");
        let runtime = DirectDispatcher::new(services, RuntimeLimits::new(16, 4));
        runtime
            .register_object(ObjectId::new(1), None, BROWSER_OBJECT_KIND.tag)
            .expect("Browser should register");
        runtime
            .register_object(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                BROWSER_CONTEXT_OBJECT_KIND.tag,
            )
            .expect("BrowserContext should register");
        runtime
            .register_object(
                ObjectId::new(3),
                Some(ObjectId::new(2)),
                PAGE_OBJECT_KIND.tag,
            )
            .expect("Page should register");
        runtime
            .register_object(
                ObjectId::new(4),
                Some(ObjectId::new(3)),
                FRAME_OBJECT_KIND.tag,
            )
            .expect("main Frame should register");
        let (sender, receiver) = broadcast::channel(event_channel_capacity);
        let object_ids: Arc<dyn ObjectIdSource> =
            Arc::new(AtomicObjectIdSource::new(5).expect("object IDs should start above zero"));
        let browser = Arc::new(FakeCdpClient::from_html(""));
        let browser_client: Arc<dyn CdpClient> = browser.clone();
        let driver = ChromiumTopologyEventDriver::new_with_production_resources(
            receiver,
            browser_client,
            session_factory,
            Arc::clone(&topology),
            object_ids,
            epoch,
            topology_events,
            None,
            utility_world,
            &resources,
        )
        .expect("topology-event driver should build");
        DriverFixture {
            sender,
            topology,
            runtime,
            driver,
            factory,
            browser,
            network_driver: None,
        }
    }

    fn discovery_fixture(event_channel_capacity: usize) -> DriverFixture {
        discovery_fixture_with_registration(event_channel_capacity, false)
    }

    fn discovery_fixture_with_registration(
        event_channel_capacity: usize,
        parallel: bool,
    ) -> DriverFixture {
        discovery_fixture_with_startup(event_channel_capacity, parallel, false)
    }

    fn discovery_fixture_with_startup(
        event_channel_capacity: usize,
        parallel: bool,
        recursive: bool,
    ) -> DriverFixture {
        let (profile, topology_events) = profiles(event_channel_capacity);
        let utility_world =
            (parallel || recursive).then_some(ChromiumUtilityWorldProfile::playwright_compatible());
        let topology_events = if recursive {
            topology_events
                .with_parallel_recursive_attachment()
                .unwrap()
        } else if parallel {
            topology_events
                .with_parallel_utility_world_registration()
                .unwrap()
        } else {
            topology_events
        };
        let profile = profile.with_topology_events(topology_events);
        let profile = match utility_world {
            Some(world) => profile.with_utility_world(world),
            None => profile,
        };
        let page_discovery = ChromiumPageDiscoveryProductionProfile::from_profiles(
            profile.connection(),
            profile.topology(),
        );
        let profile = profile.with_page_discovery(page_discovery);
        let requirements = profile.requirements();
        let resources = ProductionResourceLedger::new(exact_limits(requirements), requirements)
            .expect("declared discovery plan should fit");
        let epoch = ConnectionEpoch::new(1);
        let topology = Arc::new(Mutex::new(
            ProductionCdpTopology::new(ObjectId::new(1), epoch, profile.topology(), &resources)
                .expect("topology should build"),
        ));
        let factory = Arc::new(RecordingSessionFactory::default());
        let session_factory: Arc<dyn SessionFactory> = factory.clone();
        let services = CdpPageServices::new_with_production_resources(
            Arc::clone(&session_factory),
            &profile,
            &resources,
        )
        .expect("Page routes should build");
        let runtime = DirectDispatcher::new(services, RuntimeLimits::new(16, 4));
        runtime
            .register_object(ObjectId::new(1), None, BROWSER_OBJECT_KIND.tag)
            .expect("Browser should register");
        let (sender, receiver) = broadcast::channel(event_channel_capacity);
        let object_ids: Arc<dyn ObjectIdSource> =
            Arc::new(AtomicObjectIdSource::new(2).expect("object IDs should start above zero"));
        let browser = Arc::new(FakeCdpClient::from_html(""));
        let browser_client: Arc<dyn CdpClient> = browser.clone();
        let driver = ChromiumTopologyEventDriver::new_with_production_resources(
            receiver,
            browser_client,
            session_factory,
            Arc::clone(&topology),
            object_ids,
            epoch,
            topology_events,
            Some(page_discovery),
            utility_world,
            &resources,
        )
        .expect("discovery driver should build");
        DriverFixture {
            sender,
            topology,
            runtime,
            driver,
            factory,
            browser,
            network_driver: None,
        }
    }

    fn network_discovery_fixture(event_channel_capacity: usize) -> DriverFixture {
        let runtime_limits = RuntimeLimits::new(20, 4);
        let (profile, topology_events) =
            profiles_with_runtime(event_channel_capacity, runtime_limits);
        let page_discovery = ChromiumPageDiscoveryProductionProfile::from_profiles(
            profile.connection(),
            profile.topology(),
        );
        let adapter = CdpNetworkEventProductionProfile::new(
            NetworkLifecycleLimits::new(2, 64, 256, 16, 32, 8, 256, 64, 64, 128),
            ResourceScope::extension(41, 5),
        )
        .expect("network adapter profile should validate");
        let network = ChromiumNetworkEventsProductionProfile::new_with_context_limit(adapter, 2, 2)
            .expect("network discovery limits should validate");
        let profile = profile
            .with_page_discovery(page_discovery)
            .with_network_events(network);
        let requirements = profile.requirements();
        let resources = ProductionResourceLedger::new(exact_limits(requirements), requirements)
            .expect("declared network discovery plan should fit");
        let epoch = ConnectionEpoch::new(1);
        let mut topology =
            ProductionCdpTopology::new(ObjectId::new(1), epoch, profile.topology(), &resources)
                .expect("topology should build");
        topology
            .register_context(
                epoch,
                "context-a",
                ObjectId::new(2),
                CdpContextOwnership::External,
            )
            .expect("Context should register");
        let topology = Arc::new(Mutex::new(topology));
        let factory = Arc::new(RecordingSessionFactory::default());
        let session_factory: Arc<dyn SessionFactory> = factory.clone();
        let services = CdpPageServices::new_with_production_resources(
            Arc::clone(&session_factory),
            &profile,
            &resources,
        )
        .expect("network Page routes should build");
        services
            .synchronize(&topology.lock().expect("topology should be available"))
            .expect("Context route should synchronize");
        let runtime = DirectDispatcher::new(services, runtime_limits);
        runtime
            .register_object(ObjectId::new(1), None, BROWSER_OBJECT_KIND.tag)
            .expect("Browser should register");
        runtime
            .register_object(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                BROWSER_CONTEXT_OBJECT_KIND.tag,
            )
            .expect("Context should register");
        let (sender, receiver) = broadcast::channel(event_channel_capacity);
        let object_ids: Arc<dyn ObjectIdSource> =
            Arc::new(AtomicObjectIdSource::new(3).expect("object IDs should start above Context"));
        let browser = Arc::new(FakeCdpClient::from_html(""));
        let subscriptions = runtime
            .service()
            .network_subscription_registry()
            .expect("network subscriptions should install");
        let shared = Arc::new(ChromiumNetworkEventDriverShared {
            state: Mutex::new(ChromiumNetworkEventDriverState {
                adapters: BTreeMap::new(),
                related_sessions: BTreeMap::new(),
                pending_page_registration: None,
                pending_page_replacement: None,
                unpublished_batch: None,
            }),
            subscriptions,
            object_ids: Arc::new(
                AtomicObjectIdSource::new(20).expect("network IDs should start above topology"),
            ),
            adapter_profile: adapter,
            resources: resources.clone(),
            max_page_lanes: 2,
            max_related_sessions: 0,
        });
        let network_driver = ChromiumNetworkEventDriver {
            receiver: sender.subscribe().into(),
            shared,
            held_event: None,
        };
        let browser_client: Arc<dyn CdpClient> = browser.clone();
        let driver = ChromiumTopologyEventDriver::new_with_production_resources(
            receiver,
            browser_client,
            session_factory,
            Arc::clone(&topology),
            object_ids,
            epoch,
            topology_events,
            Some(page_discovery),
            None,
            &resources,
        )
        .expect("network discovery driver should build")
        .with_network_registration(network_driver.registration_handle());
        DriverFixture {
            sender,
            topology,
            runtime,
            driver,
            factory,
            browser,
            network_driver: Some(network_driver),
        }
    }

    #[cfg(feature = "request-frame")]
    async fn network_frame_cursor_fixture() -> DriverFixture {
        let mut fixture = network_discovery_fixture(16);
        ChromiumPageSubscriptionActivator::new(fixture.runtime.service().clone())
            .set_subscription(ObjectId::new(2), NETWORK_REQUEST_EVENT_TAG, true)
            .await
            .expect("Context network interest should activate");
        fixture.factory.set_frame_tree(
            "page-a",
            frame_tree(serde_json::json!({
                "frame": frame_value("page-a", None)
            })),
        );
        fixture
            .sender
            .send(attached_page_event(
                "page-a",
                "session-a",
                Some("context-a"),
                None,
            ))
            .expect("Page attachment should enqueue");
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("Page and network lane should materialize");
        assert!(
            fixture
                .network_driver
                .as_mut()
                .expect("network driver should be selected")
                .receive_once()
                .await
                .expect("Page attachment should be network-inert")
                .is_none()
        );
        fixture
    }

    fn frame_tree(value: serde_json::Value) -> page::FrameTree {
        serde_json::from_value(value).expect("frame-tree fixture should match generated CDP types")
    }

    fn frame_value(id: &str, parent_id: Option<&str>) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "parentId": parent_id,
            "loaderId": format!("loader-{id}"),
            "url": "about:blank",
            "domainAndRegistry": "",
            "securityOrigin": "://",
            "mimeType": "text/html",
            "secureContextType": "InsecureScheme",
            "crossOriginIsolatedContextType": "NotIsolated",
            "gatedAPIFeatures": []
        })
    }

    fn attached_page_event(
        target_id: &str,
        session_id: &str,
        browser_context_id: Option<&str>,
        opener_id: Option<&str>,
    ) -> CdpEvent {
        CdpEvent::new(
            "Target.attachedToTarget".to_owned(),
            serde_json::json!({
                "sessionId": session_id,
                "targetInfo": {
                    "targetId": target_id,
                    "type": "page",
                    "title": "",
                    "url": "about:blank",
                    "attached": true,
                    "canAccessOpener": false,
                    "browserContextId": browser_context_id,
                    "openerId": opener_id
                },
                "waitingForDebugger": true
            }),
            None,
        )
    }

    #[derive(Default)]
    struct UrlProjection(Mutex<BTreeMap<ObjectId, String>>);

    impl super::ChromiumFrameUrlObserver for UrlProjection {
        fn update(
            &self,
            frame: ObjectId,
            url: &str,
            fragment: Option<&str>,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            self.0
                .lock()
                .unwrap()
                .insert(frame, format!("{url}{}", fragment.unwrap_or("")));
            Ok(())
        }

        fn retain_live(
            &self,
            is_live: &dyn Fn(ObjectId) -> bool,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            self.0.lock().unwrap().retain(|frame, _| is_live(*frame));
            Ok(())
        }
    }

    #[tokio::test]
    async fn frame_url_observer_projects_initial_tree_including_fragment() {
        // Arrange
        let mut fixture = fixture(16);
        let observer = Arc::new(UrlProjection::default());
        fixture.driver = fixture.driver.with_frame_url_observer(observer.clone());
        let mut frame = frame_value("target-a", None);
        frame["url"] = serde_json::json!("https://initial.test/");
        frame["urlFragment"] = serde_json::json!("#fragment");
        fixture
            .factory
            .client("target-a")
            .set_frame_tree(frame_tree(serde_json::json!({ "frame": frame })));
        // Act
        fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 0)
            .await
            .unwrap();
        // Assert
        assert_eq!(
            observer.0.lock().unwrap().get(&ObjectId::new(4)).unwrap(),
            "https://initial.test/#fragment"
        );
    }

    #[derive(Debug, PartialEq, Eq)]
    struct NameObservation {
        frame: ObjectId,
        name: Option<String>,
    }

    #[derive(Default)]
    struct NavigationProjection(Mutex<Vec<NameObservation>>);

    impl super::ChromiumFrameUrlObserver for NavigationProjection {
        fn update(
            &self,
            frame: ObjectId,
            _: &str,
            _: Option<&str>,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            self.0
                .lock()
                .unwrap()
                .push(NameObservation { frame, name: None });
            Ok(())
        }
        fn update_navigation(
            &self,
            frame: ObjectId,
            _: &str,
            _: Option<&str>,
            name: &str,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            self.0.lock().unwrap().push(NameObservation {
                frame,
                name: Some(name.into()),
            });
            Ok(())
        }
        fn retain_live(
            &self,
            _: &dyn Fn(ObjectId) -> bool,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn initial_frame_tree_forwards_navigation_name() {
        // Arrange
        let mut fixture = fixture(16);
        let observer = Arc::new(NavigationProjection::default());
        fixture.driver = fixture.driver.with_frame_url_observer(observer.clone());
        let mut frame = frame_value("target-a", None);
        frame["name"] = serde_json::json!("initial-name");
        fixture
            .factory
            .client("target-a")
            .set_frame_tree(frame_tree(serde_json::json!({ "frame": frame })));
        // Act
        fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 0)
            .await
            .unwrap();
        // Assert
        assert_eq!(
            *observer.0.lock().unwrap(),
            vec![NameObservation {
                frame: ObjectId::new(4),
                name: Some("initial-name".into())
            }]
        );
    }

    #[tokio::test]
    async fn main_frame_navigation_with_missing_name_explicitly_clears_name() {
        // Arrange
        let mut fixture = fixture(16);
        let observer = Arc::new(NavigationProjection::default());
        fixture.driver = fixture.driver.with_frame_url_observer(observer.clone());
        fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 0)
            .await
            .unwrap();
        observer.0.lock().unwrap().clear();
        fixture
            .sender
            .send(CdpEvent::new(
                "Page.frameNavigated".into(),
                serde_json::json!({ "frame": frame_value("main-new", None), "type": "Navigation" }),
                Some("session-a".into()),
            ))
            .unwrap();
        // Act
        fixture.driver.receive_once(&fixture.runtime).await.unwrap();
        // Assert
        assert_eq!(
            *observer.0.lock().unwrap(),
            vec![NameObservation {
                frame: ObjectId::new(4),
                name: Some("".into())
            }]
        );
    }

    #[tokio::test]
    async fn same_document_navigation_does_not_report_a_name_change() {
        // Arrange
        let mut fixture = fixture(16);
        let observer = Arc::new(NavigationProjection::default());
        fixture.driver = fixture.driver.with_frame_url_observer(observer.clone());
        fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 0)
            .await
            .unwrap();
        observer.0.lock().unwrap().clear();
        fixture.sender.send(CdpEvent::new("Page.navigatedWithinDocument".into(), serde_json::json!({ "frameId": "target-a", "url": "about:blank#new", "navigationType": "fragment" }), Some("session-a".into()))).unwrap();
        // Act
        fixture.driver.receive_once(&fixture.runtime).await.unwrap();
        // Assert
        assert_eq!(
            *observer.0.lock().unwrap(),
            vec![NameObservation {
                frame: ObjectId::new(4),
                name: None
            }]
        );
    }

    #[tokio::test]
    async fn frame_url_observer_preserves_portable_identity_on_main_frame_replacement() {
        // Arrange
        let mut fixture = fixture(16);
        let observer = Arc::new(UrlProjection::default());
        fixture.driver = fixture.driver.with_frame_url_observer(observer.clone());
        fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 0)
            .await
            .unwrap();
        let mut frame = frame_value("main-new", None);
        frame["url"] = serde_json::json!("https://replacement.test/");
        fixture
            .sender
            .send(CdpEvent::new(
                "Page.frameNavigated".into(),
                serde_json::json!({ "frame": frame, "type": "Navigation" }),
                Some("session-a".into()),
            ))
            .unwrap();
        // Act
        fixture.driver.receive_once(&fixture.runtime).await.unwrap();
        // Assert
        assert_eq!(
            *observer.0.lock().unwrap(),
            BTreeMap::from([(ObjectId::new(4), "https://replacement.test/".into())])
        );
        assert_eq!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .frame_object_id("main-new"),
            Some(ObjectId::new(4))
        );
    }

    #[tokio::test]
    async fn idle_signal_leaves_queued_url_events_for_the_bounded_fence() {
        // Arrange
        let mut fixture = fixture(16);
        let observer = Arc::new(UrlProjection::default());
        fixture.driver = fixture.driver.with_frame_url_observer(observer.clone());
        fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 0)
            .await
            .unwrap();
        fixture.sender.send(CdpEvent::new("Page.navigatedWithinDocument".into(), serde_json::json!({ "frameId": "target-a", "url": "about:blank#new", "navigationType": "fragment" }), Some("session-a".into()))).unwrap();
        // Act
        fixture
            .driver
            .receive_once_or_signal(&fixture.runtime, ready(()))
            .await
            .unwrap();
        // Assert
        assert_eq!(
            observer.0.lock().unwrap().get(&ObjectId::new(4)).unwrap(),
            "about:blank"
        );
        fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 1)
            .await
            .unwrap();
        assert_eq!(
            observer.0.lock().unwrap().get(&ObjectId::new(4)).unwrap(),
            "about:blank#new"
        );
    }

    #[tokio::test]
    async fn unknown_session_cannot_overwrite_a_live_frame_url() {
        // Arrange
        let mut fixture = fixture(16);
        let observer = Arc::new(UrlProjection::default());
        fixture.driver = fixture.driver.with_frame_url_observer(observer.clone());
        fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 0)
            .await
            .unwrap();
        fixture.sender.send(CdpEvent::new("Page.navigatedWithinDocument".into(), serde_json::json!({ "frameId": "target-a", "url": "about:blank#stale", "navigationType": "fragment" }), Some("retired-session".into()))).unwrap();
        // Act
        fixture.driver.receive_once(&fixture.runtime).await.unwrap();
        // Assert
        assert_eq!(
            observer.0.lock().unwrap().get(&ObjectId::new(4)).unwrap(),
            "about:blank"
        );
    }

    #[tokio::test]
    async fn frame_url_observer_releases_active_entries_after_target_retirement() {
        // Arrange
        let mut fixture = fixture(16);
        let observer = Arc::new(UrlProjection::default());
        fixture.driver = fixture.driver.with_frame_url_observer(observer.clone());
        fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 0)
            .await
            .unwrap();
        fixture
            .sender
            .send(CdpEvent::new(
                "Target.detachedFromTarget".into(),
                serde_json::json!({ "sessionId": "session-a", "targetId": "target-a" }),
                None,
            ))
            .unwrap();
        // Act
        fixture.driver.receive_once(&fixture.runtime).await.unwrap();
        // Assert
        assert!(observer.0.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn frame_url_observer_failure_propagates_instead_of_reporting_a_fresh_snapshot() {
        struct Reject;
        impl super::ChromiumFrameUrlObserver for Reject {
            fn update(
                &self,
                _: ObjectId,
                _: &str,
                _: Option<&str>,
            ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                Err(std::io::Error::other("metadata budget exhausted").into())
            }
            fn retain_live(
                &self,
                _: &dyn Fn(ObjectId) -> bool,
            ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                Ok(())
            }
        }
        // Arrange
        let mut fixture = fixture(16);
        fixture.driver = fixture.driver.with_frame_url_observer(Arc::new(Reject));
        // Act
        let result = fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 0)
            .await;
        // Assert
        assert!(matches!(
            result,
            Err(super::ChromiumTopologyEventDriverError::FrameUrlObserver(_))
        ));
    }

    #[tokio::test]
    async fn utility_world_recovers_only_a_proven_frame_disappearance() {
        // Arrange
        let mut fixture = fixture_with_utility_world(
            8,
            Some(ChromiumUtilityWorldProfile::playwright_compatible()),
        );
        let _: Arc<dyn CdpClient> = fixture
            .factory
            .create_session("session-a".to_owned(), "target-a".to_owned());
        let client = fixture.factory.client("target-a");
        let initial = frame_tree(serde_json::json!({
            "frame": frame_value("target-a", None),
            "childFrames": [{"frame": frame_value("vanished-child", Some("target-a"))}]
        }));
        let refreshed = frame_tree(serde_json::json!({
            "frame": frame_value("target-a", None)
        }));
        client.set_frame_trees([initial, refreshed]);
        client.set_create_isolated_world_errors([
            None,
            Some("No frame with given id found".to_owned()),
            None,
        ]);
        fixture
            .sender
            .send(CdpEvent::new(
                "Runtime.executionContextCreated".to_owned(),
                serde_json::json!({
                    "context": {
                        "id": 17,
                        "origin": "",
                        "name": "__pwright_utility_0000000000000003",
                        "uniqueId": "utility-context-a",
                        "auxData": {"frameId": "target-a", "isDefault": false}
                    }
                }),
                Some("session-a".to_owned()),
            ))
            .expect("one synchronization event should enqueue");

        // Act
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("a frame proven absent by a fresh tree should be a tolerated race");

        // Assert
        let expected_world = "__pwright_utility_0000000000000003";
        let calls = client.calls();
        let methods: Vec<_> = calls.iter().map(|(method, _)| method.as_str()).collect();
        assert_eq!(
            methods,
            [
                "Page.enable",
                "Page.getFrameTree",
                "Runtime.enable",
                "Page.addScriptToEvaluateOnNewDocument",
                "Page.createIsolatedWorld",
                "Page.createIsolatedWorld",
                "Page.getFrameTree",
                "Page.createIsolatedWorld",
                "Target.setAutoAttach",
            ]
        );
        assert_eq!(calls[3].1[0]["source"], "");
        assert_eq!(calls[3].1[0]["worldName"], expected_world);
        for index in [4, 5, 7] {
            assert_eq!(calls[index].1[0]["worldName"], expected_world);
            assert_eq!(calls[index].1[0]["grantUniveralAccess"], true);
        }
        let topology = fixture
            .topology
            .lock()
            .expect("topology should remain available");
        assert_eq!(
            topology.topology().frame_object_id("target-a"),
            Some(ObjectId::new(4))
        );
        assert_eq!(topology.topology().frame_object_id("vanished-child"), None);
        let utility_routes: Vec<_> = topology
            .topology()
            .frame_utility_execution_routes()
            .map(|route| {
                (
                    route.object_id(),
                    route.session_id().to_owned(),
                    route.unique_context_id().to_owned(),
                )
            })
            .collect();
        assert_eq!(
            utility_routes,
            [(
                ObjectId::new(4),
                "session-a".to_owned(),
                "utility-context-a".to_owned()
            )]
        );
    }

    #[tokio::test]
    async fn utility_world_does_not_hide_a_create_failure_for_a_live_frame() {
        // Arrange
        let mut fixture = fixture_with_utility_world(
            8,
            Some(ChromiumUtilityWorldProfile::playwright_compatible()),
        );
        let _: Arc<dyn CdpClient> = fixture
            .factory
            .create_session("session-a".to_owned(), "target-a".to_owned());
        let client = fixture.factory.client("target-a");
        let current = frame_tree(serde_json::json!({
            "frame": frame_value("target-a", None)
        }));
        client.set_frame_trees([current.clone(), current]);
        client.set_create_isolated_world_errors([Some("world creation failed".to_owned())]);

        // Act
        let error = fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect_err("a still-live frame must preserve its CDP failure");

        // Assert
        assert!(matches!(
            error,
            ChromiumTopologyEventDriverError::Cdp(CdpError::Protocol {
                code: -32000,
                ref message,
            }) if message == "world creation failed"
        ));
        assert!(
            !client
                .method_names()
                .contains(&"Target.setAutoAttach".to_owned())
        );
    }

    #[tokio::test]
    async fn recursive_attachment_discovery_preserves_settings_world_and_resume_fences() {
        // Arrange
        let mut fixture = discovery_fixture_with_startup(8, false, true);
        let initializer = Arc::new(RecordingInitializer {
            factory: fixture.factory.clone(),
            calls: Mutex::new(Vec::new()),
            fail: false,
        });
        fixture.driver = fixture.driver.with_session_initializer(initializer.clone());
        fixture
            .sender
            .send(attached_page_event("page-a", "session-a", None, None))
            .unwrap();
        // Act
        fixture.driver.receive_once(&fixture.runtime).await.unwrap();
        // Assert
        assert_eq!(initializer.calls.lock().unwrap().len(), 1);
        assert_eq!(
            fixture.factory.client("page-a").method_names(),
            [
                "Page.enable",
                "Target.setAutoAttach",
                "Page.getFrameTree",
                "Runtime.enable",
                "Page.addScriptToEvaluateOnNewDocument",
                "Page.createIsolatedWorld",
                "Runtime.runIfWaitingForDebugger",
            ]
        );
        assert_eq!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .page_state("page-a"),
            Some(&pwright_backend_cdp::CdpPageState::Ready)
        );
    }

    #[tokio::test]
    async fn recursive_attachment_cannot_bypass_failed_context_settings() {
        // Arrange
        let mut fixture = discovery_fixture_with_startup(8, false, true);
        fixture.driver = fixture
            .driver
            .with_session_initializer(Arc::new(RecordingInitializer {
                factory: fixture.factory.clone(),
                calls: Mutex::new(Vec::new()),
                fail: true,
            }));
        fixture
            .sender
            .send(attached_page_event("page-a", "session-a", None, None))
            .unwrap();
        // Act
        let result = fixture.driver.receive_once(&fixture.runtime).await;
        // Assert
        assert!(matches!(
            result,
            Err(ChromiumTopologyEventDriverError::SessionInitialization(_))
        ));
        assert_eq!(
            fixture.factory.client("page-a").method_names(),
            [
                "Page.enable",
                "Target.setAutoAttach",
                "Page.getFrameTree",
                "Runtime.enable",
                "Page.addScriptToEvaluateOnNewDocument",
            ]
        );
        assert!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .page_object_id("page-a")
                .is_none()
        );
    }

    #[tokio::test]
    async fn recursive_attachment_world_failure_resumes_but_does_not_publish_page() {
        // Arrange
        let mut fixture = discovery_fixture_with_startup(8, false, true);
        fixture
            .factory
            .create_session("session-a".into(), "page-a".into());
        let client = fixture.factory.client("page-a");
        client.set_create_isolated_world_errors([Some("world creation failed".into())]);
        fixture
            .sender
            .send(attached_page_event("page-a", "session-a", None, None))
            .unwrap();
        // Act
        let result = fixture.driver.receive_once(&fixture.runtime).await;
        // Assert
        assert!(
            matches!(result, Err(ChromiumTopologyEventDriverError::Cdp(CdpError::Protocol { ref message, .. })) if message == "world creation failed")
        );
        assert_eq!(
            client
                .method_names()
                .iter()
                .filter(|method| *method == "Target.setAutoAttach")
                .count(),
            1
        );
        assert_eq!(
            client
                .method_names()
                .iter()
                .filter(|method| *method == "Runtime.runIfWaitingForDebugger")
                .count(),
            1
        );
        assert!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .page_object_id("page-a")
                .is_none()
        );
    }

    #[tokio::test]
    async fn pending_page_enable_does_not_block_network_popup_resume() {
        // Arrange
        let mut fixture = discovery_fixture_with_startup(8, false, true);
        fixture
            .sender
            .send(attached_page_event("opener", "session-opener", None, None))
            .unwrap();
        fixture.driver.receive_once(&fixture.runtime).await.unwrap();
        fixture
            .factory
            .create_session("session-a".into(), "page-a".into());
        let client = fixture.factory.client("page-a");
        let gate = Arc::new(tokio::sync::Notify::new());
        client.set_page_enable_gate(Some(gate.clone()));
        fixture
            .sender
            .send(attached_page_event(
                "page-a",
                "session-a",
                None,
                Some("opener"),
            ))
            .unwrap();
        let mut operation = Box::pin(fixture.driver.receive_once(&fixture.runtime));

        // Act
        std::future::poll_fn(|cx| {
            assert!(operation.as_mut().poll(cx).is_pending());
            std::task::Poll::Ready(())
        })
        .await;

        // Assert
        let methods = client.method_names();
        assert!(methods.iter().any(|name| name == "Page.enable"));
        assert!(
            methods
                .iter()
                .any(|name| name == "Runtime.runIfWaitingForDebugger")
        );
        gate.notify_one();
        operation.await.unwrap();
        assert_eq!(
            client
                .method_names()
                .iter()
                .filter(|name| *name == "Runtime.runIfWaitingForDebugger")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn pending_session_settings_do_not_block_network_popup_resume() {
        // Arrange
        let mut fixture = discovery_fixture_with_startup(8, false, true);
        let gate = Arc::new(tokio::sync::Notify::new());
        fixture.driver = fixture
            .driver
            .with_session_initializer(Arc::new(GatedInitializer {
                factory: fixture.factory.clone(),
                gate: gate.clone(),
            }));
        fixture
            .sender
            .send(attached_page_event("page-a", "session-a", None, None))
            .unwrap();
        let mut operation = Box::pin(fixture.driver.receive_once(&fixture.runtime));

        // Act
        std::future::poll_fn(|cx| {
            assert!(operation.as_mut().poll(cx).is_pending());
            std::task::Poll::Ready(())
        })
        .await;

        // Assert
        let client = fixture.factory.client("page-a");
        let methods = client.method_names();
        let runtime = methods
            .iter()
            .position(|name| name == "Runtime.enable")
            .expect("Runtime observation should be installed");
        let resume = methods
            .iter()
            .position(|name| name == "Runtime.runIfWaitingForDebugger")
            .expect("the paused target should be resumed");
        assert!(runtime < resume, "Runtime observation must precede resume");
        assert_eq!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .page_state("page-a"),
            Some(&pwright_backend_cdp::CdpPageState::Initializing),
            "the observer route exists before resume without reporting a public Page"
        );
        gate.notify_one();
        operation.await.unwrap();
        assert_eq!(
            client
                .method_names()
                .iter()
                .filter(|name| *name == "Runtime.runIfWaitingForDebugger")
                .count(),
            1
        );
        assert_eq!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .page_state("page-a"),
            Some(&pwright_backend_cdp::CdpPageState::Ready)
        );
    }

    #[tokio::test]
    async fn popup_header_fence_waits_for_command_enqueue_but_not_acknowledgement() {
        // Arrange
        let mut fixture = discovery_fixture_with_startup(8, false, true);
        let initializer = Arc::new(HeaderFenceInitializer {
            enqueue_gate: Arc::new(tokio::sync::Notify::new()),
            acknowledgement_gate: Arc::new(tokio::sync::Notify::new()),
            queued_header: Mutex::new(None),
        });
        fixture.driver = fixture.driver.with_session_initializer(initializer.clone());
        fixture
            .sender
            .send(attached_page_event("page-a", "session-a", None, None))
            .unwrap();
        let mut operation = Box::pin(fixture.driver.receive_once(&fixture.runtime));

        // Act: a selected header command is still waiting for queue admission.
        std::future::poll_fn(|cx| {
            assert!(operation.as_mut().poll(cx).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        let client = fixture.factory.client("page-a");
        assert!(
            !client
                .method_names()
                .iter()
                .any(|method| method == "Runtime.runIfWaitingForDebugger")
        );
        initializer.enqueue_gate.notify_one();
        std::future::poll_fn(|cx| {
            assert!(operation.as_mut().poll(cx).is_pending());
            std::task::Poll::Ready(())
        })
        .await;

        // Assert: the fake first request sees the header at resume, while the
        // native acknowledgement remains intentionally pending.
        assert_eq!(
            *initializer.queued_header.lock().unwrap(),
            Some(("foo", "bar"))
        );
        assert!(
            client
                .method_names()
                .iter()
                .any(|method| method == "Runtime.runIfWaitingForDebugger")
        );
        initializer.acknowledgement_gate.notify_one();
        operation.await.unwrap();
    }

    #[tokio::test]
    async fn pending_isolated_world_creation_does_not_block_target_resume() {
        // Arrange
        let mut fixture = discovery_fixture_with_startup(8, false, true);
        fixture
            .factory
            .create_session("session-a".into(), "page-a".into());
        let client = fixture.factory.client("page-a");
        let gate = Arc::new(tokio::sync::Notify::new());
        client.set_create_isolated_world_gate(Some(gate.clone()));
        fixture
            .sender
            .send(attached_page_event("page-a", "session-a", None, None))
            .unwrap();
        let mut operation = Box::pin(fixture.driver.receive_once(&fixture.runtime));
        // Act
        std::future::poll_fn(|cx| {
            assert!(operation.as_mut().poll(cx).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        // Assert: resume must be sent while world creation is still held.
        assert!(
            client
                .method_names()
                .iter()
                .any(|name| name == "Runtime.runIfWaitingForDebugger")
        );
        assert_eq!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .page_state("page-a"),
            Some(&pwright_backend_cdp::CdpPageState::Initializing),
            "utility-world readiness must not erase the pre-publication owner route"
        );
        gate.notify_one();
        operation.await.unwrap();
        assert_eq!(
            client
                .method_names()
                .iter()
                .filter(|name| *name == "Runtime.runIfWaitingForDebugger")
                .count(),
            1
        );
        assert_eq!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .page_state("page-a"),
            Some(&pwright_backend_cdp::CdpPageState::Ready)
        );
    }

    #[tokio::test]
    async fn initializing_popup_dialog_publishes_page_before_world_creation_settles() {
        // Arrange
        let mut fixture = discovery_fixture_with_startup(16, false, true);
        fixture
            .sender
            .send(attached_page_event("opener", "session-opener", None, None))
            .unwrap();
        fixture.driver.receive_once(&fixture.runtime).await.unwrap();
        fixture
            .factory
            .create_session("session-popup".into(), "popup".into());
        let client = fixture.factory.client("popup");
        let gate = Arc::new(tokio::sync::Notify::new());
        client.set_create_isolated_world_gate(Some(gate.clone()));
        fixture
            .sender
            .send(attached_page_event(
                "popup",
                "session-popup",
                None,
                Some("opener"),
            ))
            .unwrap();
        let mut attachment = Box::pin(fixture.driver.receive_once(&fixture.runtime));
        std::future::poll_fn(|context| {
            assert!(attachment.as_mut().poll(context).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        let opening = CdpEvent::new(
            "Page.javascriptDialogOpening".into(),
            serde_json::json!({
                "url": "about:blank",
                "frameId": "popup-frame",
                "message": "hey?",
                "type": "prompt",
                "hasBrowserHandler": true,
                "defaultPrompt": ""
            }),
            Some("session-popup".into()),
        );
        client.send_event(opening.clone());
        fixture.sender.send(opening).unwrap();

        // Act
        let attached = attachment.await.unwrap();

        // Assert
        let ChromiumTopologyEventDriverOutcome::Updated(update) = attached else {
            panic!("dialog-interrupted popup attachment should publish topology");
        };
        let semantic = update
            .effects()
            .iter()
            .filter(|effect| {
                matches!(
                    effect,
                    pwright_backend_cdp::CdpTopologyEffect::PageReady { .. }
                        | pwright_backend_cdp::CdpTopologyEffect::PopupLinked { .. }
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            semantic,
            [
                pwright_backend_cdp::CdpTopologyEffect::PageReady {
                    page: ObjectId::new(5),
                    context: ObjectId::new(2),
                },
                pwright_backend_cdp::CdpTopologyEffect::PopupLinked {
                    opener: ObjectId::new(3),
                    popup: ObjectId::new(5),
                }
            ]
        );
        assert!(
            fixture
                .driver
                .initialized_sessions
                .get("session-popup")
                .is_some_and(|session| session.resume_after_dialog)
        );

        // Arrange: consume the opening on the authoritative cursor, then
        // model the user accepting it and Chromium reporting closure.
        fixture.driver.receive_once(&fixture.runtime).await.unwrap();
        gate.notify_one();
        fixture
            .sender
            .send(CdpEvent::new(
                "Page.javascriptDialogClosed".into(),
                serde_json::json!({
                    "frameId": "popup-frame",
                    "result": true,
                    "userInput": "hello"
                }),
                Some("session-popup".into()),
            ))
            .unwrap();

        // Act
        let resumed = fixture.driver.receive_once(&fixture.runtime).await.unwrap();

        // Assert
        assert!(
            fixture
                .driver
                .initialized_sessions
                .get("session-popup")
                .is_some_and(|session| !session.resume_after_dialog)
        );
        if let ChromiumTopologyEventDriverOutcome::Updated(update) = resumed {
            assert!(!update.effects().iter().any(|effect| matches!(
                effect,
                pwright_backend_cdp::CdpTopologyEffect::PageReady { .. }
                    | pwright_backend_cdp::CdpTopologyEffect::PopupLinked { .. }
            )));
        }
    }

    #[tokio::test]
    async fn untracked_parent_attachment_is_released_through_its_announcing_parent() {
        // Arrange
        let mut fixture = fixture(8);
        fixture
            .sender
            .send(CdpEvent::new(
                "Target.attachedToTarget".into(),
                serde_json::json!({
                    "sessionId": "unwanted-child", "waitingForDebugger": true,
                    "targetInfo": {"targetId":"child-a", "type":"iframe", "title":"", "url":"",
                        "attached":true, "canAccessOpener":false, "parentFrameId":"target-a"}
                }),
                Some("old-parent-session".into()),
            ))
            .unwrap();
        // Act
        let result = fixture.driver.receive_once(&fixture.runtime).await;
        // Assert
        result.unwrap();
        assert!(fixture.browser.calls().iter().any(|(method, args)| method
            == "Target.detachFromTargetInParent"
            && *args
                == vec![
                    serde_json::json!("old-parent-session"),
                    serde_json::json!("unwanted-child")
                ]));
    }

    #[tokio::test]
    async fn recursive_attachment_existing_routes_enable_once_with_or_without_worlds() {
        for world in [
            None,
            Some(ChromiumUtilityWorldProfile::playwright_compatible()),
        ] {
            // Arrange
            let mut fixture = fixture_with_startup(8, world, false, true);
            // Act
            fixture
                .driver
                .initialize_snapshot(&fixture.runtime, 0)
                .await
                .unwrap();
            // Assert
            let methods = fixture.factory.client("target-a").method_names();
            let expected = if world.is_some() {
                vec![
                    "Page.enable",
                    "Page.getFrameTree",
                    "Runtime.enable",
                    "Page.addScriptToEvaluateOnNewDocument",
                    "Target.setAutoAttach",
                    "Page.createIsolatedWorld",
                ]
            } else {
                vec![
                    "Page.enable",
                    "Page.getFrameTree",
                    "Runtime.enable",
                    "Target.setAutoAttach",
                ]
            };
            assert_eq!(methods, expected);
        }
    }

    #[tokio::test]
    async fn recursive_attachment_announcements_remain_queued_until_parent_publication() {
        // Arrange
        let mut fixture = discovery_fixture_with_startup(8, false, true);
        fixture
            .factory
            .create_session("session-a".into(), "page-a".into());
        fixture.factory.client("page-a").set_auto_attach_events(
            fixture.sender.clone(),
            [attached_page_event(
                "page-b",
                "session-b",
                None,
                Some("page-a"),
            )],
        );
        fixture
            .sender
            .send(attached_page_event("page-a", "session-a", None, None))
            .unwrap();
        // Act
        fixture.driver.receive_once(&fixture.runtime).await.unwrap();
        // Assert: recursive wire delivery does not recursively publish a Page.
        let topology = fixture.topology.lock().unwrap();
        assert_eq!(
            topology.topology().page_state("page-a"),
            Some(&pwright_backend_cdp::CdpPageState::Ready)
        );
        assert!(topology.topology().page_object_id("page-b").is_none());
        drop(topology);
        assert!(!fixture.sender.is_empty());
    }

    #[tokio::test]
    async fn parallel_registration_discovery_preserves_context_world_and_resume_fences() {
        // Arrange
        let mut fixture = discovery_fixture_with_registration(8, true);
        let initializer = Arc::new(RecordingInitializer {
            factory: fixture.factory.clone(),
            calls: Mutex::new(Vec::new()),
            fail: false,
        });
        fixture.driver = fixture.driver.with_session_initializer(initializer.clone());
        fixture
            .sender
            .send(attached_page_event("page-a", "session-a", None, None))
            .unwrap();
        // Act
        fixture.driver.receive_once(&fixture.runtime).await.unwrap();
        // Assert
        assert_eq!(initializer.calls.lock().unwrap().len(), 1);
        assert_eq!(
            fixture.factory.client("page-a").method_names(),
            [
                "Page.enable",
                "Page.getFrameTree",
                "Runtime.enable",
                "Page.addScriptToEvaluateOnNewDocument",
                "Page.createIsolatedWorld",
                "Target.setAutoAttach",
                "Runtime.runIfWaitingForDebugger",
            ]
        );
        assert_eq!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .page_state("page-a"),
            Some(&pwright_backend_cdp::CdpPageState::Ready)
        );
    }

    #[tokio::test]
    async fn parallel_registration_world_failure_does_not_resume_or_publish_discovered_page() {
        // Arrange
        let mut fixture = discovery_fixture_with_registration(8, true);
        fixture
            .factory
            .create_session("session-a".into(), "page-a".into());
        let client = fixture.factory.client("page-a");
        client.set_create_isolated_world_errors([Some("world creation failed".into())]);
        fixture
            .sender
            .send(attached_page_event("page-a", "session-a", None, None))
            .unwrap();
        // Act
        let result = fixture.driver.receive_once(&fixture.runtime).await;
        // Assert
        assert!(
            matches!(result, Err(ChromiumTopologyEventDriverError::Cdp(CdpError::Protocol { ref message, .. })) if message == "world creation failed")
        );
        assert!(!client.method_names().iter().any(|method| matches!(
            method.as_str(),
            "Runtime.runIfWaitingForDebugger" | "Target.setAutoAttach"
        )));
        assert!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .page_object_id("page-a")
                .is_none()
        );
    }

    #[tokio::test]
    async fn parallel_registration_existing_route_registers_once_and_preserves_frame_disappearance_check()
     {
        // Arrange
        let mut fixture = fixture_with_registration(
            8,
            Some(ChromiumUtilityWorldProfile::playwright_compatible()),
            true,
        );
        fixture
            .factory
            .create_session("session-a".into(), "target-a".into());
        let client = fixture.factory.client("target-a");
        client.set_frame_trees([
            frame_tree(serde_json::json!({ "frame": frame_value("target-a", None), "childFrames": [{"frame": frame_value("vanished-child", Some("target-a"))}] })),
            frame_tree(serde_json::json!({ "frame": frame_value("target-a", None) })),
        ]);
        client.set_create_isolated_world_errors([
            None,
            Some("No frame with given id found".into()),
            None,
        ]);
        // Act
        fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 0)
            .await
            .unwrap();
        // Assert
        assert_eq!(
            client.method_names(),
            [
                "Page.enable",
                "Page.getFrameTree",
                "Runtime.enable",
                "Page.addScriptToEvaluateOnNewDocument",
                "Page.createIsolatedWorld",
                "Page.createIsolatedWorld",
                "Page.getFrameTree",
                "Page.createIsolatedWorld",
                "Target.setAutoAttach",
            ]
        );
        assert!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .frame_object_id("vanished-child")
                .is_none()
        );
    }

    #[tokio::test]
    async fn parallel_registration_without_utility_capability_sends_no_world_commands() {
        // Arrange
        let mut fixture = fixture_with_registration(8, None, true);
        // Act
        fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 0)
            .await
            .unwrap();
        // Assert
        assert_eq!(
            fixture.factory.client("target-a").method_names(),
            [
                "Page.enable",
                "Page.getFrameTree",
                "Runtime.enable",
                "Target.setAutoAttach",
            ]
        );
    }

    #[tokio::test]
    async fn initial_snapshot_exposes_default_context_without_waiting_for_a_page_event() {
        // Arrange
        let mut fixture = discovery_fixture(8);

        // Act
        let outcome = fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 0)
            .await;

        // Assert
        assert!(outcome.is_ok());
        let topology = fixture
            .topology
            .lock()
            .expect("topology should be available");
        assert!(topology.topology().default_context_route().is_some());
        assert_eq!(topology.topology().page_session_routes().count(), 0);
        assert_eq!(fixture.runtime.live_object_count(), Ok(2));
    }

    #[tokio::test]
    async fn page_discovery_auto_attach_excludes_unselected_target_classes() {
        // Arrange
        let mut fixture = discovery_fixture(8);

        // Act
        fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 0)
            .await
            .expect("initial discovery should install auto-attach");

        // Assert
        let calls = fixture.browser.calls();
        assert_eq!(
            calls[0],
            (
                "Target.setAutoAttach".to_owned(),
                vec![serde_json::json!({
                    "autoAttach": true,
                    "waitForDebuggerOnStart": true,
                    "flatten": true,
                    "includedTargetTypes": ["page", "iframe"]
                })]
            )
        );
    }

    #[tokio::test]
    async fn selected_worker_observer_adds_worker_to_auto_attach_filter() {
        // Arrange
        let mut fixture = discovery_fixture(8);
        fixture.driver = fixture
            .driver
            .with_frame_event_observer(Arc::new(WorkerRetentionObserver { network: false }))
            .expect("worker observer should fit the declared profile");

        // Act
        fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 0)
            .await
            .expect("initial discovery should install auto-attach");

        // Assert
        let calls = fixture.browser.calls();
        assert_eq!(
            calls[0],
            (
                "Target.setAutoAttach".to_owned(),
                vec![serde_json::json!({
                    "autoAttach": true,
                    "waitForDebuggerOnStart": true,
                    "flatten": true,
                    "includedTargetTypes": ["page", "iframe", "worker"]
                })]
            )
        );
    }

    #[tokio::test]
    async fn initial_snapshot_drains_preexisting_target_announcements() {
        // Arrange
        let mut fixture = discovery_fixture(8);
        fixture
            .sender
            .send(attached_page_event("page-a", "session-a", None, None))
            .expect("Page attachment should enqueue");

        // Act
        let outcome = fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 1)
            .await;

        // Assert
        assert!(outcome.is_ok());
        let topology = fixture
            .topology
            .lock()
            .expect("topology should be available");
        assert_eq!(topology.topology().page_session_routes().count(), 1);
        assert!(topology.topology().main_frame_object_id("page-a").is_some());
        assert_eq!(fixture.runtime.live_object_count(), Ok(4));
    }

    #[tokio::test]
    async fn initial_snapshot_fails_closed_when_event_budget_is_exceeded() {
        // Arrange
        let mut fixture = discovery_fixture(8);
        fixture
            .sender
            .send(attached_page_event("page-a", "session-a", None, None))
            .expect("Page attachment should enqueue");

        // Act
        let outcome = fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 0)
            .await;

        // Assert
        assert!(matches!(
            outcome,
            Err(ChromiumTopologyEventDriverError::InitialSnapshotLimitExceeded { limit: 0 })
        ));
        assert_eq!(fixture.driver.resync_required(), Some(1));
        assert_eq!(fixture.runtime.live_object_count(), Ok(2));
    }

    #[tokio::test]
    async fn discovery_initializes_default_context_page_and_complete_frame_tree() {
        // Arrange
        let mut fixture = discovery_fixture(8);
        fixture.factory.set_frame_tree(
            "page-a",
            frame_tree(serde_json::json!({
                "frame": frame_value("page-a", None),
                "childFrames": [{
                    "frame": frame_value("child-a", Some("page-a")),
                    "childFrames": [{"frame": frame_value("grandchild-a", Some("child-a"))}]
                }]
            })),
        );
        fixture
            .sender
            .send(attached_page_event("page-a", "session-a", None, None))
            .expect("Page attachment should enqueue");

        // Act
        let outcome = fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("discovered Page should initialize");

        // Assert
        let ChromiumTopologyEventDriverOutcome::Updated(update) = outcome else {
            panic!("discovery must publish topology updates");
        };
        assert!(update.effects().iter().any(|effect| matches!(
            effect,
            pwright_backend_cdp::CdpTopologyEffect::PageReady {
                page,
                context
            } if *page == ObjectId::new(3) && *context == ObjectId::new(2)
        )));
        let topology = fixture
            .topology
            .lock()
            .expect("topology should remain available");
        let default_context = topology
            .topology()
            .default_context_route()
            .expect("default context should exist");
        assert_eq!(default_context.browser_context_id(), "");
        assert_eq!(default_context.object_id(), ObjectId::new(2));
        assert_eq!(
            topology.topology().page_state("page-a"),
            Some(&pwright_backend_cdp::CdpPageState::Ready)
        );
        assert_eq!(
            topology.topology().frame_object_id("child-a"),
            Some(ObjectId::new(5))
        );
        assert_eq!(
            topology.topology().frame_object_id("grandchild-a"),
            Some(ObjectId::new(6))
        );
        drop(topology);
        assert_eq!(
            fixture.browser.method_names(),
            [
                "Target.setAutoAttach",
                "Target.getTargetInfo",
                "Target.getBrowserContexts"
            ]
        );
        assert_eq!(
            fixture.factory.client("page-a").method_names(),
            [
                "Page.enable",
                "Page.getFrameTree",
                "Runtime.enable",
                "Target.setAutoAttach",
                "Runtime.runIfWaitingForDebugger"
            ]
        );
        for object_id in 2..=6 {
            assert_eq!(
                fixture.runtime.is_object_live(ObjectId::new(object_id)),
                Ok(true)
            );
        }
    }

    #[tokio::test]
    async fn discovery_enables_inherited_context_network_before_page_publication() {
        // Arrange
        let mut fixture = network_discovery_fixture(8);
        let activator = ChromiumPageSubscriptionActivator::new(fixture.runtime.service().clone());
        activator
            .set_subscription(ObjectId::new(2), NETWORK_REQUEST_EVENT_TAG, true)
            .await
            .expect("existing Context listener should activate while it is empty");
        fixture.factory.set_frame_tree(
            "page-a",
            frame_tree(serde_json::json!({
                "frame": frame_value("page-a", None)
            })),
        );
        fixture
            .sender
            .send(attached_page_event(
                "page-a",
                "session-a",
                Some("context-a"),
                None,
            ))
            .expect("Page attachment should enqueue");

        // Act
        let outcome = fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("discovered Page should join inherited Context network state");

        // Assert
        let ChromiumTopologyEventDriverOutcome::Updated(update) = outcome else {
            panic!("network-aware discovery must publish the Page");
        };
        assert!(update.effects().iter().any(|effect| matches!(
            effect,
            pwright_backend_cdp::CdpTopologyEffect::PageReady { page, context }
                if *page == ObjectId::new(4) && *context == ObjectId::new(2)
        )));
        assert_eq!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .page_object_id("page-a"),
            Some(ObjectId::new(4))
        );
        let network_driver = fixture
            .network_driver
            .as_ref()
            .expect("network cursor owner should remain installed");
        assert_eq!(network_driver.page_count(), 1);
        assert_eq!(network_driver.page_registration_in_progress(), None);
        assert_eq!(
            fixture.factory.client("page-a").method_names(),
            [
                "Page.enable",
                "Page.getFrameTree",
                "Runtime.enable",
                "Network.enable",
                "Target.setAutoAttach",
                "Runtime.runIfWaitingForDebugger"
            ]
        );
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(4)), Ok(true));
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(5)), Ok(true));
    }

    #[tokio::test]
    async fn root_auto_attach_drops_duplicate_of_synchronized_explicit_page_session() {
        // Arrange
        let mut fixture = discovery_fixture(8);
        fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 0)
            .await
            .expect("the default Context should initialize");
        fixture.factory.set_frame_tree(
            "page-a",
            frame_tree(serde_json::json!({
                "frame": frame_value("page-a", None)
            })),
        );
        let browser_context_id = fixture
            .topology
            .lock()
            .unwrap()
            .topology()
            .default_context_route()
            .expect("the default Context should be published")
            .browser_context_id()
            .to_owned();
        let published_context_id = browser_context_id.clone();
        fixture
            .driver
            .transact_update(&fixture.runtime, move |topology| {
                let update = topology.attach_page(
                    ConnectionEpoch::new(1),
                    CdpPageTarget::new("page-a", "session-explicit", published_context_id),
                    CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
                )?;
                Ok(update.combine(topology.attach_frame(
                    ConnectionEpoch::new(1),
                    CdpFrameAttachment::main("page-a", "page-a"),
                    ObjectId::new(4),
                )?))
            })
            .expect("explicit Page publication should commit");
        fixture
            .sender
            .send(attached_page_event(
                "page-a",
                "session-auto",
                Some(&browser_context_id),
                None,
            ))
            .expect("the redundant root attachment should enqueue");

        // Act
        let outcome = fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("the redundant root attachment should be detached");

        // Assert
        assert!(matches!(
            outcome,
            ChromiumTopologyEventDriverOutcome::Updated(_)
        ));
        let topology = fixture.topology.lock().unwrap();
        let route = topology
            .topology()
            .page_session_routes()
            .find(|route| route.target_id() == "page-a")
            .expect("the explicit Page route should remain live");
        assert_eq!(route.session_id(), "session-explicit");
        drop(topology);
        assert_eq!(fixture.driver.initialized_session_count(), 1);
        assert_eq!(
            fixture.browser.calls().last(),
            Some(&(
                "Target.detachFromTarget".to_owned(),
                vec![serde_json::Value::String("session-auto".to_owned())]
            ))
        );
        assert_eq!(
            fixture.factory.client("page-a").method_names(),
            [
                "Page.enable",
                "Page.getFrameTree",
                "Runtime.enable",
                "Target.setAutoAttach",
                "Runtime.runIfWaitingForDebugger"
            ]
        );
    }

    #[tokio::test]
    async fn auxiliary_parent_page_attachment_does_not_enter_topology_discovery() {
        // Arrange
        let mut fixture = discovery_fixture(8);
        let parent = Arc::new(ChromiumAuxiliarySessionParent::new(64).unwrap());
        fixture.driver = fixture
            .driver
            .with_auxiliary_session_parent(Arc::clone(&parent))
            .unwrap();
        fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 0)
            .await
            .unwrap();
        parent.register("auxiliary-browser-root").unwrap();
        let attachment = attached_page_event("page-a", "auxiliary-page", None, None);
        fixture
            .sender
            .send(CdpEvent::new(
                attachment.method.clone(),
                attachment.params.clone(),
                Some("auxiliary-browser-root".to_owned()),
            ))
            .unwrap();
        let browser_calls_before = fixture.browser.calls();

        // Act
        let outcome = fixture.driver.receive_once(&fixture.runtime).await.unwrap();

        // Assert
        assert_eq!(outcome, ChromiumTopologyEventDriverOutcome::Ignored);
        assert_eq!(fixture.browser.calls(), browser_calls_before);
        assert_eq!(fixture.driver.initialized_session_count(), 0);
        assert_eq!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .page_session_routes()
                .count(),
            0
        );
    }

    #[tokio::test]
    async fn public_browser_root_attachment_is_not_detached_by_topology_discovery() {
        // Arrange
        let mut fixture = discovery_fixture(8);
        let parent = Arc::new(ChromiumAuxiliarySessionParent::new(64).unwrap());
        fixture.driver = fixture
            .driver
            .with_auxiliary_session_parent(Arc::clone(&parent))
            .unwrap();
        fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 0)
            .await
            .unwrap();
        parent
            .register_public_browser_session("public-browser-root")
            .unwrap();
        let attachment = attached_page_event("page-a", "public-page-child", None, None);
        fixture
            .sender
            .send(CdpEvent::new(
                attachment.method.clone(),
                attachment.params.clone(),
                Some("public-browser-root".to_owned()),
            ))
            .unwrap();
        let browser_calls_before = fixture.browser.calls();

        // Act
        let outcome = fixture.driver.receive_once(&fixture.runtime).await.unwrap();

        // Assert
        assert_eq!(outcome, ChromiumTopologyEventDriverOutcome::Ignored);
        assert_eq!(fixture.browser.calls(), browser_calls_before);
        assert_eq!(fixture.driver.initialized_session_count(), 0);
    }

    #[tokio::test]
    async fn browser_target_root_attachment_before_public_registration_is_not_detached() {
        // Arrange
        let mut fixture = discovery_fixture(8);
        let parent = Arc::new(ChromiumAuxiliarySessionParent::new(64).unwrap());
        fixture.driver = fixture
            .driver
            .with_auxiliary_session_parent(parent)
            .unwrap();
        fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 0)
            .await
            .unwrap();
        fixture
            .sender
            .send(CdpEvent::new(
                "Target.attachedToTarget".to_owned(),
                serde_json::json!({
                    "sessionId": "public-browser-root",
                    "targetInfo": {
                        "targetId": "browser",
                        "type": "browser",
                        "title": "",
                        "url": "",
                        "attached": true,
                        "canAccessOpener": false
                    },
                    "waitingForDebugger": false
                }),
                None,
            ))
            .unwrap();
        let browser_calls_before = fixture.browser.calls();

        // Act
        let outcome = fixture.driver.receive_once(&fixture.runtime).await.unwrap();

        // Assert
        assert_eq!(outcome, ChromiumTopologyEventDriverOutcome::Ignored);
        assert_eq!(fixture.browser.calls(), browser_calls_before);
        assert_eq!(fixture.driver.initialized_session_count(), 0);
    }

    #[test]
    fn auxiliary_parent_rejects_a_second_live_parent() {
        // Arrange
        let parent = ChromiumAuxiliarySessionParent::new(16).unwrap();
        parent.register("first-parent").unwrap();

        // Act
        let result = parent.register("second-parent");

        // Assert
        assert_eq!(
            result,
            Err(ChromiumAuxiliarySessionParentError::AlreadyRegistered)
        );
    }

    #[test]
    fn auxiliary_parent_bounds_native_id_before_retention() {
        // Arrange
        let parent = ChromiumAuxiliarySessionParent::new(4).unwrap();

        // Act
        let result = parent.register("12345");

        // Assert
        assert_eq!(
            result,
            Err(ChromiumAuxiliarySessionParentError::SessionIdTooLong {
                actual: 5,
                limit: 4
            })
        );
    }

    #[test]
    fn auxiliary_parent_clear_retires_only_the_exact_owner() {
        // Arrange
        let parent = ChromiumAuxiliarySessionParent::new(16).unwrap();
        parent.register("first-parent").unwrap();

        // Act
        let cleared = parent.clear("other-parent");

        // Assert
        assert_eq!(cleared, Ok(false));
        assert_eq!(parent.matches("first-parent"), Ok(true));
    }

    #[test]
    fn public_browser_roots_are_bounded_and_cleared_by_exact_id() {
        // Arrange
        let parent = ChromiumAuxiliarySessionParent::new(64).unwrap();
        parent.register("private-page-parent").unwrap();
        for index in 0..super::MAX_PUBLIC_BROWSER_SESSIONS {
            parent
                .register_public_browser_session(&format!("public-{index}"))
                .unwrap();
        }

        // Act
        let overflow = parent.register_public_browser_session("ninth");
        let unrelated_clear = parent.clear_public_browser_session("other");
        let exact_clear = parent.clear_public_browser_session("public-3");

        // Assert
        assert_eq!(
            overflow,
            Err(ChromiumAuxiliarySessionParentError::PublicSessionCapacityExceeded)
        );
        assert_eq!(unrelated_clear, Ok(false));
        assert_eq!(exact_clear, Ok(true));
        assert_eq!(parent.matches("public-3"), Ok(false));
        assert_eq!(parent.matches("private-page-parent"), Ok(true));
        assert_eq!(
            parent.register_public_browser_session("replacement"),
            Ok(())
        );
    }

    #[test]
    fn duplicate_public_browser_root_is_rejected_without_retiring_existing_owner() {
        // Arrange
        let parent = ChromiumAuxiliarySessionParent::new(64).unwrap();
        parent
            .register_public_browser_session("public-root")
            .unwrap();

        // Act
        let duplicate = parent.register_public_browser_session("public-root");

        // Assert
        assert_eq!(
            duplicate,
            Err(ChromiumAuxiliarySessionParentError::PublicSessionAlreadyRegistered)
        );
        assert_eq!(parent.matches("public-root"), Ok(true));
    }

    #[test]
    fn public_root_cannot_reuse_private_parent_id() {
        // Arrange
        let parent = ChromiumAuxiliarySessionParent::new(64).unwrap();
        parent.register("private-root").unwrap();

        // Act
        let collision = parent.register_public_browser_session("private-root");

        // Assert
        assert_eq!(
            collision,
            Err(ChromiumAuxiliarySessionParentError::PublicSessionAlreadyRegistered)
        );
        assert_eq!(parent.matches("private-root"), Ok(true));
    }

    #[test]
    fn private_parent_cannot_reuse_public_root_id() {
        // Arrange
        let parent = ChromiumAuxiliarySessionParent::new(64).unwrap();
        parent
            .register_public_browser_session("public-root")
            .unwrap();

        // Act
        let collision = parent.register("public-root");

        // Assert
        assert_eq!(
            collision,
            Err(ChromiumAuxiliarySessionParentError::AlreadyRegistered)
        );
        assert_eq!(parent.matches("public-root"), Ok(true));
    }

    #[tokio::test]
    async fn root_auto_attach_replay_of_exact_explicit_session_does_not_reinitialize_or_detach() {
        // Arrange
        let mut fixture = discovery_fixture(8);
        fixture
            .driver
            .initialize_snapshot(&fixture.runtime, 0)
            .await
            .expect("the default Context should initialize");
        fixture.factory.set_frame_tree(
            "page-a",
            frame_tree(serde_json::json!({
                "frame": frame_value("page-a", None)
            })),
        );
        let initializer = Arc::new(RecordingInitializer {
            factory: fixture.factory.clone(),
            calls: Mutex::new(Vec::new()),
            fail: false,
        });
        fixture.driver = fixture.driver.with_session_initializer(initializer.clone());
        let browser_context_id = fixture
            .topology
            .lock()
            .unwrap()
            .topology()
            .default_context_route()
            .expect("the default Context should be published")
            .browser_context_id()
            .to_owned();
        let published_context_id = browser_context_id.clone();
        fixture
            .driver
            .transact_update(&fixture.runtime, move |topology| {
                let update = topology.attach_page(
                    ConnectionEpoch::new(1),
                    CdpPageTarget::new("page-a", "session-explicit", published_context_id),
                    CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
                )?;
                Ok(update.combine(topology.attach_frame(
                    ConnectionEpoch::new(1),
                    CdpFrameAttachment::main("page-a", "page-a"),
                    ObjectId::new(4),
                )?))
            })
            .expect("explicit Page publication should commit");
        fixture
            .sender
            .send(attached_page_event(
                "page-a",
                "session-explicit",
                Some(&browser_context_id),
                None,
            ))
            .expect("the exact session replay should enqueue");

        // Act
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("the exact session replay should be ignored");

        // Assert
        assert_eq!(
            *initializer.calls.lock().unwrap(),
            [("page-a".to_owned(), CdpContextOwnership::Default, true)]
        );
        assert_eq!(fixture.driver.initialized_session_count(), 1);
        assert_eq!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .page_session_routes()
                .find(|route| route.target_id() == "page-a")
                .expect("the explicit Page route should remain live")
                .session_id(),
            "session-explicit"
        );
        assert!(
            !fixture
                .browser
                .method_names()
                .iter()
                .any(|method| method == "Target.detachFromTarget")
        );
        assert_eq!(
            fixture.factory.client("page-a").method_names(),
            [
                "Page.enable",
                "Page.getFrameTree",
                "Runtime.enable",
                "Target.setAutoAttach",
                "Runtime.runIfWaitingForDebugger"
            ]
        );
    }

    #[cfg(feature = "request-frame")]
    #[tokio::test]
    async fn network_request_frame_can_be_observed_before_topology_attaches_it() {
        // Arrange: both cursors receive the same ordered events, but only the
        // network cursor will consume the child attachment before the Request.
        let mut fixture = network_frame_cursor_fixture().await;
        fixture
            .sender
            .send(CdpEvent::new(
                "Page.frameAttached".to_owned(),
                serde_json::json!({"frameId":"child-a","parentFrameId":"page-a"}),
                Some("session-a".to_owned()),
            ))
            .expect("child attachment should enqueue");
        fixture
            .sender
            .send(CdpEvent::new(
                "Network.requestWillBeSent".to_owned(),
                serde_json::json!({
                    "requestId": "native-child-request",
                    "frameId": "child-a",
                    "type": "Fetch",
                    "request": {
                        "url": "https://example.test/child",
                        "method": "GET",
                        "headers": {}
                    }
                }),
                Some("session-a".to_owned()),
            ))
            .expect("child Request should enqueue");

        // Act: advance the independent network cursor only.
        let network_driver = fixture.network_driver.as_mut().unwrap();
        assert!(
            network_driver
                .receive_once()
                .await
                .expect("attachment should be network-inert")
                .is_none()
        );
        let batch = network_driver
            .receive_once()
            .await
            .expect("Request should normalize")
            .expect("Request should publish a batch");

        // Assert: available describes network-cursor observation, not the
        // current topology's ability to resolve the same native Frame.
        let [pwright_backend_cdp::CdpNetworkEventEffect::Request(event)] = batch.events() else {
            panic!("the observed effect should be a Request");
        };
        let frame = batch
            .request_frame(event.payload().request)
            .expect("Request should retain a Frame descriptor");
        assert_eq!(frame.native_id(), Some("child-a"));
        assert!(frame.available());
        assert_eq!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .frame_object_id("child-a"),
            None
        );
    }

    #[cfg(feature = "request-frame")]
    #[tokio::test]
    async fn topology_can_remove_frame_before_network_publishes_its_request_and_terminal() {
        // Arrange: topology consumes the attachment and removal while the
        // independent network cursor remains behind the Request.
        let mut fixture = network_frame_cursor_fixture().await;
        for event in [
            CdpEvent::new(
                "Page.frameAttached".to_owned(),
                serde_json::json!({"frameId":"child-b","parentFrameId":"page-a"}),
                Some("session-a".to_owned()),
            ),
            CdpEvent::new(
                "Network.requestWillBeSent".to_owned(),
                serde_json::json!({
                    "requestId": "native-retired-request",
                    "frameId": "child-b",
                    "type": "Fetch",
                    "request": {
                        "url": "https://example.test/retired",
                        "method": "GET",
                        "headers": {}
                    }
                }),
                Some("session-a".to_owned()),
            ),
            CdpEvent::new(
                "Page.frameDetached".to_owned(),
                serde_json::json!({"frameId":"child-b","reason":"remove"}),
                Some("session-a".to_owned()),
            ),
            CdpEvent::new(
                "Network.loadingFailed".to_owned(),
                serde_json::json!({
                    "requestId": "native-retired-request",
                    "errorText": "net::ERR_ABORTED",
                    "canceled": true
                }),
                Some("session-a".to_owned()),
            ),
        ] {
            fixture
                .sender
                .send(event)
                .expect("ordered event should enqueue");
        }
        for _ in 0..2 {
            fixture
                .driver
                .receive_once(&fixture.runtime)
                .await
                .expect("attachment and Request should advance topology");
        }
        assert!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .frame_object_id("child-b")
                .is_some(),
            "the child must exist before its removal"
        );

        // Act: retire the Frame in topology, then publish the older Request
        // and its terminal failure through the network cursor.
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("Frame removal should advance topology");
        let network_driver = fixture.network_driver.as_mut().unwrap();
        assert!(
            network_driver
                .receive_once()
                .await
                .expect("attachment should be network-inert")
                .is_none()
        );
        let request = network_driver
            .receive_once()
            .await
            .expect("Request should normalize")
            .expect("Request should publish a batch");
        let [pwright_backend_cdp::CdpNetworkEventEffect::Request(request_event)] = request.events()
        else {
            panic!("the first effect should be a Request");
        };
        let request_id = request_event.payload().request;
        network_driver
            .acknowledge_batch_publication(request.page().unwrap())
            .expect("Request publication should release the network cursor");
        assert!(
            network_driver
                .receive_once()
                .await
                .expect("detachment should be network-inert")
                .is_none()
        );
        let terminal = network_driver
            .receive_once()
            .await
            .expect("terminal should normalize")
            .expect("terminal should publish a batch");

        // Assert: both semantic effects retain the exact observed native Frame
        // even though the live topology has already removed it.
        let [pwright_backend_cdp::CdpNetworkEventEffect::RequestFailed(event)] = terminal.events()
        else {
            panic!("the terminal effect should be RequestFailed");
        };
        assert_eq!(event.payload().request.request, request_id);
        for batch in [&request, &terminal] {
            let frame = batch
                .request_frame(request_id)
                .expect("each effect should retain the Request Frame descriptor");
            assert_eq!(frame.native_id(), Some("child-b"));
            assert!(frame.available());
        }
        assert_eq!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .frame_object_id("child-b"),
            None
        );
    }

    #[tokio::test]
    async fn network_aware_discovery_atomically_replaces_page_session_and_active_requests() {
        // Arrange
        let mut fixture = network_discovery_fixture(8);
        ChromiumPageSubscriptionActivator::new(fixture.runtime.service().clone())
            .set_subscription(ObjectId::new(2), NETWORK_REQUEST_EVENT_TAG, true)
            .await
            .expect("Context network interest should activate");
        fixture.factory.set_frame_tree(
            "page-a",
            frame_tree(serde_json::json!({
                "frame": frame_value("page-a", None)
            })),
        );
        fixture
            .sender
            .send(attached_page_event(
                "page-a",
                "session-a",
                Some("context-a"),
                None,
            ))
            .expect("initial Page attachment should enqueue");
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("initial Page should materialize its network lane");
        fixture
            .sender
            .send(CdpEvent::new(
                "Network.requestWillBeSent".to_owned(),
                serde_json::json!({
                    "requestId": "native-old",
                    "type": "Fetch",
                    "request": {
                        "url": "https://example.test/old",
                        "method": "GET",
                        "headers": {}
                    }
                }),
                Some("session-a".to_owned()),
            ))
            .expect("old-session Request should enqueue");
        let network_driver = fixture.network_driver.as_mut().unwrap();
        assert!(
            network_driver
                .receive_once()
                .await
                .expect("attachment event should be harmless")
                .is_none()
        );
        let request = network_driver
            .receive_once()
            .await
            .expect("old Request should normalize")
            .expect("old Request should create a runtime child");
        fixture
            .runtime
            .apply_lifecycle_batch(request.lifecycle())
            .expect("old Request should register");
        network_driver
            .acknowledge_batch_publication(
                request
                    .page()
                    .expect("normalized Request batch has one Page"),
            )
            .expect("Request publication should release the driver barrier");
        let request_id = match request.lifecycle() {
            [pwright_core::ControlMessage::Create { object_id, .. }] => *object_id,
            lifecycle => panic!("unexpected Request lifecycle: {lifecycle:?}"),
        };
        fixture
            .sender
            .send(attached_page_event(
                "page-a",
                "session-b",
                Some("context-a"),
                None,
            ))
            .expect("replacement Page attachment should enqueue");
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("network event should remain topology-inert");

        // Act
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("replacement should publish atomically");

        // Assert
        let topology = fixture.topology.lock().unwrap();
        let route = topology
            .topology()
            .page_session_routes()
            .find(|route| route.object_id() == ObjectId::new(4))
            .expect("the Page route should remain live");
        assert_eq!(route.session_id(), "session-b");
        drop(topology);
        let network_driver = fixture.network_driver.as_ref().unwrap();
        assert_eq!(network_driver.page_count(), 1);
        assert_eq!(network_driver.page_registration_in_progress(), None);
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(4)), Ok(true));
        assert_eq!(fixture.runtime.is_object_live(request_id), Ok(false));
        let session_methods = fixture.factory.client("page-a").method_names();
        assert_eq!(
            session_methods,
            [
                "Page.enable",
                "Page.getFrameTree",
                "Runtime.enable",
                "Network.enable",
                "Target.setAutoAttach",
                "Runtime.runIfWaitingForDebugger",
                "Page.enable",
                "Page.getFrameTree",
                "Runtime.enable",
                "Network.enable",
                "Target.setAutoAttach",
                "Runtime.runIfWaitingForDebugger"
            ]
        );
        assert!(
            !fixture
                .browser
                .method_names()
                .contains(&"Target.detachFromTarget".to_owned())
        );
    }

    #[tokio::test]
    async fn failed_page_session_replacement_restores_old_lane_and_disables_new_session() {
        // Arrange
        let mut fixture = network_discovery_fixture(8);
        ChromiumPageSubscriptionActivator::new(fixture.runtime.service().clone())
            .set_subscription(ObjectId::new(2), NETWORK_REQUEST_EVENT_TAG, true)
            .await
            .expect("Context network interest should activate");
        fixture.factory.set_frame_tree(
            "page-a",
            frame_tree(serde_json::json!({
                "frame": frame_value("page-a", None)
            })),
        );
        fixture
            .sender
            .send(attached_page_event(
                "page-a",
                "session-a",
                Some("context-a"),
                None,
            ))
            .expect("initial Page attachment should enqueue");
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("initial Page should materialize");
        fixture
            .sender
            .send(attached_page_event(
                "page-a",
                "session-b",
                Some("context-a"),
                Some("unexpected-opener"),
            ))
            .expect("conflicting replacement should enqueue");

        // Act
        let error = fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect_err("conflicting replacement publication must roll back");

        // Assert
        assert!(matches!(
            error,
            ChromiumTopologyEventDriverError::Composition(_)
        ));
        let topology = fixture.topology.lock().unwrap();
        let route = topology
            .topology()
            .page_session_routes()
            .find(|route| route.object_id() == ObjectId::new(4))
            .expect("old Page route should survive");
        assert_eq!(route.session_id(), "session-a");
        drop(topology);
        let network_driver = fixture.network_driver.as_ref().unwrap();
        assert_eq!(network_driver.page_count(), 1);
        assert_eq!(network_driver.page_registration_in_progress(), None);
        let registration = network_driver.registration_handle();
        assert!(
            registration
                .validate_page_session(ObjectId::new(4), "session-a")
                .is_ok()
        );
        assert!(
            registration
                .validate_page_session(ObjectId::new(4), "session-b")
                .is_err()
        );
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(4)), Ok(true));
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(5)), Ok(true));
        let methods = fixture.factory.client("page-a").method_names();
        assert_eq!(
            methods
                .iter()
                .filter(|method| *method == "Network.enable")
                .count(),
            2
        );
        assert_eq!(methods.last().map(String::as_str), Some("Network.disable"));
        assert_eq!(
            fixture.browser.calls().last(),
            Some(&(
                "Target.detachFromTarget".to_owned(),
                vec![serde_json::Value::String("session-b".to_owned())]
            ))
        );
    }

    #[tokio::test]
    async fn cancelled_page_session_replacement_is_explicitly_recoverable() {
        // Arrange
        let mut fixture = network_discovery_fixture(8);
        ChromiumPageSubscriptionActivator::new(fixture.runtime.service().clone())
            .set_subscription(ObjectId::new(2), NETWORK_REQUEST_EVENT_TAG, true)
            .await
            .expect("Context network interest should activate");
        fixture.factory.set_frame_tree(
            "page-a",
            frame_tree(serde_json::json!({
                "frame": frame_value("page-a", None)
            })),
        );
        fixture
            .sender
            .send(attached_page_event(
                "page-a",
                "session-a",
                Some("context-a"),
                None,
            ))
            .expect("initial Page attachment should enqueue");
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("initial Page should materialize");
        let network_driver = fixture.network_driver.as_ref().unwrap();
        let registration = network_driver.registration_handle();
        let client = fixture
            .factory
            .create_session("session-b".to_owned(), "page-a".to_owned());
        let gate = Arc::new(tokio::sync::Notify::new());
        fixture
            .factory
            .client("page-a")
            .set_network_gate(Some(Arc::clone(&gate)));
        let mut replacement = Box::pin(registration.prepare_page_replacement(
            ObjectId::new(4),
            ObjectId::new(2),
            "session-b".to_owned(),
            Vec::new(),
            client,
            false,
        ));

        // Act
        assert!(matches!(
            replacement
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        ));
        drop(replacement);
        let pending = registration
            .page_registration_in_progress()
            .expect("replacement state should remain readable");
        fixture.factory.client("page-a").set_network_gate(None);
        let recovered = registration
            .recover_pending_page_registration()
            .await
            .expect("cancelled replacement should compensate");

        // Assert
        assert_eq!(pending, Some(ObjectId::new(4)));
        assert_eq!(recovered, Some(Vec::new()));
        assert_eq!(network_driver.page_count(), 1);
        assert_eq!(network_driver.page_registration_in_progress(), None);
        assert!(
            registration
                .validate_page_session(ObjectId::new(4), "session-a")
                .is_ok()
        );
        assert_eq!(
            fixture
                .factory
                .client("page-a")
                .method_names()
                .iter()
                .rev()
                .take(2)
                .cloned()
                .collect::<Vec<_>>(),
            ["Network.disable", "Network.enable"]
        );
    }

    #[tokio::test]
    async fn page_session_detach_atomically_retires_network_lane_and_runtime_subtree() {
        // Arrange
        let mut fixture = network_discovery_fixture(8);
        fixture.factory.set_frame_tree(
            "page-a",
            frame_tree(serde_json::json!({
                "frame": frame_value("page-a", None)
            })),
        );
        fixture
            .sender
            .send(attached_page_event(
                "page-a",
                "session-a",
                Some("context-a"),
                None,
            ))
            .expect("Page attachment should enqueue");
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("Page should materialize");
        fixture
            .sender
            .send(CdpEvent::new(
                "Network.requestWillBeSent".to_owned(),
                serde_json::json!({
                    "requestId": "native-a",
                    "type": "Fetch",
                    "request": {
                        "url": "https://example.test/a",
                        "method": "GET",
                        "headers": {}
                    }
                }),
                Some("session-a".to_owned()),
            ))
            .expect("network request should enqueue");
        let network_driver = fixture.network_driver.as_mut().unwrap();
        assert!(
            network_driver
                .receive_once()
                .await
                .expect("attachment event should be harmless")
                .is_none()
        );
        let request = network_driver
            .receive_once()
            .await
            .expect("request should normalize")
            .expect("request should produce a batch");
        fixture
            .runtime
            .apply_lifecycle_batch(request.lifecycle())
            .expect("Request child should register");
        network_driver
            .acknowledge_batch_publication(
                request
                    .page()
                    .expect("normalized Request batch has one Page"),
            )
            .expect("Request publication should release the driver barrier");
        let request_id = match request.lifecycle() {
            [pwright_core::ControlMessage::Create { object_id, .. }] => *object_id,
            lifecycle => panic!("unexpected Request lifecycle: {lifecycle:?}"),
        };
        fixture
            .sender
            .send(CdpEvent::new(
                "Target.detachedFromTarget".to_owned(),
                serde_json::json!({
                    "sessionId": "session-a",
                    "targetId": "page-a"
                }),
                None,
            ))
            .expect("Page detach should enqueue");
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("unrelated network event should be ignored by topology");

        // Act
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("Page detach should retire every owned view");

        // Assert
        assert_eq!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .page_object_id("page-a"),
            None
        );
        let network_driver = fixture.network_driver.as_ref().unwrap();
        assert_eq!(network_driver.page_count(), 0);
        assert_eq!(network_driver.page_registration_in_progress(), None);
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(4)), Ok(false));
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(5)), Ok(false));
        assert_eq!(fixture.runtime.is_object_live(request_id), Ok(false));
    }

    #[tokio::test]
    async fn failed_page_detach_preserves_network_lane_and_runtime_subtree() {
        // Arrange
        let mut fixture = network_discovery_fixture(8);
        fixture.factory.set_frame_tree(
            "page-a",
            frame_tree(serde_json::json!({
                "frame": frame_value("page-a", None)
            })),
        );
        fixture
            .sender
            .send(attached_page_event(
                "page-a",
                "session-a",
                Some("context-a"),
                None,
            ))
            .expect("Page attachment should enqueue");
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("Page should materialize");
        fixture
            .sender
            .send(CdpEvent::new(
                "Target.detachedFromTarget".to_owned(),
                serde_json::json!({
                    "sessionId": "session-a",
                    "targetId": "wrong-target"
                }),
                None,
            ))
            .expect("invalid detach should enqueue");

        // Act
        let error = fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect_err("a mismatched target must fail the retirement transaction");

        // Assert
        assert!(matches!(
            error,
            ChromiumTopologyEventDriverError::Composition(_)
        ));
        assert_eq!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .page_object_id("page-a"),
            Some(ObjectId::new(4))
        );
        let network_driver = fixture.network_driver.as_ref().unwrap();
        assert_eq!(network_driver.page_count(), 1);
        assert_eq!(network_driver.page_registration_in_progress(), None);
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(4)), Ok(true));
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(5)), Ok(true));
    }

    #[test]
    fn failed_network_aware_discovery_rolls_back_lane_and_detaches_session() {
        // Arrange
        let mut fixture = network_discovery_fixture(8);
        let activator = ChromiumPageSubscriptionActivator::new(fixture.runtime.service().clone());
        let mut activation =
            Box::pin(activator.set_subscription(ObjectId::new(2), NETWORK_REQUEST_EVENT_TAG, true));
        assert!(matches!(
            activation
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Ready(Ok(()))
        ));
        fixture.factory.set_frame_tree(
            "page-a",
            frame_tree(serde_json::json!({
                "frame": frame_value("page-a", None)
            })),
        );
        let _: Arc<dyn CdpClient> = fixture
            .factory
            .create_session("session-a".to_owned(), "page-a".to_owned());
        let client = fixture.factory.client("page-a");
        let gate = Arc::new(tokio::sync::Notify::new());
        client.set_network_gate(Some(Arc::clone(&gate)));
        fixture
            .sender
            .send(attached_page_event(
                "page-a",
                "session-a",
                Some("context-a"),
                None,
            ))
            .expect("Page attachment should enqueue");
        let mut discovery = Box::pin(fixture.driver.receive_once(&fixture.runtime));
        assert!(matches!(
            discovery
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        ));
        fixture
            .topology
            .lock()
            .unwrap()
            .close_context(ConnectionEpoch::new(1), "context-a")
            .expect("Context removal should invalidate discovery publication");

        // Act
        client.set_network_gate(None);
        gate.notify_one();
        let Poll::Ready(Err(error)) = discovery
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        else {
            panic!("resumed discovery should finish rollback");
        };
        drop(discovery);

        // Assert
        assert!(matches!(
            error,
            ChromiumTopologyEventDriverError::Composition(_)
        ));
        assert_eq!(
            client.method_names(),
            [
                "Page.enable",
                "Page.getFrameTree",
                "Runtime.enable",
                "Network.enable",
                "Target.setAutoAttach",
                "Runtime.runIfWaitingForDebugger",
                "Network.disable"
            ]
        );
        assert!(
            fixture
                .browser
                .method_names()
                .iter()
                .any(|method| method == "Target.detachFromTarget")
        );
        let network_driver = fixture.network_driver.as_ref().unwrap();
        assert_eq!(network_driver.page_count(), 0);
        assert_eq!(network_driver.page_registration_in_progress(), None);
        assert_eq!(
            fixture
                .topology
                .lock()
                .unwrap()
                .topology()
                .page_object_id("page-a"),
            None
        );
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(4)), Ok(false));
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(5)), Ok(false));
    }

    #[tokio::test]
    async fn discovered_popup_is_linked_only_after_its_initialization_completes() {
        // Arrange
        let mut fixture = discovery_fixture(8);
        fixture
            .sender
            .send(attached_page_event("opener", "session-opener", None, None))
            .expect("opener should enqueue");
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("opener should become ready");
        fixture
            .sender
            .send(attached_page_event(
                "popup",
                "session-popup",
                None,
                Some("opener"),
            ))
            .expect("popup should enqueue");

        // Act
        let outcome = fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("popup should initialize");

        // Assert
        let ChromiumTopologyEventDriverOutcome::Updated(update) = outcome else {
            panic!("popup should publish ready and linkage effects");
        };
        let semantic = update
            .effects()
            .iter()
            .filter(|effect| {
                matches!(
                    effect,
                    pwright_backend_cdp::CdpTopologyEffect::PageReady { .. }
                        | pwright_backend_cdp::CdpTopologyEffect::PopupLinked { .. }
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            semantic,
            [
                pwright_backend_cdp::CdpTopologyEffect::PageReady {
                    page: ObjectId::new(5),
                    context: ObjectId::new(2),
                },
                pwright_backend_cdp::CdpTopologyEffect::PopupLinked {
                    opener: ObjectId::new(3),
                    popup: ObjectId::new(5),
                }
            ]
        );
    }

    #[tokio::test]
    async fn discovery_detaches_page_from_unknown_nondefault_context() {
        // Arrange
        let mut fixture = discovery_fixture(8);
        fixture
            .browser
            .set_browser_contexts_response(vec!["foreign-context".to_owned()]);
        fixture
            .sender
            .send(attached_page_event(
                "foreign-page",
                "foreign-session",
                Some("foreign-context"),
                None,
            ))
            .expect("foreign Page should enqueue");

        // Act
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("foreign Page should be safely detached");

        // Assert
        assert_eq!(
            fixture.browser.method_names(),
            [
                "Target.setAutoAttach",
                "Target.getTargetInfo",
                "Target.getBrowserContexts",
                "Target.detachFromTarget"
            ]
        );
        assert_eq!(
            fixture
                .topology
                .lock()
                .expect("topology should remain available")
                .topology()
                .page_object_id("foreign-page"),
            None
        );
    }

    #[tokio::test]
    async fn playwright_context_fallback_attributes_foreign_page_without_owning_context() {
        // Arrange
        let mut fixture = discovery_fixture(8);
        fixture.driver = fixture.driver.with_playwright_context_fallback();
        fixture
            .browser
            .set_browser_contexts_response(vec!["foreign-context".to_owned()]);
        fixture.factory.set_frame_tree(
            "foreign-page",
            frame_tree(serde_json::json!({
                "frame": frame_value("foreign-page", None)
            })),
        );
        fixture
            .sender
            .send(attached_page_event(
                "foreign-page",
                "foreign-session",
                Some("foreign-context"),
                None,
            ))
            .unwrap();
        // Act
        fixture.driver.receive_once(&fixture.runtime).await.unwrap();
        // Assert
        let topology = fixture.topology.lock().unwrap();
        let topology = topology.topology();
        assert!(topology.page_object_id("foreign-page").is_some());
        assert!(topology.context_object_id("foreign-context").is_none());
        let default = topology.default_context_route().unwrap().object_id();
        assert!(
            topology
                .page_session_routes()
                .any(|route| route.target_id() == "foreign-page"
                    && route.browser_context_object_id() == default)
        );
        assert!(
            !fixture
                .browser
                .method_names()
                .contains(&"Target.detachFromTarget".to_owned())
        );
    }

    #[tokio::test]
    async fn malformed_initial_frame_tree_never_marks_discovered_page_ready() {
        // Arrange
        let mut fixture = discovery_fixture(8);
        fixture.factory.set_frame_tree(
            "page-a",
            frame_tree(serde_json::json!({
                "frame": frame_value("page-a", None),
                "childFrames": [{"frame": frame_value("child-a", Some("wrong-parent"))}]
            })),
        );
        fixture
            .sender
            .send(attached_page_event("page-a", "session-a", None, None))
            .expect("Page attachment should enqueue");

        // Act
        let error = fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect_err("malformed frame tree must fail initialization");

        // Assert
        assert!(matches!(
            error,
            ChromiumTopologyEventDriverError::FrameTreeParentMismatch { .. }
        ));
        let topology = fixture
            .topology
            .lock()
            .expect("topology should remain available");
        assert_eq!(topology.topology().page_state("page-a"), None);
        assert_eq!(topology.topology().frame_object_id("child-a"), None);
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(3)), Ok(false));
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(4)), Ok(false));
        assert_eq!(fixture.driver.initialized_session_count(), 0);
    }

    #[tokio::test]
    async fn page_session_frame_tree_root_with_parent_remains_rejected() {
        // Arrange a malformed top-level Page tree. Only an OOPIF session root
        // may report its embedding Page frame as parent.
        let mut fixture = discovery_fixture(8);
        fixture.factory.set_frame_tree(
            "page-a",
            frame_tree(serde_json::json!({
                "frame": frame_value("page-a", Some("foreign-parent"))
            })),
        );
        fixture
            .sender
            .send(attached_page_event("page-a", "session-a", None, None))
            .expect("Page attachment should enqueue");

        // Act
        let error = fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect_err("Page-session root parent must remain invalid");

        // Assert
        assert!(matches!(
            error,
            ChromiumTopologyEventDriverError::FrameTreeRootHasParent { .. }
        ));
        assert_eq!(fixture.driver.initialized_session_count(), 0);
    }

    #[tokio::test]
    async fn local_frame_and_main_world_publish_atomically() {
        // Arrange
        let mut fixture = fixture(8);
        fixture
            .sender
            .send(CdpEvent::new(
                "Page.frameAttached".to_owned(),
                serde_json::json!({"frameId":"child-a","parentFrameId":"target-a"}),
                Some("session-a".to_owned()),
            ))
            .expect("frame event should enqueue");

        // Act
        let attached = fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("frame event should apply");
        fixture
            .sender
            .send(CdpEvent::new(
                "Runtime.executionContextCreated".to_owned(),
                serde_json::json!({
                    "context": {
                        "id": 17,
                        "origin": "https://child.example",
                        "name": "",
                        "uniqueId": "world-child-a",
                        "auxData": {"frameId":"child-a","isDefault":true}
                    }
                }),
                Some("session-a".to_owned()),
            ))
            .expect("world event should enqueue");
        let world = fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("world event should apply");

        // Assert
        assert!(matches!(
            attached,
            ChromiumTopologyEventDriverOutcome::Updated(_)
        ));
        assert!(matches!(
            world,
            ChromiumTopologyEventDriverOutcome::Updated(_)
        ));
        let topology = fixture
            .topology
            .lock()
            .expect("topology should be available");
        assert_eq!(
            topology.topology().frame_object_id("child-a"),
            Some(ObjectId::new(5))
        );
        assert_eq!(
            topology
                .topology()
                .frame_execution_routes()
                .find(|route| route.object_id() == ObjectId::new(5))
                .map(|route| (route.session_id(), route.unique_context_id())),
            Some(("session-a", "world-child-a"))
        );
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(5)), Ok(true));
        assert!(
            fixture
                .runtime
                .service()
                .evaluation_route(ObjectId::new(5))
                .expect("evaluation routes should be available")
                .is_some()
        );
        assert_eq!(
            fixture.factory.client("target-a").method_names(),
            [
                "Page.enable",
                "Page.getFrameTree",
                "Runtime.enable",
                "Target.setAutoAttach"
            ]
        );
    }

    #[tokio::test]
    async fn root_navigation_rebinds_main_frame_and_context_clear_removes_its_world() {
        // Arrange the original main world and a root-document replacement.
        let mut fixture = fixture(8);
        for event in [
            CdpEvent::new(
                "Runtime.executionContextCreated".to_owned(),
                serde_json::json!({"context":{
                    "id":31,"origin":"https://one.example","name":"",
                    "uniqueId":"main-old",
                    "auxData":{"frameId":"target-a","isDefault":true}
                }}),
                Some("session-a".to_owned()),
            ),
            CdpEvent::new(
                "Page.frameNavigated".to_owned(),
                serde_json::json!({
                    "frame":{
                        "id":"main-new","loaderId":"loader-new","url":"https://two.example",
                        "domainAndRegistry":"two.example","securityOrigin":"https://two.example",
                        "mimeType":"text/html","secureContextType":"Secure",
                        "crossOriginIsolatedContextType":"NotIsolated","gatedAPIFeatures":[]
                    },
                    "type":"Navigation"
                }),
                Some("session-a".to_owned()),
            ),
            CdpEvent::new(
                "Runtime.executionContextCreated".to_owned(),
                serde_json::json!({"context":{
                    "id":32,"origin":"https://two.example","name":"",
                    "uniqueId":"main-new-world",
                    "auxData":{"frameId":"main-new","isDefault":true}
                }}),
                Some("session-a".to_owned()),
            ),
            CdpEvent::new(
                "Runtime.executionContextsCleared".to_owned(),
                serde_json::json!({}),
                Some("session-a".to_owned()),
            ),
        ] {
            fixture
                .sender
                .send(event)
                .expect("navigation event should enqueue");
        }

        // Act
        for _ in 0..3 {
            fixture
                .driver
                .receive_once(&fixture.runtime)
                .await
                .expect("navigation event should apply");
        }
        let rebound_route = fixture
            .topology
            .lock()
            .expect("topology should be available")
            .topology()
            .frame_execution_routes()
            .find(|route| route.object_id() == ObjectId::new(4))
            .map(|route| route.unique_context_id().to_owned());
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("world clear should apply");

        // Assert
        let topology = fixture
            .topology
            .lock()
            .expect("topology should be available");
        assert_eq!(
            topology.topology().frame_object_id("main-new"),
            Some(ObjectId::new(4))
        );
        assert_eq!(rebound_route.as_deref(), Some("main-new-world"));
        assert_eq!(
            topology
                .topology()
                .frame_execution_routes()
                .find(|route| route.object_id() == ObjectId::new(4)),
            None
        );
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(4)), Ok(true));
    }

    struct WorkerRetentionObserver {
        network: bool,
    }

    impl pwright_plugin_chromium_frame_observer::ChromiumFrameEventObserver
        for WorkerRetentionObserver
    {
        fn observes_page(&self, _page: ObjectId) -> bool {
            false
        }

        fn retains_worker_targets(&self) -> bool {
            true
        }

        fn enables_worker_network(&self) -> bool {
            self.network
        }

        fn initialize_frame(
            &self,
            _topology: &pwright_backend_cdp::CdpTopology,
            _session: &str,
            _frame: &page::Frame,
        ) -> pwright_plugin_chromium_frame_observer::FrameObservationResult {
            Ok(())
        }

        fn reconcile(
            &self,
            _topology: &pwright_backend_cdp::CdpTopology,
        ) -> pwright_plugin_chromium_frame_observer::FrameObservationResult {
            Ok(())
        }

        fn event(
            &self,
            _topology: &pwright_backend_cdp::CdpTopology,
            _event: &CdpEvent,
        ) -> pwright_plugin_chromium_frame_observer::FrameObservationResult {
            Ok(())
        }

        fn seal(&self) {}
    }

    #[tokio::test]
    async fn selected_worker_observer_retains_and_initializes_page_child_session() {
        // Arrange
        let mut fixture = fixture(8);
        fixture.driver = fixture
            .driver
            .with_frame_event_observer(Arc::new(WorkerRetentionObserver { network: false }))
            .unwrap();
        fixture
            .sender
            .send(CdpEvent::new(
                "Target.attachedToTarget".to_owned(),
                serde_json::json!({
                    "sessionId": "session-worker",
                    "targetInfo": {
                        "targetId": "target-worker",
                        "type": "worker",
                        "title": "",
                        "url": "https://example.test/worker.js",
                        "attached": true,
                        "canAccessOpener": false
                    },
                    "waitingForDebugger": true
                }),
                Some("session-a".to_owned()),
            ))
            .expect("Worker attachment should enqueue");

        // Act
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("selected Worker attachment should be retained");

        // Assert
        assert_eq!(
            fixture.factory.client("target-worker").method_names(),
            [
                "Runtime.enable",
                "Target.setAutoAttach",
                "Runtime.runIfWaitingForDebugger"
            ]
        );
        assert!(
            !fixture
                .browser
                .method_names()
                .contains(&"Target.detachFromTargetInParent".to_owned())
        );
    }

    #[tokio::test]
    async fn worker_network_observer_enables_network_before_releasing_debugger_pause() {
        // Arrange
        let mut fixture = fixture(8);
        fixture.driver = fixture
            .driver
            .with_frame_event_observer(Arc::new(WorkerRetentionObserver { network: true }))
            .unwrap();
        fixture
            .sender
            .send(CdpEvent::new(
                "Target.attachedToTarget".to_owned(),
                serde_json::json!({
                    "sessionId": "session-worker",
                    "targetInfo": {
                        "targetId": "target-worker",
                        "type": "worker",
                        "title": "",
                        "url": "https://example.test/worker.js",
                        "attached": true,
                        "canAccessOpener": false
                    },
                    "waitingForDebugger": true
                }),
                Some("session-a".to_owned()),
            ))
            .expect("Worker attachment should enqueue");

        // Act
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("selected Worker attachment should be retained");

        // Assert
        assert_eq!(
            fixture.factory.client("target-worker").method_names(),
            [
                "Runtime.enable",
                "Target.setAutoAttach",
                "Network.enable",
                "Runtime.runIfWaitingForDebugger"
            ]
        );
    }

    #[tokio::test]
    async fn oopif_attachment_initializes_child_session_before_returning() {
        // Arrange
        let mut fixture = fixture(8);
        fixture.factory.set_frame_tree(
            "frame-oopif",
            frame_tree(serde_json::json!({
                "frame": frame_value("frame-oopif", Some("target-a"))
            })),
        );
        fixture
            .sender
            .send(CdpEvent::new(
                "Target.attachedToTarget".to_owned(),
                serde_json::json!({
                    "sessionId":"session-oopif",
                    "targetInfo": {
                        "targetId":"frame-oopif",
                        "type":"iframe",
                        "title":"",
                        "url":"https://other.example",
                        "attached":true,
                        "canAccessOpener":false,
                        "parentFrameId":"target-a"
                    },
                    "waitingForDebugger":false
                }),
                Some("session-a".to_owned()),
            ))
            .expect("OOPIF event should enqueue");

        // Act
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("OOPIF event should apply");

        // Assert
        assert_eq!(fixture.driver.initialized_session_count(), 2);
        assert_eq!(
            fixture.factory.client("frame-oopif").method_names(),
            [
                "Page.enable",
                "Page.getFrameTree",
                "Runtime.enable",
                "Target.setAutoAttach"
            ]
        );
        assert_eq!(
            fixture
                .topology
                .lock()
                .expect("topology should be available")
                .topology()
                .target_session_route("session-oopif")
                .map(|route| (route.target_id(), route.page_target_id())),
            Some(("frame-oopif", "target-a"))
        );
    }

    #[tokio::test]
    async fn local_oopif_swap_preserves_identity_and_late_world_destroy_is_harmless() {
        // Arrange a local child and its first main world.
        let mut fixture = fixture(16);
        for event in [
            CdpEvent::new(
                "Page.frameAttached".to_owned(),
                serde_json::json!({"frameId":"swapping-frame","parentFrameId":"target-a"}),
                Some("session-a".to_owned()),
            ),
            CdpEvent::new(
                "Runtime.executionContextCreated".to_owned(),
                serde_json::json!({"context":{
                    "id":21,"origin":"https://one.example","name":"",
                    "uniqueId":"old-world",
                    "auxData":{"frameId":"swapping-frame","isDefault":true}
                }}),
                Some("session-a".to_owned()),
            ),
            CdpEvent::new(
                "Page.frameDetached".to_owned(),
                serde_json::json!({"frameId":"swapping-frame","reason":"swap"}),
                Some("session-a".to_owned()),
            ),
            CdpEvent::new(
                "Target.attachedToTarget".to_owned(),
                serde_json::json!({
                    "sessionId":"session-swapped",
                    "targetInfo":{
                        "targetId":"swapping-frame","type":"iframe","title":"",
                        "url":"https://two.example","attached":true,
                        "canAccessOpener":false,"parentFrameId":"target-a"
                    },
                    "waitingForDebugger":false
                }),
                Some("session-a".to_owned()),
            ),
            CdpEvent::new(
                "Runtime.executionContextCreated".to_owned(),
                serde_json::json!({"context":{
                    "id":22,"origin":"https://two.example","name":"",
                    "uniqueId":"new-world",
                    "auxData":{"frameId":"swapping-frame","isDefault":true}
                }}),
                Some("session-swapped".to_owned()),
            ),
            CdpEvent::new(
                "Runtime.executionContextDestroyed".to_owned(),
                serde_json::json!({
                    "executionContextId":21,
                    "executionContextUniqueId":"old-world"
                }),
                Some("session-a".to_owned()),
            ),
        ] {
            fixture
                .sender
                .send(event)
                .expect("swap event should enqueue");
        }

        // Act
        for _ in 0..6 {
            fixture
                .driver
                .receive_once(&fixture.runtime)
                .await
                .expect("swap event should apply");
        }

        // Assert
        let topology = fixture
            .topology
            .lock()
            .expect("topology should be available");
        assert_eq!(
            topology.topology().frame_object_id("swapping-frame"),
            Some(ObjectId::new(5))
        );
        assert_eq!(
            topology
                .topology()
                .frame_execution_routes()
                .find(|route| route.object_id() == ObjectId::new(5))
                .map(|route| (route.session_id(), route.unique_context_id())),
            Some(("session-swapped", "new-world"))
        );
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(5)), Ok(true));
    }

    #[tokio::test]
    async fn oopif_detach_then_frame_remove_disposes_one_public_frame() {
        // Arrange
        let mut fixture = fixture(8);
        for event in [
            CdpEvent::new(
                "Target.attachedToTarget".to_owned(),
                serde_json::json!({
                    "sessionId":"session-oopif",
                    "targetInfo":{
                        "targetId":"frame-oopif","type":"iframe","title":"",
                        "url":"https://other.example","attached":true,
                        "canAccessOpener":false,"parentFrameId":"target-a"
                    },
                    "waitingForDebugger":false
                }),
                Some("session-a".to_owned()),
            ),
            CdpEvent::new(
                "Target.detachedFromTarget".to_owned(),
                serde_json::json!({
                    "sessionId":"session-oopif",
                    "targetId":"frame-oopif"
                }),
                Some("session-a".to_owned()),
            ),
            CdpEvent::new(
                "Page.frameDetached".to_owned(),
                serde_json::json!({"frameId":"frame-oopif","reason":"remove"}),
                Some("session-a".to_owned()),
            ),
        ] {
            fixture
                .sender
                .send(event)
                .expect("detach event should enqueue");
        }

        // Act
        for _ in 0..3 {
            fixture
                .driver
                .receive_once(&fixture.runtime)
                .await
                .expect("detach event should apply");
        }

        // Assert
        assert_eq!(
            fixture
                .topology
                .lock()
                .expect("topology should be available")
                .topology()
                .frame_object_id("frame-oopif"),
            None
        );
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(5)), Ok(false));
        assert_eq!(fixture.driver.initialized_session_count(), 1);
    }

    #[test]
    fn portable_discovery_events_reject_ambiguous_same_connection_replay() {
        // Arrange
        let fixture = discovery_fixture(4);
        let protocol = WireProtocol::new_exact_directional(
            (),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(64, 0, 0, 64),
            FrameLimits::new(64, 0, 0, 64),
        );
        let (sender, _outbound) = TypedEventOutbound::<BrowserContextPage, _, _>::new(
            protocol,
            (),
            1,
            pwright_core::OverflowPolicy::RejectNewest,
        );
        let publisher = ChromiumTopologyPageEventPublisher::new(Some(sender), None);
        let policy = ChromiumTopologyEventLossPolicy::ReconstructSameConnection {
            maximum_reconstructions: std::num::NonZeroUsize::new(1).expect("one is nonzero"),
        };

        // Act
        let result = ChromiumTopologyConnectionPump::new(fixture.driver, publisher, policy);

        // Assert
        assert!(matches!(
            result,
            Err(
                ChromiumTopologyConnectionPumpConfigurationError::PortableDiscoveryEventsCannotReplay
            )
        ));
    }

    #[tokio::test]
    async fn event_loss_policy_can_require_a_new_remote_connection() {
        // Arrange
        let fixture = fixture(1);
        for sequence in 0..2 {
            fixture
                .sender
                .send(CdpEvent::new(
                    format!("Unknown.event{sequence}"),
                    serde_json::json!({}),
                    None,
                ))
                .expect("overflow event should enqueue");
        }
        let mut pump = ChromiumTopologyConnectionPump::new(
            fixture.driver,
            ChromiumTopologyPageEventPublisher::default(),
            ChromiumTopologyEventLossPolicy::TerminateConnection,
        )
        .expect("terminal loss policy should build");
        let control = empty_pump_control();
        let close_reasons = Arc::new(Mutex::new(Vec::new()));
        let mut close_controls = PumpCloseControl {
            reasons: Arc::clone(&close_reasons),
        };
        let mut supervisor = EncodedServerSupervisor::new(
            PumpTransport::default(),
            ServerSupervisorLimits::new(1).expect("supervisor should build"),
        );

        // Act
        let result = pump.drive_once(&fixture.runtime, &control).await;
        let shutdown = pump
            .terminate_supervisor(&mut supervisor, &mut close_controls)
            .await;

        // Assert
        assert!(matches!(
            result,
            Err(ChromiumTopologyConnectionPumpError::EventLossRequiresReconnect { skipped: 1 })
        ));
        assert_eq!(
            pump.termination(),
            Some(
                ChromiumTopologyConnectionPumpTermination::EventLossRequiresReconnect {
                    skipped: 1
                }
            )
        );
        assert!(matches!(
            shutdown,
            Ok(ServerSupervisorCloseReason::BackendEventFailed)
        ));
        assert_eq!(supervisor.phase(), ServerSupervisorPhase::Closed);
        assert_eq!(
            *close_reasons
                .lock()
                .expect("pump close reasons should be available"),
            vec![ServerSupervisorCloseReason::BackendEventFailed]
        );
        assert_eq!(supervisor.transport().shutdowns.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn closed_cdp_event_stream_terminally_disconnects_the_topology_pump() {
        // Arrange
        let DriverFixture {
            sender,
            runtime,
            driver,
            ..
        } = fixture(4);
        let mut pump = ChromiumTopologyConnectionPump::new(
            driver,
            ChromiumTopologyPageEventPublisher::default(),
            ChromiumTopologyEventLossPolicy::TerminateConnection,
        )
        .expect("topology pump should build");
        let control = empty_pump_control();
        drop(sender);

        // Act
        let first = pump.drive_once(&runtime, &control).await;
        let repeated = pump.drive_once(&runtime, &control).await;

        // Assert
        assert!(matches!(
            first,
            Err(ChromiumTopologyConnectionPumpError::Driver(
                ChromiumTopologyEventDriverError::EventStreamClosed
            ))
        ));
        assert_eq!(
            pump.termination(),
            Some(ChromiumTopologyConnectionPumpTermination::EventStreamClosed)
        );
        assert!(matches!(
            repeated,
            Err(ChromiumTopologyConnectionPumpError::AlreadyTerminated {
                reason: ChromiumTopologyConnectionPumpTermination::EventStreamClosed
            })
        ));
    }

    #[test]
    fn cdp_liveness_backend_is_sticky_after_native_disconnect() {
        // Arrange
        let mut backend = ChromiumCdpLivenessBackend::from_closed_future(Box::pin(ready(())));
        let mut context = Context::from_waker(Waker::noop());

        // Act
        let first = backend.poll_drive_once(&mut context);
        let repeated = backend.poll_drive_once(&mut context);

        // Assert
        assert!(matches!(
            first,
            Poll::Ready(Err(ChromiumCdpLivenessBackendError::NativeConnectionClosed))
        ));
        assert!(matches!(
            repeated,
            Poll::Ready(Err(ChromiumCdpLivenessBackendError::NativeConnectionClosed))
        ));
        assert!(backend.native_connection_closed());
    }

    #[test]
    fn eventful_liveness_yields_to_an_owned_close_disposal_transaction() {
        // Arrange
        let runtime = ControlledDispatcher::new(
            (),
            (),
            NoDeadlineScheduler,
            RuntimeLimits::new(1, 1).with_control_messages(1),
        );
        runtime
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("browser should register");
        runtime.pop_control_message().unwrap().unwrap();
        let prepared = runtime
            .lifecycle_handle()
            .prepare_object_disposal(ObjectId::new(1))
            .expect("owned close should reserve Browser disposal");
        let transport = Arc::new(PumpTransport::default());
        let publisher = BrowserDisconnectedPublisher::new(
            ObjectId::new(1),
            browser_event_protocol(),
            transport,
        );
        let mut backend = ChromiumEventfulCdpLivenessBackend::new(
            ChromiumCdpLivenessBackend::from_closed_future(Box::pin(ready(()))),
            runtime.lifecycle_handle(),
            ObjectId::new(1),
            publisher,
        );
        let mut context = Context::from_waker(Waker::noop());

        // Act
        let while_owned_close_is_active = backend.poll_drive_once(&mut context);
        drop(prepared);
        let after_rollback = backend.poll_drive_once(&mut context);

        // Assert
        assert!(while_owned_close_is_active.is_pending());
        assert!(matches!(
            after_rollback,
            Poll::Ready(Err(ChromiumEventfulCdpLivenessBackendError::Liveness(
                ChromiumCdpLivenessBackendError::NativeConnectionClosed
            )))
        ));
    }

    #[tokio::test]
    async fn eventful_liveness_waits_for_the_shared_publisher_after_disposal() {
        // Arrange
        let runtime = ControlledDispatcher::new(
            (),
            (),
            NoDeadlineScheduler,
            RuntimeLimits::new(1, 1).with_control_messages(1),
        );
        runtime
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .expect("browser should register");
        runtime.pop_control_message().unwrap().unwrap();
        runtime
            .lifecycle_handle()
            .prepare_object_disposal(ObjectId::new(1))
            .expect("Browser disposal should prepare")
            .commit()
            .expect("Browser disposal should commit");
        let transport = Arc::new(PumpTransport::default());
        let publisher = BrowserDisconnectedPublisher::new(
            ObjectId::new(1),
            browser_event_protocol(),
            transport,
        );
        let mut backend = ChromiumEventfulCdpLivenessBackend::new(
            ChromiumCdpLivenessBackend::from_closed_future(Box::pin(ready(()))),
            runtime.lifecycle_handle(),
            ObjectId::new(1),
            publisher.clone(),
        );
        let mut context = Context::from_waker(Waker::noop());

        // Act
        let before_event = backend.poll_drive_once(&mut context);
        publisher
            .publish()
            .await
            .expect("the owned close path should publish disconnected");
        let after_event = backend.poll_drive_once(&mut context);

        // Assert
        assert!(before_event.is_pending());
        assert!(matches!(
            after_event,
            Poll::Ready(Err(ChromiumEventfulCdpLivenessBackendError::Liveness(
                ChromiumCdpLivenessBackendError::NativeConnectionClosed
            )))
        ));
    }

    #[test]
    fn cdp_liveness_composes_with_a_pending_topology_backend() {
        // Arrange
        let fixture = fixture(4);
        let control = empty_pump_control();
        let pump = ChromiumTopologyConnectionPump::new(
            fixture.driver,
            ChromiumTopologyPageEventPublisher::default(),
            ChromiumTopologyEventLossPolicy::TerminateConnection,
        )
        .expect("topology pump should build");
        let topology = ChromiumTopologyServerBackend::new(pump, Arc::new(fixture.runtime), control);
        let liveness = ChromiumCdpLivenessBackend::from_closed_future(Box::pin(ready(())));
        let mut backend = topology.and_backend(liveness);
        let mut context = Context::from_waker(Waker::noop());

        // Act
        let result = backend.poll_drive_once(&mut context);

        // Assert
        assert!(matches!(
            result,
            Poll::Ready(Err(ServerBackendChainError::Right(
                ChromiumCdpLivenessBackendError::NativeConnectionClosed
            )))
        ));
        backend.handle_connection_terminated(ServerSupervisorCloseReason::BackendEventFailed);
        let (topology, liveness) = backend.into_parts();
        assert!(!topology.is_driving());
        assert_eq!(
            topology.connection_termination(),
            Some(ServerSupervisorCloseReason::BackendEventFailed)
        );
        assert!(liveness.native_connection_closed());
        assert_eq!(
            liveness.connection_termination(),
            Some(ServerSupervisorCloseReason::BackendEventFailed)
        );
    }

    #[test]
    fn server_backend_retains_one_pending_topology_turn_until_completion() {
        // Arrange
        let fixture = fixture(4);
        let control = empty_pump_control();
        let pump = ChromiumTopologyConnectionPump::new(
            fixture.driver,
            ChromiumTopologyPageEventPublisher::default(),
            ChromiumTopologyEventLossPolicy::TerminateConnection,
        )
        .expect("topology pump should build");
        let mut backend =
            ChromiumTopologyServerBackend::new(pump, Arc::new(fixture.runtime), control);
        let mut context = Context::from_waker(Waker::noop());

        // Act
        let first = backend.poll_drive_once(&mut context);
        fixture
            .sender
            .send(CdpEvent::new(
                "Unknown.event".to_owned(),
                serde_json::json!({}),
                None,
            ))
            .expect("backend event should enqueue");
        let second = backend.poll_drive_once(&mut context);

        // Assert
        assert!(first.is_pending());
        assert!(
            matches!(second, Poll::Ready(Ok(_))),
            "unexpected retained topology result: {second:?}"
        );
        assert!(!backend.is_driving());
        assert_eq!(
            backend
                .pump()
                .expect("completed turn should restore the pump")
                .termination(),
            None
        );
    }

    #[test]
    fn server_backend_releases_a_pending_turn_when_the_connection_run_is_dropped() {
        // Arrange
        let fixture = fixture(4);
        let control = empty_pump_control();
        let pump = ChromiumTopologyConnectionPump::new(
            fixture.driver,
            ChromiumTopologyPageEventPublisher::default(),
            ChromiumTopologyEventLossPolicy::TerminateConnection,
        )
        .expect("topology pump should build");
        let mut backend =
            ChromiumTopologyServerBackend::new(pump, Arc::new(fixture.runtime), control);
        let mut context = Context::from_waker(Waker::noop());
        assert!(backend.poll_drive_once(&mut context).is_pending());

        // Act
        backend.handle_connection_terminated(ServerSupervisorCloseReason::RunFutureDropped);
        let after_close = backend.poll_drive_once(&mut context);

        // Assert
        assert!(!backend.is_driving());
        assert!(backend.pump().is_none());
        assert_eq!(
            backend.connection_termination(),
            Some(ServerSupervisorCloseReason::RunFutureDropped)
        );
        assert!(matches!(
            after_close,
            Poll::Ready(Err(
                ChromiumTopologyServerBackendError::ConnectionTerminated {
                    reason: ServerSupervisorCloseReason::RunFutureDropped
                }
            ))
        ));
    }

    #[tokio::test]
    async fn inert_pump_can_reconstruct_once_on_the_same_live_connection() {
        // Arrange
        let fixture = discovery_fixture(1);
        for sequence in 0..2 {
            fixture
                .sender
                .send(CdpEvent::new(
                    format!("Unknown.event{sequence}"),
                    serde_json::json!({}),
                    None,
                ))
                .expect("overflow event should enqueue");
        }
        let mut pump = ChromiumTopologyConnectionPump::new(
            fixture.driver,
            ChromiumTopologyPageEventPublisher::default(),
            ChromiumTopologyEventLossPolicy::ReconstructSameConnection {
                maximum_reconstructions: std::num::NonZeroUsize::new(1).expect("one is nonzero"),
            },
        )
        .expect("inert publisher may replay topology");
        let control = empty_pump_control();

        // Act
        let outcome = pump
            .drive_once(&fixture.runtime, &control)
            .await
            .expect("same connection should reconstruct");
        let active_after_first = pump.termination();
        for sequence in 2..4 {
            fixture
                .sender
                .send(CdpEvent::new(
                    format!("Unknown.event{sequence}"),
                    serde_json::json!({}),
                    None,
                ))
                .expect("second overflow should enqueue");
        }
        let exhausted = pump.drive_once(&fixture.runtime, &control).await;

        // Assert
        assert!(matches!(
            outcome,
            ChromiumTopologyConnectionPumpOutcome::Reconstructed {
                skipped: 1,
                epoch,
                ..
            } if epoch == ConnectionEpoch::new(2)
        ));
        assert_eq!(pump.reconstruction_count(), 1);
        assert_eq!(active_after_first, None);
        assert!(matches!(
            exhausted,
            Err(ChromiumTopologyConnectionPumpError::ReconstructionLimitExceeded { limit: 1 })
        ));
        assert_eq!(pump.driver().resync_required(), Some(1));
        assert_eq!(
            pump.termination(),
            Some(
                ChromiumTopologyConnectionPumpTermination::ReconstructionLimitExceeded { limit: 1 }
            )
        );
    }

    #[tokio::test]
    async fn portable_event_overflow_makes_the_connection_pump_terminal() {
        // Arrange
        let fixture = fixture(4);
        fixture
            .sender
            .send(CdpEvent::new(
                "Unknown.event".to_owned(),
                serde_json::json!({}),
                None,
            ))
            .expect("event should enqueue behind initial Page synchronization");
        let protocol = WireProtocol::new_exact_directional(
            (),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(64, 0, 0, 64),
            FrameLimits::new(64, 0, 0, 64),
        );
        let (sender, _outbound) = TypedEventOutbound::<BrowserContextPage, _, _>::new(
            protocol,
            (),
            0,
            pwright_core::OverflowPolicy::RejectNewest,
        );
        let publisher = ChromiumTopologyPageEventPublisher::new(Some(sender.clone()), None);
        let control = empty_pump_control();
        let mut pump = ChromiumTopologyConnectionPump::new(
            fixture.driver,
            publisher,
            ChromiumTopologyEventLossPolicy::TerminateConnection,
        )
        .expect("terminal portable-event policy should build");

        // Act
        let result = pump.drive_once(&fixture.runtime, &control).await;

        // Assert
        assert!(matches!(
            result,
            Err(ChromiumTopologyConnectionPumpError::PortableEvent(_))
        ));
        assert_eq!(
            pump.termination(),
            Some(ChromiumTopologyConnectionPumpTermination::PortableEventFailed)
        );
        assert_eq!(
            sender
                .metrics()
                .expect("event metrics")
                .queue()
                .rejected_newest,
            1
        );
    }

    #[tokio::test]
    async fn dropping_a_polled_drive_future_is_detected_as_terminal() {
        // Arrange
        let fixture = fixture(4);
        let mut pump = ChromiumTopologyConnectionPump::new(
            fixture.driver,
            ChromiumTopologyPageEventPublisher::default(),
            ChromiumTopologyEventLossPolicy::TerminateConnection,
        )
        .expect("pump should build");
        let control = empty_pump_control();
        let mut first = Box::pin(pump.drive_once(&fixture.runtime, &control));
        let first_poll = first.as_mut().poll(&mut Context::from_waker(Waker::noop()));

        // Act
        drop(first);
        let second = pump.drive_once(&fixture.runtime, &control).await;

        // Assert
        assert!(matches!(first_poll, Poll::Pending));
        assert!(matches!(
            second,
            Err(ChromiumTopologyConnectionPumpError::PreviousDriveFutureDropped)
        ));
        assert_eq!(
            pump.termination(),
            Some(ChromiumTopologyConnectionPumpTermination::PreviousDriveFutureDropped)
        );
    }

    #[tokio::test]
    async fn broadcast_lag_requires_resync_without_mutating_topology() {
        // Arrange
        let mut fixture = fixture(1);
        for frame_id in ["child-a", "child-b"] {
            fixture
                .sender
                .send(CdpEvent::new(
                    "Page.frameAttached".to_owned(),
                    serde_json::json!({"frameId":frame_id,"parentFrameId":"target-a"}),
                    Some("session-a".to_owned()),
                ))
                .expect("frame event should enqueue");
        }

        // Act
        let error = fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect_err("lag must not be hidden");

        // Assert
        assert!(matches!(
            error,
            ChromiumTopologyEventDriverError::ResyncRequired { skipped: 1 }
        ));
        assert_eq!(fixture.driver.resync_required(), Some(1));
        let repeated = fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect_err("event loss must remain sticky until explicit reconstruction");
        assert!(matches!(
            repeated,
            ChromiumTopologyEventDriverError::ResyncRequired { skipped: 1 }
        ));
        let topology = fixture
            .topology
            .lock()
            .expect("topology should be available");
        assert_eq!(topology.topology().frame_object_id("child-a"), None);
        assert_eq!(topology.topology().frame_object_id("child-b"), None);
    }

    #[tokio::test]
    async fn resync_requires_browser_root_page_discovery() {
        // Arrange
        let mut fixture = fixture(1);
        for frame_id in ["child-a", "child-b"] {
            fixture
                .sender
                .send(CdpEvent::new(
                    "Page.frameAttached".to_owned(),
                    serde_json::json!({"frameId":frame_id,"parentFrameId":"target-a"}),
                    Some("session-a".to_owned()),
                ))
                .expect("frame event should enqueue");
        }
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect_err("bounded event loss should require reconstruction");

        // Act
        let error = fixture
            .driver
            .resynchronize_after_event_loss(&fixture.runtime, ConnectionEpoch::new(2))
            .await
            .expect_err("maintenance-only drivers cannot enumerate browser-root Pages");

        // Assert
        assert!(matches!(
            error,
            ChromiumTopologyEventDriverError::PageDiscoveryRequiredForResync
        ));
        assert_eq!(fixture.driver.resync_required(), Some(1));
        assert_eq!(
            fixture
                .topology
                .lock()
                .expect("topology should remain available")
                .topology()
                .epoch(),
            ConnectionEpoch::new(1)
        );
    }

    #[tokio::test]
    async fn lost_events_reconstruct_authoritative_pages_frames_and_sessions() {
        // Arrange
        let mut fixture = discovery_fixture(2);
        fixture.factory.set_frame_tree(
            "page-a",
            frame_tree(serde_json::json!({
                "frame": frame_value("page-a", None),
                "childFrames": [{"frame": frame_value("stale-child", Some("page-a"))}]
            })),
        );
        fixture
            .sender
            .send(attached_page_event("page-a", "session-a", None, None))
            .expect("first Page should enqueue");
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("first Page should initialize");
        fixture
            .sender
            .send(attached_page_event("page-b", "session-b", None, None))
            .expect("second Page should enqueue");
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("second Page should initialize");
        for sequence in 0..3 {
            fixture
                .sender
                .send(CdpEvent::new(
                    format!("Unknown.event{sequence}"),
                    serde_json::json!({}),
                    None,
                ))
                .expect("overflow event should enqueue");
        }
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect_err("the overwritten event must require reconstruction");
        fixture.factory.client("page-a").set_frame_tree(frame_tree(
            serde_json::json!({"frame": frame_value("page-a", None)}),
        ));
        fixture.browser.set_auto_attach_events(
            fixture.sender.clone(),
            [attached_page_event(
                "page-a",
                "session-a-rebound",
                None,
                None,
            )],
        );

        // Act
        let outcome = fixture
            .driver
            .resynchronize_after_event_loss(&fixture.runtime, ConnectionEpoch::new(2))
            .await
            .expect("authoritative auto-attach snapshot should rebuild topology");

        // Assert
        let ChromiumTopologyEventDriverOutcome::Updated(update) = outcome else {
            panic!("reconstruction must publish its ordered state transition");
        };
        assert!(matches!(
            update.effects().first(),
            Some(pwright_backend_cdp::CdpTopologyEffect::ReconnectStarted { epoch })
                if *epoch == ConnectionEpoch::new(2)
        ));
        assert!(update.effects().iter().any(|effect| matches!(
            effect,
            pwright_backend_cdp::CdpTopologyEffect::PageReady { page, context }
                if *page == ObjectId::new(3) && *context == ObjectId::new(2)
        )));
        assert!(update.effects().iter().any(|effect| matches!(
            effect,
            pwright_backend_cdp::CdpTopologyEffect::Lifecycle(
                pwright_core::ControlMessage::Dispose { object_id }
            ) if *object_id == ObjectId::new(5)
        )));
        assert!(update.effects().iter().any(|effect| matches!(
            effect,
            pwright_backend_cdp::CdpTopologyEffect::Lifecycle(
                pwright_core::ControlMessage::Dispose { object_id }
            ) if *object_id == ObjectId::new(6)
        )));
        assert!(matches!(
            update.effects().last(),
            Some(pwright_backend_cdp::CdpTopologyEffect::ReconnectFinished {
                epoch,
                live_pages: 1
            }) if *epoch == ConnectionEpoch::new(2)
        ));
        assert_eq!(fixture.driver.resync_required(), None);
        assert_eq!(fixture.driver.initialized_session_count(), 1);
        {
            let topology = fixture
                .topology
                .lock()
                .expect("reconstructed topology should remain available");
            assert_eq!(topology.topology().epoch(), ConnectionEpoch::new(2));
            assert_eq!(
                topology.topology().page_object_id("page-a"),
                Some(ObjectId::new(3))
            );
            assert_eq!(topology.topology().page_object_id("page-b"), None);
            assert_eq!(topology.topology().frame_object_id("stale-child"), None);
            assert_eq!(
                topology
                    .topology()
                    .target_session_route("session-a-rebound")
                    .map(|route| route.target_id()),
                Some("page-a")
            );
        }
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(3)), Ok(true));
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(4)), Ok(true));
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(5)), Ok(false));
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(6)), Ok(false));
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(7)), Ok(false));
        let browser_methods = fixture.browser.method_names();
        assert_eq!(
            &browser_methods[browser_methods.len() - 4..],
            [
                "Target.setAutoAttach",
                "Target.setAutoAttach",
                "Target.getTargetInfo",
                "Target.getBrowserContexts"
            ]
        );

        fixture
            .sender
            .send(CdpEvent::new(
                "Page.frameAttached".to_owned(),
                serde_json::json!({"frameId":"fresh-child","parentFrameId":"page-a"}),
                Some("session-a-rebound".to_owned()),
            ))
            .expect("post-reconstruction event should enqueue");
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("ordinary event consumption should resume");
        assert_eq!(
            fixture
                .topology
                .lock()
                .expect("topology should remain available")
                .topology()
                .frame_object_id("fresh-child"),
            Some(ObjectId::new(8))
        );
    }

    #[tokio::test]
    async fn lost_events_preserve_live_network_lanes_and_retire_missing_ones() {
        // Arrange
        let mut fixture = network_discovery_fixture(2);
        ChromiumPageSubscriptionActivator::new(fixture.runtime.service().clone())
            .set_subscription(ObjectId::new(2), NETWORK_REQUEST_EVENT_TAG, true)
            .await
            .expect("Context network interest should activate");
        for (target, session) in [("page-a", "session-a"), ("page-b", "session-b")] {
            fixture.factory.set_frame_tree(
                target,
                frame_tree(serde_json::json!({"frame": frame_value(target, None)})),
            );
            fixture
                .sender
                .send(attached_page_event(
                    target,
                    session,
                    Some("context-a"),
                    None,
                ))
                .expect("Page attachment should enqueue");
            fixture
                .driver
                .receive_once(&fixture.runtime)
                .await
                .expect("Page should materialize with its network lane");
        }
        assert_eq!(fixture.network_driver.as_ref().unwrap().page_count(), 2);
        for sequence in 0..3 {
            fixture
                .sender
                .send(CdpEvent::new(
                    format!("Unknown.event{sequence}"),
                    serde_json::json!({}),
                    None,
                ))
                .expect("overflow event should enqueue");
        }
        fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect_err("the overwritten event must require reconstruction");
        fixture.factory.client("page-a").set_frame_tree(frame_tree(
            serde_json::json!({"frame": frame_value("page-a", None)}),
        ));
        fixture
            .browser
            .set_browser_contexts_response(vec!["context-a".to_owned()]);
        fixture.browser.set_auto_attach_events(
            fixture.sender.clone(),
            [attached_page_event(
                "page-a",
                "session-a-rebound",
                Some("context-a"),
                None,
            )],
        );

        // Act
        fixture
            .driver
            .resynchronize_after_event_loss(&fixture.runtime, ConnectionEpoch::new(2))
            .await
            .expect("network lanes should follow the authoritative snapshot");

        // Assert
        let network_driver = fixture.network_driver.as_ref().unwrap();
        assert_eq!(network_driver.page_count(), 1);
        assert_eq!(network_driver.page_registration_in_progress(), None);
        let topology = fixture
            .topology
            .lock()
            .expect("topology should remain readable");
        assert_eq!(
            topology.topology().page_object_id("page-a"),
            Some(ObjectId::new(4))
        );
        assert_eq!(
            topology
                .topology()
                .page_session_routes()
                .find(|route| route.object_id() == ObjectId::new(4))
                .map(|route| route.session_id()),
            Some("session-a-rebound")
        );
        drop(topology);
        network_driver
            .registration_handle()
            .validate_page_session(ObjectId::new(4), "session-a-rebound")
            .expect("the live Page network adapter should use the rebound session");
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(4)), Ok(true));
        assert_eq!(fixture.runtime.is_object_live(ObjectId::new(6)), Ok(false));
        let metrics = fixture
            .runtime
            .service()
            .network_subscription_registry()
            .expect("network registry should remain installed")
            .metrics()
            .expect("network registry metrics should remain readable");
        assert_eq!(metrics.registered_page_lanes(), 1);
        assert_eq!(metrics.active_page_lanes(), 0);
        assert_eq!(metrics.active_browser_contexts(), 1);
    }

    #[tokio::test]
    async fn reconnect_makes_old_driver_stale_before_it_consumes_an_event() {
        // Arrange
        let mut fixture = fixture(2);
        fixture
            .topology
            .lock()
            .expect("topology should be available")
            .begin_reconnect(pwright_backend_cdp::ConnectionEpoch::new(2))
            .expect("reconnect should begin");
        fixture
            .sender
            .send(CdpEvent::new(
                "Page.frameAttached".to_owned(),
                serde_json::json!({"frameId":"late-child","parentFrameId":"target-a"}),
                Some("session-a".to_owned()),
            ))
            .expect("late event should enqueue");

        // Act
        let error = fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect_err("old driver must be rejected");

        // Assert
        assert!(matches!(
            error,
            ChromiumTopologyEventDriverError::StaleEpoch { driver, current }
                if driver.get() == 1 && current.get() == 2
        ));
        assert_eq!(
            fixture
                .topology
                .lock()
                .expect("topology should be available")
                .topology()
                .frame_object_id("late-child"),
            None
        );
    }

    #[tokio::test]
    async fn page_load_backend_publishes_without_consuming_the_topology_cursor() {
        // Arrange
        let mut fixture = fixture(4);
        {
            let mut topology = fixture
                .topology
                .lock()
                .expect("topology should be available");
            topology
                .attach_frame(
                    ConnectionEpoch::new(1),
                    CdpFrameAttachment::main("target-a", "frame-a"),
                    ObjectId::new(4),
                )
                .expect("main frame should bind");
            fixture
                .runtime
                .service()
                .synchronize(&topology)
                .expect("Page route should remain synchronized");
        }
        let event_protocol = WireProtocol::new_exact_directional(
            PageEventsCodec,
            ProtocolVersion::new(1, 0),
            FrameLimits::new(64, 0, 0, 64),
            FrameLimits::new(64, 0, 0, 64),
        );
        let (sender, _outbound) = TypedEventOutbound::<PageLoad, _, _>::new(
            event_protocol,
            (),
            1,
            OverflowPolicy::RejectNewest,
        );
        let session_factory: Arc<dyn SessionFactory> = fixture.factory.clone();
        let mut backend = ChromiumPageLoadServerBackend::new(
            fixture.sender.subscribe(),
            CdpPageEventAdapter::new(session_factory),
            Arc::clone(&fixture.topology),
            sender.clone(),
        );
        fixture
            .sender
            .send(CdpEvent::new(
                "Page.lifecycleEvent".to_owned(),
                serde_json::json!({"name":"load","frameId":"frame-a"}),
                Some("session-a".to_owned()),
            ))
            .expect("load event should reach both broadcast cursors");

        // Act
        let published = std::future::poll_fn(|context| backend.poll_drive_once(context))
            .await
            .expect("PageLoad backend should publish");
        let _topology_outcome = fixture
            .driver
            .receive_once(&fixture.runtime)
            .await
            .expect("topology cursor should independently observe the same event");

        // Assert
        assert_eq!(
            published,
            ChromiumPageLoadServerBackendOutcome::Published {
                page: ObjectId::new(3),
            }
        );
        assert_eq!(fixture.driver.initialized_session_count(), 1);
        assert_eq!(
            sender
                .metrics()
                .expect("event queue metrics should be available")
                .queue()
                .current_len,
            1
        );
    }
}
