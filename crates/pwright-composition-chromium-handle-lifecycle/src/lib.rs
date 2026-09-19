//! Minimal Chromium composition for explicit JavaScript Handle disposal.
//!
//! This leaf owns the bounded portable-to-native route registry shared by
//! Handle producers. Producers inject an object-local disposer and may attach
//! opaque backend state, so lifecycle never depends on evaluation. It owns no
//! browser process, transport, executor, or hidden task.

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll};

pub use pwright_backend_cdp_handle_lifecycle::{
    CdpHandleDisposeLimits, CdpHandleDisposeService, CdpHandleDisposeSetupError,
};
use pwright_cap_handle_lifecycle::{
    HANDLE_LIFECYCLE_CAPABILITY_ID, HandleDispose, HandleLifecycleErrorDetails,
};
use pwright_cdp::CdpClient;
use pwright_codec_handle_lifecycle::HandleLifecycleCodec;
pub use pwright_codec_handle_lifecycle::HandleLifecycleCodecLimits;
use pwright_core::{
    CallId, CallMetadata, Command, CommandResult, ControlMessage, FailureKind, ObjectId,
    OperationError, ProductionResourceError, ProductionResourceLedger, ProtocolVersion,
    ResourceClass, ResourceError, ResourceLease, ResourceRequest, ResourceRequirements,
    ResourceScope, Service,
};
use pwright_runtime::{
    CleanupFailure, CleanupReservation, CleanupSubmitError, ControlledLifecycleHandle,
    ReservedCleanupExecutor, RuntimeStateError,
};
use pwright_server_handle_lifecycle::{
    HandleLifecycleServerHandlers, HandleLifecycleServerServices, handle_lifecycle_server_handlers,
};
pub use pwright_transport::FrameLimits;
use pwright_transport::WireProtocol;

#[cfg(feature = "native-release")]
pub mod native_release;

#[cfg(feature = "native-release")]
type RouteResource = Arc<ResourceLease>;
#[cfg(not(feature = "native-release"))]
type RouteResource = ResourceLease;

fn route_resource(lease: ResourceLease) -> RouteResource {
    #[cfg(feature = "native-release")]
    {
        Arc::new(lease)
    }
    #[cfg(not(feature = "native-release"))]
    {
        lease
    }
}

fn route_bytes(native: usize, preview: usize) -> u64 {
    let bytes = u64::try_from(native)
        .unwrap_or(u64::MAX)
        .saturating_add(u64::try_from(preview).unwrap_or(u64::MAX));
    #[cfg(feature = "native-release")]
    let bytes = bytes.saturating_add(native_release::ROUTE_OWNER_BYTES);
    bytes
}

/// Production admission for routes retained by optional Handle producers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumHandleRouteProfile {
    scope: ResourceScope,
    max_handles: usize,
    max_native_id_bytes: usize,
    max_preview_bytes: usize,
}

impl ChromiumHandleRouteProfile {
    pub const fn new(
        scope: ResourceScope,
        max_handles: usize,
        max_native_id_bytes: usize,
        max_preview_bytes: usize,
    ) -> Self {
        Self {
            scope,
            max_handles,
            max_native_id_bytes,
            max_preview_bytes,
        }
    }

    pub const fn scope(self) -> ResourceScope {
        self.scope
    }

    pub const fn max_handles(self) -> usize {
        self.max_handles
    }

    pub const fn max_native_id_bytes(self) -> usize {
        self.max_native_id_bytes
    }

    pub const fn max_preview_bytes(self) -> usize {
        self.max_preview_bytes
    }

    pub fn requirements(self) -> ResourceRequirements {
        let handles = u64::try_from(self.max_handles).unwrap_or(u64::MAX);
        let bytes_per_handle = route_bytes(self.max_native_id_bytes, self.max_preview_bytes);
        if self.max_handles == 0 {
            return ResourceRequirements::none();
        }
        ResourceRequirements::none()
            .with_class(ResourceClass::RemoteHandle, handles, handles)
            .with_class(
                ResourceClass::BackendStateBytes,
                handles.saturating_mul(bytes_per_handle),
                handles.saturating_mul(bytes_per_handle),
            )
            .with_metadata(1, self.max_handles)
    }

    fn validate(self) -> Result<Self, ChromiumHandleLifecycleSetupError> {
        if self.max_handles == 0 {
            return Err(ChromiumHandleLifecycleSetupError::ZeroHandleCapacity);
        }
        if self.max_native_id_bytes == 0 {
            return Err(ChromiumHandleLifecycleSetupError::ZeroNativeIdBytes);
        }
        Ok(self)
    }
}

/// Complete route and wire bounds for `pwright.handle_lifecycle` 1.0.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumHandleLifecycleProfile {
    routes: ChromiumHandleRouteProfile,
    codec: HandleLifecycleCodecLimits,
    send_limits: FrameLimits,
    receive_limits: FrameLimits,
}

impl ChromiumHandleLifecycleProfile {
    pub const VERSION: ProtocolVersion = ProtocolVersion::new(1, 0);

    pub const fn new(
        routes: ChromiumHandleRouteProfile,
        codec: HandleLifecycleCodecLimits,
        send_limits: FrameLimits,
        receive_limits: FrameLimits,
    ) -> Self {
        Self {
            routes,
            codec,
            send_limits,
            receive_limits,
        }
    }

    pub const fn routes(self) -> ChromiumHandleRouteProfile {
        self.routes
    }

    pub const fn codec(self) -> HandleLifecycleCodecLimits {
        self.codec
    }

    pub const fn send_limits(self) -> FrameLimits {
        self.send_limits
    }

    pub const fn receive_limits(self) -> FrameLimits {
        self.receive_limits
    }

    pub fn requirements(self) -> ResourceRequirements {
        self.routes.requirements().combine_concurrent(
            self.unbudgeted_protocol()
                .production_resource_requirements(),
        )
    }

