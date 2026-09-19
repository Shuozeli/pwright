//! Opaque Node-API identity. No JS properties or prototype lookup prove a Handle.

use super::State;
use crate::{binding_error, evaluation_value::status};
use napi::{Env, JsValue, Unknown, ValueType, sys};
use pwright_node_host::{HostError, handles::PendingHandle};
use std::{
    ffi::c_void,
    ptr,
    sync::{Arc, Mutex, atomic::AtomicU8},
};

// Non-zero-sized, non-mergeable identity per loaded module. A global constant
// would accept another addon version's sidecar despite its unstable Rust ABI.
// This address is used only for identity, never dereferenced from the tag or
// exposed as a JavaScript property or portable object ID.
static MODULE_IDENTITY: AtomicU8 = AtomicU8::new(0);
fn tag() -> sys::napi_type_tag {
    sys::napi_type_tag {
        lower: std::ptr::addr_of!(MODULE_IDENTITY).addr() as u64,
        upper: 0x7077726967687432,
    }
}

struct Wrapper {
    state: Arc<Mutex<State>>,
}

pub(crate) enum ArgumentReference {
    Live(Arc<PendingHandle>),
    Disposed,
}

unsafe extern "C" fn finalize(_: sys::napi_env, data: *mut c_void, _: *mut c_void) {
    // SAFETY: only bind installs this finalizer, with one Box<Wrapper> pointer.
    drop(unsafe { Box::from_raw(data.cast::<Wrapper>()) });
}

pub(super) fn bind(env: &Env, value: Unknown<'_>, state: Arc<Mutex<State>>) -> napi::Result<()> {
    if value.get_type()? != ValueType::Object {
        return Err(binding_error(HostError::InvalidArgument(
            "Handle wrapper must be an object".into(),
        )));
    }
    let wrapper = Box::into_raw(Box::new(Wrapper { state }));
    // SAFETY: value is live in this Node callback. On failure ownership stays
    // here; on success Node owns exactly one finalization of the wrapper box.
    let wrapped = unsafe {
        sys::napi_wrap(
            env.raw(),
            value.raw(),
            wrapper.cast(),
            Some(finalize),
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    if wrapped != sys::Status::napi_ok {
        drop(unsafe { Box::from_raw(wrapper) });
        return status(wrapped);
    }
    let tagged = unsafe { sys::napi_type_tag_object(env.raw(), value.raw(), &tag()) };
    if tagged != sys::Status::napi_ok {
        // Never leave our tag on someone else's pointer: wrap must precede tag.
        // Remove a successful wrap if tagging fails; a failed removal remains
        // Node-owned and is reclaimed by the same finalizer, never double-freed.
        let mut removed = ptr::null_mut();
        let code = unsafe { sys::napi_remove_wrap(env.raw(), value.raw(), &mut removed) };
        if code == sys::Status::napi_ok {
            drop(unsafe { Box::from_raw(removed.cast::<Wrapper>()) });
        } else {
            return Err(binding_error(HostError::Backend(format!(
                "Handle branding failed ({tagged}); wrapper rollback failed ({code})"
            ))));
        }
        return status(tagged);
    }
    Ok(())
}

pub(crate) fn read_argument(
    env: &Env,
    value: Unknown<'_>,
) -> napi::Result<Option<ArgumentReference>> {
    let mut matches = false;
    // SAFETY: called only for an object in the active conversion callback.
    status(unsafe {
        sys::napi_check_object_type_tag(env.raw(), value.raw(), &tag(), &mut matches)
    })?;
    if !matches {
        return Ok(None);
    }
    let mut wrapper = ptr::null_mut();
    status(unsafe { sys::napi_unwrap(env.raw(), value.raw(), &mut wrapper) })?;
    if wrapper.is_null() {
        return Err(binding_error(HostError::Backend(
            "Handle wrapper has no native owner".into(),
        )));
    }
    // The unforgeable native tag was installed only after wrapping this type.
    let wrapper = unsafe { &*wrapper.cast::<Wrapper>() };
    let state = wrapper
        .state
        .lock()
        .map_err(|error| binding_error(HostError::Backend(error.to_string())))?;
    match &*state {
        State::Live(handle) => Ok(Some(ArgumentReference::Live(handle.clone()))),
        State::Disposed(_) => Ok(Some(ArgumentReference::Disposed)),
    }
}
