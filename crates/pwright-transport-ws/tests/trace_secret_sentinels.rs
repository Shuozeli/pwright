#![cfg(feature = "live-tokio")]

use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use futures_util::{SinkExt, StreamExt};
use log::{LevelFilter, Log, Metadata, Record};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, Message, frame::coding::CloseCode};

struct CapturedLog {
    lines: Mutex<Vec<String>>,
}

static CAPTURED_LOG: CapturedLog = CapturedLog {
    lines: Mutex::new(Vec::new()),
};

impl Log for CapturedLog {
    fn enabled(&self, _: &Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &Record<'_>) {
        self.lines.lock().expect("log capture lock").push(format!(
            "{}: {}",
            record.target(),
            record.args()
        ));
    }

    fn flush(&self) {}
}

struct CaptureIncoming<S> {
    inner: S,
    bytes: Arc<Mutex<Vec<u8>>>,
}

impl<S: AsyncRead + Unpin> AsyncRead for CaptureIncoming<S> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        let before = buffer.filled().len();
        let result = Pin::new(&mut this.inner).poll_read(cx, buffer);
        if let Poll::Ready(Ok(())) = result {
            this.bytes
                .lock()
                .expect("incoming bytes lock")
                .extend_from_slice(&buffer.filled()[before..]);
        }
        result
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for CaptureIncoming<S> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        data: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.get_mut().inner).poll_write(cx, data)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}

