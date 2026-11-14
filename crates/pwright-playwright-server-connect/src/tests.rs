use std::future::{Future, Ready, ready};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pwright_playwright_server_client_core::{Ingress, ReplyOutcome};
use pwright_playwright_server_endpoint_discovery::{
    DiscoveryRequest, DiscoveryResponse, FetchError,
};
use pwright_playwright_server_session::{
    EventPolicy, SessionError, SupervisorError, SupervisorState,
};
use pwright_transport_ws::{WsIncoming, WsOutgoing};
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio::time::timeout;

use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
struct SeenDiscovery {
    url: String,
    user_agent: String,
    body_limit: usize,
    timeout_ms: u32,
}

struct FakeFetcher {
    response: Result<DiscoveryResponse, FetchError>,
    seen: Mutex<Vec<SeenDiscovery>>,
}

impl FakeFetcher {
    fn success() -> Self {
        Self {
            response: Ok(DiscoveryResponse::new(
                200,
                br#"{"wsEndpointPath":"/browser-id"}"#.to_vec(),
            )),
            seen: Mutex::new(Vec::new()),
        }
    }

    fn seen(&self) -> Vec<SeenDiscovery> {
        self.seen.lock().unwrap().clone()
    }
}

impl HttpDiscoveryFetcher for FakeFetcher {
    fn fetch<'a>(
        &'a self,
        request: &'a DiscoveryRequest,
    ) -> Pin<Box<dyn Future<Output = Result<DiscoveryResponse, FetchError>> + Send + 'a>> {
        self.seen.lock().unwrap().push(SeenDiscovery {
            url: request.url().to_string(),
            user_agent: request.user_agent().to_owned(),
            body_limit: request.limits().max_body_bytes(),
            timeout_ms: request.limits().timeout_ms(),
        });
        Box::pin(async { self.response.clone() })
    }
}

struct FakeConnector {
    incoming: Mutex<Option<mpsc::UnboundedReceiver<WsIncoming>>>,
    sent: mpsc::UnboundedSender<WsOutgoing>,
    requests: Mutex<Vec<WsRequest>>,
    error: Option<WsError>,
    close_count: Arc<AtomicUsize>,
}

struct FakePeer {
    incoming: mpsc::UnboundedReceiver<WsIncoming>,
    sent: mpsc::UnboundedSender<WsOutgoing>,
    close_count: Arc<AtomicUsize>,
}

impl WsConnector for FakeConnector {
    type Peer = FakePeer;
    type Future<'a> = Ready<Result<Self::Peer, WsError>>;

    fn connect(&self, request: &WsRequest) -> Self::Future<'_> {
        self.requests.lock().unwrap().push(request.clone());
        if let Some(error) = &self.error {
            return ready(Err(error.clone()));
        }
        ready(Ok(FakePeer {
            incoming: self.incoming.lock().unwrap().take().unwrap(),
            sent: self.sent.clone(),
            close_count: Arc::clone(&self.close_count),
        }))
    }
}

impl WsPeer for FakePeer {
    type SendFuture<'a> = Ready<Result<(), WsError>>;
    type ReceiveFuture<'a> = Pin<Box<dyn Future<Output = Result<WsIncoming, WsError>> + Send + 'a>>;
    type CloseFuture<'a> = Ready<Result<(), WsError>>;

    fn send(&mut self, message: WsOutgoing) -> Self::SendFuture<'_> {
        ready(self.sent.send(message).map_err(|_| WsError::IoFailed))
    }

    fn receive(&mut self) -> Self::ReceiveFuture<'_> {
        Box::pin(async move {
            Ok(self
                .incoming
                .recv()
                .await
                .unwrap_or(WsIncoming::EndOfStream))
        })
    }

    fn close(&mut self) -> Self::CloseFuture<'_> {
        self.close_count.fetch_add(1, Ordering::SeqCst);
        ready(Ok(()))
    }
}

