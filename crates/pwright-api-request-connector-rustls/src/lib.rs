//! Optional Rustls connector for Pwright API request HTTP and HTTPS origins.
//!
//! The connector owns root configuration and the TLS handshake only. HTTP/1
//! framing, redirects, retries, decompression, proxying and cookies stay in
//! their independent capability layers. The same TLS owner can wrap a direct
//! socket or an already-established proxy tunnel without opening another
//! connection.

use std::{
    io::{self, Cursor, Read as _, Write as _},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use pwright_api_request_connector_tokio::{TokioIoConnection, connect_tcp_with_addresses};
use pwright_api_request_transport::{
    ApiConnection, ApiConnectionTarget, ApiConnector, ApiTunnelTlsUpgrader,
};
pub use pwright_api_request_transport::{ApiConnectionError, ApiConnectionErrorKind};
use tokio_rustls::{
    TlsConnector,
    rustls::{
        ClientConfig, ClientConnection, DigitallySignedStruct, Error as RustlsError, RootCertStore,
        SignatureScheme,
        client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
        crypto::{CryptoProvider, WebPkiSupportedAlgorithms, ring},
        pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime},
    },
};
use tokio_util::sync::CancellationToken;
use url::Url;

const MAX_CLIENT_IDENTITIES: usize = 64;
const MAX_ORIGIN_BYTES: usize = 4 * 1024;
const TLS_IO_BUFFER_BYTES: usize = 32 * 1024;
const TLS_BUFFER_LIMIT_BYTES: usize = 64 * 1024;

/// One already-decoded client identity selected only for its exact origin.
pub struct RustlsClientIdentity {
    origin: String,
    certificate_chain: Vec<CertificateDer<'static>>,
    private_key: PrivateKeyDer<'static>,
}

/// Lazy origin-scoped identity source selected independently from TLS I/O.
pub trait RustlsClientIdentityProvider: Send + Sync + 'static {
    /// Resolve one exact-origin identity without retaining request state.
    ///
    /// # Errors
    ///
    /// Returns a TLS error when matching material cannot be decoded.
    fn identity_for_origin(
        &self,
        origin: &str,
    ) -> Result<Option<RustlsClientIdentity>, ApiConnectionError>;
}

impl RustlsClientIdentity {
    /// Construct one identity from a DER certificate chain and PKCS#8 key.
    ///
    /// # Errors
    ///
    /// Returns a TLS error for an empty/oversized origin, empty certificate
    /// chain or empty key. Cryptographic key compatibility is checked while
    /// building the connector.
    pub fn new(
        origin: impl Into<String>,
        certificate_chain: Vec<Vec<u8>>,
        private_key_pkcs8: Vec<u8>,
    ) -> Result<Self, ApiConnectionError> {
        let origin = origin.into();
        if origin.is_empty() || origin.len() > MAX_ORIGIN_BYTES {
            return Err(ApiConnectionError::new(
                ApiConnectionErrorKind::Tls,
                format!("client certificate origin must contain 1..={MAX_ORIGIN_BYTES} bytes"),
            ));
        }
        if certificate_chain.is_empty() {
            return Err(ApiConnectionError::new(
                ApiConnectionErrorKind::Tls,
                "client certificate chain is empty",
            ));
        }
        if private_key_pkcs8.is_empty() {
            return Err(ApiConnectionError::new(
                ApiConnectionErrorKind::Tls,
                "client certificate private key is empty",
            ));
        }
        Ok(Self {
            origin: normalize_origin(&origin),
            certificate_chain: certificate_chain
                .into_iter()
                .map(CertificateDer::from)
                .collect(),
            private_key: PrivatePkcs8KeyDer::from(private_key_pkcs8).into(),
        })
    }
}

impl std::fmt::Debug for RustlsClientIdentity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RustlsClientIdentity")
            .field("origin", &self.origin)
            .field("certificate_count", &self.certificate_chain.len())
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
struct OriginClientConfigs {
    origin: String,
    verified: Arc<ClientConfig>,
    insecure: Arc<ClientConfig>,
}

#[derive(Clone)]
pub struct RustlsApiConnector {
    verified: Arc<ClientConfig>,
    insecure: Arc<ClientConfig>,
    crypto_provider: Arc<CryptoProvider>,
    roots: Arc<RootCertStore>,
    identity_provider: Option<Arc<dyn RustlsClientIdentityProvider>>,
    client_configs: Arc<Mutex<Vec<OriginClientConfigs>>>,
}

