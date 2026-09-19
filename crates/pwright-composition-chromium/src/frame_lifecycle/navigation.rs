//! Page-locked operation registration on the existing ordered observer path.
use super::*;
use pwright_backend_cdp_navigation::frame_lifecycle::navigation::{
    FrameNavigationWait, FrameNavigationWaitProfile, NavigationWaitError,
};
use pwright_cap_navigation::WaitUntil;
use tokio::sync::Notify;

/// The caller includes this plan in its operation budget before registration.
/// The Page profile independently admits the fixed registration index.
#[derive(Clone, Copy, Debug)]
pub struct NavigationRegistrationProfile {
    wait: FrameNavigationWaitProfile,
    scope: ResourceScope,
}
impl NavigationRegistrationProfile {
    pub const fn new(wait: FrameNavigationWaitProfile, scope: ResourceScope) -> Self {
        Self { wait, scope }
    }
    fn owner_requirements(self) -> ResourceRequirements {
        // Arc counters/alignment plus one consumer's notification future/waker
        // allowance. Foreign executor task stacks/queues remain caller-owned.
        let bytes = (size_of::<RegisteredNavigation>() + 2 * size_of::<usize>() + 256) as u64;
        ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, 1)
    }
    pub fn requirements(self) -> ResourceRequirements {
        self.wait
            .requirements()
            .combine_concurrent(self.owner_requirements())
    }
}

