//! Optional bounded SOCKS5 connector for Pwright API requests.
//!
//! This leaf owns the SOCKS5 no-authentication handshake and remote hostname
//! resolution. It wraps an injected connector capable of reaching the proxy,
//! and deliberately keeps TLS over the established tunnel in a separate
//! composition.

use std::sync::Arc;

use async_trait::async_trait;
use pwright_api_request_transport::{
    ApiConnection, ApiConnectionError, ApiConnectionErrorKind, ApiConnectionTarget, ApiConnector,
    ApiTunnelTlsUpgrader,
};
use tokio_util::sync::CancellationToken;
use url::{Host, Url};

const MAX_PROXY_URL_BYTES: usize = 64 * 1024;
const MAX_DOMAIN_BYTES: usize = u8::MAX as usize;

/// A connector wrapper that establishes one SOCKS5 tunnel per exchange.
pub struct Socks5ProxyConnector {
    upstream: Arc<dyn ApiConnector>,
    tls_upgrader: Option<Arc<dyn ApiTunnelTlsUpgrader>>,
    proxy_transport_url: Url,
}

impl Socks5ProxyConnector {
    /// Create a bounded unauthenticated SOCKS5 capability.
    ///
    /// Both `socks5:` and `socks5h:` resolve domain names at the proxy, matching
    /// Playwright API request behavior. Username/password authentication stays
    /// explicit unsupported work instead of being silently ignored.
    ///
    /// # Errors
    ///
    /// Returns a proxy error for invalid/oversized server or credential data.
    pub fn new(
        upstream: Arc<dyn ApiConnector>,
        server: &str,
        username: Option<&str>,
        password: Option<&str>,
    ) -> Result<Self, ApiConnectionError> {
        if server.is_empty() || server.len() > MAX_PROXY_URL_BYTES {
            return Err(proxy_error(format!(
                "proxy server must contain 1..={MAX_PROXY_URL_BYTES} bytes"
            )));
        }
        if username.is_some() || password.is_some() {
            return Err(proxy_error(
                "SOCKS5 proxy authentication is not supported by this capability",
            ));
        }
        let proxy_url = Url::parse(server)
            .map_err(|error| proxy_error(format!("invalid proxy server: {error}")))?;
        if !matches!(proxy_url.scheme(), "socks5" | "socks5h") {
            return Err(proxy_error(
                "SOCKS5 proxy capability supports only socks5 and socks5h servers",
            ));
        }
        let proxy_host = match proxy_url.host() {
            Some(Host::Domain(host)) => host.to_owned(),
            Some(Host::Ipv4(host)) => host.to_string(),
            Some(Host::Ipv6(host)) => format!("[{host}]"),
            None => return Err(proxy_error("proxy server must include a host and port")),
        };
        let proxy_port = proxy_url
            .port()
            .ok_or_else(|| proxy_error("proxy server must include a host and port"))?;
        if proxy_url.username() != "" || proxy_url.password().is_some() {
            return Err(proxy_error(
                "proxy URL credentials must use the separate username and password options",
            ));
        }
        let proxy_transport_url = Url::parse(&format!("http://{proxy_host}:{proxy_port}/"))
            .map_err(|error| {
                proxy_error(format!(
                    "could not construct SOCKS5 proxy transport URL: {error}"
                ))
            })?;
        Ok(Self {
            upstream,
            tls_upgrader: None,
            proxy_transport_url,
        })
    }

    /// Compose target-origin TLS over the established SOCKS5 tunnel.
    #[must_use]
    pub fn with_tls_upgrader(mut self, tls_upgrader: Arc<dyn ApiTunnelTlsUpgrader>) -> Self {
        self.tls_upgrader = Some(tls_upgrader);
        self
    }
}

