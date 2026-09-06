//! Minimal Chromium CDP composition for explicit Page lifecycle.
//!
//! Ordinary close is a coordinated native/topology/runtime transaction.
//! Before-unload close is deliberately initiation-only and leaves portable
//! state live. Optional route cleanup is injected through one retirement
//! plugin, so this leaf does not link Page events, Handles, or networking.

use std::convert::Infallible;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

pub use pwright_backend_cdp_page_lifecycle::{
    CdpPageCloseLimits, CdpPageClosePoll, CdpPageCloseService, CdpPageCloseSetupError,
    CdpPageRunBeforeUnloadService, ImmediatePageClosePoll,
};
pub use pwright_backend_cdp_topology::{
    CdpTopology, CdpTopologyProductionError, CdpTopologyUpdate, ProductionCdpTopology,
};
use pwright_cap_page_lifecycle::{PageClose, PageLifecycleErrorDetails, PageRunBeforeUnload};
use pwright_cdp::{CdpClient, SessionFactory};
use pwright_codec_page_lifecycle::PageLifecycleCodec;
pub use pwright_codec_page_lifecycle::PageLifecycleCodecLimits;
use pwright_core::{
    CallId, CallMetadata, Command, CommandResult, FailureKind, ObjectId, OperationError, Service,
};
pub use pwright_core::{
    ProductionResourceError, ProductionResourceLedger, ProtocolVersion, ResourceLimits,
    ResourceRequirements, ResourceScope,
};
use pwright_runtime::{
    CleanupFailure, CleanupReservation, CleanupSubmitError, ControlledLifecycleHandle,
    ReservedCleanupExecutor, RuntimeStateError,
};
use pwright_server_page_lifecycle::{
    PageLifecycleServerHandlers, PageLifecycleServerServices, page_lifecycle_server_handlers,
};
pub use pwright_transport::FrameLimits;
use pwright_transport::WireProtocol;

pub type SharedChromiumPageTopology = Arc<Mutex<ProductionCdpTopology>>;

/// Native and wire bounds for the exact `pwright.page_lifecycle` 1.0 protocol.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumPageLifecycleProfile {
    backend: CdpPageCloseLimits,
    codec: PageLifecycleCodecLimits,
    send_limits: FrameLimits,
    receive_limits: FrameLimits,
}

impl ChromiumPageLifecycleProfile {
    pub const VERSION: ProtocolVersion = ProtocolVersion::new(1, 0);

    pub const fn new(
        backend: CdpPageCloseLimits,
        codec: PageLifecycleCodecLimits,
        send_limits: FrameLimits,
        receive_limits: FrameLimits,
    ) -> Self {
        Self {
            backend,
            codec,
            send_limits,
            receive_limits,
        }
    }

    pub const fn backend(self) -> CdpPageCloseLimits {
        self.backend
    }

    pub const fn codec(self) -> PageLifecycleCodecLimits {
        self.codec
    }

    pub const fn send_limits(self) -> FrameLimits {
        self.send_limits
    }

    pub const fn receive_limits(self) -> FrameLimits {
        self.receive_limits
    }

    pub fn requirements(self) -> ResourceRequirements {
        self.unbudgeted_protocol()
            .production_resource_requirements()
    }

    fn unbudgeted_protocol(self) -> WireProtocol<PageLifecycleCodec> {
        WireProtocol::new_exact_directional(
            PageLifecycleCodec::new(self.codec),
            Self::VERSION,
            self.send_limits,
            self.receive_limits,
        )
    }
}

#[derive(Debug)]
pub enum ChromiumPageLifecycleInstallError {
    Backend(CdpPageCloseSetupError),
    Resources(ProductionResourceError),
}

impl Display for ChromiumPageLifecycleInstallError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(error) => Display::fmt(error, formatter),
            Self::Resources(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ChromiumPageLifecycleInstallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Backend(error) => Some(error),
            Self::Resources(error) => Some(error),
        }
    }
}

impl From<CdpPageCloseSetupError> for ChromiumPageLifecycleInstallError {
    fn from(error: CdpPageCloseSetupError) -> Self {
        Self::Backend(error)
    }
}

impl From<ProductionResourceError> for ChromiumPageLifecycleInstallError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Resources(error)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumPageSessionRoute<'a> {
    page_id: ObjectId,
    target_id: &'a str,
    session_id: &'a str,
}

impl<'a> ChromiumPageSessionRoute<'a> {
    pub const fn page_id(self) -> ObjectId {
        self.page_id
    }

    pub const fn target_id(self) -> &'a str {
        self.target_id
    }

    pub const fn session_id(self) -> &'a str {
        self.session_id
    }
}

