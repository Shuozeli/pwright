#[cfg(feature = "cdp-handshake-headers")]
use pwright_cdp::CdpHandshakeHeaders;
use pwright_provider_cdp::CdpWebSocketEndpoint;
use serde::Deserialize;
use url::Url;

use crate::HostError;

const MAX_ENDPOINT_BYTES: usize = 8192;
const MAX_DISCOVERY_BYTES: usize = 64 * 1024;

#[cfg(feature = "cdp-discovery-errors")]
fn unexpected_discovery_status(status: reqwest::StatusCode, url: &Url) -> HostError {
    // Playwright 1.58.0 includes the full discovery URL, including its query,
    // in this opt-in diagnostic. Query tokens can be exposed to error consumers;
    // never write this URL to a separate log.
    HostError::backend(format_args!(
        "Unexpected status {} when connecting to {}.\nThis does not look like a DevTools server, try connecting via ws://.",
        status.as_u16(),
        url
    ))
}

#[derive(Deserialize)]
struct Discovery {
    #[serde(rename = "webSocketDebuggerUrl")]
    #[cfg(not(feature = "cdp-discovery-errors"))]
    websocket: String,
    #[serde(rename = "webSocketDebuggerUrl")]
    #[cfg(feature = "cdp-discovery-errors")]
    websocket: Option<String>,
}

fn parse_discovery_websocket(bytes: &[u8]) -> Result<String, HostError> {
    let discovery: Discovery = serde_json::from_slice(bytes).map_err(HostError::backend)?;
    #[cfg(feature = "cdp-discovery-errors")]
    let websocket = discovery
        .websocket
        .ok_or_else(|| HostError::backend("Invalid URL"))?;
    #[cfg(not(feature = "cdp-discovery-errors"))]
    let websocket = discovery.websocket;
    Ok(websocket)
}

