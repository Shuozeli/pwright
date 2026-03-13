//! Mock Filesystem — Directory Reader
//!
//! Equivalent of playwright/examples/mock-filesystem/tests/directory-reader.spec.js

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
/// test('should display directory tree', async ({ page }) => {
///   await page.goto('/ls-dir.html');
///   await page.locator('button', { hasText: 'Open directory' }).click();
///   await expect(page.locator('#dir')).toContainText([
///     'file1', 'dir1', 'file2', 'file3', 'dir2', 'file4', 'file5'
///   ]);
/// });
/// ```
#[tokio::test]
async fn should_display_directory_tree() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    page.goto("/ls-dir.html", None).await.unwrap();

    // Click "Open directory"
    page.locator("button").click().await.unwrap();

    // Read directory listing
    mock.set_call_function_response(serde_json::json!({
        "result": {"value": "file1\ndir1\n  file2\n  file3\ndir2\n  file4\n  file5"}
    }));
    let text = page.locator("#dir").text_content().await.unwrap();
    assert!(text.is_some());
    let content = text.unwrap();
    for expected in &["file1", "dir1", "file2", "file3", "dir2", "file4", "file5"] {
        assert!(content.contains(expected), "should contain {}", expected);
    }
}