    fn unbudgeted_protocol(self) -> WireProtocol<HandleLifecycleCodec> {
        WireProtocol::new_exact_directional(
            HandleLifecycleCodec::new(self.codec),
            Self::VERSION,
            self.send_limits,
            self.receive_limits,
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChromiumHandleLifecycleSetupError {
    ZeroHandleCapacity,
    ZeroNativeIdBytes,
}

impl Display for ChromiumHandleLifecycleSetupError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroHandleCapacity => {
                formatter.write_str("Handle route capacity must be non-zero")
            }
            Self::ZeroNativeIdBytes => {
                formatter.write_str("Handle native-object ID bound must be non-zero")
            }
        }
    }
}

impl Error for ChromiumHandleLifecycleSetupError {}

#[derive(Debug)]
pub enum ChromiumHandleLifecycleInstallError {
    Setup(ChromiumHandleLifecycleSetupError),
    Resources(ProductionResourceError),
}

impl Display for ChromiumHandleLifecycleInstallError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Setup(error) => Display::fmt(error, formatter),
            Self::Resources(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ChromiumHandleLifecycleInstallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Setup(error) => Some(error),
            Self::Resources(error) => Some(error),
        }
    }
}

impl From<ChromiumHandleLifecycleSetupError> for ChromiumHandleLifecycleInstallError {
    fn from(error: ChromiumHandleLifecycleSetupError) -> Self {
        Self::Setup(error)
    }
}

impl From<ProductionResourceError> for ChromiumHandleLifecycleInstallError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Resources(error)
    }
}

/// Object-local native cleanup injected by a Handle-producing plugin.
pub trait ChromiumHandleDisposer: Send + Sync {
    fn dispose(
        &self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), OperationError<HandleLifecycleErrorDetails>>>
                + Send
                + '_,
        >,
    >;
}

impl ChromiumHandleDisposer for CdpHandleDisposeService {
    fn dispose(
        &self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), OperationError<HandleLifecycleErrorDetails>>>
                + Send
                + '_,
        >,
    > {
        Box::pin(self.call(()))
    }
}

/// Producer-supplied route data. Backend-specific payload remains opaque here.
pub struct ChromiumHandleRouteSpec {
    owner_frame: ObjectId,
    owner_page: ObjectId,
    native_id_bytes: usize,
    preview_bytes: usize,
    disposer: Option<Arc<dyn ChromiumHandleDisposer>>,
    payload: Arc<dyn Any + Send + Sync>,
}

impl ChromiumHandleRouteSpec {
    pub fn new(
        owner_frame: ObjectId,
        owner_page: ObjectId,
        native_id_bytes: usize,
        preview_bytes: usize,
        disposer: Option<Arc<dyn ChromiumHandleDisposer>>,
    ) -> Self {
        Self {
            owner_frame,
            owner_page,
            native_id_bytes,
            preview_bytes,
            disposer,
            payload: Arc::new(()),
        }
    }

    pub fn from_cdp(
        owner_frame: ObjectId,
        owner_page: ObjectId,
        client: Arc<dyn CdpClient>,
        native_id: Option<&str>,
        preview_bytes: usize,
        limits: CdpHandleDisposeLimits,
    ) -> Result<Self, CdpHandleDisposeSetupError> {
        let disposer = native_id
            .map(|native_id| {
                CdpHandleDisposeService::new(client, native_id, limits)
                    .map(|service| Arc::new(service) as Arc<dyn ChromiumHandleDisposer>)
            })
            .transpose()?;
        Ok(Self::new(
            owner_frame,
            owner_page,
            native_id.map_or(0, str::len),
            preview_bytes,
            disposer,
        ))
    }

