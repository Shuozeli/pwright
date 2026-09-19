//! Synchronous raw-Buffer channel validation, with policy and schema in Rust.
//!
//! Values remain callback-scoped, including arbitrary map/valueOf results. No
//! bytes, user strings, arrays or native owners are copied into Rust storage.
//! The caller admits synchronous phase storage and owns any persistent roots.
//! Escaped map callbacks use statically checked zero-capture Rust closures;
//! JavaScript forwards fresh invocation values, paths and language ports.
//!
//! This is the normal, non-isUnderTest schema. Unknown fields are ignored.
//! This selected local-upload surface has no WritableStream producer/brand:
//! client channels perform the first observable `_object` read, then fail closed.
//! Server channels check the wire GUID shape but have no registered dispatcher;
//! even correctly shaped values fail closed without granting native authority.
//! Valid stream support and whole-channel/API-wrapper parity are not claimed.

use crate::evaluation_value::status;
use napi::{
    Env, JsValue, Unknown, ValueType,
    bindgen_prelude::{Function, FunctionCallContext, JsObjectValue, Object, ToNapiValue},
    sys,
};
use std::ptr;

struct LanguagePorts<'env> {
    object: Object<'env>,
    read: Unknown<'env>,
    assign: Unknown<'env>,
    is_undefined: Unknown<'env>,
    is_null: Unknown<'env>,
    is_array: Unknown<'env>,
    is_string_object: Unknown<'env>,
    is_number_object: Unknown<'env>,
    is_buffer: Unknown<'env>,
    value_of: Unknown<'env>,
    map: Unknown<'env>,
    add: Unknown<'env>,
    mapper: Unknown<'env>,
    error: Unknown<'env>,
}

impl<'env> LanguagePorts<'env> {
    fn read(object: Object<'env>) -> napi::Result<Self> {
        Ok(Self {
            read: object.get_named_property("read")?,
            assign: object.get_named_property("assign")?,
            is_undefined: object.get_named_property("is_undefined")?,
            is_null: object.get_named_property("is_null")?,
            is_array: object.get_named_property("is_array")?,
            is_string_object: object.get_named_property("is_string_object")?,
            is_number_object: object.get_named_property("is_number_object")?,
            is_buffer: object.get_named_property("is_buffer")?,
            value_of: object.get_named_property("value_of")?,
            map: object.get_named_property("map")?,
            add: object.get_named_property("add")?,
            mapper: object.get_named_property("mapper")?,
            error: object.get_named_property("error")?,
            object,
        })
    }
}

#[derive(Clone, Copy)]
enum ElementKind {
    Payload,
    String,
    Channel,
}

#[derive(Clone, Copy)]
enum Kind {
    String,
    Float,
    Binary,
    Channel,
    Array(ElementKind),
}

struct Field {
    name: &'static str,
    kind: Kind,
    optional: bool,
}

const PARAMETERS: [Field; 6] = [
    Field {
        name: "payloads",
        kind: Kind::Array(ElementKind::Payload),
        optional: true,
    },
    Field {
        name: "localDirectory",
        kind: Kind::String,
        optional: true,
    },
    Field {
        name: "directoryStream",
        kind: Kind::Channel,
        optional: true,
    },
    Field {
        name: "localPaths",
        kind: Kind::Array(ElementKind::String),
        optional: true,
    },
    Field {
        name: "streams",
        kind: Kind::Array(ElementKind::Channel),
        optional: true,
    },
    Field {
        name: "timeout",
        kind: Kind::Float,
        optional: false,
    },
];
const PAYLOAD: [Field; 3] = [
    Field {
        name: "name",
        kind: Kind::String,
        optional: false,
    },
    Field {
        name: "mimeType",
        kind: Kind::String,
        optional: true,
    },
    Field {
        name: "buffer",
        kind: Kind::Binary,
        optional: false,
    },
];