impl RustlsApiConnector {
    /// Build verified and request-selectable insecure TLS configurations.
    ///
    /// # Errors
    ///
    /// Returns a TLS connection error if the selected Rustls crypto provider
    /// cannot construct the safe default protocol versions.
    pub fn new() -> Result<Self, ApiConnectionError> {
        Self::with_client_identities(Vec::new())
    }

    /// Build TLS configurations with independently supplied client identities.
    ///
    /// # Errors
    ///
    /// Returns a TLS connection error for excessive or duplicate identities,
    /// unsupported key material, or invalid TLS configuration.
    pub fn with_client_identities(
        identities: Vec<RustlsClientIdentity>,
    ) -> Result<Self, ApiConnectionError> {
        Self::build(identities, None)
    }

    /// Build TLS configuration with a lazily selected identity provider.
    ///
    /// The provider is called only for an HTTPS origin not already cached.
    ///
    /// # Errors
    ///
    /// Returns a TLS error if the base configurations cannot be constructed.
    pub fn with_client_identity_provider(
        provider: Arc<dyn RustlsClientIdentityProvider>,
    ) -> Result<Self, ApiConnectionError> {
        Self::build(Vec::new(), Some(provider))
    }

    fn build(
        identities: Vec<RustlsClientIdentity>,
        identity_provider: Option<Arc<dyn RustlsClientIdentityProvider>>,
    ) -> Result<Self, ApiConnectionError> {
        if identities.len() > MAX_CLIENT_IDENTITIES {
            return Err(ApiConnectionError::new(
                ApiConnectionErrorKind::Tls,
                format!("client certificate count exceeds {MAX_CLIENT_IDENTITIES}"),
            ));
        }
        for (index, identity) in identities.iter().enumerate() {
            if identities[..index]
                .iter()
                .any(|earlier| earlier.origin == identity.origin)
            {
                return Err(ApiConnectionError::new(
                    ApiConnectionErrorKind::Tls,
                    format!(
                        "multiple client certificates for origin {} are not supported yet",
                        identity.origin
                    ),
                ));
            }
        }
        let provider = Arc::new(ring::default_provider());
        let roots = Arc::new(RootCertStore::from_iter(
            webpki_roots::TLS_SERVER_ROOTS.iter().cloned(),
        ));
        let verified = client_config(provider.clone(), &roots, false, None)?;
        let insecure = client_config(provider.clone(), &roots, true, None)?;
        let client_configs = identities
            .iter()
            .map(|identity| {
                Ok(OriginClientConfigs {
                    origin: identity.origin.clone(),
                    verified: Arc::new(client_config(
                        provider.clone(),
                        &roots,
                        false,
                        Some(identity),
                    )?),
                    insecure: Arc::new(client_config(
                        provider.clone(),
                        &roots,
                        true,
                        Some(identity),
                    )?),
                })
            })
            .collect::<Result<Vec<_>, ApiConnectionError>>()?;

        Ok(Self {
            verified: Arc::new(verified),
            insecure: Arc::new(insecure),
            crypto_provider: provider,
            roots,
            identity_provider,
            client_configs: Arc::new(Mutex::new(client_configs)),
        })
    }

    fn config(
        &self,
        url: &Url,
        ignore_https_errors: bool,
    ) -> Result<Arc<ClientConfig>, ApiConnectionError> {
        let origin = url.origin().ascii_serialization();
        if let Some(configs) = self.cached_client_configs(&origin)? {
            return Ok(if ignore_https_errors {
                configs.insecure.clone()
            } else {
                configs.verified.clone()
            });
        }
        if let Some(identity) = self
            .identity_provider
            .as_ref()
            .map(|provider| provider.identity_for_origin(&origin))
            .transpose()?
            .flatten()
        {
            let configs = OriginClientConfigs {
                origin: origin.clone(),
                verified: Arc::new(client_config(
                    self.crypto_provider.clone(),
                    &self.roots,
                    false,
                    Some(&identity),
                )?),
                insecure: Arc::new(client_config(
                    self.crypto_provider.clone(),
                    &self.roots,
                    true,
                    Some(&identity),
                )?),
            };
            let selected = if ignore_https_errors {
                configs.insecure.clone()
            } else {
                configs.verified.clone()
            };
            let mut cached = self.client_configs.lock().map_err(|_| {
                ApiConnectionError::new(
                    ApiConnectionErrorKind::Tls,
                    "client certificate configuration cache is poisoned",
                )
            })?;
            if let Some(existing) = cached.iter().find(|entry| entry.origin == origin) {
                return Ok(if ignore_https_errors {
                    existing.insecure.clone()
                } else {
                    existing.verified.clone()
                });
            }
            cached.push(configs);
            return Ok(selected);
        }
        Ok(if ignore_https_errors {
            self.insecure.clone()
        } else {
            self.verified.clone()
        })
    }

