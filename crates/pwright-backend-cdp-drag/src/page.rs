//! Page-owned drag data and retained native enter/over/drop/cancel commands.
use crate::{
    AdmittedDragData, DragCaptureFactory, DragInterceptionClient, DragMove, DragToggleFuture, error,
};
use pwright_cdp::{CdpSession, connection::Result};
use pwright_core::{
    ProductionResourceLedger, ResourceClass, ResourceLease, ResourceLedger, ResourceRequirements,
    ResourceScope,
};
use std::{
    future::{Future, poll_fn},
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragPosition {
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DragEventKind {
    Enter,
    Over,
    Drop,
    Cancel,
}
impl DragEventKind {
    fn name(self) -> &'static str {
        match self {
            Self::Enter => "dragEnter",
            Self::Over => "dragOver",
            Self::Drop => "drop",
            Self::Cancel => "dragCancel",
        }
    }
}
/// Must be the same fixed connection/session as the Page's capture factory.
/// Commands are lazy, typed at this boundary and retain their native receipt.
pub trait DragEventClient: DragInterceptionClient {
    fn dispatch_drag(
        self: Arc<Self>,
        kind: DragEventKind,
        position: DragPosition,
        data: Option<AdmittedDragData>,
        modifiers: Option<i64>,
    ) -> DragToggleFuture;
}
impl DragEventClient for CdpSession {
    fn dispatch_drag(
        self: Arc<Self>,
        kind: DragEventKind,
        position: DragPosition,
        data: Option<AdmittedDragData>,
        modifiers: Option<i64>,
    ) -> DragToggleFuture {
        Box::pin(async move {
            // Borrow retained typed data into the separately budgeted outgoing
            // JSON tree; do not first deep-clone its strings/vectors.
            let payload = if kind == DragEventKind::Cancel {
                serde_json::json!({"items":[],"dragOperationsMask":65535})
            } else {
                serde_json::to_value(
                    data.as_ref()
                        .ok_or_else(|| error("drag event requires data"))?
                        .data(),
                )
                .map_err(|_| error("drag data encoding failed"))?
            };
            let mut params = serde_json::json!({"type":kind.name(),"x":position.x,"y":position.y,"data":payload});
            if let Some(modifiers) = modifiers {
                params["modifiers"] = modifiers.into();
            }
            self.send("Input.dispatchDragEvent", params).await?;
            Ok(())
        })
    }
}
struct State {
    data: Option<AdmittedDragData>,
    last: DragPosition,
    operations: usize,
    poisoned: bool,
}
struct Page {
    client: Arc<dyn DragEventClient>,
    captures: DragCaptureFactory,
    state: Mutex<State>,
    ledger: ResourceLedger,
    scope: ResourceScope,
    capacity: usize,
    _storage: ResourceLease,
}
/// One shared instance per native Page. This owns persistent drag data, not the
/// Mouse coordinate/button state. Host close must drain commands and cancel an
/// active drag before transport teardown; dropping this handle is not cleanup.
#[derive(Clone)]
pub struct DragPage(Arc<Page>);
fn page_bytes() -> usize {
    std::mem::size_of::<Page>() + 2 * std::mem::size_of::<usize>()
}
impl DragPage {
    #[cfg(feature = "mouse-dispatch")]
    pub(crate) fn matches_dispatch(
        &self,
        client: &Arc<dyn pwright_cdp::CdpClient>,
        captures: &DragCaptureFactory,
    ) -> bool {
        std::ptr::addr_eq(Arc::as_ptr(client), Arc::as_ptr(&self.0.client))
            && self.0.captures.shares_factory(captures)
    }
    /// Page/operation payloads only. Captures/data, injected command futures and
    /// outgoing/reply trees retain separate admission. Not a process RSS bound.
    pub fn requirements(capacity: usize) -> Result<ResourceRequirements> {
        if !(1..=1024).contains(&capacity) {
            return Err(error("drag Page command capacity must be 1..=1024"));
        }
        let bytes = (page_bytes() + capacity * std::mem::size_of::<DragPageOperation>()) as u64;
        Ok(ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, capacity + 1))
    }
    pub fn new(
        client: Arc<dyn DragEventClient>,
        captures: DragCaptureFactory,
        scope: ResourceScope,
        capacity: usize,
        resources: &ProductionResourceLedger,
    ) -> Result<Self> {
        let binding: Arc<dyn DragInterceptionClient> = client.clone();
        if !captures.uses_client(&binding) {
            return Err(error(
                "drag Page and capture require identical native binding",
            ));
        }
        let ledger = resources
            .claim(Self::requirements(capacity)?)
            .map_err(|_| error("drag Page resource claim failed"))?;
        let storage = ledger
            .reserve(scope, ResourceClass::BackendStateBytes, page_bytes() as u64)
            .map_err(|_| error("drag Page state admission failed"))?;
        Ok(Self(Arc::new(Page {
            client,
            captures,
            scope,
            capacity,
            ledger,
            state: Mutex::new(State {
                data: None,
                last: DragPosition { x: 0.0, y: 0.0 },
                operations: 0,
                poisoned: false,
            }),
            _storage: storage,
        })))
    }
    pub fn is_dragging(&self) -> bool {
        self.0
            .state
            .lock()
            .expect("drag Page poisoned")
            .data
            .is_some()
    }
    pub fn active_operations(&self) -> usize {
        self.0.state.lock().expect("drag Page poisoned").operations
    }
    /// Call on first polling every ordinary move, even when it will use direct
    /// pointer dispatch. Do not call for a forClick bypass move. Enter does not
    /// rewind this position after a later move has already updated it.
    pub fn record_position(&self, position: DragPosition) -> Result<()> {
        position.validate()?;
        self.0.state.lock().expect("drag Page poisoned").last = position;
        Ok(())
    }
    /// Only data actually captured by this Page's factory and retained by the
    /// completed move can be installed. Same session text on another binding is
    /// not sufficient. State installation happens on first poll, before enter.
    pub fn enter(
        &self,
        movement: &DragMove,
        data: AdmittedDragData,
        position: DragPosition,
        modifiers: i64,
    ) -> Result<DragPageOperation> {
        if !movement.owns_captured_data(&self.0.captures, &data) {
            return Err(error(
                "drag data does not belong to this Page's completed capture",
            ));
        }
        self.start(Plan::Enter(data, position, modifiers))
    }
    pub fn over(&self, position: DragPosition, modifiers: i64) -> Result<DragPageOperation> {
        self.start(Plan::Over(position, modifiers))
    }
    pub fn drop_at(&self, position: DragPosition, modifiers: i64) -> Result<DragPageOperation> {
        self.start(Plan::Drop(position, modifiers))
    }
    /// False means no active drag and no native command. On a real cancellation,
    /// use the last ordinary-move position and the empty-data/all-operations mask
    /// used by upstream; no modifier property is sent.
    pub fn cancel(&self) -> Result<DragPageOperation> {
        self.start(Plan::Cancel)
    }
    fn start(&self, plan: Plan) -> Result<DragPageOperation> {
        match &plan {
            Plan::Enter(_, pos, _) | Plan::Over(pos, _) | Plan::Drop(pos, _) => pos.validate()?,
            Plan::Cancel => {}
        }
        let mut state = self.0.state.lock().expect("drag Page poisoned");
        if state.poisoned {
            return Err(error("drag Page lost an undrained native command"));
        }
        if state.operations == self.0.capacity {
            return Err(error("drag Page command capacity exceeded"));
        }
        let storage = self
            .0
            .ledger
            .reserve(
                self.0.scope,
                ResourceClass::BackendStateBytes,
                std::mem::size_of::<DragPageOperation>() as u64,
            )
            .map_err(|_| error("drag Page command admission failed"))?;
        state.operations += 1;
        Ok(DragPageOperation {
            page: self.clone(),
            plan: Some(plan),
            pending: None,
            kind: None,
            observed: false,
            retiring: false,
            _storage: storage,
        })
    }
}
impl DragPosition {
    fn validate(self) -> Result<()> {
        if self.x.is_finite() && self.y.is_finite() {
            Ok(())
        } else {
            Err(error("drag coordinates must be finite"))
        }
    }
}
enum Plan {
    Enter(AdmittedDragData, DragPosition, i64),
    Over(DragPosition, i64),
    Drop(DragPosition, i64),
    Cancel,
}
/// Retains native work after its result future is abandoned. Result success
/// reports whether a command was sent; retirement is independently observable.
#[must_use = "retain native drag commands through their actual receipt"]
pub struct DragPageOperation {
    page: DragPage,
    plan: Option<Plan>,
    pending: Option<DragToggleFuture>,
    kind: Option<DragEventKind>,
    observed: bool,
    retiring: bool,
    _storage: ResourceLease,
}
impl DragPageOperation {
    pub fn is_retired(&self) -> bool {
        self.plan.is_none() && self.pending.is_none()
    }
    fn begin(&mut self) -> Result<bool> {
        let plan = self.plan.take().expect("fresh drag command");
        let mut state = self.page.0.state.lock().expect("drag Page poisoned");
        let (kind, pos, data, modifiers) = match plan {
            Plan::Enter(data, pos, modifiers) => {
                state.data = Some(data.clone());
                (DragEventKind::Enter, pos, Some(data), Some(modifiers))
            }
            Plan::Over(pos, modifiers) => {
                state.last = pos;
                (
                    DragEventKind::Over,
                    pos,
                    Some(
                        state
                            .data
                            .clone()
                            .ok_or_else(|| error("no active drag for over"))?,
                    ),
                    Some(modifiers),
                )
            }
            Plan::Drop(pos, modifiers) => (
                DragEventKind::Drop,
                pos,
                Some(
                    state
                        .data
                        .clone()
                        .ok_or_else(|| error("no active drag for drop"))?,
                ),
                Some(modifiers),
            ),
            Plan::Cancel => {
                if state.data.is_none() {
                    return Ok(false);
                }
                (DragEventKind::Cancel, state.last, None, None)
            }
        };
        drop(state);
        self.kind = Some(kind);
        self.pending = Some(
            self.page
                .0
                .client
                .clone()
                .dispatch_drag(kind, pos, data, modifiers),
        );
        Ok(true)
    }
    fn poll_native(&mut self, cx: &mut Context<'_>, result_path: bool) -> Poll<Result<()>> {
        let Poll::Ready(result) = self
            .pending
            .as_mut()
            .expect("retained drag command")
            .as_mut()
            .poll(cx)
        else {
            return Poll::Pending;
        };
        self.pending = None;
        // Upstream drop clears after progress.race succeeds. Cancellation before
        // observing that success skips its state update. cancelDrag has no inner
        // progress race and clears after the native reply even if its caller left.
        if result.is_ok()
            && (self.kind == Some(DragEventKind::Cancel)
                || (result_path && self.kind == Some(DragEventKind::Drop)))
        {
            self.page.0.state.lock().expect("drag Page poisoned").data = None;
        }
        Poll::Ready(result)
    }
    pub async fn retire(&mut self) -> Result<()> {
        self.retiring = true;
        self.observed = true;
        self.plan = None;
        poll_fn(|cx| {
            if self.pending.is_some() {
                self.poll_native(cx, false)
            } else {
                Poll::Ready(Ok(()))
            }
        })
        .await
    }
}
impl Future for DragPageOperation {
    type Output = Result<bool>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.observed || this.retiring {
            return Poll::Ready(Err(error(
                "drag command result already observed or cancelled",
            )));
        }
        if this.plan.is_some() {
            match this.begin() {
                Ok(true) => {}
                Ok(false) => {
                    this.observed = true;
                    return Poll::Ready(Ok(false));
                }
                Err(error) => {
                    this.observed = true;
                    return Poll::Ready(Err(error));
                }
            }
        }
        let result = this
            .poll_native(cx, true)
            .map(|result| result.map(|()| true));
        if result.is_ready() {
            this.observed = true;
        }
        result
    }
}
impl Drop for DragPageOperation {
    fn drop(&mut self) {
        let mut state = self.page.0.state.lock().expect("drag Page poisoned");
        if self.pending.is_some() {
            state.poisoned = true;
        }
        state.operations -= 1;
    }
}

#[cfg(test)]
mod tests;
