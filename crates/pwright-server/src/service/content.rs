//! Content extraction handlers — GetSnapshot, TakeScreenshot, GetText, GetPDF.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use tonic::{Request, Response, Status};

use pwright_bridge::snapshot::{RefCache, SnapshotFilter};

use super::{BrowserServiceImpl, cdp_to_status};
use crate::proto;

pub async fn get_snapshot(
    svc: &BrowserServiceImpl,
    request: Request<proto::GetSnapshotRequest>,
) -> Result<Response<proto::GetSnapshotResponse>, Status> {
    let browser = svc.get_browser().await?;
    let req = request.into_inner();

    let (tab, _permit, _lock) = svc.resolve_tab_locked(&browser, &req.tab_id).await?;

    let filter = match req.filter() {
        proto::SnapshotFilter::FilterAll => SnapshotFilter::All,
        proto::SnapshotFilter::FilterInteractive => SnapshotFilter::Interactive,
    };

    let (nodes, refs) =
        pwright_bridge::snapshot::get_snapshot(&*tab.session, &filter, req.max_depth)
            .await
            .map_err(cdp_to_status)?;

    let cache = RefCache {
        refs,
        nodes: nodes.clone(),
    };
    browser.set_ref_cache(&tab.tab_id, cache).await;

    let proto_nodes = nodes
        .into_iter()
        .map(|n| proto::A11yNode {
            r#ref: n.ref_id,
            role: n.role,
            name: n.name,
            depth: n.depth,
            value: n.value,
            disabled: n.disabled,
            focused: n.focused,
            node_id: n.node_id,
        })
        .collect();

    Ok(Response::new(proto::GetSnapshotResponse {
        tab_id: tab.tab_id,
        nodes: proto_nodes,
    }))
}

pub async fn take_screenshot(
    svc: &BrowserServiceImpl,
    request: Request<proto::TakeScreenshotRequest>,
) -> Result<Response<proto::TakeScreenshotResponse>, Status> {
    let browser = svc.get_browser().await?;
    let req = request.into_inner();

    let (tab, _permit, _lock) = svc.resolve_tab_locked(&browser, &req.tab_id).await?;

    let format = match req.format.as_str() {
        "jpeg" | "jpg" => pwright_bridge::content::ScreenshotFormat::Jpeg(req.quality),
        "webp" => pwright_bridge::content::ScreenshotFormat::Webp(req.quality),
        _ => pwright_bridge::content::ScreenshotFormat::Png,
    };

    let b64_data = pwright_bridge::content::take_screenshot(&*tab.session, &format, req.full_page)
        .await
        .map_err(cdp_to_status)?;

    let data = B64
        .decode(&b64_data)
        .map_err(|e| Status::internal(format!("decode screenshot: {}", e)))?;

    Ok(Response::new(proto::TakeScreenshotResponse {
        data,
        format: req.format,
    }))
}

pub async fn get_text(
    svc: &BrowserServiceImpl,
    request: Request<proto::GetTextRequest>,
) -> Result<Response<proto::GetTextResponse>, Status> {
    let browser = svc.get_browser().await?;
    let req = request.into_inner();

    let (tab, _permit, _lock) = svc.resolve_tab_locked(&browser, &req.tab_id).await?;

    let text = pwright_bridge::content::get_text(&*tab.session)
        .await
        .map_err(cdp_to_status)?;

    Ok(Response::new(proto::GetTextResponse {
        tab_id: tab.tab_id,
        text,
    }))
}

pub async fn get_pdf(
    svc: &BrowserServiceImpl,
    request: Request<proto::GetPdfRequest>,
) -> Result<Response<proto::GetPdfResponse>, Status> {
    let browser = svc.get_browser().await?;
    let req = request.into_inner();

    let (tab, _permit, _lock) = svc.resolve_tab_locked(&browser, &req.tab_id).await?;

    let b64_data = pwright_bridge::content::get_pdf(&*tab.session)
        .await
        .map_err(cdp_to_status)?;

    let data = B64
        .decode(&b64_data)
        .map_err(|e| Status::internal(format!("decode pdf: {}", e)))?;

    Ok(Response::new(proto::GetPdfResponse { data }))
}
