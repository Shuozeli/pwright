//! Isolated bounded Chromium CDP adapter for Pwright navigation.
//!
//! This leaf owns only `pwright.navigation` command execution against an
//! injected CDP client. It does not discover, download, launch, or own a
//! browser process and does not install any unrelated capability.

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use pwright_backend_cdp_common::classify_cdp_failure;
use pwright_browser_backend::{
    BackendDescriptor, BackendFeatureDescriptor, BackendFeatureId, BackendId,
    BackendImplementation, BackendProtocolId, BrowserBackend, BrowserEngine, NativeBackendAccess,
};
use pwright_cap_navigation::{
    BEGIN_NAVIGATION_COMMAND_ID, BeginNavigation, BeginNavigationParams, BeginNavigationResult,
    GOTO_COMMAND_ID, Goto, GotoParams, GotoResult, Header, NAVIGATION_CAPABILITY_ID,
    NavigationErrorDetails, NavigationResponse, WaitUntil,
};
use pwright_cdp::connection::CdpError;
use pwright_cdp::generated::page::NavigateReturns;
use pwright_cdp::{
    CdpClient, CdpEvent, CdpEventReceiver, CdpEventRecvError, CdpEventSubscriptionError,
};
use pwright_core::{
    CancellationReason, CommandCancellation, FailureKind, OperationContext, OperationError,
    ProductionResourceError, ProductionResourceLedger, ProtocolVersion, ResourceClass,
    ResourceError, ResourceLease, ResourceLedger, ResourceRequirements, ResourceScope, Service,
};

pub const CHROMIUM_CDP_NAVIGATION_BACKEND_ID: BackendId =
    BackendId::new("pwright.backend.chromium-cdp.navigation");

mod frame_begin;
#[cfg(feature = "frame-lifecycle")]
pub mod frame_lifecycle;
mod reload;
pub const CHROMIUM_CDP_PROTOCOL_ID: BackendProtocolId = BackendProtocolId::new("cdp");
pub const CDP_BEGIN_NAVIGATION_FEATURE: BackendFeatureId =
    BackendFeatureId::new("navigation.begin");
pub const CDP_GOTO_FEATURE: BackendFeatureId = BackendFeatureId::new("navigation.goto");
pub const CDP_RELOAD_FEATURE: BackendFeatureId = BackendFeatureId::new("navigation.reload");

pub const CHROMIUM_CDP_NAVIGATION_FEATURES: &[BackendFeatureDescriptor] = &[
    BackendFeatureDescriptor::new(
        CDP_BEGIN_NAVIGATION_FEATURE,
        NAVIGATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        BEGIN_NAVIGATION_COMMAND_ID,
        BackendImplementation::Native,
    ),
    BackendFeatureDescriptor::new(
        CDP_GOTO_FEATURE,
        NAVIGATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        GOTO_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_RELOAD_FEATURE,
        NAVIGATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 1),
        pwright_cap_navigation::RELOAD_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
];

pub enum ChromiumCdpNavigationBackend {}

impl BrowserBackend for ChromiumCdpNavigationBackend {
    const DESCRIPTOR: BackendDescriptor = BackendDescriptor::new(
        CHROMIUM_CDP_NAVIGATION_BACKEND_ID,
        "Chromium CDP navigation",
        BrowserEngine::Chromium,
        CHROMIUM_CDP_PROTOCOL_ID,
        CHROMIUM_CDP_NAVIGATION_FEATURES,
    );
}

const HTTP_STATUS_FAILURE: &str = "net::ERR_HTTP_RESPONSE_CODE_FAILURE";
const DEFAULT_NETWORK_IDLE_DELAY: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpNavigationLimits {
    pub url_bytes: usize,
    pub native_id_bytes: usize,
    pub inflight_requests: usize,
    pub response_headers: usize,
    pub response_header_bytes: usize,
    pub status_text_bytes: usize,
}

impl CdpNavigationLimits {
    pub const fn new(
        url_bytes: usize,
        native_id_bytes: usize,
        inflight_requests: usize,
        response_headers: usize,
        response_header_bytes: usize,
        status_text_bytes: usize,
    ) -> Self {
        Self {
            url_bytes,
            native_id_bytes,
            inflight_requests,
            response_headers,
            response_header_bytes,
            status_text_bytes,
        }
    }

