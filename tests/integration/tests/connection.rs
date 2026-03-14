//! Connection lifecycle tests.

use std::sync::Arc;

use pwright_cdp::{CdpConnection, CdpSession};
use pwright_integration_tests::connect_and_navigate;

fn chrome_http_url() -> String {
    let host = std::env::var("CHROME_HOST").unwrap_or_else(|_| "localhost".into());
    let port = std::env::var("CHROME_PORT").unwrap_or_else(|_| "9222".into());
    let resolved = if host == "localhost" || host.parse::<std::net::IpAddr>().is_ok() {
        host
    } else {
        use std::net::ToSocketAddrs;
        format!("{host}:{port}")
            .to_socket_addrs()
            .ok()
            .and_then(|mut addrs| addrs.next())
            .map(|addr| addr.ip().to_string())
            .unwrap_or(host)
    };
    format!("http://{resolved}:{port}")
}

async fn connect() -> Arc<CdpConnection> {
    let chrome_url = chrome_http_url();
    let resp: serde_json::Value = reqwest::get(&format!("{chrome_url}/json/version"))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let raw_ws = resp["webSocketDebuggerUrl"].as_str().unwrap();
    let ws_url = pwright_bridge::browser::rewrite_ws_url(&chrome_url, raw_ws)
        .unwrap_or_else(|_| raw_ws.to_string());
    CdpConnection::connect(&ws_url).await.unwrap()
}

/// Verify one connection can handle multiple sequential targets.
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn shared_connection_survives_multiple_targets() {
    let conn = connect().await;
    let browser = CdpSession::browser(conn.clone());

    for i in 0..5 {
        let tid = browser
            .target_create("about:blank")
            .await
            .unwrap_or_else(|e| panic!("round {i}: create failed: {e}"));

        let sid = browser
            .target_attach(&tid)
            .await
            .unwrap_or_else(|e| panic!("round {i}: attach failed: {e}"));

        let session = CdpSession::new(conn.clone(), sid, tid.clone());
        let doc = session
            .dom_get_document()
            .await
            .unwrap_or_else(|e| panic!("round {i}: getDocument failed: {e}"));

        assert!(doc["root"]["nodeId"].as_i64().unwrap() > 0);

        browser
            .target_close(&tid)
            .await
            .unwrap_or_else(|e| panic!("round {i}: close failed: {e}"));
    }
}

/// Verify concurrent targets on one connection.
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn concurrent_targets_on_shared_connection() {
    let conn = connect().await;

    let mut handles = vec![];
    for i in 0..5 {
        let conn = conn.clone();
        handles.push(tokio::spawn(async move {
            let browser = CdpSession::browser(conn.clone());
            let tid = browser.target_create("about:blank").await.unwrap();
            let sid = browser.target_attach(&tid).await.unwrap();
            let session = CdpSession::new(conn, sid, tid.clone());
            let doc = session.dom_get_document().await.unwrap();
            assert!(doc["root"]["nodeId"].as_i64().unwrap() > 0);
            browser.target_close(&tid).await.unwrap();
            i
        }));
    }

    let mut results = vec![];
    for h in handles {
        results.push(h.await.unwrap());
    }
    results.sort();
    assert_eq!(results, vec![0, 1, 2, 3, 4]);
}

/// Verify connect_and_navigate works for two sequential calls.
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn connect_and_navigate_twice() {
    let page1 = connect_and_navigate("/content.html").await;
    let title1 = page1.title().await.unwrap();
    assert_eq!(title1, "Content Page");
    drop(page1);

    // Small delay for cleanup
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let page2 = connect_and_navigate("/input.html").await;
    let title2 = page2.title().await.unwrap();
    assert_eq!(title2, "Input Page");
    drop(page2);
}

/// Navigate, click, then navigate again - like locator tests do.
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn connect_navigate_click_navigate() {
    let page1 = connect_and_navigate("/interactive.html").await;
    page1.locator("#counter").click().await.unwrap();
    let text = page1.locator("#counter").text_content().await.unwrap();
    assert_eq!(text, Some("Clicked 1 times".to_string()));
    drop(page1);

    let page2 = connect_and_navigate("/content.html").await;
    let title = page2.title().await.unwrap();
    assert_eq!(title, "Content Page");
}

/// Same but WITHOUT explicit drop/sleep (like locator tests do).
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn connect_and_navigate_no_explicit_drop() {
    {
        let page1 = connect_and_navigate("/content.html").await;
        let title1 = page1.title().await.unwrap();
        assert_eq!(title1, "Content Page");
        // page1 drops here at end of block
    }

    let page2 = connect_and_navigate("/input.html").await;
    let title2 = page2.title().await.unwrap();
    assert_eq!(title2, "Input Page");
}
