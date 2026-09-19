use crate::{NativeSession, NavigationResponse, NavigationResult, binding_error, boundary};
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
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            let result = host.page_reload(&page_id, &wait_until, timeout_ms).await?;
            Ok(NavigationResult {
                url: result.url,
                response: result.response.map(|response| NavigationResponse {
                    url: response.url,
                    status: response.status,
                    status_text: response.status_text,
                    ok: response.ok,
                    headers: response.headers.into_iter().collect(),
                }),
            })
        })
    }
}
