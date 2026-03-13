use std::collections::HashMap;
use std::sync::Arc;

use pwright_cdp::connection::Result as CdpResult;
use pwright_cdp::{CdpConnection, CdpSession};
use tokio::sync::{Mutex, RwLock, Semaphore};
use tracing::info;

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
}
