use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use pwright_playwright_server_client_core::{CloseReason, Ingress, ProtocolViolation};
use pwright_transport_ws::{
    WsCloseFrame, WsConnector, WsEndpoint, WsError, WsIncoming, WsLimits, WsOutgoing, WsPeer,
    WsRequest,
};
use serde_json::{Value, json};

use super::{
    MAX_BOOTSTRAP_BUFFERED_INGRESS, RemoteBrowserLease, SdkLanguage, SendError, SessionError,
};

#[derive(Default)]
struct FakeState {
    received: VecDeque<WsIncoming>,
    sent: Vec<WsOutgoing>,
    connected_request: Option<WsRequest>,
    close_count: usize,
    fail_send: bool,
    hang_receive: bool,
}

#[derive(Clone)]
struct FakeConnector(Arc<Mutex<FakeState>>);

struct FakePeer(Arc<Mutex<FakeState>>);

impl WsConnector for FakeConnector {
    type Peer = FakePeer;
    type Future<'a> = Pin<Box<dyn Future<Output = Result<Self::Peer, WsError>> + Send + 'a>>;

    fn connect(&self, request: &WsRequest) -> Self::Future<'_> {
        let state = Arc::clone(&self.0);
        let request = request.clone();
        Box::pin(async move {
            state.lock().unwrap().connected_request = Some(request);
            Ok(FakePeer(state))
        })
    }
}

impl WsPeer for FakePeer {
    type SendFuture<'a> = Pin<Box<dyn Future<Output = Result<(), WsError>> + Send + 'a>>;
    type ReceiveFuture<'a> = Pin<Box<dyn Future<Output = Result<WsIncoming, WsError>> + Send + 'a>>;
    type CloseFuture<'a> = Pin<Box<dyn Future<Output = Result<(), WsError>> + Send + 'a>>;

    fn send(&mut self, message: WsOutgoing) -> Self::SendFuture<'_> {
        let state = Arc::clone(&self.0);
        Box::pin(async move {
            let mut state = state.lock().unwrap();
            if state.fail_send {
                return Err(WsError::IoFailed);
            }
            state.sent.push(message);
            Ok(())
        })
    }

    fn receive(&mut self) -> Self::ReceiveFuture<'_> {
        let state = Arc::clone(&self.0);
        Box::pin(async move {
            let hang = state.lock().unwrap().hang_receive;
            if hang {
                std::future::pending::<()>().await;
            }
            Ok(state
                .lock()
                .unwrap()
                .received
                .pop_front()
                .unwrap_or(WsIncoming::EndOfStream))
        })
    }

    fn close(&mut self) -> Self::CloseFuture<'_> {
        let state = Arc::clone(&self.0);
        Box::pin(async move {
            state.lock().unwrap().close_count += 1;
            Ok(())
        })
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

fn fixture(messages: Vec<WsIncoming>) -> (FakeConnector, Arc<Mutex<FakeState>>, WsRequest) {
    let state = Arc::new(Mutex::new(FakeState {
        received: messages.into(),
        ..FakeState::default()
    }));
    let connector = FakeConnector(Arc::clone(&state));
    let endpoint = WsEndpoint::parse("ws://example.test/ws").unwrap();
    let limits = WsLimits::new(256 * 1024, 64 * 1024, 1000).unwrap();
    let request = WsRequest::new(endpoint, limits);
    (connector, state, request)
}

fn rewrite_playwright_initializer(frames: &mut [WsIncoming], edit: impl FnOnce(&mut Value)) {
    let WsIncoming::Text(frame) = &mut frames[6] else {
        panic!("expected Playwright creation frame");
    };
    let mut message: Value = serde_json::from_str(frame).unwrap();
    edit(&mut message["params"]["initializer"]);
    *frame = message.to_string();
}

async fn drain_bootstrap(browser: &mut RemoteBrowserLease<FakePeer>) {
    while browser.bootstrap_pending_count() > 0 {
        browser.receive_next().await.unwrap();
    }
}

