//! Bounded Page capture through the existing optional Rust content command.
pub(crate) mod queue;
use pwright_cap_content::PageScreenshot;
pub use pwright_cap_content::{
    ScreenshotClip, ScreenshotData, ScreenshotFormat, ScreenshotOptions, ScreenshotScale,
};
pub(crate) use queue::requirements;

use crate::{HostError, HostSession};

// The transport already bounds a response to 4 MiB. A 2 MiB binary image's
// base64 representation fits that budget without increasing transport limits.
pub const MAX_BYTES: usize = 2 * 1024 * 1024;
// Native result and Node-owned copy coexist during settlement. Admission is
// held through conversion; user-owned Buffers outlive the ticket independently.
pub const FFI_ALLOWANCE: u64 = 2 * MAX_BYTES as u64 + 4096;

/// Bounded caller-owned result at the host boundary. The FFI caller admits its
/// result allowance before capture and retains it through Node Buffer copying.
/// The core screenshot lease is released before the next queued capture starts.
pub struct ScreenshotOutput(Vec<u8>);

impl ScreenshotOutput {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    fn from_capture(data: ScreenshotData) -> Self {
        Self(data.as_bytes().to_vec())
    }
}

impl HostSession {
    pub async fn page_screenshot(
        &self,
        page: &str,
        options: ScreenshotOptions,
        timeout_ms: u32,
    ) -> Result<ScreenshotOutput, HostError> {
        validate(options)?;
        let (object, _, _operation) = self.admit_operation(page)?;
        let queue = self
            .state
            .lock()
            .map_err(HostError::backend)?
            .as_ref()
            .ok_or(HostError::TargetClosed)?
            .screenshot_queue
            .clone();
        // One deadline includes queue wait; the controlled command must not
        // restart it. Dropping that command still invokes runtime cancellation.
        queue
            .run(object, timeout_ms, &self.stop, async {
                self.invoke::<PageScreenshot>(page, options, 0)
                    .await
                    .map(ScreenshotOutput::from_capture)
            })
            .await
    }
}

fn validate(options: ScreenshotOptions) -> Result<(), HostError> {
    if options.format == Some(ScreenshotFormat::Webp) {
        return Err(HostError::InvalidArgument(
            "screenshot type must be png or jpeg".into(),
        ));
    }
    if let Some(quality) = options.quality
        && (quality > 100 || options.format != Some(ScreenshotFormat::Jpeg))
    {
        return Err(HostError::InvalidArgument(
            "quality requires jpeg and an integer in [0, 100]".into(),
        ));
    }
    if let Some(clip) = options.clip {
        if [clip.x, clip.y, clip.width, clip.height]
            .into_iter()
            .any(|value| !value.is_finite())
            || clip.x < 0.0
            || clip.y < 0.0
            || clip.width <= 0.0
            || clip.height <= 0.0
        {
            return Err(HostError::InvalidArgument(
                "clip requires finite non-negative coordinates and positive dimensions".into(),
            ));
        }
        if options.full_page == Some(true) {
            return Err(HostError::UnsupportedOperation(
                "combined fullPage and clip capture".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_handoff_releases_core_lease_before_next_capture() {
        use pwright_core::{
            BudgetedBytes, ObjectId, ResourceClass, ResourceLedger, ResourceLimits, ResourceScope,
        };
        // Arrange
        let ledger = ResourceLedger::new(ResourceLimits::deny_all(1, 1).with_class(
            ResourceClass::ScreenshotBytes,
            8,
            8,
        ));
        let data = ScreenshotData::new(
            BudgetedBytes::copy_from_slice(
                &ledger,
                ResourceScope::Object(ObjectId::new(1)),
                ResourceClass::ScreenshotBytes,
                b"image",
            )
            .unwrap(),
            ScreenshotFormat::Png,
        );
        // Act
        let output = ScreenshotOutput::from_capture(data);
        // Assert
        assert_eq!(output.as_bytes(), b"image");
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ScreenshotBytes)
                .current,
            0
        );
    }

    #[test]
    fn screenshot_accepts_jpeg_quality_endpoints() {
        // Arrange
        let options = [0, 100].map(|quality| ScreenshotOptions {
            format: Some(ScreenshotFormat::Jpeg),
            quality: Some(quality),
            ..Default::default()
        });
        // Act
        let results = options.map(validate);
        // Assert
        assert!(results.into_iter().all(|result| result.is_ok()));
    }

    #[test]
    fn screenshot_rejects_quality_on_default_png() {
        // Arrange
        let options = ScreenshotOptions {
            quality: Some(80),
            ..Default::default()
        };
        // Act
        let result = validate(options);
        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    }

    #[test]
    fn screenshot_rejects_nonfinite_geometry_before_dispatch() {
        // Arrange
        let options = ScreenshotOptions {
            clip: Some(ScreenshotClip {
                x: 0.0,
                y: 0.0,
                width: f64::NAN,
                height: 10.0,
            }),
            ..Default::default()
        };
        // Act
        let result = validate(options);
        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    }

    #[test]
    fn screenshot_combined_full_page_clip_remains_explicitly_unsupported() {
        // Arrange
        let options = ScreenshotOptions {
            full_page: Some(true),
            clip: Some(ScreenshotClip {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            }),
            ..Default::default()
        };
        // Act
        let result = validate(options);
        // Assert
        assert!(matches!(result, Err(HostError::UnsupportedOperation(_))));
    }
}
