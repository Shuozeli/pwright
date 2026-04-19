//! Tab lifecycle integration tests — open, navigate, close tabs one by one.
//!
//! Verifies:
//! - Multiple tabs can be opened and navigated independently
//! - google.com loads correctly in one tab
//! - Tabs can be closed individually and in sequence
//! - Closing a tab does not affect other open tabs
//!
//! Requires: docker compose -f tests/integration/docker-compose.local.yml up -d

use pwright_bridge::{Browser, BrowserConfig};
use pwright_integration_tests::chrome_http_url;

// ── Chrome tests ──

/// Open multiple tabs, navigate each to a different URL, then close them one by one.
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn open_multiple_tabs_and_close_sequentially() {
    let browser = Browser::connect(BrowserConfig {
        cdp_url: chrome_http_url(),
        ..Default::default()
    })
    .await
    .unwrap();

    // Open 3 tabs
    let tab1 = browser.new_tab("about:blank").await.unwrap();
    let tab2 = browser.new_tab("about:blank").await.unwrap();
    let tab3 = browser.new_tab("about:blank").await.unwrap();

    // Navigate each tab to a different URL
    let page1 = tab1.page();
    page1
        .goto("data:text/html,<h1>Tab 1</h1>", None)
        .await
        .unwrap();

    let page2 = tab2.page();
    page2
        .goto("data:text/html,<h1>Tab 2</h1>", None)
        .await
        .unwrap();

    let page3 = tab3.page();
    page3
        .goto("data:text/html,<h1>Tab 3</h1>", None)
        .await
        .unwrap();

    // Verify each tab has its own content
    let text1: String = page1
        .evaluate_into("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(text1, "Tab 1");

    let text2: String = page2
        .evaluate_into("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(text2, "Tab 2");

    let text3: String = page3
        .evaluate_into("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(text3, "Tab 3");

    // Close tabs one by one, verifying others remain open
    tab1.close().await.unwrap();
    assert!(
        tab1.close().await.is_ok(),
        "second close should be idempotent"
    );

    // Tab 2 and Tab 3 should still work
    let text2_recheck: String = page2
        .evaluate_into("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(text2_recheck, "Tab 2");

    let text3_recheck: String = page3
        .evaluate_into("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(text3_recheck, "Tab 3");

    tab2.close().await.unwrap();

    // Tab 3 should still work
    let text3_final: String = page3
        .evaluate_into("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(text3_final, "Tab 3");

    tab3.close().await.unwrap();
}

/// Open a tab and navigate to google.com, verify the page loads.
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn navigate_to_google_com() {
    let browser = Browser::connect(BrowserConfig {
        cdp_url: chrome_http_url(),
        ..Default::default()
    })
    .await
    .unwrap();

    let tab = browser.new_tab("about:blank").await.unwrap();
    let page = tab.page();

    // Wait for navigation to complete
    let opts = pwright_bridge::playwright::GotoOptions {
        wait_until: pwright_bridge::playwright::WaitStrategy::NetworkIdle,
        timeout_ms: Some(30_000),
    };
    page.goto("https://www.google.com", Some(opts))
        .await
        .unwrap();

    // Verify the page title contains "Google"
    let title = page.title().await.unwrap();
    assert!(
        title.to_lowercase().contains("google"),
        "expected title to contain 'google', got: {title}"
    );

    tab.close().await.unwrap();
}

/// Open multiple tabs including google.com, then close them in mixed order.
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn multiple_tabs_with_google_close_mixed_order() {
    let browser = Browser::connect(BrowserConfig {
        cdp_url: chrome_http_url(),
        ..Default::default()
    })
    .await
    .unwrap();

    // Open 3 tabs
    let tab1 = browser.new_tab("about:blank").await.unwrap();
    let tab2 = browser.new_tab("about:blank").await.unwrap();
    let tab3 = browser.new_tab("about:blank").await.unwrap();

    let page1 = tab1.page();
    page1
        .goto("data:text/html,<h1>Local Tab</h1>", None)
        .await
        .unwrap();

    // Navigate tab2 to google with network idle wait
    let page2 = tab2.page();
    let opts = pwright_bridge::playwright::GotoOptions {
        wait_until: pwright_bridge::playwright::WaitStrategy::NetworkIdle,
        timeout_ms: Some(30_000),
    };
    page2
        .goto("https://www.google.com", Some(opts))
        .await
        .unwrap();

    let page3 = tab3.page();
    page3
        .goto("data:text/html,<h1>Another Local Tab</h1>", None)
        .await
        .unwrap();

    // Verify all tabs have correct content
    let title2 = page2.title().await.unwrap();
    assert!(
        title2.to_lowercase().contains("google"),
        "google tab title: {title2}"
    );

    let text1: String = page1
        .evaluate_into("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(text1, "Local Tab");

    let text3: String = page3
        .evaluate_into("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(text3, "Another Local Tab");

    // Close in mixed order: middle, last, first
    tab2.close().await.unwrap();

    // Tab 1 and Tab 3 should still be accessible
    let text1_recheck: String = page1
        .evaluate_into("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(text1_recheck, "Local Tab");

    let text3_recheck: String = page3
        .evaluate_into("document.querySelector('h1').textContent")
        .await
        .unwrap();
    assert_eq!(text3_recheck, "Another Local Tab");

    tab3.close().await.unwrap();
    tab1.close().await.unwrap();
}
