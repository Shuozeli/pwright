use crate::errors::resource_error;
use pwright_cap_mouse::MouseErrorDetails;
use pwright_core::{
    OperationError, ProductionResourceError, ProductionResourceLedger, ResourceClass,
    ResourceLease, ResourceLedger, ResourceRequirements, ResourceScope,
};
use std::{
    fmt,
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
pub const MOUSE_ERROR_MAX_BYTES: usize = 4096;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CdpMouseProfileError {
    ByteBudgetOverflow,
}
impl fmt::Display for CdpMouseProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Mouse declared byte budget overflowed")
    }
}
impl std::error::Error for CdpMouseProfileError {}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpMouseProductionProfile {
    scope: ResourceScope,
    operation_bytes: usize,
    operations: NonZeroUsize,
    requirements: ResourceRequirements,
}
impl CdpMouseProductionProfile {
    pub fn new(
        scope: ResourceScope,
        operation_bytes: usize,
        operations: NonZeroUsize,
    ) -> Result<Self, CdpMouseProfileError> {
        let bytes = u64::try_from(operation_bytes)
            .ok()
            .and_then(|n| n.checked_mul(u64::try_from(operations.get()).ok()?))
            .ok_or(CdpMouseProfileError::ByteBudgetOverflow)?;
        Ok(Self {
            scope,
            operation_bytes,
            operations,
            requirements: ResourceRequirements::none()
                .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
                .with_metadata(1, operations.get()),
        })
    }
    pub const fn scope(self) -> ResourceScope {
        self.scope
    }
    pub const fn operation_bytes(self) -> usize {
        self.operation_bytes
    }
    pub const fn max_concurrent_operations(self) -> NonZeroUsize {
        self.operations
    }
    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}
#[derive(Clone)]
pub(crate) struct Budget {
    pub profile: CdpMouseProductionProfile,
    ledger: ResourceLedger,
    active: Arc<AtomicUsize>,
}
impl Budget {
    pub fn new(
        profile: CdpMouseProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        Ok(Self {
            profile,
            ledger: resources.claim(profile.requirements())?,
            active: Arc::new(AtomicUsize::new(0)),
        })
    }
    pub fn active(&self) -> usize {
        self.active.load(Ordering::Acquire)
    }
    pub fn reserve(&self) -> Result<Lease, OperationError<MouseErrorDetails>> {
        if self
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < self.profile.operations.get()).then(|| n + 1)
            })
            .is_err()
        {
            return Err(resource_error(
                "Mouse operation concurrency limit was reached",
            ));
        }
        match self.ledger.reserve(
            self.profile.scope,
            ResourceClass::BackendStateBytes,
            u64::try_from(self.profile.operation_bytes).expect("validated byte budget"),
        ) {
            Ok(resource) => Ok(Lease {
                resource: Some(resource),
                active: self.active.clone(),
            }),
            Err(_) => {
                self.active.fetch_sub(1, Ordering::AcqRel);
                Err(resource_error(
                    "Mouse backend byte lease could not be reserved",
                ))
            }
        }
    }
}
pub(crate) struct Lease {
    resource: Option<ResourceLease>,
    active: Arc<AtomicUsize>,
}
impl Drop for Lease {
    fn drop(&mut self) {
        drop(self.resource.take());
        let previous = self.active.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0);
    }
}
