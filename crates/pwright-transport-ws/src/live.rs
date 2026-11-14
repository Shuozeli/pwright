//! Opt-in direct WebSocket client. No listener, process, or background task.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpStream, lookup_host};
use tokio_tungstenite::tungstenite::Error as TungsteniteError;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue, Request};
use tokio_tungstenite::tungstenite::protocol::{Message, WebSocketConfig};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async_with_config,
    connect_async_with_config_capture_rejection_body,
};

use crate::{
    MAX_HTTP_REJECTION_BODY_BYTES, WsCloseFrame, WsConnector, WsError, WsIncoming, WsOutgoing,
    WsPeer, WsRequest,
};

/// Direct client connector. The selected Pwright Tungstenite forks suppress
/// handshake and frame contents in TRACE logs; the sentinel test exercises a
/// complete exchange with query, header, frame, and close-reason secrets.
#[derive(Clone, Copy, Debug, Default)]
pub struct LiveWsConnector;

/// Opt-in connector for Playwright Server's HTTP 428 response body. Ordinary
/// WebSocket and CDP callers keep the immediate-rejection connector above.
#[derive(Clone, Copy, Debug, Default)]
pub struct LiveWsRejectionBodyConnector;

impl LiveWsConnector {
    /// Classify resolver failures for the opt-in Playwright Server error path.
    /// The ordinary connector keeps its existing transport error policy. This
    /// preflight and the WebSocket upgrade are both bounded by one deadline.
    pub async fn connect_with_dns_classification(
        &self,
        request: &WsRequest,
    ) -> Result<LiveWsPeer, WsError> {
        let endpoint = url::Url::parse(request.endpoint().as_str())
            .map_err(|_| WsError::InvalidEndpoint("expected an absolute URL"))?;
        let port = endpoint
            .port_or_known_default()
            .ok_or(WsError::ConnectFailed)?;
        let deadline = Duration::from_millis(u64::from(request.limits().connect_timeout_ms()));
        tokio::time::timeout(deadline, async {
            if let Some(url::Host::Domain(host)) = endpoint.host() {
                let mut addresses = lookup_host((host, port))
                    .await
                    .map_err(|_| WsError::DnsLookupFailed)?;
                if addresses.next().is_none() {
                    return Err(WsError::DnsLookupFailed);
                }
            }
            self.connect(request).await
        })
        .await
        .map_err(|_| WsError::ConnectTimeout)?
    }
}

impl WsConnector for LiveWsConnector {
    type Peer = LiveWsPeer;
    type Future<'a> = Pin<Box<dyn Future<Output = Result<Self::Peer, WsError>> + Send + 'a>>;

    fn connect(&self, request: &WsRequest) -> Self::Future<'_> {
        connect_with_rejection_body_policy(request, false)
    }
}

impl WsConnector for LiveWsRejectionBodyConnector {
    type Peer = LiveWsPeer;
    type Future<'a> = Pin<Box<dyn Future<Output = Result<Self::Peer, WsError>> + Send + 'a>>;

    fn connect(&self, request: &WsRequest) -> Self::Future<'_> {
        connect_with_rejection_body_policy(request, true)
    }
}

fn connect_with_rejection_body_policy(
    request: &WsRequest,
    capture_rejection_body: bool,
) -> Pin<Box<dyn Future<Output = Result<LiveWsPeer, WsError>> + Send + 'static>> {
    let handshake = handshake_request(request);
    let limits = request.limits();
    Box::pin(async move {
        let handshake = handshake?;
        let config = WebSocketConfig::default()
            .write_buffer_size(0)
            .max_write_buffer_size(limits.outbound_message_bytes().saturating_add(1024))
            .max_message_size(Some(limits.inbound_message_bytes()))
            .max_frame_size(Some(limits.inbound_message_bytes()));
        let connect = async move {
            if capture_rejection_body {
                connect_async_with_config_capture_rejection_body(handshake, Some(config), false)
                    .await
            } else {
                connect_async_with_config(handshake, Some(config), false).await
            }
        };
        let (stream, _) = bounded_upgrade(
            Duration::from_millis(u64::from(limits.connect_timeout_ms())),
            connect,
        )
        .await?;
        Ok(LiveWsPeer { stream })
    })
}

