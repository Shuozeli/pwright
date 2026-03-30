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
            // SAFETY: URL is from user-authored script + user-supplied params.
            // No escaping applied; caller is responsible for param values.
            let url = resolve_template(&g.url, vars);
            details.insert("url".into(), url.clone());

            let opts = g
                .wait_for
                .as_ref()
                .map(|_sel| pwright_bridge::playwright::GotoOptions {
                    wait_until: pwright_bridge::playwright::WaitStrategy::Dom,
                    timeout_ms: g.timeout_ms,
                });
            page.goto(&url, opts).await.map_err(ScriptError::Cdp)?;

            if let Some(ref sel) = g.wait_for {
                // SAFETY: CSS selector from user-authored script + params.
                let selector = resolve_template(sel, vars);
                let timeout = g.timeout_ms.unwrap_or(30000);
                page.wait_for_selector(&selector, timeout)
                    .await
                    .map_err(ScriptError::Cdp)?;
            }
        }

        StepKind::Click(c) => {
            // SAFETY: CSS selector from user-authored script + params.
            let selector = resolve_template(&c.selector, vars);
            details.insert("selector".into(), selector.clone());
            page.click(&selector).await.map_err(ScriptError::Cdp)?;
        }

        StepKind::Fill(f) => {
            // SAFETY: CSS selector + fill value from user-authored script + params.
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
            // SAFETY: CSS selector from user-authored script + params.
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
                    // SAFETY: args are resolved from user-authored templates, then
                    // passed as typed JSON values to callFunctionOn (not string-interpolated
                    // into JS source). This is the safer path for parameterized JS.
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
                // SAFETY: resolved JS expression is passed directly to Runtime.evaluate.
                // Param values are string-interpolated into the JS source with no escaping.
                // This is intentional -- see resolve_template doc comment for rationale.
                // Prefer eval with ref + args over expression + templates for untrusted values.
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

        StepKind::Wait(w) => match &w.kind {
            crate::model::WaitKind::Duration(ms) => {
                details.insert("duration_ms".into(), ms.to_string());
                tokio::time::sleep(std::time::Duration::from_millis(*ms)).await;
            }
            crate::model::WaitKind::Text { text, timeout_ms } => {
                details.insert("text".into(), text.clone());
                details.insert("timeout_ms".into(), timeout_ms.to_string());
                page.wait_for_text(text, *timeout_ms)
                    .await
                    .map_err(ScriptError::Cdp)?;
            }
            crate::model::WaitKind::Selector {
                selector,
                timeout_ms,
            } => {
                details.insert("selector".into(), selector.clone());
                details.insert("timeout_ms".into(), timeout_ms.to_string());
                page.wait_for_selector(selector, *timeout_ms)
                    .await
                    .map_err(ScriptError::Cdp)?;
            }
            crate::model::WaitKind::Expression { js, timeout_ms } => {
                details.insert("expression".into(), js.clone());
                details.insert("timeout_ms".into(), timeout_ms.to_string());
                page.wait_until(js, *timeout_ms)
                    .await
                    .map_err(ScriptError::Cdp)?;
            }
        },

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

/// Resolve `{{ var }}` templates in a single pass.
///
/// Substituted values are NOT re-processed, preventing injection via
/// user input containing `{{ other_var }}`.
///
/// # Security: template injection in JS contexts
///
/// This function performs plain string substitution with no escaping. When the
/// resolved string is passed to `page.evaluate()` or similar JS execution
/// methods, a param value can inject arbitrary JavaScript. For example, a param
/// `name` with value `"; alert(1); "` inserted into `"return '{{ name }}'"` would
/// produce valid JS that executes the injected code.
///
/// This is **by design**. The script runner is a trusted-input tool: the user
/// authors both the YAML script and the parameters. It is the caller's
/// responsibility to ensure param values are safe for their intended context.
/// We deliberately do not apply heuristic escaping here because:
///
/// 1. The caller may *intend* to pass JS expressions as param values.
/// 2. Automatic escaping would silently break legitimate use cases.
/// 3. Per CLAUDE.md rules: "Do NOT preprocess, inspect, or heuristically analyze
///    user-supplied input to infer behavior."
///
/// If a param value must be treated as a JS string literal, the script author
/// should use `serde_json::to_string()` or wrap the value in the `scripts`
/// registry function that accepts arguments via `args` (which passes values
/// through CDP's typed argument mechanism, avoiding string interpolation
/// entirely).
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
                None => {
                    tracing::warn!(
                        var = var_name,
                        "unresolved template variable, substituting empty string"
                    );
                    String::new()
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── resolve_template ──

    #[test]
    fn template_basic_substitution() {
        let mut vars = HashMap::new();
        vars.insert("name".into(), serde_json::json!("Alice"));
        assert_eq!(resolve_template("Hello {{ name }}!", &vars), "Hello Alice!");
    }

    #[test]
    fn template_multiple_vars() {
        let mut vars = HashMap::new();
        vars.insert("a".into(), serde_json::json!("X"));
        vars.insert("b".into(), serde_json::json!("Y"));
        assert_eq!(resolve_template("{{ a }}-{{ b }}", &vars), "X-Y");
    }

    #[test]
    fn template_adjacent_vars() {
        let mut vars = HashMap::new();
        vars.insert("x".into(), serde_json::json!("1"));
        vars.insert("y".into(), serde_json::json!("2"));
        assert_eq!(resolve_template("{{ x }}{{ y }}", &vars), "12");
    }

    #[test]
    fn template_no_vars() {
        let vars = HashMap::new();
        assert_eq!(resolve_template("plain text", &vars), "plain text");
    }

    #[test]
    fn template_empty_string() {
        let vars = HashMap::new();
        assert_eq!(resolve_template("", &vars), "");
    }

    #[test]
    fn template_unknown_var_becomes_empty() {
        let vars = HashMap::new();
        assert_eq!(resolve_template("{{ missing }}", &vars), "");
    }

    #[test]
    fn template_empty_var_name() {
        let vars = HashMap::new();
        // {{ }} has empty var name after trim — not in vars, resolves to empty
        assert_eq!(resolve_template("a{{ }}b", &vars), "ab");
    }

    #[test]
    fn template_unclosed_brace_preserved() {
        let vars = HashMap::new();
        // {{ without }} is preserved as literal text
        assert_eq!(
            resolve_template("start {{ no close", &vars),
            "start {{ no close"
        );
    }

    #[test]
    fn template_triple_braces() {
        let mut vars = HashMap::new();
        vars.insert("x".into(), serde_json::json!("val"));
        // {{{ x }}} -> first {{ at pos 0 consumes "{ x " as var name (trimmed to "{ x"),
        // which is not in vars, so empty. Then remaining is "}".
        assert_eq!(resolve_template("{{{ x }}}", &vars), "}");
    }

    #[test]
    fn template_no_reprocessing() {
        // Value contains template syntax — must NOT be re-expanded
        let mut vars = HashMap::new();
        vars.insert("a".into(), serde_json::json!("{{ b }}"));
        vars.insert("b".into(), serde_json::json!("LEAKED"));
        assert_eq!(resolve_template("{{ a }}", &vars), "{{ b }}");
    }

    #[test]
    fn template_integer_value() {
        let mut vars = HashMap::new();
        vars.insert("count".into(), serde_json::json!(42));
        assert_eq!(resolve_template("items: {{ count }}", &vars), "items: 42");
    }

    #[test]
    fn template_boolean_value() {
        let mut vars = HashMap::new();
        vars.insert("flag".into(), serde_json::json!(true));
        assert_eq!(resolve_template("{{ flag }}", &vars), "true");
    }

    #[test]
    fn template_whitespace_around_var_name() {
        let mut vars = HashMap::new();
        vars.insert("x".into(), serde_json::json!("ok"));
        assert_eq!(resolve_template("{{x}}", &vars), "ok");
        assert_eq!(resolve_template("{{  x  }}", &vars), "ok");
    }

    // ── json_value_to_string ──

    #[test]
    fn json_value_string_extracted() {
        let v = serde_json::json!({"value": "hello"});
        assert_eq!(json_value_to_string(&v), "hello");
    }

    #[test]
    fn json_value_number_to_string() {
        let v = serde_json::json!({"value": 42});
        assert_eq!(json_value_to_string(&v), "42");
    }

    #[test]
    fn json_value_bool_to_string() {
        let v = serde_json::json!({"value": true});
        assert_eq!(json_value_to_string(&v), "true");
    }

    #[test]
    fn json_value_null_to_string() {
        let v = serde_json::json!({"value": null});
        assert_eq!(json_value_to_string(&v), "null");
    }

    #[test]
    fn json_value_missing_uses_whole_value() {
        let v = serde_json::json!({"type": "number", "result": 5});
        // No "value" key — falls through to v.to_string()
        let s = json_value_to_string(&v);
        assert!(s.contains("number"));
        assert!(s.contains("5"));
    }

    // ── ExecutionStatus ──

    #[test]
    fn execution_status_serializes() {
        assert_eq!(
            serde_json::to_string(&ExecutionStatus::Ok).unwrap(),
            r#""ok""#
        );
        assert_eq!(
            serde_json::to_string(&ExecutionStatus::Error).unwrap(),
            r#""error""#
        );
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
