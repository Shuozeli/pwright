//! Locator integration tests against real Chrome.
//!
//! Requires: docker compose -f tests/integration/docker-compose.yml up -d

use pwright_integration_tests::connect_and_navigate;

// ── Content page: queries ──

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn text_content_reads_real_dom() {
    let page = connect_and_navigate("/content.html").await;

    let text = page.locator("#heading").text_content().await.unwrap();
    assert_eq!(text, Some("Hello pwright".to_string()));
}

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn inner_text_reads_real_dom() {
    let page = connect_and_navigate("/content.html").await;

    let text = page.locator(".description").inner_text().await.unwrap();
    assert_eq!(text, "This is a test page for content extraction");
}

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn get_attribute_reads_real_dom() {
    let page = connect_and_navigate("/content.html").await;

    let href = page.locator("#link").get_attribute("href").await.unwrap();
    assert_eq!(href, Some("https://example.com".to_string()));

    let target = page.locator("#link").get_attribute("target").await.unwrap();
    assert_eq!(target, Some("_blank".to_string()));
}

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn count_real_elements() {
    let page = connect_and_navigate("/content.html").await;

    let count = page.locator("#list li").count().await.unwrap();
    assert_eq!(count, 3);
}

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn first_last_nth_real_elements() {
    let page = connect_and_navigate("/content.html").await;

    let first = page
        .locator("#list li")
        .first()
        .text_content()
        .await
        .unwrap();
    assert_eq!(first, Some("Item 1".to_string()));

    let last = page
        .locator("#list li")
        .last()
        .text_content()
        .await
        .unwrap();
    assert_eq!(last, Some("Item 3".to_string()));

    let second = page
        .locator("#list li")
        .nth(1)
        .text_content()
        .await
        .unwrap();
    assert_eq!(second, Some("Item 2".to_string()));
}

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn is_visible_hidden_real_elements() {
    let page = connect_and_navigate("/content.html").await;

    assert!(page.locator("#heading").is_visible().await.unwrap());
    assert!(page.locator("#hidden").is_hidden().await.unwrap());
}

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn data_attributes() {
    let page = connect_and_navigate("/content.html").await;

    let val = page
        .locator("#data")
        .get_attribute("data-value")
        .await
        .unwrap();
    assert_eq!(val, Some("42".to_string()));

    let label = page
        .locator("#data")
        .get_attribute("data-label")
        .await
        .unwrap();
    assert_eq!(label, Some("test".to_string()));
}

// ── Input page: form interactions ──

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn fill_and_read_input() {
    let page = connect_and_navigate("/input.html").await;

    page.locator("#text").fill("hello world").await.unwrap();
    let value = page.locator("#text").input_value().await.unwrap();
    assert_eq!(value, "hello world");
}

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn check_and_verify_checkbox() {
    let page = connect_and_navigate("/input.html").await;

    assert!(!page.locator("#checkbox").is_checked().await.unwrap());
    // Use JS click for reliable checkbox toggling (CDP mouse events
    // don't always toggle checkboxes depending on element position)
    page.locator("#checkbox")
        .evaluate("function() { this.click(); }")
        .await
        .unwrap();
    assert!(page.locator("#checkbox").is_checked().await.unwrap());
}

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn select_option() {
    let page = connect_and_navigate("/input.html").await;

    page.locator("#select").select_option("two").await.unwrap();
    let value = page.locator("#select").input_value().await.unwrap();
    assert_eq!(value, "two");
}

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn get_by_test_id() {
    let page = connect_and_navigate("/input.html").await;

    let btn = page.get_by_test_id("submit-btn");
    let text = btn.text_content().await.unwrap();
    assert_eq!(text, Some("Submit".to_string()));
}

// ── Interactive page: click ──

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn click_updates_dom() {
    let page = connect_and_navigate("/interactive.html").await;

    page.locator("#counter").click().await.unwrap();
    let text = page.locator("#counter").text_content().await.unwrap();
    assert_eq!(text, Some("Clicked 1 times".to_string()));

    page.locator("#counter").click().await.unwrap();
    let text = page.locator("#counter").text_content().await.unwrap();
    assert_eq!(text, Some("Clicked 2 times".to_string()));
}

// ── Todo page: complex interactions ──

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn todo_page_structure() {
    let page = connect_and_navigate("/todo").await;

    let count = page.locator("li.todo").count().await.unwrap();
    assert_eq!(count, 3);

    let footer = page.locator(".todo-count").text_content().await.unwrap();
    assert_eq!(footer, Some("2 items left".to_string()));

    // Second todo should be checked
    let second_toggle = page.locator(".toggle").nth(1);
    assert!(second_toggle.is_checked().await.unwrap());

    // First and third should be unchecked
    assert!(!page.locator(".toggle").first().is_checked().await.unwrap());
    assert!(!page.locator(".toggle").last().is_checked().await.unwrap());
}
