//! Node-owned Buffer conversion for the optional Rust response-body leaf.

use napi::{
    Env, JsString, Unknown,
    bindgen_prelude::{BufferSlice, PromiseRaw, ToNapiValue},
    sys,
};
use napi_derive::napi;
use pwright_node_host::{
    ffi_budget::FfiLane,
    response_body::{self, ResponseBodyOutput as HostResponseBodyOutput},
};

use crate::{NativeSession, binding_error, boundary};

struct ResponseBodyOutput(HostResponseBodyOutput);

impl ToNapiValue for ResponseBodyOutput {
    unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> napi::Result<sys::napi_value> {
        // The settlement ticket and Rust output lease cover this complete copy.
        // No external pointer or Rust finalizer survives publication to Node.
        let scope = Env::from_raw(env);
        let buffer = BufferSlice::copy_from(&scope, value.0.as_bytes())?;
        // SAFETY: the buffer was created in this live callback's environment.
        unsafe { BufferSlice::to_napi_value(env, buffer) }
    }
}

#[napi]
impl NativeSession {
    #[napi]
    pub fn frame_response_body<'env>(
        &self,
        env: &'env Env,
        page_id: JsString<'env>,
        frame_id: JsString<'env>,
        request_id: JsString<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, response_body::FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let page_id = boundary::bounded_string(page_id, 20, "Page ID")?;
        let frame_id = boundary::bounded_string(frame_id, 20, "Frame ID")?;
        let request_id = boundary::bounded_string(request_id, 256, "response request ID")?;
        let host = self.host.clone();
        boundary::spawn_published_sized(
            env,
            execution,
            self.publications.clone(),
            async move {
                host.frame_response_body(&page_id, &frame_id, &request_id)
                    .await
                    .map(ResponseBodyOutput)
            },
            |body| body.0.len() as u64,
        )
    }
}
