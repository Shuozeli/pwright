//! Chromium CDP adapter for portable, value-only JavaScript evaluation.
//!
//! This backend leaf attaches to an existing target-local CDP client. It does
//! not install navigation, Handle creation, a runtime, a browser provider,
//! Node.js, or browser acquisition.

mod evaluation;

pub use evaluation::{
    CdpEvaluationLimits, CdpEvaluationProductionProfile, CdpEvaluationService,
    CdpEvaluationServiceFactory,
};
#[doc(hidden)]
pub use evaluation::{EVALUATION_RUNTIME, EvaluationLease, parse_response};

use pwright_backend_cdp_common::classify_cdp_failure;
use pwright_browser_backend::{
    BackendDescriptor, BackendFeatureDescriptor, BackendFeatureId, BackendId,
    BackendImplementation, BackendProtocolId, BrowserBackend, BrowserEngine,
};
use pwright_cap_evaluation::{EVALUATE_VALUE_COMMAND_ID, EVALUATION_CAPABILITY_ID};
use pwright_core::ProtocolVersion;

pub const CHROMIUM_CDP_EVALUATION_BACKEND_ID: BackendId =
    BackendId::new("pwright.backend.chromium-cdp.evaluation");
pub const CHROMIUM_CDP_PROTOCOL_ID: BackendProtocolId = BackendProtocolId::new("cdp");
pub const CDP_EVALUATE_VALUE_FEATURE: BackendFeatureId =
    BackendFeatureId::new("evaluation.evaluate-value");
pub const CHROMIUM_CDP_EVALUATION_FEATURES: &[BackendFeatureDescriptor] =
    &[BackendFeatureDescriptor::new(
        CDP_EVALUATE_VALUE_FEATURE,
        EVALUATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        EVALUATE_VALUE_COMMAND_ID,
        BackendImplementation::Emulated,
    )];

pub enum ChromiumCdpEvaluationBackend {}

impl BrowserBackend for ChromiumCdpEvaluationBackend {
    const DESCRIPTOR: BackendDescriptor = BackendDescriptor::new(
        CHROMIUM_CDP_EVALUATION_BACKEND_ID,
        "Chromium CDP value evaluation",
        BrowserEngine::Chromium,
        CHROMIUM_CDP_PROTOCOL_ID,
        CHROMIUM_CDP_EVALUATION_FEATURES,
    );
}

#[cfg(test)]
mod tests {
    use pwright_browser_backend::BrowserBackend;

    use super::ChromiumCdpEvaluationBackend;

    #[test]
    fn descriptor_contains_only_value_evaluation() {
        // Arrange
        let descriptor = ChromiumCdpEvaluationBackend::DESCRIPTOR;

        // Act
        let features = descriptor
            .features
            .iter()
            .map(|feature| feature.id.as_str())
            .collect::<Vec<_>>();

        // Assert
        assert_eq!(features, ["evaluation.evaluate-value"]);
        assert!(descriptor.event_features.is_empty());
    }
}
