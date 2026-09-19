use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

use pwright_cap_navigation::NAVIGATION_CAPABILITY_ID;
use pwright_core::ProductionResourceError;
use pwright_runtime::{CleanupQueue, ControlledDispatcher, DirectDispatcher, NoDeadlineScheduler};
use pwright_runtime_control::RuntimeControlReceiver;
use pwright_server_navigation::NavigationServerHandlers;
use pwright_server_wire::{NoServerBackend, NoServerControl, NoServerOutbound};
use pwright_transport::ProductionFrameTransport;

use crate::{
    AttachedChromiumComposition, CdpPageServices, ChromiumApplicationComponents,
    ChromiumApplicationConnection, ChromiumApplicationInstallContext, ChromiumApplicationInstaller,
    ChromiumCompositionError, ChromiumGracefulControlHandler, ChromiumRemoteControlOutbound,
    ChromiumRuntimeInstaller, ChromiumTopologyConnectionPump,
    ChromiumTopologyConnectionPumpConfigurationError, ChromiumTopologyEventLossPolicy,
    ChromiumTopologyPageEventPublisher, ChromiumTopologyServerBackend, NegotiatedControlCodec,
};

/// Target-aware runtime installed by the navigation-only recipe.
///
/// The shared ownership is explicit because all generated navigation handlers
/// use the same dispatcher and its single route table.
pub type ChromiumNavigationRuntime = Arc<DirectDispatcher<CdpPageServices>>;

/// Only the three commands owned by the navigation capability.
pub type ChromiumNavigationCommands = NavigationServerHandlers<
    DirectDispatcher<CdpPageServices>,
    DirectDispatcher<CdpPageServices>,
    DirectDispatcher<CdpPageServices>,
>;

/// A fully assembled, navigation-only remote Chromium connection.
pub type ChromiumNavigationApplication<Transport, Authorization> = ChromiumApplicationConnection<
    Transport,
    ChromiumNavigationRuntime,
    Authorization,
    ChromiumNavigationCommands,
    NoServerControl,
    NoServerOutbound,
    NoServerBackend,
    (),
>;

/// Cancellation- and transport-close-aware runtime required by remote
/// lifecycle control and continuous topology discovery.
pub type ChromiumDiscoveringNavigationRuntime =
    Arc<ControlledDispatcher<CdpPageServices, CdpPageServices, NoDeadlineScheduler, CleanupQueue>>;

pub type ChromiumDiscoveringNavigationCommands = NavigationServerHandlers<
    ControlledDispatcher<CdpPageServices, CdpPageServices, NoDeadlineScheduler, CleanupQueue>,
    ControlledDispatcher<CdpPageServices, CdpPageServices, NoDeadlineScheduler, CleanupQueue>,
    ControlledDispatcher<CdpPageServices, CdpPageServices, NoDeadlineScheduler, CleanupQueue>,
>;

/// Negotiated lifecycle receiver required when topology discovery creates new
/// portable Page identities.
pub type ChromiumDiscoveringNavigationControls<Transport> = ChromiumGracefulControlHandler<
    Transport,
    RuntimeControlReceiver<ChromiumDiscoveringNavigationRuntime, NegotiatedControlCodec>,
>;

/// Ordered lifecycle writer shared by topology discovery and the supervisor's
/// outbound lane.
pub type ChromiumDiscoveringNavigationOutbound<Transport> =
    ChromiumRemoteControlOutbound<ChromiumDiscoveringNavigationRuntime, Transport>;

/// Owned topology backend used by the automatic-discovery recipe.
pub type ChromiumDiscoveringNavigationBackend<Transport> = ChromiumTopologyServerBackend<
    ControlledDispatcher<CdpPageServices, CdpPageServices, NoDeadlineScheduler, CleanupQueue>,
    ChromiumDiscoveringNavigationRuntime,
    NegotiatedControlCodec,
    Arc<Transport>,
>;

/// Navigation plus automatic Page discovery, with lifecycle control but no
/// portable Page/popup event subscriptions.
pub type ChromiumDiscoveringNavigationApplication<Transport, Authorization> =
    ChromiumApplicationConnection<
        Transport,
        ChromiumDiscoveringNavigationRuntime,
        Authorization,
        ChromiumDiscoveringNavigationCommands,
        ChromiumDiscoveringNavigationControls<Transport>,
        ChromiumDiscoveringNavigationOutbound<Transport>,
        ChromiumDiscoveringNavigationBackend<Transport>,
        (),
    >;

/// Installs the concrete CDP Page runtime required by navigation and nothing
/// else. Construction starts no task, timer, process, or browser.
#[derive(Clone, Copy, Debug, Default)]
pub struct ChromiumNavigationRuntimeInstaller;

