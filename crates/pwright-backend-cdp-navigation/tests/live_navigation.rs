use std::sync::Arc;

use pwright_backend_cdp_navigation::CdpNavigationService;
use pwright_cap_navigation::GotoParams;
use pwright_cdp::{CdpClient, CdpConnection, CdpSession};
use pwright_core::Service;

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn isolated_navigation_backend_reaches_a_real_chromium_target() {
    // Arrange
    let websocket_url = std::env::var("PWRIGHT_CDP_WS_URL")
        .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile");
    let connection = CdpConnection::connect(&websocket_url)
        .await
        .expect("CDP connection should open");
    let browser = CdpSession::browser(Arc::clone(&connection));
    let browser_context_id = browser
        .target_create_browser_context(None, None)
        .await
        .expect("temporary browser context should open");
    let target_id = browser
        .target_create_in_context("about:blank", &browser_context_id)
        .await
        .expect("temporary target should open");
    let session_id = browser
        .target_attach(&target_id)
        .await
        .expect("temporary target should attach");
    let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        session_id,
        target_id.clone(),
    ));
    let service = CdpNavigationService::new(page);

    // Act
    let navigation = service
        .call(GotoParams::new(
            "data:text/html,<title>isolated-navigation-leaf</title>",
        ))
        .await;
    browser
        .target_dispose_browser_context(&browser_context_id)
        .await
        .expect("temporary browser context should dispose");

    // Assert
    let navigation = navigation.expect("isolated navigation backend should reach real Chrome");
    assert!(!navigation.frame_id.is_empty());
    assert!(!navigation.same_document);
    assert!(navigation.response.is_none());
    assert!(navigation.url.starts_with("data:text/html,"));
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("external browser should remain alive");
}
