//! Generated opt-in typed server-handler chain for tracing.

#[doc(hidden)]
#[rustfmt::skip]
mod generated_server;

pub use generated_server::*;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_browser_model::BROWSER_OBJECT_KIND_CATALOG;
    use pwright_codec_tracing::{TracingCodec, TracingCodecLimits};
    use pwright_core::ProtocolVersion;
    use pwright_server_wire::{ServerCapabilityInventory, ServerCapabilitySource};
    use pwright_transport::{
        FrameLimits, NegotiationLimits, VersionPolicy, WireCapabilityId, WireProtocol,
    };

    use super::{TracingServerServices, tracing_server_handlers};

    #[test]
    fn generated_chain_advertises_the_complete_tracing_capability() {
        // Arrange
        let version = ProtocolVersion::new(1, 0);
        let frame_limits = FrameLimits::new(4_096, 0, 0, 4_096);
        let protocol = Arc::new(WireProtocol::new_exact_directional(
            TracingCodec::new(TracingCodecLimits::new(256)),
            version,
            frame_limits,
            frame_limits,
        ));
        let handlers = tracing_server_handlers(
            protocol,
            TracingServerServices {
                trace_group: Arc::new(()),
                trace_group_end: Arc::new(()),
                trace_start: Arc::new(()),
                trace_start_chunk: Arc::new(()),
                trace_stop: Arc::new(()),
                trace_stop_chunk: Arc::new(()),
            },
        );
        let capability = WireCapabilityId::new("pwright.tracing", 64).unwrap();

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
            Some(VersionPolicy::compatible_through(version))
        );
    }

    #[test]
    fn composition_without_tracing_handlers_does_not_advertise_tracing() {
        // Arrange
        let inventory = ServerCapabilityInventory::with_object_kinds(BROWSER_OBJECT_KIND_CATALOG)
            .expect("browser object catalog should be valid");
        let capability = WireCapabilityId::new("pwright.tracing", 64).unwrap();

        // Act
        let manifest = inventory
            .manifest(NegotiationLimits::new(8, 64))
            .expect("empty server composition should produce an empty manifest");

        // Assert
        assert_eq!(manifest.support(&capability), None);
    }
}
