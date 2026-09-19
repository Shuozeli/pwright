//! Minimal Chromium CDP composition for the optional Locator capability.
//!
//! This leaf binds one pre-existing Page or Frame object to an injected,
//! target-scoped CDP client and exposes exactly one generated Locator protocol
//! major. It does not discover, download, launch, or own Chromium; install
//! screenshot/navigation/evaluation; or select a transport, runtime, or
//! executor.

use std::convert::Infallible;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub use pwright_backend_cdp_input::CdpInputCoordinator;
pub use pwright_backend_cdp_locator::{
    CdpLocatorProductionProfile, CdpLocatorService, CdpLocatorServiceFactory,
    SelectorRuntimeMetrics,
};
use pwright_cap_locator::{
    AllInnerTexts, AllTextContents, Blur, Click, Count, Fill, Focus, GetAttribute, Hover,
    InnerHtml, InnerText, InputValue, IsChecked, IsEditable, IsEnabled, IsVisible,
    LocatorErrorDetails, Press, ScrollIntoViewIfNeeded, SelectOption, SetChecked, TextContent,
    WaitFor, v2 as locator_v2,
};
use pwright_cdp::CdpClient;
pub use pwright_codec_locator::LocatorCodecLimits;
use pwright_codec_locator::{LocatorCodec, LocatorV2Codec};
use pwright_core::{
    CancellationReason, Command, CommandCancellation, CommandResult, CommandSpec, FailureKind,
    OperationContext, OperationError, ResourceError, ResourceLease, Service,
};
pub use pwright_core::{
    ObjectId, ProductionResourceError, ProductionResourceLedger, ProtocolVersion, ResourceLimits,
    ResourceRequirements, ResourceScope,
};
use pwright_server_locator::{
    LocatorServerHandlers, LocatorServerHandlersLeft, LocatorServerHandlersRight,
    LocatorServerServices, LocatorServerServicesLeft, LocatorServerServicesRight,
    SelectedLocatorServerHandlers, locator_server_handlers,
    v2::{
        LocatorV2ServerHandlers, LocatorV2ServerHandlersLeft, LocatorV2ServerHandlersRight,
        LocatorV2ServerServices, LocatorV2ServerServicesLeft, LocatorV2ServerServicesRight,
        locator_v2_server_handlers,
    },
};
pub use pwright_transport::FrameLimits;
use pwright_transport::WireProtocol;

/// Locator protocol table installed for this leaf.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChromiumLocatorTrack {
    V1,
    V2,
}

impl ChromiumLocatorTrack {
    pub const fn version(self) -> ProtocolVersion {
        match self {
            Self::V1 => ProtocolVersion::new(1, 23),
            Self::V2 => ProtocolVersion::new(2, 6),
        }
    }
}

/// Wire and backend bounds for one installed Locator-only Page or Frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumLocatorProfile {
    backend: CdpLocatorProductionProfile,
    codec: LocatorCodecLimits,
    track: ChromiumLocatorTrack,
    send_limits: FrameLimits,
    receive_limits: FrameLimits,
}

impl ChromiumLocatorProfile {
    pub const fn v1(
        backend: CdpLocatorProductionProfile,
        codec: LocatorCodecLimits,
        send_limits: FrameLimits,
        receive_limits: FrameLimits,
    ) -> Self {
        Self {
            backend,
            codec,
            track: ChromiumLocatorTrack::V1,
            send_limits,
            receive_limits,
        }
    }

    pub const fn v2(
        backend: CdpLocatorProductionProfile,
        codec: LocatorCodecLimits,
        send_limits: FrameLimits,
        receive_limits: FrameLimits,
    ) -> Self {
        Self {
            backend,
            codec,
            track: ChromiumLocatorTrack::V2,
            send_limits,
            receive_limits,
        }
    }

    pub const fn backend(self) -> CdpLocatorProductionProfile {
        self.backend
    }

    pub const fn codec(self) -> LocatorCodecLimits {
        self.codec
    }

    pub const fn track(self) -> ChromiumLocatorTrack {
        self.track
    }