async fn bounded_upgrade<T>(
    budget: Duration,
    connect: impl Future<Output = Result<T, TungsteniteError>>,
) -> Result<T, WsError> {
    tokio::time::timeout(budget, connect)
        .await
        .map_err(|_| WsError::ConnectTimeout)?
        .map_err(classify_connect_error)
}

fn classify_connect_error(error: TungsteniteError) -> WsError {
    match error {
        TungsteniteError::Http(response) => {
            let status = response.status().as_u16();
            let body = response
                .into_body()
                .map(|bytes| bounded_rejection_body(&bytes))
                .unwrap_or_default();
            WsError::HttpRejected { status, body }
        }
        _ => WsError::ConnectFailed,
    }
}

fn bounded_rejection_body(bytes: &[u8]) -> String {
    let mut body = String::with_capacity(MAX_HTTP_REJECTION_BODY_BYTES);
    let mut remaining = &bytes[..bytes.len().min(MAX_HTTP_REJECTION_BODY_BYTES)];
    while !remaining.is_empty() && body.len() < MAX_HTTP_REJECTION_BODY_BYTES {
        match std::str::from_utf8(remaining) {
            Ok(valid) => {
                append_bounded_utf8(&mut body, valid);
                break;
            }
            Err(error) => {
                let valid = std::str::from_utf8(&remaining[..error.valid_up_to()])
                    .expect("valid_up_to is a UTF-8 boundary");
                if !append_bounded_utf8(&mut body, valid)
                    || MAX_HTTP_REJECTION_BODY_BYTES - body.len() < '\u{fffd}'.len_utf8()
                {
                    break;
                }
                body.push('\u{fffd}');
                let invalid_len = error
                    .error_len()
                    .unwrap_or(remaining.len() - error.valid_up_to());
                remaining = &remaining[error.valid_up_to() + invalid_len..];
            }
        }
    }
    body
}

fn append_bounded_utf8(body: &mut String, valid: &str) -> bool {
    let mut end = valid.len().min(MAX_HTTP_REJECTION_BODY_BYTES - body.len());
    while !valid.is_char_boundary(end) {
        end -= 1;
    }
    body.push_str(&valid[..end]);
    end == valid.len()
}

fn handshake_request(request: &WsRequest) -> Result<Request<()>, WsError> {
    let mut handshake = request
        .endpoint()
        .as_str()
        .into_client_request()
        .map_err(|_| WsError::ConnectFailed)?;
    for entry in request.headers().iter() {
        let name = HeaderName::from_bytes(entry.name().as_bytes())
            .map_err(|_| WsError::InvalidHeader("invalid handshake name"))?;
        let value = HeaderValue::from_str(entry.value())
            .map_err(|_| WsError::InvalidHeader("invalid handshake value"))?;
        handshake.headers_mut().insert(name, value);
    }
    Ok(handshake)
}

fn incoming_message(message: Message) -> Result<Option<WsIncoming>, WsError> {
    match message {
        Message::Text(text) => Ok(Some(WsIncoming::Text(text.to_string()))),
        Message::Binary(bytes) => Ok(Some(WsIncoming::Binary(bytes.to_vec()))),
        Message::Close(Some(frame)) => Ok(Some(WsIncoming::RemoteClose(Some(WsCloseFrame::new(
            u16::from(frame.code),
            frame.reason.to_string(),
        )?)))),
        Message::Close(None) => Ok(Some(WsIncoming::RemoteClose(None))),
        _ => Ok(None),
    }
}

pub struct LiveWsPeer {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

async fn flush_close_reply<Stream>(stream: &mut WebSocketStream<Stream>) -> Result<(), WsError>
where
    Stream: AsyncRead + AsyncWrite + Unpin,
{
    // This is SinkExt::close, not WebSocketStream's inherent close(None).
    // The latter attempts to send a new Close and fails after a peer Close.
    SinkExt::close(stream).await.map_err(|_| WsError::IoFailed)
}

impl WsPeer for LiveWsPeer {
    type SendFuture<'a> = Pin<Box<dyn Future<Output = Result<(), WsError>> + Send + 'a>>;
    type ReceiveFuture<'a> = Pin<Box<dyn Future<Output = Result<WsIncoming, WsError>> + Send + 'a>>;
    type CloseFuture<'a> = Pin<Box<dyn Future<Output = Result<(), WsError>> + Send + 'a>>;

