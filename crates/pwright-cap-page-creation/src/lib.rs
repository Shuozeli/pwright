//! Portable opt-in contract for creating an initialized Page and main Frame.
//!
//! This first slice is the explicit no-options `BrowserContext.newPage`
//! operation. Popup discovery remains a separate event capability.

use pwright_core::{ObjectId, ObjectKindTag};

#[doc(hidden)]
#[rustfmt::skip]
mod generated_capability;

pub use generated_capability::*;

pub type BrowserContextNewPageOptions = ();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageInitializer {
    pub object_id: ObjectId,
    pub parent_id: ObjectId,
    pub object_kind: ObjectKindTag,
    pub main_frame_id: ObjectId,
    pub main_frame_parent_id: ObjectId,
    pub main_frame_kind: ObjectKindTag,
}

impl PageInitializer {
    pub const fn new(
        object_id: ObjectId,
        parent_id: ObjectId,
        object_kind: ObjectKindTag,
        main_frame_id: ObjectId,
        main_frame_parent_id: ObjectId,
        main_frame_kind: ObjectKindTag,
    ) -> Self {
        Self {
            object_id,
            parent_id,
            object_kind,
            main_frame_id,
            main_frame_parent_id,
            main_frame_kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::{Capability, CommandSpec, ProtocolVersion, RegistryBuilder};

    use super::{
        BROWSER_CONTEXT_NEW_PAGE_COMMAND_ID, BROWSER_CONTEXT_NEW_PAGE_METHOD,
        BROWSER_CONTEXT_NEW_PAGE_TARGET_OBJECT_KINDS, BrowserContextNewPage,
        PAGE_CREATION_CAPABILITY_ID, PAGE_CREATION_COMMAND_DESCRIPTORS, PageCreationCapability,
    };

    #[test]
    fn page_creation_declares_close_dependency_and_context_target() {
        // Arrange
        let descriptor = PageCreationCapability::DESCRIPTOR;

        // Act
        let dependency = descriptor.dependencies.first().copied();

        // Assert
        assert_eq!(
            PAGE_CREATION_CAPABILITY_ID.as_str(),
            "pwright.page_creation"
        );
        assert_eq!(
            BrowserContextNewPage::METHOD,
            BROWSER_CONTEXT_NEW_PAGE_METHOD
        );
        assert_eq!(PAGE_CREATION_COMMAND_DESCRIPTORS.len(), 1);
        assert_eq!(
            BROWSER_CONTEXT_NEW_PAGE_COMMAND_ID.as_str(),
            "browser-context.new_page"
        );
        assert_eq!(BROWSER_CONTEXT_NEW_PAGE_TARGET_OBJECT_KINDS.len(), 1);
        assert_eq!(
            BROWSER_CONTEXT_NEW_PAGE_TARGET_OBJECT_KINDS[0].as_str(),
            "pwright.browser.browser_context"
        );
        assert_eq!(
            dependency.map(|value| (value.id.as_str(), value.minimum_version)),
            Some(("pwright.page_lifecycle", ProtocolVersion::new(1, 0)))
        );
    }

    #[test]
    fn registry_rejects_page_creation_without_page_lifecycle() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<PageCreationCapability>()
            .expect("page creation should register");

        // Act
        let result = builder.build();

        // Assert
        assert!(result.is_err());
    }
}
