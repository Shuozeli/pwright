//! Bounded marker-only diagnostics. Preserve typed terminal/resource/protocol
//! kinds, never infer successful cleanup from their diagnostic strings.
use pwright_cdp::connection::CdpError;
use std::fmt::{self, Write};

pub(super) const DIAGNOSTIC_BYTES: usize = 1024;
pub(super) fn bounded_text(value: &str) -> Box<str> {
    compact(value, DIAGNOSTIC_BYTES).into_boxed_str()
}
fn compact(value: &str, maximum: usize) -> String {
    let mut end = value.len().min(maximum);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    String::from(&value[..end])
}
pub(super) fn bounded_error(error: CdpError) -> CdpError {
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
            url: compact(&url, DIAGNOSTIC_BYTES / 2),
            reason: compact(&reason, DIAGNOSTIC_BYTES / 2),
        },
        CdpError::TabNotFound(value) => CdpError::TabNotFound(compact(&value, DIAGNOSTIC_BYTES)),
        CdpError::HttpFailed(value) => CdpError::HttpFailed(compact(&value, DIAGNOSTIC_BYTES)),
        CdpError::JsException(value) => CdpError::JsException(compact(&value, DIAGNOSTIC_BYTES)),
        CdpError::Other(value) => CdpError::Other(compact(&value, DIAGNOSTIC_BYTES)),
        error @ CdpError::Json(_) => CdpError::Other(bounded_display(&error)),
        CdpError::Compound { .. } => {
            CdpError::Other("compound CDP error during target marking".into())
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
fn bounded_display(value: &impl fmt::Display) -> String {
    struct Bounded(String);
    impl Write for Bounded {
        fn write_str(&mut self, text: &str) -> fmt::Result {
            let mut end = text.len().min(DIAGNOSTIC_BYTES - self.0.len());
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            self.0.push_str(&text[..end]);
            if end < text.len() {
                Err(fmt::Error)
            } else {
                Ok(())
            }
        }
    }
    let mut output = Bounded(String::with_capacity(DIAGNOSTIC_BYTES));
    let _ = write!(output, "{value}");
    output.0
}
