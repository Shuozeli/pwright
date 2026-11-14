//! Opt-in text selection against the original retained ElementHandle.
//!
//! The only browser operation is an exact-handle evaluation. In particular,
//! retrying visibility never resolves the selector again or retains an
//! observer in the page.

use std::time::Duration;

use crate::{
    HostError, HostSession,
    evaluation::EvaluationValue,
    handles::{OwnedHandleArgument, PendingHandle},
};

const SELECT_TEXT_SOURCE: &str = r#"(element, force) => {
  if (!element || !element.isConnected) return 'detached';
  if (element.nodeType !== 1) return 'invalid-target';
  const visible = node => {
    const style = node.ownerDocument.defaultView.getComputedStyle(node);
    if (!style) return true;
    if (style.display === 'contents') {
      for (let child = node.firstChild; child; child = child.nextSibling) {
        if (child.nodeType === 1 && visible(child)) return true;
        if (child.nodeType === 3) {
          const range = node.ownerDocument.createRange();
          range.selectNode(child);
          const rect = range.getBoundingClientRect();
          if (rect.width > 0 && rect.height > 0) return true;
        }
      }
      return false;
    }
    if (typeof node.checkVisibility === 'function' && !node.checkVisibility()) return false;
    if (style.visibility !== 'visible') return false;
    const rect = node.getBoundingClientRect();
    return rect.width > 0 && rect.height > 0;
  };
  if (!force && !visible(element)) return 'not-visible';
  let target = element;
  if (!target.matches('input, textarea, select') && !target.isContentEditable)
    target = target.closest('button, [role=button], [role=checkbox], [role=radio]') || target;
  if (!target.matches('a, input, textarea, button, select, [role=link], [role=button], [role=checkbox], [role=radio]') &&
      !target.isContentEditable) {
    const label = target.closest('label');
    if (label && label.control) target = label.control;
  }
  if (!target.isConnected) return 'detached';
  const name = target.nodeName.toLowerCase();
  if (name === 'input') {
    target.select();
    target.focus();
    return 'done';
  }
  if (name === 'textarea') {
    target.selectionStart = 0;
    target.selectionEnd = target.value.length;
    target.focus();
    return 'done';
  }
  const range = target.ownerDocument.createRange();
  range.selectNodeContents(target);
  const selection = target.ownerDocument.defaultView.getSelection();
  if (selection) {
    selection.removeAllRanges();
    selection.addRange(range);
  }
  target.focus();
  return 'done';
}"#;

const RETRY_DELAY: Duration = Duration::from_millis(50);
const NOT_VISIBLE_PROGRESS: &str = "retrying selectText action\n  element is not visible";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SelectionAttempt {
    Done,
    NotVisible,
}

fn parse_selection_attempt(value: EvaluationValue) -> Result<SelectionAttempt, HostError> {
    match value {
        EvaluationValue::String(status) => match status.as_str() {
            "done" => Ok(SelectionAttempt::Done),
            "not-visible" => Ok(SelectionAttempt::NotVisible),
            "detached" => Err(HostError::Backend(
                "Element is not attached to the DOM".into(),
            )),
            "invalid-target" => Err(HostError::InvalidArgument("Node is not an Element".into())),
            _ => Err(HostError::backend(
                "retained ElementHandle.select_text returned an unknown status",
            )),
        },
        _ => Err(HostError::backend(
            "retained ElementHandle.select_text returned a non-string status",
        )),
    }
}

fn timeout_error(target: &PendingHandle, timeout_ms: u32) -> HostError {
    HostError::ActionTimeout {
        api: "ElementHandle.selectText",
        timeout_ms,
        locator: target.initializer().preview.clone(),
        progress: NOT_VISIBLE_PROGRESS,
    }
}

fn remaining(deadline: Option<tokio::time::Instant>) -> Option<Duration> {
    deadline.map(|deadline| deadline.saturating_duration_since(tokio::time::Instant::now()))
}

