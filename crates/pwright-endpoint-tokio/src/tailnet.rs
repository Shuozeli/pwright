use std::convert::Infallible;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::{Ready, ready};
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use pwright_server_wire::ServerHandshakeGate;
use pwright_transport::ClientHello;
use pwright_transport_stream::StreamFrameCodec;
use pwright_transport_tokio::TokioStreamTransport;
use ring::hmac;
use ring::rand::{SecureRandom, SystemRandom};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::{TcpListener, TcpStream};
use zeroize::Zeroizing;

const AUTH_MAGIC_AND_VERSION: &[u8; 9] = b"PWRTAUTH\x01";
pub const TAILNET_PSK_BYTES: usize = 32;
pub const TAILNET_NONCE_BYTES: usize = 32;
pub const TAILNET_PROOF_BYTES: usize = 32;
pub const TAILNET_AUTH_TRANSCRIPT_BYTES: usize =
    AUTH_MAGIC_AND_VERSION.len() + TAILNET_NONCE_BYTES + (2 * TAILNET_PROOF_BYTES);
const CLIENT_PROOF_LABEL: &[u8] = b"pwright-tailnet-client-v1";
const SERVER_PROOF_LABEL: &[u8] = b"pwright-tailnet-server-v1";
const MAX_PROOF_INPUT_BYTES: usize = 64;

pub type TailnetFrameTransport = TokioStreamTransport<ReadHalf<TcpStream>, WriteHalf<TcpStream>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TailnetAddressRole {
    Bind,
    Server,
    Peer,
}

impl Display for TailnetAddressRole {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Bind => "bind",
            Self::Server => "server",
            Self::Peer => "peer",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TailnetEndpointOperation {
    Bind,
    InspectLocalAddress,
    Accept,
    Connect,
    InspectPeerAddress,
    SendChallenge,
    ReceiveChallenge,
    SendClientProof,
    ReceiveClientProof,
    SendServerProof,
    ReceiveServerProof,
}

impl Display for TailnetEndpointOperation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Bind => "bind Tailnet TCP endpoint",
            Self::InspectLocalAddress => "inspect Tailnet TCP local address",
            Self::Accept => "accept Tailnet TCP connection",
            Self::Connect => "connect Tailnet TCP endpoint",
            Self::InspectPeerAddress => "inspect Tailnet TCP peer address",
            Self::SendChallenge => "send Tailnet authentication challenge",
            Self::ReceiveChallenge => "receive Tailnet authentication challenge",
            Self::SendClientProof => "send Tailnet client proof",
            Self::ReceiveClientProof => "receive Tailnet client proof",
            Self::SendServerProof => "send Tailnet server proof",
            Self::ReceiveServerProof => "receive Tailnet server proof",
        })
    }
}

#[derive(Debug)]
pub enum TailnetAuthenticationError {
    Io {
        operation: TailnetEndpointOperation,
        source: io::Error,
    },
    AddressOutsideTailnet {
        role: TailnetAddressRole,
        address: SocketAddr,
    },
    EntropyUnavailable,
    InvalidChallenge,
    PeerAuthenticationFailed,
}

impl TailnetAuthenticationError {
    fn io(operation: TailnetEndpointOperation, source: io::Error) -> Self {
        Self::Io { operation, source }
    }
}

impl Display for TailnetAuthenticationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { operation, source } => write!(formatter, "could not {operation}: {source}"),
            Self::AddressOutsideTailnet { role, address } => {
                write!(
                    formatter,
                    "Tailnet {role} address `{address}` is outside Tailscale ranges"
                )
            }
            Self::EntropyUnavailable => {
                formatter.write_str("could not generate a Tailnet authentication challenge")
            }
            Self::InvalidChallenge => {
                formatter.write_str("peer sent an invalid Tailnet authentication challenge")
            }
            Self::PeerAuthenticationFailed => {
                formatter.write_str("Tailnet peer did not prove the configured pre-shared key")
            }
        }
    }
}

impl Error for TailnetAuthenticationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::AddressOutsideTailnet { .. }
            | Self::EntropyUnavailable
            | Self::InvalidChallenge
            | Self::PeerAuthenticationFailed => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TailnetPskError {
    actual_bytes: usize,
}

impl TailnetPskError {
    pub const fn actual_bytes(self) -> usize {
        self.actual_bytes
    }
}