struct Fixture {
    incoming: mpsc::UnboundedSender<WsIncoming>,
    sent: mpsc::UnboundedReceiver<WsOutgoing>,
    connector: FakeConnector,
}

impl Fixture {
    fn new(frames: Vec<WsIncoming>) -> Self {
        let (incoming, receiver) = mpsc::unbounded_channel();
        let (sent, sent_receiver) = mpsc::unbounded_channel();
        for frame in frames {
            incoming.send(frame).unwrap();
        }
        Self {
            incoming,
            sent: sent_receiver,
            connector: FakeConnector {
                incoming: Mutex::new(Some(receiver)),
                sent,
                requests: Mutex::new(Vec::new()),
                error: None,
                close_count: Arc::new(AtomicUsize::new(0)),
            },
        }
    }

    fn requests(&self) -> Vec<WsRequest> {
        self.connector.requests.lock().unwrap().clone()
    }

    fn close_count(&self) -> usize {
        self.connector.close_count.load(Ordering::SeqCst)
    }
}

fn text(value: Value) -> WsIncoming {
    WsIncoming::Text(value.to_string())
}

fn success_frames() -> Vec<WsIncoming> {
    vec![
        text(
            json!({"guid":"","method":"__create__","params":{"type":"BrowserType","guid":"chromium@1","initializer":{"name":"chromium","executablePath":"/browser"}}}),
        ),
        text(
            json!({"guid":"","method":"__create__","params":{"type":"BrowserType","guid":"firefox@1","initializer":{"name":"firefox","executablePath":"/browser"}}}),
        ),
        text(
            json!({"guid":"","method":"__create__","params":{"type":"BrowserType","guid":"webkit@1","initializer":{"name":"webkit","executablePath":"/browser"}}}),
        ),
        text(
            json!({"guid":"","method":"__create__","params":{"type":"Android","guid":"android@1","initializer":{}}}),
        ),
        text(
            json!({"guid":"","method":"__create__","params":{"type":"Electron","guid":"electron@1","initializer":{}}}),
        ),
        text(
            json!({"guid":"chromium@1","method":"__create__","params":{"type":"Browser","guid":"browser@1","initializer":{"name":"chromium","version":"155.0"}}}),
        ),
        text(
            json!({"guid":"","method":"__create__","params":{"type":"Playwright","guid":"playwright@1","initializer":{
                "chromium":{"guid":"chromium@1"},
                "firefox":{"guid":"firefox@1"},
                "webkit":{"guid":"webkit@1"},
                "android":{"guid":"android@1"},
                "electron":{"guid":"electron@1"},
                "preLaunchedBrowser":{"guid":"browser@1"}
            }}}),
        ),
        text(json!({"id":1,"result":{"playwright":{"guid":"playwright@1"}}})),
    ]
}

fn options(endpoint: &str) -> ConnectOptions {
    ConnectOptions::new(
        endpoint,
        "Playwright/1.58.0 test/1",
        DiscoveryLimits::new(1024, 1500).unwrap(),
        WsLimits::new(256 * 1024, 64 * 1024, 1000).unwrap(),
        SdkLanguage::Python,
        1234,
    )
}

