//! Portable explicit lifecycle contract for remote JavaScript handles.
//!
//! This opt-in capability releases an already-created handle. Handle creation
//! remains a separate capability so value-only evaluation retains no browser
//! objects and does not depend on this crate.

#[doc(hidden)]
#[rustfmt::skip]
mod generated_capability;

pub use generated_capability::*;

pub type HandleDisposeOptions = ();
pub type HandleDisposeResult = ();

#[cfg(test)]
mod tests {
    use pwright_core::{Capability, CommandSpec, RegistryBuilder};

    use super::{
        HANDLE_DISPOSE_COMMAND_ID, HANDLE_DISPOSE_METHOD, HANDLE_DISPOSE_TARGET_OBJECT_KINDS,
        HANDLE_LIFECYCLE_CAPABILITY_ID, HANDLE_LIFECYCLE_COMMAND_DESCRIPTORS, HandleDispose,
        HandleLifecycleCapability,
    };

    #[test]
    fn handle_lifecycle_capability_targets_both_handle_kinds() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<HandleLifecycleCapability>()
            .expect("handle lifecycle capability should register");

        // Act
        let registry = builder.build().expect("registry should validate");

        // Assert
        assert!(registry.contains(HANDLE_LIFECYCLE_CAPABILITY_ID));
        assert_eq!(
            registry.owner_of(HANDLE_DISPOSE_COMMAND_ID),
            Some(HANDLE_LIFECYCLE_CAPABILITY_ID)
        );
        assert_eq!(HandleDispose::METHOD, HANDLE_DISPOSE_METHOD);
        assert_eq!(HANDLE_LIFECYCLE_COMMAND_DESCRIPTORS.len(), 1);
        assert_eq!(HANDLE_DISPOSE_TARGET_OBJECT_KINDS.len(), 2);
        assert_eq!(
            HANDLE_DISPOSE_TARGET_OBJECT_KINDS[0].as_str(),
            "pwright.browser.element_handle"
        );
        assert_eq!(
            HANDLE_DISPOSE_TARGET_OBJECT_KINDS[1].as_str(),
            "pwright.browser.js_handle"
        );
        assert_eq!(
            HandleLifecycleCapability::DESCRIPTOR.plugin.id,
            HANDLE_LIFECYCLE_CAPABILITY_ID
        );
    }
}
