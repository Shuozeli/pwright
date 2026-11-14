//! Node-thread conversion staging, distinct from Rust execution/result ownership.

use std::{cell::RefCell, collections::BTreeMap, ptr, rc::Rc, sync::Arc};

use napi::{
    Env, JsValue, Unknown,
    bindgen_prelude::{FnArgs, Function, PromiseRaw, ToNapiValue},
    sys,
};
use napi_derive::napi;
use pwright_node_host::evaluation::EvaluateParams;
use pwright_node_host::{HostError, HostSession, ffi_budget::FfiExecution};

use crate::{
    binding_error, boundary,
    evaluation_value::{self, EvaluationOutput, PreparedInput, status},
    node_reference::NodeReference,
};

type Execution = FfiExecution<boundary::Published<EvaluationResult>>;

pub enum EvaluationTarget {
    FrameValue(String),
    #[cfg(feature = "wait-for-function")]
    FrameWaitForFunction(String, pwright_node_host::wait_for_function::Polling, u32),
    #[cfg(feature = "page-bindings")]
    PageBindingResult(u64),
    #[cfg(feature = "handles")]
    FrameHandle(String, Result<crate::handles::HandlePolicy, HostError>),
    #[cfg(feature = "handles")]
    HandleValue(Result<Arc<pwright_node_host::handles::PendingHandle>, HostError>),
    #[cfg(feature = "element-handle-dom")]
    HandleElementQueryAll(
        Result<Arc<pwright_node_host::handles::PendingHandle>, HostError>,
        pwright_node_host::locator_query::ElementHandleQuery,
    ),
    #[cfg(feature = "handles")]
    HandleHandle(
        Result<Arc<pwright_node_host::handles::PendingHandle>, HostError>,
        crate::handles::HandlePolicy,
    ),
}

pub enum EvaluationResult {
    Value(EvaluationOutput),
    #[cfg(feature = "handles")]
    Handle(
        pwright_node_host::handles::PendingHandle,
        crate::handles::HandlePolicy,
    ),
}

impl ToNapiValue for EvaluationResult {
    unsafe fn to_napi_value(env: sys::napi_env, value: Self) -> napi::Result<sys::napi_value> {
        // The FFI execution still covers all native-to-JS publication work.
        match value {
            Self::Value(value) => unsafe { EvaluationOutput::to_napi_value(env, value) },
            #[cfg(feature = "handles")]
            Self::Handle(value, policy) => unsafe {
                crate::handles::to_napi_value(
                    env,
                    crate::handles::NativeHandle::new(value, policy).map_err(binding_error)?,
                )
            },
        }
    }
}

impl EvaluationResult {
    fn additional_ffi_reservation(&self) -> u64 {
        #[cfg(feature = "large-evaluation-results")]
        if let Self::Value(EvaluationOutput(
            pwright_node_host::evaluation::EvaluationValue::String(value),
        )) = self
            && value.len() > pwright_node_host::evaluation::STRING_BYTES
        {
            return pwright_node_host::evaluation::LARGE_RESULT_FFI_ALLOWANCE;
        }
        0
    }
}

#[napi(object)]
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ConversionMetrics {
    pub pending: u32,
    pub binary_references: u32,
    pub backing_bytes: u32,
}

pub struct EvaluationCall {
    pub publications: crate::publication_port::PublicationPort,
    pub execution: Execution,
    pub host: Arc<HostSession>,
    pub target: EvaluationTarget,
    pub source: String,
    pub is_function: bool,
    pub input: PreparedInput,
}

