//! Generated complete server handler chain for explicit handle lifecycle.

#[doc(hidden)]
#[rustfmt::skip]
mod generated_server;

pub use generated_server::*;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_browser_model::BROWSER_OBJECT_KIND_CATALOG;
    use pwright_codec_handle_lifecycle::{HandleLifecycleCodec, HandleLifecycleCodecLimits};
    use pwright_core::ProtocolVersion;
    use pwright_server_wire::ServerCapabilitySource;
    use pwright_transport::{
        FrameLimits, NegotiationLimits, VersionPolicy, WireCapabilityId, WireProtocol,
    };

    use super::{HandleLifecycleServerServices, handle_lifecycle_server_handlers};

    #[test]
    fn generated_handler_advertises_handle_lifecycle_for_the_complete_catalog() {
        // Arrange
        let version = ProtocolVersion::new(1, 0);
        let frame_limits = FrameLimits::new(1_024, 0, 0, 1_024);
        let protocol = Arc::new(WireProtocol::new_exact_directional(
            HandleLifecycleCodec::new(HandleLifecycleCodecLimits::new(256)),
            version,
            frame_limits,
            frame_limits,
        ));
        let handlers = handle_lifecycle_server_handlers(
            protocol,
            HandleLifecycleServerServices {
                handle_dispose: Arc::new(()),
            },
        );
        let capability = WireCapabilityId::new("pwright.handle_lifecycle", 64)
            .expect("capability ID should build");

        // Act
        let manifest = handlers
            .capability_manifest_with_object_kinds(
                BROWSER_OBJECT_KIND_CATALOG,
                NegotiationLimits::new(8, 64),
            )
            .expect("Handle lifecycle handler should preflight");

        // Assert
        assert_eq!(
            manifest.support(&capability),
            Some(VersionPolicy::compatible_through(version))
        );
    }
}
