//! Canonical bounded wire codec for the optional Locator screenshot capability.
//!
//! Selector recipes are imported from `pwright.selector`; screenshot bytes use
//! one attachment and are copied into a separately budgeted result lease.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;

use pwright_cap_locator_screenshot::{
    LocatorScreenshot, LocatorScreenshotErrorDetails, LocatorScreenshotParams, ScreenshotFormat,
    ScreenshotOptions, ScreenshotScale,
};
use pwright_codec_binary::{
    BinaryValueDecodeError, BinaryValueEncodeError, BinaryValueLimits, decode_value, encode_value,
};
use pwright_codec_screenshot::{
    ScreenshotAttachmentError, ScreenshotAttachmentMetadata, decode_screenshot_attachment,
    encode_screenshot_attachment,
};
use pwright_codec_selector::{SelectorModelDecodeError, selector_from_wire, selector_to_wire};
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
    CATALOG as LOCATOR_SCREENSHOT_SCHEMA_CATALOG, COMMANDS as LOCATOR_SCREENSHOT_COMMAND_SCHEMAS,
    ROOT_SCHEMA as LOCATOR_SCREENSHOT_SCHEMA,
};

use generated_schema::{
    PwrightCommonCallMetadata, PwrightCommonDuration, PwrightCommonFailureKind,
    PwrightLocatorScreenshot as WireMessage, PwrightLocatorScreenshotError as WireError,
    PwrightLocatorScreenshotErrorDetailsRef as WireErrorDetailsRef,
    PwrightLocatorScreenshotErrorRef as WireErrorRef,
    PwrightLocatorScreenshotOutcome as WireOutcome,
    PwrightLocatorScreenshotOutcomeRef as WireOutcomeRef,
    PwrightLocatorScreenshotParamsRef as WireParamsRef,
    PwrightLocatorScreenshotRef as WireMessageRef,
    PwrightLocatorScreenshotScreenshotData as WireScreenshotData,
    PwrightLocatorScreenshotScreenshotFormat as WireScreenshotFormat,
    PwrightLocatorScreenshotScreenshotOptions as WireScreenshotOptions,
    PwrightLocatorScreenshotScreenshotScale as WireScreenshotScale,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocatorScreenshotCodecLimits {
    pub screenshot_bytes: usize,
    pub string_bytes: usize,
    pub selector_depth: usize,
    pub selector_nodes: usize,
}

impl LocatorScreenshotCodecLimits {
    pub const fn new(
        screenshot_bytes: usize,
        string_bytes: usize,
        selector_depth: usize,
        selector_nodes: usize,
    ) -> Self {
        Self {
            screenshot_bytes,
            string_bytes,
            selector_depth,
            selector_nodes,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LocatorScreenshotCodec {
    limits: LocatorScreenshotCodecLimits,
    result_ledger: ResourceLedger,
}

impl LocatorScreenshotCodec {
    pub const fn new(limits: LocatorScreenshotCodecLimits, result_ledger: ResourceLedger) -> Self {
        Self {
            limits,
            result_ledger,
        }
    }

    pub const fn limits(&self) -> LocatorScreenshotCodecLimits {
        self.limits
    }

    pub const fn result_ledger(&self) -> &ResourceLedger {
        &self.result_ledger
    }

    fn value_limits(&self) -> BinaryValueLimits {
        BinaryValueLimits::new(
            self.limits.string_bytes.saturating_mul(2),
            0,
            0,
            self.limits.selector_depth,
            self.limits.selector_nodes.saturating_add(32),
        )
    }
}

#[derive(Debug)]
pub enum LocatorScreenshotCodecError {
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
    Selector(SelectorModelDecodeError),
    Resource(ResourceError),
    Encode(BinaryValueEncodeError),
    Decode(BinaryValueDecodeError),
}

impl Display for LocatorScreenshotCodecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongRoute(route) => {
                write!(formatter, "Locator screenshot payload arrived on {route:?}")
            }
            Self::WrongMethod { expected, actual } => write!(
                formatter,
                "expected Locator screenshot method {expected}, received {actual}"
            ),
            Self::IdentityMismatch {
                field,
                outer,
                inner,
            } => write!(
                formatter,
                "Locator screenshot payload {field} {inner} does not match outer route {outer}"
            ),
            Self::UnexpectedAttachments { actual } => write!(
                formatter,
                "Locator screenshot payload has {actual} unexpected attachment(s)"
            ),
            Self::MissingScreenshotAttachment => {
                formatter.write_str("successful Locator screenshot result has no attachment")
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
                "screenshot has {actual} bytes, outside the non-empty {limit}-byte limit"
            ),
            Self::InvalidOptions(message) => formatter.write_str(message),
            Self::ErrorStringTooLong { actual, limit } => write!(
                formatter,
                "Locator screenshot error string has {actual} bytes, exceeding limit {limit}"
            ),
            Self::WrongPayloadKind { expected, actual } => write!(
                formatter,
                "expected Locator screenshot payload kind {expected}, received {actual}"
            ),
            Self::Selector(error) => write!(formatter, "invalid decoded selector: {error}"),
            Self::Resource(error) => Display::fmt(error, formatter),
            Self::Encode(error) => Display::fmt(error, formatter),
            Self::Decode(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for LocatorScreenshotCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Selector(error) => Some(error),
            Self::Resource(error) => Some(error),
            Self::Encode(error) => Some(error),
            Self::Decode(error) => Some(error),
            _ => None,
        }
    }
}

impl WireCodec<Command<LocatorScreenshot>> for LocatorScreenshotCodec {
    type Error = LocatorScreenshotCodecError;

    fn encode_parts(
        &self,
        command: &Command<LocatorScreenshot>,
    ) -> Result<EncodedParts, Self::Error> {
        validate_options(&command.params().options)?;
        let wire = WireMessageRef::LocatorScreenshotCommand {
            call_id: command.call_id().get(),
            object_id: command.object_id().get(),
            metadata: metadata_to_wire(command.metadata()),
            params: WireParamsRef {
                selector: selector_to_wire(&command.params().selector),
                options: options_to_wire(command.params().options),
            },
        };
        let payload = encode_value(&wire, self.value_limits())
            .map_err(LocatorScreenshotCodecError::Encode)?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: LocatorScreenshot::METHOD,
            },
            payload,
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<LocatorScreenshot>, Self::Error> {
        let (outer_call_id, outer_object_id) = command_route::<LocatorScreenshot>(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<WireMessage>(frame.payload(), self.value_limits())
            .map_err(LocatorScreenshotCodecError::Decode)?
        {
            WireMessage::LocatorScreenshotCommand {
                call_id,
                object_id,
                metadata,
                params,
            } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                ensure_identity("object ID", outer_object_id.get(), object_id)?;
                let params = LocatorScreenshotParams::new(
                    selector_from_wire(params.selector)
                        .map_err(LocatorScreenshotCodecError::Selector)?,
                    options_from_wire(params.options),
                );
                validate_options(&params.options)?;
                Ok(Command::new(
                    CallId::new(call_id),
                    ObjectId::new(object_id),
                    metadata_from_wire(metadata),
                    params,
                ))
            }
            other => Err(LocatorScreenshotCodecError::WrongPayloadKind {
                expected: SCREENSHOT_COMMAND,
                actual: wire_kind(&other),
            }),
        }
    }
}

impl WireCodec<CommandResult<LocatorScreenshot>> for LocatorScreenshotCodec {
    type Error = LocatorScreenshotCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<LocatorScreenshot>,
    ) -> Result<EncodedParts, Self::Error> {
        let mut attachments = Vec::new();
        let outcome = match result.outcome() {
            Ok(result) => {
                let (metadata, attachment) =
                    encode_screenshot_attachment(result, self.limits.screenshot_bytes)
                        .map_err(map_attachment_error)?;
                attachments.push(attachment);
                WireOutcomeRef::Success {
                    result: WireScreenshotData {
                        attachment_index: metadata.index,
                        byte_length: metadata.byte_length,
                        format: format_to_wire(metadata.format),
                    },
                }
            }
            Err(error) => {
                validate_operation_error(error, self.limits.string_bytes)?;
                WireOutcomeRef::Error {
                    error: operation_error_to_wire_ref(error),
                }
            }
        };
        let wire = WireMessageRef::LocatorScreenshotResult {
            call_id: result.call_id().get(),
            outcome,
        };
        let payload = encode_value(&wire, self.value_limits())
            .map_err(LocatorScreenshotCodecError::Encode)?;
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload,
            attachments,
        ))
    }

    fn decode_frame(
        &self,
        frame: WireFrame,
    ) -> Result<CommandResult<LocatorScreenshot>, Self::Error> {
        let outer_call_id = result_route(&frame)?;
        match decode_value::<WireMessage>(frame.payload(), self.value_limits())
            .map_err(LocatorScreenshotCodecError::Decode)?
        {
            WireMessage::LocatorScreenshotResult { call_id, outcome } => {
                ensure_identity("call ID", outer_call_id.get(), call_id)?;
                let outcome = match outcome {
                    WireOutcome::Success { result } => {
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
                    WireOutcome::Error { error } => {
                        ensure_no_attachments(&frame)?;
                        let error = operation_error_from_wire(error);
                        validate_operation_error(&error, self.limits.string_bytes)?;
                        Err(error)
                    }
                };
                Ok(CommandResult::new(CallId::new(call_id), outcome))
            }
            other => Err(LocatorScreenshotCodecError::WrongPayloadKind {
                expected: SCREENSHOT_RESULT,
                actual: wire_kind(&other),
            }),
        }
    }
}

