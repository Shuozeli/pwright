//! SVGOMG Example
//!
//! Equivalent of playwright/examples/svgomg/tests/example.spec.ts
//!
//! Tests: verify menu, verify defaults, verify features, reset, download, open svg

use pwright_bridge::playwright::Page;
use pwright_bridge::test_utils::MockCdpClient;
use std::sync::Arc;

fn mock_with_navigation() -> Arc<MockCdpClient> {
    let mock = Arc::new(MockCdpClient::new());
    mock.set_navigate_response(serde_json::json!({"frameId": "F1"}));
    mock.set_evaluate_response(serde_json::json!({"result": {"value": "complete"}}));
    mock
}

fn mock_with_element() -> Arc<MockCdpClient> {
    let mock = Arc::new(MockCdpClient::new());
    mock.set_query_selector_response(42);
    mock.set_resolve_node(serde_json::json!({"object": {"objectId": "obj-42"}}));
    mock.set_call_function_response(serde_json::json!({
        "result": {"value": {"x": 150.0, "y": 250.0}}
    }));
    mock
}

/// ```typescript
/// // Playwright
/// await expect(page.locator('.menu li')).toHaveText([
///   'Open SVG', 'Paste markup', 'Demo', 'Contribute'
/// ]);
/// ```
#[tokio::test]
async fn verify_menu_items() {
    let mock = mock_with_navigation();
    mock.set_query_selector_all_response(vec![1, 2, 3, 4]);

    let page = Page::new(mock.clone());
    page.goto("https://demo.playwright.dev/svgomg", None)
        .await
        .unwrap();

    let count = page.locator(".menu li").count().await.unwrap();
    assert_eq!(count, 4);

    let qs_calls = mock.calls_for("DOM.querySelectorAll");
    assert_eq!(qs_calls[0].args[0]["selector"], ".menu li");
}

/// ```typescript
/// // Playwright
/// test('verify default global settings', async ({ page }) => {
///   const menuItems = page.locator('.settings-scroller .global .setting-item-toggle');
///   await expect(menuItems).toHaveText([
///     'Show original', 'Compare gzipped', 'Prettify markup', 'Multipass',
///   ]);
///   await expect(toggle.locator('text=Show original')).not.toBeChecked();
///   await expect(toggle.locator('text=Compare gzipped')).toBeChecked();
/// });
/// ```
#[tokio::test]
async fn verify_default_global_settings() {
    let mock = mock_with_element();
    mock.set_query_selector_all_response(vec![1, 2, 3, 4]);

    let page = Page::new(mock.clone());

    // Click Demo first
    page.locator(".menu-item:nth-child(3)")
        .click()
        .await
        .unwrap();

    // Count global toggle settings
    let count = page
        .locator(".settings-scroller .global .setting-item-toggle")
        .count()
        .await
        .unwrap();
    assert_eq!(count, 4);
}

/// ```typescript
/// // Playwright
/// test('verify default features', async ({ page }) => {
///   const enabledOptions = ['Clean up attribute whitespace', 'Clean up IDs', ...];
///   const disabledOptions = ['Prefer viewBox to width/height', ...];
///   for (const option of enabledOptions) {
///     await expect(page.locator(`.setting-item-toggle >> text=${option}`)).toBeChecked();
///   }
/// });
/// ```
#[tokio::test]
async fn verify_default_features() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    // Click Demo
    page.locator(".menu-item:nth-child(3)")
        .click()
        .await
        .unwrap();

    // Check enabled features
    for option in &[
        "Clean up attribute whitespace",
        "Clean up IDs",
        "Collapse useless groups",
    ] {
        let selector = format!(".setting-item-toggle:has(text='{}')", option);
        let loc = page.locator(&selector);
        let visible = loc.is_visible().await.unwrap();
        assert!(visible, "{} should be visible", option);
    }
}

/// ```typescript
/// // Playwright
/// test('reset settings', async ({ page }) => {
///   const showOriginalSetting = page.locator('.setting-item-toggle >> text=Show original');
///   await showOriginalSetting.click();
///   await expect(showOriginalSetting).toBeChecked();
///   await page.locator('button >> text=Reset all').click();
///   await expect(showOriginalSetting).not.toBeChecked();
/// });
/// ```
#[tokio::test]
async fn reset_settings() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    // Click Demo
    page.locator(".menu-item:nth-child(3)")
        .click()
        .await
        .unwrap();

    // Toggle "Show original"
    page.locator(".setting-item-toggle").click().await.unwrap();

    // Click "Reset all"
    page.locator("button.reset-all").click().await.unwrap();

    // 3 clicks × 2 events
    let mouse_calls = mock.calls_for("Input.dispatchMouseEvent");
    assert_eq!(mouse_calls.len(), 6);
}

/// ```typescript
/// // Playwright
/// test('download result', async ({ page }) => {
///   const downloadButton = page.locator('a[title=Download]');
///   await expect(downloadButton).toHaveAttribute('href', /blob/);
///   const [download] = await Promise.all([
///     page.waitForEvent('download'),
///     downloadButton.click()
///   ]);
///   expect(download.suggestedFilename()).toBe('car-lite.svg');
/// });
/// ```
#[tokio::test]
async fn download_result() {
    let mock = mock_with_element();
    mock.set_get_attributes_response(vec![
        "title".to_string(),
        "Download".to_string(),
        "href".to_string(),
        "blob:https://demo.playwright.dev/abc123".to_string(),
    ]);

    let page = Page::new(mock.clone());

    // Check download button has blob href
    let href = page
        .locator("a[title=Download]")
        .get_attribute("href")
        .await
        .unwrap();
    assert!(href.is_some());
    assert!(href.unwrap().contains("blob"));

    // Click download
    page.locator("a[title=Download]").click().await.unwrap();

    let mouse_calls = mock.calls_for("Input.dispatchMouseEvent");
    assert!(mouse_calls.len() >= 2);
}

/// ```typescript
/// // Playwright
/// test('open svg', async ({ page }) => {
///   const [fileChooser] = await Promise.all([
///     page.waitForEvent('filechooser'),
///     page.click('text=Open SVG'),
///   ]);
///   await fileChooser.setFiles({ name: 'file.svg', ... });
///   const markup = await page.frameLocator('.svg-frame').locator('svg').evaluate(...);
///   expect(markup).toMatch(/<svg.*<\/svg>/);
/// });
/// ```
#[tokio::test]
async fn open_svg() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    // Click "Open SVG" menu item
    page.locator(".menu-item:first-child")
        .click()
        .await
        .unwrap();

    let qs_calls = mock.calls_for("DOM.querySelector");
    assert_eq!(
        qs_calls.last().unwrap().args[0]["selector"],
        ".menu-item:first-child"
    );
}