    fn cached_client_configs(
        &self,
        origin: &str,
    ) -> Result<Option<OriginClientConfigs>, ApiConnectionError> {
        self.client_configs
            .lock()
            .map_err(|_| {
                ApiConnectionError::new(
                    ApiConnectionErrorKind::Tls,
                    "client certificate configuration cache is poisoned",
                )
            })
            .map(|cached| cached.iter().find(|entry| entry.origin == origin).cloned())
    }
}

fn client_config(
    provider: Arc<CryptoProvider>,
    roots: &RootCertStore,
    insecure: bool,
    identity: Option<&RustlsClientIdentity>,
) -> Result<ClientConfig, ApiConnectionError> {
    let builder = ClientConfig::builder_with_provider(provider.clone())
        .with_safe_default_protocol_versions()
        .map_err(|error| tls_error("failed to configure TLS versions", &error))?;
    let builder = if insecure {
        builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoCertificateVerification {
                supported: provider.signature_verification_algorithms,
            }))
    } else {
        builder.with_root_certificates(roots.clone())
    };
    let mut config = match identity {
        Some(identity) => builder
            .with_client_auth_cert(
                identity.certificate_chain.clone(),
                identity.private_key.clone_key(),
            )
            .map_err(|error| tls_error("failed to configure client certificate", &error))?,
        None => builder.with_no_client_auth(),
    };
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(config)
}

fn normalize_origin(origin: &str) -> String {
    Url::parse(origin).map_or_else(
        |_| origin.to_owned(),
        |url| url.origin().ascii_serialization(),
    )
}

impl std::fmt::Debug for RustlsApiConnector {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RustlsApiConnector")
            .field("has_identity_provider", &self.identity_provider.is_some())
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl ApiConnector for RustlsApiConnector {
    async fn connect(
        &self,
        target: ApiConnectionTarget<'_>,
        cancellation: &CancellationToken,
    ) -> Result<Box<dyn ApiConnection>, ApiConnectionError> {
        if cancellation.is_cancelled() {
            return Err(cancelled());
        }
        match target.url.scheme() {
            "http" => {
                let stream =
                    connect_tcp_with_addresses(target.url, target.resolved_addresses, cancellation)
                        .await?;
                Ok(Box::new(TokioIoConnection::new(stream)))
            }
            "https" => {
                let host = target.url.host_str().ok_or_else(|| {
                    ApiConnectionError::new(ApiConnectionErrorKind::Dns, "request URL has no host")
                })?;
                let server_name = ServerName::try_from(host.to_owned()).map_err(|error| {
                    ApiConnectionError::new(
                        ApiConnectionErrorKind::Tls,
                        format!("invalid TLS server name {host}: {error}"),
                    )
                })?;
                let stream =
                    connect_tcp_with_addresses(target.url, target.resolved_addresses, cancellation)
                        .await?;
                let connector =
                    TlsConnector::from(self.config(target.url, target.ignore_https_errors)?);
                let tls = tokio::select! {
                    biased;
                    () = cancellation.cancelled() => return Err(cancelled()),
                    result = connector.connect(server_name, stream) => result.map_err(|error| {
                        tls_error("TLS handshake failed", &error)
                    })?,
                };
                Ok(Box::new(TokioIoConnection::new(tls)))
            }
            scheme => Err(ApiConnectionError::new(
                ApiConnectionErrorKind::Other,
                format!("unsupported request URL scheme: {scheme}"),
            )),
        }
    }
}

#[async_trait]
impl ApiTunnelTlsUpgrader for RustlsApiConnector {
    async fn upgrade(
        &self,
        connection: Box<dyn ApiConnection>,
        target: ApiConnectionTarget<'_>,
        cancellation: &CancellationToken,
    ) -> Result<Box<dyn ApiConnection>, ApiConnectionError> {
        if cancellation.is_cancelled() {
            return Err(cancelled());
        }
        if target.url.scheme() != "https" {
            return Err(ApiConnectionError::new(
                ApiConnectionErrorKind::Tls,
                "tunnel TLS upgrader requires an https target",
            ));
        }
        let host = target.url.host_str().ok_or_else(|| {
            ApiConnectionError::new(ApiConnectionErrorKind::Dns, "request URL has no host")
        })?;
        let server_name = ServerName::try_from(host.to_owned()).map_err(|error| {
            ApiConnectionError::new(
                ApiConnectionErrorKind::Tls,
                format!("invalid TLS server name {host}: {error}"),
            )
        })?;
        let mut tls = ClientConnection::new(
            self.config(target.url, target.ignore_https_errors)?,
            server_name,
        )
        .map_err(|error| tls_error("failed to initialize tunneled TLS", &error))?;
        tls.set_buffer_limit(Some(TLS_BUFFER_LIMIT_BYTES));
        let mut connection = RustlsTunnelConnection { connection, tls };
        connection.complete_handshake(cancellation).await?;
        Ok(Box::new(connection))
    }
}

struct RustlsTunnelConnection {
    connection: Box<dyn ApiConnection>,
    tls: ClientConnection,
}

impl RustlsTunnelConnection {
    async fn complete_handshake(
        &mut self,
        cancellation: &CancellationToken,
    ) -> Result<(), ApiConnectionError> {
        while self.tls.is_handshaking() {
            self.flush_tls(cancellation).await?;
            if !self.tls.is_handshaking() {
                break;
            }
            if !self.tls.wants_read() {
                return Err(ApiConnectionError::new(
                    ApiConnectionErrorKind::Tls,
                    "tunneled TLS handshake made no I/O progress",
                ));
            }
            if !self.read_tls(cancellation).await? {
                return Err(ApiConnectionError::new(
                    ApiConnectionErrorKind::Tls,
                    "proxy tunnel closed during TLS handshake",
                ));
            }
        }
        self.flush_tls(cancellation).await
    }

