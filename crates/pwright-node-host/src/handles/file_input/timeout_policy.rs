//! Bounded client timeout metadata, selected only with retained file input.
//!
//! These cells are not native-use authority: they retain no HostSession,
//! topology, transport, task token or browser object. Binding the correct
//! creation Frame to its Page settings remains the caller's responsibility.
//! The language adapter must read each local default only after that level's
//! observable options getter and debug check, rather than snapshotting the
//! hierarchy before invoking user code.

use std::sync::{Arc, Mutex};

use crate::HostError;
use pwright_core::{
    ProductionResourceLedger, ResourceClass, ResourceLease, ResourceLedger, ResourceLimits,
    ResourceRequirements, ResourceScope,
};

const MAX_CELLS: usize = 1024;
/// Per-cell allowance for the Arc allocation/header, immutable parent/budget
/// references, mutex/default, lease and its single request allocation. Ledger
/// lease bookkeeping is additionally declared through `with_metadata`.
const CELL_BYTES: u64 = 512;
/// Fixed factory/ledger administration: one Arc-backed budget, one ledger,
/// its single scope's accounting tree and fixed resource-class counters. The
/// ledger does not allocate a 1024-entry cell table; each cell owns its lease.
/// This stays admitted even when only settings retained by a caller survive.
const ADMIN_BYTES: u64 = 16 * 1024;
const TOTAL_BYTES: u64 = 1024 * CELL_BYTES + ADMIN_BYTES;
const SCOPE: ResourceScope = ResourceScope::extension(173, 1);

mod session;
pub use session::{ScopedTimeoutSettings, TimeoutPolicySession};

/// A selected environment's shared timeout-metadata budget.
///
/// Clones share the same 1024-slot limit for cells and issued scoped bindings.
/// Cells/bindings may outlive the factory; their allowances are returned only
/// after their last references drop. Session indices have a separate 16-slot
/// ledger so unused index bytes cannot widen this metadata limit.
#[derive(Clone)]
pub struct TimeoutPolicyBudget {
    inner: Arc<BudgetInner>,
}

struct BudgetInner {
    resources: ResourceLedger,
    indexes: ResourceLedger,
    sessions: Mutex<[Option<session::SessionSlot>; session::MAX_SESSIONS]>,
    _storage: ResourceLease,
}

enum Kind {
    Context,
    Page { context: TimeoutSettings },
}

struct Settings {
    kind: Kind,
    debug: bool,
    local_default: Mutex<Option<f64>>,
    _storage: ResourceLease,
    // Keep shared administration alive until after this cell's lease drops.
    budget: TimeoutPolicyBudget,
}

// Admission also covers the heap storage owned directly by this cell. This
// assertion must change alongside the declared allowance if its shape grows.
const _: () = assert!(
    std::mem::size_of::<Settings>()
        + 2 * std::mem::size_of::<usize>()
        + std::mem::size_of::<pwright_core::ResourceRequest>()
        <= CELL_BYTES as usize
);
const _: () = assert!(
    std::mem::size_of::<BudgetInner>()
        + 2 * std::mem::size_of::<usize>()
        + std::mem::size_of::<pwright_core::ResourceRequest>()
        < ADMIN_BYTES as usize
);

/// Shared, mutable client settings with an immutable Context -> Page shape.
///
/// Cloning preserves identity; it does not snapshot the local default. Only a
/// Context root can create Page children, so parent walks have at most two
/// levels and cannot form cycles. Metadata references own no browser lifetime.
#[derive(Clone)]
pub struct TimeoutSettings {
    inner: Arc<Settings>,
}

