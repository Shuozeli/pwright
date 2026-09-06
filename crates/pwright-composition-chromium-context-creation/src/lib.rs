//! Minimal Chromium CDP composition for BrowserContext creation and lifecycle.
//!
//! Context creation explicitly carries its portable Context-lifecycle
//! dependency. The leaf publishes one generated creation handler plus the
//! paired close handler, but owns no browser process, downloader, provider,
//! transport driver, task, or executor.

use std::convert::Infallible;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

pub use pwright_backend_cdp_context_creation::{
    CdpBrowserContextCreationService, CdpContextCreationLimits, CdpContextCreationSetupError,
    CdpCreatedBrowserContext,
};
use pwright_backend_cdp_topology::{
    CdpContextOwnership, CdpTopologyEffect, CdpTopologyError, CdpTopologyProductionError,
};
pub use pwright_browser_model::{BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND};
use pwright_cap_context_creation::{
    BrowserContextInitializer, BrowserNewContext, ContextCreationErrorDetails,
};
use pwright_cdp::CdpClient;
use pwright_codec_context_creation::ContextCreationCodec;
pub use pwright_codec_context_creation::ContextCreationCodecLimits;
pub use pwright_composition_chromium_context_lifecycle::{
    ChromiumBrowserContextCloseService, ChromiumContextLifecycleHandlers,
    ChromiumContextLifecycleInstallation, ChromiumContextLifecycleProfile,
    ChromiumContextRetirementPlugin, ContextLifecycleCodecLimits, NoopChromiumContextRetirement,
    PreparedChromiumContextRetirement, SharedChromiumContextTopology,
};
pub use pwright_core::{
    AtomicObjectIdSource, ProductionResourceError, ProductionResourceLedger, ProtocolVersion,
    ResourceLimits, ResourceRequirements, ResourceScope,
};
use pwright_core::{
    CallId, CallMetadata, Command, CommandResult, ControlMessage, FailureKind, ObjectGraphError,
    ObjectId, ObjectIdSource, OperationError, Service,
};
use pwright_runtime::{
    CleanupFailure, CleanupReservation, CleanupSubmitError, ControlledLifecycleHandle,
    ReservedCleanupExecutor, RuntimeStateError,
};
use pwright_server_context_creation::{
    ContextCreationServerHandlers, ContextCreationServerServices, context_creation_server_handlers,
};
pub use pwright_transport::FrameLimits;
use pwright_transport::WireProtocol;

/// Backend, creation-wire, and required Context-lifecycle bounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumContextCreationProfile {
    backend: CdpContextCreationLimits,
    codec: ContextCreationCodecLimits,
    send_limits: FrameLimits,
    receive_limits: FrameLimits,
    lifecycle: ChromiumContextLifecycleProfile,
}

impl ChromiumContextCreationProfile {
    pub const VERSION: ProtocolVersion = ProtocolVersion::new(1, 0);

    pub const fn new(
        backend: CdpContextCreationLimits,
        codec: ContextCreationCodecLimits,
        send_limits: FrameLimits,
        receive_limits: FrameLimits,
        lifecycle: ChromiumContextLifecycleProfile,
    ) -> Self {
        Self {
            backend,
            codec,
            send_limits,
            receive_limits,
            lifecycle,
        }
    }

    pub const fn backend(self) -> CdpContextCreationLimits {
        self.backend
    }

    pub const fn codec(self) -> ContextCreationCodecLimits {
        self.codec
    }

    pub const fn send_limits(self) -> FrameLimits {
        self.send_limits
    }

    pub const fn receive_limits(self) -> FrameLimits {
        self.receive_limits
    }

    pub const fn lifecycle(self) -> ChromiumContextLifecycleProfile {
        self.lifecycle
    }

    /// Both protocol tables remain installed simultaneously.
    pub fn requirements(self) -> ResourceRequirements {
        self.unbudgeted_protocol()
            .production_resource_requirements()
            .combine_concurrent(self.lifecycle.requirements())
    }

    fn unbudgeted_protocol(self) -> WireProtocol<ContextCreationCodec> {
        WireProtocol::new_exact_directional(
            ContextCreationCodec::new(self.codec),
            Self::VERSION,
            self.send_limits,
            self.receive_limits,
        )
    }
}

/// Why the paired creation/lifecycle leaf could not be installed.
#[derive(Debug)]
pub enum ChromiumContextCreationInstallError {
    Backend(CdpContextCreationSetupError),
    Resources(ProductionResourceError),
}

impl Display for ChromiumContextCreationInstallError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(error) => Display::fmt(error, formatter),
            Self::Resources(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ChromiumContextCreationInstallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Backend(error) => Some(error),
            Self::Resources(error) => Some(error),
        }
    }
}

