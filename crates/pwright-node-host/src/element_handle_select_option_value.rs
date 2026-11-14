//! Opt-in value-string selectOption on the original retained ElementHandle.
//! This deliberately does not claim descriptor, option-Handle, force, or
//! navigation-commit parity with Playwright's complete selectOption API.

use std::time::Duration;

use pwright_backend_cdp_selector_runtime::{
    SELECTOR_RUNTIME_SYMBOL, SELECTOR_RUNTIME_VERSION, selector_runtime_install_script,
};

use crate::{
    HostError, HostSession,
    evaluation::{self, EvaluationValue},
    handles::{OwnedHandleArgument, PendingHandle},
};

const MAX_VALUE_BYTES: usize = 64 * 1024;
const MAX_DOM_OPTIONS: usize = 4096;
const RETRY_DELAY: Duration = Duration::from_millis(50);

enum SelectAttempt {
    Done(String),
    Retry,
}

impl HostSession {
    /// Select one value or label through this exact retained DOM owner.
    /// In particular, a replacement `<select>` with the same CSS selector is
    /// never eligible. The original creation world's timeout policy applies.
    pub async fn element_handle_select_option_value(
        &self,
        target: &PendingHandle,
        value: String,
        timeout_ms: Option<u32>,
    ) -> Result<Vec<String>, HostError> {
        if !target.is_element() {
            return Err(HostError::InvalidArgument(
                "select_option_value requires an ElementHandle".into(),
            ));
        }
        validate_value(&value, value.capacity())?;
        let timeout_ms = match timeout_ms {
            Some(timeout_ms) => timeout_ms,
            None => {
                let world = target.source_world().ok_or_else(|| {
                    HostError::Backend("Handle creation-world identity is missing".into())
                })?;
                self.page_default_timeout(&world.page_id().get().to_string(), false)?
            }
        };
        let deadline = (timeout_ms != 0)
            .then(|| tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms)));
        let source = selection_source()?;
        loop {
            let argument = OwnedHandleArgument::from_slots(
                EvaluationValue::String(value.clone()),
                Vec::new(),
            )?;
            let evaluation = self.handle_evaluate_owned(target, source.clone(), argument);
            let observed = match deadline {
                Some(deadline) => tokio::time::timeout_at(deadline, evaluation)
                    .await
                    .map_err(|_| HostError::Timeout(timeout_ms))??,
                None => evaluation.await?,
            };
            match parse_attempt(observed)? {
                SelectAttempt::Done(value) => return Ok(vec![value]),
                SelectAttempt::Retry => {}
            }
            let delay = deadline.map_or(RETRY_DELAY, |deadline| {
                RETRY_DELAY.min(deadline.saturating_duration_since(tokio::time::Instant::now()))
            });
            if delay.is_zero() {
                return Err(HostError::Timeout(timeout_ms));
            }
            tokio::time::sleep(delay).await;
        }
    }
}

fn validate_value(value: &str, retained_capacity: usize) -> Result<(), HostError> {
    if value.len() > MAX_VALUE_BYTES || retained_capacity > MAX_VALUE_BYTES {
        return Err(HostError::InvalidArgument(format!(
            "selectOption value exceeds {MAX_VALUE_BYTES} UTF-8 bytes"
        )));
    }
    Ok(())
}

fn selection_source() -> Result<String, HostError> {
    let install = selector_runtime_install_script();
    let source = format!(
        r#"(element, requested) => {{
  {install};
  const __pw = globalThis[Symbol.for({runtime_symbol:?})];
  if (!__pw || __pw.version !== {runtime_version})
    throw new Error('Pwright selector runtime is unavailable');
  // `element` is the original native Handle argument, never a selector result.
  const target = __pw.retarget(element);
  if (!element.isConnected || !target || !target.isConnected)
    return {{ status: 'detached' }};
  if (target.tagName !== 'SELECT')
    return {{ status: 'invalid-target' }};
  if (!__pw.visible(target)) return {{ status: 'retry' }};
  const state = __pw.elementState(target, 'enabled');
  if (state.status !== 'done' || !state.value) return {{ status: 'retry' }};
  if (target.options.length > {max_dom_options})
    return {{ status: 'too-many-options' }};
  const normalize = value => String(value ?? '').replace(/\s+/g, ' ').trim();
  const normalizedRequested = normalize(requested);
  let selected = null;
  for (const option of target.options) {{
    const label = option.label;
    if (option.value !== requested && label !== requested &&
        (label.length > {max_value_bytes} || normalize(label) !== normalizedRequested))
      continue;
    const optionState = __pw.elementState(option, 'enabled');
    if (optionState.status !== 'done' || !optionState.value)
      return {{ status: 'retry' }};
    selected = option;
    break;
  }}
  if (!selected) return {{ status: 'retry' }};
  const selectedValue = selected.value;
  if (selectedValue.length > {max_value_bytes} ||
      new TextEncoder().encode(selectedValue).length > {max_value_bytes})
    return {{ status: 'result-too-large' }};
  target.value = undefined;
  selected.selected = true;
  target.dispatchEvent(new Event('input', {{ bubbles: true, composed: true }}));
  target.dispatchEvent(new Event('change', {{ bubbles: true }}));
  return {{ status: 'done', value: selectedValue }};
}}"#,
        runtime_symbol = SELECTOR_RUNTIME_SYMBOL,
        runtime_version = SELECTOR_RUNTIME_VERSION,
        max_value_bytes = MAX_VALUE_BYTES,
        max_dom_options = MAX_DOM_OPTIONS,
    );
    if source.len() > evaluation::SOURCE_BYTES {
        return Err(HostError::ResourceExhausted(format!(
            "retained selectOption source exceeds {} UTF-8 bytes",
            evaluation::SOURCE_BYTES
        )));
    }
    Ok(source)
}

