//! Portable lifecycle contract for an owned Browser.
//!
//! The capability describes the Playwright-shaped public operation. Merely
//! depending on this crate does not authorize process termination: a server
//! must separately bind the command to a launch-owned session grant.

#[doc(hidden)]
#[rustfmt::skip]
mod generated_capability;

pub use generated_capability::*;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BrowserCloseOptions {
    reason: Option<String>,
}

impl BrowserCloseOptions {
    pub fn new(reason: Option<String>) -> Self {
        Self { reason }
    }

    pub fn with_reason(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }

    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    pub fn into_reason(self) -> Option<String> {
        self.reason
    }
}

pub type BrowserCloseResult = ();

#[cfg(test)]
mod tests {
    use pwright_core::{Capability, CommandSpec, RegistryBuilder};

    use super::{
        BROWSER_CLOSE_COMMAND_ID, BROWSER_CLOSE_METHOD, BROWSER_CLOSE_TARGET_OBJECT_KINDS,
        BROWSER_LIFECYCLE_CAPABILITY_ID, BROWSER_LIFECYCLE_COMMAND_DESCRIPTORS, BrowserClose,
        BrowserCloseResult, BrowserLifecycleCapability,
    };

    #[test]
    fn browser_lifecycle_registers_only_owned_browser_close() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<BrowserLifecycleCapability>()
            .expect("browser lifecycle capability should register");

        // Act
        let registry = builder.build().expect("registry should validate");

        // Assert
        assert!(registry.contains(BROWSER_LIFECYCLE_CAPABILITY_ID));
        assert_eq!(
            registry.owner_of(BROWSER_CLOSE_COMMAND_ID),
            Some(BROWSER_LIFECYCLE_CAPABILITY_ID)
        );
        assert_eq!(BrowserClose::METHOD, BROWSER_CLOSE_METHOD);
        assert_eq!(BROWSER_LIFECYCLE_COMMAND_DESCRIPTORS.len(), 1);
        assert_eq!(BROWSER_CLOSE_TARGET_OBJECT_KINDS.len(), 1);
        assert_eq!(
            BROWSER_CLOSE_TARGET_OBJECT_KINDS[0].as_str(),
            "pwright.browser.browser"
        );
        assert_eq!(
            BrowserLifecycleCapability::DESCRIPTOR.plugin.id,
            BROWSER_LIFECYCLE_CAPABILITY_ID
        );
        fn assert_portable_success(_: BrowserCloseResult) {}
        assert_portable_success(());
    }
}