    /// Conservative variable-size peak for one active navigation.
    ///
    /// This includes command URL/referrer, CDP session/frame/loader/request
    /// IDs, final/result URLs, and two response metadata sets because a
    /// redirect response can be replaced while the prior value is live.
    pub fn maximum_retained_bytes_per_operation(self) -> u64 {
        let url = u64::try_from(self.url_bytes).unwrap_or(u64::MAX);
        let native_id = u64::try_from(self.native_id_bytes).unwrap_or(u64::MAX);
        let requests = u64::try_from(self.inflight_requests).unwrap_or(u64::MAX);
        let headers = u64::try_from(self.response_header_bytes).unwrap_or(u64::MAX);
        let status = u64::try_from(self.status_text_bytes).unwrap_or(u64::MAX);
        url.saturating_mul(8)
            .saturating_add(native_id.saturating_mul(requests.saturating_add(8)))
            .saturating_add(headers.saturating_mul(2))
            .saturating_add(status.saturating_mul(2))
    }
}

impl Default for CdpNavigationLimits {
    fn default() -> Self {
        Self::new(1024 * 1024, 64 * 1024, 4096, 1024, 1024 * 1024, 64 * 1024)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpNavigationProductionProfile {
    resource_scope: ResourceScope,
    limits: CdpNavigationLimits,
    max_concurrent_operations: usize,
    maximum_retained_bytes_per_operation: u64,
    requirements: ResourceRequirements,
}

impl CdpNavigationProductionProfile {
    pub fn new(
        resource_scope: ResourceScope,
        limits: CdpNavigationLimits,
        max_concurrent_operations: usize,
    ) -> Self {
        let maximum_retained_bytes_per_operation = limits.maximum_retained_bytes_per_operation();
        let operations = u64::try_from(max_concurrent_operations).unwrap_or(u64::MAX);
        let bytes = operations.saturating_mul(maximum_retained_bytes_per_operation);
        let requirements = if max_concurrent_operations == 0 {
            ResourceRequirements::none()
        } else {
            ResourceRequirements::none()
                .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
                .with_metadata(1, max_concurrent_operations)
        };
        Self {
            resource_scope,
            limits,
            max_concurrent_operations,
            maximum_retained_bytes_per_operation,
            requirements,
        }
    }

    pub const fn resource_scope(self) -> ResourceScope {
        self.resource_scope
    }

    pub const fn limits(self) -> CdpNavigationLimits {
        self.limits
    }

    pub const fn max_concurrent_operations(self) -> usize {
        self.max_concurrent_operations
    }

    pub const fn maximum_retained_bytes_per_operation(self) -> u64 {
        self.maximum_retained_bytes_per_operation
    }

    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}

#[derive(Clone)]
struct NavigationResourceBudget {
    ledger: ResourceLedger,
    profile: CdpNavigationProductionProfile,
    active_operations: Arc<AtomicUsize>,
}

struct NavigationOperationLease {
    resource: Option<ResourceLease>,
    active_operations: Arc<AtomicUsize>,
}

impl Drop for NavigationOperationLease {
    fn drop(&mut self) {
        drop(self.resource.take());
        let previous = self.active_operations.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0);
    }
}

#[derive(Clone)]
pub struct CdpNavigationService {
    client: Arc<dyn CdpClient>,
    network_idle_delay: Duration,
    limits: CdpNavigationLimits,
    resources: Option<NavigationResourceBudget>,
}

/// Creates per-target navigation services that share one production
/// concurrency counter and resource declaration.
#[derive(Clone)]
pub struct CdpNavigationServiceFactory {
    network_idle_delay: Duration,
    limits: CdpNavigationLimits,
    resources: NavigationResourceBudget,
}

impl CdpNavigationServiceFactory {
    pub fn new_with_production_resources(
        profile: CdpNavigationProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        Self::with_network_idle_delay_and_production_resources(
            DEFAULT_NETWORK_IDLE_DELAY,
            profile,
            resources,
        )
    }

    pub fn with_network_idle_delay_and_production_resources(
        network_idle_delay: Duration,
        profile: CdpNavigationProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        let ledger = resources.claim(profile.requirements())?;
        Ok(Self {
            network_idle_delay,
            limits: profile.limits(),
            resources: NavigationResourceBudget {
                ledger,
                profile,
                active_operations: Arc::new(AtomicUsize::new(0)),
            },
        })
    }

    pub fn bind(&self, client: Arc<dyn CdpClient>) -> CdpNavigationService {
        CdpNavigationService::from_parts(
            client,
            self.network_idle_delay,
            self.limits,
            Some(self.resources.clone()),
        )
    }
}

impl CdpNavigationService {
    pub fn new(client: Arc<dyn CdpClient>) -> Self {
        Self::from_parts(
            client,
            DEFAULT_NETWORK_IDLE_DELAY,
            CdpNavigationLimits::default(),
            None,
        )
    }

    pub fn with_network_idle_delay(
        client: Arc<dyn CdpClient>,
        network_idle_delay: Duration,
    ) -> Self {
        Self::from_parts(
            client,
            network_idle_delay,
            CdpNavigationLimits::default(),
            None,
        )
    }

