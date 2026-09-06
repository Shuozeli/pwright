//! Generated opt-in typed server-handler chain for Artifact and Stream.

#[doc(hidden)]
#[rustfmt::skip]
mod generated_server;

pub use generated_server::*;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_browser_model::BROWSER_OBJECT_KIND_CATALOG;
    use pwright_codec_artifact::{ArtifactCodec, ArtifactCodecLimits};
    use pwright_core::{ProtocolVersion, ResourceClass, ResourceLedger, ResourceLimits};
    use pwright_server_wire::{ServerCapabilityInventory, ServerCapabilitySource};
    use pwright_transport::{
        FrameLimits, NegotiationLimits, VersionPolicy, WireCapabilityId, WireProtocol,
    };

    use super::{ArtifactServerServices, artifact_server_handlers};

    fn protocol() -> Arc<WireProtocol<ArtifactCodec>> {
        let frame_limits = FrameLimits::new(4_096, 1, 64, 4_160);
        let ledger = ResourceLedger::new(ResourceLimits::deny_all(8, 8).with_class(
            ResourceClass::ArtifactBytes,
            64,
            64,
        ));
        Arc::new(WireProtocol::new_exact_directional(
            ArtifactCodec::new(ArtifactCodecLimits::new(64, 256), ledger),
            ProtocolVersion::new(1, 0),
            frame_limits,
            frame_limits,
        ))
    }

    #[test]
    fn generated_chain_advertises_the_complete_artifact_capability() {
        // Arrange
        let handlers = artifact_server_handlers(
            protocol(),
            ArtifactServerServices {
                artifact_delete: Arc::new(()),
                artifact_failure: Arc::new(()),
                artifact_open_stream: Arc::new(()),
                stream_close: Arc::new(()),
                stream_read: Arc::new(()),
            },
        );
        let capability = WireCapabilityId::new("pwright.artifact", 64).unwrap();

        // Act
        let manifest = handlers
            .capability_manifest_with_object_kinds(
                BROWSER_OBJECT_KIND_CATALOG,
                NegotiationLimits::new(8, 64),
            )
            .unwrap();

        // Assert
        assert_eq!(
            manifest.support(&capability),
            Some(VersionPolicy::compatible_through(ProtocolVersion::new(
                1, 0
            )))
        );
    }

    #[test]
    fn composition_without_artifact_handlers_does_not_advertise_artifact() {
        // Arrange
        let inventory = ServerCapabilityInventory::with_object_kinds(BROWSER_OBJECT_KIND_CATALOG)
            .expect("browser object catalog should be valid");
        let capability = WireCapabilityId::new("pwright.artifact", 64).unwrap();

        // Act
        let manifest = inventory
            .manifest(NegotiationLimits::new(8, 64))
            .expect("empty server composition should produce an empty manifest");

        // Assert
        assert_eq!(manifest.support(&capability), None);
    }
}
