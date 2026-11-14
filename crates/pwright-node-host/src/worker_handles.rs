//! Optional retained JavaScript Handles owned by dedicated Workers.
//!
//! Worker Handles deliberately do not enter the Frame/main-world portable
//! route. They reuse the bounded CDP Handle backend while retaining the exact
//! Worker owner, execution context and cleanup reservation in Rust.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak};

use pwright_backend_cdp_evaluation::{CdpEvaluationLimits, CdpEvaluationService};
#[cfg(any(feature = "worker-console-argument-handles", test))]
use pwright_backend_cdp_handle_evaluation::CdpHandleEvaluationLimits;
use pwright_backend_cdp_handle_evaluation::{
    CdpEvaluatedHandle, CdpHandleEvaluationRequest, CdpHandleEvaluationService, CdpHandleKind,
    CdpHandlePropertiesService, CdpHandlePropertyLimits, CdpHandleValueEvaluationService,
};
use pwright_cap_handle_evaluation::{EvaluateHandleParams, HandleEvaluationErrorDetails};
use pwright_cdp::connection::CdpError;
use pwright_core::{
    ObjectId, OperationError, ProductionResourceLedger, ResourceClass, ResourceLease,
    ResourceLedger, ResourceRequest, ResourceRequirements, ResourceScope, Service,
};
use pwright_runtime::{
    CleanupFailure, CleanupReservation, CleanupSubmitError, ReservedCleanupExecutor,
};
use pwright_runtime_tokio::{TokioCleanupExecutor, TokioCleanupReservation};
#[cfg(feature = "worker-console-argument-handles")]
use serde_json::Value;
use tokio::sync::{RwLock, watch};

use crate::evaluation::{ERROR_BYTES, FFI_ALLOWANCE, GENERATED_SCRIPT_BYTES, SOURCE_BYTES};
use crate::worker_events::WorkerOwner;
use crate::{HostError, HostSession};

pub const MAX_WORKER_HANDLES: usize = 64;
pub const MAX_CONCURRENT_WORKER_HANDLE_CALLS: usize = 8;
pub(crate) const CLEANUP_SLOTS: usize = MAX_WORKER_HANDLES;
const MAX_NATIVE_ID_BYTES: usize = 1024;
const MAX_PREVIEW_BYTES: usize = 1024;
const MAX_PROPERTIES: usize = 1024;
const MAX_PROPERTY_NAME_BYTES: usize = 16 * 1024;
const INSTALLATION_BYTES: u64 = 16 * 1024;
const OWNER_BYTES: u64 = 16 * 1024;
const TEMPORARY_NATIVE_HANDLES_PER_CALL: u64 = 3;
const INSTALLATION_SCOPE: ResourceScope = ResourceScope::extension(184, 1);
const OWNER_SCOPE: ResourceScope = ResourceScope::extension(184, 2);
const CALL_SCOPE: ResourceScope = ResourceScope::extension(184, 3);

pub(crate) const fn requirements() -> ResourceRequirements {
    ResourceRequirements::none()
        .with_class(
            ResourceClass::BackendStateBytes,
            INSTALLATION_BYTES
                + MAX_WORKER_HANDLES as u64 * OWNER_BYTES
                + MAX_CONCURRENT_WORKER_HANDLE_CALLS as u64 * FFI_ALLOWANCE,
            MAX_CONCURRENT_WORKER_HANDLE_CALLS as u64 * FFI_ALLOWANCE,
        )
        .with_class(
            ResourceClass::RemoteHandle,
            MAX_WORKER_HANDLES as u64
                + MAX_CONCURRENT_WORKER_HANDLE_CALLS as u64 * TEMPORARY_NATIVE_HANDLES_PER_CALL,
            MAX_WORKER_HANDLES as u64,
        )
        .with_metadata(
            3,
            1 + MAX_WORKER_HANDLES + MAX_CONCURRENT_WORKER_HANDLE_CALLS,
        )
}

struct State {
    closed: bool,
    next_id: u64,
    handles: BTreeMap<u64, Weak<HandleCore>>,
}