#[tokio::test]
async fn validated_prelaunched_browser_owns_connection_until_close() {
    // Arrange
    let (connector, state, mut request) = fixture(success_frames());
    request
        .add_header("x-playwright-browser", "chromium")
        .unwrap();

    // Act
    let mut browser = RemoteBrowserLease::connect(&connector, &request, SdkLanguage::Python, 1234)
        .await
        .unwrap();

    // Assert
    assert!(browser.is_connected());
    assert_eq!(browser.guid(), "browser@1");
    assert_eq!(browser.name(), "chromium");
    assert_eq!(browser.version(), "155.0");
    assert_eq!(browser.pending_count(), 0);
    assert_eq!(browser.bootstrap_pending_count(), 7);
    {
        let state = state.lock().unwrap();
        assert_eq!(state.connected_request.as_ref(), Some(&request));
        assert_eq!(state.sent.len(), 1);
        let WsOutgoing::Text(command) = &state.sent[0] else {
            panic!("expected text command");
        };
        assert_eq!(
            serde_json::from_str::<Value>(command).unwrap(),
            json!({"id":1,"guid":"","method":"initialize","params":{"sdkLanguage":"python"},"metadata":{"wallTime":1234,"apiName":"Connection.init","internal":false}})
        );
    }

    let outcome = browser.close().await;
    assert!(!browser.is_connected());
    assert!(outcome.report.failed.is_empty());
    assert!(outcome.transport_error.is_none());
}

#[tokio::test]
async fn root_initialize_error_never_creates_a_browser() {
    // Arrange
    let mut frames = success_frames();
    frames[7] = text(json!({"id":1,"error":{"error":{"name":"Error","message":"launch failed"}}}));
    let (connector, state, request) = fixture(frames);

    // Act
    let failure = RemoteBrowserLease::connect(&connector, &request, SdkLanguage::Javascript, 99)
        .await
        .err()
        .unwrap();

    // Assert
    assert!(matches!(failure.error, SessionError::Remote(_)));
    assert!(failure.report.failed.is_empty());
    assert_eq!(state.lock().unwrap().close_count, 1);
}

#[tokio::test]
async fn missing_prelaunched_browser_is_not_fabricated() {
    // Arrange
    let mut frames = success_frames();
    rewrite_playwright_initializer(&mut frames, |initializer| {
        initializer
            .as_object_mut()
            .unwrap()
            .remove("preLaunchedBrowser");
    });
    let (connector, state, request) = fixture(frames);

    // Act
    let failure = RemoteBrowserLease::connect(&connector, &request, SdkLanguage::Python, 99)
        .await
        .err()
        .unwrap();

    // Assert
    assert!(matches!(
        failure.error,
        SessionError::MalformedBootstrap("required channel reference missing")
    ));
    assert!(matches!(
        failure.report.reason,
        CloseReason::Protocol(ProtocolViolation::InvalidReply)
    ));
    assert_eq!(state.lock().unwrap().close_count, 1);
}

#[tokio::test]
async fn wrong_type_prelaunched_browser_guid_is_rejected() {
    // Arrange
    let mut frames = success_frames();
    rewrite_playwright_initializer(&mut frames, |initializer| {
        initializer["preLaunchedBrowser"] = json!({"guid":"chromium@1"});
    });
    let (connector, state, request) = fixture(frames);

    // Act
    let failure = RemoteBrowserLease::connect(&connector, &request, SdkLanguage::Python, 99)
        .await
        .err()
        .unwrap();

    // Assert
    assert!(matches!(
        failure.error,
        SessionError::MalformedBootstrap("channel reference has wrong object type")
    ));
    assert_eq!(state.lock().unwrap().close_count, 1);
}

#[tokio::test]
async fn early_remote_close_rejects_pending_initialize() {
    // Arrange
    let frame = WsCloseFrame::new(4123, "Oh my!").unwrap();
    let (connector, state, request) = fixture(vec![WsIncoming::RemoteClose(Some(frame.clone()))]);

    // Act
    let failure = RemoteBrowserLease::connect(&connector, &request, SdkLanguage::Python, 99)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(failure.error, SessionError::RemoteClose(Some(frame)));
    assert_eq!(failure.report.failed.len(), 1);
    assert_eq!(failure.report.failed[0].id.get(), 1);
    assert_eq!(failure.report.failed[0].method, "initialize");
    assert_eq!(state.lock().unwrap().close_count, 1);
}