impl EvaluationCall {
    fn start<'env>(self, env: &'env Env) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        #[cfg(feature = "handles")]
        if self.input.has_handles() {
            return self.start_owned(env);
        }
        let argument = self.input.materialize(env)?;
        let host = self.host;
        boundary::spawn_published_sized(
            env,
            self.execution,
            self.publications,
            async move {
                match self.target {
                    EvaluationTarget::FrameValue(frame) => {
                        let params = if self.is_function {
                            EvaluateParams::function(self.source, argument)
                        } else {
                            EvaluateParams::expression(self.source)
                        };
                        host.frame_evaluate(&frame, params)
                            .await
                            .map(EvaluationOutput)
                            .map(EvaluationResult::Value)
                    }
                    #[cfg(feature = "wait-for-function")]
                    EvaluationTarget::FrameWaitForFunction(frame, polling, timeout_ms) => host
                        .frame_wait_for_function(
                            &frame,
                            self.source,
                            self.is_function,
                            argument,
                            polling,
                            timeout_ms,
                        )
                        .await
                        .map(|value| EvaluationResult::Handle(value, Default::default())),
                    #[cfg(feature = "page-bindings")]
                    EvaluationTarget::PageBindingResult(call_id) => host
                        .deliver_page_binding_value(call_id, argument)
                        .await
                        .map(|()| {
                            EvaluationResult::Value(EvaluationOutput(
                                pwright_node_host::evaluation::EvaluationValue::Undefined,
                            ))
                        }),
                    #[cfg(feature = "handles")]
                    EvaluationTarget::FrameHandle(frame, policy) => {
                        use pwright_node_host::handles::EvaluateHandleParams;
                        let policy = policy?;
                        let params = if self.is_function {
                            EvaluateHandleParams::function(self.source, argument)
                        } else {
                            EvaluateHandleParams::expression(self.source)
                        };
                        host.frame_prepare_handle(&frame, params)
                            .await
                            .map(|value| EvaluationResult::Handle(value, policy))
                    }
                    #[cfg(feature = "handles")]
                    EvaluationTarget::HandleHandle(handle, policy) => {
                        let handle = handle?;
                        let params = if self.is_function {
                            pwright_node_host::handles::EvaluateHandleParams::function(
                                self.source,
                                argument,
                            )
                            .into()
                        } else {
                            pwright_node_host::handles::HandleEvaluationProgram::expression(
                                self.source,
                                argument.into(),
                            )
                        };
                        host.handle_prepare_program(&handle, params)
                            .await
                            .map(|value| EvaluationResult::Handle(value, policy))
                    }
                    #[cfg(feature = "handles")]
                    EvaluationTarget::HandleValue(handle) => {
                        let handle = handle?;
                        if !self.is_function {
                            let params =
                                pwright_node_host::handles::HandleEvaluationProgram::expression(
                                    self.source,
                                    argument.into(),
                                );
                            return host
                                .handle_evaluate_program(&handle, params)
                                .await
                                .map(EvaluationOutput)
                                .map(EvaluationResult::Value);
                        }
                        let params = pwright_node_host::handles::HandleEvaluateParams::function(
                            self.source,
                            argument,
                        );
                        host.handle_evaluate(&handle, params)
                            .await
                            .map(EvaluationOutput)
                            .map(EvaluationResult::Value)
                    }
                    #[cfg(feature = "element-handle-dom")]
                    EvaluationTarget::HandleElementQueryAll(handle, query) => {
                        let handle = handle?;
                        let argument = pwright_node_host::handles::OwnedHandleArgument::from_slots(
                            argument,
                            Vec::new(),
                        )?;
                        host.element_handle_eval_on_selector_all(
                            &handle,
                            query,
                            self.source,
                            self.is_function,
                            argument,
                        )
                        .await
                        .map(EvaluationOutput)
                        .map(EvaluationResult::Value)
                    }
                }
            },
            EvaluationResult::additional_ffi_reservation,
        )
    }

    #[cfg(feature = "handles")]
    fn start_owned<'env>(self, env: &'env Env) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        let argument = self.input.materialize_owned(env)?;
        let host = self.host;
        boundary::spawn_published_sized(
            env,
            self.execution,
            self.publications,
            async move {
                match self.target {
                    EvaluationTarget::FrameValue(frame) => host
                        .frame_evaluate_owned(&frame, self.source, self.is_function, argument)
                        .await
                        .map(EvaluationOutput)
                        .map(EvaluationResult::Value),
                    #[cfg(feature = "wait-for-function")]
                    EvaluationTarget::FrameWaitForFunction(frame, polling, timeout_ms) => host
                        .frame_wait_for_function_owned(
                            &frame,
                            self.source,
                            self.is_function,
                            argument,
                            polling,
                            timeout_ms,
                        )
                        .await
                        .map(|value| EvaluationResult::Handle(value, Default::default())),
                    #[cfg(feature = "page-bindings")]
                    EvaluationTarget::PageBindingResult(call_id) => host
                        .deliver_page_binding_owned(call_id, argument)
                        .await
                        .map(|()| {
                            EvaluationResult::Value(EvaluationOutput(
                                pwright_node_host::evaluation::EvaluationValue::Undefined,
                            ))
                        }),
                    EvaluationTarget::FrameHandle(frame, policy) => {
                        let policy = policy?;
                        host.frame_prepare_handle_owned(
                            &frame,
                            self.source,
                            self.is_function,
                            argument,
                        )
                        .await
                        .map(|value| EvaluationResult::Handle(value, policy))
                    }
                    EvaluationTarget::HandleValue(handle) => {
                        let handle = handle?;
                        host.handle_evaluate_program_owned(
                            &handle,
                            self.source,
                            self.is_function,
                            argument,
                        )
                        .await
                        .map(EvaluationOutput)
                        .map(EvaluationResult::Value)
                    }
                    EvaluationTarget::HandleHandle(handle, policy) => {
                        let handle = handle?;
                        host.handle_prepare_owned(&handle, self.source, self.is_function, argument)
                            .await
                            .map(|value| EvaluationResult::Handle(value, policy))
                    }
                    #[cfg(feature = "element-handle-dom")]
                    EvaluationTarget::HandleElementQueryAll(handle, query) => {
                        let handle = handle?;
                        host.element_handle_eval_on_selector_all(
                            &handle,
                            query,
                            self.source,
                            self.is_function,
                            argument,
                        )
                        .await
                        .map(EvaluationOutput)
                        .map(EvaluationResult::Value)
                    }
                }
            },
            EvaluationResult::additional_ffi_reservation,
        )
    }
}