fn upstream_version_mismatch_body() -> String {
    let lines = [
        "Playwright version mismatch:",
        "  - server version: v1.58",
        "  - client version: v1.57",
        "",
        "If you are using VSCode extension, restart VSCode.",
        "",
        "If you are connecting to a remote service,",
        "keep your local Playwright version in sync",
        "with the remote service version.",
        "",
        "<3 Playwright Team",
    ];
    let width = lines.iter().map(|line| line.len()).max().unwrap();
    let border = "═".repeat(width + 2);
    let contents = lines
        .iter()
        .map(|line| format!("║ {line:<width$} ║"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("╔{border}╗\n{contents}\n╚{border}╝")
}

async fn rejected_connect_error(status: u16, body: String) -> ConnectError {
    let fetcher = FakeFetcher::success();
    let mut fixture = Fixture::new(success_frames());
    fixture.connector.error = Some(WsError::HttpRejected { status, body });
    connect(
        &options("ws://server.test/?token=PRIVATE"),
        &fetcher,
        &fixture.connector,
    )
    .await
    .err()
    .unwrap()
}

async fn sent_json(sent: &mut mpsc::UnboundedReceiver<WsOutgoing>) -> Value {
    let message = timeout(Duration::from_secs(1), sent.recv())
        .await
        .unwrap()
        .unwrap();
    let WsOutgoing::Text(message) = message else {
        panic!("expected text frame");
    };
    serde_json::from_str(&message).unwrap()
}

#[tokio::test]
async fn direct_websocket_returns_live_supervisor_with_only_caller_headers() {
    // Arrange
    let fetcher = FakeFetcher::success();
    let mut fixture = Fixture::new(success_frames());
    let mut input = options("wss://server.test/endpoint?token=PRIVATE");
    input.add_websocket_header("X-Test", "value").unwrap();
    input.add_websocket_header("User-Agent", "chosen").unwrap();

    // Act
    let browser = connect(&input, &fetcher, &fixture.connector).await.unwrap();

    // Assert
    assert!(browser.is_connected());
    assert_eq!(browser.guid(), "browser@1");
    assert_eq!(browser.name(), "chromium");
    assert_eq!(browser.version(), "155.0");
    assert!(fetcher.seen().is_empty());
    let requests = fixture.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].endpoint().as_str(),
        "wss://server.test/endpoint?token=PRIVATE"
    );
    assert_eq!(
        requests[0].limits(),
        WsLimits::new(256 * 1024, 64 * 1024, 1000).unwrap()
    );
    assert_eq!(
        requests[0]
            .headers()
            .iter()
            .map(|header| (header.name(), header.value()))
            .collect::<Vec<_>>(),
        vec![("X-Test", "value"), ("User-Agent", "chosen")]
    );
    assert_eq!(
        sent_json(&mut fixture.sent).await,
        json!({"id":1,"guid":"","method":"initialize","params":{"sdkLanguage":"python"},"metadata":{"wallTime":1234,"apiName":"Connection.init","internal":false}})
    );
    for _ in 0..7 {
        assert!(matches!(
            browser.next_event().await.unwrap(),
            Ingress::Created { .. }
        ));
    }

    let browser = Arc::new(browser);
    let waiter = tokio::spawn({
        let browser = Arc::clone(&browser);
        async move {
            browser
                .call("browser@1", "syntheticEcho", json!({"value":7}), json!({}))
                .await
        }
    });
    assert_eq!(
        sent_json(&mut fixture.sent).await,
        json!({"id":2,"guid":"browser@1","method":"syntheticEcho","params":{"value":7},"metadata":{}})
    );
    fixture
        .incoming
        .send(text(json!({"id":2,"result":{"echo":7}})))
        .unwrap();
    assert_eq!(
        timeout(Duration::from_secs(1), waiter)
            .await
            .unwrap()
            .unwrap(),
        Ok(ReplyOutcome::Result(json!({"echo":7})))
    );
    assert!(browser.close().await.unwrap().report.failed.is_empty());
    assert!(!browser.is_connected());
    assert_eq!(fixture.close_count(), 1);
}

#[tokio::test]
async fn explicit_drop_policy_projects_bootstrap_without_queuing_events() {
    // Arrange
    let fetcher = FakeFetcher::success();
    let fixture = Fixture::new(success_frames());
    let input = options("ws://server.test/endpoint");

    // Act
    let browser = connect_with_event_policy(
        &input,
        &fetcher,
        &fixture.connector,
        EventPolicy::DropUnsupportedEvents,
    )
    .await
    .unwrap();

    // Assert
    assert!(browser.is_connected());
    assert_eq!(browser.bootstrap_event_count(), 0);
    assert_eq!(browser.object("browser@1").unwrap().object_type, "Browser");
    assert_eq!(
        browser.next_event().await,
        Err(SupervisorError::EventStreamDisabled)
    );
    browser.close().await.unwrap();
    assert_eq!(fixture.close_count(), 1);
}

