//! Recipe integration tests: run query recipes against test server fixtures.
//!
//! Tests only QUERY recipes (safe, read-only). Mutation recipes are not tested.
//!
//! Requires: docker compose -f docker-compose.local.yml up -d

use std::collections::HashMap;

use pwright_integration_tests::{connect_and_navigate, server_base_url};
use pwright_script::{executor, output::VecSink, parser};

// ── Research recipes ──

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn recipe_search_results() {
    let _base = server_base_url();
    let page = connect_and_navigate("/search-results.html").await;

    let script = parser::parse_yaml(
        r##"
name: "Search test"
type: query
scripts:
  extract_results: |
    (() => {
      const results = [];
      const items = document.querySelectorAll('.g');
      for (const item of items) {
        const titleEl = item.querySelector('h3');
        const linkEl = item.querySelector('a[href]');
        const snippetEl = item.querySelector('.VwiC3b');
        if (titleEl && linkEl) {
          results.push({
            title: titleEl.textContent.trim(),
            url: linkEl.href,
            snippet: snippetEl ? snippetEl.textContent.trim() : ''
          });
        }
      }
      return JSON.stringify(results);
    })()
steps:
  - eval:
      ref: extract_results
      save_as: results
  - output:
      results: "{{ results }}"
"##,
    )
    .unwrap();

    let mut sink = VecSink::new();
    let result = executor::execute(&script, &page, &HashMap::new(), &mut sink)
        .await
        .unwrap();

    assert_eq!(result.status, "ok");
    let output = &result.outputs[0]["results"];
    let parsed: Vec<serde_json::Value> = serde_json::from_str(output).unwrap();
    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0]["title"], "First Result Title");
    assert_eq!(parsed[1]["title"], "Second Result Title");
    assert!(
        parsed[0]["url"]
            .as_str()
            .unwrap()
            .contains("example.com/first")
    );
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn recipe_extract_article() {
    let page = connect_and_navigate("/article.html").await;

    let script = parser::parse_yaml(
        r##"
name: "Article test"
type: query
scripts:
  extract_article: |
    (() => {
      const container = document.querySelector('article') || document.body;
      const title = document.querySelector('h1')?.textContent?.trim() || '';
      const author = document.querySelector('.author')?.textContent?.trim() || '';
      const date = document.querySelector('time')?.getAttribute('datetime') || '';
      const clone = container.cloneNode(true);
      clone.querySelectorAll('nav, .sidebar').forEach(el => el.remove());
      const body = clone.innerText.trim();
      return JSON.stringify({ title, author, date, body_length: body.length });
    })()
steps:
  - eval:
      ref: extract_article
      save_as: article
  - output:
      article: "{{ article }}"
"##,
    )
    .unwrap();

    let mut sink = VecSink::new();
    let result = executor::execute(&script, &page, &HashMap::new(), &mut sink)
        .await
        .unwrap();

    assert_eq!(result.status, "ok");
    let article: serde_json::Value = serde_json::from_str(&result.outputs[0]["article"]).unwrap();
    assert_eq!(article["title"], "Understanding Browser Automation");
    assert_eq!(article["author"], "Jane Doe");
    assert_eq!(article["date"], "2026-03-15");
    assert!(article["body_length"].as_i64().unwrap() > 100);
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn recipe_extract_table() {
    let _base = server_base_url();
    let page = connect_and_navigate("/data-table.html").await;

    let script = parser::parse_yaml(
        r##"
name: "Table test"
type: query
scripts:
  extract_table: |
    ((selector) => {
      const table = document.querySelector(selector);
      if (!table) return JSON.stringify({ error: 'not found' });
      const headers = [];
      const headerRow = table.querySelector('thead tr');
      if (headerRow) {
        headerRow.querySelectorAll('th').forEach(cell => {
          headers.push(cell.textContent.trim());
        });
      }
      const rows = [];
      table.querySelectorAll('tbody tr').forEach(tr => {
        const cells = tr.querySelectorAll('td');
        const row = {};
        cells.forEach((cell, j) => {
          row[headers[j] || 'col_' + j] = cell.textContent.trim();
        });
        rows.push(row);
      });
      return JSON.stringify({ headers, rows, row_count: rows.length });
    })
steps:
  - eval:
      ref: extract_table
      args: ["#pricing"]
      save_as: table_data
  - output:
      data: "{{ table_data }}"
"##,
    )
    .unwrap();

    let mut sink = VecSink::new();
    let result = executor::execute(&script, &page, &HashMap::new(), &mut sink)
        .await
        .unwrap();

    assert_eq!(result.status, "ok");
    let data: serde_json::Value = serde_json::from_str(&result.outputs[0]["data"]).unwrap();
    assert_eq!(data["row_count"], 4);
    assert_eq!(data["headers"][0], "Product");
    assert_eq!(data["headers"][1], "Price");
    assert_eq!(data["rows"][0]["Product"], "Widget A");
    assert_eq!(data["rows"][0]["Price"], "$10.00");
    assert_eq!(data["rows"][3]["Product"], "Widget D");
}

// ── Monitoring recipes ──

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn recipe_check_deploy_pass() {
    let _base = server_base_url();
    let page = connect_and_navigate("/deploy-healthy.html").await;

    let script = parser::parse_yaml(
        r##"
name: "Deploy check"
type: query
scripts:
  check_health: |
    ((expectedText) => {
      const bodyText = document.body.innerText;
      const found = bodyText.includes(expectedText);
      return JSON.stringify({
        status: found ? 'pass' : 'fail',
        title: document.title,
        expected_text_found: found
      });
    })
steps:
  - eval:
      ref: check_health
      args: ["All systems operational"]
      save_as: health
  - output:
      health: "{{ health }}"
"##,
    )
    .unwrap();

    let mut sink = VecSink::new();
    let result = executor::execute(&script, &page, &HashMap::new(), &mut sink)
        .await
        .unwrap();

    assert_eq!(result.status, "ok");
    let health: serde_json::Value = serde_json::from_str(&result.outputs[0]["health"]).unwrap();
    assert_eq!(health["status"], "pass");
    assert_eq!(health["expected_text_found"], true);
    assert_eq!(health["title"], "My App");
}

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn recipe_check_deploy_fail() {
    let page = connect_and_navigate("/deploy-healthy.html").await;

    let script = parser::parse_yaml(
        r##"
name: "Deploy check fail"
type: query
scripts:
  check_health: |
    ((expectedText) => {
      const bodyText = document.body.innerText;
      const found = bodyText.includes(expectedText);
      return JSON.stringify({
        status: found ? 'pass' : 'fail',
        expected_text_found: found
      });
    })
steps:
  - eval:
      ref: check_health
      args: ["text that does not exist on page"]
      save_as: health
  - output:
      health: "{{ health }}"
"##,
    )
    .unwrap();

    let mut sink = VecSink::new();
    let result = executor::execute(&script, &page, &HashMap::new(), &mut sink)
        .await
        .unwrap();

    assert_eq!(result.status, "ok");
    let health: serde_json::Value = serde_json::from_str(&result.outputs[0]["health"]).unwrap();
    assert_eq!(health["status"], "fail");
    assert_eq!(health["expected_text_found"], false);
}

// ── Communication recipes (query only) ──

#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn recipe_notifications() {
    let page = connect_and_navigate("/notifications.html").await;

    let script = parser::parse_yaml(
        r##"
name: "Notifications test"
type: query
scripts:
  extract_notifications: |
    (() => {
      const items = document.querySelectorAll('.notifications-list-item');
      const notifications = [];
      for (const item of items) {
        const repo = item.querySelector('.text-bold')?.textContent?.trim() || '';
        const title = item.querySelector('.markdown-title')?.textContent?.trim() || '';
        const unread = item.classList.contains('notification-unread');
        const time = item.querySelector('relative-time')?.getAttribute('datetime') || '';
        if (title) notifications.push({ repo, title, unread, time });
      }
      return JSON.stringify(notifications);
    })()
steps:
  - eval:
      ref: extract_notifications
      save_as: notifications
  - output:
      notifications: "{{ notifications }}"
"##,
    )
    .unwrap();

    let mut sink = VecSink::new();
    let result = executor::execute(&script, &page, &HashMap::new(), &mut sink)
        .await
        .unwrap();

    assert_eq!(result.status, "ok");
    let notifs: Vec<serde_json::Value> =
        serde_json::from_str(&result.outputs[0]["notifications"]).unwrap();
    assert_eq!(notifs.len(), 3);
    assert_eq!(notifs[0]["title"], "Fix login bug");
    assert_eq!(notifs[0]["unread"], true);
    assert_eq!(notifs[0]["repo"], "owner/repo-one");
    assert_eq!(notifs[2]["title"], "Update docs");
    assert_eq!(notifs[2]["unread"], false);
}
