//! Language values only. Dispatch, matching, parsing and response ownership
//! belong to the Rust host; this callback copies under an admitted FFI ticket.
use crate::{NativeSession, NavigationResponse, binding_error, boundary};
use napi::{
    Env, JsString, Unknown,
    bindgen_prelude::{Array, PromiseRaw, ToNapiValue},
    sys,
};
use napi_derive::napi;
use pwright_cap_navigation::v1_2::{GotoParams, WaitUntil};
use pwright_node_host::{FrameGotoOutput, HostError, ffi_budget::FfiLane};

// <=32 KiB native inputs, <=64 KiB response strings copied/lowercased, header
// map storage and conversion overlap (including UTF-16 foreign strings).
// The Rust response arena/typed owner and boxed async body have separate leases.
const FFI_ALLOWANCE: u64 = 512 * 1024;

#[napi(object, object_from_js = false)]
pub struct FrameNavigationResult {
    pub frame_id: String,
    pub response: Option<NavigationResponse>,
}
struct NavigationOutput(FrameGotoOutput);
impl ToNapiValue for NavigationOutput {
    unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> napi::Result<sys::napi_value> {
        let result = value.0.value().map_err(binding_error)?;
        let result = FrameNavigationResult {
            frame_id: result.frame_id.to_string(),
            response: result.response.as_ref().map(|response| NavigationResponse {
                url: response.url.clone(),
                status: response.status,
                status_text: response.status_text.clone(),
                ok: (200..=299).contains(&response.status),
                headers: response
                    .headers
                    .iter()
                    .map(|header| (header.name.to_ascii_lowercase(), header.value.clone()))
                    .collect(),
            }),
        };
        // SAFETY: invoked in napi's live settlement callback. Both the FFI
        // ticket and value.0's independent host lease cover this complete copy.
        let converted = unsafe { FrameNavigationResult::to_napi_value(env, result) };
        drop(value);
        converted
    }
}

#[napi]
impl NativeSession {
    #[napi]
    pub fn frame_goto<'env>(
        &self,
        env: &'env Env,
        target: Array<'env>,
        url: JsString<'env>,
        wait_until: JsString<'env>,
        timeout_ms: u32,
        referrer: Option<JsString<'env>>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        if target.len() != 2 {
            return Err(binding_error(HostError::InvalidArgument(
                "Frame navigation target must contain Page and Frame IDs".into(),
            )));
        }
        let page_id = target
            .get::<JsString<'_>>(0)?
            .ok_or_else(|| binding_error(HostError::InvalidArgument("missing Page ID".into())))?;
        let frame_id = target
            .get::<JsString<'_>>(1)?
            .ok_or_else(|| binding_error(HostError::InvalidArgument("missing Frame ID".into())))?;
        let page_id = boundary::bounded_string(page_id, 20, "page ID")?;
        let frame_id = boundary::bounded_string(frame_id, 20, "frame ID")?;
        let url = boundary::bounded_string(url, 16 * 1024, "navigation URL")?;
        let wait_until = boundary::bounded_string(wait_until, 32, "waitUntil")?;
        let wait_until = match wait_until.as_str() {
            "commit" => WaitUntil::Commit,
            "domcontentloaded" => WaitUntil::DomContentLoaded,
            "load" => WaitUntil::Load,
            "networkidle" => WaitUntil::NetworkIdle,
            _ => {
                return Err(binding_error(HostError::InvalidArgument(
                    "waitUntil must be commit, domcontentloaded, load, or networkidle".into(),
                )));
            }
        };
        let referrer = referrer
            .map(|s| boundary::bounded_string(s, 16 * 1024, "referer"))
            .transpose()?;
        let host = self.host.clone();
        let publications = self.publications.clone();
        let params = GotoParams {
            url,
            referrer,
            wait_until,
        };
        #[cfg(feature = "extra-headers")]
        let pending = host.start_frame_goto(page_id, frame_id, params, timeout_ms);
        #[cfg(not(feature = "extra-headers"))]
        let pending = async move {
            host.frame_goto(&page_id, &frame_id, params, timeout_ms)
                .await
        };
        boundary::spawn_published(env, execution, publications, async move {
            pending.await.map(NavigationOutput)
        })
    }
}
