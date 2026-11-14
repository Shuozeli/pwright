//! Thin Node ownership for Rust-published Page dialogs.

use crate::{NativeSession, binding_error, boundary};
use napi::{Env, JsString, Unknown, bindgen_prelude::PromiseRaw};
use napi_derive::napi;
use pwright_node_host::{ffi_budget::FfiLane, page_dialogs::DialogAction};
use std::sync::Arc;

#[napi]
pub struct NativeDialog {
    action: Arc<DialogAction>,
}

impl NativeDialog {
    pub(crate) fn new(action: Arc<DialogAction>) -> Self {
        Self { action }
    }
}

#[napi]
impl NativeSession {
    #[napi]
    pub fn handle_dialog<'env>(
        &self,
        env: &'env Env,
        dialog: &NativeDialog,
        accept: bool,
        prompt_text: Option<JsString<'env>>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let prompt_text = prompt_text
            .map(|value| boundary::bounded_string(value, 16 * 1024, "dialog prompt text"))
            .transpose()?;
        let action = Arc::clone(&dialog.action);
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            action.handle(accept, prompt_text.as_deref()).await
        })
    }
}
