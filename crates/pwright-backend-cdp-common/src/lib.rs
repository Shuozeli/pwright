//! Command-free primitives shared by independently installable CDP backend
//! leaves.
//!
//! This crate owns no browser capability, service, provider, process, runtime,
//! or transport policy.

use pwright_cdp::connection::CdpError;
use pwright_core::FailureKind;

/// Maps transport and protocol failures into the stable portable categories
/// shared by CDP capability adapters.
pub fn classify_cdp_failure(error: &CdpError) -> FailureKind {
    match error {
        CdpError::Timeout => FailureKind::Timeout,
        CdpError::WebSocket(_) | CdpError::Closed | CdpError::ChannelDropped => {
            FailureKind::TransportClosed
        }
        CdpError::PageClosed | CdpError::TabNotFound(_) => FailureKind::TargetClosed,
        CdpError::Protocol { message, .. } if protocol_reports_closed_target(message) => {
            FailureKind::TargetClosed
        }
        CdpError::Json(_) => FailureKind::ProtocolViolation,
        CdpError::LimitExceeded { .. } | CdpError::Resource(_) | CdpError::IdentifierExhausted => {
            FailureKind::ResourceExhausted
        }
        CdpError::Compound { .. } => FailureKind::Cleanup,
        CdpError::Protocol { .. }
        | CdpError::ElementNotFound { .. }
        | CdpError::NavigationFailed { .. }
        | CdpError::HttpFailed(_)
        | CdpError::JsException(_)
        | CdpError::InvalidConfiguration(_)
        | CdpError::Other(_) => FailureKind::Backend,
    }
}

fn protocol_reports_closed_target(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    [
        "no target with given id",
        "target closed",
        "session with given id not found",
        "no session with given id",
        "failed to find browser context",
        "failed to find context with id",
    ]
    .iter()
    .any(|needle| message.contains(needle))
}

#[cfg(test)]
mod tests {
    use pwright_cdp::connection::CdpError;
    use pwright_core::FailureKind;

    use super::classify_cdp_failure;

    #[test]
    fn classifies_closed_session_protocol_errors_as_target_closed() {
        // Arrange
        let error = CdpError::Protocol {
            code: -32_000,
            message: "Session with given id not found.".to_owned(),
        };

        // Act
        let kind = classify_cdp_failure(&error);

        // Assert
        assert_eq!(kind, FailureKind::TargetClosed);
    }

    #[test]
    fn keeps_unrelated_protocol_errors_at_the_backend_boundary() {
        // Arrange
        let error = CdpError::Protocol {
            code: -32_000,
            message: "Execution context was destroyed.".to_owned(),
        };

        // Act
        let kind = classify_cdp_failure(&error);

        // Assert
        assert_eq!(kind, FailureKind::Backend);
    }
}
