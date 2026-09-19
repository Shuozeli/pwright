//! Optional multi-phase mouse dispatch. No drag implementation or executor here.
use super::PageState;
use crate::production::InputAdmission;
use pwright_cdp::{CdpClient, connection::Result, generated::input::DispatchMouseEventParams};
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MouseDispatchKind {
    Move { for_click: bool },
    Down,
    Up,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MouseLiveState {
    pub buttons: i32,
    pub modifiers: i32,
}

/// Read-only Page state for a deferred native move. Position and last-button
/// remain captured in the request; only live buttons/modifiers are exposed.
#[derive(Clone)]
pub struct MouseStateReader {
    page: Arc<PageState>,
    _admission: Option<Arc<InputAdmission>>,
}
impl MouseStateReader {
    pub(super) fn new(page: Arc<PageState>, admission: Option<Arc<InputAdmission>>) -> Self {
        Self {
            page,
            _admission: admission,
        }
    }
    pub fn snapshot(&self) -> MouseLiveState {
        let state = self.page.state.lock().expect("input state is not poisoned");
        MouseLiveState {
            buttons: state.buttons,
            modifiers: state.modifiers.mask(),
        }
    }
}
pub struct MouseDispatchRequest {
    pub kind: MouseDispatchKind,
    pub event: DispatchMouseEventParams,
    pub live_state: MouseStateReader,
}

/// Provider admission and construction MUST be effect-free. The provider owns
/// and bounds all boxes, child native futures, listeners, payloads and errors.
/// The sequence accounts only for its fixed slots and direct fallback storage.
/// Retained shared drag state needs a separate Page-scoped owner/lease.
pub trait CdpMouseDispatcher: Send + Sync {
    fn start(
        &self,
        client: Arc<dyn CdpClient>,
        request: MouseDispatchRequest,
    ) -> Box<dyn MouseDispatchOperation>;
}

/// Result completion is not retirement. On cancellation, poll_retire must stop
/// new pointer actions, drain started work, and clean up its own temporary state.
/// It must register a waker when pending. Ready retirement is terminal (including
/// an error receipt); anything still persistent must have a separate owner.
/// Dropping this object is NOT a retirement acknowledgement.
pub trait MouseDispatchOperation: Send {
    fn poll_result(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>>;
    fn poll_retire(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>>;
    fn is_retired(&self) -> bool;
}

type NativeFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;
async fn native_dispatch(
    client: Arc<dyn CdpClient>,
    event: DispatchMouseEventParams,
) -> Result<()> {
    client.input_dispatch_pointer_event(event).await
}

/// A single native RPC fallback. Constructing it does not send the event.
pub struct DirectMouseDispatch {
    future: Option<NativeFuture>,
    started: bool,
    observed: bool,
}
impl DirectMouseDispatch {
    pub fn new(client: Arc<dyn CdpClient>, event: DispatchMouseEventParams) -> Self {
        Self {
            future: Some(Box::pin(native_dispatch(client, event))),
            started: false,
            observed: false,
        }
    }
    /// Excludes allocations inside the injected client, event text, and allocator
    /// headers. Event text and provider allocations retain their separate bounds.
    pub fn allocation_payload_bytes(client: Arc<dyn CdpClient>) -> usize {
        std::mem::size_of::<Self>() + Self::future_payload_bytes(client)
    }
    pub(super) fn future_payload_bytes(client: Arc<dyn CdpClient>) -> usize {
        std::mem::size_of_val(&native_dispatch(
            client,
            DispatchMouseEventParams::default(),
        ))
    }
    fn poll_native(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        let result = self
            .future
            .as_mut()
            .expect("native call is retained")
            .as_mut()
            .poll(cx);
        if result.is_ready() {
            self.future = None;
        }
        result
    }
}
impl MouseDispatchOperation for DirectMouseDispatch {
    fn poll_result(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        assert!(!self.observed, "dispatch result was already observed");
        self.started = true;
        let result = self.poll_native(cx);
        self.observed = result.is_ready();
        result
    }
    fn poll_retire(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.observed = true;
        if !self.started {
            self.future = None;
        }
        if self.future.is_none() {
            Poll::Ready(Ok(()))
        } else {
            self.poll_native(cx)
        }
    }
    fn is_retired(&self) -> bool {
        self.future.is_none()
    }
}

pub(super) enum DispatchCall {
    Direct(DirectMouseDispatch),
    Delegated {
        operation: Box<dyn MouseDispatchOperation>,
        observed: bool,
    },
}
impl DispatchCall {
    pub fn observed(&self) -> bool {
        match self {
            Self::Direct(call) => call.observed,
            Self::Delegated { observed, .. } => *observed,
        }
    }
    pub fn poll_result(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        match self {
            Self::Direct(call) => call.poll_result(cx),
            Self::Delegated {
                operation,
                observed,
            } => {
                assert!(!*observed, "dispatch result was already observed");
                let result = operation.poll_result(cx);
                *observed = result.is_ready();
                result
            }
        }
    }
    pub fn poll_retire(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        match self {
            Self::Direct(call) => call.poll_retire(cx),
            Self::Delegated { operation, .. } => operation.poll_retire(cx),
        }
    }
    pub fn is_retired(&self) -> bool {
        match self {
            Self::Direct(call) => call.is_retired(),
            Self::Delegated { operation, .. } => operation.is_retired(),
        }
    }
}

#[cfg(test)]
mod tests;