    pub fn with_limits(client: Arc<dyn CdpClient>, limits: CdpNavigationLimits) -> Self {
        Self::from_parts(client, DEFAULT_NETWORK_IDLE_DELAY, limits, None)
    }

    pub fn with_network_idle_delay_and_limits(
        client: Arc<dyn CdpClient>,
        network_idle_delay: Duration,
        limits: CdpNavigationLimits,
    ) -> Self {
        Self::from_parts(client, network_idle_delay, limits, None)
    }

    pub fn new_with_production_resources(
        client: Arc<dyn CdpClient>,
        profile: CdpNavigationProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        Self::with_network_idle_delay_and_production_resources(
            client,
            DEFAULT_NETWORK_IDLE_DELAY,
            profile,
            resources,
        )
    }

    pub fn with_network_idle_delay_and_production_resources(
        client: Arc<dyn CdpClient>,
        network_idle_delay: Duration,
        profile: CdpNavigationProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        CdpNavigationServiceFactory::with_network_idle_delay_and_production_resources(
            network_idle_delay,
            profile,
            resources,
        )
        .map(|factory| factory.bind(client))
    }

    pub const fn limits(&self) -> CdpNavigationLimits {
        self.limits
    }

    fn from_parts(
        client: Arc<dyn CdpClient>,
        network_idle_delay: Duration,
        limits: CdpNavigationLimits,
        resources: Option<NavigationResourceBudget>,
    ) -> Self {
        Self {
            client,
            network_idle_delay,
            limits,
            resources,
        }
    }

    fn reserve_operation(
        &self,
        url: &String,
        referrer: Option<&String>,
    ) -> Result<Option<NavigationOperationLease>, OperationError<NavigationErrorDetails>> {
        validate_navigation_input(
            "navigation URL",
            url,
            self.limits.url_bytes,
            self.resources.is_some(),
        )?;
        if let Some(referrer) = referrer {
            validate_navigation_input(
                "navigation referrer",
                referrer,
                self.limits.url_bytes,
                self.resources.is_some(),
            )?;
        }
        let Some(resources) = &self.resources else {
            return Ok(None);
        };
        let limit = resources.profile.max_concurrent_operations();
        if resources
            .active_operations
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                (current < limit).then_some(current + 1)
            })
            .is_err()
        {
            return Err(navigation_resource_error(
                "navigation operation concurrency limit was reached",
            ));
        }
        let resource = match resources.ledger.reserve(
            resources.profile.resource_scope(),
            ResourceClass::BackendStateBytes,
            resources.profile.maximum_retained_bytes_per_operation(),
        ) {
            Ok(resource) => resource,
            Err(error) => {
                let previous = resources.active_operations.fetch_sub(1, Ordering::AcqRel);
                debug_assert!(previous > 0);
                return Err(map_navigation_resource_error(error));
            }
        };
        Ok(Some(NavigationOperationLease {
            resource: Some(resource),
            active_operations: resources.active_operations.clone(),
        }))
    }
}

impl NativeBackendAccess<ChromiumCdpNavigationBackend> for CdpNavigationService {
    type Native = dyn CdpClient;

    fn native_backend(&self) -> &Self::Native {
        self.client.as_ref()
    }
}

impl Service<BeginNavigationParams> for CdpNavigationService {
    type Response = BeginNavigationResult;
    type Error = OperationError<NavigationErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: BeginNavigationParams) -> Self::Future<'_> {
        let resources = match self.reserve_operation(&params.url, params.referrer.as_ref()) {
            Ok(resources) => resources,
            Err(error) => {
                drop(params);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            let response = self
                .client
                .page_navigate_with_referrer(&params.url, params.referrer.as_deref())
                .await
                .map_err(map_cdp_error)?;
            parse_navigation_result(response, self.limits)
        })
    }
}

impl Service<GotoParams> for CdpNavigationService {
    type Response = GotoResult;
    type Error = OperationError<NavigationErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: GotoParams) -> Self::Future<'_> {
        let resources = match self.reserve_operation(&params.url, params.referrer.as_ref()) {
            Ok(resources) => resources,
            Err(error) => {
                drop(params);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            let expected_session = self.client.event_session_id().map(ToOwned::to_owned);
            if let Some(expected_session) = &expected_session {
                validate_navigation_value(
                    "CDP session ID",
                    expected_session,
                    self.limits.native_id_bytes,
                )?;
            }
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

            let response = self
                .client
                .page_navigate_with_referrer(&params.url, params.referrer.as_deref())
                .await
                .map_err(map_cdp_error)?;
            let navigation = parse_navigation_result(response, self.limits)?;
            if navigation.is_download {
                return Err(OperationError::new(
                    FailureKind::Backend,
                    "navigation started a download",
                    Some(NavigationErrorDetails::Backend(
                        "navigation started a download".to_string(),
                    )),
                ));
            }

            if navigation.same_document {
                let url = wait_for_same_document(
                    &mut events,
                    expected_session.as_deref(),
                    &navigation.frame_id,
                    self.limits,
                )
                .await?;
                return Ok(GotoResult {
                    frame_id: navigation.frame_id,
                    loader_id: None,
                    url,
                    same_document: true,
                    response: None,
                });
            }

            let wait = NewDocumentWait {
                expected_session: expected_session.as_deref(),
                frame_id: &navigation.frame_id,
                loader_id: navigation
                    .loader_id
                    .as_deref()
                    .expect("new-document navigation has a loader"),
                requested_url: &params.url,
                wait_until: params.wait_until,
                idle_delay: self.network_idle_delay,
                limits: self.limits,
            };
            wait_for_new_document(&mut events, wait).await
        })
    }
}

