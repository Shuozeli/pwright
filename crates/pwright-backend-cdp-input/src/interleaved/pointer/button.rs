//! Lazy standalone button dispatch with explicit provider retirement.
use super::{down_event, up_event};
use crate::{
    CdpMouseDispatcher, MouseDispatchKind, MouseDispatchOperation, MouseDispatchRequest,
    MouseStateReader,
    interleaved::{Action, PageState},
    production::InputAdmission,
};
use pwright_cdp::{CdpClient, MouseButton, connection::Result};
use std::{
    future::{Future, poll_fn},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

/// A Page-owned button action, retained outside its cancellable result future.
/// Construction is effect-free. Result completion does not acknowledge cleanup;
/// always retire this owner before dropping it, including on error/cancellation.
#[must_use = "retain the button execution through result observation and retirement"]
pub struct MouseButtonExecution {
    page: Arc<PageState>,
    client: Arc<dyn CdpClient>,
    dispatcher: Arc<dyn CdpMouseDispatcher>,
    button: MouseButton,
    count: i32,
    kind: MouseDispatchKind,
    operation: Option<Box<dyn MouseDispatchOperation>>,
    observed: bool,
    _admission: Option<Arc<InputAdmission>>,
}
impl MouseButtonExecution {
    pub(crate) fn new(
        action: &Action,
        admission: Option<Arc<InputAdmission>>,
        button: MouseButton,
        count: i32,
        kind: MouseDispatchKind,
        dispatcher: Arc<dyn CdpMouseDispatcher>,
    ) -> Self {
        assert!(matches!(
            kind,
            MouseDispatchKind::Down | MouseDispatchKind::Up
        ));
        Self {
            page: action.page.clone(),
            client: action.client.clone(),
            dispatcher,
            button,
            count,
            kind,
            operation: None,
            observed: false,
            _admission: admission,
        }
    }
    /// Retained provider owners, not their separately budgeted nested RPC count.
    pub fn pending_native_calls(&self) -> usize {
        usize::from(self.operation.is_some())
    }
    /// Only this outer owner. Provider boxes, events, subcalls, errors and shared
    /// state require their own admission; shared Page/client state is excluded.
    pub const fn allocation_payload_bytes() -> usize {
        std::mem::size_of::<Self>()
    }
    pub async fn retire(&mut self) -> Result<()> {
        self.observed = true;
        poll_fn(|cx| {
            let Some(operation) = self.operation.as_mut() else {
                return Poll::Ready(Ok(()));
            };
            let result = operation.poll_retire(cx);
            if result.is_ready() {
                self.operation = None;
            }
            result
        })
        .await
    }
}
impl Future for MouseButtonExecution {
    type Output = Result<()>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        assert!(
            !this.observed,
            "button result already observed; retire the owner"
        );
        if this.operation.is_none() {
            // Logical button state changes before provider dispatch, including
            // when a drag provider suppresses the physical down/up event.
            let event = match this.kind {
                MouseDispatchKind::Down => down_event(&this.page, this.button, this.count),
                MouseDispatchKind::Up => up_event(&this.page, this.button, this.count),
                MouseDispatchKind::Move { .. } => {
                    unreachable!("button kind validated at construction")
                }
            };
            this.operation = Some(this.dispatcher.start(
                this.client.clone(),
                MouseDispatchRequest {
                    kind: this.kind,
                    event,
                    live_state: MouseStateReader::new(this.page.clone(), this._admission.clone()),
                },
            ));
        }
        let operation = this
            .operation
            .as_mut()
            .expect("button dispatch constructed");
        let result = operation.poll_result(cx);
        if result.is_ready() {
            this.observed = true;
            if operation.is_retired() {
                this.operation = None;
            }
        }
        result
    }
}
