//! Fixed-size foreign argument conversion; all input behavior stays in the host.
use crate::{NativeSession, binding_error, boundary};
use napi::{Env, JsString, Unknown, bindgen_prelude::PromiseRaw};
use napi_derive::napi;
use pwright_node_host::{HostError, ffi_budget::FfiLane, mouse::*};
use std::{sync::Arc, time::Duration};

// Numeric fields avoid owned string/object graphs before FFI admission. This is
// a private ABI, not the public JS options type. Do not accept integer coercion.
#[napi(object)]
pub struct NativeMouseOptions {
    pub button: f64,
    pub steps: f64,
    pub click_count: f64,
    pub delay_ms: f64,
}

enum Operation {
    Move(MoveParams),
    Down(DownParams),
    Up(UpParams),
    Click(ClickParams),
    Dblclick(DblclickParams),
    Wheel(WheelParams),
}
fn invalid(error: impl std::fmt::Display) -> HostError {
    HostError::InvalidArgument(error.to_string())
}
fn button(value: f64) -> Result<MouseButton, HostError> {
    match value {
        0.0 => Ok(MouseButton::Left),
        1.0 => Ok(MouseButton::Right),
        2.0 => Ok(MouseButton::Middle),
        _ => Err(invalid("invalid Mouse button")),
    }
}
fn count(value: f64) -> Result<i32, HostError> {
    if !value.is_finite()
        || value.fract() != 0.0
        || value < f64::from(i32::MIN)
        || value > f64::from(i32::MAX)
    {
        return Err(invalid("Mouse clickCount must be a signed 32-bit integer"));
    }
    Ok(value as i32)
}
fn delay(value: f64) -> Result<Option<Duration>, HostError> {
    if !value.is_finite() || value < 0.0 || value > f64::from(u32::MAX) {
        return Err(invalid(
            "Mouse delay must be finite and between 0 and 4294967295 ms",
        ));
    }
    Ok((value > 0.0).then(|| Duration::from_secs_f64(value / 1000.0)))
}
impl Operation {
    fn parse(name: &str, x: f64, y: f64, options: NativeMouseOptions) -> Result<Self, HostError> {
        Ok(match name {
            "move" => Self::Move(MoveParams {
                x: x.try_into().map_err(invalid)?,
                y: y.try_into().map_err(invalid)?,
                steps: options.steps.try_into().map_err(invalid)?,
            }),
            "down" => Self::Down(DownParams {
                button: button(options.button)?,
                click_count: count(options.click_count)?,
            }),
            "up" => Self::Up(UpParams {
                button: button(options.button)?,
                click_count: count(options.click_count)?,
            }),
            "click" => Self::Click(ClickParams {
                x: x.try_into().map_err(invalid)?,
                y: y.try_into().map_err(invalid)?,
                button: button(options.button)?,
                click_count: options.click_count.try_into().map_err(invalid)?,
                delay: delay(options.delay_ms)?,
            }),
            "dblclick" => Self::Dblclick(DblclickParams {
                x: x.try_into().map_err(invalid)?,
                y: y.try_into().map_err(invalid)?,
                button: button(options.button)?,
                delay: delay(options.delay_ms)?,
            }),
            "wheel" => Self::Wheel(WheelParams {
                delta_x: x.try_into().map_err(invalid)?,
                delta_y: y.try_into().map_err(invalid)?,
            }),
            _ => return Err(invalid("unknown Mouse operation")),
        })
    }
}

#[napi]
impl NativeSession {
    #[napi]
    pub fn mouse<'env>(
        &self,
        env: &'env Env,
        page: JsString<'env>,
        operation: JsString<'env>,
        x: f64,
        y: f64,
        options: NativeMouseOptions,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        // Admission precedes owned Page/operation strings and typed conversion.
        // spawn_published separately measures/reserves the concrete future body,
        // retaining the settlement ticket through JS delivery or env teardown.
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let page = boundary::bounded_string(page, 20, "page ID")?;
        let name = boundary::bounded_string(operation, 16, "Mouse operation")?;
        let operation = Operation::parse(&name, x, y, options).map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        let start = host.admit_input_start().map_err(binding_error)?;
        boundary::spawn_published(
            env,
            execution,
            self.publications.clone(),
            start.run(async move {
                match operation {
                    Operation::Move(params) => host.mouse_move(&page, params).await,
                    Operation::Down(params) => host.mouse_down(&page, params).await,
                    Operation::Up(params) => host.mouse_up(&page, params).await,
                    Operation::Click(params) => host.mouse_click(&page, params).await,
                    Operation::Dblclick(params) => host.mouse_dblclick(&page, params).await,
                    Operation::Wheel(params) => host.mouse_wheel(&page, params).await,
                }
            }),
        )
    }
}
