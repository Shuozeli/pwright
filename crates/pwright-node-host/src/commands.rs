//! Playwright operations translated into typed v2 commands, never JS/CDP RPCs.

use std::convert::Infallible;
use std::sync::atomic::Ordering;
use std::time::Duration;

use pwright_backend_cdp_topology::{CdpContextOwnership, CdpTopology};
use pwright_cap_dom_content::{DomContentParams, FrameContent, FrameTitle, TitleParams};
#[cfg(all(feature = "console-events", feature = "history-navigation"))]
use pwright_cap_history_navigation::{GoBack, GoForward};
#[cfg(feature = "console-events")]
use pwright_cap_navigation::Reload;
use pwright_cap_navigation::{Goto, GotoParams, WaitUntil};
use pwright_core::{
    CallId, CallMetadata, Command, CommandResult, CommandSpec, FailureKind, ObjectId,
    OperationError, Service,
};

use crate::{CommandRuntime, HostError, HostSession, context_metadata::ContextMetadata};

fn context_for_page(topology: &CdpTopology, page: ObjectId) -> Option<ObjectId> {
    topology
        .materialized_page_memberships()
        .find_map(|(candidate, context)| (candidate == page).then_some(context))
}

fn require_visible_context(
    topology: &CdpTopology,
    metadata: &ContextMetadata,
    context: ObjectId,
) -> Result<(), HostError> {
    let route = topology
        .browser_context_route(context)
        .ok_or(HostError::TargetClosed)?;
    metadata
        .visible(context, route.ownership() == CdpContextOwnership::Owned)?
        .ok_or(HostError::TargetClosed)?;
    Ok(())
}

fn require_visible_page(
    topology: &CdpTopology,
    metadata: &ContextMetadata,
    page: ObjectId,
    is_admitted_page_target: &impl Fn(ObjectId, Option<ObjectId>) -> Result<bool, HostError>,
) -> Result<(), HostError> {
    require_visible_page_target(topology, metadata, page, None, is_admitted_page_target)
}

fn require_visible_page_target(
    topology: &CdpTopology,
    metadata: &ContextMetadata,
    page: ObjectId,
    frame: Option<ObjectId>,
    is_admitted_page_target: &impl Fn(ObjectId, Option<ObjectId>) -> Result<bool, HostError>,
) -> Result<(), HostError> {
    let context = context_for_page(topology, page).ok_or(HostError::TargetClosed)?;
    require_visible_context(topology, metadata, context)?;
    if metadata.owned_contexts_only() && !is_admitted_page_target(page, frame)? {
        return Err(HostError::TargetClosed);
    }
    Ok(())
}

fn require_visible_frame(
    topology: &CdpTopology,
    metadata: &ContextMetadata,
    frame: ObjectId,
    is_admitted_page_target: &impl Fn(ObjectId, Option<ObjectId>) -> Result<bool, HostError>,
) -> Result<(), HostError> {
    let page = topology
        .page_object_id_for_frame(frame)
        .ok_or(HostError::TargetClosed)?;
    require_visible_page_target(
        topology,
        metadata,
        page,
        Some(frame),
        is_admitted_page_target,
    )
}

