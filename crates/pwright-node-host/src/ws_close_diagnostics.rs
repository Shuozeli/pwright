use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::{accept_async, accept_hdr_async};

use crate::HostSession;

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
            // Portable CI without Tailscale: this one-shot fixture is loopback
            // only and never opens a wildcard or long-running listener.
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

#[tokio::test]
async fn cdp_attach_reports_remote_close_reason_as_browser_logs() {
    // Arrange
    let (bind_ip, client_host) = one_shot_fixture_address();
    let listener = TcpListener::bind(SocketAddr::from((bind_ip, 0)))
        .await
        .expect("test listener should bind");
    let port = listener.local_addr().expect("bound address").port();
    let (header_tx, header_rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("client should connect");
        let mut header_tx = Some(header_tx);
        let mut websocket = accept_hdr_async(
            stream,
            move |request: &tokio_tungstenite::tungstenite::handshake::server::Request,
                  response| {
                let value = request
                    .headers()
                    .get("foo")
                    .map(|value| value.as_bytes().to_vec());
                header_tx
                    .take()
                    .expect("one handshake")
                    .send(value)
                    .expect("request receipt should remain active");
                Ok(response)
            },
        )
        .await
        .expect("valid websocket handshake");
        let command = tokio::time::timeout(Duration::from_secs(10), websocket.next())
            .await
            .expect("CDP command should arrive")
            .expect("connection should remain open")
            .expect("CDP command should be valid websocket data");
        assert!(command.is_text(), "Browser.getVersion must be text");
        websocket
            .send(Message::Close(Some(CloseFrame {
                code: CloseCode::from(4123),
                reason: "Oh my!".into(),
            })))
            .await
            .expect("peer close frame should write");
    });
    let endpoint = format!("ws://{client_host}:{port}/ws");

    // Act
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        HostSession::connect_over_cdp_with_headers(
            &endpoint,
            5_000,
            vec![("foo".into(), "bar".into())],
        ),
    )
    .await
    .expect("public attach should settle");
    let header = header_rx.await.expect("server should capture request");
    server.await.expect("server task should retire");

    // Assert
    let error = match result {
        Ok(_) => panic!("a peer that closes before Browser.getVersion must not attach"),
        Err(error) => error,
    };
    assert_eq!(header, Some(b"bar".to_vec()));
    assert!(error.to_string().contains("Browser logs:\n\nOh my!\n"));
}

#[tokio::test]
async fn cdp_attach_without_headers_reports_remote_close_reason() {
    // Arrange
    let Some((bind_ip, client_host)) = tailscale_fixture_address() else {
        return;
    };
    let listener = TcpListener::bind(SocketAddr::from((bind_ip, 0)))
        .await
        .expect("test listener should bind");
    let port = listener.local_addr().expect("bound address").port();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("client should connect");
        let mut websocket = accept_async(stream)
            .await
            .expect("valid websocket handshake");
        let command = tokio::time::timeout(Duration::from_secs(10), websocket.next())
            .await
            .expect("CDP command should arrive")
            .expect("connection should remain open")
            .expect("CDP command should be valid websocket data");
        assert!(command.is_text(), "Browser.getVersion must be text");
        websocket
            .send(Message::Close(Some(CloseFrame {
                code: CloseCode::from(4123),
                reason: "Oh my!".into(),
            })))
            .await
            .expect("peer close frame should write");
    });
    let endpoint = format!("ws://{client_host}:{port}/ws");

    // Act
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        HostSession::connect_over_cdp(&endpoint, 5_000),
    )
    .await
    .expect("public attach should settle");
    server.await.expect("server task should retire");

    // Assert
    let error = match result {
        Ok(_) => panic!("a closed peer must not attach"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("Browser logs:\n\nOh my!\n"));
}
