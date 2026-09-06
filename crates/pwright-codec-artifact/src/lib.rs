//! Canonical bounded binary codec for optional Artifact and Stream commands.
//!
//! A successful `stream.read` carries exactly one attachment, including an
//! empty attachment at EOF. Decoding copies that attachment into an
//! independently budgeted `ArtifactReadResult`, so dropping the wire frame
//! cannot invalidate caller-visible bytes.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use pwright_cap_artifact::{
    ArtifactDelete, ArtifactErrorDetails, ArtifactFailure, ArtifactFailureResult,
    ArtifactOpenStream, ArtifactReadResult, StreamClose, StreamInitializer, StreamRead,
    StreamReadOptions,
};
use pwright_codec_binary::{
    BinaryValueDecodeError, BinaryValueEncodeError, BinaryValueLimits, decode_value, encode_value,
};
use pwright_core::{
    BudgetedBytes, CallId, CallMetadata, CancellationId, Command, CommandResult, CommandSpec,
    FailureKind, MethodTag, ObjectId, ObjectKindTag, OperationError, ResourceClass, ResourceError,
    ResourceLedger, ResourceScope,
};
use pwright_transport::{EncodedParts, FrameRoute, WireCodec, WireFrame};

const OPEN_STREAM_COMMAND: u8 = 1;
const OPEN_STREAM_RESULT: u8 = 2;
const FAILURE_COMMAND: u8 = 3;
const FAILURE_RESULT: u8 = 4;
const DELETE_COMMAND: u8 = 5;
const DELETE_RESULT: u8 = 6;
const READ_COMMAND: u8 = 7;
const READ_RESULT: u8 = 8;
const CLOSE_COMMAND: u8 = 9;
const CLOSE_RESULT: u8 = 10;

#[doc(hidden)]
#[rustfmt::skip]
pub mod generated_schema;

pub use generated_schema::{
    CATALOG as ARTIFACT_SCHEMA_CATALOG, COMMANDS as ARTIFACT_COMMAND_SCHEMAS,
    ROOT_SCHEMA as ARTIFACT_SCHEMA,
};

