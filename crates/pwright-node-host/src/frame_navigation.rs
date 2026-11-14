//! Selected Rust executor. Node value conversion and the generated Frame API
//! are separate boundaries; this port accepts already-resolved native URLs.
mod result;
use crate::{HostError, HostSession, frame_lifecycle::registration_profile};
use pwright_backend_cdp_navigation::frame_lifecycle::navigation::{
    FrameNavigationWait, NavigationWaitError,
};
use pwright_cap_navigation::{BeginNavigationParams, WaitUntil};
use pwright_composition_chromium::{
    CdpPageServices, ChromiumFrameNavigationAccess, ChromiumFrameNavigationProfile,
    ChromiumFrameNavigationSnapshot,
    frame_lifecycle::{NavigationWaitHandle, PageLifecycleHandle},
};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};
#[cfg(not(feature = "navigation-url-match"))]
use pwright_plugin_chromium_frame_lifecycle::NavigationRegistrationProfile;
#[cfg(feature = "navigation-url-match")]
use pwright_plugin_chromium_frame_lifecycle::{
    NavigationCandidateStatus, NavigationRegistrationProfile,
};
pub use result::FrameGotoOutput;
#[cfg(feature = "navigation-url-match")]
#[derive(Debug)]
pub enum NavigationMatchError<E> {
    Host(HostError),
    Matcher(E),
}
#[cfg(feature = "navigation-url-match")]
impl<E> From<HostError> for NavigationMatchError<E> {
    fn from(error: HostError) -> Self {
        Self::Host(error)
    }
}
use std::{
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

#[cfg(feature = "frame-navigation-burst-20")]
const MAX_OPERATIONS: u64 = 21;
#[cfg(not(feature = "frame-navigation-burst-20"))]
const MAX_OPERATIONS: u64 = 8;
#[cfg(feature = "frame-navigation-burst-20")]
const MAX_WAITERS: u64 = 20;
#[cfg(not(feature = "frame-navigation-burst-20"))]
const MAX_WAITERS: u64 = 8;
#[cfg(feature = "frame-navigation-burst-20")]
const MAX_COMMANDS: u64 = 20;
#[cfg(not(feature = "frame-navigation-burst-20"))]
const MAX_COMMANDS: u64 = 8;
const MAX_URL_BYTES: usize = 16 * 1024;
// Two caller-owned strings (bounded capacity, not just length), executor/Select
// state and timer storage. Native command copies and response owners have their
// own profiles. The embedding task's stack/allocator metadata remain external.
const OPERATION_BYTES: u64 = 2 * MAX_URL_BYTES as u64 + 16 * 1024;
const OPERATION_SCOPE: ResourceScope = ResourceScope::extension(126, 1);

struct Slots {
    // High 32 bits count waiters, low 32 bits count all operations. One CAS
    // admits both counts atomically, so a 21st waiter cannot steal the burst
    // profile's separate Page.goto command/registration slot.
    active: AtomicU64,
    _lease: ResourceLease,
}
#[derive(Clone, Copy)]
enum OperationKind {
    Waiter,
    Command,
}
impl OperationKind {
    const fn increment(self) -> u64 {
        match self {
            Self::Waiter => (1_u64 << 32) + 1,
            Self::Command => 1,
        }
    }
}
struct OperationSlot(Arc<Slots>, OperationKind);
impl Drop for OperationSlot {
    fn drop(&mut self) {
        self.0
            .active
            .fetch_sub(self.1.increment(), Ordering::AcqRel);
    }
}
impl Slots {
    fn new(resources: &ProductionResourceLedger) -> Result<Arc<Self>, HostError> {
        let _lease = resources
            .claim(slot_requirements())
            .map_err(HostError::backend)?
            .reserve(
                ResourceScope::extension(126, 2),
                ResourceClass::BackendStateBytes,
                slot_requirements()
                    .class(ResourceClass::BackendStateBytes)
                    .global,
            )
            .map_err(HostError::backend)?;
        Ok(Arc::new(Self {
            active: AtomicU64::new(0),
            _lease,
        }))
    }
    fn admit(self: &Arc<Self>, kind: OperationKind) -> Result<OperationSlot, HostError> {
        self.active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |counts| {
                let active = counts & u64::from(u32::MAX);
                let waiters = counts >> 32;
                let commands = active - waiters;
                let lane_has_space = match kind {
                    OperationKind::Waiter => waiters < MAX_WAITERS,
                    OperationKind::Command => commands < MAX_COMMANDS,
                };
                (active < MAX_OPERATIONS && lane_has_space).then_some(counts + kind.increment())
            })
            .map_err(|_| {
                HostError::ResourceExhausted("Frame navigation operation capacity exhausted".into())
            })?;
        Ok(OperationSlot(self.clone(), kind))
    }
}
pub(crate) struct Installation {
    routes: ChromiumFrameNavigationAccess,
    slots: Arc<Slots>,
    results: Arc<result::ResultSlots>,
}

struct PreparedNavigationWait {
    #[cfg(feature = "response-request")]
    page_id: String,
    page: PageLifecycleHandle,
    ticket: NavigationWaitHandle,
    permit: result::ResultPermit,
    deadline: Option<tokio::time::Instant>,
    timeout_ms: u32,
    _operation: tokio_util::task::task_tracker::TaskTrackerToken,
    _storage: ResourceLease,
    // A Future dropped before polling releases storage before its count slot.
    _admission: OperationSlot,
}

