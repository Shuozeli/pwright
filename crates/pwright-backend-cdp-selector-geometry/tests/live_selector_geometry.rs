use std::sync::Arc;
use std::time::Duration;

use pwright_backend_cdp_locator_screenshot::LocatorScreenshotGeometryResolver;
use pwright_backend_cdp_selector_geometry::CdpSelectorGeometryService;
use pwright_browser_provider::AttachBrowserProvider;
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpClient, CdpSession};
use pwright_provider_cdp::{CdpAttachProvider, CdpAttachRequest, CdpWebSocketEndpoint};
use pwright_selector_model::Selector;

#[tokio::test]
#[ignore = "requires the dedicated Chrome CDP process and PWRIGHT_CDP_WS_URL"]
async fn first_party_plugin_resolves_visible_and_scrolled_geometry_in_real_chromium() {
    // Arrange an isolated temporary Context in the externally owned browser.
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile"),
    )
    .expect("dedicated Chrome endpoint should be valid");
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await
        .expect("provider should attach without browser ownership");
    let connection = Arc::clone(attachment.connection());
    let browser = CdpSession::browser(Arc::clone(&connection));
    let browser_context_id = browser
        .target_create_browser_context(None, None)
        .await
        .expect("temporary Context should open");
    let target_id = browser
        .target_create_in_context("about:blank", &browser_context_id)
        .await
        .expect("temporary Page should open");
    let session_id = browser
        .target_attach(&target_id)
        .await
        .expect("temporary Page should attach");
    let page: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
        Arc::clone(&connection),
        session_id,
        target_id.clone(),
    ));
    page.runtime_evaluate(
        r#"(() => {
          document.documentElement.style.margin = '0';
          document.body.style.margin = '0';
          document.body.style.height = '1800px';
          document.body.innerHTML = `
            <div id="visible" style="position:absolute;left:10px;top:20px;width:120px;height:80px"></div>
            <div id="below" style="position:absolute;left:15px;top:1400px;width:90px;height:60px"></div>`;
          return true;
        })()"#,
    )
    .await
    .expect("fixture should install");
    let service =
        CdpSelectorGeometryService::with_retry_delay(Arc::clone(&page), Duration::from_millis(10));

    // Act through the standalone first-party resolver only.
    let visible = service
        .resolve(Selector::css("#visible").expect("selector should validate"))
        .await
        .expect("visible selector should resolve");
    let below = service
        .resolve(Selector::css("#below").expect("selector should validate"))
        .await
        .expect("below-fold selector should scroll then resolve");
    let scroll_y = page
        .runtime_evaluate("scrollY")
        .await
        .expect("scroll position should remain observable")["result"]["value"]
        .as_f64()
        .expect("scroll position should be numeric");
    let metrics = service.metrics();
    browser
        .target_dispose_browser_context(&browser_context_id)
        .await
        .expect("temporary Context should dispose");

    // Assert geometry, explicit scrolling, shared runtime use, and external ownership.
    assert_eq!(
        (visible.x(), visible.y(), visible.width(), visible.height()),
        (10.0, 20.0, 120.0, 80.0)
    );
    assert_eq!((below.width(), below.height()), (90.0, 60.0));
    assert!(scroll_y > 0.0);
    assert_eq!(metrics.installations, 1);
    assert_eq!(metrics.scroll_commits, 1);
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("externally owned Chrome should remain alive");
}
