//! Language-owned wrappers and conversion only; all Handle commands use Rust.

use crate::{
    NativeSession, binding_error, boundary,
    deferred_conversion::{EvaluationCall, EvaluationResult, EvaluationTarget},
    evaluation_value::{self, EvaluationOutput},
};
use napi::{Env, JsString, Unknown, bindgen_prelude::PromiseRaw};
use napi_derive::napi;
use pwright_node_host::{
    HostError,
    evaluation::FFI_ALLOWANCE,
    ffi_budget::FfiLane,
    handles::{HandleRelease, PendingHandle},
};
use std::sync::{Arc, Mutex};

#[cfg(feature = "retained-file-input")]
pub(crate) mod file_input;
mod properties;
mod publication;
mod wrapper;
pub(crate) use publication::to_napi_value;
pub(crate) use wrapper::{ArgumentReference, read_argument};

enum State {
    Live(Arc<PendingHandle>),
    Disposed(HandleRelease),
}

/// Immutable producer metadata, carried through native-to-language publication.
/// The unselected shape is zero-sized; old producers remain explicitly unbound.
#[derive(Clone, Default)]
pub struct HandlePolicy {
    #[cfg(feature = "retained-file-input")]
    timeout: Option<pwright_node_host::handles::ScopedTimeoutSettings>,
}

#[cfg(not(feature = "retained-file-input"))]
const _: () = assert!(std::mem::size_of::<HandlePolicy>() == 0);

#[napi]
pub struct NativeHandle {
    state: Arc<Mutex<State>>,
    // Keep the binding after explicit disposal without retaining native owners.
    policy: HandlePolicy,
    #[cfg(feature = "retained-file-input")]
    source: pwright_node_host::handles::HandleSourceIdentity,
}

impl NativeHandle {
    pub fn new(value: PendingHandle, policy: HandlePolicy) -> Result<Self, HostError> {
        #[cfg(feature = "retained-file-input")]
        let source = value.source_identity();
        #[cfg(feature = "retained-file-input")]
        if let Some(settings) = &policy.timeout
            && let Err(error) = settings.validate_source(&source)
        {
            // Submit the existing owned cleanup before rejecting publication.
            // PendingHandle Drop is an idempotent fallback, never a leak.
            value.request_dispose();
            return Err(error);
        }
        Ok(Self {
            #[cfg(feature = "retained-file-input")]
            source,
            state: Arc::new(Mutex::new(State::Live(Arc::new(value)))),
            policy,
        })
    }

    /// Client settings metadata only, including after Handle disposal/close.
    /// Native submission must independently revalidate the actual source.
    #[cfg(feature = "retained-file-input")]
    pub(crate) fn file_input_timeout_settings(
        &self,
    ) -> Result<pwright_node_host::handles::TimeoutSettings, HostError> {
        let policy = self.policy.timeout.as_ref().ok_or_else(|| {
            HostError::InvalidArgument(
                "retained file input requires creation-Page timeout settings".into(),
            )
        })?;
        policy.validate_source(&self.source)?;
        Ok(policy.settings().clone())
    }

    fn read(&self) -> napi::Result<Arc<PendingHandle>> {
        self.read_for_evaluation().map_err(binding_error)
    }

    fn read_for_evaluation(&self) -> Result<Arc<PendingHandle>, HostError> {
        match &*self
            .state
            .lock()
            .map_err(|error| HostError::Backend(error.to_string()))?
        {
            State::Live(value) => Ok(value.clone()),
            State::Disposed(_) => Err(HostError::Backend("JSHandle is disposed".into())),
        }
    }

    fn read_for_owner_frame(&self) -> napi::Result<Arc<PendingHandle>> {
        match &*self
            .state
            .lock()
            .map_err(|error| binding_error(HostError::Backend(error.to_string())))?
        {
            State::Live(value) => Ok(value.clone()),
            State::Disposed(_) => Err(binding_error(HostError::TargetClosed)),
        }
    }

    fn release(&self) -> napi::Result<HandleRelease> {
        let mut state = self
            .state
            .lock()
            .map_err(|error| binding_error(HostError::Backend(error.to_string())))?;
        let release = match &*state {
            State::Live(value) => value.request_dispose(),
            State::Disposed(release) => return Ok(release.clone()),
        };
        // No async boundary separates ticket submission from descriptor drop.
        // The disposed state retains only the shared completion receipt;
        // immutable client policy/source metadata remains outside native state.
        *state = State::Disposed(release.clone());
        Ok(release)
    }
}

#[napi]
impl NativeHandle {
    #[napi]
    pub fn bind_wrapper(&self, env: &Env, value: Unknown<'_>) -> napi::Result<()> {
        wrapper::bind(env, value, self.state.clone())
    }

    #[napi]
    pub fn is_element(&self) -> napi::Result<bool> {
        Ok(self.read()?.is_element())
    }

    #[napi]
    pub fn preview(&self) -> napi::Result<String> {
        Ok(self.read()?.initializer().preview.clone())
    }
}