    async fn flush_tls(
        &mut self,
        cancellation: &CancellationToken,
    ) -> Result<(), ApiConnectionError> {
        while self.tls.wants_write() {
            let mut ciphertext = Vec::with_capacity(TLS_IO_BUFFER_BYTES);
            let bytes = self
                .tls
                .write_tls(&mut ciphertext)
                .map_err(|error| tls_io_error("failed to encode tunneled TLS", &error))?;
            if bytes == 0 {
                return Err(ApiConnectionError::new(
                    ApiConnectionErrorKind::Tls,
                    "tunneled TLS write made no progress",
                ));
            }
            self.connection.write_all(&ciphertext, cancellation).await?;
        }
        Ok(())
    }

    async fn read_tls(
        &mut self,
        cancellation: &CancellationToken,
    ) -> Result<bool, ApiConnectionError> {
        let mut ciphertext = [0; TLS_IO_BUFFER_BYTES];
        let bytes = self.connection.read(&mut ciphertext, cancellation).await?;
        if bytes == 0 {
            return Ok(false);
        }
        let mut input = Cursor::new(&ciphertext[..bytes]);
        let accepted = self
            .tls
            .read_tls(&mut input)
            .map_err(|error| tls_io_error("failed to read tunneled TLS", &error))?;
        if accepted != bytes {
            return Err(ApiConnectionError::new(
                ApiConnectionErrorKind::Tls,
                "tunneled TLS input exceeded the bounded Rustls buffer",
            ));
        }
        self.tls
            .process_new_packets()
            .map_err(|error| tls_error("tunneled TLS record failed", &error))?;
        Ok(true)
    }
}

#[async_trait]
impl ApiConnection for RustlsTunnelConnection {
    async fn write_all(
        &mut self,
        bytes: &[u8],
        cancellation: &CancellationToken,
    ) -> Result<(), ApiConnectionError> {
        if cancellation.is_cancelled() {
            return Err(cancelled());
        }
        let mut offset = 0;
        while offset < bytes.len() {
            match self.tls.writer().write(&bytes[offset..]) {
                Ok(0) => {
                    if !self.tls.wants_write() {
                        return Err(ApiConnectionError::new(
                            ApiConnectionErrorKind::Tls,
                            "tunneled TLS plaintext write made no progress",
                        ));
                    }
                }
                Ok(written) => offset += written,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) => {
                    return Err(tls_io_error(
                        "failed to write tunneled TLS plaintext",
                        &error,
                    ));
                }
            }
            self.flush_tls(cancellation).await?;
        }
        self.flush_tls(cancellation).await
    }

