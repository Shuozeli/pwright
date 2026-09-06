//! Optional caller-driven Tokio endpoint factories for Pwright transports.
//!
//! The Unix adapter obtains peer credentials from the kernel. The Tailnet TCP
//! adapter admits only Tailscale addresses and performs a fixed-size mutual
//! PSK challenge before exposing the Pwright frame transport. Neither starts
//! an accept loop, task, or runtime.

mod tailnet;
mod tls;

#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub use unix::{
    AcceptedUnixConnection, ConnectedUnixClient, PrivateUnixEndpoint, UnixAuthorizationError,
    UnixEndpointError, UnixEndpointOperation, UnixFrameTransport, UnixPeerGate, UnixPeerIdentity,
    connect_unix_with_server_uid,
};

pub use tailnet::{
    AcceptedTailnetConnection, ConnectedTailnetClient, TAILNET_AUTH_TRANSCRIPT_BYTES,
    TAILNET_NONCE_BYTES, TAILNET_PROOF_BYTES, TAILNET_PSK_BYTES, TailnetAddressRole,
    TailnetAuthenticationError, TailnetEndpoint, TailnetEndpointOperation, TailnetFrameTransport,
    TailnetPeerGate, TailnetPeerIdentity, TailnetPsk, TailnetPskError, connect_tailnet,
    is_tailscale_ip,
};

pub use tls::{
    AcceptedMutualTlsConnection, CertificateFingerprint, CertificatePins, CertificatePinsError,
    ConnectedMutualTlsClient, MutualTlsAddressRole, MutualTlsAuthenticationError,
    MutualTlsClientFrameTransport, MutualTlsEndpoint, MutualTlsEndpointOperation,
    MutualTlsPeerGate, MutualTlsPeerIdentity, MutualTlsPeerRole, MutualTlsServerFrameTransport,
    PWRIGHT_TLS_ALPN, RUSTLS_MAX_CERTIFICATE_BYTES, RUSTLS_MAX_HANDSHAKE_BUFFER_BYTES,
    connect_mutual_tls,
};

pub use tokio_rustls::rustls::pki_types::ServerName as MutualTlsServerName;
pub use tokio_rustls::rustls::{
    ClientConfig as MutualTlsClientConfig, ServerConfig as MutualTlsServerConfig,
};
