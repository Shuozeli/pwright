//! Browser actions — click, type, fill, press, hover, scroll, drag, select.
//! Mirrors PinchTab's actions.go and cdp.go patterns.

use pwright_cdp::CdpClient;
use pwright_cdp::connection::{CdpError, Result as CdpResult};
use pwright_cdp::{KeyEventType, MouseButton, MouseEventType};
use serde_json::json;

use crate::keys;

/// Resolve a nodeId to a Runtime objectId for use with `callFunctionOn`.
///
/// This is the shared helper for the repeated pattern:
///   `dom_resolve_node(id) -> result["object"]["objectId"]`
pub async fn resolve_to_object_id(session: &dyn CdpClient, node_id: i64) -> CdpResult<String> {
    let resolved = session.dom_resolve_node(node_id).await?;
    resolved
        .get("object")
        .and_then(|o| o.get("objectId"))
        .and_then(|id| id.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| CdpError::Other("could not resolve node to objectId".to_string()))
}

/// Get the center coordinates of an element by backendNodeId.
async fn get_element_center(
    session: &dyn CdpClient,
    backend_node_id: i64,
) -> CdpResult<(f64, f64)> {
    let result = session.dom_get_box_model(backend_node_id).await?;

    if let Some(content) = result
        .get("model")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
        && content.len() >= 8
    {
        let x = (content[0].as_f64().unwrap_or(0.0)
            + content[2].as_f64().unwrap_or(0.0)
            + content[4].as_f64().unwrap_or(0.0)
            + content[6].as_f64().unwrap_or(0.0))
            / 4.0;
        let y = (content[1].as_f64().unwrap_or(0.0)
            + content[3].as_f64().unwrap_or(0.0)
            + content[5].as_f64().unwrap_or(0.0)
            + content[7].as_f64().unwrap_or(0.0))
            / 4.0;

        // Fallback to getBoundingClientRect if zero
        if x == 0.0 && y == 0.0 {
            return get_element_center_js(session, backend_node_id).await;
        }

        return Ok((x, y));
    }

    // Fallback to JS
    get_element_center_js(session, backend_node_id).await
}

/// Fallback: resolve node and use getBoundingClientRect().
async fn get_element_center_js(
    session: &dyn CdpClient,
    backend_node_id: i64,
) -> CdpResult<(f64, f64)> {
    let object_id = resolve_to_object_id(session, backend_node_id).await?;

    let rect_fn = pwright_js::element::GET_BOUNDING_CENTER;

    let result = session
        .runtime_call_function_on(&object_id, rect_fn, vec![])
        .await?;

    let value = result
        .get("result")
        .and_then(|r| r.get("value"))
        .ok_or_else(|| CdpError::Other("getBoundingClientRect returned no value".into()))?;
    let x = value
        .get("x")
        .and_then(|x| x.as_f64())
        .ok_or_else(|| CdpError::Other("getBoundingClientRect missing x coordinate".into()))?;
    let y = value
        .get("y")
        .and_then(|y| y.as_f64())
        .ok_or_else(|| CdpError::Other("getBoundingClientRect missing y coordinate".into()))?;

    Ok((x, y))
}

/// Dispatch a single mousePressed + mouseReleased pair at the given coordinates.
async fn dispatch_click_at(
    session: &dyn CdpClient,
    x: f64,
    y: f64,
    click_count: i32,
) -> CdpResult<()> {
    session
        .input_dispatch_mouse_event(
            MouseEventType::Pressed,
            x,
            y,
            Some(MouseButton::Left),
            Some(click_count),
            Some(1),
        )
        .await?;
    session
        .input_dispatch_mouse_event(
            MouseEventType::Released,
            x,
            y,
            Some(MouseButton::Left),
            Some(click_count),
            Some(0),
        )
        .await?;
    Ok(())
}

/// Click an element by nodeId.
///
/// Scrolls into view first, then gets viewport-relative coordinates via
/// `getBoundingClientRect`, then dispatches mouse events. The coordinate
/// retrieval MUST happen after scrolling because `Input.dispatchMouseEvent`
/// expects viewport coordinates, not page-absolute coordinates.
pub async fn click_by_node_id(session: &dyn CdpClient, node_id: i64) -> CdpResult<()> {
    // 1. Scroll element into view FIRST
    session.dom_scroll_into_view(node_id).await?;

    // 2. Get viewport-relative coordinates AFTER scrolling
    //    (getBoundingClientRect returns viewport coords, not page coords)
    let (x, y) = get_element_center_js(session, node_id).await?;

    // 3. Dispatch at viewport coordinates
    dispatch_click_at(session, x, y, 1).await
}

