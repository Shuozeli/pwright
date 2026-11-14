//! Optional bounded HTTP CONNECT proxy connector for Pwright API requests.
//!
//! This leaf owns proxy parsing, credentials and tunnel establishment only. It
//! wraps any connector capable of reaching the proxy. The proxy itself may be
//! `http:` or `https:` because the injected connector owns that outer
//! transport. HTTPS request origins require a separately injected tunnel TLS
//! upgrader, preserving the proxy leaf's independence from a concrete TLS
//! stack.

use std::{cmp::min, sync::Arc};

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use pwright_api_request_transport::{
    ApiConnection, ApiConnectionError, ApiConnectionErrorKind, ApiConnectionTarget, ApiConnector,
    ApiTunnelTlsUpgrader,
};
use tokio_util::sync::CancellationToken;
use url::{Host, Url};
use zeroize::Zeroizing;

const MAX_PROXY_URL_BYTES: usize = 64 * 1024;
const MAX_CREDENTIAL_BYTES: usize = 4 * 1024;
const MAX_CONNECT_HEAD_BYTES: usize = 32 * 1024;
const READ_BUFFER_BYTES: usize = 4 * 1024;

/// A connector wrapper that establishes one HTTP CONNECT tunnel per exchange.
pub struct HttpConnectProxyConnector {
    upstream: Arc<dyn ApiConnector>,
    tls_upgrader: Option<Arc<dyn ApiTunnelTlsUpgrader>>,
    proxy_url: Url,
    authorization: Option<Zeroizing<String>>,
}

impl HttpConnectProxyConnector {
    /// Create one bounded HTTP CONNECT proxy capability.
    ///
    /// A missing scheme is interpreted as `http://`, matching Playwright's
    /// public proxy option. HTTP and HTTPS proxy servers are accepted; SOCKS
    /// proxy servers stay outside this independently selectable leaf.
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
        if username.is_some_and(|value| value.len() > MAX_CREDENTIAL_BYTES) {
            return Err(proxy_error("proxy username exceeds its configured limit"));
        }
        if password.is_some_and(|value| value.len() > MAX_CREDENTIAL_BYTES) {
            return Err(proxy_error("proxy password exceeds its configured limit"));
        }
        let value = if server.contains("://") {
            server.to_owned()
        } else {
            format!("http://{server}")
        };
        let proxy_url = Url::parse(&value)
            .map_err(|error| proxy_error(format!("invalid proxy server: {error}")))?;
        if !matches!(proxy_url.scheme(), "http" | "https") {
            return Err(proxy_error(
                "HTTP CONNECT proxy capability supports only http and https proxy servers",
            ));
        }
        if proxy_url.host().is_none() || proxy_url.port_or_known_default().is_none() {
            return Err(proxy_error("proxy server must include a host and port"));
        }
        if proxy_url.username() != "" || proxy_url.password().is_some() {
            return Err(proxy_error(
                "proxy URL credentials must use the separate username and password options",
            ));
        }
        let authorization = (username.is_some() || password.is_some()).then(|| {
            let joined = Zeroizing::new(format!(
                "{}:{}",
                username.unwrap_or_default(),
                password.unwrap_or_default()
            ));
            Zeroizing::new(format!("Basic {}", STANDARD.encode(joined.as_bytes())))
        });
        Ok(Self {
            upstream,
            tls_upgrader: None,
            proxy_url,
            authorization,
        })
    }

    /// Compose target-origin TLS over the established CONNECT tunnel.
    #[must_use]
    pub fn with_tls_upgrader(mut self, tls_upgrader: Arc<dyn ApiTunnelTlsUpgrader>) -> Self {
        self.tls_upgrader = Some(tls_upgrader);
        self
    }
}