#[tokio::test]
async fn http_discovery_uses_original_query_and_only_supplied_websocket_headers() {
    // Arrange
    let fetcher = FakeFetcher::success();
    let fixture = Fixture::new(success_frames());
    let mut input = options("https://server.test/prefix?token=PRIVATE");
    input
        .add_websocket_header("Authorization", "Bearer PRIVATE")
        .unwrap();

    // Act
    let browser = connect(&input, &fetcher, &fixture.connector).await.unwrap();

    // Assert
    assert!(browser.is_connected());
    assert_eq!(
        fetcher.seen(),
        vec![SeenDiscovery {
            url: "https://server.test/prefix/json?token=PRIVATE".into(),
            user_agent: "Playwright/1.58.0 test/1".into(),
            body_limit: 1024,
            timeout_ms: 1500,
        }]
    );
    let requests = fixture.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].endpoint().as_str(),
        "wss://server.test/prefix/browser-id?token=PRIVATE"
    );
    assert_eq!(
        requests[0]
            .headers()
            .iter()
            .map(|header| (header.name(), header.value()))
            .collect::<Vec<_>>(),
        vec![("Authorization", "Bearer PRIVATE")]
    );
    browser.close().await.unwrap();
    assert_eq!(fixture.close_count(), 1);
}

#[tokio::test]
async fn discovery_failure_never_opens_a_websocket() {
    // Arrange
    let fetcher = FakeFetcher {
        response: Ok(DiscoveryResponse::new(503, b"private body".to_vec())),
        seen: Mutex::new(Vec::new()),
    };
    let fixture = Fixture::new(success_frames());
    let input = options("http://server.test/?token=PRIVATE");

    // Act
    let error = connect(&input, &fetcher, &fixture.connector)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        error,
        ConnectError::Discovery(DiscoveryError::HttpStatus(503))
    );
    assert!(fixture.requests().is_empty());
    assert_eq!(fixture.close_count(), 0);
    assert!(!format!("{error:?} {error}").contains("PRIVATE"));
}

#[tokio::test]
async fn http_discovery_timeout_precedes_reserved_websocket_header_rejection() {
    // Arrange
    let fetcher = FakeFetcher {
        response: Err(FetchError::Timeout),
        seen: Mutex::new(Vec::new()),
    };
    let fixture = Fixture::new(success_frames());
    let mut input = options("http://server.test/connect?token=PRIVATE");
    input
        .add_websocket_header_after_http_discovery("Connection", "Close")
        .unwrap();

    // Act
    let error = connect(&input, &fetcher, &fixture.connector)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        error,
        ConnectError::Discovery(DiscoveryError::Fetch(FetchError::Timeout))
    );
    assert_eq!(fetcher.seen().len(), 1);
    assert!(fixture.requests().is_empty());
    assert!(!format!("{error:?} {error}").contains("PRIVATE"));
}

#[tokio::test]
async fn successful_http_discovery_rejects_reserved_header_before_websocket() {
    // Arrange
    let fetcher = FakeFetcher::success();
    let fixture = Fixture::new(success_frames());
    let mut input = options("https://server.test/connect");
    input
        .add_websocket_header_after_http_discovery("Connection", "Close")
        .unwrap();

    // Act
    let error = connect(&input, &fetcher, &fixture.connector)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        error,
        ConnectError::WebSocket(WsError::InvalidHeader(
            "handshake-controlled header is reserved"
        ))
    );
    assert_eq!(fetcher.seen().len(), 1);
    assert!(fixture.requests().is_empty());
}

