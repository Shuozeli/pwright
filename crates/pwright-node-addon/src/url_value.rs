//! Scoped Node WHATWG URL values. Callers own policy and admission.
use crate::binding_error;
use napi::{Env, JsString, JsValue, bindgen_prelude::FromNapiValue, sys};
use pwright_node_host::HostError;
use std::{ffi::CStr, ptr};

fn status(code: sys::napi_status) -> napi::Result<()> {
    if code == sys::Status::napi_ok {
        Ok(())
    } else {
        Err(napi::Error::new(
            napi::Status::from(code),
            "URL value conversion failed",
        ))
    }
}

fn property(env: &Env, object: sys::napi_value, key: &CStr) -> napi::Result<sys::napi_value> {
    let mut value = ptr::null_mut();
    // SAFETY: all handles belong to the active synchronous callback scope.
    status(unsafe { sys::napi_get_named_property(env.raw(), object, key.as_ptr(), &mut value) })?;
    Ok(value)
}

pub(crate) fn resolve<'env>(
    env: &'env Env,
    relative: JsString<'env>,
    base: Option<JsString<'env>>,
) -> napi::Result<Option<(JsString<'env>, JsString<'env>)>> {
    // Match the pinned try/catch around construction, toString and origin.
    // A fixed stack argument array avoids napi-rs Function's temporary Vec.
    let attempt = || {
        let mut global = ptr::null_mut();
        let mut undefined = ptr::null_mut();
        let mut object = ptr::null_mut();
        let mut serialized = ptr::null_mut();
        // SAFETY: Node initializes scoped handles; arguments stay live until return.
        unsafe {
            status(sys::napi_get_global(env.raw(), &mut global))?;
            status(sys::napi_get_undefined(env.raw(), &mut undefined))?;
            let constructor = property(env, global, c"URL")?;
            let args = [relative.raw(), base.map_or(undefined, |base| base.raw())];
            status(sys::napi_new_instance(
                env.raw(),
                constructor,
                args.len(),
                args.as_ptr(),
                &mut object,
            ))?;
            let to_string = property(env, object, c"toString")?;
            status(sys::napi_call_function(
                env.raw(),
                object,
                to_string,
                0,
                ptr::null(),
                &mut serialized,
            ))?;
        }
        let origin = property(env, object, c"origin")?;
        for value in [serialized, origin] {
            let mut kind = sys::ValueType::napi_undefined;
            // SAFETY: values were returned by Node in this callback scope.
            status(unsafe { sys::napi_typeof(env.raw(), value, &mut kind) })?;
            if kind != sys::ValueType::napi_string {
                return Err(binding_error(HostError::UnsupportedOperation(
                    "URL resolver returned non-string values".into(),
                )));
            }
        }
        // SAFETY: both handles have been checked as strings above.
        unsafe {
            Ok((
                JsString::from_napi_value(env.raw(), serialized)?,
                JsString::from_napi_value(env.raw(), origin)?,
            ))
        }
    };
    match attempt() {
        Ok(values) => Ok(Some(values)),
        Err(error) if error.status == napi::Status::PendingException => {
            let mut exception = ptr::null_mut();
            // SAFETY: clear only an exception raised within the pinned resolver try block.
            status(unsafe { sys::napi_get_and_clear_last_exception(env.raw(), &mut exception) })?;
            Ok(None)
        }
        Err(error) => Err(error),
    }
}
