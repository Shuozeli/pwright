//! Chromium retained-Handle evaluation composed over the independent Handle
//! lifecycle leaf.
//!
//! The crate implements the complete `pwright.handle_evaluation` 1.2 command
//! family. It owns bounded Frame execution-world routes but delegates every
//! retained Handle route and explicit disposal to
//! `pwright-composition-chromium-handle-lifecycle`.

mod element_adoption;
#[cfg(feature = "frame-value")]
mod frame_value;
#[cfg(feature = "handle-value")]
mod handle_value;
#[cfg(feature = "native-creation")]
pub mod native_creation;
#[cfg(feature = "handle-value")]
mod owner_frame;
#[cfg(feature = "retained-reference")]
mod retained_reference;
#[cfg(feature = "retained-reference")]
pub use retained_reference::ChromiumRetainedElementSource;

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

pub use element_adoption::{
    ContinuableAdoptedElementHandleBatch, ContinuableAdoptedElementHandleReleaseFuture,
    ContinuableElementHandleAdoptionFuture, ContinuableElementHandleAdoptionService,
};
pub use pwright_backend_cdp_evaluation::{
    CdpEvaluationLimits, CdpEvaluationProductionProfile, CdpEvaluationService,
    CdpEvaluationServiceFactory,
};
pub use pwright_backend_cdp_handle_evaluation::{
    CdpAdoptedElementHandle, CdpAdoptedElementHandleBatch, CdpElementHandleAdoptionBatchRequest,
    CdpElementHandleAdoptionBatchService, CdpElementHandleAdoptionInstallError,
    CdpElementHandleAdoptionProductionProfile, CdpElementHandleAdoptionRequest,
    CdpElementHandleAdoptionService, CdpElementHandleAdoptionSetupError, CdpEvaluatedHandle,
    CdpHandleEvaluationLimits, CdpHandleEvaluationProgram, CdpHandleEvaluationRequest,
    CdpHandleEvaluationService, CdpHandleKind, CdpHandlePropertiesService,
    CdpHandlePropertyLimitError, CdpHandlePropertyLimits, CdpHandleValueEvaluationService,
};
use pwright_cap_handle_evaluation::{
    EvaluateHandle, EvaluateHandleParams, GetPropertiesOptions, GetPropertyParams, HandleEvaluate,
    HandleEvaluateHandle, HandleEvaluateParams, HandleEvaluationErrorDetails, HandleGetProperties,
    HandleGetProperty, HandleInitializer, HandleJsonValue, HandlePropertiesResult,
    HandlePropertyInitializer, HandleValueResult, JsonValueOptions,
};
use pwright_cap_handle_lifecycle::{HandleDispose, HandleLifecycleErrorDetails};
use pwright_cdp::CdpClient;
use pwright_codec_handle_evaluation::HandleEvaluationCodec;
pub use pwright_codec_handle_evaluation::HandleEvaluationCodecLimits;
use pwright_composition_chromium_handle_lifecycle::ChromiumHandleRouteReservation;
pub use pwright_composition_chromium_handle_lifecycle::{
    ChromiumHandleDisposeCommandService, ChromiumHandleDisposer, ChromiumHandleLifecycleBindings,
    ChromiumHandleLifecycleHandlers, ChromiumHandleLifecycleInstallError,
    ChromiumHandleLifecycleInstallation, ChromiumHandleLifecycleProfile,
    ChromiumHandlePublicationError, ChromiumHandleRouteError, ChromiumHandleRouteProfile,
    ChromiumHandleRouteSnapshot, ChromiumHandleRouteSpec, ChromiumHandleRoutes,
    ContinuableHandleDisposeService, FrameLimits, HandleLifecycleCodecLimits,
    PreparedChromiumHandleConnectionRetirement, PreparedChromiumHandleOwnerRetirement,
    PreparedChromiumHandleParentRetirement,
};
use pwright_core::{
    CallId, CallMetadata, Command, CommandResult, FailureKind, ObjectId, ObjectIdSource,
    OperationError, ProductionResourceError, ProductionResourceLedger, ProtocolVersion,
    ResourceError, ResourceLease, ResourceRequirements, Service,
};
use pwright_runtime::{
    CleanupFailure, CleanupReservation, CleanupSubmitError, ControlledLifecycleHandle,
    ObjectCreationSpec, ReservedCleanupExecutor, RuntimeStateError,
};
use pwright_server_handle_evaluation::{
    HandleEvaluationServerHandlers, HandleEvaluationServerServices,
    handle_evaluation_server_handlers,
};
use pwright_transport::WireProtocol;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumHandleEvaluationBackendProfile {
    evaluation: CdpEvaluationProductionProfile,
    native_id_bytes: usize,
    preview_bytes: usize,
    properties: CdpHandlePropertyLimits,
}

impl ChromiumHandleEvaluationBackendProfile {
    pub const fn new(
        evaluation: CdpEvaluationProductionProfile,
        native_id_bytes: usize,
        preview_bytes: usize,
        properties: CdpHandlePropertyLimits,
    ) -> Self {
        Self {
            evaluation,
            native_id_bytes,
            preview_bytes,
            properties,
        }
    }

    pub const fn evaluation(self) -> CdpEvaluationProductionProfile {
        self.evaluation
    }

    pub const fn native_id_bytes(self) -> usize {
        self.native_id_bytes
    }

    pub const fn preview_bytes(self) -> usize {
        self.preview_bytes
    }

    pub const fn properties(self) -> CdpHandlePropertyLimits {
        self.properties
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumHandleEvaluationProfile {
    backend: ChromiumHandleEvaluationBackendProfile,
    codec: HandleEvaluationCodecLimits,
    send_limits: FrameLimits,
    receive_limits: FrameLimits,
    lifecycle: ChromiumHandleLifecycleProfile,
}

impl ChromiumHandleEvaluationProfile {
    pub const VERSION: ProtocolVersion = ProtocolVersion::new(1, 2);

    pub const fn new(
        backend: ChromiumHandleEvaluationBackendProfile,
        codec: HandleEvaluationCodecLimits,
        send_limits: FrameLimits,
        receive_limits: FrameLimits,
        lifecycle: ChromiumHandleLifecycleProfile,
    ) -> Self {
        Self {
            backend,
            codec,
            send_limits,
            receive_limits,
            lifecycle,
        }
    }

    pub const fn backend(self) -> ChromiumHandleEvaluationBackendProfile {
        self.backend
    }

    pub const fn native_id_bytes(self) -> usize {
        self.backend.native_id_bytes()
    }

    pub const fn preview_bytes(self) -> usize {
        self.backend.preview_bytes()
    }

    pub const fn properties(self) -> CdpHandlePropertyLimits {
        self.backend.properties()
    }

    pub const fn lifecycle(self) -> ChromiumHandleLifecycleProfile {
        self.lifecycle
    }

    pub fn requirements(self) -> ResourceRequirements {
        self.backend
            .evaluation()
            .requirements()
            .combine_concurrent(self.lifecycle.requirements())
            .combine_concurrent(
                self.unbudgeted_protocol()
                    .production_resource_requirements(),
            )
    }

    pub fn requirements_with_element_adoption(
        self,
        adoption: CdpElementHandleAdoptionProductionProfile,
    ) -> ResourceRequirements {
        self.requirements()
            .combine_concurrent(adoption.requirements())
    }

    fn unbudgeted_protocol(self) -> WireProtocol<HandleEvaluationCodec> {
        WireProtocol::new_exact_directional(
            HandleEvaluationCodec::new(self.codec),
            Self::VERSION,
            self.send_limits,
            self.receive_limits,
        )
    }

    fn validate(self) -> Result<Self, ChromiumHandleEvaluationSetupError> {
        if self.backend.evaluation().max_concurrent_operations() == 0 {
            return Err(ChromiumHandleEvaluationSetupError::ZeroOperationCapacity);
        }
        if self.backend.evaluation().max_context_routes() == 0 {
            return Err(ChromiumHandleEvaluationSetupError::ZeroFrameRouteCapacity);
        }
        if self.backend.evaluation().max_native_id_bytes() == 0 {
            return Err(ChromiumHandleEvaluationSetupError::ZeroFrameRouteBytes);
        }
        if self.backend.native_id_bytes() == 0 {
            return Err(ChromiumHandleEvaluationSetupError::ZeroHandleNativeIdBytes);
        }
        let lifecycle = self.lifecycle.routes();
        if lifecycle.max_native_id_bytes() != self.backend.native_id_bytes()
            || lifecycle.max_preview_bytes() != self.backend.preview_bytes()
        {
            return Err(ChromiumHandleEvaluationSetupError::LifecycleBoundsMismatch);
        }
        if self.backend.evaluation().resource_scope() == lifecycle.scope() {
            return Err(ChromiumHandleEvaluationSetupError::DuplicateResourceScope);
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChromiumHandleEvaluationSetupError {
    ZeroOperationCapacity,
    ZeroFrameRouteCapacity,
    ZeroFrameRouteBytes,
    ZeroHandleNativeIdBytes,
    LifecycleBoundsMismatch,
    DuplicateResourceScope,
    AdoptionBoundsMismatch,
    DuplicateAdoptionResourceScope,
}

impl Display for ChromiumHandleEvaluationSetupError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroOperationCapacity => {
                formatter.write_str("Handle evaluation operation capacity must be non-zero")
            }
            Self::ZeroFrameRouteCapacity => {
                formatter.write_str("Handle evaluation Frame-route capacity must be non-zero")
            }
            Self::ZeroFrameRouteBytes => {
                formatter.write_str("Handle evaluation Frame-route byte bound must be non-zero")
            }
            Self::ZeroHandleNativeIdBytes => {
                formatter.write_str("Handle native-object ID bound must be non-zero")
            }
            Self::LifecycleBoundsMismatch => formatter.write_str(
                "Handle evaluation native-ID/preview bounds do not match lifecycle routes",
            ),
            Self::DuplicateResourceScope => formatter.write_str(
                "Handle evaluation and lifecycle routes require distinct resource scopes",
            ),
            Self::AdoptionBoundsMismatch => formatter.write_str(
                "ElementHandle adoption native-ID/preview bounds do not match Handle evaluation",
            ),
            Self::DuplicateAdoptionResourceScope => formatter.write_str(
                "ElementHandle adoption requires a resource scope distinct from evaluation and lifecycle",
            ),
        }
    }
}

impl Error for ChromiumHandleEvaluationSetupError {}

#[derive(Debug)]
pub enum ChromiumHandleEvaluationInstallError {
    Setup(ChromiumHandleEvaluationSetupError),
    Lifecycle(ChromiumHandleLifecycleInstallError),
    Adoption(CdpElementHandleAdoptionInstallError),
    Resources(ProductionResourceError),
    RouteResources(ResourceError),
}

impl Display for ChromiumHandleEvaluationInstallError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Setup(error) => Display::fmt(error, formatter),
            Self::Lifecycle(error) => Display::fmt(error, formatter),
            Self::Adoption(error) => Display::fmt(error, formatter),
            Self::Resources(error) => Display::fmt(error, formatter),
            Self::RouteResources(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ChromiumHandleEvaluationInstallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Setup(error) => Some(error),
            Self::Lifecycle(error) => Some(error),
            Self::Adoption(error) => Some(error),
            Self::Resources(error) => Some(error),
            Self::RouteResources(error) => Some(error),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChromiumHandleEvaluationRouteError {
    CapacityExceeded { limit: usize },
    Duplicate { frame_id: ObjectId },
    InvalidExecutionContextId { execution_context_id: i64 },
    EmptyContextId,
    ContextIdTooLong { actual: usize, limit: usize },
    StatePoisoned,
}

impl Display for ChromiumHandleEvaluationRouteError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded { limit } => {
                write!(
                    formatter,
                    "Handle evaluation Frame-route limit {limit} was reached"
                )
            }
            Self::Duplicate { frame_id } => {
                write!(
                    formatter,
                    "Frame `{frame_id}` already has a Handle evaluation route"
                )
            }
            Self::InvalidExecutionContextId {
                execution_context_id,
            } => write!(
                formatter,
                "Chromium execution-context ID must be positive, got {execution_context_id}"
            ),
            Self::EmptyContextId => formatter.write_str("execution-context ID must not be empty"),
            Self::ContextIdTooLong { actual, limit } => write!(
                formatter,
                "execution-context ID has {actual} bytes, exceeding limit {limit}"
            ),
            Self::StatePoisoned => formatter.write_str("Handle evaluation routes are poisoned"),
        }
    }
}

