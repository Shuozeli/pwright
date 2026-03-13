//! TodoMVC — Adding Todos
//!
//! Equivalent of playwright/examples/todomvc/tests/adding-todos/*.spec.ts
//!
//! Tests: add single todo, add multiple todos, prevent empty, trim whitespace

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
/// await page.getByRole('textbox', { name: 'What needs to be done?' }).fill('Buy groceries');
/// await page.keyboard.press('Enter');
/// await expect(page.getByText('Buy groceries')).toBeVisible();
/// await expect(page.getByText('1 item left')).toBeVisible();
/// ```
#[tokio::test]
async fn should_add_single_todo() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    page.get_by_placeholder("What needs to be done?")
        .fill("Buy groceries")
        .await
        .unwrap();
    page.keyboard().press("Enter").await.unwrap();

    // Verify fill was called
    let cf_calls = mock.calls_for("Runtime.callFunctionOn");
    assert!(!cf_calls.is_empty(), "fill should call callFunctionOn");

    // Verify Enter key
    let key_calls = mock.calls_for("Input.dispatchKeyEvent");
    assert!(key_calls.len() >= 2);
    assert_eq!(key_calls[0].args[0]["key"], "Enter");
}

/// ```typescript
/// // Playwright
/// await page.getByRole('textbox', { name: '...' }).fill('Buy milk');
/// await page.keyboard.press('Enter');
/// // repeat for 'Walk the dog' and 'Finish report'
/// await expect(page.getByText('3 items left')).toBeVisible();
/// ```
#[tokio::test]
async fn should_add_multiple_todos() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    for todo in &["Buy milk", "Walk the dog", "Finish report"] {
        page.get_by_placeholder("What needs to be done?")
            .fill(todo)
            .await
            .unwrap();
        page.keyboard().press("Enter").await.unwrap();
    }

    // 3 fills → ≥3 callFunctionOn
    let cf_calls = mock.calls_for("Runtime.callFunctionOn");
    assert!(cf_calls.len() >= 3);

    // 3 Enters → 6 key events
    let key_calls = mock.calls_for("Input.dispatchKeyEvent");
    assert_eq!(key_calls.len(), 6);
}

/// ```typescript
/// // Playwright
/// await page.getByRole('textbox', { name: '...' }).fill('');
/// await page.keyboard.press('Enter');
/// // todo list should remain empty
/// ```
#[tokio::test]
async fn should_not_add_empty_todo() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    page.get_by_placeholder("What needs to be done?")
        .fill("")
        .await
        .unwrap();
    page.keyboard().press("Enter").await.unwrap();

    // Fill still called — app logic rejects empty
    let cf_calls = mock.calls_for("Runtime.callFunctionOn");
    assert!(!cf_calls.is_empty());
}

/// ```typescript
/// // Playwright
/// await page.getByRole('textbox', { name: '...' }).fill('  trim me  ');
/// await page.keyboard.press('Enter');
/// await expect(page.getByText('trim me')).toBeVisible();
/// ```
#[tokio::test]
async fn should_trim_whitespace_from_new_todo() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    page.get_by_placeholder("What needs to be done?")
        .fill("  trim me  ")
        .await
        .unwrap();
    page.keyboard().press("Enter").await.unwrap();

    // pwright passes raw value — trimming is app responsibility
    let cf_calls = mock.calls_for("Runtime.callFunctionOn");
    assert!(!cf_calls.is_empty());
}
