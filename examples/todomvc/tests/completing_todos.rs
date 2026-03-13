//! TodoMVC — Completing Todos
//!
//! Equivalent of playwright/examples/todomvc/tests/completing-todos/*.spec.ts
//!
//! Tests: complete single, complete multiple, toggle all, toggle incomplete, uncomplete

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
/// await page.getByRole('textbox', { name: '...' }).fill('Buy groceries');
/// await page.keyboard.press('Enter');
/// await page.getByRole('checkbox', { name: 'Toggle Todo' }).click();
/// await expect(page.getByRole('checkbox')).toBeChecked();
/// await expect(page.getByText('0 items left')).toBeVisible();
/// ```
#[tokio::test]
async fn should_complete_single_todo() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    // Add todo
    page.get_by_placeholder("What needs to be done?")
        .fill("Buy groceries")
        .await
        .unwrap();
    page.keyboard().press("Enter").await.unwrap();

    // Click toggle checkbox
    page.locator("input.toggle").click().await.unwrap();

    let mouse_calls = mock.calls_for("Input.dispatchMouseEvent");
    assert!(mouse_calls.len() >= 2);
    assert_eq!(
        mouse_calls[mouse_calls.len() - 2].args[0]["type"],
        "mousePressed"
    );
    assert_eq!(
        mouse_calls[mouse_calls.len() - 1].args[0]["type"],
        "mouseReleased"
    );
}

/// ```typescript
/// // Playwright — complete multiple todos
/// // Add 3 todos, click each toggle checkbox
/// ```
#[tokio::test]
async fn should_complete_multiple_todos() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    for text in &["Todo 1", "Todo 2", "Todo 3"] {
        page.get_by_placeholder("What needs to be done?")
            .fill(text)
            .await
            .unwrap();
        page.keyboard().press("Enter").await.unwrap();
    }

    // Click toggle on each (3 clicks)
    for _ in 0..3 {
        page.locator("input.toggle").click().await.unwrap();
    }

    // 3 clicks × 2 mouse events = 6
    let mouse_calls = mock.calls_for("Input.dispatchMouseEvent");
    assert_eq!(mouse_calls.len(), 6);
}

/// ```typescript
/// // Playwright
/// await page.getByLabel('Mark all as complete').click();
/// ```
#[tokio::test]
async fn should_toggle_all_todos_complete() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    page.locator("#toggle-all").click().await.unwrap();

    let qs_calls = mock.calls_for("DOM.querySelector");
    assert_eq!(qs_calls.last().unwrap().args[0]["selector"], "#toggle-all");
}

/// ```typescript
/// // Playwright
/// // toggle all → all complete, toggle all again → all incomplete
/// await page.getByLabel('Mark all as complete').click();
/// await page.getByLabel('Mark all as complete').click();
/// ```
#[tokio::test]
async fn should_toggle_all_todos_incomplete() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    // Toggle twice
    page.locator("#toggle-all").click().await.unwrap();
    page.locator("#toggle-all").click().await.unwrap();

    let mouse_calls = mock.calls_for("Input.dispatchMouseEvent");
    assert_eq!(mouse_calls.len(), 4); // 2 clicks × 2 events
}

/// ```typescript
/// // Playwright
/// // Complete a todo, then click the checkbox again to uncomplete
/// await page.getByRole('checkbox', { name: 'Toggle Todo' }).click();
/// await page.getByRole('checkbox', { name: 'Toggle Todo' }).click();
/// ```
#[tokio::test]
async fn should_uncomplete_completed_todo() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    // Toggle twice
    page.locator("input.toggle").click().await.unwrap();
    page.locator("input.toggle").click().await.unwrap();

    let mouse_calls = mock.calls_for("Input.dispatchMouseEvent");
    assert_eq!(mouse_calls.len(), 4);
}
