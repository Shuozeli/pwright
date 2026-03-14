use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use pwright_cdp::connection::{CdpError, Result as CdpResult};
use pwright_cdp::{CdpConnection, CdpSession};
use tokio::sync::{Mutex, RwLock, Semaphore};
use tracing::info;

use crate::playwright::Page;

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
    /// Maximum number of open tabs (0 = unlimited)
    pub max_tabs: usize,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            cdp_url: String::new(),
            max_parallel_tabs: 4,
            navigate_timeout_ms: 30_000,
            max_tabs: 0,
        }
    }
}

/// High-level browser controller. Wraps a CDP connection and manages tabs.
pub struct Browser {
    connection: Arc<CdpConnection>,
    browser_session: CdpSession,
    tabs: RwLock<HashMap<String, Tab>>,
    ref_caches: RwLock<HashMap<String, RefCache>>,
    config: BrowserConfig,
    tab_semaphore: Arc<Semaphore>,
    tab_locks: dashmap::DashMap<String, Arc<Mutex<()>>>,
    tab_counter: std::sync::atomic::AtomicU64,
}

impl Browser {
    /// Connect to a running Chrome instance via CDP WebSocket.
    pub async fn connect(config: BrowserConfig) -> CdpResult<Arc<Self>> {
        info!(url = config.cdp_url, "connecting to Chrome");
        let connection = CdpConnection::connect(&config.cdp_url).await?;
        let browser_session = CdpSession::browser(connection.clone());

        let max_parallel = config.max_parallel_tabs.max(1);

        let browser = Arc::new(Self {
            connection,
            browser_session,
            tabs: RwLock::new(HashMap::new()),
            ref_caches: RwLock::new(HashMap::new()),
            config,
            tab_semaphore: Arc::new(Semaphore::new(max_parallel)),
            tab_locks: dashmap::DashMap::new(),
            tab_counter: std::sync::atomic::AtomicU64::new(0),
        });

        info!("connected to Chrome successfully");
        Ok(browser)
    }

    /// Access the browser-level CDP session.
    pub fn browser_session(&self) -> &CdpSession {
        &self.browser_session
    }

    /// Access the underlying connection.
    pub fn connection(&self) -> &Arc<CdpConnection> {
        &self.connection
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
                pwright_cdp::connection::CdpError::Other(format!("HTTP fetch failed: {e}"))
            })?
            .json()
            .await
            .map_err(|e| {
                pwright_cdp::connection::CdpError::Other(format!("JSON parse failed: {e}"))
            })?;

        let ws_url = resp
            .get("webSocketDebuggerUrl")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                pwright_cdp::connection::CdpError::Other(
                    "webSocketDebuggerUrl not found in /json/version response".to_string(),
                )
            })?;

        let rewritten = rewrite_ws_url(http_url, ws_url)?;

        let config = BrowserConfig {
            cdp_url: rewritten,
            ..BrowserConfig::default()
        };
        Self::connect(config).await
    }

    /// Create a new tab, run the closure with a `Page`, then close the tab.
    ///
    /// The tab is always closed, even if the closure returns an error.
    ///
    /// ```rust,ignore
    /// let result = browser.with_page(|page| async move {
    ///     page.goto("https://example.com", None).await?;
    ///     page.locator("h1").text_content().await
    /// }).await?;
    /// ```
    pub async fn with_page<F, Fut, T>(self: &Arc<Self>, f: F) -> CdpResult<T>
    where
        F: FnOnce(Page) -> Fut,
        Fut: Future<Output = CdpResult<T>>,
    {
        // Create a new tab
        let target_id = self.browser_session.target_create("about:blank").await?;
        let session_id = self.browser_session.target_attach(&target_id).await?;
        let session = CdpSession::new(self.connection.clone(), session_id, target_id.clone());
        let session = Arc::new(session);

        let page = Page::with_tab(session.clone(), target_id.clone());

        // Run the closure; always close the tab afterwards
        let result = f(page).await;

        // Close the tab (best effort — ignore errors)
        let _ = self.browser_session.target_close(&target_id).await;

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
