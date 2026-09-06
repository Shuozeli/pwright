use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

use pwright_backend_cdp::CdpPageClosePoll;
use pwright_cap_content::CONTENT_CAPABILITY_ID;
use pwright_cap_context_creation::CONTEXT_CREATION_CAPABILITY_ID;
use pwright_cap_context_lifecycle::CONTEXT_LIFECYCLE_CAPABILITY_ID;
use pwright_cap_dom_content::DOM_CONTENT_CAPABILITY_ID;
use pwright_cap_evaluation::EVALUATION_CAPABILITY_ID;
use pwright_cap_handle_evaluation::HANDLE_EVALUATION_CAPABILITY_ID;
use pwright_cap_handle_lifecycle::HANDLE_LIFECYCLE_CAPABILITY_ID;
use pwright_cap_locator::LOCATOR_CAPABILITY_ID;
use pwright_cap_locator_screenshot::LOCATOR_SCREENSHOT_CAPABILITY_ID;
use pwright_cap_navigation::NAVIGATION_CAPABILITY_ID;
use pwright_cap_network_events::NETWORK_EVENTS_CAPABILITY_ID;
use pwright_cap_page_creation::PAGE_CREATION_CAPABILITY_ID;
use pwright_cap_page_events::PAGE_EVENTS_CAPABILITY_ID;
use pwright_cap_page_lifecycle::PAGE_LIFECYCLE_CAPABILITY_ID;
use pwright_core::{EventTag, ProductionResourceError};
use pwright_runtime::{CleanupQueue, ControlledDispatcher, NoDeadlineScheduler};
use pwright_server_content::ContentServerHandlers;
use pwright_server_dom_content::DomContentServerHandlers;
use pwright_server_evaluation::EvaluationServerHandlers;
use pwright_server_events::ServerEventProductionError;
use pwright_server_locator_screenshot::LocatorScreenshotServerHandlers;
use pwright_server_navigation::NavigationServerHandlers;
use pwright_server_wire::{
    ChainedServerOutbound, ConcurrentServerBackendChain, ConcurrentServerHandlerChain,
};
use pwright_transport::ProductionFrameTransport;

use crate::{
    AttachedBrowserContextCloseService, AttachedBrowserContextNewPageService,
    AttachedBrowserNewContextService, AttachedChromiumComposition, AttachedPageCloseService,
    CdpPageServices, ChromiumActivatedPageControlReceiver, ChromiumApplicationComponents,
    ChromiumApplicationConnection, ChromiumApplicationInstallContext, ChromiumApplicationInstaller,
    ChromiumBrowserContextPageOutbound, ChromiumCompositionError, ChromiumGracefulControlHandler,
    ChromiumHandleAwareRuntime, ChromiumHandleLeafDisposeCommandService,
    ChromiumHandleLeafEvaluationCommandService, ChromiumHandleLeafLifecycleRoutes,
    ChromiumNetworkEventDeliveryInstallError, ChromiumNetworkEventDeliveryInstallation,
    ChromiumNetworkEventOutbound, ChromiumNetworkLifecycleBackend,
    ChromiumNetworkLifecyclePublisher, ChromiumPageEventsInstallError,
    ChromiumPageEventsInstallation, ChromiumPageLoadOutbound, ChromiumPageLoadServerBackend,
    ChromiumPagePopupOutbound, ChromiumPageRunBeforeUnloadCommandService,
    ChromiumRemoteContextCreationHandlers, ChromiumRemoteContextLifecycleHandlers,
    ChromiumRemoteControlOutbound, ChromiumRemoteHandleEvaluationHandlers,
    ChromiumRemoteHandleLifecycleHandlers, ChromiumRemotePageCreationHandlers,
    ChromiumRemotePageLifecycleHandlers, ChromiumRuntimeInstaller,
    ChromiumSelectedLocatorServerHandlers, ChromiumTopologyConnectionPump,
    ChromiumTopologyConnectionPumpConfigurationError, ChromiumTopologyEventLossPolicy,
    ChromiumTopologyServerBackend, NegotiatedControlCodec,
};

pub type ChromiumAllCapabilitiesDispatcher =
    ControlledDispatcher<CdpPageServices, CdpPageServices, NoDeadlineScheduler, CleanupQueue>;

