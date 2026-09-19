//! Bound formatting itself, not an unbounded intermediate String allocation.
use super::*;
use std::fmt::{self, Write};

const BYTES: usize = 1024;
struct Message(String);
impl Write for Message {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let mut end = value.len().min(BYTES - self.0.len());
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        self.0.push_str(&value[..end]);
        Ok(())
    }
}

pub(super) fn bounded(value: impl fmt::Display) -> String {
    let mut message = Message(String::with_capacity(BYTES));
    // The only writer is infallible; truncation preserves valid UTF-8.
    let _ = write!(message, "{value}");
    message.0
}

pub(super) fn backend_error(
    method: &'static str,
    error: pwright_cdp::connection::CdpError,
) -> Error {
    let kind = pwright_backend_cdp_common::classify_cdp_failure(&error);
    OperationError::new(
        kind,
        bounded(format_args!("CDP {method} failed: {error}")),
        Some(PageLifecycleErrorDetails::Backend(bounded(error))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_command_diagnostics_remain_bounded_in_both_error_fields() {
        // Arrange
        let error = pwright_cdp::connection::CdpError::Other("失败".repeat(2048));
        // Act
        let result = backend_error("Target.closeTarget", error);
        // Assert
        assert_eq!(result.kind(), FailureKind::Backend);
        assert!(
            result
                .message()
                .starts_with("CDP Target.closeTarget failed: ")
        );
        assert!(result.message().len() <= BYTES);
        let Some(PageLifecycleErrorDetails::Backend(details)) = result.details() else {
            panic!("expected bounded native diagnostic details");
        };
        assert!(details.len() <= BYTES);
    }

    #[test]
    fn bounded_command_diagnostics_preserve_timeout_classification() {
        // Arrange
        let error = pwright_cdp::connection::CdpError::Timeout;
        // Act
        let result = backend_error("Target.closeTarget", error);
        // Assert
        assert_eq!(result.kind(), FailureKind::Timeout);
        assert!(result.message().contains("Timeout waiting for response"));
    }

    #[test]
    fn diagnostic_truncation_bounds_allocation_and_preserves_utf8() {
        // Arrange
        let input = "关闭".repeat(1024);
        // Act
        let result = bounded(&input);
        // Assert
        assert!(result.len() <= BYTES);
        assert_eq!(result.capacity(), BYTES);
        assert!(input.starts_with(&result));
        assert!(result.len() >= BYTES - 3);
    }

    #[test]
    fn fragmented_formatting_cannot_extend_the_admitted_buffer() {
        // Arrange
        let input = "x".repeat(BYTES);
        // Act
        let result = bounded(format_args!("{input}{}", "overflow"));
        // Assert
        assert_eq!(result, input);
        assert_eq!(result.capacity(), BYTES);
    }
}
