//! Scoped in-memory projection of pinned prepareFilesForUpload behavior.
//!
//! Rust owns input selection, property order, MIME policy and encoding choice;
//! JavaScript ports perform individual language operations. No byte copying,
//! admission, filesystem, native command, scheduler or persistent root lives
//! here. The caller supplies the async-return boundary for carrier assimilation.
//!
//! Selected non-payload inputs reject only after the observable exactly-one
//! check. Valid paths/streams and the inaccessible trusted server Frame metadata
//! read are not implemented; this is not a full dispatcher/server boundary.
//! PinnedMime's final extension alone enters Rust, in table-sized stack storage.
//! Arbitrarily long filenames and overridden methods' temporary values remain
//! scoped V8 values; neither their graphs nor arbitrary callback work is bounded
//! by this fixed Rust scratch proof.

use crate::{binding_error, evaluation_value::status};
use napi::{
    Env, JsString, JsValue, Unknown, ValueType,
    bindgen_prelude::{Function, FunctionCallContext, JsObjectValue, Object, ToNapiValue},
    sys,
};
use pwright_node_host::{HostError, file_input::PinnedMime};
use std::ptr;

struct LanguagePorts<'env> {
    object: Object<'env>,
    read: Unknown<'env>,
    filter_truthy: Unknown<'env>,
    equal: Unknown<'env>,
    map: Unknown<'env>,
    mapper: Unknown<'env>,
    string: Unknown<'env>,
    replace: Unknown<'env>,
    lower_case: Unknown<'env>,
    less_than: Unknown<'env>,
    subtract: Unknown<'env>,
    property_key: Unknown<'env>,
    serialize: Unknown<'env>,
    error: Unknown<'env>,
}

impl<'env> LanguagePorts<'env> {
    fn read(object: Object<'env>) -> napi::Result<Self> {
        Ok(Self {
            read: object.get_named_property("read")?,
            filter_truthy: object.get_named_property("filter_truthy")?,
            equal: object.get_named_property("equal")?,
            map: object.get_named_property("map")?,
            mapper: object.get_named_property("mapper")?,
            string: object.get_named_property("string")?,
            replace: object.get_named_property("replace")?,
            lower_case: object.get_named_property("lower_case")?,
            less_than: object.get_named_property("less_than")?,
            subtract: object.get_named_property("subtract")?,
            property_key: object.get_named_property("property_key")?,
            serialize: object.get_named_property("serialize")?,
            error: object.get_named_property("error")?,
            object,
        })
    }
}

pub(super) fn materialize<'env>(
    env: &'env Env,
    params: Unknown<'env>,
    ports: Object<'env>,
) -> napi::Result<Unknown<'env>> {
    let ports = LanguagePorts::read(ports)?;
    // Destructuring order is distinct from the following selection-array order.
    let payloads = read(env, params, "payloads", &ports)?;
    let streams = read(env, params, "streams", &ports)?;
    let directory_stream = read(env, params, "directoryStream", &ports)?;
    let local_paths = read(env, params, "localPaths", &ports)?;
    let local_directory = read(env, params, "localDirectory", &ports)?;
    let selected = literal_array(
        env,
        [
            payloads,
            local_paths,
            local_directory,
            streams,
            directory_stream,
        ],
    )?;
    let filtered = call(env, ports.filter_truthy, &[selected.raw()])?;
    let length = read(env, filtered, "length", &ports)?;
    let one = env.create_int32(1)?.into_unknown(env)?;
    if !call(env, ports.equal, &[length.raw(), one.raw()])?.coerce_to_bool()? {
        return plain_error(
            env,
            "Exactly one of payloads, localPaths and streams must be provided",
            &ports,
        );
    }
    // This selected boundary does not touch path()/filesystem/stream authority.
    // A poisoned filter may hide a mixed input, but that cannot enable modes
    // this implementation has not selected or admitted.
    if streams.coerce_to_bool()?
        || directory_stream.coerce_to_bool()?
        || local_paths.coerce_to_bool()?
        || local_directory.coerce_to_bool()?
    {
        return Err(binding_error(HostError::UnsupportedOperation(
            "retained file-input path/stream materialization is not implemented".into(),
        )));
    }
    let file_payloads = if matches!(payloads.get_type()?, ValueType::Null | ValueType::Undefined) {
        undefined(env)?
    } else {
        let native = native_mapper(env, map_payload)?;
        let mapper = call(env, ports.mapper, &[native.raw(), ports.object.raw()])?;
        call(env, ports.map, &[payloads.raw(), mapper.raw()])?
    };
    // Object literal, including undefined values, not validator-style omission.
    let result = empty_object(env)?;
    define(env, result, c"localPaths", local_paths)?;
    define(env, result, c"localDirectory", local_directory)?;
    define(env, result, c"filePayloads", file_payloads)?;
    Ok(result)
}

