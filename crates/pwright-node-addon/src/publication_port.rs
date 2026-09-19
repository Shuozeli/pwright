//! Environment-thread dispatch with explicit Session/Locator ownership.
//! No callback or Node value is sent into a Rust worker or held under a mutex.
use crate::{binding_error, evaluation_value::status, node_reference::NodeReference};
use napi::{Env, JsValue, Unknown, bindgen_prelude::Function, sys};
use pwright_node_host::{
    FramePublicationFence, FramePublications, HostError, HostSession, LifecycleDelivery,
    LifecycleNoticeView, PublicationDrainError, PublicationProgress,
    ffi_budget::PublicationRegistryStorage, frame_tree::FrameConversion,
};
use std::{
    cell::RefCell,
    ptr,
    rc::{Rc, Weak},
};
use tokio_util::sync::CancellationToken;

mod conversion;

const PORTS: usize = 16;
// One synchronous conversion/emit at a time under the exclusive native drain.
// Covers the port/Rc bodies, Node reference holder, ID strings and call frames;
// V8 allocations are GC-owned, not claimed as Rust RSS accounting.
const PORT_BYTES: usize = 32 * 1024;

struct State {
    #[cfg(feature = "retained-file-input")]
    file_input_scope: Option<crate::handles::file_input::PreparationScope>,
    callback: Option<NodeReference>,
    finished: bool,
    _storage: FrameConversion,
}

#[derive(Clone)]
pub struct PublicationPort {
    state: Rc<RefCell<State>>,
    book: FramePublications,
    retired: CancellationToken,
    host: std::sync::Weak<HostSession>,
}

const _: () = assert!(
    PORT_BYTES
        >= std::mem::size_of::<RefCell<State>>()
            + 2 * std::mem::size_of::<usize>()
            + std::mem::size_of::<PublicationPort>()
            + 6 * (std::mem::size_of::<String>() + 20)
            + std::mem::size_of::<crate::PageDescriptor>()
            + std::mem::size_of::<crate::ContextDescriptor>()
            + std::mem::size_of::<FramePublicationFence>()
            + 4096
);

struct Registry {
    stopped: bool,
    ports: [Option<Weak<RefCell<State>>>; PORTS],
    _storage: PublicationRegistryStorage,
}

#[derive(Clone)]
pub struct PublicationPorts(Rc<RefCell<Registry>>);

impl PublicationPorts {
    pub fn new() -> Result<Self, HostError> {
        let storage = PublicationRegistryStorage::reserve(
            std::mem::size_of::<RefCell<Registry>>()
                + 2 * std::mem::size_of::<usize>()
                + std::mem::size_of::<Self>()
                + std::mem::size_of::<[Option<Weak<RefCell<State>>>; PORTS]>()
                // Weak slots can retain an empty Rc allocation after its
                // last strong port has released the per-session lease.
                + PORTS * (std::mem::size_of::<RefCell<State>>()
                    + 2 * std::mem::size_of::<usize>()),
        )?;
        Ok(Self(Rc::new(RefCell::new(Registry {
            stopped: false,
            ports: std::array::from_fn(|_| None),
            _storage: storage,
        }))))
    }

    pub fn admit(&self, host: &std::sync::Arc<HostSession>) -> Result<PublicationPort, HostError> {
        let mut registry = self.0.borrow_mut();
        if registry.stopped {
            return Err(HostError::TargetClosed);
        }
        let slot = registry
            .ports
            .iter_mut()
            .find(|slot| {
                slot.as_ref()
                    .and_then(Weak::upgrade)
                    .is_none_or(|state| state.borrow().finished)
            })
            .ok_or_else(|| {
                HostError::ResourceExhausted("publication port capacity exceeded (16)".into())
            })?;
        let storage = host.reserve_metadata_conversion(PORT_BYTES)?;
        let state = Rc::new(RefCell::new(State {
            #[cfg(feature = "retained-file-input")]
            file_input_scope: None,
            callback: None,
            finished: false,
            _storage: storage,
        }));
        *slot = Some(Rc::downgrade(&state));
        Ok(PublicationPort {
            state,
            book: host.frame_publications(),
            retired: host.disconnect_signal(),
            host: std::sync::Arc::downgrade(host),
        })
    }

    pub fn shutdown(&self) {
        let ports = {
            let mut registry = self.0.borrow_mut();
            registry.stopped = true;
            std::mem::take(&mut registry.ports)
        };
        for state in ports
            .into_iter()
            .flatten()
            .filter_map(|weak| weak.upgrade())
        {
            finish(&state);
        }
    }
}

fn finish(state: &Rc<RefCell<State>>) {
    #[cfg(feature = "retained-file-input")]
    {
        let scope = state.borrow_mut().file_input_scope.take();
        if let Some(scope) = scope {
            scope.close();
        }
    }
    let callback = {
        let mut state = state.borrow_mut();
        state.finished = true;
        state.callback.take()
    };
    drop(callback);
}