/// Resolves one already-validated Page route to its target-local CDP client.
pub trait ChromiumPageSessionResolver: Send + Sync {
    fn resolve(
        &self,
        route: ChromiumPageSessionRoute<'_>,
    ) -> Result<Arc<dyn CdpClient>, OperationError<PageLifecycleErrorDetails>>;
}

pub struct SessionFactoryChromiumPageResolver {
    sessions: Arc<dyn SessionFactory>,
}

impl SessionFactoryChromiumPageResolver {
    pub const fn new(sessions: Arc<dyn SessionFactory>) -> Self {
        Self { sessions }
    }
}

impl ChromiumPageSessionResolver for SessionFactoryChromiumPageResolver {
    fn resolve(
        &self,
        route: ChromiumPageSessionRoute<'_>,
    ) -> Result<Arc<dyn CdpClient>, OperationError<PageLifecycleErrorDetails>> {
        Ok(self
            .sessions
            .create_session(route.session_id.to_owned(), route.target_id.to_owned()))
    }
}

pub struct ChromiumPageRetirementPlan<'a> {
    page_id: ObjectId,
    target_id: &'a str,
    session_id: &'a str,
    topology: &'a CdpTopology,
    topology_update: &'a CdpTopologyUpdate,
    disposal_order: &'a [ObjectId],
}

impl<'a> ChromiumPageRetirementPlan<'a> {
    pub const fn page_id(&self) -> ObjectId {
        self.page_id
    }

    pub const fn target_id(&self) -> &str {
        self.target_id
    }

    pub const fn session_id(&self) -> &str {
        self.session_id
    }

    pub const fn topology(&self) -> &CdpTopology {
        self.topology
    }

    pub const fn topology_update(&self) -> &CdpTopologyUpdate {
        self.topology_update
    }

    pub const fn disposal_order(&self) -> &[ObjectId] {
        self.disposal_order
    }
}

pub trait PreparedChromiumPageRetirement: Send {
    fn commit(self: Box<Self>);
}

pub trait ChromiumPageRetirementPlugin: Send + Sync {
    fn prepare(
        &self,
        plan: ChromiumPageRetirementPlan<'_>,
    ) -> Result<Box<dyn PreparedChromiumPageRetirement>, OperationError<PageLifecycleErrorDetails>>;
}

#[derive(Default)]
pub struct NoopChromiumPageRetirement;

struct PreparedNoopChromiumPageRetirement;

impl PreparedChromiumPageRetirement for PreparedNoopChromiumPageRetirement {
    fn commit(self: Box<Self>) {}
}

impl ChromiumPageRetirementPlugin for NoopChromiumPageRetirement {
    fn prepare(
        &self,
        _plan: ChromiumPageRetirementPlan<'_>,
    ) -> Result<Box<dyn PreparedChromiumPageRetirement>, OperationError<PageLifecycleErrorDetails>>
    {
        Ok(Box::new(PreparedNoopChromiumPageRetirement))
    }
}

/// Native and local ownership transaction for ordinary `Page.close`.
#[derive(Clone)]
pub struct ChromiumPageCloseService {
    browser_client: Arc<dyn CdpClient>,
    topology: SharedChromiumPageTopology,
    runtime: ControlledLifecycleHandle,
    retirement: Arc<dyn ChromiumPageRetirementPlugin>,
    backend_limits: CdpPageCloseLimits,
    poll: Arc<dyn CdpPageClosePoll>,
}

impl ChromiumPageCloseService {
    pub fn new(
        browser_client: Arc<dyn CdpClient>,
        topology: SharedChromiumPageTopology,
        runtime: ControlledLifecycleHandle,
        retirement: Arc<dyn ChromiumPageRetirementPlugin>,
        backend_limits: CdpPageCloseLimits,
        poll: Arc<dyn CdpPageClosePoll>,
    ) -> Self {
        Self {
            browser_client,
            topology,
            runtime,
            retirement,
            backend_limits,
            poll,
        }
    }

    pub fn topology(&self) -> SharedChromiumPageTopology {
        Arc::clone(&self.topology)
    }