/// Unknown IDs may be Browser or handle objects and remain the runtime's
/// responsibility. Known Context/Page/Frame IDs from a fresh owned Browser
/// must belong to a published owned Context and, for Page/Frame targets, a
/// published non-internal Page before any command is admitted.
fn require_visible_operation_target(
    topology: &CdpTopology,
    metadata: &ContextMetadata,
    object: ObjectId,
    is_admitted_page_target: &impl Fn(ObjectId, Option<ObjectId>) -> Result<bool, HostError>,
) -> Result<(), HostError> {
    if !metadata.owned_contexts_only() {
        return Ok(());
    }
    if topology.browser_context_route(object).is_some() {
        return require_visible_context(topology, metadata, object);
    }
    if context_for_page(topology, object).is_some() {
        return require_visible_page(topology, metadata, object, is_admitted_page_target);
    }
    if topology.page_object_id_for_frame(object).is_some() {
        return require_visible_frame(topology, metadata, object, is_admitted_page_target);
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct NavigationResult {
    pub url: String,
    pub response: Option<ResponseMetadata>,
}

#[derive(Clone, Debug)]
pub struct ResponseMetadata {
    pub request_id: Option<String>,
    #[cfg(feature = "response-request")]
    pub request: Option<std::sync::Arc<crate::network_events::RequestHistoryOwner>>,
    #[cfg(feature = "navigation-response-finished")]
    pub finished_owner: Option<std::sync::Arc<crate::network_events::ResponseFinishedOwner>>,
    /// Outer None means this navigation had no exact network Response match;
    /// Some(None) means a matched HTTP Response had no TLS details.
    #[cfg(feature = "response-security-details")]
    pub security_details: Option<Option<crate::network_events::CdpResponseSecurityDetails>>,
    pub url: String,
    pub status: i64,
    pub status_text: String,
    pub ok: bool,
    pub headers: Vec<(String, String)>,
}

impl NavigationResult {
    #[cfg(feature = "response-request")]
    pub fn response_request_foreign_copy_bytes(&self) -> u64 {
        self.response
            .as_ref()
            .and_then(|response| response.request.as_ref())
            .map_or(0, |request| request.foreign_copy_bytes())
    }
}

impl HostError {
    pub(crate) fn operation<D>(error: OperationError<D>, timeout: u32) -> Self {
        match error.kind() {
            FailureKind::Timeout => Self::Timeout(timeout),
            FailureKind::TargetClosed | FailureKind::TransportClosed
                if error.message().eq_ignore_ascii_case("Frame was detached") =>
            {
                Self::FrameDetached
            }
            FailureKind::TargetClosed | FailureKind::TransportClosed => Self::TargetClosed,
            FailureKind::ResourceExhausted => Self::ResourceExhausted(error.message().to_owned()),
            FailureKind::UnsupportedCapability => {
                Self::UnsupportedOperation(error.message().to_owned())
            }
            FailureKind::Cancelled
            | FailureKind::BrowserCrashed
            | FailureKind::ProtocolViolation
            | FailureKind::Backend
            | FailureKind::Cleanup => Self::backend(error),
        }
    }
}

impl HostSession {
    fn page_target_is_admitted(
        &self,
        topology: &CdpTopology,
        page: ObjectId,
        frame: Option<ObjectId>,
    ) -> Result<bool, HostError> {
        if self.frame_trees.is_published_public_page(page)? {
            return Ok(true);
        }
        #[cfg(feature = "context-storage-state")]
        {
            crate::context_storage_state::permits_internal_page_target(self, topology, page, frame)
        }
        #[cfg(not(feature = "context-storage-state"))]
        {
            let _ = (topology, frame);
            Ok(false)
        }
    }

    pub(crate) fn admit_operation(
        &self,
        id: &str,
    ) -> Result<
        (
            ObjectId,
            CallId,
            tokio_util::task::task_tracker::TaskTrackerToken,
        ),
        HostError,
    > {
        let object_id = id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable object ID".into()))?;
        let state = self.state.lock().map_err(HostError::backend)?;
        if !self.is_connected() || state.is_none() {
            return Err(HostError::TargetClosed);
        }
        if self.context_metadata.owned_contexts_only() {
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            require_visible_operation_target(
                topology.topology(),
                &self.context_metadata,
                object_id,
                &|page, frame| self.page_target_is_admitted(topology.topology(), page, frame),
            )?;
        }
        let call_id = self
            .next_call
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map(CallId::new)
            .map_err(|_| HostError::ResourceExhausted("call ID space exhausted".into()))?;
        Ok((object_id, call_id, self.operations.token()))
    }

    pub(crate) fn require_visible_page_id(&self, page: ObjectId) -> Result<(), HostError> {
        if !self.context_metadata.owned_contexts_only() {
            return Ok(());
        }
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        let topology = live.composition.topology().map_err(HostError::backend)?;
        require_visible_page(
            topology.topology(),
            &self.context_metadata,
            page,
            &|page, frame| self.page_target_is_admitted(topology.topology(), page, frame),
        )
    }

    #[cfg(any(
        feature = "console-events",
        feature = "timeout-settings",
        feature = "page-routes",
        feature = "context-bindings",
        feature = "init-scripts",
        feature = "download-events",
    ))]
    pub(crate) fn require_visible_context_id(&self, context: ObjectId) -> Result<(), HostError> {
        if !self.context_metadata.owned_contexts_only() {
            return Ok(());
        }
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        let topology = live.composition.topology().map_err(HostError::backend)?;
        require_visible_context(topology.topology(), &self.context_metadata, context)
    }

    pub(crate) fn require_visible_frame_id(&self, frame: ObjectId) -> Result<(), HostError> {
        if !self.context_metadata.owned_contexts_only() {
            return Ok(());
        }
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        let topology = live.composition.topology().map_err(HostError::backend)?;
        require_visible_frame(
            topology.topology(),
            &self.context_metadata,
            frame,
            &|page, frame| self.page_target_is_admitted(topology.topology(), page, frame),
        )
    }

    pub(crate) async fn invoke<C>(
        &self,
        id: &str,
        params: C::Params,
        timeout_ms: u32,
    ) -> Result<C::Result, HostError>
    where
        C: CommandSpec,
        CommandRuntime: Service<Command<C>, Response = CommandResult<C>, Error = Infallible>,
    {
        self.invoke_with_error::<C>(id, params, timeout_ms, HostError::operation)
            .await
    }

    pub(crate) async fn invoke_with_error<C>(
        &self,
        id: &str,
        params: C::Params,
        timeout_ms: u32,
        map_error: fn(OperationError<C::ErrorDetails>, u32) -> HostError,
    ) -> Result<C::Result, HostError>
    where
        C: CommandSpec,
        CommandRuntime: Service<Command<C>, Response = CommandResult<C>, Error = Infallible>,
    {
        let (object_id, call_id, _operation) = self.admit_operation(id)?;
        #[cfg(feature = "console-events")]
        let navigation_method = C::METHOD == Goto::METHOD || C::METHOD == Reload::METHOD;
        #[cfg(all(feature = "console-events", feature = "history-navigation"))]
        let navigation_method =
            navigation_method || C::METHOD == GoBack::METHOD || C::METHOD == GoForward::METHOD;
        #[cfg(feature = "console-events")]
        if !navigation_method {
            self.activate_console_subscriptions()?;
        }
        let runtime = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            live.runtime.clone()
        };
        let timeout = (timeout_ms != 0).then(|| Duration::from_millis(u64::from(timeout_ms)));
        let command =
            Command::<C>::new(call_id, object_id, CallMetadata::new(timeout, None), params);
        // Environment shutdown must abandon the controlled command while CDP
        // is still alive. Its drop guard schedules backend cancellation, and
        // the host drains that cleanup before closing the native transport.
        let outcome = tokio::select! {
            biased;
            () = self.stop.cancelled() => return Err(HostError::TargetClosed),
            outcome = runtime.call(command) => outcome,
        };
        let result = match outcome {
            Ok(result) => result,
            Err(never) => match never {},
        };
        result
            .into_outcome()
            .map_err(|error| map_error(error, timeout_ms))
    }

    pub async fn frame_title(&self, frame_id: &str) -> Result<String, HostError> {
        let result = self.invoke::<FrameTitle>(frame_id, TitleParams, 0).await?;
        Ok(result.as_str().to_owned())
    }

    pub async fn frame_content(&self, frame_id: &str) -> Result<String, HostError> {
        let result = self
            .invoke::<FrameContent>(frame_id, DomContentParams, 0)
            .await?;
        Ok(result.as_str().to_owned())
    }

    pub async fn page_goto(
        &self,
        page_id: &str,
        url: String,
        wait_until: &str,
        referrer: Option<String>,
        timeout_ms: u32,
    ) -> Result<NavigationResult, HostError> {
        let wait_until = parse_wait_until(wait_until)?;
        #[cfg(feature = "extra-headers")]
        let mut header_referrer = self.navigation_referrer(page_id, referrer)?;
        #[cfg(feature = "extra-headers")]
        let referrer = header_referrer.value.take();
        self.page_goto_prepared(page_id, url, wait_until, referrer, timeout_ms)
            .await
    }

    /// Capture Page-only referer policy in invocation order, before a foreign
    /// executor can schedule this navigation behind a later header setter.
    #[cfg(feature = "extra-headers")]
    pub fn start_page_goto(
        self: &std::sync::Arc<Self>,
        page_id: String,
        url: String,
        wait_until: String,
        referrer: Option<String>,
        timeout_ms: u32,
    ) -> impl Future<Output = Result<NavigationResult, HostError>> + Send + 'static + use<> {
        let prepared = parse_wait_until(&wait_until).and_then(|wait| {
            self.navigation_referrer(&page_id, referrer)
                .map(|headers| (wait, headers))
        });
        let host = self.clone();
        async move {
            let (wait, mut headers) = prepared?;
            host.page_goto_prepared(&page_id, url, wait, headers.value.take(), timeout_ms)
                .await
        }
    }

    async fn page_goto_prepared(
        &self,
        page_id: &str,
        url: String,
        wait_until: WaitUntil,
        referrer: Option<String>,
        timeout_ms: u32,
    ) -> Result<NavigationResult, HostError> {
        let url = self.resolve_page_url(page_id, &url)?;
        #[cfg(feature = "frame-navigation-burst-20")]
        let deadline = (timeout_ms != 0)
            .then(|| tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms)));
        let result = self
            .invoke::<Goto>(
                page_id,
                GotoParams {
                    url,
                    referrer,
                    wait_until,
                },
                timeout_ms,
            )
            .await;
        // The navigation driver and topology share one native stream but
        // distinct bounded cursors. Publish its already-queued URL events
        // before resolving goto; never infer state from the requested URL.
        #[cfg(feature = "frame-navigation-burst-20")]
        let publication = self.publish_navigation_topology(deadline, timeout_ms);
        #[cfg(not(feature = "frame-navigation-burst-20"))]
        let publication = self.flush_topology();
        let result =
            crate::completion::publish_command_result(result, self.is_connected(), publication)
                .await?;
        let request_id = result.loader_id;
        #[cfg(feature = "response-request")]
        let request = if result.response.is_some() {
            match request_id.as_deref() {
                Some(request_id) => Some(self.request_by_native_id(page_id, request_id).await?),
                None => None,
            }
        } else {
            None
        };
        #[cfg(feature = "navigation-response-finished")]
        let finished_owner = match (
            request.as_ref(),
            request_id.as_deref(),
            result.response.as_ref(),
        ) {
            (Some(request), Some(request_id), Some(response)) => {
                request
                    .navigation_finished_owner(request_id, &response.url, response.status)
                    .await?
            }
            _ => None,
        };
        #[cfg(feature = "response-security-details")]
        let security_details = match (
            request.as_ref(),
            request_id.as_deref(),
            result.response.as_ref(),
        ) {
            (Some(request), Some(request_id), Some(response)) => request
                .observed_navigation_response(request_id, &response.url, response.status)
                .await?
                .map(|response| response.security_details),
            _ => None,
        };
        Ok(NavigationResult {
            url: result.url,
            response: result.response.map(|response| ResponseMetadata {
                request_id,
                #[cfg(feature = "response-request")]
                request,
                #[cfg(feature = "navigation-response-finished")]
                finished_owner,
                #[cfg(feature = "response-security-details")]
                security_details,
                url: response.url,
                status: response.status,
                status_text: response.status_text,
                ok: (200..=299).contains(&response.status),
                headers: response
                    .headers
                    .into_iter()
                    .map(|header| (header.name.to_ascii_lowercase(), header.value))
                    .collect(),
            }),
        })
    }

    pub(crate) fn page_context_snapshot(
        &self,
        page_id: &str,
    ) -> Result<crate::context_metadata::ContextSnapshot, HostError> {
        let page = page_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable object ID".into()))?;
        {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let topology = topology.topology();
            let context = topology
                .page_session_routes()
                .find(|route| route.object_id() == page)
                .map(|route| route.browser_context_object_id())
                .ok_or(HostError::TargetClosed)?;
            let owned = topology
                .browser_context_routes()
                .find(|route| route.object_id() == context)
                .map(|route| route.ownership() == CdpContextOwnership::Owned)
                .ok_or(HostError::TargetClosed)?;
            self.context_metadata.visible(context, owned)?
        }
        .ok_or(HostError::TargetClosed)
    }

    #[cfg(feature = "page-routes")]
    pub(crate) fn context_snapshot(
        &self,
        context_id: &str,
    ) -> Result<crate::context_metadata::ContextSnapshot, HostError> {
        let context = context_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Context ID".into()))?;
        {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let owned = topology
                .topology()
                .browser_context_routes()
                .find(|route| route.object_id() == context)
                .map(|route| route.ownership() == CdpContextOwnership::Owned)
                .ok_or(HostError::TargetClosed)?;
            self.context_metadata.visible(context, owned)?
        }
        .ok_or(HostError::TargetClosed)
    }

    pub(crate) fn resolve_page_url(&self, page_id: &str, input: &str) -> Result<String, HostError> {
        self.page_context_snapshot(page_id)?.resolve_url(input)
    }
}