#[tokio::test]
async fn malformed_frame_closes_peer_and_rejects_initialize() {
    // Arrange
    let (connector, state, request) = fixture(vec![WsIncoming::Text("{".into())]);

    // Act
    let failure = RemoteBrowserLease::connect(&connector, &request, SdkLanguage::Python, 99)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(
        failure.error,
        SessionError::Protocol(ProtocolViolation::InvalidJson)
    );
    assert_eq!(failure.report.failed.len(), 1);
    assert_eq!(state.lock().unwrap().close_count, 1);
}

#[tokio::test]
async fn pre_reply_event_is_replayed_after_creation_messages_in_wire_order() {
    // Arrange
    let mut frames = success_frames();
    frames.splice(
        6..6,
        [
            json!({"guid":"browser@1","method":"__create__","params":{"type":"BrowserContext","guid":"context@1","initializer":{}}}),
            json!({"guid":"browser@1","method":"context","params":{"context":{"guid":"context@1"}}}),
        ]
        .into_iter()
        .map(text),
    );
    let (connector, state, request) = fixture(frames);

    // Act
    let mut browser = RemoteBrowserLease::connect(&connector, &request, SdkLanguage::Python, 99)
        .await
        .unwrap();
    let mut replayed = Vec::new();
    while browser.bootstrap_pending_count() > 0 {
        replayed.push(browser.receive_next().await.unwrap());
    }

    // Assert
    assert_eq!(replayed.len(), 9);
    assert!(matches!(&replayed[0], Ingress::Created { guid, .. } if guid == "chromium@1"));
    assert!(
        matches!(&replayed[6], Ingress::Created { parent, guid, object_type, .. }
        if parent == "browser@1" && guid == "context@1" && object_type == "BrowserContext")
    );
    assert!(
        matches!(&replayed[7], Ingress::Event { guid, method, params }
        if guid == "browser@1" && method == "context"
        && params == &json!({"context":{"guid":"context@1"}}))
    );
    assert!(matches!(&replayed[8], Ingress::Created { guid, .. } if guid == "playwright@1"));
    assert!(browser.is_connected());
    assert_eq!(state.lock().unwrap().close_count, 0);
}

#[tokio::test]
async fn pre_reply_adoption_and_disposal_are_replayed_in_wire_order() {
    // Arrange
    let mut frames = success_frames();
    let lifecycle = [
        json!({"guid":"","method":"__create__","params":{"type":"Scope","guid":"scope@1","initializer":{}}}),
        json!({"guid":"scope@1","method":"__create__","params":{"type":"Child","guid":"child@1","initializer":{}}}),
        json!({"guid":"","method":"__create__","params":{"type":"Scope","guid":"scope@2","initializer":{}}}),
        json!({"guid":"scope@2","method":"__adopt__","params":{"guid":"child@1"}}),
        json!({"guid":"scope@1","method":"__dispose__","params":{}}),
    ];
    frames.splice(7..7, lifecycle.into_iter().map(text));
    let (connector, _state, request) = fixture(frames);

    // Act
    let mut browser = RemoteBrowserLease::connect(&connector, &request, SdkLanguage::Python, 99)
        .await
        .unwrap();
    let mut replayed = Vec::new();
    while browser.bootstrap_pending_count() > 0 {
        replayed.push(browser.receive_next().await.unwrap());
    }

    // Assert
    assert_eq!(replayed.len(), 12);
    assert!(matches!(&replayed[10], Ingress::Adopted { parent, guid }
        if parent == "scope@2" && guid == "child@1"));
    assert!(matches!(&replayed[11], Ingress::Disposed { guids, .. }
        if guids == &vec!["scope@1".to_owned()]));
    assert!(browser.is_connected());
}

#[tokio::test]
async fn pre_reply_browser_close_prevents_publishing_a_connected_lease() {
    // Arrange
    let mut frames = success_frames();
    frames.insert(
        7,
        text(json!({"guid":"browser@1","method":"close","params":{}})),
    );
    let (connector, state, request) = fixture(frames);

    // Act
    let failure = RemoteBrowserLease::connect(&connector, &request, SdkLanguage::Python, 99)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(failure.error, SessionError::BrowserClosed);
    assert!(failure.report.failed.is_empty());
    assert_eq!(state.lock().unwrap().close_count, 1);
}

