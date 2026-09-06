use pwright_codec_handshake::HandshakeCodecError;
use pwright_core::{ProductionResourceError, ProductionResourceLedger, ResourceRequirements};
use pwright_provider_cdp::CdpWebSocketEndpoint;
use pwright_server_wire::{
    AcceptedServerHandshake, ConcurrentServerBackend, ConcurrentServerFrameHandler,
    ConcurrentServerOutbound, EncodedServerSupervisor, ServerConnectionRunError,
    ServerControlFrameHandler, ServerHandshakeError, ServerHandshakeGate,
    ServerSupervisorCloseReason, ServerSupervisorError, ServerSupervisorEvent,
};
use pwright_transport::{FrameTransport, ProductionFrameTransport, ShutdownFrameTransport};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::{
    AttachedChromiumComposition, AttachedChromiumServerBackend, CdpPageServices,
    ChromiumAttachFactory, ChromiumAttachFactoryError, ChromiumCompositionError,
    ChromiumNetworkEventDriver, ChromiumNetworkEventDriverError,
    ChromiumNetworkPageRegistrationHandle, ChromiumTopologyEventDriver,
    ChromiumTopologyEventDriverError,
};

/// One-shot plugin that constructs a remote transport against the exact fresh
/// ledger owned by a Chromium attachment.
pub trait ChromiumRemoteTransportBuilder {
    type Transport: ProductionFrameTransport;
    type Error;

    fn build(self, resources: &ProductionResourceLedger) -> Result<Self::Transport, Self::Error>;
}

impl<Build, Transport, BuildError> ChromiumRemoteTransportBuilder for Build
where
    Build: FnOnce(&ProductionResourceLedger) -> Result<Transport, BuildError>,
    Transport: ProductionFrameTransport,
{
    type Transport = Transport;
    type Error = BuildError;

    fn build(self, resources: &ProductionResourceLedger) -> Result<Self::Transport, Self::Error> {
        self(resources)
    }
}

/// One-shot plugin that installs exactly one caller-selected runtime against a
/// fresh native attachment.
pub trait ChromiumRuntimeInstaller {
    type Runtime;
    type Error;

    fn install(
        self,
        composition: &mut AttachedChromiumComposition,
    ) -> Result<Self::Runtime, Self::Error>;
}

impl<Install, Runtime, InstallError> ChromiumRuntimeInstaller for Install
where
    Install: FnOnce(&mut AttachedChromiumComposition) -> Result<Runtime, InstallError>,
{
    type Runtime = Runtime;
    type Error = InstallError;

    fn install(
        self,
        composition: &mut AttachedChromiumComposition,
    ) -> Result<Self::Runtime, Self::Error> {
        self(composition)
    }
}

/// Fresh native attachment and ledger-verified remote transport awaiting its
/// caller-selected runtime installation.
///
/// Construction also claims the attachment's only topology-event driver, when
/// selected. It performs no handshake and starts no task or executor.
pub struct ChromiumRemoteConnectionSeed<Transport> {
    composition: AttachedChromiumComposition,
    supervisor: EncodedServerSupervisor<Transport>,
    topology_events: Option<ChromiumTopologyEventDriver>,
}

impl<Transport> ChromiumRemoteConnectionSeed<Transport> {
    pub const fn composition(&self) -> &AttachedChromiumComposition {
        &self.composition
    }

    pub const fn supervisor(&self) -> &EncodedServerSupervisor<Transport> {
        &self.supervisor
    }

    pub const fn topology_events(&self) -> Option<&ChromiumTopologyEventDriver> {
        self.topology_events.as_ref()
    }

    /// Moves the seed through the only supported runtime-installation stage.
    /// Returning an arbitrary value without installing a composition runtime is
    /// rejected and drops all fresh connection state together.
    pub fn install_runtime<Installer>(
        mut self,
        installer: Installer,
    ) -> Result<
        ChromiumRemoteConnection<Transport, Installer::Runtime>,
        ChromiumRemoteRuntimeInstallError<Installer::Error>,
    >
    where
        Installer: ChromiumRuntimeInstaller,
    {
        let runtime = installer
            .install(&mut self.composition)
            .map_err(ChromiumRemoteRuntimeInstallError::Installer)?;
        if !self.composition.runtime_installed() {
            return Err(ChromiumRemoteRuntimeInstallError::RuntimeNotInstalled);
        }
        Ok(ChromiumRemoteConnection {
            composition: self.composition,
            supervisor: self.supervisor,
            topology_events: self.topology_events,
            runtime,
        })
    }
}

