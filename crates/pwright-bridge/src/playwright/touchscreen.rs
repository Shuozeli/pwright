//! Touchscreen API — emulates touch interactions.

use pwright_cdp::CdpClient;
use pwright_cdp::TouchEventType;
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
            .input_dispatch_touch_event(TouchEventType::Start, x, y)
            .await?;
        self.session
            .input_dispatch_touch_event(TouchEventType::End, x, y)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockCdpClient;

    #[tokio::test]
    async fn test_tap_dispatches_start_then_end() {
        let mock = Arc::new(MockCdpClient::new());
        let ts = Touchscreen::new(mock.clone());
        ts.tap(100.0, 200.0).await.unwrap();

        let calls = mock.calls_for("Input.dispatchTouchEvent");
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].args[0]["type"], "touchStart");
        assert_eq!(calls[0].args[0]["x"], 100.0);
        assert_eq!(calls[0].args[0]["y"], 200.0);
        assert_eq!(calls[1].args[0]["type"], "touchEnd");
        assert_eq!(calls[1].args[0]["x"], 100.0);
        assert_eq!(calls[1].args[0]["y"], 200.0);
    }
}