use generated_schema::{
    PwrightArtifact, PwrightArtifactArtifactDeleteOptions, PwrightArtifactArtifactFailureData,
    PwrightArtifactArtifactFailureOptions, PwrightArtifactArtifactOpenStreamOptions,
    PwrightArtifactError, PwrightArtifactErrorDetails, PwrightArtifactFailureOutcome,
    PwrightArtifactReadOutcome, PwrightArtifactStreamCloseOptions,
    PwrightArtifactStreamInitializer, PwrightArtifactStreamOutcome, PwrightArtifactStreamReadData,
    PwrightArtifactStreamReadOptions, PwrightArtifactUnitOutcome, PwrightCommonCallMetadata,
    PwrightCommonFailureKind,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArtifactCodecLimits {
    pub read_bytes: usize,
    pub error_string_bytes: usize,
}

impl ArtifactCodecLimits {
    pub const fn new(read_bytes: usize, error_string_bytes: usize) -> Self {
        Self {
            read_bytes,
            error_string_bytes,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ArtifactCodec {
    limits: ArtifactCodecLimits,
    result_ledger: ResourceLedger,
}

impl ArtifactCodec {
    pub const fn new(limits: ArtifactCodecLimits, result_ledger: ResourceLedger) -> Self {
        Self {
            limits,
            result_ledger,
        }
    }

    pub const fn limits(&self) -> ArtifactCodecLimits {
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
            64,
        )
    }
}

#[derive(Debug)]
pub enum ArtifactCodecError {
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
    UnexpectedAttachments {
        actual: usize,
    },
    MissingReadAttachment,
    InvalidAttachmentIndex(u32),
    AttachmentLengthMismatch {
        declared: u64,
        actual: usize,
    },
    ReadTooLarge {
        actual: usize,
        limit: usize,
    },
    InvalidReadSize {
        requested: u32,
        limit: usize,
    },
    ErrorStringTooLong {
        actual: usize,
        limit: usize,
    },
    ControlMetadataUnsupported,
    WrongPayloadKind {
        expected: u8,
        actual: u8,
    },
    Resource(ResourceError),
    Encode(BinaryValueEncodeError),
    Decode(BinaryValueDecodeError),
}

impl Display for ArtifactCodecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongRoute(route) => write!(formatter, "artifact payload arrived on {route:?}"),
            Self::WrongMethod { expected, actual } => {
                write!(
                    formatter,
                    "expected artifact method {expected}, received {actual}"
                )
            }
            Self::IdentityMismatch {
                field,
                outer,
                inner,
            } => write!(
                formatter,
                "artifact payload {field} {inner} does not match outer route {outer}"
            ),
            Self::UnexpectedAttachments { actual } => write!(
                formatter,
                "artifact payload has {actual} attachment(s) where none are allowed"
            ),
            Self::MissingReadAttachment => {
                formatter.write_str("successful stream read has no attachment")
            }
            Self::InvalidAttachmentIndex(index) => {
                write!(
                    formatter,
                    "stream read attachment index {index} is not zero"
                )
            }
            Self::AttachmentLengthMismatch { declared, actual } => write!(
                formatter,
                "stream read declares {declared} bytes but attachment contains {actual}"
            ),
            Self::ReadTooLarge { actual, limit } => write!(
                formatter,
                "stream read contains {actual} bytes, exceeding limit {limit}"
            ),
            Self::InvalidReadSize { requested, limit } => write!(
                formatter,
                "stream read requests {requested} bytes, outside 1..={limit}"
            ),
            Self::ErrorStringTooLong { actual, limit } => write!(
                formatter,
                "artifact error string has {actual} bytes, exceeding limit {limit}"
            ),
            Self::ControlMetadataUnsupported => formatter
                .write_str("artifact lifecycle commands do not accept timeout or cancellation"),
            Self::WrongPayloadKind { expected, actual } => write!(
                formatter,
                "expected artifact payload kind {expected}, received {actual}"
            ),
            Self::Resource(error) => Display::fmt(error, formatter),
            Self::Encode(error) => Display::fmt(error, formatter),
            Self::Decode(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ArtifactCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resource(error) => Some(error),
            Self::Encode(error) => Some(error),
            Self::Decode(error) => Some(error),
            _ => None,
        }
    }
}

macro_rules! impl_empty_command_codec {
    ($spec:ty, $variant:ident, $options:ident, $kind:expr) => {
        impl WireCodec<Command<$spec>> for ArtifactCodec {
            type Error = ArtifactCodecError;

            fn encode_parts(&self, command: &Command<$spec>) -> Result<EncodedParts, Self::Error> {
                ensure_empty_metadata(command.metadata())?;
                encode_command::<$spec>(
                    self,
                    command,
                    PwrightArtifact::$variant {
                        call_id: command.call_id().get(),
                        object_id: command.object_id().get(),
                        metadata: empty_metadata(),
                        options: $options {},
                    },
                )
            }

            fn decode_frame(&self, frame: WireFrame) -> Result<Command<$spec>, Self::Error> {
                let (outer_call_id, outer_object_id) = command_route::<$spec>(&frame)?;
                ensure_no_attachments(&frame)?;
                match decode_value::<PwrightArtifact>(frame.payload(), self.value_limits())
                    .map_err(ArtifactCodecError::Decode)?
                {
                    PwrightArtifact::$variant {
                        call_id,
                        object_id,
                        metadata,
                        options: _,
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
                    other => Err(ArtifactCodecError::WrongPayloadKind {
                        expected: $kind,
                        actual: wire_kind(&other),
                    }),
                }
            }
        }
    };
}

impl_empty_command_codec!(
    ArtifactOpenStream,
    ArtifactOpenStreamCommand,
    PwrightArtifactArtifactOpenStreamOptions,
    OPEN_STREAM_COMMAND
);
impl_empty_command_codec!(
    ArtifactFailure,
    ArtifactFailureCommand,
    PwrightArtifactArtifactFailureOptions,
    FAILURE_COMMAND
);
impl_empty_command_codec!(
    ArtifactDelete,
    ArtifactDeleteCommand,
    PwrightArtifactArtifactDeleteOptions,
    DELETE_COMMAND
);
impl_empty_command_codec!(
    StreamClose,
    StreamCloseCommand,
    PwrightArtifactStreamCloseOptions,
    CLOSE_COMMAND
);

impl WireCodec<Command<StreamRead>> for ArtifactCodec {
    type Error = ArtifactCodecError;

    fn encode_parts(&self, command: &Command<StreamRead>) -> Result<EncodedParts, Self::Error> {
        ensure_empty_metadata(command.metadata())?;
        validate_read_size(command.params().max_bytes(), self.limits.read_bytes)?;
        encode_command::<StreamRead>(
            self,
            command,
            PwrightArtifact::StreamReadCommand {
                call_id: command.call_id().get(),
                object_id: command.object_id().get(),
                metadata: empty_metadata(),
                options: PwrightArtifactStreamReadOptions {
                    max_bytes: command.params().max_bytes(),
                },
            },
        )
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<StreamRead>, Self::Error> {
        let (outer_call_id, outer_object_id) = command_route::<StreamRead>(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightArtifact>(frame.payload(), self.value_limits())
            .map_err(ArtifactCodecError::Decode)?
        {
            PwrightArtifact::StreamReadCommand {
                call_id,
                object_id,
                metadata,
                options,
            } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                ensure_identity("object ID", outer_object_id.get(), object_id)?;
                let metadata = metadata_from_wire(metadata);
                ensure_empty_metadata(metadata)?;
                validate_read_size(options.max_bytes, self.limits.read_bytes)?;
                Ok(Command::new(
                    CallId::new(call_id),
                    ObjectId::new(object_id),
                    metadata,
                    StreamReadOptions::new(options.max_bytes),
                ))
            }
            other => Err(ArtifactCodecError::WrongPayloadKind {
                expected: READ_COMMAND,
                actual: wire_kind(&other),
            }),
        }
    }
}

macro_rules! impl_unit_result_codec {
    ($spec:ty, $variant:ident, $kind:expr) => {
        impl WireCodec<CommandResult<$spec>> for ArtifactCodec {
            type Error = ArtifactCodecError;

            fn encode_parts(
                &self,
                result: &CommandResult<$spec>,
            ) -> Result<EncodedParts, Self::Error> {
                let outcome =
                    unit_outcome_to_wire(result.outcome(), self.limits.error_string_bytes)?;
                encode_result(
                    self,
                    result.call_id(),
                    PwrightArtifact::$variant {
                        call_id: result.call_id().get(),
                        outcome,
                    },
                    Vec::new(),
                )
            }

            fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<$spec>, Self::Error> {
                let outer_call_id = result_route(&frame)?;
                ensure_no_attachments(&frame)?;
                match decode_value::<PwrightArtifact>(frame.payload(), self.value_limits())
                    .map_err(ArtifactCodecError::Decode)?
                {
                    PwrightArtifact::$variant { call_id, outcome } => {
                        ensure_identity("call ID", outer_call_id.get(), call_id)?;
                        let outcome = unit_outcome_from_wire(outcome);
                        validate_outcome_error(&outcome, self.limits.error_string_bytes)?;
                        Ok(CommandResult::new(CallId::new(call_id), outcome))
                    }
                    other => Err(ArtifactCodecError::WrongPayloadKind {
                        expected: $kind,
                        actual: wire_kind(&other),
                    }),
                }
            }
        }
    };
}

impl_unit_result_codec!(ArtifactDelete, ArtifactDeleteResult, DELETE_RESULT);
impl_unit_result_codec!(StreamClose, StreamCloseResult, CLOSE_RESULT);

impl WireCodec<CommandResult<ArtifactOpenStream>> for ArtifactCodec {
    type Error = ArtifactCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<ArtifactOpenStream>,
    ) -> Result<EncodedParts, Self::Error> {
        let outcome = match result.outcome() {
            Ok(result) => PwrightArtifactStreamOutcome::Success {
                result: PwrightArtifactStreamInitializer {
                    object_id: result.object_id.get(),
                    parent_id: result.parent_id.get(),
                    object_kind: result.object_kind.get(),
                },
            },
            Err(error) => {
                validate_operation_error(error, self.limits.error_string_bytes)?;
                PwrightArtifactStreamOutcome::Error {
                    error: operation_error_to_wire(error),
                }
            }
        };
        encode_result(
            self,
            result.call_id(),
            PwrightArtifact::ArtifactOpenStreamResult {
                call_id: result.call_id().get(),
                outcome,
            },
            Vec::new(),
        )
    }

    fn decode_frame(
        &self,
        frame: WireFrame,
    ) -> Result<CommandResult<ArtifactOpenStream>, Self::Error> {
        let outer_call_id = result_route(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightArtifact>(frame.payload(), self.value_limits())
            .map_err(ArtifactCodecError::Decode)?
        {
            PwrightArtifact::ArtifactOpenStreamResult { call_id, outcome } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                let outcome = match outcome {
                    PwrightArtifactStreamOutcome::Success { result } => Ok(StreamInitializer::new(
                        ObjectId::new(result.object_id),
                        ObjectId::new(result.parent_id),
                        ObjectKindTag::new(result.object_kind),
                    )),
                    PwrightArtifactStreamOutcome::Error { error } => {
                        Err(operation_error_from_wire(error))
                    }
                };
                validate_outcome_error(&outcome, self.limits.error_string_bytes)?;
                Ok(CommandResult::new(CallId::new(call_id), outcome))
            }
            other => Err(ArtifactCodecError::WrongPayloadKind {
                expected: OPEN_STREAM_RESULT,
                actual: wire_kind(&other),
            }),
        }
    }
}

impl WireCodec<CommandResult<ArtifactFailure>> for ArtifactCodec {
    type Error = ArtifactCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<ArtifactFailure>,
    ) -> Result<EncodedParts, Self::Error> {
        let outcome = match result.outcome() {
            Ok(result) => {
                if let Some(error) = result.error() {
                    validate_string(error, self.limits.error_string_bytes)?;
                }
                PwrightArtifactFailureOutcome::Success {
                    result: PwrightArtifactArtifactFailureData {
                        error: result.error().map(str::to_owned),
                    },
                }
            }
            Err(error) => {
                validate_operation_error(error, self.limits.error_string_bytes)?;
                PwrightArtifactFailureOutcome::Error {
                    error: operation_error_to_wire(error),
                }
            }
        };
        encode_result(
            self,
            result.call_id(),
            PwrightArtifact::ArtifactFailureResult {
                call_id: result.call_id().get(),
                outcome,
            },
            Vec::new(),
        )
    }

    fn decode_frame(
        &self,
        frame: WireFrame,
    ) -> Result<CommandResult<ArtifactFailure>, Self::Error> {
        let outer_call_id = result_route(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightArtifact>(frame.payload(), self.value_limits())
            .map_err(ArtifactCodecError::Decode)?
        {
            PwrightArtifact::ArtifactFailureResult { call_id, outcome } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                let outcome = match outcome {
                    PwrightArtifactFailureOutcome::Success { result } => {
                        if let Some(error) = result.error.as_deref() {
                            validate_string(error, self.limits.error_string_bytes)?;
                        }
                        Ok(ArtifactFailureResult::copy_from_str(
                            &self.result_ledger,
                            ResourceScope::Call(CallId::new(call_id)),
                            result.error.as_deref(),
                        )
                        .map_err(ArtifactCodecError::Resource)?)
                    }
                    PwrightArtifactFailureOutcome::Error { error } => {
                        Err(operation_error_from_wire(error))
                    }
                };
                validate_outcome_error(&outcome, self.limits.error_string_bytes)?;
                Ok(CommandResult::new(CallId::new(call_id), outcome))
            }
            other => Err(ArtifactCodecError::WrongPayloadKind {
                expected: FAILURE_RESULT,
                actual: wire_kind(&other),
            }),
        }
    }
}

impl WireCodec<CommandResult<StreamRead>> for ArtifactCodec {
    type Error = ArtifactCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<StreamRead>,
    ) -> Result<EncodedParts, Self::Error> {
        let mut attachments = Vec::new();
        let outcome = match result.outcome() {
            Ok(result) => {
                validate_read_length(result.len(), self.limits.read_bytes)?;
                let byte_length =
                    u64::try_from(result.len()).map_err(|_| ArtifactCodecError::ReadTooLarge {
                        actual: result.len(),
                        limit: self.limits.read_bytes,
                    })?;
                attachments.push(result.as_bytes().to_vec());
                PwrightArtifactReadOutcome::Success {
                    result: PwrightArtifactStreamReadData {
                        attachment_index: 0,
                        byte_length,
                        eof: result.eof(),
                    },
                }
            }
            Err(error) => {
                validate_operation_error(error, self.limits.error_string_bytes)?;
                PwrightArtifactReadOutcome::Error {
                    error: operation_error_to_wire(error),
                }
            }
        };
        encode_result(
            self,
            result.call_id(),
            PwrightArtifact::StreamReadResult {
                call_id: result.call_id().get(),
                outcome,
            },
            attachments,
        )
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<StreamRead>, Self::Error> {
        let outer_call_id = result_route(&frame)?;
        match decode_value::<PwrightArtifact>(frame.payload(), self.value_limits())
            .map_err(ArtifactCodecError::Decode)?
        {
            PwrightArtifact::StreamReadResult { call_id, outcome } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                let outcome = match outcome {
                    PwrightArtifactReadOutcome::Success { result } => {
                        let bytes = decode_read_attachment(
                            &frame,
                            result.attachment_index,
                            result.byte_length,
                            self.limits.read_bytes,
                            &self.result_ledger,
                            CallId::new(call_id),
                        )?;
                        Ok(ArtifactReadResult::new(bytes, result.eof))
                    }
                    PwrightArtifactReadOutcome::Error { error } => {
                        ensure_no_attachments(&frame)?;
                        Err(operation_error_from_wire(error))
                    }
                };
                validate_outcome_error(&outcome, self.limits.error_string_bytes)?;
                Ok(CommandResult::new(CallId::new(call_id), outcome))
            }
            other => Err(ArtifactCodecError::WrongPayloadKind {
                expected: READ_RESULT,
                actual: wire_kind(&other),
            }),
        }
    }
}

