//! Input domain — mouse events, keyboard events, text insertion.

use serde_json::json;

use crate::connection::Result;
use crate::generated::input as cdp_gen;
use crate::session::CdpSession;

/// CDP mouse event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEventType {
    Pressed,
    Released,
    Moved,
}

impl MouseEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pressed => "mousePressed",
            Self::Released => "mouseReleased",
            Self::Moved => "mouseMoved",
        }
    }
}

/// CDP mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MouseButton {
    #[default]
    Left,
    Right,
    Middle,
}

impl MouseButton {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Middle => "middle",
        }
    }
}

/// CDP keyboard event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEventType {
    RawKeyDown,
    KeyUp,
}

impl KeyEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RawKeyDown => "rawKeyDown",
            Self::KeyUp => "keyUp",
        }
    }
}

/// CDP touch event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchEventType {
    Start,
    End,
}

impl TouchEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Start => "touchStart",
            Self::End => "touchEnd",
        }
    }
}

impl CdpSession {
    /// Dispatch a mouse event.
    pub async fn input_dispatch_mouse_event(
        &self,
        event_type: MouseEventType,
        x: f64,
        y: f64,
        button: Option<MouseButton>,
        click_count: Option<i32>,
        buttons: Option<i32>,
    ) -> Result<()> {
        let mut params = json!({
            "type": event_type.as_str(),
            "x": x,
            "y": y,
        });
        if let Some(b) = button {
            params["button"] = json!(b.as_str());
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
        event_type: KeyEventType,
        key: &str,
        code: &str,
        windows_virtual_key_code: Option<i64>,
    ) -> Result<()> {
        let params = cdp_gen::DispatchKeyEventParams {
            r#type: event_type.as_str().to_string(),
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

    /// Dispatch a touch event (touchStart, touchEnd).
    pub async fn input_dispatch_touch_event(
        &self,
        event_type: TouchEventType,
        x: f64,
        y: f64,
    ) -> Result<()> {
        let touch_points = match event_type {
            TouchEventType::End => json!([]),
            TouchEventType::Start => json!([{ "x": x, "y": y }]),
        };
        self.send(
            "Input.dispatchTouchEvent",
            json!({
                "type": event_type.as_str(),
                "touchPoints": touch_points,
            }),
        )
        .await?;
        Ok(())
    }
}
