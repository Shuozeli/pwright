//! Portable navigation capability contracts.
//!
//! `BeginNavigation` remains a narrow backend primitive. `Goto` is the distinct
//! Playwright-shaped operation with lifecycle waiting and an optional response.
//! Neither contract chooses a browser protocol, executor, transport, codec, or
//! browser-acquisition strategy.

#[doc(hidden)]
#[rustfmt::skip]
mod generated_capability;

pub use generated_capability::*;

impl BeginNavigationParams {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            referrer: None,
        }
    }

    pub fn with_referrer(mut self, referrer: impl Into<String>) -> Self {
        self.referrer = Some(referrer.into());
        self
    }
}

impl GotoParams {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            referrer: None,
            wait_until: WaitUntil::Load,
        }
    }

    pub fn with_referrer(mut self, referrer: impl Into<String>) -> Self {
        self.referrer = Some(referrer.into());
        self
    }

    pub const fn with_wait_until(mut self, wait_until: WaitUntil) -> Self {
        self.wait_until = wait_until;
        self
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::{Capability, CommandSpec, RegistryBuilder};

    use super::{
        BEGIN_NAVIGATION_COMMAND_ID, BEGIN_NAVIGATION_METHOD, BeginNavigation, GOTO_COMMAND_ID,
        GOTO_METHOD, Goto, GotoParams, NAVIGATION_CAPABILITY_ID, NAVIGATION_COMMAND_DESCRIPTORS,
        NavigationCapability, WaitUntil,
    };

    #[test]
    fn navigation_capability_registers_its_command_owner() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<NavigationCapability>()
            .expect("navigation capability should register");

        // Act
        let registry = builder.build().expect("registry should validate");

        // Assert
        assert!(registry.contains(NAVIGATION_CAPABILITY_ID));
        assert_eq!(
            registry.owner_of(BEGIN_NAVIGATION_COMMAND_ID),
            Some(NAVIGATION_CAPABILITY_ID)
        );
        assert_eq!(BeginNavigation::METHOD, BEGIN_NAVIGATION_METHOD);
        assert_eq!(
            registry.owner_of(GOTO_COMMAND_ID),
            Some(NAVIGATION_CAPABILITY_ID)
        );
        assert_eq!(Goto::METHOD, GOTO_METHOD);
        assert_eq!(BeginNavigation::TARGET_OBJECT_KINDS.len(), 1);
        assert_eq!(Goto::TARGET_OBJECT_KINDS.len(), 1);
        assert_eq!(
            Goto::TARGET_OBJECT_KINDS[0].as_str(),
            "pwright.browser.page"
        );
        assert_eq!(NAVIGATION_COMMAND_DESCRIPTORS.len(), 2);
        assert_eq!(
            NAVIGATION_COMMAND_DESCRIPTORS[0].command(),
            BEGIN_NAVIGATION_COMMAND_ID
        );
        assert_eq!(
            NAVIGATION_COMMAND_DESCRIPTORS[0].method(),
            BeginNavigation::METHOD
        );
        assert_eq!(NAVIGATION_COMMAND_DESCRIPTORS[1].command(), GOTO_COMMAND_ID);
        assert_eq!(NAVIGATION_COMMAND_DESCRIPTORS[1].method(), Goto::METHOD);
        assert_eq!(
            NavigationCapability::DESCRIPTOR.plugin.id,
            NAVIGATION_CAPABILITY_ID
        );
    }

    #[test]
    fn goto_defaults_to_playwright_load_wait_and_composes_options() {
        // Arrange
        let params = GotoParams::new("https://example.com")
            .with_referrer("https://referrer.example")
            .with_wait_until(WaitUntil::NetworkIdle);

        // Act
        let default = GotoParams::new("https://example.com");

        // Assert
        assert_eq!(default.wait_until, WaitUntil::Load);
        assert_eq!(params.referrer.as_deref(), Some("https://referrer.example"));
        assert_eq!(params.wait_until, WaitUntil::NetworkIdle);
    }
}
