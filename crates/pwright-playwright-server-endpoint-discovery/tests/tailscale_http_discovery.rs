#![cfg(feature = "live-reqwest")]

use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

use pwright_playwright_server_endpoint_discovery::{
    DiscoveryError, DiscoveryLimits, MAX_DISCOVERY_REDIRECTS, ReqwestDiscoveryFetcher,
    resolve_endpoint,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, lookup_host};
use tokio::time::timeout;

async fn tailscale_target() -> (Ipv4Addr, String) {
    let ip: Ipv4Addr = std::env::var("TAILSCALE_IP")
        .expect("TAILSCALE_IP is required")
        .parse()
        .expect("TAILSCALE_IP must be an IPv4 address");
    let octets = ip.octets();
    assert!(octets[0] == 100 && (64..=127).contains(&octets[1]));
    let host = std::env::var("TAILSCALE_HOST").expect("TAILSCALE_HOST is required");
    assert!(host.ends_with(".ts.net") && !host.contains(['/', ':', '@']));
    let resolved = lookup_host((host.as_str(), 80))
        .await
        .expect("MagicDNS hostname must resolve");
    assert!(
        resolved
            .into_iter()
            .any(|address| address.ip() == IpAddr::V4(ip))
    );
    (ip, host)
}

async fn read_request(stream: &mut TcpStream) -> String {
    let mut request = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        let read = timeout(Duration::from_secs(5), stream.read(&mut chunk))
            .await
            .expect("fixture request deadline")
            .expect("fixture request read");
        assert!(read > 0 && request.len() + read <= 8192);
        request.extend_from_slice(&chunk[..read]);
        if request.windows(4).any(|part| part == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8(request).expect("ASCII fixture request")
}

#[tokio::test]
#[ignore = "requires TAILSCALE_IP and full TAILSCALE_HOST on the test host"]
async fn live_http_discovery_uses_only_user_agent_and_preserves_the_base_query() {
    // Arrange
    let (ip, host) = tailscale_target().await;
    let listener = TcpListener::bind((ip, 0)).await.expect("Tailscale bind");
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        let (mut stream, _) = timeout(Duration::from_secs(5), listener.accept())
            .await
            .expect("fixture accept deadline")
            .expect("fixture accept");
        let request = read_request(&mut stream).await;
        let body = br#"{"wsEndpointPath":"/browser-123"}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();
        stream.write_all(body).await.unwrap();
        request
    });
    let fetcher = ReqwestDiscoveryFetcher::new().unwrap();
    let limits = DiscoveryLimits::new(1024, 5000).unwrap();
    let url = format!("http://{host}:{port}/prefix?token=synthetic");

    // Act
    let endpoint = resolve_endpoint(&url, "Pwright-Test-UA", limits, &fetcher)
        .await
        .unwrap();
    let request = server.await.unwrap();

    // Assert
    assert_eq!(
        endpoint.as_str(),
        format!("ws://{host}:{port}/prefix/browser-123?token=synthetic")
    );
    let lower = request.to_ascii_lowercase();
    assert!(lower.starts_with("get /prefix/json?token=synthetic http/1.1\r\n"));
    assert!(lower.contains("\r\nuser-agent: pwright-test-ua\r\n"));
    assert!(!lower.contains("x-playwright-browser:"));
    assert!(!lower.contains("x-playwright-proxy:"));
}

#[tokio::test]
#[ignore = "requires TAILSCALE_IP and full TAILSCALE_HOST on the test host"]
async fn live_http_304_redirect_follows_location_without_changing_websocket_base() {
    // Arrange
    let (ip, host) = tailscale_target().await;
    let listener = TcpListener::bind((ip, 0)).await.expect("Tailscale bind");
    let port = listener.local_addr().unwrap().port();
    let redirect_listener = TcpListener::bind((ip, 0))
        .await
        .expect("redirect Tailscale bind");
    let redirect_port = redirect_listener.local_addr().unwrap().port();
    let redirect_location = format!("http://{host}:{redirect_port}/moved/json?other=1");
    let server = tokio::spawn(async move {
        let (mut first, _) = timeout(Duration::from_secs(5), listener.accept())
            .await
            .expect("first accept deadline")
            .expect("first accept");
        let first_request = read_request(&mut first).await;
        let redirect_response = format!(
            "HTTP/1.1 304 Not Modified\r\nLocation: {redirect_location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        first.write_all(redirect_response.as_bytes()).await.unwrap();
        first.shutdown().await.unwrap();

        let (mut second, _) = timeout(Duration::from_secs(5), redirect_listener.accept())
            .await
            .expect("redirect accept deadline")
            .expect("redirect accept");
        let second_request = read_request(&mut second).await;
        let body = br#"{"wsEndpointPath":"/browser-redirected"}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        second.write_all(response.as_bytes()).await.unwrap();
        second.write_all(body).await.unwrap();
        (first_request, second_request)
    });
    let fetcher = ReqwestDiscoveryFetcher::new().unwrap();
    let limits = DiscoveryLimits::new(1024, 5000).unwrap();
    let url = format!("http://{host}:{port}/prefix?token=synthetic");

    // Act
    let endpoint = resolve_endpoint(&url, "Pwright-Test-UA", limits, &fetcher)
        .await
        .unwrap();
    let (first_request, second_request) = server.await.unwrap();

    // Assert
    assert_eq!(
        endpoint.as_str(),
        format!("ws://{host}:{port}/prefix/browser-redirected?token=synthetic")
    );
    assert!(first_request.starts_with("GET /prefix/json?token=synthetic HTTP/1.1\r\n"));
    assert!(second_request.starts_with("GET /moved/json?other=1 HTTP/1.1\r\n"));
    assert!(!second_request.to_ascii_lowercase().contains("\r\nreferer:"));
    assert!(!second_request.contains("token=synthetic"));
    assert!(
        second_request
            .to_ascii_lowercase()
            .contains("\r\nuser-agent: pwright-test-ua\r\n")
    );
}

#[tokio::test]
#[ignore = "requires TAILSCALE_IP and full TAILSCALE_HOST on the test host"]
async fn live_http_redirect_chain_stops_at_the_configured_limit() {
    // Arrange
    let (ip, host) = tailscale_target().await;
    let listener = TcpListener::bind((ip, 0)).await.expect("Tailscale bind");
    let port = listener.local_addr().unwrap().port();
    let server = tokio::spawn(async move {
        let mut received = 0;
        for hop in 0..=MAX_DISCOVERY_REDIRECTS {
            let Ok(Ok((mut stream, _))) =
                timeout(Duration::from_millis(500), listener.accept()).await
            else {
                break;
            };
            let _request = read_request(&mut stream).await;
            received += 1;
            let response = format!(
                "HTTP/1.1 302 Found\r\nLocation: /hop{}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                hop + 1
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();
        }
        received
    });
    let fetcher = ReqwestDiscoveryFetcher::new().unwrap();
    let limits = DiscoveryLimits::new(1024, 5000).unwrap();
    let url = format!("http://{host}:{port}/prefix");

    // Act
    let result = resolve_endpoint(&url, "Pwright-Test-UA", limits, &fetcher).await;
    let received = server.await.unwrap();

    // Assert
    assert_eq!(result, Err(DiscoveryError::HttpStatus(302)));
    assert_eq!(received, MAX_DISCOVERY_REDIRECTS + 1);
}
