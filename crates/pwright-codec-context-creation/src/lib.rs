//! Canonical bounded binary codec for BrowserContext creation.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use pwright_cap_context_creation::{
    BrowserContextInitializer, BrowserNewContext, ContextCreationErrorDetails,
};
use pwright_codec_binary::{
    BinaryValueDecodeError, BinaryValueEncodeError, BinaryValueLimits, decode_value, encode_value,
};
use pwright_core::{
    CallId, CallMetadata, CancellationId, Command, CommandResult, CommandSpec, FailureKind,
    MethodTag, ObjectId, ObjectKindTag, OperationError,
};
use pwright_transport::{EncodedParts, FrameRoute, WireCodec, WireFrame};

const NEW_CONTEXT_COMMAND: u8 = 1;
const NEW_CONTEXT_RESULT: u8 = 2;

#[doc(hidden)]
#[rustfmt::skip]
pub mod generated_schema;

pub use generated_schema::{
    CATALOG as CONTEXT_CREATION_SCHEMA_CATALOG, COMMANDS as CONTEXT_CREATION_COMMAND_SCHEMAS,
    ROOT_SCHEMA as CONTEXT_CREATION_SCHEMA,
};

use generated_schema::{
    PwrightCommonCallMetadata, PwrightCommonFailureKind, PwrightContextCreation,
    PwrightContextCreationBrowserContextInitializer,
    PwrightContextCreationBrowserNewContextOptions, PwrightContextCreationError,
    PwrightContextCreationErrorDetailsRef, PwrightContextCreationErrorRef,
    PwrightContextCreationOutcome, PwrightContextCreationOutcomeRef, PwrightContextCreationRef,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextCreationCodecLimits {
    pub string_bytes: usize,
}

impl ContextCreationCodecLimits {
    pub const fn new(string_bytes: usize) -> Self {
        Self { string_bytes }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ContextCreationCodec {
    limits: ContextCreationCodecLimits,
}

impl ContextCreationCodec {
    pub const fn new(limits: ContextCreationCodecLimits) -> Self {
        Self { limits }
    }

    pub const fn limits(self) -> ContextCreationCodecLimits {
        self.limits
    }

    fn value_limits(self) -> BinaryValueLimits {
        BinaryValueLimits::new(self.limits.string_bytes, 0, 0, 8, 16)
    }
}

#[derive(Debug)]
pub enum ContextCreationCodecError {
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

impl Display for ContextCreationCodecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongRoute(route) => {
                write!(formatter, "context creation payload arrived on {route:?}")
            }
            Self::WrongMethod { expected, actual } => write!(
                formatter,
                "expected context creation method {expected}, received {actual}"
            ),
            Self::IdentityMismatch {
                field,
                outer,
                inner,
            } => write!(
                formatter,
                "context creation payload {field} {inner} does not match outer route {outer}"
            ),
            Self::AttachmentsUnsupported { actual } => write!(
                formatter,
                "context creation payload has {actual} attachment(s), but none are allowed"
            ),
            Self::ControlMetadataUnsupported => formatter
                .write_str("Browser.newContext does not accept timeout or cancellation metadata"),
            Self::WrongPayloadKind { expected, actual } => write!(
                formatter,
                "expected context creation payload kind {expected}, received {actual}"
            ),
            Self::Encode(error) => Display::fmt(error, formatter),
            Self::Decode(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ContextCreationCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Encode(error) => Some(error),
            Self::Decode(error) => Some(error),
            _ => None,
        }
    }
}

impl WireCodec<Command<BrowserNewContext>> for ContextCreationCodec {
    type Error = ContextCreationCodecError;

    fn encode_parts(
        &self,
        command: &Command<BrowserNewContext>,
    ) -> Result<EncodedParts, Self::Error> {
        ensure_empty_metadata(command.metadata())?;
        let wire = PwrightContextCreationRef::BrowserNewContextCommand {
            call_id: command.call_id().get(),
            object_id: command.object_id().get(),
            metadata: empty_metadata(),
            options: PwrightContextCreationBrowserNewContextOptions {},
        };
        let payload =
            encode_value(&wire, self.value_limits()).map_err(ContextCreationCodecError::Encode)?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: BrowserNewContext::METHOD,
            },
            payload,
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<BrowserNewContext>, Self::Error> {
        let (outer_call_id, outer_object_id) = command_route::<BrowserNewContext>(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightContextCreation>(frame.payload(), self.value_limits())
            .map_err(ContextCreationCodecError::Decode)?
        {
            PwrightContextCreation::BrowserNewContextCommand {
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
            other => Err(ContextCreationCodecError::WrongPayloadKind {
                expected: NEW_CONTEXT_COMMAND,
                actual: wire_kind(&other),
            }),
        }
    }
}

impl WireCodec<CommandResult<BrowserNewContext>> for ContextCreationCodec {
    type Error = ContextCreationCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<BrowserNewContext>,
    ) -> Result<EncodedParts, Self::Error> {
        let outcome = match result.outcome() {
            Ok(initializer) => PwrightContextCreationOutcomeRef::Success {
                result: initializer_to_wire(*initializer),
            },
            Err(error) => PwrightContextCreationOutcomeRef::Error {
                error: operation_error_to_wire_ref(error),
            },
        };
        let wire = PwrightContextCreationRef::BrowserNewContextResult {
            call_id: result.call_id().get(),
            outcome,
        };
        let payload =
            encode_value(&wire, self.value_limits()).map_err(ContextCreationCodecError::Encode)?;
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
    ) -> Result<CommandResult<BrowserNewContext>, Self::Error> {
        let outer_call_id = result_route(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightContextCreation>(frame.payload(), self.value_limits())
            .map_err(ContextCreationCodecError::Decode)?
        {
            PwrightContextCreation::BrowserNewContextResult { call_id, outcome } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                let outcome = match outcome {
                    PwrightContextCreationOutcome::Success { result } => {
                        Ok(initializer_from_wire(result))
                    }
                    PwrightContextCreationOutcome::Error { error } => {
                        Err(operation_error_from_wire(error))
                    }
                };
                Ok(CommandResult::new(CallId::new(call_id), outcome))
            }
            other => Err(ContextCreationCodecError::WrongPayloadKind {
                expected: NEW_CONTEXT_RESULT,
                actual: wire_kind(&other),
            }),
        }
    }
}

fn initializer_to_wire(
    value: BrowserContextInitializer,
) -> PwrightContextCreationBrowserContextInitializer {
    PwrightContextCreationBrowserContextInitializer {
        object_id: value.object_id.get(),
        parent_id: value.parent_id.get(),
        object_kind: value.object_kind.get(),
    }
}

fn initializer_from_wire(
    value: PwrightContextCreationBrowserContextInitializer,
) -> BrowserContextInitializer {
    BrowserContextInitializer::new(
        ObjectId::new(value.object_id),
        ObjectId::new(value.parent_id),
        ObjectKindTag::new(value.object_kind),
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

fn ensure_empty_metadata(metadata: CallMetadata) -> Result<(), ContextCreationCodecError> {
    if metadata == CallMetadata::default() {
        Ok(())
    } else {
        Err(ContextCreationCodecError::ControlMetadataUnsupported)
    }
}

fn ensure_no_attachments(frame: &WireFrame) -> Result<(), ContextCreationCodecError> {
    if frame.attachments().is_empty() {
        Ok(())
    } else {
        Err(ContextCreationCodecError::AttachmentsUnsupported {
            actual: frame.attachments().len(),
        })
    }
}

fn ensure_identity(
    field: &'static str,
    outer: u64,
    inner: u64,
) -> Result<(), ContextCreationCodecError> {
    if outer == inner {
        Ok(())
    } else {
        Err(ContextCreationCodecError::IdentityMismatch {
            field,
            outer,
            inner,
        })
    }
}

fn command_route<S: CommandSpec>(
    frame: &WireFrame,
) -> Result<(CallId, ObjectId), ContextCreationCodecError> {
    let FrameRoute::Command {
        call_id,
        target,
        method,
    } = frame.route()
    else {
        return Err(ContextCreationCodecError::WrongRoute(frame.route()));
    };
    if method != S::METHOD {
        return Err(ContextCreationCodecError::WrongMethod {
            expected: S::METHOD,
            actual: method,
        });
    }
    Ok((call_id, target))
}

fn result_route(frame: &WireFrame) -> Result<CallId, ContextCreationCodecError> {
    let FrameRoute::Result { call_id } = frame.route() else {
        return Err(ContextCreationCodecError::WrongRoute(frame.route()));
    };
    Ok(call_id)
}

fn wire_kind(value: &PwrightContextCreation) -> u8 {
    match value {
        PwrightContextCreation::BrowserNewContextCommand { .. } => NEW_CONTEXT_COMMAND,
        PwrightContextCreation::BrowserNewContextResult { .. } => NEW_CONTEXT_RESULT,
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
    details: &ContextCreationErrorDetails,
) -> PwrightContextCreationErrorDetailsRef<'_> {
    match details {
        ContextCreationErrorDetails::InvalidBackendResponse(message) => {
            PwrightContextCreationErrorDetailsRef::InvalidBackendResponse { message }
        }
        ContextCreationErrorDetails::Publication(message) => {
            PwrightContextCreationErrorDetailsRef::Publication { message }
        }
        ContextCreationErrorDetails::Cleanup { primary, cleanup } => {
            PwrightContextCreationErrorDetailsRef::Cleanup { primary, cleanup }
        }
        ContextCreationErrorDetails::Backend(message) => {
            PwrightContextCreationErrorDetailsRef::Backend { message }
        }
    }
}

fn operation_error_to_wire_ref(
    error: &OperationError<ContextCreationErrorDetails>,
) -> PwrightContextCreationErrorRef<'_> {
    PwrightContextCreationErrorRef {
        failure_kind: failure_kind_to_wire(error.kind()),
        message: error.message(),
        details: error.details().map(error_details_to_wire_ref),
    }
}

fn operation_error_from_wire(
    error: PwrightContextCreationError,
) -> OperationError<ContextCreationErrorDetails> {
    OperationError::new(
        failure_kind_from_wire(error.failure_kind),
        error.message,
        error.details.map(Into::into),
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use pwright_cap_context_creation::{
        BROWSER_NEW_CONTEXT_COMMAND_ID, BROWSER_NEW_CONTEXT_METHOD, BrowserContextInitializer,
        BrowserNewContext, ContextCreationErrorDetails,
    };
    use pwright_core::{
        CallId, CallMetadata, Command, CommandResult, FailureKind, ObjectId, ObjectKindTag,
        OperationError, ProtocolVersion,
    };
    use pwright_transport::{
        FrameLimits, FrameRoute, VersionPolicy, WireCodec, WireFrame, WireProtocol,
    };

    use super::{
        CONTEXT_CREATION_COMMAND_SCHEMAS, CONTEXT_CREATION_SCHEMA, CONTEXT_CREATION_SCHEMA_CATALOG,
        ContextCreationCodec, ContextCreationCodecError, ContextCreationCodecLimits,
    };

    fn codec() -> ContextCreationCodec {
        ContextCreationCodec::new(ContextCreationCodecLimits::new(256))
    }

    fn protocol() -> WireProtocol<ContextCreationCodec> {
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
    fn new_context_command_round_trips_with_exact_browser_identity() {
        // Arrange
        let protocol = protocol();
        let command = Command::<BrowserNewContext>::new(
            CallId::new(7),
            ObjectId::new(11),
            CallMetadata::default(),
            (),
        );

        // Act
        let frame = protocol.encode(&command).expect("command should encode");
        let decoded = protocol
            .decode::<Command<BrowserNewContext>>(frame)
            .expect("command should decode");

        // Assert
        assert_eq!(decoded.call_id(), CallId::new(7));
        assert_eq!(decoded.object_id(), ObjectId::new(11));
        assert_eq!(decoded.metadata(), CallMetadata::default());
        assert_eq!(decoded.params(), &());
    }

    #[test]
    fn new_context_result_round_trips_exact_initializer() {
        // Arrange
        let protocol = protocol();
        let initializer = BrowserContextInitializer::new(
            ObjectId::new(12),
            ObjectId::new(11),
            ObjectKindTag::new(2),
        );
        let result = CommandResult::<BrowserNewContext>::new(CallId::new(7), Ok(initializer));

        // Act
        let frame = protocol.encode(&result).expect("result should encode");
        let decoded = protocol
            .decode::<CommandResult<BrowserNewContext>>(frame)
            .expect("result should decode");

        // Assert
        assert_eq!(decoded.into_outcome(), Ok(initializer));
    }

    #[test]
    fn new_context_result_round_trips_cleanup_diagnostics() {
        // Arrange
        let protocol = protocol();
        let result = CommandResult::<BrowserNewContext>::new(
            CallId::new(7),
            Err(OperationError::new(
                FailureKind::Cleanup,
                "publication and compensation failed",
                Some(ContextCreationErrorDetails::Cleanup {
                    primary: "route table full".to_string(),
                    cleanup: "native context remained visible".to_string(),
                }),
            )),
        );

        // Act
        let frame = protocol.encode(&result).expect("result should encode");
        let decoded = protocol
            .decode::<CommandResult<BrowserNewContext>>(frame)
            .expect("result should decode");

        // Assert
        assert_eq!(decoded.into_outcome(), result.into_outcome());
    }

    #[test]
    fn new_context_rejects_action_timeout_metadata_before_encoding() {
        // Arrange
        let command = Command::<BrowserNewContext>::new(
            CallId::new(1),
            ObjectId::new(2),
            CallMetadata::new(Some(Duration::from_secs(1)), None),
            (),
        );

        // Act
        let error = <ContextCreationCodec as WireCodec<Command<BrowserNewContext>>>::encode_parts(
            &codec(),
            &command,
        )
        .expect_err("newContext timeout must be rejected");

        // Assert
        assert!(matches!(
            error,
            ContextCreationCodecError::ControlMetadataUnsupported
        ));
    }

    #[test]
    fn new_context_rejects_payload_identity_different_from_outer_route() {
        // Arrange
        let protocol = protocol();
        let command = Command::<BrowserNewContext>::new(
            CallId::new(7),
            ObjectId::new(11),
            CallMetadata::default(),
            (),
        );
        let encoded = protocol.encode(&command).expect("command should encode");
        let tampered = WireFrame::new(
            encoded.version(),
            FrameRoute::Command {
                call_id: CallId::new(8),
                target: ObjectId::new(11),
                method: BROWSER_NEW_CONTEXT_METHOD,
            },
            encoded.payload().to_vec(),
            Vec::new(),
            FrameLimits::new(1024, 0, 0, 1024),
        )
        .expect("tampered frame remains structurally valid");

        // Act
        let error = match protocol.decode::<Command<BrowserNewContext>>(tampered) {
            Ok(_) => panic!("identity mismatch must fail"),
            Err(error) => error,
        };

        // Assert
        assert!(matches!(
            error,
            pwright_transport::WireProtocolError::Codec(
                ContextCreationCodecError::IdentityMismatch {
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
        let command = CONTEXT_CREATION_COMMAND_SCHEMAS[0];

        // Act
        let catalog_validation = CONTEXT_CREATION_SCHEMA_CATALOG.validate();

        // Assert
        assert_eq!(
            CONTEXT_CREATION_SCHEMA.version(),
            ProtocolVersion::new(1, 0)
        );
        assert_eq!(catalog_validation, Ok(()));
        assert_eq!(command.id(), BROWSER_NEW_CONTEXT_COMMAND_ID);
        assert_eq!(command.method(), BROWSER_NEW_CONTEXT_METHOD);
    }
}