impl HostSession {
    /// Select text on one retained ElementHandle, waiting only for visibility
    /// unless `force` is requested. Zero timeout disables the host deadline.
    pub async fn element_handle_select_text(
        &self,
        target: &PendingHandle,
        force: bool,
        timeout_ms: u32,
    ) -> Result<(), HostError> {
        if !target.is_element() {
            return Err(HostError::InvalidArgument(
                "select_text requires an ElementHandle".into(),
            ));
        }
        let (_, _, _operation) =
            self.admit_operation(&target.initializer().object_id.get().to_string())?;
        let handles = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .handles
                .clone()
        };
        // Four whole-action slots leave four of the eight ordinary Handle
        // calls available to these actions' nested exact-handle evaluations.
        let _action = handles
            .element_action_slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| {
                HostError::ResourceExhausted("ElementHandle select_text capacity exceeded".into())
            })?;
        let deadline = (timeout_ms != 0)
            .then(|| tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms)));
        loop {
            let budget = remaining(deadline);
            if budget.is_some_and(|remaining| remaining.is_zero()) {
                return Err(timeout_error(target, timeout_ms));
            }
            let selection = self.handle_evaluate_owned(
                target,
                SELECT_TEXT_SOURCE.into(),
                OwnedHandleArgument::from_slots(EvaluationValue::Bool(force), Vec::new())?,
            );
            let result = match budget {
                Some(budget) => tokio::time::timeout(budget, selection)
                    .await
                    .map_err(|_| timeout_error(target, timeout_ms))??,
                None => selection.await?,
            };
            match parse_selection_attempt(result)? {
                SelectionAttempt::Done => return Ok(()),
                SelectionAttempt::NotVisible => {
                    let delay = remaining(deadline)
                        .map_or(RETRY_DELAY, |remaining| RETRY_DELAY.min(remaining));
                    if delay.is_zero() {
                        return Err(timeout_error(target, timeout_ms));
                    }
                    tokio::select! {
                        biased;
                        () = self.stop.cancelled() => return Err(HostError::TargetClosed),
                        () = tokio::time::sleep(delay) => {},
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NOT_VISIBLE_PROGRESS, SELECT_TEXT_SOURCE, SelectionAttempt, parse_selection_attempt,
    };
    use crate::{HostError, evaluation::EvaluationValue};

    #[test]
    fn visible_selection_completes() {
        // Arrange
        let value = EvaluationValue::String("done".into());

        // Act
        let result = parse_selection_attempt(value);

        // Assert
        assert_eq!(result.unwrap(), SelectionAttempt::Done);
    }

    #[test]
    fn invisible_selection_retries_under_the_action_deadline() {
        // Arrange
        let value = EvaluationValue::String("not-visible".into());

        // Act
        let result = parse_selection_attempt(value);

        // Assert
        assert_eq!(result.unwrap(), SelectionAttempt::NotVisible);
        assert!(NOT_VISIBLE_PROGRESS.contains("element is not visible"));
    }

    #[test]
    fn detached_selection_is_terminal() {
        // Arrange
        let value = EvaluationValue::String("detached".into());

        // Act
        let result = parse_selection_attempt(value);

        // Assert
        assert!(
            matches!(result, Err(HostError::Backend(message)) if message == "Element is not attached to the DOM")
        );
    }

    #[test]
    fn unexpected_status_fails_closed() {
        // Arrange
        let value = EvaluationValue::String("selected-replacement".into());

        // Act
        let result = parse_selection_attempt(value);

        // Assert
        assert!(
            matches!(result, Err(HostError::Backend(message)) if message.contains("unknown status"))
        );
    }

    #[test]
    fn browser_program_uses_exact_element_and_no_selector_or_observer() {
        // Arrange
        let source = SELECT_TEXT_SOURCE;

        // Act
        let reads_exact_handle = source.starts_with("(element, force) =>")
            && source.contains("!visible(element)")
            && source.contains("target.selectionEnd = target.value.length");

        // Assert
        assert!(reads_exact_handle);
        assert!(!source.contains("querySelector"));
        assert!(!source.contains("MutationObserver"));
        assert!(!source.contains("setTimeout"));
    }
}
