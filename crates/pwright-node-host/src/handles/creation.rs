//! Creation continuation plus revocable Ready-result ownership. N-API delivery
//! is deliberately a later consumer, not a raw unowned Handle ID escape.

use std::{
    future::{Future, Ready, ready},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use pwright_browser_model::{
    ELEMENT_HANDLE_OBJECT_KIND_ID, FRAME_OBJECT_KIND_ID, JS_HANDLE_OBJECT_KIND_ID,
};
use pwright_cap_handle_evaluation::{
    EvaluateHandleParams, GetPropertyParams, HandleEvaluationErrorDetails,
};
#[cfg(feature = "frame-element")]
use pwright_composition_chromium::ChromiumMainWorldSnapshot;
use pwright_composition_chromium::{
    AttachedChromiumComposition, ChromiumFrameWorldAccess, ChromiumFrameWorldSnapshot,
};
use pwright_composition_chromium_handle_evaluation::ChromiumHandleEvaluationCommandService as ChromiumHandleLeafEvaluationCommandService;
#[cfg(feature = "selector-registration")]
use pwright_composition_chromium_handle_evaluation::ChromiumHandleEvaluationRoute;
use pwright_core::{
    CallMetadata, CancellationReason, Command, CommandCancellation, CommandSpec, FailureKind,
    MethodTag, ObjectId, ObjectKindId, OperationContext, OperationError,
};
use pwright_runtime::{
    CleanupFailure, CleanupReservation, ReservedCleanupExecutor, TargetedService,
};
use pwright_runtime_tokio::{TokioCleanupExecutor, TokioCleanupReservation};

use super::HandleEvaluationProgram;
use super::pending::{Admission, Owners, PendingHandle};
use super::publication::{PublicationContext, Publications};
use super::release::Releases;
use crate::{CommandRuntime, HostError, HostSession, evaluation};

pub(crate) struct Installation {
    pub(super) create: Arc<ChromiumHandleLeafEvaluationCommandService>,
    pub(super) dispose: Arc<Releases>,
    publications: Arc<Publications>,
    pub(super) batches: Arc<super::batch_publication::BatchPublications>,
    pub owners: Arc<Owners>,
    pub(crate) cleanup: TokioCleanupExecutor,
    #[cfg(feature = "element-handle-click")]
    pub(crate) parent_hit_slots: Arc<tokio::sync::Semaphore>,
    #[cfg(any(
        feature = "element-handle-scroll-into-view",
        feature = "element-handle-select-text"
    ))]
    pub(crate) element_action_slots: Arc<tokio::sync::Semaphore>,
    pub(super) worlds: ChromiumFrameWorldAccess,
}

enum CreationWorldSnapshot {
    Paired(ChromiumFrameWorldSnapshot),
    #[cfg(feature = "frame-element")]
    Main(ChromiumMainWorldSnapshot),
}
impl CreationWorldSnapshot {
    fn page(&self) -> ObjectId {
        match self {
            Self::Paired(snapshot) => snapshot.page(),
            #[cfg(feature = "frame-element")]
            Self::Main(snapshot) => snapshot.page(),
        }
    }
    #[cfg(feature = "frame-element")]
    fn client(&self) -> Arc<dyn pwright_cdp::CdpClient> {
        match self {
            Self::Paired(snapshot) => snapshot.client(),
            #[cfg(feature = "frame-element")]
            Self::Main(snapshot) => snapshot.client(),
        }
    }
    fn main_execution_context_id(&self) -> i64 {
        match self {
            Self::Paired(snapshot) => snapshot.main_execution_context_id(),
            #[cfg(feature = "frame-element")]
            Self::Main(snapshot) => snapshot.main_execution_context_id(),
        }
    }
    fn main_unique_id(&self) -> &str {
        match self {
            Self::Paired(snapshot) => snapshot.main_unique_id(),
            #[cfg(feature = "frame-element")]
            Self::Main(snapshot) => snapshot.main_unique_id(),
        }
    }
    fn native_lifetime(&self) -> Option<pwright_composition_chromium::ChromiumNativeWorldLifetime> {
        match self {
            Self::Paired(snapshot) => snapshot.native_lifetime(),
            #[cfg(feature = "frame-element")]
            Self::Main(snapshot) => snapshot.native_lifetime(),
        }
    }
}