impl Error for ChromiumHandleEvaluationRouteError {}

#[derive(Clone)]
pub struct ChromiumHandleEvaluationRoute {
    page_id: ObjectId,
    service: CdpEvaluationService,
}

impl ChromiumHandleEvaluationRoute {
    pub fn from_service(
        page_id: ObjectId,
        service: CdpEvaluationService,
    ) -> Result<Self, ChromiumHandleEvaluationRouteError> {
        let execution_context_id = service.execution_context_id().unwrap_or_default();
        if execution_context_id <= 0 {
            return Err(
                ChromiumHandleEvaluationRouteError::InvalidExecutionContextId {
                    execution_context_id,
                },
            );
        }
        let Some(unique_context_id) = service.unique_context_id() else {
            return Err(ChromiumHandleEvaluationRouteError::EmptyContextId);
        };
        if unique_context_id.is_empty() {
            return Err(ChromiumHandleEvaluationRouteError::EmptyContextId);
        }
        Ok(Self { page_id, service })
    }

    pub const fn page_id(&self) -> ObjectId {
        self.page_id
    }

    pub const fn service(&self) -> &CdpEvaluationService {
        &self.service
    }
}

impl ChromiumHandleEvaluationCommandService {
    /// Backend world identity for exact native-lifetime composition. This does
    /// not expose a RemoteObject ID or install another route registry.
    pub fn handle_world_identity(
        &self,
        object_id: ObjectId,
    ) -> Result<
        Option<ChromiumHandleEvaluationRouteIdentity>,
        OperationError<HandleEvaluationErrorDetails>,
    > {
        self.handle_routes
            .snapshot::<ChromiumEvaluatedHandleRoute>(object_id)
            .map_err(map_route_error)
            .map(|route| {
                route.map(|route| {
                    let payload = route.payload();
                    ChromiumHandleEvaluationRouteIdentity {
                        page_id: route.owner_page(),
                        execution_context_id: payload.source_execution_context_id,
                        unique_context_id: payload.source_unique_context_id.clone(),
                    }
                })
            })
    }
}

/// Injected source of the current topology-owned Frame/world routes.
///
/// Standalone installations use [`ChromiumHandleEvaluationRoutes`]. A larger
/// recipe can implement this trait over its own atomic topology snapshot so
/// the Handle leaf does not retain a second, drifting copy.
pub trait ChromiumHandleEvaluationRouteResolver: Send + Sync {
    fn resolve(
        &self,
        frame_id: ObjectId,
    ) -> Result<Option<ChromiumHandleEvaluationRoute>, ChromiumHandleEvaluationRouteError>;
}

/// One complete Frame-to-world binding supplied by a topology owner.
///
/// The Handle leaf deliberately does not discover Frames or execution worlds.
/// A larger recipe may instead build a complete snapshot and atomically
/// replace the leaf's routes through [`ChromiumHandleEvaluationRoutes::replace_all`].
#[derive(Clone)]
pub struct ChromiumHandleEvaluationRouteSpec {
    frame_id: ObjectId,
    page_id: ObjectId,
    client: Arc<dyn CdpClient>,
    execution_context_id: i64,
    unique_context_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChromiumHandleEvaluationRouteIdentity {
    page_id: ObjectId,
    execution_context_id: i64,
    unique_context_id: Arc<str>,
}

impl ChromiumHandleEvaluationRouteIdentity {
    pub const fn page_id(&self) -> ObjectId {
        self.page_id
    }

    pub const fn execution_context_id(&self) -> i64 {
        self.execution_context_id
    }

    pub fn unique_context_id(&self) -> &str {
        &self.unique_context_id
    }
}

impl ChromiumHandleEvaluationRouteSpec {
    pub fn new(
        frame_id: ObjectId,
        page_id: ObjectId,
        client: Arc<dyn CdpClient>,
        execution_context_id: i64,
        unique_context_id: impl Into<String>,
    ) -> Self {
        Self {
            frame_id,
            page_id,
            client,
            execution_context_id,
            unique_context_id: unique_context_id.into(),
        }
    }

    pub const fn frame_id(&self) -> ObjectId {
        self.frame_id
    }

    pub const fn page_id(&self) -> ObjectId {
        self.page_id
    }

    pub const fn execution_context_id(&self) -> i64 {
        self.execution_context_id
    }

