//! Canonical bounded binary codec for read-only DOM content.
//!
//! Large successful values travel as one UTF-8 attachment. Decoding copies
//! them into a separately budgeted result, so wire and caller ownership remain
//! distinct.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

use pwright_cap_dom_content::{
    DomContentData, DomContentErrorDetails, DomContentParams, FrameContent,
};
use pwright_codec_binary::{
    BinaryValueDecodeError, BinaryValueEncodeError, BinaryValueLimits, decode_value, encode_value,
};
use pwright_core::{
    BudgetedUtf8, CallId, CallMetadata, CancellationId, Command, CommandResult, CommandSpec,
    FailureKind, MethodTag, ObjectId, OperationError, ResourceClass, ResourceError, ResourceLedger,
    ResourceScope,
};
use pwright_transport::{EncodedParts, FrameRoute, WireCodec, WireFrame};

const FRAME_CONTENT_COMMAND: u8 = 1;
const FRAME_CONTENT_RESULT: u8 = 2;
const CONTENT_ATTACHMENT_INDEX: u32 = 0;

#[doc(hidden)]
#[rustfmt::skip]
pub mod generated_schema;

pub use generated_schema::{
    CATALOG as DOM_CONTENT_SCHEMA_CATALOG, COMMANDS as DOM_CONTENT_COMMAND_SCHEMAS,
    ROOT_SCHEMA as DOM_CONTENT_SCHEMA,
};