impl Installation {
    #[cfg(any(
        feature = "element-handle-click",
        feature = "element-handle-scroll-into-view"
    ))]
    pub(crate) async fn retained_element_world_snapshot(
        &self,
        frame: ObjectId,
    ) -> Result<ChromiumFrameWorldSnapshot, HostError> {
        self.worlds
            .snapshot(frame)
            .await
            .map_err(HostError::backend)
    }

    #[cfg(any(
        feature = "element-handle-click",
        feature = "element-handle-scroll-into-view"
    ))]
    pub(crate) fn retained_element_source(
        &self,
        target: ObjectId,
    ) -> Result<
        pwright_composition_chromium_handle_evaluation::ChromiumRetainedElementSource,
        OperationError<HandleEvaluationErrorDetails>,
    > {
        self.create.borrow_element_source(target)
    }

    #[cfg(any(
        feature = "element-handle-click",
        feature = "element-handle-scroll-into-view"
    ))]
    pub(crate) fn validate_retained_element_source(
        &self,
        target: ObjectId,
        source: &pwright_composition_chromium_handle_evaluation::ChromiumRetainedElementSource,
    ) -> Result<(), OperationError<HandleEvaluationErrorDetails>> {
        self.create.validate_element_source(target, source)
    }

    pub fn new(
        composition: &AttachedChromiumComposition,
        runtime: &CommandRuntime,
        cleanup: TokioCleanupExecutor,
        worlds: ChromiumFrameWorldAccess,
    ) -> Result<Self, HostError> {
        let (create, dispose) = composition
            .handle_leaf_services(runtime, cleanup.clone())
            .map_err(HostError::backend)?
            .ok_or_else(|| HostError::backend("Handle leaf services are missing"))?;
        let owners = Owners::new(composition.resources())?;
        let publications = Publications::new(composition.resources(), owners.clone())?;
        let batches = super::batch_publication::BatchPublications::new(
            composition.resources(),
            owners.clone(),
        )?;
        let runtime_objects = std::num::NonZeroUsize::new(runtime.limits().live_objects)
            .ok_or_else(|| {
                HostError::backend("Handle release requires a nonzero runtime object limit")
            })?;
        Ok(Self {
            create: Arc::new(create),
            dispose: Releases::new(
                dispose,
                &runtime.lifecycle_handle(),
                runtime_objects,
                composition.resources(),
            )?,
            owners,
            publications,
            batches,
            cleanup,
            #[cfg(feature = "element-handle-click")]
            parent_hit_slots: Arc::new(tokio::sync::Semaphore::new(super::MAX_CALLS)),
            #[cfg(any(
                feature = "element-handle-scroll-into-view",
                feature = "element-handle-select-text"
            ))]
            element_action_slots: new_element_action_slots(),
            worlds,
        })
    }
    pub fn close(&self) {
        self.owners.close();
        self.publications.close();
        self.batches.close();
    }
    pub fn finish_ready(&self) -> Result<(), HostError> {
        self.publications.finish_ready()?;
        self.batches.finish_ready()?;
        self.dispose.finish_ready()
    }
    #[cfg(feature = "console-argument-handles")]
    pub(crate) fn finish_event_handles_ready(&self) -> Result<(), HostError> {
        self.batches.finish_ready()
    }
    pub async fn changed(&self) {
        tokio::select! {
            () = self.dispose.changed() => {},
            () = self.publications.changed() => {},
            () = self.batches.changed() => {},
        }
    }
    pub fn native_event_processed(&self) {
        self.dispose.creation_polled();
    }
    pub async fn wait(&self) -> Result<(), HostError> {
        let waiting = self.owners.wait();
        tokio::pin!(waiting);
        let mut result = Ok(());
        loop {
            if let Err(error) = self.finish_ready() {
                self.dispose.close();
                result = Err(error);
            }
            tokio::select! {
                biased;
                () = &mut waiting => {
                    self.dispose.close();
                    return result;
                }
                () = self.changed() => {}
            }
        }
    }

    /// Admit and queue Handles for RemoteObjects already emitted by Chromium.
    /// No browser request is made here; the serialized driver publishes the
    /// batch at its next local commit boundary.
    #[cfg(feature = "console-argument-handles")]
    pub(crate) fn stage_event_handles(
        &self,
        frame: ObjectId,
        remotes: &[serde_json::Value],
    ) -> Result<super::PendingEventHandles, HostError> {
        if remotes.len() > super::MAX_EVENT_ARGUMENT_HANDLES {
            return Err(HostError::ResourceExhausted(format!(
                "event Handle batch exceeds {} arguments",
                super::MAX_EVENT_ARGUMENT_HANDLES
            )));
        }
        let admission = self.owners.admit_batch_exact(remotes.len())?;
        let recovery = self
            .cleanup
            .try_reserve()
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let reservations = (0..remotes.len())
            .map(|_| self.cleanup.try_reserve())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        // Runtime.enable may replay a main-world console event while the
        // topology initializer is still creating this Frame's utility world.
        // The exact current main-world route is sufficient to stage those
        // already-emitted RemoteObjects; only the optional native-retirement
        // proof requires the complete paired-world snapshot.
        let snapshot = self
            .worlds
            .snapshot_current(frame)
            .map_err(HostError::backend)?;
        let native = self
            .create
            .stage_remote_objects(frame, remotes)
            .map_err(map_leaf_error)
            .map_err(|error| {
                let mut details = None;
                let error = error.map_details(|value| details = Some(value));
                details.unwrap_or_else(|| HostError::operation(error, 0))
            })?;
        let identity = native.world_identity();
        let lifetime = snapshot.and_then(|snapshot| {
            (identity.page_id() == snapshot.page()
                && identity.execution_context_id() == snapshot.main_execution_context_id()
                && identity.unique_context_id() == snapshot.main_unique_id())
            .then(|| snapshot.native_lifetime())
            .flatten()
        });
        let received = self.batches.enqueue_event(
            native,
            super::batch_publication::BatchContext {
                admission,
                reservations,
                recovery,
                lifetime,
                releases: self.dispose.clone(),
            },
        );
        Ok(super::PendingEventHandles { received })
    }
}