/// Double-click an element by nodeId.
///
/// Sends the correct 4-event sequence: pressed(1), released(1), pressed(2), released(2).
pub async fn dblclick_by_node_id(session: &dyn CdpClient, node_id: i64) -> CdpResult<()> {
    session.dom_scroll_into_view(node_id).await?;
    let (x, y) = get_element_center_js(session, node_id).await?;

    // First click (clickCount=1)
    dispatch_click_at(session, x, y, 1).await?;
    // Second click (clickCount=2)
    dispatch_click_at(session, x, y, 2).await
}

/// Type text into an element by backendNodeId.
pub async fn type_by_node_id(session: &dyn CdpClient, node_id: i64, text: &str) -> CdpResult<()> {
    session.dom_focus(node_id).await?;
    for ch in text.chars() {
        session.input_insert_text(&ch.to_string()).await?;
    }
    Ok(())
}

/// Fill an element's value (set value + dispatch events).
pub async fn fill_by_node_id(session: &dyn CdpClient, node_id: i64, value: &str) -> CdpResult<()> {
    session.dom_focus(node_id).await?;

    let object_id = resolve_to_object_id(session, node_id).await?;

    let js = pwright_js::element::SET_VALUE;

    session
        .runtime_call_function_on(&object_id, js, vec![json!({"value": value})])
        .await?;

    Ok(())
}

/// Hover over an element by nodeId.
///
/// Scrolls into view first, then gets viewport-relative coordinates.
pub async fn hover_by_node_id(session: &dyn CdpClient, node_id: i64) -> CdpResult<()> {
    session.dom_scroll_into_view(node_id).await?;
    let (x, y) = get_element_center(session, node_id).await?;
    session
        .input_dispatch_mouse_event(MouseEventType::Moved, x, y, None, None, None)
        .await?;
    Ok(())
}

/// Scroll an element into view.
pub async fn scroll_by_node_id(session: &dyn CdpClient, node_id: i64) -> CdpResult<()> {
    session.dom_scroll_into_view(node_id).await
}

/// Scroll the page by (dx, dy) pixels.
pub async fn scroll_page(session: &dyn CdpClient, dx: i32, dy: i32) -> CdpResult<()> {
    let js = pwright_js::page::scroll_by(dx, dy);
    session.runtime_evaluate(&js).await?;
    Ok(())
}

/// Drag an element by (dx, dy) pixels.
pub async fn drag_by_node_id(
    session: &dyn CdpClient,
    node_id: i64,
    dx: i32,
    dy: i32,
) -> CdpResult<()> {
    session.dom_scroll_into_view(node_id).await?;
    let (x, y) = get_element_center(session, node_id).await?;

    const DRAG_STEP_PX: f64 = 10.0;
    const DRAG_MIN_STEPS: f64 = 5.0;
    const DRAG_MAX_STEPS: f64 = 40.0;
    let dist = ((dx * dx + dy * dy) as f64).sqrt();
    let steps = (dist / DRAG_STEP_PX).clamp(DRAG_MIN_STEPS, DRAG_MAX_STEPS) as i32;

    session
        .input_dispatch_mouse_event(MouseEventType::Moved, x, y, None, None, None)
        .await?;
    session
        .input_dispatch_mouse_event(
            MouseEventType::Pressed,
            x,
            y,
            Some(MouseButton::Left),
            Some(1),
            None,
        )
        .await?;
    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let mx = x + t * dx as f64;
        let my = y + t * dy as f64;
        session
            .input_dispatch_mouse_event(MouseEventType::Moved, mx, my, None, None, Some(1))
            .await?;
    }
    let end_x = x + dx as f64;
    let end_y = y + dy as f64;
    session
        .input_dispatch_mouse_event(
            MouseEventType::Released,
            end_x,
            end_y,
            Some(MouseButton::Left),
            Some(1),
            None,
        )
        .await?;

    Ok(())
}

