//! TodoMVC — Deleting Todos
//!
//! Equivalent of playwright/examples/todomvc/tests/deleting-todos/*.spec.ts
//!
//! Tests: delete single, delete specific from multiple, clear all completed

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
/// await page.getByTestId('todo-item').hover();
/// await expect(page.getByRole('button', { name: 'Delete' })).toBeVisible();
/// await page.getByRole('button', { name: 'Delete' }).click();
/// await expect(page.getByText('Task to delete')).not.toBeVisible();
/// ```
#[tokio::test]
async fn should_delete_single_todo() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    // Add a todo
    page.get_by_placeholder("What needs to be done?")
        .fill("Task to delete")
        .await
        .unwrap();
    page.keyboard().press("Enter").await.unwrap();

    // Hover over todo → shows delete button
    page.get_by_test_id("todo-item").hover().await.unwrap();

    let hover_calls: Vec<_> = mock
        .calls_for("Input.dispatchMouseEvent")
        .into_iter()
        .filter(|c| c.args[0]["type"] == "mouseMoved")
        .collect();
    assert!(!hover_calls.is_empty());

    // Click delete
    page.locator("button.destroy").click().await.unwrap();

    let all_mouse = mock.calls_for("Input.dispatchMouseEvent");
    assert!(all_mouse.len() >= 3); // hover + press + release
}

/// ```typescript
/// // Playwright
/// // Add 3 todos, hover on 2nd, click delete, verify only 2 remain
/// ```
#[tokio::test]
async fn should_delete_specific_todo_from_multiple() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    for text in &["Keep 1", "Delete me", "Keep 2"] {
        page.get_by_placeholder("What needs to be done?")
            .fill(text)
            .await
            .unwrap();
        page.keyboard().press("Enter").await.unwrap();
    }

    // Hover on specific item and click delete
    page.locator(".todo-list li:nth-child(2)")
        .hover()
        .await
        .unwrap();
    page.locator(".todo-list li:nth-child(2) button.destroy")
        .click()
        .await
        .unwrap();

    let qs_calls = mock.calls_for("DOM.querySelector");
    assert!(qs_calls.iter().any(|c| {
        c.args[0]["selector"]
            .as_str()
            .is_some_and(|s| s.contains("nth-child(2)"))
    }));
}

/// ```typescript
/// // Playwright
/// await page.getByRole('button', { name: 'Clear completed' }).click();
/// ```
#[tokio::test]
async fn should_clear_all_completed_todos() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    page.locator("button.clear-completed")
        .click()
        .await
        .unwrap();

    let qs_calls = mock.calls_for("DOM.querySelector");
    assert_eq!(
        qs_calls.last().unwrap().args[0]["selector"],
        "button.clear-completed"
    );
}
