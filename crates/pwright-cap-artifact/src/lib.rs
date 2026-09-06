//! Portable, opt-in Artifact and pull-Stream command contracts.
//!
//! This capability does not create an Artifact producer, filesystem, archive,
//! transport, executor, task, browser backend, tracing contributor, or browser
//! downloader. Producers and delivery compositions remain explicit.

#[doc(hidden)]
#[rustfmt::skip]
mod generated_capability;

pub use generated_capability::*;
pub use pwright_artifact_model::{
    ArtifactDeleteOptions, ArtifactFailureOptions, ArtifactFailureResult,
    ArtifactOpenStreamOptions, ArtifactReadResult, StreamCloseOptions, StreamInitializer,
    StreamReadOptions,
};

pub type ArtifactUnit = ();

#[cfg(test)]
mod tests {
    use pwright_core::{CommandSpec, RegistryBuilder};

    use super::*;

    #[test]
    fn artifact_registers_five_commands_on_exact_object_kinds() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<ArtifactCapability>()
            .expect("artifact capability should register");

        // Act
        let registry = builder.build().expect("registry should validate");

        // Assert
        assert!(registry.contains(ARTIFACT_CAPABILITY_ID));
        assert_eq!(ARTIFACT_COMMAND_DESCRIPTORS.len(), 5);
        for descriptor in ARTIFACT_COMMAND_DESCRIPTORS {
            assert_eq!(
                registry.owner_of(descriptor.command()),
                Some(ARTIFACT_CAPABILITY_ID)
            );
            assert_eq!(descriptor.targets().len(), 1);
        }
        assert_eq!(ArtifactOpenStream::METHOD, ARTIFACT_OPEN_STREAM_METHOD);
        assert_eq!(ArtifactFailure::METHOD, ARTIFACT_FAILURE_METHOD);
        assert_eq!(ArtifactDelete::METHOD, ARTIFACT_DELETE_METHOD);
        assert_eq!(StreamRead::METHOD, STREAM_READ_METHOD);
        assert_eq!(StreamClose::METHOD, STREAM_CLOSE_METHOD);
        assert_eq!(
            ARTIFACT_OPEN_STREAM_TARGET_OBJECT_KINDS[0].as_str(),
            "pwright.browser.artifact"
        );
        assert_eq!(
            STREAM_READ_TARGET_OBJECT_KINDS[0].as_str(),
            "pwright.browser.stream"
        );
    }
}
