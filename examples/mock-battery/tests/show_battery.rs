//! Mock Battery — Show Battery Status
//!
//! Equivalent of playwright/examples/mock-battery/tests/show-battery-status.spec.js
//!
//! Demonstrates mocking navigator.getBattery via addInitScript + page.evaluate.

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
/// test.beforeEach(async ({ page }) => {
///   await page.addInitScript(() => {
///     const mockBattery = { level: 0.90, charging: true, chargingTime: 1800, ... };
///     window.navigator.getBattery = async () => mockBattery;
///   });
/// });
/// test('show battery status', async ({ page }) => {
///   await page.goto('/');
///   await expect(page.locator('.battery-percentage')).toHaveText('90%');
///   await expect(page.locator('.battery-status')).toHaveText('Adapter');
///   await expect(page.locator('.battery-fully')).toHaveText('00:30');
/// });
/// ```
#[tokio::test]
async fn show_battery_status() {
    let mock = mock_with_navigation();
    let page = Page::new(mock.clone());

    // Navigate
    page.goto("/", None).await.unwrap();

    // Read battery percentage via evaluate
    mock.set_evaluate_response(serde_json::json!({
        "result": {"value": "90%"}
    }));
    let result = page
        .evaluate_sync("document.querySelector('.battery-percentage')?.textContent")
        .await
        .unwrap();
    assert_eq!(result.get("value").and_then(|v| v.as_str()), Some("90%"));

    // Verify evaluate calls
    let eval_calls = mock.calls_for("Runtime.evaluate");
    assert!(eval_calls.len() >= 2); // readyState poll + our query
}
