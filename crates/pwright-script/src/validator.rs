//! Script validator: checks schema, params, template refs, and JS registry.

use std::collections::{HashMap, HashSet};

use crate::model::{Script, StepKind};

/// Validate a parsed script with the given parameter values.
pub fn validate(
    script: &Script,
    param_values: &HashMap<String, String>,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    // 1. Check required params are supplied
    for (name, def) in &script.params {
        if def.required && !param_values.contains_key(name) && def.default_value.is_none() {
            errors.push(format!("missing required param: '{name}'"));
        }
    }

    // 2. Collect known variables (params + defaults)
    let mut known_vars: HashSet<String> = param_values.keys().cloned().collect();
    for (name, def) in &script.params {
        if let Some(ref default) = def.default_value
            && !default.is_empty()
        {
            known_vars.insert(name.clone());
        }
    }

    // 3. Check each step
    for (i, step) in script.steps.iter().enumerate() {
        // Validate step-specific fields
        match &step.kind {
            StepKind::Goto(g) => {
                if g.url.is_empty() {
                    errors.push(format!("step {i}: goto.url is empty"));
                }
                check_template_refs(&g.url, &known_vars, i, "goto.url", &mut errors);
            }
            StepKind::Click(c) => {
                if c.selector.is_empty() {
                    errors.push(format!("step {i}: click.selector is empty"));
                }
            }
            StepKind::Fill(f) => {
                if f.selector.is_empty() {
                    errors.push(format!("step {i}: fill.selector is empty"));
                }
                check_template_refs(&f.value, &known_vars, i, "fill.value", &mut errors);
            }
            StepKind::Press(p) => {
                if p.key.is_empty() {
                    errors.push(format!("step {i}: press.key is empty"));
                }
            }
            StepKind::Extract(e) => {
                if e.selector.is_empty() {
                    errors.push(format!("step {i}: extract.selector is empty"));
                }
                if e.field.is_empty() {
                    errors.push(format!("step {i}: extract.field is empty"));
                }
                // Register save_as as known variable
                if let Some(ref var) = e.save_as {
                    known_vars.insert(var.clone());
                }
            }
            StepKind::Eval(e) => {
                if e.expression.is_none() && e.js_ref.is_none() {
                    errors.push(format!(
                        "step {i}: eval must have either 'expression' or 'ref'"
                    ));
                }
                // Check JS registry ref
                if let Some(ref js_ref) = e.js_ref
                    && !script.scripts.contains_key(js_ref)
                {
                    errors.push(format!(
                        "step {i}: eval.ref '{js_ref}' not found in scripts registry"
                    ));
                }
                if let Some(ref var) = e.save_as {
                    known_vars.insert(var.clone());
                }
            }
            StepKind::Output(o) => {
                for (key, val) in &o.fields {
                    check_template_refs(val, &known_vars, i, &format!("output.{key}"), &mut errors);
                }
            }
            StepKind::Wait(_) => {} // no validation needed
        }

        // Validate on_error
        if !matches!(step.on_error.as_str(), "fail" | "continue" | "retry") {
            errors.push(format!(
                "step {i}: on_error must be 'fail', 'continue', or 'retry', got '{}'",
                step.on_error
            ));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Check that all {{ var }} references in a template string resolve to known variables.
fn check_template_refs(
    template: &str,
    known: &HashSet<String>,
    step_index: usize,
    field: &str,
    errors: &mut Vec<String>,
) {
    let mut pos = 0;
    while let Some(start) = template[pos..].find("{{") {
        let abs_start = pos + start + 2;
        if let Some(end) = template[abs_start..].find("}}") {
            let var_name = template[abs_start..abs_start + end].trim();
            if !var_name.is_empty() && !known.contains(var_name) {
                errors.push(format!(
                    "step {step_index}: {field} references unknown variable '{var_name}'"
                ));
            }
            pos = abs_start + end + 2;
        } else {
            errors.push(format!(
                "step {step_index}: {field} has unclosed '{{{{' template"
            ));
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_yaml;

    #[test]
    fn valid_script_passes() {
        let script = parse_yaml(
            r#"
name: test
params:
  url: { type: string, required: true }
steps:
  - goto: "{{ url }}"
  - extract:
      selector: "h1"
      field: text_content
      save_as: title
  - output:
      title: "{{ title }}"
"#,
        )
        .unwrap();

        let params = HashMap::from([("url".into(), "https://example.com".into())]);
        assert!(validate(&script, &params).is_ok());
    }

    #[test]
    fn missing_required_param() {
        let script = parse_yaml(
            r#"
name: test
params:
  url: { type: string, required: true }
steps:
  - goto: "{{ url }}"
"#,
        )
        .unwrap();

        let errors = validate(&script, &HashMap::new()).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("missing required param")));
    }

    #[test]
    fn unknown_template_variable() {
        let script = parse_yaml(
            r#"
name: test
steps:
  - goto: "{{ nonexistent }}"
"#,
        )
        .unwrap();

        let errors = validate(&script, &HashMap::new()).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.contains("unknown variable 'nonexistent'"))
        );
    }

    #[test]
    fn invalid_js_ref() {
        let script = parse_yaml(
            r#"
name: test
steps:
  - eval:
      ref: missing_script
      save_as: result
"#,
        )
        .unwrap();

        let errors = validate(&script, &HashMap::new()).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.contains("'missing_script' not found in scripts registry"))
        );
    }

    #[test]
    fn valid_js_ref() {
        let script = parse_yaml(
            r#"
name: test
scripts:
  get_title: "document.title"
steps:
  - eval:
      ref: get_title
      save_as: title
"#,
        )
        .unwrap();

        assert!(validate(&script, &HashMap::new()).is_ok());
    }

    #[test]
    fn empty_selector_rejected() {
        let script = parse_yaml(
            r#"
name: test
steps:
  - click: ""
"#,
        )
        .unwrap();

        let errors = validate(&script, &HashMap::new()).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("selector is empty")));
    }

    #[test]
    fn save_as_creates_known_variable() {
        let script = parse_yaml(
            r#"
name: test
steps:
  - extract:
      selector: "h1"
      field: text_content
      save_as: title
  - output:
      title: "{{ title }}"
"#,
        )
        .unwrap();

        assert!(validate(&script, &HashMap::new()).is_ok());
    }
}
