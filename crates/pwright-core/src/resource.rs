use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::{CallId, ObjectId};

const RESOURCE_CLASS_COUNT: usize = 15;

/// Resources retained by a browser connection.
///
/// Count classes use one unit per retained item. Byte classes use one unit per
/// retained byte. The ledger intentionally does not guess heap size: adapters
/// reserve before retaining a value and keep the lease beside that value.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum ResourceClass {
    LiveObject,
    PendingCall,
    ActiveCommand,
    EventItem,
    EventBytes,
    WireBytes,
    ResponseBodyBytes,
    LogBytes,
    TraceBytes,
    ArtifactBytes,
    ScreenshotBytes,
    DomContentBytes,
    SelectorBytes,
    RemoteHandle,
    BackendStateBytes,
}

impl ResourceClass {
    pub const ALL: [Self; RESOURCE_CLASS_COUNT] = [
        Self::LiveObject,
        Self::PendingCall,
        Self::ActiveCommand,
        Self::EventItem,
        Self::EventBytes,
        Self::WireBytes,
        Self::ResponseBodyBytes,
        Self::LogBytes,
        Self::TraceBytes,
        Self::ArtifactBytes,
        Self::ScreenshotBytes,
        Self::DomContentBytes,
        Self::SelectorBytes,
        Self::RemoteHandle,
        Self::BackendStateBytes,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LiveObject => "live objects",
            Self::PendingCall => "pending calls",
            Self::ActiveCommand => "active commands",
            Self::EventItem => "event items",
            Self::EventBytes => "event bytes",
            Self::WireBytes => "wire bytes",
            Self::ResponseBodyBytes => "response body bytes",
            Self::LogBytes => "log bytes",
            Self::TraceBytes => "trace bytes",
            Self::ArtifactBytes => "artifact bytes",
            Self::ScreenshotBytes => "screenshot bytes",
            Self::DomContentBytes => "DOM content bytes",
            Self::SelectorBytes => "selector bytes",
            Self::RemoteHandle => "remote handles",
            Self::BackendStateBytes => "backend state bytes",
        }
    }

    pub const fn is_byte_class(self) -> bool {
        matches!(
            self,
            Self::EventBytes
                | Self::WireBytes
                | Self::ResponseBodyBytes
                | Self::LogBytes
                | Self::TraceBytes
                | Self::ArtifactBytes
                | Self::ScreenshotBytes
                | Self::DomContentBytes
                | Self::SelectorBytes
                | Self::BackendStateBytes
        )
    }

    const fn index(self) -> usize {
        self as usize
    }
}

impl Display for ResourceClass {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Attribution boundary inside one connection-local ledger.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ResourceScope {
    Connection,
    Object(ObjectId),
    Call(CallId),
    Extension { namespace: u32, id: u64 },
}

impl ResourceScope {
    pub const fn extension(namespace: u32, id: u64) -> Self {
        Self::Extension { namespace, id }
    }
}

impl Display for ResourceScope {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connection => formatter.write_str("connection"),
            Self::Object(object_id) => write!(formatter, "object:{object_id}"),
            Self::Call(call_id) => write!(formatter, "call:{call_id}"),
            Self::Extension { namespace, id } => {
                write!(formatter, "extension:{namespace}:{id}")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResourceLimit {
    pub global: u64,
    pub per_scope: u64,
}

impl ResourceLimit {
    pub const fn new(global: u64, per_scope: u64) -> Self {
        Self { global, per_scope }
    }
}

/// Explicit deny-by-default limits for one connection ledger.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceLimits {
    classes: [ResourceLimit; RESOURCE_CLASS_COUNT],
    pub scopes: usize,
    pub leases: usize,
}

impl ResourceLimits {
    pub const fn deny_all(scopes: usize, leases: usize) -> Self {
        Self {
            classes: [ResourceLimit::new(0, 0); RESOURCE_CLASS_COUNT],
            scopes,
            leases,
        }
    }

    pub const fn with_class(mut self, class: ResourceClass, global: u64, per_scope: u64) -> Self {
        self.classes[class.index()] = ResourceLimit::new(global, per_scope);
        self
    }

    pub const fn class(self, class: ResourceClass) -> ResourceLimit {
        self.classes[class.index()]
    }
}

/// Minimum resource capacity declared by one production composition.
///
/// Combining requirements takes the maximum for each dimension. The shared
/// ledger still enforces aggregate contention dynamically; the plan proves
/// that every installed component was declared and can use its configured
/// local bound when it is the sole consumer of that class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceRequirements {
    classes: [ResourceLimit; RESOURCE_CLASS_COUNT],
    pub scopes: usize,
    pub leases: usize,
}

impl ResourceRequirements {
    pub const fn none() -> Self {
        Self {
            classes: [ResourceLimit::new(0, 0); RESOURCE_CLASS_COUNT],
            scopes: 0,
            leases: 0,
        }
    }

    pub const fn with_class(mut self, class: ResourceClass, global: u64, per_scope: u64) -> Self {
        self.classes[class.index()] = ResourceLimit::new(global, per_scope);
        self
    }

    pub const fn with_metadata(mut self, scopes: usize, leases: usize) -> Self {
        self.scopes = scopes;
        self.leases = leases;
        self
    }

    pub const fn class(self, class: ResourceClass) -> ResourceLimit {
        self.classes[class.index()]
    }

    pub fn merge(mut self, other: Self) -> Self {
        for class in ResourceClass::ALL {
            let index = class.index();
            self.classes[index].global =
                self.classes[index].global.max(other.classes[index].global);
            self.classes[index].per_scope = self.classes[index]
                .per_scope
                .max(other.classes[index].per_scope);
        }
        self.scopes = self.scopes.max(other.scopes);
        self.leases = self.leases.max(other.leases);
        self
    }