    async fn close(
        &self,
        object_id: ObjectId,
        metadata: CallMetadata,
    ) -> Result<(), OperationError<PageLifecycleErrorDetails>> {
        validate_metadata(metadata, "Page.close")?;
        validate_live_page(&self.runtime, object_id)?;

        let prepared_topology = {
            let mut topology = self.topology.lock().map_err(|_| topology_poisoned())?;
            topology
                .prepare_page_close(object_id)
                .map_err(map_topology_error)?
        };
        let prepared_runtime = self
            .runtime
            .prepare_object_disposal(object_id)
            .map_err(map_runtime_error)?;
        let prepared_retirement = self.retirement.prepare(ChromiumPageRetirementPlan {
            page_id: object_id,
            target_id: prepared_topology.target_id(),
            session_id: prepared_topology.session_id(),
            topology: prepared_topology.topology(),
            topology_update: prepared_topology.update(),
            disposal_order: prepared_runtime.disposal_order(),
        })?;
        let native_close = CdpPageCloseService::new(
            Arc::clone(&self.browser_client),
            prepared_topology.target_id(),
            self.backend_limits,
            Arc::clone(&self.poll),
        )
        .map_err(map_backend_setup_error)?;

        native_close.call(()).await?;
        {
            let mut topology = self.topology.lock().map_err(|_| topology_poisoned())?;
            prepared_topology
                .commit(&mut topology)
                .map_err(map_topology_error)?;
        }
        prepared_runtime.commit().map_err(map_runtime_error)?;
        prepared_retirement.commit();
        Ok(())
    }
}

impl Service<Command<PageClose>> for ChromiumPageCloseService {
    type Response = CommandResult<PageClose>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<PageClose>) -> Self::Future<'_> {
        let (call_id, object_id, metadata, ()) = command.into_parts();
        Box::pin(async move {
            Ok(CommandResult::new(
                call_id,
                self.close(object_id, metadata).await,
            ))
        })
    }
}

/// Initiates `beforeunload` in the exact Page session without retiring state.
#[derive(Clone)]
pub struct ChromiumPageRunBeforeUnloadService {
    topology: SharedChromiumPageTopology,
    runtime: ControlledLifecycleHandle,
    resolver: Arc<dyn ChromiumPageSessionResolver>,
}

impl ChromiumPageRunBeforeUnloadService {
    pub fn new(
        topology: SharedChromiumPageTopology,
        runtime: ControlledLifecycleHandle,
        resolver: Arc<dyn ChromiumPageSessionResolver>,
    ) -> Self {
        Self {
            topology,
            runtime,
            resolver,
        }
    }

    async fn run(
        &self,
        object_id: ObjectId,
        metadata: CallMetadata,
    ) -> Result<(), OperationError<PageLifecycleErrorDetails>> {
        validate_metadata(metadata, "Page.run_before_unload")?;
        validate_live_page(&self.runtime, object_id)?;
        let route = resolve_page_route(&self.topology, object_id)?;
        let page_client = self.resolver.resolve(route.as_borrowed())?;
        CdpPageRunBeforeUnloadService::new(page_client)
            .call(())
            .await
    }
}

impl Service<Command<PageRunBeforeUnload>> for ChromiumPageRunBeforeUnloadService {
    type Response = CommandResult<PageRunBeforeUnload>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<PageRunBeforeUnload>) -> Self::Future<'_> {
        let (call_id, object_id, metadata, ()) = command.into_parts();
        Box::pin(async move {
            Ok(CommandResult::new(
                call_id,
                self.run(object_id, metadata).await,
            ))
        })
    }
}

struct OwnedChromiumPageSessionRoute {
    page_id: ObjectId,
    target_id: String,
    session_id: String,
}

impl OwnedChromiumPageSessionRoute {
    fn as_borrowed(&self) -> ChromiumPageSessionRoute<'_> {
        ChromiumPageSessionRoute {
            page_id: self.page_id,
            target_id: &self.target_id,
            session_id: &self.session_id,
        }
    }
}

type PageCloseFutureOutput = Result<CommandResult<PageClose>, Infallible>;
type BoxPageCloseFuture = Pin<Box<dyn Future<Output = PageCloseFutureOutput> + Send + 'static>>;

pub struct ContinuablePageCloseFuture<Reservation>
where
    Reservation: CleanupReservation,
{
    inner: Option<BoxPageCloseFuture>,
    reservation: Option<Reservation>,
    started: bool,
}

impl<Reservation> Unpin for ContinuablePageCloseFuture<Reservation> where
    Reservation: CleanupReservation
{
}

impl<Reservation> Future for ContinuablePageCloseFuture<Reservation>
where
    Reservation: CleanupReservation,
{
    type Output = PageCloseFutureOutput;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.started = true;
        let outcome = self
            .inner
            .as_mut()
            .expect("completed Page close future must not be polled again")
            .as_mut()
            .poll(context);
        if outcome.is_ready() {
            self.inner = None;
            self.reservation = None;
        }
        outcome
    }
}

