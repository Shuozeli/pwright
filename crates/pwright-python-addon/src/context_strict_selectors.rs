//! Bounded, Context-owned strict-selector defaults for the Python FFI.
//!
//! An explicit selector-call option always wins. The stored default applies
//! only when the call omits `strict`, matching Playwright's FrameSelectors.

use std::collections::BTreeSet;
use std::sync::Mutex;

use pwright_node_host::HostError;

const MAX_STRICT_CONTEXTS: usize = 64;

#[derive(Default)]
pub(crate) struct ContextStrictSelectors {
    strict_context_ids: Mutex<BTreeSet<String>>,
}

impl ContextStrictSelectors {
    pub(crate) fn install(
        &self,
        context_id: &str,
        strict_selectors: bool,
    ) -> Result<(), HostError> {
        if !strict_selectors {
            return Ok(());
        }
        let mut ids = self.strict_context_ids.lock().map_err(|_| {
            HostError::Backend("Context strict-selector policy lock poisoned".into())
        })?;
        if !ids.contains(context_id) && ids.len() >= MAX_STRICT_CONTEXTS {
            return Err(HostError::ResourceExhausted(
                "Context strict-selector policy capacity exceeded".into(),
            ));
        }
        ids.insert(context_id.to_owned());
        Ok(())
    }

    pub(crate) fn resolve(
        &self,
        context_id: &str,
        explicit_strict: Option<bool>,
    ) -> Result<bool, HostError> {
        if let Some(strict) = explicit_strict {
            return Ok(strict);
        }
        let ids = self.strict_context_ids.lock().map_err(|_| {
            HostError::Backend("Context strict-selector policy lock poisoned".into())
        })?;
        Ok(ids.contains(context_id))
    }

    pub(crate) fn retire(&self, context_id: &str) -> Result<(), HostError> {
        self.strict_context_ids
            .lock()
            .map_err(|_| HostError::Backend("Context strict-selector policy lock poisoned".into()))?
            .remove(context_id);
        Ok(())
    }

    pub(crate) fn clear(&self) -> Result<(), HostError> {
        self.strict_context_ids
            .lock()
            .map_err(|_| HostError::Backend("Context strict-selector policy lock poisoned".into()))?
            .clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omitted_strict_uses_only_its_own_context_default() {
        // Arrange
        let policies = ContextStrictSelectors::default();
        policies.install("strict", true).unwrap();
        policies.install("ordinary", false).unwrap();

        // Act
        let strict = policies.resolve("strict", None).unwrap();
        let ordinary = policies.resolve("ordinary", None).unwrap();

        // Assert
        assert!(strict);
        assert!(!ordinary);
    }

    #[test]
    fn explicit_false_overrides_strict_context_and_explicit_true_overrides_ordinary() {
        // Arrange
        let policies = ContextStrictSelectors::default();
        policies.install("strict", true).unwrap();

        // Act
        let false_override = policies.resolve("strict", Some(false)).unwrap();
        let true_override = policies.resolve("ordinary", Some(true)).unwrap();

        // Assert
        assert!(!false_override);
        assert!(true_override);
    }

    #[test]
    fn retired_context_does_not_leak_default_to_reused_id() {
        // Arrange
        let policies = ContextStrictSelectors::default();
        policies.install("id", true).unwrap();

        // Act
        policies.retire("id").unwrap();

        // Assert
        assert!(!policies.resolve("id", None).unwrap());
    }

    #[test]
    fn policy_is_bounded_and_release_restores_capacity() {
        // Arrange
        let policies = ContextStrictSelectors::default();
        for index in 0..MAX_STRICT_CONTEXTS {
            policies.install(&index.to_string(), true).unwrap();
        }

        // Act
        let full = policies.install("overflow", true);
        policies.retire("0").unwrap();
        let admitted = policies.install("overflow", true);

        // Assert
        assert!(matches!(full, Err(HostError::ResourceExhausted(_))));
        assert!(admitted.is_ok());
    }
}
