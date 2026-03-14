//! Click navigation tests: verify clicking <a> tags triggers navigation.
//!
//! Requires: docker compose -f docker-compose.local.yml up -d

use pwright_integration_tests::connect_and_navigate;

/// Click an <a> tag and verify the URL changes (page navigates).
/// This was a critical bug: click dispatched mouse events but didn't
/// trigger the browser's default action (navigation).
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn click_link_navigates() {
    let page = connect_and_navigate("/navigation-extras.html").await;

    let url_before = page.url().await.unwrap();
    assert!(url_before.contains("/navigation-extras.html"));

    // Click the <a> link to page 2
    page.locator("#link-page2").click().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let url_after = page.url().await.unwrap();
    assert!(
        url_after.contains("navigation-extras-page2"),
        "URL should have changed after clicking link. Before: {url_before}, After: {url_after}"
    );
}

/// Click a button and verify DOM updates (regression test).
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn click_button_still_works() {
    let page = connect_and_navigate("/interactive.html").await;

    page.locator("#counter").click().await.unwrap();
    let text = page.locator("#counter").text_content().await.unwrap();
    assert_eq!(text, Some("Clicked 1 times".to_string()));
}

/// Click a link, then verify we can interact with the new page.
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn click_navigate_then_interact() {
    let page = connect_and_navigate("/navigation-extras.html").await;

    // Verify we're on page 1
    let heading = page.locator("#heading").text_content().await.unwrap();
    assert_eq!(heading, Some("Page 1".to_string()));

    // Click to page 2
    page.locator("#link-page2").click().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Verify we can read page 2 content
    let heading = page.locator("#heading").text_content().await.unwrap();
    assert_eq!(heading, Some("Page 2".to_string()));

    // Click back to page 1
    page.locator("#link-page1").click().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let heading = page.locator("#heading").text_content().await.unwrap();
    assert_eq!(heading, Some("Page 1".to_string()));
}