pub(crate) fn parse_wait_until(value: &str) -> Result<WaitUntil, HostError> {
    match value {
        "commit" => Ok(WaitUntil::Commit),
        "domcontentloaded" => Ok(WaitUntil::DomContentLoaded),
        "load" => Ok(WaitUntil::Load),
        "networkidle" | "networkidle0" => Ok(WaitUntil::NetworkIdle),
        _ => Err(HostError::InvalidArgument(
            "waitUntil: expected one of (load|domcontentloaded|networkidle|commit)".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_backend_cdp_topology::{
        CdpFrameAttachment, CdpPageObjectIds, CdpPageTarget, CdpTopologyLimits, ConnectionEpoch,
    };

    const EPOCH: ConnectionEpoch = ConnectionEpoch::new(1);

    #[test]
    fn navigation_wait_until_accepts_undocumented_networkidle0_alias() {
        // Arrange
        let alias = "networkidle0";

        // Act
        let parsed = parse_wait_until(alias);

        // Assert
        assert_eq!(parsed.unwrap(), WaitUntil::NetworkIdle);
    }

    #[test]
    fn navigation_wait_until_rejects_networkidle2_with_pinned_enum_error() {
        // Arrange
        let invalid = "networkidle2";

        // Act
        let parsed = parse_wait_until(invalid);

        // Assert
        assert!(matches!(
            parsed,
            Err(HostError::InvalidArgument(message))
                if message == "waitUntil: expected one of (load|domcontentloaded|networkidle|commit)"
        ));
    }

    fn topology_with_default_and_owned_pages() -> CdpTopology {
        let mut topology = CdpTopology::new(
            ObjectId::new(1),
            EPOCH,
            CdpTopologyLimits::new(32, 4, 4, 8, 4, 64),
        )
        .unwrap();
        topology
            .register_default_context(EPOCH, ObjectId::new(2))
            .unwrap();
        topology
            .attach_page(
                EPOCH,
                CdpPageTarget::new("default-page", "default-session", ""),
                CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
            )
            .unwrap();
        topology
            .register_context(
                EPOCH,
                "owned-context",
                ObjectId::new(5),
                CdpContextOwnership::Owned,
            )
            .unwrap();
        topology
            .attach_page(
                EPOCH,
                CdpPageTarget::new("owned-page", "owned-session", "owned-context"),
                CdpPageObjectIds::new(ObjectId::new(6), ObjectId::new(7)),
            )
            .unwrap();
        topology
            .attach_frame(
                EPOCH,
                CdpFrameAttachment::local("owned-page", "owned-child", "owned-page"),
                ObjectId::new(8),
            )
            .unwrap();
        topology
            .attach_page(
                EPOCH,
                CdpPageTarget::new("scratch-page", "scratch-session", "owned-context"),
                CdpPageObjectIds::new(ObjectId::new(9), ObjectId::new(10)),
            )
            .unwrap();
        topology
            .attach_frame(
                EPOCH,
                CdpFrameAttachment::local("scratch-page", "scratch-child", "scratch-page"),
                ObjectId::new(11),
            )
            .unwrap();
        topology
    }

    fn published_test_page(page: ObjectId, _frame: Option<ObjectId>) -> Result<bool, HostError> {
        Ok(page == ObjectId::new(6))
    }

    fn metadata(owned_contexts_only: bool) -> ContextMetadata {
        let metadata = ContextMetadata::new(crate::context_metadata::ContextMetadataLimits {
            active_contexts: 4,
            states: 8,
            conversions: 4,
            base_url_bytes: 128,
            bytes: 4096,
        })
        .unwrap();
        let metadata = if owned_contexts_only {
            metadata.with_owned_contexts_only().unwrap()
        } else {
            metadata
        };
        metadata
            .reserve_creation(None)
            .unwrap()
            .commit(ObjectId::new(5))
            .unwrap();
        metadata
    }

    #[test]
    fn fresh_browser_rejects_default_context_page_and_frame_operation_ids() {
        // Arrange
        let topology = topology_with_default_and_owned_pages();
        let metadata = metadata(true);

        // Act
        let results = [2, 3, 4].map(|id| {
            require_visible_operation_target(
                &topology,
                &metadata,
                ObjectId::new(id),
                &published_test_page,
            )
        });

        // Assert
        assert!(
            results
                .iter()
                .all(|result| matches!(result, Err(HostError::TargetClosed)))
        );
    }

    #[test]
    fn fresh_browser_admits_created_context_and_page_frame_descendants() {
        // Arrange
        let topology = topology_with_default_and_owned_pages();
        let metadata = metadata(true);

        // Act
        let results = [5, 6, 7, 8].map(|id| {
            require_visible_operation_target(
                &topology,
                &metadata,
                ObjectId::new(id),
                &published_test_page,
            )
        });

        // Assert
        assert!(results.iter().all(Result::is_ok));
    }

    #[test]
    fn fresh_browser_frame_read_requires_a_visible_page() {
        // Arrange
        let topology = topology_with_default_and_owned_pages();
        let metadata = metadata(true);

        // Act
        let hidden =
            require_visible_page(&topology, &metadata, ObjectId::new(3), &published_test_page);
        let owned =
            require_visible_page(&topology, &metadata, ObjectId::new(6), &published_test_page);

        // Assert
        assert!(matches!(hidden, Err(HostError::TargetClosed)));
        assert!(owned.is_ok());
    }

    #[test]
    fn fresh_browser_direct_context_entrypoints_reject_default_and_admit_created_context() {
        // Arrange
        let topology = topology_with_default_and_owned_pages();
        let metadata = metadata(true);

        // Act
        let hidden = require_visible_context(&topology, &metadata, ObjectId::new(2));
        let created = require_visible_context(&topology, &metadata, ObjectId::new(5));

        // Assert
        assert!(matches!(hidden, Err(HostError::TargetClosed)));
        assert!(created.is_ok());
    }

    #[test]
    fn attached_and_persistent_direct_context_entrypoints_keep_default_context() {
        // Arrange
        let topology = topology_with_default_and_owned_pages();
        let metadata = metadata(false);

        // Act
        let default = require_visible_context(&topology, &metadata, ObjectId::new(2));

        // Assert
        assert!(default.is_ok());
    }

    #[test]
    fn fresh_browser_rejects_unpublished_owned_context_and_descendant_ids() {
        // Arrange
        let topology = topology_with_default_and_owned_pages();
        let metadata = metadata(true);
        metadata.remove(ObjectId::new(5)).unwrap();

        // Act
        let context = require_visible_context(&topology, &metadata, ObjectId::new(5));
        let page =
            require_visible_page(&topology, &metadata, ObjectId::new(6), &published_test_page);
        let frame =
            require_visible_frame(&topology, &metadata, ObjectId::new(7), &published_test_page);
        let generic = require_visible_operation_target(
            &topology,
            &metadata,
            ObjectId::new(6),
            &published_test_page,
        );

        // Assert
        assert!(matches!(context, Err(HostError::TargetClosed)));
        assert!(matches!(page, Err(HostError::TargetClosed)));
        assert!(matches!(frame, Err(HostError::TargetClosed)));
        assert!(matches!(generic, Err(HostError::TargetClosed)));
    }

    #[test]
    fn fresh_browser_direct_frame_entrypoints_reject_default_and_admit_owned_descendants() {
        // Arrange
        let topology = topology_with_default_and_owned_pages();
        let metadata = metadata(true);

        // Act
        let hidden =
            require_visible_frame(&topology, &metadata, ObjectId::new(4), &published_test_page);
        let owned_main =
            require_visible_frame(&topology, &metadata, ObjectId::new(7), &published_test_page);
        let owned_child =
            require_visible_frame(&topology, &metadata, ObjectId::new(8), &published_test_page);

        // Assert
        assert!(matches!(hidden, Err(HostError::TargetClosed)));
        assert!(owned_main.is_ok());
        assert!(owned_child.is_ok());
    }

    #[test]
    fn fresh_browser_rejects_scratch_page_and_frame_ids_inside_a_visible_context() {
        // Arrange
        let topology = topology_with_default_and_owned_pages();
        let metadata = metadata(true);
        assert!(require_visible_context(&topology, &metadata, ObjectId::new(5)).is_ok());

        // Act
        let direct_page =
            require_visible_page(&topology, &metadata, ObjectId::new(9), &published_test_page);
        let direct_frame = require_visible_frame(
            &topology,
            &metadata,
            ObjectId::new(10),
            &published_test_page,
        );
        let generic = [9, 10, 11].map(|id| {
            require_visible_operation_target(
                &topology,
                &metadata,
                ObjectId::new(id),
                &published_test_page,
            )
        });

        // Assert
        assert!(matches!(direct_page, Err(HostError::TargetClosed)));
        assert!(matches!(direct_frame, Err(HostError::TargetClosed)));
        assert!(
            generic
                .iter()
                .all(|result| matches!(result, Err(HostError::TargetClosed)))
        );
    }

    #[test]
    fn attached_and_persistent_direct_frame_entrypoints_keep_default_frame() {
        // Arrange
        let topology = topology_with_default_and_owned_pages();
        let metadata = metadata(false);

        // Act
        let default =
            require_visible_frame(&topology, &metadata, ObjectId::new(4), &published_test_page);

        // Assert
        assert!(default.is_ok());
    }

    #[test]
    fn attached_and_persistent_page_admission_does_not_require_owned_publication() {
        // Arrange
        let topology = topology_with_default_and_owned_pages();
        let metadata = metadata(false);
        let no_owned_publication = |_page, _frame| Ok(false);

        // Act
        let page = require_visible_page(
            &topology,
            &metadata,
            ObjectId::new(3),
            &no_owned_publication,
        );
        let frame = require_visible_frame(
            &topology,
            &metadata,
            ObjectId::new(4),
            &no_owned_publication,
        );

        // Assert
        assert!(page.is_ok());
        assert!(frame.is_ok());
    }

    #[test]
    fn fresh_browser_admission_keeps_page_ownership_during_reconnect() {
        // Arrange
        let mut topology = topology_with_default_and_owned_pages();
        let metadata = metadata(true);
        topology.begin_reconnect(ConnectionEpoch::new(2)).unwrap();
        assert_eq!(topology.page_session_routes().count(), 0);

        // Act
        let hidden_page = require_visible_operation_target(
            &topology,
            &metadata,
            ObjectId::new(3),
            &published_test_page,
        );
        let hidden_frame = require_visible_operation_target(
            &topology,
            &metadata,
            ObjectId::new(4),
            &published_test_page,
        );
        let owned_page = require_visible_operation_target(
            &topology,
            &metadata,
            ObjectId::new(6),
            &published_test_page,
        );
        let owned_frame = require_visible_operation_target(
            &topology,
            &metadata,
            ObjectId::new(7),
            &published_test_page,
        );

        // Assert
        assert!(matches!(hidden_page, Err(HostError::TargetClosed)));
        assert!(matches!(hidden_frame, Err(HostError::TargetClosed)));
        assert!(owned_page.is_ok());
        assert!(owned_frame.is_ok());
    }

    #[test]
    fn fresh_browser_reconnect_does_not_expose_scratch_frame_ids() {
        // Arrange
        let mut topology = topology_with_default_and_owned_pages();
        let metadata = metadata(true);
        topology.begin_reconnect(ConnectionEpoch::new(2)).unwrap();

        // Act
        let results = [9, 10, 11].map(|id| {
            require_visible_operation_target(
                &topology,
                &metadata,
                ObjectId::new(id),
                &published_test_page,
            )
        });

        // Assert
        assert!(
            results
                .iter()
                .all(|result| matches!(result, Err(HostError::TargetClosed)))
        );
    }

    #[test]
    fn attached_and_persistent_visibility_admit_default_objects() {
        // Arrange
        let topology = topology_with_default_and_owned_pages();
        let metadata = metadata(false);

        // Act
        let results = [2, 3, 4].map(|id| {
            require_visible_operation_target(
                &topology,
                &metadata,
                ObjectId::new(id),
                &published_test_page,
            )
        });

        // Assert
        assert!(results.iter().all(Result::is_ok));
    }

    #[test]
    fn fresh_browser_leaves_browser_and_other_object_ids_to_runtime() {
        // Arrange
        let topology = topology_with_default_and_owned_pages();
        let metadata = metadata(true);

        // Act
        let browser = require_visible_operation_target(
            &topology,
            &metadata,
            ObjectId::new(1),
            &published_test_page,
        );
        let other = require_visible_operation_target(
            &topology,
            &metadata,
            ObjectId::new(100),
            &published_test_page,
        );

        // Assert
        assert!(browser.is_ok());
        assert!(other.is_ok());
    }

    #[test]
    fn runtime_timeout_preserves_the_public_timeout_budget() {
        // Arrange
        let failure = OperationError::<()>::new(FailureKind::Timeout, "native timeout", None);
        // Act
        let error = HostError::operation(failure, 123);
        // Assert
        assert_eq!(error.code(), "PWRIGHT_TIMEOUT");
        assert_eq!(error.to_string(), "Timeout 123ms exceeded");
    }

    #[test]
    fn transport_and_target_close_share_the_public_target_closed_error() {
        // Arrange
        let failures = [FailureKind::TransportClosed, FailureKind::TargetClosed];
        // Act
        let errors = failures.map(|kind| {
            HostError::operation(OperationError::<()>::new(kind, "closed", None), 5000)
        });
        // Assert
        assert!(
            errors
                .iter()
                .all(|error| error.code() == "PWRIGHT_TARGET_CLOSED")
        );
    }

    #[test]
    fn detached_frame_preserves_its_specific_target_closed_message() {
        // Arrange
        let failure =
            OperationError::<()>::new(FailureKind::TargetClosed, "Frame was detached", None);

        // Act
        let error = HostError::operation(failure, 5000);

        // Assert
        assert!(matches!(error, HostError::FrameDetached));
        assert_eq!(error.code(), "PWRIGHT_TARGET_CLOSED");
        assert_eq!(error.to_string(), "Frame was detached");
    }
}
