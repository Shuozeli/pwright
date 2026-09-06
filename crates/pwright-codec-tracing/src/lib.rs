//! Canonical bounded binary codec for the optional tracing capability.
//!
//! Trace payloads are not attachments on this control protocol. A selected
//! sink owns chunk delivery independently from these lifecycle commands.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use pwright_cap_tracing::{
    TraceGroup, TraceGroupEnd, TraceGroupEndOptions, TraceGroupOptions, TraceLocation, TraceStart,
    TraceStartChunk, TraceStartChunkOptions, TraceStartChunkResult, TraceStartOptions, TraceStop,
    TraceStopChunk, TraceStopChunkMode, TraceStopChunkOptions, TraceStopChunkResult,
    TracingErrorDetails,
};
use pwright_codec_binary::{
    BinaryValueDecodeError, BinaryValueEncodeError, BinaryValueLimits, decode_value, encode_value,
};
use pwright_core::{
    CallId, CallMetadata, CancellationId, Command, CommandResult, CommandSpec, FailureKind,
    MethodTag, ObjectId, OperationError,
};
use pwright_transport::{EncodedParts, FrameRoute, WireCodec, WireFrame};

const START_COMMAND: u8 = 1;
const START_RESULT: u8 = 2;
const START_CHUNK_COMMAND: u8 = 3;
const START_CHUNK_RESULT: u8 = 4;
const GROUP_COMMAND: u8 = 5;
const GROUP_RESULT: u8 = 6;
const GROUP_END_COMMAND: u8 = 7;
const GROUP_END_RESULT: u8 = 8;
const STOP_CHUNK_COMMAND: u8 = 9;
const STOP_CHUNK_RESULT: u8 = 10;
const STOP_COMMAND: u8 = 11;
const STOP_RESULT: u8 = 12;

#[doc(hidden)]
#[rustfmt::skip]
pub mod generated_schema;

pub use generated_schema::{
    CATALOG as TRACING_SCHEMA_CATALOG, COMMANDS as TRACING_COMMAND_SCHEMAS,
    ROOT_SCHEMA as TRACING_SCHEMA,
};

