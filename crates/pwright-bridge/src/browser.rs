use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use pwright_cdp::connection::{CdpError, Result as CdpResult};
use pwright_cdp::{CdpClient, CdpConnection, CdpSession, CdpSessionFactory, SessionFactory};
use tokio::sync::{Mutex, RwLock, Semaphore};
use tracing::{debug, info};

use crate::playwright::Page;

/// Supported URL scheme prefixes for CDP connections.
///
/// HTTP/HTTPS triggers `/json/version` discovery + URL rewrite.
/// WS/WSS connects directly.
pub const SUPPORTED_SCHEMES: &[&str] = &["ws://", "wss://", "http://", "https://"];

/// Returns `true` if the URL uses a supported CDP connection scheme.
pub fn is_supported_scheme(url: &str) -> bool {
    SUPPORTED_SCHEMES.iter().any(|s| url.starts_with(s))
}

/// Returns `true` if the URL uses an HTTP(S) scheme (needs discovery).
fn is_http_scheme(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

/// Strategy for closing a tab. Implementations choose the transport.
#[async_trait]
pub trait TabCloser: Send + Sync {
    async fn close_tab(&self, target_id: &str) -> CdpResult<()>;
}

/// Close a tab via CDP WebSocket (`Target.closeTarget`).
pub struct CdpTabCloser {
    client: Arc<dyn CdpClient>,
}

impl CdpTabCloser {
    pub fn new(client: Arc<dyn CdpClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl TabCloser for CdpTabCloser {
    async fn close_tab(&self, target_id: &str) -> CdpResult<()> {
        self.client.target_close(target_id).await
    }
}

/// Close and verify a tab via Chrome's HTTP DevTools API.
///
/// Unlike [`CdpTabCloser`] which sends `Target.closeTarget` over the shared
/// CDP WebSocket, this uses Chrome's separate HTTP server thread. This makes
/// it immune to head-of-line blocking when Chrome's renderer is busy (e.g.,
/// heavy SPA pages flooding the WebSocket with DOM/Runtime events). A close is
/// successful only after `/json/list` no longer contains the target.
pub struct HttpTabCloser {
    cdp_http_url: String,
    timeout: std::time::Duration,
}

impl HttpTabCloser {
    pub fn new(cdp_http_url: String, timeout: std::time::Duration) -> Self {
        Self {
            cdp_http_url: cdp_http_url.trim_end_matches('/').to_owned(),
            timeout,
        }
    }

    async fn target_exists(&self, client: &reqwest::Client, target_id: &str) -> CdpResult<bool> {
        let targets = client
            .get(format!("{}/json/list", self.cdp_http_url))
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|error| CdpError::HttpFailed(format!("HTTP target list failed: {error}")))?
            .json::<Vec<serde_json::Value>>()
            .await
            .map_err(|error| {
                CdpError::HttpFailed(format!("decode HTTP target list failed: {error}"))
            })?;
        Ok(targets.iter().any(|target| {
            target
                .get("id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| id == target_id)
        }))
    }

    async fn wait_until_closed(&self, client: &reqwest::Client, target_id: &str) -> CdpResult<()> {
        let started_at = tokio::time::Instant::now();
        loop {
            if !self.target_exists(client, target_id).await? {
                return Ok(());
            }
            if started_at.elapsed() >= self.timeout {
                return Err(CdpError::HttpFailed(format!(
                    "target {target_id} remained open after HTTP close"
                )));
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
}

#[async_trait]
impl TabCloser for HttpTabCloser {
    async fn close_tab(&self, target_id: &str) -> CdpResult<()> {
        let url = format!("{}/json/close/{}", self.cdp_http_url, target_id);
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| CdpError::HttpFailed(format!("build HTTP client: {e}")))?;
        let close_result = client
            .get(&url)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|error| CdpError::HttpFailed(format!("HTTP close failed: {error}")));
        let target_exists = self.target_exists(&client, target_id).await?;
        if !target_exists {
            return Ok(());
        }
        close_result?;
        self.wait_until_closed(&client, target_id).await
    }
}

struct FallbackTabCloser {
    primary: Arc<dyn TabCloser>,
    fallback: Arc<dyn TabCloser>,
}

impl FallbackTabCloser {
    fn new(primary: Arc<dyn TabCloser>, fallback: Arc<dyn TabCloser>) -> Self {
        Self { primary, fallback }
    }
}

struct BrowserContextTabCloser {
    target_closer: Arc<dyn TabCloser>,
    browser_client: Arc<dyn CdpClient>,
    browser_context_id: String,
}

impl BrowserContextTabCloser {
    fn new(
        target_closer: Arc<dyn TabCloser>,
        browser_client: Arc<dyn CdpClient>,
        browser_context_id: String,
    ) -> Self {
        Self {
            target_closer,
            browser_client,
            browser_context_id,
        }
    }
}

#[async_trait]
impl TabCloser for BrowserContextTabCloser {
    async fn close_tab(&self, target_id: &str) -> CdpResult<()> {
        let target_result = self.target_closer.close_tab(target_id).await;
        let context_result = self
            .browser_client
            .target_dispose_browser_context(&self.browser_context_id)
            .await;
        match (target_result, context_result) {
            (Ok(()), Ok(())) | (Err(_), Ok(())) => Ok(()),
            (Ok(()), Err(context_error)) => Err(CdpError::Other(format!(
                "target {target_id} closed but browser context {} cleanup failed: {context_error}",
                self.browser_context_id
            ))),
            (Err(target_error), Err(context_error)) => Err(CdpError::Other(format!(
                "target {target_id} close failed: {target_error}; browser context {} cleanup also failed: {context_error}",
                self.browser_context_id
            ))),
        }
    }
}

#[async_trait]
impl TabCloser for FallbackTabCloser {
    async fn close_tab(&self, target_id: &str) -> CdpResult<()> {
        match self.primary.close_tab(target_id).await {
            Ok(()) => Ok(()),
            Err(primary_error) => {
                tracing::warn!(
                    target_id,
                    error = %primary_error,
                    "primary tab close failed; trying fallback transport"
                );
                self.fallback
                    .close_tab(target_id)
                    .await
                    .map_err(|fallback_error| {
                        CdpError::Other(format!(
                            "primary tab close failed: {primary_error}; fallback tab close failed: {fallback_error}"
                        ))
                    })?;
                self.primary.close_tab(target_id).await.map_err(
                    |verification_error| {
                        CdpError::Other(format!(
                            "primary tab close failed: {primary_error}; fallback close was sent but target closure could not be verified: {verification_error}"
                        ))
                    },
                )
            }
        }
    }
}

/// Rewrite a WebSocket debugger URL to use the host/port from an HTTP URL.
///
/// Chrome returns `ws://127.0.0.1:9222/devtools/browser/...` but when connecting
/// through a proxy, we need `ws://proxy:9222/devtools/browser/...`.
pub fn rewrite_ws_url(http_url: &str, ws_url: &str) -> CdpResult<String> {
    let http_parsed =
        url::Url::parse(http_url).map_err(|e| CdpError::Other(format!("invalid HTTP URL: {e}")))?;
    let ws_parsed =
        url::Url::parse(ws_url).map_err(|e| CdpError::Other(format!("invalid WS URL: {e}")))?;

    let host = http_parsed
        .host_str()
        .ok_or_else(|| CdpError::Other("HTTP URL has no host".to_string()))?;
    let port = http_parsed.port().unwrap_or(80);

    let mut rewritten = ws_parsed.clone();
    rewritten
        .set_host(Some(host))
        .map_err(|_| CdpError::Other("failed to set host".to_string()))?;
    rewritten
        .set_port(Some(port))
        .map_err(|_| CdpError::Other("failed to set port".to_string()))?;

    Ok(rewritten.to_string())
}

use crate::snapshot::RefCache;
use crate::tab::Tab;

/// Configuration for connecting to a browser via CDP.
///
/// The `cdp_url` can be either:
/// - **HTTP** (e.g. `http://proxy:9222`) — fetches `/json/version` to discover the
///   WebSocket URL, rewrites host/port to match the caller's endpoint, then connects.
/// - **WebSocket** (e.g. `ws://127.0.0.1:9333/`) — connects directly.
///
/// Tab close defaults to CDP WebSocket (`Target.closeTarget`). Set
/// `tab_close_via_http` to use the HTTP DevTools API instead, which is
/// more reliable when Chrome's renderer is under heavy load.
#[derive(Debug, Clone)]
pub struct BrowserConfig {
    /// CDP endpoint URL. HTTP triggers discovery + rewrite; WS connects directly.
    pub cdp_url: String,
    /// Timeout for individual CDP commands in milliseconds.
    ///
    /// This is separate from navigation and wait timeouts because commands that
    /// return large values (for example, a bounded document body) may need more
    /// time to cross a busy remote CDP connection.
    pub command_timeout_ms: u64,
    /// Maximum number of concurrent tab operations
    pub max_parallel_tabs: usize,
    /// Default navigation timeout in milliseconds
    pub navigate_timeout_ms: u64,
    /// When true, `TabHandle::close()` uses HTTP `GET /json/close/<id>` instead
    /// of CDP WebSocket `Target.closeTarget`. HTTP close runs on Chrome's
    /// separate IO thread and avoids head-of-line blocking from event-heavy pages.
    pub tab_close_via_http: bool,
    /// Timeout for HTTP tab close requests. Only used when `tab_close_via_http` is true.
    pub tab_close_http_timeout_ms: u64,
    /// Optional proxy override for isolated ephemeral tabs.
    ///
    /// This is applied through `Target.createBrowserContext`, so it does not
    /// alter persistent tabs or the shared Chrome process.
    pub ephemeral_proxy_server: Option<String>,
    /// Optional Chromium-format bypass list for the ephemeral proxy override.
    pub ephemeral_proxy_bypass_list: Option<String>,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            cdp_url: String::new(),
            command_timeout_ms: 30_000,
            max_parallel_tabs: 4,
            navigate_timeout_ms: 30_000,
            tab_close_via_http: false,
            tab_close_http_timeout_ms: 5_000,
            ephemeral_proxy_server: None,
            ephemeral_proxy_bypass_list: None,
        }
    }
}

/// High-level browser controller. Wraps a CDP connection and manages tabs.
///
/// Note: this struct has many responsibilities (tab lifecycle, ref cache, concurrency
/// control, session factory). If it grows further, consider extracting ref_caches
/// into a standalone `RefCacheStore` and tab_locks/tab_semaphore into a `TabPool`.
pub struct Browser {
    browser_session: Arc<dyn CdpClient>,
    session_factory: Arc<dyn SessionFactory>,
    tabs: RwLock<HashMap<String, Tab>>,
    ref_caches: RwLock<HashMap<String, RefCache>>,
    config: BrowserConfig,
    tab_semaphore: Arc<Semaphore>,
    tab_locks: dashmap::DashMap<String, Arc<Mutex<()>>>,
    tab_counter: std::sync::atomic::AtomicU64,
}

impl Browser {
    fn tab_closer(&self) -> Arc<dyn TabCloser> {
        if self.config.tab_close_via_http {
            let http_url = if is_http_scheme(&self.config.cdp_url) {
                self.config.cdp_url.clone()
            } else {
                tracing::debug!(
                    "tab_close_via_http is set but cdp_url is WS; falling back to CDP close"
                );
                return Arc::new(CdpTabCloser::new(self.browser_session.clone()));
            };
            let timeout = std::time::Duration::from_millis(self.config.tab_close_http_timeout_ms);
            Arc::new(FallbackTabCloser::new(
                Arc::new(HttpTabCloser::new(http_url, timeout)),
                Arc::new(CdpTabCloser::new(self.browser_session.clone())),
            ))
        } else {
            Arc::new(CdpTabCloser::new(self.browser_session.clone()))
        }
    }

    fn ephemeral_tab_closer(&self, browser_context_id: Option<String>) -> Arc<dyn TabCloser> {
        let target_closer = self.tab_closer();
        match browser_context_id {
            Some(browser_context_id) => Arc::new(BrowserContextTabCloser::new(
                target_closer,
                self.browser_session.clone(),
                browser_context_id,
            )),
            None => target_closer,
        }
    }

    async fn create_ephemeral_target(&self, url: &str) -> CdpResult<(String, Option<String>)> {
        let has_proxy_override = self.config.ephemeral_proxy_server.is_some()
            || self.config.ephemeral_proxy_bypass_list.is_some();
        if !has_proxy_override {
            return self
                .browser_client()
                .target_create(url)
                .await
                .map(|target_id| (target_id, None));
        }

        let browser_context_id = self
            .browser_client()
            .target_create_browser_context(
                self.config.ephemeral_proxy_server.as_deref(),
                self.config.ephemeral_proxy_bypass_list.as_deref(),
            )
            .await?;
        match self
            .browser_client()
            .target_create_in_context(url, &browser_context_id)
            .await
        {
            Ok(target_id) => Ok((target_id, Some(browser_context_id))),
            Err(create_error) => {
                match self
                    .browser_client()
                    .target_dispose_browser_context(&browser_context_id)
                    .await
                {
                    Ok(()) => Err(create_error),
                    Err(dispose_error) => Err(CdpError::Other(format!(
                        "create target in browser context {browser_context_id} failed: {create_error}; context cleanup also failed: {dispose_error}"
                    ))),
                }
            }
        }
    }

    /// Close an existing target using the configured close transport.
    ///
    /// HTTP-configured browsers verify that the target disappeared before this
    /// method returns success.
    pub async fn close_target(&self, target_id: &str) -> CdpResult<()> {
        self.tab_closer().close_tab(target_id).await
    }

    /// Connect to a running browser via CDP.
    ///
    /// If `config.cdp_url` is an HTTP URL, fetches `/json/version` to discover the
    /// WebSocket URL and rewrites its host/port to match. If it's a WebSocket URL,
    /// connects directly. Tab management always uses CDP WebSocket.
    pub async fn connect(config: BrowserConfig) -> CdpResult<Arc<Self>> {
        let ws_url = if is_http_scheme(&config.cdp_url) {
            let discovered = discover_ws_url(
                &config.cdp_url,
                std::time::Duration::from_millis(config.command_timeout_ms),
            )
            .await?;
            let rewritten = rewrite_ws_url(&config.cdp_url, &discovered)?;
            debug!(
                http_url = config.cdp_url,
                raw_ws = discovered,
                rewritten_ws = rewritten,
                "discovered WebSocket URL via HTTP"
            );
            rewritten
        } else {
            config.cdp_url.clone()
        };

        info!(url = ws_url, "connecting to browser");
        let connection = CdpConnection::connect_with_timeout(
            &ws_url,
            std::time::Duration::from_millis(config.command_timeout_ms),
        )
        .await?;
        let browser = Arc::new(Self::build(connection, config));
        info!("connected to browser successfully");
        Ok(browser)
    }

    fn build(connection: Arc<CdpConnection>, config: BrowserConfig) -> Self {
        let browser_session: Arc<dyn CdpClient> = Arc::new(CdpSession::browser(connection.clone()));
        let session_factory: Arc<dyn SessionFactory> = Arc::new(CdpSessionFactory::new(connection));
        let max_parallel = config.max_parallel_tabs.max(1);
        Self {
            browser_session,
            session_factory,
            tabs: RwLock::new(HashMap::new()),
            ref_caches: RwLock::new(HashMap::new()),
            config,
            tab_semaphore: Arc::new(Semaphore::new(max_parallel)),
            tab_locks: dashmap::DashMap::new(),
            tab_counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Access the browser-level CDP client (for target management).
    pub fn browser_client(&self) -> &dyn CdpClient {
        &*self.browser_session
    }

    /// Create a new per-tab CDP session by attaching to a target.
    pub async fn attach_session(&self, target_id: &str) -> CdpResult<Arc<dyn CdpClient>> {
        let session_id = self.browser_client().target_attach(target_id).await?;
        let session = self
            .session_factory
            .create_session(session_id, target_id.to_string());
        session.page_enable().await?;
        session.runtime_enable().await?;
        Ok(session)
    }

    /// Attach to a target that this browser instance just created.
    ///
    /// A failed attach must close that exact target before returning; otherwise
    /// callers never receive a handle or target ID to clean up.
    pub(crate) async fn attach_created_target(
        &self,
        target_id: &str,
    ) -> CdpResult<Arc<dyn CdpClient>> {
        match self.attach_session(target_id).await {
            Ok(session) => Ok(session),
            Err(attach_error) => match self.close_target(target_id).await {
                Ok(()) => Err(attach_error),
                Err(close_error) => Err(CdpError::Other(format!(
                    "attach to newly created target {target_id} failed: {attach_error}; exact-target cleanup also failed: {close_error}"
                ))),
            },
        }
    }

    /// Get the config.
    pub fn config(&self) -> &BrowserConfig {
        &self.config
    }

    /// Generate a hash-based tab ID.
    pub(crate) fn next_tab_id(&self) -> String {
        let n = self
            .tab_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        format!("tab_{:08x}", n)
    }

    /// Get the tab map for reading.
    pub fn tabs(&self) -> &RwLock<HashMap<String, Tab>> {
        &self.tabs
    }

    /// Get or create a per-tab lock for sequential execution.
    pub fn tab_lock(&self, tab_id: &str) -> Arc<Mutex<()>> {
        self.tab_locks
            .entry(tab_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Acquire the cross-tab semaphore permit.
    pub fn tab_semaphore(&self) -> &Arc<Semaphore> {
        &self.tab_semaphore
    }

    /// Get the ref cache for a tab.
    pub async fn get_ref_cache(&self, tab_id: &str) -> Option<RefCache> {
        self.ref_caches.read().await.get(tab_id).cloned()
    }

    /// Set the ref cache for a tab.
    pub async fn set_ref_cache(&self, tab_id: &str, cache: RefCache) {
        self.ref_caches
            .write()
            .await
            .insert(tab_id.to_string(), cache);
    }

    /// Delete the ref cache for a tab.
    pub async fn delete_ref_cache(&self, tab_id: &str) {
        self.ref_caches.write().await.remove(tab_id);
    }

    /// Resolve a ref (e.g., "e5") to a backend DOM node ID using the ref cache.
    pub async fn resolve_ref(&self, tab_id: &str, ref_str: &str) -> Option<i64> {
        let caches = self.ref_caches.read().await;
        caches
            .get(tab_id)
            .and_then(|cache| cache.refs.get(ref_str).copied())
    }

    /// Get the number of per-tab locks (for testing cleanup).
    pub fn tab_lock_count(&self) -> usize {
        self.tab_locks.len()
    }

    /// Remove the per-tab lock for a closed tab.
    pub(crate) fn remove_tab_lock(&self, tab_id: &str) {
        self.tab_locks.remove(tab_id);
    }

    /// Open a new browser tab and return a [`TabHandle`] for lifecycle management.
    ///
    /// The caller owns the tab and is responsible for calling [`TabHandle::close`].
    pub async fn new_tab(self: &Arc<Self>, url: &str) -> CdpResult<TabHandle> {
        self.new_tab_with_created_target(url, |_| Ok(()))
            .await
            .map(|(handle, ())| handle)
    }

    /// Open a new browser tab and durably register its target before attaching.
    ///
    /// The callback runs immediately after `Target.createTarget` returns and
    /// before any attach/page setup. This lets process boundaries persist exact
    /// target ownership even when attach or cleanup later fails.
    pub async fn new_tab_with_created_target<F, T>(
        self: &Arc<Self>,
        url: &str,
        on_created: F,
    ) -> CdpResult<(TabHandle, T)>
    where
        F: FnOnce(&str) -> CdpResult<T>,
    {
        let nav_url = if url.is_empty() { "about:blank" } else { url };
        let (target_id, browser_context_id) = self.create_ephemeral_target(nav_url).await?;
        let closer = self.ephemeral_tab_closer(browser_context_id);
        let registration = match on_created(&target_id) {
            Ok(registration) => registration,
            Err(registration_error) => {
                return match closer.close_tab(&target_id).await {
                    Ok(()) => Err(registration_error),
                    Err(close_error) => Err(CdpError::Other(format!(
                        "register newly created target {target_id} failed: {registration_error}; exact-target cleanup also failed: {close_error}"
                    ))),
                };
            }
        };
        let session = match self.attach_session(&target_id).await {
            Ok(session) => session,
            Err(attach_error) => {
                return match closer.close_tab(&target_id).await {
                    Ok(()) => Err(attach_error),
                    Err(close_error) => Err(CdpError::Other(format!(
                        "attach to newly created target {target_id} failed: {attach_error}; exact-target cleanup also failed: {close_error}"
                    ))),
                };
            }
        };

        Ok((TabHandle::new(closer, session, target_id), registration))
    }

    /// Test-only constructor using injected fakes.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn new_for_test(
        browser_client: Arc<dyn CdpClient>,
        session_factory: Arc<dyn SessionFactory>,
    ) -> Arc<Self> {
        Self::new_for_test_with_config(browser_client, session_factory, BrowserConfig::default())
    }

    /// Test-only constructor using injected fakes and explicit configuration.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn new_for_test_with_config(
        browser_client: Arc<dyn CdpClient>,
        session_factory: Arc<dyn SessionFactory>,
        config: BrowserConfig,
    ) -> Arc<Self> {
        Arc::new(Self {
            browser_session: browser_client,
            session_factory,
            tabs: RwLock::new(HashMap::new()),
            ref_caches: RwLock::new(HashMap::new()),
            config,
            tab_semaphore: Arc::new(Semaphore::new(4)),
            tab_locks: dashmap::DashMap::new(),
            tab_counter: std::sync::atomic::AtomicU64::new(0),
        })
    }
}

/// Handle for an ephemeral browser tab with explicit lifecycle control.
///
/// Created by [`Browser::new_tab`]. The caller is responsible for closing
/// the tab via [`TabHandle::close`]. The configured closer may use HTTP with a
/// CDP fallback, but it must confirm ownership through the target ID.
pub struct TabHandle {
    closer: Arc<dyn TabCloser>,
    session: Arc<dyn CdpClient>,
    target_id: String,
    closed: AtomicBool,
}

impl TabHandle {
    /// Create a new `TabHandle`.
    ///
    /// - `closer`: strategy for closing the tab (CDP or HTTP)
    /// - `session`: tab-level CDP client for page operations
    /// - `target_id`: Chrome CDP target ID
    pub fn new(closer: Arc<dyn TabCloser>, session: Arc<dyn CdpClient>, target_id: String) -> Self {
        Self {
            closer,
            session,
            target_id,
            closed: AtomicBool::new(false),
        }
    }

    /// The Chrome CDP target ID for this tab.
    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    /// Create a [`Page`] for interacting with this tab.
    pub fn page(&self) -> Page {
        Page::with_tab(self.session.clone(), self.target_id.clone())
    }

    /// Clone the tab-scoped CDP client for lower-level domain operations.
    pub fn session(&self) -> Arc<dyn CdpClient> {
        self.session.clone()
    }

    /// Close the tab through the configured transport.
    ///
    /// Idempotent: calling close on an already-closed handle returns `Ok(())`.
    pub async fn close(&self) -> CdpResult<()> {
        if self.closed.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        self.closer.close_tab(&self.target_id).await
    }
}

impl Drop for TabHandle {
    fn drop(&mut self) {
        if !self.closed.load(Ordering::SeqCst) {
            tracing::warn!(
                target_id = %self.target_id,
                "TabHandle dropped without calling close() -- tab may be leaked"
            );
        }
    }
}

/// Fetch the WebSocket debugger URL from a browser's HTTP `/json/version` endpoint.
async fn discover_ws_url(http_url: &str, timeout: std::time::Duration) -> CdpResult<String> {
    let version_url = format!("{}/json/version", http_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|error| CdpError::HttpFailed(format!("build HTTP client: {error}")))?;
    let resp: serde_json::Value = client
        .get(&version_url)
        .send()
        .await
        .map_err(|e| CdpError::HttpFailed(format!("HTTP fetch failed: {e}")))?
        .json()
        .await
        .map_err(|e| CdpError::HttpFailed(format!("JSON parse failed: {e}")))?;

    resp.get("webSocketDebuggerUrl")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            CdpError::HttpFailed(
                "webSocketDebuggerUrl not found in /json/version response".to_string(),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{FakeSessionFactory, MockCdpClient};
    use std::sync::atomic::AtomicUsize;

    #[tokio::test]
    async fn test_tab_handle_close_calls_target_close() {
        let mock = Arc::new(MockCdpClient::new());
        let closer: Arc<dyn TabCloser> = Arc::new(CdpTabCloser::new(mock.clone()));
        let session = Arc::new(MockCdpClient::new());
        let handle = TabHandle::new(closer, session as Arc<dyn CdpClient>, "target-abc".into());

        handle.close().await.unwrap();

        let calls = mock.calls_for("Target.closeTarget");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].args[0], "target-abc");
    }

    #[tokio::test]
    async fn new_tab_closes_the_exact_created_target_when_attach_fails() -> CdpResult<()> {
        // Arrange
        let mock = Arc::new(MockCdpClient::new());
        mock.set_target_attach_error("injected attach failure");
        let browser = Browser::new_for_test(mock.clone(), Arc::new(FakeSessionFactory));
        let registered_targets = Arc::new(std::sync::Mutex::new(Vec::new()));
        let observed_targets = registered_targets.clone();

        // Act
        let result = browser
            .new_tab_with_created_target("about:blank", move |target_id| {
                observed_targets
                    .lock()
                    .map_err(|error| CdpError::Other(error.to_string()))?
                    .push(target_id.to_owned());
                Ok(())
            })
            .await;

        // Assert
        assert!(result.is_err());
        let registered_targets = registered_targets
            .lock()
            .map_err(|error| CdpError::Other(error.to_string()))?;
        assert_eq!(*registered_targets, vec!["target-1".to_owned()]);
        assert_eq!(mock.calls_for("Target.createTarget").len(), 1);
        let close_calls = mock.calls_for("Target.closeTarget");
        assert_eq!(close_calls.len(), 1);
        assert_eq!(close_calls[0].args[0], "target-1");
        Ok(())
    }

    #[tokio::test]
    async fn ephemeral_proxy_uses_an_isolated_context_and_disposes_it_on_close() -> CdpResult<()> {
        // Arrange
        let mock = Arc::new(MockCdpClient::new());
        let config = BrowserConfig {
            ephemeral_proxy_server: Some("socks5://proxy.test:1080".to_owned()),
            ephemeral_proxy_bypass_list: Some("<-loopback>".to_owned()),
            ..Default::default()
        };
        let browser =
            Browser::new_for_test_with_config(mock.clone(), Arc::new(FakeSessionFactory), config);

        // Act
        let handle = browser.new_tab("https://example.com/").await?;
        handle.close().await?;

        // Assert
        let context_calls = mock.calls_for("Target.createBrowserContext");
        assert_eq!(context_calls.len(), 1);
        assert_eq!(
            context_calls[0].args[0],
            serde_json::json!({
                "disposeOnDetach": true,
                "proxyServer": "socks5://proxy.test:1080",
                "proxyBypassList": "<-loopback>",
            })
        );
        let target_calls = mock.calls_for("Target.createTarget");
        assert_eq!(target_calls.len(), 1);
        assert_eq!(
            target_calls[0].args[0],
            serde_json::json!({
                "url": "https://example.com/",
                "browserContextId": "browser-context-1",
            })
        );
        assert_eq!(mock.calls_for("Target.closeTarget").len(), 1);
        let dispose_calls = mock.calls_for("Target.disposeBrowserContext");
        assert_eq!(dispose_calls.len(), 1);
        assert_eq!(dispose_calls[0].args[0], "browser-context-1");
        Ok(())
    }

    #[tokio::test]
    async fn test_tab_handle_close_idempotent() {
        let mock = Arc::new(MockCdpClient::new());
        let closer: Arc<dyn TabCloser> = Arc::new(CdpTabCloser::new(mock.clone()));
        let session = Arc::new(MockCdpClient::new());
        let handle = TabHandle::new(closer, session as Arc<dyn CdpClient>, "target-abc".into());

        handle.close().await.unwrap();
        handle.close().await.unwrap();

        let calls = mock.calls_for("Target.closeTarget");
        assert_eq!(calls.len(), 1);
    }

    #[test]
    fn test_tab_handle_target_id() {
        let closer: Arc<dyn TabCloser> =
            Arc::new(CdpTabCloser::new(Arc::new(MockCdpClient::new())));
        let session = Arc::new(MockCdpClient::new());
        let handle = TabHandle::new(closer, session as Arc<dyn CdpClient>, "target-xyz".into());

        assert_eq!(handle.target_id(), "target-xyz");
    }

    #[test]
    fn test_tab_handle_page_has_target_id() {
        let closer: Arc<dyn TabCloser> =
            Arc::new(CdpTabCloser::new(Arc::new(MockCdpClient::new())));
        let session = Arc::new(MockCdpClient::new());
        let handle = TabHandle::new(closer, session as Arc<dyn CdpClient>, "target-page".into());

        let page = handle.page();
        assert_eq!(page.target_id(), Some("target-page"));
    }

    #[tokio::test]
    async fn test_cdp_tab_closer_delegates_to_client() {
        let mock = Arc::new(MockCdpClient::new());
        let closer = CdpTabCloser::new(mock.clone() as Arc<dyn CdpClient>);

        closer.close_tab("t1").await.unwrap();

        let calls = mock.calls_for("Target.closeTarget");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].args[0], "t1");
    }

    struct TestTabCloser {
        calls: AtomicUsize,
        should_fail: bool,
    }

    #[async_trait]
    impl TabCloser for TestTabCloser {
        async fn close_tab(&self, _target_id: &str) -> CdpResult<()> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.should_fail {
                Err(CdpError::Other("injected close failure".to_owned()))
            } else {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn fallback_tab_closer_preserves_failure_when_closure_cannot_be_verified() {
        // Arrange
        let primary = Arc::new(TestTabCloser {
            calls: AtomicUsize::new(0),
            should_fail: true,
        });
        let fallback = Arc::new(TestTabCloser {
            calls: AtomicUsize::new(0),
            should_fail: false,
        });
        let closer = FallbackTabCloser::new(primary.clone(), fallback.clone());

        // Act
        let result = closer.close_tab("target-abc").await;

        // Assert
        assert!(result.is_err());
        assert_eq!(primary.calls.load(Ordering::SeqCst), 2);
        assert_eq!(fallback.calls.load(Ordering::SeqCst), 1);
    }

    struct FailOnceTabCloser {
        calls: AtomicUsize,
    }

    #[async_trait]
    impl TabCloser for FailOnceTabCloser {
        async fn close_tab(&self, _target_id: &str) -> CdpResult<()> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            if call == 0 {
                Err(CdpError::Other("injected first close failure".to_owned()))
            } else {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn fallback_tab_closer_verifies_target_after_secondary_transport() {
        // Arrange
        let primary = Arc::new(FailOnceTabCloser {
            calls: AtomicUsize::new(0),
        });
        let fallback = Arc::new(TestTabCloser {
            calls: AtomicUsize::new(0),
            should_fail: false,
        });
        let closer = FallbackTabCloser::new(primary.clone(), fallback.clone());

        // Act
        let result = closer.close_tab("target-abc").await;

        // Assert
        assert!(result.is_ok());
        assert_eq!(primary.calls.load(Ordering::SeqCst), 2);
        assert_eq!(fallback.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_rewrite_ws_url_basic() {
        let result = rewrite_ws_url(
            "http://proxy:9222",
            "ws://127.0.0.1:9222/devtools/browser/abc-123",
        )
        .unwrap();
        assert_eq!(result, "ws://proxy:9222/devtools/browser/abc-123");
    }

    #[test]
    fn test_rewrite_ws_url_different_ports() {
        let result = rewrite_ws_url(
            "http://remote-host:9333",
            "ws://127.0.0.1:9222/devtools/browser/def-456",
        )
        .unwrap();
        assert_eq!(result, "ws://remote-host:9333/devtools/browser/def-456");
    }

    #[test]
    fn test_rewrite_ws_url_preserves_path() {
        let result = rewrite_ws_url(
            "http://chrome.example.com:9000",
            "ws://localhost:9222/devtools/browser/guid-here",
        )
        .unwrap();
        assert_eq!(
            result,
            "ws://chrome.example.com:9000/devtools/browser/guid-here"
        );
    }

    #[test]
    fn test_rewrite_ws_url_invalid_http() {
        let result = rewrite_ws_url("not-a-url", "ws://127.0.0.1:9222/path");
        assert!(result.is_err());
    }

    #[test]
    fn test_rewrite_ws_url_invalid_ws() {
        let result = rewrite_ws_url("http://proxy:9222", "not-a-url");
        assert!(result.is_err());
    }
}
