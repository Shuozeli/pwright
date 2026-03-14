//! Concurrency tests against real Chrome.
//!
//! Verifies thread safety of pwright's CDP layer:
//! - Multiple tabs operating concurrently on one WebSocket
//! - Concurrent reads on different pages
//! - Page::close() from multiple tasks
//! - Shared connection under load
//!
//! All tests are deterministic (no timing dependencies).
//!
//! Requires: docker compose -f tests/integration/docker-compose.yml up -d

use pwright_integration_tests::connect_and_navigate;

/// 5 pages navigate and read content concurrently.
/// Each gets its own tab - verifies session isolation.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires docker: chrome"]
async fn concurrent_navigate_and_read_title() {
    let mut handles = vec![];

    let pages_and_titles = vec![
        ("/content.html", "Content Page"),
        ("/input.html", "Input Page"),
        ("/login", "Login"),
        ("/todo", "Todo"),
        ("/interactive.html", "Interactive Page"),
    ];

    for (path, expected_title) in pages_and_titles {
        handles.push(tokio::spawn(async move {
            let page = connect_and_navigate(path).await;
            let title = page.title().await.unwrap();
            assert_eq!(title, expected_title, "wrong title for {path}");
            title
        }));
    }

    let mut titles: Vec<String> = vec![];
    for h in handles {
        titles.push(h.await.unwrap());
    }
    titles.sort();
    assert_eq!(
        titles,
        vec![
            "Content Page",
            "Input Page",
            "Interactive Page",
            "Login",
            "Todo"
        ]
    );
}

/// 5 pages read DOM attributes concurrently.
/// Verifies querySelector + getAttributes isolation across sessions.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires docker: chrome"]
async fn concurrent_dom_queries() {
    let mut handles = vec![];

    for i in 0..5 {
        handles.push(tokio::spawn(async move {
            let page = connect_and_navigate("/content.html").await;

            // Each task performs multiple DOM operations
            let text = page.locator("#heading").text_content().await.unwrap();
            assert_eq!(text, Some("Hello pwright".to_string()));

            let href = page.locator("#link").get_attribute("href").await.unwrap();
            assert_eq!(href, Some("https://example.com".to_string()));

            let count = page.locator("#list li").count().await.unwrap();
            assert_eq!(count, 3);

            let visible = page.locator("#heading").is_visible().await.unwrap();
            assert!(visible);

            i
        }));
    }

    let mut results: Vec<usize> = vec![];
    for h in handles {
        results.push(h.await.unwrap());
    }
    results.sort();
    assert_eq!(results, vec![0, 1, 2, 3, 4]);
}

/// Concurrent first/last/nth on different pages.
/// Verifies querySelectorAll + index resolution under concurrency.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires docker: chrome"]
async fn concurrent_first_last_nth() {
    let mut handles = vec![];

    for _ in 0..5 {
        handles.push(tokio::spawn(async move {
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
        }));
    }

    for h in handles {
        h.await.unwrap();
    }
}

/// Concurrent form interactions on separate tabs.
/// Each tab fills different values and reads them back.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires docker: chrome"]
async fn concurrent_form_fill() {
    let mut handles = vec![];

    for i in 0..5 {
        handles.push(tokio::spawn(async move {
            let page = connect_and_navigate("/input.html").await;

            let value = format!("test-value-{i}");
            page.locator("#text").fill(&value).await.unwrap();
            let read_back = page.locator("#text").input_value().await.unwrap();
            assert_eq!(read_back, value, "task {i}: fill/read mismatch");

            i
        }));
    }

    let mut results: Vec<usize> = vec![];
    for h in handles {
        results.push(h.await.unwrap());
    }
    results.sort();
    assert_eq!(results, vec![0, 1, 2, 3, 4]);
}

/// Close pages concurrently from multiple tasks.
/// Verifies AtomicBool close guard prevents double-close issues.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires docker: chrome"]
async fn concurrent_page_close() {
    use std::sync::Arc;

    // Create 5 pages
    let mut pages = vec![];
    for _ in 0..5 {
        pages.push(Arc::new(connect_and_navigate("/content.html").await));
    }

    // Close each from 2 concurrent tasks (tests idempotent close)
    let mut handles = vec![];
    for page in &pages {
        let p1 = page.clone();
        let p2 = page.clone();
        handles.push(tokio::spawn(async move {
            p1.close().await.ok();
        }));
        handles.push(tokio::spawn(async move {
            p2.close().await.ok();
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    // All should be closed
    for page in &pages {
        assert!(page.is_closed());
    }
}

/// Mix of reads and writes on different pages concurrently.
/// Simulates a real scraping workload with multiple tabs.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires docker: chrome"]
async fn concurrent_mixed_workload() {
    let mut handles = vec![];

    // Task 1: Read content page
    handles.push(tokio::spawn(async {
        let page = connect_and_navigate("/content.html").await;
        let title = page.title().await.unwrap();
        assert_eq!(title, "Content Page");
        let text = page.locator("#heading").text_content().await.unwrap();
        assert_eq!(text, Some("Hello pwright".to_string()));
        "content"
    }));

    // Task 2: Fill form
    handles.push(tokio::spawn(async {
        let page = connect_and_navigate("/input.html").await;
        page.locator("#text").fill("concurrent-test").await.unwrap();
        let val = page.locator("#text").input_value().await.unwrap();
        assert_eq!(val, "concurrent-test");
        "input"
    }));

    // Task 3: Click interactive element
    handles.push(tokio::spawn(async {
        let page = connect_and_navigate("/interactive.html").await;
        page.locator("#counter").click().await.unwrap();
        let text = page.locator("#counter").text_content().await.unwrap();
        assert_eq!(text, Some("Clicked 1 times".to_string()));
        "interactive"
    }));

    // Task 4: Check todo structure
    handles.push(tokio::spawn(async {
        let page = connect_and_navigate("/todo").await;
        let count = page.locator("li.todo").count().await.unwrap();
        assert_eq!(count, 3);
        let footer = page.locator(".todo-count").text_content().await.unwrap();
        assert_eq!(footer, Some("2 items left".to_string()));
        "todo"
    }));

    // Task 5: Login flow
    handles.push(tokio::spawn(async {
        let page = connect_and_navigate("/login").await;
        let disabled = page.locator("#submit").is_disabled().await.unwrap();
        assert!(disabled);
        "login"
    }));

    let mut results: Vec<&str> = vec![];
    for h in handles {
        results.push(h.await.unwrap());
    }
    results.sort();
    assert_eq!(
        results,
        vec!["content", "input", "interactive", "login", "todo"]
    );
}
