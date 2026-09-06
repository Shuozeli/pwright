use std::convert::Infallible;
use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::future::{Ready, ready};
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use pwright_server_wire::ServerHandshakeGate;
use pwright_transport::ClientHello;
use pwright_transport_stream::{STREAM_LENGTH_PREFIX_BYTES, StreamFrameCodec};
use pwright_transport_tokio::TokioStreamTransport;
use ring::digest::{SHA256, digest};
use tokio::io::{ReadHalf, WriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tokio_rustls::rustls::pki_types::{CertificateDer, ServerName};
use tokio_rustls::rustls::{ClientConfig, ServerConfig};
use tokio_rustls::{TlsAcceptor, TlsConnector};

pub const PWRIGHT_TLS_ALPN: &[u8] = b"pwright/1";
/// rustls 0.23's defensive cap for one buffered handshake message.
pub const RUSTLS_MAX_HANDSHAKE_BUFFER_BYTES: usize = 0xffff;
/// rustls 0.23's defensive cap for one peer-controlled certificate.
pub const RUSTLS_MAX_CERTIFICATE_BYTES: usize = 0x1_0000;

type ServerTlsStream = tokio_rustls::server::TlsStream<TcpStream>;
type ClientTlsStream = tokio_rustls::client::TlsStream<TcpStream>;

pub type MutualTlsServerFrameTransport =
    TokioStreamTransport<ReadHalf<ServerTlsStream>, WriteHalf<ServerTlsStream>>;
pub type MutualTlsClientFrameTransport =
    TokioStreamTransport<ReadHalf<ClientTlsStream>, WriteHalf<ClientTlsStream>>;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CertificateFingerprint([u8; 32]);

impl CertificateFingerprint {
    pub const fn new(sha256: [u8; 32]) -> Self {
        Self(sha256)
    }

    pub fn for_der(certificate: &[u8]) -> Self {
        let hash = digest(&SHA256, certificate);
        let mut fingerprint = [0u8; 32];
        fingerprint.copy_from_slice(hash.as_ref());
        Self(fingerprint)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Debug for CertificateFingerprint {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "CertificateFingerprint({self})")
    }
}

impl Display for CertificateFingerprint {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CertificatePinsError {
    Empty,
    Duplicate { first: usize, second: usize },
}

impl Display for CertificatePinsError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("at least one certificate pin is required"),
            Self::Duplicate { first, second } => write!(
                formatter,
                "certificate pin entries {first} and {second} are duplicates"
            ),
        }
    }
}

impl Error for CertificatePinsError {}

/// Compile-time-capacity pin set. Multiple entries permit bounded certificate
/// rotation without a dynamic allowlist or a temporary authorization bypass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CertificatePins<const N: usize> {
    pins: [CertificateFingerprint; N],
}

impl<const N: usize> CertificatePins<N> {
    pub fn new(pins: [CertificateFingerprint; N]) -> Result<Self, CertificatePinsError> {
        if N == 0 {
            return Err(CertificatePinsError::Empty);
        }
        for first in 0..N {
            for second in (first + 1)..N {
                if pins[first] == pins[second] {
                    return Err(CertificatePinsError::Duplicate { first, second });
                }
            }
        }
        Ok(Self { pins })
    }

    pub const fn len(&self) -> usize {
        N
    }

    pub const fn is_empty(&self) -> bool {
        N == 0
    }

    pub fn contains(&self, fingerprint: CertificateFingerprint) -> bool {
        self.pins.contains(&fingerprint)
    }

    pub const fn as_slice(&self) -> &[CertificateFingerprint] {
        &self.pins
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MutualTlsAddressRole {
    Bind,
    Server,
}

impl Display for MutualTlsAddressRole {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Bind => "bind",
            Self::Server => "server",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MutualTlsPeerRole {
    Client,
    Server,
}

impl Display for MutualTlsPeerRole {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Client => "client",
            Self::Server => "server",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MutualTlsEndpointOperation {
    Bind,
    InspectLocalAddress,
    Accept,
    Connect,
    InspectPeerAddress,
    TlsAccept,
    TlsConnect,
}

impl Display for MutualTlsEndpointOperation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Bind => "bind mTLS TCP endpoint",
            Self::InspectLocalAddress => "inspect mTLS TCP local address",
            Self::Accept => "accept mTLS TCP connection",
            Self::Connect => "connect mTLS TCP endpoint",
            Self::InspectPeerAddress => "inspect mTLS TCP peer address",
            Self::TlsAccept => "authenticate mTLS client",
            Self::TlsConnect => "authenticate mTLS server",
        })
    }
}