impl Display for TailnetPskError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Tailnet pre-shared key must contain exactly 32 bytes, received {}",
            self.actual_bytes
        )
    }
}

impl Error for TailnetPskError {}

/// Fixed-size secret used only for the Tailnet endpoint challenge.
///
/// The raw key is never formatted or sent. Its allocation is zeroized on drop.
pub struct TailnetPsk {
    bytes: Zeroizing<[u8; TAILNET_PSK_BYTES]>,
}

impl TailnetPsk {
    pub fn new(bytes: [u8; TAILNET_PSK_BYTES]) -> Self {
        Self {
            bytes: Zeroizing::new(bytes),
        }
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self, TailnetPskError> {
        if bytes.len() != TAILNET_PSK_BYTES {
            return Err(TailnetPskError {
                actual_bytes: bytes.len(),
            });
        }
        let mut value = Zeroizing::new([0u8; TAILNET_PSK_BYTES]);
        value.copy_from_slice(bytes);
        Ok(Self { bytes: value })
    }

    fn hmac_key(&self) -> hmac::Key {
        hmac::Key::new(hmac::HMAC_SHA256, self.bytes.as_ref())
    }
}

/// Returns whether an IP is in Tailscale's device address ranges.
pub const fn is_tailscale_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => is_tailscale_ipv4(address),
        IpAddr::V6(address) => is_tailscale_ipv6(address),
    }
}

const fn is_tailscale_ipv4(address: Ipv4Addr) -> bool {
    let octets = address.octets();
    octets[0] == 100 && octets[1] >= 64 && octets[1] <= 127
}

const fn is_tailscale_ipv6(address: Ipv6Addr) -> bool {
    let segments = address.segments();
    segments[0] == 0xfd7a && segments[1] == 0x115c && segments[2] == 0xa1e0
}

