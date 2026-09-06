//! Canonical bounded binary codec for owned Browser lifecycle commands.
//!
//! Browser close is a lifecycle barrier, so timeout and cancellation metadata
//! are rejected. Process-shutdown policy remains server-side and never enters
//! the portable wire contract.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use pwright_cap_browser_lifecycle::{
    BrowserClose, BrowserCloseOptions, BrowserLifecycleErrorDetails,
};
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

#[doc(hidden)]
#[rustfmt::skip]
pub mod generated_schema;

pub use generated_schema::{
    CATALOG as BROWSER_LIFECYCLE_SCHEMA_CATALOG, COMMANDS as BROWSER_LIFECYCLE_COMMAND_SCHEMAS,
    ROOT_SCHEMA as BROWSER_LIFECYCLE_SCHEMA,
};

use generated_schema::{
    PwrightBrowserLifecycle, PwrightBrowserLifecycleBrowserCloseOptionsRef,
    PwrightBrowserLifecycleError, PwrightBrowserLifecycleErrorDetailsRef,
    PwrightBrowserLifecycleErrorRef, PwrightBrowserLifecycleOutcome,
    PwrightBrowserLifecycleOutcomeRef, PwrightBrowserLifecycleRef, PwrightCommonCallMetadata,
    PwrightCommonFailureKind,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BrowserLifecycleCodecLimits {
    pub string_bytes: usize,
}

impl BrowserLifecycleCodecLimits {
    pub const fn new(string_bytes: usize) -> Self {
        Self { string_bytes }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BrowserLifecycleCodec {
    limits: BrowserLifecycleCodecLimits,
}

impl BrowserLifecycleCodec {
    pub const fn new(limits: BrowserLifecycleCodecLimits) -> Self {
        Self { limits }
    }

    pub const fn limits(self) -> BrowserLifecycleCodecLimits {
        self.limits
    }

    fn value_limits(self) -> BinaryValueLimits {
        BinaryValueLimits::new(self.limits.string_bytes, 0, 0, 8, 16)
    }
}

#[derive(Debug)]
pub enum BrowserLifecycleCodecError {
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

impl Display for BrowserLifecycleCodecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongRoute(route) => {
                write!(formatter, "Browser lifecycle payload arrived on {route:?}")
            }
            Self::WrongMethod { expected, actual } => write!(
                formatter,
                "expected Browser lifecycle method {expected}, received {actual}"
            ),
            Self::IdentityMismatch {
                field,
                outer,
                inner,
            } => write!(
                formatter,
                "Browser lifecycle payload {field} {inner} does not match outer route {outer}"
            ),
            Self::AttachmentsUnsupported { actual } => write!(
                formatter,
                "Browser lifecycle payload has {actual} attachment(s), but none are allowed"
            ),
            Self::ControlMetadataUnsupported => formatter
                .write_str("Browser.close does not accept timeout or cancellation metadata"),
            Self::WrongPayloadKind { expected, actual } => write!(
                formatter,
                "expected Browser lifecycle payload kind {expected}, received {actual}"
            ),
            Self::Encode(error) => Display::fmt(error, formatter),
            Self::Decode(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for BrowserLifecycleCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Encode(error) => Some(error),
            Self::Decode(error) => Some(error),
            _ => None,
        }
    }
}

impl WireCodec<Command<BrowserClose>> for BrowserLifecycleCodec {
    type Error = BrowserLifecycleCodecError;

    fn encode_parts(&self, command: &Command<BrowserClose>) -> Result<EncodedParts, Self::Error> {
        ensure_empty_metadata(command.metadata())?;
        let wire = PwrightBrowserLifecycleRef::BrowserCloseCommand {
            call_id: command.call_id().get(),
            object_id: command.object_id().get(),
            metadata: empty_metadata(),
            options: PwrightBrowserLifecycleBrowserCloseOptionsRef {
                reason: command.params().reason(),
            },
        };
        let payload =
            encode_value(&wire, self.value_limits()).map_err(BrowserLifecycleCodecError::Encode)?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: BrowserClose::METHOD,
            },
            payload,
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<BrowserClose>, Self::Error> {
        let (outer_call_id, outer_object_id) = command_route::<BrowserClose>(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightBrowserLifecycle>(frame.payload(), self.value_limits())
            .map_err(BrowserLifecycleCodecError::Decode)?
        {
            PwrightBrowserLifecycle::BrowserCloseCommand {
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
                    BrowserCloseOptions::new(options.reason),
                ))
            }
            other => Err(BrowserLifecycleCodecError::WrongPayloadKind {
                expected: CLOSE_COMMAND,
                actual: wire_kind(&other),
            }),
        }
    }
}

impl WireCodec<CommandResult<BrowserClose>> for BrowserLifecycleCodec {
    type Error = BrowserLifecycleCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<BrowserClose>,
    ) -> Result<EncodedParts, Self::Error> {
        let outcome = match result.outcome() {
            Ok(()) => PwrightBrowserLifecycleOutcomeRef::Success,
            Err(error) => PwrightBrowserLifecycleOutcomeRef::Error {
                error: operation_error_to_wire_ref(error),
            },
        };
        let wire = PwrightBrowserLifecycleRef::BrowserCloseResult {
            call_id: result.call_id().get(),
            outcome,
        };
        let payload =
            encode_value(&wire, self.value_limits()).map_err(BrowserLifecycleCodecError::Encode)?;
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload,
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<BrowserClose>, Self::Error> {
        let outer_call_id = result_route(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightBrowserLifecycle>(frame.payload(), self.value_limits())
            .map_err(BrowserLifecycleCodecError::Decode)?
        {
            PwrightBrowserLifecycle::BrowserCloseResult { call_id, outcome } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                let outcome = match outcome {
                    PwrightBrowserLifecycleOutcome::Success => Ok(()),
                    PwrightBrowserLifecycleOutcome::Error { error } => {
                        Err(operation_error_from_wire(error))
                    }
                };
                Ok(CommandResult::new(CallId::new(call_id), outcome))
            }
            other => Err(BrowserLifecycleCodecError::WrongPayloadKind {
                expected: CLOSE_RESULT,
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

fn ensure_empty_metadata(metadata: CallMetadata) -> Result<(), BrowserLifecycleCodecError> {
    if metadata == CallMetadata::default() {
        Ok(())
    } else {
        Err(BrowserLifecycleCodecError::ControlMetadataUnsupported)
    }
}

fn ensure_no_attachments(frame: &WireFrame) -> Result<(), BrowserLifecycleCodecError> {
    if frame.attachments().is_empty() {
        Ok(())
    } else {
        Err(BrowserLifecycleCodecError::AttachmentsUnsupported {
            actual: frame.attachments().len(),
        })
    }
}

fn ensure_identity(
    field: &'static str,
    outer: u64,
    inner: u64,
) -> Result<(), BrowserLifecycleCodecError> {
    if outer == inner {
        Ok(())
    } else {
        Err(BrowserLifecycleCodecError::IdentityMismatch {
            field,
            outer,
            inner,
        })
    }
}

fn command_route<S: CommandSpec>(
    frame: &WireFrame,
) -> Result<(CallId, ObjectId), BrowserLifecycleCodecError> {
    let FrameRoute::Command {
        call_id,
        target,
        method,
    } = frame.route()
    else {
        return Err(BrowserLifecycleCodecError::WrongRoute(frame.route()));
    };
    if method != S::METHOD {
        return Err(BrowserLifecycleCodecError::WrongMethod {
            expected: S::METHOD,
            actual: method,
        });
    }
    Ok((call_id, target))
}

fn result_route(frame: &WireFrame) -> Result<CallId, BrowserLifecycleCodecError> {
    let FrameRoute::Result { call_id } = frame.route() else {
        return Err(BrowserLifecycleCodecError::WrongRoute(frame.route()));
    };
    Ok(call_id)
}

fn wire_kind(value: &PwrightBrowserLifecycle) -> u8 {
    match value {
        PwrightBrowserLifecycle::BrowserCloseCommand { .. } => CLOSE_COMMAND,
        PwrightBrowserLifecycle::BrowserCloseResult { .. } => CLOSE_RESULT,
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
    details: &BrowserLifecycleErrorDetails,
) -> PwrightBrowserLifecycleErrorDetailsRef<'_> {
    match details {
        BrowserLifecycleErrorDetails::NotAuthorized(message) => {
            PwrightBrowserLifecycleErrorDetailsRef::NotAuthorized { message }
        }
        BrowserLifecycleErrorDetails::Shutdown(message) => {
            PwrightBrowserLifecycleErrorDetailsRef::Shutdown { message }
        }
        BrowserLifecycleErrorDetails::Cleanup(message) => {
            PwrightBrowserLifecycleErrorDetailsRef::Cleanup { message }
        }
    }
}

fn operation_error_to_wire_ref(
    error: &OperationError<BrowserLifecycleErrorDetails>,
) -> PwrightBrowserLifecycleErrorRef<'_> {
    PwrightBrowserLifecycleErrorRef {
        failure_kind: failure_kind_to_wire(error.kind()),
        message: error.message(),
        details: error.details().map(error_details_to_wire_ref),
    }
}

fn operation_error_from_wire(
    error: PwrightBrowserLifecycleError,
) -> OperationError<BrowserLifecycleErrorDetails> {
    OperationError::new(
        failure_kind_from_wire(error.failure_kind),
        error.message,
        error.details.map(Into::into),
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use pwright_cap_browser_lifecycle::{
        BrowserClose, BrowserCloseOptions, BrowserLifecycleErrorDetails,
    };
    use pwright_core::{
        CallId, CallMetadata, Command, CommandResult, FailureKind, ObjectId, OperationError,
        ProtocolVersion,
    };
    use pwright_transport::{FrameLimits, VersionPolicy, WireCodec, WireProtocol};

    use super::{BrowserLifecycleCodec, BrowserLifecycleCodecError, BrowserLifecycleCodecLimits};

    fn protocol() -> WireProtocol<BrowserLifecycleCodec> {
        WireProtocol::new(
            BrowserLifecycleCodec::new(BrowserLifecycleCodecLimits::new(256)),
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0)).unwrap(),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(1024, 0, 0, 1024),
        )
        .unwrap()
    }

    #[test]
    fn close_command_round_trips_without_control_metadata() {
        // Arrange
        let command = Command::<BrowserClose>::new(
            CallId::new(7),
            ObjectId::new(1),
            CallMetadata::default(),
            BrowserCloseOptions::with_reason("maintenance window"),
        );

        // Act
        let frame = protocol().encode(&command).unwrap();
        let decoded = protocol().decode::<Command<BrowserClose>>(frame).unwrap();

        // Assert
        assert_eq!(decoded.call_id(), CallId::new(7));
        assert_eq!(decoded.object_id(), ObjectId::new(1));
        assert_eq!(decoded.params().reason(), Some("maintenance window"));
    }

    #[test]
    fn shutdown_failure_details_round_trip() {
        // Arrange
        let result = CommandResult::<BrowserClose>::new(
            CallId::new(9),
            Err(OperationError::new(
                FailureKind::BrowserCrashed,
                "shutdown failed",
                Some(BrowserLifecycleErrorDetails::Shutdown(
                    "process lost".into(),
                )),
            )),
        );

        // Act
        let frame = protocol().encode(&result).unwrap();
        let decoded = protocol()
            .decode::<CommandResult<BrowserClose>>(frame)
            .unwrap();

        // Assert
        assert_eq!(decoded.call_id(), result.call_id());
        assert_eq!(decoded.outcome(), result.outcome());
    }

    #[test]
    fn close_rejects_timeout_metadata() {
        // Arrange
        let command = Command::<BrowserClose>::new(
            CallId::new(11),
            ObjectId::new(1),
            CallMetadata::new(Some(Duration::from_secs(1)), None),
            BrowserCloseOptions::default(),
        );

        // Act
        let error = <BrowserLifecycleCodec as WireCodec<Command<BrowserClose>>>::encode_parts(
            &BrowserLifecycleCodec::new(BrowserLifecycleCodecLimits::new(256)),
            &command,
        )
        .unwrap_err();

        // Assert
        assert!(matches!(
            error,
            BrowserLifecycleCodecError::ControlMetadataUnsupported
        ));
    }
}
