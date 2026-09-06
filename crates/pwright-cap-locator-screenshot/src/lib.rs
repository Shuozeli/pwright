//! Portable, opt-in Locator screenshot contracts.
//!
//! The one-command boundary keeps strict selector resolution, scrolling,
//! capture, and temporary-state cleanup on the server. Filesystem paths remain
//! a client-local concern.

#[doc(hidden)]
#[rustfmt::skip]
mod generated_capability;

pub use generated_capability::*;
pub use pwright_screenshot_model::{
    ScreenshotClip, ScreenshotData, ScreenshotFormat, ScreenshotOptions, ScreenshotScale,
};
pub use pwright_selector_model::Selector;

#[derive(Clone, Debug, PartialEq)]
pub struct LocatorScreenshotParams {
    pub selector: Selector,
    pub options: ScreenshotOptions,
}

impl LocatorScreenshotParams {
    pub const fn new(selector: Selector, options: ScreenshotOptions) -> Self {
        Self { selector, options }
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::{CommandSpec, RegistryBuilder};
    use pwright_screenshot_model::ScreenshotOptions;
    use pwright_selector_model::Selector;

    use super::{
        LOCATOR_SCREENSHOT_CAPABILITY_ID, LOCATOR_SCREENSHOT_COMMAND_DESCRIPTORS,
        LOCATOR_SCREENSHOT_COMMAND_ID, LOCATOR_SCREENSHOT_METHOD,
        LOCATOR_SCREENSHOT_TARGET_OBJECT_KINDS, LocatorScreenshot, LocatorScreenshotCapability,
        LocatorScreenshotParams,
    };

    #[test]
    fn capability_registers_one_page_or_frame_scoped_command() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<LocatorScreenshotCapability>()
            .expect("Locator screenshot capability should register");

        // Act
        let registry = builder.build().expect("registry should validate");

        // Assert
        assert!(registry.contains(LOCATOR_SCREENSHOT_CAPABILITY_ID));
        assert_eq!(
            registry.owner_of(LOCATOR_SCREENSHOT_COMMAND_ID),
            Some(LOCATOR_SCREENSHOT_CAPABILITY_ID)
        );
        assert_eq!(LocatorScreenshot::METHOD, LOCATOR_SCREENSHOT_METHOD);
        assert_eq!(LOCATOR_SCREENSHOT_COMMAND_DESCRIPTORS.len(), 1);
        assert_eq!(LOCATOR_SCREENSHOT_TARGET_OBJECT_KINDS.len(), 2);
        assert_eq!(
            LOCATOR_SCREENSHOT_TARGET_OBJECT_KINDS[0].as_str(),
            "pwright.browser.frame"
        );
        assert_eq!(
            LOCATOR_SCREENSHOT_TARGET_OBJECT_KINDS[1].as_str(),
            "pwright.browser.page"
        );
    }

    #[test]
    fn params_compose_shared_selector_and_screenshot_values_without_commands() {
        // Arrange
        let selector = Selector::css("#hero").expect("selector should validate");
        let options = ScreenshotOptions::default();

        // Act
        let params = LocatorScreenshotParams::new(selector.clone(), options);

        // Assert
        assert_eq!(params.selector, selector);
        assert_eq!(params.options, options);
    }
}