    /// Combines components whose declared peaks may be live concurrently.
    ///
    /// Unlike [`Self::merge`], this is the composition-root operation: every
    /// dimension is added conservatively so an exact configured plan can hold
    /// all installed components at once. Saturation turns arithmetic overflow
    /// into an impossible-to-satisfy maximum rather than wrapping downward.
    pub fn combine_concurrent(mut self, other: Self) -> Self {
        for class in ResourceClass::ALL {
            let index = class.index();
            self.classes[index].global = self.classes[index]
                .global
                .saturating_add(other.classes[index].global);
            self.classes[index].per_scope = self.classes[index]
                .per_scope
                .saturating_add(other.classes[index].per_scope);
        }
        self.scopes = self.scopes.saturating_add(other.scopes);
        self.leases = self.leases.saturating_add(other.leases);
        self
    }
    fn validate_shape(self) -> Result<(), ProductionResourceError> {
        let mut has_class = false;
        for class in ResourceClass::ALL {
            let requirement = self.class(class);
            if (requirement.global == 0) != (requirement.per_scope == 0) {
                return Err(ProductionResourceError::InvalidClassRequirement {
                    class,
                    global: requirement.global,
                    per_scope: requirement.per_scope,
                });
            }
            has_class |= requirement.global != 0;
        }
        if has_class && self.scopes == 0 {
            return Err(ProductionResourceError::MissingScopeRequirement);
        }
        if has_class && self.leases == 0 {
            return Err(ProductionResourceError::MissingLeaseRequirement);
        }
        Ok(())
    }
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self::none()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceCapacityBoundary {
    ConfiguredLimits,
    DeclaredPlan,
}

impl Display for ResourceCapacityBoundary {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfiguredLimits => formatter.write_str("configured resource limits"),
            Self::DeclaredPlan => formatter.write_str("declared production resource plan"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProductionResourceError {
    InvalidClassRequirement {
        class: ResourceClass,
        global: u64,
        per_scope: u64,
    },
    MissingScopeRequirement,
    MissingLeaseRequirement,
    InsufficientGlobal {
        boundary: ResourceCapacityBoundary,
        class: ResourceClass,
        required: u64,
        available: u64,
    },
    InsufficientPerScope {
        boundary: ResourceCapacityBoundary,
        class: ResourceClass,
        required: u64,
        available: u64,
    },
    InsufficientScopes {
        boundary: ResourceCapacityBoundary,
        required: usize,
        available: usize,
    },
    InsufficientLeases {
        boundary: ResourceCapacityBoundary,
        required: usize,
        available: usize,
    },
    LedgerNotPristine {
        live_scopes: usize,
        active_leases: usize,
        peak_scopes: usize,
        peak_leases: usize,
        rejected_reservations: u64,
    },
    Resource(ResourceError),
}

impl Display for ProductionResourceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidClassRequirement {
                class,
                global,
                per_scope,
            } => write!(
                formatter,
                "production requirement for {class} must set both global and per-scope capacity or neither; got {global}/{per_scope}"
            ),
            Self::MissingScopeRequirement => formatter.write_str(
                "a production resource plan with retained classes must declare scope capacity",
            ),
            Self::MissingLeaseRequirement => formatter.write_str(
                "a production resource plan with retained classes must declare lease capacity",
            ),
            Self::InsufficientGlobal {
                boundary,
                class,
                required,
                available,
            } => write!(
                formatter,
                "{boundary} provide {available} global {class}, below required {required}"
            ),
            Self::InsufficientPerScope {
                boundary,
                class,
                required,
                available,
            } => write!(
                formatter,
                "{boundary} provide {available} per-scope {class}, below required {required}"
            ),
            Self::InsufficientScopes {
                boundary,
                required,
                available,
            } => write!(
                formatter,
                "{boundary} provide {available} scopes, below required {required}"
            ),
            Self::InsufficientLeases {
                boundary,
                required,
                available,
            } => write!(
                formatter,
                "{boundary} provide {available} leases, below required {required}"
            ),
            Self::LedgerNotPristine {
                live_scopes,
                active_leases,
                peak_scopes,
                peak_leases,
                rejected_reservations,
            } => write!(
                formatter,
                "production resource ledger was already used: {live_scopes} live/{peak_scopes} peak scopes, {active_leases} active/{peak_leases} peak leases, and {rejected_reservations} rejected reservations"
            ),
            Self::Resource(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ProductionResourceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resource(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ResourceError> for ProductionResourceError {
    fn from(error: ResourceError) -> Self {
        Self::Resource(error)
    }
}

/// A production-only proof that an explicit resource plan fits one pristine
/// connection ledger.
///
/// Components call [`Self::claim`] with their own requirements. A component
/// omitted from the plan cannot silently obtain the underlying ledger.
#[derive(Clone, Debug)]
pub struct ProductionResourceLedger {
    ledger: ResourceLedger,
    plan: ResourceRequirements,
}

impl ProductionResourceLedger {
    pub fn new(
        limits: ResourceLimits,
        plan: ResourceRequirements,
    ) -> Result<Self, ProductionResourceError> {
        plan.validate_shape()?;
        validate_limits(limits, plan, ResourceCapacityBoundary::ConfiguredLimits)?;
        Ok(Self {
            ledger: ResourceLedger::new(limits),
            plan,
        })
    }

    pub fn from_pristine_ledger(
        ledger: ResourceLedger,
        plan: ResourceRequirements,
    ) -> Result<Self, ProductionResourceError> {
        plan.validate_shape()?;
        validate_limits(
            ledger.limits(),
            plan,
            ResourceCapacityBoundary::ConfiguredLimits,
        )?;
        let metrics = ledger.metrics()?;
        if metrics.live_scopes != 0
            || metrics.active_leases != 0
            || metrics.peak_scopes != 0
            || metrics.peak_leases != 0
            || metrics.rejected_reservations != 0
        {
            return Err(ProductionResourceError::LedgerNotPristine {
                live_scopes: metrics.live_scopes,
                active_leases: metrics.active_leases,
                peak_scopes: metrics.peak_scopes,
                peak_leases: metrics.peak_leases,
                rejected_reservations: metrics.rejected_reservations,
            });
        }
        Ok(Self { ledger, plan })
    }

    pub const fn plan(&self) -> ResourceRequirements {
        self.plan
    }

    pub fn claim(
        &self,
        requirements: ResourceRequirements,
    ) -> Result<ResourceLedger, ProductionResourceError> {
        requirements.validate_shape()?;
        validate_requirements(
            self.plan,
            requirements,
            ResourceCapacityBoundary::DeclaredPlan,
        )?;
        Ok(self.ledger.clone())
    }

    pub fn metrics(&self) -> Result<ResourceMetrics, ResourceError> {
        self.ledger.metrics()
    }
}

fn validate_limits(
    limits: ResourceLimits,
    requirements: ResourceRequirements,
    boundary: ResourceCapacityBoundary,
) -> Result<(), ProductionResourceError> {
    let mut available = ResourceRequirements::none().with_metadata(limits.scopes, limits.leases);
    for class in ResourceClass::ALL {
        let limit = limits.class(class);
        available = available.with_class(class, limit.global, limit.per_scope);
    }
    validate_requirements(available, requirements, boundary)
}

fn validate_requirements(
    available: ResourceRequirements,
    required: ResourceRequirements,
    boundary: ResourceCapacityBoundary,
) -> Result<(), ProductionResourceError> {
    for class in ResourceClass::ALL {
        let available = available.class(class);
        let required = required.class(class);
        if available.global < required.global {
            return Err(ProductionResourceError::InsufficientGlobal {
                boundary,
                class,
                required: required.global,
                available: available.global,
            });
        }
        if available.per_scope < required.per_scope {
            return Err(ProductionResourceError::InsufficientPerScope {
                boundary,
                class,
                required: required.per_scope,
                available: available.per_scope,
            });
        }
    }
    if available.scopes < required.scopes {
        return Err(ProductionResourceError::InsufficientScopes {
            boundary,
            required: required.scopes,
            available: available.scopes,
        });
    }
    if available.leases < required.leases {
        return Err(ProductionResourceError::InsufficientLeases {
            boundary,
            required: required.leases,
            available: available.leases,
        });
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceRequest {
    pub class: ResourceClass,
    pub units: u64,
}

impl ResourceRequest {
    pub const fn new(class: ResourceClass, units: u64) -> Self {
        Self { class, units }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResourceClassMetrics {
    pub global_limit: u64,
    pub per_scope_limit: u64,
    pub current: u64,
    pub peak: u64,
    pub rejected: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceMetrics {
    classes: [ResourceClassMetrics; RESOURCE_CLASS_COUNT],
    pub live_scopes: usize,
    pub peak_scopes: usize,
    pub active_leases: usize,
    pub peak_leases: usize,
    pub rejected_reservations: u64,
}

impl ResourceMetrics {
    pub const fn class(self, class: ResourceClass) -> ResourceClassMetrics {
        self.classes[class.index()]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceScopeMetrics {
    pub scope: ResourceScope,
    classes: [u64; RESOURCE_CLASS_COUNT],
    pub active_leases: usize,
}

impl ResourceScopeMetrics {
    pub const fn units(self, class: ResourceClass) -> u64 {
        self.classes[class.index()]
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceError {
    EmptyRequest,
    ZeroUnits {
        class: ResourceClass,
    },
    DuplicateClass {
        class: ResourceClass,
    },
    TooManyClasses {
        actual: usize,
        limit: usize,
    },
    ArithmeticOverflow {
        class: ResourceClass,
    },
    GlobalLimitExceeded {
        class: ResourceClass,
        current: u64,
        requested: u64,
        limit: u64,
    },
    ScopeLimitExceeded {
        scope: ResourceScope,
        class: ResourceClass,
        current: u64,
        requested: u64,
        limit: u64,
    },
    ScopeCapacityExceeded {
        limit: usize,
    },
    LeaseCapacityExceeded {
        limit: usize,
    },
    MissingLeaseClass {
        class: ResourceClass,
    },
    ReleasedLease,
    ClassIsNotBytes {
        class: ResourceClass,
    },
    ReservationTooSmall {
        class: ResourceClass,
        reserved: u64,
        actual: u64,
    },
    StatePoisoned,
}

impl Display for ResourceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRequest => formatter.write_str("resource reservation must not be empty"),
            Self::ZeroUnits { class } => {
                write!(
                    formatter,
                    "resource reservation for {class} must be non-zero"
                )
            }
            Self::DuplicateClass { class } => {
                write!(formatter, "resource reservation repeats class {class}")
            }
            Self::TooManyClasses { actual, limit } => write!(
                formatter,
                "resource reservation has {actual} classes, exceeding limit {limit}"
            ),
            Self::ArithmeticOverflow { class } => {
                write!(formatter, "resource accounting overflowed for {class}")
            }
            Self::GlobalLimitExceeded {
                class,
                current,
                requested,
                limit,
            } => write!(
                formatter,
                "global {class} usage {current} cannot grow by {requested}; limit is {limit}"
            ),
            Self::ScopeLimitExceeded {
                scope,
                class,
                current,
                requested,
                limit,
            } => write!(
                formatter,
                "{scope} {class} usage {current} cannot grow by {requested}; per-scope limit is {limit}"
            ),
            Self::ScopeCapacityExceeded { limit } => {
                write!(
                    formatter,
                    "resource scope-table limit of {limit} was reached"
                )
            }
            Self::LeaseCapacityExceeded { limit } => {
                write!(formatter, "resource lease limit of {limit} was reached")
            }
            Self::MissingLeaseClass { class } => {
                write!(formatter, "resource lease does not contain {class}")
            }
            Self::ReleasedLease => formatter.write_str("resource lease was already released"),
            Self::ClassIsNotBytes { class } => {
                write!(formatter, "resource class {class} is not measured in bytes")
            }
            Self::ReservationTooSmall {
                class,
                reserved,
                actual,
            } => write!(
                formatter,
                "reserved {reserved} {class} but retained value needs {actual}"
            ),
            Self::StatePoisoned => formatter.write_str("resource ledger state was poisoned"),
        }
    }
}

impl Error for ResourceError {}

#[derive(Clone, Debug)]
struct ScopeUsage {
    classes: [u64; RESOURCE_CLASS_COUNT],
    leases: usize,
}

impl ScopeUsage {
    const fn new() -> Self {
        Self {
            classes: [0; RESOURCE_CLASS_COUNT],
            leases: 0,
        }
    }

    fn is_empty(&self) -> bool {
        self.leases == 0 && self.classes.iter().all(|units| *units == 0)
    }
}

#[derive(Debug)]
struct ResourceState {
    current: [u64; RESOURCE_CLASS_COUNT],
    peak: [u64; RESOURCE_CLASS_COUNT],
    rejected: [u64; RESOURCE_CLASS_COUNT],
    scopes: BTreeMap<ResourceScope, ScopeUsage>,
    peak_scopes: usize,
    active_leases: usize,
    peak_leases: usize,
    rejected_reservations: u64,
}

impl ResourceState {
    const fn new() -> Self {
        Self {
            current: [0; RESOURCE_CLASS_COUNT],
            peak: [0; RESOURCE_CLASS_COUNT],
            rejected: [0; RESOURCE_CLASS_COUNT],
            scopes: BTreeMap::new(),
            peak_scopes: 0,
            active_leases: 0,
            peak_leases: 0,
            rejected_reservations: 0,
        }
    }

    fn reject(&mut self, class: Option<ResourceClass>) {
        self.rejected_reservations = self.rejected_reservations.saturating_add(1);
        if let Some(class) = class {
            let rejected = &mut self.rejected[class.index()];
            *rejected = rejected.saturating_add(1);
        }
    }
}

#[derive(Debug)]
struct ResourceLedgerInner {
    limits: ResourceLimits,
    state: Mutex<ResourceState>,
}

/// Connection-local, dependency-free shared resource accounting.
///
/// There is no global ledger. A composition root explicitly clones this value
/// into the components that must share a budget.
#[derive(Clone, Debug)]
pub struct ResourceLedger {
    inner: Arc<ResourceLedgerInner>,
}

impl ResourceLedger {
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            inner: Arc::new(ResourceLedgerInner {
                limits,
                state: Mutex::new(ResourceState::new()),
            }),
        }
    }

    pub fn limits(&self) -> ResourceLimits {
        self.inner.limits
    }

    /// Returns whether both handles account against the same connection
    /// budget. This compares ledger identity, not merely equal limits.
    pub fn shares_budget_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    pub fn reserve(
        &self,
        scope: ResourceScope,
        class: ResourceClass,
        units: u64,
    ) -> Result<ResourceLease, ResourceError> {
        self.reserve_many(scope, &[ResourceRequest::new(class, units)])
    }

    /// Reserves multiple resource classes as one transaction.
    ///
    /// Validation and both global/per-scope limit checks finish before any
    /// counter changes. A failed reservation therefore retains no partial
    /// budget.
    pub fn reserve_many(
        &self,
        scope: ResourceScope,
        requests: &[ResourceRequest],
    ) -> Result<ResourceLease, ResourceError> {
        let requests = normalize_requests(requests)?;
        let mut state = self.lock_state()?;
        if state.active_leases >= self.inner.limits.leases {
            state.reject(None);
            return Err(ResourceError::LeaseCapacityExceeded {
                limit: self.inner.limits.leases,
            });
        }
        let new_scope = !state.scopes.contains_key(&scope);
        if new_scope && state.scopes.len() >= self.inner.limits.scopes {
            state.reject(None);
            return Err(ResourceError::ScopeCapacityExceeded {
                limit: self.inner.limits.scopes,
            });
        }

        for request in &requests {
            let index = request.class.index();
            let limit = self.inner.limits.class(request.class);
            let global = state.current[index]
                .checked_add(request.units)
                .ok_or_else(|| {
                    state.reject(Some(request.class));
                    ResourceError::ArithmeticOverflow {
                        class: request.class,
                    }
                })?;
            if global > limit.global {
                state.reject(Some(request.class));
                return Err(ResourceError::GlobalLimitExceeded {
                    class: request.class,
                    current: state.current[index],
                    requested: request.units,
                    limit: limit.global,
                });
            }
            let scope_current = state
                .scopes
                .get(&scope)
                .map_or(0, |usage| usage.classes[index]);
            let scoped = scope_current.checked_add(request.units).ok_or_else(|| {
                state.reject(Some(request.class));
                ResourceError::ArithmeticOverflow {
                    class: request.class,
                }
            })?;
            if scoped > limit.per_scope {
                state.reject(Some(request.class));
                return Err(ResourceError::ScopeLimitExceeded {
                    scope,
                    class: request.class,
                    current: scope_current,
                    requested: request.units,
                    limit: limit.per_scope,
                });
            }
        }

        state.scopes.entry(scope).or_insert_with(ScopeUsage::new);
        for request in &requests {
            let index = request.class.index();
            state.current[index] += request.units;
            state.peak[index] = state.peak[index].max(state.current[index]);
            state
                .scopes
                .get_mut(&scope)
                .expect("scope was inserted before accounting")
                .classes[index] += request.units;
        }
        state
            .scopes
            .get_mut(&scope)
            .expect("scope was inserted before lease accounting")
            .leases += 1;
        state.active_leases += 1;
        state.peak_leases = state.peak_leases.max(state.active_leases);
        state.peak_scopes = state.peak_scopes.max(state.scopes.len());
        drop(state);

        Ok(ResourceLease {
            ledger: self.clone(),
            scope,
            requests,
            active: true,
        })
    }

    pub fn metrics(&self) -> Result<ResourceMetrics, ResourceError> {
        let state = self.lock_state()?;
        let mut classes = [ResourceClassMetrics::default(); RESOURCE_CLASS_COUNT];
        for class in ResourceClass::ALL {
            let index = class.index();
            let limit = self.inner.limits.class(class);
            classes[index] = ResourceClassMetrics {
                global_limit: limit.global,
                per_scope_limit: limit.per_scope,
                current: state.current[index],
                peak: state.peak[index],
                rejected: state.rejected[index],
            };
        }
        Ok(ResourceMetrics {
            classes,
            live_scopes: state.scopes.len(),
            peak_scopes: state.peak_scopes,
            active_leases: state.active_leases,
            peak_leases: state.peak_leases,
            rejected_reservations: state.rejected_reservations,
        })
    }

    /// Reads one known scope without allocating or exporting the complete
    /// scope table.
    pub fn scope_metrics(
        &self,
        scope: ResourceScope,
    ) -> Result<Option<ResourceScopeMetrics>, ResourceError> {
        let state = self.lock_state()?;
        Ok(state.scopes.get(&scope).map(|usage| ResourceScopeMetrics {
            scope,
            classes: usage.classes,
            active_leases: usage.leases,
        }))
    }

    fn lock_state(&self) -> Result<MutexGuard<'_, ResourceState>, ResourceError> {
        self.inner
            .state
            .lock()
            .map_err(|_| ResourceError::StatePoisoned)
    }

    fn release(&self, scope: ResourceScope, requests: &[ResourceRequest]) {
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for request in requests {
            let index = request.class.index();
            debug_assert!(state.current[index] >= request.units);
            state.current[index] = state.current[index].saturating_sub(request.units);
            if let Some(scope_usage) = state.scopes.get_mut(&scope) {
                debug_assert!(scope_usage.classes[index] >= request.units);
                scope_usage.classes[index] =
                    scope_usage.classes[index].saturating_sub(request.units);
            }
        }
        if let Some(scope_usage) = state.scopes.get_mut(&scope) {
            debug_assert!(scope_usage.leases > 0);
            scope_usage.leases = scope_usage.leases.saturating_sub(1);
        }
        state.active_leases = state.active_leases.saturating_sub(1);
        if state.scopes.get(&scope).is_some_and(ScopeUsage::is_empty) {
            state.scopes.remove(&scope);
        }
    }
}

/// Exclusive proof that one scope owns the recorded resource units.
///
/// The lease is intentionally not `Clone`. Dropping it releases all units,
/// including when an operation future is cancelled before completion.
///
/// ```compile_fail
/// use pwright_core::{ResourceClass, ResourceLedger, ResourceLimits, ResourceScope};
///
/// let limits = ResourceLimits::deny_all(1, 1)
///     .with_class(ResourceClass::ResponseBodyBytes, 8, 8);
/// let ledger = ResourceLedger::new(limits);
/// let lease = ledger
///     .reserve(ResourceScope::Connection, ResourceClass::ResponseBodyBytes, 4)
///     .unwrap();
/// let duplicated = lease.clone();
/// ```
pub struct ResourceLease {
    ledger: ResourceLedger,
    scope: ResourceScope,
    requests: Vec<ResourceRequest>,
    active: bool,
}

impl ResourceLease {
    pub const fn scope(&self) -> ResourceScope {
        self.scope
    }

    pub fn units(&self, class: ResourceClass) -> Option<u64> {
        self.requests
            .iter()
            .find(|request| request.class == class)
            .map(|request| request.units)
    }

    /// Changes one class in place without releasing the other classes.
    /// Growth is checked atomically against both limits; a rejected resize
    /// leaves the lease and ledger unchanged.
    pub fn try_resize(
        &mut self,
        class: ResourceClass,
        new_units: u64,
    ) -> Result<(), ResourceError> {
        if !self.active {
            return Err(ResourceError::ReleasedLease);
        }
        let request_index = self
            .requests
            .iter()
            .position(|request| request.class == class)
            .ok_or(ResourceError::MissingLeaseClass { class })?;
        let old_units = self.requests[request_index].units;
        if old_units == new_units {
            return Ok(());
        }

        let mut state = self.ledger.lock_state()?;
        let class_index = class.index();
        if new_units > old_units {
            let additional = new_units - old_units;
            let limit = self.ledger.inner.limits.class(class);
            let global = state.current[class_index]
                .checked_add(additional)
                .ok_or_else(|| {
                    state.reject(Some(class));
                    ResourceError::ArithmeticOverflow { class }
                })?;
            if global > limit.global {
                state.reject(Some(class));
                return Err(ResourceError::GlobalLimitExceeded {
                    class,
                    current: state.current[class_index],
                    requested: additional,
                    limit: limit.global,
                });
            }
            let scope_current = state
                .scopes
                .get(&self.scope)
                .expect("an active lease has a scope record")
                .classes[class_index];
            let scoped = scope_current.checked_add(additional).ok_or_else(|| {
                state.reject(Some(class));
                ResourceError::ArithmeticOverflow { class }
            })?;
            if scoped > limit.per_scope {
                state.reject(Some(class));
                return Err(ResourceError::ScopeLimitExceeded {
                    scope: self.scope,
                    class,
                    current: scope_current,
                    requested: additional,
                    limit: limit.per_scope,
                });
            }
            state.current[class_index] = global;
            state.peak[class_index] = state.peak[class_index].max(global);
            state
                .scopes
                .get_mut(&self.scope)
                .expect("an active lease has a scope record")
                .classes[class_index] = scoped;
        } else {
            let released = old_units - new_units;
            state.current[class_index] = state.current[class_index].saturating_sub(released);
            let scope_usage = state
                .scopes
                .get_mut(&self.scope)
                .expect("an active lease has a scope record");
            scope_usage.classes[class_index] =
                scope_usage.classes[class_index].saturating_sub(released);
        }
        drop(state);
        self.requests[request_index].units = new_units;
        Ok(())
    }

    /// Atomically replaces this lease's scope and complete resource request.
    ///
    /// This supports bounded replacement policies such as `DropOldest`: the
    /// old retained value remains admitted if the replacement would exceed a
    /// global, target-scope, or scope-table limit. Successful replacement does
    /// not temporarily count both values because ownership moves in place.
    pub fn try_replace(
        &mut self,
        new_scope: ResourceScope,
        new_requests: &[ResourceRequest],
    ) -> Result<(), ResourceError> {
        if !self.active {
            return Err(ResourceError::ReleasedLease);
        }
        let new_requests = normalize_requests(new_requests)?;
        let old_scope = self.scope;
        let mut old_units = [0_u64; RESOURCE_CLASS_COUNT];
        let mut replacement_units = [0_u64; RESOURCE_CLASS_COUNT];
        for request in &self.requests {
            old_units[request.class.index()] = request.units;
        }
        for request in &new_requests {
            replacement_units[request.class.index()] = request.units;
        }

        let mut state = self.ledger.lock_state()?;
        let old_usage = state
            .scopes
            .get(&old_scope)
            .expect("an active lease has a scope record");
        let removes_old_scope =
            old_scope != new_scope && old_usage.leases == 1 && old_usage.classes == old_units;
        if !state.scopes.contains_key(&new_scope)
            && state.scopes.len() >= self.ledger.inner.limits.scopes
            && !removes_old_scope
        {
            state.reject(None);
            return Err(ResourceError::ScopeCapacityExceeded {
                limit: self.ledger.inner.limits.scopes,
            });
        }

        let mut new_global = state.current;
        let mut new_scoped = [0_u64; RESOURCE_CLASS_COUNT];
        for class in ResourceClass::ALL {
            let index = class.index();
            let without_old = state.current[index]
                .checked_sub(old_units[index])
                .expect("an active lease is included in global accounting");
            let global = without_old
                .checked_add(replacement_units[index])
                .ok_or_else(|| {
                    state.reject(Some(class));
                    ResourceError::ArithmeticOverflow { class }
                })?;
            let limit = self.ledger.inner.limits.class(class);
            if global > limit.global {
                state.reject(Some(class));
                return Err(ResourceError::GlobalLimitExceeded {
                    class,
                    current: without_old,
                    requested: replacement_units[index],
                    limit: limit.global,
                });
            }
            let target_current = state
                .scopes
                .get(&new_scope)
                .map_or(0, |usage| usage.classes[index]);
            let target_without_old = if new_scope == old_scope {
                target_current
                    .checked_sub(old_units[index])
                    .expect("an active lease is included in scope accounting")
            } else {
                target_current
            };
            let scoped = target_without_old
                .checked_add(replacement_units[index])
                .ok_or_else(|| {
                    state.reject(Some(class));
                    ResourceError::ArithmeticOverflow { class }
                })?;
            if scoped > limit.per_scope {
                state.reject(Some(class));
                return Err(ResourceError::ScopeLimitExceeded {
                    scope: new_scope,
                    class,
                    current: target_without_old,
                    requested: replacement_units[index],
                    limit: limit.per_scope,
                });
            }
            new_global[index] = global;
            new_scoped[index] = scoped;
        }

        {
            let old_usage = state
                .scopes
                .get_mut(&old_scope)
                .expect("an active lease has a scope record");
            for request in &self.requests {
                let index = request.class.index();
                old_usage.classes[index] = old_usage.classes[index]
                    .checked_sub(request.units)
                    .expect("an active lease is included in scope accounting");
            }
            old_usage.leases = old_usage
                .leases
                .checked_sub(1)
                .expect("an active lease is included in lease accounting");
        }
        if state
            .scopes
            .get(&old_scope)
            .is_some_and(ScopeUsage::is_empty)
        {
            state.scopes.remove(&old_scope);
        }
        let target = state
            .scopes
            .entry(new_scope)
            .or_insert_with(ScopeUsage::new);
        target.classes = new_scoped;
        target.leases += 1;
        state.current = new_global;
        for class in ResourceClass::ALL {
            let index = class.index();
            state.peak[index] = state.peak[index].max(state.current[index]);
        }
        state.peak_scopes = state.peak_scopes.max(state.scopes.len());
        drop(state);

        self.scope = new_scope;
        self.requests = new_requests;
        Ok(())
    }

    pub fn release(mut self) {
        if self.active {
            self.ledger.release(self.scope, &self.requests);
            self.active = false;
        }
    }
}

impl Debug for ResourceLease {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResourceLease")
            .field("scope", &self.scope)
            .field("requests", &self.requests)
            .field("active", &self.active)
            .finish_non_exhaustive()
    }
}

impl Drop for ResourceLease {
    fn drop(&mut self) {
        if self.active {
            self.ledger.release(self.scope, &self.requests);
            self.active = false;
        }
    }
}

/// Immutable retained bytes whose lifetime is coupled to a byte-budget lease.
///
/// ```compile_fail
/// use pwright_core::{BudgetedBytes, ResourceClass, ResourceLedger, ResourceLimits, ResourceScope};
///
/// let limits = ResourceLimits::deny_all(1, 1)
///     .with_class(ResourceClass::ScreenshotBytes, 8, 8);
/// let ledger = ResourceLedger::new(limits);
/// let bytes = BudgetedBytes::copy_from_slice(
///     &ledger,
///     ResourceScope::Connection,
///     ResourceClass::ScreenshotBytes,
///     &[1, 2],
/// ).unwrap();
/// let duplicate = bytes.clone();
/// ```
pub struct BudgetedBytes {
    scope: ResourceScope,
    class: ResourceClass,
    bytes: Box<[u8]>,
    _lease: Option<ResourceLease>,
}

impl BudgetedBytes {
    pub fn copy_from_slice(
        ledger: &ResourceLedger,
        scope: ResourceScope,
        class: ResourceClass,
        bytes: &[u8],
    ) -> Result<Self, ResourceError> {
        if !class.is_byte_class() {
            return Err(ResourceError::ClassIsNotBytes { class });
        }
        let units =
            u64::try_from(bytes.len()).map_err(|_| ResourceError::ArithmeticOverflow { class })?;
        let lease = if units == 0 {
            None
        } else {
            Some(ledger.reserve(scope, class, units)?)
        };
        Ok(Self {
            scope,
            class,
            bytes: bytes.into(),
            _lease: lease,
        })
    }

    /// Couples an already allocated byte vector to a lease acquired before
    /// that allocation began.
    ///
    /// The lease is shrunk to the exact retained length without a
    /// release/reacquire window. This is intended for bounded decoders that
    /// reserve their maximum output before allocating it.
    pub fn from_reserved(
        mut lease: ResourceLease,
        class: ResourceClass,
        bytes: Vec<u8>,
    ) -> Result<Self, ResourceError> {
        if !class.is_byte_class() {
            return Err(ResourceError::ClassIsNotBytes { class });
        }
        let actual =
            u64::try_from(bytes.len()).map_err(|_| ResourceError::ArithmeticOverflow { class })?;
        let reserved = lease
            .units(class)
            .ok_or(ResourceError::MissingLeaseClass { class })?;
        if actual > reserved {
            return Err(ResourceError::ReservationTooSmall {
                class,
                reserved,
                actual,
            });
        }
        let scope = lease.scope();
        let lease = if actual == 0 {
            lease.release();
            None
        } else {
            lease.try_resize(class, actual)?;
            Some(lease)
        };
        Ok(Self {
            scope,
            class,
            bytes: bytes.into_boxed_slice(),
            _lease: lease,
        })
    }

    pub const fn scope(&self) -> ResourceScope {
        self.scope
    }

    pub const fn class(&self) -> ResourceClass {
        self.class
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl Debug for BudgetedBytes {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BudgetedBytes")
            .field("scope", &self.scope)
            .field("class", &self.class)
            .field("len", &self.bytes.len())
            .finish_non_exhaustive()
    }
}

/// Immutable UTF-8 text whose lifetime is coupled to a byte-budget lease.
///
/// The value is deliberately non-cloneable: cloning retained text would need
/// a second reservation. Callers can borrow it as `str` without escaping the
/// lease that accounts for the owned allocation.
pub struct BudgetedUtf8 {
    scope: ResourceScope,
    class: ResourceClass,
    text: Box<str>,
    _lease: Option<ResourceLease>,
}

impl BudgetedUtf8 {
    pub fn copy_from_str(
        ledger: &ResourceLedger,
        scope: ResourceScope,
        class: ResourceClass,
        text: &str,
    ) -> Result<Self, ResourceError> {
        if !class.is_byte_class() {
            return Err(ResourceError::ClassIsNotBytes { class });
        }
        let units =
            u64::try_from(text.len()).map_err(|_| ResourceError::ArithmeticOverflow { class })?;
        let lease = if units == 0 {
            None
        } else {
            Some(ledger.reserve(scope, class, units)?)
        };
        Ok(Self {
            scope,
            class,
            text: text.into(),
            _lease: lease,
        })
    }

    /// Couples an owned UTF-8 string to a lease acquired before the string was
    /// produced, shrinking the reservation to the exact retained length.
    pub fn from_reserved(
        mut lease: ResourceLease,
        class: ResourceClass,
        text: String,
    ) -> Result<Self, ResourceError> {
        if !class.is_byte_class() {
            return Err(ResourceError::ClassIsNotBytes { class });
        }
        let actual =
            u64::try_from(text.len()).map_err(|_| ResourceError::ArithmeticOverflow { class })?;
        let reserved = lease
            .units(class)
            .ok_or(ResourceError::MissingLeaseClass { class })?;
        if actual > reserved {
            return Err(ResourceError::ReservationTooSmall {
                class,
                reserved,
                actual,
            });
        }
        let scope = lease.scope();
        let lease = if actual == 0 {
            lease.release();
            None
        } else {
            lease.try_resize(class, actual)?;
            Some(lease)
        };
        Ok(Self {
            scope,
            class,
            text: text.into_boxed_str(),
            _lease: lease,
        })
    }

    pub const fn scope(&self) -> ResourceScope {
        self.scope
    }

    pub const fn class(&self) -> ResourceClass {
        self.class
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

impl AsRef<str> for BudgetedUtf8 {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for BudgetedUtf8 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Debug for BudgetedUtf8 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BudgetedUtf8")
            .field("scope", &self.scope)
            .field("class", &self.class)
            .field("len", &self.text.len())
            .finish_non_exhaustive()
    }
}

fn normalize_requests(requests: &[ResourceRequest]) -> Result<Vec<ResourceRequest>, ResourceError> {
    if requests.is_empty() {
        return Err(ResourceError::EmptyRequest);
    }
    if requests.len() > RESOURCE_CLASS_COUNT {
        return Err(ResourceError::TooManyClasses {
            actual: requests.len(),
            limit: RESOURCE_CLASS_COUNT,
        });
    }
    let mut classes = [false; RESOURCE_CLASS_COUNT];
    let mut normalized = Vec::with_capacity(requests.len());
    for request in requests {
        if request.units == 0 {
            return Err(ResourceError::ZeroUnits {
                class: request.class,
            });
        }
        let class_index = request.class.index();
        if classes[class_index] {
            return Err(ResourceError::DuplicateClass {
                class: request.class,
            });
        }
        classes[class_index] = true;
        normalized.push(*request);
    }
    normalized.sort_by_key(|request| request.class);
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};
    use std::thread;

    use crate::{CallId, ObjectId};

    use super::{
        BudgetedBytes, BudgetedUtf8, ProductionResourceError, ProductionResourceLedger,
        ResourceCapacityBoundary, ResourceClass, ResourceError, ResourceLedger, ResourceLimit,
        ResourceLimits, ResourceRequest, ResourceRequirements, ResourceScope,
    };

    fn limits() -> ResourceLimits {
        ResourceLimits::deny_all(8, 8)
            .with_class(ResourceClass::EventItem, 4, 2)
            .with_class(ResourceClass::EventBytes, 100, 60)
            .with_class(ResourceClass::ResponseBodyBytes, 100, 80)
            .with_class(ResourceClass::ScreenshotBytes, 16, 16)
            .with_class(ResourceClass::DomContentBytes, 32, 32)
    }

    #[test]
    fn lease_drop_releases_current_usage_and_preserves_peak_metrics() {
        // Arrange
        let ledger = ResourceLedger::new(limits());
        let lease = ledger
            .reserve(
                ResourceScope::Object(ObjectId::new(7)),
                ResourceClass::EventBytes,
                40,
            )
            .expect("event bytes should fit");

        // Act
        drop(lease);
        let metrics = ledger.metrics().expect("metrics should be available");

        // Assert
        let bytes = metrics.class(ResourceClass::EventBytes);
        assert_eq!(bytes.current, 0);
        assert_eq!(bytes.peak, 40);
        assert_eq!(metrics.live_scopes, 0);
        assert_eq!(metrics.peak_scopes, 1);
        assert_eq!(metrics.active_leases, 0);
        assert_eq!(metrics.peak_leases, 1);
    }

    #[test]
    fn global_limit_combines_usage_from_independent_scopes() {
        // Arrange
        let ledger = ResourceLedger::new(limits());
        let first = ledger
            .reserve(
                ResourceScope::Object(ObjectId::new(1)),
                ResourceClass::EventBytes,
                60,
            )
            .expect("first scope should fit");

        // Act
        let error = ledger
            .reserve(
                ResourceScope::Object(ObjectId::new(2)),
                ResourceClass::EventBytes,
                41,
            )
            .expect_err("aggregate bytes should exceed the global cap");

        // Assert
        assert_eq!(
            error,
            ResourceError::GlobalLimitExceeded {
                class: ResourceClass::EventBytes,
                current: 60,
                requested: 41,
                limit: 100,
            }
        );
        assert_eq!(
            ledger
                .metrics()
                .expect("metrics should be available")
                .class(ResourceClass::EventBytes)
                .current,
            60
        );
        drop(first);
    }

    #[test]
    fn per_scope_limit_prevents_one_page_from_consuming_the_global_pool() {
        // Arrange
        let ledger = ResourceLedger::new(limits());
        let scope = ResourceScope::Object(ObjectId::new(3));
        let first = ledger
            .reserve(scope, ResourceClass::EventBytes, 40)
            .expect("first page reservation should fit");

        // Act
        let error = ledger
            .reserve(scope, ResourceClass::EventBytes, 21)
            .expect_err("page should exceed its per-scope cap");

        // Assert
        assert_eq!(
            error,
            ResourceError::ScopeLimitExceeded {
                scope,
                class: ResourceClass::EventBytes,
                current: 40,
                requested: 21,
                limit: 60,
            }
        );
        let scope_metrics = ledger
            .scope_metrics(scope)
            .expect("scope metrics should be available")
            .expect("active page scope should be present");
        assert_eq!(scope_metrics.units(ResourceClass::EventBytes), 40);
        assert_eq!(scope_metrics.active_leases, 1);
        drop(first);
        assert_eq!(
            ledger
                .scope_metrics(scope)
                .expect("scope metrics should be available after release"),
            None
        );
    }

    #[test]
    fn multi_class_failure_rolls_back_every_class() {
        // Arrange
        let ledger = ResourceLedger::new(limits());
        let request = [
            ResourceRequest::new(ResourceClass::EventItem, 1),
            ResourceRequest::new(ResourceClass::EventBytes, 61),
        ];

        // Act
        let error = ledger
            .reserve_many(ResourceScope::Connection, &request)
            .expect_err("second class should exceed its scope cap");
        let metrics = ledger.metrics().expect("metrics should be available");

        // Assert
        assert!(matches!(error, ResourceError::ScopeLimitExceeded { .. }));
        assert_eq!(metrics.class(ResourceClass::EventItem).current, 0);
        assert_eq!(metrics.class(ResourceClass::EventBytes).current, 0);
        assert_eq!(metrics.active_leases, 0);
        assert_eq!(metrics.live_scopes, 0);
    }

    #[test]
    fn lease_resize_supports_stream_growth_and_rejects_growth_without_mutation() {
        // Arrange
        let ledger = ResourceLedger::new(limits());
        let scope = ResourceScope::Call(CallId::new(9));
        let mut lease = ledger
            .reserve(scope, ResourceClass::ResponseBodyBytes, 20)
            .expect("initial body chunk should fit");
        lease
            .try_resize(ResourceClass::ResponseBodyBytes, 70)
            .expect("body should grow within its scope cap");

        // Act
        let error = lease
            .try_resize(ResourceClass::ResponseBodyBytes, 81)
            .expect_err("body must not grow past its scope cap");
        let metrics = ledger.metrics().expect("metrics should be available");

        // Assert
        assert!(matches!(error, ResourceError::ScopeLimitExceeded { .. }));
        assert_eq!(lease.units(ResourceClass::ResponseBodyBytes), Some(70));
        assert_eq!(metrics.class(ResourceClass::ResponseBodyBytes).current, 70);
        assert_eq!(metrics.class(ResourceClass::ResponseBodyBytes).peak, 70);
    }

    #[test]
    fn lease_replacement_atomically_moves_scope_and_resource_units() {
        // Arrange
        let ledger = ResourceLedger::new(limits());
        let old_scope = ResourceScope::Object(ObjectId::new(1));
        let new_scope = ResourceScope::Object(ObjectId::new(2));
        let mut lease = ledger
            .reserve_many(
                old_scope,
                &[
                    ResourceRequest::new(ResourceClass::EventItem, 1),
                    ResourceRequest::new(ResourceClass::EventBytes, 40),
                ],
            )
            .expect("old event should fit");

        // Act
        lease
            .try_replace(
                new_scope,
                &[
                    ResourceRequest::new(ResourceClass::EventItem, 1),
                    ResourceRequest::new(ResourceClass::EventBytes, 50),
                ],
            )
            .expect("replacement event should fit");
        let old = ledger
            .scope_metrics(old_scope)
            .expect("old scope metrics should be available");
        let new = ledger
            .scope_metrics(new_scope)
            .expect("new scope metrics should be available")
            .expect("new scope should exist");
        let metrics = ledger.metrics().expect("metrics should be available");

        // Assert
        assert!(old.is_none());
        assert_eq!(lease.scope(), new_scope);
        assert_eq!(lease.units(ResourceClass::EventItem), Some(1));
        assert_eq!(lease.units(ResourceClass::EventBytes), Some(50));
        assert_eq!(new.units(ResourceClass::EventItem), 1);
        assert_eq!(new.units(ResourceClass::EventBytes), 50);
        assert_eq!(new.active_leases, 1);
        assert_eq!(metrics.class(ResourceClass::EventBytes).current, 50);
        assert_eq!(metrics.active_leases, 1);
    }

    #[test]
    fn rejected_lease_replacement_preserves_the_old_admission() {
        // Arrange
        let ledger = ResourceLedger::new(limits());
        let old_scope = ResourceScope::Object(ObjectId::new(1));
        let target_scope = ResourceScope::Object(ObjectId::new(2));
        let mut old = ledger
            .reserve_many(
                old_scope,
                &[
                    ResourceRequest::new(ResourceClass::EventItem, 1),
                    ResourceRequest::new(ResourceClass::EventBytes, 40),
                ],
            )
            .expect("old event should fit");
        let target = ledger
            .reserve_many(
                target_scope,
                &[
                    ResourceRequest::new(ResourceClass::EventItem, 1),
                    ResourceRequest::new(ResourceClass::EventBytes, 50),
                ],
            )
            .expect("target event should fit");

        // Act
        let error = old
            .try_replace(
                target_scope,
                &[
                    ResourceRequest::new(ResourceClass::EventItem, 1),
                    ResourceRequest::new(ResourceClass::EventBytes, 20),
                ],
            )
            .expect_err("target scope should reject seventy event bytes");
        let old_metrics = ledger
            .scope_metrics(old_scope)
            .expect("old scope metrics should be available")
            .expect("old scope must remain admitted");
        let target_metrics = ledger
            .scope_metrics(target_scope)
            .expect("target scope metrics should be available")
            .expect("target scope must remain admitted");

        // Assert
        assert_eq!(
            error,
            ResourceError::ScopeLimitExceeded {
                scope: target_scope,
                class: ResourceClass::EventBytes,
                current: 50,
                requested: 20,
                limit: 60,
            }
        );
        assert_eq!(old.scope(), old_scope);
        assert_eq!(old.units(ResourceClass::EventBytes), Some(40));
        assert_eq!(old_metrics.units(ResourceClass::EventBytes), 40);
        assert_eq!(target_metrics.units(ResourceClass::EventBytes), 50);
        drop(target);
    }

    #[test]
    fn sole_lease_replacement_reuses_a_full_scope_table_slot() {
        // Arrange
        let ledger = ResourceLedger::new(ResourceLimits::deny_all(1, 1).with_class(
            ResourceClass::EventItem,
            1,
            1,
        ));
        let old_scope = ResourceScope::Object(ObjectId::new(1));
        let new_scope = ResourceScope::Object(ObjectId::new(2));
        let mut lease = ledger
            .reserve(old_scope, ResourceClass::EventItem, 1)
            .expect("only scope slot should be admitted");

        // Act
        lease
            .try_replace(
                new_scope,
                &[ResourceRequest::new(ResourceClass::EventItem, 1)],
            )
            .expect("moving the sole lease should reuse its scope slot");
        let metrics = ledger.metrics().expect("metrics should be available");

        // Assert
        assert_eq!(lease.scope(), new_scope);
        assert_eq!(metrics.live_scopes, 1);
        assert_eq!(metrics.peak_scopes, 1);
        assert_eq!(metrics.active_leases, 1);
    }

    #[test]
    fn lease_and_scope_metadata_have_independent_hard_limits() {
        // Arrange
        let lease_limited = ResourceLedger::new(ResourceLimits::deny_all(2, 1).with_class(
            ResourceClass::EventItem,
            2,
            2,
        ));
        let first = lease_limited
            .reserve(ResourceScope::Connection, ResourceClass::EventItem, 1)
            .expect("first lease should fit");
        let scope_limited = ResourceLedger::new(ResourceLimits::deny_all(1, 2).with_class(
            ResourceClass::EventItem,
            2,
            1,
        ));
        let second = scope_limited
            .reserve(
                ResourceScope::Object(ObjectId::new(1)),
                ResourceClass::EventItem,
                1,
            )
            .expect("first scope should fit");

        // Act
        let lease_error = lease_limited
            .reserve(ResourceScope::Connection, ResourceClass::EventItem, 1)
            .expect_err("second lease should exceed metadata bound");
        let scope_error = scope_limited
            .reserve(
                ResourceScope::Object(ObjectId::new(2)),
                ResourceClass::EventItem,
                1,
            )
            .expect_err("second scope should exceed metadata bound");

        // Assert
        assert_eq!(
            lease_error,
            ResourceError::LeaseCapacityExceeded { limit: 1 }
        );
        assert_eq!(
            scope_error,
            ResourceError::ScopeCapacityExceeded { limit: 1 }
        );
        drop((first, second));
    }

    #[test]
    fn concurrent_reservations_cannot_oversubscribe_one_global_limit() {
        // Arrange
        let ledger = Arc::new(ResourceLedger::new(
            ResourceLimits::deny_all(2, 2).with_class(ResourceClass::ScreenshotBytes, 10, 10),
        ));
        let barrier = Arc::new(Barrier::new(3));
        let workers = (0..2)
            .map(|id| {
                let ledger = Arc::clone(&ledger);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    ledger.reserve(
                        ResourceScope::extension(1, id),
                        ResourceClass::ScreenshotBytes,
                        6,
                    )
                })
            })
            .collect::<Vec<_>>();

        // Act
        barrier.wait();
        let outcomes = workers
            .into_iter()
            .map(|worker| worker.join().expect("worker should not panic"))
            .collect::<Vec<_>>();

        // Assert
        assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(outcomes.iter().filter(|result| result.is_err()).count(), 1);
        drop(outcomes);
        assert_eq!(
            ledger
                .metrics()
                .expect("metrics should be available")
                .class(ResourceClass::ScreenshotBytes)
                .current,
            0
        );
    }

    #[test]
    fn budgeted_bytes_hold_and_release_their_exact_retained_size() {
        // Arrange
        let ledger = ResourceLedger::new(limits());
        let bytes = [1_u8, 2, 3, 4];

        // Act
        let retained = BudgetedBytes::copy_from_slice(
            &ledger,
            ResourceScope::Object(ObjectId::new(5)),
            ResourceClass::ScreenshotBytes,
            &bytes,
        )
        .expect("screenshot should fit");

        // Assert
        assert_eq!(retained.as_bytes(), bytes);
        assert_eq!(retained.len(), 4);
        assert_eq!(
            ledger
                .metrics()
                .expect("metrics should be available")
                .class(ResourceClass::ScreenshotBytes)
                .current,
            4
        );
        drop(retained);
        assert_eq!(
            ledger
                .metrics()
                .expect("metrics should be available")
                .class(ResourceClass::ScreenshotBytes)
                .current,
            0
        );
    }

    #[test]
    fn budgeted_bytes_reject_count_classes_before_copying() {
        // Arrange
        let ledger = ResourceLedger::new(limits());

        // Act
        let error = BudgetedBytes::copy_from_slice(
            &ledger,
            ResourceScope::Connection,
            ResourceClass::EventItem,
            &[1, 2],
        )
        .expect_err("item counts must not account byte storage");

        // Assert
        assert_eq!(
            error,
            ResourceError::ClassIsNotBytes {
                class: ResourceClass::EventItem,
            }
        );
        assert_eq!(
            ledger
                .metrics()
                .expect("metrics should be available")
                .rejected_reservations,
            0
        );
    }

    #[test]
    fn budgeted_bytes_shrink_a_preallocated_lease_without_reacquiring() {
        // Arrange
        let ledger = ResourceLedger::new(limits());
        let lease = ledger
            .reserve(
                ResourceScope::Object(ObjectId::new(5)),
                ResourceClass::ScreenshotBytes,
                8,
            )
            .unwrap();

        // Act
        let retained =
            BudgetedBytes::from_reserved(lease, ResourceClass::ScreenshotBytes, vec![1, 2, 3, 4])
                .unwrap();

        // Assert
        assert_eq!(retained.as_bytes(), &[1, 2, 3, 4]);
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ScreenshotBytes)
                .current,
            4
        );
        drop(retained);
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ScreenshotBytes)
                .current,
            0
        );
    }

    #[test]
    fn budgeted_bytes_reject_data_larger_than_its_preallocated_lease() {
        // Arrange
        let ledger = ResourceLedger::new(limits());
        let lease = ledger
            .reserve(ResourceScope::Connection, ResourceClass::ScreenshotBytes, 2)
            .unwrap();

        // Act
        let error =
            BudgetedBytes::from_reserved(lease, ResourceClass::ScreenshotBytes, vec![1, 2, 3])
                .unwrap_err();

        // Assert
        assert_eq!(
            error,
            ResourceError::ReservationTooSmall {
                class: ResourceClass::ScreenshotBytes,
                reserved: 2,
                actual: 3,
            }
        );
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ScreenshotBytes)
                .current,
            0
        );
    }

