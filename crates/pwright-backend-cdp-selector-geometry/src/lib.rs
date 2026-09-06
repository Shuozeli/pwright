//! First-party Chromium selector-to-geometry plugin for Locator screenshots.
//!
//! This leaf installs Pwright's command-free selector runtime into one target,
//! resolves a lazy selector with strictness and actionability retries, scrolls
//! only at an explicit commit point, and returns validated viewport geometry.
//! It exposes no public Locator command and never retains a DOM or CDP handle.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

use pwright_backend_cdp_common::classify_cdp_failure;
use pwright_backend_cdp_locator_screenshot::{
    LocatorScreenshotGeometry, LocatorScreenshotGeometryResolver,
};
use pwright_backend_cdp_selector_runtime::{
    MAX_COMPILED_SELECTOR_BYTES, MAX_SELECTOR_RUNTIME_INSTALL_BYTES, MAX_SELECTOR_STRING_BYTES,
    SELECTOR_RUNTIME_SYMBOL, SELECTOR_RUNTIME_VERSION, compile_selector, measure_selector,
    selector_runtime_install_script,
};
use pwright_cap_locator_screenshot::LocatorScreenshotErrorDetails;
use pwright_cdp::CdpClient;
use pwright_core::{
    FailureKind, OperationError, ProductionResourceError, ProductionResourceLedger, ResourceClass,
    ResourceError, ResourceLease, ResourceLedger, ResourceRequirements, ResourceScope,
};
use pwright_selector_model::{LocatorErrorDetails, Selector};
use serde_json::Value;

const DEFAULT_RETRY_DELAY: Duration = Duration::from_millis(50);
const MAX_GEOMETRY_WRAPPER_BYTES: usize = 32 * 1024;
const MAX_GEOMETRY_SCRIPT_BYTES: usize = MAX_COMPILED_SELECTOR_BYTES + MAX_GEOMETRY_WRAPPER_BYTES;
const MAX_CONTEXT_ROUTE_STRING_COPIES: usize = 12;
const MAX_RESOLUTION_RETAINED_BYTES: usize = MAX_SELECTOR_STRING_BYTES
    + MAX_COMPILED_SELECTOR_BYTES
    + 2 * MAX_GEOMETRY_SCRIPT_BYTES
    + MAX_SELECTOR_RUNTIME_INSTALL_BYTES
    + 2 * MAX_COMPILED_SELECTOR_BYTES;

/// Resource declaration for the standalone selector-geometry plugin.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpSelectorGeometryProductionProfile {
    resource_scope: ResourceScope,
    max_concurrent_resolutions: usize,
    max_context_routes: usize,
    max_native_id_bytes: usize,
    maximum_retained_bytes_per_resolution: u64,
    maximum_retained_bytes_per_context_route: u64,
    requirements: ResourceRequirements,
}

impl CdpSelectorGeometryProductionProfile {
    pub fn new(resource_scope: ResourceScope, max_concurrent_resolutions: usize) -> Self {
        let per_resolution = u64::try_from(MAX_RESOLUTION_RETAINED_BYTES).unwrap_or(u64::MAX);
        let resolutions = u64::try_from(max_concurrent_resolutions).unwrap_or(u64::MAX);
        let selector_bytes = resolutions.saturating_mul(per_resolution);
        let requirements = if selector_bytes == 0 {
            ResourceRequirements::none()
        } else {
            ResourceRequirements::none()
                .with_class(ResourceClass::SelectorBytes, selector_bytes, selector_bytes)
                .with_metadata(1, max_concurrent_resolutions)
        };
        Self {
            resource_scope,
            max_concurrent_resolutions,
            max_context_routes: 0,
            max_native_id_bytes: 0,
            maximum_retained_bytes_per_resolution: per_resolution,
            maximum_retained_bytes_per_context_route: 0,
            requirements,
        }
    }

    /// Adds bounded persistent storage for Frame utility-world routes.
    pub fn with_context_routes(
        mut self,
        max_context_routes: usize,
        max_native_id_bytes: usize,
    ) -> Self {
        self.max_context_routes = max_context_routes;
        self.max_native_id_bytes = max_native_id_bytes;
        self.maximum_retained_bytes_per_context_route = u64::try_from(max_native_id_bytes)
            .unwrap_or(u64::MAX)
            .saturating_mul(MAX_CONTEXT_ROUTE_STRING_COPIES as u64);
        self.rebuild_requirements();
        self
    }

