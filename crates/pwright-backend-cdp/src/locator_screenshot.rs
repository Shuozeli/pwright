use std::future::Future;
use std::pin::Pin;

use pwright_backend_cdp_locator_screenshot::{
    LocatorScreenshotGeometry, LocatorScreenshotGeometryResolver,
};
use pwright_cap_locator::LocatorErrorDetails;
use pwright_cap_locator_screenshot::{LocatorScreenshotErrorDetails, Selector};
use pwright_core::OperationError;

use crate::locator::CdpLocatorService;

/// Compatibility adapter that lets the complete Locator backend provide the
/// private geometry plugin required by the independent screenshot leaf.
#[derive(Clone)]
pub struct CdpLocatorScreenshotResolver {
    locator: CdpLocatorService,
}

impl CdpLocatorScreenshotResolver {
    pub const fn new(locator: CdpLocatorService) -> Self {
        Self { locator }
    }
}

impl LocatorScreenshotGeometryResolver for CdpLocatorScreenshotResolver {
    fn resolve(
        &self,
        selector: Selector,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        LocatorScreenshotGeometry,
                        OperationError<LocatorScreenshotErrorDetails>,
                    >,
                > + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            let geometry = self
                .locator
                .prepare_screenshot_geometry(selector)
                .await
                .map_err(map_locator_error)?;
            LocatorScreenshotGeometry::new(geometry.x, geometry.y, geometry.width, geometry.height)
                .map_err(|error| {
                    let message = error.to_string();
                    OperationError::new(
                        pwright_core::FailureKind::ProtocolViolation,
                        message.clone(),
                        Some(LocatorScreenshotErrorDetails::InvalidBackendResponse(
                            message,
                        )),
                    )
                })
        })
    }
}

fn map_locator_error(
    error: OperationError<LocatorErrorDetails>,
) -> OperationError<LocatorScreenshotErrorDetails> {
    error.map_details(|details| match details {
        LocatorErrorDetails::InvalidSelector(message) => {
            LocatorScreenshotErrorDetails::InvalidSelector(message)
        }
        LocatorErrorDetails::StrictViolation { selector, matches } => {
            LocatorScreenshotErrorDetails::StrictViolation { selector, matches }
        }
        LocatorErrorDetails::InvalidBackendResponse(message) => {
            LocatorScreenshotErrorDetails::InvalidBackendResponse(message)
        }
        LocatorErrorDetails::Backend(message)
        | LocatorErrorDetails::InvalidTarget(message)
        | LocatorErrorDetails::InvalidValue(message) => {
            LocatorScreenshotErrorDetails::Backend(message)
        }
    })
}
