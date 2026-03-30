//! YAML parser: converts YAML script files into the internal Script model.

use std::collections::HashMap;
use std::path::Path;

use crate::error::ScriptError;
use crate::model::{
    ClickStep, EvalStep, ExtractStep, FillStep, GotoStep, JsFunction, OnError, OutputStep,
    ParamDef, ParamType, PressStep, Script, ScriptConfig, Step, StepKind, WaitKind, WaitStep,
};

/// Parse a YAML string into a Script.
pub fn parse_yaml(yaml: &str) -> Result<Script, ScriptError> {
    let raw: serde_yaml::Value =
        serde_yaml::from_str(yaml).map_err(|e| ScriptError::Parse(e.to_string()))?;

    let name = raw["name"].as_str().unwrap_or("unnamed").to_string();
    let description = raw["description"].as_str().unwrap_or("").to_string();
    let version = raw["version"]
        .as_i64()
        .and_then(|n| i32::try_from(n).ok())
        .unwrap_or(1);

    let params = parse_params(&raw["params"])?;
    let config = parse_config(&raw["config"]);
    let scripts = parse_scripts(&raw["scripts"])?;
    let steps = parse_steps(&raw["steps"])?;

    Ok(Script {
        name,
        description,
        version,
        params,
        config,
        scripts,
        steps,
    })
}

/// Parse a YAML file into a Script.
pub fn parse_yaml_file(path: &Path) -> Result<Script, ScriptError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| ScriptError::Parse(format!("read {path:?}: {e}")))?;
    parse_yaml(&content)
}

/// Load parameters from a YAML file (key-value pairs).
pub fn load_param_file(path: &Path) -> Result<HashMap<String, String>, ScriptError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| ScriptError::Parse(format!("read {path:?}: {e}")))?;
    let raw: serde_yaml::Value =
        serde_yaml::from_str(&content).map_err(|e| ScriptError::Parse(e.to_string()))?;
    let mut params = HashMap::new();
    if let Some(map) = raw.as_mapping() {
        for (k, v) in map {
            if let Some(key) = k.as_str() {
                let val = match v {
                    serde_yaml::Value::String(s) => s.clone(),
                    serde_yaml::Value::Number(n) => n.to_string(),
                    serde_yaml::Value::Bool(b) => b.to_string(),
                    _ => serde_yaml::to_string(v).unwrap_or_default(),
                };
                params.insert(key.to_string(), val);
            }
        }
    }
    Ok(params)
}

fn parse_params(val: &serde_yaml::Value) -> Result<HashMap<String, ParamDef>, ScriptError> {
    let mut params = HashMap::new();
    if let Some(map) = val.as_mapping() {
        for (k, v) in map {
            let name = k
                .as_str()
                .ok_or_else(|| ScriptError::Parse("param key must be string".into()))?;
            let def = if let Some(m) = v.as_mapping() {
                let param_type = match yaml_map_str(m, "type").unwrap_or("string") {
                    "integer" => ParamType::Integer,
                    "boolean" => ParamType::Boolean,
                    // Default to String for "string" or any unrecognized type
                    _ => ParamType::String,
                };
                ParamDef {
                    param_type,
                    required: yaml_map_bool(m, "required").unwrap_or(false),
                    default_value: m
                        .get(serde_yaml::Value::String("default".into()))
                        .map(yaml_value_to_string),
                    description: yaml_map_str(m, "description").unwrap_or("").to_string(),
                }
            } else {
                // Short form: param_name: { type: string, required: true }
                ParamDef {
                    param_type: ParamType::String,
                    required: false,
                    default_value: Some(yaml_value_to_string(v)),
                    description: String::new(),
                }
            };
            params.insert(name.to_string(), def);
        }
    }
    Ok(params)
}

fn parse_config(val: &serde_yaml::Value) -> ScriptConfig {
    if val.is_null() {
        return ScriptConfig::default();
    }
    let default_on_error = match val["default_on_error"].as_str().unwrap_or("fail") {
        "continue" => OnError::Continue,
        "retry" => OnError::Retry,
        // Default to Fail for "fail" or any unrecognized value
        _ => OnError::Fail,
    };
    ScriptConfig {
        default_timeout_ms: val["default_timeout_ms"].as_i64().unwrap_or(30000).max(0) as u64,
        default_on_error,
    }
}

