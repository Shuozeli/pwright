//! Minimal Chromium CDP composition for Page creation and lifecycle.
//!
//! The creation capability carries its required Page-lifecycle dependency.
//! Both generated handler sets share caller-owned topology and runtime state,
//! while creation and close use independent bounded continuation capacity.

use std::convert::Infallible;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

pub use pwright_backend_cdp_page_creation::{
    CdpCreatedPage, CdpPageCreationLimits, CdpPageCreationRequest, CdpPageCreationService,
    CdpPageCreationSetupError,
};
use pwright_backend_cdp_topology::{
    CdpFrameAttachment, CdpPageObjectIds, CdpPageTarget, CdpTopologyEffect,
    CdpTopologyProductionError,
};
pub use pwright_browser_model::{FRAME_OBJECT_KIND, PAGE_OBJECT_KIND};
use pwright_cap_page_creation::{BrowserContextNewPage, PageCreationErrorDetails, PageInitializer};
use pwright_cdp::{CdpClient, SessionFactory};
use pwright_codec_page_creation::PageCreationCodec;
pub use pwright_codec_page_creation::PageCreationCodecLimits;
pub use pwright_composition_chromium_page_lifecycle::{
    CdpPageCloseLimits, CdpPageClosePoll, ChromiumPageLifecycleBindings,
    ChromiumPageLifecycleHandlers, ChromiumPageLifecycleInstallError,
    ChromiumPageLifecycleInstallation, ChromiumPageLifecycleProfile, ChromiumPageRetirementPlugin,
    ChromiumPageSessionResolver, ImmediatePageClosePoll, NoopChromiumPageRetirement,
    PageLifecycleCodecLimits, PreparedChromiumPageRetirement, SessionFactoryChromiumPageResolver,
    SharedChromiumPageTopology,
};
pub use pwright_core::{
    AtomicObjectIdSource, ProductionResourceError, ProductionResourceLedger, ProtocolVersion,
    ResourceLimits, ResourceRequirements, ResourceScope,
};
use pwright_core::{
    CallId, CallMetadata, Command, CommandResult, ControlMessage, FailureKind, ObjectId,
    ObjectIdSource, OperationError, Service,
};
use pwright_runtime::{
    CleanupFailure, CleanupReservation, CleanupSubmitError, ControlledLifecycleHandle,
    ObjectCreationSpec, PreparedObjectCreationBatch, ReservedCleanupExecutor, RuntimeStateError,
};
use pwright_server_page_creation::{
    PageCreationServerHandlers, PageCreationServerServices, page_creation_server_handlers,
};
pub use pwright_transport::FrameLimits;
use pwright_transport::WireProtocol;

/// Backend, creation-wire, and required Page-lifecycle bounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumPageCreationProfile {
    backend: CdpPageCreationLimits,
    codec: PageCreationCodecLimits,
    send_limits: FrameLimits,
    receive_limits: FrameLimits,
    lifecycle: ChromiumPageLifecycleProfile,
}

impl ChromiumPageCreationProfile {
    pub const VERSION: ProtocolVersion = ProtocolVersion::new(1, 0);

    pub const fn new(
        backend: CdpPageCreationLimits,
        codec: PageCreationCodecLimits,
        send_limits: FrameLimits,
        receive_limits: FrameLimits,
        lifecycle: ChromiumPageLifecycleProfile,
    ) -> Self {
        Self {
            backend,
            codec,
            send_limits,
            receive_limits,
            lifecycle,
        }
    }

    pub const fn backend(self) -> CdpPageCreationLimits {
        self.backend
    }

    pub const fn codec(self) -> PageCreationCodecLimits {
        self.codec
    }

    pub const fn send_limits(self) -> FrameLimits {
        self.send_limits
    }

    pub const fn receive_limits(self) -> FrameLimits {
        self.receive_limits
    }

    pub const fn lifecycle(self) -> ChromiumPageLifecycleProfile {
        self.lifecycle
    }

    pub fn requirements(self) -> ResourceRequirements {
        self.unbudgeted_protocol()
            .production_resource_requirements()
            .combine_concurrent(self.lifecycle.requirements())
    }

    fn unbudgeted_protocol(self) -> WireProtocol<PageCreationCodec> {
        WireProtocol::new_exact_directional(
            PageCreationCodec::new(self.codec),
            Self::VERSION,
            self.send_limits,
            self.receive_limits,
        )
    }
}

