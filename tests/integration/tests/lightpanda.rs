//! Integration tests for pwright against Lightpanda browser.
//!
//! Lightpanda is a headless browser written in Zig that implements CDP.
//! These tests verify that pwright can connect, manage tabs, navigate,
//! evaluate JS, query DOM, and take snapshots against Lightpanda.
//!
//! Requires: docker compose -f tests/integration/docker-compose.local.yml up -d
//!
//! Key differences from Chrome:
//! - No HTTP debug endpoints (only /json/version) — connect via ws:// directly
//! - 1 browser context per WS connection, 1 tab per context
//! - Input events partially implemented (mousePressed/keyDown only)

use std::sync::Arc;

use pwright_bridge::playwright::Page;
use pwright_bridge::{Browser, BrowserConfig};
use pwright_cdp::{CdpConnection, CdpSession};
use pwright_integration_tests::lightpanda_ws_url;

/// Connect to Lightpanda via Browser::connect and create a page navigated to a URL.
async fn lightpanda_page(url: &str) -> (Arc<Browser>, pwright_bridge::TabHandle, Page) {
    let browser = Browser::connect(BrowserConfig {
        cdp_url: lightpanda_ws_url(),
        ..Default::default()
    })
    .await
    .unwrap();

    let handle = browser.new_tab("about:blank").await.unwrap();
    let page = handle.page();

    if url != "about:blank" {
        page.goto(url, None).await.unwrap();
        // Wait for page to settle
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    (browser, handle, page)
}

// ── Connection ──

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn connect_via_ws() {
    let browser = Browser::connect(BrowserConfig {
        cdp_url: lightpanda_ws_url(),
        ..Default::default()
    })
    .await
    .unwrap();

    let handle = browser.new_tab("about:blank").await.unwrap();
    assert!(!handle.target_id().is_empty());
    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn tab_close_idempotent() {
    let browser = Browser::connect(BrowserConfig {
        cdp_url: lightpanda_ws_url(),
        ..Default::default()
    })
    .await
    .unwrap();

    let handle = browser.new_tab("about:blank").await.unwrap();
    handle.close().await.unwrap();
    handle.close().await.unwrap(); // second close is no-op
}

// ── Navigation ──

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn navigate_and_get_title() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;

    let title = page.title().await.unwrap();
    assert_eq!(title, "Example Domain");

    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn navigate_and_get_content() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;

    let html = page.content().await.unwrap();
    assert!(html.contains("Example Domain"));
    assert!(html.contains("<html"));

    handle.close().await.unwrap();
}

// ── JavaScript evaluation ──

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn evaluate_simple_expression() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;

    let result: i64 = page.evaluate_into("1 + 1").await.unwrap();
    assert_eq!(result, 2);

    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn evaluate_document_title() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;

    let title: String = page.evaluate_into("document.title").await.unwrap();
    assert_eq!(title, "Example Domain");

    handle.close().await.unwrap();
}

// ── DOM ──

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn dom_get_document() {
    let ws_url = lightpanda_ws_url();
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

    let doc = session.dom_get_document().await.unwrap();
    let root_node_id = doc["root"]["nodeId"].as_i64().unwrap();
    assert!(root_node_id > 0);

    browser_session.target_close(&target_id).await.unwrap();
}

// ── Accessibility ──

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn accessibility_tree() {
    let ws_url = lightpanda_ws_url();
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

    // Use raw CDP send — Lightpanda returns nodeId as integer while
    // pwright's RawAXNode expects string (Chrome quirk vs Lightpanda quirk).
    let result = session
        .send(
            "Accessibility.getFullAXTree",
            serde_json::json!({"depth": -1}),
        )
        .await
        .unwrap();
    let nodes = result["nodes"].as_array().unwrap();
    assert!(!nodes.is_empty(), "AX tree should have nodes");

    // Root should be RootWebArea
    let root_role = nodes[0]["role"]["value"].as_str().unwrap();
    assert_eq!(root_role, "RootWebArea");

    browser_session.target_close(&target_id).await.unwrap();
}

