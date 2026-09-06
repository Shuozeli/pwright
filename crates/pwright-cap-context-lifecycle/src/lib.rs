//! Portable explicit lifecycle contract for browser contexts.
//!
//! This opt-in capability does not make runtime lifecycle controls public. Its
//! close command is a Playwright-shaped request whose backend must finish
//! before the runtime publishes child-first disposal.

#[doc(hidden)]
#[rustfmt::skip]
mod generated_capability;

pub use generated_capability::*;

pub type BrowserContextCloseOptions = ();
pub type BrowserContextCloseResult = ();

#[cfg(test)]
mod tests {
    use pwright_core::{Capability, CommandSpec, RegistryBuilder};

    use super::{
        BROWSER_CONTEXT_CLOSE_COMMAND_ID, BROWSER_CONTEXT_CLOSE_METHOD,
        BROWSER_CONTEXT_CLOSE_TARGET_OBJECT_KINDS, BrowserContextClose,
        CONTEXT_LIFECYCLE_CAPABILITY_ID, CONTEXT_LIFECYCLE_COMMAND_DESCRIPTORS,
        ContextLifecycleCapability,
    };

    #[test]
    fn context_lifecycle_capability_registers_only_context_close() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<ContextLifecycleCapability>()
            .expect("context lifecycle capability should register");

        // Act
        let registry = builder.build().expect("registry should validate");

        // Assert
        assert!(registry.contains(CONTEXT_LIFECYCLE_CAPABILITY_ID));
        assert_eq!(
            registry.owner_of(BROWSER_CONTEXT_CLOSE_COMMAND_ID),
            Some(CONTEXT_LIFECYCLE_CAPABILITY_ID)
        );
        assert_eq!(BrowserContextClose::METHOD, BROWSER_CONTEXT_CLOSE_METHOD);
        assert_eq!(CONTEXT_LIFECYCLE_COMMAND_DESCRIPTORS.len(), 1);
        assert_eq!(BROWSER_CONTEXT_CLOSE_TARGET_OBJECT_KINDS.len(), 1);
        assert_eq!(
            BROWSER_CONTEXT_CLOSE_TARGET_OBJECT_KINDS[0].as_str(),
            "pwright.browser.browser_context"
        );
        assert_eq!(
            ContextLifecycleCapability::DESCRIPTOR.plugin.id,
            CONTEXT_LIFECYCLE_CAPABILITY_ID
        );
    }
}
