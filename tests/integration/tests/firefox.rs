//! Integration tests for pwright against Firefox (CDP mode).
//!
//! Firefox ESR supports a subset of CDP via `--remote-debugging-port`.
//! These tests verify that pwright can connect, manage tabs, navigate,
//! evaluate JS, and query DOM against Firefox's CDP implementation.
//!
//! Requires: docker compose -f tests/integration/docker-compose.firefox.yml up -d
//!           (or docker-compose.local.yml with FIREFOX_WS_URL set)
//!
//! Key differences from Chrome:
//! - CDP support is a subset (~82 methods vs Chrome's 400+)
//! - DOM domain is NOT implemented (selectors, locators, click all fail)
//! - No HTTP `/json/version` discovery — WS URL from Firefox's stdout
//! - `--remote-debugging-port` binds to 127.0.0.1 only (host network required)
//! - Host header must match exactly (127.0.0.1:PORT) — no port remapping
//!
//! NOTE: Firefox CDP is deprecated (removed in Firefox 141+). This backend
//! tests cross-browser CDP compatibility while the current ESR is supported.

use std::sync::Arc;

use pwright_bridge::playwright::Page;
use pwright_bridge::{Browser, BrowserConfig};
use pwright_cdp::{CdpConnection, CdpSession};
use pwright_integration_tests::firefox_ws_url;