/// One remote connection's fresh native attachment, transport supervisor,
/// runtime graph, and optional unique topology-event baseline.
pub struct ChromiumRemoteConnection<Transport, Runtime> {
    composition: AttachedChromiumComposition,
    supervisor: EncodedServerSupervisor<Transport>,
    topology_events: Option<ChromiumTopologyEventDriver>,
    runtime: Runtime,
}

impl<Transport, Runtime> ChromiumRemoteConnection<Transport, Runtime> {
    pub const fn composition(&self) -> &AttachedChromiumComposition {
        &self.composition
    }

    pub fn composition_mut(&mut self) -> &mut AttachedChromiumComposition {
        &mut self.composition
    }

    pub const fn supervisor(&self) -> &EncodedServerSupervisor<Transport> {
        &self.supervisor
    }

    pub fn supervisor_mut(&mut self) -> &mut EncodedServerSupervisor<Transport> {
        &mut self.supervisor
    }

    pub const fn topology_events(&self) -> Option<&ChromiumTopologyEventDriver> {
        self.topology_events.as_ref()
    }

    pub fn topology_events_mut(&mut self) -> Option<&mut ChromiumTopologyEventDriver> {
        self.topology_events.as_mut()
    }

    pub const fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.runtime
    }

    pub fn into_parts(
        self,
    ) -> (
        AttachedChromiumComposition,
        EncodedServerSupervisor<Transport>,
        Option<ChromiumTopologyEventDriver>,
        Runtime,
    ) {
        (
            self.composition,
            self.supervisor,
            self.topology_events,
            self.runtime,
        )
    }
}

pub type ChromiumRemoteHandshakeAcceptError<AuthorizationError, TransportError> =
    ServerHandshakeError<
        HandshakeCodecError,
        HandshakeCodecError,
        AuthorizationError,
        TransportError,
    >;

impl<Transport, Runtime> ChromiumRemoteConnection<Transport, Runtime>
where
    Transport: FrameTransport + 'static,
{
    /// Accepts the only handshake that can advance this connection's own
    /// supervisor. Failure consumes and drops the fresh connection graph.
    pub async fn accept_handshake<Gate>(
        mut self,
        gate: &mut Gate,
    ) -> Result<
        ChromiumAcceptedRemoteConnection<Transport, Runtime, Gate::Authorization>,
        ChromiumRemoteHandshakeAcceptError<Gate::Error, Transport::Error>,
    >
    where
        Gate: ServerHandshakeGate,
    {
        let (codec, config) = self
            .composition
            .remote_handshake_parts()
            .expect("remote connections are constructed only from handshake-enabled profiles");
        let accepted = self
            .supervisor
            .accept_handshake(&codec, &config, gate)
            .await?;
        Ok(ChromiumAcceptedRemoteConnection {
            composition: self.composition,
            supervisor: self.supervisor,
            topology_events: self.topology_events,
            runtime: self.runtime,
            accepted,
        })
    }
}

/// A runtime-installed connection whose accepted handshake was produced by
/// the supervisor stored in this same value.
pub struct ChromiumAcceptedRemoteConnection<Transport, Runtime, Authorization> {
    composition: AttachedChromiumComposition,
    supervisor: EncodedServerSupervisor<Transport>,
    topology_events: Option<ChromiumTopologyEventDriver>,
    runtime: Runtime,
    accepted: AcceptedServerHandshake<Authorization>,
}

pub type ChromiumInstalledApplication<Transport, Runtime, Authorization, Installer> =
    ChromiumApplicationConnection<
        Transport,
        Runtime,
        Authorization,
        <Installer as ChromiumApplicationInstaller<Transport, Runtime, Authorization>>::Commands,
        <Installer as ChromiumApplicationInstaller<Transport, Runtime, Authorization>>::Controls,
        <Installer as ChromiumApplicationInstaller<Transport, Runtime, Authorization>>::Outbound,
        <Installer as ChromiumApplicationInstaller<Transport, Runtime, Authorization>>::Backend,
        <Installer as ChromiumApplicationInstaller<Transport, Runtime, Authorization>>::State,
    >;

pub type ChromiumApplicationInstallResult<Transport, Runtime, Authorization, Installer> = Result<
    ChromiumInstalledApplication<Transport, Runtime, Authorization, Installer>,
    ChromiumApplicationInstallError<
        <Installer as ChromiumApplicationInstaller<Transport, Runtime, Authorization>>::Error,
    >,