#[cfg(any(
    feature = "element-handle-scroll-into-view",
    feature = "element-handle-select-text"
))]
fn new_element_action_slots() -> Arc<tokio::sync::Semaphore> {
    // Both whole-action APIs share this single budget, leaving half of the
    // ordinary Handle call slots for their nested exact-handle evaluations.
    Arc::new(tokio::sync::Semaphore::new(super::MAX_CALLS / 2))
}

#[cfg(all(
    test,
    feature = "element-handle-scroll-into-view",
    feature = "element-handle-select-text"
))]
mod element_action_slot_tests {
    use super::new_element_action_slots;

    #[test]
    fn scroll_and_select_text_share_four_action_slots() {
        // Arrange
        let shared = new_element_action_slots();
        let scroll = shared.clone();
        let select_text = shared.clone();
        let mut held = Vec::new();

        // Act
        for slots in [&scroll, &select_text, &scroll, &select_text] {
            held.push(slots.clone().try_acquire_owned().unwrap());
        }
        let scroll_at_capacity = scroll.try_acquire_owned();
        let select_text_at_capacity = select_text.try_acquire_owned();

        // Assert
        assert_eq!(held.len(), 4);
        assert!(scroll_at_capacity.is_err());
        assert!(select_text_at_capacity.is_err());
        assert_eq!(shared.available_permits(), 0);
        assert_eq!(super::super::MAX_CALLS - held.len(), 4);
    }
}
impl Drop for Installation {
    fn drop(&mut self) {
        self.close();
        self.dispose.close();
    }
}

