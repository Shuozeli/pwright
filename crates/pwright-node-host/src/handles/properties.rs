//! One admitted command produces independently owned, named sibling Handles.
use super::{
    PendingHandle,
    batch_publication::{BatchContext, Outcome},
    creation::{Creation, Installation, failure, map_leaf_error},
    pending::BatchAdmission,
};
use crate::{HostError, HostSession};
use pwright_browser_model::{ELEMENT_HANDLE_OBJECT_KIND_ID, JS_HANDLE_OBJECT_KIND_ID};
use pwright_core::{
    CallMetadata, CancellationReason, Command, CommandCancellation, CommandSpec, FailureKind,
    MethodTag, ObjectId, ObjectKindId, OperationContext, OperationError,
};
use pwright_runtime::{ReservedCleanupExecutor, TargetedService};
use std::{
    future::{Future, Ready, ready},
    pin::Pin,
};

/// Names are bounded and ordered by the Rust property snapshot. Each Handle has
/// its own object identity and cleanup ticket, including aliased property values.
pub struct NamedPendingHandle {
    pub(super) name: String,
    pub(super) handle: PendingHandle,
    // Property names may exceed one descriptor's allowance. Retain the shared
    // batch call budget until the name is consumed by the language boundary.
    pub(super) _storage: std::sync::Arc<super::pending::Allowance>,
}
impl NamedPendingHandle {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn into_handle(self) -> PendingHandle {
        self.handle
    }
}

struct PrepareProperties;
impl CommandSpec for PrepareProperties {
    const METHOD: MethodTag = MethodTag::new(65005);
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] =
        &[JS_HANDLE_OBJECT_KIND_ID, ELEMENT_HANDLE_OBJECT_KIND_ID];
    type Params = Params;
    type Result = Vec<NamedPendingHandle>;
    type ErrorDetails = HostError;
}

pub(super) struct Params {
    source_frame: ObjectId,
    admission: BatchAdmission,
}

impl TargetedService<Params> for Installation {
    type Response = Vec<NamedPendingHandle>;
    type Error = OperationError<HostError>;
    type Future<'a> = Pin<Box<dyn Future<Output = Outcome> + Send + 'a>>;

    fn call_for(&self, target: ObjectId, call: Params) -> Self::Future<'_> {
        // Reserve the started-call continuation, whole-batch compensation and
        // every possible sibling ticket before snapshot or browser I/O.
        let capacity = (|| {
            let continuation = self.cleanup.try_reserve()?;
            let recovery = self.cleanup.try_reserve()?;
            let reservations = (0..call.admission.entries.len())
                .map(|_| self.cleanup.try_reserve())
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, pwright_runtime::CleanupSubmitError>((continuation, recovery, reservations))
        })();
        let (continuation, recovery, reservations) = match capacity {
            Ok(capacity) => capacity,
            Err(error) => {
                return Box::pin(ready(Err(failure(HostError::ResourceExhausted(
                    error.to_string(),
                )))));
            }
        };
        let service = self.create.clone();
        let releases = self.dispose.clone();
        let publications = self.batches.clone();
        let worlds = self.worlds.clone();
        let future = async move {
            let snapshot = worlds.snapshot(call.source_frame).await.map_err(|error| {
                OperationError::new(
                    FailureKind::TargetClosed,
                    error.to_string(),
                    Some(HostError::TargetClosed),
                )
            })?;
            let native = service
                .stage_get_properties(
                    target,
                    CallMetadata::default(),
                    call.admission.entries.len(),
                )
                .await
                .map_err(map_leaf_error)?;
            let identity = native.world_identity();
            let lifetime = (identity.page_id() == snapshot.page()
                && identity.execution_context_id() == snapshot.main_execution_context_id()
                && identity.unique_context_id() == snapshot.main_unique_id())
            .then(|| snapshot.native_lifetime())
            .flatten();
            publications
                .publish(
                    native,
                    BatchContext {
                        admission: call.admission,
                        reservations,
                        recovery,
                        lifetime,
                        releases,
                    },
                )
                .await
        };
        Box::pin(Creation::new(future, continuation))
    }
}

impl CommandCancellation<PrepareProperties> for Installation {
    type Future<'a> = Ready<Result<(), OperationError<HostError>>>;
    fn cancel(&self, _: OperationContext, _: CancellationReason) -> Self::Future<'_> {
        // Creation transfers started work to the pre-admitted continuation.
        ready(Ok(()))
    }
}

impl HostSession {
    pub async fn handle_get_properties(
        &self,
        handle: &PendingHandle,
    ) -> Result<Vec<NamedPendingHandle>, HostError> {
        let (target, call_id, _operation) =
            self.admit_operation(&handle.initializer().object_id.get().to_string())?;
        let (runtime, installation) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            (live.runtime.clone(), live.handles.clone())
        };
        installation
            .owners
            .check_arguments(std::iter::once(handle))?;
        let admission = installation.owners.admit_batch()?;
        let _caller_call = admission.call.clone();
        let command = Command::<PrepareProperties>::new(
            call_id,
            target,
            CallMetadata::default(),
            Params {
                source_frame: handle.initializer().parent_id,
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