    fn rebuild_requirements(&mut self) {
        let resolutions = u64::try_from(self.max_concurrent_resolutions).unwrap_or(u64::MAX);
        let selector_bytes = resolutions.saturating_mul(self.maximum_retained_bytes_per_resolution);
        let routes = u64::try_from(self.max_context_routes).unwrap_or(u64::MAX);
        let backend_bytes = routes.saturating_mul(self.maximum_retained_bytes_per_context_route);
        let mut requirements = ResourceRequirements::none();
        if selector_bytes > 0 {
            requirements = requirements.with_class(
                ResourceClass::SelectorBytes,
                selector_bytes,
                selector_bytes,
            );
        }
        if backend_bytes > 0 {
            requirements = requirements.with_class(
                ResourceClass::BackendStateBytes,
                backend_bytes,
                backend_bytes,
            );
        }
        self.requirements = if selector_bytes == 0 && backend_bytes == 0 {
            ResourceRequirements::none()
        } else {
            requirements.with_metadata(
                1,
                self.max_concurrent_resolutions
                    .saturating_add(self.max_context_routes),
            )
        };
    }

    pub const fn resource_scope(self) -> ResourceScope {
        self.resource_scope
    }

    pub const fn max_concurrent_resolutions(self) -> usize {
        self.max_concurrent_resolutions
    }

    pub const fn max_context_routes(self) -> usize {
        self.max_context_routes
    }

    pub const fn max_native_id_bytes(self) -> usize {
        self.max_native_id_bytes
    }

    pub const fn maximum_retained_bytes_per_resolution(self) -> u64 {
        self.maximum_retained_bytes_per_resolution
    }

    pub const fn maximum_retained_bytes_per_context_route(self) -> u64 {
        self.maximum_retained_bytes_per_context_route
    }

    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SelectorGeometryMetrics {
    pub installations: u64,
    pub evaluations: u64,
    pub scroll_commits: u64,
    pub missing_after_navigation: u64,
}

#[derive(Default)]
struct SelectorGeometryCounters {
    installations: AtomicU64,
    evaluations: AtomicU64,
    scroll_commits: AtomicU64,
    missing_after_navigation: AtomicU64,
}

#[derive(Clone)]
struct GeometryResourceBudget {
    ledger: ResourceLedger,
    profile: CdpSelectorGeometryProductionProfile,
    active_resolutions: Arc<AtomicUsize>,
    active_context_routes: Arc<AtomicUsize>,
}

struct ResolutionLease {
    resource: Option<ResourceLease>,
    active_resolutions: Arc<AtomicUsize>,
}

impl Drop for ResolutionLease {
    fn drop(&mut self) {
        drop(self.resource.take());
        let previous = self.active_resolutions.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0);
    }
}

struct ContextRouteLease {
    resource: Option<ResourceLease>,
    active_context_routes: Arc<AtomicUsize>,
}

impl Drop for ContextRouteLease {
    fn drop(&mut self) {
        drop(self.resource.take());
        let previous = self.active_context_routes.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0);
    }
}

/// Factory sharing one declared concurrency and context-route budget.
#[derive(Clone)]
pub struct CdpSelectorGeometryServiceFactory {
    retry_delay: Duration,
    resources: GeometryResourceBudget,
    counters: Arc<SelectorGeometryCounters>,
}

impl CdpSelectorGeometryServiceFactory {
    pub fn new_with_production_resources(
        profile: CdpSelectorGeometryProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        Self::with_retry_delay_and_production_resources(DEFAULT_RETRY_DELAY, profile, resources)
    }

    pub fn with_retry_delay_and_production_resources(
        retry_delay: Duration,
        profile: CdpSelectorGeometryProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        let ledger = resources.claim(profile.requirements())?;
        Ok(Self {
            retry_delay,
            resources: GeometryResourceBudget {
                ledger,
                profile,
                active_resolutions: Arc::new(AtomicUsize::new(0)),
                active_context_routes: Arc::new(AtomicUsize::new(0)),
            },
            counters: Arc::new(SelectorGeometryCounters::default()),
        })
    }

    pub fn bind(&self, client: Arc<dyn CdpClient>) -> CdpSelectorGeometryService {
        CdpSelectorGeometryService::from_parts(
            client,
            None,
            None,
            self.retry_delay,
            Some(self.resources.clone()),
            Arc::clone(&self.counters),
        )
    }