#[derive(Debug)]
pub enum ChromiumPageCreationInstallError {
    Backend(CdpPageCreationSetupError),
    Lifecycle(ChromiumPageLifecycleInstallError),
    Resources(ProductionResourceError),
}

impl Display for ChromiumPageCreationInstallError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(error) => Display::fmt(error, formatter),
            Self::Lifecycle(error) => Display::fmt(error, formatter),
            Self::Resources(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ChromiumPageCreationInstallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Backend(error) => Some(error),
            Self::Lifecycle(error) => Some(error),
            Self::Resources(error) => Some(error),
        }
    }
}

impl From<CdpPageCreationSetupError> for ChromiumPageCreationInstallError {
    fn from(error: CdpPageCreationSetupError) -> Self {
        Self::Backend(error)
    }
}

impl From<ChromiumPageLifecycleInstallError> for ChromiumPageCreationInstallError {
    fn from(error: ChromiumPageLifecycleInstallError) -> Self {
        Self::Lifecycle(error)
    }
}

impl From<ProductionResourceError> for ChromiumPageCreationInstallError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Resources(error)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumPagePublicationPlan {
    page_id: ObjectId,
    main_frame_id: ObjectId,
    context_id: ObjectId,
}

impl ChromiumPagePublicationPlan {
    pub const fn page_id(self) -> ObjectId {
        self.page_id
    }

    pub const fn main_frame_id(self) -> ObjectId {
        self.main_frame_id
    }

    pub const fn context_id(self) -> ObjectId {
        self.context_id
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumCreatedPageRoute<'a> {
    target_id: &'a str,
    session_id: &'a str,
    main_frame_id: &'a str,
}

impl<'a> ChromiumCreatedPageRoute<'a> {
    pub const fn target_id(self) -> &'a str {
        self.target_id
    }

    pub const fn session_id(self) -> &'a str {
        self.session_id
    }

    pub const fn main_frame_id(self) -> &'a str {
        self.main_frame_id
    }
}

/// Pre-admitted optional Page-local state with rollback-on-drop semantics.
pub trait PreparedChromiumPagePublication: Send {
    fn bind(
        &mut self,
        route: ChromiumCreatedPageRoute<'_>,
    ) -> Result<(), OperationError<PageCreationErrorDetails>>;

    fn commit(self: Box<Self>);
}

/// Optional state installed alongside each explicitly created Page.
pub trait ChromiumPagePublicationPlugin: Send + Sync {
    fn prepare(
        &self,
        plan: ChromiumPagePublicationPlan,
    ) -> Result<Box<dyn PreparedChromiumPagePublication>, OperationError<PageCreationErrorDetails>>;
}

#[derive(Default)]
pub struct NoopChromiumPagePublication;

struct PreparedNoopChromiumPagePublication;

impl PreparedChromiumPagePublication for PreparedNoopChromiumPagePublication {
    fn bind(
        &mut self,
        _route: ChromiumCreatedPageRoute<'_>,
    ) -> Result<(), OperationError<PageCreationErrorDetails>> {
        Ok(())
    }

    fn commit(self: Box<Self>) {}
}

impl ChromiumPagePublicationPlugin for NoopChromiumPagePublication {
    fn prepare(
        &self,
        _plan: ChromiumPagePublicationPlan,
    ) -> Result<Box<dyn PreparedChromiumPagePublication>, OperationError<PageCreationErrorDetails>>
    {
        Ok(Box::new(PreparedNoopChromiumPagePublication))
    }
}

#[derive(Clone)]
pub struct ChromiumBrowserContextNewPageService {
    backend: CdpPageCreationService,
    topology: SharedChromiumPageTopology,
    runtime: ControlledLifecycleHandle,
    object_ids: Arc<dyn ObjectIdSource>,
    publication: Arc<dyn ChromiumPagePublicationPlugin>,
}

impl ChromiumBrowserContextNewPageService {
    pub fn new(
        backend: CdpPageCreationService,
        topology: SharedChromiumPageTopology,
        runtime: ControlledLifecycleHandle,
        object_ids: Arc<dyn ObjectIdSource>,
        publication: Arc<dyn ChromiumPagePublicationPlugin>,
    ) -> Self {
        Self {
            backend,
            topology,
            runtime,
            object_ids,
            publication,
        }
    }