fn validate_options(options: &ScreenshotOptions) -> Result<(), LocatorScreenshotCodecError> {
    if options.full_page.is_some() {
        return Err(LocatorScreenshotCodecError::InvalidOptions(
            "full_page is not valid for Locator screenshots",
        ));
    }
    if options.clip.is_some() {
        return Err(LocatorScreenshotCodecError::InvalidOptions(
            "clip is not valid for Locator screenshots",
        ));
    }
    if options.quality.is_some() && options.format.unwrap_or_default() == ScreenshotFormat::Png {
        return Err(LocatorScreenshotCodecError::InvalidOptions(
            "screenshot quality is only valid for JPEG or WebP",
        ));
    }
    Ok(())
}

fn map_attachment_error(error: ScreenshotAttachmentError) -> LocatorScreenshotCodecError {
    match error {
        ScreenshotAttachmentError::InvalidIndex(index) => {
            LocatorScreenshotCodecError::InvalidAttachmentIndex(index)
        }
        ScreenshotAttachmentError::Missing => {
            LocatorScreenshotCodecError::MissingScreenshotAttachment
        }
        ScreenshotAttachmentError::UnexpectedCount { actual } => {
            LocatorScreenshotCodecError::UnexpectedAttachments { actual }
        }
        ScreenshotAttachmentError::LengthMismatch { declared, actual } => {
            LocatorScreenshotCodecError::AttachmentLengthMismatch { declared, actual }
        }
        ScreenshotAttachmentError::InvalidLength { actual, limit } => {
            LocatorScreenshotCodecError::ScreenshotTooLarge { actual, limit }
        }
        ScreenshotAttachmentError::Resource(error) => LocatorScreenshotCodecError::Resource(error),
    }
}