use generated_schema::{
    PwrightCommonCallMetadata, PwrightCommonFailureKind, PwrightTracing, PwrightTracingError,
    PwrightTracingErrorDetailsRef, PwrightTracingErrorRef, PwrightTracingOutcome,
    PwrightTracingOutcomeRef, PwrightTracingRef, PwrightTracingTraceGroupEndOptions,
    PwrightTracingTraceGroupOptions, PwrightTracingTraceGroupOptionsRef,
    PwrightTracingTraceLocationRef, PwrightTracingTraceStartChunkOptions,
    PwrightTracingTraceStartChunkOptionsRef, PwrightTracingTraceStartChunkOutcome,
    PwrightTracingTraceStartChunkOutcomeRef, PwrightTracingTraceStartChunkSuccess,
    PwrightTracingTraceStartOptions, PwrightTracingTraceStartOptionsRef,
    PwrightTracingTraceStopChunkMode, PwrightTracingTraceStopChunkOptions,
    PwrightTracingTraceStopChunkOutcome, PwrightTracingTraceStopChunkOutcomeRef,
    PwrightTracingTraceStopChunkSuccess, PwrightTracingTraceStopOptions,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TracingCodecLimits {
    pub string_bytes: usize,
}

impl TracingCodecLimits {
    pub const fn new(string_bytes: usize) -> Self {
        Self { string_bytes }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TracingCodec {
    limits: TracingCodecLimits,
}

impl TracingCodec {
    pub const fn new(limits: TracingCodecLimits) -> Self {
        Self { limits }
    }

    pub const fn limits(self) -> TracingCodecLimits {
        self.limits
    }

    fn value_limits(self) -> BinaryValueLimits {
        BinaryValueLimits::new(self.limits.string_bytes, 0, 0, 16, 128)
    }
}

#[derive(Debug)]
pub enum TracingCodecError {
    WrongRoute(FrameRoute),
    WrongMethod {
        expected: MethodTag,
        actual: MethodTag,
    },
    IdentityMismatch {
        field: &'static str,
        outer: u64,
        inner: u64,
    },
    AttachmentsUnsupported {
        actual: usize,
    },
    ControlMetadataUnsupported,
    WrongPayloadKind {
        expected: u8,
        actual: u8,
    },
    Encode(BinaryValueEncodeError),
    Decode(BinaryValueDecodeError),
}

impl Display for TracingCodecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongRoute(route) => write!(formatter, "tracing payload arrived on {route:?}"),
            Self::WrongMethod { expected, actual } => {
                write!(
                    formatter,
                    "expected tracing method {expected}, received {actual}"
                )
            }
            Self::IdentityMismatch {
                field,
                outer,
                inner,
            } => write!(
                formatter,
                "tracing payload {field} {inner} does not match outer route {outer}"
            ),
            Self::AttachmentsUnsupported { actual } => write!(
                formatter,
                "tracing control payload has {actual} attachment(s), but none are allowed"
            ),
            Self::ControlMetadataUnsupported => formatter
                .write_str("tracing lifecycle commands do not accept timeout or cancellation"),
            Self::WrongPayloadKind { expected, actual } => write!(
                formatter,
                "expected tracing payload kind {expected}, received {actual}"
            ),
            Self::Encode(error) => Display::fmt(error, formatter),
            Self::Decode(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for TracingCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Encode(error) => Some(error),
            Self::Decode(error) => Some(error),
            _ => None,
        }
    }
}

macro_rules! impl_command_codec {
    ($spec:ty, $variant:ident, $kind:expr, $to_wire:ident, $from_wire:ident) => {
        impl WireCodec<Command<$spec>> for TracingCodec {
            type Error = TracingCodecError;

            fn encode_parts(&self, command: &Command<$spec>) -> Result<EncodedParts, Self::Error> {
                ensure_empty_metadata(command.metadata())?;
                let wire = PwrightTracingRef::$variant {
                    call_id: command.call_id().get(),
                    object_id: command.object_id().get(),
                    metadata: empty_metadata(),
                    options: $to_wire(command.params()),
                };
                encode_command::<$spec>(self, command, wire)
            }

            fn decode_frame(&self, frame: WireFrame) -> Result<Command<$spec>, Self::Error> {
                let (outer_call_id, outer_object_id) = command_route::<$spec>(&frame)?;
                ensure_no_attachments(&frame)?;
                match decode_value::<PwrightTracing>(frame.payload(), self.value_limits())
                    .map_err(TracingCodecError::Decode)?
                {
                    PwrightTracing::$variant {
                        call_id,
                        object_id,
                        metadata,
                        options,
                    } => {
                        ensure_identity("call ID", outer_call_id.get(), call_id)?;
                        ensure_identity("object ID", outer_object_id.get(), object_id)?;
                        let metadata = metadata_from_wire(metadata);
                        ensure_empty_metadata(metadata)?;
                        Ok(Command::new(
                            CallId::new(call_id),
                            ObjectId::new(object_id),
                            metadata,
                            $from_wire(options),
                        ))
                    }
                    other => Err(TracingCodecError::WrongPayloadKind {
                        expected: $kind,
                        actual: wire_kind(&other),
                    }),
                }
            }
        }
    };
}

macro_rules! impl_unit_result_codec {
    ($spec:ty, $variant:ident, $kind:expr) => {
        impl WireCodec<CommandResult<$spec>> for TracingCodec {
            type Error = TracingCodecError;

            fn encode_parts(
                &self,
                result: &CommandResult<$spec>,
            ) -> Result<EncodedParts, Self::Error> {
                let outcome = unit_outcome_to_wire(result.outcome());
                let wire = PwrightTracingRef::$variant {
                    call_id: result.call_id().get(),
                    outcome,
                };
                encode_result(self, result.call_id(), wire)
            }

            fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<$spec>, Self::Error> {
                let outer_call_id = result_route(&frame)?;
                ensure_no_attachments(&frame)?;
                match decode_value::<PwrightTracing>(frame.payload(), self.value_limits())
                    .map_err(TracingCodecError::Decode)?
                {
                    PwrightTracing::$variant { call_id, outcome } => {
                        ensure_identity("call ID", outer_call_id.get(), call_id)?;
                        Ok(CommandResult::new(
                            CallId::new(call_id),
                            unit_outcome_from_wire(outcome),
                        ))
                    }
                    other => Err(TracingCodecError::WrongPayloadKind {
                        expected: $kind,
                        actual: wire_kind(&other),
                    }),
                }
            }
        }
    };
}

impl_command_codec!(
    TraceStart,
    TraceStartCommand,
    START_COMMAND,
    start_options_to_wire,
    start_options_from_wire
);
impl_command_codec!(
    TraceStartChunk,
    TraceStartChunkCommand,
    START_CHUNK_COMMAND,
    start_chunk_options_to_wire,
    start_chunk_options_from_wire
);
impl_command_codec!(
    TraceGroup,
    TraceGroupCommand,
    GROUP_COMMAND,
    group_options_to_wire,
    group_options_from_wire
);
impl_command_codec!(
    TraceGroupEnd,
    TraceGroupEndCommand,
    GROUP_END_COMMAND,
    group_end_options_to_wire,
    group_end_options_from_wire
);
impl_command_codec!(
    TraceStopChunk,
    TraceStopChunkCommand,
    STOP_CHUNK_COMMAND,
    stop_chunk_options_to_wire,
    stop_chunk_options_from_wire
);
impl_command_codec!(
    TraceStop,
    TraceStopCommand,
    STOP_COMMAND,
    stop_options_to_wire,
    stop_options_from_wire
);

impl_unit_result_codec!(TraceStart, TraceStartResult, START_RESULT);
impl_unit_result_codec!(TraceGroup, TraceGroupResult, GROUP_RESULT);
impl_unit_result_codec!(TraceGroupEnd, TraceGroupEndResult, GROUP_END_RESULT);
impl_unit_result_codec!(TraceStop, TraceStopResult, STOP_RESULT);

impl WireCodec<CommandResult<TraceStartChunk>> for TracingCodec {
    type Error = TracingCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<TraceStartChunk>,
    ) -> Result<EncodedParts, Self::Error> {
        let outcome = match result.outcome() {
            Ok(result) => PwrightTracingTraceStartChunkOutcomeRef::Success {
                result: PwrightTracingTraceStartChunkSuccess {
                    chunk_id: result.chunk_id(),
                },
            },
            Err(error) => PwrightTracingTraceStartChunkOutcomeRef::Error {
                error: operation_error_to_wire_ref(error),
            },
        };
        encode_result(
            self,
            result.call_id(),
            PwrightTracingRef::TraceStartChunkResult {
                call_id: result.call_id().get(),
                outcome,
            },
        )
    }

    fn decode_frame(
        &self,
        frame: WireFrame,
    ) -> Result<CommandResult<TraceStartChunk>, Self::Error> {
        let outer_call_id = result_route(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightTracing>(frame.payload(), self.value_limits())
            .map_err(TracingCodecError::Decode)?
        {
            PwrightTracing::TraceStartChunkResult { call_id, outcome } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                let outcome = match outcome {
                    PwrightTracingTraceStartChunkOutcome::Success { result } => {
                        Ok(TraceStartChunkResult::new(result.chunk_id))
                    }
                    PwrightTracingTraceStartChunkOutcome::Error { error } => {
                        Err(operation_error_from_wire(error))
                    }
                };
                Ok(CommandResult::new(CallId::new(call_id), outcome))
            }
            other => Err(TracingCodecError::WrongPayloadKind {
                expected: START_CHUNK_RESULT,
                actual: wire_kind(&other),
            }),
        }
    }
}