>;

impl<Transport, Runtime, Authorization>
    ChromiumAcceptedRemoteConnection<Transport, Runtime, Authorization>
{
    pub const fn composition(&self) -> &AttachedChromiumComposition {
        &self.composition
    }

    pub const fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub const fn accepted(&self) -> &AcceptedServerHandshake<Authorization> {
        &self.accepted
    }

    pub const fn topology_events(&self) -> Option<&ChromiumTopologyEventDriver> {
        self.topology_events.as_ref()
    }

    /// Installs all caller-selected application plugins in one transaction.
    /// If topology or network events were selected, the installer must take
    /// ownership of each unique driver before returning.
    pub fn install_application<Installer>(
        self,
        installer: Installer,
    ) -> ChromiumApplicationInstallResult<Transport, Runtime, Authorization, Installer>
    where
        Transport: FrameTransport + ShutdownFrameTransport + 'static,
        Installer: ChromiumApplicationInstaller<Transport, Runtime, Authorization>,
        Installer::Commands: ConcurrentServerFrameHandler<Transport>,
        Installer::Controls: ServerControlFrameHandler,
        Installer::Outbound: ConcurrentServerOutbound,
        Installer::Backend: ConcurrentServerBackend,
    {
        let network_events_required = self.composition.network_events_selected();
        let mut context = ChromiumApplicationInstallContext {
            composition: self.composition,
            supervisor: self.supervisor,
            runtime: self.runtime,
            accepted: self.accepted,
            topology_events: self.topology_events,
            network_events_required,
            network_events_consumed: false,
            network_registration: None,
        };
        let topology_events_required = context.topology_events.is_some();
        let components = installer
            .install(&mut context)
            .map_err(ChromiumApplicationInstallError::Installer)?;
        if topology_events_required && context.topology_events.is_some() {
            return Err(ChromiumApplicationInstallError::TopologyEventsNotConsumed);
        }
        if context.network_events_required && !context.network_events_consumed {
            return Err(ChromiumApplicationInstallError::NetworkEventsNotConsumed);
        }
        let (commands, controls, outbound, backend, state) = components.into_parts();
        let ChromiumApplicationInstallContext {
            composition,
            supervisor,
            runtime,
            accepted,
            topology_events: _,
            network_events_required: _,
            network_events_consumed: _,
            network_registration: _,
        } = context;
        let backend = composition.server_backend(backend);
        Ok(ChromiumApplicationConnection {
            composition,
            supervisor,
            runtime,
            accepted,
            commands,
            controls,
            outbound,
            backend,
            state,
        })
    }
}

/// Owned assembly view for plugins installed after handshake negotiation.
/// It has no public constructor, so the accepted handshake and transport
/// always originate from the staged connection that invokes the installer.
pub struct ChromiumApplicationInstallContext<Transport, Runtime, Authorization> {
    composition: AttachedChromiumComposition,
    supervisor: EncodedServerSupervisor<Transport>,
    runtime: Runtime,
    accepted: AcceptedServerHandshake<Authorization>,
    topology_events: Option<ChromiumTopologyEventDriver>,
    network_events_required: bool,
    network_events_consumed: bool,
    network_registration: Option<ChromiumNetworkPageRegistrationHandle>,
}

impl<Transport, Runtime, Authorization>
    ChromiumApplicationInstallContext<Transport, Runtime, Authorization>
{
    pub const fn composition(&self) -> &AttachedChromiumComposition {
        &self.composition
    }

    pub fn transport(&self) -> &std::sync::Arc<Transport> {
        self.supervisor.transport()
    }

    pub const fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub const fn accepted(&self) -> &AcceptedServerHandshake<Authorization> {
        &self.accepted
    }

    pub const fn topology_events(&self) -> Option<&ChromiumTopologyEventDriver> {
        self.topology_events.as_ref()
    }

    pub fn take_topology_events(&mut self) -> Option<ChromiumTopologyEventDriver> {
        self.topology_events
            .take()
            .map(|driver| match self.network_registration.clone() {
                Some(registration) => driver.with_network_registration(registration),
                None => driver,
            })
    }

    /// Installs and moves out the selected single-cursor network driver. Its
    /// non-owning registration handle is also attached to the topology driver,
    /// independent of which driver the installer takes first.
    pub fn take_network_events(
        &mut self,
        page_services: &CdpPageServices,
    ) -> Result<Option<ChromiumNetworkEventDriver>, ChromiumNetworkEventDriverError> {
        let driver = self.composition.network_event_driver(page_services)?;
        if let Some(driver) = driver.as_ref() {
            let registration = driver.registration_handle();
            self.network_registration = Some(registration.clone());
            if let Some(topology) = self.topology_events.take() {
                self.topology_events = Some(topology.with_network_registration(registration));
            }
            self.network_events_consumed = true;
        }
        Ok(driver)
    }
}