#[tokio::test]
async fn pre_reply_browser_disposal_is_reported_as_close_before_graph_validation() {
    // Arrange
    let mut frames = success_frames();
    frames.splice(
        7..7,
        [
            json!({"guid":"browser@1","method":"close","params":{}}),
            json!({"guid":"browser@1","method":"__dispose__","params":{}}),
        ]
        .into_iter()
        .map(text),
    );
    let (connector, state, request) = fixture(frames);

    // Act
    let failure = RemoteBrowserLease::connect(&connector, &request, SdkLanguage::Python, 99)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(failure.error, SessionError::BrowserClosed);
    assert!(failure.report.failed.is_empty());
    assert_eq!(state.lock().unwrap().close_count, 1);
}

#[tokio::test]
async fn bootstrap_ingress_count_limit_closes_and_drains_initialize() {
    // Arrange
    let mut frames = success_frames();
    let events = (0..MAX_BOOTSTRAP_BUFFERED_INGRESS)
        .map(|_| text(json!({"guid":"browser@1","method":"disconnected","params":{}})));
    frames.splice(7..7, events);
    let (connector, state, request) = fixture(frames);

    // Act
    let failure = RemoteBrowserLease::connect(&connector, &request, SdkLanguage::Python, 99)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(failure.error, SessionError::BootstrapLimit);
    assert_eq!(failure.report.failed.len(), 1);
    assert_eq!(failure.report.reason, CloseReason::ResourceLimit);
    assert_eq!(state.lock().unwrap().close_count, 1);
}

#[tokio::test]
async fn bootstrap_ingress_byte_limit_closes_before_retaining_an_unbounded_event_burst() {
    // Arrange
    let mut frames = success_frames();
    let payload = "x".repeat(230 * 1024);
    let events = (0..5)
        .map(|_| text(json!({"guid":"browser@1","method":"context","params":{"payload":payload}})));
    frames.splice(7..7, events);
    let (connector, state, request) = fixture(frames);

    // Act
    let failure = RemoteBrowserLease::connect(&connector, &request, SdkLanguage::Python, 99)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(failure.error, SessionError::BootstrapLimit);
    assert_eq!(failure.report.failed.len(), 1);
    assert_eq!(failure.report.reason, CloseReason::ResourceLimit);
    assert_eq!(state.lock().unwrap().close_count, 1);
}

#[tokio::test]
async fn bootstrap_deadline_rejects_pending_initialize() {
    // Arrange
    let (connector, state, request) = fixture(Vec::new());
    state.lock().unwrap().hang_receive = true;

    // Act
    let failure = RemoteBrowserLease::connect(&connector, &request, SdkLanguage::Python, 99)
        .await
        .err()
        .unwrap();

    // Assert
    assert_eq!(failure.error, SessionError::DeadlineExceeded);
    assert_eq!(failure.report.failed.len(), 1);
    assert_eq!(state.lock().unwrap().close_count, 1);
}

#[tokio::test]
async fn remote_close_after_bootstrap_rejects_pending_command() {
    // Arrange
    let (connector, state, request) = fixture(success_frames());
    let mut browser = RemoteBrowserLease::connect(&connector, &request, SdkLanguage::Python, 99)
        .await
        .unwrap();
    drain_bootstrap(&mut browser).await;
    state
        .lock()
        .unwrap()
        .received
        .push_back(WsIncoming::RemoteClose(None));

    // Act
    let id = browser
        .send_command("browser@1", "newContext", &json!({}), &json!({}))
        .await
        .unwrap();
    let failure = browser.receive_next().await.unwrap_err();

    // Assert
    assert_eq!(id.get(), 2);
    assert_eq!(failure.error, SessionError::RemoteClose(None));
    assert_eq!(failure.report.failed.len(), 1);
    assert_eq!(failure.report.failed[0].id, id);
    assert!(!browser.is_connected());
    assert_eq!(browser.pending_count(), 0);
}

