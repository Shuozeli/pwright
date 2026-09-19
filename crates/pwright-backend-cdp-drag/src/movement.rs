//! Caller-polled composition of temporary listeners, capture and native movement.
use crate::{
    AdmittedDragData, DragCapture, DragCaptureFactory, DragInterceptionClient, DragListener,
    DragListenerFactory, DragToggleFuture, error,
};
use pwright_cdp::{
    CdpClient,
    connection::{CdpError, Result},
};
use pwright_core::{
    ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements, ResourceScope,
};
use std::{
    future::{Future, poll_fn},
    pin::pin,
    sync::Arc,
    task::{Context, Poll},
};

/// Already selected existing utility world; no discovery or world creation is
/// performed here. The caller must retain the corresponding native lifetime.
pub struct DragWorld<'a> {
    pub client: Arc<dyn CdpClient>,
    pub unique_context: &'a str,
}

/// Retained ownership of caller-selected worlds and their independent native
/// destruction evidence. Implementations must be monotonic, nonblocking and
/// effect-free. A replacement route, crash, session detach or transport error
/// alone MUST NOT return true. The exact client AND unique world must match.
/// The caller separately budgets this owner's allocation tree before supplying it.
pub trait DragWorldLifetime: Send + Sync {
    fn world_destroyed(&self, client: &Arc<dyn CdpClient>, unique_context: &str) -> bool;
}
struct World {
    listener: DragListener,
    prepared: bool,
    observed: bool,
    cleanup_observed: bool,
}
#[derive(Clone, Copy, Eq, PartialEq)]
enum Stage {
    Preparing,
    Enabling,
    Moving,
    Observing,
    Capturing,
    Disabling,
    Done,
}

