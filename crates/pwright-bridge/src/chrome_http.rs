//! Chrome HTTP debug endpoint client.
//!
//! Chrome exposes HTTP endpoints alongside the WebSocket CDP interface:
//! - `GET /json/list` — list all targets
//! - `GET /json/new?{url}` — create a new tab
//! - `GET /json/close/{targetId}` — close a tab
//! - `GET /json/version` — browser version info
//!
//! These are more reliable under Chrome memory pressure because they
//! bypass the WebSocket state machine.

use std::time::Duration;

use pwright_cdp::connection::{CdpError, Result as CdpResult};
use pwright_cdp::domains::target::TargetInfo;
use tracing::{debug, warn};

/// Default timeout for HTTP requests to Chrome debug endpoints.
const DEFAULT_HTTP_TIMEOUT: Duration = Duration::from_secs(10);

/// HTTP client for Chrome's debug endpoints.
///
/// Handles tab lifecycle operations (list, create, close) via HTTP,
/// which is more reliable than CDP WebSocket under Chrome memory pressure.
///
/// ```rust,ignore
/// let http = ChromeHttpClient::new("http://localhost:9222");
/// let targets = http.list_targets().await?;
/// http.close_target("TARGET_ID").await?;
/// ```
#[derive(Clone)]
pub struct ChromeHttpClient {
    base_url: String,
    client: reqwest::Client,
}

impl ChromeHttpClient {
    /// Create a new HTTP client for Chrome debug endpoints.
    ///
    /// `base_url` should be the Chrome HTTP endpoint, e.g. `http://localhost:9222`.
    pub fn new(base_url: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(DEFAULT_HTTP_TIMEOUT)
            .build()
            .expect("failed to build reqwest client");
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        }
    }

    /// Create a new HTTP client with a custom timeout.
    pub fn with_timeout(base_url: &str, timeout: Duration) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("failed to build reqwest client");
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        }
    }

    /// The base URL for this client.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// List all targets (tabs, service workers, etc.).
    pub async fn list_targets(&self) -> CdpResult<Vec<TargetInfo>> {
        let url = format!("{}/json/list", self.base_url);
        debug!(url, "listing targets via HTTP");
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| CdpError::Other(format!("HTTP list_targets failed: {e}")))?;

        let targets: Vec<TargetInfo> = resp
            .json()
            .await
            .map_err(|e| CdpError::Other(format!("HTTP list_targets JSON parse failed: {e}")))?;

        debug!(count = targets.len(), "listed targets via HTTP");
        Ok(targets)
    }

    /// Close a target by ID.
    ///
    /// Returns `Ok(())` if Chrome acknowledges the close.
    pub async fn close_target(&self, target_id: &str) -> CdpResult<()> {
        let url = format!("{}/json/close/{}", self.base_url, target_id);
        debug!(target_id, "closing target via HTTP");
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| CdpError::Other(format!("HTTP close_target failed: {e}")))?;

        if resp.status().is_success() {
            debug!(target_id, "closed target via HTTP");
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            warn!(target_id, %status, body, "HTTP close_target rejected");
            Err(CdpError::Other(format!(
                "HTTP close_target returned {status}: {body}"
            )))
        }
    }

    /// Create a new tab at the given URL.
    ///
    /// Returns the target info for the new tab.
    pub async fn create_target(&self, url: &str) -> CdpResult<TargetInfo> {
        let req_url = format!("{}/json/new?{}", self.base_url, url);
        debug!(url, "creating target via HTTP");
        let resp = self
            .client
            .get(&req_url)
            .send()
            .await
            .map_err(|e| CdpError::Other(format!("HTTP create_target failed: {e}")))?;

        let target: TargetInfo = resp
            .json()
            .await
            .map_err(|e| CdpError::Other(format!("HTTP create_target JSON parse failed: {e}")))?;

        debug!(target_id = target.target_id, "created target via HTTP");
        Ok(target)
    }

    /// Get Chrome version info.
    pub async fn version(&self) -> CdpResult<serde_json::Value> {
        let url = format!("{}/json/version", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| CdpError::Other(format!("HTTP version failed: {e}")))?;

        resp.json()
            .await
            .map_err(|e| CdpError::Other(format!("HTTP version JSON parse failed: {e}")))
    }
}
