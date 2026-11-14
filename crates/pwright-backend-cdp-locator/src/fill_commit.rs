//! A direct-fill post-commit receipt for renderer navigation races.
//!
//! The fixed binding lives for one CDP target session, not for an action.
//! Every action's event cursor is dropped on completion or cancellation.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use pwright_cap_locator::LocatorErrorDetails;
use pwright_cdp::connection::CdpError;
use pwright_cdp::{CdpClient, CdpEvent, CdpEventReceiver, CdpEventRecvError, CdpEventTryRecvError};
use pwright_core::{FailureKind, OperationError};

use super::{FILL_COMMIT_BINDING, map_cdp_error};

const MAX_RECEIPT_ID_BYTES: usize = 1024;
const MAX_SCANNED_EVENTS: usize = 1024;
const RECEIPT_WAIT: Duration = Duration::from_millis(100);
// Services for the same utility world can be rebuilt while a prior action is
// settling. A process-wide, checked sequence avoids reusing its receipt ID.
static NEXT_RECEIPT_ID: AtomicU64 = AtomicU64::new(1);

pub(super) struct FillCommitReceipt {
    events: CdpEventReceiver,
    session_id: Option<String>,
    execution_context_id: i64,
    token: String,
}

impl FillCommitReceipt {
    pub(super) fn next_token(
        unique_context_id: &str,
    ) -> Result<String, OperationError<LocatorErrorDetails>> {
        if unique_context_id.is_empty() || unique_context_id.len() > MAX_RECEIPT_ID_BYTES {
            return Err(resource_error(
                "fill commit context identity exceeds its byte limit",
            ));
        }
        let id = NEXT_RECEIPT_ID
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current.checked_add(1)
            })
            .map_err(|_| resource_error("fill commit receipt identifier space was exhausted"))?;
        Ok(format!("{unique_context_id}:{id}"))
    }

    pub(super) async fn prepare(
        client: &dyn CdpClient,
        execution_context_id: i64,
        token: &str,
    ) -> Result<Self, OperationError<LocatorErrorDetails>> {
        if execution_context_id <= 0 || token.len() > MAX_RECEIPT_ID_BYTES + 21 {
            return Err(resource_error(
                "fill commit identity is invalid or too large",
            ));
        }
        let session_id = client.event_session_id().map(ToOwned::to_owned);
        if session_id
            .as_ref()
            .is_some_and(|id| id.len() > MAX_RECEIPT_ID_BYTES)
        {
            return Err(resource_error(
                "fill commit session identity exceeds its byte limit",
            ));
        }
        // Subscribe first. Binding registration is a CDP response barrier;
        // the following Runtime.evaluate cannot overtake it on this session.
        let events = client
            .subscribe_composable_events()
            .map_err(|error| resource_error(&error.to_string()))?;
        client
            .runtime_ensure_binding(FILL_COMMIT_BINDING)
            .await
            .map_err(map_cdp_error)?;
        Ok(Self {
            events,
            session_id,
            execution_context_id,
            token: token.to_owned(),
        })
    }

    /// The exact context-loss CDP response may overtake its binding event.
    /// Wait at most 100 ms; any missing, lagged, or closed stream keeps the
    /// original CDP context-loss error rather than replacing it.
    pub(super) async fn confirm_after_context_loss(mut self) -> bool {
        let deadline = tokio::time::Instant::now() + RECEIPT_WAIT;
        for _ in 0..MAX_SCANNED_EVENTS {
            let event = match self.events.try_recv() {
                Ok(event) => event,
                Err(CdpEventTryRecvError::Empty) => {
                    match tokio::time::timeout_at(deadline, self.events.recv()).await {
                        Ok(Ok(event)) => event,
                        Ok(Err(CdpEventRecvError::Lagged(_))) => {
                            return false;
                        }
                        Ok(Err(CdpEventRecvError::Closed)) => {
                            return false;
                        }
                        Err(_) => return false,
                    }
                }
                Err(CdpEventTryRecvError::Lagged(_)) => {
                    return false;
                }
                Err(CdpEventTryRecvError::Closed) => {
                    return false;
                }
            };
            if self.matches(&event) {
                return true;
            }
        }
        false
    }

    fn matches(&self, event: &CdpEvent) -> bool {
        event.method == "Runtime.bindingCalled"
            && event.session_id.as_deref() == self.session_id.as_deref()
            && event.params.get("name").and_then(|value| value.as_str())
                == Some(FILL_COMMIT_BINDING)
            && event
                .params
                .get("executionContextId")
                .and_then(|value| value.as_i64())
                == Some(self.execution_context_id)
            && event.params.get("payload").and_then(|value| value.as_str())
                == Some(self.token.as_str())
    }
}

