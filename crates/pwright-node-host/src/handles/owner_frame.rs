//! A current-DOM owner query with one admitted temporary-native continuation.

use super::{
    PendingHandle,
    creation::{Creation, Installation, failure, map_leaf_error},
    pending::ReadAdmission,
};
use crate::{HostError, HostSession};
use pwright_backend_cdp_topology::{CdpPageState, CdpTopology};
use pwright_browser_model::ELEMENT_HANDLE_OBJECT_KIND_ID;
use pwright_core::{
    CallMetadata, CancellationReason, Command, CommandCancellation, CommandSpec, FailureKind,
    MethodTag, ObjectId, ObjectKindId, OperationContext, OperationError,
};
use pwright_runtime::{ReservedCleanupExecutor, TargetedService};
use std::{
    future::{Future, Ready, ready},
    pin::Pin,
};

struct OwnerFrame;
impl CommandSpec for OwnerFrame {
    const METHOD: MethodTag = MethodTag::new(65006);
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = &[ELEMENT_HANDLE_OBJECT_KIND_ID];
    type Params = Params;
    type Result = Option<String>;
    type ErrorDetails = HostError;
}

pub(super) struct Params {
    admission: ReadAdmission,
    source_frame: ObjectId,
}

enum SourceContext {
    Current(ObjectId),
    WorldReplaced,
}

fn world_replaced() -> HostError {
    HostError::Backend(
        "Execution context was destroyed, most likely because of a navigation".into(),
    )
}

fn source_context(
    topology: &CdpTopology,
    handle: &PendingHandle,
) -> Result<SourceContext, HostError> {
    let source = handle
        .source_world()
        .ok_or_else(|| HostError::backend("Handle creation-world identity is missing"))?;
    let frame = handle.initializer().parent_id;
    let page = topology
        .page_object_id_for_frame(frame)
        .filter(|page| *page == source.page_id())
        .ok_or(HostError::TargetClosed)?;
    let route = topology
        .page_session_routes()
        .find(|route| route.object_id() == page)
        .ok_or(HostError::TargetClosed)?;
    // Native topology intentionally retires runtime Handle objects on a main
    // world replacement. Preserve that reason without guessing from an error
    // string or converting a Page-close/crash/missing-world gap to navigation.
    if matches!(
        topology.page_state(route.target_id()),
        Some(CdpPageState::Ready)
    ) && topology
        .frame_execution_routes()
        .find(|route| route.object_id() == frame)
        .is_some_and(|current| {
            source.execution_context_id() != current.execution_context_id()
                || source.unique_context_id() != current.unique_context_id()
        })
    {
        return Ok(SourceContext::WorldReplaced);
    }
    Ok(SourceContext::Current(route.browser_context_object_id()))
}

async fn publish_owner_frame_result<T>(
    outcome: Result<T, OperationError<HostError>>,
    connected: bool,
    publication: impl Future<Output = Result<(), HostError>>,
    source_world_replaced: impl FnOnce() -> bool,
) -> Result<T, HostError> {
    // HostError intentionally has no Cleanup variant. Keep the structured kind
    // before converting details, rather than guessing from a diagnostic later.
    let cleanup_failed = outcome
        .as_ref()
        .is_err_and(|error| error.kind() == FailureKind::Cleanup);
    let mut result = outcome.map_err(|error| {
        let mut details = None;
        let error = error.map_details(|value| {
            details = Some(value);
        });
        details.unwrap_or_else(|| HostError::operation(error, 0))
    });
    if cleanup_failed {
        if connected {
            // Publish any preceding side effects, but a failed fence must not
            // replace the already-established native compensation failure.
            let _ = publication.await;
        }
        return result;
    }
    // Only an ordinary closed-operation result can be refined by a proven
    // newer world. Never erase a failed temporary-object compensation.
    if matches!(result, Err(HostError::TargetClosed)) && source_world_replaced() {
        result = Err(world_replaced());
    }
    crate::completion::publish_command_result(result, connected, publication).await
}