    async fn create(
        &self,
        context_id: ObjectId,
        metadata: CallMetadata,
    ) -> Result<PageInitializer, OperationError<PageCreationErrorDetails>> {
        validate_metadata(metadata)?;
        validate_live_context(&self.runtime, context_id)?;
        let browser_context_id = resolve_context_route(&self.topology, context_id)?;
        let page_id = next_object_id(&self.object_ids)?;
        let main_frame_id = next_object_id(&self.object_ids)?;
        let mut prepared_publication = self.publication.prepare(ChromiumPagePublicationPlan {
            page_id,
            main_frame_id,
            context_id,
        })?;
        let prepared_runtime = self
            .runtime
            .prepare_object_creation_batch(&[
                ObjectCreationSpec::new(page_id, Some(context_id), PAGE_OBJECT_KIND.tag),
                ObjectCreationSpec::new(main_frame_id, Some(page_id), FRAME_OBJECT_KIND.tag),
            ])
            .map_err(map_runtime_error)?;
        let created = self
            .backend
            .call(CdpPageCreationRequest::new(browser_context_id.clone()))
            .await?;
        let route = ChromiumCreatedPageRoute {
            target_id: created.target_id(),
            session_id: created.session_id(),
            main_frame_id: created.main_frame_id(),
        };
        if let Err(primary) = prepared_publication.bind(route) {
            return Err(self
                .backend
                .compensate_created_target(created.target_id(), primary)
                .await);
        }
        let publication = CreatedPagePublication {
            browser_context_id: &browser_context_id,
            created: &created,
            context_id,
            page_id,
            main_frame_id,
            prepared_runtime,
            prepared_plugin: prepared_publication,
        };
        if let Err(primary) = publish_created_page(&self.topology, publication) {
            return Err(self
                .backend
                .compensate_created_target(created.target_id(), primary)
                .await);
        }
        Ok(PageInitializer::new(
            page_id,
            context_id,
            PAGE_OBJECT_KIND.tag,
            main_frame_id,
            page_id,
            FRAME_OBJECT_KIND.tag,
        ))
    }
}

impl Service<Command<BrowserContextNewPage>> for ChromiumBrowserContextNewPageService {
    type Response = CommandResult<BrowserContextNewPage>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<BrowserContextNewPage>) -> Self::Future<'_> {
        let (call_id, object_id, metadata, ()) = command.into_parts();
        Box::pin(async move {
            Ok(CommandResult::new(
                call_id,
                self.create(object_id, metadata).await,
            ))
        })
    }
}

struct CreatedPagePublication<'a> {
    browser_context_id: &'a str,
    created: &'a CdpCreatedPage,
    context_id: ObjectId,
    page_id: ObjectId,
    main_frame_id: ObjectId,
    prepared_runtime: PreparedObjectCreationBatch,
    prepared_plugin: Box<dyn PreparedChromiumPagePublication>,
}

fn publish_created_page(
    topology: &SharedChromiumPageTopology,
    publication: CreatedPagePublication<'_>,
) -> Result<(), OperationError<PageCreationErrorDetails>> {
    let mut topology = topology.lock().map_err(|_| topology_poisoned())?;
    let epoch = topology.topology().epoch();
    let target = CdpPageTarget::new(
        publication.created.target_id(),
        publication.created.session_id(),
        publication.browser_context_id,
    );
    let main_frame = CdpFrameAttachment::main(
        publication.created.target_id(),
        publication.created.main_frame_id(),
    );
    let prepared_topology = topology
        .prepare_transition(move |candidate| {
            let attached = candidate.attach_page(
                epoch,
                target,
                CdpPageObjectIds::new(publication.page_id, publication.main_frame_id),
            )?;
            let rebound = candidate.attach_frame(epoch, main_frame, publication.main_frame_id)?;
            Ok((attached, rebound))
        })
        .map_err(map_topology_error)?;
    let expected = [
        CdpTopologyEffect::Lifecycle(ControlMessage::Create {
            object_id: publication.page_id,
            parent: Some(publication.context_id),
            kind: PAGE_OBJECT_KIND.tag,
        }),
        CdpTopologyEffect::Lifecycle(ControlMessage::Create {
            object_id: publication.main_frame_id,
            parent: Some(publication.page_id),
            kind: FRAME_OBJECT_KIND.tag,
        }),
    ];
    if prepared_topology.output().0.effects() != expected
        || prepared_topology
            .output()
            .1
            .effects()
            .iter()
            .any(|effect| matches!(effect, CdpTopologyEffect::Lifecycle(_)))
    {
        return Err(OperationError::new(
            FailureKind::ProtocolViolation,
            "Chromium topology produced unexpected effects for explicit Page creation",
            Some(PageCreationErrorDetails::Publication(format!(
                "attach effects: {:?}; frame effects: {:?}",
                prepared_topology.output().0.effects(),
                prepared_topology.output().1.effects()
            ))),
        ));
    }
    publication
        .prepared_runtime
        .commit()
        .map_err(map_runtime_error)?;
    prepared_topology.commit();
    drop(topology);
    publication.prepared_plugin.commit();
    Ok(())
}