/// One shared target-aware runtime used by every currently implemented v2
/// Chromium command and lifecycle capability.
pub type ChromiumAllCapabilitiesInnerRuntime = Arc<ChromiumAllCapabilitiesDispatcher>;
pub type ChromiumAllCapabilitiesRuntimeTarget = ChromiumHandleAwareRuntime<
    ChromiumAllCapabilitiesInnerRuntime,
    ChromiumHandleLeafLifecycleRoutes,
>;

/// The complete recipe makes transport-close Handle retirement structural:
/// callers cannot accidentally install Handle capabilities over a bare runtime.
pub type ChromiumAllCapabilitiesRuntime = Arc<ChromiumAllCapabilitiesRuntimeTarget>;

type AllContextCreationService = AttachedBrowserNewContextService<CleanupQueue>;
type AllContextLifecycleService = AttachedBrowserContextCloseService<CleanupQueue>;
type AllPageCreationService = AttachedBrowserContextNewPageService<CleanupQueue>;
type AllPageCloseService = AttachedPageCloseService<CleanupQueue>;
type AllHandleEvaluationService = ChromiumHandleLeafEvaluationCommandService;
type AllHandleDisposeService = ChromiumHandleLeafDisposeCommandService;
type AllHandleOperationService = ChromiumHandleLeafEvaluationCommandService;

type AllContextCreationCommands<Transport> = ChromiumRemoteContextCreationHandlers<
    AllContextCreationService,
    ChromiumAllCapabilitiesRuntime,
    Transport,
>;
type AllContextLifecycleCommands<Transport> = ChromiumRemoteContextLifecycleHandlers<
    AllContextLifecycleService,
    ChromiumAllCapabilitiesRuntime,
    Transport,
>;
type AllPageCreationCommands<Transport> = ChromiumRemotePageCreationHandlers<
    AllPageCreationService,
    ChromiumAllCapabilitiesRuntime,
    Transport,
>;
type AllPageLifecycleCommands<Transport> = ChromiumRemotePageLifecycleHandlers<
    AllPageCloseService,
    ChromiumPageRunBeforeUnloadCommandService,
    ChromiumAllCapabilitiesRuntime,
    Transport,
>;
type AllEvaluationCommands = EvaluationServerHandlers<ChromiumAllCapabilitiesDispatcher>;
type AllHandleEvaluationCommands<Transport> = ChromiumRemoteHandleEvaluationHandlers<
    AllHandleEvaluationService,
    AllHandleDisposeService,
    AllHandleOperationService,
    ChromiumAllCapabilitiesRuntime,
    Transport,
    CleanupQueue,
>;
type AllHandleLifecycleCommands<Transport> = ChromiumRemoteHandleLifecycleHandlers<
    AllHandleDisposeService,
    ChromiumAllCapabilitiesRuntime,
    Transport,
>;
type AllContentCommands = ContentServerHandlers<ChromiumAllCapabilitiesDispatcher>;
type AllDomContentCommands = DomContentServerHandlers<ChromiumAllCapabilitiesDispatcher>;
type AllNavigationCommands =
    NavigationServerHandlers<ChromiumAllCapabilitiesDispatcher, ChromiumAllCapabilitiesDispatcher>;
type AllLocatorCommands = ChromiumSelectedLocatorServerHandlers<ChromiumAllCapabilitiesDispatcher>;
type AllLocatorScreenshotCommands =
    LocatorScreenshotServerHandlers<ChromiumAllCapabilitiesDispatcher>;

/// Static command chain containing every capability currently implemented by
/// the v2 Chromium composition root.
pub type ChromiumAllCapabilitiesCommands<Transport> = ConcurrentServerHandlerChain<
    AllContextCreationCommands<Transport>,
    ConcurrentServerHandlerChain<
        AllContextLifecycleCommands<Transport>,
        ConcurrentServerHandlerChain<
            AllPageCreationCommands<Transport>,
            ConcurrentServerHandlerChain<
                AllPageLifecycleCommands<Transport>,
                ConcurrentServerHandlerChain<
                    AllEvaluationCommands,
                    ConcurrentServerHandlerChain<
                        AllHandleEvaluationCommands<Transport>,
                        ConcurrentServerHandlerChain<
                            AllHandleLifecycleCommands<Transport>,
                            ConcurrentServerHandlerChain<
                                AllNavigationCommands,
                                ConcurrentServerHandlerChain<
                                    AllLocatorCommands,
                                    ConcurrentServerHandlerChain<
                                        AllLocatorScreenshotCommands,
                                        ConcurrentServerHandlerChain<
                                            AllContentCommands,
                                            AllDomContentCommands,
                                        >,
                                    >,
                                >,
                            >,
                        >,
                    >,
                >,
            >,
        >,
    >,
