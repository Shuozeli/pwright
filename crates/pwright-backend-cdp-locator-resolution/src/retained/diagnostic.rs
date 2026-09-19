//! Bounded owned diagnostics for the opt-in retained-reference pipeline.
//!
//! Preserve terminal/resource classifications and ordinary string variants.
//! JSON and compound errors have opaque/recursive owned storage, so normalize
//! those two variants to Other using a bounded streaming formatter. This does
//! not clone the error tree or first allocate its complete Display output.
//! Static strings and inline resource data are preserved without new storage.

use std::fmt::{self, Write};

use pwright_cdp::connection::CdpError;

pub(crate) const DIAGNOSTIC_BYTES: usize = 1024;

pub(crate) fn bounded_error(error: CdpError) -> CdpError {
    match error {
        CdpError::WebSocket(message) => CdpError::WebSocket(compact(&message, DIAGNOSTIC_BYTES)),
        CdpError::Protocol { code, message } => CdpError::Protocol {
            code,
            message: compact(&message, DIAGNOSTIC_BYTES),
        },
        CdpError::ElementNotFound { selector } => CdpError::ElementNotFound {
            selector: compact(&selector, DIAGNOSTIC_BYTES),
        },
        CdpError::NavigationFailed { url, reason } => CdpError::NavigationFailed {
            // Two simultaneously retained strings share one diagnostic budget.
            url: compact(&url, DIAGNOSTIC_BYTES / 2),
            reason: compact(&reason, DIAGNOSTIC_BYTES - DIAGNOSTIC_BYTES / 2),
        },
        CdpError::TabNotFound(message) => {
            CdpError::TabNotFound(compact(&message, DIAGNOSTIC_BYTES))
        }
        CdpError::HttpFailed(message) => CdpError::HttpFailed(compact(&message, DIAGNOSTIC_BYTES)),
        CdpError::JsException(message) => {
            CdpError::JsException(compact(&message, DIAGNOSTIC_BYTES))
        }
        CdpError::Other(message) => CdpError::Other(compact(&message, DIAGNOSTIC_BYTES)),
        error @ (CdpError::Json(_) | CdpError::Compound { .. }) => {
            CdpError::Other(bounded_display(&error))
        }
        error @ (CdpError::Closed
        | CdpError::SessionDetached
        | CdpError::ChannelDropped
        | CdpError::Timeout
        | CdpError::LimitExceeded { .. }
        | CdpError::InvalidConfiguration(_)
        | CdpError::IdentifierExhausted
        | CdpError::Resource(_)
        | CdpError::PageClosed) => error,
    }
}

fn compact(value: &str, maximum: usize) -> String {
    let mut end = value.len().min(maximum);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    // Do not truncate the incoming String in place: that would retain its
    // potentially huge capacity after the rest of the raw response is dropped.
    String::from(&value[..end])
}

fn bounded_display(value: &impl fmt::Display) -> String {
    let mut writer = DiagnosticWriter {
        text: String::with_capacity(DIAGNOSTIC_BYTES),
    };
    // Overflow deliberately returns fmt::Error to stop further formatting.
    let _ = write!(writer, "{value}");
    writer.text
}

struct DiagnosticWriter {
    text: String,
}

