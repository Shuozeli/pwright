//! Language-owned wrappers and conversion only; all Handle commands use Rust.

#[cfg(all(feature = "element-handle-dom", feature = "locator-bounding-box"))]
use crate::locator::LocatorBoundingBoxOutput;
use crate::{
    NativeSession, binding_error, boundary,
    deferred_conversion::{EvaluationCall, EvaluationResult, EvaluationTarget},
    evaluation_value::{self, EvaluationOutput},
};
#[cfg(feature = "element-handle-dom")]
use napi::bindgen_prelude::Either;
#[cfg(all(feature = "element-handle-dom", feature = "touchscreen-input"))]
use napi::bindgen_prelude::Object;
use napi::{Env, JsString, Unknown, bindgen_prelude::PromiseRaw};
use napi_derive::napi;
#[cfg(feature = "element-handle-dom")]
use pwright_node_host::element_handle_dom::{ElementHandleDomRead, ElementHandleDomValue};
#[cfg(feature = "element-handle-wait-for-selector")]
use pwright_node_host::element_handle_wait_for_selector::ElementHandleSelectorWaitState;
use pwright_node_host::{
    HostError,
    evaluation::FFI_ALLOWANCE,
    ffi_budget::FfiLane,
    handles::{HandleRelease, PendingHandle},
};
use std::sync::{Arc, Mutex};

#[cfg(any(
    feature = "element-handle-dom",
    feature = "element-handle-select-option-value",
    feature = "element-handle-wait-for-selector"
))]
const SELECTOR_BYTES: usize = 64 * 1024;

#[cfg(feature = "retained-file-input")]
pub(crate) mod file_input;
mod properties;
mod publication;
mod wrapper;
#[cfg(feature = "element-handle-dom")]
pub(crate) use properties::HandlesOutput;
pub(crate) use publication::to_napi_value;
pub(crate) use wrapper::{ArgumentReference, read_argument};

enum State {
    Live(Arc<PendingHandle>),
    Disposed(HandleRelease),
}

#[cfg(feature = "wait-for-function")]
#[napi(object)]
pub struct WaitForFunctionCallOptions {
    pub is_function: bool,
    pub polling_ms: u32,
    pub timeout_ms: Option<u32>,
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
    pub fn handle_content_frame<'env>(
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
            host.handle_content_frame(&handle).await
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

#[cfg(feature = "wait-for-function")]
#[napi]
impl NativeSession {
    #[napi]
    pub fn frame_wait_for_function<'env>(
        &self,
        env: &'env Env,
        frame_id: JsString<'env>,
        page_id: JsString<'env>,
        source: JsString<'env>,
        argument: Unknown<'env>,
        options: WaitForFunctionCallOptions,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let frame = boundary::bounded_string(frame_id, 20, "frame ID")?;
        let page = boundary::bounded_string(page_id, 20, "page ID")?;
        let source = evaluation_value::source(env, source, options.is_function)?;
        let timeout_ms = options
            .timeout_ms
            .map_or_else(|| self.host.page_default_timeout(&page, false), Ok)
            .map_err(binding_error)?;
        let input = self.conversions.prepare(env, argument)?;
        let polling = if options.polling_ms == 0 {
            pwright_node_host::wait_for_function::Polling::AnimationFrame
        } else {
            pwright_node_host::wait_for_function::Polling::Interval(options.polling_ms)
        };
        self.conversions.schedule(
            env,
            EvaluationCall {
                publications: self.publications.clone(),
                execution,
                host: self.host.clone(),
                target: EvaluationTarget::FrameWaitForFunction(frame, polling, timeout_ms),
                source,
                is_function: options.is_function,
                input,
            },
        )
    }
}

#[cfg(feature = "frame-element")]
#[napi]
impl NativeSession {
    /// Return the exact DOM owner in the parent Frame's main world. The host
    /// retains native acquisition and publication through cancellation.
    #[napi]
    pub fn frame_element<'env>(
        &self,
        env: &'env Env,
        frame_id: JsString<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let frame = boundary::bounded_string(frame_id, 20, "frame ID")?;
        let host = self.host.clone();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.frame_prepare_element(&frame)
                .await
                .map(|value| EvaluationResult::Handle(value, HandlePolicy::default()))
        })
    }
}