// ── Screenshot ──

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn capture_screenshot() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;

    let base64_png = page.screenshot(None).await.unwrap();
    assert!(!base64_png.is_empty(), "screenshot should not be empty");
    // Lightpanda returns base64-encoded PNG; just verify it's non-trivial
    assert!(
        base64_png.len() > 100,
        "screenshot too small to be a real PNG"
    );

    handle.close().await.unwrap();
}

// ── Cookies ──

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn set_and_get_cookies() {
    let ws_url = lightpanda_ws_url();
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

    // Set a cookie
    let set_result = session
        .send(
            "Network.setCookie",
            serde_json::json!({
                "name": "test_cookie",
                "value": "hello",
                "domain": "example.com"
            }),
        )
        .await
        .unwrap();
    assert_eq!(set_result["success"], true);

    // Get cookies
    let cookies = session
        .send("Network.getCookies", serde_json::json!({}))
        .await
        .unwrap();
    let cookie_list = cookies["cookies"].as_array().unwrap();
    let found = cookie_list
        .iter()
        .any(|c| c["name"].as_str() == Some("test_cookie"));
    assert!(found, "should find the cookie we just set");

    browser_session.target_close(&target_id).await.unwrap();
}

// ── Multiple connections (simulates multi-tab) ──

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn multiple_connections_for_parallel_tabs() {
    // Lightpanda allows only 1 tab per connection, so parallel tabs
    // require separate connections.
    let ws_url = lightpanda_ws_url();

    let browser1 = Browser::connect(BrowserConfig {
        cdp_url: ws_url.clone(),
        ..Default::default()
    })
    .await
    .unwrap();

    let browser2 = Browser::connect(BrowserConfig {
        cdp_url: ws_url,
        ..Default::default()
    })
    .await
    .unwrap();

    let tab1 = browser1.new_tab("about:blank").await.unwrap();
    let tab2 = browser2.new_tab("about:blank").await.unwrap();

    // Both tabs should be usable independently.
    // Note: Lightpanda target IDs are per-connection (both may be FID-0000000001),
    // so we verify both tabs work rather than comparing IDs.
    let page1 = tab1.page();
    let page2 = tab2.page();

    page1.goto("https://example.com", None).await.unwrap();
    page2.goto("https://example.com", None).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let title1: String = page1.evaluate_into("document.title").await.unwrap();
    let title2: String = page2.evaluate_into("document.title").await.unwrap();
    assert_eq!(title1, "Example Domain");
    assert_eq!(title2, "Example Domain");

    tab1.close().await.unwrap();
    tab2.close().await.unwrap();
}

// ═══════════════════════════════════════════════════════════════════
// Extended tests — mirrors Chrome integration test patterns
//
// NOTE: Lightpanda does NOT support data: URLs (returns UrlMalformat).
// All tests use https://example.com which has:
//   <h1>Example Domain</h1>, <p>..., <a href="https://iana.org/...">,
//   no forms/inputs (so form tests use JS-created elements).
// ═══════════════════════════════════════════════════════════════════

// ── Navigation (extended) ──

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn navigate_read_url() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;
    let url = page.url().await.unwrap();
    assert!(url.contains("example.com"), "URL: {url}");
    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn navigate_body_text() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;
    let text = page.body_text().await.unwrap();
    assert!(text.contains("Example Domain"), "body text: {text}");
    handle.close().await.unwrap();
}