/// One-shot plugin that builds application handlers, outbound publishers, an
/// optional backend lane, and arbitrary caller state from one accepted
/// connection. The composition root adds mandatory CDP liveness afterward.
pub type ChromiumApplicationInstallerResult<Transport, Runtime, Authorization, Installer> =
    ChromiumApplicationComponentsResult<
        <Installer as ChromiumApplicationInstaller<Transport, Runtime, Authorization>>::Commands,
        <Installer as ChromiumApplicationInstaller<Transport, Runtime, Authorization>>::Controls,
        <Installer as ChromiumApplicationInstaller<Transport, Runtime, Authorization>>::Outbound,
        <Installer as ChromiumApplicationInstaller<Transport, Runtime, Authorization>>::Backend,
        <Installer as ChromiumApplicationInstaller<Transport, Runtime, Authorization>>::State,
        <Installer as ChromiumApplicationInstaller<Transport, Runtime, Authorization>>::Error,
    >;

pub trait ChromiumApplicationInstaller<Transport, Runtime, Authorization>: Sized {
    type Commands;
    type Controls;
    type Outbound;
    type Backend;
    type State;
    type Error;

    fn install(
        self,
        context: &mut ChromiumApplicationInstallContext<Transport, Runtime, Authorization>,
    ) -> ChromiumApplicationInstallerResult<Transport, Runtime, Authorization, Self>;
}

impl<
    Install,
    Transport,
    Runtime,
    Authorization,
    Commands,
    Controls,
    Outbound,
    Backend,
    State,
    InstallError,
> ChromiumApplicationInstaller<Transport, Runtime, Authorization> for Install
where
    Install: FnOnce(
        &mut ChromiumApplicationInstallContext<Transport, Runtime, Authorization>,
    ) -> Result<
        ChromiumApplicationComponents<Commands, Controls, Outbound, Backend, State>,
        InstallError,
    >,
{
    type Commands = Commands;
    type Controls = Controls;
    type Outbound = Outbound;
    type Backend = Backend;
    type State = State;
    type Error = InstallError;

    fn install(
        self,
        context: &mut ChromiumApplicationInstallContext<Transport, Runtime, Authorization>,
    ) -> Result<
        ChromiumApplicationComponents<Commands, Controls, Outbound, Backend, State>,
        InstallError,
    > {
        self(context)
    }
}

/// The independently selected pieces returned by an application installer.
pub struct ChromiumApplicationComponents<Commands, Controls, Outbound, Backend, State> {
    commands: Commands,
    controls: Controls,
    outbound: Outbound,
    backend: Backend,
    state: State,
}

pub type ChromiumApplicationComponentsResult<
    Commands,
    Controls,
    Outbound,
    Backend,
    State,
    InstallError,
> = Result<
    ChromiumApplicationComponents<Commands, Controls, Outbound, Backend, State>,
    InstallError,
>;

impl<Commands, Controls, Outbound, Backend, State>
    ChromiumApplicationComponents<Commands, Controls, Outbound, Backend, State>
{
    pub const fn new(
        commands: Commands,
        controls: Controls,
        outbound: Outbound,
        backend: Backend,
        state: State,
    ) -> Self {
        Self {
            commands,
            controls,
            outbound,
            backend,
            state,
        }
    }

    pub fn into_parts(self) -> (Commands, Controls, Outbound, Backend, State) {
        (
            self.commands,
            self.controls,
            self.outbound,
            self.backend,
            self.state,
        )
    }
}

/// Fully assembled application connection. Its backend field is always the
/// caller plugin wrapped by the attachment's mandatory CDP liveness lane.
pub struct ChromiumApplicationConnection<
    Transport,
    Runtime,
    Authorization,
    Commands,
    Controls,
    Outbound,
    Backend,
    State,
> {
    composition: AttachedChromiumComposition,
    supervisor: EncodedServerSupervisor<Transport>,
    runtime: Runtime,
    accepted: AcceptedServerHandshake<Authorization>,
    commands: Commands,
    controls: Controls,
    outbound: Outbound,
    backend: AttachedChromiumServerBackend<Backend>,
    state: State,
}

