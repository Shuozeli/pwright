//! Canonical bounded binary codec for handle evaluation.
//!
//! The generated schema owns wire layout and metadata. This adapter performs
//! the semantic validation that a recursive JavaScript value graph requires.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::num::NonZeroU32;

use pwright_cap_evaluation::{
    EvaluationMode, EvaluationNumber, EvaluationNumberError, EvaluationValue,
    EvaluationValueLimits, TypedArrayKind,
};
use pwright_cap_handle_evaluation::{
    EvaluateHandle, EvaluateHandleParams, GetPropertiesOptions, GetPropertyParams, HandleEvaluate,
    HandleEvaluateHandle, HandleEvaluateParams, HandleEvaluationErrorDetails,
    HandleEvaluationProperty, HandleEvaluationValue, HandleEvaluationValueError,
    HandleGetProperties, HandleGetProperty, HandleInitializer, HandleJsonValue,
    HandlePropertiesResult, HandlePropertyInitializer, HandleValueResult, JsonValueOptions,
};
use pwright_codec_binary::{
    BinaryValueDecodeError, BinaryValueEncodeError, BinaryValueLimits, decode_value, encode_value,
};
use pwright_core::{
    CallId, CallMetadata, CancellationId, Command, CommandResult, CommandSpec, FailureKind,
    MethodTag, ObjectId, ObjectKindTag, OperationError,
};
use pwright_transport::{EncodedParts, FrameRoute, WireCodec, WireFrame};

const EVALUATE_HANDLE_COMMAND: u8 = 1;
const EVALUATE_HANDLE_RESULT: u8 = 2;
const HANDLE_EVALUATE_COMMAND: u8 = 3;
const HANDLE_EVALUATE_RESULT: u8 = 4;
const HANDLE_EVALUATE_HANDLE_COMMAND: u8 = 5;
const HANDLE_EVALUATE_HANDLE_RESULT: u8 = 6;
const HANDLE_JSON_VALUE_COMMAND: u8 = 7;
const HANDLE_JSON_VALUE_RESULT: u8 = 8;
const HANDLE_GET_PROPERTY_COMMAND: u8 = 9;
const HANDLE_GET_PROPERTY_RESULT: u8 = 10;
const HANDLE_GET_PROPERTIES_COMMAND: u8 = 11;
const HANDLE_GET_PROPERTIES_RESULT: u8 = 12;

#[doc(hidden)]
#[rustfmt::skip]
pub mod generated_schema;

pub use generated_schema::{
    CATALOG as HANDLE_EVALUATION_SCHEMA_CATALOG, COMMANDS as HANDLE_EVALUATION_COMMAND_SCHEMAS,
    ROOT_SCHEMA as HANDLE_EVALUATION_SCHEMA,
};

use generated_schema::{
    PwrightCommonCallMetadata, PwrightCommonFailureKind, PwrightHandleEvaluation,
    PwrightHandleEvaluationError, PwrightHandleEvaluationGetPropertiesOptions,
    PwrightHandleEvaluationGetPropertyParams, PwrightHandleEvaluationHandleEvaluateParams,
    PwrightHandleEvaluationJsonValueOptions, PwrightHandleEvaluationMode,
    PwrightHandleEvaluationOutcome, PwrightHandleEvaluationParams,
    PwrightHandleEvaluationPropertiesOutcome, PwrightHandleEvaluationPropertiesResult,
    PwrightHandleEvaluationProperty, PwrightHandleEvaluationPropertyInitializer,
    PwrightHandleEvaluationResult, PwrightHandleEvaluationTypedArrayKind,
    PwrightHandleEvaluationValue, PwrightHandleEvaluationValueOutcome,
    PwrightHandleEvaluationValueResult,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HandleEvaluationCodecLimits {
    source_bytes: usize,
    error_string_bytes: usize,
    value: EvaluationValueLimits,
    binary_value: BinaryValueLimits,
}

impl HandleEvaluationCodecLimits {
    pub fn new(
        source_bytes: usize,
        error_string_bytes: usize,
        value: EvaluationValueLimits,
    ) -> Result<Self, HandleEvaluationCodecLimitError> {
        let command_strings = source_bytes
            .checked_add(value.max_string_bytes())
            .ok_or(HandleEvaluationCodecLimitError::CapacityOverflow)?;
        let error_strings = error_string_bytes
            .checked_mul(4)
            .ok_or(HandleEvaluationCodecLimitError::CapacityOverflow)?;
        let string_bytes = command_strings.max(error_strings);
        let items = value.max_entries();
        let depth = value
            .max_depth()
            .checked_mul(2)
            .and_then(|count| count.checked_add(16))
            .ok_or(HandleEvaluationCodecLimitError::CapacityOverflow)?;
        let nodes = value
            .max_nodes()
            .checked_add(value.max_entries())
            .and_then(|count| count.checked_add(16))
            .ok_or(HandleEvaluationCodecLimitError::CapacityOverflow)?;
        Ok(Self {
            source_bytes,
            error_string_bytes,
            value,
            binary_value: BinaryValueLimits::new(
                string_bytes,
                value.max_binary_bytes(),
                items,
                depth,
                nodes,
            ),
        })
    }

    pub const fn source_bytes(self) -> usize {
        self.source_bytes
    }

    pub const fn error_string_bytes(self) -> usize {
        self.error_string_bytes
    }

    pub const fn value(self) -> EvaluationValueLimits {
        self.value
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandleEvaluationCodecLimitError {
    CapacityOverflow,
}

impl Display for HandleEvaluationCodecLimitError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("evaluation codec limit arithmetic overflowed")
    }
}

impl Error for HandleEvaluationCodecLimitError {}

#[derive(Clone, Copy, Debug)]
pub struct HandleEvaluationCodec {
    limits: HandleEvaluationCodecLimits,
    capability_version: pwright_core::ProtocolVersion,
}

impl HandleEvaluationCodec {
    pub const fn new(limits: HandleEvaluationCodecLimits) -> Self {
        Self {
            limits,
            capability_version: pwright_core::ProtocolVersion::new(1, 2),
        }
    }

    pub const fn limits(self) -> HandleEvaluationCodecLimits {
        self.limits
    }

    /// Selects the independently negotiated Handle-evaluation capability
    /// version. Version 1.0 remains value-only, Handle leaves require 1.1,
    /// and Handle-local commands require 1.2.
    pub const fn with_capability_version(
        mut self,
        capability_version: pwright_core::ProtocolVersion,
    ) -> Self {
        self.capability_version = capability_version;
        self
    }

    pub const fn capability_version(self) -> pwright_core::ProtocolVersion {
        self.capability_version
    }
}

#[derive(Debug)]
pub enum HandleEvaluationCodecError {
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
    TimeoutUnsupported,
    WrongPayloadKind {
        expected: u8,
        actual: u8,
    },
    SourceTooLong {
        actual: usize,
        limit: usize,
    },
    PropertyNameTooLong {
        actual: usize,
        limit: usize,
    },
    ErrorStringTooLong {
        actual: usize,
        limit: usize,
    },
    ExpressionArgumentUnsupported,
    HandleArgumentsUnsupported {
        version: pwright_core::ProtocolVersion,
    },
    HandleOperationsUnsupported {
        version: pwright_core::ProtocolVersion,
    },
    TooManyProperties {
        actual: usize,
        limit: usize,
    },
    InvalidReferenceId(u32),
    InvalidNumber(EvaluationNumberError),
    InvalidValue(HandleEvaluationValueError),
    Encode(BinaryValueEncodeError),
    Decode(BinaryValueDecodeError),
}

impl Display for HandleEvaluationCodecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongRoute(route) => {
                write!(formatter, "handle evaluation payload arrived on {route:?}")
            }
            Self::WrongMethod { expected, actual } => {
                write!(
                    formatter,
                    "expected handle evaluation method {expected}, received {actual}"
                )
            }
            Self::IdentityMismatch {
                field,
                outer,
                inner,
            } => write!(
                formatter,
                "handle evaluation payload {field} {inner} does not match outer route {outer}"
            ),
            Self::AttachmentsUnsupported { actual } => write!(
                formatter,
                "handle evaluation payload has {actual} attachment(s), but inline value codec accepts none"
            ),
            Self::TimeoutUnsupported => {
                formatter.write_str("handle evaluation does not accept timeout metadata")
            }
            Self::WrongPayloadKind { expected, actual } => write!(
                formatter,
                "expected handle evaluation payload kind {expected}, received {actual}"
            ),
            Self::SourceTooLong { actual, limit } => {
                write!(
                    formatter,
                    "evaluation source has {actual} bytes, exceeding limit {limit}"
                )
            }
            Self::PropertyNameTooLong { actual, limit } => write!(
                formatter,
                "Handle property name has {actual} bytes, exceeding limit {limit}"
            ),
            Self::ErrorStringTooLong { actual, limit } => write!(
                formatter,
                "evaluation error string has {actual} bytes, exceeding limit {limit}"
            ),
            Self::ExpressionArgumentUnsupported => {
                formatter.write_str("expression evaluation cannot carry an argument")
            }
            Self::HandleArgumentsUnsupported { version } => write!(
                formatter,
                "Handle arguments require pwright.handle_evaluation 1.1, negotiated {version}"
            ),
            Self::HandleOperationsUnsupported { version } => write!(
                formatter,
                "Handle-local operations require pwright.handle_evaluation 1.2, negotiated {version}"
            ),
            Self::TooManyProperties { actual, limit } => write!(
                formatter,
                "Handle property result has {actual} entries, exceeding limit {limit}"
            ),
            Self::InvalidReferenceId(id) => {
                write!(formatter, "evaluation reference ID {id} must be nonzero")
            }
            Self::InvalidNumber(error) => Display::fmt(error, formatter),
            Self::InvalidValue(error) => Display::fmt(error, formatter),
            Self::Encode(error) => Display::fmt(error, formatter),
            Self::Decode(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for HandleEvaluationCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidNumber(error) => Some(error),
            Self::InvalidValue(error) => Some(error),
            Self::Encode(error) => Some(error),
            Self::Decode(error) => Some(error),
            _ => None,
        }
    }
}

