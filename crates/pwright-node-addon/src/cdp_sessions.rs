//! Default-off Node-API projection of the Rust-owned Chromium CDP session.

mod lease;
mod validation;

use std::sync::Arc;

use napi::{
    Env, JsString, Unknown, ValueType,
    bindgen_prelude::{FromNapiValue, PromiseRaw, ToNapiValue, TypeName, ValidateNapiValue},
    sys,
};
use napi_derive::napi;
use pwright_node_host::{CdpSessionEventBatch, HostError, HostSession, ffi_budget::FfiLane};
use serde_json::Value;

use crate::{NativeSession, binding_error, boundary};

const MAX_PORTABLE_ID_BYTES: usize = 20;
const MAX_CDP_METHOD_BYTES: usize = 256;

/// Decode a JS BigInt directly to one word, without allocating an unbounded
/// `Vec` for a caller-supplied BigInt before FFI admission.
pub struct UnsignedBigInt(u64);

impl TypeName for UnsignedBigInt {
    fn type_name() -> &'static str {
        "BigInt"
    }

    fn value_type() -> ValueType {
        ValueType::BigInt
    }
}

impl ValidateNapiValue for UnsignedBigInt {}

impl FromNapiValue for UnsignedBigInt {
    unsafe fn from_napi_value(env: sys::napi_env, value: sys::napi_value) -> napi::Result<Self> {
        let mut number = 0;
        let mut lossless = false;
        // SAFETY: N-API supplies live env/value pointers for this conversion;
        // ValidateNapiValue has already required the JS BigInt type.
        let status =
            unsafe { sys::napi_get_value_bigint_uint64(env, value, &mut number, &mut lossless) };
        if status != sys::Status::napi_ok || !lossless {
            return Err(binding_error(HostError::InvalidArgument(
                "CDP token/generation must be an unsigned 64-bit BigInt".into(),
            )));
        }
        Ok(Self(number))
    }
}

struct MeasuredCdpResult {
    value: Value,
    bytes: u64,
}

impl MeasuredCdpResult {
    fn new(value: Value) -> Result<Self, HostError> {
        let (value, bytes) = validation::measured_result(value)?;
        Ok(Self { value, bytes })
    }

    fn foreign_copy_bytes(&self) -> u64 {
        self.bytes
    }
}

impl ToNapiValue for MeasuredCdpResult {
    unsafe fn to_napi_value(env: sys::napi_env, result: Self) -> napi::Result<sys::napi_value> {
        // SAFETY: napi invokes this only in the live JS settlement callback,
        // after the measured foreign-copy allowance has been reserved.
        unsafe { Value::to_napi_value(env, result.value) }
    }
}

/// Owns an attached child until the JS facade installs its wrapper.
/// Dropping an unread Promise result schedules native compensation.
#[napi]
pub struct NativeCdpSessionLease {
    lease: lease::ClaimLease,
}

#[napi]
impl NativeCdpSessionLease {
    #[napi]
    pub fn token(&self) -> u64 {
        self.lease.token()
    }

    #[napi]
    pub fn claim(&mut self) -> napi::Result<u64> {
        self.lease.claim().map_err(binding_error)
    }

    #[napi]
    pub fn abort(&mut self) {
        self.lease.abort();
    }
}

impl NativeCdpSessionLease {
    fn from_token(host: Arc<HostSession>, token: u64) -> Self {
        let runtime = tokio::runtime::Handle::current();
        Self {
            lease: lease::ClaimLease::new(token, move || {
                drop(runtime.spawn(async move {
                    if let Err(error) = host.cdp_session_cleanup_unclaimed(token).await {
                        tracing::error!(token, %error, "unclaimed Node CDP session cleanup failed");
                    }
                }));
            }),
        }
    }
}

/// A native event claim with Drop rollback of its unacknowledged suffix.
#[napi]
pub struct NativeCdpEventBatch {
    batch: CdpSessionEventBatch,
}