fn options_to_wire(options: ScreenshotOptions) -> WireScreenshotOptions {
    WireScreenshotOptions {
        format: options.format.map(format_to_wire),
        quality: options.quality,
        scale: options.scale.map(scale_to_wire),
    }
}

fn options_from_wire(options: WireScreenshotOptions) -> ScreenshotOptions {
    ScreenshotOptions {
        format: options.format.map(format_from_wire),
        quality: options.quality,
        full_page: None,
        clip: None,
        scale: options.scale.map(scale_from_wire),
    }
}

fn format_to_wire(format: ScreenshotFormat) -> WireScreenshotFormat {
    match format {
        ScreenshotFormat::Png => WireScreenshotFormat::Png,
        ScreenshotFormat::Jpeg => WireScreenshotFormat::Jpeg,
        ScreenshotFormat::Webp => WireScreenshotFormat::Webp,
    }
}

fn format_from_wire(format: WireScreenshotFormat) -> ScreenshotFormat {
    match format {
        WireScreenshotFormat::Png => ScreenshotFormat::Png,
        WireScreenshotFormat::Jpeg => ScreenshotFormat::Jpeg,
        WireScreenshotFormat::Webp => ScreenshotFormat::Webp,
    }
}

fn scale_to_wire(scale: ScreenshotScale) -> WireScreenshotScale {
    match scale {
        ScreenshotScale::Css => WireScreenshotScale::Css,
        ScreenshotScale::Device => WireScreenshotScale::Device,
    }
}