/// One move's temporary native ownership. A result is independent of retirement:
/// after failure/cancellation keep this object through cleanup (or Page-owned
/// quarantine after uncertain cleanup). Drop never acknowledges native cleanup.
#[must_use = "retain the move through cleanup of all native children"]
pub struct DragMove {
    worlds: Vec<World>,
    capture: DragCapture,
    movement: Option<DragToggleFuture>,
    movement_started: bool,
    stage: Stage,
    expects_drag: bool,
    data: Option<AdmittedDragData>,
    retiring: bool,
    retirement_active: bool,
    capture_cleanup_observed: bool,
    retirement_error: Option<CdpError>,
    lifetime: Option<Arc<dyn DragWorldLifetime>>,
    _storage: ResourceLease,
}
fn poll_borrowed<F: Future>(future: F, cx: &mut Context<'_>) -> Poll<F::Output> {
    pin!(future).poll(cx)
}
impl DragMove {
    pub(crate) fn owns_captured_data(
        &self,
        captures: &DragCaptureFactory,
        data: &AdmittedDragData,
    ) -> bool {
        self.is_retired() && captures.owns_data(&self.capture, data)
    }
    /// Outer owner/vector payload only. Listener/capture/data factories charge
    /// their own allocations; caller-supplied movement futures, shared native
    /// lifetimes, replies/errors and allocator overhead require separate budgets.
    pub fn requirements(worlds: usize) -> Result<ResourceRequirements> {
        if worlds > 32 {
            return Err(error("drag move supports at most 32 existing worlds"));
        }
        let bytes = (std::mem::size_of::<Self>() + worlds * std::mem::size_of::<World>()) as u64;
        Ok(ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, 1))
    }
    /// Admits before allocating the world vector. All children are created fresh
    /// and unpolled here, so construction failure cannot leave native work.
    /// `movement` must be a fresh lazy future: it should sample live Mouse
    /// buttons/modifiers when first polled, not when this owner is constructed.
    pub fn start(
        scope: ResourceScope,
        resources: &ProductionResourceLedger,
        listeners: &DragListenerFactory,
        captures: &DragCaptureFactory,
        worlds: &[DragWorld<'_>],
        movement: DragToggleFuture,
    ) -> Result<Self> {
        Self::start_with_world_lifetime(
            scope, resources, listeners, captures, worlds, movement, None,
        )
    }
    /// Same fresh, non-polling construction, additionally retaining the caller's
    /// world snapshot/proof owner through result, retirement and quarantine until
    /// this move is dropped. `None` preserves the explicit-acknowledgment API.
    pub fn start_with_world_lifetime(
        scope: ResourceScope,
        resources: &ProductionResourceLedger,
        listeners: &DragListenerFactory,
        captures: &DragCaptureFactory,
        worlds: &[DragWorld<'_>],
        movement: DragToggleFuture,
        lifetime: Option<Arc<dyn DragWorldLifetime>>,
    ) -> Result<Self> {
        let requirements = Self::requirements(worlds.len())?;
        let ledger = resources
            .claim(requirements)
            .map_err(|_| error("drag move resource claim failed"))?;
        let storage = ledger
            .reserve(
                scope,
                ResourceClass::BackendStateBytes,
                requirements.class(ResourceClass::BackendStateBytes).global,
            )
            .map_err(|_| error("drag move storage admission failed"))?;
        let capture = captures.start()?;
        let mut owned = Vec::with_capacity(worlds.len());
        for world in worlds {
            owned.push(World {
                listener: listeners.start(world.client.clone(), world.unique_context)?,
                prepared: false,
                observed: false,
                cleanup_observed: false,
            });
        }
        Ok(Self {
            worlds: owned,
            capture,
            movement: Some(movement),
            movement_started: false,
            stage: Stage::Preparing,
            expects_drag: false,
            data: None,
            retiring: false,
            retirement_active: false,
            capture_cleanup_observed: false,
            retirement_error: None,
            lifetime,
            _storage: storage,
        })
    }
    pub fn is_retired(&self) -> bool {
        self.movement.is_none()
            && self.capture.is_retired()
            && self.worlds.iter().all(|world| world.listener.is_retired())
    }
    fn poll_movement(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.movement_started = true;
        let result = self
            .movement
            .as_mut()
            .expect("retained native movement")
            .as_mut()
            .poll(cx);
        if result.is_ready() {
            self.movement = None;
        }
        result
    }
    /// Polls each world's setup/observation without serializing independent
    /// worlds. An error stops further action planning; retirement still drives
    /// every child already admitted/started. World discovery/non-stalling
    /// navigation policy belongs to the caller, not to this snapshot consumer.
    pub fn poll_result(&mut self, cx: &mut Context<'_>) -> Poll<Result<Option<AdmittedDragData>>> {
        if self.retiring || self.stage == Stage::Done {
            return Poll::Ready(Err(error("drag move result already observed or cancelled")));
        }
        let result = self.poll_action(cx);
        if result.is_ready() {
            self.stage = Stage::Done;
        }
        result
    }
    fn poll_action(&mut self, cx: &mut Context<'_>) -> Poll<Result<Option<AdmittedDragData>>> {
        loop {
            match self.stage {
                Stage::Preparing => {
                    let mut pending = false;
                    for world in &mut self.worlds {
                        if world.prepared {
                            continue;
                        }
                        match poll_borrowed(world.listener.prepare(), cx) {
                            Poll::Pending => pending = true,
                            Poll::Ready(Ok(())) => world.prepared = true,
                            Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                        }
                    }
                    if pending {
                        return Poll::Pending;
                    }
                    self.stage = Stage::Enabling;
                }
                Stage::Enabling => match poll_borrowed(self.capture.enable(), cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Ok(())) => self.stage = Stage::Moving,
                    Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                },
                Stage::Moving => match self.poll_movement(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Ok(())) => self.stage = Stage::Observing,
                    Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                },
                Stage::Observing => {
                    let mut pending = false;
                    for world in &mut self.worlds {
                        if world.observed {
                            continue;
                        }
                        match poll_borrowed(world.listener.observe(), cx) {
                            Poll::Pending => pending = true,
                            Poll::Ready(Ok(started)) => {
                                world.observed = true;
                                self.expects_drag |= started;
                            }
                            Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                        }
                    }
                    if pending {
                        return Poll::Pending;
                    }
                    self.stage = if self.expects_drag {
                        Stage::Capturing
                    } else {
                        Stage::Disabling
                    };
                }
                Stage::Capturing => match poll_borrowed(self.capture.receive(), cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Ok(data)) => {
                        self.data = Some(data);
                        self.stage = Stage::Disabling;
                    }
                    Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                },
                Stage::Disabling => match poll_borrowed(self.capture.retire(), cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Ok(())) => return Poll::Ready(Ok(self.data.take())),
                    Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                },
                Stage::Done => unreachable!("result state checked before action"),
            }
        }
    }
    pub async fn result(&mut self) -> Result<Option<AdmittedDragData>> {
        poll_fn(|cx| self.poll_result(cx)).await
    }
    /// Drains actual movement before disabling interception, and drives listener
    /// cleanup fairly even while another child's cleanup is pending. One failed
    /// child does not abandon the others. A failed retirement round can be
    /// explicitly retried; do not discard this owner if is_retired is false.
    pub fn poll_retire(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.retiring = true;
        if !self.retirement_active {
            self.retirement_active = true;
            self.retirement_error = None;
            self.capture_cleanup_observed = self.capture.is_retired();
            for world in &mut self.worlds {
                world.cleanup_observed = world.listener.is_retired();
            }
        }
        if !self.movement_started {
            self.movement = None;
        }
        let mut pending = false;
        if self.movement.is_some() {
            match self.poll_movement(cx) {
                Poll::Pending => pending = true,
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(error)) => {
                    self.retirement_error.get_or_insert(error);
                }
            }
        }
        // Never disable while trusted input still has an outstanding receipt.
        if self.movement.is_none() && !self.capture_cleanup_observed {
            match poll_borrowed(self.capture.retire(), cx) {
                Poll::Pending => pending = true,
                Poll::Ready(result) => {
                    self.capture_cleanup_observed = true;
                    if let Err(error) = result {
                        self.retirement_error.get_or_insert(error);
                    }
                }
            }
        }
        for world in &mut self.worlds {
            if world.cleanup_observed {
                continue;
            }
            let destroyed = self.lifetime.as_ref().is_some_and(|lifetime| {
                lifetime.world_destroyed(&world.listener.client, world.listener.unique_context())
            });
            let result = if destroyed {
                world.listener.poll_retire_destroyed(cx)
            } else {
                poll_borrowed(world.listener.retire(), cx)
            };
            match result {
                Poll::Pending => pending = true,
                Poll::Ready(result) => {
                    world.cleanup_observed = true;
                    if let Err(error) = result {
                        self.retirement_error.get_or_insert(error);
                    }
                }
            }
        }
        if pending {
            return Poll::Pending;
        }
        self.retirement_active = false;
        Poll::Ready(self.retirement_error.take().map_or(Ok(()), Err))
    }
    pub async fn retire(&mut self) -> Result<()> {
        poll_fn(|cx| self.poll_retire(cx)).await
    }
    /// Independently verified native destruction only, never a replacement world.
    pub fn acknowledge_destroyed_world(
        &mut self,
        client: &Arc<dyn CdpClient>,
        unique_context: &str,
    ) -> Result<()> {
        let mut found = false;
        for world in &mut self.worlds {
            if Arc::ptr_eq(&world.listener.client, client)
                && world.listener.unique_context() == unique_context
            {
                world.listener.acknowledge_destroyed_world(unique_context)?;
                found = true;
            }
        }
        if found {
            self.retiring = true;
            Ok(())
        } else {
            Err(error("destroyed world does not belong to drag move"))
        }
    }
    pub fn acknowledge_destroyed_session(
        &mut self,
        client: &Arc<dyn DragInterceptionClient>,
    ) -> Result<()> {
        if self.movement.is_some() && self.movement_started {
            return Err(error(
                "native movement receipt must be drained before destruction acknowledgement",
            ));
        }
        self.capture.acknowledge_destroyed_session(client)?;
        self.retiring = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
