//! Budgeted paired access to the existing authoritative Frame/world table.
//! No second topology copy, subscription, polling task or browser operation.

use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use pwright_cdp::CdpClient;
use pwright_core::{
    ObjectId, ProductionResourceError, ProductionResourceLedger, ResourceClass, ResourceError,
    ResourceLease, ResourceLedger, ResourceRequirements, ResourceScope,
};

use crate::{CdpPageRouteError, CdpPageServices};

// The accessor retains the complete service owner, including its route leases.
// A snapshot may retain two unique IDs and its client's target/session strings
// after those worlds leave the current table. Do not charge these only to the
// current route table or release this allowance when readiness returns.
// Includes all inline service factories/profiles, plus Arc allocation and
// allocator allowance. Derive this from the actual owner type, not a guessed
// fixed allowance that can become too small as the composition grows.
const STATE_BYTES: u64 = (std::mem::size_of::<State>() + 256) as u64;
pub(crate) const SNAPSHOT_BYTES: u64 = 8192;
pub(crate) const NATIVE_ID_BYTES: usize = 1024;
type Result<T> = std::result::Result<T, FrameWorldAccessError>;

#[derive(Debug)]
pub enum FrameWorldAccessError {
    Setup(&'static str),
    Capacity { limit: usize },
    Topology(CdpPageRouteError),
    Resource(ResourceError),
    Production(ProductionResourceError),
}
impl Display for FrameWorldAccessError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Setup(message) => write!(formatter, "invalid Frame world access: {message}"),
            Self::Capacity { limit } => {
                write!(formatter, "Frame world snapshot capacity {limit} exhausted")
            }
            Self::Topology(error) => Display::fmt(error, formatter),
            Self::Resource(error) => Display::fmt(error, formatter),
            Self::Production(error) => Display::fmt(error, formatter),
        }
    }
}
impl std::error::Error for FrameWorldAccessError {}
impl From<CdpPageRouteError> for FrameWorldAccessError {
    fn from(error: CdpPageRouteError) -> Self {
        Self::Topology(error)
    }
}
impl From<ResourceError> for FrameWorldAccessError {
    fn from(error: ResourceError) -> Self {
        Self::Resource(error)
    }
}
impl From<ProductionResourceError> for FrameWorldAccessError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Production(error)
    }
}

/// Explicit extra resources for waiting AND retained paired-world snapshots.
/// Merge `requirements()` with the attach profile's extension requirements.
#[derive(Clone, Copy, Debug)]
pub struct ChromiumFrameWorldAccessProfile {
    scope: ResourceScope,
    capacity: NonZeroUsize,
    pub(crate) native_retirement: bool,
    state_bytes: u64,
    requirements: ResourceRequirements,
}
impl ChromiumFrameWorldAccessProfile {
    pub fn new(scope: ResourceScope, capacity: NonZeroUsize) -> Result<Self> {
        let count = u64::try_from(capacity.get())
            .map_err(|_| FrameWorldAccessError::Setup("capacity overflow"))?;
        let bytes = count
            .checked_mul(SNAPSHOT_BYTES)
            .and_then(|bytes| bytes.checked_add(STATE_BYTES))
            .ok_or(FrameWorldAccessError::Setup("resource ceiling overflow"))?;
        let leases = capacity
            .get()
            .checked_add(1)
            .ok_or(FrameWorldAccessError::Setup("lease ceiling overflow"))?;
        Ok(Self {
            scope,
            capacity,
            native_retirement: false,
            state_bytes: STATE_BYTES,
            requirements: ResourceRequirements::none()
                .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
                .with_metadata(1, leases),
        })
    }
    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
    /// Install bounded, monotonic native destruction receipts. Not selected by
    /// default; route invalidation alone can never satisfy these receipts.
    pub fn with_native_retirement(mut self) -> Result<Self> {
        let extra = crate::native_world_retirement::Registry::storage_bytes(self.capacity.get())
            .ok_or(FrameWorldAccessError::Setup("retirement capacity overflow"))?;
        self.state_bytes = STATE_BYTES
            .checked_add(extra)
            .ok_or(FrameWorldAccessError::Setup("retirement state overflow"))?;
        let bytes = (self.capacity.get() as u64)
            .checked_mul(SNAPSHOT_BYTES)
            .and_then(|bytes| bytes.checked_add(self.state_bytes))
            .ok_or(FrameWorldAccessError::Setup("retirement resource overflow"))?;
        self.requirements =
            self.requirements
                .with_class(ResourceClass::BackendStateBytes, bytes, bytes);
        self.native_retirement = true;
        Ok(self)
    }
}

