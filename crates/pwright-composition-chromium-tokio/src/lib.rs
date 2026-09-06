//! Optional bridge from authenticated caller-accepted Tokio endpoints to one
//! staged Chromium remote connection.
//!
//! This crate binds no listener and creates no accept loop, runtime, executor,
//! task, browser process, or browser download. The caller supplies a pre-bound
//! endpoint and explicitly awaits each one-connection accept together with one
//! external CDP endpoint.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;

use pwright_composition_chromium::{
    ChromiumAcceptedRemoteConnection, ChromiumAttachFactory, ChromiumRemoteConnection,
    ChromiumRemoteConnectionSeed, ChromiumRemoteConnectionSeedError,
    ChromiumRemoteHandshakeAcceptError, ChromiumRemoteRuntimeInstallError,
    ChromiumRuntimeInstaller,
};
use pwright_core::ProductionResourceLedger;
use pwright_endpoint_tokio::{
    AcceptedMutualTlsConnection, AcceptedTailnetConnection, MutualTlsAuthenticationError,
    MutualTlsEndpoint, MutualTlsPeerGate, MutualTlsServerFrameTransport,
    TailnetAuthenticationError, TailnetEndpoint, TailnetFrameTransport, TailnetPeerGate,
};
#[cfg(unix)]
use pwright_endpoint_tokio::{
    AcceptedUnixConnection, PrivateUnixEndpoint, UnixEndpointError, UnixFrameTransport,
    UnixPeerGate, UnixPeerIdentity,
};
use pwright_provider_cdp::CdpWebSocketEndpoint;
use pwright_server_wire::ServerHandshakeGate;
use pwright_transport::FrameTransport;
use pwright_transport_tokio::{ProductionTokioStreamTransport, TokioStreamTransport};
use tokio::io::{AsyncRead, AsyncWrite};

/// Proof supplied by one already-authenticated endpoint connection.
///
/// Implementations split that connection into its frame transport and the
/// handshake gate derived from the same authenticated peer. The adapter keeps
/// them paired until authorization has been consumed.
pub trait AcceptedChromiumEndpoint: Sized {
    type Reader: AsyncRead + Unpin + Send;
    type Writer: AsyncWrite + Unpin + Send;
    type Gate: ServerHandshakeGate;

    fn into_transport_and_gate(
        self,
    ) -> (TokioStreamTransport<Self::Reader, Self::Writer>, Self::Gate);
}

/// One caller-owned listener which can accept exactly one authenticated
/// endpoint connection per explicitly awaited call.
///
/// The trait deliberately exposes no loop, task, retry, timeout, or admission
/// semaphore. Endpoint implementations retain those policies in their own
/// concrete types, and the application decides when another accept may begin.
pub trait ChromiumEndpointAcceptor {
    type Accepted: AcceptedChromiumEndpoint;
    type Error;

    fn accept(&self) -> impl Future<Output = Result<Self::Accepted, Self::Error>> + Send + '_;
}

impl ChromiumEndpointAcceptor for TailnetEndpoint {
    type Accepted = AcceptedTailnetConnection;
    type Error = TailnetAuthenticationError;

    fn accept(&self) -> impl Future<Output = Result<Self::Accepted, Self::Error>> + Send + '_ {
        TailnetEndpoint::accept(self)
    }
}

impl<const CLIENT_PINS: usize> ChromiumEndpointAcceptor for MutualTlsEndpoint<CLIENT_PINS> {
    type Accepted = AcceptedMutualTlsConnection;
    type Error = MutualTlsAuthenticationError;

    fn accept(&self) -> impl Future<Output = Result<Self::Accepted, Self::Error>> + Send + '_ {
        MutualTlsEndpoint::accept(self)
    }
}

impl AcceptedChromiumEndpoint for AcceptedTailnetConnection {
    type Reader = <TailnetFrameTransport as TokioTransportParts>::Reader;
    type Writer = <TailnetFrameTransport as TokioTransportParts>::Writer;
    type Gate = TailnetPeerGate;

    fn into_transport_and_gate(
        self,
    ) -> (TokioStreamTransport<Self::Reader, Self::Writer>, Self::Gate) {
        self.into_peer_gate()
    }
}

