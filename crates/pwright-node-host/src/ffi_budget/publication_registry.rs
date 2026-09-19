//! Fixed environment administrative storage, separate from pending FFI work.
use super::*;

/// Admission for a fixed foreign-thread cleanup registry. This owns no task
/// token and therefore cannot keep FfiBudget.shutdown waiting for itself.
pub struct PublicationRegistryStorage {
    _lease: ResourceLease,
}

impl PublicationRegistryStorage {
    pub fn reserve(bytes: usize) -> Result<Self, HostError> {
        let bytes = u64::try_from(bytes).map_err(HostError::backend)?;
        if bytes == 0 || bytes > 16 * 1024 {
            return Err(HostError::InvalidArgument(
                "publication registry storage must fit 16 KiB".into(),
            ));
        }
        let requirements = ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, 1);
        let limits = ResourceLimits::deny_all(1, 1).with_class(
            ResourceClass::BackendStateBytes,
            bytes,
            bytes,
        );
        let ledger = ProductionResourceLedger::new(limits, requirements)
            .and_then(|ledger| ledger.claim(requirements))
            .map_err(HostError::backend)?;
        let lease = ledger
            .reserve(
                ResourceScope::extension(122, 2),
                ResourceClass::BackendStateBytes,
                bytes,
            )
            .map_err(HostError::backend)?;
        Ok(Self { _lease: lease })
    }
}

#[cfg(feature = "retained-file-input")]
const PHASE_CAPACITY: usize = 32;
#[cfg(feature = "retained-file-input")]
const PHASE_BYTES: u64 = 16 * 1024;
#[cfg(feature = "retained-file-input")]
const PHASE_SCOPE: ResourceScope = ResourceScope::extension(172, 1);

/// Shared admission for synchronous foreign-runtime publication phases.
///
/// One selected environment owns one budget; clones share the same ledger.
/// At most 32 live phases may each retain 16 KiB of temporary control/root
/// storage, independently of revocable FFI preparation entries. This reserves
/// neither a command slot nor a running-task token. The owner must drop phase
/// storage before yielding to a Promise or other asynchronous continuation.
#[cfg(feature = "retained-file-input")]
#[derive(Clone)]
pub struct PublicationPhaseBudget {
    resources: ResourceLedger,
}

/// An exclusive synchronous-phase allowance, returned only when this drops.
///
/// Revoking a preparation or dropping its budget does not reclaim this live
/// lease. It covers bounded temporary storage, not arbitrary reachable values
/// or allocations performed by foreign user code.
#[cfg(feature = "retained-file-input")]
#[must_use = "hold phase storage until all synchronous temporary roots are released"]
pub struct PublicationPhaseStorage {
    _lease: ResourceLease,
}

#[cfg(feature = "retained-file-input")]
impl PublicationPhaseStorage {
    /// Fixed temporary-storage admission, independent of revocable FFI work.
    /// Embeddings can check concrete synchronous overlap against this bound.
    pub const CAPACITY_BYTES: usize = PHASE_BYTES as usize;
}