// ── JS Evaluation (extended — mirrors evaluate.rs) ──

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn eval_returns_string() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;
    let result = page.evaluate("'hello' + ' ' + 'world'").await.unwrap();
    assert_eq!(result["value"], "hello world");
    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn eval_returns_boolean() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;
    let result = page.evaluate("3 > 2").await.unwrap();
    assert_eq!(result["value"], true);
    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn eval_returns_null() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;
    let result = page.evaluate("null").await.unwrap();
    assert!(result["value"].is_null());
    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn eval_returns_object() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;
    let result = page.evaluate("({a: 1, b: 'two'})").await.unwrap();
    assert_eq!(result["value"]["a"], 1);
    assert_eq!(result["value"]["b"], "two");
    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn eval_returns_array() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;
    let result = page.evaluate("[1, 2, 3]").await.unwrap();
    let arr = result["value"].as_array().unwrap();
    assert_eq!(arr.len(), 3);
    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn eval_dom_element_count() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;
    // example.com has exactly 1 <p> with the "More information..." link inside
    let count: i64 = page
        .evaluate_into("document.querySelectorAll('p').length")
        .await
        .unwrap();
    assert!(count >= 1, "expected at least 1 <p>, got {count}");
    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn eval_dom_create_element() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;
    page.evaluate(
        "const el = document.createElement('div'); el.id = 'new'; el.textContent = 'created'; document.body.appendChild(el); true",
    )
    .await
    .unwrap();
    let text: String = page
        .evaluate_into("document.getElementById('new').textContent")
        .await
        .unwrap();
    assert_eq!(text, "created");
    handle.close().await.unwrap();
}

// ── Page selectors ──

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn text_content_by_selector() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;
    let text = page.text_content("h1").await.unwrap();
    assert_eq!(text, Some("Example Domain".to_string()));
    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn inner_html_by_selector() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;
    let html = page.inner_html("div").await.unwrap();
    assert!(html.contains("<h1>"), "inner_html: {html}");
    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn get_attribute() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;
    // Lightpanda doesn't implement DOM.describeNode (used by pwright's get_attribute).
    // Verify the error is clear rather than asserting success.
    let result = page.get_attribute("a", "href").await;
    if let Err(e) = &result {
        let msg = format!("{e}");
        assert!(
            msg.contains("UnknownMethod"),
            "expected UnknownMethod, got: {msg}"
        );
    } else {
        let href = result.unwrap();
        assert!(href.is_some() && href.as_ref().unwrap().contains("iana.org"));
    }
    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn is_visible_check() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;
    let visible = page.is_visible("h1").await.unwrap();
    assert!(visible);
    handle.close().await.unwrap();
}

// ── Input / Form interactions (via JS-created elements) ──

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn fill_input() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;
    // Create an input element via JS since example.com has no forms
    page.evaluate(
        "const inp = document.createElement('input'); inp.id = 'test-input'; document.body.appendChild(inp); true",
    )
    .await
    .unwrap();
    // pwright's fill() uses DOM.describeNode which Lightpanda doesn't implement.
    // Verify it fails cleanly or succeeds if Lightpanda adds support.
    let result = page.fill("#test-input", "Alice").await;
    if let Err(e) = &result {
        let msg = format!("{e}");
        assert!(
            msg.contains("UnknownMethod"),
            "expected UnknownMethod, got: {msg}"
        );
    } else {
        let val = page.input_value("#test-input").await.unwrap();
        assert_eq!(val, "Alice");
    }
    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn click_button_via_js() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;
    // Create a button with onclick handler
    page.evaluate(
        "const btn = document.createElement('button'); btn.id = 'btn'; btn.textContent = '0'; \
         btn.onclick = function() { this.textContent = String(Number(this.textContent) + 1); }; \
         document.body.appendChild(btn); true",
    )
    .await
    .unwrap();
    page.click("#btn").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let text: String = page
        .evaluate_into("document.getElementById('btn').textContent")
        .await
        .unwrap();
    assert_eq!(text, "1");
    handle.close().await.unwrap();
}

// ── Locator API ──

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn locator_text_content() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;
    let text = page.locator("h1").text_content().await.unwrap();
    assert_eq!(text, Some("Example Domain".to_string()));
    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn locator_count() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;
    let count = page.locator("p").count().await.unwrap();
    assert!(count >= 1, "expected at least 1 <p>, got {count}");
    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn locator_get_attribute() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;
    // Uses DOM.describeNode which Lightpanda doesn't implement yet.
    let result = page.locator("a").get_attribute("href").await;
    if let Err(e) = &result {
        let msg = format!("{e}");
        assert!(
            msg.contains("UnknownMethod"),
            "expected UnknownMethod, got: {msg}"
        );
    } else {
        let attr = result.unwrap();
        assert!(attr.is_some() && attr.as_ref().unwrap().contains("iana.org"));
    }
    handle.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn locator_click() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;
    // Create a clickable element
    page.evaluate(
        "const d = document.createElement('div'); d.id = 'click-target'; d.textContent = 'before'; \
         d.onclick = function() { this.textContent = 'after'; }; \
         document.body.appendChild(d); true",
    )
    .await
    .unwrap();
    page.locator("#click-target").click().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let text = page.locator("#click-target").text_content().await.unwrap();
    assert_eq!(text, Some("after".to_string()));
    handle.close().await.unwrap();
}