impl WireCodec<Command<EvaluateHandle>> for HandleEvaluationCodec {
    type Error = HandleEvaluationCodecError;

    fn encode_parts(&self, command: &Command<EvaluateHandle>) -> Result<EncodedParts, Self::Error> {
        validate_metadata(command.metadata())?;
        validate_params(command.params(), self.limits, self.capability_version)?;
        let wire = PwrightHandleEvaluation::EvaluateHandleCommand {
            call_id: command.call_id().get(),
            object_id: command.object_id().get(),
            metadata: metadata_to_wire(command.metadata()),
            params: params_to_wire(command.params()),
        };
        let payload = encode_value(&wire, self.limits.binary_value)
            .map_err(HandleEvaluationCodecError::Encode)?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: EvaluateHandle::METHOD,
            },
            payload,
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<EvaluateHandle>, Self::Error> {
        let (outer_call_id, outer_object_id) = command_route::<EvaluateHandle>(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightHandleEvaluation>(frame.payload(), self.limits.binary_value)
            .map_err(HandleEvaluationCodecError::Decode)?
        {
            PwrightHandleEvaluation::EvaluateHandleCommand {
                call_id,
                object_id,
                metadata,
                params,
            } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                ensure_identity("object ID", outer_object_id.get(), object_id)?;
                let metadata = metadata_from_wire(metadata);
                validate_metadata(metadata)?;
                let params = params_from_wire(params)?;
                validate_params(&params, self.limits, self.capability_version)?;
                Ok(Command::new(
                    CallId::new(call_id),
                    ObjectId::new(object_id),
                    metadata,
                    params,
                ))
            }
            other => Err(HandleEvaluationCodecError::WrongPayloadKind {
                expected: EVALUATE_HANDLE_COMMAND,
                actual: wire_kind(&other),
            }),
        }
    }
}

impl WireCodec<CommandResult<EvaluateHandle>> for HandleEvaluationCodec {
    type Error = HandleEvaluationCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<EvaluateHandle>,
    ) -> Result<EncodedParts, Self::Error> {
        let outcome = match result.outcome() {
            Ok(result) => PwrightHandleEvaluationOutcome::Success {
                result: initializer_to_wire(result),
            },
            Err(error) => {
                validate_operation_error(error, self.limits.error_string_bytes)?;
                PwrightHandleEvaluationOutcome::Error {
                    error: operation_error_to_wire(error),
                }
            }
        };
        let wire = PwrightHandleEvaluation::EvaluateHandleResult {
            call_id: result.call_id().get(),
            outcome,
        };
        let payload = encode_value(&wire, self.limits.binary_value)
            .map_err(HandleEvaluationCodecError::Encode)?;
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload,
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<EvaluateHandle>, Self::Error> {
        let outer_call_id = result_route(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightHandleEvaluation>(frame.payload(), self.limits.binary_value)
            .map_err(HandleEvaluationCodecError::Decode)?
        {
            PwrightHandleEvaluation::EvaluateHandleResult { call_id, outcome } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                let outcome = match outcome {
                    PwrightHandleEvaluationOutcome::Success { result } => {
                        Ok(initializer_from_wire(result))
                    }
                    PwrightHandleEvaluationOutcome::Error { error } => {
                        let error = operation_error_from_wire(error);
                        validate_operation_error(&error, self.limits.error_string_bytes)?;
                        Err(error)
                    }
                };
                Ok(CommandResult::new(CallId::new(call_id), outcome))
            }
            other => Err(HandleEvaluationCodecError::WrongPayloadKind {
                expected: EVALUATE_HANDLE_RESULT,
                actual: wire_kind(&other),
            }),
        }
    }
}

impl WireCodec<Command<HandleEvaluate>> for HandleEvaluationCodec {
    type Error = HandleEvaluationCodecError;

    fn encode_parts(&self, command: &Command<HandleEvaluate>) -> Result<EncodedParts, Self::Error> {
        require_handle_operations(self.capability_version)?;
        validate_metadata(command.metadata())?;
        validate_handle_params(command.params(), self.limits)?;
        encode_command_parts::<HandleEvaluate>(
            command.call_id(),
            command.object_id(),
            PwrightHandleEvaluation::HandleEvaluateCommand {
                call_id: command.call_id().get(),
                object_id: command.object_id().get(),
                metadata: metadata_to_wire(command.metadata()),
                params: handle_params_to_wire(command.params()),
            },
            self.limits.binary_value,
        )
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<HandleEvaluate>, Self::Error> {
        require_handle_operations(self.capability_version)?;
        let (outer_call_id, outer_object_id) = command_route::<HandleEvaluate>(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_wire(&frame, self.limits.binary_value)? {
            PwrightHandleEvaluation::HandleEvaluateCommand {
                call_id,
                object_id,
                metadata,
                params,
            } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                ensure_identity("object ID", outer_object_id.get(), object_id)?;
                let metadata = metadata_from_wire(metadata);
                validate_metadata(metadata)?;
                let params = handle_params_from_wire(params)?;
                validate_handle_params(&params, self.limits)?;
                Ok(Command::new(
                    CallId::new(call_id),
                    ObjectId::new(object_id),
                    metadata,
                    params,
                ))
            }
            other => Err(wrong_payload(HANDLE_EVALUATE_COMMAND, &other)),
        }
    }
}

impl WireCodec<CommandResult<HandleEvaluate>> for HandleEvaluationCodec {
    type Error = HandleEvaluationCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<HandleEvaluate>,
    ) -> Result<EncodedParts, Self::Error> {
        require_handle_operations(self.capability_version)?;
        let outcome = value_outcome_to_wire(result.outcome().as_ref(), self.limits)?;
        encode_result_parts(
            result.call_id(),
            PwrightHandleEvaluation::HandleEvaluateResult {
                call_id: result.call_id().get(),
                outcome,
            },
            self.limits.binary_value,
        )
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<HandleEvaluate>, Self::Error> {
        require_handle_operations(self.capability_version)?;
        let outer_call_id = result_route(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_wire(&frame, self.limits.binary_value)? {
            PwrightHandleEvaluation::HandleEvaluateResult { call_id, outcome } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                Ok(CommandResult::new(
                    CallId::new(call_id),
                    value_outcome_from_wire(outcome, self.limits)?,
                ))
            }
            other => Err(wrong_payload(HANDLE_EVALUATE_RESULT, &other)),
        }
    }
}

impl WireCodec<Command<HandleEvaluateHandle>> for HandleEvaluationCodec {
    type Error = HandleEvaluationCodecError;