impl TimeoutPolicyBudget {
    pub fn new() -> Result<Self, HostError> {
        let requirements = ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, TOTAL_BYTES, TOTAL_BYTES)
            .with_metadata(1, MAX_CELLS + 1);
        let limits = ResourceLimits::deny_all(1, MAX_CELLS + 1).with_class(
            ResourceClass::BackendStateBytes,
            TOTAL_BYTES,
            TOTAL_BYTES,
        );
        let resources = ProductionResourceLedger::new(limits, requirements)
            .and_then(|ledger| ledger.claim(requirements))
            .map_err(HostError::backend)?;
        let storage = resources
            .reserve(SCOPE, ResourceClass::BackendStateBytes, ADMIN_BYTES)
            .map_err(HostError::backend)?;
        let indexes = session::index_resources()?;
        Ok(Self {
            inner: Arc::new(BudgetInner {
                resources,
                indexes,
                sessions: Mutex::new(std::array::from_fn(|_| None)),
                _storage: storage,
            }),
        })
    }

    /// Inject the environment's already-classified debug mode. This does not
    /// inspect process environment or introduce a default timeout value.
    pub fn root(&self, debug: bool) -> Result<TimeoutSettings, HostError> {
        self.create(Kind::Context, debug)
    }

    fn create(&self, kind: Kind, debug: bool) -> Result<TimeoutSettings, HostError> {
        // Reserve before allocating the cell. A failed admission cannot retain
        // a new settings allocation or consume a native/FFI operation slot.
        let storage = self
            .inner
            .resources
            .reserve(SCOPE, ResourceClass::BackendStateBytes, CELL_BYTES)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        Ok(TimeoutSettings {
            inner: Arc::new(Settings {
                kind,
                debug,
                local_default: Mutex::new(None),
                budget: self.clone(),
                _storage: storage,
            }),
        })
    }
}

impl TimeoutSettings {
    /// Create a Page's settings from this Context, preserving shared live
    /// parent state and the same injected debug classification.
    pub fn child(&self) -> Result<Self, HostError> {
        match &self.inner.kind {
            Kind::Context => self.inner.budget.create(
                Kind::Page {
                    context: self.clone(),
                },
                self.inner.debug,
            ),
            Kind::Page { .. } => Err(HostError::InvalidArgument(
                "only Context timeout settings can create Page settings".into(),
            )),
        }
    }

    /// Read the current local value, without resolving inheritance or debug
    /// policy. The adapter controls those observable language-operation phases.
    pub fn local_default(&self) -> Result<Option<f64>, HostError> {
        self.inner
            .local_default
            .lock()
            .map(|value| *value)
            .map_err(|_| HostError::backend("timeout settings lock poisoned"))
    }

    /// Store numeric client metadata unchanged; None unsets the local default.
    /// Fractions, signed zero, NaN and infinities are not normalized here.
    /// This is not channel timeout validation or a complete JavaScript setter
    /// boundary for arbitrary non-number values.
    pub fn set_default_timeout(&self, value: Option<f64>) -> Result<(), HostError> {
        *self
            .inner
            .local_default
            .lock()
            .map_err(|_| HostError::backend("timeout settings lock poisoned"))? = value;
        Ok(())
    }

    pub fn is_debug(&self) -> bool {
        self.inner.debug
    }