impl AcceptedChromiumEndpoint for AcceptedMutualTlsConnection {
    type Reader = <MutualTlsServerFrameTransport as TokioTransportParts>::Reader;
    type Writer = <MutualTlsServerFrameTransport as TokioTransportParts>::Writer;
    type Gate = MutualTlsPeerGate;

    fn into_transport_and_gate(
        self,
    ) -> (TokioStreamTransport<Self::Reader, Self::Writer>, Self::Gate) {
        self.into_peer_gate()
    }
}

/// Adds the deployment policy needed to authorize one accepted private Unix
/// connection. The accepted connection and allowed UID remain one value.
#[cfg(unix)]
pub struct AllowedUnixConnection {
    accepted: AcceptedUnixConnection,
    allowed_uid: u32,
}

#[cfg(unix)]
impl AllowedUnixConnection {
    pub const fn new(accepted: AcceptedUnixConnection, allowed_uid: u32) -> Self {
        Self {
            accepted,
            allowed_uid,
        }
    }

    pub const fn peer(&self) -> UnixPeerIdentity {
        self.accepted.peer()
    }

    pub const fn allowed_uid(&self) -> u32 {
        self.allowed_uid
    }
}

#[cfg(unix)]
impl AcceptedChromiumEndpoint for AllowedUnixConnection {
    type Reader = <UnixFrameTransport as TokioTransportParts>::Reader;
    type Writer = <UnixFrameTransport as TokioTransportParts>::Writer;
    type Gate = UnixPeerGate;

    fn into_transport_and_gate(
        self,
    ) -> (TokioStreamTransport<Self::Reader, Self::Writer>, Self::Gate) {
        self.accepted.into_uid_gate(self.allowed_uid)
    }
}

/// Private Unix listener paired with the only UID allowed to advance a
/// connection into the Pwright handshake.
#[cfg(unix)]
pub struct AllowedUnixEndpoint {
    endpoint: PrivateUnixEndpoint,
    allowed_uid: u32,
}

#[cfg(unix)]
impl AllowedUnixEndpoint {
    pub const fn new(endpoint: PrivateUnixEndpoint, allowed_uid: u32) -> Self {
        Self {
            endpoint,
            allowed_uid,
        }
    }

    pub const fn endpoint(&self) -> &PrivateUnixEndpoint {
        &self.endpoint
    }

    pub const fn allowed_uid(&self) -> u32 {
        self.allowed_uid
    }

    pub fn close(self) -> Result<bool, UnixEndpointError> {
        self.endpoint.close()
    }
}

#[cfg(unix)]
impl ChromiumEndpointAcceptor for AllowedUnixEndpoint {
    type Accepted = AllowedUnixConnection;
    type Error = UnixEndpointError;

    async fn accept(&self) -> Result<Self::Accepted, Self::Error> {
        self.endpoint
            .accept()
            .await
            .map(|accepted| AllowedUnixConnection::new(accepted, self.allowed_uid))
    }
}

/// Extracts the generic I/O halves from the endpoint crate's public Tokio
/// transport aliases without duplicating their concrete socket/TLS types.
pub trait TokioTransportParts {
    type Reader: AsyncRead + Unpin + Send;
    type Writer: AsyncWrite + Unpin + Send;
}

impl<Reader, Writer> TokioTransportParts for TokioStreamTransport<Reader, Writer>
where
    Reader: AsyncRead + Unpin + Send,
    Writer: AsyncWrite + Unpin + Send,
{
    type Reader = Reader;
    type Writer = Writer;
}

pub type ChromiumEndpointTransport<Endpoint> = ProductionTokioStreamTransport<
    <Endpoint as AcceptedChromiumEndpoint>::Reader,
    <Endpoint as AcceptedChromiumEndpoint>::Writer,
>;

pub type ChromiumEndpointSeedError =
    ChromiumRemoteConnectionSeedError<pwright_transport_tokio::TokioTransportError>;

#[derive(Debug)]
pub enum ChromiumEndpointAcceptError<EndpointError> {
    Endpoint(EndpointError),
    Connection(ChromiumEndpointSeedError),
}

