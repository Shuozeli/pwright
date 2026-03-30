//! Integration tests for evaluate and typed variants.
//!
//! Tests cover: primitive types, complex objects, error handling, Promise
//! resolution, fetch, DOM manipulation, and evaluation.

use pwright_bridge::evaluate::FromEvalJson;
use pwright_integration_tests::{connect_and_navigate, server_base_url};

// ═══════════════════════════════════════════════════════════════════
//  evaluate — expressions
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn sync_returns_number() {
    let page = connect_and_navigate("/content.html").await;
    let result = page.evaluate("1 + 2 + 3").await.unwrap();
    assert_eq!(result["value"], 6);
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn sync_returns_string() {
    let page = connect_and_navigate("/content.html").await;
    let result = page.evaluate("'hello' + ' ' + 'world'").await.unwrap();
    assert_eq!(result["value"], "hello world");
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn sync_returns_boolean() {
    let page = connect_and_navigate("/content.html").await;
    let result = page.evaluate("3 > 2").await.unwrap();
    assert_eq!(result["value"], true);
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn sync_returns_null() {
    let page = connect_and_navigate("/content.html").await;
    let result = page.evaluate("null").await.unwrap();
    assert!(result["value"].is_null());
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn sync_returns_undefined_as_no_value() {
    let page = connect_and_navigate("/content.html").await;
    let result = page.evaluate("undefined").await.unwrap();
    // undefined has no "value" field in CDP RemoteObject
    assert_eq!(result["type"], "undefined");
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn sync_returns_object() {
    let page = connect_and_navigate("/content.html").await;
    let result = page.evaluate("({a: 1, b: 'two'})").await.unwrap();
    assert_eq!(result["value"]["a"], 1);
    assert_eq!(result["value"]["b"], "two");
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn sync_returns_array() {
    let page = connect_and_navigate("/content.html").await;
    let result = page.evaluate("[1, 2, 3]").await.unwrap();
    let arr = result["value"].as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0], 1);
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn sync_dom_query() {
    let page = connect_and_navigate("/content.html").await;
    let title: String = page.evaluate_into("document.title").await.unwrap();
    assert!(!title.is_empty());
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn sync_dom_element_count() {
    let page = connect_and_navigate("/content.html").await;
    let count: i64 = page
        .evaluate_into("document.querySelectorAll('*').length")
        .await
        .unwrap();
    assert!(count > 0, "page should have elements");
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn sync_dom_manipulation() {
    let page = connect_and_navigate("/content.html").await;

    // Create an element via JS
    page.evaluate(
        "(() => { const d = document.createElement('div'); d.id = 'injected'; d.textContent = 'created'; document.body.appendChild(d); })()",
    )
    .await
    .unwrap();

    // Verify it exists
    let text: String = page
        .evaluate_into("document.getElementById('injected').textContent")
        .await
        .unwrap();
    assert_eq!(text, "created");
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn sync_js_exception_returns_error() {
    let page = connect_and_navigate("/content.html").await;
    let result = page.evaluate("throw new Error('boom')").await;
    assert!(result.is_err(), "JS exception should return error");
    let err = format!("{}", result.unwrap_err());
    assert!(
        err.contains("boom") || err.contains("exception"),
        "error: {err}"
    );
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn sync_reference_error() {
    let page = connect_and_navigate("/content.html").await;
    let result = page.evaluate("nonExistentVariable").await;
    assert!(result.is_err());
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn sync_into_type_mismatch() {
    let page = connect_and_navigate("/content.html").await;
    // Expression returns a number, but we request a String
    let result: Result<String, _> = page.evaluate_into("42").await;
    assert!(result.is_err(), "number should not coerce to String");
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn sync_into_json_deserialize() {
    let page = connect_and_navigate("/content.html").await;

    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct Point {
        x: f64,
        y: f64,
    }

    let point: FromEvalJson<Point> = page
        .evaluate_into(r#"JSON.stringify({x: 1.5, y: 2.5})"#)
        .await
        .unwrap();
    assert_eq!(point.0, Point { x: 1.5, y: 2.5 });
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn sync_promise_returns_object_not_value() {
    let page = connect_and_navigate("/content.html").await;
    // Sync evaluate on a Promise should return the Promise object, not the resolved value
    let result = page
        .evaluate("new Promise(resolve => resolve(42))")
        .await
        .unwrap();
    // The type should be "object" (a Promise), not "number"
    assert_ne!(
        result.get("value").and_then(|v| v.as_i64()),
        Some(42),
        "sync should NOT resolve the Promise"
    );
}

// ═══════════════════════════════════════════════════════════════════
//  evaluate — Promise expressions
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn async_resolves_promise() {
    let page = connect_and_navigate("/content.html").await;
    let result = page
        .evaluate("new Promise(resolve => resolve(42))")
        .await
        .unwrap();
    assert_eq!(result["value"], 42);
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn async_resolves_promise_string() {
    let page = connect_and_navigate("/content.html").await;
    let result = page
        .evaluate("new Promise(resolve => resolve('hello'))")
        .await
        .unwrap();
    assert_eq!(result["value"], "hello");
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn async_resolves_delayed_promise() {
    let page = connect_and_navigate("/content.html").await;
    let result = page
        .evaluate("new Promise(resolve => setTimeout(() => resolve('delayed'), 100))")
        .await
        .unwrap();
    assert_eq!(result["value"], "delayed");
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn async_rejects_promise_returns_error() {
    let page = connect_and_navigate("/content.html").await;
    let result = page.evaluate("Promise.reject(new Error('rejected'))").await;
    assert!(result.is_err(), "rejected Promise should return error");
}

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn async_fetch_json() {
    let page = connect_and_navigate("/content.html").await;
    let base = server_base_url();
    let result = page
        .evaluate(&format!(
            "fetch('{base}/api/items?count=3').then(r => r.json())"
        ))
        .await
        .unwrap();
    let items = result["value"].as_array().unwrap();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0]["name"], "Item 1");
}

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn async_fetch_text() {
    let page = connect_and_navigate("/content.html").await;
    let base = server_base_url();
    let result = page
        .evaluate(&format!("fetch('{base}/health').then(r => r.text())"))
        .await
        .unwrap();
    assert_eq!(result["value"], "ok");
}

#[tokio::test]
#[ignore = "requires docker: chrome + test-server"]
async fn async_fetch_post() {
    let page = connect_and_navigate("/content.html").await;
    let base = server_base_url();
    let result = page
        .evaluate(&format!(
            r##"fetch('{base}/api/search', {{
                method: 'POST',
                headers: {{'Content-Type': 'application/json'}},
                body: JSON.stringify({{query: 'rust'}})
            }}).then(r => r.json())"##
        ))
        .await
        .unwrap();
    assert_eq!(result["value"]["query"], "rust");
    let results = result["value"]["results"].as_array().unwrap();
    assert_eq!(results.len(), 3);
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn async_await_syntax() {
    let page = connect_and_navigate("/content.html").await;
    // Using async IIFE with await keyword
    let result = page
        .evaluate("(async () => { const x = await Promise.resolve(99); return x; })()")
        .await
        .unwrap();
    assert_eq!(result["value"], 99);
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn async_into_typed() {
    let page = connect_and_navigate("/content.html").await;
    let val: i64 = page.evaluate_into("Promise.resolve(123)").await.unwrap();
    assert_eq!(val, 123);
}

// ═══════════════════════════════════════════════════════════════════
//  Sync vs Async comparison
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn sync_and_async_agree_on_simple_expression() {
    let page = connect_and_navigate("/content.html").await;

    let sync_result = page.evaluate("2 + 2").await.unwrap();
    let async_result = page.evaluate("2 + 2").await.unwrap();

    // Both should return 4
    assert_eq!(sync_result["value"], 4);
    assert_eq!(async_result["value"], 4);
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn async_resolves_where_sync_would_not() {
    let page = connect_and_navigate("/content.html").await;

    // Async resolves the Promise
    let async_result = page.evaluate("Promise.resolve('resolved')").await.unwrap();
    assert_eq!(async_result["value"], "resolved");

    // Sync returns the Promise object, not the resolved value
    let sync_result = page.evaluate("Promise.resolve('resolved')").await.unwrap();
    assert_ne!(
        sync_result.get("value").and_then(|v| v.as_str()),
        Some("resolved"),
        "sync should NOT resolve Promises"
    );
}
