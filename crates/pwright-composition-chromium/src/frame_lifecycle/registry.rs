//! Bounded composition of Page owners on one ordered driver; no new cursor.
use super::*;
use pwright_backend_cdp_navigation::frame_lifecycle::FrameLifecycleProfile;

#[derive(Clone, Copy, Debug)]
pub struct FrameLifecycleRegistryProfile {
    namespace: u32,
    pages: usize,
    owners: usize,
    frames: usize,
    requests: usize,
    id_bytes: usize,
    navigation_slots: usize,
    index_bytes: u64,
    requirements: ResourceRequirements,
}
impl FrameLifecycleRegistryProfile {
    /// Reserve this namespace exclusively for the registry. `owners` includes
    /// active Pages and retired owners retained by escaped handles.
    pub fn new(
        namespace: u32,
        pages: usize,
        owners: usize,
        frames: usize,
        requests: usize,
        id_bytes: usize,
    ) -> Result<Self> {
        Self::new_with_slots(namespace, pages, owners, frames, requests, id_bytes, 0)
    }
    pub fn with_navigation_slots(self, slots: usize) -> Result<Self> {
        if slots == 0 {
            return Err("navigation registration capacity must be positive".into());
        }
        Self::new_with_slots(
            self.namespace,
            self.pages,
            self.owners,
            self.frames,
            self.requests,
            self.id_bytes,
            slots,
        )
    }
    fn new_with_slots(
        namespace: u32,
        pages: usize,
        owners: usize,
        frames: usize,
        requests: usize,
        id_bytes: usize,
        navigation_slots: usize,
    ) -> Result<Self> {
        if pages == 0 || owners < pages {
            return Err("invalid lifecycle registry capacity".into());
        }
        let index_bytes = owners
            .checked_mul(size_of::<Option<Entry>>())
            .and_then(|n| {
                n.checked_add(
                    size_of::<FrameLifecycleRegistry>()
                        + size_of::<Terminal>()
                        + 2 * size_of::<usize>()
                        + 256,
                )
            })
            .and_then(|n| u64::try_from(n).ok())
            .ok_or("lifecycle registry size overflow")?;
        let mut profile = Self {
            namespace,
            pages,
            owners,
            frames,
            requests,
            id_bytes,
            navigation_slots,
            index_bytes,
            requirements: ResourceRequirements::none(),
        };
        // Validate the largest slot's scope arithmetic before declaring storage.
        profile.page_profile(owners - 1)?;
        let one = profile.page_profile(0)?.requirements();
        let count = u64::try_from(owners)?;
        let mut all = ResourceRequirements::none().with_metadata(
            one.scopes
                .checked_mul(owners)
                .ok_or("lifecycle scope count overflow")?,
            one.leases
                .checked_mul(owners)
                .ok_or("lifecycle lease count overflow")?,
        );
        for class in ResourceClass::ALL {
            let limit = one.class(class);
            all = all.with_class(
                class,
                limit
                    .global
                    .checked_mul(count)
                    .ok_or("lifecycle byte count overflow")?,
                limit
                    .per_scope
                    .checked_mul(count)
                    .ok_or("lifecycle scope bytes overflow")?,
            );
        }
        profile.requirements = all.combine_concurrent(profile.index_requirements());
        Ok(profile)
    }
    fn page_profile(self, slot: usize) -> Result<PageLifecycleObservationProfile> {
        let base = u64::try_from(slot)?
            .checked_mul(if self.navigation_slots == 0 { 3 } else { 4 })
            .and_then(|n| n.checked_add(1))
            .ok_or("lifecycle scope overflow")?;
        let scope = |offset| {
            base.checked_add(offset)
                .map(|id| ResourceScope::extension(self.namespace, id))
                .ok_or("lifecycle scope overflow")
        };
        let profile = PageLifecycleObservationProfile::new(
            CdpFrameLifecycleProfile::new(
                FrameLifecycleProfile::new(scope(0)?, self.frames, self.requests)?,
                scope(1)?,
                self.id_bytes,
            )?,
            scope(2)?,
        );
        if self.navigation_slots == 0 {
            Ok(profile)
        } else {
            profile.with_navigation_slots(scope(3)?, self.navigation_slots)
        }
    }
    fn index_requirements(self) -> ResourceRequirements {
        ResourceRequirements::none()
            .with_class(
                ResourceClass::BackendStateBytes,
                self.index_bytes,
                self.index_bytes,
            )
            .with_metadata(1, 1)
    }
    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}

struct Entry {
    page: ObjectId,
    owner: Arc<PageLifecycleObservation>,
}
struct Terminal {
    sealed: AtomicBool,
    closed: tokio::sync::Notify,
    _lease: ResourceLease,
}
impl Terminal {
    fn seal(&self) {
        self.sealed.store(true, Ordering::Release);
        self.closed.notify_waiters();
    }
}