pub(super) fn is_navigation_context_loss(error: &CdpError) -> bool {
    matches!(
        error,
        CdpError::Protocol { code: -32000, message }
            if message == "Cannot find context with specified id"
    )
}

fn resource_error(message: &str) -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        message,
        Some(LocatorErrorDetails::Backend(message.to_owned())),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_cdp::CdpEvent;
    use pwright_fake::FakeCdpClient;
    use serde_json::json;

    use super::*;

    fn receipt_event(context_id: i64, payload: &str, name: &str) -> CdpEvent {
        CdpEvent::new(
            "Runtime.bindingCalled".to_owned(),
            json!({
                "name": name,
                "executionContextId": context_id,
                "payload": payload,
            }),
            None,
        )
    }

    #[test]
    fn distinct_actions_in_one_world_receive_distinct_checked_tokens() {
        // Arrange
        let world = "utility-context-1";

        // Act
        let first = FillCommitReceipt::next_token(world).unwrap();
        let second = FillCommitReceipt::next_token(world).unwrap();

        // Assert
        assert_ne!(first, second);
        assert!(first.starts_with("utility-context-1:"));
        assert!(second.starts_with("utility-context-1:"));
    }

    #[tokio::test]
    async fn exact_post_commit_binding_confirms_lost_evaluation_context() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let receipt = FillCommitReceipt::prepare(client.as_ref(), 7, "unique:1")
            .await
            .unwrap();
        client.send_event(receipt_event(7, "unique:1", FILL_COMMIT_BINDING));

        // Act
        let confirmed = receipt.confirm_after_context_loss().await;

        // Assert
        assert!(confirmed);
        assert_eq!(client.method_names(), vec!["Runtime.addBinding"]);
    }

    #[tokio::test(start_paused = true)]
    async fn wrong_world_or_action_receipt_cannot_confirm_context_loss() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let receipt = FillCommitReceipt::prepare(client.as_ref(), 7, "unique:2")
            .await
            .unwrap();
        client.send_event(receipt_event(8, "unique:2", FILL_COMMIT_BINDING));
        client.send_event(receipt_event(7, "unique:1", FILL_COMMIT_BINDING));
        client.send_event(receipt_event(7, "unique:2", "pageAuthoredBinding"));

        // Act
        let confirmed = receipt.confirm_after_context_loss().await;

        // Assert
        assert!(!confirmed);
    }

    #[tokio::test]
    async fn another_cdp_session_cannot_confirm_context_loss() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let receipt = FillCommitReceipt::prepare(client.as_ref(), 7, "unique:2")
            .await
            .unwrap();
        let foreign = CdpEvent::new(
            "Runtime.bindingCalled".to_owned(),
            json!({
                "name": FILL_COMMIT_BINDING,
                "executionContextId": 7,
                "payload": "unique:2",
            }),
            Some("another-session".to_owned()),
        );

        // Act
        let matched = receipt.matches(&foreign);

        // Assert
        assert!(!matched);
    }

    #[tokio::test(start_paused = true)]
    async fn no_receipt_preserves_context_loss_as_failure() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let receipt = FillCommitReceipt::prepare(client.as_ref(), 7, "unique:3")
            .await
            .unwrap();

        // Act
        let confirmed = receipt.confirm_after_context_loss().await;

        // Assert
        assert!(!confirmed);
    }

    #[tokio::test]
    async fn lagged_receipt_stream_fails_closed() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let receipt = FillCommitReceipt::prepare(client.as_ref(), 7, "unique:4")
            .await
            .unwrap();
        for _ in 0..=MAX_SCANNED_EVENTS {
            client.send_event(receipt_event(8, "foreign", FILL_COMMIT_BINDING));
        }

        // Act
        let confirmed = receipt.confirm_after_context_loss().await;

        // Assert
        assert!(!confirmed);
    }

    #[test]
    fn only_the_exact_chromium_missing_context_error_is_eligible() {
        // Arrange
        let context_loss = CdpError::Protocol {
            code: -32000,
            message: "Cannot find context with specified id".to_owned(),
        };
        let other_failure = CdpError::Protocol {
            code: -32000,
            message: "Cannot find object with given id".to_owned(),
        };

        // Act
        let eligible = [
            is_navigation_context_loss(&context_loss),
            is_navigation_context_loss(&other_failure),
        ];

        // Assert
        assert_eq!(eligible, [true, false]);
    }
}