#[tokio::test]
async fn complete_websocket_exchange_never_logs_secret_sentinels() {
    // Arrange: this integration test is its own process, so no application
    // logger or other test can compete for the one process-global log sink.
    log::set_logger(&CAPTURED_LOG).expect("install isolated TRACE collector");
    log::set_max_level(LevelFilter::Trace);

    const QUERY: &str = "querySecretPwrtA5e6";
    const AUTH: &str = "authSecretPwrtB6f7";
    const COOKIE: &str = "cookieSecretPwrtC7g8";
    const CUSTOM: &str = "customSecretPwrtD8h9";
    const OUT_TEXT: &str = "outTextSecretPwrtE9i0";
    const OUT_BINARY: &str = "outBinarySecretPwrtF0j1";
    const IN_TEXT: &str = "inTextSecretPwrtG1k2";
    const IN_BINARY: &str = "inBinarySecretPwrtH2l3";
    const CLOSE: &str = "closeSecretPwrtI3m4";
    const REJECTED_BODY: &str = "httpBodySecretPwrtJ4n5";

    let (client_io, server_io) = tokio::io::duplex(32 * 1024);
    let captured_request = Arc::new(Mutex::new(Vec::new()));
    let server_io = CaptureIncoming {
        inner: server_io,
        bytes: Arc::clone(&captured_request),
    };
    let server = tokio::spawn(async move {
        let mut socket = tokio_tungstenite::accept_async(server_io)
            .await
            .expect("real server-side WebSocket upgrade");

        // Assert that the exact outbound bytes reached the peer.
        assert_eq!(
            socket
                .next()
                .await
                .expect("outbound text")
                .expect("text frame"),
            Message::Text(OUT_TEXT.into())
        );
        assert_eq!(
            socket
                .next()
                .await
                .expect("outbound binary")
                .expect("binary frame"),
            Message::Binary(OUT_BINARY.as_bytes().to_vec().into())
        );

        socket
            .send(Message::Text(IN_TEXT.into()))
            .await
            .expect("send inbound text");
        socket
            .send(Message::Binary(IN_BINARY.as_bytes().to_vec().into()))
            .await
            .expect("send inbound binary");
        socket
            .send(Message::Close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: CLOSE.into(),
            })))
            .await
            .expect("send close reason");
    });

    let mut request = "ws://pwright.test/ws?token=querySecretPwrtA5e6"
        .into_client_request()
        .expect("client request");
    request.headers_mut().insert(
        "authorization",
        HeaderValue::from_static("Bearer authSecretPwrtB6f7"),
    );
    request.headers_mut().insert(
        "cookie",
        HeaderValue::from_static("session=cookieSecretPwrtC7g8"),
    );
    request.headers_mut().insert(
        "x-pwright-secret",
        HeaderValue::from_static("customSecretPwrtD8h9"),
    );

    // Act: the real forked client and server perform an upgrade and exchange
    // data and Close frames over an in-memory duplex transport.
    let (mut client, _) = tokio_tungstenite::client_async(request, client_io)
        .await
        .expect("real client-side WebSocket upgrade");
    client
        .send(Message::Text(OUT_TEXT.into()))
        .await
        .expect("send outbound text");
    client
        .send(Message::Binary(OUT_BINARY.as_bytes().to_vec().into()))
        .await
        .expect("send outbound binary");
    assert_eq!(
        client
            .next()
            .await
            .expect("inbound text")
            .expect("text frame"),
        Message::Text(IN_TEXT.into())
    );
    assert_eq!(
        client
            .next()
            .await
            .expect("inbound binary")
            .expect("binary frame"),
        Message::Binary(IN_BINARY.as_bytes().to_vec().into())
    );
    let close = client
        .next()
        .await
        .expect("inbound close")
        .expect("close frame");
    assert_eq!(
        close,
        Message::Close(Some(CloseFrame {
            code: CloseCode::Normal,
            reason: CLOSE.into(),
        }))
    );
    server.await.expect("server completed");

    // A rejected HTTP upgrade also carries caller-visible body text, but
    // the forked handshake implementation must not write it to TRACE logs.
    let (http_client_io, mut http_server_io) = tokio::io::duplex(4096);
    let http_server = tokio::spawn(async move {
        let mut request = Vec::new();
        let mut chunk = [0_u8; 1024];
        while !request.ends_with(b"\r\n\r\n") {
            let count = http_server_io.read(&mut chunk).await.unwrap();
            assert_ne!(count, 0);
            request.extend_from_slice(&chunk[..count]);
        }
        http_server_io
            .write_all(b"HTTP/1.1 401 Unauthorized\r\n\r\nhttpBodySecretPwrtJ4n5")
            .await
            .unwrap();
    });
    let rejected_request = "ws://pwright.test/ws-401"
        .into_client_request()
        .expect("HTTP rejection request");
    let rejected = tokio_tungstenite::client_async(rejected_request, http_client_io)
        .await
        .expect_err("upgrade must be rejected");
    let tokio_tungstenite::tungstenite::Error::Http(response) = rejected else {
        panic!("expected HTTP handshake refusal");
    };
    assert_eq!(response.status(), 401);
    assert_eq!(response.body().as_deref(), Some(REJECTED_BODY.as_bytes()));
    http_server.await.expect("HTTP rejection server completed");

    // Assert: neither cleartext nor the frame formatter's hexadecimal
    // rendering of any secret reached any TRACE/DEBUG record.
    let request_bytes = captured_request.lock().expect("incoming bytes lock");
    let header_end = request_bytes
        .windows(4)
        .position(|bytes| bytes == b"\r\n\r\n")
        .expect("HTTP upgrade headers")
        + 4;
    let request_text = std::str::from_utf8(&request_bytes[..header_end]).expect("ASCII upgrade");
    let mut request_lines = request_text.split("\r\n");
    assert_eq!(
        request_lines.next(),
        Some(&format!("GET /ws?token={QUERY} HTTP/1.1")[..])
    );
    for (name, expected) in [
        ("authorization", format!("Bearer {AUTH}")),
        ("cookie", format!("session={COOKIE}")),
        ("x-pwright-secret", CUSTOM.to_owned()),
    ] {
        let actual = request_lines
            .clone()
            .filter_map(|line| line.split_once(':'))
            .filter(|(header_name, _)| header_name.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.trim())
            .collect::<Vec<_>>();
        assert_eq!(actual, vec![expected.as_str()]);
    }

    let lines = CAPTURED_LOG.lines.lock().expect("log capture lock");
    let logs = lines.join("\n");
    let logs_lower = logs.to_ascii_lowercase();
    for sentinel in [
        QUERY,
        AUTH,
        COOKIE,
        CUSTOM,
        OUT_TEXT,
        OUT_BINARY,
        IN_TEXT,
        IN_BINARY,
        CLOSE,
        REJECTED_BODY,
    ] {
        let hex = sentinel
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert!(
            !logs.contains(sentinel),
            "cleartext sentinel leaked in logs: {sentinel}"
        );
        assert!(
            !logs_lower.contains(&hex),
            "hex-encoded sentinel leaked in logs: {sentinel}"
        );
    }
    assert!(
        logs.contains("Client upgrade request serialized"),
        "TRACE collector did not observe the forked handshake path"
    );
    assert!(
        logs.contains("Received WebSocket message"),
        "TRACE collector did not observe the forked frame path"
    );
}
