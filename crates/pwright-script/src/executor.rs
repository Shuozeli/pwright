//! Script executor: runs script steps against a pwright Page.

use std::collections::HashMap;
use std::time::Instant;

use pwright_bridge::playwright::Page;

use crate::error::ScriptError;
use crate::model::{OnError, Script, StepKind};
use crate::output::{OutputSink, StepResult};

/// Execute a script against a Page, streaming results to the sink.
pub async fn execute(
    script: &Script,
    page: &Page,
    params: &HashMap<String, String>,
    sink: &mut dyn OutputSink,
) -> Result<ExecutionResult, ScriptError> {
    let mut vars: HashMap<String, serde_json::Value> = HashMap::new();
    let mut outputs: Vec<HashMap<String, String>> = Vec::new();

    // Initialize vars from params (with defaults)
    for (name, def) in &script.params {
        if let Some(val) = params.get(name) {
            vars.insert(name.clone(), serde_json::Value::String(val.clone()));
        } else if let Some(ref default) = def.default_value {
            vars.insert(name.clone(), serde_json::Value::String(default.clone()));
        }
    }

    let mut succeeded = 0;
    let mut failed = 0;
    let script_start = Instant::now();

    for (i, step) in script.steps.iter().enumerate() {
        let step_start = Instant::now();
        let step_type = step.kind.type_name().to_string();

        let result = execute_step(&step.kind, page, &mut vars, &script.scripts, &mut outputs).await;

        let duration_ms = step_start.elapsed().as_millis() as u64;

        match result {
            Ok(details) => {
                succeeded += 1;
                sink.emit(StepResult {
                    step_index: i as u32,
                    step_type,
                    status: ExecutionStatus::Ok,
                    duration_ms,
                    details,
                    error: None,
                })?;
            }
            Err(e) => {
                let err_msg = e.to_string();
                failed += 1;
                sink.emit(StepResult {
                    step_index: i as u32,
                    step_type,
                    status: ExecutionStatus::Error,
                    duration_ms,
                    details: HashMap::new(),
                    error: Some(err_msg.clone()),
                })?;
                if step.on_error != OnError::Continue {
                    return Ok(ExecutionResult {
                        status: ExecutionStatus::Error,
                        total_steps: script.steps.len() as u32,
                        succeeded,
                        failed,
                        skipped: (script.steps.len() - i - 1) as u32,
                        duration_ms: script_start.elapsed().as_millis() as u64,
                        outputs,
                        error: Some(format!("step {i} failed: {err_msg}")),
                    });
                }
            }
        }
    }

    Ok(ExecutionResult {
        status: ExecutionStatus::Ok,
        total_steps: script.steps.len() as u32,
        succeeded,
        failed,
        skipped: 0,
        duration_ms: script_start.elapsed().as_millis() as u64,
        outputs,
        error: None,
    })
}

