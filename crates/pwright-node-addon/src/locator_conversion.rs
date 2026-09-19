//! Bounded Node-thread rendezvous after Rust owns a prepared Locator element.
//! Only an integer notification and owned EvaluateParams cross threads.

use std::{cell::RefCell, collections::BTreeMap, rc::Rc, sync::Arc};

use napi::{
    Env, JsString, JsValue, Unknown,
    bindgen_prelude::{FnArgs, Function, JsObjectValue, Object},
    sys,
    threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode},
};
use napi_derive::napi;
use pwright_node_host::{
    HostError,
    evaluation::{EvaluateParams, EvaluationValue},
};
use tokio::sync::oneshot;

use crate::{
    binding_error,
    deferred_conversion::DeferredConversions,
    evaluation_value::{self, PreparedInput, status},
    node_reference::NodeReference,
};

const CAPACITY: usize = 32;
const CONVERSION_ERROR: &str = "Node Locator conversion failed";
type Notify = ThreadsafeFunction<u32, (), u32, napi::Status, false, true, CAPACITY>;
type Sender = oneshot::Sender<Result<EvaluateParams, HostError>>;

enum Stage {
    Raw(NodeReference),
    Binary {
        source: String,
        is_function: bool,
        input: PreparedInput,
    },
    Waiting,
}

struct Entry {
    stage: Stage,
    sender: Option<Sender>,
    error: Option<napi::Error>,
}

struct State {
    next_id: u32,
    entries: BTreeMap<u32, Entry>,
    notify: Option<Arc<Notify>>,
    scheduler: Option<NodeReference>,
    callback: Option<NodeReference>,
}

#[napi(object)]
#[derive(Default, Debug, PartialEq, Eq)]
pub struct LocatorConversionMetrics {
    pub pending: u32,
    pub raw_references: u32,
    pub binary_references: u32,
    pub backing_bytes: u32,
    pub exceptions: u32,
}

#[derive(Clone)]
pub struct LocatorConversions {
    state: Rc<RefCell<State>>,
    values: DeferredConversions,
}

pub struct Conversion {
    id: u32,
    notify: Arc<Notify>,
    receiver: oneshot::Receiver<Result<EvaluateParams, HostError>>,
}

/// Held by the Node settlement callback, including its early-error/drop paths.
pub struct PendingGuard {
    pub registry: LocatorConversions,
    pub id: u32,
}

impl Drop for PendingGuard {
    fn drop(&mut self) {
        self.registry.remove(self.id);
    }
}

impl Conversion {
    /// Invoked only inside the host's single prepared-element command.
    pub async fn run(self) -> Result<EvaluateParams, HostError> {
        match self
            .notify
            .call(self.id, ThreadsafeFunctionCallMode::NonBlocking)
        {
            napi::Status::Ok => {}
            napi::Status::Closing => return Err(HostError::TargetClosed),
            status => {
                return Err(HostError::ResourceExhausted(format!(
                    "Node Locator notification rejected: {status}"
                )));
            }
        }
        self.receiver.await.unwrap_or(Err(HostError::TargetClosed))
    }
}

impl LocatorConversions {
    pub fn new(
        env: &Env,
        scheduler: Function<'_>,
        values: DeferredConversions,
    ) -> napi::Result<Self> {
        let registry = Self {
            state: Rc::new(RefCell::new(State {
                next_id: 0,
                entries: BTreeMap::new(),
                notify: None,
                scheduler: Some(NodeReference::new(env, scheduler.to_unknown())?),
                callback: None,
            })),
            values,
        };
        let weak = Rc::downgrade(&registry.state);
        let values = registry.values.clone();
        let callback: Function<'_, u32, ()> =
            env.create_function_from_closure("pwrightConvertLocator", move |ctx| {
                if let Some(state) = weak.upgrade() {
                    let registry = Self {
                        state,
                        values: values.clone(),
                    };
                    let id = ctx.get::<u32>(0)?;
                    // Never throw user errors out of a TSFN callback. Retain them on
                    // this thread until Rust has released the temporary element.
                    if let Err(error) = registry.dispatch(ctx.env, id) {
                        registry.fail(ctx.env, id, error);
                    }
                }
                Ok(())
            })?;
        let notify = callback
            .build_threadsafe_function::<u32>()
            .weak::<true>()
            .max_queue_size::<CAPACITY>()
            .build()?;
        let mut state = registry.state.borrow_mut();
        state.callback = Some(NodeReference::new(env, callback.to_unknown())?);
        state.notify = Some(Arc::new(notify));
        drop(state);
        Ok(registry)
    }

    /// FFI command count/byte admission must precede this reference acquisition.
    pub fn admit(&self, env: &Env, converter: Function<'_>) -> napi::Result<(u32, Conversion)> {
        let mut state = self.state.borrow_mut();
        let notify = state.notify.as_ref().ok_or_else(closed)?.clone();
        if state.entries.len() >= CAPACITY {
            return Err(binding_error(HostError::ResourceExhausted(
                "Node Locator conversion capacity exceeded".into(),
            )));
        }
        let id = state.next_id.checked_add(1).ok_or_else(|| {
            binding_error(HostError::ResourceExhausted(
                "Node Locator conversion IDs exhausted".into(),
            ))
        })?;
        let reference = NodeReference::new(env, converter.to_unknown())?;
        let (sender, receiver) = oneshot::channel();
        state.next_id = id;
        state.entries.insert(
            id,
            Entry {
                stage: Stage::Raw(reference),
                sender: Some(sender),
                error: None,
            },
        );
        Ok((
            id,
            Conversion {
                id,
                notify,
                receiver,
            },
        ))
    }

