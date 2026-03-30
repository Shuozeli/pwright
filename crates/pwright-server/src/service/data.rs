//! Data handlers — Evaluate, GetCookies, SetCookies.

use tonic::{Request, Response, Status};

use super::{BrowserServiceImpl, cdp_to_status};
use crate::proto;

pub async fn evaluate(
    svc: &BrowserServiceImpl,
    request: Request<proto::EvaluateRequest>,
) -> Result<Response<proto::EvaluateResponse>, Status> {
    if svc.eval_disabled {
        return Err(Status::permission_denied(
            "JavaScript evaluation is disabled via --disable-eval",
        ));
    }

    let browser = svc.get_browser().await?;
    let req = request.into_inner();

    let (tab, _permit, _lock) = svc.resolve_tab_locked(&browser, &req.tab_id).await?;

    let result = pwright_bridge::evaluate::evaluate_sync(&*tab.session, &req.expression)
        .await
        .map_err(cdp_to_status)?;

    Ok(Response::new(proto::EvaluateResponse {
        result: serde_json::to_string(&result).map_err(|e| {
            tracing::warn!("failed to serialize evaluate result: {e}");
            Status::internal(format!("serialize evaluate result: {e}"))
        })?,
    }))
}

pub async fn get_cookies(
    svc: &BrowserServiceImpl,
    request: Request<proto::GetCookiesRequest>,
) -> Result<Response<proto::GetCookiesResponse>, Status> {
    let browser = svc.get_browser().await?;
    let req = request.into_inner();

    let (tab, _permit, _lock) = svc.resolve_tab_locked(&browser, &req.tab_id).await?;

    let cookies = pwright_bridge::cookies::get_cookies(&*tab.session)
        .await
        .map_err(cdp_to_status)?;

    let entries = cookies.into_iter().map(proto::CookieEntry::from).collect();

    Ok(Response::new(proto::GetCookiesResponse {
        cookies: entries,
    }))
}

pub async fn set_cookies(
    svc: &BrowserServiceImpl,
    request: Request<proto::SetCookiesRequest>,
) -> Result<Response<proto::SetCookiesResponse>, Status> {
    let browser = svc.get_browser().await?;
    let req = request.into_inner();

    let (tab, _permit, _lock) = svc.resolve_tab_locked(&browser, &req.tab_id).await?;

    let cookies: Vec<pwright_cdp::domains::network::Cookie> = req
        .cookies
        .into_iter()
        .map(pwright_cdp::domains::network::Cookie::from)
        .collect();

    pwright_bridge::cookies::set_cookies(&*tab.session, &cookies)
        .await
        .map_err(cdp_to_status)?;

    Ok(Response::new(proto::SetCookiesResponse { success: true }))
}
