//! Bounded committed-selection receipts for select-option actions that navigate away.
//!
//! The fixed Runtime binding is shared with fill, but each action owns only a
//! temporary event cursor and a world-scoped token. No selected DOM node is retained.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use pwright_cap_locator::LocatorErrorDetails;
use pwright_cdp::connection::CdpError;
use pwright_cdp::{CdpClient, CdpEvent, CdpEventReceiver, CdpEventRecvError, CdpEventTryRecvError};
use pwright_core::{FailureKind, OperationError};
use serde_json::Value;

#[cfg(test)]
use super::MAX_SELECT_OPTION_RECEIPT_CODE_UNITS;
use super::{
    FILL_COMMIT_BINDING, MAX_SELECT_OPTION_ITEM_BYTES, MAX_SELECT_OPTION_ITEMS,
    MAX_SELECT_OPTION_RECEIPT_BYTES, MAX_SELECT_OPTION_TOTAL_BYTES, map_cdp_error,
};

const MAX_RECEIPT_ID_BYTES: usize = 1024;
const MAX_SCANNED_EVENTS: usize = 1024;
const RECEIPT_WAIT: Duration = Duration::from_millis(500);
static NEXT_RECEIPT_ID: AtomicU64 = AtomicU64::new(1);

pub(super) struct SelectOptionCommitReceipt {
    events: CdpEventReceiver,
    session_id: Option<String>,
    execution_context_id: i64,
    unique_context_id: Arc<str>,
    frame_id: Arc<str>,
    token: String,
}