    pub fn bind_unique_context(
        &self,
        client: Arc<dyn CdpClient>,
        unique_context_id: String,
    ) -> Result<CdpSelectorGeometryService, OperationError<LocatorScreenshotErrorDetails>> {
        let unique_context_id: Arc<str> = Arc::from(unique_context_id);
        let route = self.reserve_context_route(&unique_context_id)?;
        Ok(CdpSelectorGeometryService::from_parts(
            client,
            Some(unique_context_id),
            Some(Arc::new(route)),
            self.retry_delay,
            Some(self.resources.clone()),
            Arc::clone(&self.counters),
        ))
    }

    fn reserve_context_route(
        &self,
        unique_context_id: &str,
    ) -> Result<ContextRouteLease, OperationError<LocatorScreenshotErrorDetails>> {
        if unique_context_id.len() > self.resources.profile.max_native_id_bytes() {
            return Err(resource_error(format!(
                "selector-geometry context id exceeds the {}-byte profile limit",
                self.resources.profile.max_native_id_bytes()
            )));
        }
        let limit = self.resources.profile.max_context_routes();
        if self
            .resources
            .active_context_routes
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                (current < limit).then_some(current + 1)
            })
            .is_err()
        {
            return Err(resource_error(
                "selector-geometry context-route limit was reached",
            ));
        }
        let resource = match self.resources.ledger.reserve(
            self.resources.profile.resource_scope(),
            ResourceClass::BackendStateBytes,
            self.resources
                .profile
                .maximum_retained_bytes_per_context_route(),
        ) {
            Ok(resource) => resource,
            Err(error) => {
                let previous = self
                    .resources
                    .active_context_routes
                    .fetch_sub(1, Ordering::AcqRel);
                debug_assert!(previous > 0);
                return Err(map_resource_error(error));
            }
        };
        Ok(ContextRouteLease {
            resource: Some(resource),
            active_context_routes: Arc::clone(&self.resources.active_context_routes),
        })
    }

    pub fn metrics(&self) -> SelectorGeometryMetrics {
        metrics(&self.counters)
    }

    pub const fn profile(&self) -> CdpSelectorGeometryProductionProfile {
        self.resources.profile
    }
}

/// One target-local selector-geometry resolver.
#[derive(Clone)]
pub struct CdpSelectorGeometryService {
    client: Arc<dyn CdpClient>,
    unique_context_id: Option<Arc<str>>,
    _context_route: Option<Arc<ContextRouteLease>>,
    retry_delay: Duration,
    runtime_ready: Arc<tokio::sync::OnceCell<()>>,
    counters: Arc<SelectorGeometryCounters>,
    resources: Option<GeometryResourceBudget>,
}

impl CdpSelectorGeometryService {
    pub fn new(client: Arc<dyn CdpClient>) -> Self {
        Self::with_retry_delay(client, DEFAULT_RETRY_DELAY)
    }

    pub fn with_retry_delay(client: Arc<dyn CdpClient>, retry_delay: Duration) -> Self {
        Self::from_parts(
            client,
            None,
            None,
            retry_delay,
            None,
            Arc::new(SelectorGeometryCounters::default()),
        )
    }

    fn from_parts(
        client: Arc<dyn CdpClient>,
        unique_context_id: Option<Arc<str>>,
        context_route: Option<Arc<ContextRouteLease>>,
        retry_delay: Duration,
        resources: Option<GeometryResourceBudget>,
        counters: Arc<SelectorGeometryCounters>,
    ) -> Self {
        Self {
            client,
            unique_context_id,
            _context_route: context_route,
            retry_delay,
            runtime_ready: Arc::new(tokio::sync::OnceCell::new()),
            counters,
            resources,
        }
    }

    pub fn metrics(&self) -> SelectorGeometryMetrics {
        metrics(&self.counters)
    }

    pub fn unique_context_id(&self) -> Option<&str> {
        self.unique_context_id.as_deref()
    }