impl std::fmt::Debug for Socks5ProxyConnector {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Socks5ProxyConnector")
            .field(
                "proxy_origin",
                &self.proxy_transport_url.origin().ascii_serialization(),
            )
            .field("has_tls_upgrader", &self.tls_upgrader.is_some())
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl ApiConnector for Socks5ProxyConnector {
    async fn connect(
        &self,
        target: ApiConnectionTarget<'_>,
        cancellation: &CancellationToken,
    ) -> Result<Box<dyn ApiConnection>, ApiConnectionError> {
        if cancellation.is_cancelled() {
            return Err(cancelled());
        }
        if !matches!(target.url.scheme(), "http" | "https") {
            return Err(proxy_error(
                "SOCKS5 proxy capability supports only http and https request origins",
            ));
        }
        if target.url.scheme() == "https" && self.tls_upgrader.is_none() {
            return Err(proxy_error(
                "HTTPS request origins require an injected tunnel TLS capability",
            ));
        }
        let mut connection = self
            .upstream
            .connect(
                ApiConnectionTarget {
                    url: &self.proxy_transport_url,
                    ignore_https_errors: false,
                    resolved_addresses: None,
                },
                cancellation,
            )
            .await?;

        connection
            .write_all(&[0x05, 0x01, 0x00], cancellation)
            .await?;
        let mut greeting = [0; 2];
        read_exact(&mut *connection, &mut greeting, cancellation).await?;
        if greeting != [0x05, 0x00] {
            return Err(proxy_error(format!(
                "SOCKS5 proxy rejected no-authentication method with response {:02x?}",
                greeting
            )));
        }

        let request = connect_request(target.url)?;
        connection.write_all(&request, cancellation).await?;
        read_connect_reply(&mut *connection, cancellation).await?;
        if target.url.scheme() == "https" {
            return self
                .tls_upgrader
                .as_ref()
                .expect("HTTPS target checked for TLS capability")
                .upgrade(connection, target, cancellation)
                .await;
        }
        Ok(connection)
    }
}

fn connect_request(url: &Url) -> Result<Vec<u8>, ApiConnectionError> {
    let port = url
        .port_or_known_default()
        .ok_or_else(|| proxy_error("request URL has no SOCKS5 tunnel port"))?;
    let mut request = vec![0x05, 0x01, 0x00];
    match url.host() {
        Some(Host::Domain(host)) => {
            if host.is_empty() || host.len() > MAX_DOMAIN_BYTES {
                return Err(proxy_error(
                    "SOCKS5 target domain must contain 1..=255 bytes",
                ));
            }
            request.extend_from_slice(&[0x03, host.len() as u8]);
            request.extend_from_slice(host.as_bytes());
        }
        Some(Host::Ipv4(address)) => {
            request.push(0x01);
            request.extend_from_slice(&address.octets());
        }
        Some(Host::Ipv6(address)) => {
            request.push(0x04);
            request.extend_from_slice(&address.octets());
        }
        None => return Err(proxy_error("request URL has no SOCKS5 tunnel host")),
    }
    request.extend_from_slice(&port.to_be_bytes());
    Ok(request)
}

async fn read_connect_reply(
    connection: &mut dyn ApiConnection,
    cancellation: &CancellationToken,
) -> Result<(), ApiConnectionError> {
    let mut head = [0; 4];
    read_exact(connection, &mut head, cancellation).await?;
    if head[0] != 0x05 || head[2] != 0x00 {
        return Err(proxy_error("invalid SOCKS5 CONNECT response header"));
    }
    if head[1] != 0x00 {
        return Err(proxy_error(format!(
            "SOCKS5 CONNECT failed with status {} ({})",
            head[1],
            reply_name(head[1])
        )));
    }
    let address_bytes = match head[3] {
        0x01 => 4,
        0x04 => 16,
        0x03 => {
            let mut length = [0; 1];
            read_exact(connection, &mut length, cancellation).await?;
            usize::from(length[0])
        }
        _ => return Err(proxy_error("invalid SOCKS5 CONNECT response address type")),
    };
    let mut address_and_port = vec![0; address_bytes + 2];
    read_exact(connection, &mut address_and_port, cancellation).await
}

async fn read_exact(
    connection: &mut dyn ApiConnection,
    output: &mut [u8],
    cancellation: &CancellationToken,
) -> Result<(), ApiConnectionError> {
    let mut offset = 0;
    while offset < output.len() {
        let bytes = connection.read(&mut output[offset..], cancellation).await?;
        if bytes == 0 {
            return Err(proxy_error(
                "SOCKS5 proxy closed the connection during handshake",
            ));
        }
        offset += bytes;
    }
    Ok(())
}

fn reply_name(reply: u8) -> &'static str {
    match reply {
        0x01 => "general server failure",
        0x02 => "connection not allowed",
        0x03 => "network unreachable",
        0x04 => "host unreachable",
        0x05 => "connection refused",
        0x06 => "TTL expired",
        0x07 => "command not supported",
        0x08 => "address type not supported",
        _ => "unknown error",
    }
}

