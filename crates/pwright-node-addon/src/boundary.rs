//! Borrow JS arguments until admission, and keep tickets through JS conversion.

use std::future::Future;

use napi::{
    Env, JsString, Unknown,
    bindgen_prelude::{PromiseRaw, ToNapiValue},
};
use pwright_node_host::{FramePublicationFence, HostError, ffi_budget::FfiExecution};

use crate::binding_error;

pub const SMALL_ALLOWANCE: u64 = 32 * 1024;
pub const READ_ALLOWANCE: u64 = 1024 * 1024 + 4096;
pub const NAVIGATION_ALLOWANCE: u64 = 256 * 1024;

pub fn bounded_string(value: JsString<'_>, max_bytes: usize, field: &str) -> napi::Result<String> {
    if value.utf8_len()? > max_bytes {
        return Err(binding_error(HostError::InvalidArgument(format!(
            "{field} exceeds {max_bytes} UTF-8 bytes"
        ))));
    }
    value.into_utf8()?.into_owned()
}

pub type Published<T> = (Result<T, HostError>, FramePublicationFence);

pub fn spawn_published<'env, T: Send + ToNapiValue + 'static>(
    env: &'env Env,
    execution: FfiExecution<Published<T>>,
    port: crate::publication_port::PublicationPort,
    future: impl Future<Output = Result<T, HostError>> + Send + 'static,
) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
    let publications = port.publications();
    spawn_with_env(
        env,
        execution,
        async move {
            let outcome = future.await;
            Ok((outcome, publications.fence()?))
        },
        move |env, (outcome, fence)| {
            port.publish(env, fence)?;
            outcome.map_err(binding_error)
        },
    )
}

pub fn spawn_with_env<'env, T: Send + 'static, R: ToNapiValue>(
    env: &'env Env,
    mut execution: FfiExecution<T>,
    future: impl Future<Output = Result<T, HostError>> + Send + 'static,
    convert: impl FnOnce(&Env, T) -> napi::Result<R> + 'static,
) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
    execution
        .reserve_future(std::mem::size_of_val(&future))
        .map_err(binding_error)?;
    // Keep the embedded attach future out of napi's by-value generic layers.
    // V8 callbacks have a much smaller stack than Tokio's runtime threads.
    let future = Box::pin(future);
    let execution = Box::pin(execution.run(future));
    // Even an operation error is an owned, budgeted settlement. Returning
    // Err from the worker future would release its permit before JS delivery.
    env.spawn_future_with_callback(async move { Ok(execution.await) }, move |env, ticket| {
        ticket
            .deliver(|outcome| convert(env, outcome.map_err(binding_error)?)?.into_unknown(env))
            .map_err(binding_error)?
    })
}
