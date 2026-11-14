//! Optional portable browser-history navigation contracts.
//!
//! This capability is separate from URL navigation so consumers can install
//! history traversal without widening the base `goto` / `reload` surface.

#[doc(hidden)]
#[rustfmt::skip]
mod generated_capability;

pub use generated_capability::*;

impl Default for GoBackParams {
    fn default() -> Self {
        Self {
            wait_until: WaitUntil::Load,
        }
    }
}

impl Default for GoForwardParams {
    fn default() -> Self {
        Self {
            wait_until: WaitUntil::Load,
        }
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::{CommandSpec, RegistryBuilder};

    use super::{
        GO_BACK_COMMAND_ID, GO_BACK_METHOD, GO_FORWARD_COMMAND_ID, GO_FORWARD_METHOD, GoBack,
        GoForward, HISTORY_NAVIGATION_CAPABILITY_ID, HistoryNavigationCapability,
    };

    #[test]
    fn history_navigation_registers_backward_and_forward_traversal() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<HistoryNavigationCapability>()
            .expect("history navigation capability should register");

        // Act
        let registry = builder.build().expect("registry should validate");

        // Assert
        assert_eq!(
            registry.owner_of(GO_BACK_COMMAND_ID),
            Some(HISTORY_NAVIGATION_CAPABILITY_ID)
        );
        assert_eq!(GoBack::METHOD, GO_BACK_METHOD);
        assert_eq!(GoBack::TARGET_OBJECT_KINDS.len(), 1);
        assert_eq!(
            GoBack::TARGET_OBJECT_KINDS[0].as_str(),
            "pwright.browser.page"
        );
        assert_eq!(
            registry.owner_of(GO_FORWARD_COMMAND_ID),
            Some(HISTORY_NAVIGATION_CAPABILITY_ID)
        );
        assert_eq!(GoForward::METHOD, GO_FORWARD_METHOD);
        assert_eq!(GoForward::TARGET_OBJECT_KINDS.len(), 1);
        assert_eq!(
            GoForward::TARGET_OBJECT_KINDS[0].as_str(),
            "pwright.browser.page"
        );
    }
}
