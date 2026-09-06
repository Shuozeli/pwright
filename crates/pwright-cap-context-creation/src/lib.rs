//! Portable opt-in contract for creating a no-options BrowserContext.
//!
//! This first slice intentionally excludes the full Playwright context option
//! surface. It returns only a portable object initializer and depends on the
//! separately installable context-lifecycle capability.

use pwright_core::{ObjectId, ObjectKindTag};

#[doc(hidden)]
#[rustfmt::skip]
mod generated_capability;

pub use generated_capability::*;

pub type BrowserNewContextOptions = ();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BrowserContextInitializer {
    pub object_id: ObjectId,
    pub parent_id: ObjectId,
    pub object_kind: ObjectKindTag,
}

impl BrowserContextInitializer {
    pub const fn new(object_id: ObjectId, parent_id: ObjectId, object_kind: ObjectKindTag) -> Self {
        Self {
            object_id,
            parent_id,
            object_kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::{Capability, CommandSpec, ProtocolVersion, RegistryBuilder};

    use super::{
        BROWSER_NEW_CONTEXT_COMMAND_ID, BROWSER_NEW_CONTEXT_METHOD,
        BROWSER_NEW_CONTEXT_TARGET_OBJECT_KINDS, BrowserNewContext, CONTEXT_CREATION_CAPABILITY_ID,
        CONTEXT_CREATION_COMMAND_DESCRIPTORS, ContextCreationCapability,
    };

    #[test]
    fn context_creation_declares_its_close_dependency_and_browser_target() {
        // Arrange
        let descriptor = ContextCreationCapability::DESCRIPTOR;

        // Act
        let dependency = descriptor.dependencies.first().copied();

        // Assert
        assert_eq!(
            CONTEXT_CREATION_CAPABILITY_ID.as_str(),
            "pwright.context_creation"
        );
        assert_eq!(BrowserNewContext::METHOD, BROWSER_NEW_CONTEXT_METHOD);
        assert_eq!(CONTEXT_CREATION_COMMAND_DESCRIPTORS.len(), 1);
        assert_eq!(
            BROWSER_NEW_CONTEXT_COMMAND_ID.as_str(),
            "browser.new_context"
        );
        assert_eq!(BROWSER_NEW_CONTEXT_TARGET_OBJECT_KINDS.len(), 1);
        assert_eq!(
            BROWSER_NEW_CONTEXT_TARGET_OBJECT_KINDS[0].as_str(),
            "pwright.browser.browser"
        );
        assert_eq!(
            dependency.map(|dependency| (dependency.id.as_str(), dependency.minimum_version,)),
            Some(("pwright.context_lifecycle", ProtocolVersion::new(1, 0),))
        );
    }

    #[test]
    fn registry_rejects_context_creation_without_context_lifecycle() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<ContextCreationCapability>()
            .expect("creation command ownership should register");

        // Act
        let result = builder.build();

        // Assert
        assert!(result.is_err());
    }
}