fn encode_command<S: CommandSpec>(
    codec: &ArtifactCodec,
    command: &Command<S>,
    wire: PwrightArtifact,
) -> Result<EncodedParts, ArtifactCodecError> {
    let payload = encode_value(&wire, codec.value_limits()).map_err(ArtifactCodecError::Encode)?;
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
    codec: &ArtifactCodec,
    call_id: CallId,
    wire: PwrightArtifact,
    attachments: Vec<Vec<u8>>,
) -> Result<EncodedParts, ArtifactCodecError> {
    let payload = encode_value(&wire, codec.value_limits()).map_err(ArtifactCodecError::Encode)?;
    Ok(EncodedParts::new(
        FrameRoute::Result { call_id },
        payload,
        attachments,
    ))
}

fn decode_read_attachment(
    frame: &WireFrame,
    index: u32,
    declared: u64,
    limit: usize,
    ledger: &ResourceLedger,
    call_id: CallId,
) -> Result<BudgetedBytes, ArtifactCodecError> {
    if frame.attachments().is_empty() {
        return Err(ArtifactCodecError::MissingReadAttachment);
    }
    if frame.attachments().len() != 1 {
        return Err(ArtifactCodecError::UnexpectedAttachments {
            actual: frame.attachments().len(),
        });
    }
    if index != 0 {
        return Err(ArtifactCodecError::InvalidAttachmentIndex(index));
    }
    let attachment = &frame.attachments()[0];
    if u64::try_from(attachment.len()).ok() != Some(declared) {
        return Err(ArtifactCodecError::AttachmentLengthMismatch {
            declared,
            actual: attachment.len(),
        });
    }
    validate_read_length(attachment.len(), limit)?;
    BudgetedBytes::copy_from_slice(
        ledger,
        ResourceScope::Call(call_id),
        ResourceClass::ArtifactBytes,
        attachment.as_bytes(),
    )
    .map_err(ArtifactCodecError::Resource)
}