    pub fn with_payload<T>(mut self, payload: Arc<T>) -> Self
    where
        T: Any + Send + Sync,
    {
        self.payload = payload;
        self
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChromiumHandleRouteMetrics {
    pub current: usize,
    pub reserved: usize,
    pub peak: usize,
    pub retained_bytes: u64,
    pub peak_retained_bytes: u64,
    pub created: u64,
    pub explicitly_disposed: u64,
    pub parent_retired: u64,
    pub topology_retired: u64,
    pub connection_retired: u64,
}

struct ChromiumHandleRouteEntry {
    owner_frame: ObjectId,
    owner_page: ObjectId,
    disposer: Option<Arc<dyn ChromiumHandleDisposer>>,
    payload: Arc<dyn Any + Send + Sync>,
    retained_bytes: u64,
    _resource: RouteResource,
}

struct ChromiumHandleRouteState {
    routes: BTreeMap<ObjectId, ChromiumHandleRouteEntry>,
    reserved: usize,
    metrics: ChromiumHandleRouteMetrics,
}

/// Shared lifecycle-owned route registry. Handle producers must pre-admit a
/// reservation before native creation and publish through that reservation.
#[derive(Clone)]
pub struct ChromiumHandleRoutes {
    state: Arc<Mutex<ChromiumHandleRouteState>>,
    ledger: pwright_core::ResourceLedger,
    profile: ChromiumHandleRouteProfile,
}

impl ChromiumHandleRoutes {
    /// Installs only the bounded Handle route owner.
    ///
    /// Composition roots that already own protocol registration and command
    /// continuation can use this constructor without also installing the
    /// lifecycle wire protocol or its server handlers.
    pub fn new_with_production_resources(
        profile: ChromiumHandleRouteProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumHandleLifecycleInstallError> {
        let profile = profile.validate()?;
        let ledger = resources.claim(profile.requirements())?;
        Ok(Self {
            state: Arc::new(Mutex::new(ChromiumHandleRouteState {
                routes: BTreeMap::new(),
                reserved: 0,
                metrics: ChromiumHandleRouteMetrics::default(),
            })),
            ledger,
            profile,
        })
    }

    pub const fn profile(&self) -> ChromiumHandleRouteProfile {
        self.profile
    }

    pub fn reserve(&self) -> Result<ChromiumHandleRouteReservation, ChromiumHandleRouteError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ChromiumHandleRouteError::StatePoisoned)?;
        if state.routes.len().saturating_add(state.reserved) >= self.profile.max_handles {
            return Err(ChromiumHandleRouteError::CapacityExceeded {
                limit: self.profile.max_handles,
            });
        }
        let maximum_bytes = route_bytes(
            self.profile.max_native_id_bytes,
            self.profile.max_preview_bytes,
        );
        let lease = self
            .ledger
            .reserve_many(
                self.profile.scope,
                &[
                    ResourceRequest::new(ResourceClass::RemoteHandle, 1),
                    ResourceRequest::new(ResourceClass::BackendStateBytes, maximum_bytes),
                ],
            )
            .map_err(ChromiumHandleRouteError::Resources)?;
        state.reserved = state.reserved.saturating_add(1);
        state.metrics.reserved = state.reserved;
        drop(state);
        Ok(ChromiumHandleRouteReservation {
            routes: self.clone(),
            lease: Some(lease),
            active: true,
        })
    }

    pub fn reserve_batch(
        &self,
        count: usize,
    ) -> Result<ChromiumHandleRouteBatchReservation, ChromiumHandleRouteError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ChromiumHandleRouteError::StatePoisoned)?;
        if count == 0 {
            return Err(ChromiumHandleRouteError::EmptyBatch);
        }
        if state
            .routes
            .len()
            .saturating_add(state.reserved)
            .saturating_add(count)
            > self.profile.max_handles
        {
            return Err(ChromiumHandleRouteError::CapacityExceeded {
                limit: self.profile.max_handles,
            });
        }
        let maximum_bytes = route_bytes(
            self.profile.max_native_id_bytes,
            self.profile.max_preview_bytes,
        );
        let requests = [
            ResourceRequest::new(ResourceClass::RemoteHandle, 1),
            ResourceRequest::new(ResourceClass::BackendStateBytes, maximum_bytes),
        ];
        let mut leases = Vec::with_capacity(count);
        for _ in 0..count {
            leases.push(
                self.ledger
                    .reserve_many(self.profile.scope, &requests)
                    .map_err(ChromiumHandleRouteError::Resources)?,
            );
        }
        state.reserved = state.reserved.saturating_add(count);
        state.metrics.reserved = state.reserved;
        drop(state);
        Ok(ChromiumHandleRouteBatchReservation {
            routes: self.clone(),
            leases,
            count,
            active: true,
        })
    }

    pub fn contains(&self, object_id: ObjectId) -> Result<bool, ChromiumHandleRouteError> {
        self.state
            .lock()
            .map(|state| state.routes.contains_key(&object_id))
            .map_err(|_| ChromiumHandleRouteError::StatePoisoned)
    }

    pub fn metrics(&self) -> Result<ChromiumHandleRouteMetrics, ChromiumHandleRouteError> {
        self.state
            .lock()
            .map(|state| state.metrics)
            .map_err(|_| ChromiumHandleRouteError::StatePoisoned)
    }

    pub fn snapshot<T>(
        &self,
        object_id: ObjectId,
    ) -> Result<Option<ChromiumHandleRouteSnapshot<T>>, ChromiumHandleRouteError>
    where
        T: Any + Send + Sync,
    {
        let state = self
            .state
            .lock()
            .map_err(|_| ChromiumHandleRouteError::StatePoisoned)?;
        let Some(route) = state.routes.get(&object_id) else {
            return Ok(None);
        };
        let payload = Arc::clone(&route.payload)
            .downcast::<T>()
            .map_err(|_| ChromiumHandleRouteError::PayloadTypeMismatch)?;
        Ok(Some(ChromiumHandleRouteSnapshot {
            owner_frame: route.owner_frame,
            owner_page: route.owner_page,
            payload,
        }))
    }

    pub fn retire_parent_disposals(
        &self,
        disposal_order: &[ObjectId],
    ) -> Result<usize, ChromiumHandleRouteError> {
        self.prepare_parent_retirement(disposal_order)
            .map(PreparedChromiumHandleParentRetirement::commit)
    }

    /// Locks the routes selected by an already prepared runtime subtree
    /// disposal. Dropping the prepared value preserves the routes; committing
    /// retires the exact selection as parent-owned cleanup.
    pub fn prepare_parent_retirement(
        &self,
        disposal_order: &[ObjectId],
    ) -> Result<PreparedChromiumHandleParentRetirement<'_>, ChromiumHandleRouteError> {
        let state = self
            .state
            .lock()
            .map_err(|_| ChromiumHandleRouteError::StatePoisoned)?;
        let object_ids = disposal_order
            .iter()
            .copied()
            .filter(|object_id| state.routes.contains_key(object_id))
            .collect();
        Ok(PreparedChromiumHandleParentRetirement { state, object_ids })
    }

    /// Borrow a pre-admitted stable disposal slice for a short synchronous
    /// commit, without allocating another Handle-ID list. Keep this guard in
    /// the selecting coordinator's lock order; dropping it changes no routes.
    pub fn borrow_parent_retirement<'a>(
        &'a self,
        disposal_order: &'a [ObjectId],
    ) -> Result<BorrowedChromiumHandleParentRetirement<'a>, ChromiumHandleRouteError> {
        let state = self
            .state
            .lock()
            .map_err(|_| ChromiumHandleRouteError::StatePoisoned)?;
        Ok(BorrowedChromiumHandleParentRetirement {
            state,
            object_ids: disposal_order,
        })
    }

    pub fn retire_connection(&self) -> Result<usize, ChromiumHandleRouteError> {
        self.prepare_connection_retirement()
            .map(PreparedChromiumHandleConnectionRetirement::commit)
    }

    /// Locks the current connection-owned route set for a caller-controlled
    /// two-phase shutdown transaction. Dropping the prepared value preserves
    /// every route; committing retires the exact snapshotted set.
    pub fn prepare_connection_retirement(
        &self,
    ) -> Result<PreparedChromiumHandleConnectionRetirement<'_>, ChromiumHandleRouteError> {
        let state = self
            .state
            .lock()
            .map_err(|_| ChromiumHandleRouteError::StatePoisoned)?;
        let ids = state.routes.keys().copied().collect::<Vec<_>>();
        Ok(PreparedChromiumHandleConnectionRetirement { state, ids })
    }

    /// Prepares Handle invalidation caused by execution-world or Page-owner
    /// topology changes without depending on a browser topology crate.
    pub fn prepare_owner_retirement(
        &self,
        invalid_frames: &[ObjectId],
        invalid_pages: &[ObjectId],
        retire_all: bool,
    ) -> Result<PreparedChromiumHandleOwnerRetirement<'_>, ChromiumHandleRouteError> {
        let state = self
            .state
            .lock()
            .map_err(|_| ChromiumHandleRouteError::StatePoisoned)?;
        let invalid_frames = invalid_frames.iter().copied().collect::<BTreeSet<_>>();
        let invalid_pages = invalid_pages.iter().copied().collect::<BTreeSet<_>>();
        let object_ids = state
            .routes
            .iter()
            .filter_map(|(object_id, route)| {
                (retire_all
                    || invalid_frames.contains(&route.owner_frame)
                    || invalid_pages.contains(&route.owner_page))
                .then_some(*object_id)
            })
            .collect::<Vec<_>>();
        Ok(PreparedChromiumHandleOwnerRetirement { state, object_ids })
    }

    fn disposer(
        &self,
        object_id: ObjectId,
    ) -> Result<Option<Arc<dyn ChromiumHandleDisposer>>, ChromiumHandleRouteError> {
        let state = self
            .state
            .lock()
            .map_err(|_| ChromiumHandleRouteError::StatePoisoned)?;
        let route = state
            .routes
            .get(&object_id)
            .ok_or(ChromiumHandleRouteError::Missing { object_id })?;
        Ok(route.disposer.clone())
    }

    fn retire(
        &self,
        object_ids: &[ObjectId],
        reason: ChromiumHandleRetirementReason,
    ) -> Result<usize, ChromiumHandleRouteError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ChromiumHandleRouteError::StatePoisoned)?;
        Ok(retire_locked(&mut state, object_ids, reason))
    }
}