pub(crate) struct State {
    pub(crate) services: CdpPageServices,
    pub(crate) profile: ChromiumFrameWorldAccessProfile,
    ledger: ResourceLedger,
    active: AtomicUsize,
    _lease: Arc<ResourceLease>,
}
pub(crate) struct Permit {
    pub(crate) state: Arc<State>,
    _lease: ResourceLease,
}
impl Drop for Permit {
    fn drop(&mut self) {
        self.state.active.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Independently installed access port over one existing composition.
#[derive(Clone)]
pub struct ChromiumFrameWorldAccess(pub(crate) Arc<State>);
impl ChromiumFrameWorldAccess {
    /// Narrow Page input binding from this accessor's own authoritative service
    /// table. The caller budgets any escaped native client/state references.
    pub fn page_input_binding(&self, page: ObjectId) -> Result<Option<crate::CdpPageInputBinding>> {
        self.0.services.page_input_binding(page).map_err(Into::into)
    }
    /// Terminal opt-in teardown of sessions retained by this accessor's native
    /// receipts. The caller MUST first stop topology processing and all command
    /// admissions and supply the browser client of this exact connection.
    /// This seals new receipts permanently; borrowed Pages/Contexts are not
    /// closed. Failure is terminal and must not be reported as clean shutdown.
    pub async fn retire_owned_native_sessions(&self, browser: &dyn CdpClient) -> Result<()> {
        let (registry, epoch) = {
            let routes = self
                .0
                .services
                .routes
                .lock()
                .map_err(|_| CdpPageRouteError::StatePoisoned)?;
            (
                routes
                    .native_world_retirement
                    .clone()
                    .ok_or(FrameWorldAccessError::Setup(
                        "native retirement not selected",
                    ))?,
                routes.epoch.ok_or(FrameWorldAccessError::Setup(
                    "native topology epoch missing",
                ))?,
            )
        };
        registry.retire_sessions(epoch, browser).await
    }

    pub fn new(
        services: &CdpPageServices,
        profile: ChromiumFrameWorldAccessProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self> {
        if services.world_readiness.is_none()
            || services.evaluation.is_none()
            || !services.locator_uses_utility_world
        {
            return Err(FrameWorldAccessError::Setup(
                "requires explicit world readiness, value evaluation and utility-world Locator routes",
            ));
        }
        let ledger = resources.claim(profile.requirements())?;
        let lease = Arc::new(ledger.reserve(
            profile.scope,
            ResourceClass::BackendStateBytes,
            profile.state_bytes,
        )?);
        if profile.native_retirement {
            let mut routes = services
                .routes
                .lock()
                .map_err(|_| CdpPageRouteError::StatePoisoned)?;
            if routes.native_world_retirement.is_some() {
                return Err(FrameWorldAccessError::Setup(
                    "native retirement already installed",
                ));
            }
            routes.native_world_retirement =
                Some(Arc::new(crate::native_world_retirement::Registry::new(
                    profile.capacity.get(),
                    lease.clone(),
                )));
        }
        Ok(Self(Arc::new(State {
            services: services.clone(),
            profile,
            ledger,
            active: AtomicUsize::new(0),
            _lease: lease,
        })))
    }

    pub(crate) fn admit(&self) -> Result<Permit> {
        self.0
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < self.0.profile.capacity.get()).then(|| count + 1)
            })
            .map_err(|_| FrameWorldAccessError::Capacity {
                limit: self.0.profile.capacity.get(),
            })?;
        match self.0.ledger.reserve(
            self.0.profile.scope,
            ResourceClass::BackendStateBytes,
            SNAPSHOT_BYTES,
        ) {
            Ok(lease) => Ok(Permit {
                state: self.0.clone(),
                _lease: lease,
            }),
            Err(error) => {
                self.0.active.fetch_sub(1, Ordering::AcqRel);
                Err(error.into())
            }
        }
    }