    pub const fn version(self) -> ProtocolVersion {
        self.track.version()
    }

    pub const fn send_limits(self) -> FrameLimits {
        self.send_limits
    }

    pub const fn receive_limits(self) -> FrameLimits {
        self.receive_limits
    }

    /// Minimum shared-ledger declaration for Locator backend state, an
    /// optional retained execution-context route, and one encoded frame.
    pub fn requirements(self) -> ResourceRequirements {
        let wire = match self.track {
            ChromiumLocatorTrack::V1 => self
                .unbudgeted_v1_protocol()
                .production_resource_requirements(),
            ChromiumLocatorTrack::V2 => self
                .unbudgeted_v2_protocol()
                .production_resource_requirements(),
        };
        self.backend.requirements().combine_concurrent(wire)
    }

    fn unbudgeted_v1_protocol(self) -> WireProtocol<LocatorCodec> {
        WireProtocol::new_exact_directional(
            LocatorCodec::new(self.codec).with_capability_version(self.version()),
            self.version(),
            self.send_limits,
            self.receive_limits,
        )
    }

    fn unbudgeted_v2_protocol(self) -> WireProtocol<LocatorV2Codec> {
        WireProtocol::new_exact_directional(
            LocatorV2Codec::new(self.codec).with_capability_version(self.version()),
            self.version(),
            self.send_limits,
            self.receive_limits,
        )
    }
}

/// Why a Locator-only Page or Frame could not be installed.
#[derive(Debug)]
pub enum ChromiumLocatorInstallError {
    MainTargetProfileHasContextRoutes {
        max_context_routes: usize,
        max_native_id_bytes: usize,
    },
    UniqueContextRouteNotBudgeted,
    EmptyUniqueContextId,
    UniqueContextIdTooLarge {
        actual: usize,
        limit: usize,
    },
    Resources(ProductionResourceError),
    RouteResources(ResourceError),
}

impl Display for ChromiumLocatorInstallError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MainTargetProfileHasContextRoutes {
                max_context_routes,
                max_native_id_bytes,
            } => write!(
                formatter,
                "main-target Locator does not retain an execution-context route, but the profile declares {max_context_routes} routes with {max_native_id_bytes} native-ID bytes"
            ),
            Self::UniqueContextRouteNotBudgeted => formatter.write_str(
                "unique-context Locator requires a nonzero context-route and native-ID budget",
            ),
            Self::EmptyUniqueContextId => {
                formatter.write_str("the Chromium unique execution-context ID must not be empty")
            }
            Self::UniqueContextIdTooLarge { actual, limit } => write!(
                formatter,
                "the Chromium unique execution-context ID has {actual} bytes, limit is {limit}"
            ),
            Self::Resources(error) => Display::fmt(error, formatter),
            Self::RouteResources(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ChromiumLocatorInstallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resources(error) => Some(error),
            Self::RouteResources(error) => Some(error),
            Self::MainTargetProfileHasContextRoutes { .. }
            | Self::UniqueContextRouteNotBudgeted
            | Self::EmptyUniqueContextId
            | Self::UniqueContextIdTooLarge { .. } => None,
        }
    }
}

impl From<ProductionResourceError> for ChromiumLocatorInstallError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Resources(error)
    }
}

impl From<ResourceError> for ChromiumLocatorInstallError {
    fn from(error: ResourceError) -> Self {
        Self::RouteResources(error)
    }
}

/// One explicitly routed Page or Frame backed only by the Chromium Locator
/// adapter.
pub struct ChromiumLocatorTarget {
    target_id: ObjectId,
    backend: Arc<CdpLocatorService>,
}

impl ChromiumLocatorTarget {
    pub const fn target_id(&self) -> ObjectId {
        self.target_id
    }

    pub fn backend(&self) -> &CdpLocatorService {
        self.backend.as_ref()
    }

    fn wrong_target(&self, actual: ObjectId) -> OperationError<LocatorErrorDetails> {
        OperationError::new(
            FailureKind::ProtocolViolation,
            format!(
                "Locator targeted Page or Frame `{actual}`, expected `{}`",
                self.target_id
            ),
            None,
        )
    }
}