type PageCreationFutureOutput = Result<CommandResult<BrowserContextNewPage>, Infallible>;
type BoxPageCreationFuture =
    Pin<Box<dyn Future<Output = PageCreationFutureOutput> + Send + 'static>>;

pub struct ContinuablePageCreationFuture<Reservation>
where
    Reservation: CleanupReservation,
{
    inner: Option<BoxPageCreationFuture>,
    reservation: Option<Reservation>,
    started: bool,
}

impl<Reservation> Unpin for ContinuablePageCreationFuture<Reservation> where
    Reservation: CleanupReservation
{
}

impl<Reservation> Future for ContinuablePageCreationFuture<Reservation>
where
    Reservation: CleanupReservation,
{
    type Output = PageCreationFutureOutput;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.started = true;
        let outcome = self
            .inner
            .as_mut()
            .expect("completed Page creation future must not be polled again")
            .as_mut()
            .poll(context);
        if outcome.is_ready() {
            self.inner = None;
            self.reservation = None;
        }
        outcome
    }
}

impl<Reservation> Drop for ContinuablePageCreationFuture<Reservation>
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
                    "abandoned BrowserContext.newPage continuation failed",
                )),
                Err(never) => match never {},
            }
        }));
    }
}

#[derive(Clone)]
pub struct ContinuableBrowserContextNewPageService<Continuation> {
    service: Arc<ChromiumBrowserContextNewPageService>,
    continuation: Continuation,
}

impl<Continuation> ContinuableBrowserContextNewPageService<Continuation> {
    pub const fn new(
        service: Arc<ChromiumBrowserContextNewPageService>,
        continuation: Continuation,
    ) -> Self {
        Self {
            service,
            continuation,
        }
    }
}

impl<Continuation> Service<Command<BrowserContextNewPage>>
    for ContinuableBrowserContextNewPageService<Continuation>
where
    Continuation: ReservedCleanupExecutor,
{
    type Response = CommandResult<BrowserContextNewPage>;
    type Error = Infallible;
    type Future<'a>
        = ContinuablePageCreationFuture<Continuation::Reservation>
    where
        Self: 'a;

    fn call(&self, command: Command<BrowserContextNewPage>) -> Self::Future<'_> {
        let call_id = command.call_id();
        let reservation = match self.continuation.try_reserve() {
            Ok(reservation) => reservation,
            Err(error) => {
                return ContinuablePageCreationFuture {
                    inner: Some(Box::pin(std::future::ready(Ok(continuation_failure(
                        call_id, error,
                    ))))),
                    reservation: None,
                    started: false,
                };
            }
        };
        let service = Arc::clone(&self.service);
        ContinuablePageCreationFuture {
            inner: Some(Box::pin(async move { service.call(command).await })),
            reservation: Some(reservation),
            started: false,
        }
    }
}

pub type ChromiumPageCreationHandlers<Continuation> =
    PageCreationServerHandlers<ContinuableBrowserContextNewPageService<Continuation>>;

pub struct ChromiumPageHandlers<CreateContinuation, CloseContinuation>
where
    CreateContinuation: ReservedCleanupExecutor,
    CloseContinuation: ReservedCleanupExecutor,
{
    pub creation: ChromiumPageCreationHandlers<CreateContinuation>,
    pub lifecycle: ChromiumPageLifecycleHandlers<CloseContinuation>,
}