fn parse_attempt(value: EvaluationValue) -> Result<SelectAttempt, HostError> {
    let EvaluationValue::Object { properties, .. } = value else {
        return Err(HostError::backend(
            "ElementHandle.selectOption returned a non-object status",
        ));
    };
    let field = |name: &str| {
        properties
            .iter()
            .find(|property| property.key == name)
            .map(|property| &property.value)
    };
    match field("status") {
        Some(EvaluationValue::String(status)) if status == "done" => {
            let Some(EvaluationValue::String(value)) = field("value") else {
                return Err(HostError::backend(
                    "ElementHandle.selectOption omitted the selected value",
                ));
            };
            validate_value(value, value.capacity())?;
            Ok(SelectAttempt::Done(value.clone()))
        }
        Some(EvaluationValue::String(status)) if status == "retry" => Ok(SelectAttempt::Retry),
        Some(EvaluationValue::String(status)) if status == "detached" => Err(HostError::Backend(
            "Element is not attached to the DOM".into(),
        )),
        Some(EvaluationValue::String(status)) if status == "invalid-target" => Err(
            HostError::InvalidArgument("Element is not a <select> element".into()),
        ),
        Some(EvaluationValue::String(status)) if status == "too-many-options" => {
            Err(HostError::ResourceExhausted(format!(
                "selectOption target exceeds {MAX_DOM_OPTIONS} DOM options"
            )))
        }
        Some(EvaluationValue::String(status)) if status == "result-too-large" => {
            Err(HostError::ResourceExhausted(format!(
                "selectOption selected value exceeds {MAX_VALUE_BYTES} UTF-8 bytes"
            )))
        }
        _ => Err(HostError::backend(
            "ElementHandle.selectOption returned an unknown status",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_cap_evaluation::EvaluationProperty;

    fn status(status: &str, value: Option<&str>) -> EvaluationValue {
        let mut properties = vec![EvaluationProperty::new(
            "status",
            EvaluationValue::String(status.into()),
        )];
        if let Some(value) = value {
            properties.push(EvaluationProperty::new(
                "value",
                EvaluationValue::String(value.into()),
            ));
        }
        EvaluationValue::Object {
            id: None,
            properties,
        }
    }

    #[test]
    fn selected_value_is_decoded_without_fabricating_the_requested_value() {
        // Arrange
        let response = status("done", Some("canonical-blue"));

        // Act
        let selected = parse_attempt(response).unwrap();

        // Assert
        assert!(matches!(selected, SelectAttempt::Done(value) if value == "canonical-blue"));
    }

    #[test]
    fn malformed_or_oversized_result_cannot_claim_success() {
        // Arrange
        let missing = status("done", None);
        let oversized = status("done", Some(&"x".repeat(MAX_VALUE_BYTES + 1)));

        // Act
        let missing_result = parse_attempt(missing);
        let oversized_result = parse_attempt(oversized);

        // Assert
        assert!(missing_result.is_err());
        assert!(oversized_result.is_err());
    }

    #[test]
    fn retained_input_capacity_is_rejected_even_when_the_value_is_short() {
        // Arrange
        let mut value = String::with_capacity(MAX_VALUE_BYTES + 1);
        value.push_str("blue");

        // Act
        let result = validate_value(&value, value.capacity());

        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    }

    #[test]
    fn retry_and_detachment_are_distinct_outcomes() {
        // Arrange
        let retry = status("retry", None);
        let detached = status("detached", None);

        // Act
        let retry_result = parse_attempt(retry);
        let detached_result = parse_attempt(detached);

        // Assert
        assert!(matches!(retry_result, Ok(SelectAttempt::Retry)));
        assert!(matches!(detached_result, Err(HostError::Backend(_))));
    }

    #[test]
    fn source_selects_the_retained_owner_and_dispatches_events_in_order() {
        // Arrange
        let source = selection_source().unwrap();

        // Act
        let input = source.find("new Event('input'").unwrap();
        let change = source.find("new Event('change'").unwrap();

        // Assert
        assert!(source.contains("const target = __pw.retarget(element)"));
        assert!(!source.contains("document.querySelector"));
        assert!(source.contains("target.options.length > 4096"));
        assert!(source.contains("new TextEncoder().encode(selectedValue)"));
        assert!(input < change);
    }
}