#[derive(Debug)]
pub enum MutualTlsAuthenticationError {
    Io {
        operation: MutualTlsEndpointOperation,
        source: io::Error,
    },
    UnspecifiedAddress {
        role: MutualTlsAddressRole,
        address: SocketAddr,
    },
    ZeroHandshakeLimit,
    PwrightAlpnNotConfigured {
        role: MutualTlsPeerRole,
    },
    PwrightAlpnNotNegotiated {
        role: MutualTlsPeerRole,
    },
    MissingPeerCertificate {
        role: MutualTlsPeerRole,
    },
    CertificatePinRejected {
        role: MutualTlsPeerRole,
        actual: CertificateFingerprint,
    },
}

impl MutualTlsAuthenticationError {
    fn io(operation: MutualTlsEndpointOperation, source: io::Error) -> Self {
        Self::Io { operation, source }
    }
}

impl Display for MutualTlsAuthenticationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { operation, source } => write!(formatter, "could not {operation}: {source}"),
            Self::UnspecifiedAddress { role, address } => {
                write!(
                    formatter,
                    "mTLS {role} address `{address}` must name one explicit interface"
                )
            }
            Self::ZeroHandshakeLimit => {
                formatter.write_str("mTLS concurrent handshake limit must be greater than zero")
            }
            Self::PwrightAlpnNotConfigured { role } => {
                write!(
                    formatter,
                    "mTLS {role} config does not offer the pwright/1 ALPN"
                )
            }
            Self::PwrightAlpnNotNegotiated { role } => {
                write!(
                    formatter,
                    "mTLS {role} did not negotiate the pwright/1 ALPN"
                )
            }
            Self::MissingPeerCertificate { role } => {
                write!(formatter, "mTLS {role} did not present a certificate")
            }
            Self::CertificatePinRejected { role, actual } => {
                write!(
                    formatter,
                    "mTLS {role} certificate pin `{actual}` is not authorized"
                )
            }
        }
    }
}