/// Dispatch a named key press (Enter, Tab, Escape, etc.).
pub async fn press_key(session: &dyn CdpClient, key: &str) -> CdpResult<()> {
    if let Some(def) = keys::get_key_def(key) {
        // CDP key param must be the W3C key value. "Return" is accepted by
        // get_key_def as an alias but "Enter" is the canonical W3C name.
        let w3c_key = if key == "Return" { "Enter" } else { key };

        session
            .input_dispatch_key_event(
                KeyEventType::RawKeyDown,
                w3c_key,
                &def.code,
                Some(def.virtual_key),
            )
            .await?;

        if !def.insert_text.is_empty() {
            session.input_insert_text(def.insert_text).await?;
        }

        session
            .input_dispatch_key_event(
                KeyEventType::KeyUp,
                w3c_key,
                &def.code,
                Some(def.virtual_key),
            )
            .await?;
    } else {
        session.input_insert_text(key).await?;
    }

    Ok(())
}

/// Check whether a checkbox/radio element is currently checked.
pub async fn is_checked(session: &dyn CdpClient, node_id: i64) -> CdpResult<bool> {
    let object_id = resolve_to_object_id(session, node_id).await?;
    let result = session
        .runtime_call_function_on(&object_id, pwright_js::element::IS_CHECKED, vec![])
        .await?;
    Ok(result
        .get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false))
}

