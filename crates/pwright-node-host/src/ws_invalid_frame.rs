use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use futures_util::SinkExt;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::Message;

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
            // Portable CI: a one-shot loopback fixture, never a wildcard or
            // long-running listener.
            (Ipv4Addr::LOCALHOST, Ipv4Addr::LOCALHOST.to_string())
        }
        _ => panic!("TAILSCALE_IP and TAILSCALE_HOST must be set together"),
    }
}

#[tokio::test]
async fn malformed_text_fails_cdp_attach_promptly_and_preserves_upgrade_header() {
    // Arrange
    let (bind_ip, client_host) = one_shot_fixture_address();
    let listener = TcpListener::bind(SocketAddr::from((bind_ip, 0)))
        .await
        .expect("test listener should bind");
    let port = listener.local_addr().expect("bound address").port();
    let (header_tx, header_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
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
        // Match the official Python fixture: send the default TEXT payload
        // immediately after upgrade, without closing or replying to CDP.
        websocket
            .send(Message::Text("incoming".into()))
            .await
            .expect("malformed text frame should write");
        tokio::time::timeout(Duration::from_secs(10), release_rx)
            .await
            .expect("test should release the one-shot server")
            .expect("release sender should remain active");
    });
    let endpoint = format!("ws://{client_host}:{port}/ws");

    // Act
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        HostSession::connect_over_cdp_with_headers(
            &endpoint,
            5_000,
            vec![("foo".into(), "bar".into())],
        ),
    )
    .await
    .expect("invalid CDP frame must fail before the public command timeout");
    let header = header_rx.await.expect("server should capture request");
    release_tx.send(()).expect("server should remain active");
    server.await.expect("server task should retire");

    // Assert
    let error = match result {
        Ok(_) => panic!("binary data is not a valid CDP response"),
        Err(error) => error,
    };
    assert_eq!(header, Some(b"bar".to_vec()));
    assert!(error.to_string().contains("Browser version query failed"));
    assert!(!error.to_string().contains("incoming"));
}