    async fn read(
        &mut self,
        output: &mut [u8],
        cancellation: &CancellationToken,
    ) -> Result<usize, ApiConnectionError> {
        if cancellation.is_cancelled() {
            return Err(cancelled());
        }
        loop {
            match self.tls.reader().read(output) {
                Ok(bytes) => return Ok(bytes),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) => {
                    return Err(tls_io_error(
                        "failed to read tunneled TLS plaintext",
                        &error,
                    ));
                }
            }
            self.flush_tls(cancellation).await?;
            if self.tls.wants_read() {
                if !self.read_tls(cancellation).await? {
                    return Ok(0);
                }
                continue;
            }
            if !self.tls.wants_write() {
                return Err(ApiConnectionError::new(
                    ApiConnectionErrorKind::Tls,
                    "tunneled TLS read made no I/O progress",
                ));
            }
        }
    }
}

struct NoCertificateVerification {
    supported: WebPkiSupportedAlgorithms,
}

impl std::fmt::Debug for NoCertificateVerification {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NoCertificateVerification")
            .finish_non_exhaustive()
    }
}

impl ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _certificate: &CertificateDer<'_>,
        _signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _certificate: &CertificateDer<'_>,
        _signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.supported.supported_schemes()
    }
}

fn cancelled() -> ApiConnectionError {
    ApiConnectionError::new(
        ApiConnectionErrorKind::Cancelled,
        "API request was cancelled",
    )
}

fn tls_error(stage: &str, error: &impl std::fmt::Display) -> ApiConnectionError {
    ApiConnectionError::new(ApiConnectionErrorKind::Tls, format!("{stage}: {error}"))
}

