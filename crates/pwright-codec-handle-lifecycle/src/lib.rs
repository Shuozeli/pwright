//! Canonical bounded binary codec for browser-handle lifecycle commands.
//!
//! The codec rejects timeout/cancellation metadata because explicit disposal
//! is a no-timeout lifecycle barrier, not a normal action.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use pwright_cap_handle_lifecycle::{HandleDispose, HandleLifecycleErrorDetails};
use pwright_codec_binary::{
    BinaryValueDecodeError, BinaryValueEncodeError, BinaryValueLimits, decode_value, encode_value,
};
use pwright_core::{
    CallId, CallMetadata, CancellationId, Command, CommandResult, CommandSpec, FailureKind,
    MethodTag, ObjectId, OperationError,
};
use pwright_transport::{EncodedParts, FrameRoute, WireCodec, WireFrame};

const DISPOSE_COMMAND: u8 = 1;
const DISPOSE_RESULT: u8 = 2;

#[doc(hidden)]
#[rustfmt::skip]
pub mod generated_schema;

pub use generated_schema::{
    CATALOG as HANDLE_LIFECYCLE_SCHEMA_CATALOG, COMMANDS as HANDLE_LIFECYCLE_COMMAND_SCHEMAS,
    ROOT_SCHEMA as HANDLE_LIFECYCLE_SCHEMA,
};