    fn send(&mut self, message: WsOutgoing) -> Self::SendFuture<'_> {
        Box::pin(async move {
            let message = match message {
                WsOutgoing::Text(text) => Message::Text(text.into()),
                WsOutgoing::Binary(bytes) => Message::Binary(bytes.into()),
            };
            self.stream
                .send(message)
                .await
                .map_err(|_| WsError::IoFailed)
        })
    }

    fn receive(&mut self) -> Self::ReceiveFuture<'_> {
        Box::pin(async move {
            loop {
                match self.stream.next().await {
                    Some(Ok(message)) => {
                        if let Some(incoming) = incoming_message(message)? {
                            return Ok(incoming);
                        }
                    }
                    Some(Err(_)) => return Err(WsError::IoFailed),
                    None => return Ok(WsIncoming::EndOfStream),
                }
            }
        })
    }

    fn close(&mut self) -> Self::CloseFuture<'_> {
        // Sink close flushes a queued peer-close reply, even after cancellation.
        Box::pin(async move { flush_close_reply(&mut self.stream).await })
    }
}

#[cfg(test)]
mod tests {
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use std::time::Duration;

    use futures_util::{SinkExt, StreamExt};
    use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
    use tokio_tungstenite::tungstenite::Error as TungsteniteError;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    use tokio_tungstenite::tungstenite::http::Response;
    use tokio_tungstenite::tungstenite::protocol::{CloseFrame, Message};
    use tokio_tungstenite::{accept_async, client_async};

    use super::{
        bounded_upgrade, classify_connect_error, flush_close_reply, handshake_request,
        incoming_message,
    };
    use crate::{
        MAX_HTTP_REJECTION_BODY_BYTES, WsEndpoint, WsError, WsIncoming, WsLimits, WsRequest,
    };

    #[derive(Debug)]
    struct SplitHeaders {
        stream: tokio::io::DuplexStream,
        header_bytes_remaining: usize,
    }

    impl AsyncRead for SplitHeaders {
        fn poll_read(
            self: Pin<&mut Self>,
            context: &mut Context<'_>,
            buffer: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            let this = self.get_mut();
            if this.header_bytes_remaining == 0 {
                return Pin::new(&mut this.stream).poll_read(context, buffer);
            }
            let mut chunk = [0_u8; 512];
            let limit = buffer
                .remaining()
                .min(this.header_bytes_remaining)
                .min(chunk.len());
            let mut limited = ReadBuf::new(&mut chunk[..limit]);
            match Pin::new(&mut this.stream).poll_read(context, &mut limited) {
                Poll::Ready(Ok(())) => {
                    let read = limited.filled().len();
                    buffer.put_slice(limited.filled());
                    this.header_bytes_remaining -= read;
                    Poll::Ready(Ok(()))
                }
                other => other,
            }
        }
    }

    impl AsyncWrite for SplitHeaders {
        fn poll_write(
            self: Pin<&mut Self>,
            context: &mut Context<'_>,
            bytes: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            Pin::new(&mut self.get_mut().stream).poll_write(context, bytes)
        }