    fn reserve_resolution(
        &self,
        selector: &Selector,
    ) -> Result<Option<ResolutionLease>, OperationError<LocatorScreenshotErrorDetails>> {
        let Some(resources) = &self.resources else {
            return measure_selector(selector)
                .map(|_| None)
                .map_err(map_selector_error);
        };
        let storage = measure_selector(selector).map_err(map_selector_error)?;
        if storage.string_capacity() > MAX_SELECTOR_STRING_BYTES {
            return Err(resource_error(
                "selector string capacities exceed 65536 bytes",
            ));
        }
        let limit = resources.profile.max_concurrent_resolutions();
        if resources
            .active_resolutions
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                (current < limit).then_some(current + 1)
            })
            .is_err()
        {
            return Err(resource_error(
                "selector-geometry resolution concurrency limit was reached",
            ));
        }
        let resource = match resources.ledger.reserve(
            resources.profile.resource_scope(),
            ResourceClass::SelectorBytes,
            resources.profile.maximum_retained_bytes_per_resolution(),
        ) {
            Ok(resource) => resource,
            Err(error) => {
                let previous = resources.active_resolutions.fetch_sub(1, Ordering::AcqRel);
                debug_assert!(previous > 0);
                return Err(map_resource_error(error));
            }
        };
        Ok(Some(ResolutionLease {
            resource: Some(resource),
            active_resolutions: Arc::clone(&resources.active_resolutions),
        }))
    }

    async fn evaluate(&self, expression: &str) -> pwright_cdp::connection::Result<Value> {
        match self.unique_context_id.as_deref() {
            Some(unique_context_id) => {
                self.client
                    .runtime_evaluate_in_unique_context(expression, unique_context_id)
                    .await
            }
            None => self.client.runtime_evaluate(expression).await,
        }
    }

    async fn ensure_runtime(&self) -> Result<(), OperationError<LocatorScreenshotErrorDetails>> {
        self.runtime_ready
            .get_or_try_init(|| self.install_runtime())
            .await
            .map(|_| ())
    }

    async fn install_runtime(&self) -> Result<(), OperationError<LocatorScreenshotErrorDetails>> {
        let response = self
            .evaluate(&selector_runtime_install_script())
            .await
            .map_err(map_cdp_error)?;
        let installed = response
            .get("result")
            .and_then(|result| result.get("value"))
            .and_then(Value::as_bool)
            .ok_or_else(|| {
                invalid_response("selector-runtime installation did not return a boolean")
            })?;
        if !installed {
            return Err(invalid_response(
                "selector-runtime installation reported failure",
            ));
        }
        self.counters.installations.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn resolve_geometry(
        &self,
        selector: Selector,
    ) -> Result<LocatorScreenshotGeometry, OperationError<LocatorScreenshotErrorDetails>> {
        let _resolution = self.reserve_resolution(&selector)?;
        let selector_label = selector.to_string();
        let check_script = screenshot_geometry_check_script(&selector)?;
        let commit_script = scroll_commit_script(&selector)?;
        self.ensure_runtime().await?;
        'resolve: loop {
            self.counters.evaluations.fetch_add(1, Ordering::Relaxed);
            let response = self.evaluate(&check_script).await.map_err(map_cdp_error)?;
            match parse_geometry_attempt(response, &selector_label)? {
                GeometryAttempt::Done(geometry) => return Ok(geometry),
                GeometryAttempt::Retry => tokio::time::sleep(self.retry_delay).await,
                GeometryAttempt::RuntimeMissing => {
                    self.counters
                        .missing_after_navigation
                        .fetch_add(1, Ordering::Relaxed);
                    self.install_runtime().await?;
                }
                GeometryAttempt::ReadyToScroll => {
                    self.counters.scroll_commits.fetch_add(1, Ordering::Relaxed);
                    let response = self.evaluate(&commit_script).await.map_err(map_cdp_error)?;
                    match parse_scroll_attempt(response, &selector_label)? {
                        ScrollAttempt::Done | ScrollAttempt::Retry => continue 'resolve,
                        ScrollAttempt::RuntimeMissing => {
                            self.counters
                                .missing_after_navigation
                                .fetch_add(1, Ordering::Relaxed);
                            self.install_runtime().await?;
                            continue 'resolve;
                        }
                        ScrollAttempt::ReadyToScroll => {
                            return Err(invalid_response(
                                "Locator screenshot scroll commit returned a precommit status",
                            ));
                        }
                    }
                }
            }
        }
    }
}