impl<Reservation> Drop for ContinuablePageCloseFuture<Reservation>
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
                    "abandoned Page.close continuation failed",
                )),
                Err(never) => match never {},
            }
        }));
    }
}

#[derive(Clone)]
pub struct ContinuablePageCloseService<Continuation> {
    service: Arc<ChromiumPageCloseService>,
    continuation: Continuation,
}

impl<Continuation> ContinuablePageCloseService<Continuation> {
    pub const fn new(service: Arc<ChromiumPageCloseService>, continuation: Continuation) -> Self {
        Self {
            service,
            continuation,
        }
    }

    pub const fn service(&self) -> &Arc<ChromiumPageCloseService> {
        &self.service
    }
}

impl<Continuation> Service<Command<PageClose>> for ContinuablePageCloseService<Continuation>
where
    Continuation: ReservedCleanupExecutor,
{
    type Response = CommandResult<PageClose>;
    type Error = Infallible;
    type Future<'a>
        = ContinuablePageCloseFuture<Continuation::Reservation>
    where
        Self: 'a;

    fn call(&self, command: Command<PageClose>) -> Self::Future<'_> {
        let call_id = command.call_id();
        let reservation = match self.continuation.try_reserve() {
            Ok(reservation) => reservation,
            Err(error) => {
                return ContinuablePageCloseFuture {
                    inner: Some(Box::pin(std::future::ready(Ok(continuation_failure(
                        call_id, error,
                    ))))),
                    reservation: None,
                    started: false,
                };
            }
        };
        let service = Arc::clone(&self.service);
        ContinuablePageCloseFuture {
            inner: Some(Box::pin(async move { service.call(command).await })),
            reservation: Some(reservation),
            started: false,
        }
    }
}

pub type ChromiumPageLifecycleHandlers<Continuation> = PageLifecycleServerHandlers<
    ContinuablePageCloseService<Continuation>,
    ChromiumPageRunBeforeUnloadService,
>;

pub struct ChromiumPageLifecycleInstallation<Continuation>
where
    Continuation: ReservedCleanupExecutor,
{
    close: Arc<ContinuablePageCloseService<Continuation>>,
    run_before_unload: Arc<ChromiumPageRunBeforeUnloadService>,
    protocol: Arc<WireProtocol<PageLifecycleCodec>>,
}

/// All caller-owned dependencies for the task-free Page-lifecycle leaf.
pub struct ChromiumPageLifecycleBindings<Continuation> {
    pub browser_client: Arc<dyn CdpClient>,
    pub session_resolver: Arc<dyn ChromiumPageSessionResolver>,
    pub topology: SharedChromiumPageTopology,
    pub runtime: ControlledLifecycleHandle,
    pub retirement: Arc<dyn ChromiumPageRetirementPlugin>,
    pub close_poll: Arc<dyn CdpPageClosePoll>,
    pub continuation: Continuation,
}