pub(crate) struct Installation {
    resources: ResourceLedger,
    cleanup: TokioCleanupExecutor,
    state: Mutex<State>,
    active_calls: AtomicUsize,
    _storage: ResourceLease,
}

struct HandleAdmission {
    id: u64,
    owner: ResourceLease,
    cleanup: TokioCleanupReservation,
}

struct CallSlot<'a> {
    installation: &'a Installation,
    _resources: ResourceLease,
}

impl Drop for CallSlot<'_> {
    fn drop(&mut self) {
        let previous = self
            .installation
            .active_calls
            .fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0);
    }
}

#[derive(Clone)]
pub struct WorkerHandle {
    core: Arc<HandleCore>,
}

impl std::fmt::Debug for WorkerHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkerHandle")
            .field("id", &self.core.id)
            .field("kind", &self.core.kind)
            .field("preview", &self.core.preview)
            .finish_non_exhaustive()
    }
}

impl WorkerHandle {
    pub fn is_element(&self) -> bool {
        matches!(self.core.kind, CdpHandleKind::Element)
    }

    pub fn preview(&self) -> &str {
        &self.core.preview
    }

    pub fn request_dispose(&self) -> WorkerHandleRelease {
        self.core.request_dispose()
    }
}

#[derive(Clone)]
pub struct WorkerHandleRelease {
    completion: watch::Receiver<Option<Result<(), CleanupFailure>>>,
}

impl WorkerHandleRelease {
    pub async fn wait(mut self) -> Result<(), HostError> {
        loop {
            if let Some(result) = self.completion.borrow_and_update().clone() {
                return result.map_err(HostError::backend);
            }
            self.completion.changed().await.map_err(|_| {
                HostError::Backend("Worker Handle cleanup ended without a result".into())
            })?;
        }
    }
}

struct ReleaseState {
    reservation: Option<TokioCleanupReservation>,
    completion: watch::Sender<Option<Result<(), CleanupFailure>>>,
}

struct HandleCore {
    id: u64,
    owner: Weak<WorkerOwner>,
    installation: Weak<Installation>,
    native: Arc<RwLock<Option<CdpEvaluatedHandle>>>,
    kind: CdpHandleKind,
    preview: Box<str>,
    release: Mutex<ReleaseState>,
    _owner: ResourceLease,
}

impl HandleCore {
    fn request_dispose(self: &Arc<Self>) -> WorkerHandleRelease {
        let (reservation, completion) = {
            let mut release = self
                .release
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            (release.reservation.take(), release.completion.subscribe())
        };
        if let Some(installation) = self.installation.upgrade() {
            installation.remove(self.id);
        }
        if let Some(reservation) = reservation {
            let core = Arc::clone(self);
            reservation.submit(Box::pin(async move {
                let result = core.release_native().await;
                core.release
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .completion
                    .send_replace(Some(result.clone()));
                result
            }));
        }
        WorkerHandleRelease { completion }
    }

    async fn release_native(&self) -> Result<(), CleanupFailure> {
        let Some(native) = self.native.write().await.take() else {
            return Ok(());
        };
        let owner_closed = match self.owner.upgrade() {
            Some(owner) => owner
                .is_closed()
                .map_err(|error| CleanupFailure::new(error.to_string()))?,
            None => true,
        };
        if owner_closed {
            // Target.detachedFromTarget is exact proof that Chromium retired
            // every RemoteObject in this Worker session. Sending a command to
            // that detached session would create an unacknowledgeable pending
            // command during host shutdown.
            return Ok(());
        }
        let Some(object_id) = native.object_id() else {
            return Ok(());
        };
        match native.client().runtime_release_object(object_id).await {
            Ok(())
            | Err(CdpError::Closed)
            | Err(CdpError::SessionDetached)
            | Err(CdpError::Protocol { .. }) => Ok(()),
            Err(error) => Err(CleanupFailure::new(format!(
                "Worker Handle release failed: {error}"
            ))),
        }
    }
}

