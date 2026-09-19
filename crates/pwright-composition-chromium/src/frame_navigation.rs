//! Opt-in, transaction-published Frame navigation routes without execution worlds.
use std::{
    fmt,
    mem::size_of,
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use crate::CdpPageServices;
use pwright_backend_cdp::{CdpNavigationService, CdpTopology, ConnectionEpoch};
use pwright_cap_navigation::{
    BeginNavigationParams, BeginNavigationResult, NavigationErrorDetails,
};
use pwright_core::{
    FailureKind, ObjectId, OperationError, ProductionResourceLedger, ResourceClass, ResourceLease,
    ResourceLedger, ResourceRequirements, ResourceScope,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrameNavigationAccessError {
    Setup(&'static str),
    Resources,
    Allocation,
    Capacity,
    NativeIdentity,
    StatePoisoned,
    Unavailable,
}
impl fmt::Display for FrameNavigationAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Frame navigation access: {self:?}")
    }
}
impl std::error::Error for FrameNavigationAccessError {}
impl FrameNavigationAccessError {
    pub(crate) fn kind(&self) -> FailureKind {
        match self {
            Self::Setup(_) | Self::NativeIdentity => FailureKind::ProtocolViolation,
            Self::Resources | Self::Allocation | Self::Capacity => FailureKind::ResourceExhausted,
            Self::StatePoisoned => FailureKind::Cleanup,
            Self::Unavailable => FailureKind::TargetClosed,
        }
    }
}
type Result<T> = std::result::Result<T, FrameNavigationAccessError>;

/// Explicit route-table and escaped-snapshot storage, separate from navigation
/// operation/event budgets. Merge requirements into the attachment before use.
#[derive(Clone, Copy, Debug)]
pub struct ChromiumFrameNavigationProfile {
    scope: ResourceScope,
    frames: usize,
    snapshots: usize,
    native_id_bytes: usize,
    owner_bytes: u64,
    snapshot_bytes: u64,
    requirements: ResourceRequirements,
}
impl ChromiumFrameNavigationProfile {
    pub fn new(
        scope: ResourceScope,
        frames: NonZeroUsize,
        snapshots: NonZeroUsize,
        native_id_bytes: NonZeroUsize,
    ) -> Result<Self> {
        let overflow = || FrameNavigationAccessError::Setup("storage ceiling overflow");
        // Three route IDs plus the two IDs transferred to the native client.
        // Includes route/service/Arc bodies and a conservative fixed allowance;
        // arbitrary injected SessionFactory internals and transport are external.
        let entry = native_id_bytes
            .get()
            .checked_mul(5)
            .and_then(|n| {
                n.checked_add(size_of::<Entry>() + size_of::<CdpNavigationService>() + 256)
            })
            .ok_or_else(overflow)?;
        let slot = entry
            .checked_add(size_of::<Arc<Entry>>())
            .ok_or_else(overflow)?;
        let owner_bytes = frames
            .get()
            .checked_mul(slot)
            .and_then(|n| n.checked_mul(2)) // current and prepared tables
            .and_then(|n| {
                n.checked_add(
                    size_of::<Owner>()
                        + size_of::<CdpPageServices>()
                        + 2 * size_of::<Table>()
                        + 192,
                )
            })
            .and_then(|n| u64::try_from(n).ok())
            .ok_or_else(overflow)?;
        let snapshot_bytes = entry
            .checked_add(size_of::<ChromiumFrameNavigationSnapshot>() + 64)
            .and_then(|n| u64::try_from(n).ok())
            .ok_or_else(overflow)?;
        let total = u64::try_from(snapshots.get())
            .ok()
            .and_then(|n| n.checked_mul(snapshot_bytes))
            .and_then(|n| n.checked_add(owner_bytes))
            .ok_or_else(overflow)?;
        let leases = snapshots.get().checked_add(1).ok_or_else(overflow)?;
        Ok(Self {
            scope,
            frames: frames.get(),
            snapshots: snapshots.get(),
            native_id_bytes: native_id_bytes.get(),
            owner_bytes,
            snapshot_bytes,
            requirements: ResourceRequirements::none()
                .with_class(ResourceClass::BackendStateBytes, total, total)
                .with_metadata(1, leases),
        })
    }
    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}

struct Owner {
    profile: ChromiumFrameNavigationProfile,
    ledger: ResourceLedger,
    active: AtomicUsize,
    sealed: AtomicBool,
    _lease: ResourceLease,
}
struct Entry {
    epoch: ConnectionEpoch,
    frame: ObjectId,
    page: ObjectId,
    frame_id: String,
    target_id: String,
    session_id: String,
    service: CdpNavigationService,
    client: Arc<dyn pwright_cdp::CdpClient>,
    current: AtomicBool,
}
pub(crate) struct Table {
    entries: Vec<Arc<Entry>>,
    published: bool,
    // Drop route allocations before the final allowance owner.
    owner: Arc<Owner>,
}
impl Table {
    pub(crate) fn prepare(
        &self,
        services: &CdpPageServices,
        topology: &CdpTopology,
        pages: &crate::CdpServiceRoutes,
    ) -> Result<Self> {
        let factory = services
            .navigation
            .as_ref()
            .ok_or(FrameNavigationAccessError::Unavailable)?;
        let mut entries: Vec<Arc<Entry>> = Vec::new();
        entries
            .try_reserve_exact(self.owner.profile.frames)
            .map_err(|_| FrameNavigationAccessError::Allocation)?;
        if entries.capacity() != self.owner.profile.frames {
            return Err(FrameNavigationAccessError::Allocation);
        }
        if self.owner.sealed.load(Ordering::Acquire) {
            return Ok(Self {
                owner: self.owner.clone(),
                entries,
                published: false,
            });
        }
        for (frame, _) in topology.frame_page_routes() {
            let Some(route) = topology.frame_navigation_route(frame) else {
                continue;
            };
            if entries.len() == self.owner.profile.frames {
                return Err(FrameNavigationAccessError::Capacity);
            }
            for id in [route.frame_id(), route.target_id(), route.session_id()] {
                if id.is_empty() || id.len() > self.owner.profile.native_id_bytes {
                    return Err(FrameNavigationAccessError::NativeIdentity);
                }
            }
            let previous = self.entries.iter().find(|entry| {
                entry.epoch == route.epoch()
                    && entry.frame == route.object_id()
                    && entry.page == route.page_object_id()
                    && entry.frame_id == route.frame_id()
                    && entry.target_id == route.target_id()
                    && entry.session_id == route.session_id()
                    && entry.current.load(Ordering::Acquire)
            });
            if let Some(previous) = previous {
                entries.push(previous.clone());
            } else {
                let frame_id = copy_id(route.frame_id())?;
                let target_id = copy_id(route.target_id())?;
                let session_id = copy_id(route.session_id())?;
                let shared = pages
                    .pages
                    .values()
                    .find(|page| page.target_id == target_id && page.session_id == session_id)
                    .map(|page| page.client.clone())
                    .or_else(|| {
                        entries
                            .iter()
                            .chain(self.entries.iter())
                            .find(|entry| {
                                entry.epoch == route.epoch()
                                    && entry.target_id == target_id
                                    && entry.session_id == session_id
                                    && entry.current.load(Ordering::Acquire)
                            })
                            .map(|entry| entry.client.clone())
                    });
                let client = match shared {
                    Some(client) => client,
                    None => services
                        .session_factory
                        .create_session(copy_id(route.session_id())?, copy_id(route.target_id())?),
                };
                entries.push(Arc::new(Entry {
                    epoch: route.epoch(),
                    frame: route.object_id(),
                    page: route.page_object_id(),
                    frame_id,
                    target_id,
                    session_id,
                    service: factory.bind(client.clone()),
                    client,
                    current: AtomicBool::new(true),
                }));
            }
        }
        Ok(Self {
            owner: self.owner.clone(),
            entries,
            published: false,
        })
    }
    /// Infallible commit hook under the existing route-table lock. No wakers,
    /// allocations or native calls. A discarded candidate never revokes routes.
    pub(crate) fn replace(&mut self, next: &mut Self) {
        for old in &self.entries {
            if !next.entries.iter().any(|new| Arc::ptr_eq(old, new)) {
                old.current.store(false, Ordering::Release);
            }
        }
        self.published = false;
        next.published = true;
    }
}
impl Drop for Table {
    fn drop(&mut self) {
        if self.published {
            for entry in &self.entries {
                entry.current.store(false, Ordering::Release);
            }
        }
    }
}
#[cfg(feature = "close-route-retirement")]
impl Table {
    pub(crate) fn matches_retirement(&self, topology: &CdpTopology, disposed: &[ObjectId]) -> bool {
        if self.owner.sealed.load(Ordering::Acquire) {
            return self.entries.is_empty();
        }
        let mut count = 0;
        for (frame, _) in topology.frame_page_routes() {
            let Some(route) = topology.frame_navigation_route(frame) else {
                continue;
            };
            if disposed.contains(&frame)
                || !self.entries.iter().any(|entry| {
                    entry.frame == frame
                        && entry.epoch == route.epoch()
                        && entry.page == route.page_object_id()
                        && entry.frame_id == route.frame_id()
                        && entry.target_id == route.target_id()
                        && entry.session_id == route.session_id()
                        && entry.current.load(Ordering::Acquire)
                })
            {
                return false;
            }
            count += 1;
        }
        count
            == self
                .entries
                .iter()
                .filter(|entry| !disposed.contains(&entry.frame))
                .count()
    }
    pub(crate) fn retire(&mut self, disposed: &[ObjectId]) {
        self.entries.retain(|entry| {
            if disposed.contains(&entry.frame) {
                entry.current.store(false, Ordering::Release);
                false
            } else {
                true
            }
        });
    }
}
fn copy_id(id: &str) -> Result<String> {
    let mut owned = String::new();
    owned
        .try_reserve_exact(id.len())
        .map_err(|_| FrameNavigationAccessError::Allocation)?;
    if owned.capacity() != id.len() {
        return Err(FrameNavigationAccessError::Allocation);
    }
    owned.push_str(id);
    Ok(owned)
}

/// Installed once per existing service table. Does not start a driver, select
/// evaluation, subscribe to events or perform native I/O. The next successful
/// topology synchronization publishes routes; captures before it are unavailable.
#[derive(Clone)]
pub struct ChromiumFrameNavigationAccess {
    services: Arc<CdpPageServices>,
}
impl ChromiumFrameNavigationAccess {
    pub fn new(
        services: &CdpPageServices,
        profile: ChromiumFrameNavigationProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self> {
        if services.navigation.is_none() {
            return Err(FrameNavigationAccessError::Setup(
                "navigation capability not selected",
            ));
        }
        let mut routes = services
            .routes
            .lock()
            .map_err(|_| FrameNavigationAccessError::StatePoisoned)?;
        if routes.frame_navigation.is_some() {
            return Err(FrameNavigationAccessError::Setup(
                "navigation access already installed",
            ));
        }
        let ledger = resources
            .claim(profile.requirements())
            .map_err(|_| FrameNavigationAccessError::Resources)?;
        let lease = ledger
            .reserve(
                profile.scope,
                ResourceClass::BackendStateBytes,
                profile.owner_bytes,
            )
            .map_err(|_| FrameNavigationAccessError::Resources)?;
        routes.frame_navigation = Some(Table {
            owner: Arc::new(Owner {
                profile,
                ledger,
                active: AtomicUsize::new(0),
                sealed: AtomicBool::new(false),
                _lease: lease,
            }),
            entries: Vec::new(),
            published: true,
        });
        Ok(Self {
            services: Arc::new(services.clone()),
        })
    }
    /// Capture admission precedes retaining a route. Clones share the ceiling.
    /// The snapshot cannot export a client/service Arc independently of its lease.
    pub fn snapshot(&self, frame: ObjectId) -> Result<ChromiumFrameNavigationSnapshot> {
        let routes = self
            .services
            .routes
            .lock()
            .map_err(|_| FrameNavigationAccessError::StatePoisoned)?;
        let table = routes
            .frame_navigation
            .as_ref()
            .ok_or(FrameNavigationAccessError::Unavailable)?;
        if table.owner.sealed.load(Ordering::Acquire) {
            return Err(FrameNavigationAccessError::Unavailable);
        }
        let entry = table
            .entries
            .iter()
            .find(|entry| entry.frame == frame)
            .ok_or(FrameNavigationAccessError::Unavailable)?;
        let owner = &table.owner;
        owner
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < owner.profile.snapshots).then(|| n + 1)
            })
            .map_err(|_| FrameNavigationAccessError::Capacity)?;
        let lease = match owner.ledger.reserve(
            owner.profile.scope,
            ResourceClass::BackendStateBytes,
            owner.profile.snapshot_bytes,
        ) {
            Ok(lease) => lease,
            Err(_) => {
                owner.active.fetch_sub(1, Ordering::AcqRel);
                return Err(FrameNavigationAccessError::Resources);
            }
        };
        Ok(ChromiumFrameNavigationSnapshot {
            entry: entry.clone(),
            owner: owner.clone(),
            _lease: lease,
        })
    }

    /// Terminal loss/shutdown fence. The embedding event owner calls this on
    /// disconnect, event loss or teardown before admitting further operations.
    /// Later topology publications cannot reopen this installation.
    pub fn seal(&self) -> Result<()> {
        let routes = self
            .services
            .routes
            .lock()
            .map_err(|_| FrameNavigationAccessError::StatePoisoned)?;
        let table = routes
            .frame_navigation
            .as_ref()
            .ok_or(FrameNavigationAccessError::Unavailable)?;
        table.owner.sealed.store(true, Ordering::Release);
        for entry in &table.entries {
            entry.current.store(false, Ordering::Release);
        }
        Ok(())
    }
}