impl SelectOptionCommitReceipt {
    pub(super) fn next_token(
        unique_context_id: &str,
    ) -> Result<String, OperationError<LocatorErrorDetails>> {
        if unique_context_id.is_empty() || unique_context_id.len() > MAX_RECEIPT_ID_BYTES {
            return Err(resource_error(
                "select-option commit context identity exceeds its byte limit",
            ));
        }
        let id = NEXT_RECEIPT_ID
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current.checked_add(1)
            })
            .map_err(|_| {
                resource_error("select-option commit receipt identifier space was exhausted")
            })?;
        Ok(format!("select-option:{unique_context_id}:{id}"))
    }

    pub(super) async fn prepare(
        client: &dyn CdpClient,
        execution_context_id: i64,
        unique_context_id: Arc<str>,
        frame_id: Arc<str>,
        token: &str,
    ) -> Result<Self, OperationError<LocatorErrorDetails>> {
        if execution_context_id <= 0
            || unique_context_id.is_empty()
            || unique_context_id.len() > MAX_RECEIPT_ID_BYTES
            || frame_id.is_empty()
            || frame_id.len() > MAX_RECEIPT_ID_BYTES
            || token.len() > MAX_RECEIPT_ID_BYTES + 35
        {
            return Err(resource_error(
                "select-option commit identity is invalid or too large",
            ));
        }
        let session_id = client.event_session_id().map(ToOwned::to_owned);
        if session_id
            .as_ref()
            .is_some_and(|id| id.len() > MAX_RECEIPT_ID_BYTES)
        {
            return Err(resource_error(
                "select-option commit session identity exceeds its byte limit",
            ));
        }
        // Subscribe before registering the fixed binding. Its CDP response
        // orders the subsequent evaluation after the binding is installed.
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
            unique_context_id,
            frame_id,
            token: token.to_owned(),
        })
    }

    /// Recover only after a committed-selection receipt followed by exact
    /// same-frame navigation or exact execution-context retirement. A missing,
    /// malformed, lagged, or closed stream preserves the CDP error.
    pub(super) async fn confirm_after_context_loss(mut self) -> Option<Vec<String>> {
        let deadline = tokio::time::Instant::now() + RECEIPT_WAIT;
        let mut committed_values = None;
        for _ in 0..MAX_SCANNED_EVENTS {
            let event = match self.events.try_recv() {
                Ok(event) => event,
                Err(CdpEventTryRecvError::Empty) => {
                    match tokio::time::timeout_at(deadline, self.events.recv()).await {
                        Ok(Ok(event)) => event,
                        Ok(Err(CdpEventRecvError::Lagged(_) | CdpEventRecvError::Closed))
                        | Err(_) => return None,
                    }
                }
                Err(CdpEventTryRecvError::Lagged(_) | CdpEventTryRecvError::Closed) => {
                    return None;
                }
            };
            if let Some(values) = self.values_if_exact_receipt(&event) {
                committed_values = Some(values);
            } else if committed_values.is_some() && self.is_exact_navigation_evidence(&event) {
                return committed_values;
            }
        }
        None
    }

    fn values_if_exact_receipt(&self, event: &CdpEvent) -> Option<Vec<String>> {
        if event.method != "Runtime.bindingCalled"
            || event.session_id.as_deref() != self.session_id.as_deref()
            || event.params.get("name").and_then(Value::as_str) != Some(FILL_COMMIT_BINDING)
            || event
                .params
                .get("executionContextId")
                .and_then(Value::as_i64)
                != Some(self.execution_context_id)
        {
            return None;
        }
        let payload = event.params.get("payload")?.as_str()?;
        if payload.len() > MAX_SELECT_OPTION_RECEIPT_BYTES {
            return None;
        }
        let mut parsed: Value = serde_json::from_str(payload).ok()?;
        let object = parsed.as_object_mut()?;
        if object.len() != 2 || object.get("token")?.as_str()? != self.token {
            return None;
        }
        let values = object.remove("values")?.as_array()?.to_owned();
        if values.len() > MAX_SELECT_OPTION_ITEMS {
            return None;
        }
        let mut total_bytes = 0usize;
        let mut output = Vec::with_capacity(values.len());
        for value in values {
            let Value::String(text) = value else {
                return None;
            };
            if text.len() > MAX_SELECT_OPTION_ITEM_BYTES {
                return None;
            }
            total_bytes = total_bytes.saturating_add(text.len());
            if total_bytes > MAX_SELECT_OPTION_TOTAL_BYTES {
                return None;
            }
            output.push(text);
        }
        Some(output)
    }

    fn is_exact_navigation_evidence(&self, event: &CdpEvent) -> bool {
        if event.session_id.as_deref() != self.session_id.as_deref() {
            return false;
        }
        match event.method.as_str() {
            "Page.frameNavigated" => {
                event.params.pointer("/frame/id").and_then(Value::as_str)
                    == Some(self.frame_id.as_ref())
            }
            "Runtime.executionContextDestroyed" => {
                event
                    .params
                    .get("executionContextId")
                    .and_then(Value::as_i64)
                    == Some(self.execution_context_id)
                    && event
                        .params
                        .get("executionContextUniqueId")
                        .and_then(Value::as_str)
                        == Some(self.unique_context_id.as_ref())
            }
            _ => false,
        }
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

    fn frame_navigated(frame_id: &str, session_id: Option<&str>) -> CdpEvent {
        CdpEvent::new(
            "Page.frameNavigated".to_owned(),
            json!({"frame": {"id": frame_id}}),
            session_id.map(ToOwned::to_owned),
        )
    }

    fn context_destroyed(context_id: i64, unique_id: &str) -> CdpEvent {
        CdpEvent::new(
            "Runtime.executionContextDestroyed".to_owned(),
            json!({
                "executionContextId": context_id,
                "executionContextUniqueId": unique_id,
            }),
            None,
        )
    }

    async fn prepare(client: &FakeCdpClient, token: &str) -> SelectOptionCommitReceipt {
        SelectOptionCommitReceipt::prepare(
            client,
            7,
            Arc::from("world"),
            Arc::from("frame-a"),
            token,
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn exact_receipt_followed_by_same_frame_navigation_recovers_selected_values() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let receipt = prepare(client.as_ref(), "select-option:world:1").await;
        client.send_event(receipt_event(
            7,
            r#"{"token":"select-option:world:1","values":["blue","green"]}"#,
            FILL_COMMIT_BINDING,
        ));
        client.send_event(frame_navigated("frame-a", None));

        // Act
        let result = receipt.confirm_after_context_loss().await;

        // Assert
        assert_eq!(result, Some(vec!["blue".to_owned(), "green".to_owned()]));
        assert_eq!(client.method_names(), vec!["Runtime.addBinding"]);
    }

    #[tokio::test]
    async fn exact_session_receipt_and_frame_navigation_recovers_selected_values() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let receipt = SelectOptionCommitReceipt {
            events: client.subscribe_composable_events().unwrap(),
            session_id: Some("selected-session".to_owned()),
            execution_context_id: 7,
            unique_context_id: Arc::from("world"),
            frame_id: Arc::from("frame-a"),
            token: "select-option:world:11".to_owned(),
        };
        client.send_event(CdpEvent::new(
            "Runtime.bindingCalled".to_owned(),
            json!({
                "name": FILL_COMMIT_BINDING,
                "executionContextId": 7,
                "payload": r#"{"token":"select-option:world:11","values":["blue"]}"#,
            }),
            Some("selected-session".to_owned()),
        ));
        client.send_event(frame_navigated("frame-a", Some("selected-session")));

        // Act
        let result = receipt.confirm_after_context_loss().await;

        // Assert
        assert_eq!(result, Some(vec!["blue".to_owned()]));
    }

    #[tokio::test(start_paused = true)]
    async fn wrong_context_action_or_binding_cannot_confirm_selection() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let receipt = prepare(client.as_ref(), "select-option:world:2").await;
        client.send_event(receipt_event(
            8,
            r#"{"token":"select-option:world:2","values":["blue"]}"#,
            FILL_COMMIT_BINDING,
        ));
        client.send_event(receipt_event(
            7,
            r#"{"token":"select-option:world:1","values":["blue"]}"#,
            FILL_COMMIT_BINDING,
        ));
        client.send_event(receipt_event(
            7,
            r#"{"token":"select-option:world:2","values":["blue"]}"#,
            "pageAuthoredBinding",
        ));
        client.send_event(frame_navigated("frame-a", None));

        // Act
        let result = receipt.confirm_after_context_loss().await;

        // Assert
        assert_eq!(result, None);
    }

    #[test]
    fn receipt_rejects_unbounded_or_malformed_values() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let receipt = SelectOptionCommitReceipt {
            events: client.subscribe_composable_events().unwrap(),
            session_id: None,
            execution_context_id: 7,
            unique_context_id: Arc::from("world"),
            frame_id: Arc::from("frame-a"),
            token: "select-option:world:3".to_owned(),
        };
        let oversized = "x".repeat(MAX_SELECT_OPTION_ITEM_BYTES + 1);
        let oversized_receipt = json!({
            "token": "select-option:world:3",
            "values": ["x".repeat(64_000), "x".repeat(64_000),
                "x".repeat(64_000), "x".repeat(64_000)],
        })
        .to_string();
        let events = [
            receipt_event(
                7,
                r#"{"token":"select-option:world:3","values":[null]}"#,
                FILL_COMMIT_BINDING,
            ),
            receipt_event(
                7,
                &json!({"token": "select-option:world:3", "values": [oversized]}).to_string(),
                FILL_COMMIT_BINDING,
            ),
            receipt_event(7, &oversized_receipt, FILL_COMMIT_BINDING),
        ];

        // Act
        let results = events.map(|event| receipt.values_if_exact_receipt(&event));

        // Assert
        assert!(oversized_receipt.len() > MAX_SELECT_OPTION_RECEIPT_BYTES);
        assert_eq!(results, [None, None, None]);
    }

    #[test]
    fn receipt_accepts_utf8_expansion_within_transport_bound() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let receipt = SelectOptionCommitReceipt {
            events: client.subscribe_composable_events().unwrap(),
            session_id: None,
            execution_context_id: 7,
            unique_context_id: Arc::from("world"),
            frame_id: Arc::from("frame-a"),
            token: "select-option:world:12".to_owned(),
        };
        let value = "\u{0800}".repeat(20_000);
        let payload = json!({
            "token": "select-option:world:12",
            "values": [value, value],
        })
        .to_string();
        let event = receipt_event(7, &payload, FILL_COMMIT_BINDING);

        // Act
        let result = receipt.values_if_exact_receipt(&event);

        // Assert
        assert!(payload.chars().count() < MAX_SELECT_OPTION_RECEIPT_CODE_UNITS);
        assert!(payload.len() > MAX_SELECT_OPTION_RECEIPT_CODE_UNITS);
        assert!(payload.len() <= MAX_SELECT_OPTION_RECEIPT_BYTES);
        assert_eq!(result, Some(vec![value.clone(), value]));
    }

    #[test]
    fn receipt_rejects_another_session_or_unexpected_fields() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let receipt = SelectOptionCommitReceipt {
            events: client.subscribe_composable_events().unwrap(),
            session_id: Some("selected-session".to_owned()),
            execution_context_id: 7,
            unique_context_id: Arc::from("world"),
            frame_id: Arc::from("frame-a"),
            token: "select-option:world:5".to_owned(),
        };
        let foreign = CdpEvent::new(
            "Runtime.bindingCalled".to_owned(),
            json!({
                "name": FILL_COMMIT_BINDING,
                "executionContextId": 7,
                "payload": r#"{"token":"select-option:world:5","values":["blue"]}"#,
            }),
            Some("foreign-session".to_owned()),
        );
        let extra_field = CdpEvent::new(
            "Runtime.bindingCalled".to_owned(),
            json!({
                "name": FILL_COMMIT_BINDING,
                "executionContextId": 7,
                "payload": r#"{"token":"select-option:world:5","values":["blue"],"extra":true}"#,
            }),
            Some("selected-session".to_owned()),
        );

        // Act
        let results = [
            receipt.values_if_exact_receipt(&foreign),
            receipt.values_if_exact_receipt(&extra_field),
        ];

        // Assert
        assert_eq!(results, [None, None]);
    }

    #[tokio::test]
    async fn unrelated_event_backlog_cannot_confirm_selection() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let receipt = prepare(client.as_ref(), "select-option:world:6").await;
        for _ in 0..=MAX_SCANNED_EVENTS {
            client.send_event(receipt_event(
                8,
                r#"{"token":"select-option:world:6","values":["blue"]}"#,
                FILL_COMMIT_BINDING,
            ));
        }

        // Act
        let result = receipt.confirm_after_context_loss().await;

        // Assert
        assert_eq!(result, None);
    }

    #[tokio::test(start_paused = true)]
    async fn missing_receipt_preserves_context_loss() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let receipt = prepare(client.as_ref(), "select-option:world:4").await;
        client.send_event(frame_navigated("frame-a", None));

        // Act
        let result = receipt.confirm_after_context_loss().await;

        // Assert
        assert_eq!(result, None);
    }

    #[tokio::test(start_paused = true)]
    async fn receipt_without_navigation_or_teardown_preserves_context_loss() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let receipt = prepare(client.as_ref(), "select-option:world:7").await;
        client.send_event(receipt_event(
            7,
            r#"{"token":"select-option:world:7","values":["blue"]}"#,
            FILL_COMMIT_BINDING,
        ));

        // Act
        let result = receipt.confirm_after_context_loss().await;

        // Assert
        assert_eq!(result, None);
    }

    #[tokio::test(start_paused = true)]
    async fn unrelated_frame_or_world_loss_cannot_confirm_selection() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let receipt = prepare(client.as_ref(), "select-option:world:8").await;
        client.send_event(receipt_event(
            7,
            r#"{"token":"select-option:world:8","values":["blue"]}"#,
            FILL_COMMIT_BINDING,
        ));
        client.send_event(frame_navigated("frame-b", None));
        client.send_event(frame_navigated("frame-a", Some("foreign-session")));
        client.send_event(context_destroyed(8, "world"));
        client.send_event(context_destroyed(7, "other-world"));
        client.send_event(CdpEvent::new(
            "Runtime.executionContextsCleared".to_owned(),
            json!({}),
            None,
        ));

        // Act
        let result = receipt.confirm_after_context_loss().await;

        // Assert
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn exact_world_teardown_after_receipt_recovers_selection() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let receipt = prepare(client.as_ref(), "select-option:world:9").await;
        client.send_event(receipt_event(
            7,
            r#"{"token":"select-option:world:9","values":["blue"]}"#,
            FILL_COMMIT_BINDING,
        ));
        client.send_event(context_destroyed(7, "world"));

        // Act
        let result = receipt.confirm_after_context_loss().await;

        // Assert
        assert_eq!(result, Some(vec!["blue".to_owned()]));
    }

    #[tokio::test(start_paused = true)]
    async fn navigation_before_receipt_cannot_confirm_selection() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let receipt = prepare(client.as_ref(), "select-option:world:10").await;
        client.send_event(frame_navigated("frame-a", None));
        client.send_event(receipt_event(
            7,
            r#"{"token":"select-option:world:10","values":["blue"]}"#,
            FILL_COMMIT_BINDING,
        ));

        // Act
        let result = receipt.confirm_after_context_loss().await;

        // Assert
        assert_eq!(result, None);
    }

    #[test]
    fn only_exact_missing_context_error_is_eligible() {
        // Arrange
        let exact = CdpError::Protocol {
            code: -32000,
            message: "Cannot find context with specified id".to_owned(),
        };
        let other = CdpError::Protocol {
            code: -32000,
            message: "Cannot find object with given id".to_owned(),
        };

        // Act
        let eligible = [
            is_navigation_context_loss(&exact),
            is_navigation_context_loss(&other),
        ];

        // Assert
        assert_eq!(eligible, [true, false]);
    }
}