struct Prepare;
impl CommandSpec for Prepare {
    const METHOD: MethodTag = MethodTag::new(65003);
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = &[
        FRAME_OBJECT_KIND_ID,
        JS_HANDLE_OBJECT_KIND_ID,
        ELEMENT_HANDLE_OBJECT_KIND_ID,
    ];
    type Params = Params;
    type Result = Option<PendingHandle>;
    type ErrorDetails = HostError;
}
pub(crate) struct Params {
    operation: PrepareOperation,
    source_frame: ObjectId,
    admission: Admission,
}
enum PrepareOperation {
    Frame(HandleEvaluationProgram),
    FrameOptionalElement(HandleEvaluationProgram),
    #[cfg(feature = "frame-element")]
    FrameOwnerElement {
        child_native_frame_id: String,
    },
    #[cfg(feature = "selector-registration")]
    FrameInWorld {
        program: HandleEvaluationProgram,
        source: ChromiumHandleEvaluationRoute,
    },
    #[cfg(feature = "selector-registration")]
    FrameOptionalElementInWorld {
        program: HandleEvaluationProgram,
        source: ChromiumHandleEvaluationRoute,
    },
    Handle(HandleEvaluationProgram),
    Property(GetPropertyParams),
}
pub(super) type Outcome = Result<PendingHandle, OperationError<HostError>>;
type OptionalOutcome = Result<Option<PendingHandle>, OperationError<HostError>>;
type NativeFuture<T> =
    Pin<Box<dyn Future<Output = Result<T, OperationError<HostError>>> + Send + 'static>>;
pub(super) fn failure(error: HostError) -> OperationError<HostError> {
    OperationError::new(
        FailureKind::Backend,
        "Handle result preparation failed",
        Some(error),
    )
}
#[cfg(feature = "frame-element")]
fn frame_detached() -> HostError {
    HostError::Backend("Frame has been detached.".into())
}
pub(super) fn map_error(error: OperationError<HandleEvaluationErrorDetails>) -> HostError {
    // Playwright preserves this exact detached-frame message from
    // DOM.getFrameOwner; the generic TargetClosed projection would erase it.
    #[cfg(feature = "frame-element")]
    if error.kind() == FailureKind::TargetClosed && error.message() == "Frame has been detached." {
        return frame_detached();
    }
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
pub(super) fn map_leaf_error(
    error: OperationError<HandleEvaluationErrorDetails>,
) -> OperationError<HostError> {
    OperationError::new(
        error.kind(),
        "Handle creation failed",
        Some(map_error(error)),
    )
}

pub(crate) struct Creation<T: Send + 'static = PendingHandle> {
    inner: Option<NativeFuture<T>>,
    reservation: Option<TokioCleanupReservation>,
    started: bool,
}
impl<T: Send + 'static> Creation<T> {
    pub(super) fn new(
        future: impl Future<Output = Result<T, OperationError<HostError>>> + Send + 'static,
        reservation: TokioCleanupReservation,
    ) -> Self {
        check_future_storage(&future);
        Self {
            inner: Some(Box::pin(future)),
            reservation: Some(reservation),
            started: false,
        }
    }
}
impl<T: Send + 'static> Future for Creation<T> {
    type Output = Result<T, OperationError<HostError>>;
    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.started = true;
        let result = match self
            .inner
            .as_mut()
            .expect("completed Handle creation cannot be polled again")
            .as_mut()
            .poll(context)
        {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(result) => result,
        };
        self.inner = None;
        self.reservation = None;
        Poll::Ready(result)
    }
}
impl<T: Send + 'static> Drop for Creation<T> {
    fn drop(&mut self) {
        if !self.started {
            return;
        }
        let Some(inner) = self.inner.take() else {
            return;
        };
        let reservation = self
            .reservation
            .take()
            .expect("started creation owns cleanup capacity");
        reservation.submit(Box::pin(async move {
            match inner.await {
                Ok(handle) => {
                    drop(handle);
                    Ok(())
                }
                Err(error) if error.kind() == FailureKind::Cleanup => Err(CleanupFailure::new(
                    "abandoned Handle creation reported failed native compensation",
                )),
                // Ordinary failed creation returned no owned result.
                Err(_) => Ok(()),
            }
        }));
    }
}

