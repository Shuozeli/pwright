//! Synchronous raw server-carrier projection after native target marking.
//!
//! Rust selects all reads, short-circuiting and native truth-value projection.
//! Language ports perform individual property/relational/error operations.
//! This kernel grants no marking/session/native authority and retains no roots.
//! It must not be attached to Materialized without a real marking completion.

use crate::{binding_error, evaluation_value::status};
use napi::{
    Env, JsValue, Unknown, ValueType,
    bindgen_prelude::{JsObjectValue, Object, ToNapiValue},
    sys,
};
use pwright_node_host::HostError;
use std::ptr;

struct LanguagePorts<'env> {
    read: Unknown<'env>,
    greater_than: Unknown<'env>,
    destructure_error: Unknown<'env>,
}

impl<'env> LanguagePorts<'env> {
    fn read(object: Object<'env>) -> napi::Result<Self> {
        Ok(Self {
            read: object.get_named_property("read")?,
            greater_than: object.get_named_property("greater_than")?,
            destructure_error: object.get_named_property("destructure_error")?,
        })
    }
}

pub(super) struct Projected<'env> {
    pub(super) file_payloads: Unknown<'env>,
    pub(super) local_paths: Unknown<'env>,
    pub(super) local_directory: Unknown<'env>,
    pub(super) raw_multiple: Unknown<'env>,
    pub(super) multiple: bool,
    pub(super) directory_upload: bool,
}

pub(super) fn project<'env>(
    env: &'env Env,
    items: Unknown<'env>,
    ports: Object<'env>,
) -> napi::Result<Projected<'env>> {
    let ports = LanguagePorts::read(ports)?;
    if matches!(items.get_type()?, ValueType::Null | ValueType::Undefined) {
        // Generic property access has a different engine TypeError than the
        // original destructuring. This port performs that single language error
        // operation, not the selection/read-order policy below.
        call(env, ports.destructure_error, &[items.raw()])?;
        return Err(binding_error(HostError::InvalidArgument(
            "The nullish destructuring language port did not throw".into(),
        )));
    }

    // Destructuring completes before either length read/coercion. Later
    // getters may mutate an earlier referenced value but cannot replace it.
    let file_payloads = read(env, items, "filePayloads", &ports)?;
    let local_paths = read(env, items, "localPaths", &ports)?;
    let local_directory = read(env, items, "localDirectory", &ports)?;
    let mut raw_multiple = file_payloads;
    if raw_multiple.coerce_to_bool()? {
        raw_multiple = more_than_one(env, file_payloads, &ports)?;
    }
    if !raw_multiple.coerce_to_bool()? {
        raw_multiple = local_paths;
        if raw_multiple.coerce_to_bool()? {
            raw_multiple = more_than_one(env, local_paths, &ports)?;
        }
    }
    // ToBoolean does not invoke user coercion or the replaceable global Boolean.
    // Keep the raw expression result too: false, undefined, -0, 0n and NaN are
    // observably distinct before utility argument serialization.
    Ok(Projected {
        file_payloads,
        local_paths,
        local_directory,
        raw_multiple,
        multiple: raw_multiple.coerce_to_bool()?,
        directory_upload: local_directory.coerce_to_bool()?,
    })
}

fn more_than_one<'env>(
    env: &'env Env,
    value: Unknown<'env>,
    ports: &LanguagePorts<'env>,
) -> napi::Result<Unknown<'env>> {
    let length = read(env, value, "length", ports)?;
    let one = env.create_int32(1)?.into_unknown(env)?;
    call(env, ports.greater_than, &[length.raw(), one.raw()])
}

fn read<'env>(
    env: &'env Env,
    value: Unknown<'env>,
    key: &str,
    ports: &LanguagePorts<'env>,
) -> napi::Result<Unknown<'env>> {
    let key = env.create_string(key)?.into_unknown(env)?;
    call(env, ports.read, &[value.raw(), key.raw()])
}

fn call<'env>(
    env: &'env Env,
    function: Unknown<'env>,
    arguments: &[sys::napi_value],
) -> napi::Result<Unknown<'env>> {
    let mut receiver = ptr::null_mut();
    let mut value = ptr::null_mut();
    status(unsafe { sys::napi_get_undefined(env.raw(), &mut receiver) })?;
    status(unsafe {
        sys::napi_call_function(
            env.raw(),
            receiver,
            function.raw(),
            arguments.len(),
            arguments.as_ptr(),
            &mut value,
        )
    })?;
    Ok(unsafe { Unknown::from_raw_unchecked(env.raw(), value) })
}

/// This diagnostic does not advance a preparation or stand in for marking.
/// The eventual ticket integration must retain/account the exact raw values
/// and require an unforgeable marking completion before calling this kernel.
pub(super) fn inspect<'env>(
    env: &'env Env,
    items: Unknown<'env>,
    ports: Object<'env>,
) -> napi::Result<Unknown<'env>> {
    let result = project(env, items, ports)?;
    let values = [
        result.file_payloads,
        result.local_paths,
        result.local_directory,
        result.raw_multiple,
        result.multiple.into_unknown(env)?,
        result.directory_upload.into_unknown(env)?,
    ];
    let names = [
        c"filePayloads",
        c"localPaths",
        c"localDirectory",
        c"rawMultiple",
        c"multiple",
        c"directoryUpload",
    ];
    let mut object = ptr::null_mut();
    status(unsafe { sys::napi_create_object(env.raw(), &mut object) })?;
    let properties = std::array::from_fn::<_, 6, _>(|index| sys::napi_property_descriptor {
        utf8name: names[index].as_ptr(),
        name: ptr::null_mut(),
        method: None,
        getter: None,
        setter: None,
        value: values[index].raw(),
        attributes: sys::PropertyAttributes::writable
            | sys::PropertyAttributes::enumerable
            | sys::PropertyAttributes::configurable,
        data: ptr::null_mut(),
    });
    status(unsafe {
        sys::napi_define_properties(env.raw(), object, properties.len(), properties.as_ptr())
    })?;
    Ok(unsafe { Unknown::from_raw_unchecked(env.raw(), object) })
}
