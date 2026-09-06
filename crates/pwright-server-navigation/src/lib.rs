//! Optional composable server bindings for the navigation capability.
//!
//! Applications supply one service per command and choose their own backend,
//! executor, transport, and connection supervisor. This crate starts no task,
//! runtime, browser, or Node.js process.

#[doc(hidden)]
#[rustfmt::skip]
mod generated_server;

pub use generated_server::*;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_browser_model::BROWSER_OBJECT_KIND_CATALOG;
    use pwright_codec_navigation::{NavigationCodec, NavigationCodecLimits};
    use pwright_core::ProtocolVersion;
    use pwright_server_wire::ServerCapabilitySource;
    use pwright_transport::{
        FrameLimits, NegotiationLimits, VersionPolicy, WireCapabilityId, WireProtocol,
    };

    use super::{NavigationServerServices, navigation_server_handlers};

    #[test]
    fn generated_chain_advertises_the_complete_navigation_capability() {
        // Arrange
        let version = ProtocolVersion::new(1, 0);
        let frame_limits = FrameLimits::new(1_024, 0, 0, 1_024);
        let protocol = Arc::new(WireProtocol::new_exact_directional(
            NavigationCodec::new(NavigationCodecLimits::new(256, 8, 1_024)),
            version,
            frame_limits,
            frame_limits,
        ));
        let handlers = navigation_server_handlers(
            protocol,
            NavigationServerServices {
                begin_navigation: Arc::new(()),
                goto: Arc::new(()),
            },
        );
        let capability =
            WireCapabilityId::new("pwright.navigation", 64).expect("capability ID should build");

        // Act
        let manifest = handlers
            .capability_manifest_with_object_kinds(
                BROWSER_OBJECT_KIND_CATALOG,
                NegotiationLimits::new(8, 64),
            )
            .expect("the generated chain should register every navigation command");

        // Assert
        assert_eq!(
            manifest.support(&capability),
            Some(VersionPolicy::compatible_through(version))
        );
    }
}