/// Select a value in a `<select>` element.
///
/// Uses `Runtime.callFunctionOn` to properly set `selectedIndex` and
/// dispatch input/change events, instead of just setting `.value`.
pub async fn select_by_node_id(
    session: &dyn CdpClient,
    node_id: i64,
    value: &str,
) -> CdpResult<()> {
    session.dom_focus(node_id).await?;

    let object_id = resolve_to_object_id(session, node_id).await?;

    let js = pwright_js::element::SELECT_OPTION;

    session
        .runtime_call_function_on(&object_id, js, vec![serde_json::json!({"value": value})])
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockCdpClient;

    #[tokio::test]
    async fn test_click_calls_correct_cdp_sequence() {
        let mock = MockCdpClient::new();
        // Set up callFunctionOn response for getBoundingClientRect center
        mock.set_call_function_response(serde_json::json!({
            "result": {"value": {"x": 100.0, "y": 200.0}}
        }));

        click_by_node_id(&mock, 42).await.unwrap();

        let methods = mock.method_names();
        // Expected: scrollIntoView → resolveNode → callFunctionOn(center) → mousePressed → mouseReleased
        assert_eq!(methods[0], "DOM.scrollIntoViewIfNeeded");
        assert_eq!(methods[1], "DOM.resolveNode");
        assert_eq!(methods[2], "Runtime.callFunctionOn");
        assert_eq!(methods[3], "Input.dispatchMouseEvent");
        assert_eq!(methods[4], "Input.dispatchMouseEvent");

        // Verify mouse events are press then release with buttons field
        let mouse_calls = mock.calls_for("Input.dispatchMouseEvent");
        assert_eq!(mouse_calls[0].args[0]["type"], "mousePressed");
        assert_eq!(mouse_calls[0].args[0]["buttons"], 1);
        assert_eq!(mouse_calls[1].args[0]["type"], "mouseReleased");
        assert_eq!(mouse_calls[1].args[0]["buttons"], 0);
    }

    #[tokio::test]
    async fn test_click_uses_viewport_coords_from_js() {
        let mock = MockCdpClient::new();
        // getBoundingClientRect returns viewport-relative center
        mock.set_call_function_response(serde_json::json!({
            "result": {"value": {"x": 50.0, "y": 75.0}}
        }));

        click_by_node_id(&mock, 1).await.unwrap();

        let mouse_calls = mock.calls_for("Input.dispatchMouseEvent");
        assert_eq!(mouse_calls[0].args[0]["x"], 50.0);
        assert_eq!(mouse_calls[0].args[0]["y"], 75.0);
    }

    #[tokio::test]
    async fn test_type_inserts_each_character() {
        let mock = MockCdpClient::new();
        type_by_node_id(&mock, 10, "abc").await.unwrap();

        let methods = mock.method_names();
        assert_eq!(methods[0], "DOM.focus");

        let insert_calls = mock.calls_for("Input.insertText");
        assert_eq!(insert_calls.len(), 3);
        assert_eq!(insert_calls[0].args[0], "a");
        assert_eq!(insert_calls[1].args[0], "b");
        assert_eq!(insert_calls[2].args[0], "c");
    }

    #[tokio::test]
    async fn test_fill_calls_focus_resolve_and_call_function() {
        let mock = MockCdpClient::new();
        fill_by_node_id(&mock, 10, "hello").await.unwrap();

        let methods = mock.method_names();
        assert_eq!(methods[0], "DOM.focus");
        assert_eq!(methods[1], "DOM.resolveNode");
        assert_eq!(methods[2], "Runtime.callFunctionOn");

        // Verify the value is passed
        let fn_calls = mock.calls_for("Runtime.callFunctionOn");
        let args = &fn_calls[0].args[0];
        assert_eq!(args["objectId"], "mock-obj-1");
        assert!(args["arguments"][0]["value"] == "hello");
    }

    #[tokio::test]
    async fn test_hover_moves_mouse_to_element_center() {
        let mock = MockCdpClient::new();
        hover_by_node_id(&mock, 5).await.unwrap();

        let methods = mock.method_names();
        // scroll FIRST, then get coords, then dispatch
        assert_eq!(methods[0], "DOM.scrollIntoViewIfNeeded");
        assert_eq!(methods[1], "DOM.getBoxModel");
        assert_eq!(methods[2], "Input.dispatchMouseEvent");

        let mouse = mock.calls_for("Input.dispatchMouseEvent");
        assert_eq!(mouse[0].args[0]["type"], "mouseMoved");
    }

    #[tokio::test]
    async fn test_press_key_enter_sends_three_events() {
        let mock = MockCdpClient::new();
        press_key(&mock, "Enter").await.unwrap();

        let methods = mock.method_names();
        assert_eq!(methods[0], "Input.dispatchKeyEvent"); // rawKeyDown
        assert_eq!(methods[1], "Input.insertText"); // "\r"
        assert_eq!(methods[2], "Input.dispatchKeyEvent"); // keyUp

        let key_calls = mock.calls_for("Input.dispatchKeyEvent");
        assert_eq!(key_calls[0].args[0]["type"], "rawKeyDown");
        assert_eq!(key_calls[1].args[0]["type"], "keyUp");
    }

    #[tokio::test]
    async fn test_press_key_escape_no_insert_text() {
        let mock = MockCdpClient::new();
        press_key(&mock, "Escape").await.unwrap();

        let methods = mock.method_names();
        // Escape has no insert_text, so only rawKeyDown + keyUp
        assert_eq!(methods.len(), 2);
        assert_eq!(methods[0], "Input.dispatchKeyEvent");
        assert_eq!(methods[1], "Input.dispatchKeyEvent");

        assert!(mock.calls_for("Input.insertText").is_empty());
    }

    #[tokio::test]
    async fn test_press_unknown_key_inserts_directly() {
        let mock = MockCdpClient::new();
        press_key(&mock, "x").await.unwrap();

        let methods = mock.method_names();
        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0], "Input.insertText");

        let insert = mock.calls_for("Input.insertText");
        assert_eq!(insert[0].args[0], "x");
    }

    #[tokio::test]
    async fn test_drag_creates_intermediate_steps() {
        let mock = MockCdpClient::new();
        drag_by_node_id(&mock, 1, 100, 0).await.unwrap();

        let methods = mock.method_names();
        // scrollIntoView + getBoxModel + mouseMoved(start) + mousePressed + N*mouseMoved + mouseReleased
        assert_eq!(methods[0], "DOM.scrollIntoViewIfNeeded");
        assert_eq!(methods[1], "DOM.getBoxModel");
        assert_eq!(methods[2], "Input.dispatchMouseEvent"); // mouseMoved to start

        let mouse_calls = mock.calls_for("Input.dispatchMouseEvent");
        // First=mouseMoved(start), Second=mousePressed, Middle=intermediate moves, Last=mouseReleased
        assert!(mouse_calls.len() >= 5); // at minimum 5 steps
        assert_eq!(mouse_calls.last().unwrap().args[0]["type"], "mouseReleased");
    }

    #[tokio::test]
    async fn test_scroll_page_evaluates_js() {
        let mock = MockCdpClient::new();
        scroll_page(&mock, 0, 500).await.unwrap();

        let eval_calls = mock.calls_for("Runtime.evaluate");
        assert_eq!(eval_calls.len(), 1);
        assert_eq!(eval_calls[0].args[0], "window.scrollBy(0, 500)");
    }

    #[tokio::test]
    async fn test_scroll_by_node_id_calls_scroll_into_view() {
        let mock = MockCdpClient::new();
        scroll_by_node_id(&mock, 99).await.unwrap();

        let methods = mock.method_names();
        assert_eq!(methods, vec!["DOM.scrollIntoViewIfNeeded"]);
    }
}
