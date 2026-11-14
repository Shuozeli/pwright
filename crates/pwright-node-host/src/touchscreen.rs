//! Host-owned bounded Touchscreen invocations and independent native retirement.

use crate::{HostError, HostSession};
use pwright_backend_cdp_input::{CdpInputCoordinator, CdpModifiers};
use pwright_backend_cdp_touchscreen::{
    CdpTouchscreenProductionProfile, CdpTouchscreenServiceFactory,
};
use pwright_cap_locator::LocatorErrorDetails;
pub use pwright_cap_touchscreen::{
    SelectorTapOptions, SelectorTapParams, TapParams, TouchscreenCoordinate,
};
use pwright_cdp::CdpClient;
use pwright_composition_chromium::CdpPageServices;
use pwright_composition_chromium_touchscreen::ChromiumTouchscreenPage;
use pwright_core::{
    ObjectId, OperationError, ProductionResourceLedger, ResourceClass, ResourceLease,
    ResourceLedger, ResourceRequirements, ResourceScope,
};
use std::{
    num::NonZeroUsize,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::sync::oneshot;
use tokio_util::{sync::CancellationToken, task::TaskTracker};

const RETAINED: usize = 8;
const OPERATION_BYTES: usize = 16 * 1024;
const STATE_BYTES: u64 = 8 * 1024;
const CALL_BYTES: u64 = 12 * 1024;
const STATE_SCOPE: ResourceScope = ResourceScope::extension(120, 29);
const BACKEND_SCOPE: ResourceScope = ResourceScope::extension(120, 30);

fn backend_profile() -> Result<CdpTouchscreenProductionProfile, HostError> {
    CdpTouchscreenProductionProfile::new(
        BACKEND_SCOPE,
        OPERATION_BYTES,
        NonZeroUsize::new(RETAINED).expect("positive Touchscreen capacity"),
    )
    .map_err(HostError::backend)
}

fn storage_requirements() -> ResourceRequirements {
    let bytes = STATE_BYTES + RETAINED as u64 * CALL_BYTES;
    ResourceRequirements::none()
        .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
        .with_metadata(1, RETAINED + 1)
}

pub(crate) fn requirements() -> Result<ResourceRequirements, HostError> {
    Ok(storage_requirements().combine_concurrent(backend_profile()?.requirements()))
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TouchscreenMetrics {
    pub retained_invocations: usize,
    pub retirement_tasks: usize,
    pub completed_receipts: u64,
    pub cleanup_failures: u64,
    pub closing: bool,
}

#[derive(Default)]
struct Status {
    closed: bool,
    completed: u64,
    failed: u64,
    first_error: Option<String>,
}

struct Shared {
    status: Mutex<Status>,
    active: AtomicUsize,
    tasks: TaskTracker,
    _state: ResourceLease,
}

struct CallLease {
    storage: Option<ResourceLease>,
    shared: Arc<Shared>,
}

impl Drop for CallLease {
    fn drop(&mut self) {
        drop(self.storage.take());
        let previous = self.shared.active.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0);
    }
}

pub(crate) struct Installation {
    factory: CdpTouchscreenServiceFactory,
    ledger: ResourceLedger,
    shared: Arc<Shared>,
}

impl Installation {
    pub(crate) fn new(resources: &ProductionResourceLedger) -> Result<Self, HostError> {
        resources
            .claim(requirements()?)
            .map_err(HostError::backend)?;
        let ledger = resources
            .claim(storage_requirements())
            .map_err(HostError::backend)?;
        let state = ledger
            .reserve(STATE_SCOPE, ResourceClass::BackendStateBytes, STATE_BYTES)
            .map_err(HostError::backend)?;
        Ok(Self {
            factory: CdpTouchscreenServiceFactory::new_with_production_resources(
                backend_profile()?,
                resources,
            )
            .map_err(HostError::backend)?,
            ledger,
            shared: Arc::new(Shared {
                status: Mutex::new(Status::default()),
                active: AtomicUsize::new(0),
                tasks: TaskTracker::new(),
                _state: state,
            }),
        })
    }

    fn reserve_call(&self) -> Result<Arc<CallLease>, HostError> {
        let lease = {
            let status = self.shared.status.lock().map_err(HostError::backend)?;
            if status.closed {
                return Err(HostError::TargetClosed);
            }
            if self.shared.active.load(Ordering::Acquire) >= RETAINED {
                return Err(HostError::ResourceExhausted(
                    "Touchscreen retained invocation capacity exceeded".into(),
                ));
            }
            let storage = self
                .ledger
                .reserve(STATE_SCOPE, ResourceClass::BackendStateBytes, CALL_BYTES)
                .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
            self.shared.active.fetch_add(1, Ordering::AcqRel);
            Arc::new(CallLease {
                storage: Some(storage),
                shared: self.shared.clone(),
            })
        };
        Ok(lease)
    }

    async fn tap(
        &self,
        page_id: ObjectId,
        client: Arc<dyn CdpClient>,
        input: CdpInputCoordinator,
        params: TapParams,
    ) -> Result<(), HostError> {
        self.tap_with_modifiers(page_id, client, input, params, None)
            .await
    }

    pub(super) async fn tap_with_modifiers(
        &self,
        page_id: ObjectId,
        client: Arc<dyn CdpClient>,
        input: CdpInputCoordinator,
        params: TapParams,
        modifiers: Option<CdpModifiers>,
    ) -> Result<(), HostError> {
        let lease = self.reserve_call()?;
        let page = ChromiumTouchscreenPage::bind(page_id, client, input, &self.factory)
            .map_err(|error| HostError::operation(error, 0))?;
        let mut operation = page
            .start_with_modifiers(params, modifiers)
            .map_err(|error| HostError::operation(error, 0))?;
        let (sender, receiver) = oneshot::channel();
        let shared = self.shared.clone();
        self.shared.tasks.spawn(async move {
            let outcome = operation.result().await;
            let _ = sender.send(outcome);
            let cleanup = operation.retire().await;
            let mut status = shared
                .status
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            status.completed = status.completed.saturating_add(1);
            if let Err(error) = cleanup {
                status.failed = status.failed.saturating_add(1);
                if status.first_error.is_none() {
                    status.first_error = Some(error.to_string());
                }
            }
            drop(status);
            drop(lease);
        });
        receiver
            .await
            .map_err(|_| HostError::TargetClosed)?
            .map_err(|error| HostError::operation(error, 0))
    }

    async fn selector_tap(
        &self,
        services: CdpPageServices,
        frame_id: ObjectId,
        params: SelectorTapParams,
        timeout_ms: u32,
        stop: CancellationToken,
        map_error: fn(OperationError<LocatorErrorDetails>, u32) -> HostError,
    ) -> Result<(), HostError> {
        let lease = self.reserve_call()?;
        let timeout =
            (timeout_ms != 0).then(|| std::time::Duration::from_millis(u64::from(timeout_ms)));
        let trial = params.options.trial.unwrap_or(false);
        let (sender, receiver) = oneshot::channel();
        let shared = self.shared.clone();
        self.shared.tasks.spawn(async move {
            let mut action = Box::pin(services.selector_tap_for(frame_id, params, timeout));
            let (outcome, cleanup_failure) = tokio::select! {
                outcome = &mut action => (outcome.map_err(|error| map_error(error, timeout_ms)), None),
                () = stop.cancelled() => {
                    drop(action);
                    match services.cancel_selector_tap_for(frame_id, trial).await {
                        Ok(()) => (Err(HostError::TargetClosed), None),
                        Err(error) => {
                            let message = error.to_string();
                            (Err(map_error(error, timeout_ms)), Some(message))
                        }
                    }
                }
            };
            let mut status = shared
                .status
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            status.completed = status.completed.saturating_add(1);
            if let Some(error) = cleanup_failure {
                status.failed = status.failed.saturating_add(1);
                if status.first_error.is_none() {
                    status.first_error = Some(error);
                }
            }
            drop(status);
            drop(lease);
            let _ = sender.send(outcome);
        });
        receiver.await.map_err(|_| HostError::TargetClosed)?
    }

    pub(crate) fn metrics(&self) -> Result<TouchscreenMetrics, HostError> {
        let status = self.shared.status.lock().map_err(HostError::backend)?;
        Ok(TouchscreenMetrics {
            retained_invocations: self.shared.active.load(Ordering::Acquire),
            retirement_tasks: self.shared.tasks.len(),
            completed_receipts: status.completed,
            cleanup_failures: status.failed,
            closing: status.closed,
        })
    }

    pub(crate) async fn shutdown(&self) -> Result<(), HostError> {
        {
            let mut status = self.shared.status.lock().map_err(HostError::backend)?;
            status.closed = true;
            self.shared.tasks.close();
        }
        self.shared.tasks.wait().await;
        let status = self.shared.status.lock().map_err(HostError::backend)?;
        match &status.first_error {
            Some(error) => Err(HostError::Backend(error.clone())),
            None => Ok(()),
        }
    }
}

fn page_has_touch(live: &crate::LiveSession, page_id: ObjectId) -> Result<bool, HostError> {
    let topology = live.composition.topology().map_err(HostError::backend)?;
    let topology = topology.topology();
    let page_route = topology
        .page_session_routes()
        .find(|route| route.object_id() == page_id)
        .ok_or(HostError::TargetClosed)?;
    let context = page_route.browser_context_object_id();
    let ownership = topology
        .browser_context_route(context)
        .map(|route| route.ownership())
        .ok_or(HostError::TargetClosed)?;
    live.device_emulation
        .settings(context, ownership)
        .map(|settings| settings.is_some_and(|settings| settings.has_touch()))
}

impl HostSession {
    pub async fn touchscreen_tap(&self, page: &str, params: TapParams) -> Result<(), HostError> {
        let (page_id, _call, _operation) = self.admit_operation(page)?;
        let (runtime, installation) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            if !page_has_touch(live, page_id)? {
                return Err(HostError::UnsupportedOperation(
                    "hasTouch must be enabled on the browser context before using the touchscreen."
                        .into(),
                ));
            }
            (live.runtime.clone(), live.touchscreen.clone())
        };
        let (client, input) = runtime
            .service()
            .page_input_binding(page_id)
            .map_err(HostError::backend)?
            .ok_or(HostError::TargetClosed)?
            .into_parts();
        installation.tap(page_id, client, input, params).await
    }

    pub(crate) async fn locator_tap(
        &self,
        frame: &str,
        params: SelectorTapParams,
        timeout_ms: u32,
        map_error: fn(OperationError<LocatorErrorDetails>, u32) -> HostError,
    ) -> Result<(), HostError> {
        let (frame_id, _call, _operation) = self.admit_operation(frame)?;
        let (services, installation) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            // Resolve the owning Page in a separate scope. `page_has_touch`
            // takes the same non-reentrant topology mutex, so retaining this
            // guard across that call would deadlock every selector tap.
            let page_id = {
                let topology = live.composition.topology().map_err(HostError::backend)?;
                topology
                    .topology()
                    .page_object_id_for_frame(frame_id)
                    .ok_or(HostError::TargetClosed)?
            };
            if !page_has_touch(live, page_id)? {
                return Err(HostError::UnsupportedOperation(
                    "The page does not support tap. Use hasTouch context option to enable touch support."
                        .into(),
                ));
            }
            (live.runtime.service().clone(), live.touchscreen.clone())
        };
        installation
            .selector_tap(
                services,
                frame_id,
                params,
                timeout_ms,
                self.stop.clone(),
                map_error,
            )
            .await
    }

    #[cfg(feature = "element-handle-dom")]
    pub(super) async fn element_handle_tap_at(
        &self,
        page_id: ObjectId,
        params: TapParams,
        modifiers: Option<CdpModifiers>,
    ) -> Result<(), HostError> {
        let page = page_id.get().to_string();
        let (_page_id, _call, _operation) = self.admit_operation(&page)?;
        let (runtime, installation) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            if !page_has_touch(live, page_id)? {
                return Err(HostError::UnsupportedOperation(
                    "The page does not support tap. Use hasTouch context option to enable touch support."
                        .into(),
                ));
            }
            (live.runtime.clone(), live.touchscreen.clone())
        };
        let (client, input) = runtime
            .service()
            .page_input_binding(page_id)
            .map_err(HostError::backend)?
            .ok_or(HostError::TargetClosed)?
            .into_parts();
        installation
            .tap_with_modifiers(page_id, client, input, params, modifiers)
            .await
    }

    pub fn touchscreen_metrics(&self) -> Result<TouchscreenMetrics, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        match state.as_ref() {
            Some(live) => live.touchscreen.metrics(),
            None => Ok(TouchscreenMetrics {
                closing: true,
                ..TouchscreenMetrics::default()
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_backend_cdp_touchscreen::CdpTouchscreenService;

    #[test]
    fn fixed_host_and_backend_frames_fit_declared_allowances() {
        // Arrange
        let fixed = std::mem::size_of::<Installation>()
            + std::mem::size_of::<Shared>()
            + 8 * std::mem::size_of::<usize>();
        let call = std::mem::size_of::<CallLease>()
            + std::mem::size_of::<CdpTouchscreenService>()
            + std::mem::size_of::<CdpPageServices>()
            + std::mem::size_of::<SelectorTapParams>()
            + 8 * std::mem::size_of::<usize>();

        // Act
        let within_state = fixed < STATE_BYTES as usize;
        let within_call = call < CALL_BYTES as usize;

        // Assert
        assert!(within_state);
        assert!(within_call);
        const {
            assert!(
                OPERATION_BYTES > 2 * pwright_backend_cdp_touchscreen::TOUCHSCREEN_ERROR_MAX_BYTES
            );
        }
    }
}