/// An escaped Page owner does not retain the registry, other Pages or transport.
/// No underlying Arc/Weak is exposed. Retired slots are reclaimed at the next
/// registry checkpoint once no handle remains, or when the registry is dropped.
#[derive(Clone)]
pub struct PageLifecycleHandle {
    pub(super) owner: Arc<PageLifecycleObservation>,
    terminal: Arc<Terminal>,
}
impl PageLifecycleHandle {
    pub(super) fn check(&self) -> Result<()> {
        if self.is_sealed() {
            return Err("Page lifecycle handle is sealed".into());
        }
        Ok(())
    }
    pub fn snapshot(&self, frame: ObjectId) -> Result<Option<LifecycleSnapshot>> {
        self.check()?;
        let snapshot = self.owner.snapshot(frame)?;
        self.check()?;
        Ok(snapshot)
    }
    pub fn next_deadline(&self) -> Result<Option<Duration>> {
        self.check()?;
        let deadline = self.owner.next_deadline()?;
        self.check()?;
        Ok(deadline)
    }
    pub fn is_sealed(&self) -> bool {
        self.terminal.sealed.load(Ordering::Acquire) || self.owner.is_sealed()
    }
    /// Borrowed, cancellation-safe Page lifetime wait. It retains no registry,
    /// transport, native route or navigation-registration slot. Both Notify
    /// bodies are admitted by the existing owner profiles; the embedding caller
    /// admits the inline future/waiter storage. A renderer route swap is not a
    /// close signal. Register before checking flags so early closure cannot be lost.
    pub async fn closed(&self) {
        let page = self.owner.closed.notified();
        let terminal = self.terminal.closed.notified();
        tokio::pin!(page, terminal);
        page.as_mut().enable();
        terminal.as_mut().enable();
        if self.is_sealed() {
            return;
        }
        tokio::select! {
            () = page => {},
            () = terminal => {},
        }
    }
    /// Same injected monotonic origin used by Page observations and deadlines.
    pub fn now(&self) -> Result<Duration> {
        self.check()?;
        let now = self.owner.clock.now();
        self.check()?;
        Ok(now)
    }
}