impl WireCodec<CommandResult<TraceStopChunk>> for TracingCodec {
    type Error = TracingCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<TraceStopChunk>,
    ) -> Result<EncodedParts, Self::Error> {
        let outcome = match result.outcome() {
            Ok(result) => PwrightTracingTraceStopChunkOutcomeRef::Success {
                result: PwrightTracingTraceStopChunkSuccess {
                    chunk_id: result.chunk_id(),
                    entry_count: result.entry_count(),
                    payload_bytes: result.payload_bytes(),
                },
            },
            Err(error) => PwrightTracingTraceStopChunkOutcomeRef::Error {
                error: operation_error_to_wire_ref(error),
            },
        };
        encode_result(
            self,
            result.call_id(),
            PwrightTracingRef::TraceStopChunkResult {
                call_id: result.call_id().get(),
                outcome,
            },
        )
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<TraceStopChunk>, Self::Error> {
        let outer_call_id = result_route(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightTracing>(frame.payload(), self.value_limits())
            .map_err(TracingCodecError::Decode)?
        {
            PwrightTracing::TraceStopChunkResult { call_id, outcome } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                let outcome = match outcome {
                    PwrightTracingTraceStopChunkOutcome::Success { result } => {
                        Ok(TraceStopChunkResult::new(
                            result.chunk_id,
                            result.entry_count,
                            result.payload_bytes,
                        ))
                    }
                    PwrightTracingTraceStopChunkOutcome::Error { error } => {
                        Err(operation_error_from_wire(error))
                    }
                };
                Ok(CommandResult::new(CallId::new(call_id), outcome))
            }
            other => Err(TracingCodecError::WrongPayloadKind {
                expected: STOP_CHUNK_RESULT,
                actual: wire_kind(&other),
            }),
        }
    }
}