#[must_use = "dropping prepared Handle connection retirement preserves every route"]
pub struct PreparedChromiumHandleConnectionRetirement<'a> {
    state: MutexGuard<'a, ChromiumHandleRouteState>,
    ids: Vec<ObjectId>,
}

#[must_use = "dropping prepared Handle parent retirement preserves every route"]
pub struct PreparedChromiumHandleParentRetirement<'a> {
    state: MutexGuard<'a, ChromiumHandleRouteState>,
    object_ids: Vec<ObjectId>,
}

#[must_use = "commit borrowed parent retirement or drop to preserve routes"]
pub struct BorrowedChromiumHandleParentRetirement<'a> {
    state: MutexGuard<'a, ChromiumHandleRouteState>,
    object_ids: &'a [ObjectId],
}
impl BorrowedChromiumHandleParentRetirement<'_> {
    pub fn commit(mut self) -> usize {
        retire_locked(
            &mut self.state,
            self.object_ids,
            ChromiumHandleRetirementReason::Parent,
        )
    }
}

#[must_use = "dropping prepared Handle owner retirement preserves every route"]
pub struct PreparedChromiumHandleOwnerRetirement<'a> {
    state: MutexGuard<'a, ChromiumHandleRouteState>,
    object_ids: Vec<ObjectId>,
}

impl PreparedChromiumHandleOwnerRetirement<'_> {
    pub fn lifecycle_messages(&self) -> impl Iterator<Item = ControlMessage> + '_ {
        self.object_ids
            .iter()
            .copied()
            .map(|object_id| ControlMessage::Dispose { object_id })
    }

    pub fn include_removed(&mut self, removed: &[ObjectId]) {
        for object_id in removed {
            if self.state.routes.contains_key(object_id) && !self.object_ids.contains(object_id) {
                self.object_ids.push(*object_id);
            }
        }
    }

    pub fn commit(mut self) -> usize {
        retire_locked(
            &mut self.state,
            &self.object_ids,
            ChromiumHandleRetirementReason::Topology,
        )
    }
}

impl PreparedChromiumHandleConnectionRetirement<'_> {
    pub fn commit(mut self) -> usize {
        retire_locked(
            &mut self.state,
            &self.ids,
            ChromiumHandleRetirementReason::Connection,
        )
    }
}

impl PreparedChromiumHandleParentRetirement<'_> {
    pub fn commit(mut self) -> usize {
        retire_locked(
            &mut self.state,
            &self.object_ids,
            ChromiumHandleRetirementReason::Parent,
        )
    }
}