    fn encode_parts(
        &self,
        command: &Command<HandleEvaluateHandle>,
    ) -> Result<EncodedParts, Self::Error> {
        require_handle_operations(self.capability_version)?;
        validate_metadata(command.metadata())?;
        validate_handle_params(command.params(), self.limits)?;
        encode_command_parts::<HandleEvaluateHandle>(
            command.call_id(),
            command.object_id(),
            PwrightHandleEvaluation::HandleEvaluateHandleCommand {
                call_id: command.call_id().get(),
                object_id: command.object_id().get(),
                metadata: metadata_to_wire(command.metadata()),
                params: handle_params_to_wire(command.params()),
            },
            self.limits.binary_value,
        )
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<HandleEvaluateHandle>, Self::Error> {
        require_handle_operations(self.capability_version)?;
        let (outer_call_id, outer_object_id) = command_route::<HandleEvaluateHandle>(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_wire(&frame, self.limits.binary_value)? {
            PwrightHandleEvaluation::HandleEvaluateHandleCommand {
                call_id,
                object_id,
                metadata,
                params,
            } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                ensure_identity("object ID", outer_object_id.get(), object_id)?;
                let metadata = metadata_from_wire(metadata);
                validate_metadata(metadata)?;
                let params = handle_params_from_wire(params)?;
                validate_handle_params(&params, self.limits)?;
                Ok(Command::new(
                    CallId::new(call_id),
                    ObjectId::new(object_id),
                    metadata,
                    params,
                ))
            }
            other => Err(wrong_payload(HANDLE_EVALUATE_HANDLE_COMMAND, &other)),
        }
    }
}

impl WireCodec<CommandResult<HandleEvaluateHandle>> for HandleEvaluationCodec {
    type Error = HandleEvaluationCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<HandleEvaluateHandle>,
    ) -> Result<EncodedParts, Self::Error> {
        require_handle_operations(self.capability_version)?;
        let outcome = initializer_outcome_to_wire(result.outcome().as_ref(), self.limits)?;
        encode_result_parts(
            result.call_id(),
            PwrightHandleEvaluation::HandleEvaluateHandleResult {
                call_id: result.call_id().get(),
                outcome,
            },
            self.limits.binary_value,
        )
    }

    fn decode_frame(
        &self,
        frame: WireFrame,
    ) -> Result<CommandResult<HandleEvaluateHandle>, Self::Error> {
        require_handle_operations(self.capability_version)?;
        let outer_call_id = result_route(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_wire(&frame, self.limits.binary_value)? {
            PwrightHandleEvaluation::HandleEvaluateHandleResult { call_id, outcome } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                Ok(CommandResult::new(
                    CallId::new(call_id),
                    initializer_outcome_from_wire(outcome, self.limits)?,
                ))
            }
            other => Err(wrong_payload(HANDLE_EVALUATE_HANDLE_RESULT, &other)),
        }
    }
}

impl WireCodec<Command<HandleJsonValue>> for HandleEvaluationCodec {
    type Error = HandleEvaluationCodecError;

    fn encode_parts(
        &self,
        command: &Command<HandleJsonValue>,
    ) -> Result<EncodedParts, Self::Error> {
        require_handle_operations(self.capability_version)?;
        validate_metadata(command.metadata())?;
        encode_command_parts::<HandleJsonValue>(
            command.call_id(),
            command.object_id(),
            PwrightHandleEvaluation::HandleJsonValueCommand {
                call_id: command.call_id().get(),
                object_id: command.object_id().get(),
                metadata: metadata_to_wire(command.metadata()),
                options: PwrightHandleEvaluationJsonValueOptions {},
            },
            self.limits.binary_value,
        )
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<HandleJsonValue>, Self::Error> {
        require_handle_operations(self.capability_version)?;
        let (outer_call_id, outer_object_id) = command_route::<HandleJsonValue>(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_wire(&frame, self.limits.binary_value)? {
            PwrightHandleEvaluation::HandleJsonValueCommand {
                call_id,
                object_id,
                metadata,
                options: _,
            } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                ensure_identity("object ID", outer_object_id.get(), object_id)?;
                let metadata = metadata_from_wire(metadata);
                validate_metadata(metadata)?;
                Ok(Command::new(
                    CallId::new(call_id),
                    ObjectId::new(object_id),
                    metadata,
                    JsonValueOptions,
                ))
            }
            other => Err(wrong_payload(HANDLE_JSON_VALUE_COMMAND, &other)),
        }
    }
}

impl WireCodec<CommandResult<HandleJsonValue>> for HandleEvaluationCodec {
    type Error = HandleEvaluationCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<HandleJsonValue>,
    ) -> Result<EncodedParts, Self::Error> {
        require_handle_operations(self.capability_version)?;
        let outcome = value_outcome_to_wire(result.outcome().as_ref(), self.limits)?;
        encode_result_parts(
            result.call_id(),
            PwrightHandleEvaluation::HandleJsonValueResult {
                call_id: result.call_id().get(),
                outcome,
            },
            self.limits.binary_value,
        )
    }

    fn decode_frame(
        &self,
        frame: WireFrame,
    ) -> Result<CommandResult<HandleJsonValue>, Self::Error> {
        require_handle_operations(self.capability_version)?;
        let outer_call_id = result_route(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_wire(&frame, self.limits.binary_value)? {
            PwrightHandleEvaluation::HandleJsonValueResult { call_id, outcome } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                Ok(CommandResult::new(
                    CallId::new(call_id),
                    value_outcome_from_wire(outcome, self.limits)?,
                ))
            }
            other => Err(wrong_payload(HANDLE_JSON_VALUE_RESULT, &other)),
        }
    }
}

impl WireCodec<Command<HandleGetProperty>> for HandleEvaluationCodec {
    type Error = HandleEvaluationCodecError;

    fn encode_parts(
        &self,
        command: &Command<HandleGetProperty>,
    ) -> Result<EncodedParts, Self::Error> {
        require_handle_operations(self.capability_version)?;
        validate_metadata(command.metadata())?;
        validate_property_name(command.params().name(), self.limits)?;
        encode_command_parts::<HandleGetProperty>(
            command.call_id(),
            command.object_id(),
            PwrightHandleEvaluation::HandleGetPropertyCommand {
                call_id: command.call_id().get(),
                object_id: command.object_id().get(),
                metadata: metadata_to_wire(command.metadata()),
                params: PwrightHandleEvaluationGetPropertyParams {
                    name: command.params().name().to_owned(),
                },
            },
            self.limits.binary_value,
        )
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<HandleGetProperty>, Self::Error> {
        require_handle_operations(self.capability_version)?;
        let (outer_call_id, outer_object_id) = command_route::<HandleGetProperty>(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_wire(&frame, self.limits.binary_value)? {
            PwrightHandleEvaluation::HandleGetPropertyCommand {
                call_id,
                object_id,
                metadata,
                params,
            } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                ensure_identity("object ID", outer_object_id.get(), object_id)?;
                let metadata = metadata_from_wire(metadata);
                validate_metadata(metadata)?;
                validate_property_name(&params.name, self.limits)?;
                Ok(Command::new(
                    CallId::new(call_id),
                    ObjectId::new(object_id),
                    metadata,
                    GetPropertyParams::new(params.name),
                ))
            }
            other => Err(wrong_payload(HANDLE_GET_PROPERTY_COMMAND, &other)),
        }
    }
}

impl WireCodec<CommandResult<HandleGetProperty>> for HandleEvaluationCodec {
    type Error = HandleEvaluationCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<HandleGetProperty>,
    ) -> Result<EncodedParts, Self::Error> {
        require_handle_operations(self.capability_version)?;
        let outcome = initializer_outcome_to_wire(result.outcome().as_ref(), self.limits)?;
        encode_result_parts(
            result.call_id(),
            PwrightHandleEvaluation::HandleGetPropertyResult {
                call_id: result.call_id().get(),
                outcome,
            },
            self.limits.binary_value,
        )
    }

    fn decode_frame(
        &self,
        frame: WireFrame,
    ) -> Result<CommandResult<HandleGetProperty>, Self::Error> {
        require_handle_operations(self.capability_version)?;
        let outer_call_id = result_route(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_wire(&frame, self.limits.binary_value)? {
            PwrightHandleEvaluation::HandleGetPropertyResult { call_id, outcome } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                Ok(CommandResult::new(
                    CallId::new(call_id),
                    initializer_outcome_from_wire(outcome, self.limits)?,
                ))
            }
            other => Err(wrong_payload(HANDLE_GET_PROPERTY_RESULT, &other)),
        }
    }
}

impl WireCodec<Command<HandleGetProperties>> for HandleEvaluationCodec {
    type Error = HandleEvaluationCodecError;

