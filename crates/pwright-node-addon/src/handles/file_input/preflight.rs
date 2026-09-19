//! Synchronous pinned client pre-options semantics. No registry, persistent
//! roots, byte copying, filesystem or native receiver ownership lives here.
//!
//! The caller supplies scoped single-language-operation ports and owns actual
//! await boundaries. In particular this result is NOT the async `{ payloads }`
//! carrier: assimilating its metadata would expose a different `then` receiver.

use crate::evaluation_value::status;
use napi::{
    Env, JsValue, Unknown, ValueType,
    bindgen_prelude::{Function, JsObjectValue, Object, ToNapiValue},
    sys,
};
use std::ptr;

const PAYLOAD_SIZE_LIMIT: i32 = 50 * 1024 * 1024;

/// Callback-scoped ports, never Rust-owned persistent Node references.
/// Member-call ports preserve JS boxing, original `this`, non-callable errors
/// and pending thrown values. Operators perform actual JS coercion, not f64
/// projections. The trusted caller supplies these exact primitive operations.
pub(super) struct LanguagePorts<'env> {
    owner_frame: Unknown<'env>,
    page: Unknown<'env>,
    context: Unknown<'env>,
    is_array: Unknown<'env>,
    slice: Unknown<'env>,
    some: Unknown<'env>,
    every: Unknown<'env>,
    reduce: Unknown<'env>,
    buffer: Unknown<'env>,
    byte_length: Unknown<'env>,
    add: Unknown<'env>,
    at_least: Unknown<'env>,
    unary: Unknown<'env>,
    binary: Unknown<'env>,
    error: Unknown<'env>,
}

impl<'env> LanguagePorts<'env> {
    pub(super) fn read(object: Object<'env>) -> napi::Result<Self> {
        Ok(Self {
            owner_frame: object.get_named_property("owner_frame")?,
            page: object.get_named_property("page")?,
            context: object.get_named_property("context")?,
            is_array: object.get_named_property("is_array")?,
            slice: object.get_named_property("slice")?,
            some: object.get_named_property("some")?,
            every: object.get_named_property("every")?,
            reduce: object.get_named_property("reduce")?,
            buffer: object.get_named_property("buffer")?,
            byte_length: object.get_named_property("byte_length")?,
            add: object.get_named_property("add")?,
            at_least: object.get_named_property("at_least")?,
            unary: object.get_named_property("unary")?,
            binary: object.get_named_property("binary")?,
            error: object.get_named_property("error")?,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum InputKind {
    Payloads,
    /// Classification only. Stat/path conversion has not begun.
    Paths,
}

pub(super) struct Converted<'env> {
    pub(super) context: Unknown<'env>,
    pub(super) items: Unknown<'env>,
    pub(super) kind: InputKind,
}

/// Invoke the observable public method; the caller must await this raw result.
pub(super) fn owner<'env>(
    env: &'env Env,
    receiver: Unknown<'env>,
    ports: &LanguagePorts<'env>,
) -> napi::Result<Unknown<'env>> {
    call(env, ports.owner_frame, &[receiver.raw()])
}

/// This belongs synchronously after the public owner await, outside the later
/// async conversion function. Otherwise a context error gains a microtask turn.
pub(super) fn context<'env>(
    env: &'env Env,
    owner: Unknown<'env>,
    ports: &LanguagePorts<'env>,
) -> napi::Result<Unknown<'env>> {
    if !owner.coerce_to_bool()? {
        return plain_error(env, ports, "Cannot set input files to detached element");
    }
    let page = call(env, ports.page, &[owner.raw()])?;
    call(env, ports.context, &[page.raw()])
}

/// Run only array classification and the payload-size phase. No file name,
/// MIME, options or raw bytes are read. The caller invokes this INSIDE a JS
/// async conversion function and returns its ordinary `{ payloads }` carrier.
/// All callback/user work here is synchronous and requires no registry borrow.
pub(super) fn convert<'env>(
    env: &'env Env,
    context: Unknown<'env>,
    files: Unknown<'env>,
    ports: &LanguagePorts<'env>,
) -> napi::Result<Converted<'env>> {
    let is_array = call(env, ports.is_array, &[files.raw()])?;
    let items = if is_array.coerce_to_bool()? {
        call(env, ports.slice, &[files.raw()])?
    } else {
        singleton(env, files)?
    };
    // Fresh anonymous arrow adapters preserve callback arity/non-constructibility
    // when overridden some/every inspect or retain the supplied predicate.
    let predicate = string_predicate(env, ports)?;
    let any_path = call(env, ports.some, &[items.raw(), predicate.raw()])?;
    if any_path.coerce_to_bool()? {
        let predicate = string_predicate(env, ports)?;
        let all_paths = call(env, ports.every, &[items.raw(), predicate.raw()])?;
        if !all_paths.coerce_to_bool()? {
            return plain_error(env, ports, "File paths cannot be mixed with buffers");
        }
        return Ok(Converted {
            context,
            items,
            kind: InputKind::Paths,
        });
    }
    let reducer = size_reducer(env, ports)?;
    let zero = env.create_int32(0)?;
    let total = call(env, ports.reduce, &[items.raw(), reducer.raw(), zero.raw()])?;
    let limit = env.create_int32(PAYLOAD_SIZE_LIMIT)?;
    let exceeds = call(env, ports.at_least, &[total.raw(), limit.raw()])?;
    if exceeds.coerce_to_bool()? {
        return plain_error(
            env,
            ports,
            "Cannot set buffer larger than 50Mb, please write it to a file and pass its path instead.",
        );
    }
    Ok(Converted {
        context,
        items,
        kind: InputKind::Payloads,
    })
}

