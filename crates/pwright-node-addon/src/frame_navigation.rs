//! Language values only. Dispatch, matching, parsing and response ownership
//! belong to the Rust host; this callback copies under an admitted FFI ticket.
#[cfg(feature = "response-request")]
use crate::NativeNavigationRequest;
use crate::{NativeSession, NavigationResponse, binding_error, boundary};
#[cfg(feature = "response-finished")]
use napi::bindgen_prelude::External;
use napi::{
    Env, JsString, Unknown,
    bindgen_prelude::{Array, PromiseRaw, ToNapiValue},
    sys,
};
use napi_derive::napi;
use pwright_cap_navigation::{
    WaitUntil as CoreWaitUntil,
    v1_2::{GotoParams, WaitUntil},
};
use pwright_node_host::{FrameGotoOutput, HostError, ffi_budget::FfiLane};
#[cfg(feature = "response-finished")]
use std::sync::Arc;

// <=32 KiB native inputs, <=64 KiB response strings copied/lowercased, header
// map storage and conversion overlap (including UTF-16 foreign strings).
// The Rust response arena/typed owner and boxed async body have separate leases.
const FFI_ALLOWANCE: u64 = 512 * 1024;

fn parse_wait_until(value: &str) -> napi::Result<WaitUntil> {
    match value {
        "commit" => Ok(WaitUntil::Commit),
        "domcontentloaded" => Ok(WaitUntil::DomContentLoaded),
        "load" => Ok(WaitUntil::Load),
        "networkidle" | "networkidle0" => Ok(WaitUntil::NetworkIdle),
        _ => Err(binding_error(HostError::InvalidArgument(
            "waitUntil: expected one of (load|domcontentloaded|networkidle|commit)".into(),
        ))),
    }
}

fn parse_core_wait_until(value: &str) -> napi::Result<CoreWaitUntil> {
    match value {
        "commit" => Ok(CoreWaitUntil::Commit),
        "domcontentloaded" => Ok(CoreWaitUntil::DomContentLoaded),
        "load" => Ok(CoreWaitUntil::Load),
        "networkidle" | "networkidle0" => Ok(CoreWaitUntil::NetworkIdle),
        _ => Err(binding_error(HostError::InvalidArgument(
            "waitUntil: expected one of (load|domcontentloaded|networkidle|commit)".into(),
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_goto_wait_until_accepts_networkidle0_alias() {
        // Arrange
        let alias = "networkidle0";

        // Act
        let parsed = parse_wait_until(alias);

        // Assert
        assert_eq!(parsed.unwrap(), WaitUntil::NetworkIdle);
    }

    #[test]
    fn frame_goto_wait_until_rejects_networkidle2_with_pinned_enum_error() {
        // Arrange
        let invalid = "networkidle2";

        // Act
        let parsed = parse_wait_until(invalid);

        // Assert
        assert_eq!(
            parsed.unwrap_err().reason,
            "PWRIGHT_INVALID_ARGUMENT: waitUntil: expected one of (load|domcontentloaded|networkidle|commit)"
        );
    }

    #[test]
    fn frame_load_state_parser_accepts_networkidle0_alias() {
        // Arrange
        let alias = "networkidle0";

        // Act
        let parsed = parse_core_wait_until(alias);

        // Assert
        assert_eq!(parsed.unwrap(), CoreWaitUntil::NetworkIdle);
    }
}

#[napi(object, object_from_js = false)]
pub struct FrameNavigationResult {
    pub frame_id: String,
    pub response: Option<NavigationResponse>,
}
struct NavigationOutput(FrameGotoOutput);
impl NavigationOutput {
    fn response_request_foreign_copy_bytes(&self) -> u64 {
        #[cfg(feature = "response-request")]
        {
            self.0.response_request_foreign_copy_bytes()
        }
        #[cfg(not(feature = "response-request"))]
        {
            0
        }
    }
}
impl ToNapiValue for NavigationOutput {
    unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> napi::Result<sys::napi_value> {
        let result = value.0.value().map_err(binding_error)?;
        let response_request_id = value.0.response_request_id().map(str::to_owned);
        #[cfg(feature = "response-request")]
        let response_request = value
            .0
            .response_request()
            .map(|request| NativeNavigationRequest::from(request.request()));
        let result = FrameNavigationResult {
            frame_id: result.frame_id.to_string(),
            response: result.response.as_ref().map(|response| NavigationResponse {
                request_id: response_request_id,
                #[cfg(feature = "response-request")]
                request: response_request,
                #[cfg(not(feature = "response-request"))]
                request: None,
                url: response.url.clone(),
                status: response.status,
                status_text: response.status_text.clone(),
                ok: (200..=299).contains(&response.status),
                headers: response
                    .headers
                    .iter()
                    .map(|header| (header.name.to_ascii_lowercase(), header.value.clone()))
                    .collect(),
                #[cfg(feature = "response-finished")]
                finished_owner: value
                    .0
                    .response_finished_owner()
                    .map(|owner| External::new(Arc::clone(owner))),
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
    pub fn frame_wait_for_load_state<'env>(
        &self,
        env: &'env Env,
        page_id: JsString<'env>,
        frame_id: JsString<'env>,
        state_value: JsString<'env>,
        timeout_ms: u32,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let page_id = boundary::bounded_string(page_id, 20, "page ID")?;
        let frame_id = boundary::bounded_string(frame_id, 20, "frame ID")?;
        let state_value = boundary::bounded_string(state_value, 32, "state")?;
        let state = parse_core_wait_until(&state_value)?;
        let pending = self
            .host
            .clone()
            .start_wait_for_frame_load_state(page_id, frame_id, state, timeout_ms);
        boundary::spawn_published(env, execution, self.publications.clone(), pending)
    }

    #[napi]
    pub fn frame_wait_for_navigation<'env>(
        &self,
        env: &'env Env,
        page_id: JsString<'env>,
        frame_id: JsString<'env>,
        wait_until_value: JsString<'env>,
        timeout_ms: u32,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let page_id = boundary::bounded_string(page_id, 20, "page ID")?;
        let frame_id = boundary::bounded_string(frame_id, 20, "frame ID")?;
        let wait_until_value = boundary::bounded_string(wait_until_value, 32, "waitUntil")?;
        let wait_until = parse_core_wait_until(&wait_until_value)?;
        let host = self.host.clone();
        let pending =
            host.start_wait_for_frame_navigation(page_id, frame_id, wait_until, timeout_ms);
        boundary::spawn_published_sized(
            env,
            execution,
            self.publications.clone(),
            async move { pending.await.map(NavigationOutput) },
            NavigationOutput::response_request_foreign_copy_bytes,
        )
    }

    #[napi]
    pub fn frame_goto<'env>(
        &self,
        env: &'env Env,
        target: Array<'env>,
        url: JsString<'env>,
        wait_until_value: JsString<'env>,
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
        let wait_until_value = boundary::bounded_string(wait_until_value, 32, "waitUntil")?;
        let wait_until = parse_wait_until(&wait_until_value)?;
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
        boundary::spawn_published_sized(
            env,
            execution,
            publications,
            async move { pending.await.map(NavigationOutput) },
            NavigationOutput::response_request_foreign_copy_bytes,
        )
    }
}
