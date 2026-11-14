use super::*;
use std::net::{Ipv4Addr, SocketAddr};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_hdr_async;

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
            // Portable CI without Tailscale: keep this ephemeral test-only
            // listener on loopback rather than exposing a wildcard port.
            (Ipv4Addr::LOCALHOST, Ipv4Addr::LOCALHOST.to_string())
        }
        _ => panic!("TAILSCALE_IP and TAILSCALE_HOST must be set together"),
    }
}

#[test]
fn invalid_header_value_is_rejected_without_leaking_it() {
    // Arrange
    let secret = "private-value\r\nInjected: yes";

    // Act
    let error = CdpHandshakeHeaders::new(vec![("Authorization".into(), secret.into())])
        .expect_err("CRLF must not enter an HTTP handshake");

    // Assert
    assert_eq!(error, CdpHandshakeHeadersError::InvalidValue);
    assert!(!error.to_string().contains("private-value"));
}

#[test]
fn invalid_header_name_is_rejected_and_debug_redacts_valid_values() {
    // Arrange
    let invalid = vec![("bad name".into(), "private-value".into())];
    let valid = vec![("Authorization".into(), "private-value".into())];

    // Act
    let error = CdpHandshakeHeaders::new(invalid).expect_err("spaces are invalid in names");
    let headers = CdpHandshakeHeaders::new(valid).expect("valid authorization header");

    // Assert
    assert_eq!(error, CdpHandshakeHeadersError::InvalidName);
    assert!(!error.to_string().contains("private-value"));
    assert!(!format!("{headers:?}").contains("private-value"));
}

#[test]
fn header_count_and_bytes_are_bounded_before_network_io() {
    // Arrange
    let too_many = vec![("X-Pwright".into(), "v".into()); CDP_HANDSHAKE_HEADER_MAX_COUNT + 1];
    let too_large = vec![(
        "X-Pwright".into(),
        "v".repeat(CDP_HANDSHAKE_HEADER_MAX_VALUE_BYTES + 1),
    )];

    // Act
    let count_error = CdpHandshakeHeaders::new(too_many).expect_err("count must be bounded");
    let bytes_error = CdpHandshakeHeaders::new(too_large).expect_err("value must be bounded");

    // Assert
    assert_eq!(count_error, CdpHandshakeHeadersError::TooMany);
    assert_eq!(bytes_error, CdpHandshakeHeadersError::ValueTooLong);
}

#[tokio::test]
async fn upgrade_sends_exact_caller_header_to_fake_websocket() {
    // Arrange
    let (bind_ip, client_host) = one_shot_fixture_address();
    let listener = TcpListener::bind(SocketAddr::from((bind_ip, 0)))
        .await
        .expect("test listener should bind");
    let port = listener.local_addr().expect("bound address").port();
    let (receipt_tx, receipt_rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("client should connect");
        let mut receipt_tx = Some(receipt_tx);
        let websocket = accept_hdr_async(
            stream,
            move |request: &tokio_tungstenite::tungstenite::handshake::server::Request,
                  response| {
                let receipt = request
                    .headers()
                    .get("foo")
                    .map(|value| value.as_bytes().to_vec());
                receipt_tx
                    .take()
                    .expect("one handshake")
                    .send(receipt)
                    .expect("receiver should remain active");
                Ok(response)
            },
        )
        .await
        .expect("valid websocket handshake");
        drop(websocket);
    });
    let headers =
        CdpHandshakeHeaders::new(vec![("foo".into(), "bar".into())]).expect("valid test header");
    let endpoint = format!("ws://{client_host}:{port}/ws");

    // Act
    let connection = tokio::time::timeout(
        Duration::from_secs(10),
        CdpConnection::connect_with_config_and_headers(
            &endpoint,
            ConnectionConfig::default(),
            &headers,
        ),
    )
    .await
    .expect("handshake should not hang")
    .expect("handshake should succeed");
    let receipt = tokio::time::timeout(Duration::from_secs(10), receipt_rx)
        .await
        .expect("server request receipt should not hang")
        .expect("server should report the request");
    connection.request_disconnect();
    tokio::time::timeout(Duration::from_secs(2), connection.io_finished())
        .await
        .expect("connection tasks should retire");
    server.await.expect("server task should retire");

    // Assert
    assert_eq!(receipt, Some(b"bar".to_vec()));
}
