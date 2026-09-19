//! Canonical, bounded binary payloads for the optional navigation capability.
//!
//! Routing identities are duplicated inside the payload so both client and
//! server can verify them against the outer `WireFrame` route.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::str::Utf8Error;
use std::time::Duration;

use pwright_cap_navigation::{
    BeginNavigation, Goto, GotoResult, Header, NavigationErrorDetails, NavigationResponse,
};
use pwright_codec_binary::{
    BinaryDecodeError, BinaryEncode, BinaryEncodeError, BinaryEncodeSequence,
    BinaryEncodeSequenceRef, BinaryNumericRangeError, BinarySizeRangeError, BinaryValueContext,
    BinaryValueDecodeError, BinaryValueEncodeError, BinaryValueLimitError, BinaryValueLimits,
    BinaryValueResource, BinaryWriter, decode_value, encode_value,
};
use pwright_core::{
    CallId, CallMetadata, CancellationId, Command, CommandResult, CommandSpec, FailureKind,
    MethodTag, ObjectId, OperationError,
};
use pwright_transport::{EncodedParts, FrameRoute, WireCodec, WireFrame};

const BEGIN_COMMAND: u8 = 1;
const BEGIN_RESULT: u8 = 2;
const GOTO_COMMAND: u8 = 3;
const GOTO_RESULT: u8 = 4;
const RELOAD_COMMAND: u8 = 5;
const RELOAD_RESULT: u8 = 6;

mod reload;

#[doc(hidden)]
#[rustfmt::skip]
pub mod generated_schema;

/// Explicitly selected 1.2 values and typed codecs. Root bindings remain 1.1.
#[cfg(feature = "frame-navigation")]
pub mod v1_2;

