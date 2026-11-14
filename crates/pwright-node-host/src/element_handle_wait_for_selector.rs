//! Optional scoped selector wait against one original, retained ElementHandle.
//!
//! An attached/visible sample returns the matching DOM node itself. There is
//! no second selector query between the state decision and Handle creation.

use std::{str::FromStr, time::Duration};

use pwright_backend_cdp_selector_runtime::{
    SELECTOR_RUNTIME_SYMBOL, SELECTOR_RUNTIME_VERSION, compile_selector,
    selector_runtime_install_script,
};

use crate::{
    HostError, HostSession,
    evaluation::{self, EvaluationValue},
    handles::{OwnedHandleArgument, PendingHandle},
    locator_query::{ElementHandleQuery, ElementHandleQueryPlan},
};

const RETRY_DELAY: Duration = Duration::from_millis(50);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElementHandleSelectorWaitState {
    Attached,
    Detached,
    Visible,
    Hidden,
}

impl FromStr for ElementHandleSelectorWaitState {
    type Err = HostError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "attached" => Ok(Self::Attached),
            "detached" => Ok(Self::Detached),
            "visible" => Ok(Self::Visible),
            "hidden" => Ok(Self::Hidden),
            _ => Err(HostError::InvalidArgument(
                "state: expected one of (attached|detached|visible|hidden)".into(),
            )),
        }
    }
}

impl ElementHandleSelectorWaitState {
    fn returns_element(self) -> bool {
        matches!(self, Self::Attached | Self::Visible)
    }

    fn return_source(self) -> &'static str {
        match self {
            Self::Attached => "return __pwMatches[0] || null;",
            Self::Visible => {
                "const match = __pwMatches[0]; return match && __pw.visible(match) ? match : null;"
            }
            Self::Detached => "return __pwMatches.length === 0;",
            Self::Hidden => "const match = __pwMatches[0]; return !match || !__pw.visible(match);",
        }
    }
}