/// Non-cloneable observed native route. A committed detach/swap/crash/reconnect
/// retires it permanently, even when a later route reuses the same native IDs.
/// This is not a browser-atomic lifetime or a native cancellation receipt.
pub struct ChromiumFrameNavigationSnapshot {
    entry: Arc<Entry>,
    owner: Arc<Owner>,
    _lease: ResourceLease,
}
impl ChromiumFrameNavigationSnapshot {
    pub fn frame(&self) -> ObjectId {
        self.entry.frame
    }
    pub fn page(&self) -> ObjectId {
        self.entry.page
    }
    pub fn epoch(&self) -> ConnectionEpoch {
        self.entry.epoch
    }
    pub fn frame_id(&self) -> &str {
        &self.entry.frame_id
    }
    pub fn session_id(&self) -> &str {
        &self.entry.session_id
    }
    pub fn target_id(&self) -> &str {
        &self.entry.target_id
    }
    pub fn is_current(&self) -> bool {
        self.entry.current.load(Ordering::Acquire)
    }

    /// Initiation only: check observed revocation before first-poll dispatch and
    /// after the native reply. No retry, fallback or Page.stopLoading. The full
    /// operation must additionally drive lifecycle, timeout and close handling.
    pub async fn begin_navigation(
        &self,
        params: BeginNavigationParams,
    ) -> std::result::Result<BeginNavigationResult, OperationError<NavigationErrorDetails>> {
        let receipt = self.dispatch_navigation(params).await;
        self.check_current()?;
        receipt
    }

    /// Dispatch once on the retained route and return its native receipt.
    ///
    /// A navigation can itself replace the renderer route while the portable
    /// Frame survives. An ordered navigation executor must consume that reply
    /// and decide completion from Frame lifetime and document events, not from
    /// the old route's post-dispatch currency. This primitive still rejects a
    /// route retired before first poll. It does not retry or report load success.
    pub async fn dispatch_navigation(
        &self,
        params: BeginNavigationParams,
    ) -> std::result::Result<BeginNavigationResult, OperationError<NavigationErrorDetails>> {
        self.check_current()?;
        self.entry
            .service
            .begin_frame_navigation(&self.entry.frame_id, params)
            .await
    }
    fn check_current(&self) -> std::result::Result<(), OperationError<NavigationErrorDetails>> {
        if self.is_current() {
            Ok(())
        } else {
            Err(OperationError::new(
                FailureKind::TargetClosed,
                "Frame navigation route was retired",
                None,
            ))
        }
    }
}
impl Drop for ChromiumFrameNavigationSnapshot {
    fn drop(&mut self) {
        self.owner.active.fetch_sub(1, Ordering::AcqRel);
    }
}
