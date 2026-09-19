//! Synchronous language-value operations. No persisted references or transport.
use super::NativeFrameTree;
use crate::context_settings::NativeContextSettings;
use napi::{Env, JsValue, Unknown, ValueType, bindgen_prelude::FromNapiValue, sys};
use std::{ffi::CStr, ptr};

fn status(code: sys::napi_status) -> napi::Result<()> {
    if code == sys::Status::napi_ok {
        Ok(())
    } else {
        Err(napi::Error::new(
            napi::Status::from(code),
            "Frame matcher language operation failed",
        ))
    }
}

fn property<'env>(
    env: &'env Env,
    object: Unknown<'env>,
    key: &CStr,
) -> napi::Result<Unknown<'env>> {
    let mut value = ptr::null_mut();
    // SAFETY: handles belong to this synchronous callback scope.
    unsafe {
        status(sys::napi_get_named_property(
            env.raw(),
            object.raw(),
            key.as_ptr(),
            &mut value,
        ))?;
        Unknown::from_napi_value(env.raw(), value)
    }
}

fn global<'env>(env: &'env Env, key: &CStr) -> napi::Result<Unknown<'env>> {
    let mut value = ptr::null_mut();
    // SAFETY: Node returns the callback environment's global handle.
    let global = unsafe {
        status(sys::napi_get_global(env.raw(), &mut value))?;
        Unknown::from_napi_value(env.raw(), value)?
    };
    property(env, global, key)
}

fn construct<'env>(env: &'env Env, name: &CStr, arg: Unknown<'env>) -> napi::Result<Unknown<'env>> {
    let constructor = global(env, name)?;
    let args = [arg.raw()];
    let mut value = ptr::null_mut();
    // SAFETY: scoped constructor and one stack argument stay live through call.
    unsafe {
        status(sys::napi_new_instance(
            env.raw(),
            constructor.raw(),
            args.len(),
            args.as_ptr(),
            &mut value,
        ))?;
        Unknown::from_napi_value(env.raw(), value)
    }
}

fn call<'env>(
    env: &'env Env,
    receiver: Unknown<'env>,
    function: Unknown<'env>,
    args: &[sys::napi_value],
) -> napi::Result<Unknown<'env>> {
    let mut value = ptr::null_mut();
    // SAFETY: caller supplies only scoped handles and stack argument arrays.
    unsafe {
        status(sys::napi_call_function(
            env.raw(),
            receiver.raw(),
            function.raw(),
            args.len(),
            args.as_ptr(),
            &mut value,
        ))?;
        Unknown::from_napi_value(env.raw(), value)
    }
}

fn instance_of(env: &Env, value: Unknown<'_>, name: &CStr) -> napi::Result<bool> {
    let constructor = global(env, name)?;
    let mut result = false;
    // SAFETY: handles belong to the active callback environment.
    status(unsafe {
        sys::napi_instanceof(env.raw(), value.raw(), constructor.raw(), &mut result)
    })?;
    Ok(result)
}

pub(super) fn truthy(value: Unknown<'_>) -> napi::Result<bool> {
    value.coerce_to_bool()
}

pub(super) fn matches<'env>(
    tree: &NativeFrameTree,
    settings: &NativeContextSettings,
    env: &'env Env,
    mut matcher: Unknown<'env>,
    text: &str,
) -> napi::Result<bool> {
    if matcher.get_type()? == ValueType::Undefined
        || env.strict_equals(matcher, env.create_string("")?)?
    {
        return Ok(true);
    }
    if matcher.get_type()? == ValueType::String || instance_of(env, matcher, c"String")? {
        let base = settings
            .binding
            .snapshot()
            .base_url()
            .map(|base| env.create_string(base))
            .transpose()?;
        let pattern = tree.glob_pattern(env, matcher.coerce_to_string()?, base)?;
        matcher = construct(env, c"RegExp", pattern.into_unknown(env)?)?;
    }
    let is_regex = if instance_of(env, matcher, c"RegExp")? {
        true
    } else {
        let prototype = property(env, global(env, c"Object")?, c"prototype")?;
        let stringify = property(env, prototype, c"toString")?;
        let tag = call(env, matcher, stringify, &[])?;
        env.strict_equals(tag, env.create_string("[object RegExp]")?)?
    };
    let url = env.create_string(text)?.into_unknown(env)?;
    if is_regex {
        let test = property(env, matcher, c"test")?;
        return truthy(call(env, matcher, test, &[url.raw()])?);
    }
    // Only construction is inside the pinned parseURL try/catch. Predicate
    // exceptions (including primitive/null throws) remain pending and unchanged.
    let parsed = match construct(env, c"URL", url) {
        Ok(value) => value,
        Err(error) if error.status == napi::Status::PendingException => {
            let mut exception = ptr::null_mut();
            // SAFETY: clears only the exception from URL construction above.
            status(unsafe { sys::napi_get_and_clear_last_exception(env.raw(), &mut exception) })?;
            return Ok(false);
        }
        Err(error) => return Err(error),
    };
    if matcher.get_type()? != ValueType::Function {
        return Err(napi::Error::from_reason(
            "url parameter should be string, RegExp or function",
        ));
    }
    let receiver = ().into_unknown(env)?;
    truthy(call(env, receiver, matcher, &[parsed.raw()])?)
}

use napi::bindgen_prelude::ToNapiValue;
