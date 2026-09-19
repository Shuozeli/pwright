//! Node WHATWG URL values are scoped foreign values, not a JS glob algorithm.
//! All tokenization, restoration, bounds and pattern planning remain Rust-owned.
use napi::{Env, JsString, JsStringUtf8};
use napi_derive::napi;
use pwright_node_host::{
    HostError,
    frame_lookup::{GlobUrlResolution, PreparedGlob, RESOLVED_GLOB_BYTES, URL_WORK_BYTES},
};

use super::NativeFrameTree;
use crate::{binding_error, url_value::resolve};

fn bounded_len(value: JsString<'_>, limit: usize) -> napi::Result<usize> {
    let length = value.utf8_len()?;
    if length > limit {
        return Err(binding_error(HostError::ResourceExhausted(
            "Frame URL text exceeds conversion byte limit".into(),
        )));
    }
    Ok(length)
}

/// Caller admits both the temporary UTF-16 validation and returned UTF-8 copy.
fn copy_text(value: JsString<'_>) -> napi::Result<JsStringUtf8<'_>> {
    let units = value.utf16_len()?;
    {
        let utf16 = value.into_utf16()?;
        if char::decode_utf16(utf16.as_slice()[..units].iter().copied()).any(|c| c.is_err()) {
            return Err(binding_error(HostError::UnsupportedOperation(
                "unpaired UTF-16 surrogates in Frame glob text".into(),
            )));
        }
    }
    value.into_utf8()
}

#[napi]
impl NativeFrameTree {
    /// Internal conversion primitive for the forthcoming public Frame lookup.
    /// Never performs navigation, launches a browser, or retains Node references.
    #[napi]
    pub fn glob_pattern<'env>(
        &self,
        env: &'env Env,
        glob: JsString<'env>,
        base_url: Option<JsString<'env>>,
    ) -> napi::Result<JsString<'env>> {
        let resources = self.binding.tree().lookup_resources();
        let length = bounded_len(glob, RESOLVED_GLOB_BYTES)?;
        if let Some(base) = base_url {
            bounded_len(base, RESOLVED_GLOB_BYTES)?;
        }
        // UTF-16 validation needs <=2*n+2 bytes; UTF-8 needs n+1.
        // They do not overlap. Slack includes wrapper bodies and bounded errors.
        let _input = resources
            .reserve_conversion(2 * length + 1024)
            .map_err(binding_error)?;
        let input = copy_text(glob)?;
        let prepared = PreparedGlob::new(resources, input.as_str()?).map_err(binding_error)?;
        drop(input);
        let values = match prepared.relative_path() {
            Some(relative) => resolve(env, env.create_string(relative)?, base_url)?,
            None => None,
        };
        let plan = if let Some((serialized, origin)) = values {
            let serialized_len = bounded_len(serialized, URL_WORK_BYTES)?;
            let origin_len = bounded_len(origin, URL_WORK_BYTES)?;
            // Includes both UTF-8 outputs, either temporary UTF-16 validation,
            // terminators and wrapper bodies. Reserve before either copy.
            let bytes = serialized_len + origin_len + 2 * serialized_len.max(origin_len) + 1024;
            let _output = resources.reserve_conversion(bytes).map_err(binding_error)?;
            let serialized = copy_text(serialized)?;
            let origin = copy_text(origin)?;
            prepared
                .compile(
                    resources,
                    Some(GlobUrlResolution {
                        serialized: serialized.as_str()?,
                        origin: origin.as_str()?,
                    }),
                )
                .map_err(binding_error)?
        } else {
            prepared.compile(resources, None).map_err(binding_error)?
        };
        // The plan remains leased through the real native-to-JS conversion.
        env.create_string(plan.pattern())
    }
}
