use napi::JsString;
use napi_derive::napi;

use crate::{NativeSession, binding_error, boundary};

#[napi]
impl NativeSession {
    #[napi]
    pub fn set_context_default_timeout(
        &self,
        context_id: JsString<'_>,
        navigation: bool,
        timeout_ms: Option<u32>,
    ) -> napi::Result<()> {
        let context_id = boundary::bounded_string(context_id, 20, "Context ID")?;
        self.host
            .set_context_default_timeout_override(&context_id, navigation, timeout_ms)
            .map_err(binding_error)
    }

    #[napi]
    pub fn set_page_default_timeout(
        &self,
        page_id: JsString<'_>,
        navigation: bool,
        timeout_ms: Option<u32>,
    ) -> napi::Result<()> {
        let page_id = boundary::bounded_string(page_id, 20, "Page ID")?;
        self.host
            .set_page_default_timeout_override(&page_id, navigation, timeout_ms)
            .map_err(binding_error)
    }

    #[napi]
    pub fn page_default_timeout(
        &self,
        page_id: JsString<'_>,
        navigation: bool,
    ) -> napi::Result<u32> {
        let page_id = boundary::bounded_string(page_id, 20, "Page ID")?;
        self.host
            .page_default_timeout(&page_id, navigation)
            .map_err(binding_error)
    }
}