// ── Navigation advanced ──

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn navigate_between_pages() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;

    let title1 = page.title().await.unwrap();
    assert_eq!(title1, "Example Domain");

    // Navigate to a different page
    page.goto("https://www.iana.org/help/example-domains", None)
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let url = page.url().await.unwrap();
    assert!(url.contains("iana.org"), "URL after nav: {url}");

    handle.close().await.unwrap();
}

// ── Click navigation (mirrors click_navigation.rs) ──

#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn click_link_navigates() {
    let (_browser, handle, page) = lightpanda_page("https://example.com").await;

    let url_before = page.url().await.unwrap();
    assert!(url_before.contains("example.com"));

    // Click the "More information..." link
    page.click("a").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let url_after = page.url().await.unwrap();
    assert!(
        url_after != url_before,
        "URL should change after click. Before: {url_before}, After: {url_after}"
    );

    handle.close().await.unwrap();
}

// ── Tab lifecycle ──

/// Lightpanda: open multiple tabs (separate connections), close sequentially.
#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn tab_lifecycle_multiple_tabs_close_sequentially() {
    let (_browser1, tab1, page1) = lightpanda_page("https://example.com").await;
    let (_browser2, tab2, page2) = lightpanda_page("https://example.com").await;
    let (_browser3, tab3, page3) = lightpanda_page("https://example.com").await;

    // Verify each page loaded
    let text1: String = page1
        .evaluate_into("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(text1, "Example Domain");

    let text2: String = page2
        .evaluate_into("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(text2, "Example Domain");

    let text3: String = page3
        .evaluate_into("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(text3, "Example Domain");

    // Close tabs one by one, verifying remaining tabs still work
    tab1.close().await.unwrap();
    let text2_recheck: String = page2
        .evaluate_into("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(text2_recheck, "Example Domain");

    tab2.close().await.unwrap();
    let text3_recheck: String = page3
        .evaluate_into("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(text3_recheck, "Example Domain");

    tab3.close().await.unwrap();
}

/// Lightpanda: navigate to google.com, verify page title.
#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn tab_lifecycle_navigate_to_google_com() {
    let (_browser, tab, page) = lightpanda_page("https://www.google.com").await;

    let title = page.title().await.unwrap();
    assert!(
        title.to_lowercase().contains("google"),
        "expected title to contain 'google', got: {title}"
    );

    tab.close().await.unwrap();
}

/// Lightpanda: open multiple tabs, close in mixed order.
#[tokio::test]
#[ignore = "requires docker: lightpanda"]
async fn tab_lifecycle_multiple_tabs_close_mixed_order() {
    let (_browser1, tab1, page1) = lightpanda_page("https://example.com").await;
    let (_browser2, tab2, page2) = lightpanda_page("https://example.com").await;
    let (_browser3, tab3, page3) = lightpanda_page("https://example.com").await;

    // Verify all tabs loaded
    let text1: String = page1
        .evaluate_into("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(text1, "Example Domain");

    let text2: String = page2
        .evaluate_into("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(text2, "Example Domain");

    let text3: String = page3
        .evaluate_into("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(text3, "Example Domain");

    // Close in mixed order: middle, last, first
    tab2.close().await.unwrap();

    // Tab1 and Tab3 should still be accessible
    let text1_recheck: String = page1
        .evaluate_into("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(text1_recheck, "Example Domain");

    tab3.close().await.unwrap();

    let text1_final: String = page1
        .evaluate_into("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(text1_final, "Example Domain");

    tab1.close().await.unwrap();
}