struct PreparedLoadStateWait {
    page: PageLifecycleHandle,
    frame: ObjectId,
    wait: WaitUntil,
    revision: u64,
    deadline: Option<tokio::time::Instant>,
    timeout_ms: u32,
    _operation: tokio_util::task::task_tracker::TaskTrackerToken,
    _storage: ResourceLease,
    _admission: OperationSlot,
}
fn slot_requirements() -> ResourceRequirements {
    let bytes = (std::mem::size_of::<Slots>() + 2 * std::mem::size_of::<usize>() + 256) as u64;
    ResourceRequirements::none()
        .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
        .with_metadata(1, 1)
}

fn route_profile() -> Result<ChromiumFrameNavigationProfile, HostError> {
    ChromiumFrameNavigationProfile::new(
        ResourceScope::extension(126, 0),
        NonZeroUsize::new(128).expect("positive Frame limit"),
        NonZeroUsize::new(8).expect("positive operation limit"),
        NonZeroUsize::new(256).expect("positive native ID limit"),
    )
    .map_err(HostError::backend)
}
fn operation_requirements() -> ResourceRequirements {
    ResourceRequirements::none()
        .with_class(
            ResourceClass::BackendStateBytes,
            OPERATION_BYTES * MAX_OPERATIONS,
            OPERATION_BYTES * MAX_OPERATIONS,
        )
        .with_metadata(1, MAX_OPERATIONS as usize)
}
pub(crate) fn requirements() -> Result<ResourceRequirements, HostError> {
    Ok(route_profile()?
        .requirements()
        .combine_concurrent(operation_requirements())
        .combine_concurrent(slot_requirements())
        .combine_concurrent(result::requirements()))
}
pub(crate) fn install(
    services: &CdpPageServices,
    resources: &ProductionResourceLedger,
) -> Result<Installation, HostError> {
    let slots = Slots::new(resources)?;
    let routes = ChromiumFrameNavigationAccess::new(services, route_profile()?, resources)
        .map_err(HostError::backend)?;
    let results = result::ResultSlots::new(resources)?;
    Ok(Installation {
        routes,
        slots,
        results,
    })
}
fn validate(params: &BeginNavigationParams) -> Result<(), HostError> {
    if params.url.capacity() > MAX_URL_BYTES
        || params
            .referrer
            .as_ref()
            .is_some_and(|s| s.capacity() > MAX_URL_BYTES)
    {
        return Err(HostError::ResourceExhausted(
            "Frame navigation URL storage exceeds 16 KiB per string".into(),
        ));
    }
    Ok(())
}

impl HostSession {
    pub(crate) async fn publish_navigation_topology(
        &self,
        deadline: Option<tokio::time::Instant>,
        timeout_ms: u32,
    ) -> Result<(), HostError> {
        #[cfg(feature = "frame-navigation-burst-20")]
        {
            send_burst_navigation_fence(
                &self.topology_barriers,
                &self.stop,
                &self.disconnected,
                deadline,
                timeout_ms,
            )
            .await
        }
        #[cfg(not(feature = "frame-navigation-burst-20"))]
        {
            let _ = (deadline, timeout_ms);
            self.flush_topology().await
        }
    }

    /// Rust-owned native dispatch plus ordered document/lifecycle/response wait.
    ///
    /// The input URL is already resolved; this is not yet the public Node
    /// Frame.goto contract (baseURL, header reconciliation and error formatting).
    /// Zero timeout disables the operation deadline. Dropping this future or
    /// closing the host abandons only this pending CDP call, never Page loading.
    /// The returned bounded owner contains either completion or a terminal
    /// navigation error; inspect `outcome()` without copying its response JSON.
    pub async fn navigate_frame_native(
        &self,
        page_id: &str,
        frame_id: &str,
        params: BeginNavigationParams,
        wait: WaitUntil,
        timeout_ms: u32,
    ) -> Result<FrameNavigationWait, HostError> {
        self.navigate_frame_native_with(page_id, frame_id, params, wait, timeout_ms, Ok)
            .await
    }

    /// Typed 1.2 result over the existing Rust native executor. URLs must be
    /// resolved by the caller; Context option reconciliation and Node value
    /// conversion remain separate. The returned owner keeps its allocation
    /// charge until dropped, including when the host has already closed.
    pub async fn frame_goto(
        &self,
        page_id: &str,
        frame_id: &str,
        params: pwright_cap_navigation::v1_2::GotoParams,
        timeout_ms: u32,
    ) -> Result<FrameGotoOutput, HostError> {
        #[cfg(feature = "extra-headers")]
        let mut headers = self.navigation_referrer(page_id, params.referrer)?;
        #[cfg(feature = "extra-headers")]
        let params = pwright_cap_navigation::v1_2::GotoParams {
            referrer: headers.value.take(),
            ..params
        };
        self.frame_goto_prepared(page_id, frame_id, params, timeout_ms)
            .await
    }

    /// Capture the public Frame.goto URL, options, and Page header policy before
    /// the Python awaitable is scheduled, matching the Node facade's call time.
    pub fn start_frame_goto_public(
        self: &std::sync::Arc<Self>,
        page_id: String,
        frame_id: String,
        url: String,
        wait_until: &str,
        referrer: Option<String>,
        timeout_ms: u32,
    ) -> Result<
        impl Future<Output = Result<FrameGotoOutput, HostError>> + Send + 'static + use<>,
        HostError,
    > {
        use pwright_cap_navigation::v1_2::{GotoParams, WaitUntil as FrameWaitUntil};

        let wait_until = match crate::commands::parse_wait_until(wait_until)? {
            WaitUntil::Commit => FrameWaitUntil::Commit,
            WaitUntil::DomContentLoaded => FrameWaitUntil::DomContentLoaded,
            WaitUntil::Load => FrameWaitUntil::Load,
            WaitUntil::NetworkIdle => FrameWaitUntil::NetworkIdle,
        };
        let url = self.resolve_page_url(&page_id, &url)?;
        let params = GotoParams {
            url,
            referrer,
            wait_until,
        };
        #[cfg(feature = "extra-headers")]
        let pending = self.start_frame_goto(page_id, frame_id, params, timeout_ms);
        #[cfg(not(feature = "extra-headers"))]
        let pending = {
            let host = self.clone();
            async move {
                host.frame_goto_prepared(&page_id, &frame_id, params, timeout_ms)
                    .await
            }
        };
        Ok(pending)
    }

