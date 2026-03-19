//! Integration tests: execute scripts against FakeCdpClient.

use std::collections::HashMap;
use std::sync::Arc;

use pwright_bridge::playwright::Page;
use pwright_fake::FakeCdpClient;
use pwright_script::executor;
use pwright_script::output::VecSink;
use pwright_script::parser;

fn page_with_html(html: &str) -> Page {
    let fake = Arc::new(FakeCdpClient::from_html(html));
    Page::new(fake)
}

#[tokio::test]
async fn execute_extract_title() {
    let page = page_with_html(r##"<h1 id="title">Hello pwright</h1>"##);

    let script = parser::parse_yaml(
        r##"
name: "Extract title"
params:
  greeting: { type: string, default: "Hi" }
steps:
  - extract:
      selector: "#title"
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

    assert_eq!(result.status, executor::ExecutionStatus::Ok);
    assert_eq!(result.total_steps, 2);
    assert_eq!(result.succeeded, 2);
    assert_eq!(result.outputs.len(), 1);
    assert_eq!(result.outputs[0]["title"], "Hello pwright");

    // Check step results
    assert_eq!(sink.results.len(), 2);
    assert_eq!(sink.results[0].step_type, "extract");
    assert_eq!(sink.results[0].status, executor::ExecutionStatus::Ok);
    assert_eq!(sink.results[0].details["value"], "Hello pwright");
    assert_eq!(sink.results[1].step_type, "output");
}

#[tokio::test]
async fn execute_with_params() {
    let page = page_with_html(r##"<a href="https://example.com" id="link">Link</a>"##);

    let script = parser::parse_yaml(
        r##"
name: "Param test"
params:
  target_attr: { type: string, required: true }
steps:
  - extract:
      selector: "#link"
      field: "attribute:href"
      save_as: url
  - output:
      url: "{{ url }}"
"##,
    )
    .unwrap();

    let params = HashMap::from([("target_attr".into(), "href".into())]);
    let mut sink = VecSink::new();
    let result = executor::execute(&script, &page, &params, &mut sink)
        .await
        .unwrap();

    assert_eq!(result.status, executor::ExecutionStatus::Ok);
    assert_eq!(result.outputs[0]["url"], "https://example.com");
}

#[tokio::test]
async fn execute_click_and_extract() {
    let page = page_with_html(
        r##"
        <button id="btn">Click me</button>
        <div id="status">Ready</div>
    "##,
    );

    let script = parser::parse_yaml(
        r##"
name: "Click test"
steps:
  - click: "#btn"
  - extract:
      selector: "#status"
      field: text_content
      save_as: status
  - output:
      status: "{{ status }}"
"##,
    )
    .unwrap();

    let mut sink = VecSink::new();
    let result = executor::execute(&script, &page, &HashMap::new(), &mut sink)
        .await
        .unwrap();

    assert_eq!(result.status, executor::ExecutionStatus::Ok);
    assert_eq!(result.succeeded, 3);
    assert_eq!(sink.results[0].step_type, "click");
    assert_eq!(sink.results[0].details["selector"], "#btn");
}

#[tokio::test]
async fn execute_fill_and_extract() {
    let page = page_with_html(r##"<input id="email" type="email" value="" />"##);

    let script = parser::parse_yaml(
        r##"
name: "Fill test"
steps:
  - fill:
      selector: "#email"
      value: "user@test.example"
"##,
    )
    .unwrap();

    let mut sink = VecSink::new();
    let result = executor::execute(&script, &page, &HashMap::new(), &mut sink)
        .await
        .unwrap();

    assert_eq!(result.status, executor::ExecutionStatus::Ok);
    assert_eq!(sink.results[0].step_type, "fill");
    assert_eq!(sink.results[0].details["selector"], "#email");
}

#[tokio::test]
async fn execute_js_registry_eval() {
    let page = page_with_html(r##"<div id="data">42</div>"##);

    let script = parser::parse_yaml(
        r##"
name: "JS registry test"
scripts:
  get_data: "document.getElementById('data').textContent"
steps:
  - eval:
      ref: get_data
      save_as: value
  - output:
      value: "{{ value }}"
"##,
    )
    .unwrap();

    let mut sink = VecSink::new();
    let result = executor::execute(&script, &page, &HashMap::new(), &mut sink)
        .await
        .unwrap();

    assert_eq!(result.status, executor::ExecutionStatus::Ok);
    assert_eq!(sink.results[0].step_type, "eval");
    assert_eq!(sink.results[0].details["ref"], "get_data");
}

#[tokio::test]
async fn execute_error_fail_policy() {
    let page = page_with_html(r##"<div>test</div>"##);

    let script = parser::parse_yaml(
        r##"
name: "Error test"
steps:
  - click: ".nonexistent"
  - extract:
      selector: "div"
      field: text_content
      save_as: text
"##,
    )
    .unwrap();

    let mut sink = VecSink::new();
    let result = executor::execute(&script, &page, &HashMap::new(), &mut sink)
        .await
        .unwrap();

    assert_eq!(result.status, executor::ExecutionStatus::Error);
    assert_eq!(result.succeeded, 0);
    assert_eq!(result.failed, 1);
    assert_eq!(result.skipped, 1); // second step skipped
    assert!(result.error.is_some());
}

#[tokio::test]
async fn execute_error_continue_policy() {
    let page = page_with_html(r##"<div id="ok">Success</div>"##);

    let script = parser::parse_yaml(
        r##"
name: "Continue on error"
steps:
  - click: ".nonexistent"
    on_error: continue
  - extract:
      selector: "#ok"
      field: text_content
      save_as: text
  - output:
      text: "{{ text }}"
"##,
    )
    .unwrap();

    let mut sink = VecSink::new();
    let result = executor::execute(&script, &page, &HashMap::new(), &mut sink)
        .await
        .unwrap();

    // First step errored but continued
    assert_eq!(result.status, executor::ExecutionStatus::Ok);
    assert_eq!(sink.results[0].status, executor::ExecutionStatus::Error);
    assert_eq!(sink.results[1].status, executor::ExecutionStatus::Ok);
    assert_eq!(result.outputs[0]["text"], "Success");
}

#[tokio::test]
async fn execute_jsonl_output() {
    let page = page_with_html(r##"<h1>Test</h1>"##);

    let script = parser::parse_yaml(
        r##"
name: "JSONL test"
steps:
  - extract:
      selector: "h1"
      field: text_content
      save_as: title
"##,
    )
    .unwrap();

    let mut buf = Vec::new();
    {
        let mut sink = pwright_script::output::JsonlSink::new(&mut buf);
        let result = executor::execute(&script, &page, &HashMap::new(), &mut sink)
            .await
            .unwrap();
        sink.write_summary(&script.name, &result).unwrap();
    }

    let output = String::from_utf8(buf).unwrap();
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 2); // step result + summary

    let step: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(step["step_type"], "extract");
    assert_eq!(step["status"], "ok");

    let summary: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(summary["summary"], true);
    assert_eq!(summary["status"], "ok");
}

#[tokio::test]
async fn execute_wait_step() {
    let page = page_with_html(r##"<div>test</div>"##);

    let script = parser::parse_yaml(
        r##"
name: "Wait test"
steps:
  - wait: 50
  - extract:
      selector: "div"
      field: text_content
      save_as: text
"##,
    )
    .unwrap();

    let mut sink = VecSink::new();
    let start = std::time::Instant::now();
    let result = executor::execute(&script, &page, &HashMap::new(), &mut sink)
        .await
        .unwrap();
    let elapsed = start.elapsed();

    assert_eq!(result.status, executor::ExecutionStatus::Ok);
    assert_eq!(result.succeeded, 2);
    assert_eq!(sink.results[0].step_type, "wait");
    assert_eq!(sink.results[0].details["duration_ms"], "50");
    assert!(
        elapsed.as_millis() >= 50,
        "should have waited at least 50ms"
    );
}

#[tokio::test]
async fn parse_wait_step() {
    let script = parser::parse_yaml(
        r##"
name: "Parse wait"
steps:
  - click: "button"
  - wait: 2000
  - click: "button"
"##,
    )
    .unwrap();

    assert_eq!(script.steps.len(), 3);
    assert_eq!(script.steps[1].kind.type_name(), "wait");
}