#[tokio::test]
async fn local_close_rejects_pending_command_once_and_is_idempotent() {
    // Arrange
    let (connector, state, request) = fixture(success_frames());
    let mut browser = RemoteBrowserLease::connect(&connector, &request, SdkLanguage::Python, 99)
        .await
        .unwrap();
    let id = browser
        .send_command("browser@1", "newContext", &json!({}), &json!({}))
        .await
        .unwrap();

    // Act
    let first = browser.close().await;
    let second = browser.close().await;

    // Assert
    assert_eq!(first.report.failed.len(), 1);
    assert_eq!(first.report.failed[0].id, id);
    assert!(second.report.failed.is_empty());
    assert!(!browser.is_connected());
    assert_eq!(browser.pending_count(), 0);
    assert_eq!(browser.bootstrap_pending_count(), 0);
    assert_eq!(state.lock().unwrap().close_count, 1);
}

#[tokio::test]
async fn send_failure_closes_connection_and_rejects_all_pending() {
    // Arrange
    let (connector, state, request) = fixture(success_frames());
    let mut browser = RemoteBrowserLease::connect(&connector, &request, SdkLanguage::Python, 99)
        .await
        .unwrap();
    state.lock().unwrap().fail_send = true;

    // Act
    let result = browser
        .send_command("browser@1", "newContext", &json!({}), &json!({}))
        .await;

    // Assert
    assert!(matches!(result, Err(SendError::Terminal(failure))
        if failure.error == SessionError::WebSocket(WsError::IoFailed)
        && failure.report.failed.len() == 1
        && failure.report.failed[0].id.get() == 2));
    assert!(!browser.is_connected());
    assert_eq!(browser.pending_count(), 0);
    assert_eq!(browser.bootstrap_pending_count(), 0);
}

#[tokio::test]
async fn browser_close_event_terminates_its_session() {
    // Arrange
    let (connector, state, request) = fixture(success_frames());
    let mut browser = RemoteBrowserLease::connect(&connector, &request, SdkLanguage::Python, 99)
        .await
        .unwrap();
    drain_bootstrap(&mut browser).await;
    state.lock().unwrap().received.push_back(text(json!({
        "guid":"browser@1","method":"close","params":{}
    })));

    // Act
    let failure = browser.receive_next().await.unwrap_err();

    // Assert
    assert_eq!(failure.error, SessionError::BrowserClosed);
    assert!(!browser.is_connected());
    assert_eq!(state.lock().unwrap().close_count, 1);
}

#[tokio::test]
async fn two_connections_have_independent_guid_graphs_and_lifetimes() {
    // Arrange
    let (first_connector, first_state, first_request) = fixture(success_frames());
    let (second_connector, second_state, second_request) = fixture(success_frames());

    // Act
    let (first, second) = tokio::join!(
        RemoteBrowserLease::connect(&first_connector, &first_request, SdkLanguage::Python, 1),
        RemoteBrowserLease::connect(
            &second_connector,
            &second_request,
            SdkLanguage::Javascript,
            2
        ),
    );
    let mut first = first.unwrap();
    let second = second.unwrap();
    first.close().await;

    // Assert
    assert!(!first.is_connected());
    assert!(second.is_connected());
    assert_eq!(first_state.lock().unwrap().close_count, 1);
    assert_eq!(second_state.lock().unwrap().close_count, 0);
}

#[tokio::test]
async fn success_reply_can_be_followed_by_correlated_command_reply() {
    // Arrange
    let (connector, state, request) = fixture(success_frames());
    let mut browser = RemoteBrowserLease::connect(&connector, &request, SdkLanguage::Python, 99)
        .await
        .unwrap();
    drain_bootstrap(&mut browser).await;
    state.lock().unwrap().received.push_back(text(json!({
        "id":2,"result":{}
    })));

    // Act
    let id = browser
        .send_command("browser@1", "newContext", &json!({}), &json!({}))
        .await
        .unwrap();
    let ingress = browser.receive_next().await.unwrap();

    // Assert
    assert_eq!(id.get(), 2);
    assert!(matches!(ingress, Ingress::Reply { pending, .. } if pending.id == id));
    assert_eq!(browser.pending_count(), 0);
}
