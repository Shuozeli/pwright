//! Optional Keyboard-only Page binding, without browser discovery or an executor.
//! The caller owns Page identity, the shared input coordinator and lifecycle.
pub use pwright_backend_cdp_input::{CdpInputCoordinator, CdpKeyDownInterceptor};
use pwright_backend_cdp_keyboard::CdpKeyboardServices;
pub use pwright_backend_cdp_keyboard::{
    CdpKeyboardDelay, CdpKeyboardProductionProfile, CdpKeyboardServiceFactory, KeyboardPlatform,
};
use pwright_cap_keyboard::*;
use pwright_cdp::CdpClient;
use pwright_codec_keyboard::KeyboardCodec;
pub use pwright_codec_keyboard::KeyboardCodecLimits;
use pwright_core::{
    CancellationReason, Command, CommandCancellation, CommandResult, FailureKind, OperationContext,
    OperationError, ResourceLease, Service,
};
pub use pwright_core::{
    ObjectId, ProductionResourceError, ProductionResourceLedger, ProtocolVersion, ResourceClass,
    ResourceRequirements, ResourceScope,
};
use pwright_server_keyboard::{
    KeyboardServerHandlers, KeyboardServerServices, keyboard_server_handlers,
};
pub use pwright_transport::FrameLimits;
use pwright_transport::WireProtocol;
use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

/// Complete backend and one encoded-frame plan for a single installed Page.
/// The caller separately budgets ingress/decoded arguments, queued replies,
/// transport buffers, Page input state and executor/runtime scheduling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumKeyboardProfile {
    backend: CdpKeyboardProductionProfile,
    codec: KeyboardCodecLimits,
    version: ProtocolVersion,
    send: FrameLimits,
    receive: FrameLimits,
}
impl ChromiumKeyboardProfile {
    pub const fn new(
        backend: CdpKeyboardProductionProfile,
        codec: KeyboardCodecLimits,
        version: ProtocolVersion,
        send: FrameLimits,
        receive: FrameLimits,
    ) -> Self {
        Self {
            backend,
            codec,
            version,
            send,
            receive,
        }
    }
    pub const fn backend(self) -> CdpKeyboardProductionProfile {
        self.backend
    }
    pub fn requirements(self) -> ResourceRequirements {
        self.backend
            .requirements()
            .combine_concurrent(ChromiumKeyboardPage::route_requirements())
            .combine_concurrent(self.protocol().production_resource_requirements())
    }
    fn protocol(self) -> WireProtocol<KeyboardCodec> {
        WireProtocol::new_exact_directional(
            KeyboardCodec::new(self.codec),
            self.version,
            self.send,
            self.receive,
        )
    }
}

