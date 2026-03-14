//! Concurrency tests for pwright internals (no Chrome needed).
//!
//! Verifies thread safety of Page, Locator, and CdpClient interactions
//! using FakeCdpClient with an in-memory DOM.

use std::sync::Arc;

use pwright_bridge::playwright::Page;
use pwright_fake::FakeCdpClient;

fn page_with_html(html: &str) -> Arc<Page> {
    let fake = Arc::new(FakeCdpClient::from_html(html));
    Arc::new(Page::new(fake))
}

/// Concurrent reads from multiple tasks on the same Page.
#[tokio::test]
async fn concurrent_reads_same_page() {
    let page = page_with_html(
        r#"
        <div id="app">
            <h1 id="title">Hello</h1>
            <p class="desc">World</p>
            <ul>
                <li>A</li>
                <li>B</li>
                <li>C</li>
            </ul>
        </div>
    "#,
    );

    let mut handles = vec![];
    for _ in 0..10 {
        let p = page.clone();
        handles.push(tokio::spawn(async move {
            let title = p.locator("#title").text_content().await.unwrap();
            assert_eq!(title, Some("Hello".to_string()));

            let desc = p.locator(".desc").text_content().await.unwrap();
            assert_eq!(desc, Some("World".to_string()));

            let count = p.locator("li").count().await.unwrap();
            assert_eq!(count, 3);

            let visible = p.locator("#title").is_visible().await.unwrap();
            assert!(visible);
        }));
    }

    for h in handles {
        h.await.unwrap();
    }
}

/// Concurrent first/last/nth on the same page.
#[tokio::test]
async fn concurrent_nth_same_page() {
    let page = page_with_html(
        r#"
        <ul>
            <li>Alpha</li>
            <li>Beta</li>
            <li>Gamma</li>
            <li>Delta</li>
            <li>Epsilon</li>
        </ul>
    "#,
    );

    let mut handles = vec![];
    for i in 0..5 {
        let p = page.clone();
        handles.push(tokio::spawn(async move {
            let text = p.locator("li").nth(i as i64).text_content().await.unwrap();
            let expected = ["Alpha", "Beta", "Gamma", "Delta", "Epsilon"][i];
            assert_eq!(text, Some(expected.to_string()));
        }));
    }

    for h in handles {
        h.await.unwrap();
    }
}

/// Concurrent is_checked / is_disabled queries.
#[tokio::test]
async fn concurrent_state_queries() {
    let page = page_with_html(
        r#"
        <input type="checkbox" id="c1" checked />
        <input type="checkbox" id="c2" />
        <button id="b1" disabled>Disabled</button>
        <button id="b2">Enabled</button>
    "#,
    );

    let mut handles = vec![];
    for _ in 0..10 {
        let p = page.clone();
        handles.push(tokio::spawn(async move {
            assert!(p.locator("#c1").is_checked().await.unwrap());
            assert!(!p.locator("#c2").is_checked().await.unwrap());
            assert!(p.locator("#b1").is_disabled().await.unwrap());
            assert!(!p.locator("#b2").is_disabled().await.unwrap());
        }));
    }

    for h in handles {
        h.await.unwrap();
    }
}

/// Concurrent close from multiple tasks (tests AtomicBool guard).
#[tokio::test]
async fn concurrent_close() {
    let page = page_with_html("<div>test</div>");

    let mut handles = vec![];
    for _ in 0..10 {
        let p = page.clone();
        handles.push(tokio::spawn(async move {
            // close() should be idempotent - only one should actually close
            p.close().await.ok();
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    assert!(page.is_closed());

    // Page-level operations after close should fail
    let result = page.evaluate("1+1").await;
    assert!(result.is_err());
}

/// Concurrent attribute reads.
#[tokio::test]
async fn concurrent_attribute_reads() {
    let page = page_with_html(
        r#"
        <a id="link" href="https://example.com" target="_blank" class="primary">Link</a>
    "#,
    );

    let mut handles = vec![];
    for _ in 0..10 {
        let p = page.clone();
        handles.push(tokio::spawn(async move {
            let href = p.locator("#link").get_attribute("href").await.unwrap();
            assert_eq!(href, Some("https://example.com".to_string()));

            let target = p.locator("#link").get_attribute("target").await.unwrap();
            assert_eq!(target, Some("_blank".to_string()));

            let missing = p.locator("#link").get_attribute("data-x").await.unwrap();
            assert_eq!(missing, None);
        }));
    }

    for h in handles {
        h.await.unwrap();
    }
}

/// Mix reads and close concurrently - reads should either succeed
/// or fail with "Page is closed", never panic.
#[tokio::test]
async fn concurrent_reads_during_close() {
    let page = page_with_html(r#"<h1>Title</h1><p>Body</p>"#);

    let mut handles = vec![];

    // Spawn readers
    for _ in 0..5 {
        let p = page.clone();
        handles.push(tokio::spawn(async move {
            for _ in 0..10 {
                match p.locator("h1").text_content().await {
                    Ok(Some(t)) => assert_eq!(t, "Title"),
                    Ok(None) => {}   // possible during close
                    Err(_) => break, // page closed
                }
                tokio::task::yield_now().await;
            }
        }));
    }

    // Spawn closer
    let p = page.clone();
    handles.push(tokio::spawn(async move {
        tokio::task::yield_now().await;
        p.close().await.ok();
    }));

    for h in handles {
        h.await.unwrap();
    }
}