>;

pub type ChromiumAllCapabilitiesControls<Transport> = ChromiumGracefulControlHandler<
    Transport,
    ChromiumActivatedPageControlReceiver<ChromiumAllCapabilitiesRuntime>,
>;

type AllControlOutbound<Transport> =
    ChromiumRemoteControlOutbound<ChromiumAllCapabilitiesRuntime, Transport>;
type AllPageLoadOutbound<Transport> = ChromiumPageLoadOutbound<Transport>;
type AllContextPageOutbound<Transport> = ChromiumBrowserContextPageOutbound<Transport>;
type AllPopupOutbound<Transport> = ChromiumPagePopupOutbound<Transport>;
type AllNetworkOutbound<Transport> = ChromiumNetworkEventOutbound<Transport>;

/// Ordered lifecycle output plus the three independently bounded Page-event
/// queues. The chain serializes all writes onto the one remote transport.
pub type ChromiumAllCapabilitiesOutbound<Transport> = ChainedServerOutbound<
    AllControlOutbound<Transport>,
    ChainedServerOutbound<
        AllPageLoadOutbound<Transport>,
        ChainedServerOutbound<
            AllContextPageOutbound<Transport>,
            ChainedServerOutbound<AllPopupOutbound<Transport>, AllNetworkOutbound<Transport>>,
        >,
    >,
>;

type AllTopologyBackend<Transport> = ChromiumTopologyServerBackend<
    ChromiumAllCapabilitiesDispatcher,
    ChromiumAllCapabilitiesRuntime,
    NegotiatedControlCodec,
    Arc<Transport>,
>;

type AllNetworkBackend<Transport> = ChromiumNetworkLifecycleBackend<
    crate::ChromiumNetworkEventDriver,
    ChromiumAllCapabilitiesDispatcher,
    ChromiumAllCapabilitiesRuntime,
    NegotiatedControlCodec,
    Arc<Transport>,
>;

/// Automatic topology discovery and PageLoad normalization retain independent
/// selected event cursors and are fairly composed in the supervisor backend lane.
pub type ChromiumAllCapabilitiesBackend<Transport> = ConcurrentServerBackendChain<
    ConcurrentServerBackendChain<AllTopologyBackend<Transport>, AllNetworkBackend<Transport>>,
    ChromiumPageLoadServerBackend,
>;

pub type ChromiumAllCapabilitiesApplication<Transport, Authorization> =
    ChromiumApplicationConnection<
        Transport,
        ChromiumAllCapabilitiesRuntime,
        Authorization,
        ChromiumAllCapabilitiesCommands<Transport>,
        ChromiumAllCapabilitiesControls<Transport>,
        ChromiumAllCapabilitiesOutbound<Transport>,
        ChromiumAllCapabilitiesBackend<Transport>,
        crate::ChromiumNetworkSubscriptionRegistry,
    >;

/// Installs the shared controlled Page runtime without choosing an executor or
/// clock. The bounded cleanup queue is exposed through the returned runtime
/// and remains caller-driven.
#[derive(Clone, Copy, Debug)]
pub struct ChromiumAllCapabilitiesRuntimeInstaller {
    cleanup_capacity: usize,
}

impl ChromiumAllCapabilitiesRuntimeInstaller {
    pub const fn new(cleanup_capacity: usize) -> Self {
        Self { cleanup_capacity }
    }

    pub const fn cleanup_capacity(self) -> usize {
        self.cleanup_capacity
    }
}

impl ChromiumRuntimeInstaller for ChromiumAllCapabilitiesRuntimeInstaller {
    type Runtime = ChromiumAllCapabilitiesRuntime;
    type Error = ChromiumCompositionError;