fn retire_locked(
    state: &mut ChromiumHandleRouteState,
    object_ids: &[ObjectId],
    reason: ChromiumHandleRetirementReason,
) -> usize {
    let mut removed = 0usize;
    let mut bytes = 0u64;
    for object_id in object_ids {
        if let Some(route) = state.routes.remove(object_id) {
            removed = removed.saturating_add(1);
            bytes = bytes.saturating_add(route.retained_bytes);
        }
    }
    state.metrics.current = state.metrics.current.saturating_sub(removed);
    state.metrics.retained_bytes = state.metrics.retained_bytes.saturating_sub(bytes);
    let removed_count = u64::try_from(removed).unwrap_or(u64::MAX);
    match reason {
        ChromiumHandleRetirementReason::Explicit => {
            state.metrics.explicitly_disposed = state
                .metrics
                .explicitly_disposed
                .saturating_add(removed_count);
        }
        ChromiumHandleRetirementReason::Parent => {
            state.metrics.parent_retired =
                state.metrics.parent_retired.saturating_add(removed_count);
        }
        ChromiumHandleRetirementReason::Topology => {
            state.metrics.topology_retired =
                state.metrics.topology_retired.saturating_add(removed_count);
        }
        ChromiumHandleRetirementReason::Connection => {
            state.metrics.connection_retired = state
                .metrics
                .connection_retired
                .saturating_add(removed_count);
        }
    }
    removed
}

enum ChromiumHandleRetirementReason {
    Explicit,
    Parent,
    Topology,
    Connection,
}

pub struct ChromiumHandleRouteReservation {
    routes: ChromiumHandleRoutes,
    lease: Option<ResourceLease>,
    active: bool,
}

impl ChromiumHandleRouteReservation {
    pub fn publish(
        self,
        object_id: ObjectId,
        spec: ChromiumHandleRouteSpec,
    ) -> Result<(), ChromiumHandleRouteError> {
        match self.publish_with(object_id, spec, || Ok::<(), Infallible>(())) {
            Ok(()) => Ok(()),
            Err(ChromiumHandlePublicationError::Route(error)) => Err(error),
            Err(ChromiumHandlePublicationError::Commit(never)) => match never {},
        }
    }

    pub fn publish_with<E>(
        mut self,
        object_id: ObjectId,
        spec: ChromiumHandleRouteSpec,
        commit: impl FnOnce() -> Result<(), E>,
    ) -> Result<(), ChromiumHandlePublicationError<E>> {
        if spec.native_id_bytes > self.routes.profile.max_native_id_bytes {
            return Err(ChromiumHandlePublicationError::Route(
                ChromiumHandleRouteError::NativeIdTooLong {
                    actual: spec.native_id_bytes,
                    limit: self.routes.profile.max_native_id_bytes,
                },
            ));
        }
        if spec.preview_bytes > self.routes.profile.max_preview_bytes {
            return Err(ChromiumHandlePublicationError::Route(
                ChromiumHandleRouteError::PreviewTooLong {
                    actual: spec.preview_bytes,
                    limit: self.routes.profile.max_preview_bytes,
                },
            ));
        }
        let retained_bytes = route_bytes(spec.native_id_bytes, spec.preview_bytes);
        let mut lease = self
            .lease
            .take()
            .expect("route reservation must own one lease");
        lease
            .try_resize(ResourceClass::BackendStateBytes, retained_bytes)
            .map_err(|error| {
                ChromiumHandlePublicationError::Route(ChromiumHandleRouteError::Resources(error))
            })?;
        let mut state = self.routes.state.lock().map_err(|_| {
            ChromiumHandlePublicationError::Route(ChromiumHandleRouteError::StatePoisoned)
        })?;
        if state.routes.contains_key(&object_id) {
            return Err(ChromiumHandlePublicationError::Route(
                ChromiumHandleRouteError::Duplicate { object_id },
            ));
        }
        commit().map_err(ChromiumHandlePublicationError::Commit)?;
        state.reserved = state.reserved.saturating_sub(1);
        state.metrics.reserved = state.reserved;
        state.routes.insert(
            object_id,
            ChromiumHandleRouteEntry {
                owner_frame: spec.owner_frame,
                owner_page: spec.owner_page,
                disposer: spec.disposer,
                payload: spec.payload,
                retained_bytes,
                _resource: route_resource(lease),
            },
        );
        state.metrics.current = state.metrics.current.saturating_add(1);
        state.metrics.peak = state.metrics.peak.max(state.metrics.current);
        state.metrics.retained_bytes = state.metrics.retained_bytes.saturating_add(retained_bytes);
        state.metrics.peak_retained_bytes = state
            .metrics
            .peak_retained_bytes
            .max(state.metrics.retained_bytes);
        state.metrics.created = state.metrics.created.saturating_add(1);
        self.active = false;
        Ok(())
    }
}

#[derive(Debug)]
pub enum ChromiumHandlePublicationError<E> {
    Route(ChromiumHandleRouteError),
    Commit(E),
}

impl<E: Display> Display for ChromiumHandlePublicationError<E> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Route(error) => Display::fmt(error, formatter),
            Self::Commit(error) => write!(formatter, "Handle publication commit failed: {error}"),
        }
    }
}

impl<E> Error for ChromiumHandlePublicationError<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Route(error) => Some(error),
            Self::Commit(error) => Some(error),
        }
    }
}

pub struct ChromiumHandleRouteBatchReservation {
    routes: ChromiumHandleRoutes,
    leases: Vec<ResourceLease>,
    count: usize,
    active: bool,
}

impl ChromiumHandleRouteBatchReservation {
    /// Return unused admission without reacquiring capacity. Growing a batch is
    /// forbidden; callers admit their maximum before creating native results.
    /// Trimming to zero is valid and does not publish an empty route batch.
    pub fn trim_to(&mut self, count: usize) -> Result<(), ChromiumHandleRouteError> {
        if count > self.count {
            return Err(ChromiumHandleRouteError::BatchLengthMismatch {
                reserved: self.count,
                actual: count,
            });
        }
        let mut state = self
            .routes
            .state
            .lock()
            .map_err(|_| ChromiumHandleRouteError::StatePoisoned)?;
        state.reserved = state.reserved.saturating_sub(self.count - count);
        state.metrics.reserved = state.reserved;
        self.count = count;
        self.leases.truncate(count);
        Ok(())
    }

