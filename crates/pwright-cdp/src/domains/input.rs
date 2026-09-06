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
    KeyDown,
    KeyUp,
}

/// Complete Chromium keyboard-event parameters used by Playwright-shaped
/// keyboard input. String fields may borrow static layout data or one bounded
/// caller-owned key expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyboardEventParams<'a> {
    pub event_type: KeyEventType,
    pub modifiers: i32,
    pub key: &'a str,
    pub code: &'a str,
    pub windows_virtual_key_code: i64,
    pub text: Option<&'a str>,
    pub unmodified_text: Option<&'a str>,
    pub auto_repeat: bool,
    pub location: i64,
    pub is_keypad: bool,
    pub commands: &'a [&'a str],
}

impl KeyEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RawKeyDown => "rawKeyDown",
            Self::KeyDown => "keyDown",
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
    pub async fn input_dispatch_keyboard_event(
        &self,
        params: KeyboardEventParams<'_>,
    ) -> Result<()> {
        let is_key_up = params.event_type == KeyEventType::KeyUp;
        let params = cdp_gen::DispatchKeyEventParams {
            r#type: params.event_type.as_str().to_string(),
            modifiers: Some(i64::from(params.modifiers)),
            text: params.text.map(str::to_string),
            unmodified_text: params.unmodified_text.map(str::to_string),
            key: Some(params.key.to_string()),
            code: Some(params.code.to_string()),
            windows_virtual_key_code: Some(params.windows_virtual_key_code),
            auto_repeat: (!is_key_up).then_some(params.auto_repeat),
            is_keypad: (!is_key_up).then_some(params.is_keypad),
            location: Some(params.location),
            commands: (!is_key_up && !params.commands.is_empty()).then(|| {
                params
                    .commands
                    .iter()
                    .map(|command| command.to_string())
                    .collect()
            }),
            ..Default::default()
        };
        self.send("Input.dispatchKeyEvent", serde_json::to_value(&params)?)
            .await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn input_dispatch_mouse_event(
        &self,
        event_type: MouseEventType,
        x: f64,
        y: f64,
        button: Option<MouseButton>,
        click_count: Option<i32>,
        buttons: Option<i32>,
        modifiers: Option<i32>,
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
        if let Some(modifiers) = modifiers {
            params["modifiers"] = json!(modifiers);
        }
        self.send("Input.dispatchMouseEvent", params).await?;
        Ok(())
    }

    pub async fn input_dispatch_key_event(
        &self,
        event_type: KeyEventType,
        key: &str,
        code: &str,
        windows_virtual_key_code: Option<i64>,
    ) -> Result<()> {
        self.input_dispatch_key_event_with_modifiers(
            event_type,
            key,
            code,
            windows_virtual_key_code,
            None,
        )
        .await
    }

    pub async fn input_dispatch_key_event_with_modifiers(
        &self,
        event_type: KeyEventType,
        key: &str,
        code: &str,
        windows_virtual_key_code: Option<i64>,
        modifiers: Option<i32>,
    ) -> Result<()> {
        let params = cdp_gen::DispatchKeyEventParams {
            r#type: event_type.as_str().to_string(),
            modifiers: modifiers.map(i64::from),
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

    pub async fn input_dispatch_text_key_event(
        &self,
        key: &str,
        code: &str,
        windows_virtual_key_code: Option<i64>,
        text: &str,
    ) -> Result<()> {
        let params = cdp_gen::DispatchKeyEventParams {
            r#type: KeyEventType::KeyDown.as_str().to_string(),
            key: Some(key.to_string()),
            code: Some(code.to_string()),
            text: Some(text.to_string()),
            unmodified_text: Some(text.to_string()),
            windows_virtual_key_code,
            native_virtual_key_code: windows_virtual_key_code,
            ..Default::default()
        };
        self.send("Input.dispatchKeyEvent", serde_json::to_value(&params)?)
            .await?;
        Ok(())
    }

    pub async fn input_insert_text(&self, text: &str) -> Result<()> {
        let params = cdp_gen::InsertTextParams {
            text: text.to_string(),
        };
        self.send("Input.insertText", serde_json::to_value(&params)?)
            .await?;
        Ok(())
    }

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
