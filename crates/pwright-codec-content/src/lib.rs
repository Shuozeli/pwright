//! Canonical bounded binary codec for Pwright content capture.
//!
//! Screenshot bytes travel as one attachment. Decoding copies them into a
//! separately budgeted result so releasing the wire frame cannot invalidate
//! the user-visible bytes.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

use pwright_cap_content::{
    ContentErrorDetails, PageScreenshot, ScreenshotClip, ScreenshotFormat, ScreenshotOptions,
    ScreenshotScale,
};
use pwright_codec_binary::{
    BinaryValueDecodeError, BinaryValueEncodeError, BinaryValueLimits, decode_value, encode_value,
};
use pwright_codec_screenshot::{
    ScreenshotAttachmentError, ScreenshotAttachmentMetadata, decode_screenshot_attachment,
    encode_screenshot_attachment,
};
use pwright_core::{
    CallId, CallMetadata, CancellationId, Command, CommandResult, CommandSpec, FailureKind,
    MethodTag, ObjectId, OperationError, ResourceError, ResourceLedger,
};
use pwright_transport::{EncodedParts, FrameRoute, WireCodec, WireFrame};

const SCREENSHOT_COMMAND: u8 = 1;
const SCREENSHOT_RESULT: u8 = 2;

#[doc(hidden)]
#[rustfmt::skip]
pub mod generated_schema;

pub use generated_schema::{
    CATALOG as CONTENT_SCHEMA_CATALOG, COMMANDS as CONTENT_COMMAND_SCHEMAS,
    ROOT_SCHEMA as CONTENT_SCHEMA,
};

