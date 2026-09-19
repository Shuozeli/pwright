//! Optional backend-neutral Page Mouse contracts; no browser or input state.
use pwright_input_model::{InvalidMouseClickCount, InvalidMouseMoveSteps};
pub use pwright_input_model::{MouseClickCount, MouseMoveSteps};
use std::{fmt, time::Duration};
#[doc(hidden)]
#[rustfmt::skip]
mod generated_capability;
pub use generated_capability::*;

pub type MouseSuccess = ();
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MouseArgumentError {
    NonFinite,
    Steps(InvalidMouseMoveSteps),
    ClickCount(InvalidMouseClickCount),
}
impl fmt::Display for MouseArgumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite => f.write_str("mouse coordinates and deltas must be finite"),
            Self::Steps(error) => error.fmt(f),
            Self::ClickCount(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for MouseArgumentError {}
/// Finite CSS-pixel coordinates or wheel deltas; fractional and negative values
/// are preserved. No clamping, scrolling, targeting or screen bounds are implied.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MouseNumber(f64);
impl TryFrom<f64> for MouseNumber {
    type Error = MouseArgumentError;
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if !value.is_finite() {
            return Err(MouseArgumentError::NonFinite);
        }
        Ok(Self(value))
    }
}
impl MouseNumber {
    pub const fn get(self) -> f64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoveParams {
    pub x: MouseNumber,
    pub y: MouseNumber,
    pub steps: MouseMoveSteps,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DownParams {
    pub button: MouseButton,
    pub click_count: i32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UpParams {
    pub button: MouseButton,
    pub click_count: i32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClickParams {
    pub x: MouseNumber,
    pub y: MouseNumber,
    pub button: MouseButton,
    pub click_count: MouseClickCount,
    pub delay: Option<Duration>,
}
/// Double-click always sends two pairs; its type has no independently mutable count.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DblclickParams {
    pub x: MouseNumber,
    pub y: MouseNumber,
    pub button: MouseButton,
    pub delay: Option<Duration>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelParams {
    pub delta_x: MouseNumber,
    pub delta_y: MouseNumber,
}

#[cfg(test)]
mod tests;
