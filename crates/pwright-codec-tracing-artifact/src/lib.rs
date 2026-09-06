//! Canonical bounded binary codec for tracing Artifact production.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use pwright_cap_tracing_artifact::{
    TraceArtifactResult, TraceStopChunkArtifact, TracingArtifactErrorDetails,
};
use pwright_codec_binary::{
    BinaryValueDecodeError, BinaryValueEncodeError, BinaryValueLimits, decode_value, encode_value,
};
use pwright_core::{
    CallId, CallMetadata, CancellationId, Command, CommandResult, CommandSpec, FailureKind,
    MethodTag, ObjectId, ObjectKindTag, OperationError,
};
use pwright_transport::{EncodedParts, FrameRoute, WireCodec, WireFrame};

const STOP_COMMAND: u8 = 1;
const STOP_RESULT: u8 = 2;

#[doc(hidden)]
#[rustfmt::skip]
pub mod generated_schema;

pub use generated_schema::{
    CATALOG as TRACING_ARTIFACT_SCHEMA_CATALOG, COMMANDS as TRACING_ARTIFACT_COMMAND_SCHEMAS,
    ROOT_SCHEMA as TRACING_ARTIFACT_SCHEMA,
};

use generated_schema::{
    PwrightCommonCallMetadata, PwrightCommonFailureKind, PwrightTracingArtifact,
    PwrightTracingArtifactArtifactResult, PwrightTracingArtifactError,
    PwrightTracingArtifactErrorDetailsRef, PwrightTracingArtifactErrorRef,
    PwrightTracingArtifactOutcome, PwrightTracingArtifactOutcomeRef, PwrightTracingArtifactRef,
    PwrightTracingArtifactStopOptions,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TracingArtifactCodecLimits {
    pub string_bytes: usize,
}

impl TracingArtifactCodecLimits {
    pub const fn new(string_bytes: usize) -> Self {
        Self { string_bytes }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TracingArtifactCodec {
    limits: TracingArtifactCodecLimits,
}

impl TracingArtifactCodec {
    pub const fn new(limits: TracingArtifactCodecLimits) -> Self {
        Self { limits }
    }

    pub const fn limits(self) -> TracingArtifactCodecLimits {
        self.limits
    }

    fn value_limits(self) -> BinaryValueLimits {
        BinaryValueLimits::new(self.limits.string_bytes, 0, 0, 8, 24)
    }
}

#[derive(Debug)]
pub enum TracingArtifactCodecError {
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

impl Display for TracingArtifactCodecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongRoute(route) => {
                write!(formatter, "tracing Artifact payload arrived on {route:?}")
            }
            Self::WrongMethod { expected, actual } => write!(
                formatter,
                "expected tracing Artifact method {expected}, received {actual}"
            ),
            Self::IdentityMismatch {
                field,
                outer,
                inner,
            } => write!(
                formatter,
                "tracing Artifact payload {field} {inner} does not match outer route {outer}"
            ),
            Self::AttachmentsUnsupported { actual } => write!(
                formatter,
                "tracing Artifact payload has {actual} attachment(s), but none are allowed"
            ),
            Self::ControlMetadataUnsupported => formatter.write_str(
                "tracing Artifact production does not accept timeout or cancellation metadata",
            ),
            Self::WrongPayloadKind { expected, actual } => write!(
                formatter,
                "expected tracing Artifact payload kind {expected}, received {actual}"
            ),
            Self::Encode(error) => Display::fmt(error, formatter),
            Self::Decode(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for TracingArtifactCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Encode(error) => Some(error),
            Self::Decode(error) => Some(error),
            _ => None,
        }
    }
}

impl WireCodec<Command<TraceStopChunkArtifact>> for TracingArtifactCodec {
    type Error = TracingArtifactCodecError;

    fn encode_parts(
        &self,
        command: &Command<TraceStopChunkArtifact>,
    ) -> Result<EncodedParts, Self::Error> {
        ensure_empty_metadata(command.metadata())?;
        let wire = PwrightTracingArtifactRef::TraceStopChunkArtifactCommand {
            call_id: command.call_id().get(),
            object_id: command.object_id().get(),
            metadata: empty_metadata(),
            options: PwrightTracingArtifactStopOptions {},
        };
        let payload =
            encode_value(&wire, self.value_limits()).map_err(TracingArtifactCodecError::Encode)?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: TraceStopChunkArtifact::METHOD,
            },
            payload,
            Vec::new(),
        ))
    }

    fn decode_frame(
        &self,
        frame: WireFrame,
    ) -> Result<Command<TraceStopChunkArtifact>, Self::Error> {
        let (outer_call_id, outer_object_id) = command_route::<TraceStopChunkArtifact>(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightTracingArtifact>(frame.payload(), self.value_limits())
            .map_err(TracingArtifactCodecError::Decode)?
        {
            PwrightTracingArtifact::TraceStopChunkArtifactCommand {
                call_id,
                object_id,
                metadata,
                ..
            } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                ensure_identity("object ID", outer_object_id.get(), object_id)?;
                let metadata = metadata_from_wire(metadata);
                ensure_empty_metadata(metadata)?;
                Ok(Command::new(
                    CallId::new(call_id),
                    ObjectId::new(object_id),
                    metadata,
                    (),
                ))
            }
            other => Err(TracingArtifactCodecError::WrongPayloadKind {
                expected: STOP_COMMAND,
                actual: wire_kind(&other),
            }),
        }
    }
}

