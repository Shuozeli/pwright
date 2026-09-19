//! Native reload primitive and typed command/cancellation service.
//! Unlike navigate, Page.reload returns no loader identity. Observe the first
//! new main-document request or commit, then use the shared lifecycle machine.

use super::*;
use pwright_cap_navigation::{Reload, ReloadParams};

impl Service<ReloadParams> for CdpNavigationService {
    type Response = GotoResult;
    type Error = OperationError<NavigationErrorDetails>;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, params: ReloadParams) -> Self::Future<'_> {
        Box::pin(self.reload(params.wait_until))
    }
}

impl CommandCancellation<Reload> for CdpNavigationService {
    type Future<'a> = Pin<
        Box<dyn Future<Output = Result<(), OperationError<NavigationErrorDetails>>> + Send + 'a>,
    >;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        Box::pin(async move { self.client.page_stop_loading().await.map_err(map_cdp_error) })
    }
}

impl CdpNavigationService {
    /// Reload the current document without synthesizing a URL navigation.
    ///
    /// The caller owns deadlines and cancellation (including stop-loading).
    /// This future shares the existing navigation admission/resource budget and
    /// owns one event cursor; it starts no task and never retries native input.
    pub async fn reload(
        &self,
        wait_until: WaitUntil,
    ) -> Result<GotoResult, OperationError<NavigationErrorDetails>> {
        let _resources = self.reserve_operation(&String::new(), None)?;
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
            validate_native_id("reload frame ID", &frame.id, self.limits.native_id_bytes)?;
            validate_native_id(
                "previous loader ID",
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
        self.client.page_reload().await.map_err(map_cdp_error)?;

        let (loader_id, requested_url, state) = loop {
            let event = receive_event(&mut events).await?;
            if !event_matches_session(&event, expected_session.as_deref()) {
                continue;
            }
            let Some((loader, url)) =
                new_document_identity(&event, &frame_id, &old_loader, self.limits)?
            else {
                continue;
            };
            let mut state = NewDocumentState::new(wait_until);
            apply_navigation_event(
                &mut state,
                event,
                &frame_id,
                &loader,
                wait_until,
                self.limits,
            )?;
            break (loader, url, state);
        };
        drop(old_loader);
        wait_for_new_document_with_state(
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
        .await
    }
}

fn new_document_identity(
    event: &CdpEvent,
    frame_id: &str,
    old_loader: &str,
    limits: CdpNavigationLimits,
) -> Result<Option<(String, String)>, OperationError<NavigationErrorDetails>> {
    let (id, loader, url) = match event.method.as_str() {
        "Network.requestWillBeSent" if event.params["type"].as_str() == Some("Document") => (
            &event.params["frameId"],
            &event.params["loaderId"],
            &event.params["request"]["url"],
        ),
        "Page.frameNavigated" => (
            &event.params["frame"]["id"],
            &event.params["frame"]["loaderId"],
            &event.params["frame"]["url"],
        ),
        _ => return Ok(None),
    };
    if id.as_str() != Some(frame_id) {
        return Ok(None);
    }
    let loader = loader
        .as_str()
        .ok_or_else(|| invalid_event("reload event is missing loaderId"))?;
    validate_native_id("reload loader ID", loader, limits.native_id_bytes)?;
    if loader == old_loader {
        return Ok(None);
    }
    let url = url
        .as_str()
        .ok_or_else(|| invalid_event("reload event is missing URL"))?;
    validate_navigation_value("reload URL", url, limits.url_bytes)?;
    Ok(Some((
        compact_navigation_string(loader.to_owned()),
        compact_navigation_string(url.to_owned()),
    )))
}

#[cfg(test)]
mod tests;
