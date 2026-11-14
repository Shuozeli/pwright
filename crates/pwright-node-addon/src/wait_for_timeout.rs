use napi::{Env, Unknown, bindgen_prelude::PromiseRaw};
use napi_derive::napi;

use crate::{NativeSession, binding_error, boundary};
use pwright_node_host::ffi_budget::FfiLane;

#[napi]
impl NativeSession {
    #[napi]
    pub fn wait_for_timeout<'env>(
        &self,
        env: &'env Env,
        timeout_ms: u32,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let host = self.host.clone();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.wait_for_timeout(timeout_ms).await
        })
    }
}