    /// Snapshot an already-observed world pair without waiting or browser I/O.
    /// Event observers call this only from the serialized topology cursor after
    /// matching the exact main execution-context route.
    pub fn snapshot_current(&self, frame: ObjectId) -> Result<Option<ChromiumFrameWorldSnapshot>> {
        let permit = Arc::new(self.admit()?);
        let Some(data) = permit.state.lookup(frame, Some(&permit))? else {
            return Ok(None);
        };
        validate_snapshot_data(&data)?;
        Ok(Some(ChromiumFrameWorldSnapshot {
            data,
            permit,
            retired: AtomicBool::new(false),
        }))
    }

    /// Admit synchronously, then await both observed worlds for this exact
    /// Frame. A Page ID is not substituted with its main Frame. The caller
    /// supplies its original timeout and lifecycle cancellation by dropping
    /// this future; no deadline or stop task is started here.
    pub fn snapshot(
        &self,
        frame: ObjectId,
    ) -> impl Future<Output = Result<ChromiumFrameWorldSnapshot>> + Send + 'static {
        let admitted = self.admit();
        async move {
            let permit = Arc::new(admitted?);
            let state = &permit.state;
            let readiness = state
                .services
                .world_readiness
                .as_ref()
                .expect("validated readiness");
            let data = readiness
                .resolve(|| state.lookup(frame, Some(&permit)))
                .await?;
            validate_snapshot_data(&data)?;
            Ok(ChromiumFrameWorldSnapshot {
                data,
                permit,
                retired: AtomicBool::new(false),
            })
        }
    }

    /// Admit one retained parent MAIN world without waiting for an unrelated
    /// utility world. Frame-owner lookup uses only this exact CDP session and
    /// execution context; ordinary paired-world consumers keep `snapshot`.
    pub fn snapshot_main(
        &self,
        frame: ObjectId,
    ) -> impl Future<Output = Result<ChromiumMainWorldSnapshot>> + Send + 'static {
        let admitted = self.admit();
        async move {
            let permit = Arc::new(admitted?);
            let state = &permit.state;
            let readiness = state
                .services
                .world_readiness
                .as_ref()
                .expect("validated readiness");
            let data = readiness
                .resolve(|| state.lookup_main(frame, Some(&permit)))
                .await?;
            if data.main_numeric <= 0 || data.main.is_empty() {
                return Err(FrameWorldAccessError::Setup(
                    "requires a valid main execution world",
                ));
            }
            Ok(ChromiumMainWorldSnapshot {
                data,
                permit,
                retired: AtomicBool::new(false),
            })
        }
    }
}

fn validate_snapshot_data(data: &SnapshotData) -> Result<()> {
    if data.main_numeric <= 0
        || data.utility_numeric <= 0
        || data.main_numeric == data.utility_numeric
        || data.main.is_empty()
        || data.utility.is_empty()
        || data.main == data.utility
    {
        return Err(FrameWorldAccessError::Setup(
            "requires distinct valid main and utility execution worlds",
        ));
    }
    Ok(())
}