impl LocatorScreenshotGeometryResolver for CdpSelectorGeometryService {
    fn resolve(
        &self,
        selector: Selector,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        LocatorScreenshotGeometry,
                        OperationError<LocatorScreenshotErrorDetails>,
                    >,
                > + Send
                + '_,
        >,
    > {
        Box::pin(self.resolve_geometry(selector))
    }
}

fn metrics(counters: &SelectorGeometryCounters) -> SelectorGeometryMetrics {
    SelectorGeometryMetrics {
        installations: counters.installations.load(Ordering::Relaxed),
        evaluations: counters.evaluations.load(Ordering::Relaxed),
        scroll_commits: counters.scroll_commits.load(Ordering::Relaxed),
        missing_after_navigation: counters.missing_after_navigation.load(Ordering::Relaxed),
    }
}

fn screenshot_geometry_check_script(
    selector: &Selector,
) -> Result<String, OperationError<LocatorScreenshotErrorDetails>> {
    let selector = compile_selector(selector, "document").map_err(map_selector_error)?;
    let script = format!(
        r#"(async () => {{
  const __pw = globalThis[Symbol.for({runtime_symbol:?})];
  if (!__pw || __pw.version !== {runtime_version}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (matches.length === 0) return {{ status: "missing" }};
  if (matches.length > 1) return {{ status: "strict", count: matches.length }};
  const element = matches[0];
  if (!element || !element.isConnected) return {{ status: "detached" }};
  if (element.nodeType !== 1)
    return {{ status: "invalid-target", message: "Node is not an Element" }};
  if (!__pw.visible(element) || element.getClientRects().length === 0)
    return {{ status: "notvisible" }};
  const before = element.getBoundingClientRect();
  await new Promise(resolve => {{
    if (document.visibilityState !== "visible") {{ setTimeout(resolve, 0); return; }}
    requestAnimationFrame(() => requestAnimationFrame(resolve));
  }});
  if (!element.isConnected) return {{ status: "detached" }};
  if (!__pw.visible(element) || element.getClientRects().length === 0)
    return {{ status: "notvisible" }};
  const after = element.getBoundingClientRect();
  const stable = Math.abs(before.x - after.x) <= 0.25 &&
    Math.abs(before.y - after.y) <= 0.25 &&
    Math.abs(before.width - after.width) <= 0.25 &&
    Math.abs(before.height - after.height) <= 0.25;
  if (!stable) return {{ status: "unstable" }};
  let ratio;
  try {{
    ratio = await new Promise(resolve => {{
      const observer = new IntersectionObserver(entries => {{
        observer.disconnect();
        resolve(entries[0]?.intersectionRatio ?? 0);
      }});
      observer.observe(element);
    }});
  }} catch (error) {{ return {{ status: "backend", message: String(error) }}; }}
  if (!element.isConnected) return {{ status: "detached" }};
  if (ratio !== 1) return {{ status: "ready-to-scroll" }};
  const final = element.getBoundingClientRect();
  const finalStable = Math.abs(after.x - final.x) <= 0.25 &&
    Math.abs(after.y - final.y) <= 0.25 &&
    Math.abs(after.width - final.width) <= 0.25 &&
    Math.abs(after.height - final.height) <= 0.25;
  if (!finalStable) return {{ status: "unstable" }};
  const x = Math.floor(final.left);
  const y = Math.floor(final.top);
  const right = Math.ceil(final.right);
  const bottom = Math.ceil(final.bottom);
  if (![x, y, right, bottom].every(Number.isFinite) || right <= x || bottom <= y)
    return {{ status: "no-layout" }};
  return {{ status: "done", x, y, width: right - x, height: bottom - y }};
}})()"#,
        runtime_symbol = SELECTOR_RUNTIME_SYMBOL,
        runtime_version = SELECTOR_RUNTIME_VERSION,
    );
    bounded_script(script, "Locator screenshot geometry")
}

fn scroll_commit_script(
    selector: &Selector,
) -> Result<String, OperationError<LocatorScreenshotErrorDetails>> {
    let selector = compile_selector(selector, "document").map_err(map_selector_error)?;
    let script = format!(
        r#"(() => {{
  const __pw = globalThis[Symbol.for({runtime_symbol:?})];
  if (!__pw || __pw.version !== {runtime_version}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (matches.length === 0) return {{ status: "missing" }};
  if (matches.length > 1) return {{ status: "strict", count: matches.length }};
  const element = matches[0];
  if (!element || !element.isConnected) return {{ status: "detached" }};
  if (element.nodeType !== 1)
    return {{ status: "invalid-target", message: "Node is not an Element" }};
  if (element.getClientRects().length === 0) return {{ status: "no-layout" }};
  try {{
    element.scrollIntoView({{ behavior: "instant", block: "center", inline: "center" }});
  }} catch (error) {{ return {{ status: "backend", message: String(error) }}; }}
  return {{ status: "done" }};
}})()"#,
        runtime_symbol = SELECTOR_RUNTIME_SYMBOL,
        runtime_version = SELECTOR_RUNTIME_VERSION,
    );
    bounded_script(script, "Locator screenshot scroll commit")
}

fn bounded_script(
    script: String,
    label: &str,
) -> Result<String, OperationError<LocatorScreenshotErrorDetails>> {
    if script.len() > MAX_GEOMETRY_SCRIPT_BYTES {
        Err(resource_error(format!(
            "{label} script exceeds its declared backend bound"
        )))
    } else {
        Ok(script)
    }
}

