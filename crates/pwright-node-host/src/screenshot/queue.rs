//! Host-selected, finite per-Page serialization. No task or timer is spawned.
use crate::HostError;
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceLedger,
    ResourceRequirements, ResourceScope,
};
use std::sync::{Arc, Mutex};

const CAPACITY: usize = 8;
const STATE_BYTES: u64 = 4096;
const WAITER_BYTES: u64 = 4096;
const STATE_SCOPE: ResourceScope = ResourceScope::extension(120, 16);

pub(crate) fn requirements() -> ResourceRequirements {
    ResourceRequirements::none()
        .with_class(
            ResourceClass::BackendStateBytes,
            STATE_BYTES + WAITER_BYTES * CAPACITY as u64,
            WAITER_BYTES * CAPACITY as u64,
        )
        .with_metadata(CAPACITY + 1, CAPACITY + 1)
}

struct Slot {
    page: ObjectId,
    gate: Arc<tokio::sync::Mutex<()>>,
    users: usize,
}

pub(crate) struct Queue {
    slots: Mutex<Vec<Slot>>,
    captures: tokio::sync::Semaphore,
    ledger: ResourceLedger,
    _state: ResourceLease,
}

impl Queue {
    pub(crate) async fn run<T>(
        &self,
        page: ObjectId,
        timeout_ms: u32,
        stopped: &tokio_util::sync::CancellationToken,
        capture: impl std::future::Future<Output = Result<T, HostError>>,
    ) -> Result<T, HostError> {
        let entry = self.reserve(page)?;
        let run = async {
            let _page = entry.gate.lock().await;
            let _capture = self.captures.acquire().await.map_err(HostError::backend)?;
            capture.await
        };
        tokio::select! {
            biased;
            () = stopped.cancelled() => Err(HostError::TargetClosed),
            result = async {
                if timeout_ms == 0 { run.await }
                else { tokio::time::timeout(std::time::Duration::from_millis(u64::from(timeout_ms)), run)
                    .await.map_err(|_| HostError::Timeout(timeout_ms))? }
            } => result,
        }
    }

    pub(crate) fn new(resources: &ProductionResourceLedger) -> Result<Self, HostError> {
        let ledger = resources
            .claim(requirements())
            .map_err(HostError::backend)?;
        let state = ledger
            .reserve(STATE_SCOPE, ResourceClass::BackendStateBytes, STATE_BYTES)
            .map_err(HostError::backend)?;
        Ok(Self {
            slots: Mutex::new(Vec::with_capacity(CAPACITY)),
            captures: tokio::sync::Semaphore::new(2),
            ledger,
            _state: state,
        })
    }

    pub(super) fn reserve(&self, page: ObjectId) -> Result<Entry<'_>, HostError> {
        let mut slots = self.slots.lock().map_err(HostError::backend)?;
        if slots.iter().map(|slot| slot.users).sum::<usize>() >= CAPACITY {
            return Err(HostError::ResourceExhausted(
                "screenshot queue capacity exceeded".into(),
            ));
        }
        let lease = self
            .ledger
            .reserve(
                ResourceScope::Object(page),
                ResourceClass::BackendStateBytes,
                WAITER_BYTES,
            )
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let gate = if let Some(slot) = slots.iter_mut().find(|slot| slot.page == page) {
            slot.users += 1;
            slot.gate.clone()
        } else {
            let gate = Arc::new(tokio::sync::Mutex::new(()));
            slots.push(Slot {
                page,
                gate: gate.clone(),
                users: 1,
            });
            gate
        };
        Ok(Entry {
            queue: self,
            page,
            gate,
            _lease: lease,
        })
    }
}

pub(super) struct Entry<'a> {
    queue: &'a Queue,
    page: ObjectId,
    pub(super) gate: Arc<tokio::sync::Mutex<()>>,
    _lease: ResourceLease,
}