    /// Start waiting for the next main- or child-Frame navigation without
    /// issuing a native navigation command. Registration is completed before
    /// this method returns its Future, so a fast page-triggered navigation
    /// cannot slip between foreign-call admission and observation.
    pub fn start_wait_for_frame_navigation(
        self: &Arc<Self>,
        page_id: String,
        frame_id: String,
        wait: WaitUntil,
        timeout_ms: u32,
    ) -> impl Future<Output = Result<FrameGotoOutput, HostError>> + Send + 'static + use<> {
        let prepared = crate::frame_lifecycle::registration_profile().and_then(|profile| {
            self.prepare_wait_for_navigation(&page_id, &frame_id, wait, timeout_ms, profile, true)
        });
        let host = self.clone();
        async move {
            let prepared = prepared?;
            host.finish_wait_for_navigation(prepared).await
        }
    }

    /// One synchronous registration and one deadline survive every rejected
    /// URL. The language predicate runs on this executor Future, after the
    /// lifecycle lock has released. Python currently uses the same Tokio
    /// callback context as Page.wait_for_url, not the caller's asyncio loop.
    #[cfg(feature = "navigation-url-match")]
    pub fn start_wait_for_frame_navigation_matching<E, M>(
        self: &Arc<Self>,
        page_id: String,
        frame_id: String,
        wait: WaitUntil,
        timeout_ms: u32,
        mut matches: M,
    ) -> impl Future<Output = Result<FrameGotoOutput, NavigationMatchError<E>>>
    + Send
    + 'static
    + use<E, M>
    where
        E: Send + 'static,
        M: FnMut(&str) -> Result<bool, E> + Send + 'static,
    {
        let prepared = crate::frame_lifecycle::matched_registration_profile().and_then(|profile| {
            self.prepare_wait_for_navigation(&page_id, &frame_id, wait, timeout_ms, profile, false)
        });
        let host = self.clone();
        async move {
            let mut prepared = prepared?;
            let mut url = String::new();
            url.try_reserve_exact(MAX_URL_BYTES).map_err(|error| {
                HostError::ResourceExhausted(format!(
                    "navigation matcher URL scratch allocation failed: {error}"
                ))
            })?;
            let mut index = 0_usize;
            loop {
                if prepared
                    .deadline
                    .is_some_and(|at| tokio::time::Instant::now() >= at)
                {
                    return Err(HostError::Timeout(timeout_ms).into());
                }
                match prepared
                    .ticket
                    .candidate_into(index, &mut url)
                    .map_err(|error| observation_error(&prepared.page, error))?
                {
                    NavigationCandidateStatus::Url => {
                        let matched = matches(&url).map_err(NavigationMatchError::Matcher)?;
                        if prepared
                            .deadline
                            .is_some_and(|at| tokio::time::Instant::now() >= at)
                        {
                            return Err(HostError::Timeout(timeout_ms).into());
                        }
                        if matched {
                            prepared
                                .ticket
                                .select_candidate(index)
                                .map_err(|error| observation_error(&prepared.page, error))?;
                            break;
                        }
                        index = index.checked_add(1).ok_or_else(|| {
                            HostError::ResourceExhausted(
                                "navigation candidate index exhausted".into(),
                            )
                        })?;
                    }
                    NavigationCandidateStatus::Failed => {
                        prepared
                            .ticket
                            .select_candidate(index)
                            .map_err(|error| observation_error(&prepared.page, error))?;
                        break;
                    }
                    NavigationCandidateStatus::Settled => break,
                    NavigationCandidateStatus::Pending => {
                        let next = prepared
                            .ticket
                            .next_deadline()
                            .map_err(|error| observation_error(&prepared.page, error))?;
                        let delay = next
                            .map(|at| {
                                prepared
                                    .page
                                    .now()
                                    .map(|now| at.saturating_sub(now))
                                    .map_err(|error| observation_error(&prepared.page, error))
                            })
                            .transpose()?;
                        tokio::select! {
                            biased;
                            () = host.stop.cancelled() => return Err(HostError::TargetClosed.into()),
                            () = host.disconnected.cancelled() => return Err(HostError::TargetClosed.into()),
                            () = prepared.page.closed() => return Err(HostError::TargetClosed.into()),
                            () = wait_deadline(prepared.deadline) => return Err(HostError::Timeout(timeout_ms).into()),
                            changed = prepared.ticket.changed() => {
                                changed.map_err(|error| observation_error(&prepared.page, error))?;
                            }
                            () = wait_duration(delay) => {}
                        }
                    }
                }
            }
            host.finish_wait_for_navigation(prepared)
                .await
                .map_err(NavigationMatchError::Host)
        }
    }

    /// Wait for a lifecycle state on the Frame's current document without
    /// requiring a following navigation. Preparation captures the ordered
    /// lifecycle revision before this method returns its Future, so a native
    /// event cannot slip between the foreign call and the async waiter.
    pub fn start_wait_for_frame_load_state(
        self: &Arc<Self>,
        page_id: String,
        frame_id: String,
        wait: WaitUntil,
        timeout_ms: u32,
    ) -> impl Future<Output = Result<(), HostError>> + Send + 'static + use<> {
        let prepared = self.prepare_wait_for_load_state(&page_id, &frame_id, wait, timeout_ms);
        let host = self.clone();
        async move {
            let prepared = prepared?;
            host.finish_wait_for_load_state(prepared).await
        }
    }

    fn prepare_wait_for_load_state(
        &self,
        page_id: &str,
        frame_id: &str,
        wait: WaitUntil,
        timeout_ms: u32,
    ) -> Result<PreparedLoadStateWait, HostError> {
        let page_object = page_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Page ID".into()))?;
        let (frame, _, operation) = self.admit_operation(frame_id)?;
        let (page, admission) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let page = live
                .frame_lifecycle
                .page(page_object)
                .map_err(HostError::backend)?
                .ok_or(HostError::TargetClosed)?;
            let admission = live.frame_navigation.slots.admit(OperationKind::Waiter)?;
            (page, admission)
        };
        let storage = self
            .resources
            .claim(operation_requirements())
            .map_err(HostError::backend)?
            .reserve(
                OPERATION_SCOPE,
                ResourceClass::BackendStateBytes,
                OPERATION_BYTES,
            )
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        // Read the revision first. If a transition occurs before the snapshot,
        // either the snapshot observes readiness or changed_after immediately
        // observes the advanced revision.
        let revision = page
            .revision()
            .map_err(|error| observation_error(&page, error))?;
        if page
            .snapshot(frame)
            .map_err(|error| observation_error(&page, error))?
            .is_none()
        {
            return Err(HostError::InvalidArgument(
                "Frame does not belong to the selected Page".into(),
            ));
        }
        let deadline = (timeout_ms != 0)
            .then(|| tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms)));
        Ok(PreparedLoadStateWait {
            page,
            frame,
            wait,
            revision,
            deadline,
            timeout_ms,
            _operation: operation,
            _storage: storage,
            _admission: admission,
        })
    }

    async fn finish_wait_for_load_state(
        &self,
        prepared: PreparedLoadStateWait,
    ) -> Result<(), HostError> {
        let PreparedLoadStateWait {
            page,
            frame,
            wait,
            mut revision,
            deadline,
            timeout_ms,
            _operation,
            _admission,
            _storage,
        } = prepared;
        loop {
            let snapshot = page
                .snapshot(frame)
                .map_err(|error| observation_error(&page, error))?
                .ok_or(HostError::TargetClosed)?;
            if snapshot.reached(wait) {
                break;
            }
            let next = page
                .next_deadline()
                .map_err(|error| observation_error(&page, error))?;
            let delay = next
                .map(|at| {
                    page.now()
                        .map(|now| at.saturating_sub(now))
                        .map_err(|error| observation_error(&page, error))
                })
                .transpose()?;
            tokio::select! {
                biased;
                () = self.stop.cancelled() => return Err(HostError::TargetClosed),
                () = self.disconnected.cancelled() => return Err(HostError::TargetClosed),
                () = page.closed() => return Err(HostError::TargetClosed),
                () = wait_deadline(deadline) => return Err(HostError::Timeout(timeout_ms)),
                changed = page.changed_after(revision) => {
                    revision = changed.map_err(|error| observation_error(&page, error))?;
                }
                () = wait_duration(delay) => {}
            }
        }
        crate::completion::publish_lifetime_result(
            Ok(()),
            self.is_connected(),
            self.publish_navigation_topology(deadline, timeout_ms),
            page.closed(),
            || page.is_sealed(),
        )
        .await?;
        if deadline.is_some_and(|at| tokio::time::Instant::now() >= at) {
            return Err(HostError::Timeout(timeout_ms));
        }
        Ok(())
    }

    fn prepare_wait_for_navigation(
        &self,
        page_id: &str,
        frame_id: &str,
        wait: WaitUntil,
        timeout_ms: u32,
        registration: NavigationRegistrationProfile,
        arm_next: bool,
    ) -> Result<PreparedNavigationWait, HostError> {
        let page_object = page_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(pwright_core::ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Page ID".into()))?;
        let (frame, _, operation) = self.admit_operation(frame_id)?;
        // A foreign Page/Frame wrapper is published only after the topology
        // fence that installed this registry entry. Do not add another async
        // flush here: wait registration must precede the Promise handoff.
        let (page, permit, admission) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let page = live
                .frame_lifecycle
                .page(page_object)
                .map_err(HostError::backend)?
                .ok_or(HostError::TargetClosed)?;
            let permit = live.frame_navigation.results.admit(&self.resources)?;
            let admission = live.frame_navigation.slots.admit(OperationKind::Waiter)?;
            (page, permit, admission)
        };
        if page
            .snapshot(frame)
            .map_err(|error| observation_error(&page, error))?
            .is_none()
        {
            return Err(HostError::InvalidArgument(
                "Frame does not belong to the selected Page".into(),
            ));
        }
        let storage = self
            .resources
            .claim(operation_requirements())
            .map_err(HostError::backend)?
            .reserve(
                OPERATION_SCOPE,
                ResourceClass::BackendStateBytes,
                OPERATION_BYTES,
            )
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let deadline = (timeout_ms != 0)
            .then(|| tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms)));
        let at = deadline
            .map(|deadline| {
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                page.now()
                    .map_err(HostError::backend)?
                    .checked_add(remaining)
                    .ok_or_else(|| {
                        HostError::InvalidArgument("navigation deadline overflow".into())
                    })
            })
            .transpose()?;
        let mut ticket = page
            .register_navigation(registration, &self.resources, frame, wait, at)
            .map_err(HostError::backend)?;
        if arm_next {
            ticket
                .wait_for_next_navigation()
                .map_err(|error| observation_error(&page, error))?;
        }
        Ok(PreparedNavigationWait {
            #[cfg(feature = "response-request")]
            page_id: page_id.to_owned(),
            page,
            ticket,
            permit,
            deadline,
            timeout_ms,
            _operation: operation,
            _storage: storage,
            _admission: admission,
        })
    }

    async fn finish_wait_for_navigation(
        &self,
        prepared: PreparedNavigationWait,
    ) -> Result<FrameGotoOutput, HostError> {
        let PreparedNavigationWait {
            #[cfg(feature = "response-request")]
            page_id,
            page,
            mut ticket,
            permit,
            deadline,
            timeout_ms,
            _operation,
            _admission,
            _storage,
        } = prepared;
        let operation = async {
            let result = loop {
                if let Some(result) = ticket
                    .try_take()
                    .map_err(|error| observation_error(&page, error))?
                {
                    break result;
                }
                let next = ticket
                    .next_deadline()
                    .map_err(|error| observation_error(&page, error))?;
                let delay = next
                    .map(|at| {
                        page.now()
                            .map(|now| at.saturating_sub(now))
                            .map_err(|error| observation_error(&page, error))
                    })
                    .transpose()?;
                tokio::select! {
                    biased;
                    () = self.stop.cancelled() => return Err(HostError::TargetClosed),
                    () = self.disconnected.cancelled() => return Err(HostError::TargetClosed),
                    () = page.closed() => return Err(HostError::TargetClosed),
                    () = wait_deadline(deadline) => return Err(HostError::Timeout(timeout_ms)),
                    changed = ticket.changed() => {
                        changed.map_err(|error| observation_error(&page, error))?;
                    }
                    () = wait_duration(delay) => {}
                }
            };
            drop(ticket);
            crate::completion::publish_lifetime_result(
                Ok(result),
                self.is_connected(),
                self.publish_navigation_topology(deadline, timeout_ms),
                page.closed(),
                || page.is_sealed(),
            )
            .await
            .map(|result| (result, page))
        };
        let (result, page) = tokio::select! {
            biased;
            () = self.stop.cancelled() => Err(HostError::TargetClosed),
            () = self.disconnected.cancelled() => Err(HostError::TargetClosed),
            () = wait_deadline(deadline) => Err(HostError::Timeout(timeout_ms)),
            result = operation => result,
        }?;
        let output = result::project(result, permit, timeout_ms)?;
        if page.is_sealed() || !self.is_connected() {
            return Err(HostError::TargetClosed);
        }
        if deadline.is_some_and(|at| tokio::time::Instant::now() >= at) {
            return Err(HostError::Timeout(timeout_ms));
        }
        #[cfg(feature = "response-request")]
        let mut output = output;
        #[cfg(feature = "response-request")]
        self.attach_response_request(&page_id, &mut output).await?;
        Ok(output)
    }

    /// Synchronous Page policy capture; native Frame routing and lifecycle wait
    /// remain asynchronous and retain the admitted navigation owner.
    #[cfg(feature = "extra-headers")]
    pub fn start_frame_goto(
        self: &std::sync::Arc<Self>,
        page_id: String,
        frame_id: String,
        mut params: pwright_cap_navigation::v1_2::GotoParams,
        timeout_ms: u32,
    ) -> impl Future<Output = Result<FrameGotoOutput, HostError>> + Send + 'static + use<> {
        let prepared = self.navigation_referrer(&page_id, params.referrer.take());
        let host = self.clone();
        async move {
            let mut headers = prepared?;
            params.referrer = headers.value.take();
            host.frame_goto_prepared(&page_id, &frame_id, params, timeout_ms)
                .await
        }
    }

    async fn frame_goto_prepared(
        &self,
        page_id: &str,
        frame_id: &str,
        params: pwright_cap_navigation::v1_2::GotoParams,
        timeout_ms: u32,
    ) -> Result<FrameGotoOutput, HostError> {
        let permit = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .frame_navigation
                .results
                .admit(&self.resources)?
        };
        let wait = match params.wait_until {
            pwright_cap_navigation::v1_2::WaitUntil::Commit => WaitUntil::Commit,
            pwright_cap_navigation::v1_2::WaitUntil::DomContentLoaded => {
                WaitUntil::DomContentLoaded
            }
            pwright_cap_navigation::v1_2::WaitUntil::Load => WaitUntil::Load,
            pwright_cap_navigation::v1_2::WaitUntil::NetworkIdle => WaitUntil::NetworkIdle,
        };
        let output = self
            .navigate_frame_native_with(
                page_id,
                frame_id,
                BeginNavigationParams {
                    url: params.url,
                    referrer: params.referrer,
                },
                wait,
                timeout_ms,
                |owner| result::project(owner, permit, timeout_ms),
            )
            .await?;
        #[cfg(feature = "response-request")]
        let mut output = output;
        #[cfg(feature = "response-request")]
        self.attach_response_request(page_id, &mut output).await?;
        Ok(output)
    }

    #[cfg(feature = "response-request")]
    async fn attach_response_request(
        &self,
        page_id: &str,
        output: &mut FrameGotoOutput,
    ) -> Result<(), HostError> {
        let request_id = output
            .outcome()
            .as_ref()
            .ok()
            .and_then(|result| result.response.as_ref())
            .and_then(|_| output.response_request_id())
            .map(str::to_owned);
        if let Some(request_id) = request_id {
            let request = self.request_by_native_id(page_id, &request_id).await?;
            #[cfg(feature = "response-security-details")]
            if let Some(response) = output
                .outcome()
                .as_ref()
                .ok()
                .and_then(|result| result.response.as_ref())
            {
                let details = request
                    .observed_navigation_response(&request_id, &response.url, response.status)
                    .await?
                    .map(|response| response.security_details);
                output.attach_response_security_details(details);
            }
            #[cfg(feature = "response-finished")]
            if let Some(response) = request.response().await?
                && let Some(owner) = response.finished_owner
            {
                output.attach_response_finished_owner(owner);
            }
            output.attach_response_request(request);
        }
        Ok(())
    }

    async fn navigate_frame_native_with<Output>(
        &self,
        page_id: &str,
        frame_id: &str,
        params: BeginNavigationParams,
        wait: WaitUntil,
        timeout_ms: u32,
        finish: impl FnOnce(FrameNavigationWait) -> Result<Output, HostError>,
    ) -> Result<Output, HostError> {
        validate(&params)?;
        let (frame, _, _operation) = self.admit_operation(frame_id)?;
        let _admission = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .frame_navigation
                .slots
                .admit(OperationKind::Command)?
        };
        let _storage = self
            .resources
            .claim(operation_requirements())
            .map_err(HostError::backend)?
            .reserve(
                OPERATION_SCOPE,
                ResourceClass::BackendStateBytes,
                OPERATION_BYTES,
            )
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        // Host clock boundary: start before the publication/route preparation,
        // not after it. The Page reducer still uses its injected clock origin.
        let deadline = (timeout_ms != 0)
            .then(|| tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms)));
        let operation = async {
            let page = self.frame_lifecycle(page_id).await?;
            let route = {
                let state = self.state.lock().map_err(HostError::backend)?;
                let live = state.as_ref().ok_or(HostError::TargetClosed)?;
                live.frame_navigation
                    .routes
                    .snapshot(frame)
                    .map_err(HostError::backend)?
            };
            if Some(route.page().get()) != page_id.parse::<u64>().ok() {
                return Err(HostError::InvalidArgument(
                    "Frame does not belong to the selected Page".into(),
                ));
            }
            let remaining =
                deadline.map(|at| at.saturating_duration_since(tokio::time::Instant::now()));
            let at = remaining
                .map(|remaining| {
                    page.now()
                        .map_err(HostError::backend)?
                        .checked_add(remaining)
                        .ok_or_else(|| {
                            HostError::InvalidArgument("navigation deadline overflow".into())
                        })
                })
                .transpose()?;
            let mut ticket = page
                .register_navigation(registration_profile()?, &self.resources, frame, wait, at)
                .map_err(HostError::backend)?;
            let result = drive(&route, &page, &mut ticket, params, timeout_ms).await;
            // Release route/registration before publication, but keep the exact
            // Page lifetime through the fence and synchronous result projection.
            // Renderer route replacement alone is not Page closure.
            drop(ticket);
            drop(route);
            let result = crate::completion::publish_lifetime_result(
                result,
                self.is_connected(),
                self.publish_navigation_topology(deadline, timeout_ms),
                page.closed(),
                || page.is_sealed(),
            )
            .await?;
            Ok((result, page))
        };
        let (settled, page) = tokio::select! {
            biased;
            () = self.stop.cancelled() => Err(HostError::TargetClosed),
            () = self.disconnected.cancelled() => Err(HostError::TargetClosed),
            () = wait_deadline(deadline) => Err(HostError::Timeout(timeout_ms)),
            result = operation => result,
        }?;
        // Projection is synchronous and bounded. Keep the TaskTracker token,
        // executor admission and deadline alive through this final allocation.
        let result = finish(settled);
        if page.is_sealed() || !self.is_connected() {
            return Err(HostError::TargetClosed);
        }
        let result = result?;
        if deadline.is_some_and(|at| tokio::time::Instant::now() >= at) {
            return Err(HostError::Timeout(timeout_ms));
        }
        Ok(result)
    }
}

