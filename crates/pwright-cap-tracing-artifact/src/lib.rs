//! Independent contract for stopping one trace chunk into an Artifact.
//!
//! Installing base tracing or Artifact transfer alone does not install this
//! producer. The command keeps tracing 1.0 unchanged while preserving the
//! Playwright-shaped remote result: a BrowserContext creates an Artifact child.

use pwright_core::{ObjectId, ObjectKindTag};

#[doc(hidden)]
#[rustfmt::skip]
mod generated_capability;

pub use generated_capability::*;

pub type TraceStopChunkArtifactOptions = ();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceArtifactResult {
    pub artifact_id: ObjectId,
    pub parent_id: ObjectId,
    pub object_kind: ObjectKindTag,
    pub chunk_id: u64,
    pub entry_count: u64,
    pub payload_bytes: u64,
    pub encoded_bytes: u64,
}

impl TraceArtifactResult {
    pub const fn new(
        artifact_id: ObjectId,
        parent_id: ObjectId,
        object_kind: ObjectKindTag,
        chunk_id: u64,
        entry_count: u64,
        payload_bytes: u64,
        encoded_bytes: u64,
    ) -> Self {
        Self {
            artifact_id,
            parent_id,
            object_kind,
            chunk_id,
            entry_count,
            payload_bytes,
            encoded_bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use pwright_cap_artifact::ArtifactCapability;
    use pwright_cap_tracing::TracingCapability;
    use pwright_core::{Capability, CommandSpec, ProtocolVersion, RegistryBuilder};

    use super::{
        TRACE_STOP_CHUNK_ARTIFACT_METHOD, TRACE_STOP_CHUNK_ARTIFACT_TARGET_OBJECT_KINDS,
        TRACING_ARTIFACT_CAPABILITY_ID, TraceStopChunkArtifact, TracingArtifactCapability,
    };

    #[test]
    fn producer_depends_on_tracing_and_artifact_without_expanding_either() {
        // Arrange
        let descriptor = TracingArtifactCapability::DESCRIPTOR;

        // Act
        let dependencies = descriptor
            .dependencies
            .iter()
            .map(|dependency| (dependency.id.as_str(), dependency.minimum_version))
            .collect::<Vec<_>>();

        // Assert
        assert_eq!(
            TRACING_ARTIFACT_CAPABILITY_ID.as_str(),
            "pwright.tracing_artifact"
        );
        assert_eq!(
            TraceStopChunkArtifact::METHOD,
            TRACE_STOP_CHUNK_ARTIFACT_METHOD
        );
        assert_eq!(
            TRACE_STOP_CHUNK_ARTIFACT_TARGET_OBJECT_KINDS[0].as_str(),
            "pwright.browser.browser_context"
        );
        assert_eq!(
            dependencies,
            [
                ("pwright.artifact", ProtocolVersion::new(1, 0)),
                ("pwright.tracing", ProtocolVersion::new(1, 0)),
            ]
        );
    }

    #[test]
    fn producer_cannot_be_installed_without_both_dependencies() {
        // Arrange
        let mut missing = RegistryBuilder::new();
        missing.register::<TracingArtifactCapability>().unwrap();
        let mut complete = RegistryBuilder::new();
        complete.register::<ArtifactCapability>().unwrap();
        complete.register::<TracingCapability>().unwrap();
        complete.register::<TracingArtifactCapability>().unwrap();

        // Act
        let missing_result = missing.build();
        let complete_result = complete.build();

        // Assert
        assert!(missing_result.is_err());
        assert!(complete_result.is_ok());
    }
}