impl From<CdpContextCreationSetupError> for ChromiumContextCreationInstallError {
    fn from(error: CdpContextCreationSetupError) -> Self {
        Self::Backend(error)
    }
}

impl From<ProductionResourceError> for ChromiumContextCreationInstallError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Resources(error)
    }
}

/// A pre-admitted optional publication whose final commit cannot fail.
pub trait PreparedChromiumContextPublication: Send {
    fn commit(self: Box<Self>);
}

/// Optional state installed alongside every newly created Context.
///
/// `prepare` runs before CDP. Implementations must reserve all fallible
/// capacity here and return a rollback-on-drop token. The commit runs only
/// after native creation and core topology/runtime publication succeed.
pub trait ChromiumContextPublicationPlugin: Send + Sync {
    fn prepare(
        &self,
        context_id: ObjectId,
        browser_id: ObjectId,
    ) -> Result<
        Box<dyn PreparedChromiumContextPublication>,
        OperationError<ContextCreationErrorDetails>,
    >;
}

#[derive(Default)]
pub struct NoopChromiumContextPublication;

struct PreparedNoopChromiumContextPublication;

impl PreparedChromiumContextPublication for PreparedNoopChromiumContextPublication {
    fn commit(self: Box<Self>) {}
}

impl ChromiumContextPublicationPlugin for NoopChromiumContextPublication {
    fn prepare(
        &self,
        _context_id: ObjectId,
        _browser_id: ObjectId,
    ) -> Result<
        Box<dyn PreparedChromiumContextPublication>,
        OperationError<ContextCreationErrorDetails>,
    > {
        Ok(Box::new(PreparedNoopChromiumContextPublication))
    }
}

/// Coordinates portable identity, plugin admission, native creation,
/// topology/runtime publication, and exact native compensation.
#[derive(Clone)]
pub struct ChromiumBrowserNewContextService {
    backend: CdpBrowserContextCreationService,
    browser_client: Arc<dyn CdpClient>,
    topology: SharedChromiumContextTopology,
    runtime: ControlledLifecycleHandle,
    object_ids: Arc<dyn ObjectIdSource>,
    browser_object_id: ObjectId,
    publication: Arc<dyn ChromiumContextPublicationPlugin>,
}

impl ChromiumBrowserNewContextService {
    pub fn new(
        backend: CdpBrowserContextCreationService,
        browser_client: Arc<dyn CdpClient>,
        topology: SharedChromiumContextTopology,
        runtime: ControlledLifecycleHandle,
        object_ids: Arc<dyn ObjectIdSource>,
        browser_object_id: ObjectId,
        publication: Arc<dyn ChromiumContextPublicationPlugin>,
    ) -> Self {
        Self {
            backend,
            browser_client,
            topology,
            runtime,
            object_ids,
            browser_object_id,
            publication,
        }
    }

    async fn create(
        &self,
        target: ObjectId,
        metadata: CallMetadata,
    ) -> Result<BrowserContextInitializer, OperationError<ContextCreationErrorDetails>> {
        if metadata != CallMetadata::default() {
            return Err(OperationError::new(
                FailureKind::ProtocolViolation,
                "Browser.newContext does not accept timeout or cancellation metadata",
                None,
            ));
        }
        if target != self.browser_object_id {
            return Err(OperationError::new(
                FailureKind::ProtocolViolation,
                format!(
                    "Browser.newContext targeted Browser `{target}`, expected `{}`",
                    self.browser_object_id
                ),
                None,
            ));
        }
        match self.runtime.is_object_live(target) {
            Ok(true) => {}
            Ok(false) => {
                return Err(OperationError::new(
                    FailureKind::TargetClosed,
                    format!("Browser `{target}` is closed"),
                    None,
                ));
            }
            Err(error) => return Err(map_runtime_error(error)),
        }

        let object_id = self.object_ids.next_object_id().map_err(|error| {
            OperationError::new(
                FailureKind::ResourceExhausted,
                error.to_string(),
                Some(ContextCreationErrorDetails::Publication(error.to_string())),
            )
        })?;
        let prepared_publication = self
            .publication
            .prepare(object_id, self.browser_object_id)?;
        let prepared_runtime = self
            .runtime
            .prepare_object_creation(
                object_id,
                Some(self.browser_object_id),
                BROWSER_CONTEXT_OBJECT_KIND.tag,
            )
            .map_err(map_runtime_error)?;
        let created = self.backend.call(()).await?;
        let native_id = created.browser_context_id();
        let publication = self.publish_created_context(
            prepared_runtime,
            native_id,
            object_id,
            prepared_publication,
        );
        if let Err(primary) = publication {
            return compensate_failed_publication(self.browser_client.as_ref(), native_id, primary)
                .await;
        }
        Ok(BrowserContextInitializer::new(
            object_id,
            self.browser_object_id,
            BROWSER_CONTEXT_OBJECT_KIND.tag,
        ))
    }