fn proxy_error(message: impl Into<String>) -> ApiConnectionError {
    ApiConnectionError::new(ApiConnectionErrorKind::Proxy, message)
}

fn cancelled() -> ApiConnectionError {
    ApiConnectionError::new(
        ApiConnectionErrorKind::Cancelled,
        "API request was cancelled",
    )
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use pwright_api_request_transport::{
        ApiConnection, ApiConnectionError, ApiConnectionTarget, ApiConnector, ApiTunnelTlsUpgrader,
    };
    use tokio_util::sync::CancellationToken;
    use url::Url;

    use super::Socks5ProxyConnector;

    struct ScriptedConnection {
        writes: Arc<Mutex<Vec<Vec<u8>>>>,
        reads: Vec<Vec<u8>>,
    }

    #[async_trait]
    impl ApiConnection for ScriptedConnection {
        async fn write_all(
            &mut self,
            bytes: &[u8],
            _cancellation: &CancellationToken,
        ) -> Result<(), ApiConnectionError> {
            self.writes
                .lock()
                .expect("writes lock")
                .push(bytes.to_vec());
            Ok(())
        }

        async fn read(
            &mut self,
            output: &mut [u8],
            _cancellation: &CancellationToken,
        ) -> Result<usize, ApiConnectionError> {
            if self.reads.is_empty() {
                return Ok(0);
            }
            let bytes = self.reads.remove(0);
            let count = output.len().min(bytes.len());
            output[..count].copy_from_slice(&bytes[..count]);
            if count < bytes.len() {
                self.reads.insert(0, bytes[count..].to_vec());
            }
            Ok(count)
        }
    }

    struct ScriptedConnector {
        writes: Arc<Mutex<Vec<Vec<u8>>>>,
        targets: Arc<Mutex<Vec<String>>>,
        reads: Vec<Vec<u8>>,
    }

    #[async_trait]
    impl ApiConnector for ScriptedConnector {
        async fn connect(
            &self,
            target: ApiConnectionTarget<'_>,
            _cancellation: &CancellationToken,
        ) -> Result<Box<dyn ApiConnection>, ApiConnectionError> {
            self.targets
                .lock()
                .expect("targets lock")
                .push(target.url.to_string());
            Ok(Box::new(ScriptedConnection {
                writes: self.writes.clone(),
                reads: self.reads.clone(),
            }))
        }
    }

    struct Fixture {
        writes: Arc<Mutex<Vec<Vec<u8>>>>,
        targets: Arc<Mutex<Vec<String>>>,
        connector: Arc<ScriptedConnector>,
    }

    struct RecordingTlsUpgrader {
        targets: Arc<Mutex<Vec<(String, bool)>>>,
    }

    #[async_trait]
    impl ApiTunnelTlsUpgrader for RecordingTlsUpgrader {
        async fn upgrade(
            &self,
            connection: Box<dyn ApiConnection>,
            target: ApiConnectionTarget<'_>,
            _cancellation: &CancellationToken,
        ) -> Result<Box<dyn ApiConnection>, ApiConnectionError> {
            self.targets
                .lock()
                .expect("TLS targets lock")
                .push((target.url.to_string(), target.ignore_https_errors));
            Ok(connection)
        }
    }

    fn fixture(reply: &[u8]) -> Fixture {
        let writes = Arc::new(Mutex::new(Vec::new()));
        let targets = Arc::new(Mutex::new(Vec::new()));
        let connector = Arc::new(ScriptedConnector {
            writes: writes.clone(),
            targets: targets.clone(),
            reads: vec![vec![0x05, 0x00], reply.to_vec()],
        });
        Fixture {
            writes,
            targets,
            connector,
        }
    }

    #[tokio::test]
    async fn domain_target_uses_remote_resolution_and_establishes_tunnel() {
        // Arrange
        let fixture = fixture(&[0x05, 0x00, 0x00, 0x01, 127, 0, 0, 1, 0x12, 0x34]);
        let connector =
            Socks5ProxyConnector::new(fixture.connector, "socks5://proxy.test:1080", None, None)
                .expect("SOCKS5 connector should build");
        let target = Url::parse("http://non-existent.test:8080/path").expect("target URL");

        // Act
        connector
            .connect(
                ApiConnectionTarget {
                    url: &target,
                    ignore_https_errors: false,
                    resolved_addresses: None,
                },
                &CancellationToken::new(),
            )
            .await
            .expect("SOCKS5 tunnel should connect");

        // Assert
        assert_eq!(
            fixture.targets.lock().expect("targets lock").as_slice(),
            ["http://proxy.test:1080/"]
        );
        assert_eq!(
            fixture.writes.lock().expect("writes lock").as_slice(),
            [
                vec![0x05, 0x01, 0x00],
                [
                    &[0x05, 0x01, 0x00, 0x03, 17][..],
                    &b"non-existent.test"[..],
                    &[0x1f, 0x90][..],
                ]
                .concat(),
            ]
        );
    }

    #[tokio::test]
    async fn ipv4_target_uses_binary_address_form() {
        // Arrange
        let fixture = fixture(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]);
        let connector =
            Socks5ProxyConnector::new(fixture.connector, "socks5h://proxy.test:1080", None, None)
                .expect("SOCKS5 connector should build");
        let target = Url::parse("http://192.0.2.10:81/").expect("target URL");

        // Act
        connector
            .connect(
                ApiConnectionTarget {
                    url: &target,
                    ignore_https_errors: false,
                    resolved_addresses: None,
                },
                &CancellationToken::new(),
            )
            .await
            .expect("SOCKS5 tunnel should connect");

        // Assert
        assert_eq!(
            fixture.writes.lock().expect("writes lock")[1],
            [0x05, 0x01, 0x00, 0x01, 192, 0, 2, 10, 0, 81]
        );
    }

    #[tokio::test]
    async fn ipv6_target_uses_binary_address_form() {
        // Arrange
        let fixture = fixture(&[
            0x05, 0x00, 0x00, 0x04, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        let connector =
            Socks5ProxyConnector::new(fixture.connector, "socks5://proxy.test:1080", None, None)
                .expect("SOCKS5 connector should build");
        let target = Url::parse("http://[2001:db8::1]:82/").expect("target URL");

        // Act
        connector
            .connect(
                ApiConnectionTarget {
                    url: &target,
                    ignore_https_errors: false,
                    resolved_addresses: None,
                },
                &CancellationToken::new(),
            )
            .await
            .expect("SOCKS5 tunnel should connect");

        // Assert
        let request = &fixture.writes.lock().expect("writes lock")[1];
        assert_eq!(&request[..4], &[0x05, 0x01, 0x00, 0x04]);
        let expected = "2001:db8::1"
            .parse::<std::net::Ipv6Addr>()
            .expect("IPv6")
            .octets();
        assert_eq!(&request[4..20], &expected);
        assert_eq!(&request[20..], &[0, 82]);
    }

    #[tokio::test]
    async fn failed_connect_reply_is_a_proxy_error() {
        // Arrange
        let fixture = fixture(&[0x05, 0x05, 0x00, 0x01]);
        let connector =
            Socks5ProxyConnector::new(fixture.connector, "socks5://proxy.test:1080", None, None)
                .expect("SOCKS5 connector should build");
        let target = Url::parse("http://example.test/").expect("target URL");

        // Act
        let result = connector
            .connect(
                ApiConnectionTarget {
                    url: &target,
                    ignore_https_errors: false,
                    resolved_addresses: None,
                },
                &CancellationToken::new(),
            )
            .await;
        let Err(error) = result else {
            panic!("connection-refused reply should fail");
        };

        // Assert
        assert_eq!(
            error.kind,
            pwright_api_request_transport::ApiConnectionErrorKind::Proxy
        );
        assert_eq!(
            error.message,
            "SOCKS5 CONNECT failed with status 5 (connection refused)"
        );
    }

    #[test]
    fn proxy_credentials_are_explicitly_rejected() {
        // Arrange
        let fixture = fixture(&[]);

        // Act
        let result = Socks5ProxyConnector::new(
            fixture.connector,
            "socks5://proxy.test:1080",
            Some("user"),
            Some("secret"),
        );
        let Err(error) = result else {
            panic!("SOCKS5 credentials should fail explicitly");
        };

        // Assert
        assert!(error.message.contains("authentication is not supported"));
    }

    #[tokio::test]
    async fn https_target_delegates_tls_after_socks_handshake() {
        // Arrange
        let fixture = fixture(&[0x05, 0x00, 0x00, 0x01, 127, 0, 0, 1, 0, 0]);
        let tls_targets = Arc::new(Mutex::new(Vec::new()));
        let connector =
            Socks5ProxyConnector::new(fixture.connector, "socks5://proxy.test:1080", None, None)
                .expect("SOCKS5 connector should build")
                .with_tls_upgrader(Arc::new(RecordingTlsUpgrader {
                    targets: tls_targets.clone(),
                }));
        let target = Url::parse("https://example.test/path").expect("target URL");

        // Act
        connector
            .connect(
                ApiConnectionTarget {
                    url: &target,
                    ignore_https_errors: true,
                    resolved_addresses: None,
                },
                &CancellationToken::new(),
            )
            .await
            .expect("HTTPS tunnel should compose TLS");

        // Assert
        assert_eq!(
            tls_targets.lock().expect("TLS targets lock").as_slice(),
            [("https://example.test/path".to_owned(), true)]
        );
        assert_eq!(
            fixture.writes.lock().expect("writes lock")[1],
            [
                &[0x05, 0x01, 0x00, 0x03, 12][..],
                &b"example.test"[..],
                &[0x01, 0xbb][..],
            ]
            .concat()
        );
    }

    #[tokio::test]
    async fn https_request_origin_requires_an_injected_tls_capability() {
        // Arrange
        let fixture = fixture(&[]);
        let connector =
            Socks5ProxyConnector::new(fixture.connector, "socks5://proxy.test:1080", None, None)
                .expect("SOCKS5 connector should build");
        let target = Url::parse("https://example.test/").expect("target URL");

        // Act
        let result = connector
            .connect(
                ApiConnectionTarget {
                    url: &target,
                    ignore_https_errors: false,
                    resolved_addresses: None,
                },
                &CancellationToken::new(),
            )
            .await;
        let Err(error) = result else {
            panic!("HTTPS tunnel composition is not selected");
        };

        // Assert
        assert!(error.message.contains("injected tunnel TLS capability"));
    }
}
