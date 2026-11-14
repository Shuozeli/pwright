#![cfg(feature = "live")]

use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use pwright_playwright_server_client_core::{Ingress, ReplyOutcome};
use pwright_playwright_server_connect::{ConnectOptions, connect_live};
use pwright_playwright_server_endpoint_discovery::DiscoveryLimits;
use pwright_playwright_server_session::SdkLanguage;
use pwright_transport_ws::WsLimits;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, lookup_host};
use tokio::time::timeout;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{
    Callback, ErrorResponse, Request, Response,
};

const IO_DEADLINE: Duration = Duration::from_secs(5);

struct CaptureUpgrade {
    observed: Arc<Mutex<Option<(String, String)>>>,
}

impl Callback for CaptureUpgrade {
    fn on_request(self, request: &Request, response: Response) -> Result<Response, ErrorResponse> {
        let token = request
            .headers()
            .get("X-Test-Token")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        *self.observed.lock().unwrap() = Some((request.uri().to_string(), token));
        Ok(response)
    }
}

async fn tailscale_target() -> (Ipv4Addr, String) {
    let ip: Ipv4Addr = std::env::var("TAILSCALE_IP")
        .expect("TAILSCALE_IP must be this Mac's Tailscale IPv4 address")
        .parse()
        .expect("TAILSCALE_IP must be IPv4");
    let octets = ip.octets();
    assert!(octets[0] == 100 && (64..=127).contains(&octets[1]));
    let host =
        std::env::var("TAILSCALE_HOST").expect("TAILSCALE_HOST must be the full MagicDNS hostname");
    assert!(host.ends_with(".ts.net") && !host.contains(['/', ':', '@']));
    let resolved = lookup_host((host.as_str(), 80)).await.unwrap();
    assert!(resolved.into_iter().any(|addr| addr.ip() == IpAddr::V4(ip)));
    (ip, host)
}

async fn read_http_request(stream: &mut TcpStream) -> String {
    let mut data = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        let read = timeout(IO_DEADLINE, stream.read(&mut chunk))
            .await
            .expect("HTTP request deadline")
            .expect("HTTP request read");
        assert!(read > 0 && data.len() + read <= 8192);
        data.extend_from_slice(&chunk[..read]);
        if data.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
            return String::from_utf8(data).expect("ASCII HTTP request");
        }
    }
}

fn bootstrap_frames() -> Vec<Value> {
    vec![
        json!({"guid":"","method":"__create__","params":{"type":"BrowserType","guid":"chromium@1","initializer":{"name":"chromium","executablePath":"/browser"}}}),
        json!({"guid":"","method":"__create__","params":{"type":"BrowserType","guid":"firefox@1","initializer":{"name":"firefox","executablePath":"/browser"}}}),
        json!({"guid":"","method":"__create__","params":{"type":"BrowserType","guid":"webkit@1","initializer":{"name":"webkit","executablePath":"/browser"}}}),
        json!({"guid":"","method":"__create__","params":{"type":"Android","guid":"android@1","initializer":{}}}),
        json!({"guid":"","method":"__create__","params":{"type":"Electron","guid":"electron@1","initializer":{}}}),
        json!({"guid":"chromium@1","method":"__create__","params":{"type":"Browser","guid":"browser@1","initializer":{"name":"chromium","version":"155.0"}}}),
        json!({"guid":"","method":"__create__","params":{"type":"Playwright","guid":"playwright@1","initializer":{
            "chromium":{"guid":"chromium@1"},
            "firefox":{"guid":"firefox@1"},
            "webkit":{"guid":"webkit@1"},
            "android":{"guid":"android@1"},
            "electron":{"guid":"electron@1"},
            "preLaunchedBrowser":{"guid":"browser@1"}
        }}}),
        json!({"id":1,"result":{"playwright":{"guid":"playwright@1"}}}),
    ]
}