use generated_schema::{
    PwrightCommonCallMetadata, PwrightCommonDuration, PwrightCommonFailureKind, PwrightDomContent,
    PwrightDomContentData, PwrightDomContentError, PwrightDomContentErrorDetailsRef,
    PwrightDomContentErrorRef, PwrightDomContentOutcome, PwrightDomContentOutcomeRef,
    PwrightDomContentParams, PwrightDomContentRef,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DomContentCodecLimits {
    pub content_bytes: usize,
    pub error_string_bytes: usize,
}

impl DomContentCodecLimits {
    pub const fn new(content_bytes: usize, error_string_bytes: usize) -> Self {
        Self {
            content_bytes,
            error_string_bytes,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DomContentCodec {
    limits: DomContentCodecLimits,
    result_ledger: ResourceLedger,
}

impl DomContentCodec {
    pub const fn new(limits: DomContentCodecLimits, result_ledger: ResourceLedger) -> Self {
        Self {
            limits,
            result_ledger,
        }
    }

    pub const fn limits(&self) -> DomContentCodecLimits {
        self.limits
    }

    pub const fn result_ledger(&self) -> &ResourceLedger {
        &self.result_ledger
    }

    fn value_limits(&self) -> BinaryValueLimits {
        BinaryValueLimits::new(
            self.limits.error_string_bytes.saturating_mul(2),
            0,
            0,
            16,
            32,
        )
    }
}

#[derive(Debug)]
pub enum DomContentCodecError {
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
    TimeoutUnsupported,
    UnexpectedAttachments {
        actual: usize,
    },
    MissingContentAttachment,
    InvalidAttachmentIndex(u32),
    AttachmentLengthMismatch {
        declared: u64,
        actual: usize,
    },
    ContentTooLarge {
        actual: usize,
        limit: usize,
    },
    InvalidUtf8(std::str::Utf8Error),
    ErrorStringTooLong {
        actual: usize,
        limit: usize,
    },
    WrongPayloadKind {
        expected: u8,
        actual: u8,
    },
    Resource(ResourceError),
    Encode(BinaryValueEncodeError),
    Decode(BinaryValueDecodeError),
}

impl Display for DomContentCodecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongRoute(route) => {
                write!(formatter, "DOM-content payload arrived on {route:?}")
            }
            Self::WrongMethod { expected, actual } => write!(
                formatter,
                "expected DOM-content method {expected}, received {actual}"
            ),
            Self::IdentityMismatch {
                field,
                outer,
                inner,
            } => write!(
                formatter,
                "DOM-content payload {field} {inner} does not match outer route {outer}"
            ),
            Self::TimeoutUnsupported => {
                formatter.write_str("Frame.content does not accept timeout metadata")
            }
            Self::UnexpectedAttachments { actual } => write!(
                formatter,
                "DOM-content payload has {actual} attachment(s) where none are allowed"
            ),
            Self::MissingContentAttachment => {
                formatter.write_str("successful Frame.content result has no attachment")
            }
            Self::InvalidAttachmentIndex(index) => {
                write!(
                    formatter,
                    "DOM-content attachment index {index} is not zero"
                )
            }
            Self::AttachmentLengthMismatch { declared, actual } => write!(
                formatter,
                "DOM content declares {declared} bytes but attachment contains {actual}"
            ),
            Self::ContentTooLarge { actual, limit } => write!(
                formatter,
                "DOM content has {actual} bytes, exceeding limit {limit}"
            ),
            Self::InvalidUtf8(error) => write!(formatter, "DOM content is not UTF-8: {error}"),
            Self::ErrorStringTooLong { actual, limit } => write!(
                formatter,
                "DOM-content error string has {actual} bytes, exceeding limit {limit}"
            ),
            Self::WrongPayloadKind { expected, actual } => write!(
                formatter,
                "expected DOM-content payload kind {expected}, received {actual}"
            ),
            Self::Resource(error) => Display::fmt(error, formatter),
            Self::Encode(error) => Display::fmt(error, formatter),
            Self::Decode(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for DomContentCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidUtf8(error) => Some(error),
            Self::Resource(error) => Some(error),
            Self::Encode(error) => Some(error),
            Self::Decode(error) => Some(error),
            _ => None,
        }
    }
}

impl WireCodec<Command<FrameContent>> for DomContentCodec {
    type Error = DomContentCodecError;

    fn encode_parts(&self, command: &Command<FrameContent>) -> Result<EncodedParts, Self::Error> {
        validate_metadata(command.metadata())?;
        let wire = PwrightDomContentRef::FrameContentCommand {
            call_id: command.call_id().get(),
            object_id: command.object_id().get(),
            metadata: metadata_to_wire(command.metadata()),
            params: PwrightDomContentParams {},
        };
        let payload =
            encode_value(&wire, self.value_limits()).map_err(DomContentCodecError::Encode)?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: FrameContent::METHOD,
            },
            payload,
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<FrameContent>, Self::Error> {
        let (outer_call_id, outer_object_id) = command_route::<FrameContent>(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightDomContent>(frame.payload(), self.value_limits())
            .map_err(DomContentCodecError::Decode)?
        {
            PwrightDomContent::FrameContentCommand {
                call_id,
                object_id,
                metadata,
                params: _,
            } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                ensure_identity("object ID", outer_object_id.get(), object_id)?;
                let metadata = metadata_from_wire(metadata);
                validate_metadata(metadata)?;
                Ok(Command::new(
                    CallId::new(call_id),
                    ObjectId::new(object_id),
                    metadata,
                    DomContentParams,
                ))
            }
            other => Err(DomContentCodecError::WrongPayloadKind {
                expected: FRAME_CONTENT_COMMAND,
                actual: wire_kind(&other),
            }),
        }
    }
}

impl WireCodec<CommandResult<FrameContent>> for DomContentCodec {
    type Error = DomContentCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<FrameContent>,
    ) -> Result<EncodedParts, Self::Error> {
        let mut attachments = Vec::new();
        let outcome = match result.outcome() {
            Ok(result) => {
                validate_content_length(result.len(), self.limits.content_bytes)?;
                attachments.push(result.as_str().as_bytes().to_vec());
                PwrightDomContentOutcomeRef::Success {
                    result: PwrightDomContentData {
                        attachment_index: CONTENT_ATTACHMENT_INDEX,
                        byte_length: u64::try_from(result.len()).map_err(|_| {
                            DomContentCodecError::ContentTooLarge {
                                actual: result.len(),
                                limit: self.limits.content_bytes,
                            }
                        })?,
                    },
                }
            }
            Err(error) => {
                validate_operation_error(error, self.limits.error_string_bytes)?;
                PwrightDomContentOutcomeRef::Error {
                    error: operation_error_to_wire_ref(error),
                }
            }
        };
        let wire = PwrightDomContentRef::FrameContentResult {
            call_id: result.call_id().get(),
            outcome,
        };
        let payload =
            encode_value(&wire, self.value_limits()).map_err(DomContentCodecError::Encode)?;
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload,
            attachments,
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<FrameContent>, Self::Error> {
        let outer_call_id = result_route(&frame)?;
        match decode_value::<PwrightDomContent>(frame.payload(), self.value_limits())
            .map_err(DomContentCodecError::Decode)?
        {
            PwrightDomContent::FrameContentResult { call_id, outcome } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                let outcome = match outcome {
                    PwrightDomContentOutcome::Success { result } => {
                        if result.attachment_index != CONTENT_ATTACHMENT_INDEX {
                            return Err(DomContentCodecError::InvalidAttachmentIndex(
                                result.attachment_index,
                            ));
                        }
                        if frame.attachments().len() != 1 {
                            return Err(if frame.attachments().is_empty() {
                                DomContentCodecError::MissingContentAttachment
                            } else {
                                DomContentCodecError::UnexpectedAttachments {
                                    actual: frame.attachments().len(),
                                }
                            });
                        }
                        let bytes = frame.attachments()[0].as_bytes();
                        if result.byte_length != u64::try_from(bytes.len()).unwrap_or(u64::MAX) {
                            return Err(DomContentCodecError::AttachmentLengthMismatch {
                                declared: result.byte_length,
                                actual: bytes.len(),
                            });
                        }
                        validate_content_length(bytes.len(), self.limits.content_bytes)?;
                        let text = std::str::from_utf8(bytes)
                            .map_err(DomContentCodecError::InvalidUtf8)?;
                        let text = BudgetedUtf8::copy_from_str(
                            &self.result_ledger,
                            ResourceScope::Call(CallId::new(call_id)),
                            ResourceClass::DomContentBytes,
                            text,
                        )
                        .map_err(DomContentCodecError::Resource)?;
                        Ok(DomContentData::new(text))
                    }
                    PwrightDomContentOutcome::Error { error } => {
                        ensure_no_attachments(&frame)?;
                        let error = operation_error_from_wire(error);
                        validate_operation_error(&error, self.limits.error_string_bytes)?;
                        Err(error)
                    }
                };
                Ok(CommandResult::new(CallId::new(call_id), outcome))
            }
            other => Err(DomContentCodecError::WrongPayloadKind {
                expected: FRAME_CONTENT_RESULT,
                actual: wire_kind(&other),
            }),
        }
    }
}

fn validate_metadata(metadata: CallMetadata) -> Result<(), DomContentCodecError> {
    if metadata.timeout.is_some() {
        Err(DomContentCodecError::TimeoutUnsupported)
    } else {
        Ok(())
    }
}

fn validate_content_length(actual: usize, limit: usize) -> Result<(), DomContentCodecError> {
    if actual <= limit {
        Ok(())
    } else {
        Err(DomContentCodecError::ContentTooLarge { actual, limit })
    }
}

fn metadata_to_wire(metadata: CallMetadata) -> PwrightCommonCallMetadata {
    PwrightCommonCallMetadata {
        timeout: metadata.timeout.map(|timeout| PwrightCommonDuration {
            seconds: timeout.as_secs(),
            nanoseconds: timeout.subsec_nanos(),
        }),
        cancellation_id: metadata.cancellation.map(CancellationId::get),
    }
}

fn metadata_from_wire(metadata: PwrightCommonCallMetadata) -> CallMetadata {
    let timeout = metadata
        .timeout
        .map(|timeout| Duration::new(timeout.seconds, timeout.nanoseconds));
    CallMetadata::new(timeout, metadata.cancellation_id.map(CancellationId::new))
}

fn error_details_to_wire_ref(
    details: &DomContentErrorDetails,
) -> PwrightDomContentErrorDetailsRef<'_> {
    match details {
        DomContentErrorDetails::ContentChangedDuringNavigation(message) => {
            PwrightDomContentErrorDetailsRef::ContentChangedDuringNavigation { message }
        }
        DomContentErrorDetails::InvalidBackendResponse(message) => {
            PwrightDomContentErrorDetailsRef::InvalidBackendResponse { message }
        }
        DomContentErrorDetails::Resource(message) => {
            PwrightDomContentErrorDetailsRef::Resource { message }
        }
        DomContentErrorDetails::Backend(message) => {
            PwrightDomContentErrorDetailsRef::Backend { message }
        }
    }
}