use generated_schema::{
    PwrightCommonCallMetadata, PwrightCommonFailureKind, PwrightHandleLifecycle,
    PwrightHandleLifecycleError, PwrightHandleLifecycleErrorDetailsRef,
    PwrightHandleLifecycleErrorRef, PwrightHandleLifecycleHandleDisposeOptions,
    PwrightHandleLifecycleOutcome, PwrightHandleLifecycleOutcomeRef, PwrightHandleLifecycleRef,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HandleLifecycleCodecLimits {
    pub string_bytes: usize,
}

impl HandleLifecycleCodecLimits {
    pub const fn new(string_bytes: usize) -> Self {
        Self { string_bytes }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HandleLifecycleCodec {
    limits: HandleLifecycleCodecLimits,
}

impl HandleLifecycleCodec {
    pub const fn new(limits: HandleLifecycleCodecLimits) -> Self {
        Self { limits }
    }

    pub const fn limits(self) -> HandleLifecycleCodecLimits {
        self.limits
    }

    fn value_limits(self) -> BinaryValueLimits {
        BinaryValueLimits::new(self.limits.string_bytes, 0, 0, 8, 16)
    }
}

#[derive(Debug)]
pub enum HandleLifecycleCodecError {
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

impl Display for HandleLifecycleCodecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongRoute(route) => {
                write!(formatter, "handle lifecycle payload arrived on {route:?}")
            }
            Self::WrongMethod { expected, actual } => write!(
                formatter,
                "expected handle lifecycle method {expected}, received {actual}"
            ),
            Self::IdentityMismatch {
                field,
                outer,
                inner,
            } => write!(
                formatter,
                "handle lifecycle payload {field} {inner} does not match outer route {outer}"
            ),
            Self::AttachmentsUnsupported { actual } => write!(
                formatter,
                "handle lifecycle payload has {actual} attachment(s), but none are allowed"
            ),
            Self::ControlMetadataUnsupported => formatter
                .write_str("handle disposal does not accept timeout or cancellation metadata"),
            Self::WrongPayloadKind { expected, actual } => write!(
                formatter,
                "expected handle lifecycle payload kind {expected}, received {actual}"
            ),
            Self::Encode(error) => Display::fmt(error, formatter),
            Self::Decode(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for HandleLifecycleCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Encode(error) => Some(error),
            Self::Decode(error) => Some(error),
            _ => None,
        }
    }
}

impl WireCodec<Command<HandleDispose>> for HandleLifecycleCodec {
    type Error = HandleLifecycleCodecError;

    fn encode_parts(&self, command: &Command<HandleDispose>) -> Result<EncodedParts, Self::Error> {
        ensure_empty_metadata(command.metadata())?;
        let wire = PwrightHandleLifecycleRef::HandleDisposeCommand {
            call_id: command.call_id().get(),
            object_id: command.object_id().get(),
            metadata: empty_metadata(),
            options: PwrightHandleLifecycleHandleDisposeOptions {},
        };
        let payload =
            encode_value(&wire, self.value_limits()).map_err(HandleLifecycleCodecError::Encode)?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: HandleDispose::METHOD,
            },
            payload,
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<HandleDispose>, Self::Error> {
        let (outer_call_id, outer_object_id) = command_route::<HandleDispose>(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightHandleLifecycle>(frame.payload(), self.value_limits())
            .map_err(HandleLifecycleCodecError::Decode)?
        {
            PwrightHandleLifecycle::HandleDisposeCommand {
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
            other => Err(HandleLifecycleCodecError::WrongPayloadKind {
                expected: DISPOSE_COMMAND,
                actual: wire_kind(&other),
            }),
        }
    }
}

impl WireCodec<CommandResult<HandleDispose>> for HandleLifecycleCodec {
    type Error = HandleLifecycleCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<HandleDispose>,
    ) -> Result<EncodedParts, Self::Error> {
        let outcome = match result.outcome() {
            Ok(()) => PwrightHandleLifecycleOutcomeRef::Success,
            Err(error) => PwrightHandleLifecycleOutcomeRef::Error {
                error: operation_error_to_wire_ref(error),
            },
        };
        let wire = PwrightHandleLifecycleRef::HandleDisposeResult {
            call_id: result.call_id().get(),
            outcome,
        };
        let payload =
            encode_value(&wire, self.value_limits()).map_err(HandleLifecycleCodecError::Encode)?;
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload,
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<HandleDispose>, Self::Error> {
        let outer_call_id = result_route(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightHandleLifecycle>(frame.payload(), self.value_limits())
            .map_err(HandleLifecycleCodecError::Decode)?
        {
            PwrightHandleLifecycle::HandleDisposeResult { call_id, outcome } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                let outcome = match outcome {
                    PwrightHandleLifecycleOutcome::Success => Ok(()),
                    PwrightHandleLifecycleOutcome::Error { error } => {
                        Err(operation_error_from_wire(error))
                    }
                };
                Ok(CommandResult::new(CallId::new(call_id), outcome))
            }
            other => Err(HandleLifecycleCodecError::WrongPayloadKind {
                expected: DISPOSE_RESULT,
                actual: wire_kind(&other),
            }),
        }
    }
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

fn ensure_empty_metadata(metadata: CallMetadata) -> Result<(), HandleLifecycleCodecError> {
    if metadata == CallMetadata::default() {
        Ok(())
    } else {
        Err(HandleLifecycleCodecError::ControlMetadataUnsupported)
    }
}

fn ensure_no_attachments(frame: &WireFrame) -> Result<(), HandleLifecycleCodecError> {
    if frame.attachments().is_empty() {
        Ok(())
    } else {
        Err(HandleLifecycleCodecError::AttachmentsUnsupported {
            actual: frame.attachments().len(),
        })
    }
}

fn ensure_identity(
    field: &'static str,
    outer: u64,
    inner: u64,
) -> Result<(), HandleLifecycleCodecError> {
    if outer == inner {
        Ok(())
    } else {
        Err(HandleLifecycleCodecError::IdentityMismatch {
            field,
            outer,
            inner,
        })
    }
}

fn command_route<S: CommandSpec>(
    frame: &WireFrame,
) -> Result<(CallId, ObjectId), HandleLifecycleCodecError> {
    let FrameRoute::Command {
        call_id,
        target,
        method,
    } = frame.route()
    else {
        return Err(HandleLifecycleCodecError::WrongRoute(frame.route()));
    };
    if method != S::METHOD {
        return Err(HandleLifecycleCodecError::WrongMethod {
            expected: S::METHOD,
            actual: method,
        });
    }
    Ok((call_id, target))
}

fn result_route(frame: &WireFrame) -> Result<CallId, HandleLifecycleCodecError> {
    let FrameRoute::Result { call_id } = frame.route() else {
        return Err(HandleLifecycleCodecError::WrongRoute(frame.route()));
    };
    Ok(call_id)
}

fn wire_kind(value: &PwrightHandleLifecycle) -> u8 {
    match value {
        PwrightHandleLifecycle::HandleDisposeCommand { .. } => DISPOSE_COMMAND,
        PwrightHandleLifecycle::HandleDisposeResult { .. } => DISPOSE_RESULT,
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
    details: &HandleLifecycleErrorDetails,
) -> PwrightHandleLifecycleErrorDetailsRef<'_> {
    match details {
        HandleLifecycleErrorDetails::NotOwned(message) => {
            PwrightHandleLifecycleErrorDetailsRef::NotOwned { message }
        }
        HandleLifecycleErrorDetails::Backend(message) => {
            PwrightHandleLifecycleErrorDetailsRef::Backend { message }
        }
        HandleLifecycleErrorDetails::Cleanup { primary, cleanup } => {
            PwrightHandleLifecycleErrorDetailsRef::Cleanup { primary, cleanup }
        }
    }
}

fn operation_error_to_wire_ref(
    error: &OperationError<HandleLifecycleErrorDetails>,
) -> PwrightHandleLifecycleErrorRef<'_> {
    PwrightHandleLifecycleErrorRef {
        failure_kind: failure_kind_to_wire(error.kind()),
        message: error.message(),
        details: error.details().map(error_details_to_wire_ref),
    }
}

fn operation_error_from_wire(
    error: PwrightHandleLifecycleError,
) -> OperationError<HandleLifecycleErrorDetails> {
    OperationError::new(
        failure_kind_from_wire(error.failure_kind),
        error.message,
        error.details.map(Into::into),
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use pwright_cap_handle_lifecycle::{
        HANDLE_DISPOSE_COMMAND_ID, HANDLE_DISPOSE_METHOD, HandleDispose,
        HandleLifecycleErrorDetails,
    };
    use pwright_core::{
        CallId, CallMetadata, Command, CommandResult, FailureKind, ObjectId, OperationError,
        ProtocolVersion,
    };
    use pwright_transport::{
        FrameLimits, FrameRoute, VersionPolicy, WireCodec, WireFrame, WireProtocol,
    };

    use super::{
        HANDLE_LIFECYCLE_COMMAND_SCHEMAS, HANDLE_LIFECYCLE_SCHEMA, HANDLE_LIFECYCLE_SCHEMA_CATALOG,
        HandleLifecycleCodec, HandleLifecycleCodecError, HandleLifecycleCodecLimits,
    };

    fn codec() -> HandleLifecycleCodec {
        HandleLifecycleCodec::new(HandleLifecycleCodecLimits::new(256))
    }

    fn protocol() -> WireProtocol<HandleLifecycleCodec> {
        WireProtocol::new(
            codec(),
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0))
                .expect("version policy should build"),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(1024, 0, 0, 1024),
        )
        .expect("protocol should build")
    }

    #[test]
    fn dispose_command_round_trips_with_exact_route_identity() {
        // Arrange
        let protocol = protocol();
        let command = Command::<HandleDispose>::new(
            CallId::new(7),
            ObjectId::new(11),
            CallMetadata::default(),
            (),
        );

        // Act
        let frame = protocol
            .encode(&command)
            .expect("dispose command should encode");
        let decoded = protocol
            .decode::<Command<HandleDispose>>(frame)
            .expect("dispose command should decode");

        // Assert
        assert_eq!(decoded.call_id(), CallId::new(7));
        assert_eq!(decoded.object_id(), ObjectId::new(11));
        assert_eq!(decoded.metadata(), CallMetadata::default());
        assert_eq!(decoded.params(), &());
    }

    #[test]
    fn dispose_result_round_trips_structured_failure() {
        // Arrange
        let protocol = protocol();
        let result = CommandResult::<HandleDispose>::new(
            CallId::new(7),
            Err(OperationError::new(
                FailureKind::UnsupportedCapability,
                "handle is externally owned",
                Some(HandleLifecycleErrorDetails::NotOwned(
                    "external handle".to_string(),
                )),
            )),
        );

        // Act
        let frame = protocol
            .encode(&result)
            .expect("dispose result should encode");
        let decoded = protocol
            .decode::<CommandResult<HandleDispose>>(frame)
            .expect("dispose result should decode");

        // Assert
        assert_eq!(decoded.call_id(), result.call_id());
        let error = decoded
            .into_outcome()
            .expect_err("decoded result should remain a failure");
        assert_eq!(error.kind(), FailureKind::UnsupportedCapability);
        assert_eq!(error.message(), "handle is externally owned");
        assert_eq!(
            error.details(),
            Some(&HandleLifecycleErrorDetails::NotOwned(
                "external handle".to_string()
            ))
        );
    }

    #[test]
    fn dispose_rejects_action_timeout_metadata_before_encoding() {
        // Arrange
        let command = Command::<HandleDispose>::new(
            CallId::new(1),
            ObjectId::new(2),
            CallMetadata::new(Some(Duration::from_secs(1)), None),
            (),
        );

        // Act
        let error = <HandleLifecycleCodec as WireCodec<Command<HandleDispose>>>::encode_parts(
            &codec(),
            &command,
        )
        .expect_err("dispose timeout must be rejected");

        // Assert
        assert!(matches!(
            error,
            HandleLifecycleCodecError::ControlMetadataUnsupported
        ));
    }

    #[test]
    fn dispose_rejects_payload_identity_different_from_outer_route() {
        // Arrange
        let protocol = protocol();
        let command = Command::<HandleDispose>::new(
            CallId::new(7),
            ObjectId::new(11),
            CallMetadata::default(),
            (),
        );
        let encoded = protocol
            .encode(&command)
            .expect("dispose command should encode");
        let tampered = WireFrame::new(
            encoded.version(),
            FrameRoute::Command {
                call_id: CallId::new(8),
                target: ObjectId::new(11),
                method: HANDLE_DISPOSE_METHOD,
            },
            encoded.payload().to_vec(),
            Vec::new(),
            FrameLimits::new(1024, 0, 0, 1024),
        )
        .expect("tampered frame remains structurally valid");

        // Act
        let error = match protocol.decode::<Command<HandleDispose>>(tampered) {
            Ok(_) => panic!("identity mismatch must fail"),
            Err(error) => error,
        };

        // Assert
        assert!(matches!(
            error,
            pwright_transport::WireProtocolError::Codec(
                HandleLifecycleCodecError::IdentityMismatch {
                    field: "call ID",
                    outer: 8,
                    inner: 7,
                }
            )
        ));
    }

    #[test]
    fn generated_schema_matches_capability_identity() {
        // Arrange
        let command = HANDLE_LIFECYCLE_COMMAND_SCHEMAS[0];

        // Act
        let catalog_validation = HANDLE_LIFECYCLE_SCHEMA_CATALOG.validate();

        // Assert
        assert_eq!(
            HANDLE_LIFECYCLE_SCHEMA.version(),
            ProtocolVersion::new(1, 0)
        );
        assert_eq!(catalog_validation, Ok(()));
        assert_eq!(command.id(), HANDLE_DISPOSE_COMMAND_ID);
        assert_eq!(command.method(), HANDLE_DISPOSE_METHOD);
    }
}
