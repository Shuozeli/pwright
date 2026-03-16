//! Integration tests for network capture commands (network-listen, network-list, network-get).
//!
//! Tests the second CDP session approach: one session for interaction,
//! another for network event capture on the same tab.
//!
//! Requires: docker compose -f tests/integration/docker-compose.local.yml up -d

use std::sync::Arc;

use pwright_bridge::playwright::network::{parse_network_request, parse_network_response};
use pwright_cdp::{CdpConnection, CdpSession};
use pwright_integration_tests::{chrome_http_url, connect_and_navigate, server_base_url};

/// Helper: create a second CDP session attached to the same target.
async fn attach_listener_session(
    conn: &Arc<CdpConnection>,
    browser_session: &CdpSession,
    target_id: &str,
) -> CdpSession {
    let session_id = browser_session
        .target_attach(target_id)
        .await
        .expect("failed to attach listener session");
    CdpSession::new(conn.clone(), session_id, target_id.to_string())
}

/// Second session can enable Network domain and receive events
/// while the primary session navigates.
#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn listener_session_captures_navigation_requests() {
    let page = connect_and_navigate("/content.html").await;
    let target_id = page.target_id().unwrap();

    // Get shared connection info
    let http_url = chrome_http_url();
    let version_url = format!("{http_url}/json/version");
    let resp: serde_json::Value = reqwest::get(&version_url)
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let raw_ws = resp["webSocketDebuggerUrl"].as_str().unwrap();
    let ws_url =
        pwright_bridge::browser::rewrite_ws_url(&http_url, raw_ws).unwrap_or(raw_ws.to_string());
    let conn = CdpConnection::connect(&ws_url).await.unwrap();
    let browser = CdpSession::browser(conn.clone());

    // Attach a second session for network listening
    let listener = attach_listener_session(&conn, &browser, target_id).await;
    listener.network_enable().await.unwrap();
    let mut event_rx = listener.subscribe_events();

    // Navigate on the primary session (via Page)
    let base = server_base_url();
    page.goto(&format!("{base}/api-demo"), None).await.unwrap();

    // Collect network events from the listener session
    let mut found_request = false;
    let mut found_response = false;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);

    loop {
        match tokio::time::timeout_at(deadline, event_rx.recv()).await {
            Ok(Ok(event)) => {
                if event.method == "Network.requestWillBeSent"
                    && let Some(req) = parse_network_request(&event.params)
                    && req.url.contains("/api/search")
                {
                    assert_eq!(req.method, "POST");
                    found_request = true;
                } else if event.method == "Network.responseReceived"
                    && let Some(resp) = parse_network_response(&event.params)
                    && resp.url.contains("/api/search")
                {
                    assert_eq!(resp.status, 200);
                    found_response = true;
                }
                if found_request && found_response {
                    break;
                }
            }
            Ok(Err(_)) => break,
            Err(_) => break, // timeout
        }
    }

    assert!(found_request, "listener should capture the POST request");
    assert!(found_response, "listener should capture the response");

    // Cleanup
    let _ = browser.target_detach(listener.session_id().unwrap()).await;
}

/// network-get: fetch response body by request ID from the listener output.
#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn network_get_fetches_response_body() {
    let page = connect_and_navigate("/content.html").await;

    // Use on_response to capture a request ID
    let mut rx = page.on_response().await.unwrap();

    let base = server_base_url();
    page.goto(&format!("{base}/api-demo"), None).await.unwrap();

    // Find the /api/search response
    let reqid = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(r) = rx.recv().await {
            if r.url.contains("/api/search") {
                return Ok(r.request_id);
            }
        }
        Err("no matching response")
    })
    .await
    .unwrap()
    .unwrap();

    // Fetch the body using the request ID
    let body = page.response_body(&reqid).await.unwrap();
    assert!(!body.body.is_empty(), "response body should not be empty");

    // The /api/search endpoint returns JSON
    let parsed: serde_json::Value = serde_json::from_str(&body.body).unwrap();
    assert!(parsed.get("query").is_some(), "should have query field");
    assert!(parsed.get("results").is_some(), "should have results field");
}

/// network-list: retroactive query via JS Performance API.
/// Navigate to api-demo (which triggers a fetch), then verify the
/// performance API returns the fetch as a resource entry.
#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn network_list_via_performance_api() {
    let page = connect_and_navigate("/api-demo").await;

    // api-demo page fires a fetch to /api/search on load.
    // Give it a moment to complete.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let result: String = page
        .evaluate_into(
            r##"JSON.stringify(performance.getEntriesByType('resource').map(e => e.name))"##,
        )
        .await
        .unwrap();

    let urls: Vec<String> = serde_json::from_str(&result).unwrap();

    assert!(!urls.is_empty(), "api-demo should have resource entries");
    assert!(
        urls.iter().any(|u| u.contains("/api/search")),
        "should contain the /api/search fetch, got: {urls:?}"
    );
}

/// Listener captures requests across multiple interactions on the same page.
#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn listener_captures_fetch_from_evaluate() {
    let page = connect_and_navigate("/content.html").await;

    // Start listening
    let mut rx = page.on_response().await.unwrap();

    // Trigger a fetch via JS evaluate (simulates agent interaction)
    let base = server_base_url();
    let _ = page
        .evaluate_async(&format!(
            "fetch('{base}/api/items?count=3').then(r => r.json())"
        ))
        .await
        .unwrap();

    // Verify the listener captured it
    let resp = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(r) = rx.recv().await {
            if r.url.contains("/api/items") {
                return Ok(r);
            }
        }
        Err("no matching response")
    })
    .await
    .unwrap()
    .unwrap();

    assert_eq!(resp.status, 200);
    assert!(resp.mime_type.contains("json"));
}

/// Verify request post_data is captured for POST requests.
#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn listener_captures_post_data() {
    let page = connect_and_navigate("/content.html").await;

    let mut rx = page.on_request().await.unwrap();

    let base = server_base_url();
    let _ = page
        .evaluate_async(&format!(
            r##"fetch('{base}/api/search', {{
                method: 'POST',
                headers: {{'Content-Type': 'application/json'}},
                body: JSON.stringify({{query: 'integration test'}})
            }}).then(r => r.json())"##
        ))
        .await
        .unwrap();

    let req = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(r) = rx.recv().await {
            if r.method == "POST" && r.url.contains("/api/search") {
                return Ok(r);
            }
        }
        Err("no matching request")
    })
    .await
    .unwrap()
    .unwrap();

    assert_eq!(req.method, "POST");
    let post_data = req.post_data.unwrap();
    assert!(post_data.contains("integration test"));
}
