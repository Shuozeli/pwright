//! Failure-only Playwright Server root initialization over a bounded WebSocket.
//!
//! This browser-neutral leaf sends the real Root.initialize command, but does
//! not construct a GUID object graph or return a browser. It accepts only a
//! direct `ws`/`wss` endpoint and observes the first correlated response or a
//! remote Close frame. The request entry point accepts bounded caller headers.
//!
//! The selected direct connector uses the Pwright Tungstenite forks, which
//! suppress handshake and frame content at TRACE. This says nothing about
//! other transports linked into an application, or remote Browser support.

use std::fmt::{self, Display, Formatter};
use std::time::Duration;

use pwright_transport_ws::{
    WsCloseFrame, WsConnector, WsEndpoint, WsIncoming, WsLimits, WsOutgoing, WsTransport,
};
use serde::{Deserialize, Serialize, de::IgnoredAny};
use serde_json::Value;

pub use pwright_transport_ws::LiveWsConnector;
pub use pwright_transport_ws::WsError;

pub const MAX_INITIALIZE_BUDGET_MS: u32 = 30_000;
pub const MAX_PRE_RESPONSE_MESSAGES: usize = 64;
const MAX_INBOUND_MESSAGE_BYTES: usize = 256 * 1024;
const MAX_OUTBOUND_MESSAGE_BYTES: usize = 4096;
const CLOSE_REPLY_BUDGET: Duration = Duration::from_millis(100);

/// The four SDK languages accepted by the pinned Playwright protocol.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SdkLanguage {
    Javascript,
    Python,
    Java,
    Csharp,
}

impl SdkLanguage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Javascript => "javascript",
            Self::Python => "python",
            Self::Java => "java",
            Self::Csharp => "csharp",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InitializeObservation {
    RemoteClose(Option<WsCloseFrame>),
    RootResponse,
    RemoteError { name: String, message: String },
    UnsupportedProtocol,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InitializeError {
    WebSocket(WsError),
    DeadlineExceeded,
    EncodeFailed,
}

impl Display for InitializeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WebSocket(error) => write!(formatter, "{error}"),
            Self::DeadlineExceeded => {
                formatter.write_str("Playwright Server initialization timed out")
            }
            Self::EncodeFailed => {
                formatter.write_str("Playwright Server initialization encoding failed")
            }
        }
    }
}

impl std::error::Error for InitializeError {}

impl From<WsError> for InitializeError {
    fn from(error: WsError) -> Self {
        Self::WebSocket(error)
    }
}

#[derive(Serialize)]
struct RootInitializeCommand<'a> {
    id: u64,
    guid: &'static str,
    method: &'static str,
    params: RootInitializeParams<'a>,
    metadata: RootInitializeMetadata,
}

#[derive(Serialize)]
struct RootInitializeParams<'a> {
    #[serde(rename = "sdkLanguage")]
    sdk_language: &'a str,
}

#[derive(Serialize)]
struct RootInitializeMetadata {
    #[serde(rename = "wallTime")]
    wall_time: u64,
    #[serde(rename = "apiName")]
    api_name: &'static str,
    internal: bool,
}

#[derive(Deserialize)]
struct IncomingEnvelope {
    id: Option<u64>,
    guid: Option<String>,
    method: Option<String>,
    params: Option<CreateParams>,
    result: Option<Value>,
    error: Option<Value>,
}

#[derive(Deserialize)]
struct CreateParams {
    guid: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "initializer")]
    _initializer: IgnoredAny,
}

fn root_initialize_json(
    sdk_language: SdkLanguage,
    wall_time_ms: u64,
) -> Result<String, InitializeError> {
    serde_json::to_string(&RootInitializeCommand {
        id: 1,
        guid: "",
        method: "initialize",
        params: RootInitializeParams {
            sdk_language: sdk_language.as_str(),
        },
        metadata: RootInitializeMetadata {
            wall_time: wall_time_ms,
            api_name: "Connection.init",
            internal: false,
        },
    })
    .map_err(|_| InitializeError::EncodeFailed)
}