    pub fn publish(
        self,
        entries: Vec<(ObjectId, ChromiumHandleRouteSpec)>,
    ) -> Result<(), ChromiumHandleRouteError> {
        match self.publish_with(entries, || Ok::<(), Infallible>(())) {
            Ok(()) => Ok(()),
            Err(ChromiumHandlePublicationError::Route(error)) => Err(error),
            Err(ChromiumHandlePublicationError::Commit(never)) => match never {},
        }
    }

    pub fn publish_with<E>(
        mut self,
        entries: Vec<(ObjectId, ChromiumHandleRouteSpec)>,
        commit: impl FnOnce() -> Result<(), E>,
    ) -> Result<(), ChromiumHandlePublicationError<E>> {
        if entries.len() != self.count {
            return Err(ChromiumHandlePublicationError::Route(
                ChromiumHandleRouteError::BatchLengthMismatch {
                    reserved: self.count,
                    actual: entries.len(),
                },
            ));
        }
        let mut ids = BTreeSet::new();
        let mut retained = Vec::with_capacity(entries.len());
        for (object_id, spec) in &entries {
            if !ids.insert(*object_id) {
                return Err(ChromiumHandlePublicationError::Route(
                    ChromiumHandleRouteError::Duplicate {
                        object_id: *object_id,
                    },
                ));
            }
            if spec.native_id_bytes > self.routes.profile.max_native_id_bytes {
                return Err(ChromiumHandlePublicationError::Route(
                    ChromiumHandleRouteError::NativeIdTooLong {
                        actual: spec.native_id_bytes,
                        limit: self.routes.profile.max_native_id_bytes,
                    },
                ));
            }
            if spec.preview_bytes > self.routes.profile.max_preview_bytes {
                return Err(ChromiumHandlePublicationError::Route(
                    ChromiumHandleRouteError::PreviewTooLong {
                        actual: spec.preview_bytes,
                        limit: self.routes.profile.max_preview_bytes,
                    },
                ));
            }
            retained.push(route_bytes(spec.native_id_bytes, spec.preview_bytes));
        }
        for (lease, retained_bytes) in self.leases.iter_mut().zip(&retained) {
            lease
                .try_resize(ResourceClass::BackendStateBytes, *retained_bytes)
                .map_err(|error| {
                    ChromiumHandlePublicationError::Route(ChromiumHandleRouteError::Resources(
                        error,
                    ))
                })?;
        }
        let mut state = self.routes.state.lock().map_err(|_| {
            ChromiumHandlePublicationError::Route(ChromiumHandleRouteError::StatePoisoned)
        })?;
        if let Some(object_id) = entries
            .iter()
            .map(|(object_id, _)| *object_id)
            .find(|object_id| state.routes.contains_key(object_id))
        {
            return Err(ChromiumHandlePublicationError::Route(
                ChromiumHandleRouteError::Duplicate { object_id },
            ));
        }
        commit().map_err(ChromiumHandlePublicationError::Commit)?;
        state.reserved = state.reserved.saturating_sub(self.count);
        state.metrics.reserved = state.reserved;
        let leases = std::mem::take(&mut self.leases);
        let mut retained_total = 0u64;
        for (((object_id, spec), retained_bytes), lease) in
            entries.into_iter().zip(retained).zip(leases)
        {
            retained_total = retained_total.saturating_add(retained_bytes);
            state.routes.insert(
                object_id,
                ChromiumHandleRouteEntry {
                    owner_frame: spec.owner_frame,
                    owner_page: spec.owner_page,
                    disposer: spec.disposer,
                    payload: spec.payload,
                    retained_bytes,
                    _resource: route_resource(lease),
                },
            );
        }
        let created = u64::try_from(self.count).unwrap_or(u64::MAX);
        state.metrics.current = state.metrics.current.saturating_add(self.count);
        state.metrics.peak = state.metrics.peak.max(state.metrics.current);
        state.metrics.retained_bytes = state.metrics.retained_bytes.saturating_add(retained_total);
        state.metrics.peak_retained_bytes = state
            .metrics
            .peak_retained_bytes
            .max(state.metrics.retained_bytes);
        state.metrics.created = state.metrics.created.saturating_add(created);
        self.active = false;
        Ok(())
    }
}

impl Drop for ChromiumHandleRouteBatchReservation {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        if let Ok(mut state) = self.routes.state.lock() {
            state.reserved = state.reserved.saturating_sub(self.count);
            state.metrics.reserved = state.reserved;
        }
    }
}

impl Drop for ChromiumHandleRouteReservation {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        if let Ok(mut state) = self.routes.state.lock() {
            state.reserved = state.reserved.saturating_sub(1);
            state.metrics.reserved = state.reserved;
        }
    }
}

pub struct ChromiumHandleRouteSnapshot<T> {
    owner_frame: ObjectId,
    owner_page: ObjectId,
    payload: Arc<T>,
}

impl<T> ChromiumHandleRouteSnapshot<T> {
    pub const fn owner_frame(&self) -> ObjectId {
        self.owner_frame
    }

    pub const fn owner_page(&self) -> ObjectId {
        self.owner_page
    }

    pub fn payload(&self) -> Arc<T> {
        Arc::clone(&self.payload)
    }
}

#[derive(Debug)]
pub enum ChromiumHandleRouteError {
    EmptyBatch,
    BatchLengthMismatch { reserved: usize, actual: usize },
    CapacityExceeded { limit: usize },
    Duplicate { object_id: ObjectId },
    Missing { object_id: ObjectId },
    NativeIdTooLong { actual: usize, limit: usize },
    PreviewTooLong { actual: usize, limit: usize },
    PayloadTypeMismatch,
    StatePoisoned,
    Resources(ResourceError),
}