impl CommandCancellation<BeginNavigation> for CdpNavigationService {
    type Future<'a>
        = Pin<
        Box<dyn Future<Output = Result<(), OperationError<NavigationErrorDetails>>> + Send + 'a>,
    >
    where
        Self: 'a;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        Box::pin(async move { self.client.page_stop_loading().await.map_err(map_cdp_error) })
    }
}

impl CommandCancellation<Goto> for CdpNavigationService {
    type Future<'a>
        = Pin<
        Box<dyn Future<Output = Result<(), OperationError<NavigationErrorDetails>>> + Send + 'a>,
    >
    where
        Self: 'a;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        Box::pin(async move { self.client.page_stop_loading().await.map_err(map_cdp_error) })
    }
}

async fn wait_for_same_document(
    events: &mut CdpEventReceiver,
    expected_session: Option<&str>,
    frame_id: &str,
    limits: CdpNavigationLimits,
) -> Result<String, OperationError<NavigationErrorDetails>> {
    loop {
        let event = receive_event(events).await?;
        if !event_matches_session(&event, expected_session)
            || event.method != "Page.navigatedWithinDocument"
        {
            continue;
        }
        let Some(event_frame) = event.params.get("frameId").and_then(|value| value.as_str()) else {
            continue;
        };
        if event_frame != frame_id {
            continue;
        }
        let url = event
            .params
            .get("url")
            .and_then(|value| value.as_str())
            .ok_or_else(|| invalid_event("Page.navigatedWithinDocument is missing url"))?;
        validate_navigation_value("same-document URL", url, limits.url_bytes)?;
        return Ok(compact_navigation_string(url.to_string()));
    }
}

struct NewDocumentState {
    committed: bool,
    lifecycle_reached: bool,
    final_url: Option<String>,
    response: Option<NavigationResponse>,
    document_request: Option<String>,
    tracking_requests: bool,
    inflight: HashSet<String>,
    idle_since: Option<tokio::time::Instant>,
}

impl NewDocumentState {
    fn new(wait_until: WaitUntil) -> Self {
        Self {
            committed: false,
            lifecycle_reached: wait_until == WaitUntil::Commit,
            final_url: None,
            response: None,
            document_request: None,
            tracking_requests: false,
            inflight: HashSet::new(),
            idle_since: None,
        }
    }

    fn update_idle(&mut self, idle_delay: Duration) {
        if self.committed && self.inflight.is_empty() {
            self.idle_since
                .get_or_insert_with(|| tokio::time::Instant::now() + idle_delay);
        } else {
            self.idle_since = None;
        }
    }
}

struct NewDocumentWait<'a> {
    expected_session: Option<&'a str>,
    frame_id: &'a str,
    loader_id: &'a str,
    requested_url: &'a str,
    wait_until: WaitUntil,
    idle_delay: Duration,
    limits: CdpNavigationLimits,
}

async fn wait_for_new_document(
    events: &mut CdpEventReceiver,
    wait: NewDocumentWait<'_>,
) -> Result<GotoResult, OperationError<NavigationErrorDetails>> {
    let state = NewDocumentState::new(wait.wait_until);
    wait_for_new_document_with_state(events, wait, state).await
}

