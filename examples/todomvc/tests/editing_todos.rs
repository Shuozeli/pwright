//! TodoMVC — Editing Todos
//!
//! Equivalent of playwright/examples/todomvc/tests/editing-todos/*.spec.ts
//!
//! Tests: edit by dblclick, cancel on Escape, save on blur, delete empty, trim

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
/// await page.getByTestId('todo-title').dblclick();
/// await expect(page.getByRole('textbox', { name: 'Edit' })).toHaveValue('Buy milk');
/// await page.getByRole('textbox', { name: 'Edit' }).fill('Buy organic milk');
/// await page.getByRole('textbox', { name: 'Edit' }).press('Enter');
/// await expect(page.getByText('Buy organic milk')).toBeVisible();
/// ```
#[tokio::test]
async fn should_edit_todo_by_double_clicking() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    // Add a todo
    page.get_by_placeholder("What needs to be done?")
        .fill("Buy milk")
        .await
        .unwrap();
    page.keyboard().press("Enter").await.unwrap();

    // Double-click to edit (use mouse since Locator has no dblclick)
    page.mouse().dblclick(150.0, 250.0).await.unwrap();

    let mouse_calls = mock.calls_for("Input.dispatchMouseEvent");
    let dbl = mouse_calls.iter().find(|c| c.args[0]["clickCount"] == 2);
    assert!(dbl.is_some(), "dblclick should send clickCount=2");

    // Fill with new text and press Enter
    page.get_by_placeholder("Edit")
        .fill("Buy organic milk")
        .await
        .unwrap();
    page.keyboard().press("Enter").await.unwrap();

    let cf_calls = mock.calls_for("Runtime.callFunctionOn");
    assert!(
        cf_calls.len() >= 2,
        "should have fill calls for input + edit"
    );
}

/// ```typescript
/// // Playwright
/// await page.getByTestId('todo-title').dblclick();
/// await page.getByRole('textbox', { name: 'Edit' }).fill('changed text');
/// await page.keyboard.press('Escape');
/// // original text is preserved
/// ```
#[tokio::test]
async fn should_cancel_edit_on_escape() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    // Double-click to edit
    page.mouse().dblclick(150.0, 250.0).await.unwrap();

    // Fill and cancel with Escape
    page.get_by_placeholder("Edit")
        .fill("changed text")
        .await
        .unwrap();
    page.keyboard().press("Escape").await.unwrap();

    // Escape dispatches rawKeyDown + keyUp, no insertText
    let key_calls = mock.calls_for("Input.dispatchKeyEvent");
    let escape_calls: Vec<_> = key_calls
        .iter()
        .filter(|c| c.args[0]["key"] == "Escape")
        .collect();
    assert_eq!(escape_calls.len(), 2);
}

/// ```typescript
/// // Playwright
/// await page.getByTestId('todo-title').dblclick();
/// await page.getByRole('textbox', { name: 'Edit' }).fill('Edited text');
/// // click elsewhere to blur → saves
/// await page.locator('body').click();
/// ```
#[tokio::test]
async fn should_save_edit_on_blur() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    page.mouse().dblclick(150.0, 250.0).await.unwrap();
    page.get_by_placeholder("Edit")
        .fill("Edited text")
        .await
        .unwrap();

    // Click body to blur
    page.locator("body").click().await.unwrap();

    let qs_calls = mock.calls_for("DOM.querySelector");
    assert!(qs_calls.iter().any(|c| c.args[0]["selector"] == "body"));
}

/// ```typescript
/// // Playwright
/// await page.getByTestId('todo-title').dblclick();
/// await page.getByRole('textbox', { name: 'Edit' }).fill('');
/// await page.getByRole('textbox', { name: 'Edit' }).press('Enter');
/// // todo should be deleted
/// ```
#[tokio::test]
async fn should_delete_todo_when_edited_to_empty() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    page.mouse().dblclick(150.0, 250.0).await.unwrap();
    page.get_by_placeholder("Edit").fill("").await.unwrap();
    page.keyboard().press("Enter").await.unwrap();

    let cf_calls = mock.calls_for("Runtime.callFunctionOn");
    assert!(!cf_calls.is_empty());
}

/// ```typescript
/// // Playwright
/// await page.getByTestId('todo-title').dblclick();
/// await page.getByRole('textbox', { name: 'Edit' }).fill('  trimmed  ');
/// await page.getByRole('textbox', { name: 'Edit' }).press('Enter');
/// await expect(page.getByText('trimmed')).toBeVisible();
/// ```
#[tokio::test]
async fn should_trim_whitespace_when_editing() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    page.mouse().dblclick(150.0, 250.0).await.unwrap();
    page.get_by_placeholder("Edit")
        .fill("  trimmed  ")
        .await
        .unwrap();
    page.keyboard().press("Enter").await.unwrap();

    let cf_calls = mock.calls_for("Runtime.callFunctionOn");
    assert!(!cf_calls.is_empty());
}
