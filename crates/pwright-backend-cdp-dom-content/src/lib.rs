//! Chromium CDP adapter for portable, read-only Frame DOM serialization.
//!
//! This backend leaf attaches to an existing target-local CDP client and
//! caller-provisioned utility execution context. It does not install public
//! JavaScript evaluation, screenshots, navigation, a runtime, a browser
//! provider, Node.js, or browser acquisition.

mod dom_content;

pub use dom_content::{
    CdpDomContentProductionProfile, CdpDomContentService, CdpDomContentServiceFactory,
};

use pwright_backend_cdp_common::classify_cdp_failure;
use pwright_browser_backend::{
    BackendDescriptor, BackendFeatureDescriptor, BackendFeatureId, BackendId,
    BackendImplementation, BackendProtocolId, BrowserBackend, BrowserEngine,
};
use pwright_cap_dom_content::{DOM_CONTENT_CAPABILITY_ID, FRAME_CONTENT_COMMAND_ID};
use pwright_core::ProtocolVersion;

pub const CHROMIUM_CDP_DOM_CONTENT_BACKEND_ID: BackendId =
    BackendId::new("pwright.backend.chromium-cdp.dom-content");
pub const CHROMIUM_CDP_PROTOCOL_ID: BackendProtocolId = BackendProtocolId::new("cdp");
pub const CDP_FRAME_CONTENT_FEATURE: BackendFeatureId = BackendFeatureId::new("frame.content");
pub const CHROMIUM_CDP_DOM_CONTENT_FEATURES: &[BackendFeatureDescriptor] =
    &[BackendFeatureDescriptor::new(
        CDP_FRAME_CONTENT_FEATURE,
        DOM_CONTENT_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        FRAME_CONTENT_COMMAND_ID,
        BackendImplementation::Emulated,
    )];

pub enum ChromiumCdpDomContentBackend {}

impl BrowserBackend for ChromiumCdpDomContentBackend {
    const DESCRIPTOR: BackendDescriptor = BackendDescriptor::new(
        CHROMIUM_CDP_DOM_CONTENT_BACKEND_ID,
        "Chromium CDP read-only DOM content",
        BrowserEngine::Chromium,
        CHROMIUM_CDP_PROTOCOL_ID,
        CHROMIUM_CDP_DOM_CONTENT_FEATURES,
    );
}

#[cfg(test)]
mod tests {
    use pwright_browser_backend::{BackendImplementation, BrowserBackend};

    use super::ChromiumCdpDomContentBackend;

    #[test]
    fn descriptor_contains_only_emulated_frame_content() {
        // Arrange
        let descriptor = ChromiumCdpDomContentBackend::DESCRIPTOR;

        // Act
        let features = descriptor
            .features
            .iter()
            .map(|feature| (feature.id.as_str(), feature.implementation))
            .collect::<Vec<_>>();

        // Assert
        assert_eq!(
            features,
            [("frame.content", BackendImplementation::Emulated)]
        );
        assert!(descriptor.event_features.is_empty());
    }
}