fn native_mapper<F>(env: &Env, callback: F) -> napi::Result<Function<'_, (), CallbackValue>>
where
    F: 'static + for<'scope> Fn(FunctionCallContext<'scope>) -> napi::Result<CallbackValue>,
{
    // No boxed capture may outlive phase admission or leak on NAPI failure.
    // A function ITEM is also zero-sized; never coerce it to a function pointer.
    const { assert!(std::mem::size_of::<F>() == 0) };
    env.create_function_from_closure("pwrightMaterializeFilePayload", callback)
}

fn map_payload(ctx: FunctionCallContext<'_>) -> napi::Result<CallbackValue> {
    let payload = ctx.get::<Unknown<'_>>(0)?;
    let ports = LanguagePorts::read(ctx.get::<Object<'_>>(1)?)?;
    let result = empty_object(ctx.env)?;
    let name = read(ctx.env, payload, "name", &ports)?;
    define(ctx.env, result, c"name", name)?;
    let mime = read(ctx.env, payload, "mimeType", &ports)?;
    let mime = if mime.coerce_to_bool()? {
        mime
    } else {
        // The second name read is observable and may differ from `name` above.
        let name = read(ctx.env, payload, "name", &ports)?;
        match mime_lookup(ctx.env, name, &ports)? {
            Some(mime) => text(ctx.env, mime)?,
            None => text(ctx.env, "application/octet-stream")?,
        }
    };
    define(ctx.env, result, c"mimeType", mime)?;
    let buffer = read(ctx.env, payload, "buffer", &ports)?;
    let encoding = text(ctx.env, "base64")?;
    let buffer = call(ctx.env, ports.serialize, &[buffer.raw(), encoding.raw()])?;
    define(ctx.env, result, c"buffer", buffer)?;
    let modified = read(ctx.env, payload, "lastModifiedMs", &ports)?;
    define(ctx.env, result, c"lastModifiedMs", modified)?;
    Ok(CallbackValue {
        env: ctx.env.raw(),
        value: result.raw(),
    })
}

fn mime_lookup<'env>(
    env: &'env Env,
    name: Unknown<'env>,
    ports: &LanguagePorts<'env>,
) -> napi::Result<Option<&'static str>> {
    let name = call(env, ports.string, &[name.raw()])?;
    let empty = text(env, "")?;
    let path_pattern = text(env, r"^.*[/\\]")?;
    let filename = call(
        env,
        ports.replace,
        &[name.raw(), path_pattern.raw(), empty.raw()],
    )?;
    let filename = call(env, ports.lower_case, &[filename.raw()])?;
    let extension_pattern = text(env, r"^.*\.")?;
    let extension = call(
        env,
        ports.replace,
        &[filename.raw(), extension_pattern.raw(), empty.raw()],
    )?;
    let extension = call(env, ports.lower_case, &[extension.raw()])?;
    // Preserve every raw method return and each repeated property/coercion.
    let filename_length = read(env, filename, "length", ports)?;
    let name_length = read(env, name, "length", ports)?;
    let has_path = call(
        env,
        ports.less_than,
        &[filename_length.raw(), name_length.raw()],
    )?
    .coerce_to_bool()?;
    let extension_length = read(env, extension, "length", ports)?;
    let filename_length = read(env, filename, "length", ports)?;
    let one = env.create_int32(1)?.into_unknown(env)?;
    let without_dot = call(env, ports.subtract, &[filename_length.raw(), one.raw()])?;
    let short_extension = call(
        env,
        ports.less_than,
        &[extension_length.raw(), without_dot.raw()],
    )?
    .coerce_to_bool()?;
    if !short_extension && has_path {
        return Ok(None);
    }
    // The original null-prototype MIME dictionary performs ToPropertyKey at
    // this point, including when an overridden lowerCase returned an object.
    let key = call(env, ports.property_key, &[extension.raw()])?;
    if key.get_type()? != ValueType::String {
        // Symbol dictionary keys cannot match the pinned string-only table.
        return Ok(None);
    }
    let key = unsafe { key.cast::<JsString<'_>>()? };
    let length = key.utf8_len()?;
    if length > PinnedMime::MAX_EXTENSION_BYTES {
        return Ok(None);
    }
    // NAPI copies only this already-bounded extension. No Rust String/Vec and
    // no full-filename copy, even for arbitrarily long prefixes/unknown suffixes.
    let mut bytes = [0u8; PinnedMime::MAX_EXTENSION_BYTES + 1];
    let mut written = 0;
    status(unsafe {
        sys::napi_get_value_string_utf8(
            env.raw(),
            key.raw(),
            bytes.as_mut_ptr().cast(),
            bytes.len(),
            &mut written,
        )
    })?;
    let extension = std::str::from_utf8(&bytes[..written])
        .map_err(|_| napi::Error::from_reason("MIME extension conversion was not UTF-8"))?;
    static MIME: PinnedMime = PinnedMime;
    Ok(MIME.lookup_extension(extension))
}

fn literal_array<'env>(env: &'env Env, values: [Unknown<'env>; 5]) -> napi::Result<Unknown<'env>> {
    let mut result = ptr::null_mut();
    status(unsafe { sys::napi_create_array_with_length(env.raw(), values.len(), &mut result) })?;
    let result = unsafe { Unknown::from_raw_unchecked(env.raw(), result) };
    let keys = [c"0", c"1", c"2", c"3", c"4"];
    for (key, value) in keys.into_iter().zip(values) {
        define(env, result, key, value)?;
    }
    Ok(result)
}

fn empty_object(env: &Env) -> napi::Result<Unknown<'_>> {
    let mut value = ptr::null_mut();
    status(unsafe { sys::napi_create_object(env.raw(), &mut value) })?;
    Ok(unsafe { Unknown::from_raw_unchecked(env.raw(), value) })
}

fn define(
    env: &Env,
    object: Unknown<'_>,
    key: &std::ffi::CStr,
    value: Unknown<'_>,
) -> napi::Result<()> {
    let property = sys::napi_property_descriptor {
        utf8name: key.as_ptr(),
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
    // Own data definitions reproduce object/array literals without setters.
    status(unsafe { sys::napi_define_properties(env.raw(), object.raw(), 1, &property) })
}

fn read<'env>(
    env: &'env Env,
    value: Unknown<'env>,
    key: &str,
    ports: &LanguagePorts<'env>,
) -> napi::Result<Unknown<'env>> {
    let key = text(env, key)?;
    call(env, ports.read, &[value.raw(), key.raw()])
}

