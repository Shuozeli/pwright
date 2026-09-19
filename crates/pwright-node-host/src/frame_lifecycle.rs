//! Explicit lifecycle and bounded navigation-registration host selection.
//! Native dispatch/execution and public Node Frame.goto remain separate work.
use crate::{HostError, HostSession};
use pwright_backend_cdp_navigation::frame_lifecycle::native::NativeLifecycleNotice;
#[cfg(feature = "frame-navigation")]
use pwright_backend_cdp_navigation::frame_lifecycle::navigation::FrameNavigationWaitProfile;
#[cfg(feature = "frame-navigation")]
use pwright_cap_navigation::WaitUntil;
#[cfg(feature = "frame-navigation")]
use pwright_composition_chromium::frame_lifecycle::{
    NavigationRegistrationProfile, NavigationWaitHandle,
};
use pwright_composition_chromium::{
    FrameObservationResult,
    frame_lifecycle::{
        FrameLifecycleClock, FrameLifecycleNoticeSink, FrameLifecycleRegistry,
        FrameLifecycleRegistryProfile, PageLifecycleHandle,
    },
};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};
use std::{
    mem::size_of,
    sync::Arc,
    time::{Duration, Instant},
};

// One admitted allocation is shared through the injected clock and sink ports.
// The external sink retains no envelope. Explicitly registered navigation
// owners consume ordered notices internally under the Page lifecycle lock.
struct LifecycleSupport {
    origin: Instant,
    _lease: ResourceLease,
}
impl FrameLifecycleClock for LifecycleSupport {
    fn now(&self) -> Duration {
        self.origin.elapsed()
    }
}
impl FrameLifecycleNoticeSink for LifecycleSupport {
    fn observe(&self, _notice: NativeLifecycleNotice<'_>) -> FrameObservationResult {
        Ok(())
    }
}
fn profile() -> Result<FrameLifecycleRegistryProfile, HostError> {
    // Matches 16 active host Pages, with another 16 slots for escaped retired
    // owners. Each Page has bounded Frame/request/ID storage, allocated on admit.
    let profile = FrameLifecycleRegistryProfile::new(123, 16, 32, 64, 256, 256)
        .map_err(HostError::backend)?;
    #[cfg(feature = "frame-navigation")]
    let profile = profile
        .with_navigation_slots(4)
        .map_err(HostError::backend)?;
    Ok(profile)
}
#[cfg(feature = "frame-navigation")]
pub(crate) fn registration_profile() -> Result<NavigationRegistrationProfile, HostError> {
    Ok(NavigationRegistrationProfile::new(
        FrameNavigationWaitProfile::new(
            ResourceScope::extension(125, 0),
            16,
            8,
            256,
            1024,
            64 * 1024,
        )
        .map_err(HostError::backend)?,
        ResourceScope::extension(125, 1),
    ))
}
fn support_requirements() -> ResourceRequirements {
    let bytes = (size_of::<LifecycleSupport>() + 2 * size_of::<usize>() + 256) as u64;
    ResourceRequirements::none()
        .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
        .with_metadata(1, 1)
}
pub(crate) fn requirements() -> Result<ResourceRequirements, HostError> {
    let requirements = profile()?
        .requirements()
        .combine_concurrent(support_requirements());
    // 32 active-or-escaped Page owners, each with four registration slots.
    // Escaped moved-out results remain charged and can exhaust this ceiling;
    // they do not create permission to grow beyond it after slot reclamation.
    #[cfg(feature = "frame-navigation")]
    let requirements = {
        let registration = registration_profile()?.requirements();
        (0..128).fold(requirements, |all, _| all.combine_concurrent(registration))
    };
    Ok(requirements)
}
pub(crate) fn install(
    resources: &ProductionResourceLedger,
    epoch: u64,
) -> Result<Arc<FrameLifecycleRegistry>, HostError> {
    let requirement = support_requirements();
    let lease = resources
        .claim(requirement)
        .map_err(HostError::backend)?
        .reserve(
            ResourceScope::extension(124, 0),
            ResourceClass::BackendStateBytes,
            requirement.class(ResourceClass::BackendStateBytes).global,
        )
        .map_err(HostError::backend)?;
    let support = Arc::new(LifecycleSupport {
        origin: Instant::now(),
        _lease: lease,
    });
    FrameLifecycleRegistry::new(profile()?, epoch, resources, support.clone(), support)
        .map(Arc::new)
        .map_err(HostError::backend)
}
impl HostSession {
    /// Rust-only registration, not Frame.goto: no native command is sent here.
    /// Timeout zero/None disables the deadline. The executor must dispatch the
    /// exact native route, deliver its receipt, and drive notified/deadline waits.
    #[cfg(feature = "frame-navigation")]
    pub async fn register_frame_navigation(
        &self,
        page_id: &str,
        frame_id: &str,
        wait: WaitUntil,
        timeout: Option<Duration>,
    ) -> Result<NavigationWaitHandle, HostError> {
        let frame = frame_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Frame ID".into()))?;
        let page = self.frame_lifecycle(page_id).await?;
        let now = page.now().map_err(HostError::backend)?;
        let deadline = timeout
            .filter(|timeout| !timeout.is_zero())
            .map(|timeout| {
                now.checked_add(timeout).ok_or_else(|| {
                    HostError::InvalidArgument("navigation deadline overflow".into())
                })
            })
            .transpose()?;
        page.register_navigation(
            registration_profile()?,
            &self.resources,
            frame,
            wait,
            deadline,
        )
        .map_err(HostError::backend)
    }
    /// Rust-only, opt-in readiness handle. Establishes the existing host
    /// publication fence first; this does not initiate or await navigation.
    pub async fn frame_lifecycle(&self, page_id: &str) -> Result<PageLifecycleHandle, HostError> {
        let page = page_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Page ID".into()))?;
        self.flush_topology().await?;
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        live.frame_lifecycle
            .page(page)
            .map_err(HostError::backend)?
            .ok_or(HostError::TargetClosed)
    }
}
