//! Node-owned Buffer conversion; capture and byte ownership remain in Rust.
use napi::{
    Env, JsString, Unknown,
    bindgen_prelude::{BufferSlice, PromiseRaw, ToNapiValue},
    sys,
};
use napi_derive::napi;
use pwright_node_host::{
    HostError,
    ffi_budget::FfiLane,
    screenshot::{
        self, ScreenshotClip, ScreenshotFormat, ScreenshotOptions,
        ScreenshotOutput as HostScreenshotOutput, ScreenshotScale,
    },
};

use crate::{NativeSession, binding_error, boundary};

#[napi(object)]
pub struct NativeScreenshotClip {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[napi(object)]
pub struct NativeScreenshotOptions {
    pub format: u32,
    pub quality: Option<u32>,
    pub full_page: bool,
    pub scale: u32,
    pub clip: Option<NativeScreenshotClip>,
    pub timeout_ms: u32,
}

struct ScreenshotOutput(HostScreenshotOutput);

impl ToNapiValue for ScreenshotOutput {
    unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> napi::Result<sys::napi_value> {
        // This callback's FFI settlement ticket owns the result-copy allowance.
        // Copy directly into Node memory: no external pointer or Rust finalizer
        // survives publication, and JS mutation cannot alias native bytes.
        let scope = Env::from_raw(env);
        let buffer = BufferSlice::copy_from(&scope, value.0.as_bytes())?;
        // SAFETY: the buffer was created in this live callback's environment.
        unsafe { BufferSlice::to_napi_value(env, buffer) }
    }
}

#[napi]
impl NativeSession {
    #[napi]
    pub fn page_screenshot<'env>(
        &self,
        env: &'env Env,
        page_id: JsString<'env>,
        options: NativeScreenshotOptions,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, screenshot::FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let page_id = boundary::bounded_string(page_id, 20, "Page ID")?;
        let format = match options.format {
            0 => ScreenshotFormat::Png,
            1 => ScreenshotFormat::Jpeg,
            _ => {
                return Err(binding_error(HostError::InvalidArgument(
                    "screenshot format must be png or jpeg".into(),
                )));
            }
        };
        let scale = match options.scale {
            0 => ScreenshotScale::Device,
            1 => ScreenshotScale::Css,
            _ => {
                return Err(binding_error(HostError::InvalidArgument(
                    "screenshot scale must be device or css".into(),
                )));
            }
        };
        let quality = options
            .quality
            .map(|value| {
                u8::try_from(value).map_err(|_| {
                    binding_error(HostError::InvalidArgument(
                        "screenshot quality exceeds 100".into(),
                    ))
                })
            })
            .transpose()?;
        let params = ScreenshotOptions {
            format: Some(format),
            quality,
            full_page: Some(options.full_page),
            scale: Some(scale),
            clip: options.clip.map(|clip| ScreenshotClip {
                x: clip.x,
                y: clip.y,
                width: clip.width,
                height: clip.height,
            }),
        };
        let host = self.host.clone();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.page_screenshot(&page_id, params, options.timeout_ms)
                .await
                .map(ScreenshotOutput)
        })
    }
}