fn text_outcome(text: &str) -> Option<InitializeObservation> {
    let envelope: IncomingEnvelope = match serde_json::from_str(text) {
        Ok(envelope) => envelope,
        Err(_) => return Some(InitializeObservation::UnsupportedProtocol),
    };
    if let Some(id) = envelope.id {
        if id != 1
            || envelope.guid.is_some()
            || envelope.method.is_some()
            || envelope.params.is_some()
        {
            return Some(InitializeObservation::UnsupportedProtocol);
        }
        return Some(match (envelope.result, envelope.error) {
            (Some(result), None)
                if result
                    .get("playwright")
                    .and_then(|playwright| playwright.get("guid"))
                    .and_then(Value::as_str)
                    .is_some_and(|guid| !guid.is_empty() && guid.len() <= 512) =>
            {
                InitializeObservation::RootResponse
            }
            (None, Some(error)) => match error.get("error") {
                Some(Value::Object(inner)) => {
                    match (
                        inner.get("name").and_then(Value::as_str),
                        inner.get("message").and_then(Value::as_str),
                    ) {
                        (Some(name), Some(message))
                            if !name.is_empty() && name.len() <= 256 && message.len() <= 8192 =>
                        {
                            InitializeObservation::RemoteError {
                                name: name.to_owned(),
                                message: message.to_owned(),
                            }
                        }
                        _ => InitializeObservation::UnsupportedProtocol,
                    }
                }
                _ => InitializeObservation::UnsupportedProtocol,
            },
            _ => InitializeObservation::UnsupportedProtocol,
        });
    }
    if envelope.result.is_none()
        && envelope.error.is_none()
        && envelope.guid.is_some()
        && envelope.method.as_deref() == Some("__create__")
        && envelope
            .params
            .is_some_and(|params| !params.guid.is_empty() && !params.kind.is_empty())
    {
        None
    } else {
        Some(InitializeObservation::UnsupportedProtocol)
    }
}

/// Send Root.initialize and observe one correlated response or remote close.
///
/// `wall_time_ms` is injected by the adapter; the core never reads a clock.
/// A zero requested timeout uses the 30-second safety cap. The returned
/// `RootResponse` is intentionally not a browser and must never be promoted
/// into a public connected Browser without a real GUID/object graph.
pub async fn observe_root_initialize<Connector: WsConnector>(
    connector: &Connector,
    endpoint_url: &str,
    sdk_language: SdkLanguage,
    wall_time_ms: u64,
    timeout_ms: u32,
) -> Result<InitializeObservation, InitializeError> {
    let endpoint = WsEndpoint::parse(endpoint_url)?;
    let budget_ms = if timeout_ms == 0 {
        MAX_INITIALIZE_BUDGET_MS
    } else {
        timeout_ms.min(MAX_INITIALIZE_BUDGET_MS)
    };
    let limits = WsLimits::new(
        MAX_INBOUND_MESSAGE_BYTES,
        MAX_OUTBOUND_MESSAGE_BYTES,
        budget_ms,
    )?;
    let request = pwright_transport_ws::WsRequest::new(endpoint, limits);
    observe_root_initialize_request(connector, &request, sdk_language, wall_time_ms).await
}