fn validate_read_size(requested: u32, limit: usize) -> Result<(), ArtifactCodecError> {
    let requested_usize = usize::try_from(requested).unwrap_or(usize::MAX);
    if requested == 0 || requested_usize > limit {
        Err(ArtifactCodecError::InvalidReadSize { requested, limit })
    } else {
        Ok(())
    }
}

fn validate_read_length(actual: usize, limit: usize) -> Result<(), ArtifactCodecError> {
    if actual > limit {
        Err(ArtifactCodecError::ReadTooLarge { actual, limit })
    } else {
        Ok(())
    }
}

fn validate_string(value: &str, limit: usize) -> Result<(), ArtifactCodecError> {
    if value.len() > limit {
        Err(ArtifactCodecError::ErrorStringTooLong {
            actual: value.len(),
            limit,
        })
    } else {
        Ok(())
    }
}

fn validate_operation_error(
    error: &OperationError<ArtifactErrorDetails>,
    limit: usize,
) -> Result<(), ArtifactCodecError> {
    validate_string(error.message(), limit)?;
    if let Some(details) = error.details() {
        let message = match details {
            ArtifactErrorDetails::InvalidOptions(message)
            | ArtifactErrorDetails::Resource(message)
            | ArtifactErrorDetails::Backend(message) => message,
        };
        validate_string(message, limit)?;
    }
    Ok(())
}