fn tls_io_error(stage: &str, error: &io::Error) -> ApiConnectionError {
    let kind = match error.kind() {
        io::ErrorKind::ConnectionReset => ApiConnectionErrorKind::Reset,
        io::ErrorKind::ConnectionRefused => ApiConnectionErrorKind::Refused,
        _ => ApiConnectionErrorKind::Tls,
    };
    ApiConnectionError::new(kind, format!("{stage}: {error}"))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_api_request_connector_tokio::TokioIoConnection;
    use pwright_api_request_transport::{
        ApiConnectionErrorKind, ApiConnectionTarget, ApiConnector, ApiTunnelTlsUpgrader,
    };
    use rcgen::{CertificateParams, KeyPair};
    use tokio::io::{AsyncReadExt, AsyncWriteExt, duplex};
    use tokio_rustls::{
        TlsAcceptor, TlsConnector,
        rustls::{
            ServerConfig,
            pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer, ServerName},
        },
    };
    use tokio_util::sync::CancellationToken;
    use url::Url;

    use super::RustlsApiConnector;

    #[test]
    fn verified_and_insecure_configs_are_distinct_and_http1_only() {
        // Arrange
        let connector = RustlsApiConnector::new().expect("TLS configs should build");
        let url = Url::parse("https://pwright.test/").expect("test URL should parse");

        // Act
        let verified = connector
            .config(&url, false)
            .expect("verified config should resolve");
        let insecure = connector
            .config(&url, true)
            .expect("insecure config should resolve");

        // Assert
        assert!(!Arc::ptr_eq(&verified, &insecure));
        assert_eq!(verified.alpn_protocols, [b"http/1.1".to_vec()]);
        assert_eq!(insecure.alpn_protocols, [b"http/1.1".to_vec()]);
    }

    #[tokio::test]
    async fn insecure_config_completes_self_signed_http1_handshake_in_memory() {
        // Arrange
        let connector = RustlsApiConnector::new().expect("TLS configs should build");
        let server_name = "pwright.test";
        let url = Url::parse("https://pwright.test/").expect("test URL should parse");
        let key = KeyPair::generate().expect("test key should generate");
        let certificate = CertificateParams::new(vec![server_name.into()])
            .expect("test certificate params should build")
            .self_signed(&key)
            .expect("test certificate should self-sign");
        let private_key: PrivateKeyDer<'static> =
            PrivatePkcs8KeyDer::from(key.serialize_der()).into();
        let mut server_config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![certificate.der().clone()], private_key)
            .expect("test server config should build");
        server_config.alpn_protocols = vec![b"http/1.1".to_vec()];
        let server = TlsAcceptor::from(Arc::new(server_config));
        let client = TlsConnector::from(
            connector
                .config(&url, true)
                .expect("insecure config should resolve"),
        );
        let name = ServerName::try_from(server_name)
            .expect("test server name should parse")
            .to_owned();
        let (client_io, server_io) = duplex(64 * 1024);

        // Act
        let (server_result, client_result) =
            tokio::join!(server.accept(server_io), client.connect(name, client_io));
        let _server_stream = server_result.expect("server handshake should succeed");
        let client_stream = client_result.expect("insecure client handshake should succeed");

        // Assert
        assert_eq!(
            client_stream.get_ref().1.alpn_protocol(),
            Some(&b"http/1.1"[..])
        );
    }

    #[tokio::test]
    async fn tunnel_upgrader_wraps_existing_connection_without_opening_a_socket() {
        // Arrange
        let connector = RustlsApiConnector::new().expect("TLS configs should build");
        let server_name = "pwright.test";
        let url = Url::parse("https://pwright.test/").expect("test URL should parse");
        let key = KeyPair::generate().expect("test key should generate");
        let certificate = CertificateParams::new(vec![server_name.into()])
            .expect("test certificate params should build")
            .self_signed(&key)
            .expect("test certificate should self-sign");
        let private_key: PrivateKeyDer<'static> =
            PrivatePkcs8KeyDer::from(key.serialize_der()).into();
        let mut server_config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![certificate.der().clone()], private_key)
            .expect("test server config should build");
        server_config.alpn_protocols = vec![b"http/1.1".to_vec()];
        let server = TlsAcceptor::from(Arc::new(server_config));
        let (client_io, server_io) = duplex(64 * 1024);
        let cancellation = CancellationToken::new();
        let server_exchange = async move {
            let mut stream = server
                .accept(server_io)
                .await
                .expect("server handshake should succeed");
            let mut request = [0; 4];
            stream
                .read_exact(&mut request)
                .await
                .expect("server should read request");
            assert_eq!(&request, b"ping");
            stream
                .write_all(b"pong")
                .await
                .expect("server should write response");
        };
        let client_exchange = async {
            let mut connection = connector
                .upgrade(
                    Box::new(TokioIoConnection::new(client_io)),
                    ApiConnectionTarget {
                        url: &url,
                        ignore_https_errors: true,
                        resolved_addresses: None,
                    },
                    &cancellation,
                )
                .await
                .expect("tunneled TLS handshake should succeed");
            connection
                .write_all(b"ping", &cancellation)
                .await
                .expect("client should write plaintext");
            let mut response = [0; 4];
            let mut offset = 0;
            while offset < response.len() {
                let bytes = connection
                    .read(&mut response[offset..], &cancellation)
                    .await
                    .expect("client should read plaintext");
                assert_ne!(bytes, 0, "TLS response should not end early");
                offset += bytes;
            }
            response
        };

        // Act
        let ((), response) = tokio::join!(server_exchange, client_exchange);

        // Assert
        assert_eq!(&response, b"pong");
    }

    #[tokio::test]
    async fn unsupported_scheme_is_rejected_before_dns() {
        // Arrange
        let connector = RustlsApiConnector::new().expect("TLS configs should build");
        let url = Url::parse("ftp://not-resolved.invalid/").expect("test URL should parse");

        // Act
        let Err(error) = connector
            .connect(
                ApiConnectionTarget {
                    url: &url,
                    ignore_https_errors: false,
                    resolved_addresses: None,
                },
                &CancellationToken::new(),
            )
            .await
        else {
            panic!("unsupported scheme should fail");
        };

        // Assert
        assert_eq!(error.kind, ApiConnectionErrorKind::Other);
    }

    #[tokio::test]
    async fn cancelled_https_is_rejected_before_dns_or_handshake() {
        // Arrange
        let connector = RustlsApiConnector::new().expect("TLS configs should build");
        let url = Url::parse("https://not-resolved.invalid/").expect("test URL should parse");
        let cancellation = CancellationToken::new();
        cancellation.cancel();

        // Act
        let Err(error) = connector
            .connect(
                ApiConnectionTarget {
                    url: &url,
                    ignore_https_errors: true,
                    resolved_addresses: None,
                },
                &cancellation,
            )
            .await
        else {
            panic!("cancelled HTTPS should fail");
        };

        // Assert
        assert_eq!(error.kind, ApiConnectionErrorKind::Cancelled);
    }
}
