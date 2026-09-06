//! Canonical bounded binary codec for browser-page lifecycle commands.
//!
//! The codec rejects timeout/cancellation metadata because both close forms
//! are no-timeout lifecycle operations, not ordinary actions.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use pwright_cap_page_lifecycle::{PageClose, PageLifecycleErrorDetails, PageRunBeforeUnload};
use pwright_codec_binary::{
    BinaryValueDecodeError, BinaryValueEncodeError, BinaryValueLimits, decode_value, encode_value,
};
use pwright_core::{
    CallId, CallMetadata, CancellationId, Command, CommandResult, CommandSpec, FailureKind,
    MethodTag, ObjectId, OperationError,
};
use pwright_transport::{EncodedParts, FrameRoute, WireCodec, WireFrame};

const CLOSE_COMMAND: u8 = 1;
const CLOSE_RESULT: u8 = 2;
const RUN_BEFORE_UNLOAD_COMMAND: u8 = 3;
const RUN_BEFORE_UNLOAD_RESULT: u8 = 4;

#[doc(hidden)]
#[rustfmt::skip]
pub mod generated_schema;

pub use generated_schema::{
    CATALOG as PAGE_LIFECYCLE_SCHEMA_CATALOG, COMMANDS as PAGE_LIFECYCLE_COMMAND_SCHEMAS,
    ROOT_SCHEMA as PAGE_LIFECYCLE_SCHEMA,
};