    fn encode_parts(
        &self,
        command: &Command<HandleGetProperties>,
    ) -> Result<EncodedParts, Self::Error> {
        require_handle_operations(self.capability_version)?;
        validate_metadata(command.metadata())?;
        encode_command_parts::<HandleGetProperties>(
            command.call_id(),
            command.object_id(),
            PwrightHandleEvaluation::HandleGetPropertiesCommand {
                call_id: command.call_id().get(),
                object_id: command.object_id().get(),
                metadata: metadata_to_wire(command.metadata()),
                options: PwrightHandleEvaluationGetPropertiesOptions {},
            },
            self.limits.binary_value,
        )
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<HandleGetProperties>, Self::Error> {
        require_handle_operations(self.capability_version)?;
        let (outer_call_id, outer_object_id) = command_route::<HandleGetProperties>(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_wire(&frame, self.limits.binary_value)? {
            PwrightHandleEvaluation::HandleGetPropertiesCommand {
                call_id,
                object_id,
                metadata,
                options: _,
            } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                ensure_identity("object ID", outer_object_id.get(), object_id)?;
                let metadata = metadata_from_wire(metadata);
                validate_metadata(metadata)?;
                Ok(Command::new(
                    CallId::new(call_id),
                    ObjectId::new(object_id),
                    metadata,
                    GetPropertiesOptions,
                ))
            }
            other => Err(wrong_payload(HANDLE_GET_PROPERTIES_COMMAND, &other)),
        }
    }
}

impl WireCodec<CommandResult<HandleGetProperties>> for HandleEvaluationCodec {
    type Error = HandleEvaluationCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<HandleGetProperties>,
    ) -> Result<EncodedParts, Self::Error> {
        require_handle_operations(self.capability_version)?;
        let outcome = properties_outcome_to_wire(result.outcome().as_ref(), self.limits)?;
        encode_result_parts(
            result.call_id(),
            PwrightHandleEvaluation::HandleGetPropertiesResult {
                call_id: result.call_id().get(),
                outcome,
            },
            self.limits.binary_value,
        )
    }