pub use generated_schema::{
    CATALOG as NAVIGATION_SCHEMA_CATALOG, COMMANDS as NAVIGATION_COMMAND_SCHEMAS,
    ROOT_SCHEMA as NAVIGATION_SCHEMA,
};
use generated_schema::{
    PwrightCommonCallMetadata, PwrightCommonDuration, PwrightCommonFailureKind, PwrightNavigation,
    PwrightNavigationBeginNavigationParamsRef, PwrightNavigationBeginNavigationResultRef,
    PwrightNavigationBeginOutcome, PwrightNavigationBeginOutcomeRef, PwrightNavigationError,
    PwrightNavigationErrorDetailsRef, PwrightNavigationErrorRef, PwrightNavigationGotoOutcome,
    PwrightNavigationGotoOutcomeRef, PwrightNavigationGotoParamsRef, PwrightNavigationGotoResult,
    PwrightNavigationGotoResultRef, PwrightNavigationHeaderRef, PwrightNavigationRef,
    PwrightNavigationResponse, PwrightNavigationResponseRef,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NavigationCodecLimits {
    pub string_bytes: usize,
    pub headers: usize,
    pub header_bytes: usize,
}

impl NavigationCodecLimits {
    pub const fn new(string_bytes: usize, headers: usize, header_bytes: usize) -> Self {
        Self {
            string_bytes,
            headers,
            header_bytes,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NavigationCodec {
    limits: NavigationCodecLimits,
}

impl NavigationCodec {
    pub const fn new(limits: NavigationCodecLimits) -> Self {
        Self { limits }
    }

    pub const fn limits(self) -> NavigationCodecLimits {
        self.limits
    }

    fn value_limits(self) -> BinaryValueLimits {
        BinaryValueLimits::new(
            self.limits.string_bytes,
            0,
            self.limits.headers,
            8,
            self.limits.headers.saturating_add(16),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NavigationCodecError {
    WrongRoute(FrameRoute),
    RouteIdentityMismatch {
        outer: FrameRoute,
        inner: FrameRoute,
    },
    WrongMethod {
        expected: MethodTag,
        actual: MethodTag,
    },
    AttachmentsUnsupported {
        actual: usize,
    },
    WrongPayloadKind {
        expected: u8,
        actual: u8,
    },
    StringTooLong {
        actual: usize,
        limit: usize,
    },
    TooManyHeaders {
        actual: usize,
        limit: usize,
    },
    HeaderBytesTooLarge {
        actual: usize,
        limit: usize,
    },
    NonCanonicalHeaderOrder,
    InvalidUtf8(Utf8Error),
    InvalidBoolean(u8),
    InvalidOptionFlag(u8),
    InvalidWaitUntil(u8),
    InvalidFailureKind(u8),
    InvalidErrorDetails(u8),
    InvalidOutcome(u8),
    InvalidDurationNanos(u32),
    IntegerOutOfRange {
        field: &'static str,
        value: u64,
    },
    AllocationFailed {
        entries: usize,
    },
    NumericRange(BinaryNumericRangeError),
    SizeRange(BinarySizeRangeError),
    ValueLimit(BinaryValueLimitError),
    ValueEncode(BinaryValueEncodeError),
    ValueDecode(BinaryValueDecodeError),
    Encode(BinaryEncodeError),
    Truncated {
        offset: usize,
        needed: usize,
        remaining: usize,
    },
    TrailingBytes {
        remaining: usize,
    },
}

impl Display for NavigationCodecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongRoute(route) => write!(formatter, "navigation payload arrived on {route:?}"),
            Self::RouteIdentityMismatch { outer, inner } => write!(
                formatter,
                "navigation payload route {inner:?} differs from envelope {outer:?}"
            ),
            Self::WrongMethod { expected, actual } => {
                write!(
                    formatter,
                    "expected navigation method {expected}, received {actual}"
                )
            }
            Self::AttachmentsUnsupported { actual } => write!(
                formatter,
                "navigation payload has {actual} attachment(s), but none are allowed"
            ),
            Self::WrongPayloadKind { expected, actual } => {
                write!(
                    formatter,
                    "expected payload kind {expected}, received {actual}"
                )
            }
            Self::StringTooLong { actual, limit } => {
                write!(
                    formatter,
                    "string has {actual} bytes, exceeding limit {limit}"
                )
            }
            Self::TooManyHeaders { actual, limit } => write!(
                formatter,
                "response has {actual} headers, exceeding limit {limit}"
            ),
            Self::HeaderBytesTooLarge { actual, limit } => write!(
                formatter,
                "response headers have {actual} bytes, exceeding limit {limit}"
            ),
            Self::NonCanonicalHeaderOrder => {
                formatter.write_str("response headers are not in canonical order")
            }
            Self::InvalidUtf8(error) => write!(formatter, "string is not UTF-8: {error}"),
            Self::InvalidBoolean(value) => write!(formatter, "invalid boolean value {value}"),
            Self::InvalidOptionFlag(value) => write!(formatter, "invalid option flag {value}"),
            Self::InvalidWaitUntil(value) => write!(formatter, "invalid wait-until value {value}"),
            Self::InvalidFailureKind(value) => write!(formatter, "invalid failure kind {value}"),
            Self::InvalidErrorDetails(value) => {
                write!(formatter, "invalid navigation error-details kind {value}")
            }
            Self::InvalidOutcome(value) => write!(formatter, "invalid result outcome {value}"),
            Self::InvalidDurationNanos(value) => {
                write!(formatter, "duration nanoseconds {value} are not canonical")
            }
            Self::IntegerOutOfRange { field, value } => {
                write!(formatter, "wire value {value} does not fit {field}")
            }
            Self::AllocationFailed { entries } => {
                write!(
                    formatter,
                    "could not allocate storage for {entries} headers"
                )
            }
            Self::NumericRange(error) => Display::fmt(error, formatter),
            Self::SizeRange(error) => Display::fmt(error, formatter),
            Self::ValueLimit(error) => Display::fmt(error, formatter),
            Self::ValueEncode(error) => Display::fmt(error, formatter),
            Self::ValueDecode(error) => Display::fmt(error, formatter),
            Self::Encode(error) => Display::fmt(error, formatter),
            Self::Truncated {
                offset,
                needed,
                remaining,
            } => write!(
                formatter,
                "navigation payload at byte {offset} needs {needed} bytes but only {remaining} remain"
            ),
            Self::TrailingBytes { remaining } => {
                write!(
                    formatter,
                    "navigation payload has {remaining} trailing bytes"
                )
            }
        }
    }
}

impl Error for NavigationCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidUtf8(error) => Some(error),
            Self::Encode(error) => Some(error),
            Self::NumericRange(error) => Some(error),
            Self::SizeRange(error) => Some(error),
            Self::ValueLimit(error) => Some(error),
            Self::ValueEncode(error) => Some(error),
            Self::ValueDecode(error) => Some(error),
            _ => None,
        }
    }
}

impl From<BinaryEncodeError> for NavigationCodecError {
    fn from(error: BinaryEncodeError) -> Self {
        Self::Encode(error)
    }
}

impl From<BinaryDecodeError> for NavigationCodecError {
    fn from(error: BinaryDecodeError) -> Self {
        match error {
            BinaryDecodeError::IntegerOutOfRange { field, value } => {
                Self::IntegerOutOfRange { field, value }
            }
            BinaryDecodeError::Truncated {
                offset,
                needed,
                remaining,
            } => Self::Truncated {
                offset,
                needed,
                remaining,
            },
            BinaryDecodeError::TrailingBytes { remaining } => Self::TrailingBytes { remaining },
        }
    }
}

impl WireCodec<Command<BeginNavigation>> for NavigationCodec {
    type Error = NavigationCodecError;

    fn encode_parts(
        &self,
        command: &Command<BeginNavigation>,
    ) -> Result<EncodedParts, Self::Error> {
        let params = command.params();
        let wire = PwrightNavigationRef::BeginCommand {
            call_id: command.call_id().get(),
            object_id: command.object_id().get(),
            metadata: metadata_to_wire(command.metadata()),
            params: PwrightNavigationBeginNavigationParamsRef {
                url: &params.url,
                referrer: params.referrer.as_deref(),
            },
        };
        let payload = encode_value(&wire, self.value_limits()).map_err(map_generated_encode)?;
        Ok(command_parts(command, payload))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<BeginNavigation>, Self::Error> {
        command_route::<BeginNavigation>(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightNavigation>(frame.payload(), self.value_limits())
            .map_err(|error| map_generated_decode(error, BEGIN_COMMAND))?
        {
            PwrightNavigation::BeginCommand {
                call_id,
                object_id,
                metadata,
                params,
            } => Ok(Command::new(
                CallId::new(call_id),
                ObjectId::new(object_id),
                metadata_from_wire(metadata),
                params.into(),
            )),
            other => Err(NavigationCodecError::WrongPayloadKind {
                expected: BEGIN_COMMAND,
                actual: navigation_wire_kind(&other),
            }),
        }
    }
}

impl WireCodec<CommandResult<BeginNavigation>> for NavigationCodec {
    type Error = NavigationCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<BeginNavigation>,
    ) -> Result<EncodedParts, Self::Error> {
        let outcome = match result.outcome() {
            Ok(value) => PwrightNavigationBeginOutcomeRef::Success {
                result: PwrightNavigationBeginNavigationResultRef {
                    frame_id: &value.frame_id,
                    loader_id: value.loader_id.as_deref(),
                    same_document: value.same_document,
                    is_download: value.is_download,
                },
            },
            Err(error) => PwrightNavigationBeginOutcomeRef::Error {
                error: operation_error_to_wire_ref(error),
            },
        };
        let wire = PwrightNavigationRef::BeginResult {
            call_id: result.call_id().get(),
            outcome,
        };
        let payload = encode_value(&wire, self.value_limits()).map_err(map_generated_encode)?;
        Ok(result_parts(result.call_id(), payload))
    }

    fn decode_frame(
        &self,
        frame: WireFrame,
    ) -> Result<CommandResult<BeginNavigation>, Self::Error> {
        result_route(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightNavigation>(frame.payload(), self.value_limits())
            .map_err(|error| map_generated_decode(error, BEGIN_RESULT))?
        {
            PwrightNavigation::BeginResult { call_id, outcome } => {
                let outcome = match outcome {
                    PwrightNavigationBeginOutcome::Success { result } => Ok(result.into()),
                    PwrightNavigationBeginOutcome::Error { error } => {
                        Err(operation_error_from_wire(error))
                    }
                };
                Ok(CommandResult::new(CallId::new(call_id), outcome))
            }
            other => Err(NavigationCodecError::WrongPayloadKind {
                expected: BEGIN_RESULT,
                actual: navigation_wire_kind(&other),
            }),
        }
    }
}

impl WireCodec<Command<Goto>> for NavigationCodec {
    type Error = NavigationCodecError;

    fn encode_parts(&self, command: &Command<Goto>) -> Result<EncodedParts, Self::Error> {
        let params = command.params();
        let wire = PwrightNavigationRef::GotoCommand {
            call_id: command.call_id().get(),
            object_id: command.object_id().get(),
            metadata: metadata_to_wire(command.metadata()),
            params: PwrightNavigationGotoParamsRef {
                url: &params.url,
                referrer: params.referrer.as_deref(),
                wait_until: params.wait_until.into(),
            },
        };
        let payload = encode_value(&wire, self.value_limits()).map_err(map_generated_encode)?;
        Ok(command_parts(command, payload))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<Goto>, Self::Error> {
        command_route::<Goto>(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightNavigation>(frame.payload(), self.value_limits())
            .map_err(|error| map_generated_decode(error, GOTO_COMMAND))?
        {
            PwrightNavigation::GotoCommand {
                call_id,
                object_id,
                metadata,
                params,
            } => Ok(Command::new(
                CallId::new(call_id),
                ObjectId::new(object_id),
                metadata_from_wire(metadata),
                params.into(),
            )),
            other => Err(NavigationCodecError::WrongPayloadKind {
                expected: GOTO_COMMAND,
                actual: navigation_wire_kind(&other),
            }),
        }
    }
}

impl WireCodec<CommandResult<Goto>> for NavigationCodec {
    type Error = NavigationCodecError;

    fn encode_parts(&self, result: &CommandResult<Goto>) -> Result<EncodedParts, Self::Error> {
        let canonical_headers = result
            .outcome()
            .as_ref()
            .ok()
            .and_then(|value| value.response.as_ref())
            .map(|response| CanonicalHeaders::new(&response.headers));
        let outcome = match result.outcome() {
            Ok(value) => PwrightNavigationGotoOutcomeRef::Success {
                result: self.goto_result_to_wire_ref(value, canonical_headers.as_ref())?,
            },
            Err(error) => PwrightNavigationGotoOutcomeRef::Error {
                error: operation_error_to_wire_ref(error),
            },
        };
        let wire = PwrightNavigationRef::GotoResult {
            call_id: result.call_id().get(),
            outcome,
        };
        let payload = encode_value(&wire, self.value_limits()).map_err(map_generated_encode)?;
        Ok(result_parts(result.call_id(), payload))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<Goto>, Self::Error> {
        result_route(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightNavigation>(frame.payload(), self.value_limits())
            .map_err(|error| map_generated_decode(error, GOTO_RESULT))?
        {
            PwrightNavigation::GotoResult { call_id, outcome } => {
                let outcome = match outcome {
                    PwrightNavigationGotoOutcome::Success { result } => {
                        Ok(self.goto_result_from_wire(result)?)
                    }
                    PwrightNavigationGotoOutcome::Error { error } => {
                        Err(operation_error_from_wire(error))
                    }
                };
                Ok(CommandResult::new(CallId::new(call_id), outcome))
            }
            other => Err(NavigationCodecError::WrongPayloadKind {
                expected: GOTO_RESULT,
                actual: navigation_wire_kind(&other),
            }),
        }
    }
}

impl NavigationCodec {
    fn goto_result_to_wire_ref<'a>(
        &self,
        value: &'a GotoResult,
        canonical_headers: Option<&'a CanonicalHeaders<'a>>,
    ) -> Result<PwrightNavigationGotoResultRef<'a>, NavigationCodecError> {
        Ok(PwrightNavigationGotoResultRef {
            frame_id: &value.frame_id,
            loader_id: value.loader_id.as_deref(),
            url: &value.url,
            same_document: value.same_document,
            response: match (&value.response, canonical_headers) {
                (Some(response), Some(headers)) => {
                    Some(self.response_to_wire_ref(response, headers)?)
                }
                (None, None) => None,
                _ => unreachable!("header view and response derive from the same result"),
            },
        })
    }

    fn goto_result_from_wire(
        &self,
        value: PwrightNavigationGotoResult,
    ) -> Result<GotoResult, NavigationCodecError> {
        if let Some(response) = &value.response {
            self.validate_wire_response(response)?;
        }
        Ok(value.into())
    }

    fn response_to_wire_ref<'a>(
        &self,
        response: &'a NavigationResponse,
        headers: &'a CanonicalHeaders<'a>,
    ) -> Result<PwrightNavigationResponseRef<'a>, NavigationCodecError> {
        if response.headers.len() > self.limits.headers {
            return Err(NavigationCodecError::TooManyHeaders {
                actual: response.headers.len(),
                limit: self.limits.headers,
            });
        }
        self.validate_header_bytes(
            response
                .headers
                .iter()
                .map(|header| (header.name.as_str(), header.value.as_str())),
        )?;
        Ok(PwrightNavigationResponseRef {
            url: &response.url,
            status: response.status,
            status_text: &response.status_text,
            headers: BinaryEncodeSequenceRef::new(headers),
        })
    }

    fn validate_wire_response(
        &self,
        response: &PwrightNavigationResponse,
    ) -> Result<(), NavigationCodecError> {
        self.validate_header_bytes(
            response
                .headers
                .iter()
                .map(|header| (header.name.as_str(), header.value.as_str())),
        )?;
        if response
            .headers
            .windows(2)
            .any(|pair| (&pair[0].name, &pair[0].value) > (&pair[1].name, &pair[1].value))
        {
            return Err(NavigationCodecError::NonCanonicalHeaderOrder);
        }
        Ok(())
    }

    fn validate_header_bytes<'a>(
        &self,
        mut headers: impl Iterator<Item = (&'a str, &'a str)>,
    ) -> Result<(), NavigationCodecError> {
        let header_bytes = headers.try_fold(0_usize, |total, (name, value)| {
            total
                .checked_add(name.len())
                .and_then(|total| total.checked_add(value.len()))
                .ok_or(NavigationCodecError::HeaderBytesTooLarge {
                    actual: usize::MAX,
                    limit: self.limits.header_bytes,
                })
        })?;
        if header_bytes > self.limits.header_bytes {
            Err(NavigationCodecError::HeaderBytesTooLarge {
                actual: header_bytes,
                limit: self.limits.header_bytes,
            })
        } else {
            Ok(())
        }
    }
}

struct CanonicalHeaders<'a> {
    headers: &'a [Header],
}

impl<'a> CanonicalHeaders<'a> {
    const fn new(headers: &'a [Header]) -> Self {
        Self { headers }
    }

    fn compare(&self, left: usize, right: usize) -> std::cmp::Ordering {
        let left_header = &self.headers[left];
        let right_header = &self.headers[right];
        (&left_header.name, &left_header.value, left).cmp(&(
            &right_header.name,
            &right_header.value,
            right,
        ))
    }
}

impl BinaryEncodeSequence for CanonicalHeaders<'_> {
    fn len(&self) -> usize {
        self.headers.len()
    }

    fn encode_items(
        &self,
        writer: &mut BinaryWriter,
        context: &mut BinaryValueContext,
    ) -> Result<(), BinaryValueEncodeError> {
        let mut previous = None;
        for _ in 0..self.headers.len() {
            let next = (0..self.headers.len())
                .filter(|candidate| {
                    previous.is_none_or(|prior| self.compare(*candidate, prior).is_gt())
                })
                .min_by(|left, right| self.compare(*left, *right));
            if let Some(index) = next {
                let header = &self.headers[index];
                PwrightNavigationHeaderRef {
                    name: &header.name,
                    value: &header.value,
                }
                .encode_binary(writer, context)?;
                previous = Some(index);
            }
        }
        Ok(())
    }
}

