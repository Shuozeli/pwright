//! Minimal Chromium CDP composition for explicit BrowserContext lifecycle.
//!
//! This leaf coordinates one caller-owned topology and runtime with the native
//! `Target.disposeBrowserContext` operation. Optional Page-route, Handle, and
//! network cleanup is supplied through one retirement plugin, so selecting
//! context lifecycle does not link those capabilities. It discovers, launches,
//! downloads, and owns no browser and starts no task or executor.

use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

pub use pwright_backend_cdp_context_lifecycle::{
    CdpBrowserContextCloseService, CdpContextCloseLimits, CdpContextCloseSetupError,
};
pub use pwright_backend_cdp_topology::{
    CdpContextOwnership, CdpTopology, CdpTopologyProductionError, CdpTopologyUpdate,
    ProductionCdpTopology,
};
use pwright_cap_context_lifecycle::{BrowserContextClose, ContextLifecycleErrorDetails};
use pwright_cdp::CdpClient;
use pwright_codec_context_lifecycle::ContextLifecycleCodec;
pub use pwright_codec_context_lifecycle::ContextLifecycleCodecLimits;
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
use pwright_server_context_lifecycle::{
    ContextLifecycleServerHandlers, ContextLifecycleServerServices,
    context_lifecycle_server_handlers,
};
pub use pwright_transport::FrameLimits;
use pwright_transport::WireProtocol;

pub type SharedChromiumContextTopology = Arc<Mutex<ProductionCdpTopology>>;

/// Wire bounds for the exact `pwright.context_lifecycle` 1.0 protocol.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumContextLifecycleProfile {
    codec: ContextLifecycleCodecLimits,
    send_limits: FrameLimits,
    receive_limits: FrameLimits,
}

impl ChromiumContextLifecycleProfile {
    pub const VERSION: ProtocolVersion = ProtocolVersion::new(1, 0);

    pub const fn new(
        codec: ContextLifecycleCodecLimits,
        send_limits: FrameLimits,
        receive_limits: FrameLimits,
    ) -> Self {
        Self {
            codec,
            send_limits,
            receive_limits,
        }
    }

    pub const fn codec(self) -> ContextLifecycleCodecLimits {
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

    fn unbudgeted_protocol(self) -> WireProtocol<ContextLifecycleCodec> {
        WireProtocol::new_exact_directional(
            ContextLifecycleCodec::new(self.codec),
            Self::VERSION,
            self.send_limits,
            self.receive_limits,
        )
    }
}

/// Immutable inputs offered to an optional descendant-retirement plugin.
///
/// The topology and runtime transactions are still staged when this callback
/// runs. A plugin must reserve every fallible resource in `prepare`; its
/// returned commit token is intentionally infallible.
pub struct ChromiumContextRetirementPlan<'a> {
    context_id: ObjectId,
    topology: &'a CdpTopology,
    topology_update: &'a CdpTopologyUpdate,
    disposal_order: &'a [ObjectId],
}

impl<'a> ChromiumContextRetirementPlan<'a> {
    pub const fn context_id(&self) -> ObjectId {
        self.context_id
    }

    pub const fn topology(&self) -> &'a CdpTopology {
        self.topology
    }

    pub const fn topology_update(&self) -> &'a CdpTopologyUpdate {
        self.topology_update
    }

    pub const fn disposal_order(&self) -> &'a [ObjectId] {
        self.disposal_order
    }
}

/// A fully prepared plugin transition whose publication cannot fail.
pub trait PreparedChromiumContextRetirement: Send {
    fn commit(self: Box<Self>);
}

/// Optional cleanup authority for state not owned by topology or runtime.
///
/// Applications can use this for Page-session routes, Handle routes, network
/// lanes, or a composite of several such owners. The leaf retains exactly one
/// plugin pointer and therefore does not need to depend on any plugin's crate.
pub trait ChromiumContextRetirementPlugin: Send + Sync {
    fn prepare(
        &self,
        plan: ChromiumContextRetirementPlan<'_>,
    ) -> Result<
        Box<dyn PreparedChromiumContextRetirement>,
        OperationError<ContextLifecycleErrorDetails>,
    >;
}

#[derive(Default)]
pub struct NoopChromiumContextRetirement;

struct PreparedNoopChromiumContextRetirement;

impl PreparedChromiumContextRetirement for PreparedNoopChromiumContextRetirement {
    fn commit(self: Box<Self>) {}
}

impl ChromiumContextRetirementPlugin for NoopChromiumContextRetirement {
    fn prepare(
        &self,
        _plan: ChromiumContextRetirementPlan<'_>,
    ) -> Result<
        Box<dyn PreparedChromiumContextRetirement>,
        OperationError<ContextLifecycleErrorDetails>,
    > {
        Ok(Box::new(PreparedNoopChromiumContextRetirement))
    }
}

/// Native and local ownership transaction for `BrowserContext.close`.
#[derive(Clone)]
pub struct ChromiumBrowserContextCloseService {
    browser_client: Arc<dyn CdpClient>,
    topology: SharedChromiumContextTopology,
    runtime: ControlledLifecycleHandle,
    retirement: Arc<dyn ChromiumContextRetirementPlugin>,
}

