//! Optional, protocol-neutral WebSocket connection and bounded message policy.
//!
//! The default selection has no network runtime. A caller injects a connector;
//! `live-tokio` adds a direct Tokio/Tungstenite connector without starting a
//! listener or task. This crate understands neither CDP nor Playwright JSON.
//! It does not discover endpoints, follow redirects, own browser processes, or
//! assign a public API error message to a remote close reason. A received Close
//! frame is the payload accepted by Tungstenite's WebSocket parser, not a raw
//! capture of protocol-invalid wire bytes.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;

use url::Url;

#[cfg(feature = "live-tokio")]
mod live;
#[cfg(feature = "live-tokio")]
pub use live::{LiveWsConnector, LiveWsRejectionBodyConnector};

pub const MAX_ENDPOINT_BYTES: usize = 8192;
pub const MAX_HEADER_COUNT: usize = 64;
pub const MAX_HEADER_NAME_BYTES: usize = 256;
pub const MAX_HEADER_VALUE_BYTES: usize = 8192;
pub const MAX_TOTAL_HEADER_BYTES: usize = 32 * 1024;
pub const MAX_MESSAGE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_CLOSE_REASON_BYTES: usize = 123;
pub const MAX_CONNECT_TIMEOUT_MS: u32 = 120_000;
/// Only a bounded prefix of a rejected HTTP upgrade body reaches the caller.
pub const MAX_HTTP_REJECTION_BODY_BYTES: usize = 8 * 1024;

#[derive(Clone, Eq, PartialEq)]
pub enum WsError {
    InvalidEndpoint(&'static str),
    InvalidHeader(&'static str),
    InvalidLimits(&'static str),
    LimitExceeded {
        resource: &'static str,
        actual: usize,
        limit: usize,
    },
    AllocationFailed,
    ConnectTimeout,
    DnsLookupFailed,
    ConnectFailed,
    HttpRejected {
        status: u16,
        body: String,
    },
    IoFailed,
    Closed,
}

impl fmt::Debug for WsError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::HttpRejected { status, .. } => formatter
                .debug_struct("HttpRejected")
                .field("status", status)
                .field("body", &"<redacted>")
                .finish(),
            other => write!(formatter, "{other}"),
        }
    }
}

impl Display for WsError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEndpoint(reason) => {
                write!(formatter, "invalid WebSocket endpoint: {reason}")
            }
            Self::InvalidHeader(reason) => write!(formatter, "invalid WebSocket header: {reason}"),
            Self::InvalidLimits(reason) => write!(formatter, "invalid WebSocket limits: {reason}"),
            Self::LimitExceeded {
                resource,
                actual,
                limit,
            } => write!(
                formatter,
                "WebSocket {resource} uses {actual} bytes/items, limit {limit}"
            ),
            Self::AllocationFailed => formatter.write_str("WebSocket allocation failed"),
            Self::ConnectTimeout => formatter.write_str("WebSocket connection timed out"),
            Self::DnsLookupFailed => formatter.write_str("getaddrinfo failed"),
            Self::ConnectFailed => formatter.write_str("WebSocket connection failed"),
            Self::HttpRejected { status, body } if body.is_empty() => {
                write!(
                    formatter,
                    "WebSocket HTTP handshake rejected with status {status}"
                )
            }
            Self::HttpRejected { status, body } => write!(
                formatter,
                "WebSocket HTTP handshake rejected with status {status}: {body}"
            ),
            Self::IoFailed => formatter.write_str("WebSocket I/O failed"),
            Self::Closed => formatter.write_str("WebSocket connection is closed"),
        }
    }
}

impl Error for WsError {}

fn within(resource: &'static str, actual: usize, limit: usize) -> Result<(), WsError> {
    if actual > limit {
        Err(WsError::LimitExceeded {
            resource,
            actual,
            limit,
        })
    } else {
        Ok(())
    }
}

