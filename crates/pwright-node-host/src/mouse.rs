//! Host-owned bounded Mouse invocations and independent native retirement tasks.
//! The reusable Mouse composition itself never selects an executor.
use crate::{HostError, HostSession};
use pwright_backend_cdp_mouse::{CdpMouseServices, MouseNativeFuture};
pub use pwright_cap_mouse::*;
use pwright_cdp::CdpClient;
use pwright_composition_chromium_mouse::{
    CdpInputCoordinator, CdpMouseDelay, CdpMouseProductionProfile, CdpMouseServiceFactory,
    ChromiumMouseInvocation, ChromiumMousePage, MouseRetirementDriver,
};
use pwright_core::{
    CallMetadata, CancellationReason, Command, CommandCancellation, CommandSpec, ObjectId,
    OperationContext, OperationError, ProductionResourceLedger, ResourceClass, ResourceLease,
    ResourceLedger, ResourceRequirements, ResourceScope,
};
use pwright_runtime::TargetedService;
use std::{
    num::NonZeroUsize,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

const RETAINED: usize = 16;
pub(crate) const EXECUTING: usize = 8;
pub(crate) const MAX_CLICK_COMMANDS: usize =
    pwright_input_model::INPUT_MOUSE_MAX_CLICKS as usize * 2 + 1;
// Native ownership retains the backend's EXECUTING lease until retirement.
pub(crate) const MAX_PENDING_COMMANDS: usize = EXECUTING * MAX_CLICK_COMMANDS;
const NATIVE_BYTES: usize = 32 * 1024;
const PROVIDER_FRAME_BYTES: usize = 4096;
// At most 65 native calls in a click. Pointer JSON and bounded native errors
// have a separate allowance from each measured async provider frame.
const PROVIDER_BYTES: u64 = MAX_CLICK_COMMANDS as u64 * (PROVIDER_FRAME_BYTES as u64 + 8192);
const CALL_BYTES: u64 = 16 * 1024;
const STATE_BYTES: u64 = 16 * 1024;
const STATE_SCOPE: ResourceScope = ResourceScope::extension(120, 20);

fn capacity() -> NonZeroUsize {
    NonZeroUsize::new(1).expect("positive route capacity")
}
fn backend_profile() -> Result<CdpMouseProductionProfile, HostError> {
    CdpMouseProductionProfile::new(
        ResourceScope::extension(120, 21),
        NATIVE_BYTES,
        NonZeroUsize::new(EXECUTING).expect("positive native capacity"),
    )
    .map_err(HostError::backend)
}
fn storage_requirements() -> ResourceRequirements {
    let bytes = STATE_BYTES + RETAINED as u64 * (CALL_BYTES + PROVIDER_BYTES);
    ResourceRequirements::none()
        .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
        .with_metadata(1, RETAINED + 1)
}
pub(crate) fn requirements() -> Result<ResourceRequirements, HostError> {
    let mut required = storage_requirements().combine_concurrent(backend_profile()?.requirements());
    for _ in 0..RETAINED {
        required = required.combine_concurrent(
            ChromiumMousePage::route_requirements(capacity()).map_err(HostError::backend)?,
        );
    }
    Ok(required)
}
struct HostDelay;
impl CdpMouseDelay for HostDelay {
    fn wait(self: Arc<Self>, duration: Duration) -> MouseNativeFuture {
        Box::pin(async move {
            tokio::time::sleep(duration).await;
            Ok(())
        })
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MouseMetrics {
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
    // Serializes bind/spawn with shutdown's close/wait boundary.
    status: Mutex<Status>,
    active: AtomicUsize,
    tasks: TaskTracker,
    stop: CancellationToken,
    _storage: ResourceLease,
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
// Dropped after the inner invocation so its ticket is gone before the worker
// sees completion. A runtime cleanup job retains this guard after caller drop.
struct Finished {
    signal: CancellationToken,
    _lease: Arc<CallLease>,
}
impl Drop for Finished {
    fn drop(&mut self) {
        self.signal.cancel();
    }
}
struct Invocation<C> {
    inner: ChromiumMouseInvocation<C>,
    _finished: Finished,
}
macro_rules! targeted {
    ($command:ty, $params:ty) => {
        impl TargetedService<$params> for Invocation<$command> {
            type Response = ();
            type Error = OperationError<MouseErrorDetails>;
            type Future<'a> =
                <ChromiumMouseInvocation<$command> as TargetedService<$params>>::Future<'a>;
            fn call_for(&self, page: ObjectId, params: $params) -> Self::Future<'_> {
                self.inner.call_for(page, params)
            }
        }
    };
}
targeted!(MouseMove, MoveParams);
targeted!(MouseDown, DownParams);
targeted!(MouseUp, UpParams);
targeted!(MouseClick, ClickParams);
targeted!(MouseDblclick, DblclickParams);
targeted!(MouseWheel, WheelParams);
impl<C: CommandSpec<Result = (), ErrorDetails = MouseErrorDetails>> CommandCancellation<C>
    for Invocation<C>
{
    type Future<'a>
        = <ChromiumMouseInvocation<C> as CommandCancellation<C>>::Future<'a>
    where
        Self: 'a;
    fn cancel(&self, context: OperationContext, reason: CancellationReason) -> Self::Future<'_> {
        self.inner.cancel(context, reason)
    }
}

pub(crate) struct Installation {
    #[cfg(feature = "drag-runtime")]
    drag: Option<Arc<crate::drag::host::Installation>>,
    factory: CdpMouseServiceFactory,
    resources: ProductionResourceLedger,
    ledger: ResourceLedger,
    shared: Arc<Shared>,
}
impl Drop for Installation {
    fn drop(&mut self) {
        self.shared.stop.cancel();
    }
}
impl Installation {
    #[cfg(feature = "drag-runtime")]
    pub(crate) fn drag_installation(&self) -> Option<&Arc<crate::drag::host::Installation>> {
        self.drag.as_ref()
    }
    #[cfg(feature = "drag-runtime")]
    pub(crate) fn with_drag(mut self, drag: Arc<crate::drag::host::Installation>) -> Self {
        self.drag = Some(drag);
        self
    }
    pub(crate) fn new(resources: &ProductionResourceLedger) -> Result<Self, HostError> {
        resources
            .claim(requirements()?)
            .map_err(HostError::backend)?;
        let ledger = resources
            .claim(storage_requirements())
            .map_err(HostError::backend)?;
        let storage = ledger
            .reserve(STATE_SCOPE, ResourceClass::BackendStateBytes, STATE_BYTES)
            .map_err(HostError::backend)?;
        Ok(Self {
            #[cfg(feature = "drag-runtime")]
            drag: None,
            factory: CdpMouseServiceFactory::new_with_production_resources(
                backend_profile()?,
                resources,
                Arc::new(HostDelay),
            )
            .map_err(HostError::backend)?,
            resources: resources.clone(),
            ledger,
            shared: Arc::new(Shared {
                status: Mutex::new(Status::default()),
                active: AtomicUsize::new(0),
                tasks: TaskTracker::new(),
                stop: CancellationToken::new(),
                _storage: storage,
            }),
        })
    }
    #[cfg(test)]
    fn bind<C: CommandSpec<Result = (), ErrorDetails = MouseErrorDetails>>(
        &self,
        client: Arc<dyn CdpClient>,
        input: CdpInputCoordinator,
        command: &Command<C>,
    ) -> Result<Arc<Invocation<C>>, HostError> {
        self.bind_with(command, || Ok((client, input)))
    }
    fn bind_with<C: CommandSpec<Result = (), ErrorDetails = MouseErrorDetails>>(
        &self,
        command: &Command<C>,
        binding: impl FnOnce() -> Result<(Arc<dyn CdpClient>, CdpInputCoordinator), HostError>,
    ) -> Result<Arc<Invocation<C>>, HostError> {
        let status = self.shared.status.lock().map_err(HostError::backend)?;
        if status.closed || self.shared.stop.is_cancelled() {
            return Err(HostError::TargetClosed);
        }
        if self.shared.active.load(Ordering::Acquire) >= RETAINED {
            return Err(HostError::ResourceExhausted(
                "Mouse retained invocation capacity exceeded".into(),
            ));
        }
        let storage = self
            .ledger
            .reserve(
                STATE_SCOPE,
                ResourceClass::BackendStateBytes,
                CALL_BYTES + PROVIDER_BYTES,
            )
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        self.shared.active.fetch_add(1, Ordering::AcqRel);
        let lease = Arc::new(CallLease {
            storage: Some(storage),
            shared: self.shared.clone(),
        });
        // Reserve before the retained native session/ID snapshot is allocated.
        let (client, input) = binding()?;
        // Effect-free layout checks precede any native poll. Connection buffers,
        // subscriptions and pending replies retain the host CDP ledger instead.
        let required = CdpMouseServices::required_operation_bytes(client.clone(), &input)
            .map_err(HostError::backend)?;
        if required > NATIVE_BYTES {
            return Err(HostError::ResourceExhausted(
                "Mouse native execution frame exceeds host allowance".into(),
            ));
        }
        {
            let provider = client.input_dispatch_pointer_event(Default::default());
            if std::mem::size_of_val(provider.as_ref().get_ref()) > PROVIDER_FRAME_BYTES {
                return Err(HostError::ResourceExhausted(
                    "Mouse CDP provider frame exceeds host allowance".into(),
                ));
            }
        }
        #[cfg(feature = "drag-runtime")]
        let drag = self
            .drag
            .as_ref()
            .map(|installation| installation.bind(command.object_id(), &client))
            .transpose()?;
        #[cfg(feature = "drag-runtime")]
        let (route, driver) = if let Some(drag) = &drag {
            ChromiumMousePage::bind_with_dispatcher(
                command.object_id(),
                client,
                input,
                &self.factory,
                capacity(),
                &self.resources,
                drag.dispatcher.clone(),
            )
        } else {
            ChromiumMousePage::bind(
                command.object_id(),
                client,
                input,
                &self.factory,
                capacity(),
                &self.resources,
            )
        }
        .map_err(HostError::backend)?;
        #[cfg(not(feature = "drag-runtime"))]
        let (route, driver) = ChromiumMousePage::bind(
            command.object_id(),
            client,
            input,
            &self.factory,
            capacity(),
            &self.resources,
        )
        .map_err(HostError::backend)?;
        let finished = CancellationToken::new();
        let inner = ChromiumMouseInvocation::for_command(route.clone(), command)
            .map_err(HostError::backend)?;
        let invocation = Arc::new(Invocation {
            inner,
            _finished: Finished {
                signal: finished.clone(),
                _lease: lease.clone(),
            },
        });
        let task = retire(
            route,
            driver,
            finished,
            lease,
            #[cfg(feature = "drag-runtime")]
            drag,
        );
        if std::mem::size_of_val(&task) + std::mem::size_of::<Invocation<C>>()
            > CALL_BYTES as usize / 2
        {
            return Err(HostError::ResourceExhausted(
                "Mouse retirement task frame exceeds host allowance".into(),
            ));
        }
        self.shared.tasks.spawn(task);
        drop(status);
        Ok(invocation)
    }
    pub(crate) fn metrics(&self) -> Result<MouseMetrics, HostError> {
        let state = self.shared.status.lock().map_err(HostError::backend)?;
        Ok(MouseMetrics {
            retained_invocations: self.shared.active.load(Ordering::Acquire),
            retirement_tasks: self.shared.tasks.len(),
            completed_receipts: state.completed,
            cleanup_failures: state.failed,
            closing: state.closed,
        })
    }
    pub(crate) async fn shutdown(&self) -> Result<(), HostError> {
        {
            let mut state = self.shared.status.lock().map_err(HostError::backend)?;
            state.closed = true;
            self.shared.stop.cancel();
            self.shared.tasks.close();
        }
        // No outer timeout drops native owners. Individual CDP commands have the
        // explicit connection deadline; ordinary runtime cleanup runs in parallel.
        self.shared.tasks.wait().await;
        let result = {
            let state = self.shared.status.lock().map_err(HostError::backend)?;
            match &state.first_error {
                Some(error) => Err(HostError::Backend(error.clone())),
                None => Ok(()),
            }
        };
        #[cfg(feature = "drag-runtime")]
        let result = if let Some(drag) = &self.drag {
            result.and(drag.shutdown().await)
        } else {
            result
        };
        result
    }
}
struct WorkerCompletion {
    shared: Arc<Shared>,
    complete: bool,
}
impl Drop for WorkerCompletion {
    fn drop(&mut self) {
        if !self.complete {
            let mut state = self
                .shared
                .status
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            state.failed = state.failed.saturating_add(1);
            if state.first_error.is_none() {
                state.first_error =
                    Some("Mouse retirement worker exited without completion".into());
            }
        }
    }
}
async fn retire(
    route: Arc<ChromiumMousePage>,
    mut driver: MouseRetirementDriver,
    finished: CancellationToken,
    lease: Arc<CallLease>,
    #[cfg(feature = "drag-runtime")] drag: Option<Arc<crate::drag::host::PageController>>,
) {
    let mut completion = WorkerCompletion {
        shared: lease.shared.clone(),
        complete: false,
    };
    let shared = &lease.shared;
    let mut closing = false;
    loop {
        let is_finished = finished.is_cancelled();
        if is_finished && route.queued_operations() == 0 {
            break;
        }
        tokio::select! {
            biased;
            () = shared.stop.cancelled(), if !closing => {
                closing = true;
                route.revoke_admission();
                route.request_cancel_all(CancellationReason::ParentClosed);
            }
            () = finished.cancelled(), if !is_finished => {}
            receipt = driver.next() => {
                // Always deliver the acknowledgement before recording/reporting
                // its error, so controlled cleanup cannot deadlock behind logging.
                let outcome = receipt.into_outcome();
                #[cfg(feature = "drag-runtime")]
                let drag_cleanup = if let Some(drag) = &drag { drag.drain().await } else { Ok(()) };
                let mut state = shared.status.lock().unwrap_or_else(|error| error.into_inner());
                state.completed = state.completed.saturating_add(1);
                if let Err(error) = outcome {
                    state.failed = state.failed.saturating_add(1);
                    if state.first_error.is_none() { state.first_error = Some(HostError::backend(error).to_string()); }
                }
                #[cfg(feature = "drag-runtime")]
                if let Err(error) = drag_cleanup {
                    state.failed = state.failed.saturating_add(1);
                    if state.first_error.is_none() { state.first_error = Some(error.to_string()); }
                }
            }
        }
    }
    #[cfg(feature = "drag-runtime")]
    if let Some(drag) = &drag
        && let Err(error) = drag.drain().await
    {
        let mut state = shared
            .status
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.failed = state.failed.saturating_add(1);
        if state.first_error.is_none() {
            state.first_error = Some(error.to_string());
        }
    }
    completion.complete = true;
}

impl HostSession {
    async fn mouse_invoke<C>(&self, page: &str, params: C::Params) -> Result<(), HostError>
    where
        C: CommandSpec<Result = (), ErrorDetails = MouseErrorDetails> + 'static,
        C::Params: Send + 'static,
        Invocation<C>:
            TargetedService<C::Params, Response = (), Error = OperationError<MouseErrorDetails>>,
    {
        let (page, call, _operation) = self.admit_operation(page)?;
        let (runtime, installation) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            (live.runtime.clone(), live.mouse.clone())
        };
        let command = Command::<C>::new(call, page, CallMetadata::default(), params);
        let invocation = installation.bind_with(&command, || {
            Ok(runtime
                .service()
                .page_input_binding(page)
                .map_err(HostError::backend)?
                .ok_or(HostError::TargetClosed)?
                .into_parts())
        })?;
        let dispatch = runtime.dispatch_with(command, invocation.as_ref(), invocation.clone());
        if std::mem::size_of_val(dispatch.as_ref().get_ref()) > CALL_BYTES as usize / 2 {
            return Err(HostError::ResourceExhausted(
                "Mouse controlled dispatch frame exceeds host allowance".into(),
            ));
        }
        let result = tokio::select! {
            biased;
            () = self.stop.cancelled() => return Err(HostError::TargetClosed),
            () = self.disconnected.cancelled() => return Err(HostError::TargetClosed),
            result = dispatch => result,
        };
        match result {
            Ok(result) => result
                .into_outcome()
                .map_err(|error| HostError::operation(error, 0)),
            Err(never) => match never {},
        }
    }
    pub async fn mouse_move(&self, page: &str, params: MoveParams) -> Result<(), HostError> {
        self.mouse_invoke::<MouseMove>(page, params).await
    }
    pub async fn mouse_down(&self, page: &str, params: DownParams) -> Result<(), HostError> {
        self.mouse_invoke::<MouseDown>(page, params).await
    }
    pub async fn mouse_up(&self, page: &str, params: UpParams) -> Result<(), HostError> {
        self.mouse_invoke::<MouseUp>(page, params).await
    }
    pub async fn mouse_click(&self, page: &str, params: ClickParams) -> Result<(), HostError> {
        self.mouse_invoke::<MouseClick>(page, params).await
    }
    pub async fn mouse_dblclick(
        &self,
        page: &str,
        params: DblclickParams,
    ) -> Result<(), HostError> {
        self.mouse_invoke::<MouseDblclick>(page, params).await
    }
    pub async fn mouse_wheel(&self, page: &str, params: WheelParams) -> Result<(), HostError> {
        self.mouse_invoke::<MouseWheel>(page, params).await
    }
    pub fn mouse_metrics(&self) -> Result<MouseMetrics, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        match state.as_ref() {
            Some(live) => live.mouse.metrics(),
            None => Ok(MouseMetrics {
                closing: true,
                ..MouseMetrics::default()
            }),
        }
    }
}
#[cfg(test)]
mod tests;