impl Drop for HandleCore {
    fn drop(&mut self) {
        let release = self
            .release
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(reservation) = release.reservation.take() else {
            return;
        };
        let native = Arc::get_mut(&mut self.native).and_then(|native| native.get_mut().take());
        let owner_closed = self
            .owner
            .upgrade()
            .map(|owner| owner.is_closed().unwrap_or(false))
            .unwrap_or(true);
        let completion = release.completion.clone();
        reservation.submit(Box::pin(async move {
            let result = match native {
                Some(_) if owner_closed => Ok(()),
                Some(native) if native.object_id().is_some() => {
                    let object_id = native.object_id().expect("checked object ID").to_owned();
                    match native.client().runtime_release_object(&object_id).await {
                        Ok(())
                        | Err(CdpError::Closed)
                        | Err(CdpError::SessionDetached)
                        | Err(CdpError::Protocol { .. }) => Ok(()),
                        Err(error) => Err(CleanupFailure::new(format!(
                            "Worker Handle release failed: {error}"
                        ))),
                    }
                }
                _ => Ok(()),
            };
            completion.send_replace(Some(result.clone()));
            result
        }));
    }
}

impl Installation {
    pub(crate) fn new(
        resources: &ProductionResourceLedger,
        cleanup: TokioCleanupExecutor,
    ) -> Result<Arc<Self>, HostError> {
        if std::mem::size_of::<HandleCore>() + MAX_NATIVE_ID_BYTES + MAX_PREVIEW_BYTES + 4096
            > OWNER_BYTES as usize
        {
            return Err(HostError::ResourceExhausted(
                "Worker Handle owner storage exceeds its budget".into(),
            ));
        }
        let ledger = resources
            .claim(requirements())
            .map_err(HostError::backend)?;
        let storage = ledger
            .reserve(
                INSTALLATION_SCOPE,
                ResourceClass::BackendStateBytes,
                INSTALLATION_BYTES,
            )
            .map_err(HostError::backend)?;
        Ok(Arc::new(Self {
            resources: ledger,
            cleanup,
            state: Mutex::new(State {
                closed: false,
                next_id: 1,
                handles: BTreeMap::new(),
            }),
            active_calls: AtomicUsize::new(0),
            _storage: storage,
        }))
    }

