//! One explicitly selected Page's bounded lifecycle owner. No task or queue.
use crate::{ChromiumFrameEventObserver, FrameObservationResult};
use pwright_backend_cdp::CdpTopology;
use pwright_backend_cdp_navigation::frame_lifecycle::{
    LifecycleSnapshot,
    native::{
        CdpFrameLifecycle, CdpFrameLifecycleProfile, CdpLifecycleRoutes, NativeLifecycleNotice,
    },
};
use pwright_cdp::{CdpEvent, generated::page};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};
use std::{
    error::Error,
    mem::size_of,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

mod registry;
pub use registry::{FrameLifecycleRegistry, FrameLifecycleRegistryProfile, PageLifecycleHandle};
mod navigation;
pub use navigation::{NavigationRegistrationProfile, NavigationWaitHandle};
use navigation::{NavigationSlots, NavigationSlotsProfile};

/// The embedding executor supplies one monotonic time origin for observations
/// and deadline waits. No global clock or implicit system-clock fallback.
pub trait FrameLifecycleClock: Send + Sync {
    fn now(&self) -> Duration;
}

/// Borrowed notices are consumed synchronously under the topology/owner locks.
/// The receiver separately admits any retained response/failure data and must
/// not reenter the owner/topology or invoke JavaScript from this callback.
pub trait FrameLifecycleNoticeSink: Send + Sync {
    fn observe(&self, notice: NativeLifecycleNotice<'_>) -> FrameObservationResult;
}

#[derive(Clone, Copy, Debug)]
pub struct PageLifecycleObservationProfile {
    native: CdpFrameLifecycleProfile,
    scope: ResourceScope,
    navigation: Option<NavigationSlotsProfile>,
}
impl PageLifecycleObservationProfile {
    pub const fn new(native: CdpFrameLifecycleProfile, scope: ResourceScope) -> Self {
        Self {
            native,
            scope,
            navigation: None,
        }
    }
    pub fn with_navigation_slots(mut self, scope: ResourceScope, slots: usize) -> Result<Self> {
        if scope == self.scope {
            return Err("navigation index requires its own scope".into());
        }
        self.navigation = Some(NavigationSlotsProfile::new(scope, slots)?);
        Ok(self)
    }
    fn owner_requirements(self) -> ResourceRequirements {
        // Includes the owner/Mutex, two Arc counters and conservative alignment
        // overhead. Inline native-body accounting is intentionally conservative.
        let bytes = (size_of::<PageLifecycleObservation>() + 2 * size_of::<usize>() + 256) as u64;
        ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, 1)
    }
    pub fn requirements(self) -> ResourceRequirements {
        let requirements = self
            .native
            .requirements()
            .combine_concurrent(self.owner_requirements());
        self.navigation.map_or(requirements, |profile| {
            requirements.combine_concurrent(profile.requirements())
        })
    }
}

struct State {
    native: CdpFrameLifecycle,
    seen_page: bool,
    navigation: Option<NavigationSlots>,
}
impl State {
    fn checkpoint(
        &mut self,
        now: Duration,
        notice: Option<&NativeLifecycleNotice<'_>>,
    ) -> Result<()> {
        if let Some(navigation) = &mut self.navigation {
            navigation.checkpoint(now, &self.native, notice)?;
        }
        Ok(())
    }
    fn wake_navigation(&self) {
        if let Some(navigation) = &self.navigation {
            navigation.wake_all();
        }
    }
}

/// Construct one owner for an explicit portable Page and connection epoch, then
/// install its Arc before driver initialization. Multiple Pages can be composed
/// by a caller-admitted observer; this owner never silently discovers Pages.
/// The caller owns the clock/sink allocation and any outer registry separately.
pub struct PageLifecycleObservation {
    page: ObjectId,
    epoch: u64,
    state: Mutex<State>,
    clock: Arc<dyn FrameLifecycleClock>,
    sink: Arc<dyn FrameLifecycleNoticeSink>,
    sealed: AtomicBool,
    closed: tokio::sync::Notify,
    _lease: ResourceLease,
}
impl PageLifecycleObservation {
    pub fn new(
        profile: PageLifecycleObservationProfile,
        page: ObjectId,
        epoch: u64,
        resources: &ProductionResourceLedger,
        clock: Arc<dyn FrameLifecycleClock>,
        sink: Arc<dyn FrameLifecycleNoticeSink>,
    ) -> Result<Self> {
        if page.get() == 0 || epoch == 0 {
            return Err("invalid Page lifecycle identity".into());
        }
        let requirements = profile.owner_requirements();
        let lease = resources.claim(requirements)?.reserve(
            profile.scope,
            ResourceClass::BackendStateBytes,
            requirements.class(ResourceClass::BackendStateBytes).global,
        )?;
        let native = CdpFrameLifecycle::new(profile.native, epoch, resources)?;
        Ok(Self {
            page,
            epoch,
            state: Mutex::new(State {
                native,
                seen_page: false,
                navigation: profile
                    .navigation
                    .map(|profile| NavigationSlots::new(profile, resources))
                    .transpose()?,
            }),
            clock,
            sink,
            sealed: AtomicBool::new(false),
            closed: tokio::sync::Notify::new(),
            _lease: lease,
        })
    }
    fn with_state<T>(&self, run: impl FnOnce(&mut State, Duration) -> Result<T>) -> Result<T> {
        if self.sealed.load(Ordering::Acquire) {
            return Err("Page lifecycle observer is sealed".into());
        }
        let mut state = self.state.lock().map_err(|_| {
            self.seal();
            "Page lifecycle owner unavailable"
        })?;
        if self.sealed.load(Ordering::Acquire) {
            return Err("Page lifecycle observer is sealed".into());
        }
        let now = self.clock.now();
        let result = (|| {
            // Publish elapsed timer readiness before a following commit/request
            // can erase it. All operation transitions share this Page lock.
            state.native.advance(now)?;
            state.checkpoint(now, None)?;
            let result = run(&mut state, now)?;
            state.checkpoint(now, None)?;
            Ok(result)
        })();
        if result.is_err() || self.is_sealed() {
            self.seal();
            state.wake_navigation();
        }
        result
    }
    pub fn snapshot(&self, frame: ObjectId) -> Result<Option<LifecycleSnapshot>> {
        let result = self.with_state(|state, now| {
            state.native.advance(now)?;
            Ok(state.native.snapshot(frame)?)
        })?;
        self.require_open()?;
        Ok(result)
    }
    pub fn next_deadline(&self) -> Result<Option<Duration>> {
        let result = self.with_state(|state, now| {
            state.native.advance(now)?;
            let native = state.native.next_deadline()?;
            let operation = state
                .navigation
                .as_ref()
                .map(NavigationSlots::next_deadline)
                .transpose()?
                .flatten();
            Ok(native.into_iter().chain(operation).min())
        })?;
        self.require_open()?;
        Ok(result)
    }
    fn require_open(&self) -> Result<()> {
        if self.is_sealed() {
            return Err("Page lifecycle observer is sealed".into());
        }
        Ok(())
    }
    pub fn is_sealed(&self) -> bool {
        self.sealed.load(Ordering::Acquire)
    }

    fn reconcile_state(
        &self,
        state: &mut State,
        now: Duration,
        topology: &CdpTopology,
    ) -> Result<bool> {
        if topology.epoch().get() != self.epoch {
            return Err("Page lifecycle epoch changed".into());
        }
        if topology.main_frame_native_id(self.page).is_none() {
            if state.seen_page {
                self.seal();
            }
            return Ok(false);
        }
        state.seen_page = true;
        state.native.retain_frames(now, |frame| {
            topology.page_object_id_for_frame(frame) == Some(self.page)
        })?;
        // Topology iteration is not parent-first. Repeated bounded scans avoid
        // allocating another traversal stack or keeping a second Frame map.
        loop {
            let mut missing = false;
            let mut progress = false;
            for (frame, page) in topology.frame_page_routes() {
                if page != self.page || state.native.snapshot(frame)?.is_some() {
                    continue;
                }
                missing = true;
                let parent = topology.parent_frame_object_id(frame);
                if let Some(parent) = parent
                    && state.native.snapshot(parent)?.is_none()
                {
                    continue;
                }
                state.native.attach(now, frame, parent)?;
                progress = true;
            }
            if !missing {
                return Ok(true);
            }
            if !progress {
                return Err("Page lifecycle topology has no reachable parent".into());
            }
        }
    }
}

struct Routes<'a> {
    topology: &'a CdpTopology,
    page: ObjectId,
}
impl CdpLifecycleRoutes for Routes<'_> {
    fn contains_session(&self, session: &str) -> bool {
        self.topology
            .target_session_route(session)
            .and_then(|route| self.topology.page_object_id(route.page_target_id()))
            == Some(self.page)
    }
    fn event_frame(&self, session: &str, native: &str) -> Option<ObjectId> {
        let frame = self.topology.frame_object_id(native)?;
        let route = self.topology.frame_navigation_route(frame)?;
        (route.session_id() == session && route.page_object_id() == self.page).then_some(frame)
    }
    fn request_frame(&self, session: &str, native: Option<&str>) -> Option<ObjectId> {
        native.and_then(|native| self.event_frame(session, native))
    }
    fn accepts_document_handoff(&self, frame: ObjectId, session: &str) -> bool {
        self.topology
            .frame_navigation_route(frame)
            .is_some_and(|route| {
                route.page_object_id() == self.page && route.session_id() == session
            })
    }
}