pub(super) struct RegisteredNavigation {
    frame: ObjectId,
    wait: Mutex<Option<FrameNavigationWait>>,
    changed: Notify,
    _lease: ResourceLease,
}
impl RegisteredNavigation {
    fn checkpoint(
        &self,
        now: Duration,
        native: &CdpFrameLifecycle,
        notice: Option<&NativeLifecycleNotice<'_>>,
    ) -> Result<()> {
        let mut slot = self
            .wait
            .lock()
            .map_err(|_| "navigation registration unavailable")?;
        let Some(wait) = slot.as_mut() else {
            return Ok(());
        };
        if wait.outcome().is_some() {
            return Ok(());
        }
        if let Some(notice) = notice {
            // Operation failures are retained terminal outcomes, not errors of
            // the shared Page event stream. Never propagate one into the driver.
            let _ = wait.observe(now, notice);
        }
        let _ = wait.poll(now, native.snapshot(self.frame)?);
        // Native changes can introduce an earlier idle deadline even while the
        // operation is pending. Wake the consumer to re-plan its timer. Plain
        // snapshot/deadline reads must not self-notify into a busy loop.
        if notice.is_some() || wait.outcome().is_some() {
            self.changed.notify_one();
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct NavigationSlotsProfile {
    scope: ResourceScope,
    slots: usize,
    bytes: u64,
}
impl NavigationSlotsProfile {
    pub(super) fn new(scope: ResourceScope, slots: usize) -> Result<Self> {
        if slots == 0 {
            return Err("navigation registration capacity must be positive".into());
        }
        let bytes = slots
            .checked_mul(size_of::<Option<Arc<RegisteredNavigation>>>())
            .and_then(|n| u64::try_from(n).ok())
            .ok_or("navigation registration index overflow")?;
        Ok(Self {
            scope,
            slots,
            bytes,
        })
    }
    pub(super) fn requirements(self) -> ResourceRequirements {
        ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, self.bytes, self.bytes)
            .with_metadata(1, 1)
    }
}
pub(super) struct NavigationSlots {
    entries: Vec<Option<Arc<RegisteredNavigation>>>,
    _lease: ResourceLease,
}
impl NavigationSlots {
    pub(super) fn new(
        profile: NavigationSlotsProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self> {
        let lease = resources.claim(profile.requirements())?.reserve(
            profile.scope,
            ResourceClass::BackendStateBytes,
            profile.bytes,
        )?;
        let mut entries = Vec::new();
        entries.try_reserve_exact(profile.slots)?;
        if entries.capacity() != profile.slots {
            return Err("navigation registration index capacity mismatch".into());
        }
        entries.resize_with(profile.slots, || None);
        Ok(Self {
            entries,
            _lease: lease,
        })
    }
    fn reclaim(&mut self) {
        for slot in &mut self.entries {
            // No weak handles: a discarded ticket is reclaimed at the next
            // Page checkpoint. A moved-out outcome keeps its own core lease.
            if slot
                .as_ref()
                .is_some_and(|entry| Arc::strong_count(entry) == 1)
            {
                *slot = None;
            }
        }
    }
    fn register(&mut self, params: Registration<'_>) -> Result<Arc<RegisteredNavigation>> {
        self.reclaim();
        let slot = self
            .entries
            .iter_mut()
            .find(|slot| slot.is_none())
            .ok_or("navigation registration capacity exhausted")?;
        let plan = params.profile.owner_requirements();
        let lease = params.resources.claim(plan)?.reserve(
            params.profile.scope,
            ResourceClass::BackendStateBytes,
            plan.class(ResourceClass::BackendStateBytes).global,
        )?;
        let wait = FrameNavigationWait::new(
            params.profile.wait,
            params.resources,
            params.frame,
            params.wait,
            params.deadline,
        )?;
        let entry = Arc::new(RegisteredNavigation {
            frame: params.frame,
            wait: Mutex::new(Some(wait)),
            changed: Notify::new(),
            _lease: lease,
        });
        *slot = Some(entry.clone());
        Ok(entry)
    }
    pub(super) fn checkpoint(
        &mut self,
        now: Duration,
        native: &CdpFrameLifecycle,
        notice: Option<&NativeLifecycleNotice<'_>>,
    ) -> Result<()> {
        self.reclaim();
        for entry in self.entries.iter().flatten() {
            entry.checkpoint(now, native, notice)?;
        }
        Ok(())
    }
    pub(super) fn next_deadline(&self) -> Result<Option<Duration>> {
        let mut deadline = None;
        for entry in self.entries.iter().flatten() {
            let wait = entry
                .wait
                .lock()
                .map_err(|_| "navigation registration unavailable")?;
            if let Some(next) = wait.as_ref().and_then(FrameNavigationWait::next_deadline) {
                deadline = Some(deadline.map_or(next, |old: Duration| old.min(next)));
            }
        }
        Ok(deadline)
    }
    pub(super) fn wake_all(&self) {
        for entry in self.entries.iter().flatten() {
            entry.changed.notify_one();
        }
    }
}
struct Registration<'a> {
    profile: NavigationRegistrationProfile,
    resources: &'a ProductionResourceLedger,
    frame: ObjectId,
    wait: WaitUntil,
    deadline: Option<Duration>,
}

/// One consumer, intentionally not Clone. It retains the selected Page owner,
/// not the host or browser transport. Drop then checkpoint reclaims its slot.
pub struct NavigationWaitHandle {
    page: PageLifecycleHandle,
    entry: Arc<RegisteredNavigation>,
    taken: bool,
}
impl PageLifecycleHandle {
    /// Register before obtaining/dispatching the native invocation. This checks
    /// observed Frame existence, not exact native route freshness at dispatch.
    pub fn register_navigation(
        &self,
        profile: NavigationRegistrationProfile,
        resources: &ProductionResourceLedger,
        frame: ObjectId,
        wait: WaitUntil,
        deadline: Option<Duration>,
    ) -> Result<NavigationWaitHandle> {
        self.check()?;
        let entry = self.owner.with_state(|state, _| {
            // Admission failures are local to this caller, not Page corruption.
            Ok((|| {
                if state.native.snapshot(frame)?.is_none() {
                    return Err("navigation Frame is not observed by this Page".into());
                }
                state
                    .navigation
                    .as_mut()
                    .ok_or_else(|| {
                        Box::<dyn Error + Send + Sync>::from(
                            "navigation registrations are not selected",
                        )
                    })?
                    .register(Registration {
                        profile,
                        resources,
                        frame,
                        wait,
                        deadline,
                    })
            })())
        })??;
        self.check()?;
        Ok(NavigationWaitHandle {
            page: self.clone(),
            entry,
            taken: false,
        })
    }
}
impl NavigationWaitHandle {
    /// Apply the receipt and current readiness atomically with Page events.
    /// A receipt error is retained for `try_take`; it cannot poison the Page.
    pub fn receipt(&mut self, loader: Option<&str>) -> Result<()> {
        self.require_owned()?;
        self.page.check()?;
        self.page.owner.with_state(|state, now| {
            let mut slot = self
                .entry
                .wait
                .lock()
                .map_err(|_| "navigation registration unavailable")?;
            let wait = slot.as_mut().ok_or("navigation result already taken")?;
            let _ = wait.receipt(now, loader);
            let _ = wait.poll(now, state.native.snapshot(self.entry.frame)?);
            if wait.outcome().is_some() {
                self.entry.changed.notify_one();
            }
            Ok(())
        })?;
        self.page.check()
    }
    /// Move, do not copy, settled storage out of the registration. The returned
    /// owner keeps its original response/error lease until the consumer drops it.
    pub fn try_take(&mut self) -> Result<Option<FrameNavigationWait>> {
        self.require_owned()?;
        self.page.check()?;
        let outcome = self.page.owner.with_state(|_, _| {
            let mut slot = self
                .entry
                .wait
                .lock()
                .map_err(|_| "navigation registration unavailable")?;
            Ok(
                if slot.as_ref().is_some_and(|wait| wait.outcome().is_some()) {
                    slot.take()
                } else {
                    None
                },
            )
        })?;
        self.page.check()?;
        self.taken = outcome.is_some();
        Ok(outcome)
    }
    pub fn abort(&mut self, reason: NavigationWaitError) -> Result<()> {
        self.require_owned()?;
        let mut slot = self
            .entry
            .wait
            .lock()
            .map_err(|_| "navigation registration unavailable")?;
        if let Some(wait) = slot.as_mut() {
            let _ = wait.abort(reason);
        }
        self.entry.changed.notify_one();
        Ok(())
    }
    /// One outstanding consumer wait. notify_one retains an early permit, so a
    /// completion between try_take and this await cannot be lost. The executor
    /// must also schedule `next_deadline`, including timeout/idle-only progress.
    pub async fn changed(&mut self) -> Result<()> {
        self.require_owned()?;
        self.page.check()?;
        self.entry.changed.notified().await;
        self.page.check()
    }
    pub fn next_deadline(&self) -> Result<Option<Duration>> {
        self.require_owned()?;
        self.page.next_deadline()
    }
    fn require_owned(&self) -> Result<()> {
        if self.taken {
            Err("navigation result already taken".into())
        } else {
            Ok(())
        }
    }
}