struct SnapshotData {
    epoch: pwright_backend_cdp::ConnectionEpoch,
    frame: ObjectId,
    page: ObjectId,
    client: Arc<dyn CdpClient>,
    utility: Arc<str>,
    utility_numeric: i64,
    main: Arc<str>,
    main_numeric: i64,
    lifetime: Option<crate::ChromiumNativeWorldLifetime>,
}
struct MainSnapshotData {
    epoch: pwright_backend_cdp::ConnectionEpoch,
    frame: ObjectId,
    page: ObjectId,
    client: Arc<dyn CdpClient>,
    main: Arc<str>,
    main_numeric: i64,
    lifetime: Option<crate::ChromiumNativeWorldLifetime>,
}
impl State {
    fn lookup_main(
        &self,
        frame: ObjectId,
        permit: Option<&Arc<Permit>>,
    ) -> std::result::Result<Option<MainSnapshotData>, CdpPageRouteError> {
        let routes = self
            .services
            .routes
            .lock()
            .map_err(|_| CdpPageRouteError::StatePoisoned)?;
        let index = routes
            .readiness_frames
            .binary_search_by_key(&frame, |(frame, _)| *frame)
            .map_err(|_| CdpPageRouteError::WorldTargetClosed { object_id: frame })?;
        let page_id = routes.readiness_frames[index].1;
        let page = routes
            .pages
            .get(&page_id)
            .ok_or(CdpPageRouteError::WorldTargetClosed { object_id: frame })?;
        if page.crashed {
            return Err(CdpPageRouteError::PageCrashed { object_id: frame });
        }
        let Some(main) = routes.frame_evaluations.get(&frame) else {
            return Ok(None);
        };
        if main.page_object_id != page_id {
            return Ok(None);
        }
        for (field, value) in [
            ("CDP target ID", main.target_id.as_str()),
            ("CDP session ID", main.session_id.as_str()),
            ("CDP main world unique ID", main.unique_context_id.as_ref()),
        ] {
            crate::validate_route_id(field, value, NATIVE_ID_BYTES)?;
        }
        let lifetime = if self.profile.native_retirement {
            crate::validate_route_id(
                "CDP owning Page target ID",
                &page.target_id,
                NATIVE_ID_BYTES,
            )?;
            permit
                .map(|permit| {
                    routes
                        .native_world_retirement
                        .as_ref()
                        .expect("installed native retirement")
                        .register(
                            crate::native_world_retirement::Identity {
                                epoch: routes.epoch.expect("published topology epoch"),
                                context: page.browser_context_object_id,
                                page_target: page.target_id.as_str().into(),
                                session: main.session_id.as_str().into(),
                                worlds: crate::native_world_retirement::IdentityWorlds::Main(
                                    main.unique_context_id.clone(),
                                ),
                            },
                            permit.clone(),
                        )
                })
                .transpose()?
        } else {
            None
        };
        Ok(Some(MainSnapshotData {
            epoch: routes.epoch.expect("published topology epoch"),
            frame,
            page: page_id,
            client: main.service.client(),
            main: main.unique_context_id.clone(),
            main_numeric: main.execution_context_id,
            lifetime,
        }))
    }

