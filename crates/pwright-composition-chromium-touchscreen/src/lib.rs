//! Explicit Chromium Page selection for the optional Touchscreen capability.
//! This leaf owns no task or executor; its caller must retain and retire every
//! returned native operation.

use pwright_backend_cdp_input::{CdpInputCoordinator, CdpModifiers};
use pwright_backend_cdp_touchscreen::{
    CdpTouchscreenService, CdpTouchscreenServiceFactory, CdpTouchscreenTapOperation,
};
use pwright_cap_touchscreen::{TapParams, TouchscreenErrorDetails};
use pwright_cdp::CdpClient;
use pwright_core::{ObjectId, OperationError};
use std::sync::Arc;

#[derive(Clone)]
pub struct ChromiumTouchscreenPage {
    page_id: ObjectId,
    service: CdpTouchscreenService,
}

impl ChromiumTouchscreenPage {
    pub fn bind(
        page_id: ObjectId,
        client: Arc<dyn CdpClient>,
        input: CdpInputCoordinator,
        factory: &CdpTouchscreenServiceFactory,
    ) -> Result<Self, OperationError<TouchscreenErrorDetails>> {
        Ok(Self {
            page_id,
            service: factory.bind(client, input)?,
        })
    }

    pub const fn page_id(&self) -> ObjectId {
        self.page_id
    }

    pub fn shares_input_state_with(&self, input: &CdpInputCoordinator) -> bool {
        self.service.shares_input_state_with(input)
    }

    pub fn start(
        &self,
        params: TapParams,
    ) -> Result<CdpTouchscreenTapOperation, OperationError<TouchscreenErrorDetails>> {
        self.service.start(params)
    }

    pub fn start_with_modifiers(
        &self,
        params: TapParams,
        modifiers: Option<CdpModifiers>,
    ) -> Result<CdpTouchscreenTapOperation, OperationError<TouchscreenErrorDetails>> {
        self.service.start_with_modifiers(params, modifiers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_backend_cdp_input::CdpInputPolicy;
    use pwright_backend_cdp_touchscreen::{CdpTouchscreenProductionProfile, CdpTouchscreenService};
    use pwright_core::{ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope};
    use pwright_fake::FakeCdpClient;
    use std::num::NonZeroUsize;

    #[tokio::test]
    async fn page_binding_preserves_identity_and_shared_input_owner() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
        let operation_bytes =
            CdpTouchscreenService::required_operation_bytes(client.clone(), input.clone());
        let profile = CdpTouchscreenProductionProfile::new(
            ResourceScope::extension(141, 1),
            operation_bytes,
            NonZeroUsize::new(1).unwrap(),
        )
        .unwrap();
        let required = profile.requirements();
        let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
        for class in ResourceClass::ALL {
            let bound = required.class(class);
            limits = limits.with_class(class, bound.global, bound.per_scope);
        }
        let resources = ProductionResourceLedger::new(limits, required).unwrap();
        let factory =
            CdpTouchscreenServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();

        // Act
        let page =
            ChromiumTouchscreenPage::bind(ObjectId::new(17), client, input.clone(), &factory)
                .unwrap();

        // Assert
        assert_eq!(page.page_id(), ObjectId::new(17));
        assert!(page.shares_input_state_with(&input));
    }
}
