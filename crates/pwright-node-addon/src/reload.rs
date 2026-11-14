use crate::{HostNavigationOutput, NativeSession, binding_error, boundary};
use napi::{Env, JsString, Unknown, bindgen_prelude::PromiseRaw};
use napi_derive::napi;
use pwright_node_host::ffi_budget::FfiLane;
use std::sync::Arc;

#[napi]
impl NativeSession {
    #[napi]
    pub fn page_reload<'env>(
        &self,
        env: &'env Env,
        page_id: JsString<'env>,
        wait_until: JsString<'env>,
        timeout_ms: u32,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::NAVIGATION_ALLOWANCE)
            .map_err(binding_error)?;
        let page_id = boundary::bounded_string(page_id, 20, "page ID")?;
        let wait_until = boundary::bounded_string(wait_until, 32, "waitUntil")?;
        let host = Arc::clone(&self.host);
        boundary::spawn_published_sized(
            env,
            execution,
            self.publications.clone(),
            async move {
                host.page_reload(&page_id, &wait_until, timeout_ms)
                    .await
                    .map(HostNavigationOutput)
            },
            HostNavigationOutput::response_request_foreign_copy_bytes,
        )
    }
}
