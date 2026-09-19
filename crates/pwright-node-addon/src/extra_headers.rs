//! Node language-value port only. No header merge, routing or browser policy.
use crate::{NativeSession, binding_error, boundary};
use napi::{
    Env, JsString, JsValue, Unknown, ValueType,
    bindgen_prelude::{FromNapiValue, Function, JsObjectValue, Object, PromiseRaw},
    sys,
};
use napi_derive::napi;
use pwright_node_host::{extra_headers::HeaderRef, ffi_budget::FfiLane};
use std::ptr;

const MAX_HEADERS: u32 = 256;
const MAX_BYTES: usize = 64 * 1024;
// Bounded Rust inputs, lexical handle arrays, UTF-8/UTF-16 conversion overlap,
// async borrowed projections and settlement errors. V8 enumeration's temporary
// key arrays and arbitrary user getter allocations are external VM costs.
const FFI_ALLOWANCE: u64 = 512 * 1024;

fn status(code: sys::napi_status) -> napi::Result<()> {
    if code == sys::Status::napi_ok {
        Ok(())
    } else {
        Err(napi::Error::new(
            napi::Status::from(code),
            "header language operation failed",
        ))
    }
}
fn error(message: impl Into<String>) -> napi::Error {
    napi::Error::new(napi::Status::GenericFailure, message.into())
}
fn object<'env>(env: &'env Env, value: Unknown<'env>) -> napi::Result<Object<'env>> {
    let mut object = ptr::null_mut();
    // SAFETY: callback-scoped handles; ToObject preserves null/undefined throws.
    status(unsafe { sys::napi_coerce_to_object(env.raw(), value.raw(), &mut object) })?;
    Ok(Object::from_raw(env.raw(), object))
}
fn keys<'env>(env: &'env Env, object: &Object<'env>, own: bool) -> napi::Result<Object<'env>> {
    let mut keys = ptr::null_mut();
    // SAFETY: live object/env, enumerable string keys in JS property order.
    // The VM materializes keys before we can inspect length. No claim of a hard
    // V8 heap bound is made; Rust copies are admitted and bounded separately.
    status(unsafe {
        sys::napi_get_all_property_names(
            env.raw(),
            object.raw(),
            if own {
                sys::KeyCollectionMode::own_only
            } else {
                sys::KeyCollectionMode::include_prototypes
            },
            sys::KeyFilter::enumerable | sys::KeyFilter::skip_symbols,
            sys::KeyConversion::numbers_to_strings,
            &mut keys,
        )
    })?;
    let keys = Object::from_raw(env.raw(), keys);
    if keys.get_array_length()? > MAX_HEADERS {
        return Err(error("extra header enumerable key count exceeds 256"));
    }
    Ok(keys)
}
fn string_box(env: &Env, value: Unknown<'_>) -> napi::Result<bool> {
    let constructor: Unknown<'_> = env.get_global()?.get_named_property("String")?;
    let mut result = false;
    // SAFETY: all handles belong to the live callback; instanceof may throw.
    status(unsafe {
        sys::napi_instanceof(env.raw(), value.raw(), constructor.raw(), &mut result)
    })?;
    Ok(result)
}
fn type_name(kind: ValueType) -> &'static str {
    match kind {
        ValueType::Undefined => "undefined",
        ValueType::Boolean => "boolean",
        ValueType::Number => "number",
        ValueType::String => "string",
        ValueType::Symbol => "symbol",
        ValueType::Object | ValueType::Null | ValueType::External => "object",
        ValueType::Function => "function",
        ValueType::BigInt => "bigint",
        ValueType::Unknown => "unknown",
    }
}
fn text(value: JsString<'_>, remaining: &mut usize) -> napi::Result<String> {
    let bytes = value.utf8_len()?;
    if bytes > *remaining {
        return Err(error("extra header strings exceed 65536 UTF-8 bytes"));
    }
    *remaining -= bytes;
    value.into_utf8()?.into_owned()
}
fn unbox_raw<'env>(env: &'env Env, value: Unknown<'env>) -> napi::Result<Unknown<'env>> {
    let receiver = object(env, value)?;
    let method: Unknown<'_> = receiver.get_named_property("valueOf")?;
    let mut result = ptr::null_mut();
    // SAFETY: scoped receiver/function, no arguments. Match the pinned validator's
    // valueOf call, not ToString (which can invoke a different user override).
    status(unsafe {
        sys::napi_call_function(
            env.raw(),
            receiver.raw(),
            method.raw(),
            0,
            ptr::null(),
            &mut result,
        )
    })?;
    unsafe { Unknown::from_napi_value(env.raw(), result) }
}

fn unbox<'env>(
    env: &'env Env,
    value: Unknown<'env>,
    api: &str,
    channel_error: &Function<'env>,
) -> napi::Result<Unknown<'env>> {
    match unbox_raw(env, value) {
        Err(error) if error.status == napi::Status::PendingException => {
            let mut exception = ptr::null_mut();
            // SAFETY: recover only this valueOf exception and synchronously
            // invoke the scoped JS language-error port. No handle escapes.
            status(unsafe { sys::napi_get_and_clear_last_exception(env.raw(), &mut exception) })?;
            let api = env.create_string(api)?;
            let args = [exception, api.raw()];
            let mut result = ptr::null_mut();
            status(unsafe {
                sys::napi_call_function(
                    env.raw(),
                    value.raw(),
                    channel_error.raw(),
                    args.len(),
                    args.as_ptr(),
                    &mut result,
                )
            })?;
            Err(napi::Error::from_reason(
                "header channel-error port must throw",
            ))
        }
        result => result,
    }
}

