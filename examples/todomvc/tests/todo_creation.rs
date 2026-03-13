//! TodoMVC — Todo Creation
//!
//! Equivalent of playwright/examples/todomvc/tests/todo-creation/*.spec.ts
//!
//! Tests: add single, add multiple, special chars, prevent empty, prevent whitespace

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
/// // Playwright — todo-creation/add-single-todo.spec.ts
/// await expect(page.getByRole('textbox', { name: 'What needs to be done?' })).toBeVisible();
/// await page.getByRole('textbox', { name: '...' }).fill('Buy groceries');
/// await expect(page.getByRole('textbox', { name: '...' })).toHaveValue('Buy groceries');
/// await page.keyboard.press('Enter');
/// await expect(page.getByText('Buy groceries')).toBeVisible();
/// await expect(page.getByRole('textbox', { name: '...' })).toHaveValue('');
/// await expect(page.getByText('1 item left')).toBeVisible();
/// await expect(page.getByRole('checkbox', { name: 'Toggle Todo' })).not.toBeChecked();
/// ```
#[tokio::test]
async fn add_single_todo() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    // Verify input is visible
    let visible = page
        .get_by_placeholder("What needs to be done?")
        .is_visible()
        .await
        .unwrap();
    assert!(visible);

    // Fill and submit
    page.get_by_placeholder("What needs to be done?")
        .fill("Buy groceries")
        .await
        .unwrap();
    page.keyboard().press("Enter").await.unwrap();

    let cf_calls = mock.calls_for("Runtime.callFunctionOn");
    assert!(!cf_calls.is_empty());
    let key_calls = mock.calls_for("Input.dispatchKeyEvent");
    assert_eq!(key_calls[0].args[0]["key"], "Enter");
}

/// ```typescript
/// // Playwright — todo-creation/add-multiple-todos.spec.ts
/// await newTodoInput.fill('Buy groceries');
/// await newTodoInput.press('Enter');
/// await newTodoInput.fill('Walk the dog');
/// await newTodoInput.press('Enter');
/// await newTodoInput.fill('Read a book');
/// await newTodoInput.press('Enter');
/// await expect(page.getByText('3 items left')).toBeVisible();
/// ```
#[tokio::test]
async fn add_multiple_todos() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    for text in &["Buy groceries", "Walk the dog", "Read a book"] {
        page.get_by_placeholder("What needs to be done?")
            .fill(text)
            .await
            .unwrap();
        page.keyboard().press("Enter").await.unwrap();
    }

    let cf_calls = mock.calls_for("Runtime.callFunctionOn");
    assert!(cf_calls.len() >= 3);
    let key_calls = mock.calls_for("Input.dispatchKeyEvent");
    assert_eq!(key_calls.len(), 6); // 3 × (rawKeyDown + keyUp)
}

/// ```typescript
/// // Playwright — todo-creation/add-todo-special-chars.spec.ts
/// await page.getByRole('textbox', { name: '...' }).fill('Buy @groceries & supplies (urgent!)');
/// await page.getByRole('textbox', { name: '...' }).press('Enter');
/// await expect(page.getByText('Buy @groceries & supplies (urgent!)')).toBeVisible();
/// ```
#[tokio::test]
async fn add_todo_special_chars() {
    let mock = mock_with_element();
    let page = Page::new(mock.clone());

    page.get_by_placeholder("What needs to be done?")
        .fill("Buy @groceries & supplies (urgent!)")
        .await
        .unwrap();
    page.keyboard().press("Enter").await.unwrap();

    let cf_calls = mock.calls_for("Runtime.callFunctionOn");
    assert!(!cf_calls.is_empty());
}

/// ```typescript
/// // Playwright — todo-creation/prevent-empty-todo.spec.ts
/// await page.getByRole('textbox', { name: '...' }).click();
/// await page.keyboard.press('Enter');
/// await expect(page.locator('.todo-list li')).toHaveCount(0);
/// ```
#[tokio::test]
async fn prevent_empty_todo() {
    let mock = mock_with_element();
    mock.set_query_selector_all_response(vec![]);
    let page = Page::new(mock.clone());

    // Click input, press Enter without typing
    page.get_by_placeholder("What needs to be done?")
        .click()
        .await
        .unwrap();
    page.keyboard().press("Enter").await.unwrap();

    // Verify count is 0
    let count = page.locator(".todo-list li").count().await.unwrap();
    assert_eq!(count, 0);
}

/// ```typescript
/// // Playwright — todo-creation/prevent-whitespace-todo.spec.ts
/// await page.getByRole('textbox', { name: '...' }).fill('   ');
/// await page.getByRole('textbox', { name: '...' }).press('Enter');
/// await expect(page.getByRole('listitem')).toHaveCount(0);
/// ```
#[tokio::test]
async fn prevent_whitespace_todo() {
    let mock = mock_with_element();
    mock.set_query_selector_all_response(vec![]);
    let page = Page::new(mock.clone());

    page.get_by_placeholder("What needs to be done?")
        .fill("   ")
        .await
        .unwrap();
    page.keyboard().press("Enter").await.unwrap();

    // Verify no listitem was created
    let count = page.locator("li").count().await.unwrap();
    assert_eq!(count, 0);
}
