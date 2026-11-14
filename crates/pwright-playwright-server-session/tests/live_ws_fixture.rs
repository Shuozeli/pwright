#![cfg(feature = "live-tokio")]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

#[cfg(feature = "supervisor")]
use futures_util::FutureExt;
use futures_util::{SinkExt, StreamExt};
use pwright_playwright_server_client_core::Ingress;
#[cfg(feature = "supervisor")]
use pwright_playwright_server_client_core::ReplyOutcome;
#[cfg(feature = "supervisor")]
use pwright_playwright_server_session::RemoteBrowserSupervisor;
use pwright_playwright_server_session::{LiveWsConnector, RemoteBrowserLease, SdkLanguage};
use pwright_transport_ws::{WsEndpoint, WsLimits, WsRequest};
use serde_json::{Value, json};
use tokio::net::{TcpListener, lookup_host};
#[cfg(feature = "supervisor")]
use tokio::sync::oneshot;
use tokio::time::timeout;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

const IO_DEADLINE: Duration = Duration::from_secs(5);

fn tailscale_fixture_address() -> (Ipv4Addr, String) {
    let ip: Ipv4Addr = std::env::var("TAILSCALE_IP")
        .expect("TAILSCALE_IP must be set to this Mac's Tailscale IPv4 address")
        .parse()
        .expect("TAILSCALE_IP must be an IPv4 address");
    let octets = ip.octets();
    assert!(
        octets[0] == 100 && (64..=127).contains(&octets[1]),
        "TAILSCALE_IP must be a Tailscale IPv4 address in 100.64.0.0/10"
    );
    let host = std::env::var("TAILSCALE_HOST")
        .expect("TAILSCALE_HOST must be set to this Mac's full MagicDNS hostname");
    assert!(
        host.ends_with(".ts.net") && !host.contains(['/', ':', '@']),
        "TAILSCALE_HOST must be a full MagicDNS hostname without a scheme or port"
    );
    (ip, host)
}

fn bootstrap_frames() -> Vec<Value> {
    vec![
        json!({"guid":"","method":"__create__","params":{"type":"BrowserType","guid":"chromium@1","initializer":{"name":"chromium","executablePath":"/browser"}}}),
        json!({"guid":"","method":"__create__","params":{"type":"BrowserType","guid":"firefox@1","initializer":{"name":"firefox","executablePath":"/browser"}}}),
        json!({"guid":"","method":"__create__","params":{"type":"BrowserType","guid":"webkit@1","initializer":{"name":"webkit","executablePath":"/browser"}}}),
        json!({"guid":"","method":"__create__","params":{"type":"Android","guid":"android@1","initializer":{}}}),
        json!({"guid":"","method":"__create__","params":{"type":"Electron","guid":"electron@1","initializer":{}}}),
        json!({"guid":"chromium@1","method":"__create__","params":{"type":"Browser","guid":"browser@1","initializer":{"name":"chromium","version":"155.0"}}}),
        json!({"guid":"browser@1","method":"__create__","params":{"type":"BrowserContext","guid":"context@1","initializer":{}}}),
        json!({"guid":"browser@1","method":"context","params":{"context":{"guid":"context@1"}}}),
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
#[ignore = "requires explicit TAILSCALE_IP and TAILSCALE_HOST for a Tailscale-bound live WebSocket fixture"]
async fn real_websocket_bootstrap_replays_pre_reply_context_event_and_closes_cleanly() {
    // Arrange
    let (ip, host) = tailscale_fixture_address();
    let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(ip), 0))
        .await
        .expect("fixture must bind only the verified Tailscale interface");
    let port = listener.local_addr().unwrap().port();
    let resolved = lookup_host((host.as_str(), port))
        .await
        .expect("MagicDNS hostname must resolve");
    assert!(
        resolved.into_iter().any(|address| address.ip() == ip),
        "MagicDNS hostname must resolve to the bound Tailscale IPv4 address"
    );
    let endpoint = WsEndpoint::parse(format!("ws://{host}:{port}/playwright-session"))
        .expect("fixture URL must be a valid WebSocket endpoint");
    let limits = WsLimits::new(256 * 1024, 64 * 1024, 5_000).unwrap();
    let request = WsRequest::new(endpoint, limits);
    let server = tokio::spawn(async move {
        let (stream, peer) = timeout(IO_DEADLINE, listener.accept())
            .await
            .expect("client must connect before the deadline")
            .expect("fixture must accept the connection");
        assert_eq!(peer.ip(), IpAddr::V4(ip));
        let mut socket = accept_async(stream)
            .await
            .expect("fixture must accept a real WebSocket upgrade");
        let first = timeout(IO_DEADLINE, socket.next())
            .await
            .expect("Root.initialize must arrive before the deadline")
            .expect("Root.initialize must not end the stream")
            .expect("Root.initialize must be a valid WebSocket frame");
        let Message::Text(command) = first else {
            panic!("Root.initialize must be a text frame");
        };
        assert_eq!(
            serde_json::from_str::<Value>(&command).unwrap(),
            json!({"id":1,"guid":"","method":"initialize","params":{"sdkLanguage":"python"},"metadata":{"wallTime":1234,"apiName":"Connection.init","internal":false}})
        );
        for frame in bootstrap_frames() {
            timeout(
                IO_DEADLINE,
                socket.send(Message::Text(frame.to_string().into())),
            )
            .await
            .expect("bootstrap frame send must finish before the deadline")
            .expect("bootstrap frame must be sent");
        }
        let close = timeout(IO_DEADLINE, socket.next())
            .await
            .expect("local Browser.close must reach the server before the deadline")
            .expect("local Browser.close must not end the stream without a Close frame")
            .expect("local Browser.close must send a valid WebSocket frame");
        assert!(matches!(close, Message::Close(_)));
        timeout(IO_DEADLINE, SinkExt::close(&mut socket))
            .await
            .expect("Close acknowledgement must flush before the deadline")
            .expect("Close acknowledgement must flush");
    });

    // Act
    let mut browser = timeout(
        IO_DEADLINE,
        RemoteBrowserLease::connect(&LiveWsConnector, &request, SdkLanguage::Python, 1234),
    )
    .await
    .expect("Root.initialize must finish before the deadline")
    .expect("Root.initialize must validate its GUID graph");
    let mut replayed = Vec::new();
    while browser.bootstrap_pending_count() > 0 {
        replayed.push(browser.receive_next().await.unwrap());
    }
    let close = browser.close().await;
    timeout(IO_DEADLINE, server)
        .await
        .expect("fixture server must complete before the deadline")
        .expect("fixture server must not panic");

    // Assert
    assert_eq!(browser.guid(), "browser@1");
    assert_eq!(browser.name(), "chromium");
    assert_eq!(browser.version(), "155.0");
    assert_eq!(replayed.len(), 9);
    assert!(matches!(&replayed[0], Ingress::Created { guid, .. } if guid == "chromium@1"));
    assert!(
        matches!(&replayed[5], Ingress::Created { guid, object_type, .. } if guid == "browser@1" && object_type == "Browser")
    );
    assert!(
        matches!(&replayed[6], Ingress::Created { parent, guid, object_type, .. } if parent == "browser@1" && guid == "context@1" && object_type == "BrowserContext")
    );
    assert!(
        matches!(&replayed[7], Ingress::Event { guid, method, params } if guid == "browser@1" && method == "context" && params == &json!({"context":{"guid":"context@1"}}))
    );
    assert!(
        matches!(&replayed[8], Ingress::Created { guid, object_type, .. } if guid == "playwright@1" && object_type == "Playwright")
    );
    assert!(!browser.is_connected());
    assert_eq!(browser.pending_count(), 0);
    assert!(close.report.failed.is_empty());
    assert!(close.transport_error.is_none());
}