use generated_schema::{
    PwrightCommonCallMetadata, PwrightCommonDuration, PwrightCommonFailureKind, PwrightContent,
    PwrightContentError, PwrightContentErrorDetailsRef, PwrightContentErrorRef,
    PwrightContentOutcome, PwrightContentOutcomeRef, PwrightContentRef,
    PwrightContentScreenshotClip, PwrightContentScreenshotData, PwrightContentScreenshotFormat,
    PwrightContentScreenshotOptions, PwrightContentScreenshotScale,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContentCodecLimits {
    pub screenshot_bytes: usize,
    pub error_string_bytes: usize,
}

impl ContentCodecLimits {
    pub const fn new(screenshot_bytes: usize, error_string_bytes: usize) -> Self {
        Self {
            screenshot_bytes,
            error_string_bytes,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ContentCodec {
    limits: ContentCodecLimits,
    result_ledger: ResourceLedger,
}

impl ContentCodec {
    pub const fn new(limits: ContentCodecLimits, result_ledger: ResourceLedger) -> Self {
        Self {
            limits,
            result_ledger,
        }
    }

    pub const fn limits(&self) -> ContentCodecLimits {
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
pub enum ContentCodecError {
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
    MissingScreenshotAttachment,
    InvalidAttachmentIndex(u32),
    AttachmentLengthMismatch {
        declared: u64,
        actual: usize,
    },
    ScreenshotTooLarge {
        actual: usize,
        limit: usize,
    },
    InvalidOptions(&'static str),
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

impl Display for ContentCodecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongRoute(route) => write!(formatter, "content payload arrived on {route:?}"),
            Self::WrongMethod { expected, actual } => {
                write!(
                    formatter,
                    "expected content method {expected}, received {actual}"
                )
            }
            Self::IdentityMismatch {
                field,
                outer,
                inner,
            } => write!(
                formatter,
                "content payload {field} {inner} does not match outer route {outer}"
            ),
            Self::UnexpectedAttachments { actual } => write!(
                formatter,
                "content payload has {actual} attachment(s) where none are allowed"
            ),
            Self::MissingScreenshotAttachment => {
                formatter.write_str("successful screenshot result has no attachment")
            }
            Self::InvalidAttachmentIndex(index) => {
                write!(formatter, "screenshot attachment index {index} is not zero")
            }
            Self::AttachmentLengthMismatch { declared, actual } => write!(
                formatter,
                "screenshot declares {declared} bytes but attachment contains {actual}"
            ),
            Self::ScreenshotTooLarge { actual, limit } => write!(
                formatter,
                "screenshot has {actual} bytes, exceeding limit {limit}"
            ),
            Self::InvalidOptions(message) => formatter.write_str(message),
            Self::ErrorStringTooLong { actual, limit } => write!(
                formatter,
                "content error string has {actual} bytes, exceeding limit {limit}"
            ),
            Self::WrongPayloadKind { expected, actual } => write!(
                formatter,
                "expected content payload kind {expected}, received {actual}"
            ),
            Self::Resource(error) => Display::fmt(error, formatter),
            Self::Encode(error) => Display::fmt(error, formatter),
            Self::Decode(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ContentCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resource(error) => Some(error),
            Self::Encode(error) => Some(error),
            Self::Decode(error) => Some(error),
            _ => None,
        }
    }
}

impl WireCodec<Command<PageScreenshot>> for ContentCodec {
    type Error = ContentCodecError;

    fn encode_parts(&self, command: &Command<PageScreenshot>) -> Result<EncodedParts, Self::Error> {
        validate_options(command.params())?;
        let wire = PwrightContentRef::PageScreenshotCommand {
            call_id: command.call_id().get(),
            object_id: command.object_id().get(),
            metadata: metadata_to_wire(command.metadata()),
            options: options_to_wire(*command.params()),
        };
        let payload =
            encode_value(&wire, self.value_limits()).map_err(ContentCodecError::Encode)?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: PageScreenshot::METHOD,
            },
            payload,
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<PageScreenshot>, Self::Error> {
        let (outer_call_id, outer_object_id) = command_route::<PageScreenshot>(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightContent>(frame.payload(), self.value_limits())
            .map_err(ContentCodecError::Decode)?
        {
            PwrightContent::PageScreenshotCommand {
                call_id,
                object_id,
                metadata,
                options,
            } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                ensure_identity("object ID", outer_object_id.get(), object_id)?;
                let options = options_from_wire(options);
                validate_options(&options)?;
                Ok(Command::new(
                    CallId::new(call_id),
                    ObjectId::new(object_id),
                    metadata_from_wire(metadata),
                    options,
                ))
            }
            other => Err(ContentCodecError::WrongPayloadKind {
                expected: SCREENSHOT_COMMAND,
                actual: wire_kind(&other),
            }),
        }
    }
}

impl WireCodec<CommandResult<PageScreenshot>> for ContentCodec {
    type Error = ContentCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<PageScreenshot>,
    ) -> Result<EncodedParts, Self::Error> {
        let mut attachments = Vec::new();
        let outcome = match result.outcome() {
            Ok(result) => {
                let (metadata, attachment) =
                    encode_screenshot_attachment(result, self.limits.screenshot_bytes)
                        .map_err(map_attachment_error)?;
                attachments.push(attachment);
                PwrightContentOutcomeRef::Success {
                    result: PwrightContentScreenshotData {
                        attachment_index: metadata.index,
                        byte_length: metadata.byte_length,
                        format: format_to_wire(metadata.format),
                    },
                }
            }
            Err(error) => {
                validate_operation_error(error, self.limits.error_string_bytes)?;
                PwrightContentOutcomeRef::Error {
                    error: operation_error_to_wire_ref(error),
                }
            }
        };
        let wire = PwrightContentRef::PageScreenshotResult {
            call_id: result.call_id().get(),
            outcome,
        };
        let payload =
            encode_value(&wire, self.value_limits()).map_err(ContentCodecError::Encode)?;
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload,
            attachments,
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<PageScreenshot>, Self::Error> {
        let outer_call_id = result_route(&frame)?;
        match decode_value::<PwrightContent>(frame.payload(), self.value_limits())
            .map_err(ContentCodecError::Decode)?
        {
            PwrightContent::PageScreenshotResult { call_id, outcome } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                let outcome = match outcome {
                    PwrightContentOutcome::Success { result } => {
                        let screenshot = decode_screenshot_attachment(
                            &frame,
                            ScreenshotAttachmentMetadata {
                                index: result.attachment_index,
                                byte_length: result.byte_length,
                                format: format_from_wire(result.format),
                            },
                            self.limits.screenshot_bytes,
                            &self.result_ledger,
                            CallId::new(call_id),
                        )
                        .map_err(map_attachment_error)?;
                        Ok(screenshot)
                    }
                    PwrightContentOutcome::Error { error } => {
                        ensure_no_attachments(&frame)?;
                        let error = operation_error_from_wire(error);
                        validate_operation_error(&error, self.limits.error_string_bytes)?;
                        Err(error)
                    }
                };
                Ok(CommandResult::new(CallId::new(call_id), outcome))
            }
            other => Err(ContentCodecError::WrongPayloadKind {
                expected: SCREENSHOT_RESULT,
                actual: wire_kind(&other),
            }),
        }
    }
}

fn validate_options(options: &ScreenshotOptions) -> Result<(), ContentCodecError> {
    let format = options.format.unwrap_or_default();
    if options.quality.is_some() && format == ScreenshotFormat::Png {
        return Err(ContentCodecError::InvalidOptions(
            "screenshot quality is only valid for JPEG or WebP",
        ));
    }
    if options.full_page.unwrap_or(false) && options.clip.is_some() {
        return Err(ContentCodecError::InvalidOptions(
            "full-page screenshot and clip are mutually exclusive",
        ));
    }
    if let Some(clip) = options.clip
        && (!clip.x.is_finite()
            || !clip.y.is_finite()
            || !clip.width.is_finite()
            || !clip.height.is_finite()
            || clip.x < 0.0
            || clip.y < 0.0
            || clip.width <= 0.0
            || clip.height <= 0.0)
    {
        return Err(ContentCodecError::InvalidOptions(
            "screenshot clip must be finite, non-negative, and have positive dimensions",
        ));
    }
    Ok(())
}

fn map_attachment_error(error: ScreenshotAttachmentError) -> ContentCodecError {
    match error {
        ScreenshotAttachmentError::InvalidIndex(index) => {
            ContentCodecError::InvalidAttachmentIndex(index)
        }
        ScreenshotAttachmentError::Missing => ContentCodecError::MissingScreenshotAttachment,
        ScreenshotAttachmentError::UnexpectedCount { actual } => {
            ContentCodecError::UnexpectedAttachments { actual }
        }
        ScreenshotAttachmentError::LengthMismatch { declared, actual } => {
            ContentCodecError::AttachmentLengthMismatch { declared, actual }
        }
        ScreenshotAttachmentError::InvalidLength { actual, limit } => {
            ContentCodecError::ScreenshotTooLarge { actual, limit }
        }
        ScreenshotAttachmentError::Resource(error) => ContentCodecError::Resource(error),
    }
}

fn options_to_wire(options: ScreenshotOptions) -> PwrightContentScreenshotOptions {
    PwrightContentScreenshotOptions {
        format: options.format.map(format_to_wire),
        quality: options.quality,
        full_page: options.full_page,
        clip: options.clip.map(clip_to_wire),
        scale: options.scale.map(scale_to_wire),
    }
}

fn options_from_wire(options: PwrightContentScreenshotOptions) -> ScreenshotOptions {
    ScreenshotOptions {
        format: options.format.map(format_from_wire),
        quality: options.quality,
        full_page: options.full_page,
        clip: options.clip.map(clip_from_wire),
        scale: options.scale.map(scale_from_wire),
    }
}

fn clip_to_wire(clip: ScreenshotClip) -> PwrightContentScreenshotClip {
    PwrightContentScreenshotClip {
        x: clip.x,
        y: clip.y,
        width: clip.width,
        height: clip.height,
    }
}

fn clip_from_wire(clip: PwrightContentScreenshotClip) -> ScreenshotClip {
    ScreenshotClip {
        x: clip.x,
        y: clip.y,
        width: clip.width,
        height: clip.height,
    }
}

fn format_to_wire(format: ScreenshotFormat) -> PwrightContentScreenshotFormat {
    match format {
        ScreenshotFormat::Png => PwrightContentScreenshotFormat::Png,
        ScreenshotFormat::Jpeg => PwrightContentScreenshotFormat::Jpeg,
        ScreenshotFormat::Webp => PwrightContentScreenshotFormat::Webp,
    }
}

fn format_from_wire(format: PwrightContentScreenshotFormat) -> ScreenshotFormat {
    match format {
        PwrightContentScreenshotFormat::Png => ScreenshotFormat::Png,
        PwrightContentScreenshotFormat::Jpeg => ScreenshotFormat::Jpeg,
        PwrightContentScreenshotFormat::Webp => ScreenshotFormat::Webp,
    }
}

fn scale_to_wire(scale: ScreenshotScale) -> PwrightContentScreenshotScale {
    match scale {
        ScreenshotScale::Css => PwrightContentScreenshotScale::Css,
        ScreenshotScale::Device => PwrightContentScreenshotScale::Device,
    }
}

fn scale_from_wire(scale: PwrightContentScreenshotScale) -> ScreenshotScale {
    match scale {
        PwrightContentScreenshotScale::Css => ScreenshotScale::Css,
        PwrightContentScreenshotScale::Device => ScreenshotScale::Device,
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

fn error_details_to_wire_ref(details: &ContentErrorDetails) -> PwrightContentErrorDetailsRef<'_> {
    match details {
        ContentErrorDetails::InvalidOptions(message) => {
            PwrightContentErrorDetailsRef::InvalidOptions { message }
        }
        ContentErrorDetails::InvalidBackendResponse(message) => {
            PwrightContentErrorDetailsRef::InvalidBackendResponse { message }
        }
        ContentErrorDetails::Resource(message) => {
            PwrightContentErrorDetailsRef::Resource { message }
        }
        ContentErrorDetails::Backend(message) => PwrightContentErrorDetailsRef::Backend { message },
    }
}

fn operation_error_to_wire_ref(
    error: &OperationError<ContentErrorDetails>,
) -> PwrightContentErrorRef<'_> {
    PwrightContentErrorRef {
        failure_kind: failure_kind_to_wire(error.kind()),
        message: error.message(),
        details: error.details().map(error_details_to_wire_ref),
    }
}

fn operation_error_from_wire(error: PwrightContentError) -> OperationError<ContentErrorDetails> {
    OperationError::new(
        failure_kind_from_wire(error.failure_kind),
        error.message,
        error.details.map(Into::into),
    )
}

fn validate_operation_error(
    error: &OperationError<ContentErrorDetails>,
    limit: usize,
) -> Result<(), ContentCodecError> {
    validate_error_string(error.message(), limit)?;
    if let Some(details) = error.details() {
        let message = match details {
            ContentErrorDetails::InvalidOptions(message)
            | ContentErrorDetails::InvalidBackendResponse(message)
            | ContentErrorDetails::Resource(message)
            | ContentErrorDetails::Backend(message) => message,
        };
        validate_error_string(message, limit)?;
    }
    Ok(())
}

fn validate_error_string(value: &str, limit: usize) -> Result<(), ContentCodecError> {
    if value.len() <= limit {
        Ok(())
    } else {
        Err(ContentCodecError::ErrorStringTooLong {
            actual: value.len(),
            limit,
        })
    }
}

fn ensure_no_attachments(frame: &WireFrame) -> Result<(), ContentCodecError> {
    if frame.attachments().is_empty() {
        Ok(())
    } else {
        Err(ContentCodecError::UnexpectedAttachments {
            actual: frame.attachments().len(),
        })
    }
}

fn ensure_identity(field: &'static str, outer: u64, inner: u64) -> Result<(), ContentCodecError> {
    if outer == inner {
        Ok(())
    } else {
        Err(ContentCodecError::IdentityMismatch {
            field,
            outer,
            inner,
        })
    }
}

fn command_route<S: CommandSpec>(
    frame: &WireFrame,
) -> Result<(CallId, ObjectId), ContentCodecError> {
    let FrameRoute::Command {
        call_id,
        target,
        method,
    } = frame.route()
    else {
        return Err(ContentCodecError::WrongRoute(frame.route()));
    };
    if method != S::METHOD {
        return Err(ContentCodecError::WrongMethod {
            expected: S::METHOD,
            actual: method,
        });
    }
    Ok((call_id, target))
}

fn result_route(frame: &WireFrame) -> Result<CallId, ContentCodecError> {
    let FrameRoute::Result { call_id } = frame.route() else {
        return Err(ContentCodecError::WrongRoute(frame.route()));
    };
    Ok(call_id)
}

fn wire_kind(value: &PwrightContent) -> u8 {
    match value {
        PwrightContent::PageScreenshotCommand { .. } => SCREENSHOT_COMMAND,
        PwrightContent::PageScreenshotResult { .. } => SCREENSHOT_RESULT,
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

    use pwright_cap_content::{PageScreenshot, ScreenshotFormat, ScreenshotOptions};
    use pwright_core::{
        BudgetedBytes, CallId, CallMetadata, CancellationId, Command, CommandResult, ObjectId,
        ProtocolVersion, ResourceClass, ResourceLedger, ResourceLimits, ResourceScope,
    };
    use pwright_transport::{FrameLimits, WireCodec, WireFrame, WireProtocol};

    use super::{ContentCodec, ContentCodecError, ContentCodecLimits};

    fn ledger(bytes: u64) -> ResourceLedger {
        ResourceLedger::new(
            ResourceLimits::deny_all(8, 8)
                .with_class(ResourceClass::ScreenshotBytes, bytes, bytes)
                .with_class(
                    ResourceClass::WireBytes,
                    bytes.saturating_mul(4),
                    bytes.saturating_mul(4),
                ),
        )
    }

    fn codec(ledger: &ResourceLedger) -> ContentCodec {
        ContentCodec::new(ContentCodecLimits::new(32, 256), ledger.clone())
    }

    #[test]
    fn screenshot_command_round_trips_extensible_options_and_metadata() {
        // Arrange
        let ledger = ledger(64);
        let codec = codec(&ledger);
        let command = Command::<PageScreenshot>::new(
            CallId::new(7),
            ObjectId::new(9),
            CallMetadata::new(
                Some(Duration::from_millis(25)),
                Some(CancellationId::new(11)),
            ),
            ScreenshotOptions {
                format: Some(ScreenshotFormat::Jpeg),
                quality: Some(80),
                full_page: Some(true),
                ..ScreenshotOptions::default()
            },
        );

        // Act
        let parts = codec.encode_parts(&command).expect("command should encode");
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 0),
            parts.route(),
            parts.payload().to_vec(),
            parts.attachments().to_vec(),
            FrameLimits::new(1024, 1, 32, 1056),
        )
        .expect("frame should build");
        let decoded: Command<PageScreenshot> =
            codec.decode_frame(frame).expect("command should decode");

        // Assert
        assert_eq!(decoded.call_id(), CallId::new(7));
        assert_eq!(decoded.object_id(), ObjectId::new(9));
        assert_eq!(decoded.metadata(), command.metadata());
        assert_eq!(decoded.params(), command.params());
    }

    #[test]
    fn result_attachment_is_copied_into_screenshot_budget_until_drop() {
        // Arrange
        let server_ledger = ledger(64);
        let client_ledger = ledger(64);
        let bytes = BudgetedBytes::copy_from_slice(
            &server_ledger,
            ResourceScope::Call(CallId::new(3)),
            ResourceClass::ScreenshotBytes,
            b"png-data",
        )
        .unwrap();
        let result = CommandResult::<PageScreenshot>::new(
            CallId::new(3),
            Ok(pwright_cap_content::ScreenshotData::new(
                bytes,
                ScreenshotFormat::Png,
            )),
        );
        let server_codec = codec(&server_ledger);
        let client_codec = codec(&client_ledger);
        let protocol = WireProtocol::new_exact_directional(
            server_codec,
            ProtocolVersion::new(1, 0),
            FrameLimits::new(1024, 1, 32, 1056),
            FrameLimits::new(1024, 1, 32, 1056),
        );

        // Act
        let frame = protocol.encode(&result).expect("result should encode");
        let decoded: CommandResult<PageScreenshot> = client_codec
            .decode_frame(frame)
            .expect("result should decode");

        // Assert
        let screenshot = decoded.outcome().as_ref().expect("result should succeed");
        assert_eq!(screenshot.as_bytes(), b"png-data");
        assert_eq!(screenshot.format(), ScreenshotFormat::Png);
        assert_eq!(
            client_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ScreenshotBytes)
                .current,
            8
        );
        drop(decoded);
        assert_eq!(
            client_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ScreenshotBytes)
                .current,
            0
        );
    }

    #[test]
    fn invalid_png_quality_fails_before_encoding() {
        // Arrange
        let ledger = ledger(64);
        let codec = codec(&ledger);
        let command = Command::<PageScreenshot>::new(
            CallId::new(1),
            ObjectId::new(2),
            CallMetadata::default(),
            ScreenshotOptions {
                quality: Some(90),
                ..ScreenshotOptions::default()
            },
        );

        // Act
        let result = codec.encode_parts(&command);

        // Assert
        assert!(matches!(result, Err(ContentCodecError::InvalidOptions(_))));
    }
}
