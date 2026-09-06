use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::future::{Ready, ready};
use std::io;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use pwright_server_wire::ServerHandshakeGate;
use pwright_transport::ClientHello;
use pwright_transport_stream::StreamFrameCodec;
use pwright_transport_tokio::TokioStreamTransport;
use tokio::io::{ReadHalf, WriteHalf};
use tokio::net::{UnixListener, UnixStream};

const PRIVATE_SOCKET_MODE: u32 = 0o600;

pub type UnixFrameTransport = TokioStreamTransport<ReadHalf<UnixStream>, WriteHalf<UnixStream>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnixEndpointOperation {
    Bind,
    InspectSocket,
    SetPermissions,
    Accept,
    Connect,
    ReadPeerCredentials,
    RemoveOwnedSocket,
}

impl Display for UnixEndpointOperation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Bind => "bind Unix socket",
            Self::InspectSocket => "inspect Unix socket",
            Self::SetPermissions => "set Unix socket permissions",
            Self::Accept => "accept Unix connection",
            Self::Connect => "connect Unix socket",
            Self::ReadPeerCredentials => "read Unix peer credentials",
            Self::RemoveOwnedSocket => "remove owned Unix socket",
        })
    }
}

#[derive(Debug)]
pub enum UnixEndpointError {
    Io {
        operation: UnixEndpointOperation,
        source: io::Error,
    },
    BoundPathIsNotSocket {
        path: PathBuf,
    },
    ServerUidMismatch {
        expected: u32,
        actual: u32,
    },
}

impl UnixEndpointError {
    fn io(operation: UnixEndpointOperation, source: io::Error) -> Self {
        Self::Io { operation, source }
    }
}

impl Display for UnixEndpointError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { operation, source } => write!(formatter, "could not {operation}: {source}"),
            Self::BoundPathIsNotSocket { path } => {
                write!(
                    formatter,
                    "bound endpoint `{}` is not a socket",
                    path.display()
                )
            }
            Self::ServerUidMismatch { expected, actual } => write!(
                formatter,
                "Unix endpoint server UID mismatch: expected {expected}, received {actual}"
            ),
        }
    }
}

impl Error for UnixEndpointError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::BoundPathIsNotSocket { .. } | Self::ServerUidMismatch { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnixPeerIdentity {
    uid: u32,
    gid: u32,
    pid: Option<u32>,
}

impl UnixPeerIdentity {
    fn from_stream(stream: &UnixStream) -> Result<Self, UnixEndpointError> {
        let credentials = stream.peer_cred().map_err(|error| {
            UnixEndpointError::io(UnixEndpointOperation::ReadPeerCredentials, error)
        })?;
        Ok(Self {
            uid: credentials.uid(),
            gid: credentials.gid(),
            pid: credentials.pid().and_then(|pid| u32::try_from(pid).ok()),
        })
    }

    pub const fn uid(self) -> u32 {
        self.uid
    }

    pub const fn gid(self) -> u32 {
        self.gid
    }

    pub const fn pid(self) -> Option<u32> {
        self.pid
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SocketFileIdentity {
    device: u64,
    inode: u64,
}

impl SocketFileIdentity {
    fn from_metadata(metadata: &fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
        }
    }

    fn matches(self, metadata: &fs::Metadata) -> bool {
        metadata.file_type().is_socket()
            && metadata.dev() == self.device
            && metadata.ino() == self.inode
    }
}

/// One private listener owned by the caller.
///
/// Binding never deletes an existing path. The created socket is set to mode
/// `0600`. Drop removes it only when device/inode still match, so a replacement
/// file at the same pathname is never deleted.
pub struct PrivateUnixEndpoint {
    listener: UnixListener,
    codec: StreamFrameCodec,
    path: PathBuf,
    file_identity: SocketFileIdentity,
    cleanup_owned_path: bool,
}

impl PrivateUnixEndpoint {
    pub fn bind(
        path: impl Into<PathBuf>,
        codec: StreamFrameCodec,
    ) -> Result<Self, UnixEndpointError> {
        let path = path.into();
        let listener = UnixListener::bind(&path)
            .map_err(|error| UnixEndpointError::io(UnixEndpointOperation::Bind, error))?;
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| UnixEndpointError::io(UnixEndpointOperation::InspectSocket, error))?;
        if !metadata.file_type().is_socket() {
            return Err(UnixEndpointError::BoundPathIsNotSocket { path });
        }
        let endpoint = Self {
            listener,
            codec,
            path,
            file_identity: SocketFileIdentity::from_metadata(&metadata),
            cleanup_owned_path: true,
        };
        fs::set_permissions(
            &endpoint.path,
            fs::Permissions::from_mode(PRIVATE_SOCKET_MODE),
        )
        .map_err(|error| UnixEndpointError::io(UnixEndpointOperation::SetPermissions, error))?;
        Ok(endpoint)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub async fn accept(&self) -> Result<AcceptedUnixConnection, UnixEndpointError> {
        let (stream, _) = self
            .listener
            .accept()
            .await
            .map_err(|error| UnixEndpointError::io(UnixEndpointOperation::Accept, error))?;
        let peer = UnixPeerIdentity::from_stream(&stream)?;
        Ok(AcceptedUnixConnection {
            transport: TokioStreamTransport::from_io(stream, self.codec),
            peer,
        })
    }