impl Display for ChromiumHandleRouteError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBatch => formatter.write_str("Handle route batch must not be empty"),
            Self::BatchLengthMismatch { reserved, actual } => write!(
                formatter,
                "Handle route batch reserved {reserved} entries but received {actual}"
            ),
            Self::CapacityExceeded { limit } => {
                write!(formatter, "live Handle route limit {limit} was reached")
            }
            Self::Duplicate { object_id } => {
                write!(formatter, "Handle `{object_id}` already has a route")
            }
            Self::Missing { object_id } => {
                write!(formatter, "Handle `{object_id}` has no native route")
            }
            Self::NativeIdTooLong { actual, limit } => write!(
                formatter,
                "native Handle ID has {actual} bytes, exceeding limit {limit}"
            ),
            Self::PreviewTooLong { actual, limit } => write!(
                formatter,
                "Handle preview has {actual} bytes, exceeding limit {limit}"
            ),
            Self::PayloadTypeMismatch => {
                formatter.write_str("Handle route payload has a different plugin type")
            }
            Self::StatePoisoned => formatter.write_str("Handle route registry is poisoned"),
            Self::Resources(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ChromiumHandleRouteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resources(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct ChromiumHandleDisposeCommandService {
    runtime: ControlledLifecycleHandle,
    routes: ChromiumHandleRoutes,
}

impl ChromiumHandleDisposeCommandService {
    pub const fn new(runtime: ControlledLifecycleHandle, routes: ChromiumHandleRoutes) -> Self {
        Self { runtime, routes }
    }

    async fn dispose(
        &self,
        object_id: ObjectId,
        metadata: CallMetadata,
    ) -> Result<(), OperationError<HandleLifecycleErrorDetails>> {
        if metadata != CallMetadata::default() {
            return Err(OperationError::new(
                FailureKind::ProtocolViolation,
                "Handle.dispose does not accept timeout or cancellation metadata",
                None,
            ));
        }
        match self.runtime.is_object_live(object_id) {
            Ok(false) => return Ok(()),
            Ok(true) => {}
            Err(error) => return Err(map_runtime_error(error)),
        }
        let prepared = self
            .runtime
            .prepare_object_disposal(object_id)
            .map_err(map_runtime_error)?;
        let disposer = self.routes.disposer(object_id).map_err(map_route_error)?;
        if let Some(disposer) = disposer {
            disposer.dispose().await?;
        }
        prepared.commit().map_err(map_runtime_error)?;
        self.routes
            .retire(&[object_id], ChromiumHandleRetirementReason::Explicit)
            .map_err(map_route_error)?;
        Ok(())
    }
}

impl Service<Command<HandleDispose>> for ChromiumHandleDisposeCommandService {
    type Response = CommandResult<HandleDispose>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<HandleDispose>) -> Self::Future<'_> {
        let (call_id, object_id, metadata, ()) = command.into_parts();
        Box::pin(async move {
            Ok(CommandResult::new(
                call_id,
                self.dispose(object_id, metadata).await,
            ))
        })
    }
}

type HandleDisposeFutureOutput = Result<CommandResult<HandleDispose>, Infallible>;
type BoxHandleDisposeFuture =
    Pin<Box<dyn Future<Output = HandleDisposeFutureOutput> + Send + 'static>>;

pub struct ContinuableHandleDisposeFuture<Reservation>
where
    Reservation: CleanupReservation,
{
    inner: Option<BoxHandleDisposeFuture>,
    reservation: Option<Reservation>,
    started: bool,
}

impl<Reservation> Unpin for ContinuableHandleDisposeFuture<Reservation> where
    Reservation: CleanupReservation
{
}

impl<Reservation> Future for ContinuableHandleDisposeFuture<Reservation>
where
    Reservation: CleanupReservation,
{
    type Output = HandleDisposeFutureOutput;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.started = true;
        let outcome = self
            .inner
            .as_mut()
            .expect("completed Handle disposal future must not be polled again")
            .as_mut()
            .poll(context);
        if outcome.is_ready() {
            self.inner = None;
            self.reservation = None;
        }
        outcome
    }
}

impl<Reservation> Drop for ContinuableHandleDisposeFuture<Reservation>
where
    Reservation: CleanupReservation,
{
    fn drop(&mut self) {
        if !self.started {
            return;
        }
        let Some(inner) = self.inner.take() else {
            return;
        };
        let Some(reservation) = self.reservation.take() else {
            return;
        };
        reservation.submit(Box::pin(async move {
            match inner.await {
                Ok(result) if result.outcome().is_ok() => Ok(()),
                Ok(_) => Err(CleanupFailure::new(
                    "abandoned Handle.dispose continuation failed",
                )),
                Err(never) => match never {},
            }
        }));
    }
}

#[derive(Clone)]
pub struct ContinuableHandleDisposeService<Continuation> {
    service: Arc<ChromiumHandleDisposeCommandService>,
    continuation: Continuation,
}

impl<Continuation> ContinuableHandleDisposeService<Continuation> {
    pub const fn new(
        service: Arc<ChromiumHandleDisposeCommandService>,
        continuation: Continuation,
    ) -> Self {
        Self {
            service,
            continuation,
        }
    }

    pub const fn service(&self) -> &Arc<ChromiumHandleDisposeCommandService> {
        &self.service
    }
}

impl<Continuation> Service<Command<HandleDispose>> for ContinuableHandleDisposeService<Continuation>
where
    Continuation: ReservedCleanupExecutor,
{
    type Response = CommandResult<HandleDispose>;
    type Error = Infallible;
    type Future<'a>
        = ContinuableHandleDisposeFuture<Continuation::Reservation>
    where
        Self: 'a;

    fn call(&self, command: Command<HandleDispose>) -> Self::Future<'_> {
        let call_id = command.call_id();
        let reservation = match self.continuation.try_reserve() {
            Ok(reservation) => reservation,
            Err(error) => {
                return ContinuableHandleDisposeFuture {
                    inner: Some(Box::pin(std::future::ready(Ok(continuation_failure(
                        call_id, error,
                    ))))),
                    reservation: None,
                    started: false,
                };
            }
        };
        let service = Arc::clone(&self.service);
        ContinuableHandleDisposeFuture {
            inner: Some(Box::pin(async move { service.call(command).await })),
            reservation: Some(reservation),
            started: false,
        }
    }
}