macro_rules! impl_locator_command {
    ($command:ty) => {
        impl Service<Command<$command>> for ChromiumLocatorTarget {
            type Response = CommandResult<$command>;
            type Error = Infallible;
            type Future<'a> =
                Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

            fn call(&self, command: Command<$command>) -> Self::Future<'_> {
                let (call_id, object_id, _metadata, params) = command.into_parts();
                let route = (object_id == self.target_id)
                    .then_some(())
                    .ok_or_else(|| self.wrong_target(object_id));
                Box::pin(async move {
                    let outcome =
                        match route {
                            Ok(()) => <CdpLocatorService as Service<
                                <$command as CommandSpec>::Params,
                            >>::call(self.backend.as_ref(), params)
                            .await,
                            Err(error) => Err(error),
                        };
                    Ok(CommandResult::new(call_id, outcome))
                })
            }
        }

        impl CommandCancellation<$command> for ChromiumLocatorTarget {
            type Future<'a> = Pin<
                Box<
                    dyn Future<
                            Output = Result<
                                (),
                                OperationError<<$command as CommandSpec>::ErrorDetails>,
                            >,
                        > + Send
                        + 'a,
                >,
            >;

            fn cancel(
                &self,
                context: OperationContext,
                reason: CancellationReason,
            ) -> Self::Future<'_> {
                let route = (context.object_id() == self.target_id)
                    .then_some(())
                    .ok_or_else(|| self.wrong_target(context.object_id()));
                Box::pin(async move {
                    route?;
                    <CdpLocatorService as CommandCancellation<$command>>::cancel(
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

impl_locator_command!(AllInnerTexts);
impl_locator_command!(AllTextContents);
impl_locator_command!(Blur);
impl_locator_command!(Click);
impl_locator_command!(Count);
impl_locator_command!(Fill);
impl_locator_command!(Focus);
impl_locator_command!(GetAttribute);
impl_locator_command!(Hover);
impl_locator_command!(InnerHtml);
impl_locator_command!(InnerText);
impl_locator_command!(InputValue);
impl_locator_command!(IsChecked);
impl_locator_command!(IsEditable);
impl_locator_command!(IsEnabled);
impl_locator_command!(IsVisible);
impl_locator_command!(Press);
impl_locator_command!(pwright_cap_locator::PressSequentially);
impl_locator_command!(ScrollIntoViewIfNeeded);
impl_locator_command!(SelectOption);
impl_locator_command!(SetChecked);
impl_locator_command!(TextContent);
impl_locator_command!(WaitFor);
impl_locator_command!(locator_v2::AllInnerTexts);
impl_locator_command!(locator_v2::AllTextContents);
impl_locator_command!(locator_v2::Blur);
impl_locator_command!(locator_v2::Click);
impl_locator_command!(locator_v2::Count);
impl_locator_command!(locator_v2::Fill);
impl_locator_command!(locator_v2::Focus);
impl_locator_command!(locator_v2::GetAttribute);
impl_locator_command!(locator_v2::Hover);
impl_locator_command!(locator_v2::InnerHtml);
impl_locator_command!(locator_v2::InnerText);
impl_locator_command!(locator_v2::InputValue);
impl_locator_command!(locator_v2::IsChecked);
impl_locator_command!(locator_v2::IsEditable);
impl_locator_command!(locator_v2::IsEnabled);
impl_locator_command!(locator_v2::IsVisible);
impl_locator_command!(locator_v2::Press);
impl_locator_command!(locator_v2::PressSequentially);
impl_locator_command!(locator_v2::ScrollIntoViewIfNeeded);
impl_locator_command!(locator_v2::SelectOption);
impl_locator_command!(locator_v2::SetChecked);
impl_locator_command!(locator_v2::TextContent);
impl_locator_command!(locator_v2::WaitFor);

pub type ChromiumLocatorV1Handlers = LocatorServerHandlers<
    LocatorServerHandlersLeft<
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
    >,
    LocatorServerHandlersRight<
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
    >,
>;

pub type ChromiumLocatorV2Handlers = LocatorV2ServerHandlers<
    LocatorV2ServerHandlersLeft<
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
    >,
    LocatorV2ServerHandlersRight<
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
        ChromiumLocatorTarget,
    >,
>;

pub type ChromiumLocatorHandlers =
    SelectedLocatorServerHandlers<ChromiumLocatorV1Handlers, ChromiumLocatorV2Handlers>;

/// The single codec/protocol major retained by an installation.
#[derive(Clone)]
pub enum ChromiumLocatorProtocol {
    V1(Arc<WireProtocol<LocatorCodec>>),
    V2(Arc<WireProtocol<LocatorV2Codec>>),
}

impl ChromiumLocatorProtocol {
    pub const fn track(&self) -> ChromiumLocatorTrack {
        match self {
            Self::V1(_) => ChromiumLocatorTrack::V1,
            Self::V2(_) => ChromiumLocatorTrack::V2,
        }
    }
}

/// A fully bound Locator-only server leaf. It is inert until the caller drives
/// the returned handlers with its chosen transport and executor.
pub struct ChromiumLocatorInstallation {
    target: Arc<ChromiumLocatorTarget>,
    protocol: ChromiumLocatorProtocol,
    _route_resources: Option<ResourceLease>,
}

impl ChromiumLocatorInstallation {
    pub fn install_main_target(
        target_id: ObjectId,
        client: Arc<dyn CdpClient>,
        input: CdpInputCoordinator,
        profile: ChromiumLocatorProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumLocatorInstallError> {
        let backend_profile = profile.backend();
        if backend_profile.max_context_routes() != 0 || backend_profile.max_native_id_bytes() != 0 {
            return Err(
                ChromiumLocatorInstallError::MainTargetProfileHasContextRoutes {
                    max_context_routes: backend_profile.max_context_routes(),
                    max_native_id_bytes: backend_profile.max_native_id_bytes(),
                },
            );
        }
        let factory =
            CdpLocatorServiceFactory::new_with_production_resources(backend_profile, resources)?;
        Self::finish(
            target_id,
            factory.bind_with_input(client, input),
            profile,
            resources,
            None,
        )
    }

    pub fn install_unique_context(
        target_id: ObjectId,
        client: Arc<dyn CdpClient>,
        unique_context_id: impl Into<String>,
        input: CdpInputCoordinator,
        profile: ChromiumLocatorProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumLocatorInstallError> {
        let backend_profile = profile.backend();
        if backend_profile.max_context_routes() == 0 || backend_profile.max_native_id_bytes() == 0 {
            return Err(ChromiumLocatorInstallError::UniqueContextRouteNotBudgeted);
        }
        let unique_context_id = unique_context_id.into();
        if unique_context_id.is_empty() {
            return Err(ChromiumLocatorInstallError::EmptyUniqueContextId);
        }
        if unique_context_id.len() > backend_profile.max_native_id_bytes() {
            return Err(ChromiumLocatorInstallError::UniqueContextIdTooLarge {
                actual: unique_context_id.len(),
                limit: backend_profile.max_native_id_bytes(),
            });
        }
        let factory =
            CdpLocatorServiceFactory::new_with_production_resources(backend_profile, resources)?;
        let route_resources = factory
            .reserve_context_routes()?
            .ok_or(ChromiumLocatorInstallError::UniqueContextRouteNotBudgeted)?;
        Self::finish(
            target_id,
            factory.bind_unique_context_with_input(client, unique_context_id, input),
            profile,
            resources,
            Some(route_resources),
        )
    }

    fn finish(
        target_id: ObjectId,
        backend: CdpLocatorService,
        profile: ChromiumLocatorProfile,
        resources: &ProductionResourceLedger,
        route_resources: Option<ResourceLease>,
    ) -> Result<Self, ChromiumLocatorInstallError> {
        let protocol = match profile.track() {
            ChromiumLocatorTrack::V1 => ChromiumLocatorProtocol::V1(Arc::new(
                profile
                    .unbudgeted_v1_protocol()
                    .with_production_resources(resources)?,
            )),
            ChromiumLocatorTrack::V2 => ChromiumLocatorProtocol::V2(Arc::new(
                profile
                    .unbudgeted_v2_protocol()
                    .with_production_resources(resources)?,
            )),
        };
        Ok(Self {
            target: Arc::new(ChromiumLocatorTarget {
                target_id,
                backend: Arc::new(backend),
            }),
            protocol,
            _route_resources: route_resources,
        })
    }

    pub fn target(&self) -> Arc<ChromiumLocatorTarget> {
        Arc::clone(&self.target)
    }

    pub fn protocol(&self) -> ChromiumLocatorProtocol {
        self.protocol.clone()
    }

    pub fn handlers(&self) -> ChromiumLocatorHandlers {
        match &self.protocol {
            ChromiumLocatorProtocol::V1(protocol) => SelectedLocatorServerHandlers::V1(
                v1_handlers(Arc::clone(protocol), Arc::clone(&self.target)),
            ),
            ChromiumLocatorProtocol::V2(protocol) => SelectedLocatorServerHandlers::V2(
                v2_handlers(Arc::clone(protocol), Arc::clone(&self.target)),
            ),
        }
    }
}

fn v1_handlers(
    protocol: Arc<WireProtocol<LocatorCodec>>,
    service: Arc<ChromiumLocatorTarget>,
) -> ChromiumLocatorV1Handlers {
    locator_server_handlers(
        protocol,
        LocatorServerServices {
            left: LocatorServerServicesLeft {
                all_inner_texts: Arc::clone(&service),
                all_text_contents: Arc::clone(&service),
                blur: Arc::clone(&service),
                click: Arc::clone(&service),
                count: Arc::clone(&service),
                fill: Arc::clone(&service),
                focus: Arc::clone(&service),
                get_attribute: Arc::clone(&service),
                hover: Arc::clone(&service),
                inner_html: Arc::clone(&service),
                inner_text: Arc::clone(&service),
            },
            right: LocatorServerServicesRight {
                input_value: Arc::clone(&service),
                is_checked: Arc::clone(&service),
                is_editable: Arc::clone(&service),
                is_enabled: Arc::clone(&service),
                is_visible: Arc::clone(&service),
                press: Arc::clone(&service),
                press_sequentially: Arc::clone(&service),
                scroll_into_view_if_needed: Arc::clone(&service),
                select_option: Arc::clone(&service),
                set_checked: Arc::clone(&service),
                text_content: Arc::clone(&service),
                wait_for: service,
            },
        },
    )
}

fn v2_handlers(
    protocol: Arc<WireProtocol<LocatorV2Codec>>,
    service: Arc<ChromiumLocatorTarget>,
) -> ChromiumLocatorV2Handlers {
    locator_v2_server_handlers(
        protocol,
        LocatorV2ServerServices {
            left: LocatorV2ServerServicesLeft {
                all_inner_texts: Arc::clone(&service),
                all_text_contents: Arc::clone(&service),
                blur: Arc::clone(&service),
                click: Arc::clone(&service),
                count: Arc::clone(&service),
                fill: Arc::clone(&service),
                focus: Arc::clone(&service),
                get_attribute: Arc::clone(&service),
                hover: Arc::clone(&service),
                inner_html: Arc::clone(&service),
                inner_text: Arc::clone(&service),
            },
            right: LocatorV2ServerServicesRight {
                input_value: Arc::clone(&service),
                is_checked: Arc::clone(&service),
                is_editable: Arc::clone(&service),
                is_enabled: Arc::clone(&service),
                is_visible: Arc::clone(&service),
                press: Arc::clone(&service),
                press_sequentially: Arc::clone(&service),
                scroll_into_view_if_needed: Arc::clone(&service),
                select_option: Arc::clone(&service),
                set_checked: Arc::clone(&service),
                text_content: Arc::clone(&service),
                wait_for: service,
            },
        },
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_cap_locator::{Count, CountParams, CssSelector, v2 as locator_v2};
    use pwright_core::{
        CallId, CallMetadata, Command, FailureKind, ObjectId, ProductionResourceLedger,
        ResourceClass, ResourceLimits, ResourceScope, Service,
    };
    use pwright_fake::FakeCdpClient;
    use pwright_transport::FrameLimits;
    use serde_json::json;

    use super::{
        CdpInputCoordinator, CdpLocatorProductionProfile, ChromiumLocatorInstallation,
        ChromiumLocatorProfile, ChromiumLocatorProtocol, LocatorCodecLimits,
    };

    fn profile(v2: bool, with_context_route: bool) -> ChromiumLocatorProfile {
        let mut backend =
            CdpLocatorProductionProfile::new(ResourceScope::Object(ObjectId::new(7)), 1);
        if with_context_route {
            backend = backend.with_context_routes(1, 64);
        }
        let frames = FrameLimits::new(16 * 1024 * 1024, 0, 0, 16 * 1024 * 1024);
        if v2 {
            ChromiumLocatorProfile::v2(
                backend,
                LocatorCodecLimits::new(1024 * 1024),
                frames,
                frames,
            )
        } else {
            ChromiumLocatorProfile::v1(
                backend,
                LocatorCodecLimits::new(1024 * 1024),
                frames,
                frames,
            )
        }
    }

    fn resources(profile: ChromiumLocatorProfile) -> ProductionResourceLedger {
        let requirements = profile.requirements();
        let selectors = requirements.class(ResourceClass::SelectorBytes);
        let backend = requirements.class(ResourceClass::BackendStateBytes);
        let wire = requirements.class(ResourceClass::WireBytes);
        let limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases)
            .with_class(
                ResourceClass::SelectorBytes,
                selectors.global,
                selectors.per_scope,
            )
            .with_class(
                ResourceClass::BackendStateBytes,
                backend.global,
                backend.per_scope,
            )
            .with_class(ResourceClass::WireBytes, wire.global, wire.per_scope);
        ProductionResourceLedger::new(limits, requirements)
            .expect("exact Locator resources should build")
    }

    fn selector(value: &str) -> CssSelector {
        CssSelector::new(value).expect("test selector should be valid")
    }

    #[tokio::test]
    async fn installed_v1_target_executes_count_and_builds_complete_handlers() {
        // Arrange
        let profile = profile(false, false);
        let resources = resources(profile);
        let fake = Arc::new(FakeCdpClient::from_html("<div></div><div></div>"));
        fake.set_evaluate_responses([
            json!({"result": {"value": true}}),
            json!({"result": {"value": {"status": "done", "count": 2}}}),
        ]);
        let client: Arc<dyn pwright_cdp::CdpClient> = fake.clone();
        let installation = ChromiumLocatorInstallation::install_main_target(
            ObjectId::new(7),
            client,
            CdpInputCoordinator::new(),
            profile,
            &resources,
        )
        .expect("Locator-only composition should install");
        let command = Command::<Count>::new(
            CallId::new(11),
            ObjectId::new(7),
            CallMetadata::default(),
            CountParams::new(selector("div")),
        );

        // Act
        let count = installation
            .target()
            .call(command)
            .await
            .expect("command service is infallible")
            .into_outcome()
            .expect("CDP Locator count should succeed");

        // Assert
        assert_eq!(count, 2);
        assert_eq!(
            fake.method_names(),
            ["Runtime.evaluate", "Runtime.evaluate"]
        );
        assert!(matches!(
            installation.protocol(),
            ChromiumLocatorProtocol::V1(_)
        ));
        let _handlers = installation.handlers();
    }

    #[tokio::test]
    async fn v2_unique_context_routes_commands_and_shares_page_input() {
        // Arrange
        let profile = profile(true, true);
        let resources = resources(profile);
        let input = CdpInputCoordinator::new();
        let fake = Arc::new(FakeCdpClient::from_html("<button>go</button>"));
        fake.set_evaluate_responses([
            json!({"result": {"value": true}}),
            json!({"result": {"value": {"status": "done", "count": 1}}}),
        ]);
        let client: Arc<dyn pwright_cdp::CdpClient> = fake.clone();

        // Act
        let installation = ChromiumLocatorInstallation::install_unique_context(
            ObjectId::new(8),
            client,
            "utility-world",
            input.clone(),
            profile,
            &resources,
        )
        .expect("v2 unique-context Locator should install");

        let command = Command::<locator_v2::Count>::new(
            CallId::new(12),
            ObjectId::new(8),
            CallMetadata::default(),
            CountParams::new(selector("button")),
        );
        let count = installation
            .target()
            .call(command)
            .await
            .expect("command service is infallible")
            .into_outcome()
            .expect("v2 count should reach the unique execution context");

        // Assert
        assert_eq!(count, 1);
        assert_eq!(fake.calls()[0].1[0]["uniqueContextId"], "utility-world");
        assert_eq!(fake.calls()[1].1[0]["uniqueContextId"], "utility-world");
        assert!(
            installation
                .target()
                .backend()
                .uses_input_coordinator(&input)
        );
        assert!(matches!(
            installation.protocol(),
            ChromiumLocatorProtocol::V2(_)
        ));
        assert_eq!(
            resources
                .metrics()
                .expect("route metrics should be readable")
                .class(ResourceClass::BackendStateBytes)
                .current,
            profile.backend().maximum_route_retained_bytes()
        );
        let _handlers = installation.handlers();
        drop(installation);
        assert_eq!(
            resources
                .metrics()
                .expect("released route metrics should be readable")
                .class(ResourceClass::BackendStateBytes)
                .current,
            0
        );
    }

    #[tokio::test]
    async fn v2_sequential_typing_uses_the_bound_world_and_releases_action_resources() {
        // Arrange
        let profile = profile(true, true);
        let resources = resources(profile);
        let input = CdpInputCoordinator::new();
        let fake = Arc::new(FakeCdpClient::from_html("<input>"));
        fake.set_evaluate_responses([
            json!({"result": {"value": true}}),
            json!({"result": {"value": {"status": "done", "isMac": false}}}),
        ]);
        let installation = ChromiumLocatorInstallation::install_unique_context(
            ObjectId::new(8),
            fake.clone(),
            "utility-world",
            input.clone(),
            profile,
            &resources,
        )
        .unwrap();
        let command = Command::<locator_v2::PressSequentially>::new(
            CallId::new(12),
            ObjectId::new(8),
            CallMetadata::default(),
            pwright_cap_locator::PressSequentiallyParams::new(selector("input"), "a中", None),
        );
        // Act
        let result = installation
            .target()
            .call(command)
            .await
            .unwrap()
            .into_outcome();
        // Assert
        assert_eq!(result, Ok(()));
        assert_eq!(fake.calls()[1].1[0]["uniqueContextId"], "utility-world");
        assert_eq!(
            fake.method_names().last().map(String::as_str),
            Some("Input.insertText")
        );
        assert!(
            installation
                .target()
                .backend()
                .uses_input_coordinator(&input)
        );
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::SelectorBytes)
                .current,
            0
        );
        drop(installation);
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::BackendStateBytes)
                .current,
            0
        );
    }

    #[tokio::test]
    async fn unbound_target_is_rejected_before_cdp() {
        // Arrange
        let profile = profile(false, false);
        let resources = resources(profile);
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let client: Arc<dyn pwright_cdp::CdpClient> = fake.clone();
        let installation = ChromiumLocatorInstallation::install_main_target(
            ObjectId::new(7),
            client,
            CdpInputCoordinator::new(),
            profile,
            &resources,
        )
        .expect("Locator-only composition should install");
        let command = Command::<Count>::new(
            CallId::new(13),
            ObjectId::new(9),
            CallMetadata::default(),
            CountParams::new(selector("div")),
        );

        // Act
        let error = installation
            .target()
            .call(command)
            .await
            .expect("command service is infallible")
            .into_outcome()
            .expect_err("an unbound Page or Frame must be rejected");

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert!(fake.calls().is_empty());
    }
}