/// One identity-checked route. Escaped Arc handles retain the route lease.
/// Revocation is irreversible local admission closure, not native retirement.
pub struct ChromiumKeyboardPage {
    page_id: ObjectId,
    revoked: AtomicBool,
    backend: CdpKeyboardServices,
    // Kept last so retained native bindings drop before their route storage lease.
    _route: ResourceLease,
}
impl ChromiumKeyboardPage {
    pub const fn page_id(&self) -> ObjectId {
        self.page_id
    }
    pub fn shares_input_state_with(&self, input: &CdpInputCoordinator) -> bool {
        self.backend.keyboard_down.shares_input_state_with(input)
    }
    pub fn is_revoked(&self) -> bool {
        self.revoked.load(Ordering::Acquire)
    }
    /// Reject new calls and calls not yet polled. Already-polled operations are
    /// owned by the embedding controlled runtime; this does not abort or wake them.
    pub fn revoke_admission(&self) {
        self.revoked.store(true, Ordering::Release);
    }
    pub fn route_requirements() -> ResourceRequirements {
        let bytes = (std::mem::size_of::<Self>()
            + 2 * std::mem::size_of::<usize>()
            + CdpKeyboardServices::shared_binding_bytes()) as u64;
        ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, 1)
    }
    /// Reuse one factory for all Pages that should share an operation quota.
    /// No replacement input coordinator, private factory or transport is created.
    pub fn bind(
        page_id: ObjectId,
        client: Arc<dyn CdpClient>,
        input: CdpInputCoordinator,
        factory: &CdpKeyboardServiceFactory,
        resources: &ProductionResourceLedger,
    ) -> Result<Arc<Self>, ChromiumKeyboardInstallError> {
        Self::bind_with_key_down_interceptor(page_id, client, input, factory, resources, None)
    }
    /// The embedding Page owns the interceptor's budget and native retirement.
    pub fn bind_with_key_down_interceptor(
        page_id: ObjectId,
        client: Arc<dyn CdpClient>,
        input: CdpInputCoordinator,
        factory: &CdpKeyboardServiceFactory,
        resources: &ProductionResourceLedger,
        interceptor: Option<Arc<dyn CdpKeyDownInterceptor>>,
    ) -> Result<Arc<Self>, ChromiumKeyboardInstallError> {
        let requirements = Self::route_requirements();
        // A route lives concurrently with its backend operations. Checking only
        // the smaller route subtotal could accidentally reuse a backend-only plan.
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
            requirements.class(ResourceClass::BackendStateBytes).global,
        )?;
        Ok(Arc::new(Self {
            page_id,
            revoked: AtomicBool::new(false),
            backend: factory.bind_with_key_down_interceptor(client, input, interceptor),
            _route: route,
        }))
    }
    fn validate_identity(
        &self,
        actual: ObjectId,
    ) -> Result<(), OperationError<KeyboardErrorDetails>> {
        if actual != self.page_id {
            return Err(OperationError::new(
                FailureKind::ProtocolViolation,
                "Keyboard command targeted an unbound Page",
                None,
            ));
        }
        Ok(())
    }
    fn validate_admission(
        &self,
        actual: ObjectId,
    ) -> Result<(), OperationError<KeyboardErrorDetails>> {
        self.validate_identity(actual)?;
        if self.is_revoked() {
            return Err(OperationError::new(
                FailureKind::TargetClosed,
                "Keyboard Page admission is revoked",
                None,
            ));
        }
        Ok(())
    }
}

macro_rules! page_command {
    ($command:ty, $params:ty, $service:ident) => {
        // The controlled runtime can borrow this exact route as its operation
        // handler and retain the same Arc as its cancellation handler. Distinct
        // parameter types prevent down/up from aliasing in type-based dispatch.
        #[cfg(feature = "runtime")]
        impl pwright_runtime::TargetedService<$params> for ChromiumKeyboardPage {
            type Response = ();
            type Error = OperationError<KeyboardErrorDetails>;
            type Future<'a> = pwright_backend_cdp_keyboard::KeyboardFuture<'a>;
            fn call_for(&self, object_id: ObjectId, params: $params) -> Self::Future<'_> {
                if let Err(error) = self.validate_admission(object_id) {
                    return Box::pin(async move { Err(error) });
                }
                let operation = self.backend.$service.call(params);
                Box::pin(async move {
                    self.validate_admission(object_id)?;
                    operation.await
                })
            }
        }
        impl Service<Command<$command>> for ChromiumKeyboardPage {
            type Response = CommandResult<$command>;
            type Error = Infallible;
            type Future<'a> =
                Pin<Box<dyn Future<Output = Result<Self::Response, Infallible>> + Send + 'a>>;
            fn call(&self, command: Command<$command>) -> Self::Future<'_> {
                let (call_id, object_id, _metadata, params) = command.into_parts();
                let operation = match self.validate_admission(object_id) {
                    // Preserve eager backend admission before returning this route future.
                    Ok(()) => Ok(self.backend.$service.call(params)),
                    Err(error) => Err(error),
                };
                Box::pin(async move {
                    let outcome = match operation {
                        Err(error) => Err(error),
                        Ok(operation) => match self.validate_admission(object_id) {
                            Ok(()) => operation.await,
                            Err(error) => Err(error),
                        },
                    };
                    Ok(CommandResult::new(call_id, outcome))
                })
            }
        }
        impl CommandCancellation<$command> for ChromiumKeyboardPage {
            type Future<'a> = pwright_backend_cdp_keyboard::KeyboardFuture<'a>;
            fn cancel(
                &self,
                context: OperationContext,
                reason: CancellationReason,
            ) -> Self::Future<'_> {
                // Cleanup remains addressable after admission revocation. The caller
                // must first settle/drop the operation; no native receipt is invented.
                let identity = self.validate_identity(context.object_id());
                Box::pin(async move {
                    identity?;
                    self.backend.$service.cancel(context, reason).await
                })
            }
        }
    };
}
page_command!(KeyboardDown, DownParams, keyboard_down);
page_command!(KeyboardUp, UpParams, keyboard_up);
page_command!(KeyboardPress, PressParams, keyboard_press);
page_command!(KeyboardInsertText, InsertTextParams, keyboard_insert_text);
page_command!(KeyboardType, TypeParams, keyboard_type);

