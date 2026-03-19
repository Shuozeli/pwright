//! Tab management handlers — CreateTab, CloseTab, ListTabs.

use tonic::{Request, Response, Status};

use super::{BrowserServiceImpl, cdp_to_status};
use crate::proto;

pub async fn create_tab(
    svc: &BrowserServiceImpl,
    request: Request<proto::CreateTabRequest>,
) -> Result<Response<proto::CreateTabResponse>, Status> {
    let browser = svc.get_browser().await?;
    let req = request.into_inner();

    let tab = browser.create_tab(&req.url).await.map_err(cdp_to_status)?;

    Ok(Response::new(proto::CreateTabResponse {
        tab_id: tab.tab_id,
        url: req.url,
        title: String::new(),
    }))
}

pub async fn close_tab(
    svc: &BrowserServiceImpl,
    request: Request<proto::CloseTabRequest>,
) -> Result<Response<proto::CloseTabResponse>, Status> {
    let browser = svc.get_browser().await?;
    let req = request.into_inner();

    browser
        .close_tab(&req.tab_id)
        .await
        .map_err(cdp_to_status)?;

    Ok(Response::new(proto::CloseTabResponse { closed: true }))
}

pub async fn list_tabs(
    svc: &BrowserServiceImpl,
    _request: Request<proto::ListTabsRequest>,
) -> Result<Response<proto::ListTabsResponse>, Status> {
    let browser = svc.get_browser().await?;

    let targets = browser.list_tabs().await.map_err(cdp_to_status)?;

    let tabs = targets.into_iter().map(proto::TabInfo::from).collect();

    Ok(Response::new(proto::ListTabsResponse { tabs }))
}

pub async fn bring_to_front(
    svc: &BrowserServiceImpl,
    request: Request<proto::BringToFrontRequest>,
) -> Result<Response<proto::BringToFrontResponse>, Status> {
    let browser = svc.get_browser().await?;
    let req = request.into_inner();

    let (tab, _permit, _lock) = svc.resolve_tab_locked(&browser, &req.tab_id).await?;

    tab.session
        .page_bring_to_front()
        .await
        .map_err(cdp_to_status)?;

    Ok(Response::new(proto::BringToFrontResponse { success: true }))
}
