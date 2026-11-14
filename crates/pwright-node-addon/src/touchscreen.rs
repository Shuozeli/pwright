//! Fixed-size foreign conversion for the optional Touchscreen input leaf.

use crate::{NativeSession, binding_error, boundary};
use napi::{Env, JsString, Unknown, bindgen_prelude::PromiseRaw};
use napi_derive::napi;
use pwright_node_host::{
    HostError,
    ffi_budget::FfiLane,
    touchscreen::{TapParams, TouchscreenCoordinate},
};
use std::sync::Arc;

#[napi]
impl NativeSession {
    #[napi(js_name = "touchscreenTap")]
    pub fn touchscreen_tap<'env>(
        &self,
        env: &'env Env,
        page: JsString<'env>,
        x: f64,
        y: f64,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        // Admission precedes Page ID allocation and typed coordinate conversion.
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let page = boundary::bounded_string(page, 20, "page ID")?;
        let params = TapParams::new(
            TouchscreenCoordinate::try_from(x)
                .map_err(|error| binding_error(HostError::InvalidArgument(error.to_string())))?,
            TouchscreenCoordinate::try_from(y)
                .map_err(|error| binding_error(HostError::InvalidArgument(error.to_string())))?,
        );
        let host = Arc::clone(&self.host);
        let start = host.admit_input_start().map_err(binding_error)?;
        boundary::spawn_published(
            env,
            execution,
            self.publications.clone(),
            start.run(async move { host.touchscreen_tap(&page, params).await }),
        )
    }
}