    pub fn unique_context_id(&self) -> &str {
        &self.unique_context_id
    }
}

#[derive(Clone)]
pub struct ChromiumHandleEvaluationRoutes {
    routes: Arc<Mutex<BTreeMap<ObjectId, ChromiumHandleEvaluationRoute>>>,
    factory: CdpEvaluationServiceFactory,
    max_routes: usize,
    max_native_id_bytes: usize,
    _route_resources: Arc<Option<ResourceLease>>,
}

impl ChromiumHandleEvaluationRoutes {
    /// Installs only the bounded Frame/world route owner.
    ///
    /// This lower-level constructor exists for composition roots that already
    /// own topology discovery and need to inject the resulting snapshots into
    /// the independent Handle command leaf.
    pub fn new_with_production_resources(
        profile: CdpEvaluationProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumHandleEvaluationInstallError> {
        let factory =
            CdpEvaluationServiceFactory::new_with_production_resources(profile, resources)
                .map_err(ChromiumHandleEvaluationInstallError::Resources)?;
        Self::from_factory(factory)
    }

    fn from_factory(
        factory: CdpEvaluationServiceFactory,
    ) -> Result<Self, ChromiumHandleEvaluationInstallError> {
        let profile = factory.profile();
        let resources = factory
            .reserve_context_routes()
            .map_err(ChromiumHandleEvaluationInstallError::RouteResources)?;
        Ok(Self {
            routes: Arc::new(Mutex::new(BTreeMap::new())),
            factory,
            max_routes: profile.max_context_routes(),
            max_native_id_bytes: profile.max_native_id_bytes(),
            _route_resources: Arc::new(resources),
        })
    }

    pub fn bind(
        &self,
        frame_id: ObjectId,
        page_id: ObjectId,
        client: Arc<dyn CdpClient>,
        execution_context_id: i64,
        unique_context_id: impl Into<String>,
    ) -> Result<(), ChromiumHandleEvaluationRouteError> {
        let spec = ChromiumHandleEvaluationRouteSpec::new(
            frame_id,
            page_id,
            client,
            execution_context_id,
            unique_context_id,
        );
        let route = self.route_from_spec(spec)?;
        let mut routes = self
            .routes
            .lock()
            .map_err(|_| ChromiumHandleEvaluationRouteError::StatePoisoned)?;
        if routes.contains_key(&frame_id) {
            return Err(ChromiumHandleEvaluationRouteError::Duplicate { frame_id });
        }
        if routes.len() >= self.max_routes {
            return Err(ChromiumHandleEvaluationRouteError::CapacityExceeded {
                limit: self.max_routes,
            });
        }
        routes.insert(frame_id, route);
        Ok(())
    }

    /// Atomically replaces the complete topology-owned Frame/world route set.
    ///
    /// Every candidate is validated and materialized before the live route
    /// lock is acquired. Capacity, duplicate, or identity failure therefore
    /// preserves the previously published snapshot without an unbound gap.
    pub fn replace_all(
        &self,
        specs: impl IntoIterator<Item = ChromiumHandleEvaluationRouteSpec>,
    ) -> Result<(), ChromiumHandleEvaluationRouteError> {
        let specs = specs.into_iter().collect::<Vec<_>>();
        if specs.len() > self.max_routes {
            return Err(ChromiumHandleEvaluationRouteError::CapacityExceeded {
                limit: self.max_routes,
            });
        }
        let mut next = BTreeMap::new();
        for spec in specs {
            let frame_id = spec.frame_id;
            let route = self.route_from_spec(spec)?;
            if next.insert(frame_id, route).is_some() {
                return Err(ChromiumHandleEvaluationRouteError::Duplicate { frame_id });
            }
        }
        let mut current = self
            .routes
            .lock()
            .map_err(|_| ChromiumHandleEvaluationRouteError::StatePoisoned)?;
        *current = next;
        Ok(())
    }

    pub fn unbind(&self, frame_id: ObjectId) -> Result<bool, ChromiumHandleEvaluationRouteError> {
        self.routes
            .lock()
            .map(|mut routes| routes.remove(&frame_id).is_some())
            .map_err(|_| ChromiumHandleEvaluationRouteError::StatePoisoned)
    }

    pub fn len(&self) -> Result<usize, ChromiumHandleEvaluationRouteError> {
        self.routes
            .lock()
            .map(|routes| routes.len())
            .map_err(|_| ChromiumHandleEvaluationRouteError::StatePoisoned)
    }

    pub fn is_empty(&self) -> Result<bool, ChromiumHandleEvaluationRouteError> {
        self.routes
            .lock()
            .map(|routes| routes.is_empty())
            .map_err(|_| ChromiumHandleEvaluationRouteError::StatePoisoned)
    }

    pub fn contains(&self, frame_id: ObjectId) -> Result<bool, ChromiumHandleEvaluationRouteError> {
        self.routes
            .lock()
            .map(|routes| routes.contains_key(&frame_id))
            .map_err(|_| ChromiumHandleEvaluationRouteError::StatePoisoned)
    }

    pub fn identity(
        &self,
        frame_id: ObjectId,
    ) -> Result<Option<ChromiumHandleEvaluationRouteIdentity>, ChromiumHandleEvaluationRouteError>
    {
        self.routes
            .lock()
            .map(|routes| {
                routes
                    .get(&frame_id)
                    .map(|route| ChromiumHandleEvaluationRouteIdentity {
                        page_id: route.page_id,
                        execution_context_id: route
                            .service
                            .execution_context_id()
                            .expect("Handle leaf routes always have a numeric context identity"),
                        unique_context_id: route
                            .service
                            .shared_unique_context_id()
                            .expect("Handle leaf routes always have a stable context identity"),
                    })
            })
            .map_err(|_| ChromiumHandleEvaluationRouteError::StatePoisoned)
    }

    fn route_from_spec(
        &self,
        spec: ChromiumHandleEvaluationRouteSpec,
    ) -> Result<ChromiumHandleEvaluationRoute, ChromiumHandleEvaluationRouteError> {
        if spec.execution_context_id <= 0 {
            return Err(
                ChromiumHandleEvaluationRouteError::InvalidExecutionContextId {
                    execution_context_id: spec.execution_context_id,
                },
            );
        }
        if spec.unique_context_id.is_empty() {
            return Err(ChromiumHandleEvaluationRouteError::EmptyContextId);
        }
        if spec.unique_context_id.len() > self.max_native_id_bytes {
            return Err(ChromiumHandleEvaluationRouteError::ContextIdTooLong {
                actual: spec.unique_context_id.len(),
                limit: self.max_native_id_bytes,
            });
        }
        Ok(ChromiumHandleEvaluationRoute {
            page_id: spec.page_id,
            service: self.factory.bind_execution_context(
                spec.client,
                spec.execution_context_id,
                spec.unique_context_id,
            ),
        })
    }

    fn resolve_route(
        &self,
        frame_id: ObjectId,
    ) -> Result<Option<ChromiumHandleEvaluationRoute>, ChromiumHandleEvaluationRouteError> {
        self.routes
            .lock()
            .map(|routes| routes.get(&frame_id).cloned())
            .map_err(|_| ChromiumHandleEvaluationRouteError::StatePoisoned)
    }
}

impl ChromiumHandleEvaluationRouteResolver for ChromiumHandleEvaluationRoutes {
    fn resolve(
        &self,
        frame_id: ObjectId,
    ) -> Result<Option<ChromiumHandleEvaluationRoute>, ChromiumHandleEvaluationRouteError> {
        self.resolve_route(frame_id)
    }
}
#[derive(Clone)]
struct ChromiumEvaluatedHandleRoute {
    native: CdpEvaluatedHandle,
    source_execution_context_id: i64,
    source_unique_context_id: Arc<str>,
}

type AdoptionOperationFuture<'a> = Pin<
    Box<
        dyn Future<
                Output = Result<
                    Box<dyn AdoptedArgumentBatch>,
                    OperationError<HandleEvaluationErrorDetails>,
                >,
            > + Send
            + 'a,
    >,
>;

trait ElementHandleArgumentAdopter: Send + Sync {
    fn adopt(&self, request: CdpElementHandleAdoptionBatchRequest) -> AdoptionOperationFuture<'_>;
}

trait AdoptedArgumentBatch: Send {
    fn handles(&self) -> &[CdpAdoptedElementHandle];

    fn release(
        self: Box<Self>,
    ) -> Pin<
        Box<dyn Future<Output = Result<(), OperationError<HandleEvaluationErrorDetails>>> + Send>,
    >;
}

impl<Reservation> AdoptedArgumentBatch for ContinuableAdoptedElementHandleBatch<Reservation>
where
    Reservation: CleanupReservation,
{
    fn handles(&self) -> &[CdpAdoptedElementHandle] {
        self.handles()
    }

    fn release(
        self: Box<Self>,
    ) -> Pin<
        Box<dyn Future<Output = Result<(), OperationError<HandleEvaluationErrorDetails>>> + Send>,
    > {
        Box::pin(async move { (*self).release().await })
    }
}

impl<Inner, Continuation> ElementHandleArgumentAdopter
    for ContinuableElementHandleAdoptionService<Inner, Continuation>
where
    Inner: Service<
            CdpElementHandleAdoptionBatchRequest,
            Response = CdpAdoptedElementHandleBatch,
            Error = OperationError<HandleEvaluationErrorDetails>,
        > + 'static,
    Continuation: ReservedCleanupExecutor,
{
    fn adopt(&self, request: CdpElementHandleAdoptionBatchRequest) -> AdoptionOperationFuture<'_> {
        Box::pin(async move {
            self.call(request)
                .await
                .map(|batch| Box::new(batch) as Box<dyn AdoptedArgumentBatch>)
        })
    }
}

impl ChromiumHandleDisposer for ChromiumEvaluatedHandleRoute {
    fn dispose(
        &self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), OperationError<HandleLifecycleErrorDetails>>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            self.native.release().await.map_err(|error| {
                OperationError::new(
                    pwright_backend_cdp_common::classify_cdp_failure(&error),
                    format!("CDP Runtime.releaseObject failed: {error}"),
                    Some(HandleLifecycleErrorDetails::Backend(error.to_string())),
                )
            })
        })
    }
}

#[derive(Clone)]
pub struct ChromiumHandleEvaluationCommandService {
    runtime: ControlledLifecycleHandle,
    object_ids: Arc<dyn ObjectIdSource>,
    frame_routes: Arc<dyn ChromiumHandleEvaluationRouteResolver>,
    handle_routes: ChromiumHandleRoutes,
    native_id_bytes: usize,
    preview_bytes: usize,
    property_limits: CdpHandlePropertyLimits,
    argument_adopter: Option<Arc<dyn ElementHandleArgumentAdopter>>,
}

struct ResolvedHandleOperation {
    owner_frame: ObjectId,
    owner_page: ObjectId,
    target: CdpEvaluatedHandle,
    arguments: ResolvedArguments,
    evaluation: CdpEvaluationService,
}

struct HandleCreationRoute {
    owner_frame: ObjectId,
    owner_page: ObjectId,
    evaluation: CdpEvaluationService,
    current_handles: Vec<ObjectId>,
}

struct ResolvedFrameCreation {
    route: HandleCreationRoute,
    request: CdpHandleEvaluationRequest,
    adopted: Option<Box<dyn AdoptedArgumentBatch>>,
}