    fn lookup(
        &self,
        frame: ObjectId,
        permit: Option<&Arc<Permit>>,
    ) -> std::result::Result<Option<SnapshotData>, CdpPageRouteError> {
        let routes = self
            .services
            .routes
            .lock()
            .map_err(|_| CdpPageRouteError::StatePoisoned)?;
        let index = routes
            .readiness_frames
            .binary_search_by_key(&frame, |(frame, _)| *frame)
            .map_err(|_| CdpPageRouteError::WorldTargetClosed { object_id: frame })?;
        let page_id = routes.readiness_frames[index].1;
        let page = routes
            .pages
            .get(&page_id)
            .ok_or(CdpPageRouteError::WorldTargetClosed { object_id: frame })?;
        if page.crashed {
            return Err(CdpPageRouteError::PageCrashed { object_id: frame });
        }
        let (Some(main), Some(utility)) = (
            routes.frame_evaluations.get(&frame),
            routes.frame_contexts.get(&frame),
        ) else {
            return Ok(None);
        };
        // Worlds from different target-local sessions cannot form an adoption
        // pair, including intermediate OOPIF replacement publications.
        if main.page_object_id != page_id
            || main.target_id != utility.target_id
            || main.session_id != utility.session_id
        {
            return Ok(None);
        }
        for (field, value) in [
            ("CDP target ID", main.target_id.as_str()),
            ("CDP session ID", main.session_id.as_str()),
            ("CDP main world unique ID", main.unique_context_id.as_ref()),
            (
                "CDP utility world unique ID",
                utility.unique_context_id.as_ref(),
            ),
        ] {
            crate::validate_route_id(field, value, NATIVE_ID_BYTES)?;
        }
        let lifetime = if self.profile.native_retirement {
            crate::validate_route_id(
                "CDP owning Page target ID",
                &page.target_id,
                NATIVE_ID_BYTES,
            )?;
            permit
                .map(|permit| {
                    routes
                        .native_world_retirement
                        .as_ref()
                        .expect("installed native retirement")
                        .register(
                            crate::native_world_retirement::Identity {
                                epoch: routes.epoch.expect("published topology epoch"),
                                context: page.browser_context_object_id,
                                page_target: page.target_id.as_str().into(),
                                session: main.session_id.as_str().into(),
                                worlds: crate::native_world_retirement::IdentityWorlds::Paired {
                                    main: main.unique_context_id.clone(),
                                    utility: utility.unique_context_id.clone(),
                                },
                            },
                            permit.clone(),
                        )
                })
                .transpose()?
        } else {
            None
        };
        Ok(Some(SnapshotData {
            epoch: routes.epoch.expect("published topology epoch"),
            frame,
            page: page_id,
            client: main.service.client(),
            utility: utility.unique_context_id.clone(),
            utility_numeric: utility.execution_context_id,
            main: main.unique_context_id.clone(),
            main_numeric: main.execution_context_id,
            lifetime,
        }))
    }
}

/// Budgeted owner for one exact parent main world. Its optional native receipt
/// proves destruction of that world alone; route invalidation is never proof.
pub struct ChromiumMainWorldSnapshot {
    data: MainSnapshotData,
    permit: Arc<Permit>,
    retired: AtomicBool,
}
// Main-only snapshots share the preexisting paired snapshot admission. The
// retained route and receipt never exceed its declared storage envelope.
const _: () = assert!(
    std::mem::size_of::<ChromiumMainWorldSnapshot>() + 4 * NATIVE_ID_BYTES + 1024
        < SNAPSHOT_BYTES as usize
);
impl ChromiumMainWorldSnapshot {
    pub const fn page(&self) -> ObjectId {
        self.data.page
    }
    pub fn client(&self) -> Arc<dyn CdpClient> {
        self.data.client.clone()
    }
    pub const fn main_execution_context_id(&self) -> i64 {
        self.data.main_numeric
    }
    pub fn main_unique_id(&self) -> &str {
        &self.data.main
    }
    pub fn native_lifetime(&self) -> Option<crate::ChromiumNativeWorldLifetime> {
        self.data.lifetime.clone()
    }
    pub fn is_current(&self) -> Result<bool> {
        let current = self
            .permit
            .state
            .lookup_main(self.data.frame, None)?
            .is_some_and(|current| {
                current.epoch == self.data.epoch
                    && current.page == self.data.page
                    && current.main_numeric == self.data.main_numeric
                    && current.main == self.data.main
                    && Arc::ptr_eq(&current.client, &self.data.client)
            });
        if !current {
            self.retired.store(true, Ordering::Release);
        }
        Ok(current && !self.retired.load(Ordering::Acquire))
    }
}