impl ChromiumBrowserContextCloseService {
    pub fn new(
        browser_client: Arc<dyn CdpClient>,
        topology: SharedChromiumContextTopology,
        runtime: ControlledLifecycleHandle,
        retirement: Arc<dyn ChromiumContextRetirementPlugin>,
    ) -> Self {
        Self {
            browser_client,
            topology,
            runtime,
            retirement,
        }
    }

    pub fn topology(&self) -> SharedChromiumContextTopology {
        Arc::clone(&self.topology)
    }

    async fn close(
        &self,
        object_id: ObjectId,
        metadata: CallMetadata,
    ) -> Result<(), OperationError<ContextLifecycleErrorDetails>> {
        if metadata != CallMetadata::default() {
            return Err(OperationError::new(
                FailureKind::ProtocolViolation,
                "BrowserContext.close does not accept timeout or cancellation metadata",
                None,
            ));
        }
        match self.runtime.is_object_live(object_id) {
            Ok(true) => {}
            Ok(false) => {
                return Err(OperationError::new(
                    FailureKind::TargetClosed,
                    format!("BrowserContext `{object_id}` is already closed"),
                    None,
                ));
            }
            Err(error) => return Err(map_runtime_error(error)),
        }

        let prepared_topology = {
            let mut topology = self.topology.lock().map_err(|_| topology_poisoned())?;
            topology
                .prepare_browser_context_close(object_id)
                .map_err(map_topology_error)?
        };
        if prepared_topology.ownership() != CdpContextOwnership::Owned {
            return CdpBrowserContextCloseService::close_route(
                self.browser_client.as_ref(),
                prepared_topology.browser_context_id(),
                prepared_topology.ownership(),
            )
            .await;
        }

        let prepared_runtime = self
            .runtime
            .prepare_object_disposal(object_id)
            .map_err(map_runtime_error)?;
        let prepared_retirement = self.retirement.prepare(ChromiumContextRetirementPlan {
            context_id: object_id,
            topology: prepared_topology.topology(),
            topology_update: prepared_topology.update(),
            disposal_order: prepared_runtime.disposal_order(),
        })?;

        CdpBrowserContextCloseService::close_route(
            self.browser_client.as_ref(),
            prepared_topology.browser_context_id(),
            prepared_topology.ownership(),
        )
        .await?;

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

impl Service<Command<BrowserContextClose>> for ChromiumBrowserContextCloseService {
    type Response = CommandResult<BrowserContextClose>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<BrowserContextClose>) -> Self::Future<'_> {
        let (call_id, object_id, metadata, ()) = command.into_parts();
        Box::pin(async move {
            Ok(CommandResult::new(
                call_id,
                self.close(object_id, metadata).await,
            ))
        })
    }
}

type ContextCloseFutureOutput = Result<CommandResult<BrowserContextClose>, Infallible>;
type BoxContextCloseFuture =
    Pin<Box<dyn Future<Output = ContextCloseFutureOutput> + Send + 'static>>;

/// Admitted close whose remaining state transfers to a bounded caller-owned
/// continuation queue if its polled waiter is dropped.
pub struct ContinuableContextCloseFuture<Reservation>
where
    Reservation: CleanupReservation,
{
    inner: Option<BoxContextCloseFuture>,
    reservation: Option<Reservation>,
    started: bool,
}

impl<Reservation> Unpin for ContinuableContextCloseFuture<Reservation> where
    Reservation: CleanupReservation
{
}

impl<Reservation> Future for ContinuableContextCloseFuture<Reservation>
where
    Reservation: CleanupReservation,
{
    type Output = ContextCloseFutureOutput;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.started = true;
        let outcome = self
            .inner
            .as_mut()
            .expect("completed context close future must not be polled again")
            .as_mut()
            .poll(context);
        if outcome.is_ready() {
            self.inner = None;
            self.reservation = None;
        }
        outcome
    }
}

impl<Reservation> Drop for ContinuableContextCloseFuture<Reservation>
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
                    "abandoned BrowserContext.close continuation failed",
                )),
                Err(never) => match never {},
            }
        }));
    }
}

#[derive(Clone)]
pub struct ContinuableBrowserContextCloseService<Continuation> {
    service: Arc<ChromiumBrowserContextCloseService>,
    continuation: Continuation,
}

impl<Continuation> ContinuableBrowserContextCloseService<Continuation> {
    pub const fn new(
        service: Arc<ChromiumBrowserContextCloseService>,
        continuation: Continuation,
    ) -> Self {
        Self {
            service,
            continuation,
        }
    }

    pub const fn service(&self) -> &Arc<ChromiumBrowserContextCloseService> {
        &self.service
    }
}

impl<Continuation> Service<Command<BrowserContextClose>>
    for ContinuableBrowserContextCloseService<Continuation>
