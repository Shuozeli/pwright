//! Portable explicit lifecycle contracts for Pages.
//!
//! Ordinary close is termination-oriented. Before-unload close is a separate
//! initiation-only command because a dialog may keep the Page alive.

#[doc(hidden)]
#[rustfmt::skip]
mod generated_capability;

pub use generated_capability::*;

pub type PageCloseOptions = ();
pub type PageCloseResult = ();
pub type PageRunBeforeUnloadOptions = ();
pub type PageRunBeforeUnloadResult = ();

#[cfg(test)]
mod tests {
    use pwright_core::{Capability, CommandSpec, RegistryBuilder};

    use super::{
        PAGE_CLOSE_COMMAND_ID, PAGE_CLOSE_METHOD, PAGE_CLOSE_TARGET_OBJECT_KINDS,
        PAGE_LIFECYCLE_CAPABILITY_ID, PAGE_LIFECYCLE_COMMAND_DESCRIPTORS,
        PAGE_RUN_BEFORE_UNLOAD_COMMAND_ID, PAGE_RUN_BEFORE_UNLOAD_METHOD,
        PAGE_RUN_BEFORE_UNLOAD_TARGET_OBJECT_KINDS, PageClose, PageLifecycleCapability,
        PageRunBeforeUnload,
    };

    #[test]
    fn page_lifecycle_registers_two_page_only_commands() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<PageLifecycleCapability>()
            .expect("page lifecycle capability should register");

        // Act
        let registry = builder.build().expect("registry should validate");

        // Assert
        assert!(registry.contains(PAGE_LIFECYCLE_CAPABILITY_ID));
        assert_eq!(
            registry.owner_of(PAGE_CLOSE_COMMAND_ID),
            Some(PAGE_LIFECYCLE_CAPABILITY_ID)
        );
        assert_eq!(
            registry.owner_of(PAGE_RUN_BEFORE_UNLOAD_COMMAND_ID),
            Some(PAGE_LIFECYCLE_CAPABILITY_ID)
        );
        assert_eq!(PageClose::METHOD, PAGE_CLOSE_METHOD);
        assert_eq!(PageRunBeforeUnload::METHOD, PAGE_RUN_BEFORE_UNLOAD_METHOD);
        assert_eq!(PAGE_LIFECYCLE_COMMAND_DESCRIPTORS.len(), 2);
        for targets in [
            PAGE_CLOSE_TARGET_OBJECT_KINDS,
            PAGE_RUN_BEFORE_UNLOAD_TARGET_OBJECT_KINDS,
        ] {
            assert_eq!(targets.len(), 1);
            assert_eq!(targets[0].as_str(), "pwright.browser.page");
        }
        assert_eq!(
            PageLifecycleCapability::DESCRIPTOR.plugin.id,
            PAGE_LIFECYCLE_CAPABILITY_ID
        );
    }
}