    fn admit_handle(&self) -> Result<HandleAdmission, HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        state.handles.retain(|_, handle| handle.strong_count() > 0);
        if state.handles.len() >= MAX_WORKER_HANDLES {
            return Err(HostError::ResourceExhausted(format!(
                "Worker Handle capacity exceeded ({MAX_WORKER_HANDLES})"
            )));
        }
        let id = state.next_id;
        state.next_id = id.checked_add(1).ok_or_else(|| {
            HostError::ResourceExhausted("Worker Handle ID space exhausted".into())
        })?;
        let owner = self
            .resources
            .reserve_many(
                OWNER_SCOPE,
                &[
                    ResourceRequest::new(ResourceClass::BackendStateBytes, OWNER_BYTES),
                    ResourceRequest::new(ResourceClass::RemoteHandle, 1),
                ],
            )
            .map_err(|error| {
                HostError::ResourceExhausted(format!(
                    "Worker Handle capacity exceeded ({MAX_WORKER_HANDLES}): {error}"
                ))
            })?;
        let cleanup = self
            .cleanup
            .try_reserve()
            .map_err(cleanup_admission_error)?;
        Ok(HandleAdmission { id, owner, cleanup })
    }

    fn call_slot(&self) -> Result<CallSlot<'_>, HostError> {
        self.active_calls
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < MAX_CONCURRENT_WORKER_HANDLE_CALLS).then_some(active + 1)
            })
            .map_err(|_| {
                HostError::ResourceExhausted(format!(
                    "Worker Handle call concurrency exceeded ({MAX_CONCURRENT_WORKER_HANDLE_CALLS})"
                ))
            })?;
        let resources = self.resources.reserve_many(
            CALL_SCOPE,
            &[
                ResourceRequest::new(ResourceClass::BackendStateBytes, FFI_ALLOWANCE),
                ResourceRequest::new(
                    ResourceClass::RemoteHandle,
                    TEMPORARY_NATIVE_HANDLES_PER_CALL,
                ),
            ],
        );
        match resources {
            Ok(resources) => Ok(CallSlot {
                installation: self,
                _resources: resources,
            }),
            Err(error) => {
                self.active_calls.fetch_sub(1, Ordering::AcqRel);
                Err(HostError::ResourceExhausted(error.to_string()))
            }
        }
    }

    fn retain_many(
        self: &Arc<Self>,
        owner: Arc<WorkerOwner>,
        values: Vec<(CdpEvaluatedHandle, HandleAdmission)>,
    ) -> Result<Vec<WorkerHandle>, HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        let mut handles = Vec::with_capacity(values.len());
        for (native, admission) in values {
            let kind = native.kind();
            let preview: Box<str> = native.preview().into();
            let (completion, _) = watch::channel(None);
            let core = Arc::new(HandleCore {
                id: admission.id,
                owner: Arc::downgrade(&owner),
                installation: Arc::downgrade(self),
                native: Arc::new(RwLock::new(Some(native))),
                kind,
                preview,
                release: Mutex::new(ReleaseState {
                    reservation: Some(admission.cleanup),
                    completion,
                }),
                _owner: admission.owner,
            });
            state.handles.insert(core.id, Arc::downgrade(&core));
            handles.push(WorkerHandle { core });
        }
        Ok(handles)
    }

    fn retain(
        self: &Arc<Self>,
        owner: Arc<WorkerOwner>,
        native: CdpEvaluatedHandle,
        admission: HandleAdmission,
    ) -> Result<WorkerHandle, HostError> {
        Ok(self
            .retain_many(owner, vec![(native, admission)])?
            .pop()
            .expect("one retained Worker Handle"))
    }

    fn remove(&self, id: u64) {
        if let Ok(mut state) = self.state.lock() {
            state.handles.remove(&id);
        }
    }

    fn evaluation(owner: &WorkerOwner, unique_context_id: &str) -> CdpEvaluationService {
        CdpEvaluationService::with_limits_in_unique_context(
            owner.client(),
            unique_context_id,
            CdpEvaluationLimits::new(
                SOURCE_BYTES,
                GENERATED_SCRIPT_BYTES,
                ERROR_BYTES,
                crate::evaluation::value_limits(),
            ),
        )
    }

    #[cfg(any(feature = "worker-console-argument-handles", test))]
    fn handle_limits() -> CdpHandleEvaluationLimits {
        CdpHandleEvaluationLimits::new(
            CdpEvaluationLimits::new(
                SOURCE_BYTES,
                GENERATED_SCRIPT_BYTES,
                ERROR_BYTES,
                crate::evaluation::value_limits(),
            ),
            MAX_NATIVE_ID_BYTES,
            MAX_PREVIEW_BYTES,
        )
    }

    #[cfg(feature = "worker-console-argument-handles")]
    pub(crate) fn adopt_event_handles(
        self: &Arc<Self>,
        owner: Arc<WorkerOwner>,
        arguments: &[Value],
    ) -> Result<Vec<WorkerHandle>, HostError> {
        if owner.is_closed()? {
            return Err(HostError::TargetClosed);
        }
        let admissions = (0..arguments.len())
            .map(|_| self.admit_handle())
            .collect::<Result<Vec<_>, _>>()?;
        let client = owner.client();
        let limits = Self::handle_limits();
        let values = arguments
            .iter()
            .zip(admissions)
            .map(|(remote, admission)| {
                CdpEvaluatedHandle::from_remote_object(Arc::clone(&client), remote, limits)
                    .map(|native| (native, admission))
                    .map_err(map_handle_error)
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.retain_many(owner, values)
    }

    pub(crate) async fn evaluate_handle(
        self: &Arc<Self>,
        owner: Arc<WorkerOwner>,
        params: EvaluateHandleParams,
    ) -> Result<WorkerHandle, HostError> {
        let unique_context_id = owner.ready_unique_context_id().await?;
        let _call = self.call_slot()?;
        let admission = self.admit_handle()?;
        let service = CdpHandleEvaluationService::from_evaluation(
            Self::evaluation(&owner, &unique_context_id),
            MAX_NATIVE_ID_BYTES,
            MAX_PREVIEW_BYTES,
        );
        let native = service.call(params).await.map_err(map_handle_error)?;
        self.retain(owner, native, admission)
    }

    pub(crate) async fn get_property(
        self: &Arc<Self>,
        target: &WorkerHandle,
        name: String,
    ) -> Result<WorkerHandle, HostError> {
        let Some(owner) = target.core.owner.upgrade() else {
            target.request_dispose();
            return Err(HostError::TargetClosed);
        };
        let unique_context_id = match owner.ready_unique_context_id().await {
            Ok(unique_context_id) => unique_context_id,
            Err(error @ HostError::TargetClosed) => {
                target.request_dispose();
                return Err(error);
            }
            Err(error) => return Err(error),
        };
        let _call = self.call_slot()?;
        let admission = self.admit_handle()?;
        let native = target.core.native.read().await;
        let target_native = native
            .as_ref()
            .cloned()
            .ok_or_else(|| HostError::Backend("JSHandle is disposed".into()))?;
        if owner.is_closed()? {
            target.request_dispose();
            return Err(HostError::TargetClosed);
        }
        let service = CdpHandlePropertiesService::from_evaluation(
            Self::evaluation(&owner, &unique_context_id),
            MAX_NATIVE_ID_BYTES,
            MAX_PREVIEW_BYTES,
            CdpHandlePropertyLimits::new(MAX_PROPERTIES, MAX_PROPERTY_NAME_BYTES)
                .map_err(HostError::backend)?,
        );
        let property = service
            .call_property_on_handle(ObjectId::new(target.core.id), target_native, &name)
            .await
            .map_err(map_handle_error)?;
        drop(native);
        self.retain(owner, property, admission)
    }

    pub(crate) async fn json_value(
        self: &Arc<Self>,
        target: &WorkerHandle,
    ) -> Result<crate::evaluation::EvaluationValue, HostError> {
        let Some(owner) = target.core.owner.upgrade() else {
            target.request_dispose();
            return Err(HostError::TargetClosed);
        };
        let unique_context_id = match owner.ready_unique_context_id().await {
            Ok(unique_context_id) => unique_context_id,
            Err(error @ HostError::TargetClosed) => {
                target.request_dispose();
                return Err(error);
            }
            Err(error) => return Err(error),
        };
        let _call = self.call_slot()?;
        let native = target.core.native.read().await;
        let target_native = native
            .as_ref()
            .cloned()
            .ok_or_else(|| HostError::Backend("JSHandle is disposed".into()))?;
        if owner.is_closed()? {
            target.request_dispose();
            return Err(HostError::TargetClosed);
        }
        if let Some(value) = target_native.primitive_value().map_err(map_handle_error)? {
            return Ok(value);
        }
        drop(native);
        CdpHandleValueEvaluationService::from_evaluation(
            Self::evaluation(&owner, &unique_context_id),
            MAX_NATIVE_ID_BYTES,
            MAX_PREVIEW_BYTES,
        )
        .call(CdpHandleEvaluationRequest::new_on_native_handle(
            EvaluateHandleParams::function_without_argument("target => target"),
            target_native,
        ))
        .await
        .map_err(map_handle_error)
    }

    pub(crate) fn close(&self) {
        let handles = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.closed {
                return;
            }
            state.closed = true;
            std::mem::take(&mut state.handles)
        };
        for handle in handles.into_values().filter_map(|handle| handle.upgrade()) {
            handle.request_dispose();
        }
    }
}