pub struct ChromiumPageCoreBindings {
    pub browser_client: Arc<dyn CdpClient>,
    pub session_factory: Arc<dyn SessionFactory>,
    pub session_resolver: Arc<dyn ChromiumPageSessionResolver>,
    pub topology: SharedChromiumPageTopology,
    pub runtime: ControlledLifecycleHandle,
    pub object_ids: Arc<dyn ObjectIdSource>,
}

pub struct ChromiumPagePlugins {
    pub publication: Arc<dyn ChromiumPagePublicationPlugin>,
    pub retirement: Arc<dyn ChromiumPageRetirementPlugin>,
}

impl ChromiumPagePlugins {
    pub fn none() -> Self {
        Self {
            publication: Arc::new(NoopChromiumPagePublication),
            retirement: Arc::new(NoopChromiumPageRetirement),
        }
    }
}

pub struct ChromiumPageContinuations<CreateContinuation, CloseContinuation> {
    pub creation: CreateContinuation,
    pub close: CloseContinuation,
}

pub struct ChromiumPageCreationInstallation<CreateContinuation, CloseContinuation>
where
    CreateContinuation: ReservedCleanupExecutor,
    CloseContinuation: ReservedCleanupExecutor,
{
    creation: Arc<ContinuableBrowserContextNewPageService<CreateContinuation>>,
    creation_protocol: Arc<WireProtocol<PageCreationCodec>>,
    lifecycle: ChromiumPageLifecycleInstallation<CloseContinuation>,
}

impl<CreateContinuation, CloseContinuation>
    ChromiumPageCreationInstallation<CreateContinuation, CloseContinuation>