impl<Transport, Runtime, Authorization, Commands, Controls, Outbound, Backend, State>
    ChromiumApplicationConnection<
        Transport,
        Runtime,
        Authorization,
        Commands,
        Controls,
        Outbound,
        Backend,
        State,
    >
{
    pub const fn composition(&self) -> &AttachedChromiumComposition {
        &self.composition
    }

    pub const fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub const fn accepted(&self) -> &AcceptedServerHandshake<Authorization> {
        &self.accepted
    }

    pub const fn commands(&self) -> &Commands {
        &self.commands
    }

    pub const fn controls(&self) -> &Controls {
        &self.controls
    }

    pub const fn outbound(&self) -> &Outbound {
        &self.outbound
    }

    pub const fn backend(&self) -> &AttachedChromiumServerBackend<Backend> {
        &self.backend
    }

    pub const fn state(&self) -> &State {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }
}

impl<Transport, Runtime, Authorization, Commands, Controls, Outbound, Backend, State>
    ChromiumApplicationConnection<
        Transport,
        Runtime,
        Authorization,
        Commands,
        Controls,
        Outbound,
        Backend,
        State,
    >
where
    Transport: FrameTransport + ShutdownFrameTransport + 'static,
    Commands: ConcurrentServerFrameHandler<Transport>,
    Controls: ServerControlFrameHandler,
    Outbound: ConcurrentServerOutbound,
    Backend: ConcurrentServerBackend,
{
    /// Runs the complete accepted connection and always uses the root-wrapped
    /// backend lane. Every terminal application outcome shuts down the
    /// transport write half through the supervisor's complete-run API.
    pub async fn run<Observe, ObserverError>(
        &mut self,
        observe: Observe,
    ) -> Result<
        ServerSupervisorCloseReason,
        ServerConnectionRunError<
            ServerSupervisorError<
                Transport::Error,
                Commands::Error,
                Controls::Error,
                ObserverError,
                Outbound::Error,
                <AttachedChromiumServerBackend<Backend> as ConcurrentServerBackend>::Error,
            >,
            Transport::Error,
        >,
    >
    where
        Observe: FnMut(
            ServerSupervisorEvent<
                Commands::Event,
                Controls::Event,
                Outbound::Event,
                <AttachedChromiumServerBackend<Backend> as ConcurrentServerBackend>::Event,
            >,
        ) -> Result<(), ObserverError>,
    {
        self.supervisor
            .run_connection_with_outbound_and_backend(
                &self.commands,
                &mut self.controls,
                &self.outbound,
                &mut self.backend,
                observe,
            )
            .await
    }
}

#[derive(Debug)]
pub enum ChromiumApplicationInstallError<InstallError> {
    Installer(InstallError),
    TopologyEventsNotConsumed,
    NetworkEventsNotConsumed,
}

impl<InstallError> Display for ChromiumApplicationInstallError<InstallError>
where
    InstallError: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Installer(error) => Display::fmt(error, formatter),
            Self::TopologyEventsNotConsumed => formatter.write_str(
                "application installer did not consume the selected unique topology-event driver",
            ),
            Self::NetworkEventsNotConsumed => formatter.write_str(
                "application installer did not consume the selected unique network-event driver",
            ),
        }
    }
}

impl<InstallError> Error for ChromiumApplicationInstallError<InstallError>
where
    InstallError: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Installer(error) => Some(error),
            Self::TopologyEventsNotConsumed | Self::NetworkEventsNotConsumed => None,
        }
    }
}

impl ChromiumAttachFactory {
    /// Creates one fresh native/remote connection seed. The builder receives
    /// the new attachment's ledger and must return a production transport that
    /// proves both that exact ledger identity and the declared transport plan.
    pub async fn attach_remote<Builder>(
        &self,
        endpoint: CdpWebSocketEndpoint,
        transport_builder: Builder,
    ) -> Result<
        ChromiumRemoteConnectionSeed<Builder::Transport>,
        ChromiumRemoteConnectionSeedError<Builder::Error>,
    >
    where
        Builder: ChromiumRemoteTransportBuilder,
    {
        let transport_requirements = self
            .profile()
            .remote_handshake()
            .ok_or(ChromiumRemoteConnectionSeedError::RemoteHandshakeNotInstalled)?
            .transport_requirements();
        let mut composition = self
            .attach(endpoint)
            .await
            .map_err(ChromiumRemoteConnectionSeedError::Attachment)?;
        let transport = transport_builder
            .build(composition.resources())
            .map_err(ChromiumRemoteConnectionSeedError::Transport)?;
        validate_remote_transport(composition.resources(), transport_requirements, &transport)
            .map_err(ChromiumRemoteConnectionSeedError::Connection)?;
        let supervisor = composition
            .new_supervisor(transport)
            .map_err(ChromiumRemoteConnectionSeedError::Composition)?;
        let topology_events = composition
            .topology_event_driver()
            .map_err(ChromiumRemoteConnectionSeedError::TopologyEvents)?;
        Ok(ChromiumRemoteConnectionSeed {
            composition,
            supervisor,
            topology_events,
        })
    }
}