fn text<'env>(env: &'env Env, value: &str) -> napi::Result<Unknown<'env>> {
    env.create_string(value)?.into_unknown(env)
}

fn undefined(env: &Env) -> napi::Result<Unknown<'_>> {
    let mut value = ptr::null_mut();
    status(unsafe { sys::napi_get_undefined(env.raw(), &mut value) })?;
    Ok(unsafe { Unknown::from_raw_unchecked(env.raw(), value) })
}

fn plain_error<T>(env: &Env, message: &str, ports: &LanguagePorts<'_>) -> napi::Result<T> {
    let message = text(env, message)?;
    let error = call(env, ports.error, &[message.raw()])?;
    status(unsafe { sys::napi_throw(env.raw(), error.raw()) })?;
    Err(napi::Error::new(
        napi::Status::PendingException,
        "file-input materialization rejected",
    ))
}

fn call<'env>(
    env: &'env Env,
    port: Unknown<'_>,
    args: &[sys::napi_value],
) -> napi::Result<Unknown<'env>> {
    let mut receiver = ptr::null_mut();
    let mut result = ptr::null_mut();
    // All values belong to this current Node callback; no foreign handle is
    // captured into a native closure or an asynchronous Rust future.
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

struct CallbackValue {
    env: sys::napi_env,
    value: sys::napi_value,
}
impl ToNapiValue for CallbackValue {
    unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> napi::Result<sys::napi_value> {
        if env != value.env {
            return Err(napi::Error::from_reason(
                "file-input materialization callback environment mismatch",
            ));
        }
        Ok(value.value)
    }
}