enum GeometryAttempt {
    Done(LocatorScreenshotGeometry),
    ReadyToScroll,
    Retry,
    RuntimeMissing,
}

enum ScrollAttempt {
    Done,
    ReadyToScroll,
    Retry,
    RuntimeMissing,
}

fn response_value<'a>(
    response: &'a Value,
    missing_status_message: &str,
) -> Result<&'a serde_json::Map<String, Value>, OperationError<LocatorScreenshotErrorDetails>> {
    let value = response
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing object result.value"))?;
    if !value.contains_key("status") {
        return Err(invalid_response(missing_status_message));
    }
    Ok(value)
}

fn parse_geometry_attempt(
    response: Value,
    selector: &str,
) -> Result<GeometryAttempt, OperationError<LocatorScreenshotErrorDetails>> {
    let value = response_value(
        &response,
        "Locator screenshot geometry result is missing status",
    )?;
    let status = value["status"]
        .as_str()
        .ok_or_else(|| invalid_response("Locator screenshot geometry status is not a string"))?;
    match status {
        "done" => {
            let field = |name| {
                value
                    .get(name)
                    .and_then(Value::as_f64)
                    .filter(|number| number.is_finite())
                    .ok_or_else(|| {
                        invalid_response(format!(
                            "Locator screenshot geometry result is missing finite `{name}`"
                        ))
                    })
            };
            let geometry = LocatorScreenshotGeometry::new(
                field("x")?,
                field("y")?,
                field("width")?,
                field("height")?,
            )
            .map_err(|error| invalid_response(error.to_string()))?;
            Ok(GeometryAttempt::Done(geometry))
        }
        "ready-to-scroll" => Ok(GeometryAttempt::ReadyToScroll),
        "missing" | "detached" | "notvisible" | "no-layout" | "unstable" | "retry" => {
            Ok(GeometryAttempt::Retry)
        }
        "runtime-missing" => Ok(GeometryAttempt::RuntimeMissing),
        "strict" => strict_error(value, selector).map(|()| GeometryAttempt::Retry),
        "invalid" => Err(invalid_selector_error(value, selector)),
        "invalid-target" | "backend" => Err(backend_status_error(
            value,
            "Locator screenshot geometry failed",
        )),
        other => Err(invalid_response(format!(
            "Locator screenshot geometry returned unknown status `{other}`"
        ))),
    }
}

fn parse_scroll_attempt(
    response: Value,
    selector: &str,
) -> Result<ScrollAttempt, OperationError<LocatorScreenshotErrorDetails>> {
    let value = response_value(
        &response,
        "Locator screenshot scroll result is missing status",
    )?;
    let status = value["status"]
        .as_str()
        .ok_or_else(|| invalid_response("Locator screenshot scroll status is not a string"))?;
    match status {
        "done" => Ok(ScrollAttempt::Done),
        "ready-to-scroll" => Ok(ScrollAttempt::ReadyToScroll),
        "missing" | "detached" | "no-layout" | "unstable" | "retry" => Ok(ScrollAttempt::Retry),
        "runtime-missing" => Ok(ScrollAttempt::RuntimeMissing),
        "strict" => strict_error(value, selector).map(|()| ScrollAttempt::Retry),
        "invalid" => Err(invalid_selector_error(value, selector)),
        "invalid-target" | "backend" => Err(backend_status_error(
            value,
            "native scroll operation failed",
        )),
        other => Err(invalid_response(format!(
            "Locator screenshot scroll returned unknown status `{other}`"
        ))),
    }
}

fn strict_error(
    value: &serde_json::Map<String, Value>,
    selector: &str,
) -> Result<(), OperationError<LocatorScreenshotErrorDetails>> {
    let count = value
        .get("count")
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid_response("strict result is missing match count"))?;
    let count = u32::try_from(count).map_err(|_| {
        OperationError::new(
            FailureKind::ResourceExhausted,
            "selector match count exceeds the portable limit",
            Some(LocatorScreenshotErrorDetails::StrictViolation {
                selector: selector.to_owned(),
                matches: u32::MAX,
            }),
        )
    })?;
    Err(OperationError::new(
        FailureKind::Backend,
        format!("strict selector `{selector}` resolved to {count} elements"),
        Some(LocatorScreenshotErrorDetails::StrictViolation {
            selector: selector.to_owned(),
            matches: count,
        }),
    ))
}

