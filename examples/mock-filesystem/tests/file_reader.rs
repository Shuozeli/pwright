//! Mock Filesystem — File Reader
//!
//! Equivalent of playwright/examples/mock-filesystem/tests/file-reader.spec.js

use pwright_bridge::playwright::Page;
use pwright_bridge::test_utils::MockCdpClient;
use std::sync::Arc;

fn mock_with_element() -> Arc<MockCdpClient> {
    let mock = Arc::new(MockCdpClient::new());
    mock.set_query_selector_response(42);
    mock.set_resolve_node(serde_json::json!({"object": {"objectId": "obj-42"}}));
    mock.set_call_function_response(serde_json::json!({
        "result": {"value": {"x": 150.0, "y": 250.0}}
    }));
    mock.set_navigate_response(serde_json::json!({"frameId": "F1"}));
    mock.set_evaluate_response(serde_json::json!({"result": {"value": "complete"}}));
    mock
}

/// ```javascript
/// // Playwright
/// test.beforeEach(async ({page}) => {
///   await page.addInitScript(() => {
///     class FileSystemFileHandleMock { ... }
///     window.showOpenFilePicker = async () =>
///       [new FileSystemFileHandleMock(new File(['Test content.'], "foo.txt"))];
///   });
/// });
/// test('show file picker with mock class', async ({ page }) => {
///   await page.goto('/file-picker.html');
///   await page.locator('button', { hasText: 'Open File' }).click();
///   await expect(page.locator('textarea')).toHaveValue('Test content.');
/// });
/// ```
#[tokio::test]
async fn show_file_picker_with_mock_class() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    page.goto("/file-picker.html", None).await.unwrap();

    // Click "Open File" button
    page.locator("button").click().await.unwrap();

    let qs_calls = mock.calls_for("DOM.querySelector");
    assert_eq!(qs_calls.last().unwrap().args[0]["selector"], "button");

    // Verify click
    let mouse_calls = mock.calls_for("Input.dispatchMouseEvent");
    assert!(mouse_calls.len() >= 2);
}