impl TargetedService<Params> for Installation {
    type Response = Option<String>;
    type Error = OperationError<HostError>;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call_for(&self, target: ObjectId, call: Params) -> Self::Future<'_> {
        // Reserve before native acquisition. The existing call allowance owns
        // this bounded continuation; no public Handle result ticket is needed.
        let continuation = match self.cleanup.try_reserve() {
            Ok(reservation) => reservation,
            Err(error) => {
                return Box::pin(ready(Err(failure(HostError::ResourceExhausted(
                    error.to_string(),
                )))));
            }
        };
        let service = self.create.clone();
        let worlds = self.worlds.clone();
        let future = async move {
            let ReadAdmission {
                call: _call,
                work: _work,
                native: _native,
            } = call.admission;
            // Retain the session/world registration through native cleanup.
            // Shutdown retires inspector sessions before draining this work;
            // a descriptor clone alone does not register that native lifetime.
            let _snapshot = worlds.snapshot(call.source_frame).await.map_err(|error| {
                OperationError::new(
                    pwright_core::FailureKind::TargetClosed,
                    error.to_string(),
                    Some(HostError::TargetClosed),
                )
            })?;
            service
                .owner_frame_native_id(target)
                .await
                .map_err(map_leaf_error)
        };
        Box::pin(Creation::new(future, continuation))
    }
}

impl CommandCancellation<OwnerFrame> for Installation {
    type Future<'a> = Ready<Result<(), OperationError<HostError>>>;
    fn cancel(&self, _: OperationContext, _: CancellationReason) -> Self::Future<'_> {
        // The query owns an ephemeral native receiver. Creation transfers it
        // and its read/work admission to the pre-reserved continuation.
        ready(Ok(()))
    }
}

impl HostSession {
    fn owner_frame_source_context(
        &self,
        handle: &PendingHandle,
    ) -> Result<SourceContext, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        let topology = live.composition.topology().map_err(HostError::backend)?;
        source_context(topology.topology(), handle)
    }

    /// Returns the canonical portable ID of the node's current owner Frame.
    /// Removed nodes can still have an owner; an orphan document returns None.
    pub async fn handle_owner_frame(
        &self,
        handle: &PendingHandle,
    ) -> Result<Option<String>, HostError> {
        if !handle.is_element() {
            return Err(HostError::InvalidArgument(
                "ownerFrame requires an ElementHandle".into(),
            ));
        }
        let (target, call_id, _operation) =
            self.admit_operation(&handle.initializer().object_id.get().to_string())?;
        let (runtime, installation) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            (live.runtime.clone(), live.handles.clone())
        };
        let admission = installation.owners.admit_tracked_read(handle)?;
        let context = match self.owner_frame_source_context(handle)? {
            SourceContext::Current(context) => context,
            SourceContext::WorldReplaced => return Err(world_replaced()),
        };
        let _caller_call = admission.call.clone();
        let command = Command::<OwnerFrame>::new(
            call_id,
            target,
            CallMetadata::default(),
            Params {
                admission,
                source_frame: handle.initializer().parent_id,
            },
        );
        let dispatch = runtime.dispatch_with(command, installation.as_ref(), installation.clone());
        let result = tokio::select! {
            biased;
            () = self.stop.cancelled() => return Err(HostError::TargetClosed),
            result = dispatch => result.unwrap_or_else(|never| match never {}).into_outcome(),
        };
        let native =
            publish_owner_frame_result(result, self.is_connected(), self.flush_topology(), || {
                matches!(
                    self.owner_frame_source_context(handle),
                    Ok(SourceContext::WorldReplaced)
                )
            })
            .await?;
        let Some(native) = native else {
            return Ok(None);
        };
        // Reconcile against one current topology read, never the frozen source
        // parent or a selector. Cross-Page adoption is scoped to the Context.
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        let topology = live.composition.topology().map_err(HostError::backend)?;
        let topology = topology.topology();
        let frame = topology.frame_object_id(&native).filter(|frame| {
            topology.page_object_id_for_frame(*frame).and_then(|page| {
                topology
                    .page_session_routes()
                    .find(|route| route.object_id() == page)
                    .map(|route| route.browser_context_object_id())
            }) == Some(context)
        });
        Ok(frame.map(|frame| frame.get().to_string()))
    }
}

#[cfg(test)]
mod tests;