fn command_parts<S: CommandSpec>(command: &Command<S>, payload: Vec<u8>) -> EncodedParts {
    EncodedParts::new(
        FrameRoute::Command {
            call_id: command.call_id(),
            target: command.object_id(),
            method: S::METHOD,
        },
        payload,
        Vec::new(),
    )
}

fn result_parts(call_id: CallId, payload: Vec<u8>) -> EncodedParts {
    EncodedParts::new(FrameRoute::Result { call_id }, payload, Vec::new())
}

fn ensure_no_attachments(frame: &WireFrame) -> Result<(), NavigationCodecError> {
    if frame.attachments().is_empty() {
        Ok(())
    } else {
        Err(NavigationCodecError::AttachmentsUnsupported {
            actual: frame.attachments().len(),
        })
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
    value: &NavigationErrorDetails,
) -> PwrightNavigationErrorDetailsRef<'_> {
    match value {
        NavigationErrorDetails::Network(message) => {
            PwrightNavigationErrorDetailsRef::Network { message }
        }
        NavigationErrorDetails::InvalidBackendResponse(message) => {
            PwrightNavigationErrorDetailsRef::InvalidBackendResponse { message }
        }
        NavigationErrorDetails::UnsupportedOption(message) => {
            PwrightNavigationErrorDetailsRef::UnsupportedOption { message }
        }
        NavigationErrorDetails::Backend(message) => {
            PwrightNavigationErrorDetailsRef::Backend { message }
        }
    }
}

