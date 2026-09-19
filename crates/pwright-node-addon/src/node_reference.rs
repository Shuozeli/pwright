//! Owned, explicitly thread-affine Node references. Never sent into Rust futures.

use std::{marker::PhantomData, ptr, rc::Rc};

use napi::{Env, JsValue, Unknown, sys};

use crate::evaluation_value::status;

pub struct NodeReference {
    env: sys::napi_env,
    reference: sys::napi_ref,
    _node_thread: PhantomData<Rc<()>>,
}

impl NodeReference {
    pub fn new(env: &Env, value: Unknown<'_>) -> napi::Result<Self> {
        let mut reference = ptr::null_mut();
        // SAFETY: the live callback owns this env and its scoped value.
        status(unsafe { sys::napi_create_reference(env.raw(), value.raw(), 1, &mut reference) })?;
        Ok(Self {
            env: env.raw(),
            reference,
            _node_thread: PhantomData,
        })
    }

    pub fn get<'env>(&self, env: &'env Env) -> napi::Result<Unknown<'env>> {
        if env.raw() != self.env {
            return Err(napi::Error::new(
                napi::Status::InvalidArg,
                "Node reference environment mismatch",
            ));
        }
        let mut value = ptr::null_mut();
        // SAFETY: this non-Send owner is used only by its environment's thread.
        status(unsafe { sys::napi_get_reference_value(self.env, self.reference, &mut value) })?;
        Ok(unsafe { Unknown::from_raw_unchecked(self.env, value) })
    }
}

impl Drop for NodeReference {
    fn drop(&mut self) {
        // The environment registry drops every reference in its cleanup hook,
        // before Node destroys the env. Later finalizers own only empty cells.
        let code = unsafe { sys::napi_delete_reference(self.env, self.reference) };
        if code != sys::Status::napi_ok {
            eprintln!("Pwright environment cleanup failed: Node reference deletion status {code}");
        }
    }
}