struct ResolvedArguments {
    arguments: Vec<(ObjectId, CdpEvaluatedHandle)>,
    adopted: Option<Box<dyn AdoptedArgumentBatch>>,
}

impl ResolvedArguments {
    fn exact(arguments: Vec<(ObjectId, CdpEvaluatedHandle)>) -> Self {
        Self {
            arguments,
            adopted: None,
        }
    }
}

impl ChromiumHandleEvaluationCommandService {
    pub fn new(
        runtime: ControlledLifecycleHandle,
        object_ids: Arc<dyn ObjectIdSource>,
        frame_routes: ChromiumHandleEvaluationRoutes,
        handle_routes: ChromiumHandleRoutes,
        backend: ChromiumHandleEvaluationBackendProfile,
    ) -> Self {
        Self::new_with_route_resolver(
            runtime,
            object_ids,
            Arc::new(frame_routes),
            handle_routes,
            backend,
        )
    }

    pub fn new_with_route_resolver(
        runtime: ControlledLifecycleHandle,
        object_ids: Arc<dyn ObjectIdSource>,
        frame_routes: Arc<dyn ChromiumHandleEvaluationRouteResolver>,
        handle_routes: ChromiumHandleRoutes,
        backend: ChromiumHandleEvaluationBackendProfile,
    ) -> Self {
        Self {
            runtime,
            object_ids,
            frame_routes,
            handle_routes,
            native_id_bytes: backend.native_id_bytes(),
            preview_bytes: backend.preview_bytes(),
            property_limits: backend.properties(),
            argument_adopter: None,
        }
    }

    fn with_argument_adopter(mut self, adopter: Arc<dyn ElementHandleArgumentAdopter>) -> Self {
        self.argument_adopter = Some(adopter);
        self
    }

    /// Opts this command service into temporary cross-world ElementHandle
    /// adoption. Callers choose and budget the backend plus continuation; the
    /// default command service retains no adoption machinery.
    pub fn with_element_handle_adoption<Continuation>(
        self,
        adoption: Arc<CdpElementHandleAdoptionBatchService>,
        continuation: Continuation,
    ) -> Self
    where
        Continuation: ReservedCleanupExecutor,
    {
        self.with_argument_adopter(Arc::new(ContinuableElementHandleAdoptionService::new(
            adoption,
            continuation,
        )))
    }

    async fn evaluate_handle_for_frame(
        &self,
        frame_id: ObjectId,
        metadata: CallMetadata,
        params: EvaluateHandleParams,
    ) -> Result<HandleInitializer, OperationError<HandleEvaluationErrorDetails>> {
        let creation = self
            .resolve_frame_creation(frame_id, metadata, params.into())
            .await?;
        self.create_handle(creation.route, creation.request, creation.adopted)
            .await
    }

    async fn resolve_frame_creation(
        &self,
        frame_id: ObjectId,
        metadata: CallMetadata,
        params: CdpHandleEvaluationProgram,
    ) -> Result<ResolvedFrameCreation, OperationError<HandleEvaluationErrorDetails>> {
        validate_metadata(metadata, "Frame.evaluateHandle")?;
        validate_live(&self.runtime, frame_id, "Frame")?;
        let evaluation = self.frame_route(frame_id)?;
        let handle_ids = params
            .referenced_handles(evaluation.service.limits().value)
            .map_err(invalid_argument)?;
        let arguments = self
            .resolve_arguments(
                frame_id,
                evaluation.page_id,
                &evaluation.service,
                &handle_ids,
            )
            .await?;
        let ResolvedArguments { arguments, adopted } = arguments;
        Ok(ResolvedFrameCreation {
            route: HandleCreationRoute {
                owner_frame: frame_id,
                owner_page: evaluation.page_id,
                evaluation: evaluation.service,
                current_handles: handle_ids,
            },
            request: CdpHandleEvaluationRequest::new_program(params, None, arguments),
            adopted,
        })
    }

    async fn evaluate_value(
        &self,
        target_id: ObjectId,
        metadata: CallMetadata,
        params: HandleEvaluateParams,
    ) -> Result<HandleValueResult, OperationError<HandleEvaluationErrorDetails>> {
        validate_metadata(metadata, "Handle.evaluate")?;
        validate_live(&self.runtime, target_id, "Handle")?;
        let limits = self.value_limits(target_id)?;
        let ids = params
            .referenced_handles(limits)
            .map_err(invalid_argument)?;
        let resolved = self.resolve_handle(target_id, &ids).await?;
        let ResolvedArguments { arguments, adopted } = resolved.arguments;
        let outcome = CdpHandleValueEvaluationService::from_evaluation(
            resolved.evaluation,
            self.native_id_bytes,
            self.preview_bytes,
        )
        .call(CdpHandleEvaluationRequest::new_on_handle(
            params.as_frame_params(),
            (target_id, resolved.target),
            arguments,
        ))
        .await;
        finish_adopted_operation(outcome, adopted)
            .await
            .map(HandleValueResult::new)
    }

    async fn evaluate_handle_for_handle(
        &self,
        target_id: ObjectId,
        metadata: CallMetadata,
        params: HandleEvaluateParams,
    ) -> Result<HandleInitializer, OperationError<HandleEvaluationErrorDetails>> {
        validate_metadata(metadata, "Handle.evaluateHandle")?;
        validate_live(&self.runtime, target_id, "Handle")?;
        let limits = self.value_limits(target_id)?;
        let ids = params
            .referenced_handles(limits)
            .map_err(invalid_argument)?;
        let resolved = self.resolve_handle(target_id, &ids).await?;
        let mut current_handles = Vec::with_capacity(ids.len().saturating_add(1));
        current_handles.push(target_id);
        current_handles.extend(ids);
        let ResolvedArguments { arguments, adopted } = resolved.arguments;
        self.create_handle(
            HandleCreationRoute {
                owner_frame: resolved.owner_frame,
                owner_page: resolved.owner_page,
                evaluation: resolved.evaluation,
                current_handles,
            },
            CdpHandleEvaluationRequest::new_on_handle(
                params.as_frame_params(),
                (target_id, resolved.target),
                arguments,
            ),
            adopted,
        )
        .await
    }

    async fn json_value(
        &self,
        target_id: ObjectId,
        metadata: CallMetadata,
    ) -> Result<HandleValueResult, OperationError<HandleEvaluationErrorDetails>> {
        validate_metadata(metadata, "Handle.jsonValue")?;
        self.evaluate_value(
            target_id,
            CallMetadata::default(),
            HandleEvaluateParams::function_without_argument("target => target"),
        )
        .await
    }

    async fn get_property(
        &self,
        target_id: ObjectId,
        metadata: CallMetadata,
        params: GetPropertyParams,
    ) -> Result<HandleInitializer, OperationError<HandleEvaluationErrorDetails>> {
        validate_metadata(metadata, "Handle.getProperty")?;
        if params.name().len() > self.property_limits.property_name_bytes() {
            return Err(OperationError::new(
                FailureKind::ResourceExhausted,
                format!(
                    "property name has {} bytes, limit is {}",
                    params.name().len(),
                    self.property_limits.property_name_bytes()
                ),
                Some(HandleEvaluationErrorDetails::InvalidValue(
                    "property name exceeds configured byte limit".to_owned(),
                )),
            ));
        }
        validate_live(&self.runtime, target_id, "Handle")?;
        let resolved = self.resolve_handle(target_id, &[]).await?;
        let object_id = next_id(&self.object_ids)?;
        // The persistent result is admitted before the backend creates even its
        // temporary snapshot. Property values must not be awaited directly.
        let reservation = self.handle_routes.reserve().map_err(map_route_error)?;
        let native = CdpHandlePropertiesService::from_evaluation(
            resolved.evaluation.clone(),
            self.native_id_bytes,
            self.preview_bytes,
            self.property_limits,
        )
        .call_property_on_handle(target_id, resolved.target, params.name())
        .await?;
        self.publish_handle(
            HandleCreationRoute {
                owner_frame: resolved.owner_frame,
                owner_page: resolved.owner_page,
                evaluation: resolved.evaluation,
                current_handles: vec![target_id],
            },
            object_id,
            reservation,
            native,
        )
        .await
    }

    async fn get_properties(
        &self,
        target_id: ObjectId,
        metadata: CallMetadata,
    ) -> Result<HandlePropertiesResult, OperationError<HandleEvaluationErrorDetails>> {
        validate_metadata(metadata, "Handle.getProperties")?;
        validate_live(&self.runtime, target_id, "Handle")?;
        let resolved = self.resolve_handle(target_id, &[]).await?;
        let evaluation = resolved.evaluation.clone();
        let native = CdpHandlePropertiesService::from_evaluation(
            resolved.evaluation,
            self.native_id_bytes,
            self.preview_bytes,
            self.property_limits,
        )
        .call_on_handle(target_id, resolved.target)
        .await?;
        if native.is_empty() {
            return Ok(HandlePropertiesResult::default());
        }
        self.publish_batch(
            target_id,
            resolved.owner_frame,
            resolved.owner_page,
            &evaluation,
            native,
        )
        .await
        .map(HandlePropertiesResult::new)
    }

