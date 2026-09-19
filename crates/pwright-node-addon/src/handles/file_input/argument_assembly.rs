//! Scoped post-conversion argument construction, before channel validation.
//!
//! Rust orders the converted spread, options spread and timeout resolution.
//! No file bytes, persistent roots, native authority or asynchronous work are
//! owned here. The caller supplies synchronous phase admission and runs this
//! directly after the conversion await, without introducing another await.

use crate::evaluation_value::status;
use napi::{
    Env, JsValue, Unknown,
    bindgen_prelude::{JsObjectValue, Object},
    sys,
};
use std::ptr;

/// A callback-scoped JavaScript CopyDataProperties language port.
///
/// The trusted port is exactly `(target, source) => ({ ...target, ...source })`.
/// It returns a fresh ordinary object with own writable/enumerable/configurable
/// data properties. Intermediate targets never escape to user callbacks, so
/// re-spreading their data properties does not re-read original source getters.
/// Arbitrary custom ports are not covered by that compatibility contract.
pub(super) struct LanguagePorts<'env> {
    spread: Unknown<'env>,
}

impl<'env> LanguagePorts<'env> {
    /// Read outside every registry borrow. The normal caller supplies its
    /// frozen port table; this stores no persistent Node reference.
    pub(super) fn read(object: Object<'env>) -> napi::Result<Self> {
        Ok(Self {
            spread: object.get_named_property("spread")?,
        })
    }
}

/// Construct `{ ...converted, ...options, timeout: resolve_timeout() }`.
///
/// Original source ownKeys, descriptor and getter order (including symbols and
/// primitive/null spreading) belong to the native language spread operation.
/// Rust selects the two source operations, then invokes the supplied Rust
/// resolver exactly once. Its result stays an unvalidated language value.
/// Pending thrown values propagate without adding a channel API error prefix.
///
/// The extra intermediate objects are callback-scoped, not persistent roots.
/// The caller's phase budget accounts for control/handle storage, not arbitrary
/// V8 bytes copied by spread or allocated by user getters. Nothing here escapes
/// into a callback, Promise or native task.
pub(super) fn assemble<'env>(
    env: &'env Env,
    converted: Unknown<'env>,
    options: Unknown<'env>,
    ports: &LanguagePorts<'env>,
    resolve_timeout: impl FnOnce() -> napi::Result<Unknown<'env>>,
) -> napi::Result<Unknown<'env>> {
    let initial = ordinary_object(env)?;
    let converted = spread(env, ports.spread, initial, converted)?;
    let arguments = spread(env, ports.spread, converted, options)?;
    let timeout = resolve_timeout()?;

    let property = sys::napi_property_descriptor {
        utf8name: c"timeout".as_ptr(),
        name: ptr::null_mut(),
        method: None,
        getter: None,
        setter: None,
        value: timeout.raw(),
        attributes: sys::PropertyAttributes::writable
            | sys::PropertyAttributes::enumerable
            | sys::PropertyAttributes::configurable,
        data: ptr::null_mut(),
    };
    // SAFETY: the trusted spread port returned an ordinary scoped object with
    // configurable own data properties. Defining an existing timeout changes
    // its value without moving its insertion position; a new key is appended.
    // This bypasses inherited setters, as an object literal definition does.
    status(unsafe { sys::napi_define_properties(env.raw(), arguments.raw(), 1, &property) })?;
    Ok(arguments)
}

fn ordinary_object<'env>(env: &'env Env) -> napi::Result<Unknown<'env>> {
    let mut value = ptr::null_mut();
    // SAFETY: create an ordinary object in this live callback environment.
    unsafe {
        status(sys::napi_create_object(env.raw(), &mut value))?;
        Ok(Unknown::from_raw_unchecked(env.raw(), value))
    }
}

fn spread<'env>(
    env: &'env Env,
    port: Unknown<'env>,
    target: Unknown<'env>,
    source: Unknown<'env>,
) -> napi::Result<Unknown<'env>> {
    let mut receiver = ptr::null_mut();
    let mut result = ptr::null_mut();
    let args = [target.raw(), source.raw()];
    // SAFETY: all arguments and the trusted port belong to this callback's
    // handle scope. No registry borrow or raw byte pointer spans user code.
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
