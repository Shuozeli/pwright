use super::*;
use pwright_browser_model::FRAME_OBJECT_KIND_ID;
use pwright_cap_locator::Selector;
use pwright_core::{
    CallMetadata, CancellationReason, Command, CommandCancellation, CommandSpec, FailureKind,
    MethodTag, ObjectId, ObjectKindId, OperationContext, OperationError,
};
use pwright_file_input_fs::PreparedInput;
use pwright_runtime::TargetedService;
use std::{
    future::{Future, Ready, pending, ready},
    pin::Pin,
    time::Duration,
};

pub(crate) enum Input {
    Payloads(PreparedFileInput),
    Paths(FileInputJob<std::fs::File>, SourceSlot),
}
pub(crate) struct Params {
    selector: Selector,
    input: Input,
    // Input buffers have source leases. This separate lease admits command,
    // route-adapter and deadline storage before boxed futures are constructed.
    _lease: CallLease,
}
struct SetInputFiles;
impl CommandSpec for SetInputFiles {
    // Host-local typed operation; no portable wire tag is registered.
    const METHOD: MethodTag = MethodTag::new(65002);
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = &[FRAME_OBJECT_KIND_ID];
    type Params = Params;
    type Result = ();
    type ErrorDetails = HostError;
}
type Outcome = Result<(), OperationError<HostError>>;
fn failure(error: HostError) -> OperationError<HostError> {
    OperationError::new(
        FailureKind::Backend,
        "Locator file input failed",
        Some(error),
    )
}
impl TargetedService<Params> for Installation {
    type Response = ();
    type Error = OperationError<HostError>;
    type Future<'a> = Pin<Box<dyn Future<Output = Outcome> + Send + 'a>>;
    fn call_for(&self, frame: ObjectId, call: Params) -> Self::Future<'_> {
        let work = async move {
            let _lease = call._lease;
            let (payloads, slot) = match call.input {
                Input::Payloads(input) => {
                    if !Arc::ptr_eq(&input.origin, &self.origin) {
                        return Err(failure(HostError::InvalidArgument("file input belongs to another session".into())));
                    }
                    (input.payloads, input.slot)
                }
                Input::Paths(job, slot) => match job.await.map_err(|error| failure(HostError::backend(error)))? {
                    PreparedInput::Payloads(payloads) => (payloads, slot),
                    PreparedInput::Native(_) => return Err(failure(HostError::UnsupportedOperation(
                        "native directory uploads require a selected shared-filesystem and input-event completion path".into()))),
                },
            };
            // Preparation is complete before strict resolution. Keep this
            // source across retries, then never retry a submitted mutation.
            let element = self
                .resolver
                .resolve_for_file_input(frame, call.selector)
                .await
                .map_err(failure)?;
            self.upload
                .prepare_retaining(element, payloads, slot)
                .map_err(|error| failure(HostError::backend(error)))?
                .await
                .map(|_| ())
                .map_err(|error| failure(HostError::backend(error)))
        };
        if std::mem::size_of_val(&work) > profile::CALL_BYTES as usize - 4096 {
            return Box::pin(ready(Err(failure(HostError::ResourceExhausted(
                "file-input command exceeds admitted storage".into(),
            )))));
        }
        Box::pin(work)
    }
}
impl CommandCancellation<SetInputFiles> for Installation {
    type Future<'a> = Ready<Outcome>;
    fn cancel(&self, _: OperationContext, _: CancellationReason) -> Self::Future<'_> {
        // Acknowledges public cancellation, not native completion. The dropped
        // phase's pre-admitted owner continues cleanup and shutdown drains it.
        ready(Ok(()))
    }
}
impl HostSession {
    pub(crate) async fn locator_set_input_files(
        &self,
        frame: ObjectId,
        selector: Selector,
        timeout_ms: u32,
        input: impl FnOnce(&Installation) -> Result<Input, HostError>,
    ) -> Result<(), HostError> {
        let deadline_at =
            tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms));
        let (_, call_id, _operation) = self.admit_operation(&frame.get().to_string())?;
        let (runtime, installation) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            (live.runtime.clone(), live.file_input.clone())
        };
        let deadline = async {
            if timeout_ms == 0 {
                pending::<()>().await;
            }
            tokio::time::sleep_until(deadline_at).await;
        };
        let params = Params {
            selector,
            _lease: installation.admit()?,
            input: input(&installation)?,
        };
        let command =
            Command::<SetInputFiles>::new(call_id, frame, CallMetadata::default(), params);
        let dispatch = runtime.dispatch_with(command, installation.as_ref(), installation.clone());
        let operation = async {
            let result = match dispatch.await {
                Ok(result) => result,
                Err(never) => match never {},
            };
            result.into_outcome().map_err(|error| {
                let mut details = None;
                let error = error.map_details(|value| {
                    details = Some(value);
                });
                details.unwrap_or_else(|| HostError::operation(error, timeout_ms))
            })
        };
        let outcome = tokio::select! {
            biased;
            _ = self.stop.cancelled() => Err(HostError::TargetClosed),
            _ = deadline => Err(HostError::Timeout(timeout_ms)),
            outcome = operation => outcome,
        };
        crate::completion::publish_command_result(
            outcome,
            self.is_connected(),
            self.flush_topology(),
        )
        .await
    }
}
impl crate::locator::HostLocator {
    pub async fn set_input_files(
        &self,
        input: PreparedFileInput,
        timeout_ms: u32,
    ) -> Result<(), HostError> {
        let host = self.host()?;
        let (frame, selector, _query) = self.file_input_query()?;
        host.locator_set_input_files(frame, selector, timeout_ms, |_| Ok(Input::Payloads(input)))
            .await
    }
    /// Explicit base is supplied by the embedding boundary, not process globals.
    /// connectOverCDP uses Separate even for a loopback browser endpoint.
    pub async fn set_input_files_paths(
        &self,
        base: &Path,
        paths: &[&str],
        timeout_ms: u32,
    ) -> Result<(), HostError> {
        let host = self.host()?;
        let (frame, selector, _query) = self.file_input_query()?;
        host.locator_set_input_files(frame, selector, timeout_ms, |installation| {
            let slot = installation.source_slot()?;
            installation
                .paths(base, paths)
                .map(|job| Input::Paths(job, slot))
        })
        .await
    }
}
