//! Browser lifecycle handlers — ConnectBrowser, Health.

use tonic::{Request, Response, Status};
use tracing::{debug, info};

use super::BrowserServiceImpl;
use crate::proto;

pub async fn connect_browser(
    svc: &BrowserServiceImpl,
    request: Request<proto::ConnectBrowserRequest>,
) -> Result<Response<proto::ConnectBrowserResponse>, Status> {
    let req = request.into_inner();
    let url = if req.cdp_url.is_empty() {
        svc.default_cdp_url
            .as_deref()
            .ok_or_else(|| Status::invalid_argument("cdp_url required"))?
            .to_string()
    } else {
        req.cdp_url
    };

    if !pwright_bridge::is_supported_scheme(&url) {
        return Err(Status::invalid_argument(format!(
            "cdp_url must use a supported scheme: {}",
            pwright_bridge::SUPPORTED_SCHEMES.join(", ")
        )));
    }

    debug!(url = url, "connecting to Chrome");
    info!("connecting to Chrome CDP endpoint");
    svc.do_connect(&url).await?;

    Ok(Response::new(proto::ConnectBrowserResponse {
        connected: true,
        message: "connected".to_string(),
    }))
}

pub async fn health(
    svc: &BrowserServiceImpl,
    _request: Request<proto::HealthRequest>,
) -> Result<Response<proto::HealthResponse>, Status> {
    let guard = svc.browser.read().await;
    let (connected, tab_count) = if let Some(browser) = guard.as_ref() {
        let tabs = browser.tabs().read().await;
        (true, i32::try_from(tabs.len()).unwrap_or(i32::MAX))
    } else {
        (false, 0)
    };

    Ok(Response::new(proto::HealthResponse {
        healthy: connected,
        browser_connected: connected,
        open_tabs: tab_count,
    }))
}