    fn publish_created_context(
        &self,
        prepared_runtime: pwright_runtime::PreparedObjectCreation,
        browser_context_id: &str,
        object_id: ObjectId,
        prepared_publication: Box<dyn PreparedChromiumContextPublication>,
    ) -> Result<(), OperationError<ContextCreationErrorDetails>> {
        let mut topology = self.topology.lock().map_err(|_| {
            OperationError::new(
                FailureKind::Cleanup,
                "Chromium topology state is unavailable after context creation",
                Some(ContextCreationErrorDetails::Publication(
                    "topology mutex is poisoned".to_string(),
                )),
            )
        })?;
        let epoch = topology.topology().epoch();
        let native_id = browser_context_id.to_owned();
        let prepared_topology = topology
            .prepare_transition(move |candidate| {
                candidate.register_context(epoch, native_id, object_id, CdpContextOwnership::Owned)
            })
            .map_err(map_topology_error)?;
        let expected = CdpTopologyEffect::Lifecycle(ControlMessage::Create {
            object_id,
            parent: Some(self.browser_object_id),
            kind: BROWSER_CONTEXT_OBJECT_KIND.tag,
        });
        if prepared_topology.output().effects() != [expected] {
            return Err(OperationError::new(
                FailureKind::ProtocolViolation,
                "Chromium topology produced unexpected effects for a new empty context",
                Some(ContextCreationErrorDetails::Publication(format!(
                    "unexpected topology effects: {:?}",
                    prepared_topology.output().effects()
                ))),
            ));
        }
        prepared_runtime.commit().map_err(map_runtime_error)?;
        prepared_topology.commit();
        drop(topology);
        prepared_publication.commit();
        Ok(())
    }
}

impl Service<Command<BrowserNewContext>> for ChromiumBrowserNewContextService {
    type Response = CommandResult<BrowserNewContext>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<BrowserNewContext>) -> Self::Future<'_> {
        let (call_id, object_id, metadata, ()) = command.into_parts();
        Box::pin(async move {
            Ok(CommandResult::new(
                call_id,
                self.create(object_id, metadata).await,
            ))
        })
    }
}

type ContextCreationFutureOutput = Result<CommandResult<BrowserNewContext>, Infallible>;
type BoxContextCreationFuture =
    Pin<Box<dyn Future<Output = ContextCreationFutureOutput> + Send + 'static>>;

pub struct ContinuableContextCreationFuture<Reservation>
where
    Reservation: CleanupReservation,
{
    inner: Option<BoxContextCreationFuture>,
    reservation: Option<Reservation>,
    started: bool,
}

impl<Reservation> Unpin for ContinuableContextCreationFuture<Reservation> where
    Reservation: CleanupReservation
{
}

impl<Reservation> Future for ContinuableContextCreationFuture<Reservation>
where
    Reservation: CleanupReservation,
{
    type Output = ContextCreationFutureOutput;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.started = true;
        let outcome = self
            .inner
            .as_mut()
            .expect("completed context creation future must not be polled again")
            .as_mut()
            .poll(context);
        if outcome.is_ready() {
            self.inner = None;
            self.reservation = None;
        }
        outcome
    }
}

impl<Reservation> Drop for ContinuableContextCreationFuture<Reservation>
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
                    "abandoned Browser.newContext continuation failed",
                )),
                Err(never) => match never {},
            }
        }));
    }
}

#[derive(Clone)]
pub struct ContinuableBrowserNewContextService<Continuation> {
    service: Arc<ChromiumBrowserNewContextService>,
    continuation: Continuation,
}

impl<Continuation> ContinuableBrowserNewContextService<Continuation> {
    pub const fn new(
        service: Arc<ChromiumBrowserNewContextService>,
        continuation: Continuation,
    ) -> Self {
        Self {
            service,
            continuation,
        }
    }
}

impl<Continuation> Service<Command<BrowserNewContext>>
    for ContinuableBrowserNewContextService<Continuation>