    async fn create_handle(
        &self,
        route: HandleCreationRoute,
        request: CdpHandleEvaluationRequest,
        adopted: Option<Box<dyn AdoptedArgumentBatch>>,
    ) -> Result<HandleInitializer, OperationError<HandleEvaluationErrorDetails>> {
        let object_id = next_id(&self.object_ids)?;
        let reservation = self.handle_routes.reserve().map_err(map_route_error)?;
        let outcome = CdpHandleEvaluationService::from_evaluation(
            route.evaluation.clone(),
            self.native_id_bytes,
            self.preview_bytes,
        )
        .call(request)
        .await;
        let native = finish_adopted_handle_creation(outcome, adopted).await?;
        self.publish_handle(route, object_id, reservation, native)
            .await
    }

    async fn publish_handle(
        &self,
        route: HandleCreationRoute,
        object_id: ObjectId,
        reservation: ChromiumHandleRouteReservation,
        native: CdpEvaluatedHandle,
    ) -> Result<HandleInitializer, OperationError<HandleEvaluationErrorDetails>> {
        let evaluation = &route.evaluation;
        if let Err(error) = self
            .ensure_current_frame_route(route.owner_frame, route.owner_page, evaluation)
            .and_then(|()| self.ensure_current_handles(route.owner_page, &route.current_handles))
        {
            return release_failed(native, error).await;
        }
        let prepared = match self.runtime.prepare_object_creation(
            object_id,
            Some(route.owner_frame),
            native.object_kind(),
        ) {
            Ok(prepared) => prepared,
            Err(error) => return release_failed(native, map_runtime_error(error)).await,
        };
        let initializer = HandleInitializer::new(
            object_id,
            route.owner_frame,
            native.object_kind(),
            native.preview().to_owned(),
        );
        let payload = Arc::new(ChromiumEvaluatedHandleRoute {
            native: native.clone(),
            source_execution_context_id: evaluation
                .execution_context_id()
                .expect("bound Handle evaluation route has a numeric execution-context ID"),
            source_unique_context_id: evaluation
                .shared_unique_context_id()
                .expect("bound Handle evaluation route has a stable execution-context ID"),
        });
        let disposer = native
            .object_id()
            .map(|_| Arc::clone(&payload) as Arc<dyn ChromiumHandleDisposer>);
        let spec = ChromiumHandleRouteSpec::new(
            route.owner_frame,
            route.owner_page,
            native.native_argument_bytes(),
            native.preview().len(),
            disposer,
        )
        .with_payload(payload);
        match reservation.publish_with(object_id, spec, || prepared.commit()) {
            Ok(()) => Ok(initializer),
            Err(error) => release_failed(native, map_publication_error(error)).await,
        }
    }

    async fn publish_batch(
        &self,
        source_handle: ObjectId,
        owner_frame: ObjectId,
        owner_page: ObjectId,
        evaluation: &CdpEvaluationService,
        native: Vec<(String, CdpEvaluatedHandle)>,
    ) -> Result<Vec<HandlePropertyInitializer>, OperationError<HandleEvaluationErrorDetails>> {
        let reservation = match self.handle_routes.reserve_batch(native.len()) {
            Ok(reservation) => reservation,
            Err(error) => return release_failed_batch(native, map_route_error(error)).await,
        };
        let mut ids = Vec::with_capacity(native.len());
        for _ in &native {
            match next_id(&self.object_ids) {
                Ok(id) => ids.push(id),
                Err(error) => return release_failed_batch(native, error).await,
            }
        }
        if let Err(error) = self.ensure_current_frame_route(owner_frame, owner_page, evaluation) {
            return release_failed_batch(native, error).await;
        }
        if let Err(error) = self.ensure_current_handles(owner_page, &[source_handle]) {
            return release_failed_batch(native, error).await;
        }
        let creation_specs = ids
            .iter()
            .zip(&native)
            .map(|(id, (_, handle))| {
                ObjectCreationSpec::new(*id, Some(owner_frame), handle.object_kind())
            })
            .collect::<Vec<_>>();
        let prepared = match self.runtime.prepare_object_creation_batch(&creation_specs) {
            Ok(prepared) => prepared,
            Err(error) => return release_failed_batch(native, map_runtime_error(error)).await,
        };
        let initializers = ids
            .iter()
            .zip(&native)
            .map(|(id, (name, handle))| {
                HandlePropertyInitializer::new(
                    name.clone(),
                    HandleInitializer::new(
                        *id,
                        owner_frame,
                        handle.object_kind(),
                        handle.preview().to_owned(),
                    ),
                )
            })
            .collect::<Vec<_>>();
        let entries = ids
            .into_iter()
            .zip(native.iter().map(|(_, handle)| handle))
            .map(|(id, handle)| {
                let route = Arc::new(ChromiumEvaluatedHandleRoute {
                    native: handle.clone(),
                    source_execution_context_id: evaluation
                        .execution_context_id()
                        .expect("bound Handle evaluation route has a numeric execution-context ID"),
                    source_unique_context_id: evaluation
                        .shared_unique_context_id()
                        .expect("bound Handle evaluation route has a stable execution-context ID"),
                });
                let disposer = handle
                    .object_id()
                    .map(|_| Arc::clone(&route) as Arc<dyn ChromiumHandleDisposer>);
                (
                    id,
                    ChromiumHandleRouteSpec::new(
                        owner_frame,
                        owner_page,
                        handle.native_argument_bytes(),
                        handle.preview().len(),
                        disposer,
                    )
                    .with_payload(route),
                )
            })
            .collect();
        match reservation.publish_with(entries, || prepared.commit()) {
            Ok(()) => Ok(initializers),
            Err(error) => release_failed_batch(native, map_publication_error(error)).await,
        }
    }

    fn frame_route(
        &self,
        frame_id: ObjectId,
    ) -> Result<ChromiumHandleEvaluationRoute, OperationError<HandleEvaluationErrorDetails>> {
        self.frame_routes
            .resolve(frame_id)
            .map_err(map_frame_route_error)?
            .ok_or_else(|| missing_frame_route(frame_id))
    }

    fn value_limits(
        &self,
        target_id: ObjectId,
    ) -> Result<
        pwright_cap_handle_evaluation::EvaluationValueLimits,
        OperationError<HandleEvaluationErrorDetails>,
    > {
        let route = self.handle_snapshot(target_id)?;
        Ok(self
            .frame_route(route.owner_frame())?
            .service
            .limits()
            .value)
    }

    fn handle_snapshot(
        &self,
        object_id: ObjectId,
    ) -> Result<
        ChromiumHandleRouteSnapshot<ChromiumEvaluatedHandleRoute>,
        OperationError<HandleEvaluationErrorDetails>,
    > {
        self.handle_routes
            .snapshot(object_id)
            .map_err(map_route_error)?
            .ok_or_else(|| missing_handle(object_id))
    }

    async fn resolve_handle(
        &self,
        target_id: ObjectId,
        argument_ids: &[ObjectId],
    ) -> Result<ResolvedHandleOperation, OperationError<HandleEvaluationErrorDetails>> {
        let target = self.handle_snapshot(target_id)?;
        let evaluation = self.frame_route(target.owner_frame())?;
        let payload = target.payload();
        if target.owner_page() != evaluation.page_id
            || !handle_belongs_to_world(&payload, &evaluation.service)
        {
            return Err(foreign_handle(target_id));
        }
        let arguments = self
            .resolve_arguments(
                target.owner_frame(),
                target.owner_page(),
                &evaluation.service,
                argument_ids,
            )
            .await?;
        Ok(ResolvedHandleOperation {
            owner_frame: target.owner_frame(),
            owner_page: target.owner_page(),
            target: payload.native.clone(),
            arguments,
            evaluation: evaluation.service,
        })
    }

    async fn resolve_arguments(
        &self,
        owner_frame: ObjectId,
        owner_page: ObjectId,
        evaluation: &CdpEvaluationService,
        object_ids: &[ObjectId],
    ) -> Result<ResolvedArguments, OperationError<HandleEvaluationErrorDetails>> {
        enum ArgumentSource {
            Exact(CdpEvaluatedHandle),
            Adopted(usize),
        }

        let mut sources = Vec::with_capacity(object_ids.len());
        let mut requests = Vec::new();
        for object_id in object_ids {
            let route = self.handle_snapshot(*object_id)?;
            let payload = route.payload();
            if route.owner_frame() == owner_frame
                && route.owner_page() == owner_page
                && handle_belongs_to_world(&payload, evaluation)
            {
                sources.push(ArgumentSource::Exact(payload.native.clone()));
                continue;
            }
            if self.argument_adopter.is_none()
                || route.owner_page() != owner_page
                || payload.native.kind() != CdpHandleKind::Element
                || !payload.native.belongs_to_client(&evaluation.client())
            {
                return Err(foreign_handle(*object_id));
            }
            let source_world = self.frame_route(route.owner_frame())?;
            if source_world.page_id != route.owner_page()
                || !handle_belongs_to_world(&payload, &source_world.service)
            {
                return Err(foreign_handle(*object_id));
            }
            let adopted_index = requests.len();
            requests.push(CdpElementHandleAdoptionRequest::new(
                payload.native.clone(),
                evaluation.client(),
                evaluation
                    .execution_context_id()
                    .expect("bound Handle evaluation route has a numeric execution-context ID"),
            ));
            sources.push(ArgumentSource::Adopted(adopted_index));
        }
        if requests.is_empty() {
            let arguments = object_ids
                .iter()
                .copied()
                .zip(sources.into_iter().map(|source| match source {
                    ArgumentSource::Exact(handle) => handle,
                    ArgumentSource::Adopted(_) => {
                        unreachable!("empty adoption batch has no adopted sources")
                    }
                }))
                .collect();
            return Ok(ResolvedArguments::exact(arguments));
        }
        let adopted = self
            .argument_adopter
            .as_ref()
            .expect("adoption requests require one configured adopter")
            .adopt(CdpElementHandleAdoptionBatchRequest::new(requests))
            .await?;
        let arguments = object_ids
            .iter()
            .copied()
            .zip(sources.into_iter().map(|source| match source {
                ArgumentSource::Exact(handle) => handle,
                ArgumentSource::Adopted(index) => adopted.handles()[index].handle().clone(),
            }))
            .collect();
        Ok(ResolvedArguments {
            arguments,
            adopted: Some(adopted),
        })
    }