#[cfg(feature = "retained-file-input")]
impl PublicationPhaseBudget {
    pub fn new() -> Result<Self, HostError> {
        let bytes = u64::try_from(PHASE_CAPACITY)
            .ok()
            .and_then(|capacity| capacity.checked_mul(PHASE_BYTES))
            .ok_or_else(|| {
                HostError::ResourceExhausted("publication phase capacity overflow".into())
            })?;
        let requirements = ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, PHASE_CAPACITY);
        let limits = ResourceLimits::deny_all(1, PHASE_CAPACITY).with_class(
            ResourceClass::BackendStateBytes,
            bytes,
            bytes,
        );
        let resources = ProductionResourceLedger::new(limits, requirements)
            .and_then(|ledger| ledger.claim(requirements))
            .map_err(HostError::backend)?;
        Ok(Self { resources })
    }

    /// Reserve before copying roots or invoking any reentrant language code.
    /// Every reservation uses the same ledger; failure retains no partial lease.
    pub fn reserve(&self) -> Result<PublicationPhaseStorage, HostError> {
        let lease = self
            .resources
            .reserve(PHASE_SCOPE, ResourceClass::BackendStateBytes, PHASE_BYTES)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        Ok(PublicationPhaseStorage { _lease: lease })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_rejects_zero_storage() {
        // Arrange
        let bytes = 0;
        // Act
        let result = PublicationRegistryStorage::reserve(bytes);
        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    }

    #[test]
    fn registry_rejects_storage_above_fixed_limit() {
        // Arrange
        let bytes = 16 * 1024 + 1;
        // Act
        let result = PublicationRegistryStorage::reserve(bytes);
        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    }

    #[test]
    fn registry_admits_the_exact_boundary_without_ffi_work() {
        // Arrange
        let bytes = 16 * 1024;
        // Act
        let result = PublicationRegistryStorage::reserve(bytes);
        // Assert
        assert!(result.is_ok());
    }

    #[cfg(feature = "retained-file-input")]
    #[test]
    fn phase_budget_rejects_the_thirty_third_live_phase() {
        // Arrange
        let budget = PublicationPhaseBudget::new().unwrap();
        let _storage: [PublicationPhaseStorage; PHASE_CAPACITY] =
            std::array::from_fn(|_| budget.reserve().unwrap());

        // Act
        let result = budget.reserve();

        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        let metrics = budget.resources.metrics().unwrap();
        assert_eq!(metrics.active_leases, 32);
        assert_eq!(metrics.live_scopes, 1);
        assert_eq!(
            metrics.class(ResourceClass::BackendStateBytes).current,
            512 * 1024
        );
        assert_eq!(metrics.class(ResourceClass::PendingCall).current, 0);
    }

    #[cfg(feature = "retained-file-input")]
    #[test]
    fn dropping_one_phase_returns_exactly_one_reusable_allowance() {
        // Arrange
        let budget = PublicationPhaseBudget::new().unwrap();
        let mut storage: Vec<_> = (0..PHASE_CAPACITY)
            .map(|_| budget.reserve().unwrap())
            .collect();

        // Act
        drop(storage.pop());
        let replacement = budget.reserve();
        let excess = budget.reserve();

        // Assert
        assert!(replacement.is_ok());
        assert!(matches!(excess, Err(HostError::ResourceExhausted(_))));
        assert_eq!(budget.resources.metrics().unwrap().active_leases, 32);
        drop(replacement);
        drop(storage);
        let metrics = budget.resources.metrics().unwrap();
        assert_eq!(metrics.active_leases, 0);
        assert_eq!(metrics.live_scopes, 0);
        assert_eq!(metrics.class(ResourceClass::BackendStateBytes).current, 0);
    }

    #[cfg(feature = "retained-file-input")]
    #[test]
    fn dropping_budget_preserves_live_phase_storage_until_its_owner_drops() {
        // Arrange
        let budget = PublicationPhaseBudget::new().unwrap();
        let resources = budget.resources.clone();
        let storage = budget.reserve().unwrap();

        // Act
        drop(budget);

        // Assert
        let held = resources.metrics().unwrap();
        assert_eq!(PublicationPhaseStorage::CAPACITY_BYTES, 16 * 1024);
        assert_eq!(held.active_leases, 1);
        assert_eq!(
            held.class(ResourceClass::BackendStateBytes).current,
            PublicationPhaseStorage::CAPACITY_BYTES as u64
        );
        drop(storage);
        let released = resources.metrics().unwrap();
        assert_eq!(released.active_leases, 0);
        assert_eq!(released.live_scopes, 0);
        assert_eq!(released.class(ResourceClass::BackendStateBytes).current, 0);
    }

    #[cfg(feature = "retained-file-input")]
    #[test]
    fn phase_budget_clones_share_one_capacity_without_ffi_admission() {
        // Arrange
        let budget = PublicationPhaseBudget::new().unwrap();
        let peer = budget.clone();
        let _storage: [PublicationPhaseStorage; PHASE_CAPACITY] =
            std::array::from_fn(|_| budget.reserve().unwrap());

        // Act
        let result = peer.reserve();

        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(peer.resources.metrics().unwrap().active_leases, 32);
        assert_eq!(
            peer.resources
                .metrics()
                .unwrap()
                .class(ResourceClass::PendingCall)
                .current,
            0
        );
    }
}
