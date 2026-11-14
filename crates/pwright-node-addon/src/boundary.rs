//! Borrow JS arguments until admission, and keep tickets through JS conversion.

use std::future::Future;
#[cfg(feature = "handles")]
use std::sync::Arc;

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
    spawn_published_sized(env, execution, port, future, |_| 0)
}

#[cfg(feature = "handles")]
pub fn spawn_published_wait<'env, T: Send + ToNapiValue + 'static>(
    env: &'env Env,
    execution: FfiExecution<Published<T>>,
    port: crate::publication_port::PublicationPort,
    wait_permit: crate::wait_admission::WaitPermit,
    future: impl Future<Output = Result<T, HostError>> + Send + 'static,
) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
    let wait_permit = Arc::new(wait_permit);
    let running_permit = Arc::clone(&wait_permit);
    spawn_published_sized_guarded(
        env,
        execution,
        port,
        wait_permit,
        async move {
            let outcome = future.await;
            drop(running_permit);
            outcome
        },
        |_| 0,
    )
}

pub fn spawn_published_sized<'env, T: Send + ToNapiValue + 'static>(
    env: &'env Env,
    execution: FfiExecution<Published<T>>,
    port: crate::publication_port::PublicationPort,
    future: impl Future<Output = Result<T, HostError>> + Send + 'static,
    additional_result_bytes: impl Fn(&T) -> u64 + Send + 'static,
) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
    spawn_published_sized_guarded(env, execution, port, (), future, additional_result_bytes)
}

fn spawn_published_sized_guarded<'env, T: Send + ToNapiValue + 'static, G: Send + 'static>(
    env: &'env Env,
    execution: FfiExecution<Published<T>>,
    port: crate::publication_port::PublicationPort,
    settlement_guard: G,
    future: impl Future<Output = Result<T, HostError>> + Send + 'static,
    additional_result_bytes: impl Fn(&T) -> u64 + Send + 'static,
) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
    let publications = port.publications();
    let reservation = execution.reservation();
    spawn_with_env_guarded(
        env,
        execution,
        async move {
            let outcome = match future.await {
                Ok(value) => reservation
                    .reserve_additional(additional_result_bytes(&value))
                    .map(|()| value),
                Err(error) => Err(error),
            };
            Ok((outcome, publications.fence()?))
        },
        settlement_guard,
        move |env, (outcome, fence)| {
            port.publish(env, fence)?;
            outcome.map_err(binding_error)
        },
    )
}

pub fn spawn_with_env<'env, T: Send + 'static, R: ToNapiValue>(
    env: &'env Env,
    execution: FfiExecution<T>,
    future: impl Future<Output = Result<T, HostError>> + Send + 'static,
    convert: impl FnOnce(&Env, T) -> napi::Result<R> + 'static,
) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
    spawn_with_env_guarded(env, execution, future, (), convert)
}

fn spawn_with_env_guarded<'env, T: Send + 'static, R: ToNapiValue, G: Send + 'static>(
    env: &'env Env,
    mut execution: FfiExecution<T>,
    future: impl Future<Output = Result<T, HostError>> + Send + 'static,
    settlement_guard: G,
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
        let result = ticket
            .deliver(|outcome| convert(env, outcome.map_err(binding_error)?)?.into_unknown(env))
            .map_err(binding_error)?;
        // The wait guard outlives FFI Command's settlement, including JS
        // conversion. It is also dropped if callback registration fails.
        drop(settlement_guard);
        result
    })
}