#[cfg(feature = "frame-navigation-burst-20")]
async fn send_burst_navigation_fence(
    queue: &tokio::sync::mpsc::Sender<crate::lifecycle::DriverRequest>,
    stop: &tokio_util::sync::CancellationToken,
    disconnected: &tokio_util::sync::CancellationToken,
    deadline: Option<tokio::time::Instant>,
    timeout_ms: u32,
) -> Result<(), HostError> {
    let (done, completion) = tokio::sync::oneshot::channel();
    let publication = async {
        queue
            .send(crate::lifecycle::DriverRequest::Fence(done))
            .await
            .map_err(|_| {
                stop.cancel();
                HostError::TargetClosed
            })?;
        completion.await.map_err(|_| {
            stop.cancel();
            HostError::TargetClosed
        })
    };
    tokio::select! {
        biased;
        () = stop.cancelled() => Err(HostError::TargetClosed),
        () = disconnected.cancelled() => Err(HostError::TargetClosed),
        () = wait_deadline(deadline) => Err(HostError::Timeout(timeout_ms)),
        result = publication => result,
    }
}

async fn wait_deadline(deadline: Option<tokio::time::Instant>) {
    match deadline {
        Some(at) => tokio::time::sleep_until(at).await,
        None => std::future::pending().await,
    }
}