#[cfg(feature = "page-bindings")]
#[napi]
impl NativeSession {
    #[napi]
    pub fn take_page_binding_handle<'env>(
        &self,
        env: &'env Env,
        call_id: JsString<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let call_id = boundary::bounded_string(call_id, 20, "binding call ID")?;
        let call_id = pwright_node_host::HostSession::parse_page_binding_call_id(&call_id)
            .map_err(binding_error)?;
        let host = self.host.clone();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.take_page_binding_handle(call_id)
                .await
                .map(|value| EvaluationResult::Handle(value, HandlePolicy::default()))
        })
    }
}

#[cfg(feature = "element-handle-scroll-into-view")]
#[napi]
impl NativeSession {
    #[napi]
    pub fn element_handle_scroll_into_view_if_needed<'env>(
        &self,
        env: &'env Env,
        handle: &NativeHandle,
        timeout_ms: Option<u32>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let handle = handle.read()?;
        let host = self.host.clone();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.element_handle_scroll_into_view_if_needed(&handle, timeout_ms)
                .await
        })
    }
}

#[cfg(feature = "element-handle-wait-for-selector")]
#[napi]
impl NativeSession {
    #[napi]
    pub fn element_handle_wait_for_selector<'env>(
        &self,
        env: &'env Env,
        handle: &NativeHandle,
        selector: JsString<'env>,
        state: JsString<'env>,
        strict: bool,
        timeout_ms: Option<u32>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let wait_permit = self.wait_admissions.admit().map_err(binding_error)?;
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let draft = self
            .queries
            .prepare_leaf(selector.utf8_len()?)
            .map_err(binding_error)?;
        let selector = boundary::bounded_string(selector, SELECTOR_BYTES, "selector")?;
        let query = draft.element_handle(selector).map_err(binding_error)?;
        let state = boundary::bounded_string(state, 16, "ElementHandle wait state")?;
        let state = state
            .parse::<ElementHandleSelectorWaitState>()
            .map_err(binding_error)?;
        let policy = handle.policy.clone();
        let handle = handle.read()?;
        let host = self.host.clone();
        boundary::spawn_published_wait(
            env,
            execution,
            self.publications.clone(),
            wait_permit,
            async move {
                host.element_handle_wait_for_selector(&handle, query, state, strict, timeout_ms)
                    .await
                    .map(|value| match value {
                        Some(value) => EvaluationResult::Handle(value, policy),
                        None => EvaluationResult::Value(EvaluationOutput(
                            pwright_node_host::evaluation::EvaluationValue::Null,
                        )),
                    })
            },
        )
    }
}

#[cfg(feature = "element-handle-select-option-value")]
#[napi]
impl NativeSession {
    #[napi]
    pub fn element_handle_select_option_value<'env>(
        &self,
        env: &'env Env,
        handle: &NativeHandle,
        value: JsString<'env>,
        timeout_ms: Option<u32>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let wait_permit = self.wait_admissions.admit().map_err(binding_error)?;
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let value = boundary::bounded_string(value, SELECTOR_BYTES, "selectOption value")?;
        let handle = handle.read()?;
        let host = self.host.clone();
        boundary::spawn_published_wait(
            env,
            execution,
            self.publications.clone(),
            wait_permit,
            async move {
                host.element_handle_select_option_value(&handle, value, timeout_ms)
                    .await
            },
        )
    }
}