impl WireCodec<CommandResult<TraceStopChunkArtifact>> for TracingArtifactCodec {
    type Error = TracingArtifactCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<TraceStopChunkArtifact>,
    ) -> Result<EncodedParts, Self::Error> {
        let outcome = match result.outcome() {
            Ok(result) => PwrightTracingArtifactOutcomeRef::Success {
                result: result_to_wire(*result),
            },
            Err(error) => PwrightTracingArtifactOutcomeRef::Error {
                error: operation_error_to_wire_ref(error),
            },
        };
        let wire = PwrightTracingArtifactRef::TraceStopChunkArtifactResult {
            call_id: result.call_id().get(),
            outcome,
        };
        let payload =
            encode_value(&wire, self.value_limits()).map_err(TracingArtifactCodecError::Encode)?;
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload,
            Vec::new(),
        ))
    }

    fn decode_frame(
        &self,
        frame: WireFrame,
    ) -> Result<CommandResult<TraceStopChunkArtifact>, Self::Error> {
        let outer_call_id = result_route(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightTracingArtifact>(frame.payload(), self.value_limits())
            .map_err(TracingArtifactCodecError::Decode)?
        {
            PwrightTracingArtifact::TraceStopChunkArtifactResult { call_id, outcome } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                let outcome = match outcome {
                    PwrightTracingArtifactOutcome::Success { result } => {
                        Ok(result_from_wire(result))
                    }
                    PwrightTracingArtifactOutcome::Error { error } => {
                        Err(operation_error_from_wire(error))
                    }
                };
                Ok(CommandResult::new(CallId::new(call_id), outcome))
            }
            other => Err(TracingArtifactCodecError::WrongPayloadKind {
                expected: STOP_RESULT,
                actual: wire_kind(&other),
            }),
        }
    }
}

fn result_to_wire(value: TraceArtifactResult) -> PwrightTracingArtifactArtifactResult {
    PwrightTracingArtifactArtifactResult {
        artifact_id: value.artifact_id.get(),
        parent_id: value.parent_id.get(),
        object_kind: value.object_kind.get(),
        chunk_id: value.chunk_id,
        entry_count: value.entry_count,
        payload_bytes: value.payload_bytes,
        encoded_bytes: value.encoded_bytes,
    }
}

fn result_from_wire(value: PwrightTracingArtifactArtifactResult) -> TraceArtifactResult {
    TraceArtifactResult::new(
        ObjectId::new(value.artifact_id),
        ObjectId::new(value.parent_id),
        ObjectKindTag::new(value.object_kind),
        value.chunk_id,
        value.entry_count,
        value.payload_bytes,
        value.encoded_bytes,
    )
}

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

fn ensure_empty_metadata(metadata: CallMetadata) -> Result<(), TracingArtifactCodecError> {
    if metadata == CallMetadata::default() {
        Ok(())
    } else {
        Err(TracingArtifactCodecError::ControlMetadataUnsupported)
    }
}

fn ensure_no_attachments(frame: &WireFrame) -> Result<(), TracingArtifactCodecError> {
    if frame.attachments().is_empty() {
        Ok(())
    } else {
        Err(TracingArtifactCodecError::AttachmentsUnsupported {
            actual: frame.attachments().len(),
        })
    }
}

fn ensure_identity(
    field: &'static str,
    outer: u64,
    inner: u64,
) -> Result<(), TracingArtifactCodecError> {
    if outer == inner {
        Ok(())
    } else {
        Err(TracingArtifactCodecError::IdentityMismatch {
            field,
            outer,
            inner,
        })
    }
}

fn command_route<S: CommandSpec>(
    frame: &WireFrame,
) -> Result<(CallId, ObjectId), TracingArtifactCodecError> {
    let FrameRoute::Command {
        call_id,
        target,
        method,
    } = frame.route()
    else {
        return Err(TracingArtifactCodecError::WrongRoute(frame.route()));
    };
    if method != S::METHOD {
        return Err(TracingArtifactCodecError::WrongMethod {
            expected: S::METHOD,
            actual: method,
        });
    }
    Ok((call_id, target))
}

fn result_route(frame: &WireFrame) -> Result<CallId, TracingArtifactCodecError> {
    let FrameRoute::Result { call_id } = frame.route() else {
        return Err(TracingArtifactCodecError::WrongRoute(frame.route()));
    };
    Ok(call_id)
}

