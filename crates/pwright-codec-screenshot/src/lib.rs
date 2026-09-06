//! Command-free screenshot attachment validation shared by capture codecs.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use pwright_core::{CallId, ResourceClass, ResourceError, ResourceLedger, ResourceScope};
use pwright_screenshot_model::{ScreenshotData, ScreenshotFormat};
use pwright_transport::WireFrame;

pub const SCREENSHOT_ATTACHMENT_INDEX: u32 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScreenshotAttachmentMetadata {
    pub index: u32,
    pub byte_length: u64,
    pub format: ScreenshotFormat,
}

#[derive(Debug)]
pub enum ScreenshotAttachmentError {
    InvalidIndex(u32),
    Missing,
    UnexpectedCount { actual: usize },
    LengthMismatch { declared: u64, actual: usize },
    InvalidLength { actual: usize, limit: usize },
    Resource(ResourceError),
}

impl Display for ScreenshotAttachmentError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIndex(index) => {
                write!(formatter, "screenshot attachment index {index} is not zero")
            }
            Self::Missing => formatter.write_str("successful screenshot result has no attachment"),
            Self::UnexpectedCount { actual } => write!(
                formatter,
                "successful screenshot result has {actual} attachments instead of one"
            ),
            Self::LengthMismatch { declared, actual } => write!(
                formatter,
                "screenshot declares {declared} bytes but attachment contains {actual}"
            ),
            Self::InvalidLength { actual, limit } => write!(
                formatter,
                "screenshot has {actual} bytes, outside the non-empty {limit}-byte limit"
            ),
            Self::Resource(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ScreenshotAttachmentError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resource(error) => Some(error),
            _ => None,
        }
    }
}

pub fn encode_screenshot_attachment(
    screenshot: &ScreenshotData,
    limit: usize,
) -> Result<(ScreenshotAttachmentMetadata, Vec<u8>), ScreenshotAttachmentError> {
    validate_screenshot_length(screenshot.len(), limit)?;
    let byte_length =
        u64::try_from(screenshot.len()).map_err(|_| ScreenshotAttachmentError::InvalidLength {
            actual: screenshot.len(),
            limit,
        })?;
    Ok((
        ScreenshotAttachmentMetadata {
            index: SCREENSHOT_ATTACHMENT_INDEX,
            byte_length,
            format: screenshot.format(),
        },
        screenshot.as_bytes().to_vec(),
    ))
}

pub fn decode_screenshot_attachment(
    frame: &WireFrame,
    metadata: ScreenshotAttachmentMetadata,
    limit: usize,
    result_ledger: &ResourceLedger,
    call_id: CallId,
) -> Result<ScreenshotData, ScreenshotAttachmentError> {
    if metadata.index != SCREENSHOT_ATTACHMENT_INDEX {
        return Err(ScreenshotAttachmentError::InvalidIndex(metadata.index));
    }
    if frame.attachments().len() != 1 {
        return Err(if frame.attachments().is_empty() {
            ScreenshotAttachmentError::Missing
        } else {
            ScreenshotAttachmentError::UnexpectedCount {
                actual: frame.attachments().len(),
            }
        });
    }
    let bytes = frame.attachments()[0].as_bytes();
    if metadata.byte_length != u64::try_from(bytes.len()).unwrap_or(u64::MAX) {
        return Err(ScreenshotAttachmentError::LengthMismatch {
            declared: metadata.byte_length,
            actual: bytes.len(),
        });
    }
    validate_screenshot_length(bytes.len(), limit)?;
    let bytes = pwright_core::BudgetedBytes::copy_from_slice(
        result_ledger,
        ResourceScope::Call(call_id),
        ResourceClass::ScreenshotBytes,
        bytes,
    )
    .map_err(ScreenshotAttachmentError::Resource)?;
    Ok(ScreenshotData::new(bytes, metadata.format))
}

pub fn validate_screenshot_length(
    actual: usize,
    limit: usize,
) -> Result<(), ScreenshotAttachmentError> {
    if actual == 0 || actual > limit {
        Err(ScreenshotAttachmentError::InvalidLength { actual, limit })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::{
        BudgetedBytes, CallId, MethodTag, ObjectId, ProtocolVersion, ResourceClass, ResourceLedger,
        ResourceLimits, ResourceScope,
    };
    use pwright_screenshot_model::{ScreenshotData, ScreenshotFormat};
    use pwright_transport::{FrameLimits, FrameRoute, WireFrame};

    use super::{
        ScreenshotAttachmentMetadata, decode_screenshot_attachment, encode_screenshot_attachment,
    };

    #[test]
    fn shared_attachment_round_trip_retains_and_releases_exact_result_bytes() {
        // Arrange
        let source_ledger = ResourceLedger::new(ResourceLimits::deny_all(1, 1).with_class(
            ResourceClass::ScreenshotBytes,
            16,
            16,
        ));
        let result_ledger = ResourceLedger::new(ResourceLimits::deny_all(1, 1).with_class(
            ResourceClass::ScreenshotBytes,
            16,
            16,
        ));
        let screenshot = ScreenshotData::new(
            BudgetedBytes::copy_from_slice(
                &source_ledger,
                ResourceScope::Call(CallId::new(1)),
                ResourceClass::ScreenshotBytes,
                b"image",
            )
            .expect("source bytes should fit"),
            ScreenshotFormat::Png,
        );

        // Act
        let (metadata, attachment) =
            encode_screenshot_attachment(&screenshot, 16).expect("attachment should encode");
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Command {
                call_id: CallId::new(1),
                target: ObjectId::new(2),
                method: MethodTag::new(3),
            },
            Vec::new(),
            vec![attachment],
            FrameLimits::new(0, 1, 16, 16),
        )
        .expect("frame should build");
        let decoded =
            decode_screenshot_attachment(&frame, metadata, 16, &result_ledger, CallId::new(1))
                .expect("attachment should decode");

        // Assert
        assert_eq!(decoded.as_bytes(), b"image");
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
    fn invalid_index_is_rejected_before_copying_bytes() {
        // Arrange
        let ledger = ResourceLedger::new(ResourceLimits::deny_all(1, 1).with_class(
            ResourceClass::ScreenshotBytes,
            16,
            16,
        ));
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Result {
                call_id: CallId::new(1),
            },
            Vec::new(),
            vec![b"image".to_vec()],
            FrameLimits::new(0, 1, 16, 16),
        )
        .expect("frame should build");

        // Act
        let error = decode_screenshot_attachment(
            &frame,
            ScreenshotAttachmentMetadata {
                index: 1,
                byte_length: 5,
                format: ScreenshotFormat::Png,
            },
            16,
            &ledger,
            CallId::new(1),
        )
        .expect_err("non-zero index must fail");

        // Assert
        assert_eq!(
            error.to_string(),
            "screenshot attachment index 1 is not zero"
        );
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ScreenshotBytes)
                .current,
            0
        );
    }
}