    fn install(
        self,
        composition: &mut AttachedChromiumComposition,
    ) -> Result<Self::Runtime, Self::Error> {
        let runtime = Arc::new(composition.new_controlled_page_runtime_with_handle_leaf(
            NoDeadlineScheduler,
            CleanupQueue::new(self.cleanup_capacity),
        )?);
        let routes = runtime.service().handle_leaf_routes().ok_or_else(|| {
            ChromiumCompositionError::InvalidHandleSetup(
                "the complete capability recipe requires Handle lifecycle-leaf ownership"
                    .to_owned(),
            )
        })?;
        Ok(Arc::new(ChromiumHandleAwareRuntime::new(runtime, routes)))
    }
}

#[derive(Debug)]
pub enum ChromiumAllCapabilitiesApplicationInstallError {
    CapabilityNotNegotiated(&'static str),
    ComponentNotSelected(&'static str),
    RemoteEventNotSelected(EventTag),
    Composition(ChromiumCompositionError),
    Resources(ProductionResourceError),
    Events(ServerEventProductionError),
    PageEvents(ChromiumPageEventsInstallError),
    NetworkEventDelivery(ChromiumNetworkEventDeliveryInstallError),
    NetworkDriver(crate::ChromiumNetworkEventDriverError),
    EventSubscription(pwright_cdp::CdpEventSubscriptionError),
    TopologyPump(ChromiumTopologyConnectionPumpConfigurationError),
}

impl Display for ChromiumAllCapabilitiesApplicationInstallError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapabilityNotNegotiated(capability) => write!(
                formatter,
                "the remote peer did not negotiate required all-capability component `{capability}`"
            ),
            Self::ComponentNotSelected(component) => {
                write!(
                    formatter,
                    "all-capability recipe component `{component}` was not selected"
                )
            }
            Self::RemoteEventNotSelected(event) => write!(
                formatter,
                "all-capability recipe remote event `{event}` was not selected"
            ),
            Self::Composition(error) => Display::fmt(error, formatter),
            Self::Resources(error) => Display::fmt(error, formatter),
            Self::Events(error) => Display::fmt(error, formatter),
            Self::PageEvents(error) => Display::fmt(error, formatter),
            Self::NetworkEventDelivery(error) => Display::fmt(error, formatter),
            Self::NetworkDriver(error) => Display::fmt(error, formatter),
            Self::EventSubscription(error) => Display::fmt(error, formatter),
            Self::TopologyPump(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ChromiumAllCapabilitiesApplicationInstallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Composition(error) => Some(error),
            Self::Resources(error) => Some(error),
            Self::Events(error) => Some(error),
            Self::PageEvents(error) => Some(error),
            Self::NetworkEventDelivery(error) => Some(error),
            Self::NetworkDriver(error) => Some(error),
            Self::EventSubscription(error) => Some(error),
            Self::TopologyPump(error) => Some(error),
            Self::CapabilityNotNegotiated(_)
            | Self::ComponentNotSelected(_)
            | Self::RemoteEventNotSelected(_) => None,
        }
    }
}

/// One-shot installer for every browser capability currently implemented by
/// the v2 Chromium composition.
///
/// Page-close polling remains an injected deployment policy. All other
/// components are selected by the attachment profile and must be negotiated;
/// the installer never silently degrades to a smaller recipe.
#[derive(Clone)]
pub struct ChromiumAllCapabilitiesApplicationInstaller {
    page_close_poll: Arc<dyn CdpPageClosePoll>,
}

impl ChromiumAllCapabilitiesApplicationInstaller {
    pub fn new(page_close_poll: Arc<dyn CdpPageClosePoll>) -> Self {
        Self { page_close_poll }
    }
}

impl<Transport, Authorization>
    ChromiumApplicationInstaller<Transport, ChromiumAllCapabilitiesRuntime, Authorization>
    for ChromiumAllCapabilitiesApplicationInstaller