fn wire_kind(value: &PwrightTracingArtifact) -> u8 {
    match value {
        PwrightTracingArtifact::TraceStopChunkArtifactCommand { .. } => STOP_COMMAND,
        PwrightTracingArtifact::TraceStopChunkArtifactResult { .. } => STOP_RESULT,
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

fn error_details_to_wire_ref(
    details: &TracingArtifactErrorDetails,
) -> PwrightTracingArtifactErrorDetailsRef<'_> {
    match details {
        TracingArtifactErrorDetails::InvalidState(message) => {
            PwrightTracingArtifactErrorDetailsRef::InvalidState { message }
        }
        TracingArtifactErrorDetails::Resource(message) => {
            PwrightTracingArtifactErrorDetailsRef::Resource { message }
        }
        TracingArtifactErrorDetails::Lifecycle(message) => {
            PwrightTracingArtifactErrorDetailsRef::Lifecycle { message }
        }
        TracingArtifactErrorDetails::Cleanup(message) => {
            PwrightTracingArtifactErrorDetailsRef::Cleanup { message }
        }
    }
}

fn operation_error_to_wire_ref(
    error: &OperationError<TracingArtifactErrorDetails>,
) -> PwrightTracingArtifactErrorRef<'_> {
    PwrightTracingArtifactErrorRef {
        failure_kind: failure_kind_to_wire(error.kind()),
        message: error.message(),
        details: error.details().map(error_details_to_wire_ref),
    }
}

fn operation_error_from_wire(
    error: PwrightTracingArtifactError,
) -> OperationError<TracingArtifactErrorDetails> {
    OperationError::new(
        failure_kind_from_wire(error.failure_kind),
        error.message,
        error.details.map(Into::into),
    )
}

#[cfg(test)]
mod tests {
    use pwright_cap_tracing_artifact::{
        TRACE_STOP_CHUNK_ARTIFACT_COMMAND_ID, TRACE_STOP_CHUNK_ARTIFACT_METHOD,
        TraceArtifactResult, TraceStopChunkArtifact, TracingArtifactErrorDetails,
    };
    use pwright_core::{
        CallId, CallMetadata, Command, CommandResult, FailureKind, ObjectId, ObjectKindTag,
        OperationError, ProtocolVersion,
    };
    use pwright_transport::{FrameLimits, VersionPolicy, WireProtocol};

    use super::{
        TRACING_ARTIFACT_COMMAND_SCHEMAS, TRACING_ARTIFACT_SCHEMA, TRACING_ARTIFACT_SCHEMA_CATALOG,
        TracingArtifactCodec, TracingArtifactCodecLimits,
    };

    fn protocol() -> WireProtocol<TracingArtifactCodec> {
        WireProtocol::new(
            TracingArtifactCodec::new(TracingArtifactCodecLimits::new(256)),
            VersionPolicy::exact(ProtocolVersion::new(1, 0)),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(1024, 0, 0, 1024),
        )
        .unwrap()
    }

    #[test]
    fn command_and_success_result_round_trip_exactly() {
        // Arrange
        let command = Command::<TraceStopChunkArtifact>::new(
            CallId::new(7),
            ObjectId::new(2),
            CallMetadata::default(),
            (),
        );
        let expected = TraceArtifactResult::new(
            ObjectId::new(3),
            ObjectId::new(2),
            ObjectKindTag::new(9),
            4,
            5,
            6,
            70,
        );

        // Act
        let command: Command<TraceStopChunkArtifact> = protocol()
            .decode(protocol().encode(&command).unwrap())
            .unwrap();
        let result = protocol()
            .decode::<CommandResult<TraceStopChunkArtifact>>(
                protocol()
                    .encode(&CommandResult::new(CallId::new(7), Ok(expected)))
                    .unwrap(),
            )
            .unwrap();

        // Assert
        assert_eq!(command.object_id(), ObjectId::new(2));
        assert_eq!(result.into_outcome(), Ok(expected));
    }

    #[test]
    fn typed_error_and_generated_identity_are_preserved() {
        // Arrange
        let error = OperationError::new(
            FailureKind::ResourceExhausted,
            "full",
            Some(TracingArtifactErrorDetails::Resource("bytes".to_owned())),
        );
        let result = CommandResult::<TraceStopChunkArtifact>::new(CallId::new(8), Err(error));

        // Act
        let decoded = protocol()
            .decode::<CommandResult<TraceStopChunkArtifact>>(protocol().encode(&result).unwrap())
            .unwrap();

        // Assert
        assert_eq!(decoded.into_outcome(), result.into_outcome());
        assert_eq!(
            TRACING_ARTIFACT_SCHEMA.version(),
            ProtocolVersion::new(1, 0)
        );
        assert_eq!(TRACING_ARTIFACT_SCHEMA_CATALOG.validate(), Ok(()));
        assert_eq!(
            TRACING_ARTIFACT_COMMAND_SCHEMAS[0].id(),
            TRACE_STOP_CHUNK_ARTIFACT_COMMAND_ID
        );
        assert_eq!(
            TRACING_ARTIFACT_COMMAND_SCHEMAS[0].method(),
            TRACE_STOP_CHUNK_ARTIFACT_METHOD
        );
    }
}
