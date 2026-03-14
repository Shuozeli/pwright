//! Advanced navigation tests: multi-page, back/forward, empty page.
//!
//! Requires: docker compose -f docker-compose.local.yml up -d

use pwright_integration_tests::{connect_and_navigate, server_base_url};

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn navigate_between_pages() {
    let page = connect_and_navigate("/navigation-extras.html").await;

    let title = page.locator("#heading").text_content().await.unwrap();
    assert_eq!(title, Some("Page 1".to_string()));

    // Click link to page 2
    page.locator("#link-page2").click().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let title = page.locator("#heading").text_content().await.unwrap();
    assert_eq!(title, Some("Page 2".to_string()));
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn navigate_back_and_forward() {
    let page = connect_and_navigate("/navigation-extras.html").await;

    // Navigate to page 2
    let base = server_base_url();
    page.goto(&format!("{base}/navigation-extras-page2.html"), None)
        .await
        .unwrap();

    let title = page.locator("#heading").text_content().await.unwrap();
    assert_eq!(title, Some("Page 2".to_string()));

    // Go back
    page.go_back().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let title = page.locator("#heading").text_content().await.unwrap();
    assert_eq!(title, Some("Page 1".to_string()));

    // Go forward
    page.go_forward().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let title = page.locator("#heading").text_content().await.unwrap();
    assert_eq!(title, Some("Page 2".to_string()));
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn empty_page_has_no_elements() {
    let page = connect_and_navigate("/empty.html").await;

    let count = page.locator("div").count().await.unwrap();
    assert_eq!(count, 0);

    let title = page.title().await.unwrap();
    assert_eq!(title, "Empty Page");
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn page_url_changes_after_navigation() {
    let page = connect_and_navigate("/content.html").await;

    let url1 = page.url().await.unwrap();
    assert!(url1.contains("/content.html"));

    let base = server_base_url();
    page.goto(&format!("{base}/input.html"), None)
        .await
        .unwrap();

    let url2 = page.url().await.unwrap();
    assert!(url2.contains("/input.html"));
    assert_ne!(url1, url2);
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn inner_html_returns_markup() {
    let page = connect_and_navigate("/content.html").await;

    let html = page.locator("#nested").inner_html().await.unwrap();
    assert!(html.contains("<b>"), "should contain <b> tag: {html}");
    assert!(html.contains("<i>"), "should contain <i> tag: {html}");
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn evaluate_returns_result() {
    let page = connect_and_navigate("/content.html").await;

    let result = page.evaluate("1 + 2 + 3").await.unwrap();
    assert_eq!(result["value"], 6);
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn body_text_returns_full_page() {
    let page = connect_and_navigate("/content.html").await;

    let text = page.body_text().await.unwrap();
    assert!(text.contains("Hello pwright"), "should contain heading");
    assert!(text.contains("Example Link"), "should contain link text");
}
