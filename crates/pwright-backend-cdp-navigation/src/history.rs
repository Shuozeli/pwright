//! Optional browser-history traversal using Chromium's native history IDs.
//!
//! This leaf reuses the navigation backend's bounded admission and lifecycle
//! machine, but is advertised as a separate installable capability.

use super::*;
use pwright_cap_history_navigation::{
    GoBack, GoBackParams, GoForward, GoForwardParams, Header as HistoryHeader,
    HistoryNavigationEntry, HistoryNavigationErrorDetails, HistoryNavigationResponse,
    HistoryNavigationResult, WaitUntil as HistoryWaitUntil,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HistoryDirection {
    Back,
    Forward,
}

impl Service<GoBackParams> for CdpNavigationService {
    type Response = HistoryNavigationResult;
    type Error = OperationError<HistoryNavigationErrorDetails>;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, params: GoBackParams) -> Self::Future<'_> {
        Box::pin(self.traverse_history(HistoryDirection::Back, params.wait_until))
    }
}

impl Service<GoForwardParams> for CdpNavigationService {
    type Response = HistoryNavigationResult;
    type Error = OperationError<HistoryNavigationErrorDetails>;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, params: GoForwardParams) -> Self::Future<'_> {
        Box::pin(self.traverse_history(HistoryDirection::Forward, params.wait_until))
    }
}

impl CommandCancellation<GoBack> for CdpNavigationService {
    type Future<'a> = Pin<
        Box<
            dyn Future<Output = Result<(), OperationError<HistoryNavigationErrorDetails>>>
                + Send
                + 'a,
        >,
    >;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        Box::pin(async move {
            self.client
                .page_stop_loading()
                .await
                .map_err(map_cdp_error)
                .map_err(map_history_error)
        })
    }
}

impl CommandCancellation<GoForward> for CdpNavigationService {
    type Future<'a> = Pin<
        Box<
            dyn Future<Output = Result<(), OperationError<HistoryNavigationErrorDetails>>>
                + Send
                + 'a,
        >,
    >;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        Box::pin(async move {
            self.client
                .page_stop_loading()
                .await
                .map_err(map_cdp_error)
                .map_err(map_history_error)
        })
    }
}

impl CdpNavigationService {
    async fn traverse_history(
        &self,
        direction: HistoryDirection,
        wait_until: HistoryWaitUntil,
    ) -> Result<HistoryNavigationResult, OperationError<HistoryNavigationErrorDetails>> {
        self.traverse_history_navigation(direction, wait_until)
            .await
            .map_err(map_history_error)
    }