impl Error for MutualTlsAuthenticationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::UnspecifiedAddress { .. }
            | Self::ZeroHandshakeLimit
            | Self::PwrightAlpnNotConfigured { .. }
            | Self::PwrightAlpnNotNegotiated { .. }
            | Self::MissingPeerCertificate { .. }
            | Self::CertificatePinRejected { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MutualTlsPeerIdentity {
    address: SocketAddr,
    certificate: CertificateFingerprint,
}

impl MutualTlsPeerIdentity {
    pub const fn address(self) -> SocketAddr {
        self.address
    }

    pub const fn certificate(self) -> CertificateFingerprint {
        self.certificate
    }
}

/// Proof-carrying Pwright handshake gate. Its constructor is private to the
/// endpoint's successful TLS-and-pin path.
pub struct MutualTlsPeerGate {
    peer: MutualTlsPeerIdentity,
}

impl MutualTlsPeerGate {
    pub const fn peer(&self) -> MutualTlsPeerIdentity {
        self.peer
    }
}

impl ServerHandshakeGate for MutualTlsPeerGate {
    type Authorization = MutualTlsPeerIdentity;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Authorization, Self::Error>>;

    fn authorize<'a>(&'a mut self, _hello: &'a ClientHello) -> Self::Future<'a> {
        ready(Ok(self.peer))
    }
}

pub struct AcceptedMutualTlsConnection {
    transport: MutualTlsServerFrameTransport,
    peer: MutualTlsPeerIdentity,
}

impl AcceptedMutualTlsConnection {
    pub const fn peer(&self) -> MutualTlsPeerIdentity {
        self.peer
    }

    pub fn into_parts(self) -> (MutualTlsServerFrameTransport, MutualTlsPeerIdentity) {
        (self.transport, self.peer)
    }

    pub fn into_peer_gate(self) -> (MutualTlsServerFrameTransport, MutualTlsPeerGate) {
        (self.transport, MutualTlsPeerGate { peer: self.peer })
    }
}

pub struct ConnectedMutualTlsClient {
    transport: MutualTlsClientFrameTransport,
    server: MutualTlsPeerIdentity,
}

impl ConnectedMutualTlsClient {
    pub const fn server(&self) -> MutualTlsPeerIdentity {
        self.server
    }

    pub fn into_parts(self) -> (MutualTlsClientFrameTransport, MutualTlsPeerIdentity) {
        (self.transport, self.server)
    }

    pub fn into_transport(self) -> MutualTlsClientFrameTransport {
        self.transport
    }
}

/// Caller-driven mTLS TCP listener with an internal bound on simultaneous TLS
/// handshakes. Established frame transports are owned and bounded separately.
pub struct MutualTlsEndpoint<const CLIENT_PINS: usize> {
    listener: TcpListener,
    acceptor: TlsAcceptor,
    allowed_clients: CertificatePins<CLIENT_PINS>,
    codec: StreamFrameCodec,
    local_address: SocketAddr,
    handshake_slots: Semaphore,
    handshake_limit: usize,
    tls_buffer_limit: usize,
}

impl<const CLIENT_PINS: usize> MutualTlsEndpoint<CLIENT_PINS> {
    pub async fn bind(
        address: SocketAddr,
        server_config: Arc<ServerConfig>,
        allowed_clients: CertificatePins<CLIENT_PINS>,
        handshake_limit: usize,
        codec: StreamFrameCodec,
    ) -> Result<Self, MutualTlsAuthenticationError> {
        validate_explicit_address(MutualTlsAddressRole::Bind, address)?;
        if handshake_limit == 0 {
            return Err(MutualTlsAuthenticationError::ZeroHandshakeLimit);
        }
        validate_alpn_configuration(MutualTlsPeerRole::Server, &server_config.alpn_protocols)?;
        let tls_buffer_limit = tls_buffer_limit(codec);
        let listener = TcpListener::bind(address).await.map_err(|error| {
            MutualTlsAuthenticationError::io(MutualTlsEndpointOperation::Bind, error)
        })?;
        let local_address = listener.local_addr().map_err(|error| {
            MutualTlsAuthenticationError::io(MutualTlsEndpointOperation::InspectLocalAddress, error)
        })?;
        validate_explicit_address(MutualTlsAddressRole::Bind, local_address)?;
        Ok(Self {
            listener,
            acceptor: TlsAcceptor::from(server_config),
            allowed_clients,
            codec,
            local_address,
            handshake_slots: Semaphore::new(handshake_limit),
            handshake_limit,
            tls_buffer_limit,
        })
    }

    pub const fn local_address(&self) -> SocketAddr {
        self.local_address
    }

    pub const fn handshake_limit(&self) -> usize {
        self.handshake_limit
    }

    pub fn active_handshakes(&self) -> usize {
        self.handshake_limit
            .saturating_sub(self.handshake_slots.available_permits())
    }

    pub const fn tls_buffer_limit(&self) -> usize {
        self.tls_buffer_limit
    }

    pub async fn accept(
        &self,
    ) -> Result<AcceptedMutualTlsConnection, MutualTlsAuthenticationError> {
        let permit = self
            .handshake_slots
            .acquire()
            .await
            .map_err(|_| MutualTlsAuthenticationError::ZeroHandshakeLimit)?;
        let (stream, peer_address) = self.listener.accept().await.map_err(|error| {
            MutualTlsAuthenticationError::io(MutualTlsEndpointOperation::Accept, error)
        })?;
        let (tls, identity) = authenticate_server_stream(
            stream,
            peer_address,
            &self.acceptor,
            &self.allowed_clients,
            self.tls_buffer_limit,
        )
        .await?;
        drop(permit);
        Ok(AcceptedMutualTlsConnection {
            transport: TokioStreamTransport::from_io(tls, self.codec),
            peer: identity,
        })
    }
}

pub async fn connect_mutual_tls<const SERVER_PINS: usize>(
    server_address: SocketAddr,
    server_name: ServerName<'static>,
    client_config: Arc<ClientConfig>,
    allowed_servers: &CertificatePins<SERVER_PINS>,
    codec: StreamFrameCodec,
) -> Result<ConnectedMutualTlsClient, MutualTlsAuthenticationError> {
    validate_explicit_address(MutualTlsAddressRole::Server, server_address)?;
    validate_alpn_configuration(MutualTlsPeerRole::Client, &client_config.alpn_protocols)?;
    let stream = TcpStream::connect(server_address).await.map_err(|error| {
        MutualTlsAuthenticationError::io(MutualTlsEndpointOperation::Connect, error)
    })?;
    let peer_address = stream.peer_addr().map_err(|error| {
        MutualTlsAuthenticationError::io(MutualTlsEndpointOperation::InspectPeerAddress, error)
    })?;
    let connector = TlsConnector::from(client_config);
    let buffer_limit = tls_buffer_limit(codec);
    let (tls, identity) = authenticate_client_stream(
        stream,
        peer_address,
        server_name,
        &connector,
        allowed_servers,
        buffer_limit,
    )
    .await?;
    Ok(ConnectedMutualTlsClient {
        transport: TokioStreamTransport::from_io(tls, codec),
        server: identity,
    })
}

fn validate_explicit_address(
    role: MutualTlsAddressRole,
    address: SocketAddr,
) -> Result<(), MutualTlsAuthenticationError> {
    if address.ip().is_unspecified() {
        Err(MutualTlsAuthenticationError::UnspecifiedAddress { role, address })
    } else {
        Ok(())
    }
}

fn validate_alpn_configuration(
    role: MutualTlsPeerRole,
    protocols: &[Vec<u8>],
) -> Result<(), MutualTlsAuthenticationError> {
    if protocols
        .iter()
        .any(|protocol| protocol == PWRIGHT_TLS_ALPN)
    {
        Ok(())
    } else {
        Err(MutualTlsAuthenticationError::PwrightAlpnNotConfigured { role })
    }
}

fn tls_buffer_limit(codec: StreamFrameCodec) -> usize {
    RUSTLS_MAX_CERTIFICATE_BYTES.max(
        codec
            .limits()
            .envelope_bytes()
            .saturating_add(STREAM_LENGTH_PREFIX_BYTES),
    )
}

async fn authenticate_server_stream<Io, const PINS: usize>(
    stream: Io,
    peer_address: SocketAddr,
    acceptor: &TlsAcceptor,
    allowed_clients: &CertificatePins<PINS>,
    buffer_limit: usize,
) -> Result<
    (tokio_rustls::server::TlsStream<Io>, MutualTlsPeerIdentity),
    MutualTlsAuthenticationError,
>
where
    Io: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let tls = acceptor
        .accept_with(stream, |connection| {
            connection.set_buffer_limit(Some(buffer_limit));
        })
        .await
        .map_err(|error| {
            MutualTlsAuthenticationError::io(MutualTlsEndpointOperation::TlsAccept, error)
        })?;
    let connection = tls.get_ref().1;
    validate_negotiated_alpn(MutualTlsPeerRole::Client, connection.alpn_protocol())?;
    let fingerprint = peer_fingerprint(MutualTlsPeerRole::Client, connection.peer_certificates())?;
    validate_pin(MutualTlsPeerRole::Client, fingerprint, allowed_clients)?;
    Ok((
        tls,
        MutualTlsPeerIdentity {
            address: peer_address,
            certificate: fingerprint,
        },
    ))
}