    fn dispatch(&self, env: &Env, id: u32) -> napi::Result<()> {
        let stage = {
            let mut state = self.state.borrow_mut();
            let Some(entry) = state.entries.get_mut(&id) else {
                return Ok(());
            };
            if entry.sender.as_ref().is_none_or(Sender::is_closed) {
                entry.stage = Stage::Waiting;
                return Ok(());
            }
            std::mem::replace(&mut entry.stage, Stage::Waiting)
        };
        // No registry borrow spans source coercion, getters, parser calls or the
        // user-injected scheduler. Reentrant calls keep their own FFI admission.
        let (source, is_function, input) = match stage {
            Stage::Raw(reference) => {
                let converter =
                    unsafe { reference.get(env)?.cast::<Function<'_, (), Object<'_>>>()? };
                let value = converter.call(())?;
                let is_function: bool = value.get_named_property("isFunction")?;
                let source: JsString<'_> = value.get_named_property("source")?;
                let source = evaluation_value::source(env, source, is_function)?;
                let input = self
                    .values
                    .prepare(env, value.get_named_property::<Unknown<'_>>("argument")?)?;
                if input.binary_references() != 0 {
                    let (scheduler, callback) = {
                        let mut state = self.state.borrow_mut();
                        let Some(entry) = state.entries.get_mut(&id) else {
                            return Ok(());
                        };
                        entry.stage = Stage::Binary {
                            source,
                            is_function,
                            input,
                        };
                        (
                            state.scheduler.as_ref().ok_or_else(closed)?.get(env)?,
                            state.callback.as_ref().ok_or_else(closed)?.get(env)?,
                        )
                    };
                    let scheduler = unsafe {
                        scheduler.cast::<Function<'_, FnArgs<(Unknown<'_>, u32)>, Unknown<'_>>>()?
                    };
                    scheduler.call((callback, id).into())?;
                    return Ok(());
                }
                (source, is_function, input)
            }
            Stage::Binary {
                source,
                is_function,
                input,
            } => (source, is_function, input),
            Stage::Waiting => return Ok(()),
        };
        let argument = input.materialize(env)?;
        let params = if is_function {
            EvaluateParams::function(source, argument)
        } else {
            EvaluateParams::expression(source)
        };
        if let Some(sender) = self
            .state
            .borrow_mut()
            .entries
            .get_mut(&id)
            .and_then(|entry| entry.sender.take())
        {
            let _ = sender.send(Ok(params));
        }
        Ok(())
    }

    fn fail(&self, env: &Env, id: u32, error: napi::Error) {
        let error = capture_exception(env, error);
        if let Some(entry) = self.state.borrow_mut().entries.get_mut(&id) {
            entry.stage = Stage::Waiting;
            entry.error = Some(error);
            if let Some(sender) = entry.sender.take() {
                let _ = sender.send(Err(HostError::Backend(CONVERSION_ERROR.into())));
            }
        }
    }

    pub fn remove(&self, id: u32) {
        self.state.borrow_mut().entries.remove(&id);
    }

    pub fn finish(
        &self,
        id: u32,
        result: Result<EvaluationValue, HostError>,
    ) -> napi::Result<evaluation_value::EvaluationOutput> {
        let entry = self.state.borrow_mut().entries.remove(&id);
        match result {
            Err(HostError::Backend(ref message)) if message == CONVERSION_ERROR => Err(entry
                .and_then(|entry| entry.error)
                .unwrap_or_else(|| binding_error(HostError::Backend(CONVERSION_ERROR.into())))),
            // Cleanup/liveness failures must never be hidden by a saved getter
            // exception. Only the exact untouched conversion failure unwraps it.
            other => other
                .map(evaluation_value::EvaluationOutput)
                .map_err(binding_error),
        }
    }

    pub fn metrics(&self) -> LocatorConversionMetrics {
        let state = self.state.borrow();
        let mut metrics = LocatorConversionMetrics {
            pending: state.entries.len() as u32,
            ..Default::default()
        };
        for entry in state.entries.values() {
            metrics.exceptions += u32::from(entry.error.is_some());
            match &entry.stage {
                Stage::Raw(_) => metrics.raw_references += 1,
                Stage::Binary { input, .. } => {
                    metrics.binary_references += input.binary_references() as u32;
                    metrics.backing_bytes += input.backing_bytes as u32;
                }
                Stage::Waiting => {}
            }
        }
        metrics
    }

    /// Runs on the owning Node thread before FFI/native cleanup is joined.
    pub fn shutdown(&self) {
        let (entries, notify, scheduler, callback) = {
            let mut state = self.state.borrow_mut();
            (
                std::mem::take(&mut state.entries),
                state.notify.take(),
                state.scheduler.take(),
                state.callback.take(),
            )
        };
        drop(entries);
        // Rust FFI cancellation drains the remaining Arc owners before the
        // environment hook returns. Dropping the last Arc releases the TSFN.
        drop(notify);
        drop(scheduler);
        drop(callback);
    }
}

fn closed() -> napi::Error {
    binding_error(HostError::TargetClosed)
}

fn capture_exception(env: &Env, error: napi::Error) -> napi::Error {
    let mut pending = false;
    if status(unsafe { sys::napi_is_exception_pending(env.raw(), &mut pending) }).is_ok() && pending
    {
        let mut value = std::ptr::null_mut();
        if status(unsafe { sys::napi_get_and_clear_last_exception(env.raw(), &mut value) }).is_ok()
        {
            return napi::Error::from_unknown_without_coercion(unsafe {
                Unknown::from_raw_unchecked(env.raw(), value)
            });
        }
    }
    error
}
