//! Action handler — ExecuteAction.

use tonic::{Request, Response, Status};

use super::BrowserServiceImpl;
use crate::proto;

pub async fn execute_action(
    svc: &BrowserServiceImpl,
    request: Request<proto::ExecuteActionRequest>,
) -> Result<Response<proto::ExecuteActionResponse>, Status> {
    let browser = svc.get_browser().await?;
    let req = request.into_inner();

    let (tab, _permit, _lock) = svc.resolve_tab_locked(&browser, &req.tab_id).await?;

    let session: &dyn pwright_bridge::CdpClient = &*tab.session;

    // Resolve ref to node ID if provided
    let node_id = if !req.r#ref.is_empty() {
        Some(
            svc.resolve_ref_or_node(&browser, &tab.tab_id, &req.r#ref)
                .await?,
        )
    } else {
        None
    };

    match req.kind() {
        proto::ActionKind::Click => {
            let nid = node_id.ok_or_else(|| Status::invalid_argument("ref required for click"))?;
            pwright_bridge::actions::click_by_node_id(session, nid)
                .await
                .map_err(|e| Status::internal(format!("click: {}", e)))?;
        }
        proto::ActionKind::Type => {
            let nid = node_id.ok_or_else(|| Status::invalid_argument("ref required for type"))?;
            pwright_bridge::actions::type_by_node_id(session, nid, &req.text)
                .await
                .map_err(|e| Status::internal(format!("type: {}", e)))?;
        }
        proto::ActionKind::Fill => {
            let nid = node_id.ok_or_else(|| Status::invalid_argument("ref required for fill"))?;
            pwright_bridge::actions::fill_by_node_id(session, nid, &req.text)
                .await
                .map_err(|e| Status::internal(format!("fill: {}", e)))?;
        }
        proto::ActionKind::Press => {
            pwright_bridge::actions::press_key(session, &req.key)
                .await
                .map_err(|e| Status::internal(format!("press: {}", e)))?;
        }
        proto::ActionKind::Focus => {
            let nid = node_id.ok_or_else(|| Status::invalid_argument("ref required for focus"))?;
            session
                .dom_focus(nid)
                .await
                .map_err(|e| Status::internal(format!("focus: {}", e)))?;
        }
        proto::ActionKind::Hover => {
            let nid = node_id.ok_or_else(|| Status::invalid_argument("ref required for hover"))?;
            pwright_bridge::actions::hover_by_node_id(session, nid)
                .await
                .map_err(|e| Status::internal(format!("hover: {}", e)))?;
        }
        proto::ActionKind::Select => {
            let nid = node_id.ok_or_else(|| Status::invalid_argument("ref required for select"))?;
            let val = if req.value.is_empty() {
                &req.text
            } else {
                &req.value
            };
            pwright_bridge::actions::select_by_node_id(session, nid, val)
                .await
                .map_err(|e| Status::internal(format!("select: {}", e)))?;
        }
        proto::ActionKind::Scroll => {
            if let Some(nid) = node_id {
                pwright_bridge::actions::scroll_by_node_id(session, nid)
                    .await
                    .map_err(|e| Status::internal(format!("scroll: {}", e)))?;
            } else {
                let dx = if req.scroll_x != 0 { req.scroll_x } else { 0 };
                let dy = if req.scroll_y != 0 { req.scroll_y } else { 800 };
                pwright_bridge::actions::scroll_page(session, dx, dy)
                    .await
                    .map_err(|e| Status::internal(format!("scroll: {}", e)))?;
            }
        }
        proto::ActionKind::Drag => {
            let nid = node_id.ok_or_else(|| Status::invalid_argument("ref required for drag"))?;
            pwright_bridge::actions::drag_by_node_id(session, nid, req.drag_x, req.drag_y)
                .await
                .map_err(|e| Status::internal(format!("drag: {}", e)))?;
        }
        proto::ActionKind::Check => {
            let nid = node_id.ok_or_else(|| Status::invalid_argument("ref required for check"))?;
            pwright_bridge::actions::click_by_node_id(session, nid)
                .await
                .map_err(|e| Status::internal(format!("check: {}", e)))?;
        }
        proto::ActionKind::Uncheck => {
            let nid =
                node_id.ok_or_else(|| Status::invalid_argument("ref required for uncheck"))?;
            pwright_bridge::actions::click_by_node_id(session, nid)
                .await
                .map_err(|e| Status::internal(format!("uncheck: {}", e)))?;
        }
        proto::ActionKind::Dblclick => {
            let nid =
                node_id.ok_or_else(|| Status::invalid_argument("ref required for dblclick"))?;
            pwright_bridge::actions::dblclick_by_node_id(session, nid)
                .await
                .map_err(|e| Status::internal(format!("dblclick: {}", e)))?;
        }
        proto::ActionKind::ActionUnspecified => {
            return Err(Status::invalid_argument("action kind required"));
        }
    }

    Ok(Response::new(proto::ExecuteActionResponse {
        success: true,
        message: String::new(),
    }))
}