#[tokio::test]
#[ignore = "requires TAILSCALE_IP and TAILSCALE_HOST for a Tailscale-bound synthetic HTTP/WebSocket fixture"]
async fn live_http_discovery_websocket_bootstrap_command_and_close_are_composed() {
    // Arrange
    let (ip, host) = tailscale_target().await;
    let listener = TcpListener::bind((ip, 0))
        .await
        .expect("Tailscale-only bind");
    let port = listener.local_addr().unwrap().port();
    let observed_ws = Arc::new(Mutex::new(None::<(String, String)>));
    let server = tokio::spawn({
        let observed_ws = Arc::clone(&observed_ws);
        async move {
            let (mut http, peer) = timeout(IO_DEADLINE, listener.accept())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(peer.ip(), IpAddr::V4(ip));
            let discovery_request = read_http_request(&mut http).await;
            let body = br#"{"wsEndpointPath":"/browser-123"}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            http.write_all(response.as_bytes()).await.unwrap();
            http.write_all(body).await.unwrap();
            http.shutdown().await.unwrap();

            let (ws_stream, peer) = timeout(IO_DEADLINE, listener.accept())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(peer.ip(), IpAddr::V4(ip));
            let mut socket = accept_hdr_async(
                ws_stream,
                CaptureUpgrade {
                    observed: observed_ws,
                },
            )
            .await
            .unwrap();
            let Message::Text(initialize) = timeout(IO_DEADLINE, socket.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap()
            else {
                panic!("expected Root.initialize text frame");
            };
            for frame in bootstrap_frames() {
                timeout(
                    IO_DEADLINE,
                    socket.send(Message::Text(frame.to_string().into())),
                )
                .await
                .unwrap()
                .unwrap();
            }
            let Message::Text(command) = timeout(IO_DEADLINE, socket.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap()
            else {
                panic!("expected synthetic command text frame");
            };
            timeout(
                IO_DEADLINE,
                socket.send(Message::Text(
                    json!({"id":2,"result":{"echo":7}}).to_string().into(),
                )),
            )
            .await
            .unwrap()
            .unwrap();
            let close = timeout(IO_DEADLINE, socket.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert!(matches!(close, Message::Close(_)));
            timeout(IO_DEADLINE, SinkExt::close(&mut socket))
                .await
                .unwrap()
                .unwrap();
            (
                discovery_request,
                initialize.to_string(),
                command.to_string(),
            )
        }
    });
    let mut input = ConnectOptions::new(
        format!("http://{host}:{port}/prefix?token=synthetic"),
        "Pwright-Discovery-Test/1",
        DiscoveryLimits::new(1024, 5000).unwrap(),
        WsLimits::new(256 * 1024, 64 * 1024, 5000).unwrap(),
        SdkLanguage::Javascript,
        6789,
    );
    input
        .add_websocket_header("X-Test-Token", "synthetic")
        .unwrap();

    // Act
    let browser = timeout(IO_DEADLINE, connect_live(&input))
        .await
        .expect("connect composition deadline")
        .expect("validated remote Browser");
    assert!(browser.is_connected());
    for _ in 0..7 {
        assert!(matches!(
            browser.next_event().await.unwrap(),
            Ingress::Created { .. }
        ));
    }
    let browser = Arc::new(browser);
    let reply = timeout(
        IO_DEADLINE,
        browser.call("browser@1", "syntheticEcho", json!({"value":7}), json!({})),
    )
    .await
    .unwrap()
    .unwrap();
    let close = timeout(IO_DEADLINE, browser.close())
        .await
        .unwrap()
        .unwrap();
    let (discovery_request, initialize, command) =
        timeout(IO_DEADLINE, server).await.unwrap().unwrap();

    // Assert
    assert_eq!(browser.guid(), "browser@1");
    assert_eq!(reply, ReplyOutcome::Result(json!({"echo":7})));
    assert!(close.report.failed.is_empty());
    assert!(close.transport_error.is_none());
    assert!(!browser.is_connected());
    let lower = discovery_request.to_ascii_lowercase();
    assert!(lower.starts_with("get /prefix/json?token=synthetic http/1.1\r\n"));
    assert!(lower.contains("\r\nuser-agent: pwright-discovery-test/1\r\n"));
    assert!(!lower.contains("x-test-token:"));
    assert_eq!(
        observed_ws.lock().unwrap().as_ref(),
        Some(&(
            "/prefix/browser-123?token=synthetic".to_owned(),
            "synthetic".into()
        ))
    );
    assert_eq!(
        serde_json::from_str::<Value>(&initialize).unwrap(),
        json!({"id":1,"guid":"","method":"initialize","params":{"sdkLanguage":"javascript"},"metadata":{"wallTime":6789,"apiName":"Connection.init","internal":false}})
    );
    assert_eq!(
        serde_json::from_str::<Value>(&command).unwrap(),
        json!({"id":2,"guid":"browser@1","method":"syntheticEcho","params":{"value":7},"metadata":{}})
    );
}