    async fn traverse_history_navigation(
        &self,
        direction: HistoryDirection,
        wait_until: HistoryWaitUntil,
    ) -> Result<HistoryNavigationResult, OperationError<NavigationErrorDetails>> {
        let _resources = self.reserve_operation(&String::new(), None)?;
        let history = self
            .client
            .page_get_navigation_history()
            .await
            .map_err(map_cdp_error)?;
        let Some(target) = history_target(&history, direction, self.limits)? else {
            return Ok(HistoryNavigationResult { navigation: None });
        };
        let wait_until = navigation_wait_until(wait_until);
        let expected_session = self.client.event_session_id().map(ToOwned::to_owned);
        if let Some(session) = &expected_session {
            validate_navigation_value("CDP session ID", session, self.limits.native_id_bytes)?;
        }
        let (frame_id, old_loader) = {
            let frame = self
                .client
                .page_get_frame_tree()
                .await
                .map_err(map_cdp_error)?
                .frame;
            validate_native_id("history frame ID", &frame.id, self.limits.native_id_bytes)?;
            validate_native_id(
                "history previous loader ID",
                &frame.loader_id,
                self.limits.native_id_bytes,
            )?;
            (
                compact_navigation_string(frame.id),
                compact_navigation_string(frame.loader_id),
            )
        };
        let mut events = self
            .client
            .subscribe_composable_events()
            .map_err(map_event_subscription_error)?;
        self.client.page_enable().await.map_err(map_cdp_error)?;
        self.client
            .page_set_lifecycle_events_enabled(true)
            .await
            .map_err(map_cdp_error)?;
        self.client.network_enable().await.map_err(map_cdp_error)?;
        self.client
            .page_navigate_to_history_entry(target.id)
            .await
            .map_err(map_cdp_error)?;

        loop {
            let event = receive_event(&mut events).await?;
            if !event_matches_session(&event, expected_session.as_deref()) {
                continue;
            }
            if let Some(observed_url) = same_document_history_url(&event, &frame_id, self.limits)? {
                if observed_url != target.url {
                    continue;
                }
                return Ok(HistoryNavigationResult {
                    navigation: Some(HistoryNavigationEntry {
                        frame_id,
                        loader_id: None,
                        url: observed_url,
                        same_document: true,
                        response: None,
                    }),
                });
            }
            let Some((loader_id, requested_url)) =
                reload::new_document_identity(&event, &frame_id, &old_loader, self.limits)?
            else {
                continue;
            };
            let mut state = NewDocumentState::new(wait_until);
            apply_navigation_event(
                &mut state,
                event,
                &frame_id,
                &loader_id,
                wait_until,
                self.limits,
            )?;
            let result = wait_for_new_document_with_state(
                &mut events,
                NewDocumentWait {
                    expected_session: expected_session.as_deref(),
                    frame_id: &frame_id,
                    loader_id: &loader_id,
                    requested_url: &requested_url,
                    wait_until,
                    idle_delay: self.network_idle_delay,
                    limits: self.limits,
                },
                state,
            )
            .await?;
            return Ok(HistoryNavigationResult {
                navigation: Some(history_entry(result)),
            });
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HistoryTarget {
    id: i64,
    url: String,
}

fn history_target(
    value: &serde_json::Value,
    direction: HistoryDirection,
    limits: CdpNavigationLimits,
) -> Result<Option<HistoryTarget>, OperationError<NavigationErrorDetails>> {
    let current = value
        .get("currentIndex")
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| invalid_event("Page.getNavigationHistory is missing currentIndex"))?;
    let entries = value
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| invalid_event("Page.getNavigationHistory is missing entries"))?;
    if entries.len() > limits.inflight_requests {
        return Err(navigation_limit_error(
            "navigation history entry count",
            entries.len(),
            limits.inflight_requests,
        ));
    }
    if current < 0 {
        return Err(invalid_event(
            "Page.getNavigationHistory currentIndex is negative",
        ));
    }
    let current_index = usize::try_from(current)
        .map_err(|_| invalid_event("navigation history index is outside usize"))?;
    if current_index >= entries.len() {
        return Err(invalid_event("navigation history index is outside entries"));
    }
    let target_index = match direction {
        HistoryDirection::Back if current_index == 0 => return Ok(None),
        HistoryDirection::Back => current_index - 1,
        HistoryDirection::Forward if current_index + 1 == entries.len() => return Ok(None),
        HistoryDirection::Forward => current_index + 1,
    };
    let target = entries
        .get(target_index)
        .ok_or_else(|| invalid_event("navigation history index is outside entries"))?;
    let id = target
        .get("id")
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| invalid_event("navigation history entry is missing id"))?;
    if id < 0 {
        return Err(invalid_event("navigation history entry ID is negative"));
    }
    let url = target
        .get("url")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invalid_event("navigation history entry is missing url"))?;
    validate_navigation_value("navigation history URL", url, limits.url_bytes)?;
    Ok(Some(HistoryTarget {
        id,
        url: compact_navigation_string(url.to_owned()),
    }))
}

fn same_document_history_url(
    event: &CdpEvent,
    frame_id: &str,
    limits: CdpNavigationLimits,
) -> Result<Option<String>, OperationError<NavigationErrorDetails>> {
    if event.method != "Page.navigatedWithinDocument"
        || event
            .params
            .get("frameId")
            .and_then(serde_json::Value::as_str)
            != Some(frame_id)
    {
        return Ok(None);
    }
    let url = event
        .params
        .get("url")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| invalid_event("Page.navigatedWithinDocument is missing url"))?;
    validate_navigation_value("same-document history URL", url, limits.url_bytes)?;
    Ok(Some(compact_navigation_string(url.to_owned())))
}

fn navigation_wait_until(value: HistoryWaitUntil) -> WaitUntil {
    match value {
        HistoryWaitUntil::Commit => WaitUntil::Commit,
        HistoryWaitUntil::DomContentLoaded => WaitUntil::DomContentLoaded,
        HistoryWaitUntil::Load => WaitUntil::Load,
        HistoryWaitUntil::NetworkIdle => WaitUntil::NetworkIdle,
    }
}

fn history_entry(result: GotoResult) -> HistoryNavigationEntry {
    HistoryNavigationEntry {
        frame_id: result.frame_id,
        loader_id: result.loader_id,
        url: result.url,
        same_document: result.same_document,
        response: result.response.map(|response| HistoryNavigationResponse {
            url: response.url,
            status: response.status,
            status_text: response.status_text,
            headers: response
                .headers
                .into_iter()
                .map(|header| HistoryHeader {
                    name: header.name,
                    value: header.value,
                })
                .collect(),
        }),
    }
}