impl Drop for Installation {
    fn drop(&mut self) {
        self.close();
    }
}

impl HostSession {
    pub async fn worker_evaluate_handle(
        &self,
        owner: Arc<WorkerOwner>,
        params: EvaluateHandleParams,
    ) -> Result<WorkerHandle, HostError> {
        self.require_visible_page_id(owner.page())?;
        self.worker_events()?.require_current_owner(&owner)?;
        let installation = self.worker_handles()?;
        tokio::select! {
            result = installation.evaluate_handle(owner, params) => result,
            () = self.stop.cancelled() => Err(HostError::TargetClosed),
            () = self.disconnected.cancelled() => Err(HostError::TargetClosed),
        }
    }

    pub async fn worker_handle_get_property(
        &self,
        target: &WorkerHandle,
        name: String,
    ) -> Result<WorkerHandle, HostError> {
        let installation = self.worker_handles()?;
        self.require_visible_worker_handle(target, &installation)?;
        tokio::select! {
            result = installation.get_property(target, name) => result,
            () = self.stop.cancelled() => Err(HostError::TargetClosed),
            () = self.disconnected.cancelled() => Err(HostError::TargetClosed),
        }
    }

    pub async fn worker_handle_json_value(
        &self,
        target: &WorkerHandle,
    ) -> Result<crate::evaluation::EvaluationValue, HostError> {
        let installation = self.worker_handles()?;
        self.require_visible_worker_handle(target, &installation)?;
        tokio::select! {
            result = installation.json_value(target) => result,
            () = self.stop.cancelled() => Err(HostError::TargetClosed),
            () = self.disconnected.cancelled() => Err(HostError::TargetClosed),
        }
    }

