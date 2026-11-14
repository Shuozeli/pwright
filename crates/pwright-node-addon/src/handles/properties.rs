//! Failure-safe conversion of a Rust-owned property batch; no browser logic.
use super::{HandlePolicy, NativeHandle, State, publication};
use crate::evaluation_value::status;
use napi::{bindgen_prelude::ToNapiValue, sys};
use pwright_node_host::handles::NamedPendingHandle;
#[cfg(feature = "element-handle-dom")]
use pwright_node_host::handles::PendingHandle;
use std::{
    ptr,
    sync::{Arc, Mutex},
};

pub(super) struct PropertiesOutput(pub Vec<NamedPendingHandle>, pub HandlePolicy);
#[cfg(feature = "element-handle-dom")]
pub(crate) struct HandlesOutput(pub Vec<PendingHandle>, pub HandlePolicy);

struct Recovery {
    states: Vec<Arc<Mutex<State>>>,
    armed: bool,
}
impl Drop for Recovery {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        for state in &self.states {
            // Cleanup-only poison recovery: rollback must revoke every native
            // wrapper already installed in JS, without waiting for GC. This
            // only submits existing owned tickets; it performs no native I/O.
            let mut state = state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let State::Live(handle) = &*state {
                let release = handle.request_dispose();
                *state = State::Disposed(release);
            }
        }
    }
}

impl ToNapiValue for PropertiesOutput {
    unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> napi::Result<sys::napi_value> {
        let mut recovery = Recovery {
            states: Vec::with_capacity(value.0.len()),
            armed: true,
        };
        let mut array = ptr::null_mut();
        // SAFETY: the host admitted and bounded this result count before I/O;
        // all conversions occur inside the active Node callback environment.
        status(unsafe { sys::napi_create_array_with_length(env, value.0.len(), &mut array) })?;
        for (index, property) in value.0.into_iter().enumerate() {
            let mut name = ptr::null_mut();
            // The property's shared call allowance lives through copying its
            // borrowed name. No extra Rust String copy or unowned ID escapes.
            status(unsafe {
                sys::napi_create_string_utf8(
                    env,
                    property.name().as_ptr().cast(),
                    isize::try_from(property.name().len()).map_err(|_| {
                        napi::Error::from_reason("property name exceeds N-API capacity")
                    })?,
                    &mut name,
                )
            })?;
            let handle = NativeHandle::new(property.into_handle(), value.1.clone())
                .map_err(crate::binding_error)?;
            recovery.states.push(handle.state.clone());
            let native = unsafe { publication::to_napi_value(env, handle) }?;
            let mut row = ptr::null_mut();
            status(unsafe { sys::napi_create_object(env, &mut row) })?;
            // Define OWN data properties, bypassing poisoned Object/Array
            // prototype setters that could swallow or intercept publication.
            let descriptors = [
                descriptor(c"name".as_ptr(), ptr::null_mut(), name),
                descriptor(c"handle".as_ptr(), ptr::null_mut(), native),
            ];
            status(unsafe {
                sys::napi_define_properties(env, row, descriptors.len(), descriptors.as_ptr())
            })?;
            let index = index.to_string();
            let mut key = ptr::null_mut();
            status(unsafe {
                sys::napi_create_string_utf8(
                    env,
                    index.as_ptr().cast(),
                    isize::try_from(index.len()).map_err(|_| {
                        napi::Error::from_reason("property index exceeds N-API capacity")
                    })?,
                    &mut key,
                )
            })?;
            let entry = descriptor(ptr::null(), key, row);
            status(unsafe { sys::napi_define_properties(env, array, 1, &entry) })?;
        }
        recovery.armed = false;
        Ok(array)
    }
}

#[cfg(feature = "element-handle-dom")]
impl ToNapiValue for HandlesOutput {
    unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> napi::Result<sys::napi_value> {
        let mut recovery = Recovery {
            states: Vec::with_capacity(value.0.len()),
            armed: true,
        };
        let mut array = ptr::null_mut();
        // SAFETY: the host admitted and validated the complete element batch;
        // conversion runs inside the active Node callback environment.
        status(unsafe { sys::napi_create_array_with_length(env, value.0.len(), &mut array) })?;
        for (index, pending) in value.0.into_iter().enumerate() {
            let handle =
                NativeHandle::new(pending, value.1.clone()).map_err(crate::binding_error)?;
            recovery.states.push(handle.state.clone());
            let native = unsafe { publication::to_napi_value(env, handle) }?;
            let index = index.to_string();
            let mut key = ptr::null_mut();
            status(unsafe {
                sys::napi_create_string_utf8(
                    env,
                    index.as_ptr().cast(),
                    isize::try_from(index.len()).map_err(|_| {
                        napi::Error::from_reason("Handle index exceeds N-API capacity")
                    })?,
                    &mut key,
                )
            })?;
            let entry = descriptor(ptr::null(), key, native);
            status(unsafe { sys::napi_define_properties(env, array, 1, &entry) })?;
        }
        recovery.armed = false;
        Ok(array)
    }
}

fn descriptor(
    utf8name: *const std::ffi::c_char,
    name: sys::napi_value,
    value: sys::napi_value,
) -> sys::napi_property_descriptor {
    sys::napi_property_descriptor {
        utf8name,
        name,
        method: None,
        getter: None,
        setter: None,
        value,
        attributes: sys::PropertyAttributes::writable
            | sys::PropertyAttributes::enumerable
            | sys::PropertyAttributes::configurable,
        data: ptr::null_mut(),
    }
}