    pub fn close(mut self) -> Result<bool, UnixEndpointError> {
        let removed = remove_if_owned(&self.path, self.file_identity).map_err(|error| {
            UnixEndpointError::io(UnixEndpointOperation::RemoveOwnedSocket, error)
        })?;
        self.cleanup_owned_path = false;
        Ok(removed)
    }
}

impl Drop for PrivateUnixEndpoint {
    fn drop(&mut self) {
        if self.cleanup_owned_path {
            let _ = remove_if_owned(&self.path, self.file_identity);
        }
    }
}

fn remove_if_owned(path: &Path, identity: SocketFileIdentity) -> io::Result<bool> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    if !identity.matches(&metadata) {
        return Ok(false);
    }
    fs::remove_file(path)?;
    Ok(true)
}

pub struct AcceptedUnixConnection {
    transport: UnixFrameTransport,
    peer: UnixPeerIdentity,
}

impl AcceptedUnixConnection {
    pub const fn peer(&self) -> UnixPeerIdentity {
        self.peer
    }

    pub fn into_parts(self) -> (UnixFrameTransport, UnixPeerIdentity) {
        (self.transport, self.peer)
    }

    pub fn into_uid_gate(self, allowed_uid: u32) -> (UnixFrameTransport, UnixPeerGate) {
        let gate = UnixPeerGate::new(self.peer, allowed_uid);
        (self.transport, gate)
    }
}

pub struct ConnectedUnixClient {
    transport: UnixFrameTransport,
    server: UnixPeerIdentity,
}

impl ConnectedUnixClient {
    pub const fn server(&self) -> UnixPeerIdentity {
        self.server
    }

    pub fn into_parts(self) -> (UnixFrameTransport, UnixPeerIdentity) {
        (self.transport, self.server)
    }

    pub fn into_transport(self) -> UnixFrameTransport {
        self.transport
    }
}

/// Connects and verifies the server process identity before exposing the
/// transport for a Pwright handshake.
pub async fn connect_unix_with_server_uid(
    path: impl AsRef<Path>,
    expected_server_uid: u32,
    codec: StreamFrameCodec,
) -> Result<ConnectedUnixClient, UnixEndpointError> {
    let stream = UnixStream::connect(path.as_ref())
        .await
        .map_err(|error| UnixEndpointError::io(UnixEndpointOperation::Connect, error))?;
    let server = UnixPeerIdentity::from_stream(&stream)?;
    if server.uid != expected_server_uid {
        return Err(UnixEndpointError::ServerUidMismatch {
            expected: expected_server_uid,
            actual: server.uid,
        });
    }
    Ok(ConnectedUnixClient {
        transport: TokioStreamTransport::from_io(stream, codec),
        server,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnixAuthorizationError {
    expected_uid: u32,
    actual_uid: u32,
}

impl UnixAuthorizationError {
    pub const fn expected_uid(self) -> u32 {
        self.expected_uid
    }

    pub const fn actual_uid(self) -> u32 {
        self.actual_uid
    }
}

impl Display for UnixAuthorizationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Unix client UID mismatch: expected {}, received {}",
            self.expected_uid, self.actual_uid
        )
    }
}

impl Error for UnixAuthorizationError {}

/// Authorizes the kernel-derived peer identity before protocol negotiation.
pub struct UnixPeerGate {
    peer: UnixPeerIdentity,
    allowed_uid: u32,
}

impl UnixPeerGate {
    pub const fn new(peer: UnixPeerIdentity, allowed_uid: u32) -> Self {
        Self { peer, allowed_uid }
    }