impl Write for DiagnosticWriter {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let available = DIAGNOSTIC_BYTES - self.text.len();
        let mut end = value.len().min(available);
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        self.text.push_str(&value[..end]);
        if end < value.len() {
            Err(fmt::Error)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_core::{ResourceClass, ResourceError};
    use std::cell::Cell;

    fn oversized() -> String {
        let mut value = String::with_capacity(DIAGNOSTIC_BYTES * 64);
        value.push_str(&"é漢".repeat(DIAGNOSTIC_BYTES));
        value
    }

    fn assert_bounded(value: &str, capacity: usize, limit: usize) {
        assert!(value.len() <= limit);
        assert!(capacity <= limit);
        assert!(value.is_char_boundary(value.len()));
    }

    #[test]
    fn normal_owned_string_variants_preserve_their_kind_without_oversized_storage() {
        for variant in 0..7 {
            // Arrange
            let input = oversized();
            let error = match variant {
                0 => CdpError::WebSocket(input),
                1 => CdpError::ElementNotFound { selector: input },
                2 => CdpError::TabNotFound(input),
                3 => CdpError::HttpFailed(input),
                4 => CdpError::JsException(input),
                5 => CdpError::Other(input),
                _ => CdpError::Protocol {
                    code: -32007,
                    message: input,
                },
            };
            let discriminant = std::mem::discriminant(&error);

            // Act
            let output = bounded_error(error);

            // Assert
            assert_eq!(std::mem::discriminant(&output), discriminant);
            let value = match &output {
                CdpError::WebSocket(value)
                | CdpError::TabNotFound(value)
                | CdpError::HttpFailed(value)
                | CdpError::JsException(value)
                | CdpError::Other(value) => value,
                CdpError::ElementNotFound { selector } => selector,
                CdpError::Protocol { code, message } => {
                    assert_eq!(*code, -32007);
                    message
                }
                _ => panic!("unexpected error variant"),
            };
            assert_bounded(value, value.capacity(), DIAGNOSTIC_BYTES);
            assert!("é漢".repeat(DIAGNOSTIC_BYTES).starts_with(value));
        }
    }

    #[test]
    fn small_messages_do_not_keep_the_original_large_capacity() {
        // Arrange
        let mut input = String::with_capacity(DIAGNOSTIC_BYTES * 64);
        input.push_str("small sentinel");

        // Act
        let output = bounded_error(CdpError::Other(input));

        // Assert
        let CdpError::Other(message) = output else {
            panic!("variant changed")
        };
        assert_eq!(message, "small sentinel");
        assert_bounded(&message, message.capacity(), DIAGNOSTIC_BYTES);
    }

    #[test]
    fn navigation_url_and_reason_share_one_total_budget() {
        // Arrange
        let error = CdpError::NavigationFailed {
            url: oversized(),
            reason: oversized(),
        };

        // Act
        let output = bounded_error(error);

        // Assert
        let CdpError::NavigationFailed { url, reason } = output else {
            panic!("variant changed")
        };
        assert_bounded(&url, url.capacity(), DIAGNOSTIC_BYTES / 2);
        assert_bounded(&reason, reason.capacity(), DIAGNOSTIC_BYTES / 2);
        assert!(url.capacity() + reason.capacity() <= DIAGNOSTIC_BYTES);
    }

    #[test]
    fn compound_error_is_normalized_without_retaining_its_owned_tree() {
        // Arrange
        let error = CdpError::Compound {
            source: Box::new(CdpError::Other("source sentinel".into())),
            system: Box::new(CdpError::Compound {
                source: Box::new(CdpError::JsException(oversized())),
                system: Box::new(CdpError::Other(oversized())),
            }),
        };

        // Act
        let output = bounded_error(error);

        // Assert
        let CdpError::Other(message) = output else {
            panic!("compound storage survived")
        };
        assert!(message.contains("source sentinel"));
        assert_bounded(&message, message.capacity(), DIAGNOSTIC_BYTES);
    }

    #[test]
    fn opaque_json_error_is_normalized_to_bounded_utf8_storage() {
        // Arrange
        let error = CdpError::Json(serde_json::Error::io(std::io::Error::other(oversized())));

        // Act
        let output = bounded_error(error);

        // Assert
        let CdpError::Other(message) = output else {
            panic!("opaque JSON storage survived")
        };
        assert!(message.starts_with("JSON error: "));
        assert_bounded(&message, message.capacity(), DIAGNOSTIC_BYTES);
    }

    #[test]
    fn bounded_formatter_stops_incremental_display_at_the_capacity_limit() {
        struct Repeated<'a>(&'a Cell<usize>);
        impl fmt::Display for Repeated<'_> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                for _ in 0..1_000_000 {
                    self.0.set(self.0.get() + 1);
                    formatter.write_str("é漢")?;
                }
                Ok(())
            }
        }
        // Arrange
        let written = Cell::new(0);

        // Act
        let text = bounded_display(&Repeated(&written));

        // Assert
        assert!(written.get() <= DIAGNOSTIC_BYTES / "é漢".len() + 1);
        assert_bounded(&text, text.capacity(), DIAGNOSTIC_BYTES);
    }

    #[test]
    fn terminal_timeout_identifier_and_resource_variants_are_preserved() {
        for error in [
            CdpError::Closed,
            CdpError::SessionDetached,
            CdpError::ChannelDropped,
            CdpError::Timeout,
            CdpError::PageClosed,
            CdpError::IdentifierExhausted,
            CdpError::InvalidConfiguration("configuration sentinel"),
            CdpError::LimitExceeded {
                resource: "command",
                actual: 77,
                limit: 64,
            },
            CdpError::Resource(ResourceError::GlobalLimitExceeded {
                class: ResourceClass::RemoteHandle,
                current: 2,
                requested: 1,
                limit: 2,
            }),
        ] {
            // Arrange
            let discriminant = std::mem::discriminant(&error);
            let original = format!("{error:?}");

            // Act
            let bounded = bounded_error(error);

            // Assert
            assert_eq!(std::mem::discriminant(&bounded), discriminant);
            assert_eq!(format!("{bounded:?}"), original);
        }
    }
}