fn encode_command<S: CommandSpec>(
    codec: &TracingCodec,
    command: &Command<S>,
    wire: PwrightTracingRef<'_>,
) -> Result<EncodedParts, TracingCodecError> {
    let payload = encode_value(&wire, codec.value_limits()).map_err(TracingCodecError::Encode)?;
    Ok(EncodedParts::new(
        FrameRoute::Command {
            call_id: command.call_id(),
            target: command.object_id(),
            method: S::METHOD,
        },
        payload,
        Vec::new(),
    ))
}

fn encode_result(
    codec: &TracingCodec,
    call_id: CallId,
    wire: PwrightTracingRef<'_>,
) -> Result<EncodedParts, TracingCodecError> {
    let payload = encode_value(&wire, codec.value_limits()).map_err(TracingCodecError::Encode)?;
    Ok(EncodedParts::new(
        FrameRoute::Result { call_id },
        payload,
        Vec::new(),
    ))
}

fn start_options_to_wire(options: &TraceStartOptions) -> PwrightTracingTraceStartOptionsRef<'_> {
    PwrightTracingTraceStartOptionsRef {
        name: options.name(),
    }
}

fn start_options_from_wire(options: PwrightTracingTraceStartOptions) -> TraceStartOptions {
    TraceStartOptions::new(options.name)
}

fn start_chunk_options_to_wire(
    options: &TraceStartChunkOptions,
) -> PwrightTracingTraceStartChunkOptionsRef<'_> {
    PwrightTracingTraceStartChunkOptionsRef {
        name: options.name(),
        title: options.title(),
    }
}

fn start_chunk_options_from_wire(
    options: PwrightTracingTraceStartChunkOptions,
) -> TraceStartChunkOptions {
    TraceStartChunkOptions::new(options.name, options.title)
}

fn group_options_to_wire(options: &TraceGroupOptions) -> PwrightTracingTraceGroupOptionsRef<'_> {
    PwrightTracingTraceGroupOptionsRef {
        name: options.name(),
        location: options
            .location()
            .map(|location| PwrightTracingTraceLocationRef {
                file: location.file(),
                line: location.line(),
                column: location.column(),
            }),
    }
}

fn group_options_from_wire(options: PwrightTracingTraceGroupOptions) -> TraceGroupOptions {
    TraceGroupOptions::new(
        options.name,
        options
            .location
            .map(|location| TraceLocation::new(location.file, location.line, location.column)),
    )
}

fn group_end_options_to_wire(_: &TraceGroupEndOptions) -> PwrightTracingTraceGroupEndOptions {
    PwrightTracingTraceGroupEndOptions {}
}

fn group_end_options_from_wire(_: PwrightTracingTraceGroupEndOptions) -> TraceGroupEndOptions {}

fn stop_chunk_options_to_wire(
    options: &TraceStopChunkOptions,
) -> PwrightTracingTraceStopChunkOptions {
    PwrightTracingTraceStopChunkOptions {
        mode: match options.mode() {
            TraceStopChunkMode::Discard => PwrightTracingTraceStopChunkMode::Discard,
            TraceStopChunkMode::Sink => PwrightTracingTraceStopChunkMode::Sink,
        },
    }
}

