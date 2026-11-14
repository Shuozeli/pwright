use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
struct SeenRequest {
    url: String,
    user_agent: String,
    body_limit: usize,
    timeout_ms: u32,
}

struct FakeFetcher {
    reply: Result<DiscoveryResponse, FetchError>,
    seen: Mutex<Vec<SeenRequest>>,
}

impl FakeFetcher {
    fn reply(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            reply: Ok(DiscoveryResponse::new(status, body.into())),
            seen: Mutex::new(Vec::new()),
        }
    }

    fn seen(&self) -> Vec<SeenRequest> {
        self.seen.lock().expect("fake lock").clone()
    }
}

impl HttpDiscoveryFetcher for FakeFetcher {
    fn fetch<'a>(
        &'a self,
        request: &'a DiscoveryRequest,
    ) -> Pin<Box<dyn Future<Output = Result<DiscoveryResponse, FetchError>> + Send + 'a>> {
        self.seen.lock().expect("fake lock").push(SeenRequest {
            url: request.url().to_string(),
            user_agent: request.user_agent().to_owned(),
            body_limit: request.limits().max_body_bytes(),
            timeout_ms: request.limits().timeout_ms(),
        });
        Box::pin(async { self.reply.clone() })
    }
}

fn limits() -> DiscoveryLimits {
    DiscoveryLimits::new(1024, 1500).expect("valid limits")
}

