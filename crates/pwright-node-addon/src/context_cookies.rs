//! Optional Node conversion port for Rust-owned BrowserContext cookies.
//!
//! Read borrowed JS fields only after the FFI command is admitted. This port
//! limits both the count and the UTF-8 copies before scheduling the host call;
//! cookie validation and Chromium Storage commands remain in the safe host.

use napi::{
    Env, JsString,
    bindgen_prelude::{Array, JsObjectValue, Object, PromiseRaw},
};
use napi_derive::napi;
use pwright_node_host::{
    HostError,
    context_cookies::{
        BrowserContextCookieInput, FFI_ALLOWANCE, MAX_COOKIE_BATCH_BYTES, MAX_COOKIE_FIELD_BYTES,
        MAX_COOKIE_URL_BYTES, MAX_COOKIES,
    },
    ffi_budget::FfiLane,
};

use crate::{NativeSession, binding_error, boundary};

fn cookie_string(
    value: JsString<'_>,
    maximum: usize,
    field: &str,
    remaining: &mut usize,
) -> napi::Result<String> {
    let bytes = value.utf8_len()?;
    if bytes > maximum {
        return Err(binding_error(HostError::ResourceExhausted(format!(
            "{field} exceeds {maximum} UTF-8 bytes"
        ))));
    }
    *remaining = remaining.checked_sub(bytes).ok_or_else(|| {
        binding_error(HostError::ResourceExhausted(format!(
            "cookie input batch exceeds {MAX_COOKIE_BATCH_BYTES} bytes"
        )))
    })?;
    value.into_utf8()?.into_owned()
}

fn optional_cookie_string(
    value: Option<JsString<'_>>,
    maximum: usize,
    field: &str,
    remaining: &mut usize,
) -> napi::Result<Option<String>> {
    value
        .map(|value| cookie_string(value, maximum, field, remaining))
        .transpose()
}

fn read_cookie(
    cookie: Object<'_>,
    remaining: &mut usize,
) -> napi::Result<BrowserContextCookieInput> {
    Ok(BrowserContextCookieInput {
        name: cookie_string(
            cookie.get_named_property("name")?,
            MAX_COOKIE_FIELD_BYTES,
            "cookie name",
            remaining,
        )?,
        value: cookie_string(
            cookie.get_named_property("value")?,
            MAX_COOKIE_FIELD_BYTES,
            "cookie value",
            remaining,
        )?,
        url: optional_cookie_string(
            cookie.get_named_property("url")?,
            MAX_COOKIE_URL_BYTES,
            "cookie URL",
            remaining,
        )?,
        domain: optional_cookie_string(
            cookie.get_named_property("domain")?,
            MAX_COOKIE_FIELD_BYTES,
            "cookie domain",
            remaining,
        )?,
        path: optional_cookie_string(
            cookie.get_named_property("path")?,
            MAX_COOKIE_FIELD_BYTES,
            "cookie path",
            remaining,
        )?,
        expires: cookie.get_named_property("expires")?,
        http_only: cookie.get_named_property("httpOnly")?,
        secure: cookie.get_named_property("secure")?,
        same_site: optional_cookie_string(
            cookie.get_named_property("sameSite")?,
            MAX_COOKIE_FIELD_BYTES,
            "cookie sameSite",
            remaining,
        )?,
        partition_key: optional_cookie_string(
            cookie.get_named_property("partitionKey")?,
            MAX_COOKIE_URL_BYTES,
            "cookie partitionKey",
            remaining,
        )?,
        partition_has_cross_site_ancestor: None,
    })
}

fn read_cookies(cookies: Array<'_>) -> napi::Result<Vec<BrowserContextCookieInput>> {
    if cookies.len() > MAX_COOKIES as u32 {
        return Err(binding_error(HostError::ResourceExhausted(format!(
            "cookie input count exceeds {MAX_COOKIES}"
        ))));
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(cookies.len() as usize)
        .map_err(|_| {
            binding_error(HostError::ResourceExhausted(
                "cookie input allocation failed".into(),
            ))
        })?;
    let mut remaining = MAX_COOKIE_BATCH_BYTES;
    for index in 0..cookies.len() {
        let cookie = cookies
            .get::<Object<'_>>(index)?
            .ok_or_else(|| binding_error(HostError::InvalidArgument("missing cookie".into())))?;
        output.push(read_cookie(cookie, &mut remaining)?);
    }
    Ok(output)
}

#[napi]
impl NativeSession {
    #[napi]
    pub fn context_add_cookies<'env>(
        &self,
        env: &'env Env,
        context_id: JsString<'env>,
        cookies: Array<'env>,
    ) -> napi::Result<PromiseRaw<'env, napi::Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let context_id = boundary::bounded_string(context_id, 20, "BrowserContext ID")?;
        let cookies = read_cookies(cookies)?;
        let host = self.host.clone();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.context_add_cookies(&context_id, cookies).await
        })
    }
}