impl Drop for Entry<'_> {
    fn drop(&mut self) {
        let mut slots = self
            .queue
            .slots
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(index) = slots.iter().position(|slot| slot.page == self.page) {
            slots[index].users -= 1;
            if slots[index].users == 0 {
                slots.swap_remove(index);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_core::ResourceLimits;

    fn resources() -> ProductionResourceLedger {
        let request = requirements();
        let bound = request.class(ResourceClass::BackendStateBytes);
        ProductionResourceLedger::new(
            ResourceLimits::deny_all(request.scopes, request.leases).with_class(
                ResourceClass::BackendStateBytes,
                bound.global,
                bound.per_scope,
            ),
            request,
        )
        .unwrap()
    }

    #[test]
    fn queue_rejects_ninth_entry_and_reclaims_cancelled_admission() {
        // Arrange
        let resources = resources();
        let queue = Queue::new(&resources).unwrap();
        let mut entries: Vec<_> = (0..8)
            .map(|_| queue.reserve(ObjectId::new(1)).unwrap())
            .collect();
        // Act
        let rejected = queue.reserve(ObjectId::new(1));
        // Assert
        assert!(matches!(rejected, Err(HostError::ResourceExhausted(_))));
        entries.pop();
        assert!(queue.reserve(ObjectId::new(1)).is_ok());
        drop(entries);
        assert!(queue.slots.lock().unwrap().is_empty());
    }

    #[test]
    fn same_page_shares_a_gate_and_different_pages_do_not() {
        // Arrange
        let resources = resources();
        let queue = Queue::new(&resources).unwrap();
        let first = queue.reserve(ObjectId::new(1)).unwrap();
        let second = queue.reserve(ObjectId::new(1)).unwrap();
        let other = queue.reserve(ObjectId::new(2)).unwrap();
        let _locked = first.gate.try_lock().unwrap();
        // Act
        let same_page = second.gate.try_lock();
        let other_page = other.gate.try_lock();
        // Assert
        assert!(same_page.is_err());
        assert!(other_page.is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn cancelled_waiter_releases_its_slot_without_unlocking_the_owner() {
        use std::future::Future;
        use std::task::{Context, Poll, Waker};
        // Arrange
        let resources = resources();
        let queue = Queue::new(&resources).unwrap();
        let owner = queue.reserve(ObjectId::new(1)).unwrap();
        let locked = owner.gate.lock().await;
        let waiting = queue.reserve(ObjectId::new(1)).unwrap();
        let mut future = Box::pin(waiting.gate.lock());
        assert!(matches!(
            future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        ));
        // Act
        drop(future);
        drop(waiting);
        // Assert
        assert_eq!(queue.slots.lock().unwrap()[0].users, 1);
        assert!(owner.gate.try_lock().is_err());
        drop(locked);
        drop(owner);
        assert!(queue.slots.lock().unwrap().is_empty());
    }

    #[test]
    fn queue_storage_is_covered_by_its_reserved_metadata_bytes() {
        // Arrange
        let bytes = std::mem::size_of::<Queue>()
            + CAPACITY
                * (std::mem::size_of::<Slot>()
                    + std::mem::size_of::<tokio::sync::Mutex<()>>()
                    + 64);
        // Act
        let covered = bytes <= STATE_BYTES as usize;
        // Assert
        assert!(covered);
    }

    #[tokio::test(start_paused = true)]
    async fn deadline_includes_queue_wait_and_does_not_start_capture() {
        // Arrange
        let resources = resources();
        let queue = Queue::new(&resources).unwrap();
        let owner = queue.reserve(ObjectId::new(1)).unwrap();
        let _locked = owner.gate.lock().await;
        let stopped = tokio_util::sync::CancellationToken::new();
        let called = std::cell::Cell::new(false);
        // Act
        let result = queue
            .run(ObjectId::new(1), 5, &stopped, async {
                called.set(true);
                Ok(())
            })
            .await;
        // Assert
        assert!(matches!(result, Err(HostError::Timeout(5))));
        assert!(!called.get());
        assert_eq!(queue.slots.lock().unwrap()[0].users, 1);
    }

    #[tokio::test]
    async fn shutdown_cancels_unlimited_queue_wait_without_capture() {
        // Arrange
        let resources = resources();
        let queue = Queue::new(&resources).unwrap();
        let owner = queue.reserve(ObjectId::new(1)).unwrap();
        let _locked = owner.gate.lock().await;
        let stopped = tokio_util::sync::CancellationToken::new();
        stopped.cancel();
        let called = std::cell::Cell::new(false);
        // Act
        let result = queue
            .run(ObjectId::new(1), 0, &stopped, async {
                called.set(true);
                Ok(())
            })
            .await;
        // Assert
        assert!(matches!(result, Err(HostError::TargetClosed)));
        assert!(!called.get());
        assert_eq!(queue.slots.lock().unwrap()[0].users, 1);
    }
}