    pub fn parent(&self) -> Option<Self> {
        match &self.inner.kind {
            Kind::Context => None,
            Kind::Page { context } => Some(context.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_cell_capacity_rejects_an_extra_root_without_native_work() {
        // Arrange
        let budget = TimeoutPolicyBudget::new().unwrap();
        let _cells: Vec<_> = (0..MAX_CELLS)
            .map(|_| budget.root(false).unwrap())
            .collect();

        // Act
        let excess = budget.root(false);

        // Assert
        assert!(matches!(excess, Err(HostError::ResourceExhausted(_))));
        let metrics = budget.inner.resources.metrics().unwrap();
        assert_eq!(metrics.active_leases, MAX_CELLS + 1);
        assert_eq!(metrics.live_scopes, 1);
        assert_eq!(
            metrics.class(ResourceClass::BackendStateBytes).current,
            TOTAL_BYTES
        );
        assert_eq!(metrics.class(ResourceClass::RemoteHandle).current, 0);
        assert_eq!(metrics.class(ResourceClass::PendingCall).current, 0);
    }

    #[test]
    fn final_cell_drop_returns_exactly_one_reusable_allowance() {
        // Arrange
        let budget = TimeoutPolicyBudget::new().unwrap();
        let mut cells: Vec<_> = (0..MAX_CELLS)
            .map(|_| budget.root(false).unwrap())
            .collect();

        // Act
        drop(cells.pop());
        let replacement = budget.root(true);
        let excess = budget.root(false);

        // Assert
        assert!(replacement.is_ok());
        assert!(matches!(excess, Err(HostError::ResourceExhausted(_))));
        assert_eq!(
            budget.inner.resources.metrics().unwrap().active_leases,
            MAX_CELLS + 1
        );
        drop(replacement);
        drop(cells);
        let released = budget.inner.resources.metrics().unwrap();
        assert_eq!(released.active_leases, 1);
        assert_eq!(released.live_scopes, 1);
        assert_eq!(
            released.class(ResourceClass::BackendStateBytes).current,
            ADMIN_BYTES
        );
    }

    #[test]
    fn cloned_factories_share_the_same_cell_limit() {
        // Arrange
        let budget = TimeoutPolicyBudget::new().unwrap();
        let peer = budget.clone();
        let _cells: Vec<_> = (0..MAX_CELLS)
            .map(|_| budget.root(false).unwrap())
            .collect();

        // Act
        let result = peer.root(false);

        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(
            peer.inner.resources.metrics().unwrap().active_leases,
            MAX_CELLS + 1
        );
    }

    #[test]
    fn page_creation_shares_the_root_cell_budget() {
        // Arrange
        let budget = TimeoutPolicyBudget::new().unwrap();
        let context = budget.root(false).unwrap();
        let _pages: Vec<_> = (1..MAX_CELLS).map(|_| context.child().unwrap()).collect();

        // Act
        let excess = context.child();

        // Assert
        assert!(matches!(excess, Err(HostError::ResourceExhausted(_))));
        assert_eq!(
            budget.inner.resources.metrics().unwrap().active_leases,
            MAX_CELLS + 1
        );
    }

    #[test]
    fn page_cannot_add_a_third_hierarchy_level() {
        // Arrange
        let budget = TimeoutPolicyBudget::new().unwrap();
        let context = budget.root(false).unwrap();
        let page = context.child().unwrap();

        // Act
        let result = page.child();

        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
        assert!(context.parent().is_none());
        assert!(page.parent().unwrap().parent().is_none());
        assert_eq!(budget.inner.resources.metrics().unwrap().active_leases, 3);
    }

    #[test]
    fn parent_reads_observe_updates_after_page_creation() {
        // Arrange
        let budget = TimeoutPolicyBudget::new().unwrap();
        let context = budget.root(false).unwrap();
        context.set_default_timeout(Some(12.5)).unwrap();
        let page = context.child().unwrap();
        let parent = page.parent().unwrap();

        // Act
        context.set_default_timeout(Some(27.25)).unwrap();

        // Assert
        assert_eq!(parent.local_default().unwrap(), Some(27.25));
        assert_eq!(page.local_default().unwrap(), None);
        assert_eq!(budget.inner.resources.metrics().unwrap().active_leases, 3);
    }

    #[test]
    fn sibling_pages_have_independent_local_defaults_and_one_shared_parent() {
        // Arrange
        let budget = TimeoutPolicyBudget::new().unwrap();
        let context = budget.root(false).unwrap();
        context.set_default_timeout(Some(30.5)).unwrap();
        let page = context.child().unwrap();
        let sibling = context.child().unwrap();

        // Act
        page.set_default_timeout(Some(0.25)).unwrap();

        // Assert
        assert_eq!(page.local_default().unwrap(), Some(0.25));
        assert_eq!(sibling.local_default().unwrap(), None);
        assert_eq!(page.parent().unwrap().local_default().unwrap(), Some(30.5));
        assert_eq!(
            sibling.parent().unwrap().local_default().unwrap(),
            Some(30.5)
        );
        assert_eq!(budget.inner.resources.metrics().unwrap().active_leases, 4);
    }

    #[test]
    fn clearing_local_default_does_not_change_the_parent() {
        // Arrange
        let budget = TimeoutPolicyBudget::new().unwrap();
        let context = budget.root(false).unwrap();
        context.set_default_timeout(Some(7.5)).unwrap();
        let page = context.child().unwrap();
        page.set_default_timeout(Some(4.5)).unwrap();

        // Act
        page.set_default_timeout(None).unwrap();

        // Assert
        assert_eq!(page.local_default().unwrap(), None);
        assert_eq!(context.local_default().unwrap(), Some(7.5));
    }

    #[test]
    fn debug_mode_is_injected_and_inherited_without_changing_stored_defaults() {
        // Arrange
        let budget = TimeoutPolicyBudget::new().unwrap();
        let context = budget.root(true).unwrap();
        context.set_default_timeout(Some(25.5)).unwrap();

        // Act
        let page = context.child().unwrap();

        // Assert
        assert!(context.is_debug());
        assert!(page.is_debug());
        assert_eq!(context.local_default().unwrap(), Some(25.5));
        assert_eq!(page.local_default().unwrap(), None);
    }

    #[test]
    fn numeric_metadata_preserves_every_supplied_float_bit_pattern() {
        // Arrange
        let budget = TimeoutPolicyBudget::new().unwrap();
        let values = [
            0.125,
            -0.0,
            -10.25,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::from_bits(0x7ff8_0000_0000_0042),
        ];
        let settings: Vec<_> = values.iter().map(|_| budget.root(false).unwrap()).collect();

        // Act
        for (settings, value) in settings.iter().zip(values) {
            settings.set_default_timeout(Some(value)).unwrap();
        }

        // Assert
        for (settings, value) in settings.iter().zip(values) {
            assert_eq!(
                settings.local_default().unwrap().unwrap().to_bits(),
                value.to_bits()
            );
        }
    }

    #[test]
    fn cells_outlive_the_factory_without_snapshotting_defaults() {
        // Arrange
        let budget = TimeoutPolicyBudget::new().unwrap();
        let resources = budget.inner.resources.clone();
        let context = budget.root(false).unwrap();
        let page = context.child().unwrap();

        // Act
        drop(budget);
        context.set_default_timeout(Some(8.75)).unwrap();

        // Assert
        assert_eq!(page.parent().unwrap().local_default().unwrap(), Some(8.75));
        let retained = resources.metrics().unwrap();
        assert_eq!(retained.active_leases, 3);
        assert_eq!(
            retained.class(ResourceClass::BackendStateBytes).current,
            ADMIN_BYTES + 2 * CELL_BYTES
        );
        drop(page);
        drop(context);
        let released = resources.metrics().unwrap();
        assert_eq!(released.active_leases, 0);
        assert_eq!(released.live_scopes, 0);
        assert_eq!(released.class(ResourceClass::BackendStateBytes).current, 0);
    }

    #[test]
    fn page_keeps_only_parent_metadata_alive_until_the_last_cell_reference_drops() {
        // Arrange
        let budget = TimeoutPolicyBudget::new().unwrap();
        let context = budget.root(false).unwrap();
        context.set_default_timeout(Some(3.5)).unwrap();
        let page = context.child().unwrap();

        // Act
        drop(context);

        // Assert
        assert_eq!(page.parent().unwrap().local_default().unwrap(), Some(3.5));
        assert_eq!(budget.inner.resources.metrics().unwrap().active_leases, 3);
        drop(page);
        assert_eq!(budget.inner.resources.metrics().unwrap().active_leases, 1);
    }

    #[test]
    fn cloning_a_cell_keeps_one_admission_until_the_last_reference_drops() {
        // Arrange
        let budget = TimeoutPolicyBudget::new().unwrap();
        let settings = budget.root(false).unwrap();
        let retained = settings.clone();

        // Act
        drop(settings);

        // Assert
        assert_eq!(budget.inner.resources.metrics().unwrap().active_leases, 2);
        assert_eq!(retained.local_default().unwrap(), None);
        assert!(!retained.is_debug());
        drop(retained);
        assert_eq!(budget.inner.resources.metrics().unwrap().active_leases, 1);
    }

    #[test]
    fn a_captured_page_retains_live_parent_and_administration_after_factory_drop() {
        // Arrange
        let budget = TimeoutPolicyBudget::new().unwrap();
        let resources = budget.inner.resources.clone();
        let context = budget.root(false).unwrap();
        let page = context.child().unwrap();
        let read_later = move || page.parent().unwrap().local_default();
        context.set_default_timeout(Some(9.25)).unwrap();
        drop(budget);
        drop(context);

        // Act
        let value = read_later().unwrap();

        // Assert
        assert_eq!(value, Some(9.25));
        let held = resources.metrics().unwrap();
        assert_eq!(held.active_leases, 3);
        assert_eq!(
            held.class(ResourceClass::BackendStateBytes).current,
            ADMIN_BYTES + 2 * CELL_BYTES
        );
        assert_eq!(held.class(ResourceClass::PendingCall).current, 0);
        drop(read_later);
        let released = resources.metrics().unwrap();
        assert_eq!(released.active_leases, 0);
        assert_eq!(released.class(ResourceClass::BackendStateBytes).current, 0);
    }
}
