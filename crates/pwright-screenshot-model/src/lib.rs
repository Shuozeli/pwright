//! Command-free screenshot values shared by optional capture capabilities.
//!
//! This crate defines values and retained-byte ownership only. It has no
//! command, protocol, transport, runtime, browser backend, or filesystem API.

use pwright_core::BudgetedBytes;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ScreenshotFormat {
    #[default]
    Png,
    Jpeg,
    Webp,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ScreenshotScale {
    Css,
    #[default]
    Device,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenshotClip {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScreenshotOptions {
    pub format: Option<ScreenshotFormat>,
    pub quality: Option<u8>,
    pub full_page: Option<bool>,
    pub clip: Option<ScreenshotClip>,
    pub scale: Option<ScreenshotScale>,
}

#[derive(Debug)]
pub struct ScreenshotData {
    bytes: BudgetedBytes,
    format: ScreenshotFormat,
}

impl ScreenshotData {
    pub const fn new(bytes: BudgetedBytes, format: ScreenshotFormat) -> Self {
        Self { bytes, format }
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_bytes()
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub const fn format(&self) -> ScreenshotFormat {
        self.format
    }

    pub const fn budgeted_bytes(&self) -> &BudgetedBytes {
        &self.bytes
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::{
        BudgetedBytes, CallId, ResourceClass, ResourceLedger, ResourceLimits, ResourceScope,
    };

    use super::{ScreenshotData, ScreenshotFormat};

    #[test]
    fn screenshot_data_retains_and_releases_its_exact_byte_lease() {
        // Arrange
        let ledger = ResourceLedger::new(ResourceLimits::deny_all(1, 1).with_class(
            ResourceClass::ScreenshotBytes,
            8,
            8,
        ));
        let bytes = BudgetedBytes::copy_from_slice(
            &ledger,
            ResourceScope::Call(CallId::new(7)),
            ResourceClass::ScreenshotBytes,
            b"png!",
        )
        .expect("four screenshot bytes should fit");

        // Act
        let screenshot = ScreenshotData::new(bytes, ScreenshotFormat::Png);

        // Assert
        assert_eq!(screenshot.as_bytes(), b"png!");
        assert_eq!(screenshot.len(), 4);
        assert!(!screenshot.is_empty());
        assert_eq!(screenshot.format(), ScreenshotFormat::Png);
        assert_eq!(
            ledger
                .metrics()
                .expect("metrics should be available")
                .class(ResourceClass::ScreenshotBytes)
                .current,
            4
        );
        drop(screenshot);
        assert_eq!(
            ledger
                .metrics()
                .expect("metrics should be available")
                .class(ResourceClass::ScreenshotBytes)
                .current,
            0
        );
    }
}