    fn require_visible_worker_handle(
        &self,
        target: &WorkerHandle,
        installation: &Arc<Installation>,
    ) -> Result<(), HostError> {
        if !target
            .core
            .installation
            .upgrade()
            .is_some_and(|registered| Arc::ptr_eq(&registered, installation))
        {
            return Err(HostError::TargetClosed);
        }
        let owner = target.core.owner.upgrade().ok_or(HostError::TargetClosed)?;
        self.require_visible_page_id(owner.page())?;
        self.worker_events()?.require_current_owner(&owner)
    }

    fn worker_handles(&self) -> Result<Arc<Installation>, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        Ok(Arc::clone(&live.worker_handles))
    }
}

fn map_handle_error(error: OperationError<HandleEvaluationErrorDetails>) -> HostError {
    if let Some(HandleEvaluationErrorDetails::Javascript {
        name,
        message,
        stack,
    }) = error.details()
    {
        HostError::Javascript {
            name: name.clone(),
            message: message.clone(),
            stack: stack.clone(),
        }
    } else {
        HostError::operation(error, 0)
    }
}

fn cleanup_admission_error(error: CleanupSubmitError) -> HostError {
    match error {
        CleanupSubmitError::CapacityExceeded { .. } => {
            HostError::ResourceExhausted(error.to_string())
        }
        CleanupSubmitError::Unavailable => HostError::TargetClosed,
        CleanupSubmitError::StatePoisoned => HostError::backend(error),
    }
}

#[cfg(test)]
mod tests {
    use pwright_backend_cdp_handle_evaluation::{CdpEvaluatedHandle, CdpHandleEvaluationLimits};
    use pwright_core::{ResourceClass, ResourceLimits};
    use pwright_fake::FakeCdpClient;

    use super::*;

    fn resources() -> ProductionResourceLedger {
        let required = requirements();
        let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
        for class in ResourceClass::ALL {
            let bound = required.class(class);
            limits = limits.with_class(class, bound.global, bound.per_scope);
        }
        ProductionResourceLedger::new(limits, required).unwrap()
    }

    fn object_handle(fake: Arc<FakeCdpClient>, id: &str) -> CdpEvaluatedHandle {
        let client: Arc<dyn pwright_cdp::CdpClient> = fake;
        CdpEvaluatedHandle::from_remote_object(
            client,
            &serde_json::json!({
                "type": "object",
                "objectId": id,
                "description": "Object"
            }),
            CdpHandleEvaluationLimits::new(
                CdpEvaluationLimits::default(),
                MAX_NATIVE_ID_BYTES,
                MAX_PREVIEW_BYTES,
            ),
        )
        .unwrap()
    }

    fn retained(
        installation: &Arc<Installation>,
        owner: Arc<WorkerOwner>,
        fake: Arc<FakeCdpClient>,
        id: &str,
    ) -> WorkerHandle {
        let admission = installation.admit_handle().unwrap();
        installation
            .retain(owner, object_handle(fake, id), admission)
            .unwrap()
    }