#[test]
fn direct_websocket_endpoint_keeps_reserved_header_preflight() {
    // Arrange
    let mut input = options("ws://server.test/connect");

    // Act
    let strict = input.add_websocket_header("Connection", "Close");
    let deferred = input.add_websocket_header_after_http_discovery("Connection", "Close");

    // Assert
    assert_eq!(
        strict,
        Err(WsError::InvalidHeader(
            "handshake-controlled header is reserved"
        ))
    );
    assert_eq!(
        deferred,
        Err(WsError::InvalidEndpoint(
            "deferred WebSocket headers require HTTP(S) discovery"
        ))
    );
}

#[tokio::test]
async fn websocket_failure_has_typed_session_error_without_secret_diagnostics() {
    // Arrange
    let fetcher = FakeFetcher::success();
    let mut fixture = Fixture::new(success_frames());
    fixture.connector.error = Some(WsError::ConnectFailed);
    let input = options("ws://server.test/?token=PRIVATE");

    // Act
    let error = connect(&input, &fetcher, &fixture.connector)
        .await
        .err()
        .unwrap();

    // Assert
    assert!(matches!(error, ConnectError::Session(ref failure)
        if failure.error == SessionError::WebSocket(WsError::ConnectFailed)));
    assert_eq!(fixture.requests().len(), 1);
    assert_eq!(fixture.close_count(), 0);
    assert!(!format!("{error:?} {error}").contains("PRIVATE"));
}

#[tokio::test]
async fn websocket_connect_timeout_remains_typed_inside_session_failure() {
    // Arrange
    let fetcher = FakeFetcher::success();
    let mut fixture = Fixture::new(success_frames());
    fixture.connector.error = Some(WsError::ConnectTimeout);
    let input = options("ws://server.test/?token=PRIVATE");

    // Act
    let error = connect(&input, &fetcher, &fixture.connector)
        .await
        .err()
        .unwrap();

    // Assert
    assert!(matches!(error, ConnectError::Session(ref failure)
        if failure.error == SessionError::WebSocket(WsError::ConnectTimeout)));
    assert_eq!(fixture.requests().len(), 1);
    assert_eq!(fixture.close_count(), 0);
    assert!(!format!("{error:?} {error}").contains("PRIVATE"));
}

#[tokio::test]
async fn canonical_428_exposes_only_server_and_client_versions() {
    // Arrange
    let body = upstream_version_mismatch_body();

    // Act
    let error = rejected_connect_error(428, body).await;
    let mismatch = error.playwright_version_mismatch().unwrap();

    // Assert
    assert_eq!(
        mismatch,
        PlaywrightVersionMismatch {
            server_version: "1.58".into(),
            client_version: "1.57".into(),
        }
    );
    assert_eq!(mismatch.server_version(), "1.58");
    assert_eq!(mismatch.client_version(), "1.57");
    assert_eq!(error.to_string(), "Playwright Server WebSocket failed");
    assert!(!format!("{error:?}").contains("VSCode"));
    assert!(!format!("{error:?}").contains("PRIVATE"));
}

#[tokio::test]
async fn non_428_rejection_does_not_expose_version_lines() {
    // Arrange
    let body = upstream_version_mismatch_body();

    // Act
    let error = rejected_connect_error(401, body).await;

    // Assert
    assert_eq!(error.playwright_version_mismatch(), None);
    assert_eq!(error.to_string(), "Playwright Server WebSocket failed");
}

#[tokio::test]
async fn arbitrary_428_body_does_not_expose_version_lines() {
    // Arrange
    let body = "Playwright version mismatch:\n  - server version: v1.58\n  - client version: v1.57\nBearer PRIVATE".to_owned();

    // Act
    let error = rejected_connect_error(428, body).await;

    // Assert
    assert_eq!(error.playwright_version_mismatch(), None);
    assert!(!format!("{error:?} {error}").contains("PRIVATE"));
}

#[tokio::test]
async fn malformed_428_version_is_not_reported() {
    // Arrange
    let body = upstream_version_mismatch_body().replace(
        "client version: v1.57",
        "client version: v1.57?token=PRIVATE",
    );

    // Act
    let error = rejected_connect_error(428, body).await;

    // Assert
    assert_eq!(error.playwright_version_mismatch(), None);
    assert!(!format!("{error:?} {error}").contains("PRIVATE"));
}

