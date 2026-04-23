//! gRPC service — wires protobuf RPCs to the pwright-bridge.
//!
//! Split into handler submodules by RPC group:
//! - `lifecycle` — ConnectBrowser, Health
//! - `tabs` — CreateTab, CloseTab, ListTabs
//! - `navigation` — Navigate
//! - `content` — GetSnapshot, TakeScreenshot, GetText, GetPDF
//! - `actions` — ExecuteAction
//! - `data` — Evaluate, GetCookies, SetCookies

use std::sync::Arc;

use pwright_cdp::connection::CdpError;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};

use pwright_bridge::{Browser, BrowserConfig};

use crate::proto;
use crate::proto::browser_service_server::BrowserService;

/// Convert a CDP error to an appropriate gRPC status code.
fn cdp_to_status(e: CdpError) -> Status {
    match &e {
        CdpError::Timeout => Status::deadline_exceeded(e.to_string()),
        CdpError::Closed | CdpError::ChannelDropped => Status::unavailable(e.to_string()),
        CdpError::ElementNotFound { .. } => Status::not_found(e.to_string()),
        CdpError::NavigationFailed { .. } => Status::internal(e.to_string()),
        CdpError::WebSocket(_) => Status::unavailable(e.to_string()),
        CdpError::PageClosed => Status::failed_precondition(e.to_string()),
        CdpError::TabNotFound(_) => Status::not_found(e.to_string()),
        CdpError::HttpFailed(_) => Status::unavailable(e.to_string()),
        CdpError::JsException(_) => Status::internal(e.to_string()),
        CdpError::Protocol { .. }
        | CdpError::Json(_)
        | CdpError::Compound { .. }
        | CdpError::Other(_) => Status::internal(e.to_string()),
    }
}

/// Require a resolved node ID, returning INVALID_ARGUMENT if absent.
macro_rules! require_node_id {
    ($node_id:expr, $action:expr) => {
        $node_id.ok_or_else(|| Status::invalid_argument(format!("ref required for {}", $action)))?
    };
}

mod actions;
mod content;
mod conversions;
mod data;
mod lifecycle;
mod navigation;
mod tabs;

#[cfg(test)]
mod tests;

pub struct BrowserServiceImpl {
    pub(crate) browser: RwLock<Option<Arc<Browser>>>,
    pub(crate) default_cdp_url: Option<String>,
    pub(crate) max_parallel_tabs: usize,
    pub(crate) nav_timeout_ms: u64,
    pub(crate) eval_disabled: bool,
    /// Optional directory to restrict file uploads to. If set, `set_input_files`
    /// rejects paths outside this directory.
    pub(crate) upload_dir: Option<std::path::PathBuf>,
}

impl BrowserServiceImpl {
    pub fn new(
        cdp_url: Option<String>,
        max_parallel_tabs: usize,
        nav_timeout_ms: u64,
        eval_disabled: bool,
    ) -> Self {
        Self {
            browser: RwLock::new(None),
            default_cdp_url: cdp_url,
            max_parallel_tabs,
            nav_timeout_ms,
            eval_disabled,
            upload_dir: None,
        }
    }

    pub(crate) async fn get_browser(&self) -> Result<Arc<Browser>, Status> {
        let guard = self.browser.read().await;
        guard.clone().ok_or_else(|| {
            Status::failed_precondition("browser not connected — call ConnectBrowser first")
        })
    }

    pub(crate) async fn do_connect(&self, cdp_url: &str) -> Result<Arc<Browser>, Status> {
        let config = BrowserConfig {
            cdp_url: cdp_url.to_string(),
            max_parallel_tabs: self.max_parallel_tabs,
            navigate_timeout_ms: self.nav_timeout_ms,
            ..Default::default()
        };

        let browser = Browser::connect(config).await.map_err(cdp_to_status)?;

        let mut guard = self.browser.write().await;
        *guard = Some(browser.clone());

        Ok(browser)
    }

    /// Resolve a tab and acquire the per-tab lock + semaphore permit.
    /// Returns (tab, semaphore_permit, tab_lock_guard) ensuring exclusive access.
    ///
    /// **Locking order:** semaphore → per-tab mutex. All code paths that need
    /// both locks MUST acquire them in this order to prevent deadlocks.
    /// This is the only function that acquires both — do not duplicate this
    /// pattern elsewhere.
    pub(crate) async fn resolve_tab_locked(
        &self,
        browser: &Arc<Browser>,
        tab_id: &str,
    ) -> Result<
        (
            pwright_bridge::Tab,
            tokio::sync::OwnedSemaphorePermit,
            tokio::sync::OwnedMutexGuard<()>,
        ),
        Status,
    > {
        let tab = browser
            .resolve_tab(tab_id)
            .await
            .map_err(|e| Status::not_found(format!("tab: {}", e)))?;

        let permit = browser
            .tab_semaphore()
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| Status::resource_exhausted("tab semaphore closed"))?;

        let lock = browser.tab_lock(&tab.tab_id).lock_owned().await;