pub type ChromiumHandleLifecycleHandlers<Continuation> =
    HandleLifecycleServerHandlers<ContinuableHandleDisposeService<Continuation>>;

pub struct ChromiumHandleLifecycleBindings<Continuation> {
    pub runtime: ControlledLifecycleHandle,
    pub continuation: Continuation,
}

pub struct ChromiumHandleLifecycleInstallation<Continuation>
where
    Continuation: ReservedCleanupExecutor,
{
    routes: ChromiumHandleRoutes,
    raw_dispose: Arc<ChromiumHandleDisposeCommandService>,
    dispose: Arc<ContinuableHandleDisposeService<Continuation>>,
    protocol: Arc<WireProtocol<HandleLifecycleCodec>>,
}

impl<Continuation> ChromiumHandleLifecycleInstallation<Continuation>
where
    Continuation: ReservedCleanupExecutor,
{
    pub fn install(
        bindings: ChromiumHandleLifecycleBindings<Continuation>,
        profile: ChromiumHandleLifecycleProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumHandleLifecycleInstallError> {
        let routes =
            ChromiumHandleRoutes::new_with_production_resources(profile.routes(), resources)?;
        let protocol = profile
            .unbudgeted_protocol()
            .with_production_resources(resources)?;
        let raw_dispose = Arc::new(ChromiumHandleDisposeCommandService::new(
            bindings.runtime,
            routes.clone(),
        ));
        Ok(Self {
            routes,
            raw_dispose: Arc::clone(&raw_dispose),
            dispose: Arc::new(ContinuableHandleDisposeService::new(
                raw_dispose,
                bindings.continuation,
            )),
            protocol: Arc::new(protocol),
        })
    }

    pub fn routes(&self) -> ChromiumHandleRoutes {
        self.routes.clone()
    }

    pub fn dispose_service(&self) -> Arc<ContinuableHandleDisposeService<Continuation>> {
        Arc::clone(&self.dispose)
    }

    /// Returns the backend disposal service before caller-selected
    /// continuation or remote ordering policies are applied.
    pub fn raw_dispose_service(&self) -> Arc<ChromiumHandleDisposeCommandService> {
        Arc::clone(&self.raw_dispose)
    }

    pub fn protocol(&self) -> Arc<WireProtocol<HandleLifecycleCodec>> {
        Arc::clone(&self.protocol)
    }

    pub fn handlers(&self) -> ChromiumHandleLifecycleHandlers<Continuation> {
        handle_lifecycle_server_handlers(
            Arc::clone(&self.protocol),
            HandleLifecycleServerServices {
                handle_dispose: Arc::clone(&self.dispose),
            },
        )
    }
}

fn continuation_failure(
    call_id: CallId,
    error: CleanupSubmitError,
) -> CommandResult<HandleDispose> {
    let kind = match error {
        CleanupSubmitError::CapacityExceeded { .. } => FailureKind::ResourceExhausted,
        CleanupSubmitError::Unavailable | CleanupSubmitError::StatePoisoned => FailureKind::Cleanup,
    };
    CommandResult::new(
        call_id,
        Err(OperationError::new(
            kind,
            format!("Handle.dispose continuation admission failed: {error}"),
            Some(HandleLifecycleErrorDetails::Cleanup {
                primary: "Handle.dispose was rejected before native I/O".to_owned(),
                cleanup: error.to_string(),
            }),
        )),
    )
}

fn map_runtime_error(error: RuntimeStateError) -> OperationError<HandleLifecycleErrorDetails> {
    let kind = match error {
        RuntimeStateError::TransportClosed => FailureKind::TransportClosed,
        RuntimeStateError::ObjectGraph(_) => FailureKind::TargetClosed,
        RuntimeStateError::Resource(_) => FailureKind::ResourceExhausted,
        _ => FailureKind::Cleanup,
    };
    OperationError::new(
        kind,
        format!("runtime could not dispose Handle: {error}"),
        Some(HandleLifecycleErrorDetails::Backend(error.to_string())),
    )
}

fn map_route_error(error: ChromiumHandleRouteError) -> OperationError<HandleLifecycleErrorDetails> {
    let kind = match error {
        ChromiumHandleRouteError::CapacityExceeded { .. }
        | ChromiumHandleRouteError::NativeIdTooLong { .. }
        | ChromiumHandleRouteError::PreviewTooLong { .. }
        | ChromiumHandleRouteError::Resources(_) => FailureKind::ResourceExhausted,
        ChromiumHandleRouteError::Missing { .. } => FailureKind::TargetClosed,
        ChromiumHandleRouteError::Duplicate { .. }
        | ChromiumHandleRouteError::EmptyBatch
        | ChromiumHandleRouteError::BatchLengthMismatch { .. }
        | ChromiumHandleRouteError::PayloadTypeMismatch => FailureKind::ProtocolViolation,
        ChromiumHandleRouteError::StatePoisoned => FailureKind::Cleanup,
    };
    OperationError::new(
        kind,
        error.to_string(),
        Some(HandleLifecycleErrorDetails::Backend(error.to_string())),
    )
}

/// Exposed so capability registries can assert that this leaf owns exactly the
/// independently negotiated Handle-lifecycle surface.
pub const fn capability_id() -> pwright_core::CapabilityId {
    HANDLE_LIFECYCLE_CAPABILITY_ID
}