    fn ensure_current_frame_route(
        &self,
        owner_frame: ObjectId,
        owner_page: ObjectId,
        evaluation: &CdpEvaluationService,
    ) -> Result<(), OperationError<HandleEvaluationErrorDetails>> {
        let route = self.frame_route(owner_frame)?;
        if route.page_id != owner_page || !same_execution_world(&route.service, evaluation) {
            return Err(missing_frame_route(owner_frame));
        }
        Ok(())
    }

    fn ensure_current_handles(
        &self,
        owner_page: ObjectId,
        object_ids: &[ObjectId],
    ) -> Result<(), OperationError<HandleEvaluationErrorDetails>> {
        for object_id in object_ids {
            let route = self.handle_snapshot(*object_id)?;
            let payload = route.payload();
            let source_world = self.frame_route(route.owner_frame())?;
            if route.owner_page() != owner_page
                || source_world.page_id != route.owner_page()
                || !handle_belongs_to_world(&payload, &source_world.service)
            {
                return Err(foreign_handle(*object_id));
            }
        }
        Ok(())
    }
}

fn same_execution_world(left: &CdpEvaluationService, right: &CdpEvaluationService) -> bool {
    Arc::ptr_eq(&left.client(), &right.client())
        && left.execution_context_id() == right.execution_context_id()
        && left.unique_context_id() == right.unique_context_id()
}

fn handle_belongs_to_world(
    handle: &ChromiumEvaluatedHandleRoute,
    evaluation: &CdpEvaluationService,
) -> bool {
    handle.native.belongs_to_client(&evaluation.client())
        && evaluation.execution_context_id() == Some(handle.source_execution_context_id)
        && evaluation.unique_context_id() == Some(handle.source_unique_context_id.as_ref())
}

macro_rules! impl_command {
    ($spec:ty, $method:ident, $params:pat => $argument:expr) => {
        impl Service<Command<$spec>> for ChromiumHandleEvaluationCommandService {
            type Response = CommandResult<$spec>;
            type Error = Infallible;
            type Future<'a> =
                Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

            fn call(&self, command: Command<$spec>) -> Self::Future<'_> {
                let (call_id, object_id, metadata, params) = command.into_parts();
                let $params = params;
                Box::pin(async move {
                    Ok(CommandResult::new(
                        call_id,
                        self.$method(object_id, metadata, $argument).await,
                    ))
                })
            }
        }
    };
}

impl_command!(EvaluateHandle, evaluate_handle_for_frame, params => params);
impl_command!(HandleEvaluate, evaluate_value, params => params);
impl_command!(HandleEvaluateHandle, evaluate_handle_for_handle, params => params);
impl_command!(HandleGetProperty, get_property, params => params);

impl Service<Command<HandleJsonValue>> for ChromiumHandleEvaluationCommandService {
    type Response = CommandResult<HandleJsonValue>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<HandleJsonValue>) -> Self::Future<'_> {
        let (call_id, object_id, metadata, JsonValueOptions) = command.into_parts();
        Box::pin(async move {
            Ok(CommandResult::new(
                call_id,
                self.json_value(object_id, metadata).await,
            ))
        })
    }
}

impl Service<Command<HandleGetProperties>> for ChromiumHandleEvaluationCommandService {
    type Response = CommandResult<HandleGetProperties>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<HandleGetProperties>) -> Self::Future<'_> {
        let (call_id, object_id, metadata, GetPropertiesOptions) = command.into_parts();
        Box::pin(async move {
            Ok(CommandResult::new(
                call_id,
                self.get_properties(object_id, metadata).await,
            ))
        })
    }
}

#[doc(hidden)]
pub trait HandleCreationCommand:
    pwright_core::CommandSpec<ErrorDetails = HandleEvaluationErrorDetails>
{
    fn created_handles(result: &Self::Result) -> Vec<HandleInitializer>;
    fn operation_name() -> &'static str;
}

impl HandleCreationCommand for EvaluateHandle {
    fn created_handles(result: &Self::Result) -> Vec<HandleInitializer> {
        vec![result.clone()]
    }

    fn operation_name() -> &'static str {
        "Frame.evaluateHandle"
    }
}

impl HandleCreationCommand for HandleEvaluateHandle {
    fn created_handles(result: &Self::Result) -> Vec<HandleInitializer> {
        vec![result.clone()]
    }

    fn operation_name() -> &'static str {
        "Handle.evaluateHandle"
    }
}

impl HandleCreationCommand for HandleGetProperty {
    fn created_handles(result: &Self::Result) -> Vec<HandleInitializer> {
        vec![result.clone()]
    }

    fn operation_name() -> &'static str {
        "Handle.getProperty"
    }
}

impl HandleCreationCommand for HandleGetProperties {
    fn created_handles(result: &Self::Result) -> Vec<HandleInitializer> {
        result
            .properties
            .iter()
            .map(|property| property.handle.clone())
            .collect()
    }

    fn operation_name() -> &'static str {
        "Handle.getProperties"
    }
}

type BoxCreationFuture<Spec> =
    Pin<Box<dyn Future<Output = Result<CommandResult<Spec>, Infallible>> + Send + 'static>>;

pub struct ContinuableHandleEvaluationFuture<Spec, Dispose, Reservation>
where
    Spec: HandleCreationCommand,
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Reservation: CleanupReservation,
{
    inner: Option<BoxCreationFuture<Spec>>,
    dispose: Option<Arc<Dispose>>,
    reservation: Option<Reservation>,
    call_id: CallId,
    started: bool,
}

impl<Spec, Dispose, Reservation> Unpin
    for ContinuableHandleEvaluationFuture<Spec, Dispose, Reservation>
where
    Spec: HandleCreationCommand,
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Reservation: CleanupReservation,
{
}

impl<Spec, Dispose, Reservation> Future
    for ContinuableHandleEvaluationFuture<Spec, Dispose, Reservation>
where
    Spec: HandleCreationCommand,
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Reservation: CleanupReservation,
{
    type Output = Result<CommandResult<Spec>, Infallible>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.started = true;
        let outcome = self
            .inner
            .as_mut()
            .expect("completed Handle evaluation future must not be polled again")
            .as_mut()
            .poll(context);
        if outcome.is_ready() {
            self.inner = None;
            self.dispose = None;
            self.reservation = None;
        }
        outcome
    }
}

impl<Spec, Dispose, Reservation> Drop
    for ContinuableHandleEvaluationFuture<Spec, Dispose, Reservation>
where
    Spec: HandleCreationCommand,
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Reservation: CleanupReservation,
{
    fn drop(&mut self) {
        if !self.started {
            return;
        }
        let Some(inner) = self.inner.take() else {
            return;
        };
        let Some(dispose) = self.dispose.take() else {
            return;
        };
        let Some(reservation) = self.reservation.take() else {
            return;
        };
        let call_id = self.call_id;
        reservation.submit(Box::pin(async move {
            let handles = match inner.await {
                Ok(result) => match result.into_outcome() {
                    Ok(value) => Spec::created_handles(&value),
                    Err(error) if error.kind() == FailureKind::Cleanup => {
                        return Err(CleanupFailure::new(
                            "abandoned Handle evaluation reported failed native compensation",
                        ));
                    }
                    Err(_) => return Ok(()),
                },
                Err(never) => match never {},
            };
            let mut cleanup_failed = false;
            for initializer in handles {
                let result = match dispose
                    .call(Command::<HandleDispose>::new(
                        call_id,
                        initializer.object_id,
                        CallMetadata::default(),
                        (),
                    ))
                    .await
                {
                    Ok(result) => result,
                    Err(never) => match never {},
                };
                // Every result owns an independent route. One refused release
                // must not strand later handles in an unobserved batch.
                cleanup_failed |= result.into_outcome().is_err();
            }
            if cleanup_failed {
                Err(CleanupFailure::new(
                    "abandoned Handle evaluation result could not be disposed",
                ))
            } else {
                Ok(())
            }
        }));
    }
}

#[derive(Clone)]
pub struct ContinuableHandleEvaluationService<Inner, Dispose, Continuation> {
    service: Arc<Inner>,
    dispose: Arc<Dispose>,
    continuation: Continuation,
}

impl<Inner, Dispose, Continuation>
    ContinuableHandleEvaluationService<Inner, Dispose, Continuation>
{
    pub const fn new(
        service: Arc<Inner>,
        dispose: Arc<Dispose>,
        continuation: Continuation,
    ) -> Self {
        Self {
            service,
            dispose,
            continuation,
        }
    }
}

impl<Spec, Inner, Dispose, Continuation> Service<Command<Spec>>
    for ContinuableHandleEvaluationService<Inner, Dispose, Continuation>
