//! Navigation handler — Navigate.

use tonic::{Request, Response, Status};

use super::BrowserServiceImpl;
use crate::proto;

pub async fn navigate(
    svc: &BrowserServiceImpl,
    request: Request<proto::NavigateRequest>,
) -> Result<Response<proto::NavigateResponse>, Status> {
    let browser = svc.get_browser().await?;
    let req = request.into_inner();

    // For new tabs, no lock needed (no contention). For existing tabs, acquire lock.
    let (_permit, _lock);
    let tab = if req.new_tab || req.tab_id.is_empty() {
        let url = if req.url.is_empty() {
            "about:blank"
        } else {
            &req.url
        };
        let t = browser
            .create_tab(url)
            .await
            .map_err(|e| Status::internal(format!("create tab: {}", e)))?;
        _permit = browser
            .tab_semaphore()
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| Status::resource_exhausted("semaphore closed"))?;
        _lock = browser.tab_lock(&t.tab_id).lock_owned().await;
        t
    } else {
        let (t, p, l) = svc.resolve_tab_locked(&browser, &req.tab_id).await?;
        _permit = p;
        _lock = l;
        t
    };

    let wait_for = match req.wait_for() {
        proto::WaitStrategy::WaitNone => pwright_bridge::navigate::WaitStrategy::None,
        proto::WaitStrategy::WaitDom => pwright_bridge::navigate::WaitStrategy::Dom,
        proto::WaitStrategy::WaitNetworkIdle => pwright_bridge::navigate::WaitStrategy::NetworkIdle,
        proto::WaitStrategy::WaitSelector => {
            pwright_bridge::navigate::WaitStrategy::Selector(req.wait_selector.clone())
        }
    };

    let timeout_ms = if req.timeout_ms > 0.0 {
        req.timeout_ms as u64
    } else {
        svc.nav_timeout_ms
    };

    let opts = pwright_bridge::navigate::NavigateOptions {
        wait_for,
        timeout: std::time::Duration::from_millis(timeout_ms),
        block_images: req.block_images,
        block_media: req.block_media,
    };

    let result = pwright_bridge::navigate::navigate(&*tab.session, &tab.tab_id, &req.url, &opts)
        .await
        .map_err(|e| Status::internal(format!("navigate: {}", e)))?;

    Ok(Response::new(proto::NavigateResponse {
        tab_id: result.tab_id,
        url: result.url,
        title: result.title,
    }))
}

pub async fn reload(
    svc: &BrowserServiceImpl,
    request: Request<proto::ReloadRequest>,
) -> Result<Response<proto::ReloadResponse>, Status> {
    let browser = svc.get_browser().await?;
    let req = request.into_inner();

    let (tab, _permit, _lock) = svc.resolve_tab_locked(&browser, &req.tab_id).await?;

    tab.session
        .page_reload()
        .await
        .map_err(|e| Status::internal(format!("reload: {}", e)))?;

    // Wait for page to be interactive (reusing bridge layer helper)
    let _ = pwright_bridge::navigate::poll_ready_state(
        tab.session.as_ref(),
        std::time::Duration::from_secs(10),
    )
    .await;

    Ok(Response::new(proto::ReloadResponse { success: true }))
}

pub async fn go_back(
    svc: &BrowserServiceImpl,
    request: Request<proto::GoBackRequest>,
) -> Result<Response<proto::GoBackResponse>, Status> {
    let browser = svc.get_browser().await?;
    let req = request.into_inner();

    let (tab, _permit, _lock) = svc.resolve_tab_locked(&browser, &req.tab_id).await?;

    pwright_bridge::evaluate::evaluate(tab.session.as_ref(), "history.back()")
        .await
        .map_err(|e| Status::internal(format!("go_back: {}", e)))?;

    Ok(Response::new(proto::GoBackResponse { success: true }))
}

pub async fn go_forward(
    svc: &BrowserServiceImpl,
    request: Request<proto::GoForwardRequest>,
) -> Result<Response<proto::GoForwardResponse>, Status> {
    let browser = svc.get_browser().await?;
    let req = request.into_inner();

    let (tab, _permit, _lock) = svc.resolve_tab_locked(&browser, &req.tab_id).await?;

    pwright_bridge::evaluate::evaluate(tab.session.as_ref(), "history.forward()")
        .await
        .map_err(|e| Status::internal(format!("go_forward: {}", e)))?;

    Ok(Response::new(proto::GoForwardResponse { success: true }))
}