fn operation_error_to_wire_ref(
    error: &OperationError<DomContentErrorDetails>,
) -> PwrightDomContentErrorRef<'_> {
    PwrightDomContentErrorRef {
        failure_kind: failure_kind_to_wire(error.kind()),
        message: error.message(),
        details: error.details().map(error_details_to_wire_ref),
    }
}

fn operation_error_from_wire(
    error: PwrightDomContentError,
) -> OperationError<DomContentErrorDetails> {
    OperationError::new(
        failure_kind_from_wire(error.failure_kind),
        error.message,
        error.details.map(Into::into),
    )
}

fn validate_operation_error(
    error: &OperationError<DomContentErrorDetails>,
    limit: usize,
) -> Result<(), DomContentCodecError> {
    validate_error_string(error.message(), limit)?;
    if let Some(details) = error.details() {
        let message = match details {
            DomContentErrorDetails::ContentChangedDuringNavigation(message)
            | DomContentErrorDetails::InvalidBackendResponse(message)
            | DomContentErrorDetails::Resource(message)
            | DomContentErrorDetails::Backend(message) => message,
        };
        validate_error_string(message, limit)?;
    }
    Ok(())
}

fn validate_error_string(value: &str, limit: usize) -> Result<(), DomContentCodecError> {
    if value.len() <= limit {
        Ok(())
    } else {
        Err(DomContentCodecError::ErrorStringTooLong {
            actual: value.len(),
            limit,
        })
    }
}