async fn authenticate_client_stream<Io, const PINS: usize>(
    stream: Io,
    peer_address: SocketAddr,
    server_name: ServerName<'static>,
    connector: &TlsConnector,
    allowed_servers: &CertificatePins<PINS>,
    buffer_limit: usize,
) -> Result<
    (tokio_rustls::client::TlsStream<Io>, MutualTlsPeerIdentity),
    MutualTlsAuthenticationError,
>
where
    Io: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let tls = connector
        .connect_with(server_name, stream, |connection| {
            connection.set_buffer_limit(Some(buffer_limit));
        })
        .await
        .map_err(|error| {
            MutualTlsAuthenticationError::io(MutualTlsEndpointOperation::TlsConnect, error)
        })?;
    let connection = tls.get_ref().1;
    validate_negotiated_alpn(MutualTlsPeerRole::Server, connection.alpn_protocol())?;
    let fingerprint = peer_fingerprint(MutualTlsPeerRole::Server, connection.peer_certificates())?;
    validate_pin(MutualTlsPeerRole::Server, fingerprint, allowed_servers)?;
    Ok((
        tls,
        MutualTlsPeerIdentity {
            address: peer_address,
            certificate: fingerprint,
        },
    ))
}

fn validate_negotiated_alpn(
    role: MutualTlsPeerRole,
    negotiated: Option<&[u8]>,
) -> Result<(), MutualTlsAuthenticationError> {
    if negotiated == Some(PWRIGHT_TLS_ALPN) {
        Ok(())
    } else {
        Err(MutualTlsAuthenticationError::PwrightAlpnNotNegotiated { role })
    }
}