fn call<'env>(
    env: &'env Env,
    port: Unknown<'env>,
    args: &[sys::napi_value],
) -> napi::Result<Unknown<'env>> {
    let mut undefined = ptr::null_mut();
    let mut result = ptr::null_mut();
    // SAFETY: every port/argument is rooted in the current callback scope. The
    // JS port itself performs the user member call with its original receiver.
    unsafe {
        status(sys::napi_get_undefined(env.raw(), &mut undefined))?;
        status(sys::napi_call_function(
            env.raw(),
            undefined,
            port.raw(),
            args.len(),
            args.as_ptr(),
            &mut result,
        ))?;
        Ok(Unknown::from_raw_unchecked(env.raw(), result))
    }
}

fn singleton<'env>(env: &'env Env, value: Unknown<'env>) -> napi::Result<Unknown<'env>> {
    let mut array = ptr::null_mut();
    let property = sys::napi_property_descriptor {
        utf8name: c"0".as_ptr(),
        name: ptr::null_mut(),
        method: None,
        getter: None,
        setter: None,
        value: value.raw(),
        attributes: sys::PropertyAttributes::writable
            | sys::PropertyAttributes::enumerable
            | sys::PropertyAttributes::configurable,
        data: ptr::null_mut(),
    };
    // SAFETY: create an ordinary array literal analogue without consulting the
    // replaceable Array constructor or inherited index setters.
    unsafe {
        status(sys::napi_create_array_with_length(env.raw(), 1, &mut array))?;
        status(sys::napi_define_properties(env.raw(), array, 1, &property))?;
        Ok(Unknown::from_raw_unchecked(env.raw(), array))
    }
}

fn string_predicate<'env>(
    env: &'env Env,
    ports: &LanguagePorts<'env>,
) -> napi::Result<Unknown<'env>> {
    let native: Function<'_, (), bool> = env
        .create_function_from_closure("pwrightFileInputIsString", |ctx| {
            Ok(ctx.get::<Unknown<'_>>(0)?.get_type()? == ValueType::String)
        })?;
    call(env, ports.unary, &[native.raw()])
}

// napi's closure Return type is not lifetime-polymorphic. This private,
// immediate return value is consumed by ToNapiValue in the SAME invocation;
// no callback-scoped Env/Unknown is captured or extended to 'static.
struct CallbackValue {
    env: sys::napi_env,
    value: sys::napi_value,
}
impl ToNapiValue for CallbackValue {
    unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> napi::Result<sys::napi_value> {
        if env != value.env {
            return Err(napi::Error::from_reason(
                "file-input callback environment mismatch",
            ));
        }
        Ok(value.value)
    }
}

fn size_reducer<'env>(env: &'env Env, ports: &LanguagePorts<'env>) -> napi::Result<Unknown<'env>> {
    let native: Function<'_, (), CallbackValue> =
        env.create_function_from_closure("pwrightFileInputSize", |ctx| {
            // The JS binary adapter supplies the current invocation's operands
            // AND ports, including when a poisoned reduce escapes this callback.
            // This native closure captures no handles, registry or session.
            let size = ctx.get::<Unknown<'_>>(0)?;
            let item = ctx.get::<Unknown<'_>>(1)?;
            let buffer = ctx.get::<Unknown<'_>>(2)?;
            let byte_length = ctx.get::<Unknown<'_>>(3)?;
            let add = ctx.get::<Unknown<'_>>(4)?;
            let first = call(ctx.env, buffer, &[item.raw()])?;
            let bytes = if first.coerce_to_bool()? {
                let second = call(ctx.env, buffer, &[item.raw()])?;
                call(ctx.env, byte_length, &[second.raw()])?
            } else {
                env_zero(ctx.env)?
            };
            let value = call(ctx.env, add, &[size.raw(), bytes.raw()])?;
            Ok(CallbackValue {
                env: ctx.env.raw(),
                value: value.raw(),
            })
        })?;
    call(
        env,
        ports.binary,
        &[
            native.raw(),
            ports.buffer.raw(),
            ports.byte_length.raw(),
            ports.add.raw(),
        ],
    )
}

fn env_zero(env: &Env) -> napi::Result<Unknown<'_>> {
    env.create_int32(0)?.into_unknown(env)
}

fn plain_error<T>(env: &Env, ports: &LanguagePorts<'_>, message: &str) -> napi::Result<T> {
    let message = env.create_string(message)?;
    let error = call(env, ports.error, &[message.raw()])?;
    // SAFETY: the port performs `new Error(message)` (including an overridden
    // constructor) and returns its scoped result. No extra N-API error code is
    // added. Existing user exceptions from ports are never cleared/replaced.
    unsafe {
        status(sys::napi_throw(env.raw(), error.raw()))?;
    }
    Err(napi::Error::new(
        napi::Status::PendingException,
        "file-input preflight rejected",
    ))
}