fn ensure_no_attachments(frame: &WireFrame) -> Result<(), DomContentCodecError> {
    if frame.attachments().is_empty() {
        Ok(())
    } else {
        Err(DomContentCodecError::UnexpectedAttachments {
            actual: frame.attachments().len(),
        })
    }
}

fn ensure_identity(
    field: &'static str,
    outer: u64,
    inner: u64,
) -> Result<(), DomContentCodecError> {
    if outer == inner {
        Ok(())
    } else {
        Err(DomContentCodecError::IdentityMismatch {
            field,
            outer,
            inner,
        })
    }
}

fn command_route<S: CommandSpec>(
    frame: &WireFrame,
) -> Result<(CallId, ObjectId), DomContentCodecError> {
    let FrameRoute::Command {
        call_id,
        target,
        method,
    } = frame.route()
    else {
        return Err(DomContentCodecError::WrongRoute(frame.route()));
    };
    if method != S::METHOD {
        return Err(DomContentCodecError::WrongMethod {
            expected: S::METHOD,
            actual: method,
        });
    }
    Ok((call_id, target))
}

fn result_route(frame: &WireFrame) -> Result<CallId, DomContentCodecError> {
    let FrameRoute::Result { call_id } = frame.route() else {
        return Err(DomContentCodecError::WrongRoute(frame.route()));
    };
    Ok(call_id)
}