async fn wait_duration(delay: Option<Duration>) {
    match delay {
        Some(delay) => tokio::time::sleep(delay).await,
        None => std::future::pending().await,
    }
}

async fn drive(
    route: &ChromiumFrameNavigationSnapshot,
    page: &PageLifecycleHandle,
    ticket: &mut NavigationWaitHandle,
    params: BeginNavigationParams,
    timeout_ms: u32,
) -> Result<FrameNavigationWait, HostError> {
    let native = route.dispatch_navigation(params);
    tokio::pin!(native);
    let mut received = false;
    loop {
        if let Some(result) = ticket
            .try_take()
            .map_err(|error| observation_error(page, error))?
        {
            return Ok(result);
        }
        let next = ticket
            .next_deadline()
            .map_err(|error| observation_error(page, error))?;
        let delay = next
            .map(|at| page.now().map(|now| at.saturating_sub(now)))
            .transpose()
            .map_err(|error| observation_error(page, error))?;
        tokio::select! {
            receipt = &mut native, if !received => {
                let receipt = receipt.map_err(|error| HostError::operation(error, timeout_ms))?;
                received = true;
                if receipt.is_download {
                    ticket.abort(NavigationWaitError::Download).map_err(|error| observation_error(page, error))?;
                } else {
                    ticket.receipt(receipt.loader_id.as_deref()).map_err(|error| observation_error(page, error))?;
                }
            }
            changed = ticket.changed() => changed.map_err(|error| observation_error(page, error))?,
            () = async {
                match delay {
                    Some(delay) => tokio::time::sleep(delay).await,
                    None => std::future::pending().await,
                }
            } => {},
        }
    }
}