fn operation_error_to_wire_ref(
    value: &OperationError<NavigationErrorDetails>,
) -> PwrightNavigationErrorRef<'_> {
    PwrightNavigationErrorRef {
        failure_kind: failure_kind_to_wire(value.kind()),
        message: value.message(),
        details: value.details().map(error_details_to_wire_ref),
    }
}

fn operation_error_from_wire(
    value: PwrightNavigationError,
) -> OperationError<NavigationErrorDetails> {
    OperationError::new(
        failure_kind_from_wire(value.failure_kind),
        value.message,
        value.details.map(Into::into),
    )
}

fn navigation_wire_kind(value: &PwrightNavigation) -> u8 {
    match value {
        PwrightNavigation::BeginCommand { .. } => BEGIN_COMMAND,
        PwrightNavigation::BeginResult { .. } => BEGIN_RESULT,
        PwrightNavigation::GotoCommand { .. } => GOTO_COMMAND,
        PwrightNavigation::GotoResult { .. } => GOTO_RESULT,
        PwrightNavigation::ReloadCommand { .. } => RELOAD_COMMAND,
        PwrightNavigation::ReloadResult { .. } => RELOAD_RESULT,
    }
}

fn map_generated_encode(error: BinaryValueEncodeError) -> NavigationCodecError {
    match error {
        BinaryValueEncodeError::Limit(error)
            if error.resource == BinaryValueResource::StringBytes =>
        {
            NavigationCodecError::StringTooLong {
                actual: error.actual,
                limit: error.limit,
            }
        }
        BinaryValueEncodeError::Limit(error)
            if error.resource == BinaryValueResource::RepeatedItems =>
        {
            NavigationCodecError::TooManyHeaders {
                actual: error.actual,
                limit: error.limit,
            }
        }
        BinaryValueEncodeError::Limit(error) => NavigationCodecError::ValueLimit(error),
        BinaryValueEncodeError::NumericRange(error) => map_numeric_range(error),
        BinaryValueEncodeError::SizeRange(error) => NavigationCodecError::SizeRange(error),
        BinaryValueEncodeError::Primitive(error) => NavigationCodecError::Encode(error),
        error => NavigationCodecError::ValueEncode(error),
    }
}

fn map_generated_decode(error: BinaryValueDecodeError, expected: u8) -> NavigationCodecError {
    match error {
        BinaryValueDecodeError::Limit(error)
            if error.resource == BinaryValueResource::StringBytes =>
        {
            NavigationCodecError::StringTooLong {
                actual: error.actual,
                limit: error.limit,
            }
        }
        BinaryValueDecodeError::Limit(error)
            if error.resource == BinaryValueResource::RepeatedItems =>
        {
            NavigationCodecError::TooManyHeaders {
                actual: error.actual,
                limit: error.limit,
            }
        }
        BinaryValueDecodeError::Limit(error) => NavigationCodecError::ValueLimit(error),
        BinaryValueDecodeError::NumericRange(error) => map_numeric_range(error),
        BinaryValueDecodeError::SizeRange(error) => NavigationCodecError::SizeRange(error),
        BinaryValueDecodeError::Primitive(error) => error.into(),
        BinaryValueDecodeError::InvalidUtf8 { source, .. } => {
            NavigationCodecError::InvalidUtf8(source)
        }
        BinaryValueDecodeError::InvalidBoolean { value, .. } => {
            NavigationCodecError::InvalidBoolean(value)
        }
        BinaryValueDecodeError::InvalidOptionFlag { value, .. } => {
            NavigationCodecError::InvalidOptionFlag(value)
        }
        BinaryValueDecodeError::InvalidVariantTag { type_id, value } => match type_id {
            "pwright.navigation" => NavigationCodecError::WrongPayloadKind {
                expected,
                actual: value,
            },
            "pwright.navigation.wait_until" => NavigationCodecError::InvalidWaitUntil(value),
            "pwright.common.failure_kind" => NavigationCodecError::InvalidFailureKind(value),
            "pwright.navigation.error_details" => NavigationCodecError::InvalidErrorDetails(value),
            "pwright.navigation.begin_outcome"
            | "pwright.navigation.goto_outcome"
            | "pwright.navigation.frame_goto_outcome" => {
                NavigationCodecError::InvalidOutcome(value)
            }
            _ => NavigationCodecError::WrongPayloadKind {
                expected,
                actual: value,
            },
        },
        BinaryValueDecodeError::AllocationFailed { entries, .. } => {
            NavigationCodecError::AllocationFailed { entries }
        }
        error => NavigationCodecError::ValueDecode(error),
    }
}

fn map_numeric_range(error: BinaryNumericRangeError) -> NavigationCodecError {
    if error.field == "pwright.common.duration.nanoseconds"
        && let Ok(value) = u32::try_from(error.value)
    {
        NavigationCodecError::InvalidDurationNanos(value)
    } else {
        NavigationCodecError::NumericRange(error)
    }
}

fn command_route<S: CommandSpec>(
    frame: &WireFrame,
) -> Result<(CallId, ObjectId), NavigationCodecError> {
    let FrameRoute::Command {
        call_id,
        target,
        method,
    } = frame.route()
    else {
        return Err(NavigationCodecError::WrongRoute(frame.route()));
    };
    if method != S::METHOD {
        return Err(NavigationCodecError::WrongMethod {
            expected: S::METHOD,
            actual: method,
        });
    }
    Ok((call_id, target))
}