where
    Continuation: ReservedCleanupExecutor,
{
    type Response = CommandResult<BrowserNewContext>;
    type Error = Infallible;
    type Future<'a>
        = ContinuableContextCreationFuture<Continuation::Reservation>
    where
        Self: 'a;

    fn call(&self, command: Command<BrowserNewContext>) -> Self::Future<'_> {
        let call_id = command.call_id();
        let reservation = match self.continuation.try_reserve() {
            Ok(reservation) => reservation,
            Err(error) => {
                return ContinuableContextCreationFuture {
                    inner: Some(Box::pin(std::future::ready(Ok(continuation_failure(
                        call_id, error,
                    ))))),
                    reservation: None,
                    started: false,
                };
            }
        };
        let service = Arc::clone(&self.service);
        ContinuableContextCreationFuture {
            inner: Some(Box::pin(async move { service.call(command).await })),
            reservation: Some(reservation),
            started: false,
        }
    }
}

pub type ChromiumContextCreationHandlers<Continuation> =
    ContextCreationServerHandlers<ContinuableBrowserNewContextService<Continuation>>;

pub struct ChromiumContextHandlers<CreateContinuation, CloseContinuation>
where
    CreateContinuation: ReservedCleanupExecutor,
    CloseContinuation: ReservedCleanupExecutor,
{
    pub creation: ChromiumContextCreationHandlers<CreateContinuation>,
    pub lifecycle: ChromiumContextLifecycleHandlers<CloseContinuation>,
}

/// Caller-owned browser identity and ownership stores shared by creation and
/// lifecycle handlers.
pub struct ChromiumContextCoreBindings {
    pub browser_client: Arc<dyn CdpClient>,
    pub topology: SharedChromiumContextTopology,
    pub runtime: ControlledLifecycleHandle,
    pub object_ids: Arc<dyn ObjectIdSource>,
    pub browser_object_id: ObjectId,
}

/// Optional Context-local state owners for creation and retirement.
pub struct ChromiumContextPlugins {
    pub publication: Arc<dyn ChromiumContextPublicationPlugin>,
    pub retirement: Arc<dyn ChromiumContextRetirementPlugin>,
}

impl ChromiumContextPlugins {
    pub fn none() -> Self {
        Self {
            publication: Arc::new(NoopChromiumContextPublication),
            retirement: Arc::new(NoopChromiumContextRetirement),
        }
    }
}

/// Separately bounded continuation owners for irreversible create and close
/// operations.
pub struct ChromiumContextContinuations<CreateContinuation, CloseContinuation> {
    pub creation: CreateContinuation,
    pub close: CloseContinuation,
}

/// Complete paired Context creation/lifecycle server leaf.
pub struct ChromiumContextCreationInstallation<CreateContinuation, CloseContinuation>
where
    CreateContinuation: ReservedCleanupExecutor,
    CloseContinuation: ReservedCleanupExecutor,
{
    creation: Arc<ContinuableBrowserNewContextService<CreateContinuation>>,
    creation_protocol: Arc<WireProtocol<ContextCreationCodec>>,
    lifecycle: ChromiumContextLifecycleInstallation<CloseContinuation>,
}

impl<CreateContinuation, CloseContinuation>
    ChromiumContextCreationInstallation<CreateContinuation, CloseContinuation>