fn observation_error(page: &PageLifecycleHandle, error: impl std::fmt::Display) -> HostError {
    if page.is_sealed() {
        HostError::TargetClosed
    } else {
        HostError::backend(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_core::ResourceLimits;
    use std::{
        future::Future,
        task::{Context, Waker},
    };

    #[test]
    fn executor_admission_has_a_fixed_concurrent_ceiling_and_reclaims_on_drop() {
        // Arrange
        // A larger combined plan must not weaken this feature's count bound.
        let requirements = operation_requirements()
            .combine_concurrent(operation_requirements())
            .combine_concurrent(slot_requirements());
        let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let bound = requirements.class(class);
            limits = limits.with_class(class, bound.global, bound.per_scope);
        }
        let resources = ProductionResourceLedger::new(limits, requirements).unwrap();
        let ledger = resources.claim(requirements).unwrap();
        let slots = Slots::new(&resources).unwrap();
        let mut leases: Vec<_> = (0..MAX_WAITERS)
            .map(|_| slots.admit(OperationKind::Waiter).unwrap())
            .collect();
        // Act
        let excess = slots.admit(OperationKind::Waiter);
        // Assert
        assert!(excess.is_err());
        drop(leases.pop());
        assert!(slots.admit(OperationKind::Waiter).is_ok());
        drop(slots);
        assert!(
            ledger
                .scope_metrics(ResourceScope::extension(126, 2))
                .unwrap()
                .is_some()
        );
        drop(leases);
        assert!(
            ledger
                .scope_metrics(ResourceScope::extension(126, 2))
                .unwrap()
                .is_none()
        );
    }

    #[cfg(feature = "frame-navigation-burst-20")]
    #[test]
    fn burst_profile_declares_twenty_waiters_plus_one_command_operation() {
        // Arrange
        let one = OPERATION_BYTES;

        // Act
        let declared = operation_requirements();

        // Assert
        assert_eq!(MAX_OPERATIONS, 21);
        assert_eq!(MAX_WAITERS, 20);
        assert_eq!(MAX_COMMANDS, 20);
        assert_eq!(
            declared.class(ResourceClass::BackendStateBytes).global,
            21 * one
        );
        assert_eq!(declared.leases, 21);
    }

    #[cfg(feature = "frame-navigation-burst-20")]
    #[test]
    fn twenty_navigation_waiters_reject_a_twenty_first_waiter() {
        // Arrange
        let requirements = operation_requirements().combine_concurrent(slot_requirements());
        let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let bound = requirements.class(class);
            limits = limits.with_class(class, bound.global, bound.per_scope);
        }
        let resources = ProductionResourceLedger::new(limits, requirements).unwrap();
        let slots = Slots::new(&resources).unwrap();
        let waiters: Vec<_> = (0..20)
            .map(|_| slots.admit(OperationKind::Waiter).unwrap())
            .collect();

        // Act
        let rejected_waiter = slots.admit(OperationKind::Waiter);

        // Assert
        assert!(matches!(
            rejected_waiter,
            Err(HostError::ResourceExhausted(_))
        ));
        assert_eq!(slots.active.load(Ordering::Acquire), (20_u64 << 32) + 20);
        drop(waiters);
        assert_eq!(slots.active.load(Ordering::Acquire), 0);
    }

    #[cfg(feature = "frame-navigation-burst-20")]
    #[test]
    fn twenty_navigation_waiters_leave_one_goto_command_slot() {
        // Arrange
        let requirements = operation_requirements().combine_concurrent(slot_requirements());
        let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let bound = requirements.class(class);
            limits = limits.with_class(class, bound.global, bound.per_scope);
        }
        let resources = ProductionResourceLedger::new(limits, requirements).unwrap();
        let slots = Slots::new(&resources).unwrap();
        let waiters: Vec<_> = (0..20)
            .map(|_| slots.admit(OperationKind::Waiter).unwrap())
            .collect();

        // Act
        let goto = slots.admit(OperationKind::Command);

        // Assert
        assert!(goto.is_ok());
        assert_eq!(slots.active.load(Ordering::Acquire), (20_u64 << 32) + 21);
        drop(goto);
        assert_eq!(slots.active.load(Ordering::Acquire), (20_u64 << 32) + 20);
        drop(waiters);
        assert_eq!(slots.active.load(Ordering::Acquire), 0);
    }

    #[cfg(feature = "frame-navigation-burst-20")]
    #[tokio::test]
    async fn twenty_waiter_fences_and_goto_fence_drain_through_eight_item_queue() {
        // Arrange
        let (queue, mut receiver) = tokio::sync::mpsc::channel(8);
        let stop = tokio_util::sync::CancellationToken::new();
        let disconnected = tokio_util::sync::CancellationToken::new();
        let mut fences: Vec<_> = (0..20)
            .map(|_| {
                Box::pin(send_burst_navigation_fence(
                    &queue,
                    &stop,
                    &disconnected,
                    None,
                    0,
                ))
            })
            .chain(std::iter::once(Box::pin(send_burst_navigation_fence(
                &queue,
                &stop,
                &disconnected,
                None,
                0,
            ))))
            .collect();
        let mut completed = [false; 21];
        let mut context = Context::from_waker(Waker::noop());

        // Act
        for fence in &mut fences {
            assert!(fence.as_mut().poll(&mut context).is_pending());
        }
        assert_eq!(receiver.len(), 8);
        for _ in 0..21 {
            let crate::lifecycle::DriverRequest::Fence(reply) = receiver.try_recv().unwrap() else {
                panic!("navigation publication must send only a fence");
            };
            reply.send(()).unwrap();
            for (fence, done) in fences.iter_mut().zip(&mut completed) {
                if !*done {
                    if let std::task::Poll::Ready(result) = fence.as_mut().poll(&mut context) {
                        result.unwrap();
                        *done = true;
                    }
                }
            }
        }

        // Assert
        assert!(completed.into_iter().all(|done| done));
        assert!(receiver.is_empty());
        assert!(!stop.is_cancelled());
    }

    #[cfg(feature = "frame-navigation-burst-20")]
    #[tokio::test]
    async fn stopped_navigation_fence_does_not_enqueue_after_capacity_frees() {
        // Arrange
        let (queue, mut receiver) = tokio::sync::mpsc::channel(8);
        let stop = tokio_util::sync::CancellationToken::new();
        let disconnected = tokio_util::sync::CancellationToken::new();
        let mut receipts = Vec::new();
        for _ in 0..8 {
            let (reply, receipt) = tokio::sync::oneshot::channel();
            assert!(
                queue
                    .try_send(crate::lifecycle::DriverRequest::Fence(reply))
                    .is_ok()
            );
            receipts.push(receipt);
        }
        let mut blocked = Box::pin(send_burst_navigation_fence(
            &queue,
            &stop,
            &disconnected,
            None,
            0,
        ));
        assert!(
            blocked
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );

        // Act
        stop.cancel();
        let result = blocked.await;

        // Assert
        assert!(matches!(result, Err(HostError::TargetClosed)));
        assert_eq!(receiver.len(), 8);
        assert!(matches!(
            receiver.try_recv(),
            Ok(crate::lifecycle::DriverRequest::Fence(_))
        ));
        assert_eq!(receiver.len(), 7);
        drop(receipts);
    }

    #[cfg(feature = "frame-navigation-burst-20")]
    #[tokio::test(start_paused = true)]
    async fn timed_out_navigation_fence_does_not_enqueue_after_capacity_frees() {
        // Arrange
        let (queue, mut receiver) = tokio::sync::mpsc::channel(8);
        let stop = tokio_util::sync::CancellationToken::new();
        let disconnected = tokio_util::sync::CancellationToken::new();
        for _ in 0..8 {
            let (reply, _receipt) = tokio::sync::oneshot::channel();
            assert!(
                queue
                    .try_send(crate::lifecycle::DriverRequest::Fence(reply))
                    .is_ok()
            );
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
        let mut blocked = Box::pin(send_burst_navigation_fence(
            &queue,
            &stop,
            &disconnected,
            Some(deadline),
            1_000,
        ));
        assert!(
            blocked
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );

        // Act
        tokio::time::advance(Duration::from_secs(2)).await;
        let result = blocked.await;

        // Assert
        assert!(matches!(result, Err(HostError::Timeout(1_000))));
        assert_eq!(receiver.len(), 8);
        assert!(matches!(
            receiver.try_recv(),
            Ok(crate::lifecycle::DriverRequest::Fence(_))
        ));
        assert_eq!(receiver.len(), 7);
        assert!(!stop.is_cancelled());
    }

    #[cfg(not(feature = "frame-navigation-burst-20"))]
    #[test]
    fn ordinary_navigation_profile_retains_eight_shared_operations() {
        // Arrange
        let requirements = operation_requirements().combine_concurrent(slot_requirements());
        let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let bound = requirements.class(class);
            limits = limits.with_class(class, bound.global, bound.per_scope);
        }
        let resources = ProductionResourceLedger::new(limits, requirements).unwrap();
        let slots = Slots::new(&resources).unwrap();
        let waiters: Vec<_> = (0..8)
            .map(|_| slots.admit(OperationKind::Waiter).unwrap())
            .collect();

        // Act
        let goto = slots.admit(OperationKind::Command);

        // Assert
        assert_eq!(MAX_OPERATIONS, 8);
        assert!(matches!(goto, Err(HostError::ResourceExhausted(_))));
        drop(waiters);
        assert_eq!(slots.active.load(Ordering::Acquire), 0);
    }

    #[test]
    fn short_strings_with_excess_capacity_do_not_bypass_owned_input_budget() {
        // Arrange
        let mut url = String::with_capacity(MAX_URL_BYTES + 1);
        url.push_str("about:blank");
        // Act
        let result = validate(&BeginNavigationParams::new(url));
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    }

    #[tokio::test(start_paused = true)]
    async fn absolute_deadline_includes_time_elapsed_during_preparation() {
        // Arrange
        let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
        tokio::time::advance(Duration::from_secs(2)).await;
        // Act
        let wait = wait_deadline(Some(deadline));
        tokio::pin!(wait);
        let result = wait.as_mut().poll(&mut Context::from_waker(Waker::noop()));
        // Assert
        assert!(result.is_ready());
    }

    #[tokio::test(start_paused = true)]
    async fn disabled_deadline_does_not_fire_after_elapsed_time() {
        // Arrange
        let wait = wait_deadline(None);
        tokio::pin!(wait);
        tokio::time::advance(Duration::from_secs(3600)).await;
        // Act
        let result = wait.as_mut().poll(&mut Context::from_waker(Waker::noop()));
        // Assert
        assert!(result.is_pending());
    }
}