fn result_route(frame: &WireFrame) -> Result<CallId, NavigationCodecError> {
    let FrameRoute::Result { call_id } = frame.route() else {
        return Err(NavigationCodecError::WrongRoute(frame.route()));
    };
    Ok(call_id)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use pwright_cap_navigation::{
        BEGIN_NAVIGATION_COMMAND_ID, BEGIN_NAVIGATION_METHOD, BeginNavigation,
        BeginNavigationParams, BeginNavigationResult, GOTO_COMMAND_ID, GOTO_METHOD, Goto,
        GotoParams, GotoResult, Header, NavigationErrorDetails, NavigationResponse, WaitUntil,
    };
    use pwright_codec_binary::{
        BinaryValueDecodeError, BinaryValueEncodeError, BinaryValueLimits, BinaryValueResource,
        decode_value, encode_value,
    };
    use pwright_core::{
        CallId, CallMetadata, CancellationId, Command, CommandResult, CommandSpec, FailureKind,
        ObjectId, OperationError, ProtocolVersion,
    };
    use pwright_transport::{
        FrameLimits, FrameRoute, VersionPolicy, WireCodec, WireFrame, WireProtocol,
    };

    use super::{
        BEGIN_COMMAND, BEGIN_RESULT, GOTO_COMMAND, GOTO_RESULT, NAVIGATION_COMMAND_SCHEMAS,
        NAVIGATION_SCHEMA, NAVIGATION_SCHEMA_CATALOG, NavigationCodec, NavigationCodecError,
        NavigationCodecLimits,
    };
    use crate::generated_schema::{
        PwrightCommonCallMetadata, PwrightCommonDuration, PwrightNavigation,
        PwrightNavigationBeginNavigationResult, PwrightNavigationBeginOutcome,
        PwrightNavigationGotoParams, PwrightNavigationResponse, PwrightNavigationWaitUntil,
    };

    fn codec() -> NavigationCodec {
        NavigationCodec::new(NavigationCodecLimits::new(256, 4, 256))
    }

    fn protocol() -> WireProtocol<NavigationCodec> {
        WireProtocol::new(
            codec(),
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0))
                .expect("version policy should build"),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(4096, 0, 0, 4096),
        )
        .expect("protocol should build")
    }

    fn metadata() -> CallMetadata {
        CallMetadata::new(Some(Duration::new(12, 345)), Some(CancellationId::new(9)))
    }

    fn response() -> NavigationResponse {
        NavigationResponse {
            url: "https://example.com/final".to_string(),
            status: 404,
            status_text: "Not Found".to_string(),
            headers: vec![
                Header {
                    name: "x-z".to_string(),
                    value: "last".to_string(),
                },
                Header {
                    name: "content-type".to_string(),
                    value: "text/html".to_string(),
                },
            ],
        }
    }

    fn goto_command_frame(params: GotoParams, metadata: CallMetadata) -> WireFrame {
        protocol()
            .encode(&Command::<Goto>::new(
                CallId::new(1),
                ObjectId::new(2),
                metadata,
                params,
            ))
            .expect("goto command should encode")
    }

    fn rebuild_with_payload(frame: &WireFrame, payload: Vec<u8>) -> WireFrame {
        WireFrame::new(
            frame.version(),
            frame.route(),
            payload,
            Vec::new(),
            FrameLimits::new(4096, 0, 0, 4096),
        )
        .expect("mutated frame should fit")
    }

    fn decode_goto(frame: WireFrame) -> Result<Command<Goto>, NavigationCodecError> {
        <NavigationCodec as WireCodec<Command<Goto>>>::decode_frame(&codec(), frame)
    }

    #[test]
    fn both_command_types_round_trip_with_metadata_and_options() {
        // Arrange
        let protocol = protocol();
        let begin = Command::<BeginNavigation>::new(
            CallId::new(1),
            ObjectId::new(2),
            metadata(),
            BeginNavigationParams::new("https://example.com")
                .with_referrer("https://referrer.example"),
        );
        let goto = Command::<Goto>::new(
            CallId::new(3),
            ObjectId::new(4),
            metadata(),
            GotoParams::new("https://example.com")
                .with_referrer("https://referrer.example")
                .with_wait_until(WaitUntil::NetworkIdle),
        );

        // Act
        let begin = protocol
            .decode::<Command<BeginNavigation>>(protocol.encode(&begin).expect("encode begin"))
            .expect("decode begin");
        let goto = protocol
            .decode::<Command<Goto>>(protocol.encode(&goto).expect("encode goto"))
            .expect("decode goto");

        // Assert
        assert_eq!(begin.call_id(), CallId::new(1));
        assert_eq!(begin.object_id(), ObjectId::new(2));
        assert_eq!(begin.metadata(), metadata());
        assert_eq!(
            begin.params().referrer.as_deref(),
            Some("https://referrer.example")
        );
        assert_eq!(goto.call_id(), CallId::new(3));
        assert_eq!(goto.params().wait_until, WaitUntil::NetworkIdle);
    }

    #[test]
    fn navigation_schema_is_valid_and_simple_goto_bytes_are_stable() {
        // Arrange
        let command = Command::<Goto>::new(
            CallId::new(1),
            ObjectId::new(2),
            CallMetadata::default(),
            GotoParams::new("x"),
        );

        // Act
        let encoded = codec().encode_parts(&command).expect("goto should encode");

        // Assert
        assert_eq!(NAVIGATION_SCHEMA.validate(), Ok(()));
        assert_eq!(NAVIGATION_SCHEMA_CATALOG.validate(), Ok(()));
        assert_eq!(
            encoded.payload(),
            &[
                GOTO_COMMAND,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                1,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                2,
                0,
                0,
                0,
                0,
                0,
                1,
                b'x',
                0,
                2,
            ]
        );
    }

    #[test]
    fn navigation_reuses_the_common_wire_types_and_catalog_entries() {
        // Arrange
        let direct = pwright_codec_common::PwrightCommonDuration {
            seconds: 3,
            nanoseconds: 7,
        };
        let reexported: PwrightCommonDuration = direct;

        // Act
        let common_entries_are_shared = pwright_codec_common::CATALOG
            .types()
            .iter()
            .all(|common| NAVIGATION_SCHEMA_CATALOG.types().contains(common));

        // Assert
        assert_eq!(reexported.seconds, 3);
        assert_eq!(reexported.nanoseconds, 7);
        assert!(common_entries_are_shared);
    }

    #[test]
    fn generated_command_metadata_matches_the_typed_navigation_contract() {
        // Arrange
        let begin = NAVIGATION_COMMAND_SCHEMAS[0];
        let goto = NAVIGATION_COMMAND_SCHEMAS[1];

        // Act
        let wire_mappings = [
            (
                begin.id(),
                begin.method(),
                begin.command_tag(),
                begin.result_tag(),
            ),
            (
                goto.id(),
                goto.method(),
                goto.command_tag(),
                goto.result_tag(),
            ),
        ];

        // Assert
        assert_eq!(
            wire_mappings,
            [
                (
                    BEGIN_NAVIGATION_COMMAND_ID,
                    BEGIN_NAVIGATION_METHOD,
                    BEGIN_COMMAND,
                    BEGIN_RESULT,
                ),
                (GOTO_COMMAND_ID, GOTO_METHOD, GOTO_COMMAND, GOTO_RESULT),
            ]
        );
    }

    #[test]
    fn generated_navigation_codec_matches_the_published_goto_bytes() {
        // Arrange
        let wire = PwrightNavigation::GotoCommand {
            call_id: 1,
            object_id: 2,
            metadata: PwrightCommonCallMetadata {
                timeout: None,
                cancellation_id: None,
            },
            params: PwrightNavigationGotoParams {
                url: "x".to_owned(),
                referrer: None,
                wait_until: PwrightNavigationWaitUntil::Load,
            },
        };
        let limits = BinaryValueLimits::new(256, 0, 4, 8, 16);
        let published = goto_command_frame(GotoParams::new("x"), CallMetadata::default());

        // Act
        let encoded = encode_value(&wire, limits).expect("generated value should encode");
        let decoded = decode_value::<PwrightNavigation>(&encoded, limits)
            .expect("generated value should decode");

        // Assert
        assert_eq!(encoded, published.payload());
        assert_eq!(decoded, wire);
    }

    #[test]
    fn generated_navigation_codec_enforces_value_and_graph_limits() {
        // Arrange
        let wire = PwrightNavigation::GotoCommand {
            call_id: 1,
            object_id: 2,
            metadata: PwrightCommonCallMetadata {
                timeout: None,
                cancellation_id: None,
            },
            params: PwrightNavigationGotoParams {
                url: "x".to_owned(),
                referrer: None,
                wait_until: PwrightNavigationWaitUntil::Load,
            },
        };
        let encoded = encode_value(&wire, BinaryValueLimits::new(256, 0, 4, 8, 16))
            .expect("fixture should encode");

        // Act
        let encode_error = encode_value(&wire, BinaryValueLimits::new(0, 0, 4, 8, 16))
            .expect_err("oversized generated string must fail");
        let decode_error =
            decode_value::<PwrightNavigation>(&encoded, BinaryValueLimits::new(256, 0, 4, 1, 16))
                .expect_err("nested generated message must respect depth");

        // Assert
        assert!(matches!(
            encode_error,
            BinaryValueEncodeError::Limit(ref error)
                if error.resource == BinaryValueResource::StringBytes
                    && error.field == "pwright.navigation.goto_params.url"
                    && error.actual == 1
                    && error.limit == 0
        ));
        assert!(matches!(
            decode_error,
            BinaryValueDecodeError::Limit(ref error)
                if error.resource == BinaryValueResource::NestingDepth
                    && error.field == "pwright.common.call_metadata"
                    && error.actual == 2
                    && error.limit == 1
        ));
    }

    #[test]
    fn generated_repeated_decoder_preflights_minimum_bytes_before_allocation() {
        // Arrange
        let mut payload = vec![0; 16];
        payload.extend_from_slice(&1_u32.to_be_bytes());
        let limits = BinaryValueLimits::new(256, 0, 4, 8, 16);

        // Act
        let error = decode_value::<PwrightNavigationResponse>(&payload, limits)
            .expect_err("missing header body must fail before allocation");

        // Assert
        assert_eq!(
            error,
            BinaryValueDecodeError::Primitive(pwright_codec_binary::BinaryDecodeError::Truncated {
                offset: 20,
                needed: 8,
                remaining: 0,
            })
        );
    }

    #[test]
    fn generated_navigation_decoder_rejects_invalid_enum_and_boolean_tags() {
        // Arrange
        let limits = BinaryValueLimits::new(256, 0, 4, 8, 16);
        let command = PwrightNavigation::GotoCommand {
            call_id: 1,
            object_id: 2,
            metadata: PwrightCommonCallMetadata {
                timeout: None,
                cancellation_id: None,
            },
            params: PwrightNavigationGotoParams {
                url: "x".to_owned(),
                referrer: None,
                wait_until: PwrightNavigationWaitUntil::Load,
            },
        };
        let mut invalid_enum = encode_value(&command, limits).expect("command should encode");
        *invalid_enum.last_mut().expect("command has a wait tag") = 99;
        let outcome = PwrightNavigationBeginOutcome::Success {
            result: PwrightNavigationBeginNavigationResult {
                frame_id: String::new(),
                loader_id: None,
                same_document: false,
                is_download: false,
            },
        };
        let mut invalid_bool = encode_value(&outcome, limits).expect("outcome should encode");
        invalid_bool[6] = 2;

        // Act
        let enum_error = decode_value::<PwrightNavigation>(&invalid_enum, limits)
            .expect_err("unknown wait-until tag must fail");
        let bool_error = decode_value::<PwrightNavigationBeginOutcome>(&invalid_bool, limits)
            .expect_err("noncanonical boolean must fail");

        // Assert
        assert!(matches!(
            enum_error,
            BinaryValueDecodeError::InvalidVariantTag {
                type_id: "pwright.navigation.wait_until",
                value: 99,
            }
        ));
        assert!(matches!(
            bool_error,
            BinaryValueDecodeError::InvalidBoolean {
                field: "pwright.navigation.begin_navigation_result.same_document",
                value: 2,
            }
        ));
    }

    #[test]
    fn generated_duration_encoder_rejects_noncanonical_nanoseconds() {
        // Arrange
        let duration = PwrightCommonDuration {
            seconds: 1,
            nanoseconds: 1_000_000_000,
        };
        let limits = BinaryValueLimits::new(0, 0, 0, 4, 4);

        // Act
        let error = encode_value(&duration, limits)
            .expect_err("nanoseconds outside the schema range must fail");

        // Assert
        assert!(matches!(
            error,
            BinaryValueEncodeError::NumericRange(error)
                if error.field == "pwright.common.duration.nanoseconds"
                    && error.value == 1_000_000_000
        ));
    }

    #[test]
    fn successful_results_round_trip_and_headers_become_canonical() {
        // Arrange
        let protocol = protocol();
        let begin = CommandResult::<BeginNavigation>::new(
            CallId::new(1),
            Ok(BeginNavigationResult {
                frame_id: "frame".to_string(),
                loader_id: None,
                same_document: true,
                is_download: false,
            }),
        );
        let goto = CommandResult::<Goto>::new(
            CallId::new(2),
            Ok(GotoResult {
                frame_id: "frame".to_string(),
                loader_id: Some("loader".to_string()),
                url: "https://example.com/final".to_string(),
                same_document: false,
                response: Some(response()),
            }),
        );

        // Act
        let begin = protocol
            .decode::<CommandResult<BeginNavigation>>(
                protocol.encode(&begin).expect("encode begin result"),
            )
            .expect("decode begin result");
        let goto = protocol
            .decode::<CommandResult<Goto>>(protocol.encode(&goto).expect("encode goto result"))
            .expect("decode goto result")
            .into_outcome()
            .expect("goto should succeed");

        // Assert
        assert!(
            begin
                .into_outcome()
                .expect("begin should succeed")
                .same_document
        );
        let headers = goto.response.expect("response should exist").headers;
        assert_eq!(headers[0].name, "content-type");
        assert_eq!(headers[1].name, "x-z");
    }

    #[test]
    fn borrowed_result_encoding_canonicalizes_duplicate_headers_without_losing_items() {
        // Arrange
        let result = CommandResult::<Goto>::new(
            CallId::new(41),
            Ok(GotoResult {
                frame_id: "frame".to_owned(),
                loader_id: None,
                url: "https://example.com".to_owned(),
                same_document: false,
                response: Some(NavigationResponse {
                    url: "https://example.com".to_owned(),
                    status: 200,
                    status_text: "OK".to_owned(),
                    headers: vec![
                        Header {
                            name: "z".to_owned(),
                            value: "last".to_owned(),
                        },
                        Header {
                            name: "a".to_owned(),
                            value: "same".to_owned(),
                        },
                        Header {
                            name: "a".to_owned(),
                            value: "first".to_owned(),
                        },
                        Header {
                            name: "a".to_owned(),
                            value: "same".to_owned(),
                        },
                    ],
                }),
            }),
        );

        // Act
        let decoded = protocol()
            .decode::<CommandResult<Goto>>(protocol().encode(&result).expect("encode result"))
            .expect("decode result")
            .into_outcome()
            .expect("result should succeed");
        let headers = decoded.response.expect("response should exist").headers;

        // Assert
        assert_eq!(
            headers
                .iter()
                .map(|header| (header.name.as_str(), header.value.as_str()))
                .collect::<Vec<_>>(),
            [("a", "first"), ("a", "same"), ("a", "same"), ("z", "last")]
        );
    }

    #[test]
    fn every_error_detail_and_failure_kind_round_trips() {
        // Arrange
        let protocol = protocol();
        let details = [
            NavigationErrorDetails::Network("network".to_string()),
            NavigationErrorDetails::InvalidBackendResponse("invalid".to_string()),
            NavigationErrorDetails::UnsupportedOption("option".to_string()),
            NavigationErrorDetails::Backend("backend".to_string()),
        ];
        let kinds = [
            FailureKind::Timeout,
            FailureKind::Cancelled,
            FailureKind::TargetClosed,
            FailureKind::BrowserCrashed,
            FailureKind::UnsupportedCapability,
            FailureKind::ResourceExhausted,
            FailureKind::TransportClosed,
            FailureKind::ProtocolViolation,
            FailureKind::Backend,
            FailureKind::Cleanup,
        ];

        // Act / Assert
        for (index, kind) in kinds.into_iter().enumerate() {
            let detail = details.get(index % details.len()).cloned();
            let expected = OperationError::new(kind, format!("failure {index}"), detail);
            let result = CommandResult::<Goto>::new(CallId::new(index as u64 + 1), Err(expected));
            let actual = protocol
                .decode::<CommandResult<Goto>>(protocol.encode(&result).expect("encode failure"))
                .expect("decode failure")
                .into_outcome()
                .expect_err("outcome should fail");
            assert_eq!(actual.kind(), kind);
            assert_eq!(actual.message(), format!("failure {index}"));
            assert_eq!(
                actual.details(),
                result.outcome().as_ref().unwrap_err().details()
            );
        }
    }

    #[test]
    fn declared_string_and_header_counts_are_rejected_before_allocation() {
        // Arrange
        let tiny_string = NavigationCodec::new(NavigationCodecLimits::new(3, 4, 256));
        let tiny_headers = NavigationCodec::new(NavigationCodecLimits::new(256, 1, 256));
        let command = Command::<Goto>::new(
            CallId::new(1),
            ObjectId::new(2),
            CallMetadata::default(),
            GotoParams::new("long"),
        );
        let result = CommandResult::<Goto>::new(
            CallId::new(1),
            Ok(GotoResult {
                frame_id: "f".to_string(),
                loader_id: None,
                url: "url".to_string(),
                same_document: false,
                response: Some(response()),
            }),
        );

        // Act
        let string_error = tiny_string.encode_parts(&command);
        let header_error = tiny_headers.encode_parts(&result);

        // Assert
        assert_eq!(
            string_error,
            Err(NavigationCodecError::StringTooLong {
                actual: 4,
                limit: 3
            })
        );
        assert!(matches!(
            header_error,
            Err(NavigationCodecError::TooManyHeaders {
                actual: 2,
                limit: 1
            })
        ));
    }

    #[test]
    fn wrong_routes_methods_kinds_attachments_and_trailing_bytes_fail() {
        // Arrange
        let frame_limits = FrameLimits::new(128, 1, 1, 128);
        let wrong_route = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Control,
            vec![GOTO_COMMAND],
            Vec::new(),
            frame_limits,
        )
        .expect("frame should build");
        let wrong_method = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Command {
                call_id: CallId::new(1),
                target: ObjectId::new(2),
                method: pwright_core::MethodTag::new(99),
            },
            vec![GOTO_COMMAND],
            Vec::new(),
            frame_limits,
        )
        .expect("frame should build");
        let wrong_kind = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Command {
                call_id: CallId::new(1),
                target: ObjectId::new(2),
                method: Goto::METHOD,
            },
            vec![99],
            Vec::new(),
            frame_limits,
        )
        .expect("frame should build");
        let attached = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Command {
                call_id: CallId::new(1),
                target: ObjectId::new(2),
                method: Goto::METHOD,
            },
            vec![GOTO_COMMAND],
            vec![vec![]],
            frame_limits,
        )
        .expect("frame should build");
        let valid = goto_command_frame(GotoParams::new("x"), CallMetadata::default());
        let mut trailing_payload = valid.payload().to_vec();
        trailing_payload.push(0);
        let trailing = rebuild_with_payload(&valid, trailing_payload);

        // Act
        let route =
            <NavigationCodec as WireCodec<Command<Goto>>>::decode_frame(&codec(), wrong_route);
        let method =
            <NavigationCodec as WireCodec<Command<Goto>>>::decode_frame(&codec(), wrong_method);
        let kind =
            <NavigationCodec as WireCodec<Command<Goto>>>::decode_frame(&codec(), wrong_kind);
        let attachment =
            <NavigationCodec as WireCodec<Command<Goto>>>::decode_frame(&codec(), attached);
        let trailing = decode_goto(trailing);

        // Assert
        assert!(matches!(route, Err(NavigationCodecError::WrongRoute(_))));
        assert!(matches!(
            method,
            Err(NavigationCodecError::WrongMethod { .. })
        ));
        assert!(matches!(
            kind,
            Err(NavigationCodecError::WrongPayloadKind {
                expected: GOTO_COMMAND,
                actual: 99
            })
        ));
        assert!(matches!(
            attachment,
            Err(NavigationCodecError::AttachmentsUnsupported { actual: 1 })
        ));
        assert_eq!(
            trailing.err(),
            Some(NavigationCodecError::TrailingBytes { remaining: 1 })
        );
    }

    #[test]
    fn malformed_command_flags_duration_wait_and_utf8_are_rejected() {
        // Arrange
        let plain = goto_command_frame(GotoParams::new("x"), CallMetadata::default());
        let mut option_payload = plain.payload().to_vec();
        option_payload[17] = 2;
        let invalid_option = rebuild_with_payload(&plain, option_payload);
        let mut wait_payload = plain.payload().to_vec();
        *wait_payload.last_mut().expect("wait byte should exist") = 4;
        let invalid_wait = rebuild_with_payload(&plain, wait_payload);
        let mut utf8_payload = plain.payload().to_vec();
        utf8_payload[23] = 0xff;
        let invalid_utf8 = rebuild_with_payload(&plain, utf8_payload);
        let timed = goto_command_frame(
            GotoParams::new("x"),
            CallMetadata::new(Some(Duration::ZERO), None),
        );
        let mut duration_payload = timed.payload().to_vec();
        duration_payload[26..30].copy_from_slice(&1_000_000_000_u32.to_be_bytes());
        let invalid_duration = rebuild_with_payload(&timed, duration_payload);

        // Act
        let option = decode_goto(invalid_option);
        let wait = decode_goto(invalid_wait);
        let utf8 = decode_goto(invalid_utf8);
        let duration = decode_goto(invalid_duration);

        // Assert
        assert_eq!(
            option.err(),
            Some(NavigationCodecError::InvalidOptionFlag(2))
        );
        assert_eq!(wait.err(), Some(NavigationCodecError::InvalidWaitUntil(4)));
        assert!(matches!(utf8, Err(NavigationCodecError::InvalidUtf8(_))));
        assert_eq!(
            duration.err(),
            Some(NavigationCodecError::InvalidDurationNanos(1_000_000_000))
        );
    }

    #[test]
    fn declared_oversized_string_is_rejected_before_missing_bytes_are_read() {
        // Arrange
        let valid = goto_command_frame(GotoParams::new("x"), CallMetadata::default());
        let mut payload = valid.payload()[..23].to_vec();
        payload[19..23].copy_from_slice(&257_u32.to_be_bytes());
        let declared_oversized = rebuild_with_payload(&valid, payload);

        // Act
        let error = decode_goto(declared_oversized);

        // Assert
        assert_eq!(
            error.err(),
            Some(NavigationCodecError::StringTooLong {
                actual: 257,
                limit: 256,
            })
        );
    }

    #[test]
    fn result_discriminants_and_boolean_values_are_canonical() {
        // Arrange
        let successful = protocol()
            .encode(&CommandResult::<BeginNavigation>::new(
                CallId::new(1),
                Ok(BeginNavigationResult {
                    frame_id: "f".to_string(),
                    loader_id: None,
                    same_document: false,
                    is_download: false,
                }),
            ))
            .expect("begin result should encode");
        let mut outcome_payload = successful.payload().to_vec();
        outcome_payload[9] = 2;
        let invalid_outcome = rebuild_with_payload(&successful, outcome_payload);
        let mut boolean_payload = successful.payload().to_vec();
        let boolean_index = boolean_payload.len() - 2;
        boolean_payload[boolean_index] = 2;
        let invalid_boolean = rebuild_with_payload(&successful, boolean_payload);

        // Act
        let outcome = <NavigationCodec as WireCodec<CommandResult<BeginNavigation>>>::decode_frame(
            &codec(),
            invalid_outcome,
        );
        let boolean = <NavigationCodec as WireCodec<CommandResult<BeginNavigation>>>::decode_frame(
            &codec(),
            invalid_boolean,
        );

        // Assert
        assert_eq!(outcome.err(), Some(NavigationCodecError::InvalidOutcome(2)));
        assert_eq!(boolean.err(), Some(NavigationCodecError::InvalidBoolean(2)));
    }

    #[test]
    fn truncated_payload_and_unknown_error_tags_are_rejected() {
        // Arrange
        let valid_command = goto_command_frame(GotoParams::new("x"), CallMetadata::default());
        let truncated = rebuild_with_payload(&valid_command, Vec::new());
        let failure = protocol()
            .encode(&CommandResult::<Goto>::new(
                CallId::new(1),
                Err(OperationError::new(
                    FailureKind::Backend,
                    "m",
                    Some(NavigationErrorDetails::Backend("d".to_string())),
                )),
            ))
            .expect("failure should encode");
        let mut failure_kind_payload = failure.payload().to_vec();
        failure_kind_payload[10] = 10;
        let invalid_failure_kind = rebuild_with_payload(&failure, failure_kind_payload);
        let mut detail_kind_payload = failure.payload().to_vec();
        detail_kind_payload[17] = 4;
        let invalid_detail_kind = rebuild_with_payload(&failure, detail_kind_payload);

        // Act
        let truncated = decode_goto(truncated);
        let failure_kind = <NavigationCodec as WireCodec<CommandResult<Goto>>>::decode_frame(
            &codec(),
            invalid_failure_kind,
        );
        let detail_kind = <NavigationCodec as WireCodec<CommandResult<Goto>>>::decode_frame(
            &codec(),
            invalid_detail_kind,
        );

        // Assert
        assert!(matches!(
            truncated,
            Err(NavigationCodecError::Truncated { .. })
        ));
        assert_eq!(
            failure_kind.err(),
            Some(NavigationCodecError::InvalidFailureKind(10))
        );
        assert_eq!(
            detail_kind.err(),
            Some(NavigationCodecError::InvalidErrorDetails(4))
        );
    }

    #[test]
    fn response_header_decode_limits_and_canonical_order_are_enforced() {
        // Arrange
        let result = CommandResult::<Goto>::new(
            CallId::new(1),
            Ok(GotoResult {
                frame_id: "f".to_string(),
                loader_id: None,
                url: "u".to_string(),
                same_document: false,
                response: Some(NavigationResponse {
                    url: "u".to_string(),
                    status: 200,
                    status_text: "OK".to_string(),
                    headers: vec![
                        Header {
                            name: "z-header".to_string(),
                            value: "z-value".to_string(),
                        },
                        Header {
                            name: "a-header".to_string(),
                            value: "a-value".to_string(),
                        },
                    ],
                }),
            }),
        );
        let encoded = protocol()
            .encode(&result)
            .expect("goto result should encode");
        let too_few_headers = NavigationCodec::new(NavigationCodecLimits::new(256, 1, 256));
        let too_few_bytes = NavigationCodec::new(NavigationCodecLimits::new(256, 4, 1));
        let mut unordered_payload = encoded.payload().to_vec();
        let a_start = unordered_payload
            .windows(b"a-header".len())
            .position(|window| window == b"a-header")
            .expect("canonical first header should exist");
        let z_start = unordered_payload
            .windows(b"z-header".len())
            .position(|window| window == b"z-header")
            .expect("canonical second header should exist");
        for offset in 0..b"a-header".len() {
            unordered_payload.swap(a_start + offset, z_start + offset);
        }
        let unordered = rebuild_with_payload(&encoded, unordered_payload);

        // Act
        let count = <NavigationCodec as WireCodec<CommandResult<Goto>>>::decode_frame(
            &too_few_headers,
            rebuild_with_payload(&encoded, encoded.payload().to_vec()),
        );
        let bytes = <NavigationCodec as WireCodec<CommandResult<Goto>>>::decode_frame(
            &too_few_bytes,
            rebuild_with_payload(&encoded, encoded.payload().to_vec()),
        );
        let order =
            <NavigationCodec as WireCodec<CommandResult<Goto>>>::decode_frame(&codec(), unordered);

        // Assert
        assert!(matches!(
            count,
            Err(NavigationCodecError::TooManyHeaders {
                actual: 2,
                limit: 1
            })
        ));
        assert!(matches!(
            bytes,
            Err(NavigationCodecError::HeaderBytesTooLarge { limit: 1, .. })
        ));
        assert_eq!(
            order.err(),
            Some(NavigationCodecError::NonCanonicalHeaderOrder)
        );
    }
}