use generated_schema::{
    PwrightCommonCallMetadata, PwrightCommonFailureKind, PwrightPageLifecycle,
    PwrightPageLifecycleError, PwrightPageLifecycleErrorDetailsRef, PwrightPageLifecycleErrorRef,
    PwrightPageLifecycleOutcome, PwrightPageLifecycleOutcomeRef,
    PwrightPageLifecyclePageCloseOptions, PwrightPageLifecyclePageRunBeforeUnloadOptions,
    PwrightPageLifecycleRef,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageLifecycleCodecLimits {
    pub string_bytes: usize,
}

impl PageLifecycleCodecLimits {
    pub const fn new(string_bytes: usize) -> Self {
        Self { string_bytes }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PageLifecycleCodec {
    limits: PageLifecycleCodecLimits,
}

impl PageLifecycleCodec {
    pub const fn new(limits: PageLifecycleCodecLimits) -> Self {
        Self { limits }
    }

    pub const fn limits(self) -> PageLifecycleCodecLimits {
        self.limits
    }

    fn value_limits(self) -> BinaryValueLimits {
        BinaryValueLimits::new(self.limits.string_bytes, 0, 0, 8, 16)
    }
}

#[derive(Debug)]
pub enum PageLifecycleCodecError {
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

impl Display for PageLifecycleCodecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongRoute(route) => {
                write!(formatter, "page lifecycle payload arrived on {route:?}")
            }
            Self::WrongMethod { expected, actual } => write!(
                formatter,
                "expected page lifecycle method {expected}, received {actual}"
            ),
            Self::IdentityMismatch {
                field,
                outer,
                inner,
            } => write!(
                formatter,
                "page lifecycle payload {field} {inner} does not match outer route {outer}"
            ),
            Self::AttachmentsUnsupported { actual } => write!(
                formatter,
                "page lifecycle payload has {actual} attachment(s), but none are allowed"
            ),
            Self::ControlMetadataUnsupported => {
                formatter.write_str("page close does not accept timeout or cancellation metadata")
            }
            Self::WrongPayloadKind { expected, actual } => write!(
                formatter,
                "expected page lifecycle payload kind {expected}, received {actual}"
            ),
            Self::Encode(error) => Display::fmt(error, formatter),
            Self::Decode(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for PageLifecycleCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Encode(error) => Some(error),
            Self::Decode(error) => Some(error),
            _ => None,
        }
    }
}

impl WireCodec<Command<PageClose>> for PageLifecycleCodec {
    type Error = PageLifecycleCodecError;

    fn encode_parts(&self, command: &Command<PageClose>) -> Result<EncodedParts, Self::Error> {
        ensure_empty_metadata(command.metadata())?;
        let wire = PwrightPageLifecycleRef::PageCloseCommand {
            call_id: command.call_id().get(),
            object_id: command.object_id().get(),
            metadata: empty_metadata(),
            options: PwrightPageLifecyclePageCloseOptions {},
        };
        let payload =
            encode_value(&wire, self.value_limits()).map_err(PageLifecycleCodecError::Encode)?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: PageClose::METHOD,
            },
            payload,
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<PageClose>, Self::Error> {
        let (outer_call_id, outer_object_id) = command_route::<PageClose>(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightPageLifecycle>(frame.payload(), self.value_limits())
            .map_err(PageLifecycleCodecError::Decode)?
        {
            PwrightPageLifecycle::PageCloseCommand {
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
            other => Err(PageLifecycleCodecError::WrongPayloadKind {
                expected: CLOSE_COMMAND,
                actual: wire_kind(&other),
            }),
        }
    }
}

impl WireCodec<CommandResult<PageClose>> for PageLifecycleCodec {
    type Error = PageLifecycleCodecError;

    fn encode_parts(&self, result: &CommandResult<PageClose>) -> Result<EncodedParts, Self::Error> {
        let outcome = match result.outcome() {
            Ok(()) => PwrightPageLifecycleOutcomeRef::Success,
            Err(error) => PwrightPageLifecycleOutcomeRef::Error {
                error: operation_error_to_wire_ref(error),
            },
        };
        let wire = PwrightPageLifecycleRef::PageCloseResult {
            call_id: result.call_id().get(),
            outcome,
        };
        let payload =
            encode_value(&wire, self.value_limits()).map_err(PageLifecycleCodecError::Encode)?;
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload,
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<PageClose>, Self::Error> {
        let outer_call_id = result_route(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightPageLifecycle>(frame.payload(), self.value_limits())
            .map_err(PageLifecycleCodecError::Decode)?
        {
            PwrightPageLifecycle::PageCloseResult { call_id, outcome } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                let outcome = match outcome {
                    PwrightPageLifecycleOutcome::Success => Ok(()),
                    PwrightPageLifecycleOutcome::Error { error } => {
                        Err(operation_error_from_wire(error))
                    }
                };
                Ok(CommandResult::new(CallId::new(call_id), outcome))
            }
            other => Err(PageLifecycleCodecError::WrongPayloadKind {
                expected: CLOSE_RESULT,
                actual: wire_kind(&other),
            }),
        }
    }
}

impl WireCodec<Command<PageRunBeforeUnload>> for PageLifecycleCodec {
    type Error = PageLifecycleCodecError;

    fn encode_parts(
        &self,
        command: &Command<PageRunBeforeUnload>,
    ) -> Result<EncodedParts, Self::Error> {
        ensure_empty_metadata(command.metadata())?;
        let wire = PwrightPageLifecycleRef::PageRunBeforeUnloadCommand {
            call_id: command.call_id().get(),
            object_id: command.object_id().get(),
            metadata: empty_metadata(),
            options: PwrightPageLifecyclePageRunBeforeUnloadOptions {},
        };
        let payload =
            encode_value(&wire, self.value_limits()).map_err(PageLifecycleCodecError::Encode)?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: PageRunBeforeUnload::METHOD,
            },
            payload,
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<PageRunBeforeUnload>, Self::Error> {
        let (outer_call_id, outer_object_id) = command_route::<PageRunBeforeUnload>(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightPageLifecycle>(frame.payload(), self.value_limits())
            .map_err(PageLifecycleCodecError::Decode)?
        {
            PwrightPageLifecycle::PageRunBeforeUnloadCommand {
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
            other => Err(PageLifecycleCodecError::WrongPayloadKind {
                expected: RUN_BEFORE_UNLOAD_COMMAND,
                actual: wire_kind(&other),
            }),
        }
    }
}

impl WireCodec<CommandResult<PageRunBeforeUnload>> for PageLifecycleCodec {
    type Error = PageLifecycleCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<PageRunBeforeUnload>,
    ) -> Result<EncodedParts, Self::Error> {
        let outcome = match result.outcome() {
            Ok(()) => PwrightPageLifecycleOutcomeRef::Success,
            Err(error) => PwrightPageLifecycleOutcomeRef::Error {
                error: operation_error_to_wire_ref(error),
            },
        };
        let wire = PwrightPageLifecycleRef::PageRunBeforeUnloadResult {
            call_id: result.call_id().get(),
            outcome,
        };
        let payload =
            encode_value(&wire, self.value_limits()).map_err(PageLifecycleCodecError::Encode)?;
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
    ) -> Result<CommandResult<PageRunBeforeUnload>, Self::Error> {
        let outer_call_id = result_route(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightPageLifecycle>(frame.payload(), self.value_limits())
            .map_err(PageLifecycleCodecError::Decode)?
        {
            PwrightPageLifecycle::PageRunBeforeUnloadResult { call_id, outcome } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                let outcome = match outcome {
                    PwrightPageLifecycleOutcome::Success => Ok(()),
                    PwrightPageLifecycleOutcome::Error { error } => {
                        Err(operation_error_from_wire(error))
                    }
                };
                Ok(CommandResult::new(CallId::new(call_id), outcome))
            }
            other => Err(PageLifecycleCodecError::WrongPayloadKind {
                expected: RUN_BEFORE_UNLOAD_RESULT,
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

fn ensure_empty_metadata(metadata: CallMetadata) -> Result<(), PageLifecycleCodecError> {
    if metadata == CallMetadata::default() {
        Ok(())
    } else {
        Err(PageLifecycleCodecError::ControlMetadataUnsupported)
    }
}

fn ensure_no_attachments(frame: &WireFrame) -> Result<(), PageLifecycleCodecError> {
    if frame.attachments().is_empty() {
        Ok(())
    } else {
        Err(PageLifecycleCodecError::AttachmentsUnsupported {
            actual: frame.attachments().len(),
        })
    }
}

fn ensure_identity(
    field: &'static str,
    outer: u64,
    inner: u64,
) -> Result<(), PageLifecycleCodecError> {
    if outer == inner {
        Ok(())
    } else {
        Err(PageLifecycleCodecError::IdentityMismatch {
            field,
            outer,
            inner,
        })
    }
}

fn command_route<S: CommandSpec>(
    frame: &WireFrame,
) -> Result<(CallId, ObjectId), PageLifecycleCodecError> {
    let FrameRoute::Command {
        call_id,
        target,
        method,
    } = frame.route()
    else {
        return Err(PageLifecycleCodecError::WrongRoute(frame.route()));
    };
    if method != S::METHOD {
        return Err(PageLifecycleCodecError::WrongMethod {
            expected: S::METHOD,
            actual: method,
        });
    }
    Ok((call_id, target))
}

fn result_route(frame: &WireFrame) -> Result<CallId, PageLifecycleCodecError> {
    let FrameRoute::Result { call_id } = frame.route() else {
        return Err(PageLifecycleCodecError::WrongRoute(frame.route()));
    };
    Ok(call_id)
}

fn wire_kind(value: &PwrightPageLifecycle) -> u8 {
    match value {
        PwrightPageLifecycle::PageCloseCommand { .. } => CLOSE_COMMAND,
        PwrightPageLifecycle::PageCloseResult { .. } => CLOSE_RESULT,
        PwrightPageLifecycle::PageRunBeforeUnloadCommand { .. } => RUN_BEFORE_UNLOAD_COMMAND,
        PwrightPageLifecycle::PageRunBeforeUnloadResult { .. } => RUN_BEFORE_UNLOAD_RESULT,
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
    details: &PageLifecycleErrorDetails,
) -> PwrightPageLifecycleErrorDetailsRef<'_> {
    match details {
        PageLifecycleErrorDetails::InvalidBackendResponse(message) => {
            PwrightPageLifecycleErrorDetailsRef::InvalidBackendResponse { message }
        }
        PageLifecycleErrorDetails::TargetStillOpen(message) => {
            PwrightPageLifecycleErrorDetailsRef::TargetStillOpen { message }
        }
        PageLifecycleErrorDetails::Backend(message) => {
            PwrightPageLifecycleErrorDetailsRef::Backend { message }
        }
    }
}

fn operation_error_to_wire_ref(
    error: &OperationError<PageLifecycleErrorDetails>,
) -> PwrightPageLifecycleErrorRef<'_> {
    PwrightPageLifecycleErrorRef {
        failure_kind: failure_kind_to_wire(error.kind()),
        message: error.message(),
        details: error.details().map(error_details_to_wire_ref),
    }
}

fn operation_error_from_wire(
    error: PwrightPageLifecycleError,
) -> OperationError<PageLifecycleErrorDetails> {
    OperationError::new(
        failure_kind_from_wire(error.failure_kind),
        error.message,
        error.details.map(Into::into),
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use pwright_cap_page_lifecycle::{
        PAGE_CLOSE_COMMAND_ID, PAGE_CLOSE_METHOD, PAGE_RUN_BEFORE_UNLOAD_COMMAND_ID,
        PAGE_RUN_BEFORE_UNLOAD_METHOD, PageClose, PageLifecycleErrorDetails, PageRunBeforeUnload,
    };
    use pwright_core::{
        CallId, CallMetadata, Command, CommandResult, FailureKind, ObjectId, OperationError,
        ProtocolVersion,
    };
    use pwright_transport::{
        FrameLimits, FrameRoute, VersionPolicy, WireCodec, WireFrame, WireProtocol,
    };

    use super::{
        PAGE_LIFECYCLE_COMMAND_SCHEMAS, PAGE_LIFECYCLE_SCHEMA, PAGE_LIFECYCLE_SCHEMA_CATALOG,
        PageLifecycleCodec, PageLifecycleCodecError, PageLifecycleCodecLimits,
    };

    fn codec() -> PageLifecycleCodec {
        PageLifecycleCodec::new(PageLifecycleCodecLimits::new(256))
    }

    fn protocol() -> WireProtocol<PageLifecycleCodec> {
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
    fn close_command_round_trips_with_exact_route_identity() {
        // Arrange
        let protocol = protocol();
        let command = Command::<PageClose>::new(
            CallId::new(7),
            ObjectId::new(11),
            CallMetadata::default(),
            (),
        );

        // Act
        let frame = protocol
            .encode(&command)
            .expect("close command should encode");
        let decoded = protocol
            .decode::<Command<PageClose>>(frame)
            .expect("close command should decode");

        // Assert
        assert_eq!(decoded.call_id(), CallId::new(7));
        assert_eq!(decoded.object_id(), ObjectId::new(11));
        assert_eq!(decoded.metadata(), CallMetadata::default());
        assert_eq!(decoded.params(), &());
    }

    #[test]
    fn close_result_round_trips_structured_failure() {
        // Arrange
        let protocol = protocol();
        let result = CommandResult::<PageClose>::new(
            CallId::new(7),
            Err(OperationError::new(
                FailureKind::Backend,
                "target remained open",
                Some(PageLifecycleErrorDetails::TargetStillOpen(
                    "target-7".to_string(),
                )),
            )),
        );

        // Act
        let frame = protocol
            .encode(&result)
            .expect("close result should encode");
        let decoded = protocol
            .decode::<CommandResult<PageClose>>(frame)
            .expect("close result should decode");

        // Assert
        assert_eq!(decoded.call_id(), result.call_id());
        let error = decoded
            .into_outcome()
            .expect_err("decoded result should remain a failure");
        assert_eq!(error.kind(), FailureKind::Backend);
        assert_eq!(error.message(), "target remained open");
        assert_eq!(
            error.details(),
            Some(&PageLifecycleErrorDetails::TargetStillOpen(
                "target-7".to_string()
            ))
        );
    }

    #[test]
    fn run_before_unload_command_and_result_round_trip_independently() {
        // Arrange
        let protocol = protocol();
        let command = Command::<PageRunBeforeUnload>::new(
            CallId::new(13),
            ObjectId::new(17),
            CallMetadata::default(),
            (),
        );

        // Act
        let command = protocol
            .decode::<Command<PageRunBeforeUnload>>(
                protocol
                    .encode(&command)
                    .expect("before-unload command should encode"),
            )
            .expect("before-unload command should decode");
        let result = CommandResult::<PageRunBeforeUnload>::new(command.call_id(), Ok(()));
        let result = protocol
            .decode::<CommandResult<PageRunBeforeUnload>>(
                protocol
                    .encode(&result)
                    .expect("before-unload result should encode"),
            )
            .expect("before-unload result should decode");

        // Assert
        assert_eq!(command.object_id(), ObjectId::new(17));
        assert_eq!(command.metadata(), CallMetadata::default());
        assert_eq!(result.into_outcome(), Ok(()));
    }

    #[test]
    fn close_rejects_action_timeout_metadata_before_encoding() {
        // Arrange
        let command = Command::<PageClose>::new(
            CallId::new(1),
            ObjectId::new(2),
            CallMetadata::new(Some(Duration::from_secs(1)), None),
            (),
        );

        // Act
        let error =
            <PageLifecycleCodec as WireCodec<Command<PageClose>>>::encode_parts(&codec(), &command)
                .expect_err("close timeout must be rejected");

        // Assert
        assert!(matches!(
            error,
            PageLifecycleCodecError::ControlMetadataUnsupported
        ));
    }

    #[test]
    fn close_rejects_payload_identity_different_from_outer_route() {
        // Arrange
        let protocol = protocol();
        let command = Command::<PageClose>::new(
            CallId::new(7),
            ObjectId::new(11),
            CallMetadata::default(),
            (),
        );
        let encoded = protocol
            .encode(&command)
            .expect("close command should encode");
        let tampered = WireFrame::new(
            encoded.version(),
            FrameRoute::Command {
                call_id: CallId::new(8),
                target: ObjectId::new(11),
                method: PAGE_CLOSE_METHOD,
            },
            encoded.payload().to_vec(),
            Vec::new(),
            FrameLimits::new(1024, 0, 0, 1024),
        )
        .expect("tampered frame remains structurally valid");

        // Act
        let error = match protocol.decode::<Command<PageClose>>(tampered) {
            Ok(_) => panic!("identity mismatch must fail"),
            Err(error) => error,
        };

        // Assert
        assert!(matches!(
            error,
            pwright_transport::WireProtocolError::Codec(
                PageLifecycleCodecError::IdentityMismatch {
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
        let command = PAGE_LIFECYCLE_COMMAND_SCHEMAS[0];

        // Act
        let catalog_validation = PAGE_LIFECYCLE_SCHEMA_CATALOG.validate();

        // Assert
        assert_eq!(PAGE_LIFECYCLE_SCHEMA.version(), ProtocolVersion::new(1, 0));
        assert_eq!(catalog_validation, Ok(()));
        assert_eq!(command.id(), PAGE_CLOSE_COMMAND_ID);
        assert_eq!(command.method(), PAGE_CLOSE_METHOD);
        assert_eq!(PAGE_LIFECYCLE_COMMAND_SCHEMAS.len(), 2);
        assert_eq!(
            PAGE_LIFECYCLE_COMMAND_SCHEMAS[1].id(),
            PAGE_RUN_BEFORE_UNLOAD_COMMAND_ID
        );
        assert_eq!(
            PAGE_LIFECYCLE_COMMAND_SCHEMAS[1].method(),
            PAGE_RUN_BEFORE_UNLOAD_METHOD
        );
    }
}