fn scale_from_wire(scale: WireScreenshotScale) -> ScreenshotScale {
    match scale {
        WireScreenshotScale::Css => ScreenshotScale::Css,
        WireScreenshotScale::Device => ScreenshotScale::Device,
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
    CallMetadata::new(
        metadata
            .timeout
            .map(|timeout| Duration::new(timeout.seconds, timeout.nanoseconds)),
        metadata.cancellation_id.map(CancellationId::new),
    )
}

fn error_details_to_wire_ref(details: &LocatorScreenshotErrorDetails) -> WireErrorDetailsRef<'_> {
    match details {
        LocatorScreenshotErrorDetails::InvalidOptions(message) => {
            WireErrorDetailsRef::InvalidOptions { message }
        }
        LocatorScreenshotErrorDetails::InvalidSelector(message) => {
            WireErrorDetailsRef::InvalidSelector { message }
        }
        LocatorScreenshotErrorDetails::StrictViolation { selector, matches } => {
            WireErrorDetailsRef::StrictViolation {
                selector,
                matches: *matches,
            }
        }
        LocatorScreenshotErrorDetails::InvalidBackendResponse(message) => {
            WireErrorDetailsRef::InvalidBackendResponse { message }
        }
        LocatorScreenshotErrorDetails::Resource(message) => {
            WireErrorDetailsRef::Resource { message }
        }
        LocatorScreenshotErrorDetails::Backend(message) => WireErrorDetailsRef::Backend { message },
    }
}

fn operation_error_to_wire_ref(
    error: &OperationError<LocatorScreenshotErrorDetails>,
) -> WireErrorRef<'_> {
    WireErrorRef {
        failure_kind: failure_kind_to_wire(error.kind()),
        message: error.message(),
        details: error.details().map(error_details_to_wire_ref),
    }
}

fn operation_error_from_wire(error: WireError) -> OperationError<LocatorScreenshotErrorDetails> {
    OperationError::new(
        failure_kind_from_wire(error.failure_kind),
        error.message,
        error.details.map(Into::into),
    )
}

fn validate_operation_error(
    error: &OperationError<LocatorScreenshotErrorDetails>,
    limit: usize,
) -> Result<(), LocatorScreenshotCodecError> {
    validate_error_string(error.message(), limit)?;
    if let Some(details) = error.details() {
        match details {
            LocatorScreenshotErrorDetails::StrictViolation { selector, .. } => {
                validate_error_string(selector, limit)?
            }
            LocatorScreenshotErrorDetails::InvalidOptions(message)
            | LocatorScreenshotErrorDetails::InvalidSelector(message)
            | LocatorScreenshotErrorDetails::InvalidBackendResponse(message)
            | LocatorScreenshotErrorDetails::Resource(message)
            | LocatorScreenshotErrorDetails::Backend(message) => {
                validate_error_string(message, limit)?
            }
        }
    }
    Ok(())
}

fn validate_error_string(value: &str, limit: usize) -> Result<(), LocatorScreenshotCodecError> {
    if value.len() <= limit {
        Ok(())
    } else {
        Err(LocatorScreenshotCodecError::ErrorStringTooLong {
            actual: value.len(),
            limit,
        })
    }
}

fn ensure_no_attachments(frame: &WireFrame) -> Result<(), LocatorScreenshotCodecError> {
    if frame.attachments().is_empty() {
        Ok(())
    } else {
        Err(LocatorScreenshotCodecError::UnexpectedAttachments {
            actual: frame.attachments().len(),
        })
    }
}

fn ensure_identity(
    field: &'static str,
    outer: u64,
    inner: u64,
) -> Result<(), LocatorScreenshotCodecError> {
    if outer == inner {
        Ok(())
    } else {
        Err(LocatorScreenshotCodecError::IdentityMismatch {
            field,
            outer,
            inner,
        })
    }
}

fn command_route<S: CommandSpec>(
    frame: &WireFrame,
) -> Result<(CallId, ObjectId), LocatorScreenshotCodecError> {
    let FrameRoute::Command {
        call_id,
        target,
        method,
    } = frame.route()
    else {
        return Err(LocatorScreenshotCodecError::WrongRoute(frame.route()));
    };
    if method != S::METHOD {
        return Err(LocatorScreenshotCodecError::WrongMethod {
            expected: S::METHOD,
            actual: method,
        });
    }
    Ok((call_id, target))
}

fn result_route(frame: &WireFrame) -> Result<CallId, LocatorScreenshotCodecError> {
    let FrameRoute::Result { call_id } = frame.route() else {
        return Err(LocatorScreenshotCodecError::WrongRoute(frame.route()));
    };
    Ok(call_id)
}