fn read<'env>(
    env: &'env Env,
    value: Unknown<'env>,
    api: &str,
    parameter: &str,
    channel_error: &Function<'env>,
) -> napi::Result<Vec<(String, String)>> {
    let value = object(env, value)?;
    // Pinned validateHeaders reads each own value before the second enumeration.
    let own = keys(env, &value, true)?;
    for index in 0..own.get_array_length()? {
        let key: JsString<'_> = own.get_element(index)?;
        let value: Unknown<'_> = value.get_property(key)?;
        let kind = value.get_type()?;
        if kind != ValueType::Undefined && kind != ValueType::String && !string_box(env, value)? {
            let name = boundary::bounded_string(key, MAX_BYTES, "header name")?;
            return Err(error(format!(
                "Expected value of header \"{name}\" to be String, but \"{}\" is found.",
                type_name(kind)
            )));
        }
    }
    let all = keys(env, &value, false)?;
    // The client collects every value before channel validation invokes boxed
    // valueOf. Keep only callback-scoped handles here: an earlier box may mutate
    // a later property, but it must not change that property's captured value.
    let mut entries = Vec::with_capacity(all.get_array_length()? as usize);
    for index in 0..all.get_array_length()? {
        let key: JsString<'_> = all.get_element(index)?;
        let entry: Unknown<'_> = value.get_property(key)?;
        if entry.get_type()? == ValueType::Undefined {
            continue;
        }
        entries.push((key, entry));
    }
    let mut headers = Vec::with_capacity(entries.len());
    let mut bytes = MAX_BYTES;
    for (key, mut entry) in entries {
        if entry.get_type()? != ValueType::String && string_box(env, entry)? {
            entry = unbox(env, entry, api, channel_error)?;
        }
        let kind = entry.get_type()?;
        if kind != ValueType::String {
            return Err(error(format!(
                "{api}: {parameter}[{}].value: expected string, got {}",
                headers.len(),
                type_name(kind)
            )));
        }
        // SAFETY: the type was checked above, and the lexical handle cannot
        // escape into the future; only the admitted owned UTF-8 copy does.
        let entry = unsafe { JsString::from_napi_value(env.raw(), entry.raw())? };
        headers.push((text(key, &mut bytes)?, text(entry, &mut bytes)?));
    }
    Ok(headers)
}

impl NativeSession {
    fn set_headers<'env>(
        &self,
        env: &'env Env,
        id: JsString<'env>,
        headers: Unknown<'env>,
        channel_error: Function<'env>,
        context: bool,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let id = boundary::bounded_string(id, 20, "header target ID")?;
        let api = if context {
            "browserContext.setExtraHTTPHeaders"
        } else {
            "page.setExtraHTTPHeaders"
        };
        let headers = read(env, headers, api, "headers", &channel_error)?;
        let host = self.host.clone();
        let input: Vec<_> = headers
            .iter()
            .map(|(name, value)| HeaderRef { name, value })
            .collect();
        use pwright_node_host::extra_headers::NativeHeaderTarget;
        let target = if context {
            NativeHeaderTarget::Context(&id)
        } else {
            NativeHeaderTarget::Page(&id)
        };
        let pending = host.start_set_extra_http_headers_native(target, &input);
        boundary::spawn_published(env, execution, self.publications.clone(), pending)
    }
}
#[napi]
impl NativeSession {
    #[napi]
    pub fn new_context_with_extra_http_headers<'env>(
        &self,
        env: &'env Env,
        headers: Unknown<'env>,
        channel_error: Function<'env>,
        base_url: Option<JsString<'env>>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE + boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let headers = read(
            env,
            headers,
            "browser.newContext",
            "extraHTTPHeaders",
            &channel_error,
        )?;
        let base_url = crate::context_settings::read_base_url(env, base_url)?;
        let host = self.host.clone();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            let input: Vec<_> = headers
                .iter()
                .map(|(name, value)| HeaderRef { name, value })
                .collect();
            crate::ContextDescriptor::try_from(
                host.new_context_with_extra_http_headers_native(base_url.as_deref(), &input)
                    .await?,
            )
        })
    }

    #[napi]
    pub fn set_context_extra_http_headers<'env>(
        &self,
        env: &'env Env,
        id: JsString<'env>,
        headers: Unknown<'env>,
        channel_error: Function<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        self.set_headers(env, id, headers, channel_error, true)
    }
    #[napi]
    pub fn set_page_extra_http_headers<'env>(
        &self,
        env: &'env Env,
        id: JsString<'env>,
        headers: Unknown<'env>,
        channel_error: Function<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        self.set_headers(env, id, headers, channel_error, false)
    }
}