fn peer_fingerprint(
    role: MutualTlsPeerRole,
    certificates: Option<&[CertificateDer<'static>]>,
) -> Result<CertificateFingerprint, MutualTlsAuthenticationError> {
    certificates
        .and_then(|certificates| certificates.first())
        .map(|certificate| CertificateFingerprint::for_der(certificate.as_ref()))
        .ok_or(MutualTlsAuthenticationError::MissingPeerCertificate { role })
}

fn validate_pin<const PINS: usize>(
    role: MutualTlsPeerRole,
    fingerprint: CertificateFingerprint,
    allowed: &CertificatePins<PINS>,
) -> Result<(), MutualTlsAuthenticationError> {
    if allowed.contains(fingerprint) {
        Ok(())
    } else {
        Err(MutualTlsAuthenticationError::CertificatePinRejected {
            role,
            actual: fingerprint,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::sync::Arc;
    use std::task::{Context, Poll, Waker};

    use pwright_client_wire::EncodedClientHandshake;
    use pwright_codec_handshake::HandshakeCodec;
    use pwright_core::ProtocolVersion;
    use pwright_server_wire::{EncodedServerSupervisor, ServerSupervisorLimits};
    use pwright_transport::{
        CapabilityManifest, CapabilityRequest, ClientHello, FrameLimits, FrameRoute,
        FrameTransport, NegotiationLimits, ServerHandshakeConfig, VersionPolicy, WireFrame,
        WireProtocol,
    };
    use pwright_transport_stream::{StreamFrameCodec, StreamFrameLimits};
    use pwright_transport_tokio::TokioStreamTransport;
    use rcgen::{
        BasicConstraints, CertificateParams, ExtendedKeyUsagePurpose, IsCa, KeyPair,
        KeyUsagePurpose,
    };
    use tokio::io::duplex;
    use tokio_rustls::rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
    use tokio_rustls::rustls::server::WebPkiClientVerifier;
    use tokio_rustls::rustls::{ClientConfig, RootCertStore, ServerConfig};
    use tokio_rustls::{TlsAcceptor, TlsConnector};

    use super::{
        CertificateFingerprint, CertificatePins, CertificatePinsError, MutualTlsAddressRole,
        MutualTlsAuthenticationError, MutualTlsEndpoint, MutualTlsPeerRole, PWRIGHT_TLS_ALPN,
        RUSTLS_MAX_CERTIFICATE_BYTES, RUSTLS_MAX_HANDSHAKE_BUFFER_BYTES,
        authenticate_client_stream, authenticate_server_stream, connect_mutual_tls,
        tls_buffer_limit, validate_negotiated_alpn,
    };

    const TEST_SERVER_NAME: &str = "pwright.test";

    struct TestPki {
        server: Arc<ServerConfig>,
        client: Arc<ClientConfig>,
        server_fingerprint: CertificateFingerprint,
        client_fingerprint: Option<CertificateFingerprint>,
    }

    fn test_pki(client_certificate: bool, alpn: bool) -> TestPki {
        let mut ca_params = CertificateParams::new(Vec::new()).expect("CA params should build");
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        ca_params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::CrlSign,
        ];
        let ca_key = KeyPair::generate().expect("CA key should generate");
        let ca = ca_params.self_signed(&ca_key).expect("CA should self-sign");

        let server_key = KeyPair::generate().expect("server key should generate");
        let mut server_params = CertificateParams::new(vec![TEST_SERVER_NAME.to_owned()])
            .expect("server params should build");
        server_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let server_certificate = server_params
            .signed_by(&server_key, &ca, &ca_key)
            .expect("server certificate should sign");

        let client_key = KeyPair::generate().expect("client key should generate");
        let mut client_params = CertificateParams::new(vec!["pwright-client.test".to_owned()])
            .expect("client params should build");
        client_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
        let issued_client_certificate = client_params
            .signed_by(&client_key, &ca, &ca_key)
            .expect("client certificate should sign");

        let mut roots = RootCertStore::empty();
        roots
            .add(ca.der().clone())
            .expect("CA should enter root store");
        let server_builder = ServerConfig::builder();
        let mut server = if client_certificate {
            let verifier = WebPkiClientVerifier::builder(Arc::new(roots.clone()))
                .build()
                .expect("client verifier should build");
            server_builder.with_client_cert_verifier(verifier)
        } else {
            server_builder.with_no_client_auth()
        }
        .with_single_cert(
            vec![server_certificate.der().clone()],
            private_key(&server_key),
        )
        .expect("server TLS config should build");

        let client_builder = ClientConfig::builder().with_root_certificates(roots);
        let mut client = if client_certificate {
            client_builder
                .with_client_auth_cert(
                    vec![issued_client_certificate.der().clone()],
                    private_key(&client_key),
                )
                .expect("client TLS config should build")
        } else {
            client_builder.with_no_client_auth()
        };
        if alpn {
            server.alpn_protocols = vec![PWRIGHT_TLS_ALPN.to_vec()];
            client.alpn_protocols = vec![PWRIGHT_TLS_ALPN.to_vec()];
        }

        TestPki {
            server: Arc::new(server),
            client: Arc::new(client),
            server_fingerprint: CertificateFingerprint::for_der(server_certificate.der().as_ref()),
            client_fingerprint: client_certificate
                .then(|| CertificateFingerprint::for_der(issued_client_certificate.der().as_ref())),
        }
    }

    fn private_key(key: &KeyPair) -> PrivateKeyDer<'static> {
        PrivatePkcs8KeyDer::from(key.serialize_der()).into()
    }

    fn codec() -> StreamFrameCodec {
        StreamFrameCodec::new(StreamFrameLimits::new(
            FrameLimits::new(512, 0, 512, 512),
            1024,
        ))
    }

    fn pins<const N: usize>(values: [CertificateFingerprint; N]) -> CertificatePins<N> {
        CertificatePins::new(values).expect("test pins should be unique and nonempty")
    }

    fn server_name() -> ServerName<'static> {
        ServerName::try_from(TEST_SERVER_NAME)
            .expect("test server name should be valid")
            .to_owned()
    }

    #[test]
    fn pin_set_is_nonempty_unique_and_rotation_bounded() {
        // Arrange
        let first = CertificateFingerprint::new([1; 32]);
        let second = CertificateFingerprint::new([2; 32]);

        // Act
        let empty = CertificatePins::<0>::new([]);
        let duplicate = CertificatePins::new([first, first]);
        let rotating =
            CertificatePins::new([first, second]).expect("two distinct rotation pins should build");

        // Assert
        assert_eq!(empty, Err(CertificatePinsError::Empty));
        assert_eq!(
            duplicate,
            Err(CertificatePinsError::Duplicate {
                first: 0,
                second: 1,
            })
        );
        assert_eq!(rotating.len(), 2);
        assert!(rotating.contains(first));
        assert!(rotating.contains(second));
        assert_eq!(RUSTLS_MAX_HANDSHAKE_BUFFER_BYTES, 65_535);
    }

    #[test]
    fn negotiated_alpn_must_be_exactly_the_pwright_protocol() {
        // Arrange
        let absent = None;
        let other = Some(b"http/1.1".as_slice());
        let pwright = Some(PWRIGHT_TLS_ALPN);

        // Act
        let absent_result = validate_negotiated_alpn(MutualTlsPeerRole::Server, absent);
        let other_result = validate_negotiated_alpn(MutualTlsPeerRole::Server, other);
        let pwright_result = validate_negotiated_alpn(MutualTlsPeerRole::Server, pwright);

        // Assert
        assert!(matches!(
            absent_result,
            Err(MutualTlsAuthenticationError::PwrightAlpnNotNegotiated {
                role: MutualTlsPeerRole::Server,
            })
        ));
        assert!(matches!(
            other_result,
            Err(MutualTlsAuthenticationError::PwrightAlpnNotNegotiated {
                role: MutualTlsPeerRole::Server,
            })
        ));
        assert!(pwright_result.is_ok());
    }

    #[test]
    fn tls_output_buffer_covers_rustls_and_frame_envelopes() {
        // Arrange
        let small_codec = codec();
        let large_envelope = RUSTLS_MAX_CERTIFICATE_BYTES + 128;
        let large_codec = StreamFrameCodec::new(StreamFrameLimits::new(
            FrameLimits::new(512, 0, 512, 512),
            large_envelope,
        ));

        // Act
        let small_limit = tls_buffer_limit(small_codec);
        let large_limit = tls_buffer_limit(large_codec);

        // Assert
        assert_eq!(small_limit, RUSTLS_MAX_CERTIFICATE_BYTES);
        assert_eq!(
            large_limit,
            large_envelope + pwright_transport_stream::STREAM_LENGTH_PREFIX_BYTES
        );
    }

    #[tokio::test]
    async fn mutual_tls_authentication_preserves_following_frame_boundary() {
        // Arrange
        let pki = test_pki(true, true);
        let client_fingerprint = pki
            .client_fingerprint
            .expect("mTLS fixture should have client certificate");
        let client_pins = pins([client_fingerprint]);
        let server_pins = pins([pki.server_fingerprint]);
        let acceptor = TlsAcceptor::from(Arc::clone(&pki.server));
        let connector = TlsConnector::from(Arc::clone(&pki.client));
        let (client_io, server_io) = duplex(4096);
        let client_address = SocketAddr::from((Ipv4Addr::LOCALHOST, 41001));
        let server_address = SocketAddr::from((Ipv4Addr::LOCALHOST, 41002));

        // Act
        let (server, client) = tokio::join!(
            authenticate_server_stream(
                server_io,
                client_address,
                &acceptor,
                &client_pins,
                RUSTLS_MAX_CERTIFICATE_BYTES,
            ),
            authenticate_client_stream(
                client_io,
                server_address,
                server_name(),
                &connector,
                &server_pins,
                RUSTLS_MAX_CERTIFICATE_BYTES,
            ),
        );
        let (server_tls, server_identity) = server.expect("server should authenticate client");
        let (client_tls, client_identity) = client.expect("client should authenticate server");
        let server_transport = TokioStreamTransport::from_io(server_tls, codec());
        let client_transport = TokioStreamTransport::from_io(client_tls, codec());
        let frame = WireFrame::new(
            ProtocolVersion::new(4, 1),
            FrameRoute::Control,
            vec![3, 1, 4],
            Vec::new(),
            FrameLimits::new(512, 0, 512, 512),
        )
        .expect("test frame should fit");
        client_transport
            .send(frame.clone())
            .await
            .expect("TLS frame should send");
        let received = server_transport
            .receive()
            .await
            .expect("TLS frame should receive")
            .expect("TLS frame should exist");

        // Assert
        assert_eq!(server_identity.certificate(), client_fingerprint);
        assert_eq!(client_identity.certificate(), pki.server_fingerprint);
        assert_eq!(received, frame);
    }

    #[tokio::test]
    async fn valid_ca_certificate_is_still_rejected_when_not_pinned() {
        // Arrange
        let pki = test_pki(true, true);
        let client_pins = pins([pki
            .client_fingerprint
            .expect("mTLS fixture should have client certificate")]);
        let wrong_server_pins = pins([CertificateFingerprint::new([9; 32])]);
        let acceptor = TlsAcceptor::from(Arc::clone(&pki.server));
        let connector = TlsConnector::from(Arc::clone(&pki.client));
        let (client_io, server_io) = duplex(4096);

        // Act
        let (_server, client) = tokio::join!(
            authenticate_server_stream(
                server_io,
                SocketAddr::from((Ipv4Addr::LOCALHOST, 42001)),
                &acceptor,
                &client_pins,
                RUSTLS_MAX_CERTIFICATE_BYTES,
            ),
            authenticate_client_stream(
                client_io,
                SocketAddr::from((Ipv4Addr::LOCALHOST, 42002)),
                server_name(),
                &connector,
                &wrong_server_pins,
                RUSTLS_MAX_CERTIFICATE_BYTES,
            ),
        );

        // Assert
        assert!(matches!(
            client,
            Err(MutualTlsAuthenticationError::CertificatePinRejected {
                role: MutualTlsPeerRole::Server,
                actual,
            }) if actual == pki.server_fingerprint
        ));
    }

    #[tokio::test]
    async fn tls_without_a_client_certificate_never_exposes_server_transport() {
        // Arrange
        let pki = test_pki(false, true);
        let placeholder_client_pins = pins([CertificateFingerprint::new([7; 32])]);
        let server_pins = pins([pki.server_fingerprint]);
        let acceptor = TlsAcceptor::from(Arc::clone(&pki.server));
        let connector = TlsConnector::from(Arc::clone(&pki.client));
        let (client_io, server_io) = duplex(4096);

        // Act
        let (server, client) = tokio::join!(
            authenticate_server_stream(
                server_io,
                SocketAddr::from((Ipv4Addr::LOCALHOST, 43001)),
                &acceptor,
                &placeholder_client_pins,
                RUSTLS_MAX_CERTIFICATE_BYTES,
            ),
            authenticate_client_stream(
                client_io,
                SocketAddr::from((Ipv4Addr::LOCALHOST, 43002)),
                server_name(),
                &connector,
                &server_pins,
                RUSTLS_MAX_CERTIFICATE_BYTES,
            ),
        );

        // Assert
        assert!(matches!(
            server,
            Err(MutualTlsAuthenticationError::MissingPeerCertificate {
                role: MutualTlsPeerRole::Client,
            })
        ));
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn handshake_slots_bound_pending_accepts_and_release_on_cancellation() {
        // Arrange
        let pki = test_pki(true, true);
        let client_pins = pins([pki
            .client_fingerprint
            .expect("mTLS fixture should have client certificate")]);
        let endpoint = MutualTlsEndpoint::bind(
            SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            Arc::clone(&pki.server),
            client_pins,
            1,
            codec(),
        )
        .await
        .expect("loopback test endpoint should bind");
        let mut first = Box::pin(endpoint.accept());
        let mut second = Box::pin(endpoint.accept());
        let mut context = Context::from_waker(Waker::noop());

        // Act
        let first_poll = first.as_mut().poll(&mut context);
        let second_poll = second.as_mut().poll(&mut context);

        // Assert
        assert!(matches!(first_poll, Poll::Pending));
        assert!(matches!(second_poll, Poll::Pending));
        assert_eq!(endpoint.active_handshakes(), 1);

        drop(first);
        assert_eq!(
            endpoint.active_handshakes(),
            1,
            "the released slot is transferred directly to the queued accept"
        );
        assert!(matches!(second.as_mut().poll(&mut context), Poll::Pending));
        assert_eq!(endpoint.active_handshakes(), 1);

        drop(second);
        assert_eq!(endpoint.active_handshakes(), 0);
    }

    #[tokio::test]
    async fn wildcard_and_unbounded_endpoint_configs_fail_before_bind() {
        // Arrange
        let pki = test_pki(true, true);
        let client_pins = pins([pki
            .client_fingerprint
            .expect("mTLS fixture should have client certificate")]);
        let wildcard = SocketAddr::from((Ipv4Addr::UNSPECIFIED, 0));

        // Act
        let wildcard_result =
            MutualTlsEndpoint::bind(wildcard, Arc::clone(&pki.server), client_pins, 1, codec())
                .await;
        let zero_result = MutualTlsEndpoint::bind(
            SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
            Arc::clone(&pki.server),
            client_pins,
            0,
            codec(),
        )
        .await;

        // Assert
        assert!(matches!(
            wildcard_result,
            Err(MutualTlsAuthenticationError::UnspecifiedAddress {
                role: MutualTlsAddressRole::Bind,
                address,
            }) if address == wildcard
        ));
        assert!(matches!(
            zero_result,
            Err(MutualTlsAuthenticationError::ZeroHandshakeLimit)
        ));
    }

    #[tokio::test]
    #[ignore = "requires TAILSCALE_IP to name a local interface"]
    async fn live_mtls_tcp_authenticates_before_pwright_handshake() {
        // Arrange
        let local_ip = std::env::var("TAILSCALE_IP")
            .expect("TAILSCALE_IP should name the test interface")
            .parse::<IpAddr>()
            .expect("TAILSCALE_IP should be an IP address");
        let pki = test_pki(true, true);
        let client_fingerprint = pki
            .client_fingerprint
            .expect("mTLS fixture should have client certificate");
        let endpoint = MutualTlsEndpoint::bind(
            SocketAddr::new(local_ip, 0),
            Arc::clone(&pki.server),
            pins([client_fingerprint]),
            1,
            codec(),
        )
        .await
        .expect("mTLS endpoint should bind explicit Tailscale interface");
        let address = endpoint.local_address();
        let server_pins = pins([pki.server_fingerprint]);

        // Act
        let (accepted, connected) = tokio::join!(
            endpoint.accept(),
            connect_mutual_tls(
                address,
                server_name(),
                Arc::clone(&pki.client),
                &server_pins,
                codec(),
            ),
        );
        let accepted = accepted.expect("server should authenticate pinned client");
        let connected = connected.expect("client should authenticate pinned server");
        let (server_transport, mut gate) = accepted.into_peer_gate();
        let expected_peer = gate.peer();
        let handshake_limits = NegotiationLimits::new(0, 0);
        let frame_limits = FrameLimits::new(512, 0, 0, 512);
        let handshake_codec = HandshakeCodec::new(handshake_limits);
        let hello = ClientHello::new(
            VersionPolicy::exact(ProtocolVersion::new(1, 0)),
            frame_limits,
            CapabilityRequest::new(Vec::new(), Vec::new(), handshake_limits)
                .expect("empty capability request should build"),
        );
        let config = ServerHandshakeConfig::new(
            VersionPolicy::exact(ProtocolVersion::new(1, 0)),
            frame_limits,
            CapabilityManifest::new(Vec::new(), handshake_limits)
                .expect("empty capability manifest should build"),
        );
        let client_protocol = WireProtocol::new_exact_directional(
            handshake_codec,
            ProtocolVersion::new(1, 0),
            frame_limits,
            frame_limits,
        );
        let client_handshake = EncodedClientHandshake::new(connected.into_transport());
        let mut supervisor = EncodedServerSupervisor::new_handshake_required(
            server_transport,
            ServerSupervisorLimits::new(1).expect("supervisor limit should be valid"),
        );
        let (client_result, server_result) = tokio::join!(
            client_handshake.establish(client_protocol, hello),
            supervisor.accept_handshake(&handshake_codec, &config, &mut gate),
        );

        // Assert
        assert_eq!(
            server_result
                .expect("mTLS-authorized server handshake should succeed")
                .authorization(),
            &expected_peer
        );
        assert_eq!(
            client_result
                .expect("mTLS client should accept server hello")
                .hello()
                .selected_version(),
            ProtocolVersion::new(1, 0)
        );
        assert_eq!(endpoint.active_handshakes(), 0);
    }
}
