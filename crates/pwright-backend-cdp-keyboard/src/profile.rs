use crate::errors::resource_error;
use pwright_cap_keyboard::{KEYBOARD_TEXT_MAX_BYTES, KeyboardErrorDetails};
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

pub const KEYBOARD_ERROR_MAX_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CdpKeyboardProfileError {
    ArgumentLimitTooLarge,
    ByteBudgetOverflow,
}
impl fmt::Display for CdpKeyboardProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ArgumentLimitTooLarge => {
                "Keyboard argument profile exceeds the 1 MiB portable bound"
            }
            Self::ByteBudgetOverflow => "Keyboard declared byte budget overflowed",
        })
    }
}
impl std::error::Error for CdpKeyboardProfileError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpKeyboardProductionProfile {
    scope: ResourceScope,
    argument_bytes: usize,
    operations: NonZeroUsize,
    requirements: ResourceRequirements,
}
impl CdpKeyboardProductionProfile {
    pub fn new(
        scope: ResourceScope,
        argument_bytes: usize,
        operations: NonZeroUsize,
    ) -> Result<Self, CdpKeyboardProfileError> {
        if argument_bytes > KEYBOARD_TEXT_MAX_BYTES {
            return Err(CdpKeyboardProfileError::ArgumentLimitTooLarge);
        }
        let per_operation = u64::try_from(argument_bytes + 2 * KEYBOARD_ERROR_MAX_BYTES)
            .map_err(|_| CdpKeyboardProfileError::ByteBudgetOverflow)?;
        let count = u64::try_from(operations.get())
            .map_err(|_| CdpKeyboardProfileError::ByteBudgetOverflow)?;
        let bytes = per_operation
            .checked_mul(count)
            .ok_or(CdpKeyboardProfileError::ByteBudgetOverflow)?;
        Ok(Self {
            scope,
            argument_bytes,
            operations,
            requirements: ResourceRequirements::none()
                .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
                .with_metadata(1, operations.get()),
        })
    }
    pub const fn scope(self) -> ResourceScope {
        self.scope
    }
    pub const fn argument_bytes(self) -> usize {
        self.argument_bytes
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
    pub profile: CdpKeyboardProductionProfile,
    ledger: ResourceLedger,
    active: Arc<AtomicUsize>,
}
impl Budget {
    pub fn new(
        profile: CdpKeyboardProductionProfile,
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
    pub fn reserve<P>(
        &self,
        params: P,
        capacity: usize,
    ) -> Result<Admitted<P>, OperationError<KeyboardErrorDetails>> {
        if capacity > self.profile.argument_bytes {
            return Err(resource_error(
                "Keyboard argument capacity exceeds the selected profile",
            ));
        }
        if self
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < self.profile.operations.get()).then(|| n + 1)
            })
            .is_err()
        {
            return Err(resource_error(
                "Keyboard operation concurrency limit was reached",
            ));
        }
        let resource = match self.ledger.reserve(
            self.profile.scope,
            ResourceClass::BackendStateBytes,
            u64::try_from(capacity + 2 * KEYBOARD_ERROR_MAX_BYTES)
                .expect("validated portable argument budget fits u64"),
        ) {
            Ok(resource) => resource,
            Err(_) => {
                self.active.fetch_sub(1, Ordering::AcqRel);
                return Err(resource_error(
                    "Keyboard backend byte lease could not be reserved",
                ));
            }
        };
        Ok(Admitted {
            params,
            _lease: Lease {
                resource: Some(resource),
                active: self.active.clone(),
            },
        })
    }
}

// Field order is intentional: drop the owned argument before its admission lease.
pub(crate) struct Admitted<P> {
    pub params: P,
    _lease: Lease,
}
struct Lease {
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
