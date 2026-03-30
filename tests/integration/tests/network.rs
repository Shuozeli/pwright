//! Network interception integration tests against real Chrome.
//!
//! Requires: docker compose -f docker-compose.local.yml up -d

use pwright_integration_tests::{connect_and_navigate, server_base_url};

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn wait_for_response_captures_api_call() {
    let page = connect_and_navigate("/content.html").await;

    let mut rx = page.on_response().await.unwrap();

    let base = server_base_url();
    page.goto(&format!("{base}/api-demo"), None).await.unwrap();

    let resp = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(r) = rx.recv().await {
            if r.url.contains("/api/search") {
                return Ok(r);
            }
        }
        Err("channel closed")
    })
    .await
    .unwrap()
    .unwrap();

    assert_eq!(resp.status, 200);
    assert!(resp.url.contains("/api/search"));
}

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn wait_for_request_captures_post() {
    let page = connect_and_navigate("/content.html").await;

    let mut rx = page.on_request().await.unwrap();

    let base = server_base_url();
    page.goto(&format!("{base}/api-demo"), None).await.unwrap();

    let req = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(r) = rx.recv().await {
            if r.method == "POST" && r.url.contains("/api/search") {
                return Ok(r);
            }
        }
        Err("channel closed")
    })
    .await
    .unwrap()
    .unwrap();

    assert_eq!(req.method, "POST");
}

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn evaluate_fetch_from_page() {
    let page = connect_and_navigate("/content.html").await;

    let base = server_base_url();
    let result = page
        .evaluate(&format!(
            "fetch('{base}/api/items?count=5').then(r => r.json())"
        ))
        .await
        .unwrap();

    let items = result["value"].as_array().unwrap();
    assert_eq!(items.len(), 5);
    assert_eq!(items[0]["name"], "Item 1");
}

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn evaluate_with_arg_no_injection() {
    let page = connect_and_navigate("/content.html").await;

    let result = page
        .evaluate_with_arg(
            "function(greeting) { return greeting + ' from pwright'; }",
            &serde_json::json!("hello"),
        )
        .await
        .unwrap();

    assert_eq!(result["value"], "hello from pwright");
}