impl<EndpointError> Display for ChromiumEndpointAcceptError<EndpointError>
where
    EndpointError: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Endpoint(error) => write!(formatter, "endpoint acceptance failed: {error}"),
            Self::Connection(error) => Display::fmt(error, formatter),
        }
    }
}

impl<EndpointError> Error for ChromiumEndpointAcceptError<EndpointError>
where
    EndpointError: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Endpoint(error) => Some(error),
            Self::Connection(error) => Some(error),
        }
    }
}

/// Caller-driven listener recipe for a fresh Chromium remote connection.
///
/// Each `accept_connection` call first completes endpoint authentication, then
/// gives the accepted transport to a fresh attachment created by the stateless
/// factory. It does not install a runtime or application recipe, so those
/// capability choices remain per-connection and statically typed.
pub struct ChromiumEndpointServer<Endpoint> {
    factory: ChromiumAttachFactory,
    cdp_endpoint: CdpWebSocketEndpoint,
    endpoint: Endpoint,
}

impl<Endpoint> ChromiumEndpointServer<Endpoint> {
    pub const fn new(
        factory: ChromiumAttachFactory,
        cdp_endpoint: CdpWebSocketEndpoint,
        endpoint: Endpoint,
    ) -> Self {
        Self {
            factory,
            cdp_endpoint,
            endpoint,
        }
    }

    pub const fn factory(&self) -> &ChromiumAttachFactory {
        &self.factory
    }

    pub const fn cdp_endpoint(&self) -> &CdpWebSocketEndpoint {
        &self.cdp_endpoint
    }

    pub const fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    pub fn into_parts(self) -> (ChromiumAttachFactory, CdpWebSocketEndpoint, Endpoint) {
        (self.factory, self.cdp_endpoint, self.endpoint)
    }
}

impl<Endpoint> ChromiumEndpointServer<Endpoint>
where
    Endpoint: ChromiumEndpointAcceptor,
{
    pub async fn accept_connection(
        &self,
    ) -> Result<
        ChromiumAuthenticatedConnectionSeed<
            ChromiumEndpointTransport<Endpoint::Accepted>,
            <Endpoint::Accepted as AcceptedChromiumEndpoint>::Gate,
        >,
        ChromiumEndpointAcceptError<Endpoint::Error>,
    > {
        let accepted = self
            .endpoint
            .accept()
            .await
            .map_err(ChromiumEndpointAcceptError::Endpoint)?;
        attach_authenticated_endpoint(&self.factory, self.cdp_endpoint.clone(), accepted)
            .await
            .map_err(ChromiumEndpointAcceptError::Connection)
    }
}

/// One native attachment plus the authorization gate from the exact accepted
/// endpoint connection used to construct its remote transport.
pub struct ChromiumAuthenticatedConnectionSeed<Transport, Gate> {
    connection: ChromiumRemoteConnectionSeed<Transport>,
    gate: Gate,
}

impl<Transport, Gate> ChromiumAuthenticatedConnectionSeed<Transport, Gate> {
    pub const fn connection(&self) -> &ChromiumRemoteConnectionSeed<Transport> {
        &self.connection
    }

    pub const fn gate(&self) -> &Gate {
        &self.gate
    }

    pub fn install_runtime<Installer>(
        self,
        installer: Installer,
    ) -> Result<
        ChromiumAuthenticatedConnection<Transport, Installer::Runtime, Gate>,
        ChromiumRemoteRuntimeInstallError<Installer::Error>,
    >
    where
        Installer: ChromiumRuntimeInstaller,
    {
        let connection = self.connection.install_runtime(installer)?;
        Ok(ChromiumAuthenticatedConnection {
            connection,
            gate: self.gate,
        })
    }
}

/// A runtime-installed connection that still owns its endpoint-derived gate.
pub struct ChromiumAuthenticatedConnection<Transport, Runtime, Gate> {
    connection: ChromiumRemoteConnection<Transport, Runtime>,
    gate: Gate,
}