pub fn validate_endpoint(endpoint: &str) -> Result<Url, HostError> {
    if endpoint.len() > MAX_ENDPOINT_BYTES {
        return Err(HostError::InvalidArgument(
            "CDP endpoint exceeds 8192 bytes".into(),
        ));
    }
    let url = Url::parse(endpoint).map_err(|_| {
        HostError::InvalidArgument("CDP endpoint must be an absolute HTTP(S) or WS(S) URL".into())
    })?;
    if !matches!(url.scheme(), "http" | "https" | "ws" | "wss") || url.host_str().is_none() {
        return Err(HostError::InvalidArgument(
            "CDP endpoint must use HTTP(S) or WS(S)".into(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        return Err(HostError::InvalidArgument(
            "CDP endpoint credentials and fragments are unsupported".into(),
        ));
    }
    Ok(url)
}

pub async fn resolve(endpoint: &str) -> Result<CdpWebSocketEndpoint, HostError> {
    resolve_inner(
        endpoint,
        #[cfg(feature = "cdp-handshake-headers")]
        None,
    )
    .await
}

#[cfg(feature = "cdp-handshake-headers")]
pub async fn resolve_with_headers(
    endpoint: &str,
    headers: &CdpHandshakeHeaders,
) -> Result<CdpWebSocketEndpoint, HostError> {
    resolve_inner(endpoint, Some(headers)).await
}

async fn resolve_inner(
    endpoint: &str,
    #[cfg(feature = "cdp-handshake-headers")] headers: Option<&CdpHandshakeHeaders>,
) -> Result<CdpWebSocketEndpoint, HostError> {
    let mut url = validate_endpoint(endpoint)?;
    if matches!(url.scheme(), "ws" | "wss") {
        return CdpWebSocketEndpoint::parse(url.as_str()).map_err(HostError::backend);
    }
    let path = format!("{}/json/version/", url.path().trim_end_matches('/'));
    url.set_path(&path);
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(HostError::backend)?;
    let request = client.get(url);
    #[cfg(feature = "cdp-handshake-headers")]
    let request = if let Some(headers) = headers {
        headers.iter().fold(request, |request, (name, value)| {
            request.header(name.clone(), value.clone())
        })
    } else {
        request
    };
    let response = request
        .send()
        .await
        .map_err(|error| HostError::backend(error.without_url()))?;
    #[cfg(feature = "cdp-discovery-errors")]
    let mut response = {
        if response.status() != reqwest::StatusCode::OK {
            return Err(unexpected_discovery_status(
                response.status(),
                response.url(),
            ));
        }
        response
    };
    #[cfg(not(feature = "cdp-discovery-errors"))]
    let mut response = response
        .error_for_status()
        .map_err(|error| HostError::backend(error.without_url()))?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(HostError::backend)? {
        if chunk.len() > MAX_DISCOVERY_BYTES.saturating_sub(bytes.len()) {
            return Err(HostError::ResourceExhausted(
                "CDP discovery response exceeds 64 KiB".into(),
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    let websocket = parse_discovery_websocket(&bytes)?;
    let websocket = validate_endpoint(&websocket)?;
    CdpWebSocketEndpoint::parse(websocket.as_str()).map_err(HostError::backend)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::net::{Ipv4Addr, SocketAddr};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[cfg(feature = "cdp-handshake-headers")]
    fn one_shot_fixture_address() -> (Ipv4Addr, String) {
        match (
            std::env::var("TAILSCALE_IP"),
            std::env::var("TAILSCALE_HOST"),
        ) {
            (Ok(ip), Ok(host)) => {
                let bind_ip = ip.parse::<Ipv4Addr>().expect("TAILSCALE_IP must be IPv4");
                assert!(
                    host.ends_with(".ts.net") && !host.contains('/') && !host.contains(':'),
                    "TAILSCALE_HOST must be a full MagicDNS hostname"
                );
                (bind_ip, host)
            }
            (Err(std::env::VarError::NotPresent), Err(std::env::VarError::NotPresent)) => {
                // Portable CI without Tailscale: this one-shot test fixture
                // stays on loopback and never exposes a wildcard port.
                (Ipv4Addr::LOCALHOST, Ipv4Addr::LOCALHOST.to_string())
            }
            _ => panic!("TAILSCALE_IP and TAILSCALE_HOST must be set together"),
        }
    }

    fn tailscale_fixture_address() -> Option<(Ipv4Addr, String)> {
        let (Ok(ip), Ok(host)) = (
            std::env::var("TAILSCALE_IP"),
            std::env::var("TAILSCALE_HOST"),
        ) else {
            return None;
        };
        let bind_ip = ip.parse::<Ipv4Addr>().expect("TAILSCALE_IP must be IPv4");
        assert!(
            host.ends_with(".ts.net") && !host.contains('/') && !host.contains(':'),
            "TAILSCALE_HOST must be a full MagicDNS hostname"
        );
        Some((bind_ip, host))
    }

    #[test]
    fn credentials_are_rejected_without_echoing_the_secret() {
        // Arrange
        let endpoint = "http://user:private-value@127.0.0.1:9222";
        // Act
        let error = validate_endpoint(endpoint).expect_err("credentials must fail");
        // Assert
        assert!(matches!(error, HostError::InvalidArgument(_)));
        assert!(!error.to_string().contains("private-value"));
    }

    #[test]
    fn oversized_endpoint_is_rejected_before_url_parsing() {
        // Arrange
        let endpoint = "x".repeat(MAX_ENDPOINT_BYTES + 1);
        // Act
        let result = validate_endpoint(&endpoint);
        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    }

    #[cfg(feature = "cdp-discovery-errors")]
    #[test]
    fn non_200_status_uses_bounded_full_discovery_url() {
        // Arrange
        let url = Url::parse("http://example.test/base/json/version/?token=private-value")
            .expect("valid fixture URL");

        // Act
        let error = unexpected_discovery_status(reqwest::StatusCode::NOT_FOUND, &url);

        // Assert
        assert_eq!(
            error.to_string(),
            "Unexpected status 404 when connecting to http://example.test/base/json/version/?token=private-value.\nThis does not look like a DevTools server, try connecting via ws://."
        );
        assert!(error.to_string().len() <= 4096);
    }

    #[cfg(feature = "cdp-discovery-errors")]
    #[test]
    fn created_status_is_not_treated_as_discovery_success() {
        // Arrange
        let url = Url::parse("http://example.test/json/version/").expect("valid fixture URL");

        // Act
        let error = unexpected_discovery_status(reqwest::StatusCode::CREATED, &url);

        // Assert
        assert!(
            error
                .to_string()
                .starts_with("Unexpected status 201 when connecting to ")
        );
    }

    #[cfg(feature = "cdp-discovery-errors")]
    #[test]
    fn redirect_status_is_not_followed_by_discovery() {
        // Arrange
        let url = Url::parse("http://example.test/json/version/").expect("valid fixture URL");

        // Act
        let error = unexpected_discovery_status(reqwest::StatusCode::FOUND, &url);

        // Assert
        assert!(
            error
                .to_string()
                .starts_with("Unexpected status 302 when connecting to ")
        );
    }

    #[cfg(feature = "cdp-discovery-errors")]
    #[test]
    fn long_status_diagnostic_stays_within_backend_error_budget() {
        // Arrange
        let raw = format!("http://example.test/{}", "x".repeat(6000));
        let url = validate_endpoint(&raw).expect("bounded endpoint is valid");

        // Act
        let error = unexpected_discovery_status(reqwest::StatusCode::NOT_FOUND, &url);

        // Assert
        assert_eq!(error.to_string().len(), 4096);
    }

    #[cfg(feature = "cdp-discovery-errors")]
    #[test]
    fn missing_websocket_url_uses_invalid_url_error() {
        // Arrange
        let body = br#"{}"#;

        // Act
        let error = parse_discovery_websocket(body).expect_err("field is absent");

        // Assert
        assert_eq!(error.to_string(), "Invalid URL");
    }

    #[cfg(feature = "cdp-discovery-errors")]
    #[test]
    fn malformed_json_keeps_parse_diagnostic() {
        // Arrange
        let body = b"{";

        // Act
        let error = parse_discovery_websocket(body).expect_err("JSON is malformed");

        // Assert
        assert_ne!(error.to_string(), "Invalid URL");
        assert!(error.to_string().contains("EOF"));
    }

    #[cfg(not(feature = "cdp-discovery-errors"))]
    #[test]
    fn default_missing_websocket_field_error_is_unchanged() {
        // Arrange
        let body = br#"{}"#;

        // Act
        let error = parse_discovery_websocket(body).expect_err("field is absent");

        // Assert
        assert!(
            error
                .to_string()
                .contains("missing field `webSocketDebuggerUrl`")
        );
    }

    async fn one_shot_discovery_error(
        status: u16,
        body: &'static str,
        query: &str,
    ) -> Option<(HostError, String, String)> {
        // The real-network fixture is Tailscale-only. Pure tests above run in
        // portable environments without opening a loopback or wildcard server.
        let (bind_ip, client_host) = tailscale_fixture_address()?;
        let listener = tokio::net::TcpListener::bind(SocketAddr::from((bind_ip, 0)))
            .await
            .expect("test listener should bind");
        let port = listener.local_addr().expect("bound address").port();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("client should connect");
            let mut request = Vec::new();
            loop {
                let mut chunk = [0_u8; 1024];
                let count = stream.read(&mut chunk).await.expect("request should read");
                assert!(count > 0, "request must have headers");
                request.extend_from_slice(&chunk[..count]);
                assert!(request.len() <= 8192, "request header must stay bounded");
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let response = format!(
                "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("response should write");
            String::from_utf8(request).expect("HTTP request should be UTF-8")
        });
        let endpoint = format!("http://{client_host}:{port}/base{query}");
        let result = tokio::time::timeout(std::time::Duration::from_secs(10), resolve(&endpoint))
            .await
            .expect("discovery should not hang")
            .expect_err("fixture response must fail discovery");
        let request = server.await.expect("server task should retire");
        Some((result, endpoint, request))
    }

    #[cfg(feature = "cdp-discovery-errors")]
    #[tokio::test]
    async fn discovery_non_200_reports_playwright_status_and_full_url() {
        // Arrange
        let query = "?token=private-value";

        // Act
        let Some((error, endpoint, request)) =
            one_shot_discovery_error(404, r#"{"webSocketDebuggerUrl":"dont-use-me"}"#, query).await
        else {
            return;
        };

        // Assert
        let expected = format!(
            "Unexpected status 404 when connecting to {}/json/version/{query}.\nThis does not look like a DevTools server, try connecting via ws://.",
            endpoint.trim_end_matches(query)
        );
        assert_eq!(error.to_string(), expected);
        assert!(request.starts_with("GET /base/json/version/?token=private-value HTTP/1.1\r\n"));
    }

    #[cfg(feature = "cdp-discovery-errors")]
    #[tokio::test]
    async fn discovery_success_status_other_than_200_is_rejected() {
        // Arrange
        let status = 201;

        // Act
        let Some((error, _, _)) = one_shot_discovery_error(status, "{}", "").await else {
            return;
        };

        // Assert
        assert!(
            error
                .to_string()
                .starts_with("Unexpected status 201 when connecting to ")
        );
    }

    #[cfg(feature = "cdp-discovery-errors")]
    #[tokio::test]
    async fn discovery_redirect_is_rejected_as_non_200() {
        // Arrange
        let status = 302;

        // Act
        let Some((error, _, _)) = one_shot_discovery_error(status, "{}", "").await else {
            return;
        };

        // Assert
        assert!(
            error
                .to_string()
                .starts_with("Unexpected status 302 when connecting to ")
        );
    }

    #[cfg(feature = "cdp-discovery-errors")]
    #[tokio::test]
    async fn discovery_missing_websocket_url_reports_invalid_url() {
        // Arrange
        let body = "{}";

        // Act
        let Some((error, _, _)) = one_shot_discovery_error(200, body, "").await else {
            return;
        };

        // Assert
        assert_eq!(error.to_string(), "Invalid URL");
    }

    #[cfg(feature = "cdp-discovery-errors")]
    #[tokio::test]
    async fn discovery_malformed_json_keeps_its_parse_error() {
        // Arrange
        let body = "{";

        // Act
        let Some((error, _, _)) = one_shot_discovery_error(200, body, "").await else {
            return;
        };

        // Assert
        assert_ne!(error.to_string(), "Invalid URL");
        assert!(error.to_string().contains("EOF"));
    }

    #[cfg(not(feature = "cdp-discovery-errors"))]
    #[tokio::test]
    async fn default_status_error_is_unchanged() {
        // Arrange
        let body = "{}";

        // Act
        let Some((status_error, _, _)) = one_shot_discovery_error(404, body, "").await else {
            return;
        };

        // Assert
        assert!(
            status_error
                .to_string()
                .starts_with("HTTP status client error (404")
        );
        assert!(!status_error.to_string().contains("Unexpected status 404"));
    }

    #[cfg(feature = "cdp-handshake-headers")]
    #[tokio::test]
    async fn discovery_sends_custom_header_to_fake_http_endpoint() {
        // Arrange
        let (bind_ip, client_host) = one_shot_fixture_address();
        let listener = tokio::net::TcpListener::bind(SocketAddr::from((bind_ip, 0)))
            .await
            .expect("test listener should bind");
        let port = listener.local_addr().expect("bound address").port();
        let expected_websocket = format!("ws://{client_host}:9/ws");
        let discovery_websocket = expected_websocket.clone();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("client should connect");
            let mut request = Vec::new();
            loop {
                let mut chunk = [0_u8; 1024];
                let count = stream.read(&mut chunk).await.expect("request should read");
                assert!(count > 0, "request must have headers");
                request.extend_from_slice(&chunk[..count]);
                assert!(request.len() <= 8192, "request header must stay bounded");
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let body = format!(r#"{{"webSocketDebuggerUrl":"{discovery_websocket}"}}"#);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("response should write");
            String::from_utf8(request).expect("HTTP header text should be UTF-8")
        });
        let headers = CdpHandshakeHeaders::new(vec![("foo".into(), "bar".into())])
            .expect("valid test header");

        // Act
        let endpoint = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            resolve_with_headers(&format!("http://{client_host}:{port}"), &headers),
        )
        .await
        .expect("discovery should not hang")
        .expect("discovery should succeed");
        let request = server.await.expect("server task should retire");

        // Assert
        assert_eq!(endpoint.as_str(), expected_websocket);
        assert!(request.starts_with("GET /json/version/ HTTP/1.1\r\n"));
        assert!(request.to_ascii_lowercase().contains("\r\nfoo: bar\r\n"));
    }
}