impl std::fmt::Debug for HttpConnectProxyConnector {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HttpConnectProxyConnector")
            .field(
                "proxy_origin",
                &self.proxy_url.origin().ascii_serialization(),
            )
            .field(
                "authorization",
                &self.authorization.as_ref().map(|_| "[REDACTED]"),
            )
            .field("has_tls_upgrader", &self.tls_upgrader.is_some())
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl ApiConnector for HttpConnectProxyConnector {
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
                "HTTP CONNECT proxy capability supports only http and https request origins",
            ));
        }
        if target.url.scheme() == "https" && self.tls_upgrader.is_none() {
            return Err(proxy_error(
                "HTTPS request origins require an injected tunnel TLS capability",
            ));
        }
        let authority = target_authority(target.url)?;
        let mut connection = self
            .upstream
            .connect(
                ApiConnectionTarget {
                    url: &self.proxy_url,
                    ignore_https_errors: target.ignore_https_errors,
                    resolved_addresses: None,
                },
                cancellation,
            )
            .await?;
        let mut request = format!(
            "CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\nProxy-Connection: keep-alive\r\n"
        );
        if let Some(authorization) = &self.authorization {
            request.push_str("Proxy-Authorization: ");
            request.push_str(authorization);
            request.push_str("\r\n");
        }
        request.push_str("\r\n");
        connection
            .write_all(request.as_bytes(), cancellation)
            .await?;
        let prefix = read_connect_response(&mut *connection, cancellation).await?;
        let tunnel: Box<dyn ApiConnection> = Box::new(PrefixedConnection {
            connection,
            prefix,
            offset: 0,
        });
        if target.url.scheme() == "https" {
            return self
                .tls_upgrader
                .as_ref()
                .expect("HTTPS target checked for TLS capability")
                .upgrade(tunnel, target, cancellation)
                .await;
        }
        Ok(tunnel)
    }
}

async fn read_connect_response(
    connection: &mut dyn ApiConnection,
    cancellation: &CancellationToken,
) -> Result<Vec<u8>, ApiConnectionError> {
    let mut head = Vec::new();
    let mut chunk = [0; READ_BUFFER_BYTES];
    let header_end = loop {
        if head.len() >= MAX_CONNECT_HEAD_BYTES {
            return Err(proxy_error(
                "proxy CONNECT response headers exceeded 32 KiB",
            ));
        }
        let bytes = connection.read(&mut chunk, cancellation).await?;
        if bytes == 0 {
            return Err(proxy_error(
                "proxy closed the connection before completing CONNECT",
            ));
        }
        head.extend_from_slice(&chunk[..bytes]);
        if let Some(end) = head.windows(4).position(|window| window == b"\r\n\r\n") {
            let header_end = end + 4;
            if header_end > MAX_CONNECT_HEAD_BYTES {
                return Err(proxy_error(
                    "proxy CONNECT response headers exceeded 32 KiB",
                ));
            }
            break header_end;
        }
        if head.len() >= MAX_CONNECT_HEAD_BYTES {
            return Err(proxy_error(
                "proxy CONNECT response headers exceeded 32 KiB",
            ));
        }
    };
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut response = httparse::Response::new(&mut headers);
    match response.parse(&head[..header_end]) {
        Ok(httparse::Status::Complete(_)) => {}
        Ok(httparse::Status::Partial) => {
            return Err(proxy_error("proxy CONNECT response headers are incomplete"));
        }
        Err(error) => {
            return Err(proxy_error(format!(
                "invalid proxy CONNECT response: {error}"
            )));
        }
    }
    let status = response
        .code
        .ok_or_else(|| proxy_error("proxy CONNECT response has no status"))?;
    if status != 200 {
        return Err(proxy_error(format!(
            "proxy CONNECT failed with status {status}"
        )));
    }
    Ok(head.split_off(header_end))
}

fn target_authority(url: &Url) -> Result<String, ApiConnectionError> {
    let host = match url.host() {
        Some(Host::Domain(host)) => host.to_owned(),
        Some(Host::Ipv4(host)) => host.to_string(),
        Some(Host::Ipv6(host)) => format!("[{host}]"),
        None => return Err(proxy_error("request URL has no proxy tunnel host")),
    };
    let port = url
        .port_or_known_default()
        .ok_or_else(|| proxy_error("request URL has no proxy tunnel port"))?;
    let authority = format!("{host}:{port}");
    if authority.len() > MAX_PROXY_URL_BYTES {
        return Err(proxy_error(
            "proxy tunnel authority exceeds its configured limit",
        ));
    }
    Ok(authority)
}

struct PrefixedConnection {
    connection: Box<dyn ApiConnection>,
    prefix: Vec<u8>,
    offset: usize,
}