impl<Transport, Runtime, Gate> ChromiumAuthenticatedConnection<Transport, Runtime, Gate>
where
    Transport: FrameTransport + 'static,
    Gate: ServerHandshakeGate,
{
    pub const fn connection(&self) -> &ChromiumRemoteConnection<Transport, Runtime> {
        &self.connection
    }

    pub const fn gate(&self) -> &Gate {
        &self.gate
    }

    /// Consumes the paired gate so an accepted authorization can only be
    /// produced by the supervisor and endpoint connection stored together.
    pub async fn accept_handshake(
        mut self,
    ) -> Result<
        ChromiumAcceptedRemoteConnection<Transport, Runtime, Gate::Authorization>,
        ChromiumRemoteHandshakeAcceptError<Gate::Error, Transport::Error>,
    > {
        self.connection.accept_handshake(&mut self.gate).await
    }
}

/// Attaches one caller-accepted authenticated endpoint to one fresh Chromium
/// composition. Production transport accounting is claimed from the same
/// fresh ledger and checked against the handshake's declared transport plan.
pub async fn attach_authenticated_endpoint<Endpoint>(
    factory: &ChromiumAttachFactory,
    cdp_endpoint: CdpWebSocketEndpoint,
    endpoint: Endpoint,
) -> Result<
    ChromiumAuthenticatedConnectionSeed<ChromiumEndpointTransport<Endpoint>, Endpoint::Gate>,
    ChromiumEndpointSeedError,
