//! Generated complete server handler chain for optional Locator screenshot.

#[doc(hidden)]
#[rustfmt::skip]
mod generated_server;

pub use generated_server::*;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_browser_model::BROWSER_OBJECT_KIND_CATALOG;
    use pwright_codec_locator_screenshot::{LocatorScreenshotCodec, LocatorScreenshotCodecLimits};
    use pwright_core::{ProtocolVersion, ResourceClass, ResourceLedger, ResourceLimits};
    use pwright_server_wire::ServerCapabilitySource;
    use pwright_transport::{
        FrameLimits, NegotiationLimits, VersionPolicy, WireCapabilityId, WireProtocol,
    };

    use super::{LocatorScreenshotServerServices, locator_screenshot_server_handlers};

    #[test]
    fn generated_chain_advertises_only_the_locator_screenshot_capability() {
        // Arrange
        let version = ProtocolVersion::new(1, 0);
        let frame_limits = FrameLimits::new(8_192, 1, 1_024, 9_216);
        let ledger = ResourceLedger::new(ResourceLimits::deny_all(8, 8).with_class(
            ResourceClass::ScreenshotBytes,
            1_024,
            1_024,
        ));
        let protocol = Arc::new(WireProtocol::new_exact_directional(
            LocatorScreenshotCodec::new(
                LocatorScreenshotCodecLimits::new(1_024, 256, 32, 64),
                ledger,
            ),
            version,
            frame_limits,
            frame_limits,
        ));
        let handlers = locator_screenshot_server_handlers(
            protocol,
            LocatorScreenshotServerServices {
                locator_screenshot: Arc::new(()),
            },
        );
        let locator_screenshot = WireCapabilityId::new("pwright.locator_screenshot", 64)
            .expect("capability ID should build");
        let page_screenshot =
            WireCapabilityId::new("pwright.content", 64).expect("capability ID should build");
        let locator =
            WireCapabilityId::new("pwright.locator", 64).expect("capability ID should build");

        // Act
        let manifest = handlers
            .capability_manifest_with_object_kinds(
                BROWSER_OBJECT_KIND_CATALOG,
                NegotiationLimits::new(8, 64),
            )
            .expect("the generated chain should register its command");

        // Assert
        assert_eq!(
            manifest.support(&locator_screenshot),
            Some(VersionPolicy::compatible_through(version))
        );
        assert_eq!(manifest.support(&page_screenshot), None);
        assert_eq!(manifest.support(&locator), None);
    }
}