pub async fn set_input_files(
    svc: &BrowserServiceImpl,
    request: Request<proto::SetInputFilesRequest>,
) -> Result<Response<proto::SetInputFilesResponse>, Status> {
    let browser = svc.get_browser().await?;
    let req = request.into_inner();

    let (tab, _permit, _lock) = svc.resolve_tab_locked(&browser, &req.tab_id).await?;

    let session: &dyn pwright_bridge::CdpClient = &*tab.session;

    let node_id = if !req.r#ref.is_empty() {
        svc.resolve_ref_or_node(&browser, &tab.tab_id, &req.r#ref)
            .await?
    } else if !req.selector.is_empty() {
        let doc = session
            .dom_get_document()
            .await
            .map_err(|e| Status::internal(format!("dom: {}", e)))?;
        let root_id = doc
            .get("root")
            .and_then(|r| r.get("nodeId"))
            .and_then(|n| n.as_i64())
            .unwrap_or(1);
        session
            .dom_query_selector(root_id, &req.selector)
            .await
            .map_err(|e| Status::internal(format!("query: {}", e)))?
    } else {
        return Err(Status::invalid_argument("ref or selector required"));
    };

    // Validate file paths to prevent directory traversal.
    // If upload_dir is configured, restrict to that directory.
    if let Some(ref allowed_dir) = svc.upload_dir {
        for path_str in &req.files {
            let path = std::path::Path::new(path_str);
            let canonical = std::fs::canonicalize(path)
                .map_err(|_| Status::invalid_argument(format!("file not found: {path_str}")))?;
            if !canonical.starts_with(allowed_dir) {
                return Err(Status::permission_denied(format!(
                    "file path outside allowed directory: {path_str}"
                )));
            }
        }
    }

    session
        .dom_set_file_input_files(node_id, &req.files)
        .await
        .map_err(|e| Status::internal(format!("set_input_files: {}", e)))?;

    Ok(Response::new(proto::SetInputFilesResponse {
        success: true,
    }))
}

pub async fn touch_tap(
    svc: &BrowserServiceImpl,
    request: Request<proto::TouchTapRequest>,
) -> Result<Response<proto::TouchTapResponse>, Status> {
    let browser = svc.get_browser().await?;
    let req = request.into_inner();

    let (tab, _permit, _lock) = svc.resolve_tab_locked(&browser, &req.tab_id).await?;

    tab.session
        .input_dispatch_touch_event("touchStart", req.x, req.y)
        .await
        .map_err(|e| Status::internal(format!("touch start: {}", e)))?;
    tab.session
        .input_dispatch_touch_event("touchEnd", req.x, req.y)
        .await
        .map_err(|e| Status::internal(format!("touch end: {}", e)))?;

    Ok(Response::new(proto::TouchTapResponse { success: true }))
}

pub async fn expect_download(
    svc: &BrowserServiceImpl,
    request: Request<proto::ExpectDownloadRequest>,
) -> Result<Response<proto::ExpectDownloadResponse>, Status> {
    let browser = svc.get_browser().await?;
    let req = request.into_inner();

    let (tab, _permit, _lock) = svc.resolve_tab_locked(&browser, &req.tab_id).await?;

    let session: &dyn pwright_bridge::CdpClient = &*tab.session;
    let page = pwright_bridge::playwright::Page::new(tab.session.clone());

    let action_req = req
        .action
        .ok_or_else(|| Status::invalid_argument("action is required to trigger download"))?;

    // We reuse the execute_action logic, but executed inside page.expect_download
    let result = page
        .expect_download(|| async {
            // Resolve ref to node ID if provided for the sub-action
            let node_id = if !action_req.r#ref.is_empty() {
                Some(
                    svc.resolve_ref_or_node(&browser, &tab.tab_id, &action_req.r#ref)
                        .await
                        .map_err(|e| pwright_cdp::connection::CdpError::Other(e.to_string()))?,
                )
            } else {
                None
            };

            match action_req.kind() {
                proto::ActionKind::Click => {
                    let nid = node_id.ok_or_else(|| {
                        pwright_cdp::connection::CdpError::Other(
                            "ref required for click".to_string(),
                        )
                    })?;
                    pwright_bridge::actions::click_by_node_id(session, nid).await?;
                }
                proto::ActionKind::Press => {
                    pwright_bridge::actions::press_key(session, &action_req.key).await?;
                }
                _ => {
                    return Err(pwright_cdp::connection::CdpError::Other(
                        "Only Click and Press actions are supported for triggering downloads"
                            .to_string(),
                    ));
                }
            }
            Ok(())
        })
        .await
        .map_err(|e| Status::internal(format!("expect_download error: {}", e)))?;

    Ok(Response::new(proto::ExpectDownloadResponse {
        file_path: result,
        success: true,
        message: String::new(),
    }))
}