where
    Continuation: ReservedCleanupExecutor,
{
    type Response = CommandResult<BrowserContextClose>;
    type Error = Infallible;
    type Future<'a>
        = ContinuableContextCloseFuture<Continuation::Reservation>
    where
        Self: 'a;

    fn call(&self, command: Command<BrowserContextClose>) -> Self::Future<'_> {
        let call_id = command.call_id();
        let reservation = match self.continuation.try_reserve() {
            Ok(reservation) => reservation,
            Err(error) => {
                return ContinuableContextCloseFuture {
                    inner: Some(Box::pin(std::future::ready(Ok(continuation_failure(
                        call_id, error,
                    ))))),
                    reservation: None,
                    started: false,
                };
            }
        };
        let service = Arc::clone(&self.service);
        ContinuableContextCloseFuture {
            inner: Some(Box::pin(async move { service.call(command).await })),
            reservation: Some(reservation),
            started: false,
        }
    }
}

pub type ChromiumContextLifecycleHandlers<Continuation> =
    ContextLifecycleServerHandlers<ContinuableBrowserContextCloseService<Continuation>>;

/// Complete task-free Context-lifecycle server leaf.
pub struct ChromiumContextLifecycleInstallation<Continuation>
where
    Continuation: ReservedCleanupExecutor,
{
    service: Arc<ContinuableBrowserContextCloseService<Continuation>>,
    protocol: Arc<WireProtocol<ContextLifecycleCodec>>,
}

impl<Continuation> ChromiumContextLifecycleInstallation<Continuation>
where
    Continuation: ReservedCleanupExecutor,
{
    pub fn install(
        browser_client: Arc<dyn CdpClient>,
        topology: SharedChromiumContextTopology,
        runtime: ControlledLifecycleHandle,
        retirement: Arc<dyn ChromiumContextRetirementPlugin>,
        continuation: Continuation,
        profile: ChromiumContextLifecycleProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        let protocol = profile
            .unbudgeted_protocol()
            .with_production_resources(resources)?;
        let service =
            ChromiumBrowserContextCloseService::new(browser_client, topology, runtime, retirement);
        Ok(Self {
            service: Arc::new(ContinuableBrowserContextCloseService::new(
                Arc::new(service),
                continuation,
            )),
            protocol: Arc::new(protocol),
        })
    }

    pub fn install_without_retirement_plugins(
        browser_client: Arc<dyn CdpClient>,
        topology: SharedChromiumContextTopology,
        runtime: ControlledLifecycleHandle,
        continuation: Continuation,
        profile: ChromiumContextLifecycleProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        Self::install(
            browser_client,
            topology,
            runtime,
            Arc::new(NoopChromiumContextRetirement),
            continuation,
            profile,
            resources,
        )
    }

    pub fn service(&self) -> Arc<ContinuableBrowserContextCloseService<Continuation>> {
        Arc::clone(&self.service)
    }

    pub fn protocol(&self) -> Arc<WireProtocol<ContextLifecycleCodec>> {
        Arc::clone(&self.protocol)
    }

    pub fn handlers(&self) -> ChromiumContextLifecycleHandlers<Continuation> {
        context_lifecycle_server_handlers(
            Arc::clone(&self.protocol),
            ContextLifecycleServerServices {
                browser_context_close: Arc::clone(&self.service),
            },
        )
    }
}

fn continuation_failure(
    call_id: CallId,
    error: CleanupSubmitError,
) -> CommandResult<BrowserContextClose> {
    let kind = match error {
        CleanupSubmitError::CapacityExceeded { .. } => FailureKind::ResourceExhausted,
        CleanupSubmitError::Unavailable | CleanupSubmitError::StatePoisoned => FailureKind::Cleanup,
    };
    CommandResult::new(
        call_id,
        Err(OperationError::new(
            kind,
            "BrowserContext.close continuation capacity is unavailable",
            Some(ContextLifecycleErrorDetails::Backend(
                "close was rejected before browser interaction".to_string(),
            )),
        )),
    )
}

fn map_runtime_error(error: RuntimeStateError) -> OperationError<ContextLifecycleErrorDetails> {
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
        format!("runtime could not close BrowserContext: {error}"),
        Some(ContextLifecycleErrorDetails::Backend(error.to_string())),
    )
}

fn map_topology_error(
    error: CdpTopologyProductionError,
) -> OperationError<ContextLifecycleErrorDetails> {
    let kind = match &error {
        CdpTopologyProductionError::TransactionInProgress
        | CdpTopologyProductionError::TransactionIdExhausted
        | CdpTopologyProductionError::ProductionResource(_)
        | CdpTopologyProductionError::Resource(_) => FailureKind::ResourceExhausted,
        _ => FailureKind::Cleanup,
    };
    OperationError::new(
        kind,
        format!("Chromium topology could not close BrowserContext: {error}"),
        Some(ContextLifecycleErrorDetails::InvalidBackendResponse(
            error.to_string(),
        )),
    )
}

fn topology_poisoned() -> OperationError<ContextLifecycleErrorDetails> {
    OperationError::new(
        FailureKind::Cleanup,
        "Chromium topology state is unavailable during BrowserContext.close",
        Some(ContextLifecycleErrorDetails::Backend(
            "topology mutex is poisoned".to_string(),
        )),
    )
}
