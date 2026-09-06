//! Explicit live test for external-browser ownership semantics.
//!
//! Run with:
//! `PWRIGHT_CDP_WS_URL=ws://... cargo test -p pwright-provider-cdp --release --test live_attachment -- --ignored`

use pwright_browser_provider::AttachBrowserProvider;
use pwright_cdp::connection::ConnectionConfig;
use pwright_provider_cdp::{CdpAttachProvider, CdpAttachRequest, CdpWebSocketEndpoint};
use serde_json::json;

#[tokio::test]
#[ignore = "requires an explicit existing Chrome CDP WebSocket endpoint"]
async fn dropping_external_attachment_leaves_browser_reconnectable() {
    // Arrange
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must identify an existing browser"),
    )
    .expect("PWRIGHT_CDP_WS_URL must be a ws:// or wss:// endpoint");
    let provider = CdpAttachProvider::new();
    let first = provider
        .attach(CdpAttachRequest::new(
            endpoint.clone(),
            ConnectionConfig::default(),
        ))
        .await
        .expect("first external attachment should connect");
    first
        .connection()
        .send_command("Browser.getVersion", json!({}))
        .await
        .expect("first attachment should reach the browser");

    // Act
    drop(first);
    let second = provider
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await
        .expect("browser should remain reconnectable after attachment drop");

    // Assert
    let version = second
        .connection()
        .send_command("Browser.getVersion", json!({}))
        .await
        .expect("second attachment should reach the same browser process");
    assert!(
        version
            .get("product")
            .and_then(|value| value.as_str())
            .is_some()
    );
}
