//! Explicit opt-in Page Mouse dispatcher; no executor or Frame discovery.
use crate::{
    DragCaptureFactory, DragMove, DragPage, DragPageOperation, DragPosition, DragToggleFuture,
    error,
};
use pwright_backend_cdp_input::{
    CdpKeyDownInterceptor, CdpMouseDispatcher, KeyDownInterceptionFuture, MouseDispatchKind,
    MouseDispatchOperation, MouseDispatchRequest, MouseStateReader,
};
use pwright_cdp::{
    CdpClient,
    connection::{CdpError, Result},
    generated::input::MouseButton,
};
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

/// Caller-supplied bounded snapshot composition. Binding is immutable and must
/// match the Page. start creates only fresh unpolled children; it must not send
/// IO, create worlds, or spawn tasks. All source allocations need own admission.
pub trait DragMoveSource: Send + Sync {
    fn captures(&self) -> &DragCaptureFactory;
    fn start(&self, movement: DragToggleFuture) -> Result<DragMove>;
}
const MAX_OPERATIONS: usize = 1024;
/// Fixed storage, charged with the registry/driver; no per-round allocation.
#[derive(Default)]
struct SlotMask([u64; MAX_OPERATIONS / 64]);
impl SlotMask {
    fn contains(&self, slot: usize) -> bool {
        self.0[slot / 64] & (1_u64 << (slot % 64)) != 0
    }
    fn insert(&mut self, slot: usize) {
        self.0[slot / 64] |= 1_u64 << (slot % 64);
    }
    fn remove(&mut self, slot: usize) {
        self.0[slot / 64] &= !(1_u64 << (slot % 64));
    }
    fn is_empty(&self) -> bool {
        self.0.iter().all(|word| *word == 0)
    }
    fn count(&self) -> usize {
        self.0.iter().map(|word| word.count_ones() as usize).sum()
    }
}
struct Registry {
    occupied: SlotMask,
    quarantine_mask: SlotMask,
    quarantined: Vec<Option<Work>>,
    stopped: bool,
}
struct Shared {
    client: Arc<dyn CdpClient>,
    page: DragPage,
    source: Arc<dyn DragMoveSource>,
    ledger: ResourceLedger,
    scope: ResourceScope,
    registry: Mutex<Registry>,
    _storage: ResourceLease,
}
#[derive(Clone)]
pub struct MouseDragDispatcher(Arc<Shared>);
/// Unique caller-polled cleanup driver, returned with the dispatcher. Keep it
/// with the Page and drain quarantined native owners before transport teardown.
pub struct MouseDragRetirementDriver {
    shared: Arc<Shared>,
    round: bool,
    done: SlotMask,
    failure: Option<CdpError>,
}
async fn actual_pointer(
    client: Option<Arc<dyn CdpClient>>,
    request: Option<MouseDispatchRequest>,
) -> Result<()> {
    let client = client.expect("bound Mouse client");
    let mut request = request.expect("admitted Mouse request");
    if matches!(request.kind, MouseDispatchKind::Move { .. }) {
        let live = request.live_state.snapshot();
        request.event.buttons = Some(i64::from(live.buttons));
        request.event.modifiers = Some(i64::from(live.modifiers));
        request.event.force = Some(if live.buttons != 0 { 0.5 } else { 0.0 });
    }
    client.input_dispatch_pointer_event(request.event).await
}
fn operation_bytes() -> usize {
    std::mem::size_of::<Operation>() + std::mem::size_of_val(&actual_pointer(None, None)) + 32
}
fn state_bytes(capacity: usize) -> usize {
    std::mem::size_of::<Shared>()
        + 2 * std::mem::size_of::<usize>()
        + capacity * std::mem::size_of::<Option<Work>>()
        + std::mem::size_of::<MouseDragRetirementDriver>()
}
impl MouseDragDispatcher {
    /// Fixed dispatcher/driver/slot/operation and native wrapper payloads only.
    /// The outer request type string is bounded to 32 capacity bytes. Child
    /// move/Page/source/client futures, wire trees and errors need own budgets.
    pub fn requirements(capacity: usize) -> Result<ResourceRequirements> {
        if !(1..=MAX_OPERATIONS).contains(&capacity) {
            return Err(error("Mouse drag operation capacity must be 1..=1024"));
        }
        let bytes = (state_bytes(capacity) + capacity * operation_bytes()) as u64;
        Ok(ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, capacity + 1))
    }
    pub fn new(
        client: Arc<dyn CdpClient>,
        page: DragPage,
        source: Arc<dyn DragMoveSource>,
        scope: ResourceScope,
        capacity: usize,
        resources: &ProductionResourceLedger,
    ) -> Result<(Self, MouseDragRetirementDriver)> {
        if !page.matches_dispatch(&client, source.captures()) {
            return Err(error("Mouse drag source/Page/native bindings differ"));
        }
        let ledger = resources
            .claim(Self::requirements(capacity)?)
            .map_err(|_| error("Mouse drag resource claim failed"))?;
        let storage = ledger
            .reserve(
                scope,
                ResourceClass::BackendStateBytes,
                state_bytes(capacity) as u64,
            )
            .map_err(|_| error("Mouse drag state admission failed"))?;
        let mut slots = Vec::with_capacity(capacity);
        slots.resize_with(capacity, || None);
        let shared = Arc::new(Shared {
            client,
            page,
            source,
            scope,
            ledger,
            registry: Mutex::new(Registry {
                occupied: SlotMask::default(),
                quarantine_mask: SlotMask::default(),
                quarantined: slots,
                stopped: false,
            }),
            _storage: storage,
        });
        Ok((
            Self(shared.clone()),
            MouseDragRetirementDriver {
                shared,
                round: false,
                done: SlotMask::default(),
                failure: None,
            },
        ))
    }
    /// Prevent new result work. Existing operations must still be retired by
    /// their owners; this does not discard them or cancel persistent Page data.
    pub fn stop(&self) {
        self.0
            .registry
            .lock()
            .expect("Mouse drag registry poisoned")
            .stopped = true;
    }
    pub fn is_drained(&self) -> bool {
        self.0
            .registry
            .lock()
            .expect("Mouse drag registry poisoned")
            .occupied
            .is_empty()
    }
    pub fn quarantined_operations(&self) -> usize {
        self.0
            .registry
            .lock()
            .expect("Mouse drag registry poisoned")
            .quarantine_mask
            .count()
    }
}
struct Rejected;
impl MouseDispatchOperation for Rejected {
    fn poll_result(&mut self, _: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Err(error(
            "Mouse drag binding, admission or lifecycle rejected dispatch",
        )))
    }
    fn poll_retire(&mut self, _: &mut Context<'_>) -> Poll<Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn is_retired(&self) -> bool {
        true
    }
}
impl Future for Rejected {
    type Output = Result<bool>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(Err(error(
            "drag Escape binding, admission or lifecycle rejected",
        )))
    }
}
impl CdpKeyDownInterceptor for MouseDragDispatcher {
    fn intercept(
        self: Arc<Self>,
        client: Arc<dyn CdpClient>,
        code: &'static str,
    ) -> Option<KeyDownInterceptionFuture> {
        if code != "Escape" {
            return None;
        }
        if !Arc::ptr_eq(&client, &self.0.client) {
            return Some(Box::pin(Rejected));
        }
        Some(match self.admit(None, true) {
            Ok(operation) => Box::pin(EscapeOperation(operation)),
            Err(_) => Box::pin(Rejected),
        })
    }
}
struct EscapeOperation(Operation);
impl Future for EscapeOperation {
    type Output = Result<bool>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.poll_result(cx).map(|result| {
            result.map(|()| {
                self.0
                    .work
                    .as_ref()
                    .expect("retained Escape owner")
                    .cancelled
            })
        })
    }
}
impl CdpMouseDispatcher for MouseDragDispatcher {
    fn start(
        &self,
        client: Arc<dyn CdpClient>,
        request: MouseDispatchRequest,
    ) -> Box<dyn MouseDispatchOperation> {
        // A zero-sized rejected owner makes no heap allocation before admission.
        if !Arc::ptr_eq(&client, &self.0.client)
            || request
                .event
                .r#type
                .capacity()
                .checked_add(
                    request
                        .event
                        .pointer_type
                        .as_ref()
                        .map_or(0, String::capacity),
                )
                .is_none_or(|bytes| bytes > 32)
            || !request.event.x.is_finite()
            || !request.event.y.is_finite()
        {
            return Box::new(Rejected);
        }
        match self.admit(Some(request), false) {
            Ok(operation) => Box::new(operation),
            Err(_) => Box::new(Rejected),
        }
    }
}
impl MouseDragDispatcher {
    fn admit(
        &self,
        request: Option<MouseDispatchRequest>,
        cancel: bool,
    ) -> std::result::Result<Operation, ()> {
        let mut registry = self
            .0
            .registry
            .lock()
            .expect("Mouse drag registry poisoned");
        if registry.stopped || !registry.quarantine_mask.is_empty() {
            return Err(());
        }
        let Some(slot) =
            (0..registry.quarantined.len()).find(|slot| !registry.occupied.contains(*slot))
        else {
            return Err(());
        };
        let Ok(storage) = self.0.ledger.reserve(
            self.0.scope,
            ResourceClass::BackendStateBytes,
            operation_bytes() as u64,
        ) else {
            return Err(());
        };
        registry.occupied.insert(slot);
        drop(registry);
        Ok(Operation {
            shared: self.0.clone(),
            slot,
            observed: false,
            work: Some(Work {
                request,
                cancel_pending: cancel,
                cancelled: false,
                direct: None,
                direct_started: false,
                movement: None,
                command: None,
                movement_finished: false,
                position: DragPosition { x: 0.0, y: 0.0 },
                live: None,
                started: false,
                _storage: storage,
            }),
        })
    }
}
struct Work {
    request: Option<MouseDispatchRequest>,
    cancel_pending: bool,
    cancelled: bool,
    direct: Option<DragToggleFuture>,
    direct_started: bool,
    movement: Option<DragMove>,
    command: Option<DragPageOperation>,
    movement_finished: bool,
    position: DragPosition,
    live: Option<MouseStateReader>,
    started: bool,
    _storage: ResourceLease,
}
impl Work {
    fn begin(&mut self, shared: &Shared) -> Result<()> {
        self.started = true;
        let request = self.request.take().expect("fresh Mouse drag request");
        self.position = DragPosition {
            x: request.event.x,
            y: request.event.y,
        };
        match request.kind {
            MouseDispatchKind::Move { for_click: true } => {
                self.direct = Some(Box::pin(actual_pointer(
                    Some(shared.client.clone()),
                    Some(request),
                )));
            }
            MouseDispatchKind::Move { for_click: false } => {
                shared.page.record_position(self.position)?;
                let live = request.live_state.snapshot();
                if shared.page.is_dragging() {
                    self.command =
                        Some(shared.page.over(self.position, i64::from(live.modifiers))?);
                } else if request.event.button != Some(MouseButton::Left) {
                    self.direct = Some(Box::pin(actual_pointer(
                        Some(shared.client.clone()),
                        Some(request),
                    )));
                } else {
                    self.live = Some(request.live_state.clone());
                    self.movement = Some(shared.source.start(Box::pin(actual_pointer(
                        Some(shared.client.clone()),
                        Some(request),
                    )))?);
                }
            }
            MouseDispatchKind::Down => {
                if !shared.page.is_dragging() {
                    self.direct = Some(Box::pin(actual_pointer(
                        Some(shared.client.clone()),
                        Some(request),
                    )));
                }
            }
            MouseDispatchKind::Up => {
                if shared.page.is_dragging() {
                    self.command = Some(
                        shared
                            .page
                            .drop_at(self.position, request.event.modifiers.unwrap_or(0))?,
                    );
                } else {
                    self.direct = Some(Box::pin(actual_pointer(
                        Some(shared.client.clone()),
                        Some(request),
                    )));
                }
            }
        }
        Ok(())
    }
    fn is_retired(&self) -> bool {
        !self.cancel_pending
            && self.request.is_none()
            && self.direct.is_none()
            && self.movement.as_ref().is_none_or(DragMove::is_retired)
            && self
                .command
                .as_ref()
                .is_none_or(DragPageOperation::is_retired)
    }
    fn poll_result(&mut self, shared: &Shared, cx: &mut Context<'_>) -> Poll<Result<()>> {
        if self.cancel_pending {
            self.cancel_pending = false;
            self.started = true;
            self.command = Some(shared.page.cancel()?);
        }
        if self.request.is_some() {
            self.begin(shared)?;
        }
        loop {
            if let Some(direct) = &mut self.direct {
                self.direct_started = true;
                let result = direct.as_mut().poll(cx);
                if result.is_ready() {
                    self.direct = None;
                }
                return result;
            }
            if let Some(command) = &mut self.command {
                let result = Pin::new(command).poll(cx).map(|result| {
                    result.map(|cancelled| {
                        self.cancelled = cancelled;
                    })
                });
                // First polling enter installs Page state before releasing the
                // completed capture interval. No false idle window is created.
                if self.movement_finished {
                    self.movement = None;
                }
                return result;
            }
            if let Some(movement) = &mut self.movement {
                match movement.poll_result(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                    Poll::Ready(Ok(None)) => {
                        self.movement = None;
                        return Poll::Ready(Ok(()));
                    }
                    Poll::Ready(Ok(Some(data))) => {
                        self.movement_finished = true;
                        let modifiers = self
                            .live
                            .as_ref()
                            .expect("retained live Mouse state")
                            .snapshot()
                            .modifiers;
                        self.command = Some(shared.page.enter(
                            movement,
                            data,
                            self.position,
                            i64::from(modifiers),
                        )?);
                    }
                }
            } else {
                return Poll::Ready(Ok(()));
            }
        }
    }
    fn poll_retire(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.request = None;
        self.cancel_pending = false;
        if !self.direct_started {
            self.direct = None;
        }
        if let Some(direct) = &mut self.direct {
            let Poll::Ready(result) = direct.as_mut().poll(cx) else {
                return Poll::Pending;
            };
            self.direct = None;
            result?;
        }
        if let Some(command) = &mut self.command {
            let Poll::Ready(result) = std::pin::pin!(command.retire()).poll(cx) else {
                return Poll::Pending;
            };
            // Even an error here is a drained native receipt. Persistent drag
            // state remains admitted on DragPage, not hidden in this operation.
            self.command = None;
            result?;
        }
        if let Some(movement) = &mut self.movement {
            let result = movement.poll_retire(cx);
            if result.is_ready() && movement.is_retired() {
                self.movement = None;
            }
            return result;
        }
        Poll::Ready(Ok(()))
    }
}
struct Operation {
    shared: Arc<Shared>,
    slot: usize,
    work: Option<Work>,
    observed: bool,
}
impl Operation {
    fn release(&mut self) {
        let work = self.work.take();
        drop(work);
        self.shared
            .registry
            .lock()
            .expect("Mouse drag registry poisoned")
            .occupied
            .remove(self.slot);
    }
    fn quarantine(&mut self) {
        let mut registry = self
            .shared
            .registry
            .lock()
            .expect("Mouse drag registry poisoned");
        registry.quarantined[self.slot] = self.work.take();
        registry.quarantine_mask.insert(self.slot);
    }
}
impl MouseDispatchOperation for Operation {
    fn poll_result(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        if self.observed {
            return Poll::Ready(Err(error("Mouse drag result already observed")));
        }
        let blocked = {
            let registry = self
                .shared
                .registry
                .lock()
                .expect("Mouse drag registry poisoned");
            registry.stopped || !registry.quarantine_mask.is_empty()
        };
        let result = if blocked {
            Poll::Ready(Err(error("Mouse drag Page stopped or awaiting cleanup")))
        } else {
            self.work
                .as_mut()
                .expect("active Mouse drag owner")
                .poll_result(&self.shared, cx)
        };
        if result.is_ready() {
            self.observed = true;
        }
        result
    }
    fn poll_retire(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.observed = true;
        let Some(work) = &mut self.work else {
            return Poll::Ready(Ok(()));
        };
        let result = work.poll_retire(cx);
        if result.is_ready() {
            if work.is_retired() {
                self.release();
            } else {
                self.quarantine();
            }
        }
        result
    }
    fn is_retired(&self) -> bool {
        self.work.as_ref().is_none_or(Work::is_retired)
    }
}
impl Drop for Operation {
    fn drop(&mut self) {
        if let Some(work) = &self.work {
            if !work.started || work.is_retired() {
                self.release();
            } else {
                self.quarantine();
            }
        }
    }
}
impl MouseDragRetirementDriver {
    /// One fair cleanup round over quarantined owners. Errors do not drop dirty
    /// work or automatically retry it in the same round. A later explicit call
    /// retries; active outer operations must be separately retired by the host.
    pub async fn retire_quarantined(&mut self) -> Result<()> {
        poll_fn(|cx| self.poll_quarantined(cx)).await
    }
    pub fn poll_quarantined(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        if !self.round {
            self.round = true;
            self.done = SlotMask::default();
            self.failure = None;
        }
        let capacity = self
            .shared
            .registry
            .lock()
            .expect("Mouse drag registry poisoned")
            .quarantined
            .len();
        let mut pending = false;
        for slot in 0..capacity {
            if self.done.contains(slot) {
                continue;
            }
            let work = self
                .shared
                .registry
                .lock()
                .expect("Mouse drag registry poisoned")
                .quarantined[slot]
                .take();
            let Some(mut work) = work else {
                self.done.insert(slot);
                continue;
            };
            // Never hold registry locks across a child future poll.
            let result = work.poll_retire(cx);
            let retired = work.is_retired();
            if !retired {
                self.shared
                    .registry
                    .lock()
                    .expect("Mouse drag registry poisoned")
                    .quarantined[slot] = Some(work);
            } else {
                drop(work);
                let mut registry = self
                    .shared
                    .registry
                    .lock()
                    .expect("Mouse drag registry poisoned");
                registry.occupied.remove(slot);
                registry.quarantine_mask.remove(slot);
            }
            match result {
                Poll::Pending => pending = true,
                Poll::Ready(result) => {
                    self.done.insert(slot);
                    if let Err(error) = result {
                        self.failure.get_or_insert(error);
                    }
                }
            }
        }
        if pending {
            return Poll::Pending;
        }
        self.round = false;
        Poll::Ready(self.failure.take().map_or(Ok(()), Err))
    }
}

#[cfg(test)]
mod tests;
