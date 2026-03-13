//! Touchscreen API — emulates touch interactions.

use pwright_cdp::CdpClient;
use pwright_cdp::connection::Result as CdpResult;
use std::sync::Arc;

/// Touchscreen interface for dispatching touch events.
pub struct Touchscreen {
    session: Arc<dyn CdpClient>,
}

impl Touchscreen {
    pub fn new(session: Arc<dyn CdpClient>) -> Self {
        Self { session }
    }

    /// Dispatch a tap (touchStart + touchEnd) at the given coordinates.
    pub async fn tap(&self, x: f64, y: f64) -> CdpResult<()> {
        self.session
            .input_dispatch_touch_event("touchStart", x, y)
            .await?;
        self.session
            .input_dispatch_touch_event("touchEnd", x, y)
            .await?;
        Ok(())
    }
}