fn parse_scripts(val: &serde_yaml::Value) -> Result<HashMap<String, JsFunction>, ScriptError> {
    let mut scripts = HashMap::new();
    if let Some(map) = val.as_mapping() {
        for (k, v) in map {
            let name = k
                .as_str()
                .ok_or_else(|| ScriptError::Parse("script key must be string".into()))?;
            // Scripts can be a plain string (body) or an object with body + async flag.
            //   simple:    my_script: "return document.title"
            //   explicit:  my_script: { body: "await fetch(...)", async: true }
            let (body, is_async) = if let Some(s) = v.as_str() {
                (s.to_string(), false)
            } else if v.is_mapping() {
                let body = v["body"]
                    .as_str()
                    .ok_or_else(|| {
                        ScriptError::Parse(format!(
                            "script '{name}' object must have 'body' string"
                        ))
                    })?
                    .to_string();
                let is_async = v["async"].as_bool().unwrap_or(false);
                (body, is_async)
            } else {
                return Err(ScriptError::Parse(format!(
                    "script '{name}' must be a string or object with 'body'"
                )));
            };
            scripts.insert(
                name.to_string(),
                JsFunction {
                    body,
                    description: String::new(),
                    is_async,
                },
            );
        }
    }
    Ok(scripts)
}

fn parse_steps(val: &serde_yaml::Value) -> Result<Vec<Step>, ScriptError> {
    let arr = val
        .as_sequence()
        .ok_or_else(|| ScriptError::Parse("steps must be an array".into()))?;
    arr.iter()
        .enumerate()
        .map(|(i, v)| parse_step(v, i))
        .collect()
}

fn parse_step(val: &serde_yaml::Value, index: usize) -> Result<Step, ScriptError> {
    let on_error = match val["on_error"].as_str().unwrap_or("fail") {
        "continue" => OnError::Continue,
        "retry" => OnError::Retry,
        _ => OnError::Fail,
    };

    let kind = if let Some(url) = val["goto"].as_str() {
        StepKind::Goto(GotoStep {
            url: url.to_string(),
            wait_for: val["wait_for"].as_str().map(|s| s.to_string()),
            timeout_ms: val["timeout_ms"].as_i64().map(|n| n.max(0) as u64),
        })
    } else if let Some(selector) = val["click"].as_str() {
        StepKind::Click(ClickStep {
            selector: selector.to_string(),
            wait_for: val["wait_for"].as_str().map(|s| s.to_string()),
        })
    } else if val["fill"].is_mapping() {
        let m = &val["fill"];
        StepKind::Fill(FillStep {
            selector: m["selector"]
                .as_str()
                .ok_or_else(|| ScriptError::Parse(format!("step {index}: fill.selector required")))?
                .to_string(),
            value: m["value"]
                .as_str()
                .ok_or_else(|| ScriptError::Parse(format!("step {index}: fill.value required")))?
                .to_string(),
        })
    } else if let Some(key) = val["press"].as_str() {
        StepKind::Press(PressStep {
            key: key.to_string(),
        })
    } else if val["extract"].is_mapping() {
        let m = &val["extract"];
        StepKind::Extract(ExtractStep {
            selector: m["selector"]
                .as_str()
                .ok_or_else(|| {
                    ScriptError::Parse(format!("step {index}: extract.selector required"))
                })?
                .to_string(),
            field: m["field"]
                .as_str()
                .ok_or_else(|| ScriptError::Parse(format!("step {index}: extract.field required")))?
                .to_string(),
            save_as: m["save_as"].as_str().map(|s| s.to_string()),
        })
    } else if val["eval"].is_mapping() {
        let m = &val["eval"];
        StepKind::Eval(EvalStep {
            expression: m["expression"].as_str().map(|s| s.to_string()),
            js_ref: m["ref"].as_str().map(|s| s.to_string()),
            save_as: m["save_as"].as_str().map(|s| s.to_string()),
            args: m["args"]
                .as_sequence()
                .map(|arr| arr.iter().map(yaml_value_to_string).collect())
                .unwrap_or_default(),
        })
    } else if val["output"].is_mapping() {
        let m = &val["output"];
        let mut fields = HashMap::new();
        if let Some(map) = m.as_mapping() {
            for (k, v) in map {
                if let Some(key) = k.as_str() {
                    fields.insert(key.to_string(), yaml_value_to_string(v));
                }
            }
        }
        StepKind::Output(OutputStep { fields })
    } else if let Some(ms) = val["wait"].as_i64() {
        StepKind::Wait(WaitStep {
            kind: WaitKind::Duration(ms.max(0) as u64),
        })
    } else if let Some(ms) = val["wait"].as_f64() {
        StepKind::Wait(WaitStep {
            kind: WaitKind::Duration(ms.max(0.0) as u64),
        })
    } else if let Some(text) = val["wait_for_text"].as_str() {
        let timeout_ms = val["timeout_ms"]
            .as_i64()
            .map(|n| n.max(0) as u64)
            .unwrap_or(30000);
        StepKind::Wait(WaitStep {
            kind: WaitKind::Text {
                text: text.to_string(),
                timeout_ms,
            },
        })
    } else if val["wait_for"].is_string() && val.get("goto").is_none() {
        // Standalone wait_for (not the goto.wait_for field)
        let selector = val["wait_for"]
            .as_str()
            .ok_or_else(|| ScriptError::Parse("wait_for must be a string".into()))?
            .to_string();
        let timeout_ms = val["timeout_ms"]
            .as_i64()
            .map(|n| n.max(0) as u64)
            .unwrap_or(30000);
        StepKind::Wait(WaitStep {
            kind: WaitKind::Selector {
                selector,
                timeout_ms,
            },
        })
    } else if let Some(js) = val["wait_until"].as_str() {
        let timeout_ms = val["timeout_ms"]
            .as_i64()
            .map(|n| n.max(0) as u64)
            .unwrap_or(30000);
        StepKind::Wait(WaitStep {
            kind: WaitKind::Expression {
                js: js.to_string(),
                timeout_ms,
            },
        })
    } else {
        return Err(ScriptError::Parse(format!(
            "step {index}: unrecognized step type"
        )));
    };

    Ok(Step { kind, on_error })
}