fn stop_chunk_options_from_wire(
    options: PwrightTracingTraceStopChunkOptions,
) -> TraceStopChunkOptions {
    TraceStopChunkOptions::new(match options.mode {
        PwrightTracingTraceStopChunkMode::Discard => TraceStopChunkMode::Discard,
        PwrightTracingTraceStopChunkMode::Sink => TraceStopChunkMode::Sink,
    })
}

fn stop_options_to_wire(_: &()) -> PwrightTracingTraceStopOptions {
    PwrightTracingTraceStopOptions {}
}

fn stop_options_from_wire(_: PwrightTracingTraceStopOptions) {}

fn empty_metadata() -> PwrightCommonCallMetadata {
    PwrightCommonCallMetadata {
        timeout: None,
        cancellation_id: None,
    }
}

fn metadata_from_wire(metadata: PwrightCommonCallMetadata) -> CallMetadata {
    let timeout = metadata
        .timeout
        .map(|timeout| std::time::Duration::new(timeout.seconds, timeout.nanoseconds));
    CallMetadata::new(timeout, metadata.cancellation_id.map(CancellationId::new))
}

fn ensure_empty_metadata(metadata: CallMetadata) -> Result<(), TracingCodecError> {
    if metadata == CallMetadata::default() {
        Ok(())
    } else {
        Err(TracingCodecError::ControlMetadataUnsupported)
    }
}

fn ensure_no_attachments(frame: &WireFrame) -> Result<(), TracingCodecError> {
    if frame.attachments().is_empty() {
        Ok(())
    } else {
        Err(TracingCodecError::AttachmentsUnsupported {
            actual: frame.attachments().len(),
        })
    }
}

fn ensure_identity(field: &'static str, outer: u64, inner: u64) -> Result<(), TracingCodecError> {
    if outer == inner {
        Ok(())
    } else {
        Err(TracingCodecError::IdentityMismatch {
            field,
            outer,
            inner,
        })
    }
}

fn command_route<S: CommandSpec>(
    frame: &WireFrame,
) -> Result<(CallId, ObjectId), TracingCodecError> {
    let FrameRoute::Command {
        call_id,
        target,
        method,
    } = frame.route()
    else {
        return Err(TracingCodecError::WrongRoute(frame.route()));
    };
    if method != S::METHOD {
        return Err(TracingCodecError::WrongMethod {
            expected: S::METHOD,
            actual: method,
        });
    }
    Ok((call_id, target))
}

fn result_route(frame: &WireFrame) -> Result<CallId, TracingCodecError> {
    let FrameRoute::Result { call_id } = frame.route() else {
        return Err(TracingCodecError::WrongRoute(frame.route()));
    };
    Ok(call_id)
}

fn wire_kind(value: &PwrightTracing) -> u8 {
    match value {
        PwrightTracing::TraceStartCommand { .. } => START_COMMAND,
        PwrightTracing::TraceStartResult { .. } => START_RESULT,
        PwrightTracing::TraceStartChunkCommand { .. } => START_CHUNK_COMMAND,
        PwrightTracing::TraceStartChunkResult { .. } => START_CHUNK_RESULT,
        PwrightTracing::TraceGroupCommand { .. } => GROUP_COMMAND,
        PwrightTracing::TraceGroupResult { .. } => GROUP_RESULT,
        PwrightTracing::TraceGroupEndCommand { .. } => GROUP_END_COMMAND,
        PwrightTracing::TraceGroupEndResult { .. } => GROUP_END_RESULT,
        PwrightTracing::TraceStopChunkCommand { .. } => STOP_CHUNK_COMMAND,
        PwrightTracing::TraceStopChunkResult { .. } => STOP_CHUNK_RESULT,
        PwrightTracing::TraceStopCommand { .. } => STOP_COMMAND,
        PwrightTracing::TraceStopResult { .. } => STOP_RESULT,
    }
}

fn unit_outcome_to_wire(
    outcome: &Result<(), OperationError<TracingErrorDetails>>,
) -> PwrightTracingOutcomeRef<'_> {
    match outcome {
        Ok(()) => PwrightTracingOutcomeRef::Success,
        Err(error) => PwrightTracingOutcomeRef::Error {
            error: operation_error_to_wire_ref(error),
        },
    }
}

