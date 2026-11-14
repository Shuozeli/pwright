//! Optional bounded Rust-owned timeout waiting.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

use pwright_core::{
    ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements, ResourceScope,
};

use crate::{HostError, HostSession};

pub mod profile {
    pub const OPERATIONS: u64 = 16;
    pub const BYTES: u64 = 4 * 1024 + OPERATIONS * 512;
}

const MAX_OPERATIONS: u64 = profile::OPERATIONS;
const OPERATION_BYTES: u64 = 512;
const STORAGE_BYTES: u64 = 4 * 1024;
const STORAGE_SCOPE: ResourceScope = ResourceScope::extension(140, 1);
const OPERATION_SCOPE: ResourceScope = ResourceScope::extension(140, 2);

struct Slots {
    active: AtomicU64,
    resources: ProductionResourceLedger,
    _storage: ResourceLease,
}

struct OperationPermit {
    owner: Arc<Slots>,
    _storage: ResourceLease,
}

impl Drop for OperationPermit {
    fn drop(&mut self) {
        self.owner.active.fetch_sub(1, Ordering::AcqRel);
    }
}

impl Slots {
    fn new(resources: &ProductionResourceLedger) -> Result<Arc<Self>, HostError> {
        let storage = resources
            .claim(storage_requirements())
            .map_err(HostError::backend)?
            .reserve(
                STORAGE_SCOPE,
                ResourceClass::BackendStateBytes,
                STORAGE_BYTES,
            )
            .map_err(HostError::backend)?;
        Ok(Arc::new(Self {
            active: AtomicU64::new(0),
            resources: resources.clone(),
            _storage: storage,
        }))
    }

    fn admit(self: &Arc<Self>) -> Result<OperationPermit, HostError> {
        self.active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < MAX_OPERATIONS).then_some(active + 1)
            })
            .map_err(|_| {
                HostError::ResourceExhausted(format!(
                    "wait_for_timeout capacity exceeded ({MAX_OPERATIONS})"
                ))
            })?;
        let storage = self
            .resources
            .claim(operation_requirements())
            .map_err(HostError::backend)
            .and_then(|ledger| {
                ledger
                    .reserve(
                        OPERATION_SCOPE,
                        ResourceClass::BackendStateBytes,
                        OPERATION_BYTES,
                    )
                    .map_err(HostError::backend)
            });
        match storage {
            Ok(storage) => Ok(OperationPermit {
                owner: self.clone(),
                _storage: storage,
            }),
            Err(error) => {
                self.active.fetch_sub(1, Ordering::AcqRel);
                Err(error)
            }
        }
    }
}

pub(crate) struct Installation {
    slots: Arc<Slots>,
}

const fn storage_requirements() -> ResourceRequirements {
    ResourceRequirements::none()
        .with_class(
            ResourceClass::BackendStateBytes,
            STORAGE_BYTES,
            STORAGE_BYTES,
        )
        .with_metadata(1, 1)
}

const fn operation_requirements() -> ResourceRequirements {
    ResourceRequirements::none()
        .with_class(
            ResourceClass::BackendStateBytes,
            OPERATION_BYTES * MAX_OPERATIONS,
            OPERATION_BYTES * MAX_OPERATIONS,
        )
        .with_metadata(1, MAX_OPERATIONS as usize)
}

pub(crate) fn requirements() -> ResourceRequirements {
    operation_requirements().combine_concurrent(storage_requirements())
}

pub(crate) fn install(resources: &ProductionResourceLedger) -> Result<Installation, HostError> {
    Ok(Installation {
        slots: Slots::new(resources)?,
    })
}

impl HostSession {
    pub async fn wait_for_timeout(&self, timeout_ms: u32) -> Result<(), HostError> {
        let _permit = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .wait_for_timeout
                .slots
                .admit()?
        };
        tokio::select! {
            biased;
            () = self.stop.cancelled() => Err(HostError::TargetClosed),
            () = self.disconnected.cancelled() => Err(HostError::TargetClosed),
            () = tokio::time::sleep(Duration::from_millis(u64::from(timeout_ms))) => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_core::ResourceLimits;

    fn resources() -> ProductionResourceLedger {
        let plan = requirements();
        let mut limits = ResourceLimits::deny_all(plan.scopes, plan.leases);
        for class in ResourceClass::ALL {
            let bound = plan.class(class);
            limits = limits.with_class(class, bound.global, bound.per_scope);
        }
        ProductionResourceLedger::new(limits, plan).unwrap()
    }

    #[test]
    fn operation_capacity_is_fixed_and_reclaimed_on_drop() {
        // Arrange
        let resources = resources();
        let slots = Slots::new(&resources).unwrap();
        let mut permits: Vec<_> = (0..MAX_OPERATIONS)
            .map(|_| slots.admit().unwrap())
            .collect();

        // Act
        let excess = slots.admit();
        drop(permits.pop());
        let reclaimed = slots.admit();

        // Assert
        assert!(matches!(excess, Err(HostError::ResourceExhausted(_))));
        assert!(reclaimed.is_ok());
    }
}