    #[tokio::test]
    async fn worker_close_rejects_property_read_before_native_io() {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let cleanup = TokioCleanupExecutor::new(MAX_WORKER_HANDLES);
        let installation = Installation::new(&resources(), cleanup.clone()).unwrap();
        let owner = WorkerOwner::for_test(ObjectId::new(7), fake.clone());
        let handle = retained(
            &installation,
            Arc::clone(&owner),
            Arc::clone(&fake),
            "worker-1",
        );
        owner.close().unwrap();
        let calls_before = fake.calls().len();

        // Act
        let result = installation.get_property(&handle, "self".to_owned()).await;

        // Assert
        assert!(matches!(result, Err(HostError::TargetClosed)));
        assert_eq!(fake.calls().len(), calls_before);
        handle.request_dispose().wait().await.unwrap();
        let report = cleanup.shutdown().await;
        assert_eq!(report.metrics.failed, 0);
        let releases = fake
            .calls()
            .into_iter()
            .filter(|call| call.0 == "Runtime.releaseObject")
            .count();
        assert_eq!(releases, 0);
    }

    #[tokio::test]
    async fn concurrent_dispose_requests_release_native_object_once() {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let cleanup = TokioCleanupExecutor::new(MAX_WORKER_HANDLES);
        let installation = Installation::new(&resources(), cleanup.clone()).unwrap();
        let owner = WorkerOwner::for_test(ObjectId::new(7), fake.clone());
        let handle = retained(
            &installation,
            Arc::clone(&owner),
            Arc::clone(&fake),
            "worker-1",
        );

        // Act
        let (first, second) = tokio::join!(
            handle.request_dispose().wait(),
            handle.request_dispose().wait()
        );

        // Assert
        first.unwrap();
        second.unwrap();
        let releases = fake
            .calls()
            .into_iter()
            .filter(|call| call.0 == "Runtime.releaseObject")
            .count();
        assert_eq!(releases, 1);
        installation.close();
        let report = cleanup.shutdown().await;
        assert_eq!(report.metrics.failed, 0);
    }

    #[tokio::test]
    async fn installation_close_drains_cleanup_while_descriptor_is_retained() {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let cleanup = TokioCleanupExecutor::new(MAX_WORKER_HANDLES);
        let installation = Installation::new(&resources(), cleanup.clone()).unwrap();
        let owner = WorkerOwner::for_test(ObjectId::new(7), fake.clone());
        let handle = retained(
            &installation,
            Arc::clone(&owner),
            Arc::clone(&fake),
            "worker-1",
        );

        // Act
        installation.close();
        let report = cleanup.shutdown().await;

        // Assert
        assert_eq!(report.metrics.completed, 1);
        assert_eq!(report.metrics.failed, 0);
        handle.request_dispose().wait().await.unwrap();
        let releases = fake
            .calls()
            .into_iter()
            .filter(|call| call.0 == "Runtime.releaseObject")
            .count();
        assert_eq!(releases, 1);
    }

    #[tokio::test]
    async fn disposed_descriptor_does_not_retain_worker_session_owner() {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let cleanup = TokioCleanupExecutor::new(MAX_WORKER_HANDLES);
        let installation = Installation::new(&resources(), cleanup.clone()).unwrap();
        let owner = WorkerOwner::for_test(ObjectId::new(7), fake.clone());
        let owner_witness = Arc::downgrade(&owner);
        let handle = retained(
            &installation,
            Arc::clone(&owner),
            Arc::clone(&fake),
            "worker-1",
        );

        // Act
        handle.request_dispose().wait().await.unwrap();
        drop(owner);
        let report = cleanup.shutdown().await;

        // Assert
        assert!(owner_witness.upgrade().is_none());
        assert_eq!(report.metrics.completed, 1);
        assert_eq!(report.metrics.failed, 0);
    }

    #[tokio::test]
    async fn handle_capacity_is_hard_bounded_before_native_io() {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let cleanup = TokioCleanupExecutor::new(MAX_WORKER_HANDLES);
        let installation = Installation::new(&resources(), cleanup.clone()).unwrap();
        let admissions = (0..MAX_WORKER_HANDLES)
            .map(|_| installation.admit_handle().unwrap())
            .collect::<Vec<_>>();

        // Act
        let overflow = installation.admit_handle();

        // Assert
        assert!(matches!(overflow, Err(HostError::ResourceExhausted(_))));
        assert_eq!(fake.calls().len(), 0);
        drop(admissions);
        installation.close();
        let report = cleanup.shutdown().await;
        assert_eq!(report.metrics.failed, 0);
    }