    #[test]
    fn budgeted_utf8_holds_exact_text_and_releases_with_the_value() {
        // Arrange
        let ledger = ResourceLedger::new(limits());
        let scope = ResourceScope::Call(CallId::new(17));

        // Act
        let content = BudgetedUtf8::copy_from_str(
            &ledger,
            scope,
            ResourceClass::DomContentBytes,
            "<p>你好</p>",
        )
        .expect("DOM content should fit");

        // Assert
        assert_eq!(content.as_str(), "<p>你好</p>");
        assert_eq!(content.as_ref(), "<p>你好</p>");
        assert_eq!(content.to_string(), "<p>你好</p>");
        assert_eq!(content.scope(), scope);
        assert_eq!(content.class(), ResourceClass::DomContentBytes);
        assert_eq!(content.len(), 13);
        assert!(!content.is_empty());
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::DomContentBytes)
                .current,
            13
        );
        drop(content);
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::DomContentBytes)
                .current,
            0
        );
    }

    #[test]
    fn budgeted_utf8_shrinks_a_preallocated_lease_without_reacquiring() {
        // Arrange
        let ledger = ResourceLedger::new(limits());
        let lease = ledger
            .reserve(
                ResourceScope::Connection,
                ResourceClass::DomContentBytes,
                32,
            )
            .expect("maximum result should reserve");

        // Act
        let content = BudgetedUtf8::from_reserved(
            lease,
            ResourceClass::DomContentBytes,
            "<html></html>".to_owned(),
        )
        .expect("owned content should fit its reservation");

        // Assert
        assert_eq!(content.as_str(), "<html></html>");
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::DomContentBytes)
                .current,
            13
        );
        drop(content);
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::DomContentBytes)
                .current,
            0
        );
    }

    #[test]
    fn budgeted_utf8_rejects_text_larger_than_its_preallocated_lease() {
        // Arrange
        let ledger = ResourceLedger::new(limits());
        let lease = ledger
            .reserve(ResourceScope::Connection, ResourceClass::DomContentBytes, 2)
            .expect("small reservation should fit");

        // Act
        let error =
            BudgetedUtf8::from_reserved(lease, ResourceClass::DomContentBytes, "abc".to_owned())
                .expect_err("larger retained text must fail closed");

        // Assert
        assert_eq!(
            error,
            ResourceError::ReservationTooSmall {
                class: ResourceClass::DomContentBytes,
                reserved: 2,
                actual: 3,
            }
        );
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::DomContentBytes)
                .current,
            0
        );
    }

    #[test]
    fn invalid_multi_requests_never_create_a_scope() {
        // Arrange
        let ledger = ResourceLedger::new(limits());
        let requests = [
            ResourceRequest::new(ResourceClass::EventItem, 1),
            ResourceRequest::new(ResourceClass::EventItem, 1),
        ];
        let too_many_requests = ResourceClass::ALL
            .into_iter()
            .chain([ResourceClass::EventItem])
            .map(|class| ResourceRequest::new(class, 1))
            .collect::<Vec<_>>();

        // Act
        let duplicate = ledger
            .reserve_many(ResourceScope::Connection, &requests)
            .expect_err("duplicate classes should fail validation");
        let zero = ledger
            .reserve(ResourceScope::Connection, ResourceClass::EventBytes, 0)
            .expect_err("zero-unit lease should not retain metadata");
        let too_many = ledger
            .reserve_many(ResourceScope::Connection, &too_many_requests)
            .expect_err("request vectors are bounded before normalization allocation");

        // Assert
        assert_eq!(
            duplicate,
            ResourceError::DuplicateClass {
                class: ResourceClass::EventItem,
            }
        );
        assert_eq!(
            zero,
            ResourceError::ZeroUnits {
                class: ResourceClass::EventBytes,
            }
        );
        assert_eq!(
            too_many,
            ResourceError::TooManyClasses {
                actual: ResourceClass::ALL.len() + 1,
                limit: ResourceClass::ALL.len(),
            }
        );
        assert_eq!(
            ledger
                .metrics()
                .expect("metrics should be available")
                .live_scopes,
            0
        );
    }

    #[test]
    fn production_plan_claims_share_one_validated_ledger() {
        // Arrange
        let plan = ResourceRequirements::none()
            .with_class(ResourceClass::PendingCall, 2, 1)
            .with_class(ResourceClass::WireBytes, 16, 16)
            .with_metadata(4, 4);
        let production = ProductionResourceLedger::new(
            ResourceLimits::deny_all(4, 4)
                .with_class(ResourceClass::PendingCall, 2, 1)
                .with_class(ResourceClass::WireBytes, 16, 16),
            plan,
        )
        .expect("declared plan should fit configured limits");
        let pending = production
            .claim(
                ResourceRequirements::none()
                    .with_class(ResourceClass::PendingCall, 2, 1)
                    .with_metadata(2, 2),
            )
            .expect("declared runtime component should claim the ledger");
        let wire = production
            .claim(
                ResourceRequirements::none()
                    .with_class(ResourceClass::WireBytes, 16, 16)
                    .with_metadata(1, 1),
            )
            .expect("declared wire component should claim the ledger");

        // Act
        let call = pending
            .reserve(
                ResourceScope::Call(CallId::new(1)),
                ResourceClass::PendingCall,
                1,
            )
            .expect("pending call should fit");
        let bytes = wire
            .reserve(
                ResourceScope::Call(CallId::new(1)),
                ResourceClass::WireBytes,
                8,
            )
            .expect("wire bytes should fit");
        let active = production.metrics().expect("metrics should load");
        drop((call, bytes));
        let released = production.metrics().expect("metrics should load");

        // Assert
        assert_eq!(production.plan(), plan);
        assert_eq!(active.class(ResourceClass::PendingCall).current, 1);
        assert_eq!(active.class(ResourceClass::WireBytes).current, 8);
        assert_eq!(active.active_leases, 2);
        assert_eq!(released.active_leases, 0);
    }

    #[test]
    fn production_plan_rejects_insufficient_configured_class_capacity() {
        // Arrange
        let plan = ResourceRequirements::none()
            .with_class(ResourceClass::PendingCall, 2, 1)
            .with_metadata(2, 2);

        // Act
        let global = ProductionResourceLedger::new(
            ResourceLimits::deny_all(2, 2).with_class(ResourceClass::PendingCall, 1, 1),
            plan,
        )
        .expect_err("global limit below plan should fail");
        let per_scope = ProductionResourceLedger::new(
            ResourceLimits::deny_all(2, 2).with_class(ResourceClass::PendingCall, 2, 0),
            plan,
        )
        .expect_err("per-scope limit below plan should fail");

        // Assert
        assert_eq!(
            global,
            ProductionResourceError::InsufficientGlobal {
                boundary: ResourceCapacityBoundary::ConfiguredLimits,
                class: ResourceClass::PendingCall,
                required: 2,
                available: 1,
            }
        );
        assert_eq!(
            per_scope,
            ProductionResourceError::InsufficientPerScope {
                boundary: ResourceCapacityBoundary::ConfiguredLimits,
                class: ResourceClass::PendingCall,
                required: 1,
                available: 0,
            }
        );
    }

    #[test]
    fn production_claim_rejects_component_omitted_from_declared_plan() {
        // Arrange
        let plan = ResourceRequirements::none()
            .with_class(ResourceClass::PendingCall, 1, 1)
            .with_metadata(2, 2);
        let production = ProductionResourceLedger::new(
            ResourceLimits::deny_all(2, 2)
                .with_class(ResourceClass::PendingCall, 1, 1)
                .with_class(ResourceClass::WireBytes, 16, 16),
            plan,
        )
        .expect("configured limits should cover the declared plan");

        // Act
        let error = production
            .claim(
                ResourceRequirements::none()
                    .with_class(ResourceClass::WireBytes, 16, 16)
                    .with_metadata(1, 1),
            )
            .expect_err("undeclared wire component must not obtain the ledger");

        // Assert
        assert_eq!(
            error,
            ProductionResourceError::InsufficientGlobal {
                boundary: ResourceCapacityBoundary::DeclaredPlan,
                class: ResourceClass::WireBytes,
                required: 16,
                available: 0,
            }
        );
    }

    #[test]
    fn production_plan_rejects_missing_or_insufficient_metadata_capacity() {
        // Arrange
        let missing_metadata =
            ResourceRequirements::none().with_class(ResourceClass::PendingCall, 1, 1);
        let planned_metadata = missing_metadata.with_metadata(2, 3);

        // Act
        let missing = ProductionResourceLedger::new(
            ResourceLimits::deny_all(2, 3).with_class(ResourceClass::PendingCall, 1, 1),
            missing_metadata,
        )
        .expect_err("class requirements must declare metadata capacity");
        let insufficient = ProductionResourceLedger::new(
            ResourceLimits::deny_all(1, 3).with_class(ResourceClass::PendingCall, 1, 1),
            planned_metadata,
        )
        .expect_err("scope metadata below plan should fail");

        // Assert
        assert_eq!(missing, ProductionResourceError::MissingScopeRequirement);
        assert_eq!(
            insufficient,
            ProductionResourceError::InsufficientScopes {
                boundary: ResourceCapacityBoundary::ConfiguredLimits,
                required: 2,
                available: 1,
            }
        );
    }

    #[test]
    fn production_wrapper_rejects_a_previously_used_ledger() {
        // Arrange
        let limits = ResourceLimits::deny_all(1, 1).with_class(ResourceClass::PendingCall, 1, 1);
        let ledger = ResourceLedger::new(limits);
        let lease = ledger
            .reserve(
                ResourceScope::Call(CallId::new(1)),
                ResourceClass::PendingCall,
                1,
            )
            .expect("test lease should fit");
        drop(lease);
        let plan = ResourceRequirements::none()
            .with_class(ResourceClass::PendingCall, 1, 1)
            .with_metadata(1, 1);

        // Act
        let error = ProductionResourceLedger::from_pristine_ledger(ledger, plan)
            .expect_err("historically used ledger should not become a production root");

        // Assert
        assert_eq!(
            error,
            ProductionResourceError::LedgerNotPristine {
                live_scopes: 0,
                active_leases: 0,
                peak_scopes: 1,
                peak_leases: 1,
                rejected_reservations: 0,
            }
        );
    }

    #[test]
    fn production_wrapper_rejects_a_ledger_with_rejection_history() {
        // Arrange
        let limits = ResourceLimits::deny_all(1, 1);
        let ledger = ResourceLedger::new(limits);
        let error = ledger
            .reserve(
                ResourceScope::Call(CallId::new(1)),
                ResourceClass::PendingCall,
                1,
            )
            .expect_err("deny-all ledger should reject the reservation");
        assert!(matches!(error, ResourceError::GlobalLimitExceeded { .. }));

        // Act
        let production =
            ProductionResourceLedger::from_pristine_ledger(ledger, ResourceRequirements::none())
                .expect_err("rejection history means the ledger was already observed");

        // Assert
        assert_eq!(
            production,
            ProductionResourceError::LedgerNotPristine {
                live_scopes: 0,
                active_leases: 0,
                peak_scopes: 0,
                peak_leases: 0,
                rejected_reservations: 1,
            }
        );
    }

    #[test]
    fn requirement_merge_keeps_the_strongest_installed_component_boundary() {
        // Arrange
        let runtime = ResourceRequirements::none()
            .with_class(ResourceClass::PendingCall, 4, 1)
            .with_metadata(8, 12);
        let client = ResourceRequirements::none()
            .with_class(ResourceClass::PendingCall, 2, 2)
            .with_class(ResourceClass::WireBytes, 64, 32)
            .with_metadata(4, 6);

        // Act
        let combined = runtime.merge(client);

        // Assert
        assert_eq!(
            combined.class(ResourceClass::PendingCall),
            ResourceLimit::new(4, 2)
        );
        assert_eq!(
            combined.class(ResourceClass::WireBytes),
            ResourceLimit::new(64, 32)
        );
        assert_eq!(combined.scopes, 8);
        assert_eq!(combined.leases, 12);
    }

    #[test]
    fn concurrent_requirement_composition_adds_simultaneously_live_peaks() {
        // Arrange
        let topology = ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, 64, 64)
            .with_metadata(1, 2);
        let routes = ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, 16, 16)
            .with_metadata(1, 1);

        // Act
        let combined = topology.combine_concurrent(routes);

        // Assert
        assert_eq!(
            combined.class(ResourceClass::BackendStateBytes),
            ResourceLimit::new(80, 80)
        );
        assert_eq!(combined.scopes, 2);
        assert_eq!(combined.leases, 3);
    }
}