#[async_trait]
impl ApiConnection for PrefixedConnection {
    async fn write_all(
        &mut self,
        bytes: &[u8],
        cancellation: &CancellationToken,
    ) -> Result<(), ApiConnectionError> {
        self.connection.write_all(bytes, cancellation).await
    }

    async fn read(
        &mut self,
        output: &mut [u8],
        cancellation: &CancellationToken,
    ) -> Result<usize, ApiConnectionError> {
        if self.offset < self.prefix.len() {
            let bytes = min(output.len(), self.prefix.len() - self.offset);
            output[..bytes].copy_from_slice(&self.prefix[self.offset..self.offset + bytes]);
            self.offset += bytes;
            return Ok(bytes);
        }
        self.connection.read(output, cancellation).await
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

    use super::HttpConnectProxyConnector;

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
        targets: Arc<Mutex<Vec<(String, bool)>>>,
        response: Vec<u8>,
    }

    struct Fixture {
        writes: Arc<Mutex<Vec<Vec<u8>>>>,
        targets: Arc<Mutex<Vec<(String, bool)>>>,
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
                .push((target.url.to_string(), target.ignore_https_errors));
            Ok(Box::new(ScriptedConnection {
                writes: self.writes.clone(),
                reads: vec![self.response.clone()],
            }))
        }
    }

    fn fixture(response: &[u8]) -> Fixture {
        let writes = Arc::new(Mutex::new(Vec::new()));
        let targets = Arc::new(Mutex::new(Vec::new()));
        let connector = Arc::new(ScriptedConnector {
            writes: writes.clone(),
            targets: targets.clone(),
            response: response.to_vec(),
        });
        Fixture {
            writes,
            targets,
            connector,
        }
    }

    #[tokio::test]
    async fn connect_establishes_an_authenticated_tunnel_and_preserves_prefetched_bytes() {
        // Arrange
        let fixture =
            fixture(b"HTTP/1.1 200 Connection Established\r\nProxy-Agent: test\r\n\r\nprefetched");
        let connector = HttpConnectProxyConnector::new(
            fixture.connector,
            "proxy.test:3128",
            Some("user"),
            Some("secret"),
        )
        .expect("proxy connector should build");
        let target = Url::parse("http://example.test/path").expect("target URL");
        let cancellation = CancellationToken::new();

        // Act
        let mut connection = connector
            .connect(
                ApiConnectionTarget {
                    url: &target,
                    ignore_https_errors: false,
                    resolved_addresses: None,
                },
                &cancellation,
            )
            .await
            .expect("tunnel should connect");
        let mut output = [0; 16];
        let bytes = connection
            .read(&mut output, &cancellation)
            .await
            .expect("prefetched data should read");

        // Assert
        assert_eq!(
            fixture.targets.lock().expect("targets lock").as_slice(),
            [("http://proxy.test:3128/".to_owned(), false)]
        );
        let request = String::from_utf8(fixture.writes.lock().expect("writes lock")[0].clone())
            .expect("CONNECT should be UTF-8");
        assert!(request.starts_with("CONNECT example.test:80 HTTP/1.1\r\n"));
        assert!(request.contains("Proxy-Authorization: Basic dXNlcjpzZWNyZXQ=\r\n"));
        assert_eq!(&output[..bytes], b"prefetched");
    }

    #[tokio::test]
    async fn non_successful_connect_status_is_rejected_as_proxy_error() {
        // Arrange
        let fixture =
            fixture(b"HTTP/1.1 407 Proxy Authentication Required\r\nContent-Length: 0\r\n\r\n");
        let connector =
            HttpConnectProxyConnector::new(fixture.connector, "http://proxy.test:3128", None, None)
                .expect("proxy connector should build");
        let target = Url::parse("http://example.test/").expect("target URL");
        let cancellation = CancellationToken::new();

        // Act
        let result = connector
            .connect(
                ApiConnectionTarget {
                    url: &target,
                    ignore_https_errors: false,
                    resolved_addresses: None,
                },
                &cancellation,
            )
            .await;
        let Err(error) = result else {
            panic!("407 should fail");
        };

        // Assert
        assert_eq!(
            error.kind,
            pwright_api_request_transport::ApiConnectionErrorKind::Proxy
        );
        assert_eq!(error.message, "proxy CONNECT failed with status 407");
    }

    #[tokio::test]
    async fn https_target_delegates_tls_after_connect() {
        // Arrange
        let fixture = fixture(b"HTTP/1.1 200 Connection Established\r\n\r\n");
        let tls_targets = Arc::new(Mutex::new(Vec::new()));
        let connector =
            HttpConnectProxyConnector::new(fixture.connector, "proxy.test:3128", None, None)
                .expect("proxy connector should build")
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
        let request = String::from_utf8(fixture.writes.lock().expect("writes lock")[0].clone())
            .expect("CONNECT should be UTF-8");
        assert!(request.starts_with("CONNECT example.test:443 HTTP/1.1\r\n"));
        assert_eq!(
            tls_targets.lock().expect("TLS targets lock").as_slice(),
            [("https://example.test/path".to_owned(), true)]
        );
    }

    #[tokio::test]
    async fn https_request_origin_requires_an_injected_tls_capability() {
        // Arrange
        let fixture = fixture(b"");
        let connector =
            HttpConnectProxyConnector::new(fixture.connector, "proxy.test:3128", None, None)
                .expect("proxy connector should build");
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
        assert_eq!(
            error.kind,
            pwright_api_request_transport::ApiConnectionErrorKind::Proxy
        );
        assert!(error.message.contains("injected tunnel TLS capability"));
    }

    #[tokio::test]
    async fn https_proxy_delegates_outer_tls_and_ignore_policy_to_upstream_connector() {
        // Arrange
        let fixture = fixture(b"HTTP/1.1 200 Connection Established\r\n\r\n");
        let connector = HttpConnectProxyConnector::new(
            fixture.connector,
            "https://proxy.test:8443",
            None,
            None,
        )
        .expect("HTTPS proxy connector should build");
        let target = Url::parse("http://example.test/").expect("target URL");

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
            .expect("outer TLS connector should establish the tunnel");

        // Assert
        assert_eq!(
            fixture.targets.lock().expect("targets lock").as_slice(),
            [("https://proxy.test:8443/".to_owned(), true)]
        );
    }

    #[tokio::test]
    async fn password_without_username_still_emits_basic_proxy_authorization() {
        // Arrange
        let fixture = fixture(b"HTTP/1.1 200 Connection Established\r\n\r\n");
        let connector = HttpConnectProxyConnector::new(
            fixture.connector,
            "proxy.test:3128",
            None,
            Some("secret"),
        )
        .expect("proxy connector should build");
        let target = Url::parse("http://example.test/").expect("target URL");

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
            .expect("tunnel should connect");

        // Assert
        let request = String::from_utf8(fixture.writes.lock().expect("writes lock")[0].clone())
            .expect("CONNECT should be UTF-8");
        assert!(request.contains("Proxy-Authorization: Basic OnNlY3JldA==\r\n"));
    }

    #[tokio::test]
    async fn connect_response_header_terminator_after_limit_is_rejected() {
        // Arrange
        let mut response = b"HTTP/1.1 200 Connection Established\r\nX-Fill: ".to_vec();
        response.resize(super::MAX_CONNECT_HEAD_BYTES + 1, b'a');
        response.extend_from_slice(b"\r\n\r\n");
        let fixture = fixture(&response);
        let connector =
            HttpConnectProxyConnector::new(fixture.connector, "proxy.test:3128", None, None)
                .expect("proxy connector should build");
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
            panic!("oversized CONNECT response should fail");
        };

        // Assert
        assert_eq!(
            error.kind,
            pwright_api_request_transport::ApiConnectionErrorKind::Proxy
        );
        assert_eq!(
            error.message,
            "proxy CONNECT response headers exceeded 32 KiB"
        );
    }

    #[test]
    fn credentials_are_bounded_and_redacted() {
        // Arrange
        let fixture = fixture(b"");

        // Act
        let connector = HttpConnectProxyConnector::new(
            fixture.connector,
            "proxy.test:3128",
            Some("secret-user"),
            Some("secret-password"),
        )
        .expect("proxy connector should build");
        let debug = format!("{connector:?}");

        // Assert
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("secret-user"));
        assert!(!debug.contains("secret-password"));
    }
}
