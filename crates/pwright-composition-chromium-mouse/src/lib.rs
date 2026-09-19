//! Optional Mouse Page route and caller-polled native retirement composition.
//! No runtime or deadline engine is started or implicitly installed.
pub use pwright_backend_cdp_input::{CdpInputCoordinator, CdpMouseDispatcher};
pub use pwright_backend_cdp_mouse::{
    CdpMouseDelay, CdpMouseProductionProfile, CdpMouseServiceFactory, MouseResult,
};
use pwright_cap_mouse::*;
use pwright_cdp::CdpClient;
use pwright_codec_mouse::MouseCodec;
pub use pwright_codec_mouse::MouseCodecLimits;
pub use pwright_core::{
    CallId, CancellationReason, ObjectId, ProductionResourceError, ProductionResourceLedger,
    ProtocolVersion, ResourceClass, ResourceRequirements, ResourceScope,
};
use pwright_core::{Command, CommandResult, CommandSpec, OperationContext, Service};
use pwright_server_mouse::{MouseServerHandlers, MouseServerServices, mouse_server_handlers};
pub use pwright_transport::FrameLimits;
use pwright_transport::WireProtocol;
use std::{
    convert::Infallible,
    future::Future,
    marker::PhantomData,
    num::NonZeroUsize,
    pin::Pin,
    sync::{Arc, atomic::Ordering},
    task::{Context, Poll},
};
mod queue;
pub use queue::{MouseRetirementDriver, MouseRetirementReceipt};
#[cfg(feature = "runtime")]
mod controlled;
#[cfg(feature = "runtime")]
pub use controlled::ChromiumMouseInvocation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumMouseProfile {
    backend: CdpMouseProductionProfile,
    capacity: NonZeroUsize,
    codec: MouseCodecLimits,
    version: ProtocolVersion,
    send: FrameLimits,
    receive: FrameLimits,
}
impl ChromiumMouseProfile {
    pub fn new(
        backend: CdpMouseProductionProfile,
        capacity: NonZeroUsize,
        codec: MouseCodecLimits,
        version: ProtocolVersion,
        send: FrameLimits,
        receive: FrameLimits,
    ) -> Result<Self, ChromiumMouseInstallError> {
        ChromiumMousePage::route_requirements(capacity)?;
        Ok(Self {
            backend,
            capacity,
            codec,
            version,
            send,
            receive,
        })
    }
    pub const fn backend(self) -> CdpMouseProductionProfile {
        self.backend
    }
    pub const fn capacity(self) -> NonZeroUsize {
        self.capacity
    }
    pub fn requirements(self) -> ResourceRequirements {
        self.backend
            .requirements()
            .combine_concurrent(
                ChromiumMousePage::route_requirements(self.capacity)
                    .expect("validated route capacity"),
            )
            .combine_concurrent(self.protocol().production_resource_requirements())
    }
    fn protocol(self) -> WireProtocol<MouseCodec> {
        WireProtocol::new_exact_directional(
            MouseCodec::new(self.codec),
            self.version,
            self.send,
            self.receive,
        )
    }
}
pub struct ChromiumMousePage {
    shared: Arc<queue::Shared>,
}
impl ChromiumMousePage {
    pub fn page_id(&self) -> ObjectId {
        self.shared.page_id
    }
    pub fn shares_input_state_with(&self, input: &CdpInputCoordinator) -> bool {
        self.shared
            .backend
            .mouse_down
            .shares_input_state_with(input)
    }
    pub fn is_revoked(&self) -> bool {
        self.shared.revoked.load(Ordering::Acquire)
    }
    /// Reject new and not-yet-polled work. Active calls are not implicitly
    /// cancelled; request cancellation explicitly and consume native receipts.
    pub fn revoke_admission(&self) {
        self.shared.revoked.store(true, Ordering::Release);
    }
    pub fn retained_records(&self) -> usize {
        self.shared.retained_records()
    }
    pub fn queued_operations(&self) -> usize {
        self.shared.queued_operations()
    }
    /// Validates complete call/Page/method/cancellation identity. This only
    /// requests retirement; it does not acknowledge completion of native work.
    pub fn request_cancel(
        &self,
        context: OperationContext,
        reason: CancellationReason,
    ) -> MouseResult {
        self.shared.request_cancel(context, reason)
    }
    pub fn request_cancel_all(&self, reason: CancellationReason) {
        self.shared.request_cancel_all(reason)
    }
    fn base_route_bytes(capacity: NonZeroUsize) -> Result<usize, ChromiumMouseInstallError> {
        queue::Shared::allocation_bytes(capacity.get())?
            .checked_add(
                std::mem::size_of::<Self>()
                    + 2 * std::mem::size_of::<usize>()
                    + std::mem::size_of::<MouseRetirementDriver>(),
            )
            .ok_or(ChromiumMouseInstallError::CapacityOverflow)
    }
    pub fn route_requirements(
        capacity: NonZeroUsize,
    ) -> Result<ResourceRequirements, ChromiumMouseInstallError> {
        let record_bytes = queue::Shared::record_bytes()
            .checked_mul(capacity.get())
            .ok_or(ChromiumMouseInstallError::CapacityOverflow)?;
        let bytes = Self::base_route_bytes(capacity)?
            .checked_add(record_bytes)
            .ok_or(ChromiumMouseInstallError::CapacityOverflow)?;
        let bytes =
            u64::try_from(bytes).map_err(|_| ChromiumMouseInstallError::CapacityOverflow)?;
        let leases = capacity
            .get()
            .checked_add(1)
            .ok_or(ChromiumMouseInstallError::CapacityOverflow)?;
        Ok(ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, leases))
    }
    /// The returned unique driver must outlive response futures and remain
    /// owned through shutdown until queued native work has been retired.
    pub fn bind(
        page_id: ObjectId,
        client: Arc<dyn CdpClient>,
        input: CdpInputCoordinator,
        factory: &CdpMouseServiceFactory,
        capacity: NonZeroUsize,
        resources: &ProductionResourceLedger,
    ) -> Result<(Arc<Self>, MouseRetirementDriver), ChromiumMouseInstallError> {
        Self::bind_selected(page_id, client, input, factory, capacity, resources, None)
    }
    /// Explicit Page-scoped selection; response cancellation keeps delegated
    /// cleanup in the existing retirement driver. Persistent provider state and
    /// allocations still require their own owner/admission outside this route.
    pub fn bind_with_dispatcher(
        page_id: ObjectId,
        client: Arc<dyn CdpClient>,
        input: CdpInputCoordinator,
        factory: &CdpMouseServiceFactory,
        capacity: NonZeroUsize,
        resources: &ProductionResourceLedger,
        dispatcher: Arc<dyn CdpMouseDispatcher>,
    ) -> Result<(Arc<Self>, MouseRetirementDriver), ChromiumMouseInstallError> {
        Self::bind_selected(
            page_id,
            client,
            input,
            factory,
            capacity,
            resources,
            Some(dispatcher),
        )
    }
    fn bind_selected(
        page_id: ObjectId,
        client: Arc<dyn CdpClient>,
        input: CdpInputCoordinator,
        factory: &CdpMouseServiceFactory,
        capacity: NonZeroUsize,
        resources: &ProductionResourceLedger,
        dispatcher: Option<Arc<dyn CdpMouseDispatcher>>,
    ) -> Result<(Arc<Self>, MouseRetirementDriver), ChromiumMouseInstallError> {
        let requirements = Self::route_requirements(capacity)?;
        resources.claim(
            factory
                .profile()
                .requirements()
                .combine_concurrent(requirements),
        )?;
        let ledger = resources.claim(requirements)?;
        let route = ledger.reserve(
            ResourceScope::Object(page_id),
            ResourceClass::BackendStateBytes,
            u64::try_from(Self::base_route_bytes(capacity)?)
                .map_err(|_| ChromiumMouseInstallError::CapacityOverflow)?,
        )?;
        let backend = match dispatcher {
            Some(dispatcher) => factory.bind_with_dispatcher(client, input, dispatcher),
            None => factory.bind(client, input),
        }
        .map_err(ChromiumMouseInstallError::Backend)?;
        let shared = Arc::new(queue::Shared::new(
            page_id,
            backend,
            capacity.get(),
            ledger,
            route,
        ));
        Ok((
            Arc::new(Self {
                shared: shared.clone(),
            }),
            MouseRetirementDriver::new(shared),
        ))
    }
}
pub struct MouseCommandFuture<C: CommandSpec<Result = (), ErrorDetails = MouseErrorDetails>> {
    response: queue::Response,
    call_id: CallId,
    marker: PhantomData<fn() -> C>,
}
impl<C: CommandSpec<Result = (), ErrorDetails = MouseErrorDetails>> Future
    for MouseCommandFuture<C>
{
    type Output = Result<CommandResult<C>, Infallible>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        Pin::new(&mut this.response)
            .poll(cx)
            .map(|outcome| Ok(CommandResult::new(this.call_id, outcome)))
    }
}
macro_rules! mouse_command {
    ($command:ty,$service:ident) => {
        impl Service<Command<$command>> for ChromiumMousePage {
            type Response = CommandResult<$command>;
            type Error = Infallible;
            type Future<'a> = MouseCommandFuture<$command>;
            fn call(&self, command: Command<$command>) -> Self::Future<'_> {
                let (call_id, object_id, metadata, params) = command.into_parts();
                let context = OperationContext::new(
                    call_id,
                    object_id,
                    <$command>::METHOD,
                    metadata.cancellation,
                );
                let response = self
                    .shared
                    .admit(context, || self.shared.backend.$service.start(params));
                MouseCommandFuture {
                    response,
                    call_id,
                    marker: PhantomData,
                }
            }
        }
    };
}
mouse_command!(MouseMove, mouse_move);
mouse_command!(MouseDown, mouse_down);
mouse_command!(MouseUp, mouse_up);
mouse_command!(MouseClick, mouse_click);
mouse_command!(MouseDblclick, mouse_dblclick);
mouse_command!(MouseWheel, mouse_wheel);
pub type ChromiumMouseHandlers = MouseServerHandlers<
    ChromiumMousePage,
    ChromiumMousePage,
    ChromiumMousePage,
    ChromiumMousePage,
    ChromiumMousePage,
    ChromiumMousePage,