where
    Transport: ProductionFrameTransport + Send + Sync + 'static,
    Transport::Error: Send,
{
    type Commands = ChromiumAllCapabilitiesCommands<Transport>;
    type Controls = ChromiumAllCapabilitiesControls<Transport>;
    type Outbound = ChromiumAllCapabilitiesOutbound<Transport>;
    type Backend = ChromiumAllCapabilitiesBackend<Transport>;
    type State = crate::ChromiumNetworkSubscriptionRegistry;
    type Error = ChromiumAllCapabilitiesApplicationInstallError;

    fn install(
        self,
        context: &mut ChromiumApplicationInstallContext<
            Transport,
            ChromiumAllCapabilitiesRuntime,
            Authorization,
        >,
    ) -> Result<
        ChromiumApplicationComponents<
            Self::Commands,
            Self::Controls,
            Self::Outbound,
            Self::Backend,
            Self::State,
        >,
        Self::Error,
    > {
        ensure_all_capabilities_negotiated(context)?;

        let runtime = Arc::clone(context.runtime());
        let dispatcher = Arc::clone(runtime.inner());
        let transport = Arc::clone(context.transport());
        let continuation = dispatcher.cleanup_executor().clone();
        let network_driver = context
            .take_network_events(dispatcher.service())
            .map_err(Self::Error::NetworkDriver)?
            .ok_or(Self::Error::ComponentNotSelected("network event driver"))?;
        let network_registration = network_driver.registration_handle();

        let (
            commands,
            controls,
            control_outbound,
            load_outbound,
            page_event_publisher,
            context_outbound,
            popup_outbound,
            network_sender,
            network_outbound,
            load_backend,
        ) = {
            let composition = context.composition();
            let accepted = context.accepted();
            if !composition.page_discovery_selected() {
                return Err(Self::Error::ComponentNotSelected("page discovery"));
            }
            if !composition.utility_world_selected() {
                return Err(Self::Error::ComponentNotSelected("Chromium utility world"));
            }

            let control_outbound = composition
                .remote_control_outbound(Arc::clone(&runtime), Arc::clone(&transport), accepted)
                .map_err(Self::Error::Composition)?
                .ok_or(Self::Error::ComponentNotSelected("remote control"))?;
            let activated_control = composition
                .remote_activated_page_control_receiver(
                    Arc::clone(&runtime),
                    dispatcher.service().clone(),
                    accepted,
                )
                .map_err(Self::Error::Composition)?
                .ok_or(Self::Error::ComponentNotSelected(
                    "Page event subscription control",
                ))?;
            let controls = composition
                .remote_graceful_control_handler(
                    Arc::clone(&transport),
                    activated_control,
                    accepted,
                )
                .map_err(Self::Error::Composition)?
                .ok_or(Self::Error::ComponentNotSelected("graceful remote control"))?;

            let context_creation_service = composition
                .browser_new_context_service(dispatcher.lifecycle_handle(), continuation.clone())
                .map_err(Self::Error::Composition)?
                .ok_or(Self::Error::ComponentNotSelected("Browser.newContext"))?;
            let page_creation_service = composition
                .browser_context_new_page_service_with_network(
                    dispatcher.as_ref(),
                    network_registration.clone(),
                    continuation.clone(),
                )
                .map_err(Self::Error::Composition)?
                .ok_or(Self::Error::ComponentNotSelected("BrowserContext.newPage"))?;
            let context_lifecycle_service = composition
                .browser_context_close_service_for_page_runtime_with_network(
                    dispatcher.as_ref(),
                    network_registration.clone(),
                    continuation.clone(),
                )
                .ok_or(Self::Error::ComponentNotSelected("BrowserContext.close"))?;
            let (page_close_service, page_before_unload_service) = composition
                .page_lifecycle_services_with_network(
                    dispatcher.as_ref(),
                    network_registration,
                    continuation.clone(),
                    Arc::clone(&self.page_close_poll),
                )
                .ok_or(Self::Error::ComponentNotSelected("Page lifecycle"))?;

            let context_creation = composition
                .remote_context_creation_handlers(
                    Arc::new(context_creation_service),
                    &control_outbound,
                    accepted,
                )
                .map_err(Self::Error::Resources)?
                .ok_or(Self::Error::ComponentNotSelected(
                    "context creation handlers",
                ))?;
            let context_lifecycle = composition
                .remote_context_lifecycle_handlers(
                    Arc::new(context_lifecycle_service),
                    &control_outbound,
                    accepted,
                )
                .map_err(Self::Error::Resources)?
                .ok_or(Self::Error::ComponentNotSelected(
                    "context lifecycle handlers",
                ))?;
            let page_creation = composition
                .remote_page_creation_handlers(
                    Arc::new(page_creation_service),
                    &control_outbound,
                    accepted,
                )
                .map_err(Self::Error::Resources)?
                .ok_or(Self::Error::ComponentNotSelected("page creation handlers"))?;
            let page_lifecycle = composition
                .remote_page_lifecycle_handlers(
                    Arc::new(page_close_service),
                    Arc::new(page_before_unload_service),
                    &control_outbound,
                    accepted,
                )
                .map_err(Self::Error::Resources)?
                .ok_or(Self::Error::ComponentNotSelected("page lifecycle handlers"))?;
            let evaluation = composition
                .remote_evaluation_handlers(Arc::clone(&dispatcher), accepted)
                .map_err(Self::Error::Resources)?
                .ok_or(Self::Error::ComponentNotSelected("evaluation handlers"))?;
            let (handle_evaluation_service, handle_dispose_service) = composition
                .handle_leaf_services(dispatcher.as_ref(), continuation.clone())
                .map_err(Self::Error::Composition)?
                .ok_or(Self::Error::ComponentNotSelected("Handle services"))?;
            let handle_evaluation = composition
                .remote_handle_evaluation_handlers(
                    Arc::new(handle_evaluation_service.clone()),
                    Arc::new(handle_dispose_service.clone()),
                    Arc::new(handle_evaluation_service),
                    continuation,
                    &control_outbound,
                    accepted,
                )
                .map_err(Self::Error::Resources)?
                .ok_or(Self::Error::ComponentNotSelected(
                    "Handle evaluation handlers",
                ))?;
            let handle_lifecycle = composition
                .remote_handle_lifecycle_handlers(
                    Arc::new(handle_dispose_service),
                    &control_outbound,
                    accepted,
                )
                .map_err(Self::Error::Resources)?
                .ok_or(Self::Error::ComponentNotSelected(
                    "Handle lifecycle handlers",
                ))?;
            let navigation = composition
                .remote_navigation_handlers(Arc::clone(&dispatcher), accepted)
                .map_err(Self::Error::Resources)?
                .ok_or(Self::Error::ComponentNotSelected("navigation handlers"))?;
            let locator = composition
                .remote_selected_locator_handlers(Arc::clone(&dispatcher), accepted)
                .map_err(Self::Error::Composition)?
                .ok_or(Self::Error::ComponentNotSelected("locator handlers"))?;
            let locator_screenshot = composition
                .remote_locator_screenshot_handlers(Arc::clone(&dispatcher), accepted)
                .map_err(Self::Error::Resources)?
                .ok_or(Self::Error::ComponentNotSelected(
                    "Locator screenshot handlers",
                ))?;
            let content = composition
                .remote_content_handlers(Arc::clone(&dispatcher), accepted)
                .map_err(Self::Error::Resources)?
                .ok_or(Self::Error::ComponentNotSelected("content handlers"))?;
            let dom_content = composition
                .remote_dom_content_handlers(Arc::clone(&dispatcher), accepted)
                .map_err(Self::Error::Resources)?
                .ok_or(Self::Error::ComponentNotSelected("DOM-content handlers"))?;
            let commands = ConcurrentServerHandlerChain::new(
                context_creation,
                ConcurrentServerHandlerChain::new(
                    context_lifecycle,
                    ConcurrentServerHandlerChain::new(
                        page_creation,
                        ConcurrentServerHandlerChain::new(
                            page_lifecycle,
                            ConcurrentServerHandlerChain::new(
                                evaluation,
                                ConcurrentServerHandlerChain::new(
                                    handle_evaluation,
                                    ConcurrentServerHandlerChain::new(
                                        handle_lifecycle,
                                        ConcurrentServerHandlerChain::new(
                                            navigation,
                                            ConcurrentServerHandlerChain::new(
                                                locator,
                                                ConcurrentServerHandlerChain::new(
                                                    locator_screenshot,
                                                    ConcurrentServerHandlerChain::new(
                                                        content,
                                                        dom_content,
                                                    ),
                                                ),
                                            ),
                                        ),
                                    ),
                                ),
                            ),
                        ),
                    ),
                ),
            );

            let page_events_profile =
                composition
                    .page_events_leaf_profile()
                    .ok_or(Self::Error::ComponentNotSelected(
                        "complete Page-events leaf profile",
                    ))?;
            let page_event_adapter = composition
                .page_event_adapter()
                .ok_or(Self::Error::ComponentNotSelected("Page-event adapter"))?;
            let page_event_receiver = composition
                .connection()
                .subscribe_composable_events()
                .map_err(Self::Error::EventSubscription)?;
            let page_events = ChromiumPageEventsInstallation::install(
                page_event_receiver,
                page_event_adapter,
                composition.shared_topology(),
                Arc::clone(&transport),
                page_events_profile,
                composition.resources(),
            )
            .map_err(Self::Error::PageEvents)?;
            let (
                page_event_publisher,
                load_backend,
                load_outbound,
                context_outbound,
                popup_outbound,
            ) = page_events.into_parts();
            let network_delivery_profile = composition
                .network_event_delivery_leaf_profile()
                .ok_or(Self::Error::ComponentNotSelected(
                    "ordered network-event delivery leaf profile",
                ))?;
            let network_delivery = ChromiumNetworkEventDeliveryInstallation::install(
                Arc::clone(&transport),
                network_delivery_profile,
                composition.resources(),
            )
            .map_err(Self::Error::NetworkEventDelivery)?;
            let (network_sender, network_outbound) = network_delivery.into_parts();
            (
                commands,
                controls,
                control_outbound,
                load_outbound,
                page_event_publisher,
                context_outbound,
                popup_outbound,
                network_sender,
                network_outbound,
                load_backend,
            )
        };

        let driver = context
            .take_topology_events()
            .ok_or(Self::Error::ComponentNotSelected("topology event driver"))?;
        let pump = ChromiumTopologyConnectionPump::new(
            driver,
            page_event_publisher,
            ChromiumTopologyEventLossPolicy::TerminateConnection,
        )
        .map_err(Self::Error::TopologyPump)?;
        let topology_backend = ChromiumTopologyServerBackend::new(
            pump,
            Arc::clone(&dispatcher),
            control_outbound.clone(),
        );
        let network_registry = dispatcher.service().network_subscription_registry().ok_or(
            Self::Error::ComponentNotSelected("network subscription registry"),
        )?;
        let network_publisher = ChromiumNetworkLifecyclePublisher::new(
            network_sender,
            Arc::new(network_registry.clone()),
        );
        let network_backend = ChromiumNetworkLifecycleBackend::new(
            network_driver,
            network_publisher,
            Arc::clone(&dispatcher),
            control_outbound.clone(),
        );
        let backend = ConcurrentServerBackendChain::new(
            ConcurrentServerBackendChain::new(topology_backend, network_backend),
            load_backend,
        );
        let outbound = ChainedServerOutbound::new(
            control_outbound,
            ChainedServerOutbound::new(
                load_outbound,
                ChainedServerOutbound::new(
                    context_outbound,
                    ChainedServerOutbound::new(popup_outbound, network_outbound),
                ),
            ),
        );

        Ok(ChromiumApplicationComponents::new(
            commands,
            controls,
            outbound,
            backend,
            network_registry,
        ))
    }
}

