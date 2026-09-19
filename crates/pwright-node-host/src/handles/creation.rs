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
use pwright_composition_chromium::{AttachedChromiumComposition, ChromiumFrameWorldAccess};
use pwright_composition_chromium_handle_evaluation::ChromiumHandleEvaluationCommandService as ChromiumHandleLeafEvaluationCommandService;
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
    pub(super) cleanup: TokioCleanupExecutor,
    pub(super) worlds: ChromiumFrameWorldAccess,
}

impl Installation {
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
        Ok(Self {
            create: Arc::new(create),
            dispose: Releases::new(
                dispose,
                &runtime.lifecycle_handle(),
                composition.resources(),
            )?,
            owners,
            publications,
            batches,
            cleanup,
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
    type Result = PendingHandle;
    type ErrorDetails = HostError;
}
pub(crate) struct Params {
    operation: PrepareOperation,
    source_frame: ObjectId,
    admission: Admission,
}
enum PrepareOperation {
    Frame(HandleEvaluationProgram),
    Handle(HandleEvaluationProgram),
    Property(GetPropertyParams),
}
pub(super) type Outcome = Result<PendingHandle, OperationError<HostError>>;
type NativeFuture<T> =
    Pin<Box<dyn Future<Output = Result<T, OperationError<HostError>>> + Send + 'static>>;
pub(super) fn failure(error: HostError) -> OperationError<HostError> {
    OperationError::new(
        FailureKind::Backend,
        "Handle result preparation failed",
        Some(error),
    )
}
pub(super) fn map_error(error: OperationError<HandleEvaluationErrorDetails>) -> HostError {
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
    type Response = PendingHandle;
    type Error = OperationError<HostError>;
    type Future<'a> = Pin<Box<dyn Future<Output = Outcome> + Send + 'a>>;
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
            let snapshot = worlds.snapshot(call.source_frame).await.map_err(|error| {
                OperationError::new(
                    FailureKind::TargetClosed,
                    error.to_string(),
                    Some(HostError::TargetClosed),
                )
            })?;
            let native = match call.operation {
                PrepareOperation::Frame(params) => {
                    service
                        .stage_evaluate_program(target, CallMetadata::default(), params)
                        .await
                }
                PrepareOperation::Handle(params) => {
                    service
                        .stage_evaluate_program_for_handle(target, CallMetadata::default(), params)
                        .await
                }
                PrepareOperation::Property(params) => {
                    service
                        .stage_get_property(target, CallMetadata::default(), params)
                        .await
                }
            }
            .map_err(map_leaf_error)?;
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
        self.prepare_result(frame_id, None, PrepareOperation::Frame(params))
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
        self.prepare_result(
            &initializer.object_id.get().to_string(),
            Some(initializer.parent_id),
            operation,
        )
        .await
    }

    async fn prepare_result(
        &self,
        target_id: &str,
        source_frame: Option<ObjectId>,
        operation: PrepareOperation,
    ) -> Result<PendingHandle, HostError> {
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
        let params = match &operation {
            PrepareOperation::Frame(params) | PrepareOperation::Handle(params) => Some(params),
            PrepareOperation::Property(_) => None,
        };
        if params.is_some_and(|params| params.source().len() > evaluation::SOURCE_BYTES) {
            return Err(HostError::InvalidArgument(
                "evaluation source exceeds 32768 UTF-8 bytes".into(),
            ));
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