struct Pending {
    call: EvaluationCall,
    deferred: sys::napi_deferred,
}

#[derive(Default)]
struct State {
    stopped: bool,
    next_id: u32,
    pending: BTreeMap<u32, Pending>,
    scheduler: Option<NodeReference>,
    callback: Option<NodeReference>,
    maximum: Option<NodeReference>,
}

/// Rc/RefCell and NodeReference deliberately make this registry non-Send.
/// Only a prepared call's materialized owned tree can enter the safe host.
#[derive(Clone)]
pub struct DeferredConversions(Rc<RefCell<State>>);

impl DeferredConversions {
    pub fn new(env: &Env, scheduler: Function<'_>, maximum: Function<'_>) -> napi::Result<Self> {
        let registry = Self(Rc::new(RefCell::new(State {
            scheduler: Some(NodeReference::new(env, scheduler.to_unknown())?),
            maximum: Some(NodeReference::new(env, maximum.to_unknown())?),
            ..State::default()
        })));
        let weak = Rc::downgrade(&registry.0);
        let callback: Function<'_, u32, ()> =
            env.create_function_from_closure("pwrightMaterializeEvaluation", move |ctx| {
                if let Some(registry) = weak.upgrade() {
                    DeferredConversions(registry).dispatch(ctx.env, ctx.get::<u32>(0)?)?;
                }
                Ok(())
            })?;
        registry.0.borrow_mut().callback = Some(NodeReference::new(env, callback.to_unknown())?);
        Ok(registry)
    }