/// One non-cloneable retained pair of OBSERVED worlds. This is not an atomic
/// document lease: a browser can navigate before its event reaches the driver.
/// Retain this budget owner while using any cloned native routing values and
/// revalidate before/after adoption; never retry already-dispatched user code.
pub struct ChromiumFrameWorldSnapshot {
    data: SnapshotData,
    permit: Arc<Permit>,
    retired: AtomicBool,
}
impl ChromiumFrameWorldSnapshot {
    pub const fn frame(&self) -> ObjectId {
        self.data.frame
    }
    pub const fn page(&self) -> ObjectId {
        self.data.page
    }
    /// Use this SAME client for utility selection and main-world adoption.
    pub fn client(&self) -> Arc<dyn CdpClient> {
        self.data.client.clone()
    }
    pub fn utility_unique_id(&self) -> &str {
        &self.data.utility
    }
    /// Numeric and unique identity are captured under the same route-table
    /// lock. The numeric ID alone is not proof against browser-side reuse.
    pub const fn utility_execution_context_id(&self) -> i64 {
        self.data.utility_numeric
    }
    pub fn main_unique_id(&self) -> &str {
        &self.data.main
    }
    pub const fn main_execution_context_id(&self) -> i64 {
        self.data.main_numeric
    }
    pub fn native_lifetime(&self) -> Option<crate::ChromiumNativeWorldLifetime> {
        self.data.lifetime.clone()
    }

    /// False is authoritative observed replacement/removal, never an inference
    /// from a failed CDP call. Frame/Page retirement and crash remain errors.
    /// Once observed retired, this snapshot never becomes current again.
    pub fn is_current(&self) -> Result<bool> {
        self.current().map_err(Into::into)
    }
    fn current(&self) -> std::result::Result<bool, CdpPageRouteError> {
        let current = self
            .permit
            .state
            .lookup(self.data.frame, None)?
            .is_some_and(|current| {
                current.epoch == self.data.epoch
                    && current.page == self.data.page
                    && current.main_numeric == self.data.main_numeric
                    && current.main == self.data.main
                    && current.utility_numeric == self.data.utility_numeric
                    && current.utility == self.data.utility
                    && Arc::ptr_eq(&current.client, &self.data.client)
            });
        if !current {
            self.retired.store(true, Ordering::Release);
        }
        Ok(current && !self.retired.load(Ordering::Acquire))
    }

    /// Caller-polled invalidation, using the existing bounded readiness waiters
    /// and commit notification. Mutable borrow prevents duplicate waits on one
    /// snapshot. Dropping the wait preserves the snapshot and releases its
    /// separate readiness slot; the host still supplies transport shutdown.
    pub async fn wait_invalidated(&mut self) -> Result<()> {
        self.permit
            .state
            .services
            .world_readiness
            .as_ref()
            .expect("validated readiness")
            .resolve(|| self.current().map(|current| (!current).then_some(())))
            .await
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declared_allowances_cover_owner_types_and_reject_overflow() {
        // Arrange
        let count = NonZeroUsize::new(3).unwrap();
        // Act
        let profile =
            ChromiumFrameWorldAccessProfile::new(ResourceScope::extension(98, 3), count).unwrap();
        let overflow = ChromiumFrameWorldAccessProfile::new(
            ResourceScope::extension(98, 3),
            NonZeroUsize::new(usize::MAX).unwrap(),
        );
        // Assert
        assert_eq!(
            profile
                .requirements()
                .class(ResourceClass::BackendStateBytes)
                .global,
            STATE_BYTES + 3 * SNAPSHOT_BYTES
        );
        assert_eq!(profile.requirements().leases, 4);
        assert!(std::mem::size_of::<State>() + 64 < STATE_BYTES as usize);
        assert!(
            std::mem::size_of::<ChromiumFrameWorldSnapshot>() + 6 * NATIVE_ID_BYTES + 1024
                < SNAPSHOT_BYTES as usize
        );
        assert!(matches!(overflow, Err(FrameWorldAccessError::Setup(_))));
    }
}