/// Observe initialization using a prevalidated request with bounded headers.
///
/// The caller may supply transport headers but cannot enlarge this leaf's
/// message or deadline budgets. The request is never promoted to a Browser.
pub async fn observe_root_initialize_request<Connector: WsConnector>(
    connector: &Connector,
    request: &pwright_transport_ws::WsRequest,
    sdk_language: SdkLanguage,
    wall_time_ms: u64,
) -> Result<InitializeObservation, InitializeError> {
    let limits = request.limits();
    if limits.inbound_message_bytes() > MAX_INBOUND_MESSAGE_BYTES
        || limits.outbound_message_bytes() > MAX_OUTBOUND_MESSAGE_BYTES
    {
        return Err(InitializeError::WebSocket(WsError::InvalidLimits(
            "Playwright Server initialization message budget exceeded",
        )));
    }
    let budget_ms = limits.connect_timeout_ms().min(MAX_INITIALIZE_BUDGET_MS);
    let budget = Duration::from_millis(u64::from(budget_ms));
    let command = root_initialize_json(sdk_language, wall_time_ms)?;

    let observation = tokio::time::timeout(budget, async {
        let mut transport = WsTransport::connect(connector, request).await?;
        transport.send(WsOutgoing::Text(command)).await?;
        let mut pre_response_messages = 0;
        loop {
            let outcome = match transport.receive().await? {
                WsIncoming::Text(text) => text_outcome(&text),
                WsIncoming::RemoteClose(frame) => Some(InitializeObservation::RemoteClose(frame)),
                WsIncoming::Binary(_) | WsIncoming::EndOfStream => {
                    Some(InitializeObservation::UnsupportedProtocol)
                }
            };
            if let Some(outcome) = outcome {
                break Ok::<_, InitializeError>((transport, outcome));
            }
            pre_response_messages += 1;
            if pre_response_messages >= MAX_PRE_RESPONSE_MESSAGES {
                break Ok((transport, InitializeObservation::UnsupportedProtocol));
            }
        }
    })
    .await
    .map_err(|_| InitializeError::DeadlineExceeded)??;

    // A received close frame deserves its own result even if the bounded
    // best-effort close reply cannot be flushed before this cleanup deadline.
    let (mut transport, outcome) = observation;
    let _ = tokio::time::timeout(CLOSE_REPLY_BUDGET, transport.close()).await;
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::future::{Ready, ready};
    use std::sync::{Arc, Mutex};

    use super::*;
    use pwright_transport_ws::{WsPeer, WsRequest};

    #[derive(Default)]
    struct FakeState {
        sent: Vec<WsOutgoing>,
        received: VecDeque<WsIncoming>,
        closed: bool,
        connected_request: Option<WsRequest>,
    }

    #[derive(Clone)]
    struct FakeConnector(Arc<Mutex<FakeState>>);

    struct FakePeer(Arc<Mutex<FakeState>>);

    impl WsConnector for FakeConnector {
        type Peer = FakePeer;
        type Future<'a> = Ready<Result<Self::Peer, WsError>>;

        fn connect(&self, request: &WsRequest) -> Self::Future<'_> {
            self.0.lock().unwrap().connected_request = Some(request.clone());
            ready(Ok(FakePeer(Arc::clone(&self.0))))
        }
    }

    impl WsPeer for FakePeer {
        type SendFuture<'a> = Ready<Result<(), WsError>>;
        type ReceiveFuture<'a> = Ready<Result<WsIncoming, WsError>>;
        type CloseFuture<'a> = Ready<Result<(), WsError>>;

        fn send(&mut self, message: WsOutgoing) -> Self::SendFuture<'_> {
            self.0.lock().unwrap().sent.push(message);
            ready(Ok(()))
        }

        fn receive(&mut self) -> Self::ReceiveFuture<'_> {
            ready(Ok(self
                .0
                .lock()
                .unwrap()
                .received
                .pop_front()
                .unwrap_or(WsIncoming::EndOfStream)))
        }

        fn close(&mut self) -> Self::CloseFuture<'_> {
            self.0.lock().unwrap().closed = true;
            ready(Ok(()))
        }
    }

    fn fake_with(
        messages: impl IntoIterator<Item = WsIncoming>,
    ) -> (FakeConnector, Arc<Mutex<FakeState>>) {
        let state = Arc::new(Mutex::new(FakeState {
            received: messages.into_iter().collect(),
            ..FakeState::default()
        }));
        (FakeConnector(Arc::clone(&state)), state)
    }

    #[tokio::test]
    async fn sends_exact_root_initialize_and_preserves_remote_close() {
        // Arrange
        let frame = WsCloseFrame::new(4123, "Oh my!").unwrap();
        let (connector, state) = fake_with([WsIncoming::RemoteClose(Some(frame.clone()))]);

        // Act
        let observed = observe_root_initialize(
            &connector,
            "ws://example.test/ws",
            SdkLanguage::Python,
            123,
            500,
        )
        .await
        .unwrap();

        // Assert
        assert_eq!(observed, InitializeObservation::RemoteClose(Some(frame)));
        let state = state.lock().unwrap();
        assert_eq!(state.sent, vec![WsOutgoing::Text("{\"id\":1,\"guid\":\"\",\"method\":\"initialize\",\"params\":{\"sdkLanguage\":\"python\"},\"metadata\":{\"wallTime\":123,\"apiName\":\"Connection.init\",\"internal\":false}}".into())]);
        assert!(state.closed);
    }

    #[tokio::test]
    async fn prevalidated_request_forwards_headers_and_javascript_language() {
        // Arrange
        let frame = WsCloseFrame::new(4123, "Oh my!").unwrap();
        let (connector, state) = fake_with([WsIncoming::RemoteClose(Some(frame.clone()))]);
        let endpoint = WsEndpoint::parse("ws://example.test/ws?secret=redacted").unwrap();
        let limits =
            WsLimits::new(MAX_INBOUND_MESSAGE_BYTES, MAX_OUTBOUND_MESSAGE_BYTES, 500).unwrap();
        let mut request = WsRequest::new(endpoint, limits);
        request
            .add_header("x-playwright-launch-options", "{}")
            .unwrap();

        // Act
        let observed =
            observe_root_initialize_request(&connector, &request, SdkLanguage::Javascript, 456)
                .await
                .unwrap();

        // Assert
        assert_eq!(observed, InitializeObservation::RemoteClose(Some(frame)));
        let state = state.lock().unwrap();
        assert_eq!(state.connected_request.as_ref(), Some(&request));
        assert_eq!(state.sent, vec![WsOutgoing::Text("{\"id\":1,\"guid\":\"\",\"method\":\"initialize\",\"params\":{\"sdkLanguage\":\"javascript\"},\"metadata\":{\"wallTime\":456,\"apiName\":\"Connection.init\",\"internal\":false}}".into())]);
        assert!(state.closed);
    }

    #[tokio::test]
    async fn create_events_then_response_never_become_a_browser() {
        // Arrange
        let (connector, state) = fake_with([
            WsIncoming::Text("{\"guid\":\"\",\"method\":\"__create__\",\"params\":{\"guid\":\"p1\",\"type\":\"Playwright\",\"initializer\":{}}}".into()),
            WsIncoming::Text("{\"id\":1,\"result\":{\"playwright\":{\"guid\":\"p1\"}}}".into()),
        ]);

        // Act
        let observed = observe_root_initialize(
            &connector,
            "wss://example.test/ws",
            SdkLanguage::Javascript,
            456,
            500,
        )
        .await
        .unwrap();

        // Assert
        assert_eq!(observed, InitializeObservation::RootResponse);
        assert!(state.lock().unwrap().closed);
    }

    #[tokio::test]
    async fn correlated_remote_error_is_not_a_root_response() {
        // Arrange
        let (connector, state) = fake_with([WsIncoming::Text(
            "{\"id\":1,\"error\":{\"error\":{\"name\":\"Error\",\"message\":\"remote init failed\"}}}"
                .into(),
        )]);

        // Act
        let observed = observe_root_initialize(
            &connector,
            "ws://example.test/ws",
            SdkLanguage::Python,
            0,
            500,
        )
        .await
        .unwrap();

        // Assert
        assert_eq!(
            observed,
            InitializeObservation::RemoteError {
                name: "Error".into(),
                message: "remote init failed".into(),
            }
        );
        assert!(state.lock().unwrap().closed);
    }

    #[tokio::test]
    async fn missing_playwright_guid_is_not_a_root_response() {
        // Arrange
        let (connector, state) = fake_with([WsIncoming::Text(
            "{\"id\":1,\"result\":{\"playwright\":{}}}".into(),
        )]);

        // Act
        let observed = observe_root_initialize(
            &connector,
            "ws://example.test/ws",
            SdkLanguage::Python,
            0,
            500,
        )
        .await
        .unwrap();

        // Assert
        assert_eq!(observed, InitializeObservation::UnsupportedProtocol);
        assert!(state.lock().unwrap().closed);
    }

    #[tokio::test]
    async fn id_only_reply_is_not_a_root_response() {
        // Arrange
        let (connector, state) = fake_with([WsIncoming::Text("{\"id\":1}".into())]);

        // Act
        let observed = observe_root_initialize(
            &connector,
            "ws://example.test/ws",
            SdkLanguage::Python,
            0,
            500,
        )
        .await
        .unwrap();

        // Assert
        assert_eq!(observed, InitializeObservation::UnsupportedProtocol);
        assert!(state.lock().unwrap().closed);
    }

    #[tokio::test]
    async fn malformed_first_message_is_unsupported_protocol() {
        // Arrange
        let (connector, state) = fake_with([WsIncoming::Text("not-json".into())]);

        // Act
        let observed = observe_root_initialize(
            &connector,
            "ws://example.test/ws",
            SdkLanguage::Python,
            0,
            500,
        )
        .await
        .unwrap();

        // Assert
        assert_eq!(observed, InitializeObservation::UnsupportedProtocol);
        assert!(state.lock().unwrap().closed);
    }

    #[tokio::test]
    async fn rejects_http_endpoint_without_connecting() {
        // Arrange
        let (connector, state) = fake_with([]);

        // Act
        let observed = observe_root_initialize(
            &connector,
            "http://example.test/ws",
            SdkLanguage::Python,
            0,
            500,
        )
        .await;

        // Assert
        assert!(matches!(
            observed,
            Err(InitializeError::WebSocket(WsError::InvalidEndpoint(_)))
        ));
        assert!(state.lock().unwrap().sent.is_empty());
    }
}
