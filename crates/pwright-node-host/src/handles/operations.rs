//! Retained-Handle value reads through the same controlled runtime as creation.

use std::{
    future::Future,
    future::{Ready, ready},
    pin::Pin,
};

use pwright_browser_model::{
    ELEMENT_HANDLE_OBJECT_KIND_ID, FRAME_OBJECT_KIND_ID, JS_HANDLE_OBJECT_KIND_ID,
};
use pwright_cap_handle_evaluation::{
    HandleEvaluate, HandleEvaluateParams, HandleJsonValue, JsonValueOptions,
};
use pwright_core::{
    CallId, CallMetadata, CancellationReason, Command, CommandCancellation, CommandSpec, MethodTag,
    ObjectId, ObjectKindId, OperationContext, OperationError, Service,
};
use pwright_runtime::TargetedService;

use super::{
    HandleEvaluationProgram,
    creation::{Installation, map_leaf_error},
    pending::{Allowance, PendingHandle},
};
use crate::{
    HostError, HostSession,
    evaluation::{self, EvaluationValue},
};

struct Read;
impl CommandSpec for Read {
    const METHOD: MethodTag = MethodTag::new(65004);
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = &[
        JS_HANDLE_OBJECT_KIND_ID,
        ELEMENT_HANDLE_OBJECT_KIND_ID,
        FRAME_OBJECT_KIND_ID,
    ];
    type Params = ReadParams;
    type Result = EvaluationValue;
    type ErrorDetails = HostError;
}
pub(super) struct ReadParams {
    call_id: CallId,
    operation: ReadOperation,
    allowance: Allowance,
}
enum ReadOperation {
    JsonValue,
    Evaluate(HandleEvaluateParams),
    FrameValue(HandleEvaluationProgram),
    HandleProgram(HandleEvaluationProgram),
}

#[derive(Clone, Copy)]
enum ReadTarget<'a> {
    Frame(&'a str),
    Handle(&'a PendingHandle),
}
impl TargetedService<ReadParams> for Installation {
    type Response = EvaluationValue;
    type Error = OperationError<HostError>;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<EvaluationValue, Self::Error>> + Send + 'a>>;

    fn call_for(&self, object: ObjectId, params: ReadParams) -> Self::Future<'_> {
        Box::pin(async move {
            let ReadParams {
                call_id,
                operation,
                allowance: _allowance,
            } = params;
            let result = match operation {
                ReadOperation::HandleProgram(params) => {
                    self.create
                        .evaluate_program_for_handle(object, CallMetadata::default(), params)
                        .await
                }
                ReadOperation::FrameValue(params) => {
                    self.create
                        .evaluate_program_for_frame(object, CallMetadata::default(), params)
                        .await
                }
                ReadOperation::JsonValue => self
                    .create
                    .call(Command::<HandleJsonValue>::new(
                        call_id,
                        object,
                        CallMetadata::default(),
                        JsonValueOptions,
                    ))
                    .await
                    .unwrap_or_else(|never| match never {})
                    .into_outcome(),
                ReadOperation::Evaluate(params) => self
                    .create
                    .call(Command::<HandleEvaluate>::new(
                        call_id,
                        object,
                        CallMetadata::default(),
                        params,
                    ))
                    .await
                    .unwrap_or_else(|never| match never {})
                    .into_outcome(),
            };
            result.map(|value| value.value).map_err(map_leaf_error)
        })
    }
}
impl CommandCancellation<Read> for Installation {
    type Future<'a> = Ready<Result<(), OperationError<HostError>>>;
    fn cancel(&self, _: OperationContext, _: CancellationReason) -> Self::Future<'_> {
        // Reads return by value and create no native remote-object owner.
        ready(Ok(()))
    }
}

impl HostSession {
    pub async fn handle_json_value(
        &self,
        handle: &PendingHandle,
    ) -> Result<EvaluationValue, HostError> {
        self.read_handle(handle, ReadOperation::JsonValue).await
    }

    pub async fn handle_evaluate(
        &self,
        handle: &PendingHandle,
        params: HandleEvaluateParams,
    ) -> Result<EvaluationValue, HostError> {
        if params.source().len() > evaluation::SOURCE_BYTES {
            return Err(HostError::InvalidArgument(
                "evaluation source exceeds 32768 UTF-8 bytes".into(),
            ));
        }
        if let Some(argument) = params.argument() {
            argument
                .validate(evaluation::value_limits())
                .map_err(|error| HostError::InvalidArgument(error.to_string()))?;
        }
        self.read_handle(handle, ReadOperation::Evaluate(params))
            .await
    }

    async fn read_handle(
        &self,
        handle: &PendingHandle,
        operation: ReadOperation,
    ) -> Result<EvaluationValue, HostError> {
        self.read_target(ReadTarget::Handle(handle), operation)
            .await
    }

    pub(super) async fn evaluate_frame_argument(
        &self,
        frame: &str,
        params: HandleEvaluationProgram,
    ) -> Result<EvaluationValue, HostError> {
        if params.source().len() > evaluation::SOURCE_BYTES {
            return Err(HostError::InvalidArgument(
                "evaluation source exceeds 32768 UTF-8 bytes".into(),
            ));
        }
        if let Some(argument) = params.argument() {
            argument
                .validate(evaluation::value_limits())
                .map_err(|error| HostError::InvalidArgument(error.to_string()))?;
        }
        self.read_target(ReadTarget::Frame(frame), ReadOperation::FrameValue(params))
            .await
    }

    pub async fn handle_evaluate_program(
        &self,
        handle: &PendingHandle,
        params: HandleEvaluationProgram,
    ) -> Result<EvaluationValue, HostError> {
        if params.source().len() > evaluation::SOURCE_BYTES {
            return Err(HostError::InvalidArgument(
                "evaluation source exceeds 32768 UTF-8 bytes".into(),
            ));
        }
        if let Some(argument) = params.argument() {
            argument
                .validate(evaluation::value_limits())
                .map_err(|error| HostError::InvalidArgument(error.to_string()))?;
        }
        self.read_handle(handle, ReadOperation::HandleProgram(params))
            .await
    }

    async fn read_target(
        &self,
        target: ReadTarget<'_>,
        operation: ReadOperation,
    ) -> Result<EvaluationValue, HostError> {
        let id = match target {
            ReadTarget::Frame(frame) => frame.to_owned(),
            ReadTarget::Handle(handle) => handle.initializer().object_id.get().to_string(),
        };
        let (object, call_id, _operation) = self.admit_operation(&id)?;
        let (runtime, installation) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            (live.runtime.clone(), live.handles.clone())
        };
        let allowance = match target {
            ReadTarget::Frame(_) => installation.owners.admit_call()?,
            ReadTarget::Handle(handle) => installation.owners.admit_read(handle)?,
        };
        let command = Command::<Read>::new(
            call_id,
            object,
            CallMetadata::default(),
            ReadParams {
                call_id,
                operation,
                allowance,
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