async fn wait_for_new_document_with_state(
    events: &mut CdpEventReceiver,
    wait: NewDocumentWait<'_>,
    mut state: NewDocumentState,
) -> Result<GotoResult, OperationError<NavigationErrorDetails>> {
    let NewDocumentWait {
        expected_session,
        frame_id,
        loader_id,
        requested_url,
        wait_until,
        idle_delay,
        limits,
    } = wait;
    let requires_response =
        requested_url.starts_with("http://") || requested_url.starts_with("https://");

    loop {
        if state.committed
            && state.lifecycle_reached
            && (!requires_response || state.response.is_some())
            && wait_until != WaitUntil::NetworkIdle
        {
            return Ok(finish_new_document(
                frame_id,
                loader_id,
                requested_url,
                state,
            ));
        }

        let event = if wait_until == WaitUntil::NetworkIdle {
            state.update_idle(idle_delay);
            if let Some(deadline) = state.idle_since {
                tokio::select! {
                    event = receive_event(events) => Some(event?),
                    () = tokio::time::sleep_until(deadline) => None,
                }
            } else {
                Some(receive_event(events).await?)
            }
        } else {
            Some(receive_event(events).await?)
        };

        let Some(event) = event else {
            if !requires_response || state.response.is_some() {
                return Ok(finish_new_document(
                    frame_id,
                    loader_id,
                    requested_url,
                    state,
                ));
            }
            state.idle_since = None;
            continue;
        };
        if !event_matches_session(&event, expected_session) {
            continue;
        }
        apply_navigation_event(&mut state, event, frame_id, loader_id, wait_until, limits)?;
    }
}

fn apply_navigation_event(
    state: &mut NewDocumentState,
    event: CdpEvent,
    frame_id: &str,
    loader_id: &str,
    wait_until: WaitUntil,
    limits: CdpNavigationLimits,
) -> Result<(), OperationError<NavigationErrorDetails>> {
    match event.method.as_str() {
        "Page.frameNavigated" => {
            let Some(frame) = event.params.get("frame") else {
                return Ok(());
            };
            if frame.get("id").and_then(|value| value.as_str()) != Some(frame_id) {
                return Ok(());
            }
            let actual_loader = frame
                .get("loaderId")
                .and_then(|value| value.as_str())
                .ok_or_else(|| invalid_event("Page.frameNavigated is missing loaderId"))?;
            validate_native_id("frame loader ID", actual_loader, limits.native_id_bytes)?;
            let url = frame
                .get("url")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            validate_navigation_value("committed URL", url, limits.url_bytes)?;
            if actual_loader != loader_id {
                return Err(OperationError::new(
                    FailureKind::Backend,
                    format!("navigation was interrupted by a new document at {url}"),
                    Some(NavigationErrorDetails::Backend(format!(
                        "expected loader {loader_id}, received {actual_loader}"
                    ))),
                ));
            }
            let final_url = match frame.get("urlFragment").and_then(|value| value.as_str()) {
                Some(fragment) => {
                    let combined = url.len().checked_add(fragment.len()).ok_or_else(|| {
                        navigation_resource_error("committed URL length overflowed")
                    })?;
                    if combined > limits.url_bytes {
                        return Err(navigation_limit_error(
                            "committed URL",
                            combined,
                            limits.url_bytes,
                        ));
                    }
                    compact_navigation_string(format!("{url}{fragment}"))
                }
                None => compact_navigation_string(url.to_string()),
            };
            state.committed = true;
            state.final_url = Some(final_url);
        }
        "Page.lifecycleEvent" => {
            if event.params.get("frameId").and_then(|value| value.as_str()) != Some(frame_id)
                || event
                    .params
                    .get("loaderId")
                    .and_then(|value| value.as_str())
                    != Some(loader_id)
            {
                return Ok(());
            }
            let name = event.params.get("name").and_then(|value| value.as_str());
            state.lifecycle_reached |= match wait_until {
                WaitUntil::Commit => false,
                WaitUntil::DomContentLoaded => name == Some("DOMContentLoaded"),
                WaitUntil::Load => name == Some("load"),
                WaitUntil::NetworkIdle => false,
            };
        }
        "Network.requestWillBeSent" => {
            let Some(request_id) = event
                .params
                .get("requestId")
                .and_then(|value| value.as_str())
            else {
                return Ok(());
            };
            validate_native_id("network request ID", request_id, limits.native_id_bytes)?;
            let resource_type = event.params.get("type").and_then(|value| value.as_str());
            let url = event
                .params
                .get("request")
                .and_then(|request| request.get("url"))
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            let is_main_document = resource_type == Some("Document")
                && event
                    .params
                    .get("loaderId")
                    .and_then(|value| value.as_str())
                    == Some(loader_id)
                && event.params.get("frameId").and_then(|value| value.as_str()) == Some(frame_id);
            let tracking_requests = state.tracking_requests || is_main_document;
            let should_track = tracking_requests
                && resource_type != Some("EventSource")
                && !url.ends_with("/favicon.ico");
            if should_track
                && !state.inflight.contains(request_id)
                && state.inflight.len() >= limits.inflight_requests
            {
                return Err(navigation_limit_error(
                    "in-flight network request count",
                    state.inflight.len().saturating_add(1),
                    limits.inflight_requests,
                ));
            }
            if is_main_document {
                state.document_request = Some(compact_navigation_string(request_id.to_string()));
                state.tracking_requests = true;
            }
            if !tracking_requests {
                return Ok(());
            }
            if should_track {
                state
                    .inflight
                    .insert(compact_navigation_string(request_id.to_string()));
                state.idle_since = None;
            }
        }
        "Network.loadingFinished" => {
            if let Some(request_id) = event
                .params
                .get("requestId")
                .and_then(|value| value.as_str())
            {
                state.inflight.remove(request_id);
            }
        }
        "Network.loadingFailed" => {
            let request_id = event
                .params
                .get("requestId")
                .and_then(|value| value.as_str());
            state
                .inflight
                .retain(|candidate| Some(candidate.as_str()) != request_id);
            if request_id == state.document_request.as_deref() {
                let error = event
                    .params
                    .get("errorText")
                    .and_then(|value| value.as_str())
                    .unwrap_or("document request failed");
                validate_navigation_value("network error text", error, limits.status_text_bytes)?;
                return Err(OperationError::new(
                    FailureKind::Backend,
                    format!("document navigation failed: {error}"),
                    Some(NavigationErrorDetails::Network(error.to_string())),
                ));
            }
        }
        "Network.responseReceived" => {
            if event
                .params
                .get("loaderId")
                .and_then(|value| value.as_str())
                != Some(loader_id)
                || event.params.get("frameId").and_then(|value| value.as_str()) != Some(frame_id)
                || event.params.get("type").and_then(|value| value.as_str()) != Some("Document")
            {
                return Ok(());
            }
            let response = parse_response(&event.params, limits)?;
            if response.url.starts_with("http://") || response.url.starts_with("https://") {
                state.response = Some(response);
            }
        }
        _ => {}
    }
    Ok(())
}