/// Canonical validated WebSocket address, without hidden endpoint discovery.
#[derive(Clone, Eq, PartialEq)]
pub struct WsEndpoint(String);

impl fmt::Debug for WsEndpoint {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("WsEndpoint")
            .field(&"<redacted>")
            .finish()
    }
}

impl WsEndpoint {
    pub fn parse(value: impl Into<String>) -> Result<Self, WsError> {
        let value = value.into();
        within("endpoint", value.len(), MAX_ENDPOINT_BYTES)?;
        if value.bytes().any(|byte| byte <= 0x20 || byte == 0x7f) {
            return Err(WsError::InvalidEndpoint(
                "unescaped whitespace or controls are unsupported",
            ));
        }
        let parsed =
            Url::parse(&value).map_err(|_| WsError::InvalidEndpoint("expected an absolute URL"))?;
        if !matches!(parsed.scheme(), "ws" | "wss") {
            return Err(WsError::InvalidEndpoint("scheme must be ws or wss"));
        }
        if parsed.host_str().is_none() {
            return Err(WsError::InvalidEndpoint("host is required"));
        }
        if !parsed.username().is_empty() || parsed.password().is_some() {
            return Err(WsError::InvalidEndpoint(
                "embedded credentials are unsupported",
            ));
        }
        if parsed.fragment().is_some() {
            return Err(WsError::InvalidEndpoint("fragments are unsupported"));
        }
        let canonical = parsed.to_string();
        within("endpoint", canonical.len(), MAX_ENDPOINT_BYTES)?;
        Ok(Self(canonical))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Browser-facing connection diagnostics omit query parameters, which can
    /// carry credentials. The validated endpoint has no fragment or userinfo.
    pub fn without_query(&self) -> &str {
        self.0.split_once('?').map_or(&self.0, |(base, _)| base)
    }
}

/// Per-connection limits. No default or unbounded mode is offered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WsLimits {
    inbound_message_bytes: usize,
    outbound_message_bytes: usize,
    connect_timeout_ms: u32,
}

impl WsLimits {
    pub fn new(
        inbound_message_bytes: usize,
        outbound_message_bytes: usize,
        connect_timeout_ms: u32,
    ) -> Result<Self, WsError> {
        if inbound_message_bytes == 0 || outbound_message_bytes == 0 {
            return Err(WsError::InvalidLimits("message limits must be nonzero"));
        }
        within("inbound message", inbound_message_bytes, MAX_MESSAGE_BYTES)?;
        within(
            "outbound message",
            outbound_message_bytes,
            MAX_MESSAGE_BYTES,
        )?;
        if connect_timeout_ms == 0 || connect_timeout_ms > MAX_CONNECT_TIMEOUT_MS {
            return Err(WsError::InvalidLimits("connect timeout is out of range"));
        }
        Ok(Self {
            inbound_message_bytes,
            outbound_message_bytes,
            connect_timeout_ms,
        })
    }

    pub const fn inbound_message_bytes(self) -> usize {
        self.inbound_message_bytes
    }

    pub const fn outbound_message_bytes(self) -> usize {
        self.outbound_message_bytes
    }

    pub const fn connect_timeout_ms(self) -> u32 {
        self.connect_timeout_ms
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct WsHeader {
    name: String,
    value: String,
}

impl fmt::Debug for WsHeader {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WsHeader")
            .field("name", &self.name)
            .field("value", &"<redacted>")
            .finish()
    }
}

impl WsHeader {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WsHeaders {
    entries: Vec<WsHeader>,
    total_bytes: usize,
}

impl WsHeaders {
    pub fn push(&mut self, name: &str, value: &str) -> Result<(), WsError> {
        self.push_checked(name, value, false)
    }