pub(super) fn validate<'env>(
    env: &'env Env,
    params: Unknown<'env>,
    ports: Object<'env>,
) -> napi::Result<Unknown<'env>> {
    let ports = LanguagePorts::read(ports)?;
    object::<false>(env, params, text(env, "")?, &PARAMETERS, &ports)
}

/// Server-side raw-Buffer schema pass, without a stream dispatcher registry.
/// Direction is statically propagated into every escaped ZST array callback.
pub(super) fn validate_server<'env>(
    env: &'env Env,
    params: Unknown<'env>,
    ports: Object<'env>,
) -> napi::Result<Unknown<'env>> {
    let ports = LanguagePorts::read(ports)?;
    object::<true>(env, params, text(env, "")?, &PARAMETERS, &ports)
}

fn object<'env, const SERVER: bool>(
    env: &'env Env,
    value: Unknown<'env>,
    path: Unknown<'env>,
    fields: &[Field],
    ports: &LanguagePorts<'env>,
) -> napi::Result<Unknown<'env>> {
    if predicate(env, ports.is_null, value)? {
        return rejection(env, path, ": expected object, got null", ports);
    }
    if !matches!(
        value.get_type()?,
        ValueType::Object | ValueType::Null | ValueType::External
    ) {
        return type_error(env, value, path, ": expected object, got ", ports);
    }
    let mut result = ptr::null_mut();
    // SAFETY: ordinary object-literal analogue in this callback's environment.
    // Subsequent assignment deliberately DOES consult inherited setters.
    status(unsafe { sys::napi_create_object(env.raw(), &mut result) })?;
    let result = unsafe { Unknown::from_raw_unchecked(env.raw(), result) };
    for field in fields {
        let key = text(env, field.name)?;
        // Upstream reads the member before building its diagnostic path.
        let input = call(env, ports.read, &[value.raw(), key.raw()])?;
        let field_path = if path.coerce_to_bool()? {
            let prefix = add(env, path, text(env, ".")?, ports)?;
            add(env, prefix, key, ports)?
        } else {
            key
        };
        let output = if field.optional && predicate(env, ports.is_undefined, input)? {
            input
        } else {
            validate_kind::<SERVER>(env, input, field_path, field.kind, ports)?
        };
        if !predicate(env, ports.is_undefined, output)? {
            // Match `result[key] = output`, not Object.assign/defineProperty.
            call(env, ports.assign, &[result.raw(), key.raw(), output.raw()])?;
        }
    }
    Ok(result)
}

fn validate_kind<'env, const SERVER: bool>(
    env: &'env Env,
    value: Unknown<'env>,
    path: Unknown<'env>,
    kind: Kind,
    ports: &LanguagePorts<'env>,
) -> napi::Result<Unknown<'env>> {
    match kind {
        Kind::String => {
            if predicate(env, ports.is_string_object, value)? {
                return call(env, ports.value_of, &[value.raw()]);
            }
            if value.get_type()? == ValueType::String {
                return Ok(value);
            }
            type_error(env, value, path, ": expected string, got ", ports)
        }
        Kind::Float => {
            if predicate(env, ports.is_number_object, value)? {
                return call(env, ports.value_of, &[value.raw()]);
            }
            if value.get_type()? == ValueType::Number {
                return Ok(value);
            }
            type_error(env, value, path, ": expected float, got ", ports)
        }
        Kind::Binary => {
            if predicate(env, ports.is_buffer, value)? {
                return Ok(value);
            }
            type_error(env, value, path, ": expected Buffer, got ", ports)
        }
        Kind::Channel => {
            if SERVER {
                return server_channel(env, value, path, ports);
            }
            let key = text(env, "_object")?;
            call(env, ports.read, &[value.raw(), key.raw()])?;
            // No local WritableStream brand exists. Never grant authority by
            // duck typing _type/_guid or by accepting another module's owner.
            rejection(env, path, ": expected channel WritableStream", ports)
        }
        Kind::Array(kind) => array::<SERVER>(env, value, path, kind, ports),
    }
}