fn yaml_map_str<'a>(m: &'a serde_yaml::Mapping, key: &str) -> Option<&'a str> {
    m.get(serde_yaml::Value::String(key.into()))
        .and_then(|v| v.as_str())
}

fn yaml_map_bool(m: &serde_yaml::Mapping, key: &str) -> Option<bool> {
    m.get(serde_yaml::Value::String(key.into()))
        .and_then(|v| v.as_bool())
}

fn yaml_value_to_string(v: &serde_yaml::Value) -> String {
    match v {
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Null => String::new(),
        _ => serde_yaml::to_string(v).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_script() {
        let yaml = r#"
name: "Test"
params:
  url:
    type: string
    required: true
steps:
  - goto: "{{ url }}"
  - extract:
      selector: "h1"
      field: text_content
      save_as: title
"#;
        let script = parse_yaml(yaml).unwrap();
        assert_eq!(script.name, "Test");
        assert_eq!(script.params.len(), 1);
        assert!(script.params["url"].required);
        assert_eq!(script.steps.len(), 2);
        assert!(matches!(&script.steps[0].kind, StepKind::Goto(_)));
        assert!(matches!(&script.steps[1].kind, StepKind::Extract(_)));
    }

    #[test]
    fn parse_script_with_js_registry() {
        let yaml = r#"
name: "JS test"
scripts:
  get_title: |
    document.title
steps:
  - eval:
      ref: get_title
      save_as: title
"#;
        let script = parse_yaml(yaml).unwrap();
        assert_eq!(script.scripts.len(), 1);
        assert!(script.scripts.contains_key("get_title"));
        if let StepKind::Eval(e) = &script.steps[0].kind {
            assert_eq!(e.js_ref.as_deref(), Some("get_title"));
            assert_eq!(e.save_as.as_deref(), Some("title"));
        } else {
            panic!("expected eval step");
        }
    }

    #[test]
    fn parse_all_step_types() {
        let yaml = r##"
name: "All steps"
steps:
  - goto: "https://example.com"
    wait_for: "h1"
  - click: "button"
  - fill:
      selector: "#email"
      value: "user@test.example"
  - press: "Enter"
  - extract:
      selector: "h1"
      field: text_content
      save_as: title
  - eval:
      expression: "1+1"
      save_as: result
  - output:
      title: "{{ title }}"
"##;
        let script = parse_yaml(yaml).unwrap();
        assert_eq!(script.steps.len(), 7);
    }

    #[test]
    fn parse_invalid_yaml() {
        let result = parse_yaml("not: valid: yaml: [");
        assert!(result.is_err());
    }

    #[test]
    fn parse_missing_steps() {
        let result = parse_yaml("name: test\n");
        assert!(result.is_err());
    }

    #[test]
    fn parse_config() {
        let yaml = r#"
name: "Config test"
config:
  default_timeout_ms: 5000
  default_on_error: continue
steps:
  - goto: "https://example.com"
"#;
        let script = parse_yaml(yaml).unwrap();
        assert_eq!(script.config.default_timeout_ms, 5000);
        assert_eq!(script.config.default_on_error, OnError::Continue);
    }
}
