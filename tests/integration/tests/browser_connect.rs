//! Integration tests for browser connection and tab lifecycle.
//!
//! Verifies that `Browser::connect()` works with both HTTP and WS URLs,
//! and that tab close always uses CDP WebSocket (`Target.closeTarget`).
//!
//! Requires: docker compose -f tests/integration/docker-compose.local.yml up -d

use pwright_bridge::{Browser, BrowserConfig};
use pwright_integration_tests::chrome_http_url;

// ── Browser::connect with HTTP URL (discovery + rewrite) ──

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn connect_with_http_url() {
    let browser = Browser::connect(BrowserConfig {
        cdp_url: chrome_http_url(),
        ..Default::default()
    })
    .await
    .unwrap();

    let handle = browser.new_tab("about:blank").await.unwrap();
    assert!(!handle.target_id().is_empty());
    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn connect_new_tab_page_interaction() {
    let browser = Browser::connect(BrowserConfig {
        cdp_url: chrome_http_url(),
        ..Default::default()
    })
    .await
    .unwrap();

    let handle = browser.new_tab("about:blank").await.unwrap();
    let page = handle.page();

    page.goto("data:text/html,<h1>Hello</h1>", None)
        .await
        .unwrap();
    let title: String = page
        .evaluate_sync_into("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(title, "Hello");

    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn tab_handle_close_idempotent() {
    let browser = Browser::connect(BrowserConfig {
        cdp_url: chrome_http_url(),
        ..Default::default()
    })
    .await
    .unwrap();

    let handle = browser.new_tab("about:blank").await.unwrap();
    handle.close().await.unwrap();
    // Second close is a no-op (idempotent)
    handle.close().await.unwrap();
}

// ── Browser::connect with WS URL (direct) ──

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn connect_with_ws_url() {
    // Discover WS URL manually, then connect directly
    let http_url = chrome_http_url();
    let version_url = format!("{http_url}/json/version");
    let resp: serde_json::Value = reqwest::get(&version_url)
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let raw_ws = resp["webSocketDebuggerUrl"].as_str().unwrap();
    let ws_url = pwright_bridge::rewrite_ws_url(&http_url, raw_ws).unwrap();

    let browser = Browser::connect(BrowserConfig {
        cdp_url: ws_url,
        ..Default::default()
    })
    .await
    .unwrap();

    let handle = browser.new_tab("about:blank").await.unwrap();
    assert!(!handle.target_id().is_empty());
    handle.close().await.unwrap();
}
