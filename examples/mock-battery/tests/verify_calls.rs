//! Mock Battery — Verify API Calls
//!
//! Equivalent of playwright/examples/mock-battery/tests/verify-calls.spec.js

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
/// test('verify battery calls', async ({ page }) => {
///   await page.goto('/');
///   await expect(page.locator('.battery-percentage')).toHaveText('75%');
///   expect(log).toEqual([
///     'getBattery',
///     'addEventListener:chargingchange',
///     'addEventListener:levelchange'
///   ]);
/// });
/// ```
#[tokio::test]
async fn verify_battery_api_calls() {
    let mock = mock_with_navigation();
    let page = Page::new(mock.clone());

    page.goto("/", None).await.unwrap();

    // Verify API calls via evaluate
    mock.set_evaluate_response(serde_json::json!({
        "result": {"value": [
            "getBattery",
            "addEventListener:chargingchange",
            "addEventListener:levelchange"
        ]}
    }));
    let result = page.evaluate_sync("window._apiLog").await.unwrap();
    let log = result.get("value").and_then(|v| v.as_array()).unwrap();
    assert_eq!(log.len(), 3);
    assert_eq!(log[0], "getBattery");
    assert_eq!(log[1], "addEventListener:chargingchange");
    assert_eq!(log[2], "addEventListener:levelchange");
}
