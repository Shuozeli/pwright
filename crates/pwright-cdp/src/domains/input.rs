//! Input domain — mouse events, keyboard events, text insertion.

use serde_json::json;

use crate::connection::Result;
use crate::generated::input as cdp_gen;
use crate::session::CdpSession;

impl CdpSession {
    /// Dispatch a mouse event.
    pub async fn input_dispatch_mouse_event(
        &self,
        event_type: &str,
        x: f64,
        y: f64,
        button: Option<&str>,
        click_count: Option<i32>,
        buttons: Option<i32>,
    ) -> Result<()> {
        let mut params = json!({
            "type": event_type,
            "x": x,
            "y": y,
        });
        if let Some(b) = button {
            params["button"] = json!(b);
        }
        if let Some(cc) = click_count {
            params["clickCount"] = json!(cc);
        }
        if let Some(btns) = buttons {
            params["buttons"] = json!(btns);
        }
        self.send("Input.dispatchMouseEvent", params).await?;
        Ok(())
    }

    /// Dispatch a key event.
    pub async fn input_dispatch_key_event(
        &self,
        event_type: &str,
        key: &str,
        code: &str,
        windows_virtual_key_code: Option<i64>,
    ) -> Result<()> {
        let params = cdp_gen::DispatchKeyEventParams {
            r#type: event_type.to_string(),
            key: Some(key.to_string()),
            code: Some(code.to_string()),
            windows_virtual_key_code,
            native_virtual_key_code: windows_virtual_key_code,
            ..Default::default()
        };
        self.send("Input.dispatchKeyEvent", serde_json::to_value(&params)?)
            .await?;
        Ok(())
    }

    /// Insert text (for character input after focus).
    pub async fn input_insert_text(&self, text: &str) -> Result<()> {
        let params = cdp_gen::InsertTextParams {
            text: text.to_string(),
        };
        self.send("Input.insertText", serde_json::to_value(&params)?)
            .await?;
        Ok(())
    }

    /// Dispatch a touch event (touchStart, touchMove, touchEnd).
    pub async fn input_dispatch_touch_event(&self, event_type: &str, x: f64, y: f64) -> Result<()> {
        // Touch points must be empty for touchEnd, populated for touchStart/touchMove.
        let touch_points = if event_type == "touchEnd" {
            json!([])
        } else {
            json!([{ "x": x, "y": y }])
        };
        self.send(
            "Input.dispatchTouchEvent",
            json!({
                "type": event_type,
                "touchPoints": touch_points,
            }),
        )
        .await?;
        Ok(())
    }
}