async fn execute_step(
    kind: &StepKind,
    page: &Page,
    vars: &mut HashMap<String, serde_json::Value>,
    scripts: &HashMap<String, crate::model::JsFunction>,
    outputs: &mut Vec<HashMap<String, String>>,
) -> Result<HashMap<String, String>, ScriptError> {
    let mut details = HashMap::new();

    match kind {
        StepKind::Goto(g) => {
            let url = resolve_template(&g.url, vars);
            details.insert("url".into(), url.clone());

            let opts = g
                .wait_for
                .as_ref()
                .map(|_sel| pwright_bridge::playwright::GotoOptions {
                    wait_until: pwright_bridge::playwright::WaitUntil::DomContentLoaded,
                    timeout_ms: g.timeout_ms,
                });
            page.goto(&url, opts).await.map_err(ScriptError::Cdp)?;

            if let Some(ref sel) = g.wait_for {
                let selector = resolve_template(sel, vars);
                let timeout = g.timeout_ms.unwrap_or(30000);
                page.wait_for_selector(&selector, timeout)
                    .await
                    .map_err(ScriptError::Cdp)?;
            }
        }

        StepKind::Click(c) => {
            let selector = resolve_template(&c.selector, vars);
            details.insert("selector".into(), selector.clone());
            page.click(&selector).await.map_err(ScriptError::Cdp)?;
        }

        StepKind::Fill(f) => {
            let selector = resolve_template(&f.selector, vars);
            let value = resolve_template(&f.value, vars);
            details.insert("selector".into(), selector.clone());
            page.fill(&selector, &value)
                .await
                .map_err(ScriptError::Cdp)?;
        }

        StepKind::Press(p) => {
            let key = resolve_template(&p.key, vars);
            details.insert("key".into(), key.clone());
            page.keyboard()
                .press(&key)
                .await
                .map_err(ScriptError::Cdp)?;
        }

        StepKind::Extract(e) => {
            let selector = resolve_template(&e.selector, vars);
            details.insert("selector".into(), selector.clone());
            details.insert("field".into(), e.field.clone());

            let value = extract_field(page, &selector, &e.field).await?;
            details.insert("value".into(), value.clone());

            if let Some(ref var_name) = e.save_as {
                vars.insert(var_name.clone(), serde_json::Value::String(value));
            }
        }

        StepKind::Eval(e) => {
            let value = if let Some(ref js_ref) = e.js_ref {
                // Look up in scripts registry
                let func = scripts.get(js_ref).ok_or_else(|| {
                    ScriptError::Validation(format!("script '{js_ref}' not found"))
                })?;
                details.insert("ref".into(), js_ref.clone());

                if e.args.is_empty() {
                    // Simple eval (use evaluate_async if script contains await)
                    let result = if func.is_async {
                        page.evaluate_async(&func.body)
                            .await
                            .map_err(ScriptError::Cdp)?
                    } else {
                        page.evaluate(&func.body).await.map_err(ScriptError::Cdp)?
                    };
                    json_value_to_string(&result)
                } else {
                    // Call with args
                    let args: Vec<String> =
                        e.args.iter().map(|a| resolve_template(a, vars)).collect();
                    let arg_json = serde_json::Value::Array(
                        args.iter()
                            .map(|a| serde_json::Value::String(a.clone()))
                            .collect(),
                    );
                    let result = page
                        .evaluate_with_arg(&func.body, &arg_json)
                        .await
                        .map_err(ScriptError::Cdp)?;
                    json_value_to_string(&result)
                }
            } else if let Some(ref expr) = e.expression {
                let resolved = resolve_template(expr, vars);
                details.insert("expression".into(), resolved.clone());
                let result = page.evaluate(&resolved).await.map_err(ScriptError::Cdp)?;
                json_value_to_string(&result)
            } else {
                return Err(ScriptError::Validation(
                    "eval: need expression or ref".into(),
                ));
            };

            details.insert("value".into(), value.clone());
            if let Some(ref var_name) = e.save_as {
                vars.insert(var_name.clone(), serde_json::Value::String(value));
            }
        }

        StepKind::Wait(w) => {
            details.insert("duration_ms".into(), w.duration_ms.to_string());
            tokio::time::sleep(std::time::Duration::from_millis(w.duration_ms)).await;
        }

        StepKind::Output(o) => {
            let mut row = HashMap::new();
            for (key, val_template) in &o.fields {
                let resolved = resolve_template(val_template, vars);
                row.insert(key.clone(), resolved);
            }
            details = row.clone();
            outputs.push(row);
        }
    }

    Ok(details)
}

async fn extract_field(page: &Page, selector: &str, field: &str) -> Result<String, ScriptError> {
    let loc = page.locator(selector);
    match field {
        "text_content" => Ok(loc
            .text_content()
            .await
            .map_err(ScriptError::Cdp)?
            .unwrap_or_default()),
        "inner_text" => Ok(loc.inner_text().await.map_err(ScriptError::Cdp)?),
        "inner_html" => Ok(loc.inner_html().await.map_err(ScriptError::Cdp)?),
        "input_value" => Ok(loc.input_value().await.map_err(ScriptError::Cdp)?),
        "is_visible" => Ok(loc
            .is_visible()
            .await
            .map_err(ScriptError::Cdp)?
            .to_string()),
        "is_checked" => Ok(loc
            .is_checked()
            .await
            .map_err(ScriptError::Cdp)?
            .to_string()),
        "is_disabled" => Ok(loc
            .is_disabled()
            .await
            .map_err(ScriptError::Cdp)?
            .to_string()),
        f if f.starts_with("attribute:") => {
            let attr_name = &f["attribute:".len()..];
            Ok(loc
                .get_attribute(attr_name)
                .await
                .map_err(ScriptError::Cdp)?
                .unwrap_or_default())
        }
        _ => Err(ScriptError::Validation(format!("unknown field: '{field}'"))),
    }
}

/// Resolve {{ var }} templates in a single pass.
/// Substituted values are NOT re-processed, preventing injection via
/// user input containing `{{ other_var }}`.
fn resolve_template(template: &str, vars: &HashMap<String, serde_json::Value>) -> String {
    let mut result = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        result.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        if let Some(end) = after.find("}}") {
            let var_name = after[..end].trim();
            let value = match vars.get(var_name) {
                Some(serde_json::Value::String(s)) => s.clone(),
                Some(other) => other.to_string(),
                None => String::new(), // Validator catches unknown refs; runtime allows empty for optional vars
            };
            result.push_str(&value);
            rest = &after[end + 2..];
        } else {
            result.push_str("{{");
            rest = after;
        }
    }
    result.push_str(rest);
    result
}

fn json_value_to_string(v: &serde_json::Value) -> String {
    match v.get("value") {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
        None => v.to_string(),
    }
}

/// Execution outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Ok,
    Error,
}

/// Result of script execution.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub status: ExecutionStatus,
    pub total_steps: u32,
    pub succeeded: u32,
    pub failed: u32,
    pub skipped: u32,
    pub duration_ms: u64,
    pub outputs: Vec<HashMap<String, String>>,
    pub error: Option<String>,
}