fn parse_response(
    params: &serde_json::Value,
    limits: CdpNavigationLimits,
) -> Result<NavigationResponse, OperationError<NavigationErrorDetails>> {
    let response = params
        .get("response")
        .and_then(|value| value.as_object())
        .ok_or_else(|| invalid_event("Network.responseReceived is missing response"))?;
    let url = response
        .get("url")
        .and_then(|value| value.as_str())
        .ok_or_else(|| invalid_event("navigation response is missing url"))?;
    validate_navigation_value("navigation response URL", url, limits.url_bytes)?;
    let status = response
        .get("status")
        .and_then(|value| value.as_i64())
        .ok_or_else(|| invalid_event("navigation response is missing integer status"))?;
    let status_text = response
        .get("statusText")
        .and_then(|value| value.as_str())
        .ok_or_else(|| invalid_event("navigation response is missing statusText"))?;
    validate_navigation_value(
        "navigation response status text",
        status_text,
        limits.status_text_bytes,
    )?;
    let headers = response
        .get("headers")
        .and_then(|value| value.as_object())
        .ok_or_else(|| invalid_event("navigation response is missing headers"))?;
    if headers.len() > limits.response_headers {
        return Err(navigation_limit_error(
            "navigation response header count",
            headers.len(),
            limits.response_headers,
        ));
    }
    let mut header_bytes = 0_usize;
    for (name, value) in headers {
        let value = value
            .as_str()
            .ok_or_else(|| invalid_event("navigation response header value is not a string"))?;
        header_bytes = header_bytes
            .checked_add(name.len())
            .and_then(|total| total.checked_add(value.len()))
            .ok_or_else(|| navigation_resource_error("response header size overflowed"))?;
        if header_bytes > limits.response_header_bytes {
            return Err(navigation_limit_error(
                "navigation response header bytes",
                header_bytes,
                limits.response_header_bytes,
            ));
        }
    }
    let mut headers = headers
        .iter()
        .map(|(name, value)| Header {
            name: compact_navigation_string(name.clone()),
            value: compact_navigation_string(
                value
                    .as_str()
                    .expect("header values were validated before allocation")
                    .to_string(),
            ),
        })
        .collect::<Vec<_>>();
    headers.sort_by(|left, right| (&left.name, &left.value).cmp(&(&right.name, &right.value)));
    Ok(NavigationResponse {
        url: compact_navigation_string(url.to_string()),
        status,
        status_text: compact_navigation_string(status_text.to_string()),
        headers,
    })
}

fn finish_new_document(
    frame_id: &str,
    loader_id: &str,
    requested_url: &str,
    state: NewDocumentState,
) -> GotoResult {
    let url = state
        .final_url
        .or_else(|| state.response.as_ref().map(|response| response.url.clone()))
        .unwrap_or_else(|| requested_url.to_string());
    GotoResult {
        frame_id: compact_navigation_string(frame_id.to_string()),
        loader_id: Some(compact_navigation_string(loader_id.to_string())),
        url,
        same_document: false,
        response: state.response,
    }
}