fn unit_outcome_from_wire(
    outcome: PwrightTracingOutcome,
) -> Result<(), OperationError<TracingErrorDetails>> {
    match outcome {
        PwrightTracingOutcome::Success => Ok(()),
        PwrightTracingOutcome::Error { error } => Err(operation_error_from_wire(error)),
    }
}

fn failure_kind_to_wire(value: FailureKind) -> PwrightCommonFailureKind {
    match value {
        FailureKind::Timeout => PwrightCommonFailureKind::Timeout,
        FailureKind::Cancelled => PwrightCommonFailureKind::Cancelled,
        FailureKind::TargetClosed => PwrightCommonFailureKind::TargetClosed,
        FailureKind::BrowserCrashed => PwrightCommonFailureKind::BrowserCrashed,
        FailureKind::UnsupportedCapability => PwrightCommonFailureKind::UnsupportedCapability,
        FailureKind::ResourceExhausted => PwrightCommonFailureKind::ResourceExhausted,
        FailureKind::TransportClosed => PwrightCommonFailureKind::TransportClosed,
        FailureKind::ProtocolViolation => PwrightCommonFailureKind::ProtocolViolation,
        FailureKind::Backend => PwrightCommonFailureKind::Backend,
        FailureKind::Cleanup => PwrightCommonFailureKind::Cleanup,
    }
}

fn failure_kind_from_wire(value: PwrightCommonFailureKind) -> FailureKind {
    match value {
        PwrightCommonFailureKind::Timeout => FailureKind::Timeout,
        PwrightCommonFailureKind::Cancelled => FailureKind::Cancelled,
        PwrightCommonFailureKind::TargetClosed => FailureKind::TargetClosed,
        PwrightCommonFailureKind::BrowserCrashed => FailureKind::BrowserCrashed,
        PwrightCommonFailureKind::UnsupportedCapability => FailureKind::UnsupportedCapability,
        PwrightCommonFailureKind::ResourceExhausted => FailureKind::ResourceExhausted,
        PwrightCommonFailureKind::TransportClosed => FailureKind::TransportClosed,
        PwrightCommonFailureKind::ProtocolViolation => FailureKind::ProtocolViolation,
        PwrightCommonFailureKind::Backend => FailureKind::Backend,
        PwrightCommonFailureKind::Cleanup => FailureKind::Cleanup,
    }
}

fn error_details_to_wire_ref(details: &TracingErrorDetails) -> PwrightTracingErrorDetailsRef<'_> {
    match details {
        TracingErrorDetails::InvalidState(message) => {
            PwrightTracingErrorDetailsRef::InvalidState { message }
        }
        TracingErrorDetails::InvalidOptions(message) => {
            PwrightTracingErrorDetailsRef::InvalidOptions { message }
        }
        TracingErrorDetails::Resource(message) => {
            PwrightTracingErrorDetailsRef::Resource { message }
        }
        TracingErrorDetails::Sink(message) => PwrightTracingErrorDetailsRef::Sink { message },
    }
}

fn operation_error_to_wire_ref(
    error: &OperationError<TracingErrorDetails>,
) -> PwrightTracingErrorRef<'_> {
    PwrightTracingErrorRef {
        failure_kind: failure_kind_to_wire(error.kind()),
        message: error.message(),
        details: error.details().map(error_details_to_wire_ref),
    }
}

