//! Script execution integration tests: run YAML scripts against real Chrome.
//!
//! Requires: docker compose -f docker-compose.local.yml up -d

use std::collections::HashMap;

use pwright_integration_tests::{connect_and_navigate, server_base_url};
use pwright_script::{executor, output::VecSink, parser, validator};

/// Run a simple extract script against the test server.
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn script_extract_title() {
    let page = connect_and_navigate("/content.html").await;

    let script = parser::parse_yaml(
        r##"
name: "Extract title"
steps:
  - extract:
      selector: "#heading"
      field: text_content
      save_as: title
  - output:
      title: "{{ title }}"
"##,
    )
    .unwrap();

    let mut sink = VecSink::new();
    let result = executor::execute(&script, &page, &HashMap::new(), &mut sink)
        .await
        .unwrap();

    assert_eq!(result.status, "ok");
    assert_eq!(result.outputs.len(), 1);
    assert_eq!(result.outputs[0]["title"], "Hello pwright");
}

/// Run a script with goto + wait_for + extract against real Chrome.
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn script_goto_and_extract() {
    let base = server_base_url();
    let yaml = r##"
name: "Goto and extract"
params:
  url: { type: string, required: true }
steps:
  - goto: "{{ url }}"
    wait_for: "#heading"
  - extract:
      selector: "#heading"
      field: text_content
      save_as: heading
  - extract:
      selector: "#list li"
      field: text_content
      save_as: first_item
  - output:
      heading: "{{ heading }}"
      item: "{{ first_item }}"
"##;

    // Need a fresh page for goto
    let page = connect_and_navigate("/empty.html").await;
    let script = parser::parse_yaml(yaml).unwrap();
    let params = HashMap::from([("url".into(), format!("{base}/content.html"))]);

    validator::validate(&script, &params).unwrap();

    let mut sink = VecSink::new();
    let result = executor::execute(&script, &page, &params, &mut sink)
        .await
        .unwrap();

    assert_eq!(result.status, "ok");
    assert_eq!(result.outputs[0]["heading"], "Hello pwright");
}

/// Run a script with wait step (anti-bot delay).
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn script_wait_step() {
    let page = connect_and_navigate("/content.html").await;

    let script = parser::parse_yaml(
        r##"
name: "Wait test"
steps:
  - wait: 100
  - extract:
      selector: "#heading"
      field: text_content
      save_as: title
  - output:
      title: "{{ title }}"
"##,
    )
    .unwrap();

    let start = std::time::Instant::now();
    let mut sink = VecSink::new();
    let result = executor::execute(&script, &page, &HashMap::new(), &mut sink)
        .await
        .unwrap();

    assert_eq!(result.status, "ok");
    assert!(
        start.elapsed().as_millis() >= 100,
        "should wait at least 100ms"
    );
    assert_eq!(result.outputs[0]["title"], "Hello pwright");
}

/// Run a script with on_error: continue.
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn script_error_continue() {
    let page = connect_and_navigate("/content.html").await;

    let script = parser::parse_yaml(
        r##"
name: "Error continue"
steps:
  - click: ".nonexistent"
    on_error: continue
  - extract:
      selector: "#heading"
      field: text_content
      save_as: title
  - output:
      title: "{{ title }}"
"##,
    )
    .unwrap();

    let mut sink = VecSink::new();
    let result = executor::execute(&script, &page, &HashMap::new(), &mut sink)
        .await
        .unwrap();

    assert_eq!(result.status, "ok");
    // First step errored but continued
    assert_eq!(sink.results[0].status, "error");
    assert_eq!(sink.results[1].status, "ok");
    assert_eq!(result.outputs[0]["title"], "Hello pwright");
}

/// Validate a script without executing.
#[tokio::test]
#[ignore = "requires docker: chrome"]
async fn script_validation() {
    let valid = parser::parse_yaml(
        r##"
name: "Valid"
params:
  url: { type: string, required: true }
steps:
  - goto: "{{ url }}"
  - extract:
      selector: "h1"
      field: text_content
      save_as: title
"##,
    )
    .unwrap();

    let params = HashMap::from([("url".into(), "https://example.com".into())]);
    assert!(validator::validate(&valid, &params).is_ok());

    // Missing required param
    assert!(validator::validate(&valid, &HashMap::new()).is_err());

    // Invalid JS ref
    let bad = parser::parse_yaml(
        r##"
name: "Bad ref"
steps:
  - eval:
      ref: nonexistent
      save_as: x
"##,
    )
    .unwrap();
    assert!(validator::validate(&bad, &HashMap::new()).is_err());
}