fn map_history_error(
    error: OperationError<NavigationErrorDetails>,
) -> OperationError<HistoryNavigationErrorDetails> {
    error.map_details(|details| match details {
        NavigationErrorDetails::InvalidBackendResponse(message) => {
            HistoryNavigationErrorDetails::InvalidBackendResponse(message)
        }
        NavigationErrorDetails::Network(message)
        | NavigationErrorDetails::UnsupportedOption(message)
        | NavigationErrorDetails::Backend(message) => {
            HistoryNavigationErrorDetails::Backend(message)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_target_selects_the_previous_native_entry() {
        // Arrange
        let history = serde_json::json!({
            "currentIndex": 2,
            "entries": [
                { "id": 10, "url": "https://example.test/one" },
                { "id": 11, "url": "https://example.test/two" },
                { "id": 12, "url": "https://example.test/three" }
            ]
        });

        // Act
        let target = history_target(
            &history,
            HistoryDirection::Back,
            CdpNavigationLimits::default(),
        );

        // Assert
        assert_eq!(
            target.unwrap(),
            Some(HistoryTarget {
                id: 11,
                url: "https://example.test/two".to_owned(),
            })
        );
    }

    #[test]
    fn history_target_returns_none_at_the_first_entry() {
        // Arrange
        let history = serde_json::json!({
            "currentIndex": 0,
            "entries": [{ "id": 10, "url": "https://example.test/one" }]
        });

        // Act
        let target = history_target(
            &history,
            HistoryDirection::Back,
            CdpNavigationLimits::default(),
        );

        // Assert
        assert_eq!(target.unwrap(), None);
    }

    #[test]
    fn history_target_rejects_a_negative_current_index() {
        // Arrange
        let history = serde_json::json!({
            "currentIndex": -1,
            "entries": [{ "id": 10, "url": "https://example.test/one" }]
        });

        // Act
        let target = history_target(
            &history,
            HistoryDirection::Back,
            CdpNavigationLimits::default(),
        );

        // Assert
        assert!(target.is_err());
    }

    #[test]
    fn history_target_rejects_a_negative_native_entry_id() {
        // Arrange
        let history = serde_json::json!({
            "currentIndex": 1,
            "entries": [
                { "id": -1, "url": "https://example.test/one" },
                { "id": 11, "url": "https://example.test/two" }
            ]
        });

        // Act
        let target = history_target(
            &history,
            HistoryDirection::Back,
            CdpNavigationLimits::default(),
        );

        // Assert
        assert!(target.is_err());
    }

    #[test]
    fn history_target_selects_the_next_native_entry() {
        // Arrange
        let history = serde_json::json!({
            "currentIndex": 1,
            "entries": [
                { "id": 10, "url": "https://example.test/one" },
                { "id": 11, "url": "https://example.test/two" },
                { "id": 12, "url": "https://example.test/three" }
            ]
        });

        // Act
        let target = history_target(
            &history,
            HistoryDirection::Forward,
            CdpNavigationLimits::default(),
        );

        // Assert
        assert_eq!(
            target.unwrap(),
            Some(HistoryTarget {
                id: 12,
                url: "https://example.test/three".to_owned(),
            })
        );
    }

    #[test]
    fn history_target_returns_none_at_the_last_entry() {
        // Arrange
        let history = serde_json::json!({
            "currentIndex": 1,
            "entries": [
                { "id": 10, "url": "https://example.test/one" },
                { "id": 11, "url": "https://example.test/two" }
            ]
        });

        // Act
        let target = history_target(
            &history,
            HistoryDirection::Forward,
            CdpNavigationLimits::default(),
        );

        // Assert
        assert_eq!(target.unwrap(), None);
    }

    #[test]
    fn history_target_rejects_an_out_of_range_current_index() {
        // Arrange
        let history = serde_json::json!({
            "currentIndex": 2,
            "entries": [
                { "id": 10, "url": "https://example.test/one" },
                { "id": 11, "url": "https://example.test/two" }
            ]
        });

        // Act
        let target = history_target(
            &history,
            HistoryDirection::Forward,
            CdpNavigationLimits::default(),
        );

        // Assert
        assert!(target.is_err());
    }

    #[test]
    fn same_document_history_event_preserves_hash_url() {
        // Arrange
        let event = CdpEvent::new(
            "Page.navigatedWithinDocument".to_owned(),
            serde_json::json!({
                "frameId": "frame-1",
                "url": "https://example.test/#/active"
            }),
            None,
        );

        // Act
        let url = same_document_history_url(&event, "frame-1", CdpNavigationLimits::default());

        // Assert
        assert_eq!(
            url.unwrap().as_deref(),
            Some("https://example.test/#/active")
        );
    }
}