#[tokio::test]
async fn incomplete_428_box_is_not_reported() {
    // Arrange
    let body = upstream_version_mismatch_body()
        .lines()
        .take(4)
        .collect::<Vec<_>>()
        .join("\n");

    // Act
    let error = rejected_connect_error(428, body).await;

    // Assert
    assert_eq!(error.playwright_version_mismatch(), None);
}

#[test]
fn direct_websocket_http_rejection_display_redacts_remote_body() {
    // Arrange
    let error = ConnectError::WebSocket(WsError::HttpRejected {
        status: 428,
        body: "Bearer PRIVATE".into(),
    });

    // Act
    let display = error.to_string();

    // Assert
    assert_eq!(display, "WebSocket HTTP handshake rejected with status 428");
    assert_eq!(error.playwright_version_mismatch(), None);
    assert!(!format!("{error:?}").contains("PRIVATE"));
}

#[tokio::test]
async fn malformed_initialize_closes_socket_before_a_handle_is_published() {
    // Arrange
    let fetcher = FakeFetcher::success();
    let mut frames = success_frames();
    frames[7] = text(json!({"id":1,"result":{"wrong":{}}}));
    let fixture = Fixture::new(frames);

    // Act
    let error = connect(&options("ws://server.test"), &fetcher, &fixture.connector)
        .await
        .err()
        .unwrap();

    // Assert
    assert!(matches!(error, ConnectError::Session(ref failure)
        if matches!(failure.error, SessionError::MalformedBootstrap(_))));
    assert_eq!(fixture.close_count(), 1);
}

#[tokio::test]
async fn startup_event_pressure_rejects_handle_and_closes_socket() {
    // Arrange
    let fetcher = FakeFetcher::success();
    let mut frames = success_frames();
    for index in 0..=128 {
        frames.insert(
            7 + index,
            text(json!({"guid":"browser@1","method":"notice","params":{"index":index}})),
        );
    }
    let fixture = Fixture::new(frames);

    // Act
    let error = connect(&options("ws://server.test"), &fetcher, &fixture.connector)
        .await
        .err()
        .unwrap();

    // Assert
    assert!(matches!(error, ConnectError::Session(ref failure)
        if failure.error == SessionError::EventQueueFull));
    assert_eq!(fixture.close_count(), 1);
}

#[tokio::test]
async fn remote_end_of_stream_transitions_supervisor_out_of_connected_state() {
    // Arrange
    let fetcher = FakeFetcher::success();
    let fixture = Fixture::new(success_frames());
    let browser = connect(&options("ws://server.test"), &fetcher, &fixture.connector)
        .await
        .unwrap();

    // Act
    fixture.incoming.send(WsIncoming::EndOfStream).unwrap();
    for _ in 0..7 {
        browser.next_event().await.unwrap();
    }
    let terminal = timeout(Duration::from_secs(1), browser.next_event())
        .await
        .unwrap();

    // Assert
    assert!(terminal.is_err());
    assert!(matches!(browser.state(), SupervisorState::Closed { .. }));
    assert!(!browser.is_connected());
    assert_eq!(fixture.close_count(), 0);
}

#[test]
fn caller_headers_are_validated_and_debug_output_is_redacted() {
    // Arrange
    let mut input = options("ws://server.test/?token=PRIVATE");

    // Act
    input
        .add_websocket_header("Authorization", "Bearer PRIVATE")
        .unwrap();
    let invalid = input.add_websocket_header("X-Injected", "bad\r\nheader");
    let debug = format!("{input:?}");

    // Assert
    assert!(matches!(invalid, Err(WsError::InvalidHeader(_))));
    assert!(!debug.contains("PRIVATE"));
    assert!(!debug.contains("Bearer"));
    assert!(debug.contains("<redacted>"));
}
