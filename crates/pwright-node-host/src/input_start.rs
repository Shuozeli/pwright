//! Bounded synchronous submission order for the first poll of raw input calls.
//! This is not a transaction lock: native replies and later continuations overlap.
use crate::{HostError, HostSession};
use pwright_core::{
    ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements, ResourceScope,
};
use std::{
    future::{Future, poll_fn},
    pin::pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};
use tokio_util::task::task_tracker::TaskTrackerToken;

const CAPACITY: usize = 32;
const SCOPE: ResourceScope = ResourceScope::extension(120, 22);
struct Slot {
    id: u64,
    running: bool,
    waker: Option<Waker>,
}
struct State {
    next: u64,
    closed: bool,
    slots: [Option<Slot>; CAPACITY],
}
struct Inner {
    state: Mutex<State>,
    _storage: ResourceLease,
}
#[derive(Clone)]
pub(crate) struct Order(Arc<Inner>);

pub(crate) fn requirements() -> ResourceRequirements {
    // Includes the concrete fixed array, mutex, lease and Arc control words.
    // Waker implementations and the ordered future retain their caller budgets.
    let bytes = (std::mem::size_of::<Inner>() + 2 * std::mem::size_of::<usize>()) as u64;
    ResourceRequirements::none()
        .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
        .with_metadata(1, 1)
}
impl State {
    fn head(&self) -> Option<usize> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| slot.as_ref().map(|s| (i, s.id)))
            .min_by_key(|(_, id)| *id)
            .map(|(i, _)| i)
    }
}
impl Order {
    pub(crate) fn new(resources: &ProductionResourceLedger) -> Result<Self, HostError> {
        let bytes = (std::mem::size_of::<Inner>() + 2 * std::mem::size_of::<usize>()) as u64;
        let ledger = resources
            .claim(requirements())
            .map_err(HostError::backend)?;
        let storage = ledger
            .reserve(SCOPE, ResourceClass::BackendStateBytes, bytes)
            .map_err(HostError::backend)?;
        Ok(Self(Arc::new(Inner {
            state: Mutex::new(State {
                next: 1,
                closed: false,
                slots: std::array::from_fn(|_| None),
            }),
            _storage: storage,
        })))
    }
    fn admit(&self, lifetime: Option<TaskTrackerToken>) -> Result<InputStartTicket, HostError> {
        let mut state = self.0.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        let index = state
            .slots
            .iter()
            .position(Option::is_none)
            .ok_or_else(|| {
                HostError::ResourceExhausted("raw input start capacity exceeded (32)".into())
            })?;
        let id = state.next;
        state.next = id.checked_add(1).ok_or_else(|| {
            HostError::ResourceExhausted("raw input start identity exhausted".into())
        })?;
        state.slots[index] = Some(Slot {
            id,
            running: false,
            waker: None,
        });
        Ok(InputStartTicket {
            order: self.0.clone(),
            index,
            id,
            _lifetime: lifetime,
        })
    }
    pub(crate) fn close(&self) {
        let wake = {
            let mut state = self.0.state.lock().unwrap_or_else(|e| e.into_inner());
            state.closed = true;
            std::array::from_fn::<_, CAPACITY, _>(|i| {
                state.slots[i].as_mut().and_then(|slot| slot.waker.take())
            })
        };
        for waker in wake.into_iter().flatten() {
            waker.wake();
        }
    }
}

impl Drop for Order {
    fn drop(&mut self) {
        // Owner teardown (including topology-driver failure) wakes queued calls.
        // Tickets retain storage but are not installation owners.
        self.close();
    }
}

/// A synchronous raw-input submission. Run or drop it; shutdown joins its lifetime.
/// The first poll alone is ordered. A queued runtime admission or a later native
/// continuation is not made atomic by this ticket, nor is HTML drag implemented.
pub struct InputStartTicket {
    order: Arc<Inner>,
    index: usize,
    id: u64,
    _lifetime: Option<TaskTrackerToken>,
}
impl InputStartTicket {
    fn turn(&self, cx: &Context<'_>) -> Poll<Result<(), HostError>> {
        let mut state = match self.order.state.lock() {
            Ok(state) => state,
            Err(error) => return Poll::Ready(Err(HostError::backend(error))),
        };
        if state.closed {
            return Poll::Ready(Err(HostError::TargetClosed));
        }
        let head = state.head();
        let slot = state.slots[self.index]
            .as_mut()
            .expect("live start ticket owns its slot");
        assert_eq!(slot.id, self.id);
        if head == Some(self.index) && !slot.running {
            slot.running = true;
            Poll::Ready(Ok(()))
        } else {
            slot.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
    pub async fn run<T>(
        self,
        future: impl Future<Output = Result<T, HostError>>,
    ) -> Result<T, HostError> {
        let mut first = Some(self);
        let mut future = pin!(future);
        poll_fn(move |cx| {
            if let Some(ticket) = first.as_ref() {
                match ticket.turn(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Err(error)) => {
                        first.take();
                        return Poll::Ready(Err(error));
                    }
                    Poll::Ready(Ok(())) => {}
                }
            }
            // Drop this guard only AFTER first poll returns (including unwind).
            // Never hold the order mutex while polling injected/native code.
            let guard = first.take();
            let result = future.as_mut().poll(cx);
            drop(guard);
            result
        })
        .await
    }
}
impl Drop for InputStartTicket {
    fn drop(&mut self) {
        let wake = {
            let mut state = self.order.state.lock().unwrap_or_else(|e| e.into_inner());
            let slot = state.slots[self.index]
                .take()
                .expect("live start ticket owns its slot");
            assert_eq!(slot.id, self.id);
            state
                .head()
                .and_then(|index| state.slots[index].as_mut())
                .filter(|slot| !slot.running)
                .and_then(|slot| slot.waker.take())
        };
        if let Some(waker) = wake {
            waker.wake();
        }
    }
}
impl HostSession {
    /// Call after foreign argument conversion but before spawning a raw input
    /// future. Mouse and Keyboard share this session's order; no JS queue needed.
    pub fn admit_input_start(&self) -> Result<InputStartTicket, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if self.stop.is_cancelled() || self.disconnected.is_cancelled() {
            return Err(HostError::TargetClosed);
        }
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        live.input_start.admit(Some(self.operations.token()))
    }
}

#[cfg(test)]
mod tests;