fn validate_outcome_error<T>(
    outcome: &Result<T, OperationError<ArtifactErrorDetails>>,
    limit: usize,
) -> Result<(), ArtifactCodecError> {
    if let Err(error) = outcome {
        validate_operation_error(error, limit)?;
    }
    Ok(())
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

fn ensure_empty_metadata(metadata: CallMetadata) -> Result<(), ArtifactCodecError> {
    if metadata == CallMetadata::default() {
        Ok(())
    } else {
        Err(ArtifactCodecError::ControlMetadataUnsupported)
    }
}

fn ensure_no_attachments(frame: &WireFrame) -> Result<(), ArtifactCodecError> {
    if frame.attachments().is_empty() {
        Ok(())
    } else {
        Err(ArtifactCodecError::UnexpectedAttachments {
            actual: frame.attachments().len(),
        })
    }
}

fn ensure_identity(field: &'static str, outer: u64, inner: u64) -> Result<(), ArtifactCodecError> {
    if outer == inner {
        Ok(())
    } else {
        Err(ArtifactCodecError::IdentityMismatch {
            field,
            outer,
            inner,
        })
    }
}

fn command_route<S: CommandSpec>(
    frame: &WireFrame,
) -> Result<(CallId, ObjectId), ArtifactCodecError> {
    let FrameRoute::Command {
        call_id,
        target,
        method,
    } = frame.route()
    else {
        return Err(ArtifactCodecError::WrongRoute(frame.route()));
    };
    if method != S::METHOD {
        return Err(ArtifactCodecError::WrongMethod {
            expected: S::METHOD,
            actual: method,
        });
    }
    Ok((call_id, target))
}

fn result_route(frame: &WireFrame) -> Result<CallId, ArtifactCodecError> {
    let FrameRoute::Result { call_id } = frame.route() else {
        return Err(ArtifactCodecError::WrongRoute(frame.route()));
    };
    Ok(call_id)
}

fn wire_kind(value: &PwrightArtifact) -> u8 {
    match value {
        PwrightArtifact::ArtifactOpenStreamCommand { .. } => OPEN_STREAM_COMMAND,
        PwrightArtifact::ArtifactOpenStreamResult { .. } => OPEN_STREAM_RESULT,
        PwrightArtifact::ArtifactFailureCommand { .. } => FAILURE_COMMAND,
        PwrightArtifact::ArtifactFailureResult { .. } => FAILURE_RESULT,
        PwrightArtifact::ArtifactDeleteCommand { .. } => DELETE_COMMAND,
        PwrightArtifact::ArtifactDeleteResult { .. } => DELETE_RESULT,
        PwrightArtifact::StreamReadCommand { .. } => READ_COMMAND,
        PwrightArtifact::StreamReadResult { .. } => READ_RESULT,
        PwrightArtifact::StreamCloseCommand { .. } => CLOSE_COMMAND,
        PwrightArtifact::StreamCloseResult { .. } => CLOSE_RESULT,
    }
}

fn unit_outcome_to_wire(
    outcome: &Result<(), OperationError<ArtifactErrorDetails>>,
    limit: usize,
) -> Result<PwrightArtifactUnitOutcome, ArtifactCodecError> {
    match outcome {
        Ok(()) => Ok(PwrightArtifactUnitOutcome::Success),
        Err(error) => {
            validate_operation_error(error, limit)?;
            Ok(PwrightArtifactUnitOutcome::Error {
                error: operation_error_to_wire(error),
            })
        }
    }
}

fn unit_outcome_from_wire(
    outcome: PwrightArtifactUnitOutcome,
) -> Result<(), OperationError<ArtifactErrorDetails>> {
    match outcome {
        PwrightArtifactUnitOutcome::Success => Ok(()),
        PwrightArtifactUnitOutcome::Error { error } => Err(operation_error_from_wire(error)),
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

fn error_details_to_wire(details: &ArtifactErrorDetails) -> PwrightArtifactErrorDetails {
    match details {
        ArtifactErrorDetails::InvalidOptions(message) => {
            PwrightArtifactErrorDetails::InvalidOptions {
                message: message.clone(),
            }
        }
        ArtifactErrorDetails::Resource(message) => PwrightArtifactErrorDetails::Resource {
            message: message.clone(),
        },
        ArtifactErrorDetails::Backend(message) => PwrightArtifactErrorDetails::Backend {
            message: message.clone(),
        },
    }
}

fn operation_error_to_wire(error: &OperationError<ArtifactErrorDetails>) -> PwrightArtifactError {
    PwrightArtifactError {
        failure_kind: failure_kind_to_wire(error.kind()),
        message: error.message().to_owned(),
        details: error.details().map(error_details_to_wire),
    }
}

fn operation_error_from_wire(error: PwrightArtifactError) -> OperationError<ArtifactErrorDetails> {
    OperationError::new(
        failure_kind_from_wire(error.failure_kind),
        error.message,
        error.details.map(Into::into),
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use pwright_cap_artifact::{
        ArtifactDelete, ArtifactErrorDetails, ArtifactFailure, ArtifactFailureResult,
        ArtifactOpenStream, StreamClose, StreamInitializer, StreamRead, StreamReadOptions,
    };
    use pwright_core::{
        BudgetedBytes, CallId, CallMetadata, CancellationId, Command, CommandResult, FailureKind,
        ObjectId, ObjectKindTag, OperationError, ProtocolVersion, ResourceClass, ResourceLedger,
        ResourceLimits, ResourceScope,
    };
    use pwright_transport::{FrameLimits, WireCodec, WireFrame, WireProtocol};

    use super::{ArtifactCodec, ArtifactCodecError, ArtifactCodecLimits};

    fn ledger(bytes: u64) -> ResourceLedger {
        ResourceLedger::new(
            ResourceLimits::deny_all(8, 8)
                .with_class(ResourceClass::ArtifactBytes, bytes, bytes)
                .with_class(
                    ResourceClass::WireBytes,
                    bytes.saturating_mul(16),
                    bytes.saturating_mul(16),
                ),
        )
    }

    fn codec(ledger: &ResourceLedger) -> ArtifactCodec {
        ArtifactCodec::new(ArtifactCodecLimits::new(16, 64), ledger.clone())
    }

    fn protocol(codec: ArtifactCodec) -> WireProtocol<ArtifactCodec> {
        WireProtocol::new_exact_directional(
            codec,
            ProtocolVersion::new(1, 0),
            FrameLimits::new(4096, 1, 16, 4112),
            FrameLimits::new(4096, 1, 16, 4112),
        )
    }

    #[test]
    fn all_command_shapes_round_trip_on_their_exact_routes() {
        // Arrange
        let resource_ledger = ledger(64);
        let protocol = protocol(codec(&resource_ledger));
        let open = Command::<ArtifactOpenStream>::new(
            CallId::new(1),
            ObjectId::new(10),
            CallMetadata::default(),
            (),
        );
        let failure = Command::<ArtifactFailure>::new(
            CallId::new(2),
            ObjectId::new(10),
            CallMetadata::default(),
            (),
        );
        let delete = Command::<ArtifactDelete>::new(
            CallId::new(3),
            ObjectId::new(10),
            CallMetadata::default(),
            (),
        );
        let read = Command::<StreamRead>::new(
            CallId::new(4),
            ObjectId::new(11),
            CallMetadata::default(),
            StreamReadOptions::new(8),
        );
        let close = Command::<StreamClose>::new(
            CallId::new(5),
            ObjectId::new(11),
            CallMetadata::default(),
            (),
        );

        // Act
        let open = protocol.decode::<Command<ArtifactOpenStream>>(protocol.encode(&open).unwrap());
        let failure =
            protocol.decode::<Command<ArtifactFailure>>(protocol.encode(&failure).unwrap());
        let delete = protocol.decode::<Command<ArtifactDelete>>(protocol.encode(&delete).unwrap());
        let read = protocol.decode::<Command<StreamRead>>(protocol.encode(&read).unwrap());
        let close = protocol.decode::<Command<StreamClose>>(protocol.encode(&close).unwrap());

        // Assert
        assert_eq!(open.unwrap().object_id(), ObjectId::new(10));
        assert_eq!(failure.unwrap().call_id(), CallId::new(2));
        assert_eq!(delete.unwrap().call_id(), CallId::new(3));
        assert_eq!(read.unwrap().params().max_bytes(), 8);
        assert_eq!(close.unwrap().object_id(), ObjectId::new(11));
    }

    #[test]
    fn typed_non_byte_results_and_errors_round_trip() {
        // Arrange
        let resource_ledger = ledger(64);
        let protocol = protocol(codec(&resource_ledger));
        let open = CommandResult::<ArtifactOpenStream>::new(
            CallId::new(1),
            Ok(StreamInitializer::new(
                ObjectId::new(11),
                ObjectId::new(10),
                ObjectKindTag::new(10),
            )),
        );
        let failure = CommandResult::<ArtifactFailure>::new(
            CallId::new(2),
            Ok(ArtifactFailureResult::copy_from_str(
                &resource_ledger,
                ResourceScope::Call(CallId::new(2)),
                Some("archive failed"),
            )
            .unwrap()),
        );
        let delete = CommandResult::<ArtifactDelete>::new(
            CallId::new(3),
            Err(OperationError::new(
                FailureKind::Backend,
                "delete failed",
                Some(ArtifactErrorDetails::Backend("disk unavailable".into())),
            )),
        );
        let close = CommandResult::<StreamClose>::new(CallId::new(4), Ok(()));

        // Act
        let open = protocol
            .decode::<CommandResult<ArtifactOpenStream>>(protocol.encode(&open).unwrap())
            .unwrap();
        let failure = protocol
            .decode::<CommandResult<ArtifactFailure>>(protocol.encode(&failure).unwrap())
            .unwrap();
        let delete = protocol
            .decode::<CommandResult<ArtifactDelete>>(protocol.encode(&delete).unwrap())
            .unwrap();
        let close = protocol
            .decode::<CommandResult<StreamClose>>(protocol.encode(&close).unwrap())
            .unwrap();

        // Assert
        assert_eq!(
            open.outcome().as_ref().unwrap().object_id,
            ObjectId::new(11)
        );
        assert_eq!(
            failure.outcome().as_ref().unwrap().error(),
            Some("archive failed")
        );
        assert_eq!(
            delete.outcome().as_ref().unwrap_err().kind(),
            FailureKind::Backend
        );
        assert!(close.outcome().is_ok());
    }

    #[test]
    fn read_attachment_is_copied_into_an_independent_artifact_budget() {
        // Arrange
        let source_ledger = ledger(64);
        let result_ledger = ledger(64);
        let bytes = BudgetedBytes::copy_from_slice(
            &source_ledger,
            ResourceScope::Call(CallId::new(7)),
            ResourceClass::ArtifactBytes,
            b"chunk",
        )
        .unwrap();
        let result = CommandResult::<StreamRead>::new(
            CallId::new(7),
            Ok(pwright_cap_artifact::ArtifactReadResult::new(bytes, false)),
        );
        let server_codec = codec(&source_ledger);
        let client_codec = codec(&result_ledger);
        let protocol = protocol(server_codec);

        // Act
        let frame = protocol.encode(&result).unwrap();
        let decoded: CommandResult<StreamRead> = client_codec
            .decode_frame(frame)
            .expect("read result should decode");

        // Assert
        let read = decoded.outcome().as_ref().unwrap();
        assert_eq!(read.as_bytes(), b"chunk");
        assert!(!read.eof());
        assert_eq!(
            result_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ArtifactBytes)
                .current,
            5
        );
        drop(decoded);
        assert_eq!(
            result_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ArtifactBytes)
                .current,
            0
        );
    }

    #[test]
    fn empty_eof_still_round_trips_as_one_structural_attachment() {
        // Arrange
        let source_ledger = ledger(64);
        let result_ledger = ledger(64);
        let bytes = BudgetedBytes::copy_from_slice(
            &source_ledger,
            ResourceScope::Call(CallId::new(8)),
            ResourceClass::ArtifactBytes,
            b"",
        )
        .unwrap();
        let result = CommandResult::<StreamRead>::new(
            CallId::new(8),
            Ok(pwright_cap_artifact::ArtifactReadResult::new(bytes, true)),
        );
        let server_codec = codec(&source_ledger);
        let client_codec = codec(&result_ledger);

        // Act
        let parts = server_codec.encode_parts(&result).unwrap();
        let attachment_count = parts.attachments().len();
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 0),
            parts.route(),
            parts.payload().to_vec(),
            parts.attachments().to_vec(),
            FrameLimits::new(4096, 1, 16, 4112),
        )
        .unwrap();
        let decoded: CommandResult<StreamRead> = client_codec.decode_frame(frame).unwrap();

        // Assert
        assert_eq!(attachment_count, 1);
        let read = decoded.outcome().as_ref().unwrap();
        assert!(read.is_empty());
        assert!(read.eof());
    }

    #[test]
    fn invalid_read_size_and_control_metadata_fail_before_encoding() {
        // Arrange
        let resource_ledger = ledger(64);
        let artifact_codec = codec(&resource_ledger);
        let zero = Command::<StreamRead>::new(
            CallId::new(1),
            ObjectId::new(11),
            CallMetadata::default(),
            StreamReadOptions::new(0),
        );
        let oversized = Command::<StreamRead>::new(
            CallId::new(2),
            ObjectId::new(11),
            CallMetadata::default(),
            StreamReadOptions::new(17),
        );
        let metadata = Command::<ArtifactDelete>::new(
            CallId::new(3),
            ObjectId::new(10),
            CallMetadata::new(Some(Duration::from_secs(1)), Some(CancellationId::new(9))),
            (),
        );

        // Act
        let zero = artifact_codec.encode_parts(&zero);
        let oversized = artifact_codec.encode_parts(&oversized);
        let metadata = artifact_codec.encode_parts(&metadata);

        // Assert
        assert!(matches!(
            zero,
            Err(ArtifactCodecError::InvalidReadSize { requested: 0, .. })
        ));
        assert!(matches!(
            oversized,
            Err(ArtifactCodecError::InvalidReadSize { requested: 17, .. })
        ));
        assert!(matches!(
            metadata,
            Err(ArtifactCodecError::ControlMetadataUnsupported)
        ));
    }

    #[test]
    fn malformed_read_attachment_is_rejected_before_result_budgeting() {
        // Arrange
        let source_ledger = ledger(64);
        let result_ledger = ledger(64);
        let bytes = BudgetedBytes::copy_from_slice(
            &source_ledger,
            ResourceScope::Call(CallId::new(9)),
            ResourceClass::ArtifactBytes,
            b"data",
        )
        .unwrap();
        let result = CommandResult::<StreamRead>::new(
            CallId::new(9),
            Ok(pwright_cap_artifact::ArtifactReadResult::new(bytes, true)),
        );
        let server_codec = codec(&source_ledger);
        let client_codec = codec(&result_ledger);
        let parts = server_codec.encode_parts(&result).unwrap();
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 0),
            parts.route(),
            parts.payload().to_vec(),
            vec![b"bad".to_vec()],
            FrameLimits::new(4096, 1, 16, 4112),
        )
        .unwrap();

        // Act
        let decoded: Result<CommandResult<StreamRead>, _> = client_codec.decode_frame(frame);

        // Assert
        assert!(matches!(
            decoded,
            Err(ArtifactCodecError::AttachmentLengthMismatch {
                declared: 4,
                actual: 3
            })
        ));
        assert_eq!(
            result_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ArtifactBytes)
                .current,
            0
        );
    }
}
