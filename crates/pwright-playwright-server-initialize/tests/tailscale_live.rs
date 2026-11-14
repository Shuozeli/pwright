//! Explicit opt-in real WebSocket fixture bound only to the Tailscale interface.

use std::net::IpAddr;

use futures_util::{SinkExt, StreamExt};
use pwright_playwright_server_initialize::{
    InitializeObservation, LiveWsConnector, SdkLanguage, observe_root_initialize,
    observe_root_initialize_request,
};
use pwright_transport_ws::{WsCloseFrame, WsEndpoint, WsLimits, WsRequest};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::handshake::server::{
    Callback, ErrorResponse, Request, Response,
};
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, Message, frame::coding::CloseCode};

struct CheckJavascriptUpgrade;

impl Callback for CheckJavascriptUpgrade {
    fn on_request(self, request: &Request, response: Response) -> Result<Response, ErrorResponse> {
        assert_eq!(request.uri().path(), "/ws");
        assert_eq!(request.headers()["x-playwright-launch-options"], "{}");
        Ok(response)
    }
}
use tokio_tungstenite::{accept_async, accept_hdr_async};

#[tokio::test]
#[ignore = "requires explicit TAILSCALE_IP and full TAILSCALE_HOST"]
async fn real_ws_fixture_receives_exact_root_initialize_and_closes_with_reason() {
    // Arrange: a temporary listener on the Tailscale interface, never loopback
    // or a public wildcard. The client addresses its full MagicDNS hostname.
    let ip: IpAddr = std::env::var("TAILSCALE_IP")
        .expect("set TAILSCALE_IP explicitly")
        .parse()
        .expect("TAILSCALE_IP must be a single IP address");
    let host = std::env::var("TAILSCALE_HOST").expect("set full TAILSCALE_HOST explicitly");
    assert!(host.contains('.'), "TAILSCALE_HOST must include its suffix");
    let listener = TcpListener::bind((ip, 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let endpoint = format!("ws://{host}:{port}/ws");
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut peer = accept_async(stream).await.unwrap();
        let Some(Ok(Message::Text(text))) = peer.next().await else {
            panic!("expected Root.initialize text frame");
        };
        assert_eq!(
            text.as_str(),
            "{\"id\":1,\"guid\":\"\",\"method\":\"initialize\",\"params\":{\"sdkLanguage\":\"python\"},\"metadata\":{\"wallTime\":123,\"apiName\":\"Connection.init\",\"internal\":false}}"
        );
        peer.send(Message::Close(Some(CloseFrame {
            code: CloseCode::from(4123),
            reason: "Oh my!".into(),
        })))
        .await
        .unwrap();
        let outcome = peer.next().await;
        let Some(Ok(Message::Close(Some(reply)))) = outcome else {
            panic!("expected client Close reply, got {outcome:?}");
        };
        assert_eq!(u16::from(reply.code), 4123);
        assert_eq!(reply.reason, "Oh my!");
    });

    // Act
    let observed =
        observe_root_initialize(&LiveWsConnector, &endpoint, SdkLanguage::Python, 123, 2_000)
            .await
            .unwrap();

    // Assert
    assert_eq!(
        observed,
        InitializeObservation::RemoteClose(Some(WsCloseFrame::new(4123, "Oh my!").unwrap()))
    );
    server.await.unwrap();
}

#[tokio::test]
#[ignore = "requires explicit TAILSCALE_IP and full TAILSCALE_HOST"]
async fn javascript_request_forwards_headers_and_observes_custom_close() {
    // Arrange: the live fixture is Tailscale-only and inspects the upgrade.
    let ip: IpAddr = std::env::var("TAILSCALE_IP")
        .expect("set TAILSCALE_IP explicitly")
        .parse()
        .expect("TAILSCALE_IP must be a single IP address");
    let host = std::env::var("TAILSCALE_HOST").expect("set full TAILSCALE_HOST explicitly");
    assert!(host.contains('.'), "TAILSCALE_HOST must include its suffix");
    let listener = TcpListener::bind((ip, 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let endpoint = WsEndpoint::parse(format!("ws://{host}:{port}/ws")).unwrap();
    let limits = WsLimits::new(256 * 1024, 4096, 2_000).unwrap();
    let mut request = WsRequest::new(endpoint, limits);
    request
        .add_header("x-playwright-launch-options", "{}")
        .unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut peer = accept_hdr_async(stream, CheckJavascriptUpgrade)
            .await
            .unwrap();
        let Some(Ok(Message::Text(text))) = peer.next().await else {
            panic!("expected Root.initialize text frame");
        };
        assert_eq!(
            text.as_str(),
            "{\"id\":1,\"guid\":\"\",\"method\":\"initialize\",\"params\":{\"sdkLanguage\":\"javascript\"},\"metadata\":{\"wallTime\":456,\"apiName\":\"Connection.init\",\"internal\":false}}"
        );
        peer.send(Message::Close(Some(CloseFrame {
            code: CloseCode::from(4123),
            reason: "Oh my!".into(),
        })))
        .await
        .unwrap();
        let outcome = peer.next().await;
        let Some(Ok(Message::Close(Some(reply)))) = outcome else {
            panic!("expected client Close reply, got {outcome:?}");
        };
        assert_eq!(u16::from(reply.code), 4123);
        assert_eq!(reply.reason, "Oh my!");
    });

    // Act
    let observed =
        observe_root_initialize_request(&LiveWsConnector, &request, SdkLanguage::Javascript, 456)
            .await
            .unwrap();

    // Assert
    assert_eq!(
        observed,
        InitializeObservation::RemoteClose(Some(WsCloseFrame::new(4123, "Oh my!").unwrap()))
    );
    server.await.unwrap();
}