    pub const fn peer(&self) -> UnixPeerIdentity {
        self.peer
    }
}

impl ServerHandshakeGate for UnixPeerGate {
    type Authorization = UnixPeerIdentity;
    type Error = UnixAuthorizationError;
    type Future<'a> = Ready<Result<Self::Authorization, Self::Error>>;

    fn authorize<'a>(&'a mut self, _hello: &'a ClientHello) -> Self::Future<'a> {
        ready(if self.peer.uid == self.allowed_uid {
            Ok(self.peer)
        } else {
            Err(UnixAuthorizationError {
                expected_uid: self.allowed_uid,
                actual_uid: self.peer.uid,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use pwright_client_wire::EncodedClientHandshake;
    use pwright_codec_handshake::HandshakeCodec;
    use pwright_core::{CallId, MethodTag, ObjectId, ProtocolVersion};
    use pwright_server_wire::{
        EncodedServerSupervisor, ServerHandshakeGate, ServerSupervisorLimits,
    };
    use pwright_transport::{
        CapabilityManifest, CapabilityRequest, ClientHello, FrameLimits, FrameRoute,
        FrameTransport, NegotiationLimits, ServerHandshakeConfig, VersionPolicy, WireFrame,
        WireProtocol,
    };
    use pwright_transport_stream::{StreamFrameCodec, StreamFrameLimits};

    use super::{
        PrivateUnixEndpoint, UnixEndpointError, UnixPeerGate, connect_unix_with_server_uid,
    };

    static TEST_DIRECTORY_ID: AtomicU64 = AtomicU64::new(1);

    fn test_directory() -> PathBuf {
        let id = TEST_DIRECTORY_ID.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "pwright-endpoint-tokio-{}-{id}",
            std::process::id()
        ));
        fs::create_dir(&directory).expect("unique test directory should be created");
        directory
    }

    fn frame_limits() -> FrameLimits {
        FrameLimits::new(64, 0, 0, 64)
    }

    fn codec() -> StreamFrameCodec {
        StreamFrameCodec::new(StreamFrameLimits::new(frame_limits(), 128))
    }

    fn handshake_frame_limits() -> FrameLimits {
        FrameLimits::new(128, 0, 0, 128)
    }

    fn handshake_stream_codec() -> StreamFrameCodec {
        StreamFrameCodec::new(StreamFrameLimits::new(handshake_frame_limits(), 256))
    }

    fn current_uid(directory: &PathBuf) -> u32 {
        fs::metadata(directory)
            .expect("test directory metadata should be readable")
            .uid()
    }

    fn client_hello() -> ClientHello {
        ClientHello::new(
            VersionPolicy::exact(ProtocolVersion::new(1, 0)),
            frame_limits(),
            CapabilityRequest::new(Vec::new(), Vec::new(), NegotiationLimits::new(0, 0))
                .expect("empty capability request should build"),
        )
    }

    #[tokio::test]
    async fn private_endpoint_sets_owner_only_mode_and_removes_its_socket() {
        // Arrange
        let directory = test_directory();
        let path = directory.join("server.sock");

        // Act
        let endpoint = PrivateUnixEndpoint::bind(&path, codec())
            .expect("private endpoint should bind a new path");
        let metadata = fs::symlink_metadata(&path).expect("socket metadata should exist");
        drop(endpoint);

        // Assert
        assert!(metadata.file_type().is_socket());
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        assert!(!path.exists());
        fs::remove_dir(directory).expect("empty test directory should be removed");
    }

    #[tokio::test]
    async fn both_unix_peers_receive_kernel_verified_identity() {
        // Arrange
        let directory = test_directory();
        let expected_uid = current_uid(&directory);
        let path = directory.join("server.sock");
        let endpoint = PrivateUnixEndpoint::bind(&path, codec())
            .expect("private endpoint should bind a new path");

        // Act
        let (client, server) = tokio::join!(
            connect_unix_with_server_uid(&path, expected_uid, codec()),
            endpoint.accept(),
        );

        // Assert
        let client = client.expect("client should verify the server UID");
        let server = server.expect("server should read the client UID");
        assert_eq!(client.server().uid(), expected_uid);
        assert_eq!(server.peer().uid(), expected_uid);
        drop((client, server, endpoint));
        fs::remove_dir(directory).expect("empty test directory should be removed");
    }

    #[tokio::test]
    async fn unix_client_rejects_an_unexpected_server_uid() {
        // Arrange
        let directory = test_directory();
        let actual_uid = current_uid(&directory);
        let unexpected_uid = actual_uid.wrapping_add(1);
        let path = directory.join("server.sock");
        let endpoint = PrivateUnixEndpoint::bind(&path, codec())
            .expect("private endpoint should bind a new path");

        // Act
        let (client, server) = tokio::join!(
            connect_unix_with_server_uid(&path, unexpected_uid, codec()),
            endpoint.accept(),
        );

        // Assert
        assert!(matches!(
            client,
            Err(UnixEndpointError::ServerUidMismatch { expected, actual })
                if expected == unexpected_uid && actual == actual_uid
        ));
        assert_eq!(
            server
                .expect("server can identify a client that rejects it")
                .peer()
                .uid(),
            actual_uid
        );
        drop(endpoint);
        fs::remove_dir(directory).expect("empty test directory should be removed");
    }

    #[tokio::test]
    async fn accepted_unix_transport_preserves_bounded_frame_io() {
        // Arrange
        let directory = test_directory();
        let expected_uid = current_uid(&directory);
        let path = directory.join("server.sock");
        let endpoint = PrivateUnixEndpoint::bind(&path, codec())
            .expect("private endpoint should bind a new path");
        let (client, server) = tokio::join!(
            connect_unix_with_server_uid(&path, expected_uid, codec()),
            endpoint.accept(),
        );
        let client = client.expect("client should connect").into_transport();
        let (server, _) = server.expect("server should accept").into_parts();
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Command {
                call_id: CallId::new(7),
                target: ObjectId::new(2),
                method: MethodTag::new(3),
            },
            vec![1, 2, 3],
            Vec::new(),
            frame_limits(),
        )
        .expect("test frame should build");

        // Act
        client.send(frame.clone()).await.expect("frame should send");
        let received = server
            .receive()
            .await
            .expect("frame should receive")
            .expect("stream should remain open");

        // Assert
        assert_eq!(received, frame);
        drop((client, server, endpoint));
        fs::remove_dir(directory).expect("empty test directory should be removed");
    }

    #[tokio::test]
    async fn peer_gate_returns_identity_only_for_the_allowed_uid() {
        // Arrange
        let directory = test_directory();
        let expected_uid = current_uid(&directory);
        let path = directory.join("server.sock");
        let endpoint = PrivateUnixEndpoint::bind(&path, codec())
            .expect("private endpoint should bind a new path");
        let (_, accepted) = tokio::join!(
            connect_unix_with_server_uid(&path, expected_uid, codec()),
            endpoint.accept(),
        );
        let accepted = accepted.expect("server should accept");
        let (_, mut gate) = accepted.into_uid_gate(expected_uid);

        // Act
        let authorization = gate.authorize(&client_hello()).await;

        // Assert
        assert_eq!(
            authorization.expect("matching UID should authorize").uid(),
            expected_uid
        );
        drop(endpoint);
        fs::remove_dir(directory).expect("empty test directory should be removed");
    }

    #[tokio::test]
    async fn peer_gate_rejects_a_different_allowed_uid() {
        // Arrange
        let directory = test_directory();
        let actual_uid = current_uid(&directory);
        let path = directory.join("server.sock");
        let endpoint = PrivateUnixEndpoint::bind(&path, codec())
            .expect("private endpoint should bind a new path");
        let (_, accepted) = tokio::join!(
            connect_unix_with_server_uid(&path, actual_uid, codec()),
            endpoint.accept(),
        );
        let accepted = accepted.expect("server should accept");
        let (_, mut gate) = accepted.into_uid_gate(actual_uid.wrapping_add(1));

        // Act
        let error = gate
            .authorize(&client_hello())
            .await
            .expect_err("different UID must be rejected");

        // Assert
        assert_eq!(error.actual_uid(), actual_uid);
        assert_eq!(error.expected_uid(), actual_uid.wrapping_add(1));
        drop(endpoint);
        fs::remove_dir(directory).expect("empty test directory should be removed");
    }

    #[tokio::test]
    async fn kernel_peer_identity_authorizes_the_real_hello_exchange() {
        // Arrange
        let directory = test_directory();
        let expected_uid = current_uid(&directory);
        let path = directory.join("server.sock");
        let endpoint = PrivateUnixEndpoint::bind(&path, handshake_stream_codec())
            .expect("private endpoint should bind a new path");
        let (client, accepted) = tokio::join!(
            connect_unix_with_server_uid(&path, expected_uid, handshake_stream_codec()),
            endpoint.accept(),
        );
        let client = client.expect("client should verify server identity");
        let accepted = accepted.expect("server should read client identity");
        let (server_transport, mut gate) = accepted.into_uid_gate(expected_uid);
        let mut supervisor = EncodedServerSupervisor::new_handshake_required(
            server_transport,
            ServerSupervisorLimits::new(1).expect("supervisor limit should be valid"),
        );
        let hello = ClientHello::new(
            VersionPolicy::exact(ProtocolVersion::new(1, 0)),
            handshake_frame_limits(),
            CapabilityRequest::new(Vec::new(), Vec::new(), NegotiationLimits::new(0, 0))
                .expect("empty capability request should build"),
        );
        let config = ServerHandshakeConfig::new(
            VersionPolicy::exact(ProtocolVersion::new(1, 0)),
            handshake_frame_limits(),
            CapabilityManifest::new(Vec::new(), NegotiationLimits::new(0, 0))
                .expect("empty capability manifest should build"),
        );
        let handshake_codec = HandshakeCodec::new(NegotiationLimits::new(0, 0));
        let client_protocol = WireProtocol::new_exact_directional(
            handshake_codec,
            ProtocolVersion::new(1, 0),
            handshake_frame_limits(),
            handshake_frame_limits(),
        );
        let client_handshake = EncodedClientHandshake::new(client.into_transport());

        // Act
        let (client_result, server_result) = tokio::join!(
            client_handshake.establish(client_protocol, hello),
            supervisor.accept_handshake(&handshake_codec, &config, &mut gate),
        );

        // Assert
        assert_eq!(
            server_result
                .expect("kernel-authorized server handshake should succeed")
                .authorization()
                .uid(),
            expected_uid
        );
        assert_eq!(
            client_result
                .expect("client handshake should accept the server hello")
                .hello()
                .selected_version(),
            ProtocolVersion::new(1, 0)
        );
        drop(endpoint);
        fs::remove_dir(directory).expect("empty test directory should be removed");
    }

    #[tokio::test]
    async fn endpoint_drop_does_not_remove_a_replacement_at_the_same_path() {
        // Arrange
        let directory = test_directory();
        let path = directory.join("server.sock");
        let endpoint = PrivateUnixEndpoint::bind(&path, codec())
            .expect("private endpoint should bind a new path");
        fs::remove_file(&path).expect("owned socket path should be unlinked");
        fs::write(&path, b"replacement").expect("replacement file should be created");

        // Act
        drop(endpoint);

        // Assert
        assert_eq!(
            fs::read(&path).expect("replacement should remain"),
            b"replacement"
        );
        fs::remove_file(path).expect("replacement should be removed");
        fs::remove_dir(directory).expect("empty test directory should be removed");
    }

    #[tokio::test]
    async fn bind_never_deletes_an_existing_path() {
        // Arrange
        let directory = test_directory();
        let path = directory.join("server.sock");
        fs::write(&path, b"existing").expect("existing file should be created");

        // Act
        let result = PrivateUnixEndpoint::bind(&path, codec());

        // Assert
        assert!(matches!(result, Err(UnixEndpointError::Io { .. })));
        assert_eq!(
            fs::read(&path).expect("existing path should remain"),
            b"existing"
        );
        fs::remove_file(path).expect("existing file should be removed");
        fs::remove_dir(directory).expect("empty test directory should be removed");
    }

    #[test]
    fn gate_can_be_composed_from_an_injected_identity_without_io() {
        // Arrange
        let peer = super::UnixPeerIdentity {
            uid: 42,
            gid: 7,
            pid: Some(9),
        };
        let mut gate = UnixPeerGate::new(peer, 42);

        // Act
        let authorization = gate.authorize(&client_hello()).into_inner();

        // Assert
        assert_eq!(authorization, Ok(peer));
    }
}
