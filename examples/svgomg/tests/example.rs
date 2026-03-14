//! SVGOMG Example
//!
//! Equivalent of playwright/examples/svgomg/tests/example.spec.ts
//!
//! Tests: verify menu, verify defaults, verify features, reset, download, open svg
//!
//! Uses FakeCdpClient for behavior assertions (count, visibility, attributes)
//! and MockCdpClient for CDP call sequence tests (mouse events).

use pwright_bridge::playwright::Page;
use pwright_bridge::test_utils::MockCdpClient;
use pwright_fake::FakeCdpClient;
use std::sync::Arc;

fn svgomg_page() -> (Arc<FakeCdpClient>, Page) {
    let fake = Arc::new(FakeCdpClient::from_html(
        r#"
        <div class="app">
            <nav class="menu">
                <ul>
                    <li class="menu-item">Open SVG</li>
                    <li class="menu-item">Paste markup</li>
                    <li class="menu-item">Demo</li>
                    <li class="menu-item">Contribute</li>
                </ul>
            </nav>
            <div class="settings-scroller">
                <div class="global">
                    <div class="setting-item-toggle">Show original</div>
                    <div class="setting-item-toggle">Compare gzipped</div>
                    <div class="setting-item-toggle">Prettify markup</div>
                    <div class="setting-item-toggle">Multipass</div>
                </div>
                <div class="features">
                    <div class="setting-item-toggle">Clean up attribute whitespace</div>
                    <div class="setting-item-toggle">Clean up IDs</div>
                    <div class="setting-item-toggle">Collapse useless groups</div>
                </div>
            </div>
            <a title="Download" href="blob:https://demo.playwright.dev/abc123">Download</a>
            <button class="reset-all">Reset all</button>
        </div>
    "#,
    ));
    let page = Page::new(fake.clone());
    (fake, page)
}

/// ```typescript
/// await expect(page.locator('.menu li')).toHaveText([
///   'Open SVG', 'Paste markup', 'Demo', 'Contribute'
/// ]);
/// ```
#[tokio::test]
async fn verify_menu_items() {
    let (_fake, page) = svgomg_page();

    // Real DOM count via FakeCdpClient
    let count = page.locator(".menu li").count().await.unwrap();
    assert_eq!(count, 4);

    // Verify first menu item text
    let first = page
        .locator(".menu li")
        .first()
        .text_content()
        .await
        .unwrap();
    assert_eq!(first, Some("Open SVG".to_string()));

    // Verify last menu item text
    let last = page
        .locator(".menu li")
        .last()
        .text_content()
        .await
        .unwrap();
    assert_eq!(last, Some("Contribute".to_string()));
}

/// ```typescript
/// const menuItems = page.locator('.settings-scroller .global .setting-item-toggle');
/// await expect(menuItems).toHaveCount(4);
/// ```
#[tokio::test]
async fn verify_default_global_settings() {
    let (_fake, page) = svgomg_page();

    let count = page
        .locator(".settings-scroller .global .setting-item-toggle")
        .count()
        .await
        .unwrap();
    assert_eq!(count, 4);
}

/// ```typescript
/// for (const option of enabledOptions) {
///     await expect(page.locator(`.setting-item-toggle:has(text=${option})`)).toBeVisible();
/// }
/// ```
#[tokio::test]
async fn verify_default_features() {
    let (_fake, page) = svgomg_page();

    // Check feature options are visible
    let features = page.locator(".features .setting-item-toggle");
    let count = features.count().await.unwrap();
    assert_eq!(count, 3);

    let first = features.first().text_content().await.unwrap();
    assert_eq!(first, Some("Clean up attribute whitespace".to_string()));
}

/// ```typescript
/// await page.locator('button >> text=Reset all').click();
/// ```
#[tokio::test]
async fn reset_settings() {
    // Use MockCdpClient for click CDP call verification
    let mock = Arc::new(MockCdpClient::new());
    mock.set_query_selector_response(42);
    mock.set_resolve_node(serde_json::json!({"object": {"objectId": "obj-42"}}));
    mock.set_call_function_response(serde_json::json!({
        "result": {"value": {"x": 150.0, "y": 250.0}}
    }));

    let page = Page::new(mock.clone());

    // Click Demo, toggle setting, click reset
    page.locator(".menu-item").click().await.unwrap();
    page.locator(".setting-item-toggle").click().await.unwrap();
    page.locator("button.reset-all").click().await.unwrap();

    let mouse_calls = mock.calls_for("Input.dispatchMouseEvent");
    assert_eq!(mouse_calls.len(), 6); // 3 clicks x 2 events
}

/// ```typescript
/// await expect(downloadButton).toHaveAttribute('href', /blob/);
/// ```
#[tokio::test]
async fn download_result() {
    let (_fake, page) = svgomg_page();

    // Real DOM attribute check
    let href = page
        .locator(r#"a[title="Download"]"#)
        .get_attribute("href")
        .await
        .unwrap();
    assert!(href.is_some());
    assert!(href.unwrap().contains("blob"));
}

/// ```typescript
/// await page.click('text=Open SVG');
/// ```
#[tokio::test]
async fn open_svg() {
    let (_fake, page) = svgomg_page();

    // Click first menu item (Open SVG)
    page.locator(".menu-item").first().click().await.unwrap();

    // Verify it's the right element by text
    let text = page
        .locator(".menu-item")
        .first()
        .text_content()
        .await
        .unwrap();
    assert_eq!(text, Some("Open SVG".to_string()));
}
