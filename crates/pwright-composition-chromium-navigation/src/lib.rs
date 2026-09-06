//! Minimal Chromium CDP composition for the optional navigation capability.
//!
//! This leaf binds one pre-existing Page object to an injected CDP client and
//! exposes the generated `pwright.navigation` server handlers. It does not
//! discover, download, launch, or own Chromium; select an executor; or install
//! any unrelated Pwright capability.

use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub use pwright_backend_cdp_navigation::{
    CdpNavigationLimits, CdpNavigationProductionProfile, CdpNavigationService,
    CdpNavigationServiceFactory,
};
use pwright_cap_navigation::{BeginNavigation, Goto, NavigationErrorDetails};
use pwright_cdp::CdpClient;
use pwright_codec_navigation::NavigationCodec;
pub use pwright_codec_navigation::NavigationCodecLimits;
use pwright_core::{
    CancellationReason, Command, CommandCancellation, CommandResult, FailureKind, OperationContext,
    OperationError, Service,
};
pub use pwright_core::{
    ObjectId, ProductionResourceError, ProductionResourceLedger, ProtocolVersion, ResourceLimits,
    ResourceRequirements, ResourceScope,
};
use pwright_server_navigation::{
    NavigationServerHandlers, NavigationServerServices, navigation_server_handlers,
};
pub use pwright_transport::FrameLimits;
use pwright_transport::WireProtocol;

/// Wire and backend bounds for one installed navigation-only Page.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumNavigationProfile {
    backend: CdpNavigationProductionProfile,
    codec: NavigationCodecLimits,
    version: ProtocolVersion,
    send_limits: FrameLimits,
    receive_limits: FrameLimits,
}

impl ChromiumNavigationProfile {
    pub const fn new(
        backend: CdpNavigationProductionProfile,
        codec: NavigationCodecLimits,
        version: ProtocolVersion,
        send_limits: FrameLimits,
        receive_limits: FrameLimits,
    ) -> Self {
        Self {
            backend,
            codec,
            version,
            send_limits,
            receive_limits,
        }
    }

    pub const fn backend(self) -> CdpNavigationProductionProfile {
        self.backend
    }

    pub const fn codec(self) -> NavigationCodecLimits {
        self.codec
    }

    pub const fn version(self) -> ProtocolVersion {
        self.version
    }

    pub const fn send_limits(self) -> FrameLimits {
        self.send_limits
    }

    pub const fn receive_limits(self) -> FrameLimits {
        self.receive_limits
    }

    /// Minimum shared-ledger declaration for the backend operation and one
    /// simultaneously encoded navigation frame.
    pub fn requirements(self) -> ResourceRequirements {
        self.backend.requirements().combine_concurrent(
            self.unbudgeted_protocol()
                .production_resource_requirements(),
        )
    }

    fn unbudgeted_protocol(self) -> WireProtocol<NavigationCodec> {
        WireProtocol::new_exact_directional(
            NavigationCodec::new(self.codec),
            self.version,
            self.send_limits,
            self.receive_limits,
        )
    }
}

/// One explicitly routed Page backed only by the Chromium CDP navigation
/// adapter.
#[derive(Clone)]
pub struct ChromiumNavigationPage {
    page_id: ObjectId,
    backend: Arc<CdpNavigationService>,
}

impl ChromiumNavigationPage {
    pub const fn page_id(&self) -> ObjectId {
        self.page_id
    }

    pub fn backend(&self) -> &CdpNavigationService {
        self.backend.as_ref()
    }

    fn wrong_target(&self, actual: ObjectId) -> OperationError<NavigationErrorDetails> {
        OperationError::new(
            FailureKind::ProtocolViolation,
            format!(
                "navigation targeted Page `{actual}`, expected `{}`",
                self.page_id
            ),
            None,
        )
    }
}

impl Service<Command<BeginNavigation>> for ChromiumNavigationPage {
    type Response = CommandResult<BeginNavigation>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<BeginNavigation>) -> Self::Future<'_> {
        let (call_id, object_id, _metadata, params) = command.into_parts();
        let route = (object_id == self.page_id)
            .then_some(())
            .ok_or_else(|| self.wrong_target(object_id));
        Box::pin(async move {
            let outcome = match route {
                Ok(()) => self.backend.call(params).await,
                Err(error) => Err(error),
            };
            Ok(CommandResult::new(call_id, outcome))
        })
    }
}

impl Service<Command<Goto>> for ChromiumNavigationPage {
    type Response = CommandResult<Goto>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<Goto>) -> Self::Future<'_> {
        let (call_id, object_id, _metadata, params) = command.into_parts();
        let route = (object_id == self.page_id)
            .then_some(())
            .ok_or_else(|| self.wrong_target(object_id));
        Box::pin(async move {
            let outcome = match route {
                Ok(()) => self.backend.call(params).await,
                Err(error) => Err(error),
            };
            Ok(CommandResult::new(call_id, outcome))
        })
    }
}

macro_rules! impl_navigation_cancellation {
    ($command:ty) => {
        impl CommandCancellation<$command> for ChromiumNavigationPage {
            type Future<'a> = Pin<
                Box<
                    dyn Future<Output = Result<(), OperationError<NavigationErrorDetails>>>
                        + Send
                        + 'a,
                >,
            >;

            fn cancel(
                &self,
                context: OperationContext,
                reason: CancellationReason,
            ) -> Self::Future<'_> {
                let route = (context.object_id() == self.page_id)
                    .then_some(())
                    .ok_or_else(|| self.wrong_target(context.object_id()));
                Box::pin(async move {
                    route?;
                    <CdpNavigationService as CommandCancellation<$command>>::cancel(
                        self.backend.as_ref(),
                        context,
                        reason,
                    )
                    .await
                })
            }
        }
    };
}