where
    CreateContinuation: ReservedCleanupExecutor,
    CloseContinuation: ReservedCleanupExecutor,
{
    pub fn install(
        bindings: ChromiumContextCoreBindings,
        plugins: ChromiumContextPlugins,
        continuations: ChromiumContextContinuations<CreateContinuation, CloseContinuation>,
        profile: ChromiumContextCreationProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumContextCreationInstallError> {
        let backend = CdpBrowserContextCreationService::new(
            Arc::clone(&bindings.browser_client),
            profile.backend(),
        )?;
        let lifecycle = ChromiumContextLifecycleInstallation::install(
            Arc::clone(&bindings.browser_client),
            Arc::clone(&bindings.topology),
            bindings.runtime.clone(),
            plugins.retirement,
            continuations.close,
            profile.lifecycle(),
            resources,
        )?;
        let creation_protocol = profile
            .unbudgeted_protocol()
            .with_production_resources(resources)?;
        let creation = ChromiumBrowserNewContextService::new(
            backend,
            bindings.browser_client,
            bindings.topology,
            bindings.runtime,
            bindings.object_ids,
            bindings.browser_object_id,
            plugins.publication,
        );
        Ok(Self {
            creation: Arc::new(ContinuableBrowserNewContextService::new(
                Arc::new(creation),
                continuations.creation,
            )),
            creation_protocol: Arc::new(creation_protocol),
            lifecycle,
        })
    }

    pub fn install_without_plugins(
        bindings: ChromiumContextCoreBindings,
        continuations: ChromiumContextContinuations<CreateContinuation, CloseContinuation>,
        profile: ChromiumContextCreationProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumContextCreationInstallError> {
        Self::install(
            bindings,
            ChromiumContextPlugins::none(),
            continuations,
            profile,
            resources,
        )
    }

    pub fn creation_service(&self) -> Arc<ContinuableBrowserNewContextService<CreateContinuation>> {
        Arc::clone(&self.creation)
    }

    pub const fn lifecycle(&self) -> &ChromiumContextLifecycleInstallation<CloseContinuation> {
        &self.lifecycle
    }

    pub fn creation_handlers(&self) -> ChromiumContextCreationHandlers<CreateContinuation> {
        context_creation_server_handlers(
            Arc::clone(&self.creation_protocol),
            ContextCreationServerServices {
                browser_new_context: Arc::clone(&self.creation),
            },
        )
    }

    pub fn handlers(&self) -> ChromiumContextHandlers<CreateContinuation, CloseContinuation> {
        ChromiumContextHandlers {
            creation: self.creation_handlers(),
            lifecycle: self.lifecycle.handlers(),
        }
    }
}

async fn compensate_failed_publication(
    client: &dyn CdpClient,
    browser_context_id: &str,
    primary: OperationError<ContextCreationErrorDetails>,
) -> Result<BrowserContextInitializer, OperationError<ContextCreationErrorDetails>> {
    match pwright_composition_chromium_context_lifecycle::CdpBrowserContextCloseService::close_route(
        client,
        browser_context_id,
        CdpContextOwnership::Owned,
    )
    .await
    {
        Ok(()) => Err(primary),
        Err(cleanup) => Err(OperationError::new(
            FailureKind::Cleanup,
            "Browser.newContext publication failed and native compensation also failed",
            Some(ContextCreationErrorDetails::Cleanup {
                primary: primary.to_string(),
                cleanup: cleanup.to_string(),
            }),
        )),
    }
}

fn continuation_failure(
    call_id: CallId,
    error: CleanupSubmitError,
) -> CommandResult<BrowserNewContext> {
    let kind = match error {
        CleanupSubmitError::CapacityExceeded { .. } => FailureKind::ResourceExhausted,
        CleanupSubmitError::Unavailable | CleanupSubmitError::StatePoisoned => FailureKind::Cleanup,
    };
    CommandResult::new(
        call_id,
        Err(OperationError::new(
            kind,
            "Browser.newContext continuation capacity is unavailable",
            Some(ContextCreationErrorDetails::Publication(
                "creation was rejected before browser interaction".to_string(),
            )),
        )),
    )
}

fn map_runtime_error(error: RuntimeStateError) -> OperationError<ContextCreationErrorDetails> {
    let kind = match error {
        RuntimeStateError::TransportClosed => FailureKind::TransportClosed,
        RuntimeStateError::ObjectGraph(_)
        | RuntimeStateError::UnknownObjectKind { .. }
        | RuntimeStateError::MissingRequiredParent { .. }
        | RuntimeStateError::InvalidParentKind { .. }
        | RuntimeStateError::UnsupportedLifecycleMessage { .. } => FailureKind::ProtocolViolation,
        RuntimeStateError::ControlMessageCapacityExceeded { .. }
        | RuntimeStateError::LifecycleTransactionIdExhausted
        | RuntimeStateError::ProductionResource(_)
        | RuntimeStateError::Resource(_) => FailureKind::ResourceExhausted,
        _ => FailureKind::Cleanup,
    };
    OperationError::new(
        kind,
        format!("runtime could not prepare BrowserContext creation: {error}"),
        Some(ContextCreationErrorDetails::Publication(error.to_string())),
    )
}

fn map_topology_error(
    error: CdpTopologyProductionError,
) -> OperationError<ContextCreationErrorDetails> {
    let kind = match &error {
        CdpTopologyProductionError::Resource(_)
        | CdpTopologyProductionError::CapacityOverflow
        | CdpTopologyProductionError::RetainedBytesInvariant { .. }
        | CdpTopologyProductionError::Topology(CdpTopologyError::CapacityExceeded { .. })
        | CdpTopologyProductionError::Topology(CdpTopologyError::ObjectGraph(
            ObjectGraphError::CapacityExceeded { .. },
        )) => FailureKind::ResourceExhausted,
        CdpTopologyProductionError::Topology(_) => FailureKind::ProtocolViolation,
        _ => FailureKind::Cleanup,
    };
    OperationError::new(
        kind,
        format!("Chromium topology could not publish BrowserContext creation: {error}"),
        Some(ContextCreationErrorDetails::Publication(error.to_string())),
    )
}
