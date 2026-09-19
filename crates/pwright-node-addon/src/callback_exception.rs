//! A scoped language exception port, not a command failure or deferred queue.
use crate::NativeSession;
use napi::{Env, JsValue, Unknown, sys};
use napi_derive::napi;

#[napi]
impl NativeSession {
    /// Report the original synchronously thrown listener value through Node's
    /// uncaught-exception channel. Never retain it, stringify it, acquire a host
    /// lock, or reject the unrelated command/watch Promise with it.
    #[napi]
    pub fn report_callback_exception(&self, env: Env, exception: Unknown<'_>) -> napi::Result<()> {
        // SAFETY: env and exception belong to this live synchronous Node scope.
        // Node may invoke process error handlers here; no Rust lock/lease guard
        // is held. Call unconditionally in release, and pass the original value
        // rather than converting it to a fresh napi::Error/JavaScript Error.
        let status = unsafe { sys::napi_fatal_exception(env.raw(), exception.raw()) };
        if status == sys::Status::napi_ok {
            Ok(())
        } else {
            Err(napi::Error::new(
                napi::Status::from(status),
                "Node listener exception reporting failed",
            ))
        }
    }
}
