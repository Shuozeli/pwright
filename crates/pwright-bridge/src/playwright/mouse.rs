//! Mouse input — wraps CDP Input.dispatchMouseEvent.

use std::sync::Arc;

use pwright_cdp::CdpClient;
use pwright_cdp::connection::Result as CdpResult;

/// Click options for mouse operations.
#[derive(Debug, Clone, Default)]
pub struct ClickOptions {
    /// Mouse button: "left" (default), "right", "middle".
    pub button: Option<String>,
    /// Number of clicks (1 = click, 2 = dblclick).
    pub click_count: Option<i32>,
    /// Delay between mousedown and mouseup in milliseconds.
    pub delay_ms: Option<u64>,
}

/// Playwright-compatible Mouse.
///
/// ```rust,ignore
/// page.mouse().click(100.0, 200.0, None).await?;
/// page.mouse().move_to(300.0, 400.0).await?;
/// page.mouse().wheel(0.0, 500.0).await?;
/// ```
pub struct Mouse {
    session: Arc<dyn CdpClient>,
}

impl Mouse {
    pub(crate) fn new(session: Arc<dyn CdpClient>) -> Self {
        Self { session }
    }

    /// Click at (x, y).
    pub async fn click(&self, x: f64, y: f64, options: Option<ClickOptions>) -> CdpResult<()> {
        let opts = options.unwrap_or_default();
        let button = opts.button.as_deref().unwrap_or("left");
        let count = opts.click_count.unwrap_or(1);

        self.session
            .input_dispatch_mouse_event("mousePressed", x, y, Some(button), Some(count), Some(1))
            .await?;

        if let Some(delay) = opts.delay_ms {
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
        }

        self.session
            .input_dispatch_mouse_event("mouseReleased", x, y, Some(button), Some(count), Some(0))
            .await?;

        Ok(())
    }

    /// Double-click at (x, y).
    pub async fn dblclick(&self, x: f64, y: f64) -> CdpResult<()> {
        self.click(
            x,
            y,
            Some(ClickOptions {
                click_count: Some(2),
                ..Default::default()
            }),
        )
        .await
    }

    /// Move mouse to (x, y).
    pub async fn move_to(&self, x: f64, y: f64) -> CdpResult<()> {
        self.session
            .input_dispatch_mouse_event("mouseMoved", x, y, None, None, None)
            .await
    }

    /// Press mouse button down.
    pub async fn down(&self, button: Option<&str>) -> CdpResult<()> {
        let btn = button.unwrap_or("left");
        self.session
            .input_dispatch_mouse_event("mousePressed", 0.0, 0.0, Some(btn), Some(1), Some(1))
            .await
    }

    /// Release mouse button.
    pub async fn up(&self, button: Option<&str>) -> CdpResult<()> {
        let btn = button.unwrap_or("left");
        self.session
            .input_dispatch_mouse_event("mouseReleased", 0.0, 0.0, Some(btn), Some(1), Some(0))
            .await
    }

    /// Scroll by (dx, dy) pixels.
    pub async fn wheel(&self, dx: f64, dy: f64) -> CdpResult<()> {
        // mouseWheel event uses deltaX/deltaY but we use the existing dispatch
        // for now we use the runtime scroll approach
        let js = pwright_js::page::scroll_by(dx as i32, dy as i32);
        self.session.runtime_evaluate(&js).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockCdpClient;

    #[tokio::test]
    async fn test_mouse_click() {
        let mock = Arc::new(MockCdpClient::new());
        let mouse = Mouse::new(mock.clone());
        mouse.click(150.0, 250.0, None).await.unwrap();

        let calls = mock.calls_for("Input.dispatchMouseEvent");
        assert_eq!(calls.len(), 2);

        // First call is mousePressed
        let args = &calls[0].args[0];
        assert_eq!(args["type"], "mousePressed");
        assert_eq!(args["x"], 150.0);
        assert_eq!(args["y"], 250.0);

        // Second call is mouseReleased
        assert_eq!(calls[1].args[0]["type"], "mouseReleased");
    }

    #[tokio::test]
    async fn test_mouse_dblclick() {
        let mock = Arc::new(MockCdpClient::new());
        let mouse = Mouse::new(mock.clone());
        mouse.dblclick(100.0, 200.0).await.unwrap();

        let calls = mock.calls_for("Input.dispatchMouseEvent");
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].args[0]["clickCount"], 2);
    }

    #[tokio::test]
    async fn test_mouse_move() {
        let mock = Arc::new(MockCdpClient::new());
        let mouse = Mouse::new(mock.clone());
        mouse.move_to(300.0, 400.0).await.unwrap();

        let calls = mock.calls_for("Input.dispatchMouseEvent");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].args[0]["type"], "mouseMoved");
    }
}
