//! Bounded value conversion on the Node thread; no CDP or browser semantics.

mod binary;
mod input;
mod output;

use napi::{
    Env, JsString, Unknown,
    bindgen_prelude::{Function, JsObjectValue, Object, ToNapiValue},
    sys,
};
use pwright_node_host::HostError;
use pwright_node_host::evaluation::{SOURCE_BYTES, TypedArrayKind};

use crate::binding_error;
pub use input::{PreparedInput, read};
pub use output::EvaluationOutput;

pub(crate) fn status(code: sys::napi_status) -> napi::Result<()> {
    if code == sys::Status::napi_ok {
        Ok(())
    } else {
        Err(napi::Error::new(
            napi::Status::from(code),
            "Node value conversion failed",
        ))
    }
}

fn invalid(message: impl Into<String>) -> napi::Error {
    binding_error(HostError::InvalidArgument(message.into()))
}

fn unsupported(message: impl Into<String>) -> napi::Error {
    binding_error(HostError::UnsupportedOperation(message.into()))
}

fn global(env: &Env) -> napi::Result<Object<'_>> {
    let mut value = std::ptr::null_mut();
    // SAFETY: env is the active callback environment; the output is a scoped handle.
    status(unsafe { sys::napi_get_global(env.raw(), &mut value) })?;
    Ok(Object::from_raw(env.raw(), value))
}

fn js_value<T: ToNapiValue>(env: &Env, value: T) -> napi::Result<Unknown<'_>> {
    // SAFETY: conversion and its result remain in this active Node callback scope.
    let raw = unsafe { T::to_napi_value(env.raw(), value)? };
    Ok(unsafe { Unknown::from_raw_unchecked(env.raw(), raw) })
}

fn string(value: JsString<'_>, maximum: usize) -> napi::Result<String> {
    if value.utf8_len()? > maximum {
        return Err(invalid("evaluation string-byte limit exceeded"));
    }
    // Validate UTF-16 before copying to Rust UTF-8; never replace an unpaired surrogate silently.
    // napi-rs into_utf16 includes its terminator, which is not part of the JS value.
    let units = value.utf16_len()?;
    let utf16 = value.into_utf16()?;
    String::from_utf16(&utf16.as_slice()[..units])
        .map_err(|_| unsupported("unpaired UTF-16 surrogates in evaluation values"))
}

/// Function-to-source normalization uses Node's parser only for conversion;
/// the resulting function is never executed in Node. Browser evaluation is Rust-owned.
pub fn source(env: &Env, value: JsString<'_>, is_function: bool) -> napi::Result<String> {
    let value = string(value, SOURCE_BYTES)?;
    let trimmed = value.trim();
    if !is_function {
        return Ok(trimmed.to_owned());
    }
    let constructor: Function<String> = global(env)?.get_named_property("Function")?;
    let parse = |source: &str| -> napi::Result<()> {
        constructor.new_instance(format!("({source})")).map(|_| ())
    };
    if parse(trimmed).is_ok() {
        return Ok(trimmed.to_owned());
    }
    // Clear only the parser's caught SyntaxError before trying method shorthand.
    clear_exception(env)?;
    let normalized = if let Some(rest) = trimmed.strip_prefix("async ") {
        format!("async function {rest}")
    } else {
        format!("function {trimmed}")
    };
    if parse(&normalized).is_err() {
        clear_exception(env)?;
        return Err(invalid("Passed function is not well-serializable!"));
    }
    if normalized.len() > SOURCE_BYTES {
        return Err(invalid(
            "normalized evaluation source exceeds its byte limit",
        ));
    }
    Ok(normalized)
}

fn clear_exception(env: &Env) -> napi::Result<()> {
    let mut pending = false;
    // SAFETY: only inspected/cleared on the synchronous Node conversion thread.
    status(unsafe { sys::napi_is_exception_pending(env.raw(), &mut pending) })?;
    if pending {
        let mut exception = std::ptr::null_mut();
        status(unsafe { sys::napi_get_and_clear_last_exception(env.raw(), &mut exception) })?;
    }
    Ok(())
}

fn typed_kind(native: sys::napi_typedarray_type) -> napi::Result<TypedArrayKind> {
    use sys::TypedarrayType as T;
    Ok(match native {
        T::int8_array => TypedArrayKind::Int8,
        T::uint8_array => TypedArrayKind::Uint8,
        T::uint8_clamped_array => TypedArrayKind::Uint8Clamped,
        T::int16_array => TypedArrayKind::Int16,
        T::uint16_array => TypedArrayKind::Uint16,
        T::int32_array => TypedArrayKind::Int32,
        T::uint32_array => TypedArrayKind::Uint32,
        T::float32_array => TypedArrayKind::Float32,
        T::float64_array => TypedArrayKind::Float64,
        T::bigint64_array => TypedArrayKind::Bigint64,
        T::biguint64_array => TypedArrayKind::Biguint64,
        _ => return Err(unsupported("unknown typed array kind")),
    })
}

fn typed_name(kind: TypedArrayKind) -> &'static str {
    match kind {
        TypedArrayKind::Int8 => "Int8Array",
        TypedArrayKind::Uint8 => "Uint8Array",
        TypedArrayKind::Uint8Clamped => "Uint8ClampedArray",
        TypedArrayKind::Int16 => "Int16Array",
        TypedArrayKind::Uint16 => "Uint16Array",
        TypedArrayKind::Int32 => "Int32Array",
        TypedArrayKind::Uint32 => "Uint32Array",
        TypedArrayKind::Float32 => "Float32Array",
        TypedArrayKind::Float64 => "Float64Array",
        TypedArrayKind::Bigint64 => "BigInt64Array",
        TypedArrayKind::Biguint64 => "BigUint64Array",
    }
}

fn native_kind(kind: TypedArrayKind) -> sys::napi_typedarray_type {
    use sys::TypedarrayType as T;
    match kind {
        TypedArrayKind::Int8 => T::int8_array,
        TypedArrayKind::Uint8 => T::uint8_array,
        TypedArrayKind::Uint8Clamped => T::uint8_clamped_array,
        TypedArrayKind::Int16 => T::int16_array,
        TypedArrayKind::Uint16 => T::uint16_array,
        TypedArrayKind::Int32 => T::int32_array,
        TypedArrayKind::Uint32 => T::uint32_array,
        TypedArrayKind::Float32 => T::float32_array,
        TypedArrayKind::Float64 => T::float64_array,
        TypedArrayKind::Bigint64 => T::bigint64_array,
        TypedArrayKind::Biguint64 => T::biguint64_array,
    }
}
