//! Page-locked operation registration on the existing ordered observer path.
use super::*;
use pwright_backend_cdp_navigation::frame_lifecycle::navigation::{
    FrameNavigationWait, FrameNavigationWaitProfile, NavigationCandidate, NavigationWaitError,
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
    // Drop last, after the wait arena and owner lease release their budget.
    _shared_permit: Option<SharedNavigationPermit>,
}

/// An opt-in connection-wide ceiling independent of the shared byte ledger.
/// Unused budgets from other capabilities cannot admit an extra registration.
pub(super) struct SharedNavigationPool {
    active: AtomicUsize,
    limit: usize,
}
pub(super) struct SharedNavigationPermit(Arc<SharedNavigationPool>);
impl SharedNavigationPool {
    pub(super) fn new(limit: usize) -> Arc<Self> {
        Arc::new(Self {
            active: AtomicUsize::new(0),
            limit,
        })
    }

    fn admit(self: &Arc<Self>) -> Result<SharedNavigationPermit> {
        self.active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < self.limit).then_some(active + 1)
            })
            .map_err(|_| "shared navigation registration capacity exhausted")?;
        Ok(SharedNavigationPermit(self.clone()))
    }
}
impl Drop for SharedNavigationPermit {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::AcqRel);
    }
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
    shared_pool: Option<Arc<SharedNavigationPool>>,
    _lease: ResourceLease,
}
impl NavigationSlots {
    pub(super) fn new(
        profile: NavigationSlotsProfile,
        resources: &ProductionResourceLedger,
        shared_pool: Option<Arc<SharedNavigationPool>>,
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
            shared_pool,
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
        let shared_permit = self
            .shared_pool
            .as_ref()
            .map(SharedNavigationPool::admit)
            .transpose()?;
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
            _shared_permit: shared_permit,
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
/// The candidate URL, when present, is copied into caller-owned bounded
/// storage before this method returns and the lifecycle lock is released.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NavigationCandidateStatus {
    Pending,
    Url,
    Failed,
    Settled,
}
fn copy_candidate_into(
    slot: &Mutex<Option<FrameNavigationWait>>,
    index: usize,
    url: &mut String,
) -> Result<NavigationCandidateStatus> {
    let slot = slot
        .lock()
        .map_err(|_| "navigation registration unavailable")?;
    let wait = slot.as_ref().ok_or("navigation result already taken")?;
    if wait.outcome().is_some() {
        return Ok(NavigationCandidateStatus::Settled);
    }
    match wait.candidate(index)? {
        None => Ok(NavigationCandidateStatus::Pending),
        Some(NavigationCandidate::Failed) => Ok(NavigationCandidateStatus::Failed),
        Some(NavigationCandidate::Url(candidate)) => {
            if candidate.len() > url.capacity() {
                return Err("navigation candidate URL exceeds admitted scratch".into());
            }
            url.clear();
            url.push_str(candidate);
            Ok(NavigationCandidateStatus::Url)
        }
    }
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
    /// Examine one queued event without consuming the registration. The caller
    /// must run any user predicate only after this call has returned.
    pub fn candidate_into(
        &mut self,
        index: usize,
        url: &mut String,
    ) -> Result<NavigationCandidateStatus> {
        self.require_owned()?;
        self.page.check()?;
        let status = self
            .page
            .owner
            .with_state(|_, _| Ok(copy_candidate_into(&self.entry.wait, index, url)))??;
        self.page.check()?;
        Ok(status)
    }

    /// Select one observed candidate atomically with Page lifecycle progress.
    pub fn select_candidate(&mut self, index: usize) -> Result<()> {
        self.require_owned()?;
        self.page.check()?;
        self.page.owner.with_state(|state, now| {
            let mut slot = self
                .entry
                .wait
                .lock()
                .map_err(|_| "navigation registration unavailable")?;
            let wait = slot.as_mut().ok_or("navigation result already taken")?;
            let _ = wait.select_candidate(now, index);
            let _ = wait.poll(now, state.native.snapshot(self.entry.frame)?);
            self.entry.changed.notify_one();
            Ok(())
        })?;
        self.page.check()
    }
    /// Select the first navigation observed after this registration. This is
    /// distinct from a native command receipt, which pins one loader ID.
    pub fn wait_for_next_navigation(&mut self) -> Result<()> {
        self.require_owned()?;
        self.page.check()?;
        self.page.owner.with_state(|state, now| {
            let mut slot = self
                .entry
                .wait
                .lock()
                .map_err(|_| "navigation registration unavailable")?;
            let wait = slot.as_mut().ok_or("navigation result already taken")?;
            let _ = wait.wait_for_next_navigation(now);
            let _ = wait.poll(now, state.native.snapshot(self.entry.frame)?);
            if wait.outcome().is_some() {
                self.entry.changed.notify_one();
            }
            Ok(())
        })?;
        self.page.check()
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_core::ResourceLimits;

    #[test]
    fn candidate_copy_releases_wait_lock_before_predicate_handoff() {
        // Arrange
        let profile =
            FrameNavigationWaitProfile::new(ResourceScope::extension(241, 0), 2, 1, 32, 64, 128)
                .unwrap()
                .with_url_bytes(128)
                .unwrap();
        let resources = resources(profile.requirements());
        let mut wait = FrameNavigationWait::new(
            profile,
            &resources,
            ObjectId::new(1),
            WaitUntil::Load,
            Some(Duration::from_secs(1)),
        )
        .unwrap();
        wait.observe(
            Duration::ZERO,
            &NativeLifecycleNotice::SameDocument {
                frame: ObjectId::new(1),
                url: Some("https://test/third.html"),
            },
        )
        .unwrap();
        let slot = Mutex::new(Some(wait));
        let mut scratch = String::with_capacity(128);

        // Act: the simulated predicate reenters the wait lock after handoff.
        let status = copy_candidate_into(&slot, 0, &mut scratch).unwrap();
        let callback_reentered = slot.try_lock().is_ok();

        // Assert
        assert_eq!(status, NavigationCandidateStatus::Url);
        assert_eq!(scratch, "https://test/third.html");
        assert!(callback_reentered);
    }

    fn resources(requirements: ResourceRequirements) -> ProductionResourceLedger {
        let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let bound = requirements.class(class);
            limits = limits.with_class(class, bound.global, bound.per_scope);
        }
        ProductionResourceLedger::new(limits, requirements).unwrap()
    }

    fn profiles() -> (NavigationSlotsProfile, NavigationRegistrationProfile) {
        let slots = NavigationSlotsProfile::new(ResourceScope::extension(240, 0), 20).unwrap();
        let registration = NavigationRegistrationProfile::new(
            FrameNavigationWaitProfile::new(
                ResourceScope::extension(240, 1),
                1,
                1,
                8,
                8,
                64 * 1024,
            )
            .unwrap(),
            ResourceScope::extension(240, 2),
        );
        (slots, registration)
    }

    #[test]
    fn shared_pool_admits_twenty_across_pages_rejects_next_and_reclaims_drop() {
        // Arrange: one Page can hold the official twenty-wait burst while a
        // second Page still has a vacant index under the same connection pool.
        let (slot_profile, registration) = profiles();
        let requirements = (0..20).fold(
            slot_profile
                .requirements()
                .combine_concurrent(slot_profile.requirements()),
            |all, _| all.combine_concurrent(registration.requirements()),
        );
        let resources = resources(requirements);
        let pool = SharedNavigationPool::new(20);
        let mut first = NavigationSlots::new(slot_profile, &resources, Some(pool.clone())).unwrap();
        let mut second =
            NavigationSlots::new(slot_profile, &resources, Some(pool.clone())).unwrap();
        let params = || Registration {
            profile: registration,
            resources: &resources,
            frame: ObjectId::new(1),
            wait: WaitUntil::Load,
            deadline: None,
        };
        let mut tickets = Vec::new();
        for _ in 0..20 {
            tickets.push(first.register(params()).unwrap());
        }

        // Act: a vacant Page slot cannot borrow slack from another budget.
        let excess = second.register(params());
        let dropped = tickets.remove(0);
        let cancellation = dropped
            .wait
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .abort(NavigationWaitError::Canceled);
        let still_retained = second.register(params());
        drop(dropped);
        first.reclaim();
        let replacement = second.register(params());

        // Assert
        assert!(matches!(cancellation, Err(NavigationWaitError::Canceled)));
        assert!(excess.is_err());
        assert!(still_retained.is_err());
        assert_eq!(
            excess.err().unwrap().to_string(),
            "shared navigation registration capacity exhausted"
        );
        assert!(replacement.is_ok());
        assert_eq!(pool.active.load(Ordering::Acquire), 20);
        drop(first);
        drop(second);
        assert_eq!(pool.active.load(Ordering::Acquire), 20);
        drop(tickets);
        drop(replacement);
        assert_eq!(pool.active.load(Ordering::Acquire), 0);
    }

    #[test]
    fn failed_wait_allocation_rolls_back_shared_permit_and_owner_lease() {
        // Arrange: no wait arena is in the plan, so admission must fail after
        // the shared permit and registration owner lease have been acquired.
        let (slot_profile, registration) = profiles();
        let requirements = slot_profile
            .requirements()
            .combine_concurrent(registration.owner_requirements());
        let resources = resources(requirements);
        let ledger = resources.claim(requirements).unwrap();
        let pool = SharedNavigationPool::new(20);
        let mut slots = NavigationSlots::new(slot_profile, &resources, Some(pool.clone())).unwrap();

        // Act
        let result = slots.register(Registration {
            profile: registration,
            resources: &resources,
            frame: ObjectId::new(1),
            wait: WaitUntil::Load,
            deadline: None,
        });

        // Assert
        assert!(result.is_err());
        assert_eq!(pool.active.load(Ordering::Acquire), 0);
        assert!(slots.entries.iter().all(Option::is_none));
        assert!(ledger.scope_metrics(registration.scope).unwrap().is_none());
    }
}