>
where
    Endpoint: AcceptedChromiumEndpoint,
{
    let (transport, gate) = endpoint.into_transport_and_gate();
    let connection = factory
        .attach_remote(cdp_endpoint, move |resources: &ProductionResourceLedger| {
            transport.with_production_resources(resources)
        })
        .await?;
    Ok(ChromiumAuthenticatedConnectionSeed { connection, gate })
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::MetadataExt;
    #[cfg(unix)]
    use std::path::PathBuf;
    #[cfg(unix)]
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::{future::ready, io};

    use pwright_backend_cdp::{
        CdpNavigationLimits, CdpNavigationProductionProfile, CdpTopologyLimits,
        CdpTopologyProductionProfile,
    };
    use pwright_cdp::CdpConnectionProductionProfile;
    use pwright_cdp::connection::ConnectionConfig;
    use pwright_codec_handshake::HandshakeCodec;
    use pwright_codec_navigation::NavigationCodecLimits;
    use pwright_composition_chromium::{
        AttachedChromiumComposition, CapabilityWireProfile, CdpPageRouteProductionProfile,
        ChromiumAttachFactory, ChromiumAttachProductionProfile, ChromiumRemoteHandshakeProfile,
        NavigationInstallationProfile,
    };
    use pwright_core::{
        ProductionResourceLedger, ProtocolVersion, ResourceClass, ResourceLimits,
        ResourceRequirements,
    };
    use pwright_endpoint_tokio::{AcceptedMutualTlsConnection, AcceptedTailnetConnection};
    #[cfg(unix)]
    use pwright_endpoint_tokio::{PrivateUnixEndpoint, connect_unix_with_server_uid};
    use pwright_provider_cdp::CdpWebSocketEndpoint;
    use pwright_runtime::RuntimeLimits;
    use pwright_server_wire::{AllowUnauthenticated, ServerSupervisorLimits};
    use pwright_transport::{
        CapabilityRequest, ClientHello, FrameLimits, FrameRoute, FrameTransport, NegotiationLimits,
        ProductionFrameTransport, VersionPolicy, WireCodec, WireFrame,
    };
    use pwright_transport_stream::{StreamFrameCodec, StreamFrameLimits};
    use pwright_transport_tokio::{
        TokioStreamTransport, production_resource_requirements_for_codec,
    };
    use tokio::io::{DuplexStream, ReadHalf, WriteHalf};

    use super::{
        AcceptedChromiumEndpoint, ChromiumEndpointAcceptError, ChromiumEndpointAcceptor,
        ChromiumEndpointServer, ChromiumEndpointTransport, attach_authenticated_endpoint,
    };
    #[cfg(unix)]
    use super::{AllowedUnixConnection, AllowedUnixEndpoint};

    #[cfg(unix)]
    static TEST_DIRECTORY_ID: AtomicU64 = AtomicU64::new(1);

    struct TestAcceptedEndpoint {
        transport: TokioStreamTransport<ReadHalf<DuplexStream>, WriteHalf<DuplexStream>>,
    }

    impl AcceptedChromiumEndpoint for TestAcceptedEndpoint {
        type Reader = ReadHalf<DuplexStream>;
        type Writer = WriteHalf<DuplexStream>;
        type Gate = AllowUnauthenticated;

        fn into_transport_and_gate(
            self,
        ) -> (TokioStreamTransport<Self::Reader, Self::Writer>, Self::Gate) {
            (self.transport, AllowUnauthenticated)
        }
    }

    struct RejectingEndpoint;

    impl ChromiumEndpointAcceptor for RejectingEndpoint {
        type Accepted = TestAcceptedEndpoint;
        type Error = io::Error;

        fn accept(
            &self,
        ) -> impl std::future::Future<Output = Result<Self::Accepted, Self::Error>> + Send + '_
        {
            ready(Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "rejected before attachment",
            )))
        }
    }

    fn exact_limits(requirements: pwright_core::ResourceRequirements) -> ResourceLimits {
        let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let required = requirements.class(class);
            limits = limits.with_class(class, required.global, required.per_scope);
        }
        limits
    }

    #[cfg(unix)]
    fn test_directory() -> PathBuf {
        let id = TEST_DIRECTORY_ID.fetch_add(1, Ordering::Relaxed);
        let directory = PathBuf::from("/tmp").join(format!("pct-{}-{id}", std::process::id()));
        fs::create_dir(&directory).expect("unique test directory should be created");
        directory
    }

    fn remote_factory(
        transport_requirements: pwright_core::ResourceRequirements,
    ) -> ChromiumAttachFactory {
        let wire = FrameLimits::new(4096, 0, 0, 4096);
        let connection = CdpConnectionProductionProfile::new(
            pwright_core::ResourceScope::Connection,
            ConnectionConfig::default(),
        );
        let topology = CdpTopologyProductionProfile::new(
            CdpTopologyLimits::new(16, 4, 4, 8, 8, 128),
            pwright_core::ResourceScope::extension(11, 1),
        )
        .expect("topology profile should validate");
        let page_routes = CdpPageRouteProductionProfile::from_topology(
            pwright_core::ResourceScope::extension(11, 2),
            topology,
        );
        let navigation = NavigationInstallationProfile::new(
            CdpNavigationProductionProfile::new(
                pwright_core::ResourceScope::extension(11, 3),
                CdpNavigationLimits::new(1024, 256, 16, 16, 2048, 256),
                2,
            ),
            NavigationCodecLimits::new(2048, 16, 2048),
            CapabilityWireProfile::new(ProtocolVersion::new(1, 0), wire, wire),
        );
        let handshake = ChromiumRemoteHandshakeProfile::new(
            VersionPolicy::exact(ProtocolVersion::new(1, 0)),
            wire,
            NegotiationLimits::new(2, 128),
            4096,
        )
        .with_transport_requirements(transport_requirements);
        let profile = ChromiumAttachProductionProfile::new(
            connection,
            topology,
            page_routes,
            RuntimeLimits::new(16, 4).with_active_commands(2),
            ServerSupervisorLimits::new(2).expect("supervisor limits should validate"),
        )
        .with_navigation(navigation)
        .with_remote_handshake(handshake);
        ChromiumAttachFactory::new(
            profile.clone(),
            exact_limits(profile.requirements()),
            pwright_core::ObjectId::new(1),
        )
        .expect("authenticated remote profile should validate")
    }

    fn client_hello_frame() -> WireFrame {
        let limits = NegotiationLimits::new(2, 128);
        let codec = HandshakeCodec::new(limits);
        let frame_limits = FrameLimits::new(4096, 0, 0, 4096);
        let hello = ClientHello::new(
            VersionPolicy::exact(ProtocolVersion::new(1, 0)),
            frame_limits,
            CapabilityRequest::new(Vec::new(), Vec::new(), limits)
                .expect("empty capability request should validate"),
        );
        let encoded = codec
            .encode_parts(&hello)
            .expect("client hello should encode");
        WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Handshake,
            encoded.payload().to_vec(),
            encoded.attachments().to_vec(),
            frame_limits,
        )
        .expect("client hello should fit")
    }

    #[test]
    fn authenticated_network_endpoints_have_production_transport_adapters() {
        // Arrange
        fn assert_endpoint<Endpoint>()
        where
            Endpoint: AcceptedChromiumEndpoint,
            ChromiumEndpointTransport<Endpoint>: pwright_transport::ProductionFrameTransport,
        {
        }

        // Act
        assert_endpoint::<AcceptedTailnetConnection>();
        assert_endpoint::<AcceptedMutualTlsConnection>();

        // Assert
        // Compilation proves both authenticated endpoint families retain a
        // concrete production transport type without dynamic dispatch.
    }

    #[test]
    fn authenticated_listeners_have_caller_driven_acceptor_adapters() {
        // Arrange
        fn assert_acceptor<Endpoint>()
        where
            Endpoint: ChromiumEndpointAcceptor,
        {
        }

        // Act
        assert_acceptor::<pwright_endpoint_tokio::TailnetEndpoint>();
        assert_acceptor::<pwright_endpoint_tokio::MutualTlsEndpoint<1>>();
        #[cfg(unix)]
        assert_acceptor::<AllowedUnixEndpoint>();

        // Assert
        // Compilation proves each concrete listener retains its own accepted
        // endpoint and error type without dynamic dispatch.
    }

    #[tokio::test]
    async fn endpoint_rejection_happens_before_a_chromium_attachment() {
        // Arrange
        let factory = remote_factory(ResourceRequirements::none());
        let cdp_endpoint =
            CdpWebSocketEndpoint::parse("ws://127.0.0.1:9/devtools/browser/must-not-connect")
                .expect("synthetic CDP endpoint should parse");
        let server = ChromiumEndpointServer::new(factory, cdp_endpoint, RejectingEndpoint);

        // Act
        let result = server.accept_connection().await;

        // Assert
        assert!(matches!(
            result,
            Err(ChromiumEndpointAcceptError::Endpoint(error))
                if error.kind() == io::ErrorKind::PermissionDenied
        ));
    }

    #[test]
    fn accepted_endpoint_transport_claims_the_callers_exact_ledger() {
        // Arrange
        let (stream, _peer) = tokio::io::duplex(512);
        let (reader, writer) = tokio::io::split(stream);
        let codec = StreamFrameCodec::new(StreamFrameLimits::new(
            FrameLimits::new(128, 0, 0, 128),
            256,
        ));
        let raw = TokioStreamTransport::from_parts(reader, writer, codec);
        let requirements = raw
            .production_resource_requirements()
            .expect("bounded stream profile should calculate");
        let resources = ProductionResourceLedger::new(exact_limits(requirements), requirements)
            .expect("exact transport plan should create a production ledger");
        let endpoint = TestAcceptedEndpoint { transport: raw };

        // Act
        let (transport, _gate) = endpoint.into_transport_and_gate();
        let production = transport
            .with_production_resources(&resources)
            .expect("endpoint transport should claim the caller's ledger");

        // Assert
        let proof = production.production_resources();
        assert_eq!(proof.requirements(), requirements);
        let expected = resources
            .claim(requirements)
            .expect("declared transport plan should remain claimable");
        assert!(proof.ledger().shares_budget_with(&expected));
    }

    #[cfg(unix)]
    #[tokio::test]
    #[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
    async fn unix_peer_gate_stays_paired_through_chromium_handshake() {
        // Arrange
        let directory = test_directory();
        let socket_path = directory.join("pwright.sock");
        let uid = fs::metadata(&directory)
            .expect("test directory should have metadata")
            .uid();
        let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(
            FrameLimits::new(4096, 0, 0, 4096),
            8192,
        ));
        let transport_requirements = production_resource_requirements_for_codec(stream_codec)
            .expect("bounded endpoint codec should declare its transport plan");
        let factory = remote_factory(transport_requirements);
        let cdp_endpoint = CdpWebSocketEndpoint::parse(
            std::env::var("PWRIGHT_CDP_WS_URL")
                .expect("PWRIGHT_CDP_WS_URL must identify the dedicated Chrome endpoint"),
        )
        .expect("dedicated Chrome endpoint should parse");
        let endpoint = PrivateUnixEndpoint::bind(&socket_path, stream_codec)
            .expect("private Unix endpoint should bind");
        let (accepted, client) = tokio::join!(
            endpoint.accept(),
            connect_unix_with_server_uid(&socket_path, uid, stream_codec)
        );
        let accepted = accepted.expect("server should accept the local Unix peer");
        let client = client.expect("client should verify the local Unix server");
        let client_transport = client.into_transport();
        client_transport
            .send(client_hello_frame())
            .await
            .expect("client hello should reach the accepted endpoint");
        let seed = attach_authenticated_endpoint(
            &factory,
            cdp_endpoint.clone(),
            AllowedUnixConnection::new(accepted, uid),
        )
        .await
        .expect("authenticated endpoint should stage with the fresh Chromium attachment");
        assert_eq!(seed.gate().peer().uid(), uid);
        let connection = seed
            .install_runtime(|composition: &mut AttachedChromiumComposition| {
                composition.new_page_runtime()
            })
            .expect("runtime should install before accepting the paired handshake");

        // Act
        let accepted = connection
            .accept_handshake()
            .await
            .expect("kernel-derived peer gate should authorize its paired supervisor");

        // Assert
        assert_eq!(accepted.accepted().authorization().uid(), uid);
        assert!(!accepted.composition().connection().is_closed());
        drop(accepted);
        drop(client_transport);
        let reattached = factory
            .attach(cdp_endpoint)
            .await
            .expect("dropping the server connection must not terminate external Chrome");
        assert!(!reattached.connection().is_closed());
        drop(reattached);
        assert!(endpoint.close().expect("owned Unix socket should close"));
        fs::remove_dir(&directory).expect("empty test directory should be removable");
    }

    #[cfg(unix)]
    #[tokio::test]
    #[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
    async fn packaged_unix_listener_accepts_one_fresh_authenticated_connection() {
        // Arrange
        let directory = test_directory();
        let socket_path = directory.join("pwright-server.sock");
        let uid = fs::metadata(&directory)
            .expect("test directory should have metadata")
            .uid();
        let stream_codec = StreamFrameCodec::new(StreamFrameLimits::new(
            FrameLimits::new(4096, 0, 0, 4096),
            8192,
        ));
        let transport_requirements = production_resource_requirements_for_codec(stream_codec)
            .expect("bounded endpoint codec should declare its transport plan");
        let factory = remote_factory(transport_requirements);
        let cdp_endpoint = CdpWebSocketEndpoint::parse(
            std::env::var("PWRIGHT_CDP_WS_URL")
                .expect("PWRIGHT_CDP_WS_URL must identify the dedicated Chrome endpoint"),
        )
        .expect("dedicated Chrome endpoint should parse");
        let endpoint = PrivateUnixEndpoint::bind(&socket_path, stream_codec)
            .expect("private Unix endpoint should bind");
        let server = ChromiumEndpointServer::new(
            factory,
            cdp_endpoint,
            AllowedUnixEndpoint::new(endpoint, uid),
        );
        let (seed, client) = tokio::join!(
            server.accept_connection(),
            connect_unix_with_server_uid(&socket_path, uid, stream_codec)
        );
        let seed = seed.expect("packaged listener should stage a fresh connection");
        let client = client.expect("client should verify the Unix server identity");
        let client_transport = client.into_transport();
        client_transport
            .send(client_hello_frame())
            .await
            .expect("client hello should reach the packaged listener connection");

        // Act
        let accepted = seed
            .install_runtime(|composition: &mut AttachedChromiumComposition| {
                composition.new_page_runtime()
            })
            .expect("runtime should install")
            .accept_handshake()
            .await
            .expect("endpoint-derived UID gate should authorize the same connection");

        // Assert
        assert_eq!(accepted.accepted().authorization().uid(), uid);
        assert!(!accepted.composition().connection().is_closed());
        drop(accepted);
        drop(client_transport);
        let (factory, cdp_endpoint, endpoint) = server.into_parts();
        let reattached = factory
            .attach(cdp_endpoint)
            .await
            .expect("external Chrome should remain attachable after remote connection drop");
        assert!(!reattached.connection().is_closed());
        drop(reattached);
        assert!(endpoint.close().expect("owned Unix socket should close"));
        fs::remove_dir(&directory).expect("empty test directory should be removable");
    }
}