#[cfg(feature = "element-handle-dom")]
#[napi]
impl NativeSession {
    #[napi]
    pub fn element_handle_preview<'env>(
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
            host.element_handle_preview(&handle).await
        })
    }

    #[napi]
    pub fn element_handle_inner_text<'env>(
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
            host.element_handle_inner_text(&handle).await
        })
    }

    #[napi]
    pub fn element_handle_text_content<'env>(
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
            host.element_handle_text_content(&handle).await
        })
    }

    #[napi]
    pub fn element_handle_focus<'env>(
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
            host.element_handle_focus(&handle).await
        })
    }

    #[napi]
    pub fn element_handle_dom_read<'env>(
        &self,
        env: &'env Env,
        handle: &NativeHandle,
        operation: JsString<'env>,
        attribute: Option<JsString<'env>>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let operation = boundary::bounded_string(operation, 32, "ElementHandle DOM operation")?;
        let operation = operation
            .parse::<ElementHandleDomRead>()
            .map_err(binding_error)?;
        let attribute = attribute
            .map(|name| boundary::bounded_string(name, SELECTOR_BYTES, "attribute name"))
            .transpose()?;
        let handle = handle.read()?;
        let host = self.host.clone();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            let value = host
                .element_handle_dom_read(&handle, operation, attribute)
                .await?;
            Ok(match value {
                ElementHandleDomValue::Bool(value) => Either::A(value),
                ElementHandleDomValue::Text(value) => Either::B(value),
            })
        })
    }

    #[napi]
    pub fn element_handle_query_selector<'env>(
        &self,
        env: &'env Env,
        handle: &NativeHandle,
        selector: JsString<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let draft = self
            .queries
            .prepare_leaf(selector.utf8_len()?)
            .map_err(binding_error)?;
        let selector = boundary::bounded_string(selector, SELECTOR_BYTES, "selector")?;
        let query = draft.element_handle(selector).map_err(binding_error)?;
        let policy = handle.policy.clone();
        let handle = handle.read()?;
        let host = self.host.clone();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.element_handle_query_selector(&handle, query)
                .await
                .map(|value| value.map(|value| EvaluationResult::Handle(value, policy)))
        })
    }

    #[napi]
    pub fn element_handle_query_selector_all<'env>(
        &self,
        env: &'env Env,
        handle: &NativeHandle,
        selector: JsString<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let draft = self
            .queries
            .prepare_leaf(selector.utf8_len()?)
            .map_err(binding_error)?;
        let selector = boundary::bounded_string(selector, SELECTOR_BYTES, "selector")?;
        let query = draft.element_handle(selector).map_err(binding_error)?;
        let policy = handle.policy.clone();
        let handle = handle.read()?;
        let host = self.host.clone();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.element_handle_query_selector_all(&handle, query)
                .await
                .map(|values| properties::HandlesOutput(values, policy))
        })
    }

    #[napi]
    pub fn element_handle_eval_on_selector_all<'env>(
        &self,
        env: &'env Env,
        handle: &NativeHandle,
        selector: JsString<'env>,
        source: JsString<'env>,
        is_function: bool,
        argument: Unknown<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let draft = self
            .queries
            .prepare_leaf(selector.utf8_len()?)
            .map_err(binding_error)?;
        let selector = boundary::bounded_string(selector, SELECTOR_BYTES, "selector")?;
        let query = draft.element_handle(selector).map_err(binding_error)?;
        let source = evaluation_value::source(env, source, is_function)?;
        let input = self.conversions.prepare(env, argument)?;
        let handle = handle.read_for_evaluation();
        self.conversions.schedule(
            env,
            EvaluationCall {
                publications: self.publications.clone(),
                execution,
                host: self.host.clone(),
                target: EvaluationTarget::HandleElementQueryAll(handle, query),
                source,
                is_function,
                input,
            },
        )
    }
}

#[cfg(all(feature = "element-handle-dom", feature = "touchscreen-input"))]
#[napi]
impl NativeSession {
    #[napi]
    pub fn element_handle_tap<'env>(
        &self,
        env: &'env Env,
        handle: &NativeHandle,
        timeout_ms: u32,
        options: Object<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let options = crate::locator::actions::tap_options(&options)?;
        let handle = handle.read()?;
        let host = self.host.clone();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.element_handle_tap(&handle, options, timeout_ms).await
        })
    }
}

#[cfg(all(feature = "element-handle-dom", feature = "locator-bounding-box"))]
#[napi]
impl NativeSession {
    #[napi]
    pub fn element_handle_bounding_box<'env>(
        &self,
        env: &'env Env,
        handle: &NativeHandle,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, boundary::SMALL_ALLOWANCE)
            .map_err(binding_error)?;
        let handle = handle.read()?;
        let host = self.host.clone();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.element_handle_bounding_box(&handle)
                .await
                .map(|value| value.map(LocatorBoundingBoxOutput::from))
        })
    }
}

#[cfg(all(feature = "element-handle-dom", feature = "locator-dom-actions"))]
#[napi]
impl NativeSession {
    #[napi]
    pub fn element_handle_dispatch_event<'env>(
        &self,
        env: &'env Env,
        handle: &NativeHandle,
        event_type: JsString<'env>,
        event_init: Unknown<'env>,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let execution = self
            .ffi
            .admit(FfiLane::Command, FFI_ALLOWANCE)
            .map_err(binding_error)?;
        let event_type = boundary::bounded_string(
            event_type,
            pwright_node_host::locator::MAX_EVENT_TYPE_BYTES,
            "event type",
        )?;
        let event_init = self
            .conversions
            .prepare(env, event_init)?
            .materialize_owned(env)?;
        let handle = handle.read()?;
        let host = self.host.clone();
        boundary::spawn_published(env, execution, self.publications.clone(), async move {
            host.element_handle_dispatch_event_owned(&handle, event_type, event_init)
                .await
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