fn wire_kind(value: &PwrightDomContent) -> u8 {
    match value {
        PwrightDomContent::FrameContentCommand { .. } => FRAME_CONTENT_COMMAND,
        PwrightDomContent::FrameContentResult { .. } => FRAME_CONTENT_RESULT,
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use pwright_cap_dom_content::{DomContentData, DomContentParams, FrameContent};
    use pwright_codec_binary::encode_value;
    use pwright_core::{
        BudgetedUtf8, CallId, CallMetadata, Command, CommandResult, ObjectId, ProtocolVersion,
        ResourceClass, ResourceLedger, ResourceLimits, ResourceScope,
    };
    use pwright_transport::{FrameLimits, WireCodec, WireFrame, WireProtocol};

    use super::{
        DomContentCodec, DomContentCodecError, DomContentCodecLimits,
        generated_schema::{PwrightDomContent, PwrightDomContentData, PwrightDomContentOutcome},
    };

    fn ledger(bytes: u64) -> ResourceLedger {
        ResourceLedger::new(
            ResourceLimits::deny_all(8, 8)
                .with_class(ResourceClass::DomContentBytes, bytes, bytes)
                .with_class(
                    ResourceClass::WireBytes,
                    bytes.saturating_mul(4),
                    bytes.saturating_mul(4),
                ),
        )
    }

    fn codec(ledger: &ResourceLedger) -> DomContentCodec {
        DomContentCodec::new(DomContentCodecLimits::new(128, 256), ledger.clone())
    }

    fn success_frame(
        codec: &DomContentCodec,
        attachment_index: u32,
        byte_length: u64,
        attachments: Vec<Vec<u8>>,
    ) -> WireFrame {
        let payload = encode_value(
            &PwrightDomContent::FrameContentResult {
                call_id: 3,
                outcome: PwrightDomContentOutcome::Success {
                    result: PwrightDomContentData {
                        attachment_index,
                        byte_length,
                    },
                },
            },
            codec.value_limits(),
        )
        .expect("test result payload should encode");
        WireFrame::new(
            ProtocolVersion::new(1, 0),
            pwright_transport::FrameRoute::Result {
                call_id: CallId::new(3),
            },
            payload,
            attachments,
            FrameLimits::new(1024, 2, 512, 2048),
        )
        .expect("test result frame should fit transport limits")
    }

    #[test]
    fn frame_content_command_round_trips_without_timeout() {
        // Arrange
        let ledger = ledger(256);
        let codec = codec(&ledger);
        let command = Command::<FrameContent>::new(
            CallId::new(7),
            ObjectId::new(9),
            CallMetadata::default(),
            DomContentParams,
        );

        // Act
        let parts = codec.encode_parts(&command).expect("command should encode");
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 0),
            parts.route(),
            parts.payload().to_vec(),
            parts.attachments().to_vec(),
            FrameLimits::new(1024, 1, 128, 1152),
        )
        .expect("frame should build");
        let decoded: Command<FrameContent> =
            codec.decode_frame(frame).expect("command should decode");

        // Assert
        assert_eq!(decoded.call_id(), CallId::new(7));
        assert_eq!(decoded.object_id(), ObjectId::new(9));
        assert_eq!(decoded.metadata(), CallMetadata::default());
    }

    #[test]
    fn timeout_metadata_is_rejected_before_encoding() {
        // Arrange
        let ledger = ledger(256);
        let codec = codec(&ledger);
        let command = Command::<FrameContent>::new(
            CallId::new(7),
            ObjectId::new(9),
            CallMetadata::new(Some(Duration::from_millis(1)), None),
            DomContentParams,
        );

        // Act
        let result = codec.encode_parts(&command);

        // Assert
        assert!(matches!(
            result,
            Err(DomContentCodecError::TimeoutUnsupported)
        ));
    }

    #[test]
    fn result_attachment_is_retained_as_budgeted_utf8_until_drop() {
        // Arrange
        let server_ledger = ledger(256);
        let client_ledger = ledger(256);
        let text = BudgetedUtf8::copy_from_str(
            &server_ledger,
            ResourceScope::Call(CallId::new(3)),
            ResourceClass::DomContentBytes,
            "<!DOCTYPE html><html></html>",
        )
        .unwrap();
        let result =
            CommandResult::<FrameContent>::new(CallId::new(3), Ok(DomContentData::new(text)));
        let server_codec = codec(&server_ledger);
        let client_codec = codec(&client_ledger);
        let protocol = WireProtocol::new_exact_directional(
            server_codec,
            ProtocolVersion::new(1, 0),
            FrameLimits::new(1024, 1, 128, 1152),
            FrameLimits::new(1024, 1, 128, 1152),
        );

        // Act
        let frame = protocol.encode(&result).expect("result should encode");
        let decoded: CommandResult<FrameContent> = client_codec
            .decode_frame(frame)
            .expect("result should decode");

        // Assert
        let content = decoded.outcome().as_ref().expect("result should succeed");
        assert_eq!(content.as_str(), "<!DOCTYPE html><html></html>");
        assert_eq!(
            client_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::DomContentBytes)
                .current,
            28
        );
        drop(decoded);
        assert_eq!(
            client_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::DomContentBytes)
                .current,
            0
        );
    }

    #[test]
    fn invalid_utf8_attachment_fails_before_result_publication() {
        // Arrange
        let server_ledger = ledger(256);
        let client_ledger = ledger(256);
        let text = BudgetedUtf8::copy_from_str(
            &server_ledger,
            ResourceScope::Call(CallId::new(3)),
            ResourceClass::DomContentBytes,
            "x",
        )
        .unwrap();
        let result =
            CommandResult::<FrameContent>::new(CallId::new(3), Ok(DomContentData::new(text)));
        let server_codec = codec(&server_ledger);
        let parts = server_codec.encode_parts(&result).unwrap();
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 0),
            parts.route(),
            parts.payload().to_vec(),
            vec![vec![0xff]],
            FrameLimits::new(1024, 1, 128, 1152),
        )
        .unwrap();

        // Act
        let decoded: Result<CommandResult<FrameContent>, _> =
            codec(&client_ledger).decode_frame(frame);
        let error = match decoded {
            Ok(_) => panic!("invalid UTF-8 must fail"),
            Err(error) => error,
        };

        // Assert
        assert!(matches!(error, DomContentCodecError::InvalidUtf8(_)));
        assert_eq!(
            client_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::DomContentBytes)
                .current,
            0
        );
    }

    #[test]
    fn corrupt_attachment_index_length_count_and_size_are_rejected_without_retention() {
        // Arrange
        let cases = [
            (
                success_frame(&codec(&ledger(256)), 1, 1, vec![b"x".to_vec()]),
                "index",
            ),
            (
                success_frame(&codec(&ledger(256)), 0, 2, vec![b"x".to_vec()]),
                "length",
            ),
            (success_frame(&codec(&ledger(256)), 0, 0, vec![]), "missing"),
            (
                success_frame(
                    &codec(&ledger(256)),
                    0,
                    1,
                    vec![b"x".to_vec(), b"y".to_vec()],
                ),
                "count",
            ),
            (
                success_frame(&codec(&ledger(256)), 0, 129, vec![vec![b'x'; 129]]),
                "size",
            ),
        ];
        let client_ledger = ledger(256);
        let client_codec = codec(&client_ledger);

        // Act and assert each malformed shape independently.
        for (frame, expected) in cases {
            let result: Result<CommandResult<FrameContent>, _> = client_codec.decode_frame(frame);
            let error = match result {
                Ok(_) => panic!("corrupt {expected} must fail"),
                Err(error) => error,
            };
            assert!(
                matches!(
                    (&error, expected),
                    (DomContentCodecError::InvalidAttachmentIndex(1), "index")
                        | (
                            DomContentCodecError::AttachmentLengthMismatch { .. },
                            "length"
                        )
                        | (DomContentCodecError::MissingContentAttachment, "missing")
                        | (
                            DomContentCodecError::UnexpectedAttachments { actual: 2 },
                            "count"
                        )
                        | (
                            DomContentCodecError::ContentTooLarge {
                                actual: 129,
                                limit: 128
                            },
                            "size"
                        )
                ),
                "unexpected {expected} error: {error}"
            );
            assert_eq!(
                client_ledger
                    .metrics()
                    .unwrap()
                    .class(ResourceClass::DomContentBytes)
                    .current,
                0
            );
        }
    }

    #[test]
    fn decoded_utf8_requires_its_declared_result_budget_before_publication() {
        // Arrange
        let client_ledger = ledger(2);
        let client_codec = codec(&client_ledger);
        let frame = success_frame(&client_codec, 0, 3, vec![b"abc".to_vec()]);

        // Act
        let decoded: Result<CommandResult<FrameContent>, _> = client_codec.decode_frame(frame);
        let error = match decoded {
            Ok(_) => panic!("result exceeding the caller ledger must fail"),
            Err(error) => error,
        };

        // Assert
        assert!(matches!(error, DomContentCodecError::Resource(_)));
        assert_eq!(
            client_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::DomContentBytes)
                .current,
            0
        );
    }
}