fn validate_address(
    role: TailnetAddressRole,
    address: SocketAddr,
) -> Result<(), TailnetAuthenticationError> {
    if is_tailscale_ip(address.ip()) {
        Ok(())
    } else {
        Err(TailnetAuthenticationError::AddressOutsideTailnet { role, address })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TailnetPeerIdentity {
    address: SocketAddr,
}

impl TailnetPeerIdentity {
    pub const fn address(self) -> SocketAddr {
        self.address
    }
}

/// A proof-carrying handshake gate created only after endpoint authentication.
pub struct TailnetPeerGate {
    peer: TailnetPeerIdentity,
}

impl TailnetPeerGate {
    pub const fn peer(&self) -> TailnetPeerIdentity {
        self.peer
    }
}

impl ServerHandshakeGate for TailnetPeerGate {
    type Authorization = TailnetPeerIdentity;
    type Error = Infallible;
    type Future<'a> = Ready<Result<Self::Authorization, Self::Error>>;

    fn authorize<'a>(&'a mut self, _hello: &'a ClientHello) -> Self::Future<'a> {
        ready(Ok(self.peer))
    }
}

pub struct AcceptedTailnetConnection {
    transport: TailnetFrameTransport,
    peer: TailnetPeerIdentity,
}

impl AcceptedTailnetConnection {
    pub const fn peer(&self) -> TailnetPeerIdentity {
        self.peer
    }

    pub fn into_parts(self) -> (TailnetFrameTransport, TailnetPeerIdentity) {
        (self.transport, self.peer)
    }

    pub fn into_peer_gate(self) -> (TailnetFrameTransport, TailnetPeerGate) {
        (self.transport, TailnetPeerGate { peer: self.peer })
    }
}

pub struct ConnectedTailnetClient {
    transport: TailnetFrameTransport,
    server: TailnetPeerIdentity,
}

impl ConnectedTailnetClient {
    pub const fn server(&self) -> TailnetPeerIdentity {
        self.server
    }

    pub fn into_parts(self) -> (TailnetFrameTransport, TailnetPeerIdentity) {
        (self.transport, self.server)
    }

    pub fn into_transport(self) -> TailnetFrameTransport {
        self.transport
    }
}

/// Caller-owned Tailnet-only TCP listener with mutual PSK authentication.
///
/// Binding rejects wildcard, loopback, LAN, and public addresses. `accept`
/// performs one fixed-size challenge exchange and returns no transport until
/// the peer proves the same key. The caller owns admission, deadlines, and the
/// accept loop.
pub struct TailnetEndpoint {
    listener: TcpListener,
    psk: TailnetPsk,
    codec: StreamFrameCodec,
    local_address: SocketAddr,
}

impl TailnetEndpoint {
    pub async fn bind(
        address: SocketAddr,
        psk: TailnetPsk,
        codec: StreamFrameCodec,
    ) -> Result<Self, TailnetAuthenticationError> {
        validate_address(TailnetAddressRole::Bind, address)?;
        let listener = TcpListener::bind(address).await.map_err(|error| {
            TailnetAuthenticationError::io(TailnetEndpointOperation::Bind, error)
        })?;
        let local_address = listener.local_addr().map_err(|error| {
            TailnetAuthenticationError::io(TailnetEndpointOperation::InspectLocalAddress, error)
        })?;
        validate_address(TailnetAddressRole::Bind, local_address)?;
        Ok(Self {
            listener,
            psk,
            codec,
            local_address,
        })
    }

    pub const fn local_address(&self) -> SocketAddr {
        self.local_address
    }

    pub async fn accept(&self) -> Result<AcceptedTailnetConnection, TailnetAuthenticationError> {
        let (mut stream, peer_address) = self.listener.accept().await.map_err(|error| {
            TailnetAuthenticationError::io(TailnetEndpointOperation::Accept, error)
        })?;
        validate_address(TailnetAddressRole::Peer, peer_address)?;
        authenticate_server(&mut stream, &self.psk).await?;
        Ok(AcceptedTailnetConnection {
            transport: TokioStreamTransport::from_io(stream, self.codec),
            peer: TailnetPeerIdentity {
                address: peer_address,
            },
        })
    }
}

/// Connects only to a Tailscale address and mutually proves the supplied PSK
/// before exposing a frame transport.
pub async fn connect_tailnet(
    server_address: SocketAddr,
    psk: &TailnetPsk,
    codec: StreamFrameCodec,
) -> Result<ConnectedTailnetClient, TailnetAuthenticationError> {
    validate_address(TailnetAddressRole::Server, server_address)?;
    let mut stream = TcpStream::connect(server_address).await.map_err(|error| {
        TailnetAuthenticationError::io(TailnetEndpointOperation::Connect, error)
    })?;
    let peer_address = stream.peer_addr().map_err(|error| {
        TailnetAuthenticationError::io(TailnetEndpointOperation::InspectPeerAddress, error)
    })?;
    validate_address(TailnetAddressRole::Server, peer_address)?;
    authenticate_client(&mut stream, psk).await?;
    Ok(ConnectedTailnetClient {
        transport: TokioStreamTransport::from_io(stream, codec),
        server: TailnetPeerIdentity {
            address: peer_address,
        },
    })
}

async fn authenticate_server<Io>(
    io: &mut Io,
    psk: &TailnetPsk,
) -> Result<(), TailnetAuthenticationError>
where
    Io: AsyncRead + AsyncWrite + Unpin,
{
    let mut nonce = [0u8; TAILNET_NONCE_BYTES];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| TailnetAuthenticationError::EntropyUnavailable)?;
    authenticate_server_with_nonce(io, psk, nonce).await
}

async fn authenticate_server_with_nonce<Io>(
    io: &mut Io,
    psk: &TailnetPsk,
    nonce: [u8; TAILNET_NONCE_BYTES],
) -> Result<(), TailnetAuthenticationError>
where
    Io: AsyncRead + AsyncWrite + Unpin,
{
    io.write_all(AUTH_MAGIC_AND_VERSION)
        .await
        .map_err(|error| {
            TailnetAuthenticationError::io(TailnetEndpointOperation::SendChallenge, error)
        })?;
    io.write_all(&nonce).await.map_err(|error| {
        TailnetAuthenticationError::io(TailnetEndpointOperation::SendChallenge, error)
    })?;

    let mut client_proof = [0u8; TAILNET_PROOF_BYTES];
    io.read_exact(&mut client_proof).await.map_err(|error| {
        TailnetAuthenticationError::io(TailnetEndpointOperation::ReceiveClientProof, error)
    })?;
    verify_proof(psk, CLIENT_PROOF_LABEL, &nonce, &client_proof)?;

    let server_proof = make_proof(psk, SERVER_PROOF_LABEL, &nonce);
    io.write_all(&server_proof).await.map_err(|error| {
        TailnetAuthenticationError::io(TailnetEndpointOperation::SendServerProof, error)
    })?;
    io.flush().await.map_err(|error| {
        TailnetAuthenticationError::io(TailnetEndpointOperation::SendServerProof, error)
    })
}

async fn authenticate_client<Io>(
    io: &mut Io,
    psk: &TailnetPsk,
) -> Result<(), TailnetAuthenticationError>
where
    Io: AsyncRead + AsyncWrite + Unpin,
{
    let mut prefix = [0u8; AUTH_MAGIC_AND_VERSION.len()];
    io.read_exact(&mut prefix).await.map_err(|error| {
        TailnetAuthenticationError::io(TailnetEndpointOperation::ReceiveChallenge, error)
    })?;
    if &prefix != AUTH_MAGIC_AND_VERSION {
        return Err(TailnetAuthenticationError::InvalidChallenge);
    }
    let mut nonce = [0u8; TAILNET_NONCE_BYTES];
    io.read_exact(&mut nonce).await.map_err(|error| {
        TailnetAuthenticationError::io(TailnetEndpointOperation::ReceiveChallenge, error)
    })?;

    let client_proof = make_proof(psk, CLIENT_PROOF_LABEL, &nonce);
    io.write_all(&client_proof).await.map_err(|error| {
        TailnetAuthenticationError::io(TailnetEndpointOperation::SendClientProof, error)
    })?;
    io.flush().await.map_err(|error| {
        TailnetAuthenticationError::io(TailnetEndpointOperation::SendClientProof, error)
    })?;

    let mut server_proof = [0u8; TAILNET_PROOF_BYTES];
    io.read_exact(&mut server_proof).await.map_err(|error| {
        TailnetAuthenticationError::io(TailnetEndpointOperation::ReceiveServerProof, error)
    })?;
    verify_proof(psk, SERVER_PROOF_LABEL, &nonce, &server_proof)
}

fn make_proof(
    psk: &TailnetPsk,
    label: &[u8],
    nonce: &[u8; TAILNET_NONCE_BYTES],
) -> [u8; TAILNET_PROOF_BYTES] {
    let (input, length) = proof_input(label, nonce);
    let tag = hmac::sign(&psk.hmac_key(), &input[..length]);
    let mut proof = [0u8; TAILNET_PROOF_BYTES];
    proof.copy_from_slice(tag.as_ref());
    proof
}

fn verify_proof(
    psk: &TailnetPsk,
    label: &[u8],
    nonce: &[u8; TAILNET_NONCE_BYTES],
    proof: &[u8; TAILNET_PROOF_BYTES],
) -> Result<(), TailnetAuthenticationError> {
    let (input, length) = proof_input(label, nonce);
    hmac::verify(&psk.hmac_key(), &input[..length], proof)
        .map_err(|_| TailnetAuthenticationError::PeerAuthenticationFailed)
}

fn proof_input(
    label: &[u8],
    nonce: &[u8; TAILNET_NONCE_BYTES],
) -> ([u8; MAX_PROOF_INPUT_BYTES], usize) {
    let length = label.len() + nonce.len();
    debug_assert!(length <= MAX_PROOF_INPUT_BYTES);
    let mut input = [0u8; MAX_PROOF_INPUT_BYTES];
    input[..label.len()].copy_from_slice(label);
    input[label.len()..length].copy_from_slice(nonce);
    (input, length)
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

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
    use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};

    use super::{
        AUTH_MAGIC_AND_VERSION, CLIENT_PROOF_LABEL, SERVER_PROOF_LABEL,
        TAILNET_AUTH_TRANSCRIPT_BYTES, TAILNET_NONCE_BYTES, TAILNET_PROOF_BYTES,
        TailnetAddressRole, TailnetAuthenticationError, TailnetEndpoint, TailnetPsk,
        authenticate_client, authenticate_server_with_nonce, connect_tailnet, is_tailscale_ip,
        make_proof, verify_proof,
    };

    fn psk(byte: u8) -> TailnetPsk {
        TailnetPsk::new([byte; 32])
    }

    fn codec() -> StreamFrameCodec {
        StreamFrameCodec::new(StreamFrameLimits::new(
            FrameLimits::new(256, 0, 256, 256),
            512,
        ))
    }

    #[test]
    fn tailscale_address_ranges_have_exact_boundaries() {
        // Arrange
        let inside_v4 = [
            Ipv4Addr::new(100, 64, 0, 0),
            Ipv4Addr::new(100, 127, 255, 255),
        ];
        let outside_v4 = [
            Ipv4Addr::new(100, 63, 255, 255),
            Ipv4Addr::new(100, 128, 0, 0),
            Ipv4Addr::LOCALHOST,
        ];
        let inside_v6 = Ipv6Addr::new(0xfd7a, 0x115c, 0xa1e0, 1, 2, 3, 4, 5);
        let outside_v6 = Ipv6Addr::new(0xfd7a, 0x115c, 0xa1df, 1, 2, 3, 4, 5);

        // Act / Assert
        assert!(
            inside_v4
                .into_iter()
                .all(|address| is_tailscale_ip(IpAddr::V4(address)))
        );
        assert!(
            outside_v4
                .into_iter()
                .all(|address| !is_tailscale_ip(IpAddr::V4(address)))
        );
        assert!(is_tailscale_ip(IpAddr::V6(inside_v6)));
        assert!(!is_tailscale_ip(IpAddr::V6(outside_v6)));
    }

    #[tokio::test]
    async fn endpoint_and_client_reject_non_tailnet_addresses_before_io() {
        // Arrange
        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
        let client_psk = psk(7);

        // Act
        let bind = TailnetEndpoint::bind(address, psk(7), codec()).await;
        let connect = connect_tailnet(address, &client_psk, codec()).await;

        // Assert
        assert!(matches!(
            bind,
            Err(TailnetAuthenticationError::AddressOutsideTailnet {
                role: TailnetAddressRole::Bind,
                address: actual,
            }) if actual == address
        ));
        assert!(matches!(
            connect,
            Err(TailnetAuthenticationError::AddressOutsideTailnet {
                role: TailnetAddressRole::Server,
                address: actual,
            }) if actual == address
        ));
    }

    #[tokio::test]
    async fn mutual_authentication_preserves_the_following_frame_boundary() {
        // Arrange
        let (mut client_io, mut server_io) = duplex(512);
        let client_psk = psk(11);
        let server_psk = psk(11);
        let nonce = [29u8; TAILNET_NONCE_BYTES];

        // Act
        let (client_auth, server_auth) = tokio::join!(
            authenticate_client(&mut client_io, &client_psk),
            authenticate_server_with_nonce(&mut server_io, &server_psk, nonce),
        );
        client_auth.expect("client should authenticate server");
        server_auth.expect("server should authenticate client");
        let client = TokioStreamTransport::from_io(client_io, codec());
        let server = TokioStreamTransport::from_io(server_io, codec());
        let frame = WireFrame::new(
            ProtocolVersion::new(3, 2),
            FrameRoute::Control,
            vec![1, 2, 3],
            Vec::new(),
            FrameLimits::new(256, 0, 256, 256),
        )
        .expect("test frame should fit");
        client.send(frame.clone()).await.expect("frame should send");
        let received = server
            .receive()
            .await
            .expect("frame should receive")
            .expect("frame should exist");

        // Assert
        assert_eq!(received, frame);
    }

    #[tokio::test]
    async fn wrong_client_proof_is_rejected_before_frame_transport_exists() {
        // Arrange
        let (mut attacker, mut server) = duplex(128);
        let server_psk = psk(3);
        let nonce = [5u8; TAILNET_NONCE_BYTES];
        let attack = async {
            let mut challenge = [0u8; AUTH_MAGIC_AND_VERSION.len() + TAILNET_NONCE_BYTES];
            attacker
                .read_exact(&mut challenge)
                .await
                .expect("attacker should receive challenge");
            attacker
                .write_all(&[0u8; TAILNET_PROOF_BYTES])
                .await
                .expect("attacker should submit invalid proof");
        };

        // Act
        let ((), result) = tokio::join!(
            attack,
            authenticate_server_with_nonce(&mut server, &server_psk, nonce),
        );

        // Assert
        assert!(matches!(
            result,
            Err(TailnetAuthenticationError::PeerAuthenticationFailed)
        ));
    }

    #[test]
    fn proof_cannot_be_replayed_for_a_fresh_challenge() {
        // Arrange
        let key = psk(17);
        let first_nonce = [1u8; TAILNET_NONCE_BYTES];
        let second_nonce = [2u8; TAILNET_NONCE_BYTES];
        let captured = make_proof(&key, CLIENT_PROOF_LABEL, &first_nonce);

        // Act
        let result = verify_proof(&key, CLIENT_PROOF_LABEL, &second_nonce, &captured);

        // Assert
        assert!(matches!(
            result,
            Err(TailnetAuthenticationError::PeerAuthenticationFailed)
        ));
    }

    #[test]
    fn psk_rejects_every_non_exact_length() {
        // Arrange / Act
        let short = TailnetPsk::from_slice(&[1u8; 31]);
        let exact = TailnetPsk::from_slice(&[1u8; 32]);
        let long = TailnetPsk::from_slice(&[1u8; 33]);

        // Assert
        assert!(matches!(short, Err(error) if error.actual_bytes() == 31));
        assert!(exact.is_ok());
        assert!(matches!(long, Err(error) if error.actual_bytes() == 33));
    }

    #[tokio::test]
    async fn forged_server_proof_is_rejected_by_the_client() {
        // Arrange
        let (mut client, mut impostor) = duplex(128);
        let client_psk = psk(23);
        let nonce = [31u8; TAILNET_NONCE_BYTES];
        let impersonation = async {
            impostor
                .write_all(AUTH_MAGIC_AND_VERSION)
                .await
                .expect("impostor should send challenge prefix");
            impostor
                .write_all(&nonce)
                .await
                .expect("impostor should send challenge nonce");
            let mut client_proof = [0u8; TAILNET_PROOF_BYTES];
            impostor
                .read_exact(&mut client_proof)
                .await
                .expect("impostor should receive client proof");
            impostor
                .write_all(&[0u8; TAILNET_PROOF_BYTES])
                .await
                .expect("impostor should send forged server proof");
        };

        // Act
        let (result, ()) =
            tokio::join!(authenticate_client(&mut client, &client_psk), impersonation);

        // Assert
        assert!(matches!(
            result,
            Err(TailnetAuthenticationError::PeerAuthenticationFailed)
        ));
    }

    #[test]
    fn direction_labels_prevent_reflecting_a_client_proof() {
        // Arrange
        let key = psk(37);
        let nonce = [41u8; TAILNET_NONCE_BYTES];
        let client_proof = make_proof(&key, CLIENT_PROOF_LABEL, &nonce);

        // Act
        let reflected = verify_proof(&key, SERVER_PROOF_LABEL, &nonce, &client_proof);

        // Assert
        assert!(matches!(
            reflected,
            Err(TailnetAuthenticationError::PeerAuthenticationFailed)
        ));
        assert_eq!(TAILNET_AUTH_TRANSCRIPT_BYTES, 105);
    }

    #[tokio::test]
    #[ignore = "requires TAILSCALE_IP to name a local Tailscale interface"]
    async fn live_tailnet_tcp_authenticates_before_pwright_handshake() {
        // Arrange
        let local_ip = std::env::var("TAILSCALE_IP")
            .expect("TAILSCALE_IP should name the test interface")
            .parse::<IpAddr>()
            .expect("TAILSCALE_IP should be an IP address");
        assert!(is_tailscale_ip(local_ip));
        let endpoint = TailnetEndpoint::bind(SocketAddr::new(local_ip, 0), psk(43), codec())
            .await
            .expect("Tailnet endpoint should bind its Tailscale interface");
        let address = endpoint.local_address();
        let client_psk = psk(43);

        // Act
        let (accepted, connected) = tokio::join!(
            endpoint.accept(),
            connect_tailnet(address, &client_psk, codec()),
        );
        let accepted = accepted.expect("server should authenticate Tailnet client");
        let connected = connected.expect("client should authenticate Tailnet server");
        let (server, mut gate) = accepted.into_peer_gate();
        let authorized_peer = gate.peer();
        let handshake_limits = NegotiationLimits::new(0, 0);
        let frame_limits = FrameLimits::new(256, 0, 0, 256);
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
            server,
            ServerSupervisorLimits::new(1).expect("supervisor limit should be valid"),
        );
        let (client_result, server_result) = tokio::join!(
            client_handshake.establish(client_protocol, hello),
            supervisor.accept_handshake(&handshake_codec, &config, &mut gate),
        );

        // Assert
        assert!(is_tailscale_ip(authorized_peer.address().ip()));
        assert_eq!(
            server_result
                .expect("authenticated server handshake should succeed")
                .authorization(),
            &authorized_peer
        );
        assert_eq!(
            client_result
                .expect("client should accept authenticated server hello")
                .hello()
                .selected_version(),
            ProtocolVersion::new(1, 0)
        );
    }
}