    pub fn prepare<'env>(
        &self,
        env: &'env Env,
        input: Unknown<'env>,
    ) -> napi::Result<PreparedInput> {
        let maximum = self
            .0
            .borrow()
            .maximum
            .as_ref()
            .ok_or_else(closed)?
            .get(env)?;
        // SAFETY: this reference came from the validated constructor Function.
        let maximum = unsafe { maximum.cast::<Function<'_, (), f64>>()? };
        // No registry borrow spans getters or other user-observable conversion.
        evaluation_value::read(env, input, maximum)
    }

    pub fn schedule<'env>(
        &self,
        env: &'env Env,
        call: EvaluationCall,
    ) -> napi::Result<PromiseRaw<'env, Unknown<'env>>> {
        if call.input.binary_references() == 0 {
            return call.start(env);
        }
        let (id, scheduler, callback) = {
            let mut state = self.0.borrow_mut();
            if state.stopped {
                return Err(closed());
            }
            if state.pending.len() >= 32 {
                return Err(binding_error(HostError::ResourceExhausted(
                    "Node conversion capacity exceeded".into(),
                )));
            }
            let id = state.next_id.checked_add(1).ok_or_else(|| {
                binding_error(HostError::ResourceExhausted(
                    "Node conversion IDs exhausted".into(),
                ))
            })?;
            state.next_id = id;
            (
                id,
                state.scheduler.as_ref().ok_or_else(closed)?.get(env)?,
                state.callback.as_ref().ok_or_else(closed)?.get(env)?,
            )
        };
        // Validate the callback-scoped function before allocating a deferred
        // or transferring ownership into the pending registry.
        let scheduler =
            unsafe { scheduler.cast::<Function<'_, FnArgs<(Unknown<'_>, u32)>, Unknown<'_>>>()? };
        let mut deferred = ptr::null_mut();
        let mut promise = ptr::null_mut();
        // SAFETY: created and settled exclusively on this Node environment.
        status(unsafe { sys::napi_create_promise(env.raw(), &mut deferred, &mut promise) })?;
        self.0
            .borrow_mut()
            .pending
            .insert(id, Pending { call, deferred });
        if let Err(error) = scheduler.call((callback, id).into()) {
            self.0.borrow_mut().pending.remove(&id);
            // This internal promise has not escaped. Release its deferred
            // without an unhandled rejection; the public call returns error.
            let undefined = unsafe { <()>::to_napi_value(env.raw(), ())? };
            status(unsafe { sys::napi_resolve_deferred(env.raw(), deferred, undefined) })?;
            return Err(error);
        }
        Ok(PromiseRaw::new(env.raw(), promise))
    }

    fn dispatch(&self, env: &Env, id: u32) -> napi::Result<()> {
        let pending = self.0.borrow_mut().pending.remove(&id);
        let Some(Pending { call, deferred }) = pending else {
            return Ok(());
        };
        match call.start(env) {
            Ok(promise) => {
                status(unsafe { sys::napi_resolve_deferred(env.raw(), deferred, promise.raw()) })
            }
            Err(error) => {
                let error = unsafe { napi::Error::to_napi_value(env.raw(), error)? };
                status(unsafe { sys::napi_reject_deferred(env.raw(), deferred, error) })
            }
        }
    }

    pub fn metrics(&self) -> ConversionMetrics {
        let state = self.0.borrow();
        ConversionMetrics {
            pending: state.pending.len() as u32,
            binary_references: state
                .pending
                .values()
                .map(|pending| pending.call.input.binary_references() as u32)
                .sum(),
            backing_bytes: state
                .pending
                .values()
                .map(|pending| pending.call.input.backing_bytes as u32)
                .sum(),
        }
    }

    /// Called by the one environment hook, before joining the Rust FFI tracker.
    pub fn shutdown(&self) {
        let (pending, references) = {
            let mut state = self.0.borrow_mut();
            state.stopped = true;
            (
                std::mem::take(&mut state.pending),
                (
                    state.scheduler.take(),
                    state.callback.take(),
                    state.maximum.take(),
                ),
            )
        };
        // No JS execution or callback delivery is required. This also drops
        // unstarted FfiExecutions, so their TaskTracker tokens cannot deadlock.
        drop(pending);
        drop(references);
    }
}

fn closed() -> napi::Error {
    binding_error(HostError::TargetClosed)
}
