//! Failure-only, bounded attribution for errors retained by the native driver.
//! `HostSession::close` joins that driver; its error need not originate in close.
use crate::HostError;

#[derive(Clone, Copy)]
pub(crate) enum Phase {
    Driver,
    Receive,
    Snapshot,
    PeerTransport,
    MouseRetirement,
    SessionRetirement,
    Transport,
}

impl Phase {
    fn label(self) -> &'static str {
        match self {
            Self::Driver => "native driver",
            Self::Receive => "topology receive",
            Self::Snapshot => "topology snapshot",
            Self::PeerTransport => "peer-loss transport close",
            Self::MouseRetirement => "mouse retirement",
            Self::SessionRetirement => "owned session retirement",
            Self::Transport => "transport close",
        }
    }

    pub(crate) fn annotate(self, error: HostError) -> HostError {
        match error {
            // Preserve the existing code, bound and UTF-8 validity. Other
            // classifications retain their exact structured values/messages.
            HostError::Backend(message) => {
                HostError::backend(format!("{}: {message}", self.label()))
            }
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_driver_error_identifies_the_original_phase() {
        // Arrange
        let original = HostError::backend(pwright_cdp::connection::CdpError::Timeout);
        // Act
        let error = Phase::Driver.annotate(Phase::Snapshot.annotate(original));
        // Assert
        assert_eq!(error.code(), "PWRIGHT_BACKEND");
        assert_eq!(
            error.to_string(),
            "native driver: topology snapshot: Timeout waiting for response"
        );
    }

    #[test]
    fn error_phase_does_not_reclassify_typed_failures() {
        // Arrange
        let original = HostError::Timeout(123);
        // Act
        let error = Phase::Driver.annotate(original);
        // Assert
        assert!(matches!(error, HostError::Timeout(123)));
        assert_eq!(error.code(), "PWRIGHT_TIMEOUT");
    }

    #[test]
    fn phase_and_message_share_the_existing_utf8_byte_limit() {
        // Arrange
        let original = HostError::backend("错误".repeat(4096));
        // Act
        let error = Phase::Driver.annotate(Phase::Receive.annotate(original));
        // Assert
        let message = error.to_string();
        assert!(message.starts_with("native driver: topology receive: "));
        assert!((4093..=4096).contains(&message.len()));
    }
}