fn invalid_selector_error(
    value: &serde_json::Map<String, Value>,
    selector: &str,
) -> OperationError<LocatorScreenshotErrorDetails> {
    let message = status_message(value, "invalid selector");
    OperationError::new(
        FailureKind::Backend,
        format!("invalid selector `{selector}`: {message}"),
        Some(LocatorScreenshotErrorDetails::InvalidSelector(message)),
    )
}

fn backend_status_error(
    value: &serde_json::Map<String, Value>,
    fallback: &str,
) -> OperationError<LocatorScreenshotErrorDetails> {
    let message = status_message(value, fallback);
    OperationError::new(
        FailureKind::Backend,
        message.clone(),
        Some(LocatorScreenshotErrorDetails::Backend(message)),
    )
}

fn status_message(value: &serde_json::Map<String, Value>, fallback: &str) -> String {
    value
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_owned()
}

fn map_selector_error(
    error: OperationError<LocatorErrorDetails>,
) -> OperationError<LocatorScreenshotErrorDetails> {
    error.map_details(|details| match details {
        LocatorErrorDetails::InvalidSelector(message) => {
            LocatorScreenshotErrorDetails::InvalidSelector(message)
        }
        LocatorErrorDetails::StrictViolation { selector, matches } => {
            LocatorScreenshotErrorDetails::StrictViolation { selector, matches }
        }
        LocatorErrorDetails::InvalidBackendResponse(message) => {
            LocatorScreenshotErrorDetails::InvalidBackendResponse(message)
        }
        LocatorErrorDetails::Backend(message)
        | LocatorErrorDetails::InvalidTarget(message)
        | LocatorErrorDetails::InvalidValue(message) => {
            LocatorScreenshotErrorDetails::Backend(message)
        }
    })
}

fn invalid_response(message: impl Into<String>) -> OperationError<LocatorScreenshotErrorDetails> {
    let message = message.into();
    OperationError::new(
        FailureKind::ProtocolViolation,
        message.clone(),
        Some(LocatorScreenshotErrorDetails::InvalidBackendResponse(
            message,
        )),
    )
}

fn resource_error(message: impl Into<String>) -> OperationError<LocatorScreenshotErrorDetails> {
    let message = message.into();
    OperationError::new(
        FailureKind::ResourceExhausted,
        message.clone(),
        Some(LocatorScreenshotErrorDetails::Resource(message)),
    )
}

fn map_resource_error(error: ResourceError) -> OperationError<LocatorScreenshotErrorDetails> {
    resource_error(error.to_string())
}

