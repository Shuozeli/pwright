//! Integration tests for ChromeHttpClient and HTTP-based tab lifecycle.
//!
//! Verifies that HTTP debug endpoints work correctly against real Chrome:
//! - list targets, create target, close target, version
//! - TabHandle with HttpTabCloser closes tabs via HTTP
//! - Browser::connect_http auto-selects HTTP closer for new_tab
//!
//! Requires: docker compose -f tests/integration/docker-compose.local.yml up -d

use pwright_bridge::Browser;
use pwright_bridge::chrome_http::ChromeHttpClient;
use pwright_integration_tests::chrome_http_url;

// ── ChromeHttpClient direct tests ──

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn http_client_version() {
    let http = ChromeHttpClient::new(&chrome_http_url());
    let version = http.version().await.unwrap();

    assert!(version.get("Browser").is_some(), "missing Browser field");
    assert!(
        version.get("webSocketDebuggerUrl").is_some(),
        "missing webSocketDebuggerUrl field"
    );
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn http_client_list_targets() {
    let http = ChromeHttpClient::new(&chrome_http_url());
    let targets = http.list_targets().await.unwrap();

    // Chrome always has at least one target (the default blank page)
    assert!(!targets.is_empty(), "expected at least one target");
    // All targets should have a non-empty target_id
    for t in &targets {
        assert!(!t.target_id.is_empty(), "target_id should not be empty");
    }
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn http_client_create_and_close_target() {
    let http = ChromeHttpClient::new(&chrome_http_url());

    // Create a tab
    let target = http.create_target("about:blank").await.unwrap();
    assert!(!target.target_id.is_empty());

    // Verify it appears in the list
    let targets = http.list_targets().await.unwrap();
    let found = targets.iter().any(|t| t.target_id == target.target_id);
    assert!(found, "new target should appear in list");

    // Close it
    http.close_target(&target.target_id).await.unwrap();

    // Verify it's gone (may take a moment)
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let targets = http.list_targets().await.unwrap();
    let found = targets.iter().any(|t| t.target_id == target.target_id);
    assert!(!found, "closed target should not appear in list");
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn http_client_close_nonexistent_target() {
    let http = ChromeHttpClient::new(&chrome_http_url());
    // Closing a nonexistent target should fail
    let result = http.close_target("DOES_NOT_EXIST").await;
    assert!(result.is_err(), "closing nonexistent target should fail");
}

// ── Browser::connect_http + TabHandle HTTP close ──

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn connect_http_stores_http_url() {
    let browser = Browser::connect_http(&chrome_http_url()).await.unwrap();
    assert!(browser.http_url().is_some(), "http_url should be set");
    assert!(
        browser.http_client().is_some(),
        "http_client should be available"
    );
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn new_tab_with_http_close() {
    let http = ChromeHttpClient::new(&chrome_http_url());
    let browser = Browser::connect_http(&chrome_http_url()).await.unwrap();

    // Create tab via Browser (WebSocket for create + attach, HTTP for close)
    let handle = browser.new_tab("about:blank").await.unwrap();
    let target_id = handle.target_id().to_string();

    // Verify tab exists
    let targets = http.list_targets().await.unwrap();
    assert!(
        targets.iter().any(|t| t.target_id == target_id),
        "tab should exist after new_tab"
    );

    // Close via TabHandle (should use HTTP internally since we used connect_http)
    handle.close().await.unwrap();

    // Verify tab is gone
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let targets = http.list_targets().await.unwrap();
    assert!(
        !targets.iter().any(|t| t.target_id == target_id),
        "tab should be gone after close"
    );
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn new_tab_page_interaction_then_http_close() {
    let browser = Browser::connect_http(&chrome_http_url()).await.unwrap();

    let handle = browser.new_tab("about:blank").await.unwrap();
    let page = handle.page();

    // DOM interactions go over WebSocket (unchanged)
    page.goto("data:text/html,<h1>Hello</h1>", None)
        .await
        .unwrap();
    let title = page
        .evaluate("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(title["value"].as_str().unwrap_or(""), "Hello");

    // Close goes over HTTP
    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn tab_handle_close_idempotent_with_http() {
    let browser = Browser::connect_http(&chrome_http_url()).await.unwrap();

    let handle = browser.new_tab("about:blank").await.unwrap();

    // First close succeeds
    handle.close().await.unwrap();
    // Second close is a no-op (idempotent)
    handle.close().await.unwrap();
}