impl HostSession {
    /// Keep the original Handle and its creation world throughout this wait.
    /// A destroyed execution context is an error, not permission to re-query
    /// the same selector in a replacement document or Frame.
    pub async fn element_handle_wait_for_selector(
        &self,
        target: &PendingHandle,
        query: ElementHandleQuery,
        state: ElementHandleSelectorWaitState,
        strict: bool,
        timeout_ms: Option<u32>,
    ) -> Result<Option<PendingHandle>, HostError> {
        if !target.is_element() {
            return Err(HostError::InvalidArgument(
                "wait_for_selector requires an ElementHandle".into(),
            ));
        }
        let timeout_ms = match timeout_ms {
            Some(timeout_ms) => timeout_ms,
            None => {
                let creation_world = target.source_world().ok_or_else(|| {
                    HostError::Backend("Handle creation-world identity is missing".into())
                })?;
                self.page_default_timeout(&creation_world.page_id().get().to_string(), false)?
            }
        };
        let (plan, _query_lease) = query.command()?;
        let source = scoped_wait_source(&plan, state, strict)?;
        let deadline = (timeout_ms != 0)
            .then(|| tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms)));

        loop {
            let argument = OwnedHandleArgument::from_slots(EvaluationValue::Undefined, Vec::new())?;
            if state.returns_element() {
                let sample = self.handle_prepare_owned(target, source.clone(), true, argument);
                let sampled = match deadline {
                    Some(deadline) => tokio::time::timeout_at(deadline, sample)
                        .await
                        .map_err(|_| HostError::Timeout(timeout_ms))??,
                    None => sample.await?,
                };
                if sampled.is_element() {
                    return Ok(Some(sampled));
                }
                // A nonmatch is a temporary primitive Handle. Release it
                // before the next poll; drop remains the cancellation fallback.
                let cleanup = sampled.dispose();
                match deadline {
                    Some(deadline) => tokio::time::timeout_at(deadline, cleanup)
                        .await
                        .map_err(|_| HostError::Timeout(timeout_ms))??,
                    None => cleanup.await?,
                }
            } else {
                let sample = self.handle_evaluate_owned(target, source.clone(), argument);
                let sampled = match deadline {
                    Some(deadline) => tokio::time::timeout_at(deadline, sample)
                        .await
                        .map_err(|_| HostError::Timeout(timeout_ms))??,
                    None => sample.await?,
                };
                match sampled {
                    EvaluationValue::Bool(true) => return Ok(None),
                    EvaluationValue::Bool(false) => {}
                    _ => {
                        return Err(HostError::Backend(
                            "ElementHandle scoped wait returned a non-boolean value".into(),
                        ));
                    }
                }
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

fn scoped_wait_source(
    plan: &ElementHandleQueryPlan,
    state: ElementHandleSelectorWaitState,
    strict: bool,
) -> Result<String, HostError> {
    let matches = match plan {
        ElementHandleQueryPlan::Selector(selector) => {
            compile_selector(selector, "element").map_err(HostError::backend)?
        }
        ElementHandleQueryPlan::XPath(expression) => {
            let expression = serde_json::to_string(expression)
                .map_err(|error| HostError::backend(format!("could not encode XPath: {error}")))?;
            format!("__pw.xpath(element, {expression})")
        }
    };
    let install = selector_runtime_install_script();
    let strict_check = if strict {
        "if (__pwMatches.length > 1) throw new Error('strict mode violation');"
    } else {
        ""
    };
    let result = state.return_source();
    let source = format!(
        r#"(element) => {{
  if (!element.isConnected) throw new Error('Element is not attached to the DOM');
  {install};
  const __pw = globalThis[Symbol.for({runtime_symbol:?})];
  if (!__pw || __pw.version !== {runtime_version})
    throw new Error('Pwright selector runtime is unavailable');
  const __pwMatches = {matches};
  {strict_check}
  {result}
}}"#,
        runtime_symbol = SELECTOR_RUNTIME_SYMBOL,
        runtime_version = SELECTOR_RUNTIME_VERSION,
    );
    if source.len() > evaluation::SOURCE_BYTES {
        return Err(HostError::ResourceExhausted(format!(
            "ElementHandle scoped wait source exceeds {} UTF-8 bytes",
            evaluation::SOURCE_BYTES
        )));
    }
    Ok(source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locator_query::{QueryBudget, QueryLimits};

    fn plan(source: &str) -> ElementHandleQueryPlan {
        QueryBudget::new(QueryLimits {
            recipes: 1,
            bytes: 128 * 1024,
        })
        .unwrap()
        .prepare_leaf(source.len())
        .unwrap()
        .element_handle(source.to_owned())
        .unwrap()
        .command()
        .unwrap()
        .0
    }

    #[test]
    fn scoped_wait_keeps_root_and_state_to_handle_decision_in_one_program() {
        // Arrange
        let selector = plan("span");

        // Act
        let source =
            scoped_wait_source(&selector, ElementHandleSelectorWaitState::Visible, true).unwrap();

        // Assert
        assert!(source.contains("if (!element.isConnected)"));
        assert!(source.contains("strict mode violation"));
        assert!(source.contains("__pw.visible(match) ? match : null"));
        assert!(!source.contains("document.querySelector"));
    }

    #[test]
    fn hidden_and_detached_return_only_completion_status() {
        // Arrange
        let selector = plan("span");

        // Act
        let hidden =
            scoped_wait_source(&selector, ElementHandleSelectorWaitState::Hidden, false).unwrap();
        let detached =
            scoped_wait_source(&selector, ElementHandleSelectorWaitState::Detached, false).unwrap();

        // Assert
        assert!(hidden.contains("return !match || !__pw.visible(match)"));
        assert!(detached.contains("return __pwMatches.length === 0"));
    }

    #[test]
    fn xpath_wait_remains_rooted_in_the_original_element() {
        // Arrange
        let selector = plan("xpath=.//span");

        // Act
        let source =
            scoped_wait_source(&selector, ElementHandleSelectorWaitState::Attached, false).unwrap();

        // Assert
        assert!(source.contains("__pw.xpath(element, \".//span\")"));
        assert!(source.contains("return __pwMatches[0] || null"));
    }
}
