//! Navigation integration tests against real Chrome.
//!
//! Requires: docker compose -f tests/integration/docker-compose.yml up -d

use pwright_integration_tests::connect_and_navigate;

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn goto_and_read_title() {
    let page = connect_and_navigate("/content.html").await;
    let title = page.title().await.unwrap();
    assert_eq!(title, "Content Page");
}

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn goto_and_read_url() {
    let page = connect_and_navigate("/content.html").await;
    let url = page.url().await.unwrap();
    assert!(url.contains("/content.html"), "URL: {url}");
}

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn goto_and_read_content() {
    let page = connect_and_navigate("/content.html").await;
    let html = page.content().await.unwrap();
    assert!(html.contains("Hello pwright"), "Content: {html}");
}

// TODO: reload() needs to wait for page load (like goto does).
// Currently Page.reload() fires CDP Page.reload and returns immediately,
// but the JS execution context resets on reload. Subsequent Runtime.evaluate
// calls may hit the old context and return empty results.