fn server_channel<'env>(
    env: &'env Env,
    value: Unknown<'env>,
    path: Unknown<'env>,
    ports: &LanguagePorts<'env>,
) -> napi::Result<Unknown<'env>> {
    // Match the server's short-circuit checks before either observable GUID
    // read. A function with a GUID is not a wire channel; null is not read.
    if value.coerce_to_bool()?
        && matches!(
            value.get_type()?,
            ValueType::Object | ValueType::Null | ValueType::External
        )
    {
        let key = text(env, "guid")?;
        let first = call(env, ports.read, &[value.raw(), key.raw()])?;
        if first.get_type()? == ValueType::String {
            // Upstream reads again for the registry key. An accessor may now
            // return any value; do not reuse or project the first string.
            let guid = call(env, ports.read, &[value.raw(), key.raw()])?;
            // There is no WritableStream dispatcher registry in this selected
            // capability. Shape alone never grants a borrowed/foreign owner.
            let message = add(env, path, text(env, ": no object with guid ")?, ports)?;
            // Template interpolation uses string-hint coercion and throws on
            // Symbol, unlike String(value) or a direct default-hint `+`.
            let template = ports.object.get_named_property("template")?;
            let guid = call(env, template, &[guid.raw()])?;
            return throw(env, add(env, message, guid, ports)?, ports);
        }
    }
    rejection(env, path, ": expected guid for WritableStream", ports)
}

fn array<'env, const SERVER: bool>(
    env: &'env Env,
    value: Unknown<'env>,
    path: Unknown<'env>,
    kind: ElementKind,
    ports: &LanguagePorts<'env>,
) -> napi::Result<Unknown<'env>> {
    if !predicate(env, ports.is_array, value)? {
        return type_error(env, value, path, ": expected array, got ", ports);
    }
    // Actual no-capture closures, not function pointers or a captured enum.
    // napi-rs boxes F before its fallible create/finalizer calls; a ZST has no
    // Rust heap allocation to leak or retain after this phase has completed.
    let native = match kind {
        ElementKind::Payload => {
            native_mapper(env, |ctx| map_element::<SERVER>(ctx, ElementKind::Payload))
        }
        ElementKind::String => {
            native_mapper(env, |ctx| map_element::<SERVER>(ctx, ElementKind::String))
        }
        ElementKind::Channel => {
            native_mapper(env, |ctx| map_element::<SERVER>(ctx, ElementKind::Channel))
        }
    }?;
    // Trusted adapter: (native,path,ports) =>
    // (value,index) => native(value,index,path,ports). Fresh, arity 2, no `new`.
    let callback = call(
        env,
        ports.mapper,
        &[native.raw(), path.raw(), ports.object.raw()],
    )?;
    // Do not inspect/copy/normalize the returned value: custom map may return
    // a non-array, a Promise, sparse results, or boxed/otherwise invalid values.
    call(env, ports.map, &[value.raw(), callback.raw()])
}