        fn poll_flush(
            self: Pin<&mut Self>,
            context: &mut Context<'_>,
        ) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.get_mut().stream).poll_flush(context)
        }

        fn poll_shutdown(
            self: Pin<&mut Self>,
            context: &mut Context<'_>,
        ) -> Poll<std::io::Result<()>> {
            Pin::new(&mut self.get_mut().stream).poll_shutdown(context)
        }
    }

    async fn split_http_rejection(content_length: Option<usize>, body: Vec<u8>) -> WsError {
        let headers = match content_length {
            Some(length) => format!(
                "HTTP/1.1 401 Unauthorized\r\nContent-Length: {length}\r\nConnection: keep-alive\r\n\r\n"
            ),
            None => "HTTP/1.1 401 Unauthorized\r\nConnection: keep-alive\r\n\r\n".into(),
        };
        let header_len = headers.len();
        let (client_io, mut server_io) = tokio::io::duplex(32 * 1024);
        let (release, held_open) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::spawn(async move {
            let mut request = Vec::new();
            let mut chunk = [0_u8; 1024];
            while !request.ends_with(b"\r\n\r\n") {
                let count = server_io.read(&mut chunk).await.unwrap();
                assert_ne!(count, 0);
                request.extend_from_slice(&chunk[..count]);
            }
            server_io.write_all(headers.as_bytes()).await.unwrap();
            server_io.write_all(&body).await.unwrap();
            held_open.await.unwrap();
        });
        let client_io = SplitHeaders {
            stream: client_io,
            header_bytes_remaining: header_len,
        };

        let rejected = tokio::time::timeout(
            Duration::from_secs(1),
            client_async("ws://pwright.test/ws-401", client_io),
        )
        .await
        .expect("HTTP rejection must complete without waiting for keep-alive EOF")
        .unwrap_err();
        release.send(()).unwrap();
        server.await.unwrap();
        classify_connect_error(rejected)
    }

    #[tokio::test]
    async fn peer_close_reply_flushes_without_sending_a_second_close() {
        // Arrange
        let (client_io, server_io) = tokio::io::duplex(16 * 1024);
        let server = tokio::spawn(async move {
            let mut peer = accept_async(server_io).await.unwrap();
            peer.send(Message::Close(Some(CloseFrame {
                code: 4123.into(),
                reason: "Oh my!".into(),
            })))
            .await
            .unwrap();
            let reply = peer.next().await;
            let Some(Ok(Message::Close(Some(reply)))) = reply else {
                panic!("expected matching Close reply, got {reply:?}");
            };
            assert_eq!(u16::from(reply.code), 4123);
            assert_eq!(reply.reason, "Oh my!");
        });
        let (mut client, _) = client_async("ws://example.test/ws", client_io)
            .await
            .unwrap();
        let incoming = client.next().await.unwrap().unwrap();
        assert!(matches!(incoming, Message::Close(Some(_))));

        // Act
        let closed = flush_close_reply(&mut client).await;

        // Assert
        assert_eq!(closed, Ok(()));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn rejected_upgrade_body_survives_real_websocket_handshake() {
        // Arrange
        let (client_io, mut server_io) = tokio::io::duplex(16 * 1024);
        let server = tokio::spawn(async move {
            let mut request = Vec::new();
            let mut bytes = [0_u8; 1024];
            while !request.ends_with(b"\r\n\r\n") {
                let count = server_io.read(&mut bytes).await.unwrap();
                assert_ne!(count, 0);
                request.extend_from_slice(&bytes[..count]);
            }
            assert!(request.starts_with(b"GET /ws-401 HTTP/1.1\r\n"));
            server_io
                .write_all(b"HTTP/1.1 401 Unauthorized\r\n\r\nUnauthorized body")
                .await
                .unwrap();
        });
        let request = "ws://pwright.test/ws-401".into_client_request().unwrap();

        // Act
        let error = tokio_tungstenite::client_async(request, client_io)
            .await
            .unwrap_err();
        let error = classify_connect_error(error);
        server.await.unwrap();

        // Assert
        assert_eq!(
            error,
            WsError::HttpRejected {
                status: 401,
                body: "Unauthorized body".into(),
            }
        );
    }

    #[tokio::test]
    async fn rejected_upgrade_reads_declared_body_after_split_headers_without_eof() {
        // Arrange
        let expected = b"Unauthorized body".to_vec();

        // Act
        let error = split_http_rejection(Some(expected.len()), expected).await;

        // Assert
        assert_eq!(
            error,
            WsError::HttpRejected {
                status: 401,
                body: "Unauthorized body".into(),
            }
        );
    }

    #[tokio::test]
    async fn rejected_upgrade_reads_only_eight_kib_of_oversized_declared_body() {
        // Arrange
        let body = vec![b'x'; MAX_HTTP_REJECTION_BODY_BYTES * 2];

        // Act
        let error = split_http_rejection(Some(body.len()), body).await;

        // Assert
        assert_eq!(
            error,
            WsError::HttpRejected {
                status: 401,
                body: "x".repeat(MAX_HTTP_REJECTION_BODY_BYTES),
            }
        );
    }

    #[tokio::test]
    async fn rejected_upgrade_caps_invalid_utf8_after_split_body_reads() {
        // Arrange
        let body = vec![0xff; MAX_HTTP_REJECTION_BODY_BYTES];

        // Act
        let error = split_http_rejection(Some(body.len()), body).await;

        // Assert
        let WsError::HttpRejected { status, body } = error else {
            panic!("expected HTTP rejection");
        };
        assert_eq!(status, 401);
        assert_eq!(body.len(), MAX_HTTP_REJECTION_BODY_BYTES - 2);
        assert!(body.chars().all(|character| character == '\u{fffd}'));
        assert!(body.capacity() <= MAX_HTTP_REJECTION_BODY_BYTES);
    }

    #[tokio::test]
    async fn rejected_upgrade_without_length_returns_without_waiting_for_eof() {
        // Arrange
        let body = b"body in later read".to_vec();

        // Act
        let error = split_http_rejection(None, body).await;

        // Assert
        assert_eq!(
            error,
            WsError::HttpRejected {
                status: 401,
                body: String::new(),
            }
        );
    }

    #[tokio::test]
    async fn opt_in_rejected_428_collects_split_body_without_content_length() {
        // Arrange
        let (client_io, mut server_io) = tokio::io::duplex(16 * 1024);
        let headers = b"HTTP/1.1 428 Precondition Required\r\n\r\n";
        let server = tokio::spawn(async move {
            let mut request = Vec::new();
            let mut bytes = [0_u8; 1024];
            while !request.ends_with(b"\r\n\r\n") {
                let count = server_io.read(&mut bytes).await.unwrap();
                assert_ne!(count, 0);
                request.extend_from_slice(&bytes[..count]);
            }
            server_io.write_all(headers).await.unwrap();
            server_io
                .write_all(b"Playwright version mismatch:\n  - server version: v1.2\n  - client version: v1.58")
                .await
                .unwrap();
        });
        let request = "ws://pwright.test/ws-428".into_client_request().unwrap();
        let client_io = SplitHeaders {
            stream: client_io,
            header_bytes_remaining: headers.len(),
        };

        // Act
        let error = tokio_tungstenite::client_async_with_config_capture_rejection_body(
            request, client_io, None,
        )
        .await
        .unwrap_err();
        server.await.unwrap();

        // Assert
        assert_eq!(
            classify_connect_error(error),
            WsError::HttpRejected {
                status: 428,
                body: "Playwright version mismatch:\n  - server version: v1.2\n  - client version: v1.58".into(),
            }
        );
    }

    #[tokio::test]
    async fn opt_in_undeclared_rejection_body_is_capped_and_connect_is_deadline_bounded() {
        // Arrange
        let (client_io, mut server_io) = tokio::io::duplex(32 * 1024);
        let server = tokio::spawn(async move {
            let mut request = Vec::new();
            let mut bytes = [0_u8; 1024];
            while !request.ends_with(b"\r\n\r\n") {
                let count = server_io.read(&mut bytes).await.unwrap();
                assert_ne!(count, 0);
                request.extend_from_slice(&bytes[..count]);
            }
            server_io
                .write_all(b"HTTP/1.1 428 Precondition Required\r\n\r\n")
                .await
                .unwrap();
            server_io
                .write_all(&vec![b'x'; MAX_HTTP_REJECTION_BODY_BYTES * 2])
                .await
                .unwrap();
            // Keep the stream open: reaching the cap must finish the rejection.
            tokio::time::sleep(Duration::from_millis(200)).await;
        });
        let request = "ws://pwright.test/ws-428".into_client_request().unwrap();

        // Act
        let error = tokio::time::timeout(
            Duration::from_millis(100),
            tokio_tungstenite::client_async_with_config_capture_rejection_body(
                request, client_io, None,
            ),
        )
        .await
        .expect("bounded rejection body must not wait for keep-alive EOF")
        .unwrap_err();
        server.await.unwrap();

        // Assert
        assert_eq!(
            classify_connect_error(error),
            WsError::HttpRejected {
                status: 428,
                body: "x".repeat(MAX_HTTP_REJECTION_BODY_BYTES),
            }
        );
    }

    #[tokio::test]
    async fn opt_in_partial_428_held_open_uses_finite_connect_timeout() {
        // Arrange
        let (client_io, mut server_io) = tokio::io::duplex(16 * 1024);
        let server = tokio::spawn(async move {
            let mut request = Vec::new();
            let mut bytes = [0_u8; 1024];
            while !request.ends_with(b"\r\n\r\n") {
                let count = server_io.read(&mut bytes).await.unwrap();
                assert_ne!(count, 0);
                request.extend_from_slice(&bytes[..count]);
            }
            server_io
                .write_all(b"HTTP/1.1 428 Precondition Required\r\n\r\npartial body")
                .await
                .unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
        });
        let request = "ws://pwright.test/ws-428".into_client_request().unwrap();

        // Act
        let outcome = bounded_upgrade(
            Duration::from_millis(20),
            tokio_tungstenite::client_async_with_config_capture_rejection_body(
                request, client_io, None,
            ),
        )
        .await;
        server.await.unwrap();

        // Assert
        assert!(matches!(outcome, Err(WsError::ConnectTimeout)));
    }

    #[test]
    fn rejected_http_upgrade_preserves_bounded_status_and_body() {
        // Arrange
        let response = Response::builder()
            .status(401)
            .body(Some(b"Unauthorized body".to_vec()))
            .unwrap();

        // Act
        let error = classify_connect_error(TungsteniteError::Http(response));

        // Assert
        assert_eq!(
            error,
            WsError::HttpRejected {
                status: 401,
                body: "Unauthorized body".into(),
            }
        );
        assert!(error.to_string().contains("401"));
        assert!(error.to_string().contains("Unauthorized body"));
        assert!(!format!("{error:?}").contains("Unauthorized body"));
    }

    #[test]
    fn rejected_http_upgrade_caps_body_before_exposing_it() {
        // Arrange
        let response = Response::builder()
            .status(503)
            .body(Some(vec![b'x'; MAX_HTTP_REJECTION_BODY_BYTES + 1]))
            .unwrap();

        // Act
        let error = classify_connect_error(TungsteniteError::Http(response));

        // Assert
        let WsError::HttpRejected { status, body } = error else {
            panic!("HTTP refusal was not classified");
        };
        assert_eq!(status, 503);
        assert_eq!(body.len(), MAX_HTTP_REJECTION_BODY_BYTES);
    }

    #[test]
    fn rejected_http_upgrade_caps_lossy_utf8_expansion() {
        // Arrange
        let response = Response::builder()
            .status(500)
            .body(Some(vec![0xff; MAX_HTTP_REJECTION_BODY_BYTES]))
            .unwrap();

        // Act
        let error = classify_connect_error(TungsteniteError::Http(response));

        // Assert
        let WsError::HttpRejected { body, .. } = error else {
            panic!("HTTP refusal was not classified");
        };
        assert!(body.len() <= MAX_HTTP_REJECTION_BODY_BYTES);
        assert!(body.is_char_boundary(body.len()));
    }

    #[test]
    fn checked_headers_are_present_without_replacing_upgrade_fields() {
        // Arrange
        let endpoint = WsEndpoint::parse("wss://example.test/ws?token=redacted").unwrap();
        let limits = WsLimits::new(4096, 4096, 1000).unwrap();
        let mut request = WsRequest::new(endpoint, limits);
        request.add_header("foo", "bar").unwrap();

        // Act
        let handshake = handshake_request(&request).unwrap();

        // Assert
        assert_eq!(handshake.headers()["foo"], "bar");
        assert_eq!(handshake.headers()["connection"], "Upgrade");
        assert!(handshake.headers().contains_key("sec-websocket-key"));
        assert_eq!(
            handshake.uri().path_and_query().unwrap().as_str(),
            "/ws?token=redacted"
        );
    }

    #[test]
    fn canonical_unicode_endpoint_builds_a_live_handshake() {
        // Arrange
        let endpoint = WsEndpoint::parse("wss://bücher.example/ws").unwrap();
        let limits = WsLimits::new(4096, 4096, 1000).unwrap();
        let request = WsRequest::new(endpoint, limits);

        // Act
        let handshake = handshake_request(&request).unwrap();

        // Assert
        assert_eq!(handshake.uri().host(), Some("xn--bcher-kva.example"));
    }

    #[test]
    fn live_message_conversion_preserves_nonstandard_close_code_and_reason() {
        // Arrange
        let frame = CloseFrame {
            code: 4123.into(),
            reason: "Oh my!".into(),
        };

        // Act
        let event = incoming_message(Message::Close(Some(frame))).unwrap();

        // Assert
        let Some(WsIncoming::RemoteClose(Some(frame))) = event else {
            panic!("close frame was not delivered");
        };
        assert_eq!(frame.code(), 4123);
        assert_eq!(frame.reason(), "Oh my!");
    }
}
