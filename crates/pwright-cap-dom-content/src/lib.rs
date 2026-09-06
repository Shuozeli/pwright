//! Portable, read-only Frame DOM serialization contracts.
//!
//! This capability exposes no arbitrary JavaScript source and no DOM mutation.

use pwright_core::BudgetedUtf8;

#[doc(hidden)]
#[rustfmt::skip]
mod generated_capability;

pub use generated_capability::*;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DomContentParams;

#[derive(Debug)]
pub struct DomContentData {
    text: BudgetedUtf8,
}

impl DomContentData {
    pub const fn new(text: BudgetedUtf8) -> Self {
        Self { text }
    }

    pub fn as_str(&self) -> &str {
        self.text.as_str()
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub const fn budgeted_utf8(&self) -> &BudgetedUtf8 {
        &self.text
    }
}

impl AsRef<str> for DomContentData {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for DomContentData {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::{CommandSpec, RegistryBuilder};

    use super::{
        DOM_CONTENT_CAPABILITY_ID, DOM_CONTENT_COMMAND_DESCRIPTORS, DomContentCapability,
        FRAME_CONTENT_COMMAND_ID, FRAME_CONTENT_METHOD, FRAME_CONTENT_TARGET_OBJECT_KINDS,
        FrameContent,
    };

    #[test]
    fn dom_content_registers_one_frame_only_read_command() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<DomContentCapability>()
            .expect("DOM-content capability should register");

        // Act
        let registry = builder.build().expect("registry should validate");

        // Assert
        assert!(registry.contains(DOM_CONTENT_CAPABILITY_ID));
        assert_eq!(
            registry.owner_of(FRAME_CONTENT_COMMAND_ID),
            Some(DOM_CONTENT_CAPABILITY_ID)
        );
        assert_eq!(FrameContent::METHOD, FRAME_CONTENT_METHOD);
        assert_eq!(DOM_CONTENT_COMMAND_DESCRIPTORS.len(), 1);
        assert_eq!(FRAME_CONTENT_TARGET_OBJECT_KINDS.len(), 1);
        assert_eq!(
            FRAME_CONTENT_TARGET_OBJECT_KINDS[0].as_str(),
            "pwright.browser.frame"
        );
    }
}