impl TargetedService<Params> for Installation {
    type Response = Option<PendingHandle>;
    type Error = OperationError<HostError>;
    type Future<'a> = Pin<Box<dyn Future<Output = OptionalOutcome> + Send + 'a>>;
    fn call_for(&self, target: ObjectId, call: Params) -> Self::Future<'_> {
        let reservation = match self.cleanup.try_reserve() {
            Ok(reservation) => reservation,
            Err(error) => {
                return Box::pin(ready(Err(failure(HostError::ResourceExhausted(
                    error.to_string(),
                )))));
            }
        };
        let service = self.create.clone();
        let releases = self.dispose.clone();
        let publications = self.publications.clone();
        let recovery = match self.cleanup.try_reserve() {
            Ok(reservation) => reservation,
            Err(error) => {
                return Box::pin(ready(Err(failure(HostError::ResourceExhausted(
                    error.to_string(),
                )))));
            }
        };
        let worlds = self.worlds.clone();
        let native_call = call.admission.call.clone();
        let future = async move {
            // Publication may return its envelope copy before this continuation
            // consumes the reply. Retain the same admission through that gap,
            // including when Creation is handed to cancellation recovery.
            let _native_call = native_call;
            let snapshot_error = |error: pwright_composition_chromium::FrameWorldAccessError| {
                OperationError::new(
                    FailureKind::TargetClosed,
                    error.to_string(),
                    Some(HostError::TargetClosed),
                )
            };
            #[cfg(feature = "frame-element")]
            let snapshot = if matches!(&call.operation, PrepareOperation::FrameOwnerElement { .. })
            {
                CreationWorldSnapshot::Main(
                    worlds
                        .snapshot_main(call.source_frame)
                        .await
                        .map_err(snapshot_error)?,
                )
            } else {
                CreationWorldSnapshot::Paired(
                    worlds
                        .snapshot(call.source_frame)
                        .await
                        .map_err(snapshot_error)?,
                )
            };
            #[cfg(not(feature = "frame-element"))]
            let snapshot = CreationWorldSnapshot::Paired(
                worlds
                    .snapshot(call.source_frame)
                    .await
                    .map_err(snapshot_error)?,
            );
            let native = match call.operation {
                PrepareOperation::Frame(params) => service
                    .stage_evaluate_program(target, CallMetadata::default(), params)
                    .await
                    .map(Some),
                PrepareOperation::FrameOptionalElement(params) => {
                    service
                        .stage_evaluate_optional_element_program(
                            target,
                            CallMetadata::default(),
                            params,
                        )
                        .await
                }
                #[cfg(feature = "frame-element")]
                PrepareOperation::FrameOwnerElement {
                    child_native_frame_id,
                } => service
                    .stage_frame_owner_element(
                        target,
                        &child_native_frame_id,
                        snapshot.client(),
                        snapshot.main_execution_context_id(),
                        snapshot.main_unique_id(),
                    )
                    .await
                    .map(Some),
                #[cfg(feature = "selector-registration")]
                PrepareOperation::FrameInWorld { program, source } => service
                    .stage_evaluate_element_program_in_world(
                        target,
                        CallMetadata::default(),
                        source,
                        program,
                    )
                    .await
                    .map(Some),
                #[cfg(feature = "selector-registration")]
                PrepareOperation::FrameOptionalElementInWorld { program, source } => {
                    service
                        .stage_evaluate_optional_element_program_in_world(
                            target,
                            CallMetadata::default(),
                            source,
                            program,
                        )
                        .await
                }
                PrepareOperation::Handle(params) => service
                    .stage_evaluate_program_for_handle(target, CallMetadata::default(), params)
                    .await
                    .map(Some),
                PrepareOperation::Property(params) => service
                    .stage_get_property(target, CallMetadata::default(), params)
                    .await
                    .map(Some),
            }
            .map_err(map_leaf_error)?;
            let Some(native) = native else {
                return Ok(None);
            };
            // Native identity, not current Frame liveness, binds this receipt.
            let identity = native.world_identity();
            let lifetime = (identity.page_id() == snapshot.page()
                && identity.execution_context_id() == snapshot.main_execution_context_id()
                && identity.unique_context_id() == snapshot.main_unique_id())
            .then(|| snapshot.native_lifetime())
            .flatten();
            publications
                .publish(
                    native,
                    PublicationContext {
                        admission: call.admission,
                        reservation: recovery,
                        lifetime,
                        releases,
                    },
                )
                .await
                .map(Some)
        };
        // Check the concrete compiler-generated layout at build time, before
        // publishing an artifact that would reject every creation at runtime.
        check_future_storage(&future);
        Box::pin(Creation {
            inner: Some(Box::pin(future)),
            reservation: Some(reservation),
            started: false,
        })
    }
}

