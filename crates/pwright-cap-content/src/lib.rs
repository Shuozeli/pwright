//! Portable, opt-in Page content capture contracts.
//!
//! The server returns bytes. Filesystem paths remain a client-local concern.

#[doc(hidden)]
#[rustfmt::skip]
mod generated_capability;

pub use generated_capability::*;
pub use pwright_screenshot_model::{
    ScreenshotClip, ScreenshotData, ScreenshotFormat, ScreenshotOptions, ScreenshotScale,
};

#[cfg(test)]
mod tests {
    use pwright_core::{CommandSpec, RegistryBuilder};

    use super::{
        CONTENT_CAPABILITY_ID, CONTENT_COMMAND_DESCRIPTORS, ContentCapability,
        PAGE_SCREENSHOT_COMMAND_ID, PAGE_SCREENSHOT_METHOD, PAGE_SCREENSHOT_TARGET_OBJECT_KINDS,
        PageScreenshot, ScreenshotOptions,
    };

    #[test]
    fn content_registers_one_page_only_screenshot_command() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<ContentCapability>()
            .expect("content capability should register");

        // Act
        let registry = builder.build().expect("registry should validate");

        // Assert
        assert!(registry.contains(CONTENT_CAPABILITY_ID));
        assert_eq!(
            registry.owner_of(PAGE_SCREENSHOT_COMMAND_ID),
            Some(CONTENT_CAPABILITY_ID)
        );
        assert_eq!(PageScreenshot::METHOD, PAGE_SCREENSHOT_METHOD);
        assert_eq!(CONTENT_COMMAND_DESCRIPTORS.len(), 1);
        assert_eq!(PAGE_SCREENSHOT_TARGET_OBJECT_KINDS.len(), 1);
        assert_eq!(
            PAGE_SCREENSHOT_TARGET_OBJECT_KINDS[0].as_str(),
            "pwright.browser.page"
        );
    }

    #[test]
    fn content_reexports_the_shared_screenshot_model_without_wrapping_it() {
        // Arrange
        let shared = pwright_screenshot_model::ScreenshotOptions::default();

        // Act
        let public: ScreenshotOptions = shared;

        // Assert
        assert_eq!(public, ScreenshotOptions::default());
    }
}