fn map_cdp_error(
    error: pwright_cdp::connection::CdpError,
) -> OperationError<LocatorScreenshotErrorDetails> {
    let kind = classify_cdp_failure(&error);
    let message = error.to_string();
    OperationError::new(
        kind,
        message.clone(),
        Some(LocatorScreenshotErrorDetails::Backend(message)),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use pwright_backend_cdp_locator_screenshot::LocatorScreenshotGeometryResolver;
    use pwright_cap_locator_screenshot::LocatorScreenshotErrorDetails;
    use pwright_core::{
        FailureKind, ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope,
    };
    use pwright_fake::FakeCdpClient;
    use pwright_selector_model::Selector;
    use serde_json::json;

    use super::{
        CdpSelectorGeometryProductionProfile, CdpSelectorGeometryService,
        CdpSelectorGeometryServiceFactory, SelectorGeometryMetrics,
    };

    fn selector(value: &str) -> Selector {
        Selector::css(value).expect("selector should validate")
    }

    fn runtime_installed() -> serde_json::Value {
        json!({"result": {"value": true}})
    }

    fn production_factory(
        profile: CdpSelectorGeometryProductionProfile,
    ) -> CdpSelectorGeometryServiceFactory {
        let requirements = profile.requirements();
        let selector = requirements.class(ResourceClass::SelectorBytes);
        let backend = requirements.class(ResourceClass::BackendStateBytes);
        let limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases)
            .with_class(
                ResourceClass::SelectorBytes,
                selector.global,
                selector.per_scope,
            )
            .with_class(
                ResourceClass::BackendStateBytes,
                backend.global,
                backend.per_scope,
            );
        let ledger = ProductionResourceLedger::new(limits, requirements)
            .expect("selector-geometry production plan should fit");
        CdpSelectorGeometryServiceFactory::with_retry_delay_and_production_resources(
            Duration::ZERO,
            profile,
            &ledger,
        )
        .expect("selector-geometry factory should install")
    }

    #[test]
    fn profile_declares_selector_and_optional_context_storage_separately() {
        // Arrange
        let base = CdpSelectorGeometryProductionProfile::new(ResourceScope::Connection, 2);

        // Act
        let profile = base.with_context_routes(3, 64);
        let requirements = profile.requirements();

        // Assert
        assert_eq!(
            requirements.class(ResourceClass::SelectorBytes).global,
            2 * profile.maximum_retained_bytes_per_resolution()
        );
        assert_eq!(
            requirements.class(ResourceClass::BackendStateBytes).global,
            3 * profile.maximum_retained_bytes_per_context_route()
        );
        assert_eq!(requirements.leases, 5);
    }

    #[tokio::test]
    async fn resolves_stable_geometry_with_one_shared_runtime_installation() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>Save</button>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "x": 10.0, "y": 20.0, "width": 30.0, "height": 40.0}}}),
        ]);
        let service = CdpSelectorGeometryService::with_retry_delay(client.clone(), Duration::ZERO);

        // Act
        let geometry = service
            .resolve(selector("button"))
            .await
            .expect("stable target should resolve");

        // Assert
        assert_eq!((geometry.x(), geometry.y()), (10.0, 20.0));
        assert_eq!((geometry.width(), geometry.height()), (30.0, 40.0));
        assert_eq!(
            service.metrics(),
            SelectorGeometryMetrics {
                installations: 1,
                evaluations: 1,
                scroll_commits: 0,
                missing_after_navigation: 0,
            }
        );
        assert_eq!(
            client.method_names(),
            vec!["Runtime.evaluate", "Runtime.evaluate"]
        );
    }

    #[tokio::test]
    async fn scrolls_at_commit_then_resolves_fresh_geometry() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<div id='below'></div>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "ready-to-scroll"}}}),
            json!({"result": {"value": {"status": "done"}}}),
            json!({"result": {"value": {"status": "done", "x": 1.0, "y": 2.0, "width": 3.0, "height": 4.0}}}),
        ]);
        let service = CdpSelectorGeometryService::with_retry_delay(client, Duration::ZERO);

        // Act
        let geometry = service
            .resolve(selector("#below"))
            .await
            .expect("scrolled target should resolve");

        // Assert
        assert_eq!(geometry.height(), 4.0);
        assert_eq!(service.metrics().evaluations, 2);
        assert_eq!(service.metrics().scroll_commits, 1);
    }

    #[tokio::test]
    async fn preserves_strict_violation_without_installing_locator_commands() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(
            "<button>A</button><button>B</button>",
        ));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "strict", "count": 2}}}),
        ]);
        let service = CdpSelectorGeometryService::new(client);

        // Act
        let error = service
            .resolve(selector("button"))
            .await
            .expect_err("multiple matches should fail strictness");

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert_eq!(
            error.details(),
            Some(&LocatorScreenshotErrorDetails::StrictViolation {
                selector: "button".to_owned(),
                matches: 2,
            })
        );
    }

    #[tokio::test]
    async fn zero_resolution_capacity_rejects_before_cdp() {
        // Arrange
        let profile = CdpSelectorGeometryProductionProfile::new(ResourceScope::Connection, 0);
        let factory = production_factory(profile);
        let client = Arc::new(FakeCdpClient::from_html("<button>Save</button>"));
        let service = factory.bind(client.clone());

        // Act
        let error = service
            .resolve(selector("button"))
            .await
            .expect_err("zero capacity should reject");

        // Assert
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
        assert!(client.calls().is_empty());
    }

    #[test]
    fn unique_context_routes_are_bounded_and_released_with_service_clones() {
        // Arrange
        let profile = CdpSelectorGeometryProductionProfile::new(ResourceScope::Connection, 1)
            .with_context_routes(1, 16);
        let factory = production_factory(profile);
        let client = Arc::new(FakeCdpClient::from_html("<div></div>"));

        // Act
        let first = factory
            .bind_unique_context(client.clone(), "world-1".to_owned())
            .expect("first context route should fit");
        let clone = first.clone();
        let rejected = factory.bind_unique_context(client.clone(), "world-2".to_owned());
        drop(first);
        let still_rejected = factory.bind_unique_context(client.clone(), "world-2".to_owned());
        drop(clone);
        let second = factory
            .bind_unique_context(client, "world-2".to_owned())
            .expect("route should be released after the last clone");

        // Assert
        assert!(rejected.is_err());
        assert!(still_rejected.is_err());
        assert_eq!(second.unique_context_id(), Some("world-2"));
    }
}