fn check_future_storage<F: Future>(_: &F) {
    const { assert!(std::mem::size_of::<F>() <= super::CREATION_FUTURE_BYTES) };
}
impl CommandCancellation<Prepare> for Installation {
    type Future<'a> = Ready<Result<(), OperationError<HostError>>>;
    fn cancel(&self, _: OperationContext, _: CancellationReason) -> Self::Future<'_> {
        // Dropping Creation hands its native work to a pre-admitted continuation.
        // This is not proof of release; session close drains and reports it.
        ready(Ok(()))
    }
}

impl HostSession {
    /// Materialize the exact current owner DOM Element of a child Frame in its
    /// parent's main world. Closed shadow-root owners require DOM.getFrameOwner;
    /// no CSS or page-world lookup is a valid substitute.
    #[cfg(feature = "frame-element")]
    pub async fn frame_prepare_element(
        &self,
        child_frame_id: &str,
    ) -> Result<PendingHandle, HostError> {
        let child = child_frame_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Frame ID".into()))?;
        let (parent, child_native_frame_id) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let topology = topology.topology();
            let parent = topology
                .parent_frame_object_id(child)
                .ok_or_else(frame_detached)?;
            let native = topology.frame_native_id(child).ok_or_else(frame_detached)?;
            if native.is_empty() || native.len() > super::MAX_NATIVE_ID_BYTES {
                return Err(HostError::ResourceExhausted(
                    "native child Frame identity exceeds its declared bound".into(),
                ));
            }
            (parent, native.to_owned())
        };
        self.require_visible_frame_id(child)?;
        let handle = self
            .prepare_required_result(
                &parent.get().to_string(),
                None,
                PrepareOperation::FrameOwnerElement {
                    child_native_frame_id,
                },
            )
            .await?;
        // The command publication fence has now drained topology. Do not
        // return an owner for a child that detached during native acquisition.
        let attached = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            topology.topology().parent_frame_object_id(child) == Some(parent)
        };
        if !attached {
            handle.dispose().await?;
            return Err(frame_detached());
        }
        Ok(handle)
    }

    /// Prepare an owned Handle for a future language publication layer. No raw
    /// result escapes: abandonment and session close both schedule disposal.
    /// Persistent JSHandle/ElementHandle operations are not exposed by this API.
    pub async fn frame_prepare_handle(
        &self,
        frame_id: &str,
        params: EvaluateHandleParams,
    ) -> Result<PendingHandle, HostError> {
        self.frame_prepare_program(frame_id, params.into()).await
    }

    pub(super) async fn frame_prepare_program(
        &self,
        frame_id: &str,
        params: HandleEvaluationProgram,
    ) -> Result<PendingHandle, HostError> {
        self.prepare_required_result(frame_id, None, PrepareOperation::Frame(params))
            .await
    }

    pub(crate) async fn frame_prepare_optional_element_program(
        &self,
        frame_id: &str,
        params: HandleEvaluationProgram,
    ) -> Result<Option<PendingHandle>, HostError> {
        self.prepare_result(
            frame_id,
            None,
            PrepareOperation::FrameOptionalElement(params),
        )
        .await
    }

    #[cfg(feature = "selector-registration")]
    pub(crate) async fn frame_prepare_element_program_in_world(
        &self,
        frame_id: &str,
        params: HandleEvaluationProgram,
        source: ChromiumHandleEvaluationRoute,
    ) -> Result<PendingHandle, HostError> {
        self.prepare_required_result(
            frame_id,
            None,
            PrepareOperation::FrameInWorld {
                program: params,
                source,
            },
        )
        .await
    }

    #[cfg(feature = "selector-registration")]
    pub(crate) async fn frame_prepare_optional_element_program_in_world(
        &self,
        frame_id: &str,
        params: HandleEvaluationProgram,
        source: ChromiumHandleEvaluationRoute,
    ) -> Result<Option<PendingHandle>, HostError> {
        self.prepare_result(
            frame_id,
            None,
            PrepareOperation::FrameOptionalElementInWorld {
                program: params,
                source,
            },
        )
        .await
    }

    pub async fn handle_prepare_program(
        &self,
        target: &PendingHandle,
        params: HandleEvaluationProgram,
    ) -> Result<PendingHandle, HostError> {
        self.prepare_handle_result(target, PrepareOperation::Handle(params))
            .await
    }

    pub async fn handle_get_property(
        &self,
        target: &PendingHandle,
        name: String,
    ) -> Result<PendingHandle, HostError> {
        self.prepare_handle_result(
            target,
            PrepareOperation::Property(GetPropertyParams::new(name)),
        )
        .await
    }

    async fn prepare_handle_result(
        &self,
        target: &PendingHandle,
        operation: PrepareOperation,
    ) -> Result<PendingHandle, HostError> {
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .handles
                .clone()
        };
        installation
            .owners
            .check_arguments(std::iter::once(target))?;
        let initializer = target.initializer();
        self.prepare_required_result(
            &initializer.object_id.get().to_string(),
            Some(initializer.parent_id),
            operation,
        )
        .await
    }

    async fn prepare_required_result(
        &self,
        target_id: &str,
        source_frame: Option<ObjectId>,
        operation: PrepareOperation,
    ) -> Result<PendingHandle, HostError> {
        self.prepare_result(target_id, source_frame, operation)
            .await?
            .ok_or_else(|| HostError::Backend("required Handle creation returned no Handle".into()))
    }

    async fn prepare_result(
        &self,
        target_id: &str,
        source_frame: Option<ObjectId>,
        operation: PrepareOperation,
    ) -> Result<Option<PendingHandle>, HostError> {
        let (target, call_id, _operation) = self.admit_operation(target_id)?;
        let (runtime, installation) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            (live.runtime.clone(), live.handles.clone())
        };
        let admission = installation.owners.admit()?;
        // The caller can remain suspended after driver publication. Its input
        // owner vector and inline future stay covered until reply/topology
        // completion; this shares the existing lease rather than admitting more.
        let _caller_call = admission.call.clone();
        let (params, source_bytes) = match &operation {
            PrepareOperation::Frame(params)
            | PrepareOperation::FrameOptionalElement(params)
            | PrepareOperation::Handle(params) => (Some(params), evaluation::SOURCE_BYTES),
            #[cfg(feature = "frame-element")]
            PrepareOperation::FrameOwnerElement { .. } => (None, evaluation::SOURCE_BYTES),
            #[cfg(feature = "selector-registration")]
            PrepareOperation::FrameInWorld { program, .. }
            | PrepareOperation::FrameOptionalElementInWorld { program, .. } => (
                Some(program),
                crate::locator_evaluation::COLLECTION_SOURCE_BYTES,
            ),
            PrepareOperation::Property(_) => (None, evaluation::SOURCE_BYTES),
        };
        if params.is_some_and(|params| params.source().len() > source_bytes) {
            return Err(HostError::InvalidArgument(format!(
                "evaluation source exceeds {source_bytes} UTF-8 bytes"
            )));
        }
        if let Some(argument) = params.and_then(HandleEvaluationProgram::argument) {
            argument
                .validate(evaluation::value_limits())
                .map_err(|error| HostError::InvalidArgument(error.to_string()))?;
        }
        let command = Command::<Prepare>::new(
            call_id,
            target,
            CallMetadata::default(),
            Params {
                operation,
                source_frame: source_frame.unwrap_or(target),
                admission,
            },
        );
        let dispatch = runtime.dispatch_with(command, installation.as_ref(), installation.clone());
        let result = tokio::select! {
            biased;
            () = self.stop.cancelled() => return Err(HostError::TargetClosed),
            result = dispatch => result.unwrap_or_else(|never| match never {}).into_outcome(),
        }
        .map_err(|error| {
            let mut details = None;
            let error = error.map_details(|value| {
                details = Some(value);
            });
            details.unwrap_or_else(|| HostError::operation(error, 0))
        });
        crate::completion::publish_command_result(
            result,
            self.is_connected(),
            self.flush_topology(),
        )
        .await
    }
}