pub type ChromiumKeyboardHandlers = KeyboardServerHandlers<
    ChromiumKeyboardPage,
    ChromiumKeyboardPage,
    ChromiumKeyboardPage,
    ChromiumKeyboardPage,
    ChromiumKeyboardPage,
>;

/// Inert single-Page wire recipe. No runtime, executor, browser or task is started.
pub struct ChromiumKeyboardInstallation {
    page: Arc<ChromiumKeyboardPage>,
    protocol: Arc<WireProtocol<KeyboardCodec>>,
}
impl ChromiumKeyboardInstallation {
    pub fn install(
        page_id: ObjectId,
        client: Arc<dyn CdpClient>,
        input: CdpInputCoordinator,
        profile: ChromiumKeyboardProfile,
        delay: Arc<dyn CdpKeyboardDelay>,
        platform: KeyboardPlatform,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumKeyboardInstallError> {
        // Claim the whole concurrent plan before binding anything.
        resources.claim(profile.requirements())?;
        let factory = CdpKeyboardServiceFactory::new_with_production_resources(
            profile.backend,
            resources,
            delay,
            platform,
        )?;
        let protocol = Arc::new(profile.protocol().with_production_resources(resources)?);
        let page = ChromiumKeyboardPage::bind(page_id, client, input, &factory, resources)?;
        Ok(Self { page, protocol })
    }
    pub fn page(&self) -> Arc<ChromiumKeyboardPage> {
        self.page.clone()
    }
    pub fn protocol(&self) -> Arc<WireProtocol<KeyboardCodec>> {
        self.protocol.clone()
    }
    pub fn handlers(&self) -> ChromiumKeyboardHandlers {
        keyboard_server_handlers(
            self.protocol.clone(),
            KeyboardServerServices {
                keyboard_down: self.page.clone(),
                keyboard_up: self.page.clone(),
                keyboard_press: self.page.clone(),
                keyboard_insert_text: self.page.clone(),
                keyboard_type: self.page.clone(),
            },
        )
    }
}

#[derive(Debug)]
pub enum ChromiumKeyboardInstallError {
    Production(ProductionResourceError),
    Route(pwright_core::ResourceError),
}
impl From<ProductionResourceError> for ChromiumKeyboardInstallError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Production(error)
    }
}
impl From<pwright_core::ResourceError> for ChromiumKeyboardInstallError {
    fn from(error: pwright_core::ResourceError) -> Self {
        Self::Route(error)
    }
}
impl std::fmt::Display for ChromiumKeyboardInstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Production(error) => error.fmt(f),
            Self::Route(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for ChromiumKeyboardInstallError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Production(error) => Some(error),
            Self::Route(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests;