#[tokio::test]
async fn direct_websocket_endpoint_skips_http_discovery() {
    // Arrange
    let fetcher = FakeFetcher::reply(200, br#"{"wsEndpointPath":"ignored"}"#.to_vec());

    // Act
    let endpoint = resolve_endpoint(
        "wss://server.test/path?token=secret",
        "",
        limits(),
        &fetcher,
    )
    .await
    .expect("direct WebSocket endpoint");

    // Assert
    assert_eq!(endpoint.as_str(), "wss://server.test/path?token=secret");
    assert!(fetcher.seen().is_empty());
}

#[tokio::test]
async fn http_discovery_appends_json_and_preserves_query_without_extra_headers() {
    // Arrange
    let fetcher = FakeFetcher::reply(200, br#"{"wsEndpointPath":"/browser-id"}"#.to_vec());

    // Act
    let endpoint = resolve_endpoint(
        "http://server.test/prefix?token=secret",
        "Playwright/1.58.0 node/26.7",
        limits(),
        &fetcher,
    )
    .await
    .expect("discovered endpoint");

    // Assert
    assert_eq!(
        endpoint.as_str(),
        "ws://server.test/prefix/browser-id?token=secret"
    );
    assert_eq!(
        fetcher.seen(),
        vec![SeenRequest {
            url: "http://server.test/prefix/json?token=secret".into(),
            user_agent: "Playwright/1.58.0 node/26.7".into(),
            body_limit: 1024,
            timeout_ms: 1500,
        }]
    );
}

#[tokio::test]
async fn https_root_discovery_maps_to_secure_websocket() {
    // Arrange
    let fetcher = FakeFetcher::reply(200, br#"{"wsEndpointPath":"/session"}"#.to_vec());

    // Act
    let endpoint = resolve_endpoint(
        "https://server.test",
        "Playwright/1.58.0",
        limits(),
        &fetcher,
    )
    .await
    .expect("discovered endpoint");

    // Assert
    assert_eq!(endpoint.as_str(), "wss://server.test/session");
    assert_eq!(fetcher.seen()[0].url, "https://server.test/json");
}

#[tokio::test]
async fn one_leading_slash_is_removed_from_discovered_path() {
    // Arrange
    let fetcher = FakeFetcher::reply(200, br#"{"wsEndpointPath":"//session"}"#.to_vec());

    // Act
    let endpoint = resolve_endpoint("http://server.test/base/", "UA", limits(), &fetcher)
        .await
        .expect("discovered endpoint");

    // Assert
    assert_eq!(endpoint.as_str(), "ws://server.test/base//session");
    assert_eq!(fetcher.seen()[0].url, "http://server.test/base/json");
}

#[tokio::test]
async fn discovery_path_is_path_text_not_an_authority_or_query() {
    // Arrange
    let fetcher = FakeFetcher::reply(200, br#"{"wsEndpointPath":"/part?new=1"}"#.to_vec());

    // Act
    let endpoint = resolve_endpoint("http://server.test/base?old=1", "UA", limits(), &fetcher)
        .await
        .expect("discovered endpoint");

    // Assert
    assert_eq!(
        endpoint.as_str(),
        "ws://server.test/base/part%3Fnew=1?old=1"
    );
}

#[tokio::test]
async fn non_200_response_is_not_treated_as_discovery_json() {
    // Arrange
    let fetcher = FakeFetcher::reply(302, br#"{"wsEndpointPath":"/ignored"}"#.to_vec());

    // Act
    let error = resolve_endpoint("http://server.test", "UA", limits(), &fetcher)
        .await
        .expect_err("redirect is not followed");

    // Assert
    assert_eq!(error, DiscoveryError::HttpStatus(302));
    assert_eq!(fetcher.seen().len(), 1);
}

#[tokio::test]
async fn malformed_or_missing_discovery_path_is_rejected() {
    // Arrange
    let invalid_json = FakeFetcher::reply(200, b"not-json".to_vec());
    let missing_path = FakeFetcher::reply(200, br#"{"other":"value"}"#.to_vec());

    // Act
    let malformed = resolve_endpoint("http://server.test", "UA", limits(), &invalid_json).await;
    let missing = resolve_endpoint("http://server.test", "UA", limits(), &missing_path).await;

    // Assert
    assert_eq!(
        malformed,
        Err(DiscoveryError::InvalidResponse("body is not valid JSON"))
    );
    assert_eq!(
        missing,
        Err(DiscoveryError::InvalidResponse(
            "wsEndpointPath is missing or not a string"
        ))
    );
}

#[tokio::test]
async fn oversized_fake_response_is_rejected_before_parsing() {
    // Arrange
    let fetcher = FakeFetcher::reply(200, vec![b'x'; 1025]);

    // Act
    let error = resolve_endpoint("http://server.test", "UA", limits(), &fetcher)
        .await
        .expect_err("body cap");

    // Assert
    assert_eq!(error, DiscoveryError::BodyTooLarge);
}

#[tokio::test]
async fn live_fetcher_body_limit_error_has_same_public_class_as_fake_overflow() {
    // Arrange
    let fetcher = FakeFetcher {
        reply: Err(FetchError::BodyTooLarge),
        seen: Mutex::new(Vec::new()),
    };

    // Act
    let error = resolve_endpoint("http://server.test", "UA", limits(), &fetcher)
        .await
        .expect_err("body cap");

    // Assert
    assert_eq!(error, DiscoveryError::BodyTooLarge);
}

#[tokio::test]
async fn invalid_endpoint_or_user_agent_causes_no_network_request() {
    // Arrange
    let fetcher = FakeFetcher::reply(200, br#"{"wsEndpointPath":"/session"}"#.to_vec());

    // Act
    let embedded_credentials =
        resolve_endpoint("http://u:p@server.test", "UA", limits(), &fetcher).await;
    let invalid_ua = resolve_endpoint("http://server.test", "bad\nUA", limits(), &fetcher).await;

    // Assert
    assert!(matches!(
        embedded_credentials,
        Err(DiscoveryError::InvalidEndpoint(_))
    ));
    assert!(matches!(
        invalid_ua,
        Err(DiscoveryError::InvalidUserAgent(_))
    ));
    assert!(fetcher.seen().is_empty());
}

#[test]
fn limits_reject_unbounded_body_or_timeout() {
    // Arrange
    let body_limit = MAX_DISCOVERY_BODY_BYTES + 1;
    let timeout = MAX_CONNECT_TIMEOUT_MS + 1;

    // Act
    let oversized_body = DiscoveryLimits::new(body_limit, 1000);
    let oversized_timeout = DiscoveryLimits::new(1024, timeout);

    // Assert
    assert!(matches!(
        oversized_body,
        Err(DiscoveryError::InvalidLimits(_))
    ));
    assert!(matches!(
        oversized_timeout,
        Err(DiscoveryError::InvalidLimits(_))
    ));
}

#[test]
fn debug_and_display_never_print_discovery_query_or_body() {
    // Arrange
    let request = DiscoveryRequest {
        url: Url::parse("http://server.test/json?token=secret").expect("URL"),
        user_agent: "private-user-agent".into(),
        limits: limits(),
    };
    let response = DiscoveryResponse::new(200, b"private-body".to_vec());
    let error = DiscoveryError::HttpStatus(401);

    // Act
    let diagnostics = format!("{request:?} {response:?} {error:?} {error}");

    // Assert
    assert!(!diagnostics.contains("secret"));
    assert!(!diagnostics.contains("private-user-agent"));
    assert!(!diagnostics.contains("private-body"));
    assert!(diagnostics.contains("401"));
}
