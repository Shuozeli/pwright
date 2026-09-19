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
    ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements, ResourceScope,
};
pub use result::FrameGotoOutput;
use std::{
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

const MAX_OPERATIONS: u64 = 8;
const MAX_URL_BYTES: usize = 16 * 1024;
// Two caller-owned strings (bounded capacity, not just length), executor/Select
// state and timer storage. Native command copies and response owners have their
// own profiles. The embedding task's stack/allocator metadata remain external.
const OPERATION_BYTES: u64 = 2 * MAX_URL_BYTES as u64 + 16 * 1024;
const OPERATION_SCOPE: ResourceScope = ResourceScope::extension(126, 1);

struct Slots {
    active: AtomicU64,
    _lease: ResourceLease,
}
struct OperationSlot(Arc<Slots>);
impl Drop for OperationSlot {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::AcqRel);
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
    fn admit(self: &Arc<Self>) -> Result<OperationSlot, HostError> {
        self.active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < MAX_OPERATIONS).then_some(active + 1)
            })
            .map_err(|_| {
                HostError::ResourceExhausted("Frame navigation operation capacity exhausted".into())
            })?;
        Ok(OperationSlot(self.clone()))
    }
}
pub(crate) struct Installation {
    routes: ChromiumFrameNavigationAccess,
    slots: Arc<Slots>,
    results: Arc<result::ResultSlots>,
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
        self.navigate_frame_native_with(
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
        .await
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
                .admit()?
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
                self.flush_topology(),
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

async fn wait_deadline(deadline: Option<tokio::time::Instant>) {
    match deadline {
        Some(at) => tokio::time::sleep_until(at).await,
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
        let mut leases: Vec<_> = (0..MAX_OPERATIONS)
            .map(|_| slots.admit().unwrap())
            .collect();
        // Act
        let excess = slots.admit();
        // Assert
        assert!(excess.is_err());
        drop(leases.pop());
        assert!(slots.admit().is_ok());
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