#[cfg(feature = "supervisor")]
#[tokio::test]
#[ignore = "requires explicit TAILSCALE_IP and TAILSCALE_HOST for a synthetic Tailscale-bound live WebSocket fixture"]
async fn real_websocket_supervisor_replays_bootstrap_and_correlates_command_before_close() {
    // Arrange
    let (ip, host) = tailscale_fixture_address();
    let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(ip), 0))
        .await
        .expect("fixture must bind only the verified Tailscale interface");
    let port = listener.local_addr().unwrap().port();
    let resolved = lookup_host((host.as_str(), port))
        .await
        .expect("MagicDNS hostname must resolve");
    assert!(
        resolved.into_iter().any(|address| address.ip() == ip),
        "MagicDNS hostname must resolve to the bound Tailscale IPv4 address"
    );
    let endpoint = WsEndpoint::parse(format!("ws://{host}:{port}/synthetic-supervisor"))
        .expect("fixture URL must be a valid WebSocket endpoint");
    let request = WsRequest::new(
        endpoint,
        WsLimits::new(256 * 1024, 64 * 1024, 5_000).unwrap(),
    );
    let (bootstrap_observed, bootstrap_gate) = oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        let (stream, peer) = timeout(IO_DEADLINE, listener.accept())
            .await
            .expect("client must connect before the deadline")
            .expect("fixture must accept the connection");
        assert_eq!(peer.ip(), IpAddr::V4(ip));
        let mut socket = accept_async(stream)
            .await
            .expect("fixture must accept a real WebSocket upgrade");
        let first = timeout(IO_DEADLINE, socket.next())
            .await
            .expect("Root.initialize must arrive before the deadline")
            .expect("Root.initialize must not end the stream")
            .expect("Root.initialize must be a valid WebSocket frame");
        let Message::Text(command) = first else {
            panic!("Root.initialize must be a text frame");
        };
        assert_eq!(
            serde_json::from_str::<Value>(&command).unwrap(),
            json!({"id":1,"guid":"","method":"initialize","params":{"sdkLanguage":"python"},"metadata":{"wallTime":2345,"apiName":"Connection.init","internal":false}})
        );
        for frame in bootstrap_frames() {
            timeout(
                IO_DEADLINE,
                socket.send(Message::Text(frame.to_string().into())),
            )
            .await
            .expect("bootstrap frame send must finish before the deadline")
            .expect("bootstrap frame must be sent");
        }

        // The server sends no further frames until the client has observed
        // every pre-reply event from the supervisor's startup queue.
        timeout(IO_DEADLINE, bootstrap_gate)
            .await
            .expect("bootstrap replay must be observed before the deadline")
            .expect("client must acknowledge bootstrap replay");
        let next = timeout(IO_DEADLINE, socket.next())
            .await
            .expect("command must arrive while no inbound frame is available")
            .expect("command must not end the stream")
            .expect("command must be a valid WebSocket frame");
        let Message::Text(command) = next else {
            panic!("supervised command must be a text frame");
        };
        assert_eq!(
            serde_json::from_str::<Value>(&command).unwrap(),
            json!({"id":2,"guid":"browser@1","method":"syntheticEcho","params":{"value":7},"metadata":{}})
        );
        timeout(
            IO_DEADLINE,
            socket.send(Message::Text(
                json!({"id":2,"result":{"echo":7}}).to_string().into(),
            )),
        )
        .await
        .expect("synthetic reply must send before the deadline")
        .expect("synthetic reply must send");
        timeout(
            IO_DEADLINE,
            socket.send(Message::Text(
                json!({"guid":"browser@1","method":"syntheticNotice","params":{"order":1}})
                    .to_string()
                    .into(),
            )),
        )
        .await
        .expect("later event must send before the deadline")
        .expect("later event must send");
        let close = timeout(IO_DEADLINE, socket.next())
            .await
            .expect("local close must reach the server before the deadline")
            .expect("local close must not end the stream without a Close frame")
            .expect("local close must send a valid WebSocket frame");
        assert!(matches!(close, Message::Close(_)));
        timeout(IO_DEADLINE, SinkExt::close(&mut socket))
            .await
            .expect("Close acknowledgement must flush before the deadline")
            .expect("Close acknowledgement must flush");
    });

    // Act
    let lease = timeout(
        IO_DEADLINE,
        RemoteBrowserLease::connect(&LiveWsConnector, &request, SdkLanguage::Python, 2345),
    )
    .await
    .expect("Root.initialize must finish before the deadline")
    .expect("Root.initialize must validate its GUID graph");
    let browser = timeout(IO_DEADLINE, RemoteBrowserSupervisor::start(lease))
        .await
        .expect("supervisor startup must finish before the deadline")
        .expect("supervisor must queue all bootstrap ingress");
    let mut replayed = Vec::new();
    for _ in 0..9 {
        replayed.push(
            browser
                .next_event()
                .now_or_never()
                .expect("bootstrap ingress must already be queued when start returns")
                .expect("bootstrap ingress must be valid"),
        );
    }
    bootstrap_observed
        .send(())
        .expect("fixture server must still be waiting for bootstrap replay");
    tokio::task::yield_now().await;
    let reply = timeout(
        IO_DEADLINE,
        browser.call("browser@1", "syntheticEcho", json!({"value":7}), json!({})),
    )
    .await
    .expect("command must complete while the actor awaits receive")
    .expect("synthetic command must receive its correlated reply");
    let later = timeout(IO_DEADLINE, browser.next_event())
        .await
        .expect("later event must be delivered before the deadline")
        .expect("later event must be valid");
    let close = timeout(IO_DEADLINE, browser.close())
        .await
        .expect("local close must finish before the deadline")
        .expect("local close must succeed");
    timeout(IO_DEADLINE, server)
        .await
        .expect("fixture server must complete before the deadline")
        .expect("fixture server must not panic");

    // Assert
    assert_eq!(browser.guid(), "browser@1");
    assert_eq!(browser.name(), "chromium");
    assert_eq!(browser.version(), "155.0");
    assert_eq!(replayed.len(), 9);
    assert!(
        matches!(&replayed[6], Ingress::Created { parent, guid, object_type, .. } if parent == "browser@1" && guid == "context@1" && object_type == "BrowserContext")
    );
    assert!(
        matches!(&replayed[7], Ingress::Event { guid, method, params } if guid == "browser@1" && method == "context" && params == &json!({"context":{"guid":"context@1"}}))
    );
    assert!(matches!(&replayed[8], Ingress::Created { guid, .. } if guid == "playwright@1"));
    assert_eq!(reply, ReplyOutcome::Result(json!({"echo":7})));
    assert!(
        matches!(later, Ingress::Event { guid, method, params } if guid == "browser@1" && method == "syntheticNotice" && params == json!({"order":1}))
    );
    assert!(!browser.is_connected());
    assert!(close.report.failed.is_empty());
    assert!(close.transport_error.is_none());
}