impl ChromiumFrameEventObserver for PageLifecycleObservation {
    fn observes_page(&self, page: ObjectId) -> bool {
        page == self.page && !self.is_sealed()
    }
    fn initialize_frame(
        &self,
        topology: &CdpTopology,
        session: &str,
        frame: &page::Frame,
    ) -> FrameObservationResult {
        if self.is_sealed() {
            return Ok(());
        }
        self.with_state(|state, now| {
            if !self.reconcile_state(state, now, topology)? {
                return Ok(());
            }
            let routes = Routes {
                topology,
                page: self.page,
            };
            if let Some(id) = routes.event_frame(session, &frame.id) {
                let notice = state.native.initialize_frame(
                    now,
                    id,
                    topology.parent_frame_object_id(id),
                    &frame.loader_id,
                )?;
                state.checkpoint(now, Some(&notice))?;
                if !matches!(notice, NativeLifecycleNotice::Ignored) {
                    self.sink.observe(notice)?;
                }
            }
            Ok(())
        })
    }
    fn reconcile(&self, topology: &CdpTopology) -> FrameObservationResult {
        if self.is_sealed() {
            return Ok(());
        }
        self.with_state(|state, now| {
            self.reconcile_state(state, now, topology)?;
            Ok(())
        })
    }
    fn event(&self, topology: &CdpTopology, event: &CdpEvent) -> FrameObservationResult {
        if self.is_sealed() {
            return Ok(());
        }
        self.with_state(|state, now| {
            if !self.reconcile_state(state, now, topology)? {
                return Ok(());
            }
            let notice = state.native.apply(
                now,
                self.epoch,
                event,
                &Routes {
                    topology,
                    page: self.page,
                },
            )?;
            state.checkpoint(now, Some(&notice))?;
            if !matches!(notice, NativeLifecycleNotice::Ignored) {
                self.sink.observe(notice)?;
            }
            Ok(())
        })
    }
    fn seal(&self) {
        self.sealed.store(true, Ordering::Release);
        self.closed.notify_waiters();
        // Never wait on an in-progress callback. Its with_state exit also wakes
        // consumers if sealing raced that callback's Page lock.
        if let Ok(state) = self.state.try_lock() {
            state.wake_navigation();
        }
    }
}