pub(crate) fn validate_remote_transport<Transport>(
    resources: &ProductionResourceLedger,
    declared: ResourceRequirements,
    transport: &Transport,
) -> Result<(), ChromiumRemoteConnectionError>
where
    Transport: ProductionFrameTransport,
{
    let actual = transport.production_resources();
    if actual.requirements() != declared {
        return Err(ChromiumRemoteConnectionError::TransportPlanMismatch {
            declared: Box::new(declared),
            actual: Box::new(actual.requirements()),
        });
    }
    let connection_ledger = resources
        .claim(declared)
        .map_err(ChromiumRemoteConnectionError::Resources)?;
    if !actual.ledger().shares_budget_with(&connection_ledger) {
        return Err(ChromiumRemoteConnectionError::TransportLedgerMismatch);
    }
    Ok(())
}

#[derive(Debug)]
pub enum ChromiumRemoteConnectionError {
    TransportPlanMismatch {
        declared: Box<ResourceRequirements>,
        actual: Box<ResourceRequirements>,
    },
    TransportLedgerMismatch,
    Resources(ProductionResourceError),
}

impl Display for ChromiumRemoteConnectionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::TransportPlanMismatch { declared, actual } => write!(
                formatter,
                "remote transport resource plan {actual:?} does not match declared plan {declared:?}"
            ),
            Self::TransportLedgerMismatch => formatter.write_str(
                "remote transport does not share the fresh Chromium connection resource ledger",
            ),
            Self::Resources(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ChromiumRemoteConnectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resources(error) => Some(error),
            Self::TransportPlanMismatch { .. } | Self::TransportLedgerMismatch => None,
        }
    }
}

#[derive(Debug)]
pub enum ChromiumRemoteConnectionSeedError<TransportError> {
    RemoteHandshakeNotInstalled,
    Attachment(ChromiumAttachFactoryError),
    Transport(TransportError),
    Connection(ChromiumRemoteConnectionError),
    Composition(ChromiumCompositionError),
    TopologyEvents(ChromiumTopologyEventDriverError),
}

impl<TransportError> Display for ChromiumRemoteConnectionSeedError<TransportError>
where
    TransportError: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::RemoteHandshakeNotInstalled => formatter.write_str(
                "a fresh remote Chromium connection requires the remote handshake component",
            ),
            Self::Attachment(error) => Display::fmt(error, formatter),
            Self::Transport(error) => Display::fmt(error, formatter),
            Self::Connection(error) => Display::fmt(error, formatter),
            Self::Composition(error) => Display::fmt(error, formatter),
            Self::TopologyEvents(error) => Display::fmt(error, formatter),
        }
    }
}

impl<TransportError> Error for ChromiumRemoteConnectionSeedError<TransportError>
where
    TransportError: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::RemoteHandshakeNotInstalled => None,
            Self::Attachment(error) => Some(error),
            Self::Transport(error) => Some(error),
            Self::Connection(error) => Some(error),
            Self::Composition(error) => Some(error),
            Self::TopologyEvents(error) => Some(error),
        }
    }
}

#[derive(Debug)]
pub enum ChromiumRemoteRuntimeInstallError<InstallError> {
    Installer(InstallError),
    RuntimeNotInstalled,
}

impl<InstallError> Display for ChromiumRemoteRuntimeInstallError<InstallError>
where
    InstallError: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Installer(error) => Display::fmt(error, formatter),
            Self::RuntimeNotInstalled => formatter.write_str(
                "Chromium runtime installer returned without installing the fresh attachment runtime",
            ),
        }
    }
}

impl<InstallError> Error for ChromiumRemoteRuntimeInstallError<InstallError>
where
    InstallError: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Installer(error) => Some(error),
            Self::RuntimeNotInstalled => None,
        }
    }
}