/// Connect to Firefox via Browser::connect and create a page navigated to a URL.
async fn firefox_page(url: &str) -> (Arc<Browser>, pwright_bridge::TabHandle, Page) {
    let browser = Browser::connect(BrowserConfig {
        cdp_url: firefox_ws_url(),
        ..Default::default()
    })
    .await
    .unwrap();

    let handle = browser.new_tab("about:blank").await.unwrap();
    let page = handle.page();

    if url != "about:blank" {
        page.goto(url, None).await.unwrap();
        // Firefox CDP navigation can be slower than Chrome
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    (browser, handle, page)
}

// ── Connection ──

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn connect_via_ws_url() {
    // Arrange — Firefox uses direct WS connection (no HTTP discovery)
    let ws_url = firefox_ws_url();

    // Act
    let browser = Browser::connect(BrowserConfig {
        cdp_url: ws_url,
        ..Default::default()
    })
    .await
    .unwrap();

    // Assert
    let handle = browser.new_tab("about:blank").await.unwrap();
    assert!(!handle.target_id().is_empty());
    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn tab_close_idempotent() {
    // Arrange
    let browser = Browser::connect(BrowserConfig {
        cdp_url: firefox_ws_url(),
        ..Default::default()
    })
    .await
    .unwrap();

    // Act + Assert — no panic on double close
    let handle = browser.new_tab("about:blank").await.unwrap();
    handle.close().await.unwrap();
    handle.close().await.unwrap(); // second close is no-op
}

// ── Navigation ──

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn navigate_and_get_title() {
    // Arrange + Act
    let (_browser, handle, page) = firefox_page("https://example.com").await;

    // Assert
    let title = page.title().await.unwrap();
    assert_eq!(title, "Example Domain");

    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn navigate_and_get_url() {
    // Arrange + Act
    let (_browser, handle, page) = firefox_page("https://example.com").await;

    // Assert
    let url = page.url().await.unwrap();
    assert!(url.contains("example.com"), "URL: {url}");

    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn navigate_and_get_content() {
    // Arrange + Act
    let (_browser, handle, page) = firefox_page("https://example.com").await;

    // Assert
    let html = page.content().await.unwrap();
    assert!(html.contains("Example Domain"), "Content: {html}");
    assert!(html.contains("<html"), "Should contain HTML tag");

    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn navigate_and_get_body_text() {
    // Arrange + Act
    let (_browser, handle, page) = firefox_page("https://example.com").await;

    // Assert
    let text = page.body_text().await.unwrap();
    assert!(text.contains("Example Domain"), "body text: {text}");

    handle.close().await.unwrap();
}

// ── JavaScript evaluation ──

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn evaluate_simple_expression() {
    // Arrange
    let (_browser, handle, page) = firefox_page("https://example.com").await;

    // Act
    let result: i64 = page.evaluate_into("1 + 1").await.unwrap();

    // Assert
    assert_eq!(result, 2);

    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn evaluate_string() {
    // Arrange
    let (_browser, handle, page) = firefox_page("https://example.com").await;

    // Act
    let result = page.evaluate("'hello' + ' ' + 'world'").await.unwrap();

    // Assert
    assert_eq!(result["value"], "hello world");

    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn evaluate_document_title() {
    // Arrange
    let (_browser, handle, page) = firefox_page("https://example.com").await;

    // Act
    let title: String = page.evaluate_into("document.title").await.unwrap();

    // Assert
    assert_eq!(title, "Example Domain");

    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn evaluate_returns_object() {
    // Arrange
    let (_browser, handle, page) = firefox_page("https://example.com").await;

    // Act
    let result = page.evaluate("({a: 1, b: 'two'})").await.unwrap();

    // Assert
    assert_eq!(result["value"]["a"], 1);
    assert_eq!(result["value"]["b"], "two");

    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn evaluate_returns_array() {
    // Arrange
    let (_browser, handle, page) = firefox_page("https://example.com").await;

    // Act
    let result = page.evaluate("[1, 2, 3]").await.unwrap();

    // Assert
    let arr = result["value"].as_array().unwrap();
    assert_eq!(arr.len(), 3);

    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn evaluate_dom_element_count() {
    // Arrange
    let (_browser, handle, page) = firefox_page("https://example.com").await;

    // Act
    let count: i64 = page
        .evaluate_into("document.querySelectorAll('p').length")
        .await
        .unwrap();

    // Assert
    assert!(count >= 1, "expected at least 1 <p>, got {count}");

    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn evaluate_dom_create_element() {
    // Arrange
    let (_browser, handle, page) = firefox_page("https://example.com").await;

    // Act
    page.evaluate(
        "const el = document.createElement('div'); el.id = 'created'; \
         el.textContent = 'hello from firefox'; document.body.appendChild(el); true",
    )
    .await
    .unwrap();
    let text: String = page
        .evaluate_into("document.getElementById('created').textContent")
        .await
        .unwrap();

    // Assert
    assert_eq!(text, "hello from firefox");

    handle.close().await.unwrap();
}

// ── Page selectors (DOM domain — NOT supported by Firefox CDP) ──
//
// Firefox's CDP implementation does not include the DOM domain.
// These tests verify the expected error rather than asserting success.

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn text_content_by_selector_unsupported() {
    // Arrange
    let (_browser, handle, page) = firefox_page("https://example.com").await;

    // Act
    let result = page.text_content("h1").await;

    // Assert — Firefox CDP returns error for DOM.getDocument
    let err = result.unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("DOM.getDocument"),
        "expected DOM.getDocument error, got: {msg}"
    );

    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn inner_html_by_selector_unsupported() {
    // Arrange
    let (_browser, handle, page) = firefox_page("https://example.com").await;

    // Act
    let result = page.inner_html("div").await;

    // Assert
    let err = result.unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("DOM.getDocument"),
        "expected DOM.getDocument error, got: {msg}"
    );

    handle.close().await.unwrap();
}

// ── Locator API (depends on DOM domain — NOT supported) ──

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn locator_text_content_unsupported() {
    // Arrange
    let (_browser, handle, page) = firefox_page("https://example.com").await;

    // Act
    let result = page.locator("h1").text_content().await;

    // Assert
    let err = result.unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("DOM.getDocument"),
        "expected DOM.getDocument error, got: {msg}"
    );

    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn locator_count_unsupported() {
    // Arrange
    let (_browser, handle, page) = firefox_page("https://example.com").await;

    // Act
    let result = page.locator("p").count().await;

    // Assert
    let err = result.unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("DOM.getDocument"),
        "expected DOM.getDocument error, got: {msg}"
    );

    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn locator_is_visible_unsupported() {
    // Arrange
    let (_browser, handle, page) = firefox_page("https://example.com").await;

    // Act
    let result = page.is_visible("h1").await;

    // Assert
    let err = result.unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("DOM.getDocument"),
        "expected DOM.getDocument error, got: {msg}"
    );

    handle.close().await.unwrap();
}

// ── Click / Input ──

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn click_by_selector_unsupported() {
    // Arrange — Firefox CDP lacks DOM.getDocument needed for selector-based click
    let (_browser, handle, page) = firefox_page("https://example.com").await;
    page.evaluate(
        "const btn = document.createElement('button'); btn.id = 'btn'; btn.textContent = '0'; \
         btn.onclick = function() { this.textContent = String(Number(this.textContent) + 1); }; \
         document.body.appendChild(btn); true",
    )
    .await
    .unwrap();

    // Act
    let result = page.click("#btn").await;

    // Assert — click uses DOM.getDocument which Firefox doesn't support
    let err = result.unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("DOM.getDocument"),
        "expected DOM.getDocument error, got: {msg}"
    );

    handle.close().await.unwrap();
}

// ── Tab lifecycle ──

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn multiple_tabs() {
    // Arrange
    let browser = Browser::connect(BrowserConfig {
        cdp_url: firefox_ws_url(),
        ..Default::default()
    })
    .await
    .unwrap();

    // Act
    let tab1 = browser.new_tab("about:blank").await.unwrap();
    let tab2 = browser.new_tab("about:blank").await.unwrap();

    let page1 = tab1.page();
    let page2 = tab2.page();

    page1.goto("https://example.com", None).await.unwrap();
    page2.goto("https://example.com", None).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Assert
    let title1: String = page1.evaluate_into("document.title").await.unwrap();
    let title2: String = page2.evaluate_into("document.title").await.unwrap();
    assert_eq!(title1, "Example Domain");
    assert_eq!(title2, "Example Domain");

    tab1.close().await.unwrap();
    // Tab2 should still work after tab1 closes
    let title2_recheck: String = page2.evaluate_into("document.title").await.unwrap();
    assert_eq!(title2_recheck, "Example Domain");
    tab2.close().await.unwrap();
}

// ── Raw CDP session ──

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn raw_cdp_dom_get_document_unsupported() {
    // Arrange — Firefox CDP does not implement DOM domain
    let ws_url = firefox_ws_url();

    let conn = CdpConnection::connect(&ws_url).await.unwrap();
    let browser_session = CdpSession::browser(conn.clone());

    let target_id = browser_session.target_create("about:blank").await.unwrap();
    let session_id = browser_session.target_attach(&target_id).await.unwrap();
    let session = Arc::new(CdpSession::new(conn.clone(), session_id, target_id.clone()));

    session.page_enable().await.unwrap();
    session.runtime_enable().await.unwrap();

    let page = Page::with_tab(session.clone(), target_id.clone());
    page.goto("https://example.com", None).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Act
    let result = session.dom_get_document().await;

    // Assert — DOM.getDocument is not supported
    assert!(
        result.is_err(),
        "expected DOM.getDocument to fail on Firefox"
    );
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("DOM.getDocument"), "error: {msg}");

    browser_session.target_close(&target_id).await.unwrap();
}

// ── Screenshot ──

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn capture_screenshot() {
    // Arrange
    let (_browser, handle, page) = firefox_page("https://example.com").await;

    // Act
    let base64_png = page.screenshot(None).await.unwrap();

    // Assert
    assert!(!base64_png.is_empty(), "screenshot should not be empty");
    assert!(
        base64_png.len() > 100,
        "screenshot too small to be a real PNG"
    );

    handle.close().await.unwrap();
}

// ── Navigation between pages ──

#[tokio::test]
#[ignore = "requires docker: firefox"]
async fn navigate_between_pages() {
    // Arrange
    let (_browser, handle, page) = firefox_page("https://example.com").await;
    let title1 = page.title().await.unwrap();
    assert_eq!(title1, "Example Domain");

    // Act
    page.goto("https://www.iana.org/help/example-domains", None)
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Assert
    let url = page.url().await.unwrap();
    assert!(url.contains("iana.org"), "URL after nav: {url}");

    handle.close().await.unwrap();
}
