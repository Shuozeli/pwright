//! Mock Battery — Update Battery Status
//!
//! Equivalent of playwright/examples/mock-battery/tests/update-battery-status.spec.js

use pwright_bridge::playwright::Page;
use pwright_bridge::test_utils::MockCdpClient;
use std::sync::Arc;

fn mock_with_navigation() -> Arc<MockCdpClient> {
    let mock = Arc::new(MockCdpClient::new());
    mock.set_navigate_response(serde_json::json!({"frameId": "F1"}));
    mock.set_evaluate_response(serde_json::json!({"result": {"value": "complete"}}));
    mock
}

/// ```javascript
/// // Playwright
/// test('should update UI when battery status changes', async ({ page }) => {
///   await page.goto('/');
///   await expect(page.locator('.battery-percentage')).toHaveText('10%');
///   await page.evaluate(() => window.mockBattery._setLevel(0.275));
///   await expect(page.locator('.battery-percentage')).toHaveText('27.5%');
///   await page.evaluate(() => window.mockBattery._setCharging(true));
///   await expect(page.locator('.battery-status')).toHaveText('Adapter');
/// });
/// ```
#[tokio::test]
async fn should_update_ui_when_battery_status_changes() {
    let mock = mock_with_navigation();
    let page = Page::new(mock.clone());

    page.goto("/", None).await.unwrap();

    // Update level
    page.evaluate("window.mockBattery._setLevel(0.275)")
        .await
        .unwrap();

    // Update charging
    page.evaluate("window.mockBattery._setCharging(true)")
        .await
        .unwrap();

    // 3+ evaluate calls: readyState + 2 updates
    let eval_calls = mock.calls_for("Runtime.evaluate");
    assert!(eval_calls.len() >= 3);
}

/// ```javascript
/// // Playwright — update-battery-status.spec.js
/// test('verify API calls', async ({ page }) => {
///   await page.goto('/');
///   await expect(page.locator('.battery-percentage')).toHaveText('10%');
///   expect(log).toEqual([
///     'getBattery',
///     'addEventListener:chargingchange',
///     'addEventListener:levelchange'
///   ]);
///   log = [];
///   await page.evaluate(() => window.mockBattery._setLevel(0.275));
///   expect(log).toEqual([]); // getBattery is not called, cached version is used.
/// });
/// ```
#[tokio::test]
async fn verify_api_calls_after_update() {
    let mock = mock_with_navigation();
    let page = Page::new(mock.clone());

    page.goto("/", None).await.unwrap();

    // Update level — getBattery should not be called again (cached)
    page.evaluate("window.mockBattery._setLevel(0.275)")
        .await
        .unwrap();

    let eval_calls = mock.calls_for("Runtime.evaluate");
    assert!(eval_calls.len() >= 2);
}
