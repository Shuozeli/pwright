//! TodoMVC — Filtering Todos
//!
//! Equivalent of playwright/examples/todomvc/tests/filtering-todos/*.spec.ts
//!
//! Tests: filter active todos

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
    mock
}

/// ```typescript
/// // Playwright
/// // 1. Add 3 todos: 'Active 1', 'Active 2', 'Will complete'
/// // 2. Complete 'Will complete'
/// await page.getByRole('listitem').filter({ hasText: 'Will complete' })
///   .getByLabel('Toggle Todo').click();
/// // 3. Click Active filter
/// await page.getByRole('link', { name: 'Active' }).click();
/// await expect(page).toHaveURL(/#\/active$/);
/// await expect(page.getByText('Active 1')).toBeVisible();
/// await expect(page.getByText('Will complete')).not.toBeVisible();
/// ```
#[tokio::test]
async fn should_filter_active_todos() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    // Add 3 todos
    for text in &["Active 1", "Active 2", "Will complete"] {
        page.get_by_placeholder("What needs to be done?")
            .fill(text)
            .await
            .unwrap();
        page.keyboard().press("Enter").await.unwrap();
    }

    // Complete third todo
    page.locator("input.toggle:last-of-type")
        .click()
        .await
        .unwrap();

    // Click Active filter link
    page.locator("a[href='#/active']").click().await.unwrap();

    let qs_calls = mock.calls_for("DOM.querySelector");
    assert!(qs_calls.iter().any(|c| {
        c.args[0]["selector"]
            .as_str()
            .is_some_and(|s| s.contains("#/active"))
    }));
}