where
    CreateContinuation: ReservedCleanupExecutor,
    CloseContinuation: ReservedCleanupExecutor,
{
    pub fn install(
        bindings: ChromiumPageCoreBindings,
        plugins: ChromiumPagePlugins,
        continuations: ChromiumPageContinuations<CreateContinuation, CloseContinuation>,
        close_poll: Arc<dyn CdpPageClosePoll>,
        profile: ChromiumPageCreationProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumPageCreationInstallError> {
        let backend = CdpPageCreationService::with_cleanup_poll(
            Arc::clone(&bindings.browser_client),
            Arc::clone(&bindings.session_factory),
            profile.backend(),
            Arc::clone(&close_poll),
        )?;
        let lifecycle = ChromiumPageLifecycleInstallation::install(
            ChromiumPageLifecycleBindings {
                browser_client: Arc::clone(&bindings.browser_client),
                session_resolver: bindings.session_resolver,
                topology: Arc::clone(&bindings.topology),
                runtime: bindings.runtime.clone(),
                retirement: plugins.retirement,
                close_poll,
                continuation: continuations.close,
            },
            profile.lifecycle(),
            resources,
        )?;
        let creation_protocol = profile
            .unbudgeted_protocol()
            .with_production_resources(resources)?;
        let creation = ChromiumBrowserContextNewPageService::new(
            backend,
            bindings.topology,
            bindings.runtime,
            bindings.object_ids,
            plugins.publication,
        );
        Ok(Self {
            creation: Arc::new(ContinuableBrowserContextNewPageService::new(
                Arc::new(creation),
                continuations.creation,
            )),
            creation_protocol: Arc::new(creation_protocol),
            lifecycle,
        })
    }

    pub fn install_without_plugins(
        bindings: ChromiumPageCoreBindings,
        continuations: ChromiumPageContinuations<CreateContinuation, CloseContinuation>,
        close_poll: Arc<dyn CdpPageClosePoll>,
        profile: ChromiumPageCreationProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumPageCreationInstallError> {
        Self::install(
            bindings,
            ChromiumPagePlugins::none(),
            continuations,
            close_poll,
            profile,
            resources,
        )
    }

    pub fn creation_service(
        &self,
    ) -> Arc<ContinuableBrowserContextNewPageService<CreateContinuation>> {
        Arc::clone(&self.creation)
    }

    pub const fn lifecycle(&self) -> &ChromiumPageLifecycleInstallation<CloseContinuation> {
        &self.lifecycle
    }

    pub fn creation_handlers(&self) -> ChromiumPageCreationHandlers<CreateContinuation> {
        page_creation_server_handlers(
            Arc::clone(&self.creation_protocol),
            PageCreationServerServices {
                browser_context_new_page: Arc::clone(&self.creation),
            },
        )
    }

    pub fn handlers(&self) -> ChromiumPageHandlers<CreateContinuation, CloseContinuation> {
        ChromiumPageHandlers {
            creation: self.creation_handlers(),
            lifecycle: self.lifecycle.handlers(),
        }
    }
}

fn resolve_context_route(
    topology: &SharedChromiumPageTopology,
    context_id: ObjectId,
) -> Result<String, OperationError<PageCreationErrorDetails>> {
    topology
        .lock()
        .map_err(|_| topology_poisoned())?
        .topology()
        .browser_context_route(context_id)
        .map(|route| route.browser_context_id().to_owned())
        .ok_or_else(|| {
            OperationError::new(
                FailureKind::TargetClosed,
                format!("BrowserContext `{context_id}` has no Chromium route"),
                Some(PageCreationErrorDetails::Publication(
                    "portable Context is absent from Chromium topology".to_string(),
                )),
            )
        })
}

fn validate_metadata(
    metadata: CallMetadata,
) -> Result<(), OperationError<PageCreationErrorDetails>> {
    if metadata == CallMetadata::default() {
        return Ok(());
    }
    Err(OperationError::new(
        FailureKind::ProtocolViolation,
        "BrowserContext.newPage does not accept timeout or cancellation metadata",
        None,
    ))
}

fn validate_live_context(
    runtime: &ControlledLifecycleHandle,
    context_id: ObjectId,
) -> Result<(), OperationError<PageCreationErrorDetails>> {
    match runtime.is_object_live(context_id) {
        Ok(true) => Ok(()),
        Ok(false) => Err(OperationError::new(
            FailureKind::TargetClosed,
            format!("BrowserContext `{context_id}` is closed"),
            None,
        )),
        Err(error) => Err(map_runtime_error(error)),
    }
}

fn next_object_id(
    source: &Arc<dyn ObjectIdSource>,
) -> Result<ObjectId, OperationError<PageCreationErrorDetails>> {
    source.next_object_id().map_err(|error| {
        OperationError::new(
            FailureKind::ResourceExhausted,
            error.to_string(),
            Some(PageCreationErrorDetails::Publication(error.to_string())),
        )
    })
}

fn continuation_failure(
    call_id: CallId,
    error: CleanupSubmitError,
) -> CommandResult<BrowserContextNewPage> {
    let kind = match error {
        CleanupSubmitError::CapacityExceeded { .. } => FailureKind::ResourceExhausted,
        CleanupSubmitError::Unavailable | CleanupSubmitError::StatePoisoned => FailureKind::Cleanup,
    };
    CommandResult::new(
        call_id,
        Err(OperationError::new(
            kind,
            "BrowserContext.newPage continuation capacity is unavailable",
            Some(PageCreationErrorDetails::Publication(
                "creation was rejected before browser interaction".to_string(),
            )),
        )),
    )
}

fn map_runtime_error(error: RuntimeStateError) -> OperationError<PageCreationErrorDetails> {
    let kind = match &error {
        RuntimeStateError::TransportClosed => FailureKind::TransportClosed,
        RuntimeStateError::LifecycleTransactionInProgress
        | RuntimeStateError::LifecycleTransactionIdExhausted
        | RuntimeStateError::ControlMessageCapacityExceeded { .. }
        | RuntimeStateError::ProductionResource(_)
        | RuntimeStateError::Resource(_) => FailureKind::ResourceExhausted,
        _ => FailureKind::ProtocolViolation,
    };
    OperationError::new(
        kind,
        format!("runtime could not prepare Page creation: {error}"),
        Some(PageCreationErrorDetails::Publication(error.to_string())),
    )
}

fn map_topology_error(
    error: CdpTopologyProductionError,
) -> OperationError<PageCreationErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        format!("Chromium topology could not publish Page creation: {error}"),
        Some(PageCreationErrorDetails::Publication(error.to_string())),
    )
}

fn topology_poisoned() -> OperationError<PageCreationErrorDetails> {
    OperationError::new(
        FailureKind::Cleanup,
        "Chromium topology state is unavailable during Page creation",
        Some(PageCreationErrorDetails::Publication(
            "topology mutex is poisoned".to_string(),
        )),
    )
}