pub struct FrameLifecycleRegistry {
    profile: FrameLifecycleRegistryProfile,
    epoch: u64,
    entries: Mutex<Vec<Option<Entry>>>,
    resources: ProductionResourceLedger,
    clock: Arc<dyn FrameLifecycleClock>,
    sink: Arc<dyn FrameLifecycleNoticeSink>,
    // Escaped handles retain this small terminal body, not the registry. The
    // index allowance conservatively follows it until the last handle drops.
    terminal: Arc<Terminal>,
}
impl FrameLifecycleRegistry {
    pub fn new(
        profile: FrameLifecycleRegistryProfile,
        epoch: u64,
        resources: &ProductionResourceLedger,
        clock: Arc<dyn FrameLifecycleClock>,
        sink: Arc<dyn FrameLifecycleNoticeSink>,
    ) -> Result<Self> {
        if epoch == 0 {
            return Err("invalid lifecycle registry epoch".into());
        }
        // Check the complete selected plan, then reserve the index before its
        // fixed allocation. Page bodies are admitted by prepare_page, not lazily
        // by a readiness query or after native domains have been enabled.
        resources.claim(profile.requirements())?;
        let lease = resources.claim(profile.index_requirements())?.reserve(
            ResourceScope::extension(profile.namespace, 0),
            ResourceClass::BackendStateBytes,
            profile.index_bytes,
        )?;
        let mut entries = Vec::new();
        entries.try_reserve_exact(profile.owners)?;
        if entries.capacity() != profile.owners {
            return Err("lifecycle index capacity mismatch".into());
        }
        entries.resize_with(profile.owners, || None);
        Ok(Self {
            profile,
            epoch,
            entries: Mutex::new(entries),
            resources: resources.clone(),
            clock,
            sink,
            terminal: Arc::new(Terminal {
                sealed: AtomicBool::new(false),
                closed: tokio::sync::Notify::new(),
                _lease: lease,
            }),
        })
    }
    fn with_entries<T>(&self, run: impl FnOnce(&mut [Option<Entry>]) -> Result<T>) -> Result<T> {
        if self.terminal.sealed.load(Ordering::Acquire) {
            return Err("lifecycle registry is sealed".into());
        }
        let mut entries = match self.entries.lock() {
            Ok(entries) => entries,
            Err(_) => {
                self.seal();
                return Err("lifecycle registry unavailable".into());
            }
        };
        if self.terminal.sealed.load(Ordering::Acquire) {
            return Err("lifecycle registry is sealed".into());
        }
        let result = run(&mut entries);
        if result.is_err() || self.terminal.sealed.load(Ordering::Acquire) {
            self.terminal.seal();
            for entry in entries.iter().flatten() {
                entry.owner.seal();
            }
            if result.is_ok() {
                return Err("lifecycle registry is sealed".into());
            }
        }
        result
    }
    fn reconcile_entries(
        &self,
        entries: &mut [Option<Entry>],
        topology: &CdpTopology,
    ) -> Result<()> {
        if topology.epoch().get() != self.epoch {
            return Err("lifecycle registry epoch changed".into());
        }
        for slot in entries {
            if let Some(entry) = slot {
                entry.owner.reconcile(topology)?;
                if entry.owner.is_sealed() && Arc::strong_count(&entry.owner) == 1 {
                    *slot = None;
                }
            }
        }
        Ok(())
    }
    pub fn page(&self, page: ObjectId) -> Result<Option<PageLifecycleHandle>> {
        self.with_entries(|entries| {
            Ok(entries
                .iter()
                .flatten()
                .find(|entry| entry.page == page && !entry.owner.is_sealed())
                .map(|entry| PageLifecycleHandle {
                    owner: entry.owner.clone(),
                    terminal: self.terminal.clone(),
                }))
        })
    }
}
impl ChromiumFrameEventObserver for FrameLifecycleRegistry {
    fn observes_page(&self, _page: ObjectId) -> bool {
        !self.terminal.sealed.load(Ordering::Acquire)
    }
    fn prepare_page(&self, topology: &CdpTopology, page: ObjectId) -> FrameObservationResult {
        self.with_entries(|entries| {
            self.reconcile_entries(entries, topology)?;
            if entries
                .iter()
                .flatten()
                .any(|entry| entry.page == page && !entry.owner.is_sealed())
            {
                return Ok(());
            }
            if entries
                .iter()
                .flatten()
                .filter(|entry| !entry.owner.is_sealed())
                .count()
                == self.profile.pages
            {
                return Err("active lifecycle Page capacity exhausted".into());
            }
            let slot = entries
                .iter()
                .position(Option::is_none)
                .ok_or("retained lifecycle owner capacity exhausted")?;
            let owner = Arc::new(PageLifecycleObservation::new(
                self.profile.page_profile(slot)?,
                page,
                self.epoch,
                &self.resources,
                self.clock.clone(),
                self.sink.clone(),
            )?);
            entries[slot] = Some(Entry { page, owner });
            Ok(())
        })
    }
    fn discard_prepared_page(
        &self,
        topology: &CdpTopology,
        page: ObjectId,
    ) -> FrameObservationResult {
        self.with_entries(|entries| {
            if topology.main_frame_native_id(page).is_some() {
                return Ok(());
            }
            for entry in entries.iter().flatten().filter(|entry| entry.page == page) {
                entry.owner.seal();
            }
            self.reconcile_entries(entries, topology)
        })
    }
    fn initialize_frame(
        &self,
        topology: &CdpTopology,
        session: &str,
        frame: &page::Frame,
    ) -> FrameObservationResult {
        if self.terminal.sealed.load(Ordering::Acquire) {
            return Ok(());
        }
        self.with_entries(|entries| {
            self.reconcile_entries(entries, topology)?;
            let page = topology
                .frame_object_id(&frame.id)
                .and_then(|id| topology.page_object_id_for_frame(id));
            let Some(page) = page else {
                return Ok(());
            };
            let entry = entries
                .iter()
                .flatten()
                .find(|entry| entry.page == page && !entry.owner.is_sealed())
                .ok_or("Page lifecycle was not admitted before initialization")?;
            entry.owner.initialize_frame(topology, session, frame)
        })
    }
    fn reconcile(&self, topology: &CdpTopology) -> FrameObservationResult {
        if self.terminal.sealed.load(Ordering::Acquire) {
            return Ok(());
        }
        self.with_entries(|entries| self.reconcile_entries(entries, topology))
    }
    fn event(&self, topology: &CdpTopology, event: &CdpEvent) -> FrameObservationResult {
        if self.terminal.sealed.load(Ordering::Acquire) {
            return Ok(());
        }
        self.with_entries(|entries| {
            self.reconcile_entries(entries, topology)?;
            let page = event
                .session_id
                .as_deref()
                .and_then(|session| topology.target_session_route(session))
                .and_then(|route| topology.page_object_id(route.page_target_id()));
            if let Some(entry) = entries
                .iter()
                .flatten()
                .find(|entry| Some(entry.page) == page && !entry.owner.is_sealed())
            {
                entry.owner.event(topology, event)?;
            }
            Ok(())
        })
    }
    fn seal(&self) {
        self.terminal.seal();
        // The driver calls seal outside callbacks. A concurrent callback may
        // own this lock, so retained handles also share the terminal flag below.
        if let Ok(entries) = self.entries.try_lock() {
            for entry in entries.iter().flatten() {
                entry.owner.seal();
            }
        }
    }
}
impl Drop for FrameLifecycleRegistry {
    fn drop(&mut self) {
        self.seal();
    }
}
