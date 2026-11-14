//! Python wrapper ownership for the optional Rust Handle capability.

use std::sync::{Arc, Mutex};

use pwright_node_host::{
    HostError, HostSession,
    handles::{HandleRelease, PendingHandle},
};
#[cfg(feature = "worker-handle-evaluation")]
use pwright_node_host::{WorkerHandle, WorkerHandleRelease};
use pyo3::prelude::*;

use crate::binding_error;

enum HandleValue {
    Frame(Arc<PendingHandle>),
    #[cfg(feature = "worker-handle-evaluation")]
    Worker(WorkerHandle),
}

enum State {
    Live(HandleValue),
    Disposed(NativeHandleRelease),
}

#[derive(Clone)]
pub(crate) enum NativeHandleRelease {
    Frame(HandleRelease),
    #[cfg(feature = "worker-handle-evaluation")]
    Worker(WorkerHandleRelease),
}

impl NativeHandleRelease {
    pub(crate) async fn wait(self) -> Result<(), HostError> {
        match self {
            Self::Frame(release) => release.wait().await,
            #[cfg(feature = "worker-handle-evaluation")]
            Self::Worker(release) => release.wait().await,
        }
    }
}

/// Capture an optional, Rust-owned Node preview before publication. A plain
/// Handle build retains its original CDP description and performs no DOM work.
pub(crate) async fn with_node_preview(
    host: &HostSession,
    value: PendingHandle,
) -> Result<(PendingHandle, Option<String>), HostError> {
    #[cfg(feature = "element-handle-dom")]
    if value.is_element() {
        let preview = host.element_handle_preview(&value).await?;
        return Ok((value, Some(preview)));
    }
    #[cfg(not(feature = "element-handle-dom"))]
    let _ = host;
    Ok((value, None))
}

#[pyclass(module = "pwright._native")]
pub(crate) struct NativeHandle {
    state: Arc<Mutex<State>>,
    preview: Option<String>,
}

impl NativeHandle {
    pub(crate) fn new(value: PendingHandle) -> Self {
        Self::new_with_preview(value, None)
    }

    pub(crate) fn new_with_preview(value: PendingHandle, preview: Option<String>) -> Self {
        Self {
            state: Arc::new(Mutex::new(State::Live(HandleValue::Frame(Arc::new(value))))),
            preview,
        }
    }

    #[cfg(feature = "console-argument-handles")]
    pub(crate) fn from_shared(value: Arc<PendingHandle>) -> Self {
        Self {
            state: Arc::new(Mutex::new(State::Live(HandleValue::Frame(value)))),
            preview: None,
        }
    }

    #[cfg(feature = "worker-handle-evaluation")]
    pub(crate) fn from_worker(value: WorkerHandle) -> Self {
        Self {
            preview: Some(value.preview().to_owned()),
            state: Arc::new(Mutex::new(State::Live(HandleValue::Worker(value)))),
        }
    }

    pub(crate) fn read_frame(&self) -> Result<Arc<PendingHandle>, HostError> {
        match &*self
            .state
            .lock()
            .map_err(|error| HostError::Backend(error.to_string()))?
        {
            State::Live(HandleValue::Frame(value)) => Ok(value.clone()),
            #[cfg(feature = "worker-handle-evaluation")]
            State::Live(HandleValue::Worker(_)) => Err(HostError::InvalidArgument(
                "Worker JSHandle cannot be used by a Frame-only operation".into(),
            )),
            State::Disposed(_) => Err(HostError::Backend("JSHandle is disposed".into())),
        }
    }

    #[cfg(feature = "worker-handle-evaluation")]
    pub(crate) fn read_worker(&self) -> Result<Option<WorkerHandle>, HostError> {
        match &*self
            .state
            .lock()
            .map_err(|error| HostError::Backend(error.to_string()))?
        {
            State::Live(HandleValue::Frame(_)) => Ok(None),
            State::Live(HandleValue::Worker(value)) => Ok(Some(value.clone())),
            State::Disposed(_) => Err(HostError::Backend("JSHandle is disposed".into())),
        }
    }

    pub(crate) fn release(&self) -> Result<NativeHandleRelease, HostError> {
        let mut state = self
            .state
            .lock()
            .map_err(|error| HostError::Backend(error.to_string()))?;
        let release = match &*state {
            State::Live(HandleValue::Frame(value)) => {
                NativeHandleRelease::Frame(value.request_dispose())
            }
            #[cfg(feature = "worker-handle-evaluation")]
            State::Live(HandleValue::Worker(value)) => {
                NativeHandleRelease::Worker(value.request_dispose())
            }
            State::Disposed(release) => return Ok(release.clone()),
        };
        *state = State::Disposed(release.clone());
        Ok(release)
    }
}

#[pymethods]
impl NativeHandle {
    fn is_element(&self) -> PyResult<bool> {
        match &*self
            .state
            .lock()
            .map_err(|error| binding_error(HostError::Backend(error.to_string())))?
        {
            State::Live(HandleValue::Frame(value)) => Ok(value.is_element()),
            #[cfg(feature = "worker-handle-evaluation")]
            State::Live(HandleValue::Worker(value)) => Ok(value.is_element()),
            State::Disposed(_) => Err(binding_error(HostError::Backend(
                "JSHandle is disposed".into(),
            ))),
        }
    }

    fn preview(&self) -> PyResult<String> {
        if let Some(preview) = &self.preview {
            return Ok(preview.clone());
        }
        match &*self
            .state
            .lock()
            .map_err(|error| binding_error(HostError::Backend(error.to_string())))?
        {
            State::Live(HandleValue::Frame(value)) => Ok(value.initializer().preview.clone()),
            #[cfg(feature = "worker-handle-evaluation")]
            State::Live(HandleValue::Worker(value)) => Ok(value.preview().to_owned()),
            State::Disposed(_) => Err(binding_error(HostError::Backend(
                "JSHandle is disposed".into(),
            ))),
        }
    }
}