        Ok((tab, permit, lock))
    }

    pub(crate) async fn resolve_ref_or_node(
        &self,
        browser: &Browser,
        tab_id: &str,
        ref_str: &str,
    ) -> Result<i64, Status> {
        if ref_str.is_empty() {
            return Err(Status::invalid_argument("ref is required"));
        }
        browser.resolve_ref(tab_id, ref_str).await.ok_or_else(|| {
            Status::not_found(format!(
                "ref '{}' not found — take a snapshot first",
                ref_str
            ))
        })
    }
}

#[tonic::async_trait]
impl BrowserService for BrowserServiceImpl {
    async fn connect_browser(
        &self,
        request: Request<proto::ConnectBrowserRequest>,
    ) -> Result<Response<proto::ConnectBrowserResponse>, Status> {
        lifecycle::connect_browser(self, request).await
    }

    async fn health(
        &self,
        request: Request<proto::HealthRequest>,
    ) -> Result<Response<proto::HealthResponse>, Status> {
        lifecycle::health(self, request).await
    }

    async fn create_tab(
        &self,
        request: Request<proto::CreateTabRequest>,
    ) -> Result<Response<proto::CreateTabResponse>, Status> {
        tabs::create_tab(self, request).await
    }

    async fn close_tab(
        &self,
        request: Request<proto::CloseTabRequest>,
    ) -> Result<Response<proto::CloseTabResponse>, Status> {
        tabs::close_tab(self, request).await
    }

    async fn list_tabs(
        &self,
        request: Request<proto::ListTabsRequest>,
    ) -> Result<Response<proto::ListTabsResponse>, Status> {
        tabs::list_tabs(self, request).await
    }

    async fn bring_to_front(
        &self,
        request: Request<proto::BringToFrontRequest>,
    ) -> Result<Response<proto::BringToFrontResponse>, Status> {
        tabs::bring_to_front(self, request).await
    }

    async fn navigate(
        &self,
        request: Request<proto::NavigateRequest>,
    ) -> Result<Response<proto::NavigateResponse>, Status> {
        navigation::navigate(self, request).await
    }

    async fn reload(
        &self,
        request: Request<proto::ReloadRequest>,
    ) -> Result<Response<proto::ReloadResponse>, Status> {
        navigation::reload(self, request).await
    }

    async fn go_back(
        &self,
        request: Request<proto::GoBackRequest>,
    ) -> Result<Response<proto::GoBackResponse>, Status> {
        navigation::go_back(self, request).await
    }

    async fn go_forward(
        &self,
        request: Request<proto::GoForwardRequest>,
    ) -> Result<Response<proto::GoForwardResponse>, Status> {
        navigation::go_forward(self, request).await
    }

    async fn get_snapshot(
        &self,
        request: Request<proto::GetSnapshotRequest>,
    ) -> Result<Response<proto::GetSnapshotResponse>, Status> {
        content::get_snapshot(self, request).await
    }

    async fn take_screenshot(
        &self,
        request: Request<proto::TakeScreenshotRequest>,
    ) -> Result<Response<proto::TakeScreenshotResponse>, Status> {
        content::take_screenshot(self, request).await
    }

    async fn get_text(
        &self,
        request: Request<proto::GetTextRequest>,
    ) -> Result<Response<proto::GetTextResponse>, Status> {
        content::get_text(self, request).await
    }

    async fn get_pdf(
        &self,
        request: Request<proto::GetPdfRequest>,
    ) -> Result<Response<proto::GetPdfResponse>, Status> {
        content::get_pdf(self, request).await
    }

    async fn execute_action(
        &self,
        request: Request<proto::ExecuteActionRequest>,
    ) -> Result<Response<proto::ExecuteActionResponse>, Status> {
        actions::execute_action(self, request).await
    }

    async fn set_input_files(
        &self,
        request: Request<proto::SetInputFilesRequest>,
    ) -> Result<Response<proto::SetInputFilesResponse>, Status> {
        actions::set_input_files(self, request).await
    }

    async fn touch_tap(
        &self,
        request: Request<proto::TouchTapRequest>,
    ) -> Result<Response<proto::TouchTapResponse>, Status> {
        actions::touch_tap(self, request).await
    }

    async fn evaluate(
        &self,
        request: Request<proto::EvaluateRequest>,
    ) -> Result<Response<proto::EvaluateResponse>, Status> {
        data::evaluate(self, request).await
    }

    async fn get_cookies(
        &self,
        request: Request<proto::GetCookiesRequest>,
    ) -> Result<Response<proto::GetCookiesResponse>, Status> {
        data::get_cookies(self, request).await
    }

    async fn set_cookies(
        &self,
        request: Request<proto::SetCookiesRequest>,
    ) -> Result<Response<proto::SetCookiesResponse>, Status> {
        data::set_cookies(self, request).await
    }

    async fn expect_download(
        &self,
        request: Request<proto::ExpectDownloadRequest>,
    ) -> Result<Response<proto::ExpectDownloadResponse>, Status> {
        actions::expect_download(self, request).await
    }
}
