//! Node-owned string conversion and typed delegation only.
use crate::{NativeSession, binding_error, boundary};
use napi::{Env, JsString, Unknown, bindgen_prelude::PromiseRaw};
use napi_derive::napi;
use pwright_node_host::{
    HostError,
    ffi_budget::FfiLane,
    keyboard::{KEYBOARD_KEY_MAX_BYTES, KEYBOARD_TEXT_MAX_BYTES},
};
use std::{sync::Arc, time::Duration};

enum Operation {
    Down,
    Up,
    Press,
    InsertText,
    Type,
}
impl Operation {
    fn parse(value: &str) -> Result<Self, HostError> {
        match value {
            "down" => Ok(Self::Down),
            "up" => Ok(Self::Up),
            "press" => Ok(Self::Press),
            "insertText" => Ok(Self::InsertText),
            "type" => Ok(Self::Type),
            _ => Err(HostError::InvalidArgument(
                "unknown Keyboard operation".into(),
            )),
        }
    }
    fn maximum(&self) -> usize {
        match self {
            Self::Down | Self::Up | Self::Press => KEYBOARD_KEY_MAX_BYTES,
            Self::InsertText | Self::Type => KEYBOARD_TEXT_MAX_BYTES,
        }
    }
}
fn input(value: JsString<'_>, maximum: usize) -> napi::Result<String> {
    if value.utf8_len()? > maximum {
        return Err(binding_error(HostError::ResourceExhausted(format!(
            "Keyboard string exceeds {maximum} bytes"
        ))));
    }
    let length = value.utf16_len()?;
    let units = value.into_utf16()?;
    String::from_utf16(&units.as_slice()[..length]).map_err(|_| {
        binding_error(HostError::UnsupportedOperation(
            "unpaired UTF-16 in Keyboard input".into(),
        ))
    })
}

#[napi]
impl NativeSession {
    #[napi]
    pub fn keyboard<'env>(
        &self,
        env: &'env Env,
        page: JsString<'env>,
        operation: JsString<'env>,
        value: JsString<'env>,
        delay_ms: f64,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        // Covers overlapping UTF-16/UTF-8 input conversion, owned strings and
        // command/result settlement; admission precedes any owned argument copy.
        let execution = self
            .ffi
            .admit(
                FfiLane::Command,
                6 * KEYBOARD_TEXT_MAX_BYTES as u64 + boundary::SMALL_ALLOWANCE,
            )
            .map_err(binding_error)?;
        let page = boundary::bounded_string(page, 20, "page ID")?;
        let operation = Operation::parse(&boundary::bounded_string(
            operation,
            16,
            "Keyboard operation",
        )?)
        .map_err(binding_error)?;
        let value = input(value, operation.maximum())?;
        if !delay_ms.is_finite() || delay_ms < 0.0 || delay_ms > f64::from(u32::MAX) {
            return Err(binding_error(HostError::InvalidArgument(
                "Keyboard delay must be finite and between 0 and 4294967295 ms".into(),
            )));
        }
        let delay = Duration::from_secs_f64(delay_ms / 1000.0);
        let host = Arc::clone(&self.host);
        let start = host.admit_input_start().map_err(binding_error)?;
        boundary::spawn_published(
            env,
            execution,
            self.publications.clone(),
            start.run(async move {
                match operation {
                    Operation::Down => host.keyboard_down(&page, &value).await,
                    Operation::Up => host.keyboard_up(&page, &value).await,
                    Operation::Press => host.keyboard_press(&page, &value, delay).await,
                    Operation::InsertText => host.keyboard_insert_text(&page, &value).await,
                    Operation::Type => host.keyboard_type(&page, &value, delay).await,
                }
            }),
        )
    }
}