fn event_matches_session(event: &CdpEvent, expected_session: Option<&str>) -> bool {
    event.session_id.as_deref() == expected_session
}

async fn receive_event(
    events: &mut CdpEventReceiver,
) -> Result<CdpEvent, OperationError<NavigationErrorDetails>> {
    match events.recv().await {
        Ok(event) => Ok(event),
        Err(CdpEventRecvError::Lagged(skipped)) => Err(OperationError::new(
            FailureKind::ResourceExhausted,
            format!("navigation event stream lagged by {skipped} events"),
            Some(NavigationErrorDetails::Backend(format!(
                "event stream lagged by {skipped}"
            ))),
        )),
        Err(CdpEventRecvError::Closed) => Err(OperationError::new(
            FailureKind::TransportClosed,
            "navigation event stream closed",
            Some(NavigationErrorDetails::Backend(
                "navigation event stream closed".to_string(),
            )),
        )),
    }
}

fn invalid_event(message: &str) -> OperationError<NavigationErrorDetails> {
    OperationError::new(
        FailureKind::ProtocolViolation,
        message,
        Some(NavigationErrorDetails::InvalidBackendResponse(
            message.to_string(),
        )),
    )
}

fn navigation_resource_error(message: &str) -> OperationError<NavigationErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        message,
        Some(NavigationErrorDetails::Backend(message.to_string())),
    )
}

fn navigation_limit_error(
    field: &str,
    actual: usize,
    limit: usize,
) -> OperationError<NavigationErrorDetails> {
    navigation_resource_error(&format!(
        "{field} is {actual}, exceeding the configured limit of {limit}"
    ))
}

fn validate_navigation_value(
    field: &str,
    value: &str,
    limit: usize,
) -> Result<(), OperationError<NavigationErrorDetails>> {
    if value.len() > limit {
        Err(navigation_limit_error(field, value.len(), limit))
    } else {
        Ok(())
    }
}

fn compact_navigation_string(value: String) -> String {
    value.into_boxed_str().into_string()
}

fn validate_navigation_input(
    field: &str,
    value: &String,
    limit: usize,
    validate_capacity: bool,
) -> Result<(), OperationError<NavigationErrorDetails>> {
    validate_navigation_value(field, value, limit)?;
    if validate_capacity && value.capacity() > limit {
        return Err(navigation_limit_error(
            &format!("{field} capacity"),
            value.capacity(),
            limit,
        ));
    }
    Ok(())
}

fn validate_native_id(
    field: &str,
    value: &str,
    limit: usize,
) -> Result<(), OperationError<NavigationErrorDetails>> {
    if value.is_empty() {
        return Err(invalid_event(&format!("{field} must not be empty")));
    }
    validate_navigation_value(field, value, limit)
}

fn map_navigation_resource_error(error: ResourceError) -> OperationError<NavigationErrorDetails> {
    let message = error.to_string();
    OperationError::new(
        FailureKind::ResourceExhausted,
        message.clone(),
        Some(NavigationErrorDetails::Backend(message)),
    )
}

fn parse_navigation_result(
    value: serde_json::Value,
    limits: CdpNavigationLimits,
) -> Result<BeginNavigationResult, OperationError<NavigationErrorDetails>> {
    let response: NavigateReturns = serde_json::from_value(value).map_err(|error| {
        OperationError::new(
            FailureKind::ProtocolViolation,
            format!("invalid Page.navigate response: {error}"),
            Some(NavigationErrorDetails::InvalidBackendResponse(
                error.to_string(),
            )),
        )
    })?;

    validate_native_id(
        "navigation frame ID",
        &response.frame_id,
        limits.native_id_bytes,
    )?;
    if let Some(loader_id) = &response.loader_id {
        validate_native_id("navigation loader ID", loader_id, limits.native_id_bytes)?;
    }

    if let Some(error_text) = response.error_text.as_deref()
        && !error_text.is_empty()
        && error_text != HTTP_STATUS_FAILURE
    {
        validate_navigation_value(
            "navigation error text",
            error_text,
            limits.status_text_bytes,
        )?;
        return Err(OperationError::new(
            FailureKind::Backend,
            format!("document navigation failed: {error_text}"),
            Some(NavigationErrorDetails::Network(error_text.to_string())),
        ));
    }

    let same_document = response.loader_id.is_none();
    Ok(BeginNavigationResult {
        frame_id: compact_navigation_string(response.frame_id),
        loader_id: response.loader_id.map(compact_navigation_string),
        same_document,
        is_download: response.is_download.unwrap_or(false),
    })
}

fn map_cdp_error(error: CdpError) -> OperationError<NavigationErrorDetails> {
    let kind = classify_cdp_failure(&error);
    let message = error.to_string();
    let details = match error {
        CdpError::NavigationFailed { .. } | CdpError::HttpFailed(_) => {
            NavigationErrorDetails::Network(message.clone())
        }
        _ => NavigationErrorDetails::Backend(message.clone()),
    };
    OperationError::new(kind, message, Some(details))
}