    /// Validate and retain an HTTP(S) connect option before endpoint discovery.
    /// The resulting headers are not a WebSocket handshake: `WsRequest` must
    /// revalidate them with `push` after discovery and before any connection.
    pub fn push_for_http_discovery(&mut self, name: &str, value: &str) -> Result<(), WsError> {
        self.push_checked(name, value, true)
    }

    fn push_checked(
        &mut self,
        name: &str,
        value: &str,
        defer_reserved: bool,
    ) -> Result<(), WsError> {
        if name.is_empty() || name.len() > MAX_HEADER_NAME_BYTES || !name.bytes().all(is_http_token)
        {
            return Err(WsError::InvalidHeader(
                "name must be an HTTP token within 256 bytes",
            ));
        }
        if value.len() > MAX_HEADER_VALUE_BYTES
            || !value
                .bytes()
                .all(|byte| byte == b'\t' || (0x20..=0x7e).contains(&byte))
        {
            return Err(WsError::InvalidHeader(
                "value must be ASCII text or tab within 8192 bytes",
            ));
        }
        if !defer_reserved
            && matches!(
                name.to_ascii_lowercase().as_str(),
                "host"
                    | "connection"
                    | "upgrade"
                    | "sec-websocket-key"
                    | "sec-websocket-version"
                    | "sec-websocket-protocol"
                    | "sec-websocket-extensions"
            )
        {
            return Err(WsError::InvalidHeader(
                "handshake-controlled header is reserved",
            ));
        }
        if self
            .entries
            .iter()
            .any(|entry| entry.name.eq_ignore_ascii_case(name))
        {
            return Err(WsError::InvalidHeader("duplicate header name"));
        }
        within("header count", self.entries.len() + 1, MAX_HEADER_COUNT)?;
        let total_bytes = self
            .total_bytes
            .checked_add(name.len())
            .and_then(|bytes| bytes.checked_add(value.len()))
            .ok_or(WsError::AllocationFailed)?;
        within("request headers", total_bytes, MAX_TOTAL_HEADER_BYTES)?;
        self.entries
            .try_reserve(1)
            .map_err(|_| WsError::AllocationFailed)?;
        self.entries.push(WsHeader {
            name: name.to_owned(),
            value: value.to_owned(),
        });
        self.total_bytes = total_bytes;
        Ok(())
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &WsHeader> {
        self.entries.iter()
    }

    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }
}

fn is_http_token(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WsRequest {
    endpoint: WsEndpoint,
    headers: WsHeaders,
    limits: WsLimits,
}

impl WsRequest {
    pub const fn new(endpoint: WsEndpoint, limits: WsLimits) -> Self {
        Self {
            endpoint,
            headers: WsHeaders {
                entries: Vec::new(),
                total_bytes: 0,
            },
            limits,
        }
    }

    pub fn add_header(&mut self, name: &str, value: &str) -> Result<(), WsError> {
        self.headers.push(name, value)
    }

    pub const fn endpoint(&self) -> &WsEndpoint {
        &self.endpoint
    }

    pub const fn headers(&self) -> &WsHeaders {
        &self.headers
    }

    pub const fn limits(&self) -> WsLimits {
        self.limits
    }
}

/// A parser-accepted Close frame; `reason` is exact UTF-8, not an error class.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WsCloseFrame {
    code: u16,
    reason: String,
}

impl WsCloseFrame {
    pub fn new(code: u16, reason: impl Into<String>) -> Result<Self, WsError> {
        let reason = reason.into();
        within("close reason", reason.len(), MAX_CLOSE_REASON_BYTES)?;
        Ok(Self { code, reason })
    }