    fn decode_frame(
        &self,
        frame: WireFrame,
    ) -> Result<CommandResult<HandleGetProperties>, Self::Error> {
        require_handle_operations(self.capability_version)?;
        let outer_call_id = result_route(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_wire(&frame, self.limits.binary_value)? {
            PwrightHandleEvaluation::HandleGetPropertiesResult { call_id, outcome } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                Ok(CommandResult::new(
                    CallId::new(call_id),
                    properties_outcome_from_wire(outcome, self.limits)?,
                ))
            }
            other => Err(wrong_payload(HANDLE_GET_PROPERTIES_RESULT, &other)),
        }
    }
}

fn encode_command_parts<S: CommandSpec>(
    call_id: CallId,
    object_id: ObjectId,
    wire: PwrightHandleEvaluation,
    limits: BinaryValueLimits,
) -> Result<EncodedParts, HandleEvaluationCodecError> {
    let payload = encode_value(&wire, limits).map_err(HandleEvaluationCodecError::Encode)?;
    Ok(EncodedParts::new(
        FrameRoute::Command {
            call_id,
            target: object_id,
            method: S::METHOD,
        },
        payload,
        Vec::new(),
    ))
}

fn encode_result_parts(
    call_id: CallId,
    wire: PwrightHandleEvaluation,
    limits: BinaryValueLimits,
) -> Result<EncodedParts, HandleEvaluationCodecError> {
    let payload = encode_value(&wire, limits).map_err(HandleEvaluationCodecError::Encode)?;
    Ok(EncodedParts::new(
        FrameRoute::Result { call_id },
        payload,
        Vec::new(),
    ))
}

fn decode_wire(
    frame: &WireFrame,
    limits: BinaryValueLimits,
) -> Result<PwrightHandleEvaluation, HandleEvaluationCodecError> {
    decode_value(frame.payload(), limits).map_err(HandleEvaluationCodecError::Decode)
}

fn wrong_payload(expected: u8, actual: &PwrightHandleEvaluation) -> HandleEvaluationCodecError {
    HandleEvaluationCodecError::WrongPayloadKind {
        expected,
        actual: wire_kind(actual),
    }
}

fn require_handle_operations(
    version: pwright_core::ProtocolVersion,
) -> Result<(), HandleEvaluationCodecError> {
    if version.satisfies(pwright_core::ProtocolVersion::new(1, 2)) {
        Ok(())
    } else {
        Err(HandleEvaluationCodecError::HandleOperationsUnsupported { version })
    }
}

fn validate_handle_params(
    params: &HandleEvaluateParams,
    limits: HandleEvaluationCodecLimits,
) -> Result<(), HandleEvaluationCodecError> {
    if params.source().len() > limits.source_bytes {
        return Err(HandleEvaluationCodecError::SourceTooLong {
            actual: params.source().len(),
            limit: limits.source_bytes,
        });
    }
    if let Some(argument) = params.argument() {
        argument
            .validate(limits.value)
            .map_err(HandleEvaluationCodecError::InvalidValue)?;
    }
    Ok(())
}

fn validate_property_name(
    name: &str,
    limits: HandleEvaluationCodecLimits,
) -> Result<(), HandleEvaluationCodecError> {
    if name.len() <= limits.value.max_string_bytes() {
        Ok(())
    } else {
        Err(HandleEvaluationCodecError::PropertyNameTooLong {
            actual: name.len(),
            limit: limits.value.max_string_bytes(),
        })
    }
}

fn handle_params_to_wire(
    params: &HandleEvaluateParams,
) -> PwrightHandleEvaluationHandleEvaluateParams {
    PwrightHandleEvaluationHandleEvaluateParams {
        source: params.source().to_owned(),
        argument: params.argument().map(value_to_wire),
    }
}

fn handle_params_from_wire(
    params: PwrightHandleEvaluationHandleEvaluateParams,
) -> Result<HandleEvaluateParams, HandleEvaluationCodecError> {
    match params.argument.map(value_from_wire).transpose()? {
        Some(argument) => Ok(HandleEvaluateParams::function(params.source, argument)),
        None => Ok(HandleEvaluateParams::function_without_argument(
            params.source,
        )),
    }
}

fn initializer_outcome_to_wire(
    outcome: Result<&HandleInitializer, &OperationError<HandleEvaluationErrorDetails>>,
    limits: HandleEvaluationCodecLimits,
) -> Result<PwrightHandleEvaluationOutcome, HandleEvaluationCodecError> {
    match outcome {
        Ok(result) => Ok(PwrightHandleEvaluationOutcome::Success {
            result: initializer_to_wire(result),
        }),
        Err(error) => {
            validate_operation_error(error, limits.error_string_bytes)?;
            Ok(PwrightHandleEvaluationOutcome::Error {
                error: operation_error_to_wire(error),
            })
        }
    }
}

fn initializer_outcome_from_wire(
    outcome: PwrightHandleEvaluationOutcome,
    limits: HandleEvaluationCodecLimits,
) -> Result<
    Result<HandleInitializer, OperationError<HandleEvaluationErrorDetails>>,
    HandleEvaluationCodecError,
> {
    match outcome {
        PwrightHandleEvaluationOutcome::Success { result } => Ok(Ok(initializer_from_wire(result))),
        PwrightHandleEvaluationOutcome::Error { error } => {
            let error = operation_error_from_wire(error);
            validate_operation_error(&error, limits.error_string_bytes)?;
            Ok(Err(error))
        }
    }
}

fn value_outcome_to_wire(
    outcome: Result<&HandleValueResult, &OperationError<HandleEvaluationErrorDetails>>,
    limits: HandleEvaluationCodecLimits,
) -> Result<PwrightHandleEvaluationValueOutcome, HandleEvaluationCodecError> {
    match outcome {
        Ok(result) => {
            result.value.validate(limits.value).map_err(|error| {
                HandleEvaluationCodecError::InvalidValue(HandleEvaluationValueError::Value(error))
            })?;
            Ok(PwrightHandleEvaluationValueOutcome::Success {
                result: PwrightHandleEvaluationValueResult {
                    value: value_to_wire(&HandleEvaluationValue::from(result.value.clone())),
                },
            })
        }
        Err(error) => {
            validate_operation_error(error, limits.error_string_bytes)?;
            Ok(PwrightHandleEvaluationValueOutcome::Error {
                error: operation_error_to_wire(error),
            })
        }
    }
}

fn value_outcome_from_wire(
    outcome: PwrightHandleEvaluationValueOutcome,
    limits: HandleEvaluationCodecLimits,
) -> Result<
    Result<HandleValueResult, OperationError<HandleEvaluationErrorDetails>>,
    HandleEvaluationCodecError,
> {
    match outcome {
        PwrightHandleEvaluationValueOutcome::Success { result } => {
            let value = EvaluationValue::try_from(value_from_wire(result.value)?)
                .map_err(HandleEvaluationCodecError::InvalidValue)?;
            value.validate(limits.value).map_err(|error| {
                HandleEvaluationCodecError::InvalidValue(HandleEvaluationValueError::Value(error))
            })?;
            Ok(Ok(HandleValueResult::new(value)))
        }
        PwrightHandleEvaluationValueOutcome::Error { error } => {
            let error = operation_error_from_wire(error);
            validate_operation_error(&error, limits.error_string_bytes)?;
            Ok(Err(error))
        }
    }
}

fn properties_outcome_to_wire(
    outcome: Result<&HandlePropertiesResult, &OperationError<HandleEvaluationErrorDetails>>,
    limits: HandleEvaluationCodecLimits,
) -> Result<PwrightHandleEvaluationPropertiesOutcome, HandleEvaluationCodecError> {
    match outcome {
        Ok(result) => {
            validate_property_count(result.properties.len(), limits)?;
            for property in &result.properties {
                validate_property_name(&property.name, limits)?;
            }
            Ok(PwrightHandleEvaluationPropertiesOutcome::Success {
                result: PwrightHandleEvaluationPropertiesResult {
                    properties: result
                        .properties
                        .iter()
                        .map(|property| PwrightHandleEvaluationPropertyInitializer {
                            name: property.name.clone(),
                            handle: initializer_to_wire(&property.handle),
                        })
                        .collect(),
                },
            })
        }
        Err(error) => {
            validate_operation_error(error, limits.error_string_bytes)?;
            Ok(PwrightHandleEvaluationPropertiesOutcome::Error {
                error: operation_error_to_wire(error),
            })
        }
    }
}

fn properties_outcome_from_wire(
    outcome: PwrightHandleEvaluationPropertiesOutcome,
    limits: HandleEvaluationCodecLimits,
) -> Result<
    Result<HandlePropertiesResult, OperationError<HandleEvaluationErrorDetails>>,
    HandleEvaluationCodecError,
> {
    match outcome {
        PwrightHandleEvaluationPropertiesOutcome::Success { result } => {
            validate_property_count(result.properties.len(), limits)?;
            let mut names = std::collections::BTreeSet::new();
            let properties = result
                .properties
                .into_iter()
                .map(|property| {
                    validate_property_name(&property.name, limits)?;
                    if !names.insert(property.name.clone()) {
                        return Err(HandleEvaluationCodecError::InvalidValue(
                            HandleEvaluationValueError::Value(
                                pwright_cap_evaluation::EvaluationValueError::DuplicateObjectKey(
                                    property.name,
                                ),
                            ),
                        ));
                    }
                    Ok(HandlePropertyInitializer::new(
                        property.name,
                        initializer_from_wire(property.handle),
                    ))
                })
                .collect::<Result<_, HandleEvaluationCodecError>>()?;
            Ok(Ok(HandlePropertiesResult::new(properties)))
        }
        PwrightHandleEvaluationPropertiesOutcome::Error { error } => {
            let error = operation_error_from_wire(error);
            validate_operation_error(&error, limits.error_string_bytes)?;
            Ok(Err(error))
        }
    }
}

fn validate_property_count(
    actual: usize,
    limits: HandleEvaluationCodecLimits,
) -> Result<(), HandleEvaluationCodecError> {
    let limit = limits.value.max_entries();
    if actual <= limit {
        Ok(())
    } else {
        Err(HandleEvaluationCodecError::TooManyProperties { actual, limit })
    }
}

fn initializer_to_wire(initializer: &HandleInitializer) -> PwrightHandleEvaluationResult {
    PwrightHandleEvaluationResult {
        object_id: initializer.object_id.get(),
        parent_id: initializer.parent_id.get(),
        object_kind: initializer.object_kind.get(),
        preview: initializer.preview.clone(),
    }
}

fn initializer_from_wire(initializer: PwrightHandleEvaluationResult) -> HandleInitializer {
    HandleInitializer::new(
        ObjectId::new(initializer.object_id),
        ObjectId::new(initializer.parent_id),
        ObjectKindTag::new(initializer.object_kind),
        initializer.preview,
    )
}

fn validate_params(
    params: &EvaluateHandleParams,
    limits: HandleEvaluationCodecLimits,
    capability_version: pwright_core::ProtocolVersion,
) -> Result<(), HandleEvaluationCodecError> {
    if params.source().len() > limits.source_bytes {
        return Err(HandleEvaluationCodecError::SourceTooLong {
            actual: params.source().len(),
            limit: limits.source_bytes,
        });
    }
    if params.mode() == EvaluationMode::Expression && params.argument().is_some() {
        return Err(HandleEvaluationCodecError::ExpressionArgumentUnsupported);
    }
    if let Some(argument) = params.argument() {
        argument
            .validate(limits.value)
            .map_err(HandleEvaluationCodecError::InvalidValue)?;
        if !capability_version.satisfies(pwright_core::ProtocolVersion::new(1, 1))
            && !params
                .referenced_handles(limits.value)
                .map_err(HandleEvaluationCodecError::InvalidValue)?
                .is_empty()
        {
            return Err(HandleEvaluationCodecError::HandleArgumentsUnsupported {
                version: capability_version,
            });
        }
    }
    Ok(())
}

fn params_to_wire(params: &EvaluateHandleParams) -> PwrightHandleEvaluationParams {
    PwrightHandleEvaluationParams {
        source: params.source().to_owned(),
        mode: mode_to_wire(params.mode()),
        argument: params.argument().map(value_to_wire),
    }
}

fn params_from_wire(
    params: PwrightHandleEvaluationParams,
) -> Result<EvaluateHandleParams, HandleEvaluationCodecError> {
    let argument = params.argument.map(value_from_wire).transpose()?;
    match (mode_from_wire(params.mode), argument) {
        (EvaluationMode::Expression, None) => Ok(EvaluateHandleParams::expression(params.source)),
        (EvaluationMode::Expression, Some(_)) => {
            Err(HandleEvaluationCodecError::ExpressionArgumentUnsupported)
        }
        (EvaluationMode::Function, Some(argument)) => {
            Ok(EvaluateHandleParams::function(params.source, argument))
        }
        (EvaluationMode::Function, None) => Ok(EvaluateHandleParams::function_without_argument(
            params.source,
        )),
    }
}

fn mode_to_wire(mode: EvaluationMode) -> PwrightHandleEvaluationMode {
    match mode {
        EvaluationMode::Expression => PwrightHandleEvaluationMode::Expression,
        EvaluationMode::Function => PwrightHandleEvaluationMode::Function,
    }
}

fn mode_from_wire(mode: PwrightHandleEvaluationMode) -> EvaluationMode {
    match mode {
        PwrightHandleEvaluationMode::Expression => EvaluationMode::Expression,
        PwrightHandleEvaluationMode::Function => EvaluationMode::Function,
    }
}

fn typed_array_to_wire(kind: TypedArrayKind) -> PwrightHandleEvaluationTypedArrayKind {
    match kind {
        TypedArrayKind::Int8 => PwrightHandleEvaluationTypedArrayKind::Int8,
        TypedArrayKind::Uint8 => PwrightHandleEvaluationTypedArrayKind::Uint8,
        TypedArrayKind::Uint8Clamped => PwrightHandleEvaluationTypedArrayKind::Uint8Clamped,
        TypedArrayKind::Int16 => PwrightHandleEvaluationTypedArrayKind::Int16,
        TypedArrayKind::Uint16 => PwrightHandleEvaluationTypedArrayKind::Uint16,
        TypedArrayKind::Int32 => PwrightHandleEvaluationTypedArrayKind::Int32,
        TypedArrayKind::Uint32 => PwrightHandleEvaluationTypedArrayKind::Uint32,
        TypedArrayKind::Float32 => PwrightHandleEvaluationTypedArrayKind::Float32,
        TypedArrayKind::Float64 => PwrightHandleEvaluationTypedArrayKind::Float64,
        TypedArrayKind::Bigint64 => PwrightHandleEvaluationTypedArrayKind::Bigint64,
        TypedArrayKind::Biguint64 => PwrightHandleEvaluationTypedArrayKind::Biguint64,
    }
}

fn typed_array_from_wire(kind: PwrightHandleEvaluationTypedArrayKind) -> TypedArrayKind {
    match kind {
        PwrightHandleEvaluationTypedArrayKind::Int8 => TypedArrayKind::Int8,
        PwrightHandleEvaluationTypedArrayKind::Uint8 => TypedArrayKind::Uint8,
        PwrightHandleEvaluationTypedArrayKind::Uint8Clamped => TypedArrayKind::Uint8Clamped,
        PwrightHandleEvaluationTypedArrayKind::Int16 => TypedArrayKind::Int16,
        PwrightHandleEvaluationTypedArrayKind::Uint16 => TypedArrayKind::Uint16,
        PwrightHandleEvaluationTypedArrayKind::Int32 => TypedArrayKind::Int32,
        PwrightHandleEvaluationTypedArrayKind::Uint32 => TypedArrayKind::Uint32,
        PwrightHandleEvaluationTypedArrayKind::Float32 => TypedArrayKind::Float32,
        PwrightHandleEvaluationTypedArrayKind::Float64 => TypedArrayKind::Float64,
        PwrightHandleEvaluationTypedArrayKind::Bigint64 => TypedArrayKind::Bigint64,
        PwrightHandleEvaluationTypedArrayKind::Biguint64 => TypedArrayKind::Biguint64,
    }
}

fn value_to_wire(value: &HandleEvaluationValue) -> PwrightHandleEvaluationValue {
    match value {
        HandleEvaluationValue::Undefined => PwrightHandleEvaluationValue::Undefined,
        HandleEvaluationValue::Null => PwrightHandleEvaluationValue::Null,
        HandleEvaluationValue::Bool(value) => PwrightHandleEvaluationValue::Bool { value: *value },
        HandleEvaluationValue::Number(value) => {
            PwrightHandleEvaluationValue::Number { bits: value.bits() }
        }
        HandleEvaluationValue::Nan => PwrightHandleEvaluationValue::Nan,
        HandleEvaluationValue::Infinity => PwrightHandleEvaluationValue::Infinity,
        HandleEvaluationValue::NegativeInfinity => PwrightHandleEvaluationValue::NegativeInfinity,
        HandleEvaluationValue::NegativeZero => PwrightHandleEvaluationValue::NegativeZero,
        HandleEvaluationValue::String(value) => PwrightHandleEvaluationValue::String {
            value: value.clone(),
        },
        HandleEvaluationValue::Bigint(value) => PwrightHandleEvaluationValue::Bigint {
            value: value.clone(),
        },
        HandleEvaluationValue::Date(value) => PwrightHandleEvaluationValue::Date {
            value: value.clone(),
        },
        HandleEvaluationValue::Url(value) => PwrightHandleEvaluationValue::Url {
            value: value.clone(),
        },
        HandleEvaluationValue::Regexp { pattern, flags } => PwrightHandleEvaluationValue::Regexp {
            pattern: pattern.clone(),
            flags: flags.clone(),
        },
        HandleEvaluationValue::Error {
            name,
            message,
            stack,
        } => PwrightHandleEvaluationValue::Error {
            name: name.clone(),
            message: message.clone(),
            stack: stack.clone(),
        },
        HandleEvaluationValue::TypedArray { kind, bytes } => {
            PwrightHandleEvaluationValue::TypedArray {
                kind: typed_array_to_wire(*kind),
                bytes: bytes.clone(),
            }
        }
        HandleEvaluationValue::Array { id, items } => PwrightHandleEvaluationValue::Array {
            id: id.map(NonZeroU32::get),
            items: items.iter().map(value_to_wire).collect(),
        },
        HandleEvaluationValue::Object { id, properties } => PwrightHandleEvaluationValue::Object {
            id: id.map(NonZeroU32::get),
            properties: properties
                .iter()
                .map(|property| PwrightHandleEvaluationProperty {
                    key: property.key.clone(),
                    value: value_to_wire(&property.value),
                })
                .collect(),
        },
        HandleEvaluationValue::Ref(id) => PwrightHandleEvaluationValue::Ref { id: id.get() },
        HandleEvaluationValue::Handle(object_id) => PwrightHandleEvaluationValue::Handle {
            object_id: object_id.get(),
        },
    }
}

fn value_from_wire(
    value: PwrightHandleEvaluationValue,
) -> Result<HandleEvaluationValue, HandleEvaluationCodecError> {
    Ok(match value {
        PwrightHandleEvaluationValue::Undefined => HandleEvaluationValue::Undefined,
        PwrightHandleEvaluationValue::Null => HandleEvaluationValue::Null,
        PwrightHandleEvaluationValue::Bool { value } => HandleEvaluationValue::Bool(value),
        PwrightHandleEvaluationValue::Number { bits } => HandleEvaluationValue::Number(
            EvaluationNumber::from_bits(bits).map_err(HandleEvaluationCodecError::InvalidNumber)?,
        ),
        PwrightHandleEvaluationValue::Nan => HandleEvaluationValue::Nan,
        PwrightHandleEvaluationValue::Infinity => HandleEvaluationValue::Infinity,
        PwrightHandleEvaluationValue::NegativeInfinity => HandleEvaluationValue::NegativeInfinity,
        PwrightHandleEvaluationValue::NegativeZero => HandleEvaluationValue::NegativeZero,
        PwrightHandleEvaluationValue::String { value } => HandleEvaluationValue::String(value),
        PwrightHandleEvaluationValue::Bigint { value } => HandleEvaluationValue::Bigint(value),
        PwrightHandleEvaluationValue::Date { value } => HandleEvaluationValue::Date(value),
        PwrightHandleEvaluationValue::Url { value } => HandleEvaluationValue::Url(value),
        PwrightHandleEvaluationValue::Regexp { pattern, flags } => {
            HandleEvaluationValue::Regexp { pattern, flags }
        }
        PwrightHandleEvaluationValue::Error {
            name,
            message,
            stack,
        } => HandleEvaluationValue::Error {
            name,
            message,
            stack,
        },
        PwrightHandleEvaluationValue::TypedArray { kind, bytes } => {
            HandleEvaluationValue::TypedArray {
                kind: typed_array_from_wire(kind),
                bytes,
            }
        }
        PwrightHandleEvaluationValue::Array { id, items } => HandleEvaluationValue::Array {
            id: nonzero(id)?,
            items: items
                .into_iter()
                .map(value_from_wire)
                .collect::<Result<_, _>>()?,
        },
        PwrightHandleEvaluationValue::Object { id, properties } => HandleEvaluationValue::Object {
            id: nonzero(id)?,
            properties: properties
                .into_iter()
                .map(|property| {
                    Ok(HandleEvaluationProperty::new(
                        property.key,
                        value_from_wire(property.value)?,
                    ))
                })
                .collect::<Result<_, HandleEvaluationCodecError>>()?,
        },
        PwrightHandleEvaluationValue::Ref { id } => HandleEvaluationValue::Ref(
            NonZeroU32::new(id).ok_or(HandleEvaluationCodecError::InvalidReferenceId(id))?,
        ),
        PwrightHandleEvaluationValue::Handle { object_id } => {
            HandleEvaluationValue::Handle(ObjectId::new(object_id))
        }
    })
}

fn nonzero(id: Option<u32>) -> Result<Option<NonZeroU32>, HandleEvaluationCodecError> {
    id.map(|id| NonZeroU32::new(id).ok_or(HandleEvaluationCodecError::InvalidReferenceId(id)))
        .transpose()
}

fn metadata_to_wire(metadata: CallMetadata) -> PwrightCommonCallMetadata {
    PwrightCommonCallMetadata {
        timeout: None,
        cancellation_id: metadata.cancellation.map(CancellationId::get),
    }
}

fn metadata_from_wire(metadata: PwrightCommonCallMetadata) -> CallMetadata {
    let timeout = metadata
        .timeout
        .map(|timeout| std::time::Duration::new(timeout.seconds, timeout.nanoseconds));
    CallMetadata::new(timeout, metadata.cancellation_id.map(CancellationId::new))
}

fn validate_metadata(metadata: CallMetadata) -> Result<(), HandleEvaluationCodecError> {
    if metadata.timeout.is_none() {
        Ok(())
    } else {
        Err(HandleEvaluationCodecError::TimeoutUnsupported)
    }
}

fn ensure_no_attachments(frame: &WireFrame) -> Result<(), HandleEvaluationCodecError> {
    if frame.attachments().is_empty() {
        Ok(())
    } else {
        Err(HandleEvaluationCodecError::AttachmentsUnsupported {
            actual: frame.attachments().len(),
        })
    }
}

fn ensure_identity(
    field: &'static str,
    outer: u64,
    inner: u64,
) -> Result<(), HandleEvaluationCodecError> {
    if outer == inner {
        Ok(())
    } else {
        Err(HandleEvaluationCodecError::IdentityMismatch {
            field,
            outer,
            inner,
        })
    }
}

fn command_route<S: CommandSpec>(
    frame: &WireFrame,
) -> Result<(CallId, ObjectId), HandleEvaluationCodecError> {
    let FrameRoute::Command {
        call_id,
        target,
        method,
    } = frame.route()
    else {
        return Err(HandleEvaluationCodecError::WrongRoute(frame.route()));
    };
    if method != S::METHOD {
        return Err(HandleEvaluationCodecError::WrongMethod {
            expected: S::METHOD,
            actual: method,
        });
    }
    Ok((call_id, target))
}

fn result_route(frame: &WireFrame) -> Result<CallId, HandleEvaluationCodecError> {
    let FrameRoute::Result { call_id } = frame.route() else {
        return Err(HandleEvaluationCodecError::WrongRoute(frame.route()));
    };
    Ok(call_id)
}

fn wire_kind(value: &PwrightHandleEvaluation) -> u8 {
    match value {
        PwrightHandleEvaluation::EvaluateHandleCommand { .. } => EVALUATE_HANDLE_COMMAND,
        PwrightHandleEvaluation::EvaluateHandleResult { .. } => EVALUATE_HANDLE_RESULT,
        PwrightHandleEvaluation::HandleEvaluateCommand { .. } => HANDLE_EVALUATE_COMMAND,
        PwrightHandleEvaluation::HandleEvaluateResult { .. } => HANDLE_EVALUATE_RESULT,
        PwrightHandleEvaluation::HandleEvaluateHandleCommand { .. } => {
            HANDLE_EVALUATE_HANDLE_COMMAND
        }
        PwrightHandleEvaluation::HandleEvaluateHandleResult { .. } => HANDLE_EVALUATE_HANDLE_RESULT,
        PwrightHandleEvaluation::HandleJsonValueCommand { .. } => HANDLE_JSON_VALUE_COMMAND,
        PwrightHandleEvaluation::HandleJsonValueResult { .. } => HANDLE_JSON_VALUE_RESULT,
        PwrightHandleEvaluation::HandleGetPropertyCommand { .. } => HANDLE_GET_PROPERTY_COMMAND,
        PwrightHandleEvaluation::HandleGetPropertyResult { .. } => HANDLE_GET_PROPERTY_RESULT,
        PwrightHandleEvaluation::HandleGetPropertiesCommand { .. } => HANDLE_GET_PROPERTIES_COMMAND,
        PwrightHandleEvaluation::HandleGetPropertiesResult { .. } => HANDLE_GET_PROPERTIES_RESULT,
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

fn operation_error_to_wire(
    error: &OperationError<HandleEvaluationErrorDetails>,
) -> PwrightHandleEvaluationError {
    PwrightHandleEvaluationError {
        failure_kind: failure_kind_to_wire(error.kind()),
        message: error.message().to_owned(),
        details: error.details().cloned().map(Into::into),
    }
}

fn operation_error_from_wire(
    error: PwrightHandleEvaluationError,
) -> OperationError<HandleEvaluationErrorDetails> {
    OperationError::new(
        failure_kind_from_wire(error.failure_kind),
        error.message,
        error.details.map(Into::into),
    )
}

fn validate_operation_error(
    error: &OperationError<HandleEvaluationErrorDetails>,
    limit: usize,
) -> Result<(), HandleEvaluationCodecError> {
    validate_error_string(error.message(), limit)?;
    match error.details() {
        None => {}
        Some(HandleEvaluationErrorDetails::Javascript {
            name,
            message,
            stack,
        }) => {
            validate_error_string(name, limit)?;
            validate_error_string(message, limit)?;
            if let Some(stack) = stack {
                validate_error_string(stack, limit)?;
            }
        }
        Some(HandleEvaluationErrorDetails::InvalidValue(message))
        | Some(HandleEvaluationErrorDetails::InvalidBackendResponse(message))
        | Some(HandleEvaluationErrorDetails::Backend(message)) => {
            validate_error_string(message, limit)?;
        }
        Some(HandleEvaluationErrorDetails::Cleanup { primary, cleanup }) => {
            validate_error_string(primary, limit)?;
            validate_error_string(cleanup, limit)?;
        }
    }
    Ok(())
}

fn validate_error_string(value: &str, limit: usize) -> Result<(), HandleEvaluationCodecError> {
    if value.len() <= limit {
        Ok(())
    } else {
        Err(HandleEvaluationCodecError::ErrorStringTooLong {
            actual: value.len(),
            limit,
        })
    }
}

#[cfg(test)]
mod tests {
    use pwright_cap_evaluation::{EvaluationValue, EvaluationValueLimits};
    use pwright_cap_handle_evaluation::{
        EVALUATE_HANDLE_COMMAND_ID, EVALUATE_HANDLE_METHOD, EvaluateHandle, EvaluateHandleParams,
        GetPropertiesOptions, GetPropertyParams, HandleEvaluate, HandleEvaluateHandle,
        HandleEvaluateParams, HandleEvaluationProperty, HandleEvaluationValue, HandleGetProperties,
        HandleGetProperty, HandleInitializer, HandleJsonValue, HandlePropertiesResult,
        HandlePropertyInitializer, HandleValueResult, JsonValueOptions,
    };
    use pwright_core::{
        CallId, CallMetadata, CancellationId, Command, CommandResult, ObjectId, ObjectKindTag,
        ProtocolVersion,
    };
    use pwright_transport::{FrameLimits, VersionPolicy, WireCodec, WireProtocol};

    use super::{
        HANDLE_EVALUATION_COMMAND_SCHEMAS, HANDLE_EVALUATION_SCHEMA,
        HANDLE_EVALUATION_SCHEMA_CATALOG, HandleEvaluationCodec, HandleEvaluationCodecLimits,
    };

    fn codec() -> HandleEvaluationCodec {
        HandleEvaluationCodec::new(
            HandleEvaluationCodecLimits::new(
                4096,
                1024,
                EvaluationValueLimits::new(16, 128, 128, 4096, 4096).unwrap(),
            )
            .unwrap(),
        )
    }

    fn protocol() -> WireProtocol<HandleEvaluationCodec> {
        WireProtocol::new(
            codec(),
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0)).unwrap(),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(64 * 1024, 0, 0, 64 * 1024),
        )
        .unwrap()
    }

    #[test]
    fn command_round_trips_value_argument_and_exact_route() {
        // Arrange
        let command = Command::<EvaluateHandle>::new(
            CallId::new(7),
            ObjectId::new(11),
            CallMetadata::default(),
            EvaluateHandleParams::function(
                "value => ({ value })",
                EvaluationValue::String("x".into()),
            ),
        );

        // Act
        let frame = protocol().encode(&command).unwrap();
        let decoded = protocol().decode::<Command<EvaluateHandle>>(frame).unwrap();

        // Assert
        assert_eq!(decoded.call_id(), CallId::new(7));
        assert_eq!(decoded.object_id(), ObjectId::new(11));
        assert_eq!(decoded.params(), command.params());
    }

    #[test]
    fn command_round_trips_remote_cancellation_identity() {
        // Arrange
        let metadata = CallMetadata::new(None, Some(CancellationId::new(17)));
        let command = Command::<EvaluateHandle>::new(
            CallId::new(7),
            ObjectId::new(11),
            metadata,
            EvaluateHandleParams::expression("document.body"),
        );

        // Act
        let frame = protocol().encode(&command).unwrap();
        let decoded = protocol().decode::<Command<EvaluateHandle>>(frame).unwrap();

        // Assert
        assert_eq!(decoded.metadata(), metadata);
    }

    #[test]
    fn initializer_round_trips_without_a_native_cdp_identifier() {
        // Arrange
        let initializer = HandleInitializer::new(
            ObjectId::new(12),
            ObjectId::new(11),
            ObjectKindTag::new(5),
            "JSHandle@object",
        );
        let result = CommandResult::<EvaluateHandle>::new(CallId::new(7), Ok(initializer.clone()));

        // Act
        let frame = protocol().encode(&result).unwrap();
        let decoded = protocol()
            .decode::<CommandResult<EvaluateHandle>>(frame)
            .unwrap();

        // Assert
        assert_eq!(decoded.into_outcome(), Ok(initializer));
    }

    #[test]
    fn generated_catalog_matches_command_identity() {
        // Arrange
        let command = HANDLE_EVALUATION_COMMAND_SCHEMAS[0];

        // Act
        let validation = HANDLE_EVALUATION_SCHEMA_CATALOG.validate();

        // Assert
        assert_eq!(validation, Ok(()));
        assert_eq!(
            HANDLE_EVALUATION_SCHEMA.version(),
            ProtocolVersion::new(1, 2)
        );
        assert_eq!(command.id(), EVALUATE_HANDLE_COMMAND_ID);
        assert_eq!(command.method(), EVALUATE_HANDLE_METHOD);
    }

    #[test]
    fn nested_handle_argument_round_trips_as_a_portable_object_id() {
        // Arrange
        let argument = HandleEvaluationValue::Object {
            id: None,
            properties: vec![HandleEvaluationProperty::new(
                "node",
                HandleEvaluationValue::Handle(ObjectId::new(41)),
            )],
        };
        let command = Command::<EvaluateHandle>::new(
            CallId::new(8),
            ObjectId::new(11),
            CallMetadata::default(),
            EvaluateHandleParams::function("value => value.node", argument),
        );

        // Act
        let frame = protocol().encode(&command).unwrap();
        let decoded = protocol().decode::<Command<EvaluateHandle>>(frame).unwrap();

        // Assert
        assert_eq!(decoded.params(), command.params());
        assert_eq!(
            decoded
                .params()
                .referenced_handles(codec().limits().value())
                .unwrap(),
            [ObjectId::new(41)]
        );
    }

    #[test]
    fn capability_version_one_zero_rejects_handle_leaves() {
        // Arrange
        let command = Command::<EvaluateHandle>::new(
            CallId::new(8),
            ObjectId::new(11),
            CallMetadata::default(),
            EvaluateHandleParams::function(
                "value => value",
                HandleEvaluationValue::Handle(ObjectId::new(41)),
            ),
        );
        let old_codec = codec().with_capability_version(ProtocolVersion::new(1, 0));

        // Act
        let error = old_codec.encode_parts(&command).unwrap_err();

        // Assert
        assert!(matches!(
            error,
            super::HandleEvaluationCodecError::HandleArgumentsUnsupported {
                version: ProtocolVersion { major: 1, minor: 0 }
            }
        ));
    }

    #[test]
    fn all_handle_local_commands_round_trip_at_version_one_two() {
        // Arrange
        let evaluate = Command::<HandleEvaluate>::new(
            CallId::new(20),
            ObjectId::new(41),
            CallMetadata::default(),
            HandleEvaluateParams::function(
                "(target, arg) => target[arg]",
                EvaluationValue::String("name".to_owned()),
            ),
        );
        let evaluate_handle = Command::<HandleEvaluateHandle>::new(
            CallId::new(21),
            ObjectId::new(41),
            CallMetadata::default(),
            HandleEvaluateParams::function_without_argument("target => target.firstChild"),
        );
        let json_value = Command::<HandleJsonValue>::new(
            CallId::new(22),
            ObjectId::new(41),
            CallMetadata::default(),
            JsonValueOptions,
        );
        let get_property = Command::<HandleGetProperty>::new(
            CallId::new(23),
            ObjectId::new(41),
            CallMetadata::default(),
            GetPropertyParams::new("name"),
        );
        let get_properties = Command::<HandleGetProperties>::new(
            CallId::new(24),
            ObjectId::new(41),
            CallMetadata::default(),
            GetPropertiesOptions,
        );

        // Act
        let evaluate_decoded = protocol()
            .decode::<Command<HandleEvaluate>>(protocol().encode(&evaluate).unwrap())
            .unwrap();
        let evaluate_handle_decoded = protocol()
            .decode::<Command<HandleEvaluateHandle>>(protocol().encode(&evaluate_handle).unwrap())
            .unwrap();
        let json_value_decoded = protocol()
            .decode::<Command<HandleJsonValue>>(protocol().encode(&json_value).unwrap())
            .unwrap();
        let get_property_decoded = protocol()
            .decode::<Command<HandleGetProperty>>(protocol().encode(&get_property).unwrap())
            .unwrap();
        let get_properties_decoded = protocol()
            .decode::<Command<HandleGetProperties>>(protocol().encode(&get_properties).unwrap())
            .unwrap();

        // Assert
        assert_eq!(evaluate_decoded.params(), evaluate.params());
        assert_eq!(evaluate_handle_decoded.params(), evaluate_handle.params());
        assert_eq!(json_value_decoded.params(), json_value.params());
        assert_eq!(get_property_decoded.params(), get_property.params());
        assert_eq!(get_properties_decoded.params(), get_properties.params());
    }

    #[test]
    fn handle_local_results_round_trip_values_and_ordered_properties() {
        // Arrange
        let value = CommandResult::<HandleEvaluate>::new(
            CallId::new(20),
            Ok(HandleValueResult::new(EvaluationValue::NegativeZero)),
        );
        let handle = HandleInitializer::new(
            ObjectId::new(51),
            ObjectId::new(4),
            ObjectKindTag::new(5),
            "JSHandle@object",
        );
        let handle_result =
            CommandResult::<HandleEvaluateHandle>::new(CallId::new(21), Ok(handle.clone()));
        let json = CommandResult::<HandleJsonValue>::new(
            CallId::new(22),
            Ok(HandleValueResult::new(EvaluationValue::String(
                "ok".to_owned(),
            ))),
        );
        let property = CommandResult::<HandleGetProperty>::new(CallId::new(23), Ok(handle.clone()));
        let properties = CommandResult::<HandleGetProperties>::new(
            CallId::new(24),
            Ok(HandlePropertiesResult::new(vec![
                HandlePropertyInitializer::new("first", handle.clone()),
                HandlePropertyInitializer::new(
                    "second",
                    HandleInitializer::new(
                        ObjectId::new(52),
                        ObjectId::new(4),
                        ObjectKindTag::new(6),
                        "button",
                    ),
                ),
            ])),
        );

        // Act
        let value_decoded = protocol()
            .decode::<CommandResult<HandleEvaluate>>(protocol().encode(&value).unwrap())
            .unwrap();
        let handle_decoded = protocol()
            .decode::<CommandResult<HandleEvaluateHandle>>(
                protocol().encode(&handle_result).unwrap(),
            )
            .unwrap();
        let json_decoded = protocol()
            .decode::<CommandResult<HandleJsonValue>>(protocol().encode(&json).unwrap())
            .unwrap();
        let property_decoded = protocol()
            .decode::<CommandResult<HandleGetProperty>>(protocol().encode(&property).unwrap())
            .unwrap();
        let properties_decoded = protocol()
            .decode::<CommandResult<HandleGetProperties>>(protocol().encode(&properties).unwrap())
            .unwrap();

        // Assert
        assert_eq!(value_decoded.into_outcome(), value.into_outcome());
        assert_eq!(handle_decoded.into_outcome(), handle_result.into_outcome());
        assert_eq!(json_decoded.into_outcome(), json.into_outcome());
        assert_eq!(property_decoded.into_outcome(), property.into_outcome());
        assert_eq!(properties_decoded.into_outcome(), properties.into_outcome());
    }

    #[test]
    fn version_one_one_rejects_handle_local_commands_before_encoding() {
        // Arrange
        let command = Command::<HandleJsonValue>::new(
            CallId::new(22),
            ObjectId::new(41),
            CallMetadata::default(),
            JsonValueOptions,
        );
        let old_codec = codec().with_capability_version(ProtocolVersion::new(1, 1));

        // Act
        let error = old_codec.encode_parts(&command).unwrap_err();

        // Assert
        assert!(matches!(
            error,
            super::HandleEvaluationCodecError::HandleOperationsUnsupported {
                version: ProtocolVersion { major: 1, minor: 1 }
            }
        ));
    }
}