    #[cfg(feature = "worker-console-argument-handles")]
    #[tokio::test]
    async fn event_argument_adoption_preserves_order_and_exact_worker_owner() {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let cleanup = TokioCleanupExecutor::new(MAX_WORKER_HANDLES);
        let installation = Installation::new(&resources(), cleanup.clone()).unwrap();
        let owner = WorkerOwner::for_test(ObjectId::new(7), fake.clone());
        let arguments = serde_json::json!([
            {"type": "number", "value": 1},
            {"type": "string", "value": "two"},
            {"type": "object", "objectId": "worker-object", "description": "Object"},
            {"type": "undefined"}
        ]);

        // Act
        let handles = installation
            .adopt_event_handles(
                Arc::clone(&owner),
                arguments.as_array().expect("argument array"),
            )
            .unwrap();

        // Assert
        assert_eq!(
            handles
                .iter()
                .map(WorkerHandle::preview)
                .collect::<Vec<_>>(),
            vec!["1", "two", "Object", "undefined"]
        );
        assert!(handles.iter().all(|handle| Arc::ptr_eq(
            &handle.core.owner.upgrade().expect("live Worker owner"),
            &owner
        )));
        for handle in handles {
            handle.request_dispose().wait().await.unwrap();
        }
        installation.close();
        let report = cleanup.shutdown().await;
        assert_eq!(report.metrics.failed, 0);
        let releases = fake
            .calls()
            .into_iter()
            .filter(|call| call.0 == "Runtime.releaseObject")
            .count();
        assert_eq!(releases, 1);
    }

    #[tokio::test]
    async fn primitive_json_value_uses_retained_value_without_native_io() {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let cleanup = TokioCleanupExecutor::new(MAX_WORKER_HANDLES);
        let installation = Installation::new(&resources(), cleanup.clone()).unwrap();
        let owner = WorkerOwner::for_test(ObjectId::new(7), fake.clone());
        let client: Arc<dyn pwright_cdp::CdpClient> = fake.clone();
        let native = CdpEvaluatedHandle::from_remote_object(
            client,
            &serde_json::json!({"type": "string", "value": "null"}),
            Installation::handle_limits(),
        )
        .unwrap();
        let admission = installation.admit_handle().unwrap();
        let handle = installation
            .retain(Arc::clone(&owner), native, admission)
            .unwrap();

        // Act
        let value = installation.json_value(&handle).await.unwrap();

        // Assert
        assert_eq!(
            value,
            crate::evaluation::EvaluationValue::String("null".to_owned())
        );
        assert!(fake.calls().is_empty());
        handle.request_dispose().wait().await.unwrap();
        installation.close();
        let report = cleanup.shutdown().await;
        assert_eq!(report.metrics.failed, 0);
    }

    #[tokio::test]
    async fn object_json_value_executes_in_exact_worker_context() {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html(""));
        fake.set_evaluate_response(serde_json::json!({
            "result": {"value": {"ok": true, "value": {"t": "str", "v": "done"}}}
        }));
        let cleanup = TokioCleanupExecutor::new(MAX_WORKER_HANDLES);
        let installation = Installation::new(&resources(), cleanup.clone()).unwrap();
        let owner = WorkerOwner::for_test(ObjectId::new(7), fake.clone());
        let handle = retained(
            &installation,
            Arc::clone(&owner),
            Arc::clone(&fake),
            "worker-object",
        );

        // Act
        let value = installation.json_value(&handle).await.unwrap();

        // Assert
        assert_eq!(
            value,
            crate::evaluation::EvaluationValue::String("done".to_owned())
        );
        let calls = fake.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "Runtime.callFunctionOn");
        assert_eq!(calls[0].1[0]["uniqueContextId"], "worker-test-world");
        assert_eq!(calls[0].1[0]["returnByValue"], true);
        assert_eq!(calls[0].1[0]["arguments"][1]["objectId"], "worker-object");
        handle.request_dispose().wait().await.unwrap();
        installation.close();
        let report = cleanup.shutdown().await;
        assert_eq!(report.metrics.failed, 0);
    }
}