#[napi]
impl NativeCdpEventBatch {
    #[napi]
    pub fn peek(&self) -> napi::Result<Option<(String, Value)>> {
        self.batch
            .peek()
            .map_err(binding_error)?
            .map(|event| {
                if !event.params.is_object() {
                    return Err(binding_error(HostError::Backend(
                        "CDP event parameters are not an object".into(),
                    )));
                }
                Ok((event.method, event.params))
            })
            .transpose()
    }

    #[napi]
    pub fn ack_next(&self) -> napi::Result<()> {
        self.batch.ack_next().map_err(binding_error)
    }

    #[napi]
    pub fn rollback(&self) -> napi::Result<()> {
        self.batch.rollback().map_err(binding_error)
    }
}

#[napi]
impl NativeSession {
    #[napi]
    pub fn cdp_session_create<'env>(
        &self,
        env: &'env Env,
        context_id: JsString<'_>,
        page_id: JsString<'_>,
        frame_id: Option<JsString<'_>>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let context_id = boundary::bounded_string(context_id, MAX_PORTABLE_ID_BYTES, "Context ID")?;
        let page_id = boundary::bounded_string(page_id, MAX_PORTABLE_ID_BYTES, "Page ID")?;
        let frame_id = frame_id
            .map(|value| boundary::bounded_string(value, MAX_PORTABLE_ID_BYTES, "Frame ID"))
            .transpose()?;
        let host = Arc::clone(&self.host);
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            let token = host
                .cdp_session_create(&context_id, &page_id, frame_id.as_deref())
                .await?;
            // No suspension point separates native attachment from its RAII
            // owner, including when a settled Promise result goes unread.
            Ok(NativeCdpSessionLease::from_token(host, token))
        })
    }

    #[napi]
    pub fn cdp_session_send<'env>(
        &self,
        env: &'env Env,
        token: UnsignedBigInt,
        method: JsString<'_>,
        params_json: JsString<'_>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::READ_ALLOWANCE)
            .map_err(binding_error)?;
        let method = boundary::bounded_string(method, MAX_CDP_METHOD_BYTES, "CDP method")?;
        let params_json = boundary::bounded_string(
            params_json,
            validation::MAX_CDP_PARAMS_JSON_BYTES,
            "CDP params JSON",
        )?;
        let params = validation::parse_params_json(&params_json).map_err(binding_error)?;
        let token = token.0;
        let host = Arc::clone(&self.host);
        boundary::spawn_published_sized(
            env,
            execution,
            self.publications.clone(),
            async move {
                let result = host.cdp_session_send(token, &method, params).await?;
                MeasuredCdpResult::new(result)
            },
            MeasuredCdpResult::foreign_copy_bytes,
        )
    }

    #[napi]
    pub fn cdp_session_claim<'env>(
        &self,
        env: &'env Env,
        token: UnsignedBigInt,
        max_events: u32,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let token = token.0;
        let host = Arc::clone(&self.host);
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            let max_events = usize::try_from(max_events).map_err(|_| {
                HostError::InvalidArgument("CDP event claim count exceeds platform limit".into())
            })?;
            let batch = host.cdp_session_claim(token, max_events).await?;
            Ok(NativeCdpEventBatch { batch })
        })
    }

    #[napi]
    pub fn next_cdp_session_event_generation<'env>(
        &self,
        env: &'env Env,
        after: UnsignedBigInt,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Watch, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let after = after.0;
        let host = Arc::clone(&self.host);
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.next_cdp_session_event_generation(after).await
        })
    }

    #[napi]
    pub fn cdp_session_detach<'env>(
        &self,
        env: &'env Env,
        token: UnsignedBigInt,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let token = token.0;
        let host: Arc<HostSession> = Arc::clone(&self.host);
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.cdp_session_detach(token).await
        })
    }
}

#[cfg(feature = "browser-cdp-session")]
#[napi]
impl NativeSession {
    #[napi]
    pub fn cdp_browser_session_create<'env>(
        &self,
        env: &'env Env,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let host = Arc::clone(&self.host);
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            let token = host.cdp_browser_session_create().await?;
            Ok(NativeCdpSessionLease::from_token(host, token))
        })
    }
}
