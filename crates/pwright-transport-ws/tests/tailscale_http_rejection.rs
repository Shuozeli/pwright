#![cfg(feature = "live-tokio")]

use std::net::{IpAddr, ToSocketAddrs};

use pwright_transport_ws::{LiveWsConnector, WsEndpoint, WsError, WsLimits, WsRequest};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
#[ignore = "requires explicit TAILSCALE_IP and full TAILSCALE_HOST"]
async fn live_connector_preserves_http_rejection_over_tailscale_fixture() {
    // Arrange
    let bind: IpAddr = std::env::var("TAILSCALE_IP")
        .expect("explicit TAILSCALE_IP")
        .parse()
        .expect("Tailscale IP address");
    assert!(bind.is_ipv4());
    let host = std::env::var("TAILSCALE_HOST").expect("explicit TAILSCALE_HOST");
    assert!(host.ends_with(".ts.net"));
    let resolved: Vec<_> = (host.as_str(), 0)
        .to_socket_addrs()
        .expect("MagicDNS lookup")
        .map(|address| address.ip())
        .collect();
    assert!(
        resolved.contains(&bind),
        "MagicDNS host must resolve to bind IP"
    );

    let listener = TcpListener::bind((bind, 0)).await.expect("Tailscale bind");
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut chunk = [0_u8; 1024];
        while !request.ends_with(b"\r\n\r\n") {
            let count = socket.read(&mut chunk).await.unwrap();
            assert_ne!(count, 0);
            request.extend_from_slice(&chunk[..count]);
        }
        assert!(request.starts_with(b"GET /ws-401 HTTP/1.1\r\n"));
        socket
            .write_all(b"HTTP/1.1 401 Unauthorized\r\n\r\nUnauthorized body")
            .await
            .unwrap();
    });
    let endpoint = WsEndpoint::parse(format!("ws://{host}:{port}/ws-401")).unwrap();
    let limits = WsLimits::new(4096, 4096, 5000).unwrap();
    let request = WsRequest::new(endpoint, limits);

    // Act
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        LiveWsConnector.connect_with_dns_classification(&request),
    )
    .await
    .expect("live connector deadline");
    server.await.expect("server completed");

    // Assert
    assert_eq!(
        result.err(),
        Some(WsError::HttpRejected {
            status: 401,
            body: "Unauthorized body".into(),
        })
    );
}