fn wire_kind(value: &WireMessage) -> u8 {
    match value {
        WireMessage::LocatorScreenshotCommand { .. } => SCREENSHOT_COMMAND,
        WireMessage::LocatorScreenshotResult { .. } => SCREENSHOT_RESULT,
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
    use pwright_cap_locator_screenshot::{
        LocatorScreenshot, LocatorScreenshotParams, ScreenshotData, ScreenshotFormat,
        ScreenshotOptions, Selector,
    };
    use pwright_core::{
        BudgetedBytes, CallId, CallMetadata, Command, CommandResult, ObjectId, ResourceClass,
        ResourceLedger, ResourceLimits, ResourceScope,
    };
    use pwright_selector_model::{AriaRole, RoleOptions};
    use pwright_transport::{FrameLimits, WireCodec, WireProtocol};

    use super::{
        LocatorScreenshotCodec, LocatorScreenshotCodecError, LocatorScreenshotCodecLimits,
    };

    fn codec() -> LocatorScreenshotCodec {
        let ledger = ResourceLedger::new(ResourceLimits::deny_all(8, 8).with_class(
            ResourceClass::ScreenshotBytes,
            1_024,
            1_024,
        ));
        LocatorScreenshotCodec::new(
            LocatorScreenshotCodecLimits::new(1_024, 256, 32, 64),
            ledger,
        )
    }

    #[test]
    fn command_round_trips_imported_selector_and_locator_options() {
        // Arrange
        let codec = codec();
        let params = LocatorScreenshotParams::new(
            Selector::role(AriaRole::Button, RoleOptions::new().with_name("Save", true))
                .expect("role selector should validate"),
            ScreenshotOptions {
                format: Some(ScreenshotFormat::Jpeg),
                quality: Some(80),
                ..ScreenshotOptions::default()
            },
        );
        let command = Command::<LocatorScreenshot>::new(
            CallId::new(7),
            ObjectId::new(9),
            CallMetadata::default(),
            params.clone(),
        );
        let frame_limits = FrameLimits::new(8_192, 1, 1_024, 9_216);
        let protocol = WireProtocol::new_exact_directional(
            codec,
            pwright_core::ProtocolVersion::new(1, 0),
            frame_limits,
            frame_limits,
        );

        // Act
        let frame = protocol.encode(&command).expect("command should encode");
        let decoded: Command<LocatorScreenshot> =
            protocol.decode(frame).expect("command should decode");

        // Assert
        assert_eq!(decoded.call_id(), CallId::new(7));
        assert_eq!(decoded.object_id(), ObjectId::new(9));
        assert_eq!(decoded.params(), &params);
    }

    #[test]
    fn successful_result_round_trips_one_budgeted_attachment() {
        // Arrange
        let codec = codec();
        let result_ledger = codec.result_ledger().clone();
        let source_ledger = ResourceLedger::new(ResourceLimits::deny_all(1, 1).with_class(
            ResourceClass::ScreenshotBytes,
            16,
            16,
        ));
        let bytes = BudgetedBytes::copy_from_slice(
            &source_ledger,
            ResourceScope::Call(CallId::new(11)),
            ResourceClass::ScreenshotBytes,
            b"image",
        )
        .expect("source screenshot should fit");
        let result = CommandResult::<LocatorScreenshot>::new(
            CallId::new(11),
            Ok(ScreenshotData::new(bytes, ScreenshotFormat::Png)),
        );
        let frame_limits = FrameLimits::new(8_192, 1, 1_024, 9_216);
        let protocol = WireProtocol::new_exact_directional(
            codec,
            pwright_core::ProtocolVersion::new(1, 0),
            frame_limits,
            frame_limits,
        );

        // Act
        let frame = protocol.encode(&result).expect("result should encode");
        let decoded: CommandResult<LocatorScreenshot> =
            protocol.decode(frame).expect("result should decode");

        // Assert
        let screenshot = decoded.outcome().as_ref().expect("result should succeed");
        assert_eq!(screenshot.as_bytes(), b"image");
        assert_eq!(screenshot.format(), ScreenshotFormat::Png);
        assert_eq!(
            result_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ScreenshotBytes)
                .current,
            5
        );
        drop(decoded);
        assert_eq!(
            result_ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ScreenshotBytes)
                .current,
            0
        );
    }

    #[test]
    fn page_only_options_are_rejected_before_encoding() {
        // Arrange
        let codec = codec();
        let command = Command::<LocatorScreenshot>::new(
            CallId::new(1),
            ObjectId::new(2),
            CallMetadata::default(),
            LocatorScreenshotParams::new(
                Selector::css("#hero").expect("selector should validate"),
                ScreenshotOptions {
                    full_page: Some(false),
                    ..ScreenshotOptions::default()
                },
            ),
        );

        // Act
        let error = codec
            .encode_parts(&command)
            .expect_err("full_page must not be silently ignored");

        // Assert
        assert!(matches!(
            error,
            LocatorScreenshotCodecError::InvalidOptions(
                "full_page is not valid for Locator screenshots"
            )
        ));
    }
}