    pub const fn code(&self) -> u16 {
        self.code
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WsIncoming {
    Text(String),
    Binary(Vec<u8>),
    RemoteClose(Option<WsCloseFrame>),
    EndOfStream,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WsOutgoing {
    Text(String),
    Binary(Vec<u8>),
}

impl WsOutgoing {
    fn len(&self) -> usize {
        match self {
            Self::Text(text) => text.len(),
            Self::Binary(bytes) => bytes.len(),
        }
    }
}

/// Injectable connection. Implementations must honor the request's connect
/// timeout; the runtime-free transport cannot impose a wall-clock deadline.
/// The caller owns polling, cancellation and lifetime.
pub trait WsConnector: Send + Sync {
    type Peer: WsPeer;
    type Future<'a>: Future<Output = Result<Self::Peer, WsError>> + Send
    where
        Self: 'a;

    fn connect(&self, request: &WsRequest) -> Self::Future<'_>;
}

pub trait WsPeer: Send {
    type SendFuture<'a>: Future<Output = Result<(), WsError>> + Send
    where
        Self: 'a;
    type ReceiveFuture<'a>: Future<Output = Result<WsIncoming, WsError>> + Send
    where
        Self: 'a;
    type CloseFuture<'a>: Future<Output = Result<(), WsError>> + Send
    where
        Self: 'a;

    fn send(&mut self, message: WsOutgoing) -> Self::SendFuture<'_>;
    fn receive(&mut self) -> Self::ReceiveFuture<'_>;
    /// Drive the local Close frame or a queued reply to a remote Close frame.
    /// This operation must be safe to retry if its future is canceled or fails.
    fn close(&mut self) -> Self::CloseFuture<'_>;
}

/// Enforces the same bounds for a fake or a live peer and preserves closure.
pub struct WsTransport<Peer> {
    peer: Peer,
    limits: WsLimits,
    state: WsState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WsState {
    Open,
    RemoteClosePending,
    EndOfStream,
    Poisoned,
    ClosePending,
    Closed,
}

impl<Peer: WsPeer> WsTransport<Peer> {
    pub async fn connect<Connector: WsConnector<Peer = Peer>>(
        connector: &Connector,
        request: &WsRequest,
    ) -> Result<Self, WsError> {
        let peer = connector.connect(request).await?;
        Ok(Self {
            peer,
            limits: request.limits,
            state: WsState::Open,
        })
    }

    pub async fn send(&mut self, message: WsOutgoing) -> Result<(), WsError> {
        if self.state != WsState::Open {
            return Err(WsError::Closed);
        }
        within(
            "outbound message",
            message.len(),
            self.limits.outbound_message_bytes,
        )?;
        let result = self.peer.send(message).await;
        if result.is_err() {
            self.state = WsState::Poisoned;
        }
        result
    }

    /// Deliver one message. After `RemoteClose`, call [`Self::close`] to flush
    /// the peer's queued close reply before dropping the connection.
    pub async fn receive(&mut self) -> Result<WsIncoming, WsError> {
        if self.state != WsState::Open {
            return Err(WsError::Closed);
        }
        let incoming = match self.peer.receive().await {
            Ok(incoming) => incoming,
            Err(error) => {
                self.state = WsState::Poisoned;
                return Err(error);
            }
        };
        let size = match &incoming {
            WsIncoming::Text(text) => within(
                "inbound message",
                text.len(),
                self.limits.inbound_message_bytes,
            ),
            WsIncoming::Binary(bytes) => within(
                "inbound message",
                bytes.len(),
                self.limits.inbound_message_bytes,
            ),
            WsIncoming::RemoteClose(Some(frame)) => {
                within("close reason", frame.reason.len(), MAX_CLOSE_REASON_BYTES)
            }
            WsIncoming::RemoteClose(None) | WsIncoming::EndOfStream => Ok(()),
        };
        if let Err(error) = size {
            self.state = WsState::Poisoned;
            return Err(error);
        }
        match &incoming {
            WsIncoming::RemoteClose(_) => self.state = WsState::RemoteClosePending,
            WsIncoming::EndOfStream => self.state = WsState::EndOfStream,
            WsIncoming::Text(_) | WsIncoming::Binary(_) => {}
        }
        Ok(incoming)
    }

    pub async fn close(&mut self) -> Result<(), WsError> {
        if matches!(self.state, WsState::EndOfStream | WsState::Closed) {
            return Ok(());
        }
        self.state = WsState::ClosePending;
        self.peer.close().await?;
        self.state = WsState::Closed;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::future::{Future, Ready, ready};
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use std::task::{Context, Poll};

    use super::*;

    fn request() -> WsRequest {
        WsRequest::new(
            WsEndpoint::parse("ws://example.test/ws").unwrap(),
            WsLimits::new(16, 16, 1000).unwrap(),
        )
    }

    #[derive(Default)]
    struct State {
        request: Option<WsRequest>,
        received: VecDeque<WsIncoming>,
        sent: Vec<WsOutgoing>,
        closes: usize,
        close_outcomes: VecDeque<CloseOutcome>,
    }

    #[derive(Clone)]
    enum CloseOutcome {
        Pending,
        Ready(Result<(), WsError>),
    }

    struct FakeCloseFuture(CloseOutcome);

    impl Future for FakeCloseFuture {
        type Output = Result<(), WsError>;

        fn poll(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<Self::Output> {
            match &self.0 {
                CloseOutcome::Pending => Poll::Pending,
                CloseOutcome::Ready(result) => Poll::Ready(result.clone()),
            }
        }
    }

    struct FakeConnector(Arc<Mutex<State>>);
    struct FakePeer(Arc<Mutex<State>>);

    impl WsConnector for FakeConnector {
        type Peer = FakePeer;
        type Future<'a> = Ready<Result<Self::Peer, WsError>>;

        fn connect(&self, request: &WsRequest) -> Self::Future<'_> {
            self.0.lock().unwrap().request = Some(request.clone());
            ready(Ok(FakePeer(Arc::clone(&self.0))))
        }
    }

    impl WsPeer for FakePeer {
        type SendFuture<'a> = Ready<Result<(), WsError>>;
        type ReceiveFuture<'a> = Ready<Result<WsIncoming, WsError>>;
        type CloseFuture<'a> = FakeCloseFuture;

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
            let mut state = self.0.lock().unwrap();
            state.closes += 1;
            FakeCloseFuture(
                state
                    .close_outcomes
                    .pop_front()
                    .unwrap_or(CloseOutcome::Ready(Ok(()))),
            )
        }
    }

    #[test]
    fn endpoint_rejects_credentials_fragments_and_non_ws_schemes() {
        // Arrange
        let invalid = [
            "http://example.test",
            "ws://user:pass@example.test",
            "ws://example.test/#secret",
        ];

        // Act
        let results = invalid.map(WsEndpoint::parse);

        // Assert
        assert!(results.iter().all(Result::is_err));
        assert!(format!("{}", results[1].as_ref().unwrap_err()).contains("credentials"));
        assert!(!format!("{}", results[1].as_ref().unwrap_err()).contains("pass"));
    }

    #[test]
    fn endpoint_byte_limit_rejects_before_url_parse() {
        // Arrange
        let oversized = format!("ws://example.test/{}", "x".repeat(MAX_ENDPOINT_BYTES));

        // Act
        let result = WsEndpoint::parse(oversized);

        // Assert
        assert!(matches!(
            result,
            Err(WsError::LimitExceeded {
                resource: "endpoint",
                ..
            })
        ));
    }

    #[test]
    fn endpoint_rejects_unescaped_whitespace_and_controls() {
        // Arrange
        let invalid = [
            " ws://example.test",
            "ws://example.test/ ",
            "ws://example.test/\t",
            "ws://example.test/\n",
            "ws://example.test/\u{7f}",
        ];

        // Act
        let results = invalid.map(WsEndpoint::parse);

        // Assert
        assert!(
            results
                .iter()
                .all(|result| matches!(result, Err(WsError::InvalidEndpoint(_))))
        );
    }

    #[test]
    fn endpoint_canonicalizes_unicode_host_before_live_handshake() {
        // Arrange
        let input = "wss://bücher.example/path?q=%C3%A9";

        // Act
        let endpoint = WsEndpoint::parse(input).unwrap();

        // Assert
        assert_eq!(
            endpoint.as_str(),
            "wss://xn--bcher-kva.example/path?q=%C3%A9"
        );
    }

    #[test]
    fn endpoint_diagnostic_omits_sensitive_query() {
        // Arrange
        let endpoint =
            WsEndpoint::parse("ws://does-not-exist.problem-domain:10987?secret=MYSECRET").unwrap();

        // Act
        let diagnostic = endpoint.without_query();

        // Assert
        assert_eq!(diagnostic, "ws://does-not-exist.problem-domain:10987/");
        assert!(!diagnostic.contains("MYSECRET"));
    }

    #[test]
    fn header_checks_reject_injection_duplicates_and_reserved_names() {
        // Arrange
        let mut request = request();
        request.add_header("foo", "bar").unwrap();

        // Act
        let newline = request.add_header("other", "x\r\ny");
        let duplicate = request.add_header("FOO", "again");
        let reserved = request.add_header("Sec-WebSocket-Key", "override");

        // Assert
        assert!(matches!(newline, Err(WsError::InvalidHeader(_))));
        assert!(matches!(duplicate, Err(WsError::InvalidHeader(_))));
        assert!(matches!(reserved, Err(WsError::InvalidHeader(_))));
        assert_eq!(request.headers().iter().len(), 1);
    }

    #[test]
    fn http_discovery_headers_keep_limits_but_cannot_enter_a_websocket_request() {
        // Arrange
        let mut deferred = WsHeaders::default();
        deferred
            .push_for_http_discovery("Connection", "Close")
            .unwrap();

        // Act
        let injection = deferred.push_for_http_discovery("X-Other", "bad\r\nvalue");
        let duplicate = deferred.push_for_http_discovery("connection", "again");
        let mut request = request();
        let handshake = request.add_header("Connection", "Close");

        // Assert
        assert!(matches!(injection, Err(WsError::InvalidHeader(_))));
        assert!(matches!(duplicate, Err(WsError::InvalidHeader(_))));
        assert_eq!(
            handshake,
            Err(WsError::InvalidHeader(
                "handshake-controlled header is reserved"
            ))
        );
        assert_eq!(deferred.iter().len(), 1);
        assert_eq!(deferred.total_bytes(), "Connection".len() + "Close".len());
        assert_eq!(request.headers().iter().len(), 0);
    }

    #[test]
    fn http_discovery_reserved_header_still_consumes_count_and_byte_budget() {
        // Arrange
        let mut deferred = WsHeaders::default();
        deferred.push_for_http_discovery("Connection", "x").unwrap();
        for index in 1..MAX_HEADER_COUNT {
            deferred
                .push_for_http_discovery(&format!("x-{index}"), "")
                .unwrap();
        }

        // Act
        let overflow = deferred.push_for_http_discovery("x-overflow", "");
        let mut bytes = WsHeaders::default();
        for name in ["a", "b", "c"] {
            bytes
                .push_for_http_discovery(name, &"x".repeat(MAX_HEADER_VALUE_BYTES))
                .unwrap();
        }
        let byte_overflow =
            bytes.push_for_http_discovery("Connection", &"x".repeat(MAX_HEADER_VALUE_BYTES));

        // Assert
        assert!(matches!(
            overflow,
            Err(WsError::LimitExceeded {
                resource: "header count",
                ..
            })
        ));
        assert!(matches!(
            byte_overflow,
            Err(WsError::LimitExceeded {
                resource: "request headers",
                ..
            })
        ));
        assert_eq!(deferred.iter().len(), MAX_HEADER_COUNT);
        assert_eq!(bytes.iter().len(), 3);
    }

    #[test]
    fn header_budget_rejects_before_mutating_request() {
        // Arrange
        let mut request = request();
        let value = "x".repeat(MAX_HEADER_VALUE_BYTES);
        for name in ["a", "b", "c"] {
            request.add_header(name, &value).unwrap();
        }
        let previous_total = request.headers().total_bytes();

        // Act
        let result = request.add_header("d", &value);

        // Assert
        assert!(matches!(
            result,
            Err(WsError::LimitExceeded {
                resource: "request headers",
                ..
            })
        ));
        assert_eq!(request.headers().total_bytes(), previous_total);
        assert_eq!(request.headers().iter().len(), 3);
    }

    #[test]
    fn individual_header_value_limit_rejects_before_mutating_request() {
        // Arrange
        let mut request = request();
        let oversized = "x".repeat(MAX_HEADER_VALUE_BYTES + 1);

        // Act
        let result = request.add_header("foo", &oversized);

        // Assert
        assert!(matches!(result, Err(WsError::InvalidHeader(_))));
        assert_eq!(request.headers().total_bytes(), 0);
    }

    #[test]
    fn header_count_limit_rejects_before_mutating_request() {
        // Arrange
        let mut request = request();
        for index in 0..MAX_HEADER_COUNT {
            request.add_header(&format!("x-{index}"), "").unwrap();
        }
        let previous_total = request.headers().total_bytes();

        // Act
        let result = request.add_header("overflow", "");

        // Assert
        assert!(matches!(
            result,
            Err(WsError::LimitExceeded {
                resource: "header count",
                ..
            })
        ));
        assert_eq!(request.headers().iter().len(), MAX_HEADER_COUNT);
        assert_eq!(request.headers().total_bytes(), previous_total);
    }

    #[test]
    fn close_reason_has_exact_utf8_and_wire_limit() {
        // Arrange
        let reason = "Oh my!";

        // Act
        let frame = WsCloseFrame::new(4123, reason).unwrap();
        let too_long = WsCloseFrame::new(4123, "x".repeat(MAX_CLOSE_REASON_BYTES + 1));

        // Assert
        assert_eq!(frame.code(), 4123);
        assert_eq!(frame.reason(), reason);
        assert!(matches!(
            too_long,
            Err(WsError::LimitExceeded {
                resource: "close reason",
                ..
            })
        ));
    }

    #[tokio::test]
    async fn fake_connector_receives_exact_checked_request_and_close_reason() {
        // Arrange
        let state = Arc::new(Mutex::new(State::default()));
        state
            .lock()
            .unwrap()
            .received
            .push_back(WsIncoming::RemoteClose(Some(
                WsCloseFrame::new(4123, "Oh my!").unwrap(),
            )));
        let connector = FakeConnector(Arc::clone(&state));
        let mut request = request();
        request.add_header("foo", "bar").unwrap();

        // Act
        let mut transport = WsTransport::connect(&connector, &request).await.unwrap();
        let result = transport.receive().await.unwrap();

        // Assert
        assert_eq!(state.lock().unwrap().request.as_ref(), Some(&request));
        assert_eq!(
            result,
            WsIncoming::RemoteClose(Some(WsCloseFrame::new(4123, "Oh my!").unwrap()))
        );
        assert!(matches!(transport.receive().await, Err(WsError::Closed)));
    }

    #[tokio::test]
    async fn remote_close_receipt_drives_the_queued_reply_on_close() {
        // Arrange
        let state = Arc::new(Mutex::new(State::default()));
        state
            .lock()
            .unwrap()
            .received
            .push_back(WsIncoming::RemoteClose(Some(
                WsCloseFrame::new(4123, "Oh my!").unwrap(),
            )));
        let connector = FakeConnector(Arc::clone(&state));
        let mut transport = WsTransport::connect(&connector, &request()).await.unwrap();

        // Act
        let receipt = transport.receive().await.unwrap();
        transport.close().await.unwrap();
        transport.close().await.unwrap();

        // Assert
        assert_eq!(
            receipt,
            WsIncoming::RemoteClose(Some(WsCloseFrame::new(4123, "Oh my!").unwrap()))
        );
        assert_eq!(state.lock().unwrap().closes, 1);
    }

    #[tokio::test]
    async fn canceled_close_future_remains_retryable() {
        // Arrange
        let state = Arc::new(Mutex::new(State::default()));
        state
            .lock()
            .unwrap()
            .close_outcomes
            .extend([CloseOutcome::Pending, CloseOutcome::Ready(Ok(()))]);
        let connector = FakeConnector(Arc::clone(&state));
        let mut transport = WsTransport::connect(&connector, &request()).await.unwrap();

        // Act
        std::future::poll_fn(|context| {
            let mut canceled = Box::pin(transport.close());
            assert!(canceled.as_mut().poll(context).is_pending());
            Poll::Ready(())
        })
        .await;
        let retry = transport.close().await;

        // Assert
        assert_eq!(retry, Ok(()));
        assert_eq!(state.lock().unwrap().closes, 2);
    }

    #[tokio::test]
    async fn failed_close_remains_retryable_instead_of_silent_success() {
        // Arrange
        let state = Arc::new(Mutex::new(State::default()));
        state.lock().unwrap().close_outcomes.extend([
            CloseOutcome::Ready(Err(WsError::IoFailed)),
            CloseOutcome::Ready(Ok(())),
        ]);
        let connector = FakeConnector(Arc::clone(&state));
        let mut transport = WsTransport::connect(&connector, &request()).await.unwrap();

        // Act
        let failure = transport.close().await;
        let retry = transport.close().await;

        // Assert
        assert_eq!(failure, Err(WsError::IoFailed));
        assert_eq!(retry, Ok(()));
        assert_eq!(state.lock().unwrap().closes, 2);
    }

    #[tokio::test]
    async fn oversized_send_never_reaches_peer() {
        // Arrange
        let state = Arc::new(Mutex::new(State::default()));
        let connector = FakeConnector(Arc::clone(&state));
        let mut transport = WsTransport::connect(&connector, &request()).await.unwrap();

        // Act
        let result = transport.send(WsOutgoing::Text("x".repeat(17))).await;

        // Assert
        assert!(matches!(
            result,
            Err(WsError::LimitExceeded {
                resource: "outbound message",
                ..
            })
        ));
        assert!(state.lock().unwrap().sent.is_empty());
    }

    #[tokio::test]
    async fn oversized_receive_is_rejected_even_from_a_fake_peer() {
        // Arrange
        let state = Arc::new(Mutex::new(State::default()));
        state
            .lock()
            .unwrap()
            .received
            .push_back(WsIncoming::Binary(vec![0; 17]));
        let connector = FakeConnector(Arc::clone(&state));
        let mut transport = WsTransport::connect(&connector, &request()).await.unwrap();

        // Act
        let result = transport.receive().await;

        // Assert
        assert!(matches!(
            result,
            Err(WsError::LimitExceeded {
                resource: "inbound message",
                ..
            })
        ));
        assert!(matches!(transport.receive().await, Err(WsError::Closed)));
        transport.close().await.unwrap();
        assert_eq!(state.lock().unwrap().closes, 1);
    }

    #[tokio::test]
    async fn close_is_idempotent_and_eof_is_not_a_close_frame() {
        // Arrange
        let state = Arc::new(Mutex::new(State::default()));
        let connector = FakeConnector(Arc::clone(&state));
        let mut transport = WsTransport::connect(&connector, &request()).await.unwrap();

        // Act
        let eof = transport.receive().await.unwrap();
        transport.close().await.unwrap();

        // Assert
        assert_eq!(eof, WsIncoming::EndOfStream);
        assert_eq!(state.lock().unwrap().closes, 0);
    }
}
