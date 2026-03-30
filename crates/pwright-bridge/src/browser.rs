use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use pwright_cdp::connection::{CdpError, Result as CdpResult};
use pwright_cdp::{CdpClient, CdpConnection, CdpSession, CdpSessionFactory, SessionFactory};
use tokio::sync::{Mutex, RwLock, Semaphore};
use tracing::info;

use crate::chrome_http::ChromeHttpClient;
use crate::playwright::Page;

/// Strategy for closing a tab. Implementations choose the transport
/// (CDP WebSocket or Chrome HTTP debug endpoint).
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

/// Close a tab via Chrome's HTTP debug endpoint (`GET /json/close/{targetId}`).
pub struct HttpTabCloser {
    http_client: ChromeHttpClient,
}

impl HttpTabCloser {
    pub fn new(http_client: ChromeHttpClient) -> Self {
        Self { http_client }
    }
}

#[async_trait]
impl TabCloser for HttpTabCloser {
    async fn close_tab(&self, target_id: &str) -> CdpResult<()> {
        self.http_client.close_target(target_id).await
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

/// Configuration for connecting to Chrome.
#[derive(Debug, Clone)]
pub struct BrowserConfig {
    /// WebSocket URL for Chrome DevTools (e.g. ws://127.0.0.1:9222/devtools/browser/...)
    pub cdp_url: String,
    /// Maximum number of concurrent tab operations
    pub max_parallel_tabs: usize,
    /// Default navigation timeout in milliseconds
    pub navigate_timeout_ms: u64,
    // TODO(feature): max_tabs enforcement was removed because it was never checked.
    // If tab limits are needed in the future, implement them in the server layer
    // (pwright-bridge is stateless by design).
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            cdp_url: String::new(),
            max_parallel_tabs: 4,
            navigate_timeout_ms: 30_000,
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
    http_url: Option<String>,
    tab_semaphore: Arc<Semaphore>,
    tab_locks: dashmap::DashMap<String, Arc<Mutex<()>>>,
    tab_counter: std::sync::atomic::AtomicU64,
}

impl Browser {
    /// Connect to a running Chrome instance via CDP WebSocket.
    pub async fn connect(config: BrowserConfig) -> CdpResult<Arc<Self>> {
        Self::connect_inner(config, None).await
    }

    fn build(
        connection: Arc<CdpConnection>,
        config: BrowserConfig,
        http_url: Option<String>,
    ) -> Self {
        let browser_session: Arc<dyn CdpClient> = Arc::new(CdpSession::browser(connection.clone()));
        let session_factory: Arc<dyn SessionFactory> = Arc::new(CdpSessionFactory::new(connection));
        let max_parallel = config.max_parallel_tabs.max(1);
        Self {
            browser_session,
            session_factory,
            tabs: RwLock::new(HashMap::new()),
            ref_caches: RwLock::new(HashMap::new()),
            config,
            http_url,
            tab_semaphore: Arc::new(Semaphore::new(max_parallel)),
            tab_locks: dashmap::DashMap::new(),
            tab_counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    async fn connect_inner(
        config: BrowserConfig,
        http_url: Option<String>,
    ) -> CdpResult<Arc<Self>> {
        info!(url = config.cdp_url, "connecting to Chrome");
        let connection = CdpConnection::connect(&config.cdp_url).await?;
        let browser = Arc::new(Self::build(connection, config, http_url));
        info!("connected to Chrome successfully");
        Ok(browser)
    }

    /// Access the browser-level CDP client (for target management).
    pub(crate) fn browser_client(&self) -> &dyn CdpClient {
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

    /// Connect to Chrome via an HTTP endpoint (e.g. `http://proxy:9222`).
    ///
    /// Fetches `/json/version` to discover the WebSocket debugger URL,
    /// then rewrites its host/port to match the HTTP URL (for proxied setups).
    pub async fn connect_http(http_url: &str) -> CdpResult<Arc<Self>> {
        let version_url = format!("{}/json/version", http_url.trim_end_matches('/'));
        let resp: serde_json::Value = reqwest::get(&version_url)
            .await
            .map_err(|e| {
                pwright_cdp::connection::CdpError::HttpFailed(format!("HTTP fetch failed: {e}"))
            })?
            .json()
            .await
            .map_err(|e| {
                pwright_cdp::connection::CdpError::HttpFailed(format!("JSON parse failed: {e}"))
            })?;

        let ws_url = resp
            .get("webSocketDebuggerUrl")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                pwright_cdp::connection::CdpError::HttpFailed(
                    "webSocketDebuggerUrl not found in /json/version response".to_string(),
                )
            })?;

        let rewritten = rewrite_ws_url(http_url, ws_url)?;

        let config = BrowserConfig {
            cdp_url: rewritten,
            ..BrowserConfig::default()
        };
        Self::connect_inner(config, Some(http_url.trim_end_matches('/').to_string())).await
    }

    /// The Chrome HTTP debug URL, if connected via [`Browser::connect_http`].
    pub fn http_url(&self) -> Option<&str> {
        self.http_url.as_deref()
    }

    /// Get a [`ChromeHttpClient`] for HTTP-based tab management.
    ///
    /// Returns `None` if the browser was connected via WebSocket directly
    /// (no HTTP URL available).
    pub fn http_client(&self) -> Option<CdpResult<ChromeHttpClient>> {
        self.http_url.as_ref().map(|url| ChromeHttpClient::new(url))
    }

    /// Open a new browser tab and return a [`TabHandle`] for lifecycle management.
    ///
    /// The caller owns the tab and is responsible for calling [`TabHandle::close`].
    pub async fn new_tab(self: &Arc<Self>, url: &str) -> CdpResult<TabHandle> {
        let nav_url = if url.is_empty() { "about:blank" } else { url };
        let target_id = self.browser_client().target_create(nav_url).await?;
        let session = self.attach_session(&target_id).await?;

        let closer: Arc<dyn TabCloser> = match &self.http_url {
            Some(url) => Arc::new(HttpTabCloser::new(ChromeHttpClient::new(url)?)),
            None => Arc::new(CdpTabCloser::new(self.browser_session.clone())),
        };

        Ok(TabHandle::new(closer, session, target_id))
    }

    /// Test-only constructor using injected fakes.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn new_for_test(
        browser_client: Arc<dyn CdpClient>,
        session_factory: Arc<dyn SessionFactory>,
    ) -> Arc<Self> {
        Arc::new(Self {
            browser_session: browser_client,
            session_factory,
            tabs: RwLock::new(HashMap::new()),
            ref_caches: RwLock::new(HashMap::new()),
            config: BrowserConfig::default(),
            http_url: None,
            tab_semaphore: Arc::new(Semaphore::new(4)),
            tab_locks: dashmap::DashMap::new(),
            tab_counter: std::sync::atomic::AtomicU64::new(0),
        })
    }
}

/// Handle for an ephemeral browser tab with explicit lifecycle control.
///
/// Created by [`Browser::new_tab`]. The caller is responsible for closing
/// the tab via [`TabHandle::close`]. If the browser was connected via HTTP,
/// close uses the HTTP endpoint; otherwise it uses CDP WebSocket.
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

    /// Close the tab. Uses HTTP or CDP depending on how the browser was connected.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockCdpClient;

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