fn native_mapper<F>(env: &Env, callback: F) -> napi::Result<Function<'_, (), CallbackValue>>
where
    F: 'static + for<'scope> Fn(FunctionCallContext<'scope>) -> napi::Result<CallbackValue>,
{
    const { assert!(std::mem::size_of::<F>() == 0) };
    env.create_function_from_closure("pwrightFileInputChannelElement", callback)
}

fn map_element<const SERVER: bool>(
    ctx: FunctionCallContext<'_>,
    kind: ElementKind,
) -> napi::Result<CallbackValue> {
    // A custom map can escape this callback. Every later invocation supplies
    // values from its own live scope; no Env/path/ports/permit is captured.
    let value = ctx.get::<Unknown<'_>>(0)?;
    let index = ctx.get::<Unknown<'_>>(1)?;
    let path = ctx.get::<Unknown<'_>>(2)?;
    let ports = LanguagePorts::read(ctx.get::<Object<'_>>(3)?)?;
    // Preserve left-to-right JS +, including Symbol/ToPrimitive errors and
    // arbitrary indices supplied by an overridden map function.
    let path = add(ctx.env, path, text(ctx.env, "[")?, &ports)?;
    let path = add(ctx.env, path, index, &ports)?;
    let path = add(ctx.env, path, text(ctx.env, "]")?, &ports)?;
    let value = match kind {
        ElementKind::Payload => object::<SERVER>(ctx.env, value, path, &PAYLOAD, &ports),
        ElementKind::String => validate_kind::<SERVER>(ctx.env, value, path, Kind::String, &ports),
        ElementKind::Channel => {
            validate_kind::<SERVER>(ctx.env, value, path, Kind::Channel, &ports)
        }
    }?;
    Ok(CallbackValue {
        env: ctx.env.raw(),
        value: value.raw(),
    })
}

fn predicate(env: &Env, port: Unknown<'_>, value: Unknown<'_>) -> napi::Result<bool> {
    call(env, port, &[value.raw()])?.coerce_to_bool()
}

fn add<'env>(
    env: &'env Env,
    left: Unknown<'env>,
    right: Unknown<'env>,
    ports: &LanguagePorts<'env>,
) -> napi::Result<Unknown<'env>> {
    call(env, ports.add, &[left.raw(), right.raw()])
}

fn text<'env>(env: &'env Env, value: &str) -> napi::Result<Unknown<'env>> {
    env.create_string(value)?.into_unknown(env)
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

fn type_error<'env, T>(
    env: &'env Env,
    value: Unknown<'env>,
    path: Unknown<'env>,
    expected: &str,
    ports: &LanguagePorts<'env>,
) -> napi::Result<T> {
    let message = add(env, path, text(env, expected)?, ports)?;
    let message = add(
        env,
        message,
        text(env, type_name(value.get_type()?))?,
        ports,
    )?;
    throw(env, message, ports)
}

fn rejection<'env, T>(
    env: &'env Env,
    path: Unknown<'env>,
    suffix: &str,
    ports: &LanguagePorts<'env>,
) -> napi::Result<T> {
    throw(env, add(env, path, text(env, suffix)?, ports)?, ports)
}

fn throw<T>(env: &Env, message: Unknown<'_>, ports: &LanguagePorts<'_>) -> napi::Result<T> {
    let error = call(env, ports.error, &[message.raw()])?;
    // SAFETY: the scoped language port creates ValidationError. Any preceding
    // user exception is propagated unchanged rather than cleared/replaced.
    status(unsafe { sys::napi_throw(env.raw(), error.raw()) })?;
    Err(napi::Error::new(
        napi::Status::PendingException,
        "file-input channel validation rejected",
    ))
}

fn call<'env>(
    env: &'env Env,
    port: Unknown<'_>,
    args: &[sys::napi_value],
) -> napi::Result<Unknown<'env>> {
    let mut receiver = ptr::null_mut();
    let mut result = ptr::null_mut();
    // SAFETY: ports and arguments belong to the active Node callback scope.
    unsafe {
        status(sys::napi_get_undefined(env.raw(), &mut receiver))?;
        status(sys::napi_call_function(
            env.raw(),
            receiver,
            port.raw(),
            args.len(),
            args.as_ptr(),
            &mut result,
        ))?;
        Ok(Unknown::from_raw_unchecked(env.raw(), result))
    }
}

// Immediate callback return, consumed by napi-rs in that SAME invocation.
// This does not extend a scoped value's lifetime or permit cross-thread use.
struct CallbackValue {
    env: sys::napi_env,
    value: sys::napi_value,
}
impl ToNapiValue for CallbackValue {
    unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> napi::Result<sys::napi_value> {
        if env != value.env {
            return Err(napi::Error::from_reason(
                "file-input channel callback environment mismatch",
            ));
        }
        Ok(value.value)
    }
}