impl_navigation_cancellation!(BeginNavigation);
impl_navigation_cancellation!(Goto);

pub type ChromiumNavigationHandlers =
    NavigationServerHandlers<ChromiumNavigationPage, ChromiumNavigationPage>;

/// A fully bound navigation-only server leaf. It is inert until the caller
/// drives the returned handlers with its chosen transport and executor.
pub struct ChromiumNavigationInstallation {
    page: Arc<ChromiumNavigationPage>,
    protocol: Arc<WireProtocol<NavigationCodec>>,
}

impl ChromiumNavigationInstallation {
    pub fn install(
        page_id: ObjectId,
        client: Arc<dyn CdpClient>,
        profile: ChromiumNavigationProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        let backend = CdpNavigationServiceFactory::new_with_production_resources(
            profile.backend(),
            resources,
        )?
        .bind(client);
        let protocol = profile
            .unbudgeted_protocol()
            .with_production_resources(resources)?;
        Ok(Self {
            page: Arc::new(ChromiumNavigationPage {
                page_id,
                backend: Arc::new(backend),
            }),
            protocol: Arc::new(protocol),
        })
    }

    pub fn page(&self) -> Arc<ChromiumNavigationPage> {
        Arc::clone(&self.page)
    }

    pub fn protocol(&self) -> Arc<WireProtocol<NavigationCodec>> {
        Arc::clone(&self.protocol)
    }

    pub fn handlers(&self) -> ChromiumNavigationHandlers {
        navigation_server_handlers(
            Arc::clone(&self.protocol),
            NavigationServerServices {
                begin_navigation: Arc::clone(&self.page),
                goto: Arc::clone(&self.page),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_backend_cdp_navigation::{CdpNavigationLimits, CdpNavigationProductionProfile};
    use pwright_cap_navigation::{BeginNavigation, BeginNavigationParams};
    use pwright_core::{
        CallId, CallMetadata, Command, FailureKind, ObjectId, ProductionResourceLedger,
        ProtocolVersion, ResourceClass, ResourceLimits, ResourceScope, Service,
    };
    use pwright_fake::FakeCdpClient;
    use pwright_transport::FrameLimits;
    use serde_json::json;

    use super::{ChromiumNavigationInstallation, ChromiumNavigationProfile};

    fn profile() -> ChromiumNavigationProfile {
        ChromiumNavigationProfile::new(
            CdpNavigationProductionProfile::new(
                ResourceScope::Object(ObjectId::new(7)),
                CdpNavigationLimits::new(256, 64, 4, 4, 256, 64),
                1,
            ),
            pwright_codec_navigation::NavigationCodecLimits::new(256, 4, 256),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(1_024, 0, 0, 1_024),
            FrameLimits::new(1_024, 0, 0, 1_024),
        )
    }

    fn resources(profile: ChromiumNavigationProfile) -> ProductionResourceLedger {
        let requirements = profile.requirements();
        let backend = requirements.class(ResourceClass::BackendStateBytes);
        let wire = requirements.class(ResourceClass::WireBytes);
        let limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases)
            .with_class(
                ResourceClass::BackendStateBytes,
                backend.global,
                backend.per_scope,
            )
            .with_class(ResourceClass::WireBytes, wire.global, wire.per_scope);
        ProductionResourceLedger::new(limits, requirements)
            .expect("exact navigation resources should build")
    }

    #[tokio::test]
    async fn installed_page_executes_navigation_through_the_cdp_adapter() {
        // Arrange
        let profile = profile();
        let resources = resources(profile);
        let fake = Arc::new(FakeCdpClient::from_html(""));
        fake.set_navigate_response(json!({
            "frameId": "frame-1",
            "loaderId": "loader-1"
        }));
        let client: Arc<dyn pwright_cdp::CdpClient> = fake;
        let installation =
            ChromiumNavigationInstallation::install(ObjectId::new(7), client, profile, &resources)
                .expect("navigation-only composition should install");
        let command = Command::<BeginNavigation>::new(
            CallId::new(11),
            ObjectId::new(7),
            CallMetadata::default(),
            BeginNavigationParams::new("https://example.com"),
        );

        // Act
        let result = installation
            .page()
            .call(command)
            .await
            .expect("command service is infallible")
            .into_outcome()
            .expect("CDP navigation should succeed");

        // Assert
        assert_eq!(result.frame_id, "frame-1");
        assert_eq!(result.loader_id.as_deref(), Some("loader-1"));
        assert!(!result.same_document);
        let _handlers = installation.handlers();
    }

    #[tokio::test]
    async fn installed_page_rejects_an_unbound_object_before_cdp() {
        // Arrange
        let profile = profile();
        let resources = resources(profile);
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let client: Arc<dyn pwright_cdp::CdpClient> = fake;
        let installation =
            ChromiumNavigationInstallation::install(ObjectId::new(7), client, profile, &resources)
                .expect("navigation-only composition should install");
        let command = Command::<BeginNavigation>::new(
            CallId::new(12),
            ObjectId::new(8),
            CallMetadata::default(),
            BeginNavigationParams::new("https://example.com"),
        );

        // Act
        let error = installation
            .page()
            .call(command)
            .await
            .expect("command service is infallible")
            .into_outcome()
            .expect_err("an unbound Page must be rejected");

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    }
}