impl ChromiumRuntimeInstaller for ChromiumNavigationRuntimeInstaller {
    type Runtime = ChromiumNavigationRuntime;
    type Error = ChromiumCompositionError;

    fn install(
        self,
        composition: &mut AttachedChromiumComposition,
    ) -> Result<Self::Runtime, Self::Error> {
        composition.new_page_runtime().map(Arc::new)
    }
}

/// Installs the control-aware Page runtime used by automatic discovery.
/// Cleanup remains caller-driven through a bounded queue; this installer does
/// not start a task or executor.
#[derive(Clone, Copy, Debug)]
pub struct ChromiumDiscoveringNavigationRuntimeInstaller {
    cleanup_capacity: usize,
}

impl ChromiumDiscoveringNavigationRuntimeInstaller {
    pub const fn new(cleanup_capacity: usize) -> Self {
        Self { cleanup_capacity }
    }

    pub const fn cleanup_capacity(self) -> usize {
        self.cleanup_capacity
    }
}

impl ChromiumRuntimeInstaller for ChromiumDiscoveringNavigationRuntimeInstaller {
    type Runtime = ChromiumDiscoveringNavigationRuntime;
    type Error = ChromiumCompositionError;

    fn install(
        self,
        composition: &mut AttachedChromiumComposition,
    ) -> Result<Self::Runtime, Self::Error> {
        composition
            .new_controlled_page_runtime(
                NoDeadlineScheduler,
                CleanupQueue::new(self.cleanup_capacity),
            )
            .map(Arc::new)
    }
}

#[derive(Debug)]
pub enum ChromiumNavigationApplicationInstallError {
    NavigationNotSelected,
    NavigationNotNegotiated,
    TopologyEventsRequireBackend,
    Resources(ProductionResourceError),
}

impl Display for ChromiumNavigationApplicationInstallError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NavigationNotSelected => {
                formatter.write_str("the navigation capability was not selected")
            }
            Self::NavigationNotNegotiated => formatter
                .write_str("the remote peer did not negotiate the selected navigation capability"),
            Self::TopologyEventsRequireBackend => formatter.write_str(
                "the navigation-only recipe cannot omit a selected topology-event backend",
            ),
            Self::Resources(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ChromiumNavigationApplicationInstallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resources(error) => Some(error),
            Self::NavigationNotSelected
            | Self::NavigationNotNegotiated
            | Self::TopologyEventsRequireBackend => None,
        }
    }
}

impl From<ProductionResourceError> for ChromiumNavigationApplicationInstallError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Resources(error)
    }
}

/// Installs only negotiated navigation commands.
///
/// Control, portable outbound events, and optional backend work remain
/// explicitly absent. Native CDP liveness is still added by the composition
/// root. Profiles selecting continuous topology events need a separate
/// topology backend recipe rather than silently discarding that unique driver.
#[derive(Clone, Copy, Debug, Default)]
pub struct ChromiumNavigationApplicationInstaller;

impl<Transport, Authorization>
    ChromiumApplicationInstaller<Transport, ChromiumNavigationRuntime, Authorization>
    for ChromiumNavigationApplicationInstaller
{
    type Commands = ChromiumNavigationCommands;
    type Controls = NoServerControl;
    type Outbound = NoServerOutbound;
    type Backend = NoServerBackend;
    type State = ();
    type Error = ChromiumNavigationApplicationInstallError;

    fn install(
        self,
        context: &mut ChromiumApplicationInstallContext<
            Transport,
            ChromiumNavigationRuntime,
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
        if context.topology_events().is_some() {
            return Err(ChromiumNavigationApplicationInstallError::TopologyEventsRequireBackend);
        }
        if !context
            .accepted()
            .hello()
            .enabled_capabilities()
            .iter()
            .any(|capability| capability.id().as_str() == NAVIGATION_CAPABILITY_ID.as_str())
        {
            return Err(ChromiumNavigationApplicationInstallError::NavigationNotNegotiated);
        }
        let runtime = Arc::clone(context.runtime());
        let commands = context
            .composition()
            .remote_navigation_handlers(runtime, context.accepted())?
            .ok_or(ChromiumNavigationApplicationInstallError::NavigationNotSelected)?;
        Ok(ChromiumApplicationComponents::new(
            commands,
            NoServerControl,
            NoServerOutbound,
            NoServerBackend,
            (),
        ))
    }
}

#[derive(Debug)]
pub enum ChromiumDiscoveringNavigationApplicationInstallError {
    NavigationNotSelected,
    NavigationNotNegotiated,
    RemoteControlNotSelected,
    PageDiscoveryNotSelected,
    TopologyEventsNotSelected,
    Composition(ChromiumCompositionError),
    Resources(ProductionResourceError),
    TopologyPump(ChromiumTopologyConnectionPumpConfigurationError),
}