#[napi]
impl NativeSession {
    #[napi]
    pub fn handle_owner_frame<'env>(
        &self,
        env: &'env Env,
        handle: &NativeHandle,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let handle = handle.read_for_owner_frame()?;
        let host = self.host.clone();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.handle_owner_frame(&handle).await
        })
    }

    #[napi]
    pub fn frame_evaluate_handle<'env>(
        &self,
        env: &'env Env,
        frame_id: JsString<'env>,
        source: JsString<'env>,
        is_function: bool,
        argument: Unknown<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let frame = boundary::bounded_string(frame_id, 20, "frame ID")?;
        let source = evaluation_value::source(env, source, is_function)?;
        let input = self.conversions.prepare(env, argument)?;
        self.conversions.schedule(
            env,
            EvaluationCall {
                publications: self.publications.clone(),
                execution,
                host: self.host.clone(),
                target: EvaluationTarget::FrameHandle(frame, Ok(HandlePolicy::default())),
                source,
                is_function,
                input,
            },
        )
    }

    #[napi]
    pub fn handle_json_value<'env>(
        &self,
        env: &'env Env,
        handle: &NativeHandle,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let handle = handle.read()?;
        let host = self.host.clone();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.handle_json_value(&handle).await.map(EvaluationOutput)
        })
    }

    #[napi]
    pub fn handle_evaluate<'env>(
        &self,
        env: &'env Env,
        handle: &NativeHandle,
        source: JsString<'env>,
        is_function: bool,
        argument: Unknown<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let source = evaluation_value::source(env, source, is_function)?;
        let input = self.conversions.prepare(env, argument)?;
        // A disposed target still follows getters and deferred binary conversion.
        // Preserve its error as data until Rust execution begins.
        let handle = handle.read_for_evaluation();
        self.conversions.schedule(
            env,
            EvaluationCall {
                publications: self.publications.clone(),
                execution,
                host: self.host.clone(),
                target: EvaluationTarget::HandleValue(handle),
                source,
                is_function,
                input,
            },
        )
    }

    #[napi]
    pub fn handle_evaluate_handle<'env>(
        &self,
        env: &'env Env,
        handle: &NativeHandle,
        source: JsString<'env>,
        is_function: bool,
        argument: Unknown<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let policy = handle.policy.clone();
        let source = evaluation_value::source(env, source, is_function)?;
        let input = self.conversions.prepare(env, argument)?;
        let handle = handle.read_for_evaluation();
        self.conversions.schedule(
            env,
            EvaluationCall {
                publications: self.publications.clone(),
                execution,
                host: self.host.clone(),
                target: EvaluationTarget::HandleHandle(handle, policy),
                source,
                is_function,
                input,
            },
        )
    }

    #[napi]
    pub fn handle_get_property<'env>(
        &self,
        env: &'env Env,
        handle: &NativeHandle,
        name: JsString<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let name = boundary::bounded_string(
            name,
            pwright_node_host::evaluation::SOURCE_BYTES,
            "property name",
        )?;
        let policy = handle.policy.clone();
        let handle = handle.read()?;
        let host = self.host.clone();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.handle_get_property(&handle, name)
                .await
                .map(|value| EvaluationResult::Handle(value, policy))
        })
    }

    #[napi]
    pub fn handle_dispose<'env>(
        &self,
        env: &'env Env,
        handle: &NativeHandle,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let release = handle.release()?;
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            release.wait().await
        })
    }

    #[napi]
    pub fn handle_get_properties<'env>(
        &self,
        env: &'env Env,
        handle: &NativeHandle,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let policy = handle.policy.clone();
        let handle = handle.read()?;
        let host = self.host.clone();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.handle_get_properties(&handle)
                .await
                .map(|values| properties::PropertiesOutput(values, policy))
        })
    }
}

#[cfg(feature = "retained-file-input")]
#[napi]
impl NativeSession {
    /// Bind creation-Page metadata at initial admission, before any evaluation
    /// argument callback or await. Lookup errors are data until argument
    /// materialization finishes; the produced Handle verifies exact provenance.
    #[napi]
    pub fn frame_evaluate_handle_with_page_timeout<'env>(
        &self,
        env: &'env Env,
        frame_id: JsString<'env>,
        source: JsString<'env>,
        is_function: bool,
        argument: Unknown<'env>,
        page_id: JsString<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let frame = boundary::bounded_string(frame_id, 20, "frame ID")?;
        let page = boundary::bounded_string(page_id, 20, "Page ID")?;
        // The selected session owns canonical Page settings. A closed Page or
        // exhausted metadata budget must not skip source/argument conversion.
        let policy = self
            .file_input_scope
            .timeout_policies()
            .and_then(|policies| policies.page(&self.host, &page))
            .map(|settings| HandlePolicy {
                timeout: Some(settings),
            });
        let source = evaluation_value::source(env, source, is_function)?;
        let input = self.conversions.prepare(env, argument)?;
        self.conversions.schedule(
            env,
            EvaluationCall {
                publications: self.publications.clone(),
                execution,
                host: self.host.clone(),
                target: EvaluationTarget::FrameHandle(frame, policy),
                source,
                is_function,
                input,
            },
        )
    }
}