fn ensure_all_capabilities_negotiated<Transport, Authorization>(
    context: &ChromiumApplicationInstallContext<
        Transport,
        ChromiumAllCapabilitiesRuntime,
        Authorization,
    >,
) -> Result<(), ChromiumAllCapabilitiesApplicationInstallError> {
    let enabled = context.accepted().hello().enabled_capabilities();
    for capability in [
        CONTEXT_CREATION_CAPABILITY_ID.as_str(),
        CONTEXT_LIFECYCLE_CAPABILITY_ID.as_str(),
        PAGE_CREATION_CAPABILITY_ID.as_str(),
        PAGE_LIFECYCLE_CAPABILITY_ID.as_str(),
        EVALUATION_CAPABILITY_ID.as_str(),
        HANDLE_LIFECYCLE_CAPABILITY_ID.as_str(),
        HANDLE_EVALUATION_CAPABILITY_ID.as_str(),
        NAVIGATION_CAPABILITY_ID.as_str(),
        LOCATOR_CAPABILITY_ID.as_str(),
        LOCATOR_SCREENSHOT_CAPABILITY_ID.as_str(),
        CONTENT_CAPABILITY_ID.as_str(),
        DOM_CONTENT_CAPABILITY_ID.as_str(),
        PAGE_EVENTS_CAPABILITY_ID.as_str(),
        NETWORK_EVENTS_CAPABILITY_ID.as_str(),
    ] {
        if !enabled
            .iter()
            .any(|selected| selected.id().as_str() == capability)
        {
            return Err(
                ChromiumAllCapabilitiesApplicationInstallError::CapabilityNotNegotiated(capability),
            );
        }
    }
    Ok(())
}