impl Display for ChromiumDiscoveringNavigationApplicationInstallError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NavigationNotSelected => {
                formatter.write_str("the navigation capability was not selected")
            }
            Self::NavigationNotNegotiated => formatter
                .write_str("the remote peer did not negotiate the selected navigation capability"),
            Self::RemoteControlNotSelected => formatter.write_str(
                "automatic Page discovery requires the negotiated remote control component",
            ),
            Self::PageDiscoveryNotSelected => formatter
                .write_str("the automatic Page-discovery production profile was not selected"),
            Self::TopologyEventsNotSelected => formatter.write_str(
                "automatic Page discovery requires the Chromium topology-event component",
            ),
            Self::Composition(error) => Display::fmt(error, formatter),
            Self::Resources(error) => Display::fmt(error, formatter),
            Self::TopologyPump(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ChromiumDiscoveringNavigationApplicationInstallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Composition(error) => Some(error),
            Self::Resources(error) => Some(error),
            Self::TopologyPump(error) => Some(error),
            Self::NavigationNotSelected
            | Self::NavigationNotNegotiated
            | Self::RemoteControlNotSelected
            | Self::PageDiscoveryNotSelected
            | Self::TopologyEventsNotSelected => None,
        }
    }
}

/// Installs navigation together with the minimum continuous Page-discovery
/// path: negotiated lifecycle control and one owned topology backend.
///
/// The topology publisher is intentionally inert, so `BrowserContext.page`
/// and `Page.popup` events are not installed. New object identities still
/// reach the peer through ordered lifecycle `Create` messages.
#[derive(Clone, Copy, Debug, Default)]
pub struct ChromiumDiscoveringNavigationApplicationInstaller;

impl<Transport, Authorization>
    ChromiumApplicationInstaller<Transport, ChromiumDiscoveringNavigationRuntime, Authorization>
    for ChromiumDiscoveringNavigationApplicationInstaller
where
    Transport: ProductionFrameTransport + Send + Sync + 'static,
    Transport::Error: Send,
{
    type Commands = ChromiumDiscoveringNavigationCommands;
    type Controls = ChromiumDiscoveringNavigationControls<Transport>;
    type Outbound = ChromiumDiscoveringNavigationOutbound<Transport>;
    type Backend = ChromiumDiscoveringNavigationBackend<Transport>;
    type State = ();
    type Error = ChromiumDiscoveringNavigationApplicationInstallError;

    fn install(
        self,
        context: &mut ChromiumApplicationInstallContext<
            Transport,
            ChromiumDiscoveringNavigationRuntime,
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
        if !context
            .accepted()
            .hello()
            .enabled_capabilities()
            .iter()
            .any(|capability| capability.id().as_str() == NAVIGATION_CAPABILITY_ID.as_str())
        {
            return Err(
                ChromiumDiscoveringNavigationApplicationInstallError::NavigationNotNegotiated,
            );
        }
        if !context.composition().page_discovery_selected() {
            return Err(
                ChromiumDiscoveringNavigationApplicationInstallError::PageDiscoveryNotSelected,
            );
        }
        let runtime = Arc::clone(context.runtime());
        let transport = Arc::clone(context.transport());
        let commands = context
            .composition()
            .remote_navigation_handlers(Arc::clone(&runtime), context.accepted())
            .map_err(ChromiumDiscoveringNavigationApplicationInstallError::Resources)?
            .ok_or(ChromiumDiscoveringNavigationApplicationInstallError::NavigationNotSelected)?;
        let (controls, outbound) = context
            .composition()
            .remote_control_parts(Arc::clone(&runtime), transport, context.accepted())
            .map_err(ChromiumDiscoveringNavigationApplicationInstallError::Composition)?
            .ok_or(
                ChromiumDiscoveringNavigationApplicationInstallError::RemoteControlNotSelected,
            )?;
        let driver = context.take_topology_events().ok_or(
            ChromiumDiscoveringNavigationApplicationInstallError::TopologyEventsNotSelected,
        )?;
        let pump = ChromiumTopologyConnectionPump::new(
            driver,
            ChromiumTopologyPageEventPublisher::default(),
            ChromiumTopologyEventLossPolicy::TerminateConnection,
        )
        .map_err(ChromiumDiscoveringNavigationApplicationInstallError::TopologyPump)?;
        let backend = ChromiumTopologyServerBackend::new(pump, runtime, outbound.clone());
        Ok(ChromiumApplicationComponents::new(
            commands,
            controls,
            outbound,
            backend,
            (),
        ))
    }
}
