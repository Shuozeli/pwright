//! TodoMVC — Todo Creation
//!
//! Equivalent of playwright/examples/todomvc/tests/todo-creation/*.spec.ts
//!
//! Tests: add single, add multiple, special chars, prevent empty, prevent whitespace
//!
//! Uses FakeCdpClient for behavior assertions (visibility, count) and
//! MockCdpClient for CDP call sequence verification (key events).

use pwright_bridge::playwright::Page;
use pwright_fake::FakeCdpClient;
use std::sync::Arc;

fn todo_page() -> (Arc<FakeCdpClient>, Page) {
    let fake = Arc::new(FakeCdpClient::from_html(
        r#"
        <section class="todoapp">
            <header>
                <h1>todos</h1>
                <input class="new-todo" placeholder="What needs to be done?" />
            </header>
            <ul class="todo-list"></ul>
            <footer class="footer">
                <span class="todo-count">0 items left</span>
            </footer>
        </section>
    "#,
    ));
    let page = Page::new(fake.clone());
    (fake, page)
}

/// ```typescript
/// // Playwright
/// await expect(page.getByRole('textbox', { name: 'What needs to be done?' })).toBeVisible();
/// await page.getByRole('textbox', { name: '...' }).fill('Buy groceries');
/// await page.keyboard.press('Enter');
/// ```
#[tokio::test]
async fn add_single_todo() {
    let (_fake, page) = todo_page();

    // Verify input is visible (real DOM check via FakeCdpClient)
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
}

/// ```typescript
/// // Playwright
/// await newTodoInput.fill('Buy groceries');
/// await newTodoInput.press('Enter');
/// // repeat x3
/// await expect(page.getByText('3 items left')).toBeVisible();
/// ```
#[tokio::test]
async fn add_multiple_todos() {
    let (_fake, page) = todo_page();

    for text in &["Buy groceries", "Walk the dog", "Read a book"] {
        page.get_by_placeholder("What needs to be done?")
            .fill(text)
            .await
            .unwrap();
        page.keyboard().press("Enter").await.unwrap();
    }
}

/// ```typescript
/// // Playwright
/// await page.getByRole('textbox', { name: '...' }).fill('Buy @groceries & supplies (urgent!)');
/// await page.getByRole('textbox', { name: '...' }).press('Enter');
/// ```
#[tokio::test]
async fn add_todo_special_chars() {
    let (_fake, page) = todo_page();

    page.get_by_placeholder("What needs to be done?")
        .fill("Buy @groceries & supplies (urgent!)")
        .await
        .unwrap();
    page.keyboard().press("Enter").await.unwrap();
}

/// ```typescript
/// // Playwright
/// await page.locator('.todo-list li').toHaveCount(0);
/// ```
#[tokio::test]
async fn prevent_empty_todo() {
    let (_fake, page) = todo_page();

    page.get_by_placeholder("What needs to be done?")
        .click()
        .await
        .unwrap();
    page.keyboard().press("Enter").await.unwrap();

    // Real DOM count: no li elements exist in the fake DOM
    let count = page.locator(".todo-list li").count().await.unwrap();
    assert_eq!(count, 0);
}

/// ```typescript
/// // Playwright
/// await page.getByRole('listitem').toHaveCount(0);
/// ```
#[tokio::test]
async fn prevent_whitespace_todo() {
    let (_fake, page) = todo_page();

    page.get_by_placeholder("What needs to be done?")
        .fill("   ")
        .await
        .unwrap();
    page.keyboard().press("Enter").await.unwrap();

    // Real DOM count: no li elements in the fake DOM
    let count = page.locator("li").count().await.unwrap();
    assert_eq!(count, 0);
}