impl PublicationPort {
    #[cfg(feature = "retained-file-input")]
    pub(crate) fn bind_file_input_scope(
        &self,
        scope: crate::handles::file_input::PreparationScope,
    ) {
        self.state.borrow_mut().file_input_scope = Some(scope);
    }

    #[cfg(feature = "retained-file-input")]
    fn revoke_file_input_scope(&self) {
        let scope = self.state.borrow().file_input_scope.clone();
        if let Some(scope) = scope {
            scope.close();
        }
    }
    pub fn publications(&self) -> FramePublications {
        self.book.clone()
    }

    pub fn register(&self, env: &Env, callback: Function<'_>) -> napi::Result<()> {
        let mut state = self.state.borrow_mut();
        if state.finished {
            return Ok(());
        }
        if state.callback.is_some() {
            return Err(napi::Error::from_reason(
                "lifecycle callback is already registered",
            ));
        }
        state.callback = Some(NodeReference::new(env, callback.to_unknown())?);
        Ok(())
    }

    fn prepare<'env>(
        &self,
        env: &'env Env,
        view: LifecycleNoticeView<'_>,
    ) -> napi::Result<Option<Unknown<'env>>> {
        let callback = {
            let state = self.state.borrow();
            if state.finished {
                return Err(binding_error(HostError::TargetClosed));
            }
            state
                .callback
                .as_ref()
                .map(|callback| callback.get(env))
                .transpose()?
        };
        // Before the Browser wrapper exists there can be no attached listeners.
        let Some(callback) = callback else {
            return Ok(None);
        };
        let callback = unsafe { callback.cast::<conversion::Callback<'_>>()? };
        let prepared = callback.call(conversion::arguments(&view)?.into())?;
        conversion::finish(view, prepared)
    }

    pub fn publish(&self, env: &Env, fence: FramePublicationFence) -> napi::Result<()> {
        #[cfg(feature = "retained-file-input")]
        if self.retired.is_cancelled()
            || self.host.upgrade().is_none_or(|host| !host.is_connected())
        {
            // Revoke before terminal callbacks can reenter. Ordinary Page or
            // Context closure must not revoke this still-connected session.
            self.revoke_file_input_scope();
        }
        let result = self.publish_inner(env, fence);
        if result.is_err() {
            // Internal conversion/dispatch failure is not a listener exception.
            // Fail closed rather than spinning the watcher on an unconsumable notice.
            self.book.discard();
            finish(&self.state);
            if let Some(host) = self.host.upgrade() {
                host.request_shutdown();
            }
        }
        result
    }

    fn publish_inner(&self, env: &Env, fence: FramePublicationFence) -> napi::Result<()> {
        let mut drain = fence
            .try_begin()
            .map_err(|error| napi::Error::from_reason(error.to_string()))?;
        loop {
            match drain.publish_next() {
                Ok(PublicationProgress::Published) => {}
                Ok(PublicationProgress::Complete) => break,
                Err(PublicationDrainError::NoticePending) => {
                    let prepared = drain
                        .with_next_notice(|value| self.prepare(env, value))
                        .map_err(|error| napi::Error::from_reason(error.to_string()))??;
                    drain
                        .commit_notice()
                        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
                    let exception = prepared
                        .map(|callback| emit(env, callback))
                        .transpose()?
                        .flatten();
                    let delivery = if exception.is_some() {
                        LifecycleDelivery::ListenerThrew
                    } else {
                        LifecycleDelivery::Delivered
                    };
                    drain
                        .acknowledge_notice(delivery)
                        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
                    if let Some(exception) = exception {
                        // Acknowledge before Node may synchronously reenter from
                        // uncaughtException. Never convert/reject with this value.
                        status(unsafe { sys::napi_fatal_exception(env.raw(), exception.raw()) })?;
                    }
                }
                Err(error) => return Err(napi::Error::from_reason(error.to_string())),
            }
        }
        if self.retired.is_cancelled() && self.book.is_drained().map_err(binding_error)? {
            finish(&self.state);
        }
        Ok(())
    }
}

/// Preserve the original thrown Error, object or primitive, without converting
/// it to napi::Error or allowing it to reject the operation/watch Promise.
fn emit<'env>(env: &'env Env, callback: Unknown<'env>) -> napi::Result<Option<Unknown<'env>>> {
    let mut receiver = ptr::null_mut();
    status(unsafe { sys::napi_get_undefined(env.raw(), &mut receiver) })?;
    let mut result = ptr::null_mut();
    let code = unsafe {
        sys::napi_call_function(
            env.raw(),
            receiver,
            callback.raw(),
            0,
            ptr::null(),
            &mut result,
        )
    };
    if code == sys::Status::napi_pending_exception {
        let mut exception = ptr::null_mut();
        status(unsafe { sys::napi_get_and_clear_last_exception(env.raw(), &mut exception) })?;
        Ok(Some(unsafe {
            Unknown::from_raw_unchecked(env.raw(), exception)
        }))
    } else {
        status(code)?;
        Ok(None)
    }
}