fn operation_error_from_wire(error: PwrightTracingError) -> OperationError<TracingErrorDetails> {
    OperationError::new(
        failure_kind_from_wire(error.failure_kind),
        error.message,
        error.details.map(Into::into),
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use pwright_cap_tracing::{
        TraceGroup, TraceGroupOptions, TraceLocation, TraceStart, TraceStartChunk,
        TraceStartChunkOptions, TraceStartChunkResult, TraceStartOptions, TraceStopChunk,
        TraceStopChunkMode, TraceStopChunkOptions, TraceStopChunkResult, TracingErrorDetails,
    };
    use pwright_core::{
        CallId, CallMetadata, Command, CommandResult, FailureKind, ObjectId, OperationError,
        ProtocolVersion,
    };
    use pwright_transport::{FrameLimits, VersionPolicy, WireCodec, WireProtocol};

    use super::{TracingCodec, TracingCodecError, TracingCodecLimits};

    fn protocol() -> WireProtocol<TracingCodec> {
        WireProtocol::new(
            TracingCodec::new(TracingCodecLimits::new(256)),
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0)).unwrap(),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(4096, 0, 0, 4096),
        )
        .unwrap()
    }

    #[test]
    fn start_chunk_and_group_commands_round_trip_exact_options() {
        // Arrange
        let protocol = protocol();
        let start = Command::<TraceStartChunk>::new(
            CallId::new(7),
            ObjectId::new(11),
            CallMetadata::default(),
            TraceStartChunkOptions::new(Some("chunk".into()), Some("Checkout".into())),
        );
        let group = Command::<TraceGroup>::new(
            CallId::new(8),
            ObjectId::new(11),
            CallMetadata::default(),
            TraceGroupOptions::new(
                "submit",
                Some(TraceLocation::new("flow.rs", Some(12), Some(4))),
            ),
        );

        // Act
        let start = protocol
            .decode::<Command<TraceStartChunk>>(protocol.encode(&start).unwrap())
            .unwrap();
        let group = protocol
            .decode::<Command<TraceGroup>>(protocol.encode(&group).unwrap())
            .unwrap();

        // Assert
        assert_eq!(start.params().name(), Some("chunk"));
        assert_eq!(start.params().title(), Some("Checkout"));
        assert_eq!(group.params().name(), "submit");
        assert_eq!(group.params().location().unwrap().file(), "flow.rs");
    }

    #[test]
    fn typed_chunk_results_and_sink_mode_round_trip() {
        // Arrange
        let protocol = protocol();
        let mode = Command::<TraceStopChunk>::new(
            CallId::new(9),
            ObjectId::new(11),
            CallMetadata::default(),
            TraceStopChunkOptions::new(TraceStopChunkMode::Sink),
        );
        let started = CommandResult::<TraceStartChunk>::new(
            CallId::new(7),
            Ok(TraceStartChunkResult::new(3)),
        );
        let stopped = CommandResult::<TraceStopChunk>::new(
            CallId::new(9),
            Ok(TraceStopChunkResult::new(3, 14, 512)),
        );

        // Act
        let mode = protocol
            .decode::<Command<TraceStopChunk>>(protocol.encode(&mode).unwrap())
            .unwrap();
        let started = protocol
            .decode::<CommandResult<TraceStartChunk>>(protocol.encode(&started).unwrap())
            .unwrap();
        let stopped = protocol
            .decode::<CommandResult<TraceStopChunk>>(protocol.encode(&stopped).unwrap())
            .unwrap();

        // Assert
        assert_eq!(mode.params().mode(), TraceStopChunkMode::Sink);
        assert_eq!(started.outcome().as_ref().unwrap().chunk_id(), 3);
        assert_eq!(stopped.outcome().as_ref().unwrap().entry_count(), 14);
        assert_eq!(stopped.outcome().as_ref().unwrap().payload_bytes(), 512);
    }

    #[test]
    fn unit_failure_preserves_typed_diagnostic() {
        // Arrange
        let protocol = protocol();
        let result = CommandResult::<TraceStart>::new(
            CallId::new(2),
            Err(OperationError::new(
                FailureKind::ResourceExhausted,
                "trace limit reached",
                Some(TracingErrorDetails::Resource("TraceBytes".into())),
            )),
        );

        // Act
        let decoded = protocol
            .decode::<CommandResult<TraceStart>>(protocol.encode(&result).unwrap())
            .unwrap();

        // Assert
        assert_eq!(decoded.outcome(), result.outcome());
    }

    #[test]
    fn tracing_rejects_timeout_metadata() {
        // Arrange
        let command = Command::<TraceStart>::new(
            CallId::new(2),
            ObjectId::new(11),
            CallMetadata::new(Some(Duration::from_secs(1)), None),
            TraceStartOptions::default(),
        );

        // Act
        let error = <TracingCodec as WireCodec<Command<TraceStart>>>::encode_parts(
            &TracingCodec::new(TracingCodecLimits::new(256)),
            &command,
        )
        .unwrap_err();

        // Assert
        assert!(matches!(
            error,
            TracingCodecError::ControlMetadataUnsupported
        ));
    }
}