>;
/// Inert wire recipe. Caller supplies deadline/cancellation supervision and
/// continuously drives/consumes retirement receipts. The optional `runtime`
/// feature supplies a separate per-call invocation adapter; enabling it does
/// not insert a runtime into these plain generated wire handlers.
pub struct ChromiumMouseInstallation {
    page: Arc<ChromiumMousePage>,
    protocol: Arc<WireProtocol<MouseCodec>>,
    retirement: MouseRetirementDriver,
}
impl ChromiumMouseInstallation {
    pub fn install(
        page_id: ObjectId,
        client: Arc<dyn CdpClient>,
        input: CdpInputCoordinator,
        profile: ChromiumMouseProfile,
        delay: Arc<dyn CdpMouseDelay>,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumMouseInstallError> {
        resources.claim(profile.requirements())?;
        let factory = CdpMouseServiceFactory::new_with_production_resources(
            profile.backend,
            resources,
            delay,
        )?;
        let protocol = Arc::new(profile.protocol().with_production_resources(resources)?);
        let (page, retirement) = ChromiumMousePage::bind(
            page_id,
            client,
            input,
            &factory,
            profile.capacity,
            resources,
        )?;
        Ok(Self {
            page,
            protocol,
            retirement,
        })
    }
    pub fn page(&self) -> Arc<ChromiumMousePage> {
        self.page.clone()
    }
    pub fn protocol(&self) -> Arc<WireProtocol<MouseCodec>> {
        self.protocol.clone()
    }
    pub fn retirement(&mut self) -> &mut MouseRetirementDriver {
        &mut self.retirement
    }
    pub fn into_parts(
        self,
    ) -> (
        Arc<ChromiumMousePage>,
        Arc<WireProtocol<MouseCodec>>,
        MouseRetirementDriver,
    ) {
        (self.page, self.protocol, self.retirement)
    }
    pub fn handlers(&self) -> ChromiumMouseHandlers {
        mouse_server_handlers(
            self.protocol.clone(),
            MouseServerServices {
                mouse_move: self.page.clone(),
                mouse_down: self.page.clone(),
                mouse_up: self.page.clone(),
                mouse_click: self.page.clone(),
                mouse_dblclick: self.page.clone(),
                mouse_wheel: self.page.clone(),
            },
        )
    }
}
#[derive(Debug)]
pub enum ChromiumMouseInstallError {
    Production(ProductionResourceError),
    Route(pwright_core::ResourceError),
    Backend(pwright_core::OperationError<MouseErrorDetails>),
    CapacityOverflow,
}
impl From<ProductionResourceError> for ChromiumMouseInstallError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Production(error)
    }
}
impl From<pwright_core::ResourceError> for ChromiumMouseInstallError {
    fn from(error: pwright_core::ResourceError) -> Self {
        Self::Route(error)
    }
}
impl std::fmt::Display for ChromiumMouseInstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Production(e) => e.fmt(f),
            Self::Route(e) => e.fmt(f),
            Self::Backend(e) => e.fmt(f),
            Self::CapacityOverflow => f.write_str("Mouse route capacity overflows storage budget"),
        }
    }
}
impl std::error::Error for ChromiumMouseInstallError {}
#[cfg(test)]
mod tests;