impl<Continuation> ChromiumPageLifecycleInstallation<Continuation>
where
    Continuation: ReservedCleanupExecutor,
{
    pub fn install(
        bindings: ChromiumPageLifecycleBindings<Continuation>,
        profile: ChromiumPageLifecycleProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumPageLifecycleInstallError> {
        let backend_limits = profile.backend().validate()?;
        let protocol = profile
            .unbudgeted_protocol()
            .with_production_resources(resources)?;
        let close = ChromiumPageCloseService::new(
            Arc::clone(&bindings.browser_client),
            Arc::clone(&bindings.topology),
            bindings.runtime.clone(),
            bindings.retirement,
            backend_limits,
            bindings.close_poll,
        );
        let run_before_unload = ChromiumPageRunBeforeUnloadService::new(
            bindings.topology,
            bindings.runtime,
            bindings.session_resolver,
        );
        Ok(Self {
            close: Arc::new(ContinuablePageCloseService::new(
                Arc::new(close),
                bindings.continuation,
            )),
            run_before_unload: Arc::new(run_before_unload),
            protocol: Arc::new(protocol),
        })
    }

    pub fn close_service(&self) -> Arc<ContinuablePageCloseService<Continuation>> {
        Arc::clone(&self.close)
    }

    pub fn run_before_unload_service(&self) -> Arc<ChromiumPageRunBeforeUnloadService> {
        Arc::clone(&self.run_before_unload)
    }

    pub fn protocol(&self) -> Arc<WireProtocol<PageLifecycleCodec>> {
        Arc::clone(&self.protocol)
    }

    pub fn handlers(&self) -> ChromiumPageLifecycleHandlers<Continuation> {
        page_lifecycle_server_handlers(
            Arc::clone(&self.protocol),
            PageLifecycleServerServices {
                page_close: Arc::clone(&self.close),
                page_run_before_unload: Arc::clone(&self.run_before_unload),
            },
        )
    }
}

fn resolve_page_route(
    topology: &SharedChromiumPageTopology,
    object_id: ObjectId,
) -> Result<OwnedChromiumPageSessionRoute, OperationError<PageLifecycleErrorDetails>> {
    let topology = topology.lock().map_err(|_| topology_poisoned())?;
    let route = topology
        .topology()
        .page_session_routes()
        .find(|route| route.object_id() == object_id)
        .ok_or_else(|| {
            OperationError::new(
                FailureKind::TargetClosed,
                format!("Page `{object_id}` has no live Chromium session"),
                None,
            )
        })?;
    Ok(OwnedChromiumPageSessionRoute {
        page_id: object_id,
        target_id: route.target_id().to_owned(),
        session_id: route.session_id().to_owned(),
    })
}

fn validate_metadata(
    metadata: CallMetadata,
    method: &'static str,
) -> Result<(), OperationError<PageLifecycleErrorDetails>> {
    if metadata == CallMetadata::default() {
        return Ok(());
    }
    Err(OperationError::new(
        FailureKind::ProtocolViolation,
        format!("{method} does not accept timeout or cancellation metadata"),
        None,
    ))
}

fn validate_live_page(
    runtime: &ControlledLifecycleHandle,
    object_id: ObjectId,
) -> Result<(), OperationError<PageLifecycleErrorDetails>> {
    match runtime.is_object_live(object_id) {
        Ok(true) => Ok(()),
        Ok(false) => Err(OperationError::new(
            FailureKind::TargetClosed,
            format!("Page `{object_id}` is already closed"),
            None,
        )),
        Err(error) => Err(map_runtime_error(error)),
    }
}

fn continuation_failure(call_id: CallId, error: CleanupSubmitError) -> CommandResult<PageClose> {
    let kind = match error {
        CleanupSubmitError::CapacityExceeded { .. } => FailureKind::ResourceExhausted,
        CleanupSubmitError::Unavailable | CleanupSubmitError::StatePoisoned => FailureKind::Cleanup,
    };
    CommandResult::new(
        call_id,
        Err(OperationError::new(
            kind,
            "Page.close continuation capacity is unavailable",
            Some(PageLifecycleErrorDetails::Backend(
                "close was rejected before browser interaction".to_string(),
            )),
        )),
    )
}

fn map_backend_setup_error(
    error: CdpPageCloseSetupError,
) -> OperationError<PageLifecycleErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        format!("Page close route exceeds configured native bounds: {error}"),
        Some(PageLifecycleErrorDetails::InvalidBackendResponse(
            error.to_string(),
        )),
    )
}

fn map_runtime_error(error: RuntimeStateError) -> OperationError<PageLifecycleErrorDetails> {
    let kind = match &error {
        RuntimeStateError::TransportClosed => FailureKind::TransportClosed,
        RuntimeStateError::LifecycleTransactionInProgress
        | RuntimeStateError::LifecycleTransactionIdExhausted
        | RuntimeStateError::ControlMessageCapacityExceeded { .. }
        | RuntimeStateError::ProductionResource(_)
        | RuntimeStateError::Resource(_) => FailureKind::ResourceExhausted,
        _ => FailureKind::Cleanup,
    };
    OperationError::new(
        kind,
        format!("runtime could not process Page lifecycle: {error}"),
        Some(PageLifecycleErrorDetails::Backend(error.to_string())),
    )
}

fn map_topology_error(
    error: CdpTopologyProductionError,
) -> OperationError<PageLifecycleErrorDetails> {
    let kind = match &error {
        CdpTopologyProductionError::TransactionInProgress
        | CdpTopologyProductionError::TransactionIdExhausted
        | CdpTopologyProductionError::ProductionResource(_)
        | CdpTopologyProductionError::Resource(_) => FailureKind::ResourceExhausted,
        _ => FailureKind::Cleanup,
    };
    OperationError::new(
        kind,
        format!("Chromium topology could not process Page lifecycle: {error}"),
        Some(PageLifecycleErrorDetails::InvalidBackendResponse(
            error.to_string(),
        )),
    )
}

fn topology_poisoned() -> OperationError<PageLifecycleErrorDetails> {
    OperationError::new(
        FailureKind::Cleanup,
        "Chromium topology state is unavailable during Page lifecycle",
        Some(PageLifecycleErrorDetails::Backend(
            "topology mutex is poisoned".to_string(),
        )),
    )
}