fn map_event_subscription_error(
    error: CdpEventSubscriptionError,
) -> OperationError<NavigationErrorDetails> {
    let message = error.to_string();
    OperationError::new(
        FailureKind::ResourceExhausted,
        message.clone(),
        Some(NavigationErrorDetails::Backend(message)),
    )
}

#[cfg(test)]
mod tests {
    use pwright_browser_backend::BrowserBackend;
    use pwright_cap_navigation::NavigationErrorDetails;
    use pwright_cdp::CdpEvent;
    use pwright_core::FailureKind;
    use serde_json::json;

    use super::{
        CdpNavigationLimits, ChromiumCdpNavigationBackend, event_matches_session, map_cdp_error,
        parse_navigation_result, parse_response,
    };

    #[test]
    fn descriptor_contains_only_navigation_commands() {
        // Arrange
        let expected = ["navigation.begin", "navigation.goto", "navigation.reload"];

        // Act
        let descriptor = ChromiumCdpNavigationBackend::DESCRIPTOR;
        let actual = descriptor
            .features
            .iter()
            .map(|feature| feature.id.as_str())
            .collect::<Vec<_>>();

        // Assert
        assert_eq!(actual, expected);
        assert!(descriptor.event_features.is_empty());
    }

    #[test]
    fn event_filter_requires_exact_cdp_session_identity() {
        // Arrange
        let browser_event = CdpEvent::new("Page.frameNavigated".to_owned(), json!({}), None);
        let page_event = CdpEvent::new(
            "Page.frameNavigated".to_owned(),
            json!({}),
            Some("session-1".to_owned()),
        );

        // Act
        let browser_match = event_matches_session(&browser_event, None);
        let page_match = event_matches_session(&page_event, Some("session-1"));
        let wrong_page = event_matches_session(&page_event, Some("session-2"));

        // Assert
        assert!(browser_match);
        assert!(page_match);
        assert!(!wrong_page);
    }

    #[test]
    fn http_status_failure_is_preserved_as_a_successful_navigation() {
        // Arrange
        let response = json!({
            "frameId": "frame-1",
            "loaderId": "loader-1",
            "errorText": "net::ERR_HTTP_RESPONSE_CODE_FAILURE"
        });

        // Act
        let result = parse_navigation_result(response, CdpNavigationLimits::default())
            .expect("HTTP status pages are successful navigations");

        // Assert
        assert_eq!(result.frame_id, "frame-1");
        assert_eq!(result.loader_id.as_deref(), Some("loader-1"));
        assert!(!result.same_document);
    }

    #[test]
    fn network_failure_is_not_misclassified_as_an_http_response() {
        // Arrange
        let response = json!({
            "frameId": "frame-1",
            "errorText": "net::ERR_NAME_NOT_RESOLVED"
        });

        // Act
        let error = parse_navigation_result(response, CdpNavigationLimits::default())
            .expect_err("network failure should fail navigation");

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert_eq!(
            error.details(),
            Some(&NavigationErrorDetails::Network(
                "net::ERR_NAME_NOT_RESOLVED".to_owned()
            ))
        );
    }

    #[test]
    fn closed_cdp_connection_maps_to_transport_closed() {
        // Arrange
        let cdp_error = pwright_cdp::connection::CdpError::Closed;

        // Act
        let error = map_cdp_error(cdp_error);

        // Assert
        assert_eq!(error.kind(), FailureKind::TransportClosed);
    }

    #[test]
    fn response_header_bytes_and_types_are_validated_before_copying() {
        // Arrange
        let limits = CdpNavigationLimits::new(256, 64, 4, 4, 3, 64);
        let oversized = json!({"response": {
            "url": "https://example.com",
            "status": 200,
            "statusText": "OK",
            "headers": {"name": "value"}
        }});
        let non_string = json!({"response": {
            "url": "https://example.com",
            "status": 200,
            "statusText": "OK",
            "headers": {"x-count": 3}
        }});

        // Act
        let oversized_error = parse_response(&oversized, limits)
            .expect_err("aggregate header bytes should be bounded before copying");
        let type_error = parse_response(
            &non_string,
            CdpNavigationLimits::new(256, 64, 4, 4, 256, 64),
        )
        .expect_err("non-string CDP header values should fail explicitly");

        // Assert
        assert_eq!(oversized_error.kind(), FailureKind::ResourceExhausted);
        assert!(oversized_error.message().contains("response header bytes"));
        assert_eq!(type_error.kind(), FailureKind::ProtocolViolation);
        assert!(type_error.message().contains("not a string"));
    }
}
