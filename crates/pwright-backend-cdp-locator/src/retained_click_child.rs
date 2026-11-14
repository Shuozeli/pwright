//! One-link, same-session Frame geometry for opt-in retained Element clicks.
//! A deeper or OOPIF route is deliberately not inferred from local CSS pixels.

use pwright_cap_locator::LocatorErrorDetails;
use pwright_cdp::{CdpClient, generated::page::FrameTree};
use pwright_core::OperationError;
use serde_json::Value;

use crate::{ClickPoint, invalid_response, map_cdp_error, retained_click_target_error};

#[derive(Clone, Copy)]
pub(super) struct ChildFrame<'a> {
    pub frame_id: &'a str,
    pub parent_frame_id: &'a str,
}

#[derive(Clone, Copy)]
pub(super) struct Viewport {
    width: f64,
    height: f64,
}

impl Viewport {
    pub(super) fn from_actionability(
        response: &Value,
    ) -> Result<Self, OperationError<LocatorErrorDetails>> {
        let value = response
            .get("result")
            .and_then(|result| result.get("value"))
            .ok_or_else(|| invalid_response("retained child click is missing actionability"))?;
        let dimension = |name| {
            value
                .get(name)
                .and_then(Value::as_f64)
                .filter(|number| number.is_finite() && *number > 0.0)
                .ok_or_else(|| invalid_response("retained child click has invalid viewport size"))
        };
        Ok(Self {
            width: dimension("viewportWidth")?,
            height: dimension("viewportHeight")?,
        })
    }
}

pub(super) async fn verify_one_link_route(
    client: &dyn CdpClient,
    child: ChildFrame<'_>,
) -> Result<(), OperationError<LocatorErrorDetails>> {
    let tree = client.page_get_frame_tree().await.map_err(map_cdp_error)?;
    if tree.frame.parent_id.is_some()
        || tree.frame.id != child.parent_frame_id
        || !tree.child_frames.as_deref().is_some_and(|children| {
            children.iter().any(|entry: &FrameTree| {
                entry.frame.id == child.frame_id
                    && entry.frame.parent_id.as_deref() == Some(child.parent_frame_id)
            })
        })
    {
        return Err(retained_click_target_error(
            "retained Element click child Frame route was replaced or is not one local parent link",
        ));
    }
    Ok(())
}

pub(super) async fn translate(
    client: &dyn CdpClient,
    child: ChildFrame<'_>,
    point: ClickPoint,
    viewport: Viewport,
) -> Result<ClickPoint, OperationError<LocatorErrorDetails>> {
    verify_one_link_route(client, child).await?;
    let owner = client
        .dom_get_frame_owner(child.frame_id)
        .await
        .map_err(map_cdp_error)?;
    let box_model = client
        .dom_get_box_model_by_backend_node(owner)
        .await
        .map_err(map_cdp_error)?;
    let quad = box_model
        .get("model")
        .and_then(|model| model.get("content"))
        .and_then(Value::as_array)
        .filter(|quad| quad.len() == 8)
        .ok_or_else(|| invalid_response("retained child click is missing iframe content quad"))?;
    let mut corners = [0.0_f64; 8];
    for (index, coordinate) in quad.iter().enumerate() {
        corners[index] = coordinate
            .as_f64()
            .filter(|number| number.is_finite())
            .ok_or_else(|| invalid_response("retained child click has invalid content quad"))?;
    }
    let [x0, y0, x1, y1, x2, y2, x3, y3] = corners;
    let horizontal = (x1 - x0, y1 - y0);
    let vertical = (x3 - x0, y3 - y0);
    let area = horizontal.0 * vertical.1 - horizontal.1 * vertical.0;
    if !area.is_finite()
        || area.abs() < 1.0
        || (x1 + x3 - x0 - x2).abs() > 0.25
        || (y1 + y3 - y0 - y2).abs() > 0.25
    {
        return Err(retained_click_target_error(
            "retained child click requires a nondegenerate affine iframe content quad",
        ));
    }
    let x = x0 + horizontal.0 * point.x / viewport.width + vertical.0 * point.y / viewport.height;
    let y = y0 + horizontal.1 * point.x / viewport.width + vertical.1 * point.y / viewport.height;
    if !x.is_finite() || !y.is_finite() {
        return Err(invalid_response(
            "retained child click translated outside finite CSS geometry",
        ));
    }
    Ok(ClickPoint {
        x,
        y,
        is_mac: point.is_mac,
    })
}