where
    Spec: HandleCreationCommand + Send + Sync + 'static,
    Spec::Params: Send,
    Spec::Result: Send,
    Inner: Service<Command<Spec>, Response = CommandResult<Spec>, Error = Infallible> + 'static,
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Continuation: ReservedCleanupExecutor,
{
    type Response = CommandResult<Spec>;
    type Error = Infallible;
    type Future<'a>
        = ContinuableHandleEvaluationFuture<Spec, Dispose, Continuation::Reservation>
    where
        Self: 'a;

    fn call(&self, command: Command<Spec>) -> Self::Future<'_> {
        let call_id = command.call_id();
        let reservation = match self.continuation.try_reserve() {
            Ok(reservation) => reservation,
            Err(error) => {
                return ContinuableHandleEvaluationFuture {
                    inner: Some(Box::pin(std::future::ready(Ok(
                        continuation_failure::<Spec>(call_id, error),
                    )))),
                    dispose: None,
                    reservation: None,
                    call_id,
                    started: false,
                };
            }
        };
        let service = Arc::clone(&self.service);
        ContinuableHandleEvaluationFuture {
            inner: Some(Box::pin(async move { service.call(command).await })),
            dispose: Some(Arc::clone(&self.dispose)),
            reservation: Some(reservation),
            call_id,
            started: false,
        }
    }
}

macro_rules! delegate_value_command {
    ($spec:ty) => {
        impl<Inner, Dispose, Continuation> Service<Command<$spec>>
            for ContinuableHandleEvaluationService<Inner, Dispose, Continuation>
        where
            Inner: Service<Command<$spec>, Response = CommandResult<$spec>, Error = Infallible>
                + 'static,
            Dispose: Send + Sync + 'static,
            Continuation: ReservedCleanupExecutor,
        {
            type Response = CommandResult<$spec>;
            type Error = Infallible;
            type Future<'a> =
                Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

            fn call(&self, command: Command<$spec>) -> Self::Future<'_> {
                Box::pin(async move { self.service.call(command).await })
            }
        }
    };
}

delegate_value_command!(HandleEvaluate);
delegate_value_command!(HandleJsonValue);

pub type ChromiumHandleEvaluationHandlers<Creation, Raw> =
    HandleEvaluationServerHandlers<Creation, Raw, Creation, Creation, Creation, Raw>;

pub struct ChromiumHandleEvaluationBindings<CreationContinuation, LifecycleContinuation> {
    pub runtime: ControlledLifecycleHandle,
    pub object_ids: Arc<dyn ObjectIdSource>,
    pub creation_continuation: CreationContinuation,
    pub lifecycle_continuation: LifecycleContinuation,
}

pub struct ChromiumHandleEvaluationInstallation<CreationContinuation, LifecycleContinuation>
where
    CreationContinuation: ReservedCleanupExecutor,
    LifecycleContinuation: ReservedCleanupExecutor,
{
    lifecycle: ChromiumHandleLifecycleInstallation<LifecycleContinuation>,
    routes: ChromiumHandleEvaluationRoutes,
    raw: Arc<ChromiumHandleEvaluationCommandService>,
    creation: Arc<
        ContinuableHandleEvaluationService<
            ChromiumHandleEvaluationCommandService,
            ContinuableHandleDisposeService<LifecycleContinuation>,
            CreationContinuation,
        >,
    >,
    protocol: Arc<WireProtocol<HandleEvaluationCodec>>,
}

impl<CreationContinuation, LifecycleContinuation>
    ChromiumHandleEvaluationInstallation<CreationContinuation, LifecycleContinuation>
