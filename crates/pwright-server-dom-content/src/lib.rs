//! Optional composable server binding for read-only DOM content.

#[doc(hidden)]
#[rustfmt::skip]
mod generated_server;

pub use generated_server::*;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_browser_model::BROWSER_OBJECT_KIND_CATALOG;
    use pwright_codec_dom_content::{DomContentCodec, DomContentCodecLimits};
    use pwright_core::{ProtocolVersion, ResourceClass, ResourceLedger, ResourceLimits};
    use pwright_server_wire::ServerCapabilitySource;
    use pwright_transport::{
        FrameLimits, NegotiationLimits, VersionPolicy, WireCapabilityId, WireProtocol,
    };

    use super::{DomContentServerServices, dom_content_server_handlers};

    #[test]
    fn generated_handlers_advertise_read_only_dom_content_1_1() {
        // Arrange
        let version = ProtocolVersion::new(1, 0);
        let frames = FrameLimits::new(16 * 1024, 1, 1024, 17 * 1024);
        let ledger = ResourceLedger::new(ResourceLimits::deny_all(2, 2).with_class(
            ResourceClass::DomContentBytes,
            1024,
            1024,
        ));
        let protocol = Arc::new(WireProtocol::new_exact_directional(
            DomContentCodec::new(DomContentCodecLimits::new(1024, 256), ledger),
            version,
            frames,
            frames,
        ));
        let handlers = dom_content_server_handlers(
            protocol,
            DomContentServerServices {
                frame_content: Arc::new(()),
                frame_title: Arc::new(()),
            },
        );
        let capability = WireCapabilityId::new("pwright.dom_content", 64).unwrap();

        // Act
        let manifest = handlers
            .capability_manifest_with_object_kinds(
                BROWSER_OBJECT_KIND_CATALOG,
                NegotiationLimits::new(8, 64),
            )
            .expect("handler should preflight");

        // Assert
        assert_eq!(
            manifest.support(&capability),
            Some(VersionPolicy::compatible_through(ProtocolVersion::new(
                1, 1
            )))
        );
    }
}
