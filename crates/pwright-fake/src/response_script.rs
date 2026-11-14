//! Deterministic response/error precedence for configurable CDP fakes.

use std::collections::VecDeque;
use std::sync::Mutex;

/// One independently configured method family.
///
/// Errors and one-shot responses are consumed in FIFO order. A cloned fallback
/// remains available after the queues drain. This type intentionally owns no
/// method defaults, strict-mode policy, async gate, or CDP error conversion.
pub struct CdpResponseScript<T> {
    state: Mutex<CdpResponseScriptState<T>>,
}

struct CdpResponseScriptState<T> {
    fallback: Option<T>,
    responses: VecDeque<T>,
    errors: VecDeque<String>,
}

impl<T> CdpResponseScript<T> {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(CdpResponseScriptState {
                fallback: None,
                responses: VecDeque::new(),
                errors: VecDeque::new(),
            }),
        }
    }

    /// Replace the persistent fallback and discard queued one-shot responses.
    pub fn set_fallback(&self, response: T) {
        let mut state = self.state.lock().unwrap();
        state.responses.clear();
        state.fallback = Some(response);
    }

    /// Replace one-shot responses and clear the persistent fallback.
    pub fn set_responses(&self, responses: impl IntoIterator<Item = T>) {
        let mut state = self.state.lock().unwrap();
        state.fallback = None;
        state.responses = responses.into_iter().collect();
    }

    /// Replace the ordered protocol-error script without changing responses.
    pub fn set_errors(&self, errors: impl IntoIterator<Item = String>) {
        self.state.lock().unwrap().errors = errors.into_iter().collect();
    }
}

impl<T: Clone> CdpResponseScript<T> {
    /// Consume one configured result. Errors take precedence without consuming
    /// a queued response; one-shot responses precede the persistent fallback.
    pub fn resolve(&self) -> Result<Option<T>, String> {
        let mut state = self.state.lock().unwrap();
        if let Some(error) = state.errors.pop_front() {
            return Err(error);
        }
        if let Some(response) = state.responses.pop_front() {
            return Ok(Some(response));
        }
        Ok(state.fallback.clone())
    }
}

impl<T> Default for CdpResponseScript<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_precede_without_discarding_the_next_response() {
        // Arrange
        let script = CdpResponseScript::new();
        script.set_responses([7]);
        script.set_errors(["first failure".to_owned()]);

        // Act
        let resolved = [script.resolve(), script.resolve()];

        // Assert
        assert_eq!(resolved, [Err("first failure".to_owned()), Ok(Some(7))]);
    }

    #[test]
    fn one_shot_responses_are_fifo_and_exhaustible() {
        // Arrange
        let script = CdpResponseScript::new();
        script.set_responses([1, 2]);

        // Act
        let resolved = [script.resolve(), script.resolve(), script.resolve()];

        // Assert
        assert_eq!(resolved, [Ok(Some(1)), Ok(Some(2)), Ok(None)]);
    }

    #[test]
    fn fallback_response_remains_available() {
        // Arrange
        let script = CdpResponseScript::new();
        script.set_fallback(9);

        // Act
        let resolved = [script.resolve(), script.resolve()];

        // Assert
        assert_eq!(resolved, [Ok(Some(9)), Ok(Some(9))]);
    }

    #[test]
    fn response_reconfiguration_preserves_the_pending_error_script() {
        // Arrange
        let script = CdpResponseScript::new();
        script.set_errors(["pending failure".to_owned()]);

        // Act
        script.set_fallback(11);
        let resolved = [script.resolve(), script.resolve()];

        // Assert
        assert_eq!(resolved, [Err("pending failure".to_owned()), Ok(Some(11))]);
    }
}