where
    CreationContinuation: ReservedCleanupExecutor,
    LifecycleContinuation: ReservedCleanupExecutor,
{
    pub fn install(
        bindings: ChromiumHandleEvaluationBindings<CreationContinuation, LifecycleContinuation>,
        profile: ChromiumHandleEvaluationProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumHandleEvaluationInstallError> {
        Self::install_with_argument_adopter(bindings, profile, resources, None)
    }

    pub fn install_with_element_adoption<AdoptionContinuation>(
        bindings: ChromiumHandleEvaluationBindings<CreationContinuation, LifecycleContinuation>,
        profile: ChromiumHandleEvaluationProfile,
        adoption_profile: CdpElementHandleAdoptionProductionProfile,
        adoption_continuation: AdoptionContinuation,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumHandleEvaluationInstallError>
    where
        AdoptionContinuation: ReservedCleanupExecutor,
    {
        let backend = profile.backend();
        if adoption_profile.limits()
            != CdpHandleEvaluationLimits::new(
                backend.evaluation().limits(),
                backend.native_id_bytes(),
                backend.preview_bytes(),
            )
        {
            return Err(ChromiumHandleEvaluationInstallError::Setup(
                ChromiumHandleEvaluationSetupError::AdoptionBoundsMismatch,
            ));
        }
        if adoption_profile.scope() == backend.evaluation().resource_scope()
            || adoption_profile.scope() == profile.lifecycle().routes().scope()
        {
            return Err(ChromiumHandleEvaluationInstallError::Setup(
                ChromiumHandleEvaluationSetupError::DuplicateAdoptionResourceScope,
            ));
        }
        let adoption = CdpElementHandleAdoptionBatchService::new_with_production_resources(
            adoption_profile,
            resources,
        )
        .map_err(ChromiumHandleEvaluationInstallError::Adoption)?;
        let adopter: Arc<dyn ElementHandleArgumentAdopter> = Arc::new(
            ContinuableElementHandleAdoptionService::new(Arc::new(adoption), adoption_continuation),
        );
        Self::install_with_argument_adopter(bindings, profile, resources, Some(adopter))
    }

    fn install_with_argument_adopter(
        bindings: ChromiumHandleEvaluationBindings<CreationContinuation, LifecycleContinuation>,
        profile: ChromiumHandleEvaluationProfile,
        resources: &ProductionResourceLedger,
        argument_adopter: Option<Arc<dyn ElementHandleArgumentAdopter>>,
    ) -> Result<Self, ChromiumHandleEvaluationInstallError> {
        let profile = profile
            .validate()
            .map_err(ChromiumHandleEvaluationInstallError::Setup)?;
        let lifecycle = ChromiumHandleLifecycleInstallation::install(
            ChromiumHandleLifecycleBindings {
                runtime: bindings.runtime.clone(),
                continuation: bindings.lifecycle_continuation,
            },
            profile.lifecycle(),
            resources,
        )
        .map_err(ChromiumHandleEvaluationInstallError::Lifecycle)?;
        let routes = ChromiumHandleEvaluationRoutes::new_with_production_resources(
            profile.backend().evaluation(),
            resources,
        )?;
        let mut raw = ChromiumHandleEvaluationCommandService::new(
            bindings.runtime,
            bindings.object_ids,
            routes.clone(),
            lifecycle.routes(),
            profile.backend(),
        );
        if let Some(adopter) = argument_adopter {
            raw = raw.with_argument_adopter(adopter);
        }
        let raw = Arc::new(raw);
        let creation = Arc::new(ContinuableHandleEvaluationService::new(
            Arc::clone(&raw),
            lifecycle.dispose_service(),
            bindings.creation_continuation,
        ));
        let protocol = profile
            .unbudgeted_protocol()
            .with_production_resources(resources)
            .map_err(ChromiumHandleEvaluationInstallError::Resources)?;
        Ok(Self {
            lifecycle,
            routes,
            raw,
            creation,
            protocol: Arc::new(protocol),
        })
    }

    pub const fn lifecycle(&self) -> &ChromiumHandleLifecycleInstallation<LifecycleContinuation> {
        &self.lifecycle
    }

    pub fn routes(&self) -> ChromiumHandleEvaluationRoutes {
        self.routes.clone()
    }

    pub fn raw_service(&self) -> Arc<ChromiumHandleEvaluationCommandService> {
        Arc::clone(&self.raw)
    }

    pub fn creation_service(
        &self,
    ) -> Arc<
        ContinuableHandleEvaluationService<
            ChromiumHandleEvaluationCommandService,
            ContinuableHandleDisposeService<LifecycleContinuation>,
            CreationContinuation,
        >,
    > {
        Arc::clone(&self.creation)
    }

    /// Returns the exact application protocol so optional remote-delivery
    /// plugins can wrap the raw services without depending on this crate's
    /// local handler policy.
    pub fn protocol(&self) -> Arc<WireProtocol<HandleEvaluationCodec>> {
        Arc::clone(&self.protocol)
    }

    pub fn handlers(
        &self,
    ) -> ChromiumHandleEvaluationHandlers<
        ContinuableHandleEvaluationService<
            ChromiumHandleEvaluationCommandService,
            ContinuableHandleDisposeService<LifecycleContinuation>,
            CreationContinuation,
        >,
        ChromiumHandleEvaluationCommandService,
    > {
        handle_evaluation_server_handlers(
            Arc::clone(&self.protocol),
            HandleEvaluationServerServices {
                evaluate_handle: Arc::clone(&self.creation),
                handle_evaluate: Arc::clone(&self.raw),
                handle_evaluate_handle: Arc::clone(&self.creation),
                handle_get_properties: Arc::clone(&self.creation),
                handle_get_property: Arc::clone(&self.creation),
                handle_json_value: Arc::clone(&self.raw),
            },
        )
    }
}

fn validate_metadata(
    metadata: CallMetadata,
    operation: &str,
) -> Result<(), OperationError<HandleEvaluationErrorDetails>> {
    if metadata.timeout.is_none() {
        Ok(())
    } else {
        Err(OperationError::new(
            FailureKind::ProtocolViolation,
            format!("{operation} does not accept timeout metadata"),
            None,
        ))
    }
}

fn validate_live(
    runtime: &ControlledLifecycleHandle,
    object_id: ObjectId,
    kind: &str,
) -> Result<(), OperationError<HandleEvaluationErrorDetails>> {
    match runtime.is_object_live(object_id) {
        Ok(true) => Ok(()),
        Ok(false) => Err(OperationError::new(
            FailureKind::TargetClosed,
            format!("{kind} `{object_id}` is closed"),
            None,
        )),
        Err(error) => Err(map_runtime_error(error)),
    }
}

fn next_id(
    source: &Arc<dyn ObjectIdSource>,
) -> Result<ObjectId, OperationError<HandleEvaluationErrorDetails>> {
    source.next_object_id().map_err(|error| {
        OperationError::new(
            FailureKind::ResourceExhausted,
            error.to_string(),
            Some(HandleEvaluationErrorDetails::InvalidBackendResponse(
                error.to_string(),
            )),
        )
    })
}

fn invalid_argument(
    error: pwright_cap_handle_evaluation::HandleEvaluationValueError,
) -> OperationError<HandleEvaluationErrorDetails> {
    OperationError::new(
        FailureKind::ProtocolViolation,
        format!("invalid Handle evaluation argument: {error}"),
        Some(HandleEvaluationErrorDetails::InvalidValue(
            error.to_string(),
        )),
    )
}

fn missing_handle(object_id: ObjectId) -> OperationError<HandleEvaluationErrorDetails> {
    OperationError::new(
        FailureKind::TargetClosed,
        format!("Handle `{object_id}` is disposed or unknown"),
        Some(HandleEvaluationErrorDetails::InvalidValue(
            "portable Handle has no live Chromium route".to_owned(),
        )),
    )
}

fn foreign_handle(object_id: ObjectId) -> OperationError<HandleEvaluationErrorDetails> {
    OperationError::new(
        FailureKind::ProtocolViolation,
        format!("Handle `{object_id}` belongs to a different execution world"),
        Some(HandleEvaluationErrorDetails::InvalidValue(
            "Handle and target do not share one Frame, Page, CDP client, and execution world"
                .to_owned(),
        )),
    )
}

fn missing_frame_route(frame_id: ObjectId) -> OperationError<HandleEvaluationErrorDetails> {
    OperationError::new(
        FailureKind::TargetClosed,
        format!("Frame `{frame_id}` has no active Handle evaluation route"),
        Some(HandleEvaluationErrorDetails::InvalidBackendResponse(
            "portable Frame is absent from Chromium execution-world routes".to_owned(),
        )),
    )
}

fn map_frame_route_error(
    error: ChromiumHandleEvaluationRouteError,
) -> OperationError<HandleEvaluationErrorDetails> {
    OperationError::new(
        FailureKind::Cleanup,
        error.to_string(),
        Some(HandleEvaluationErrorDetails::InvalidBackendResponse(
            error.to_string(),
        )),
    )
}

fn map_route_error(
    error: ChromiumHandleRouteError,
) -> OperationError<HandleEvaluationErrorDetails> {
    let kind = match error {
        ChromiumHandleRouteError::CapacityExceeded { .. }
        | ChromiumHandleRouteError::NativeIdTooLong { .. }
        | ChromiumHandleRouteError::PreviewTooLong { .. }
        | ChromiumHandleRouteError::Resources(_) => FailureKind::ResourceExhausted,
        ChromiumHandleRouteError::Missing { .. } => FailureKind::TargetClosed,
        ChromiumHandleRouteError::StatePoisoned => FailureKind::Cleanup,
        _ => FailureKind::ProtocolViolation,
    };
    OperationError::new(
        kind,
        error.to_string(),
        Some(HandleEvaluationErrorDetails::InvalidBackendResponse(
            error.to_string(),
        )),
    )
}

fn map_runtime_error(error: RuntimeStateError) -> OperationError<HandleEvaluationErrorDetails> {
    let kind = match error {
        RuntimeStateError::TransportClosed => FailureKind::TransportClosed,
        RuntimeStateError::ObjectGraph(pwright_core::ObjectGraphError::CapacityExceeded {
            ..
        })
        | RuntimeStateError::Resource(_)
        | RuntimeStateError::ProductionResource(_) => FailureKind::ResourceExhausted,
        RuntimeStateError::ObjectGraph(_) => FailureKind::ProtocolViolation,
        _ => FailureKind::Cleanup,
    };
    OperationError::new(
        kind,
        format!("runtime could not publish Handle: {error}"),
        Some(HandleEvaluationErrorDetails::InvalidBackendResponse(
            error.to_string(),
        )),
    )
}

fn map_publication_error(
    error: ChromiumHandlePublicationError<RuntimeStateError>,
) -> OperationError<HandleEvaluationErrorDetails> {
    match error {
        ChromiumHandlePublicationError::Route(error) => map_route_error(error),
        ChromiumHandlePublicationError::Commit(error) => map_runtime_error(error),
    }
}

async fn finish_adopted_operation<T>(
    outcome: Result<T, OperationError<HandleEvaluationErrorDetails>>,
    adopted: Option<Box<dyn AdoptedArgumentBatch>>,
) -> Result<T, OperationError<HandleEvaluationErrorDetails>> {
    let cleanup = match adopted {
        Some(batch) => batch.release().await,
        None => Ok(()),
    };
    match (outcome, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Ok(_), Err(cleanup)) => Err(cleanup),
        (Err(primary), Ok(())) => Err(primary),
        (Err(primary), Err(cleanup)) => Err(combine_adoption_cleanup(primary, cleanup)),
    }
}

async fn finish_adopted_handle_creation(
    outcome: Result<CdpEvaluatedHandle, OperationError<HandleEvaluationErrorDetails>>,
    adopted: Option<Box<dyn AdoptedArgumentBatch>>,
) -> Result<CdpEvaluatedHandle, OperationError<HandleEvaluationErrorDetails>> {
    let cleanup = match adopted {
        Some(batch) => batch.release().await,
        None => Ok(()),
    };
    match (outcome, cleanup) {
        (Ok(native), Ok(())) => Ok(native),
        (Ok(native), Err(cleanup)) => release_failed(native, cleanup).await,
        (Err(primary), Ok(())) => Err(primary),
        (Err(primary), Err(cleanup)) => Err(combine_adoption_cleanup(primary, cleanup)),
    }
}

fn combine_adoption_cleanup(
    primary: OperationError<HandleEvaluationErrorDetails>,
    cleanup: OperationError<HandleEvaluationErrorDetails>,
) -> OperationError<HandleEvaluationErrorDetails> {
    OperationError::new(
        FailureKind::Cleanup,
        format!(
            "{}; temporary ElementHandle cleanup failed: {}",
            primary.message(),
            cleanup.message()
        ),
        Some(HandleEvaluationErrorDetails::Cleanup {
            primary: primary.to_string(),
            cleanup: cleanup.to_string(),
        }),
    )
}

async fn release_failed<T>(
    native: CdpEvaluatedHandle,
    primary: OperationError<HandleEvaluationErrorDetails>,
) -> Result<T, OperationError<HandleEvaluationErrorDetails>> {
    match native.release().await {
        Ok(()) => Err(primary),
        Err(cleanup) => Err(OperationError::new(
            FailureKind::Cleanup,
            format!("{}; Handle cleanup failed: {cleanup}", primary.message()),
            Some(HandleEvaluationErrorDetails::Cleanup {
                primary: primary.to_string(),
                cleanup: cleanup.to_string(),
            }),
        )),
    }
}

async fn release_failed_batch<T>(
    native: Vec<(String, CdpEvaluatedHandle)>,
    primary: OperationError<HandleEvaluationErrorDetails>,
) -> Result<T, OperationError<HandleEvaluationErrorDetails>> {
    let mut cleanup = None;
    for (_, handle) in native {
        if let Err(error) = handle.release().await
            && cleanup.is_none()
        {
            cleanup = Some(error.to_string());
        }
    }
    match cleanup {
        None => Err(primary),
        Some(cleanup) => Err(OperationError::new(
            FailureKind::Cleanup,
            format!(
                "{}; Handle batch cleanup failed: {cleanup}",
                primary.message()
            ),
            Some(HandleEvaluationErrorDetails::Cleanup {
                primary: primary.to_string(),
                cleanup,
            }),
        )),
    }
}

fn continuation_failure<Spec>(call_id: CallId, error: CleanupSubmitError) -> CommandResult<Spec>
where
    Spec: HandleCreationCommand,
{
    let kind = match error {
        CleanupSubmitError::CapacityExceeded { .. } => FailureKind::ResourceExhausted,
        CleanupSubmitError::Unavailable | CleanupSubmitError::StatePoisoned => FailureKind::Cleanup,
    };
    CommandResult::new(
        call_id,
        Err(OperationError::new(
            kind,
            format!(
                "{} continuation capacity is unavailable",
                Spec::operation_name()
            ),
            Some(HandleEvaluationErrorDetails::Cleanup {
                primary: format!("{} was rejected before CDP I/O", Spec::operation_name()),
                cleanup: error.to_string(),
            }),
        )),
    )
}
