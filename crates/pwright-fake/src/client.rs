//! FakeCdpClient — in-memory browser simulation implementing `CdpClient`.
//!
//! Unlike `MockCdpClient` which records calls and returns canned responses,
//! `FakeCdpClient` holds an in-memory DOM tree that CDP operations work against.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use pwright_cdp::connection::Result as CdpResult;
use pwright_cdp::domains::accessibility::{AXValue, RawAXNode};
use pwright_cdp::domains::network::{Cookie, ResponseBody};
use pwright_cdp::domains::target::TargetInfo;
use pwright_cdp::{CdpClient, KeyEventType, MouseButton, MouseEventType, TouchEventType};
use serde_json::Value;

use crate::builder;
use crate::dom::DomNode;
use crate::selector;

/// In-memory browser fake for testing.
///
/// ```rust,ignore
/// let fake = FakeCdpClient::from_html(r#"
///     <div id="app">
///         <button class="submit" disabled>Submit</button>
///         <input type="checkbox" checked />
///     </div>
/// "#);
/// let page = Page::new(Arc::new(fake));
/// assert!(page.is_visible("button.submit").await?);
/// ```
pub struct FakeCdpClient {
    dom: Mutex<DomNode>,
    /// objectId -> nodeId mapping (assigned during dom_resolve_node).
    object_map: Mutex<HashMap<String, i64>>,
    next_object_id: Mutex<u64>,
    /// Configurable response for Runtime.evaluate (for complex JS).
    evaluate_response: Mutex<Option<Value>>,
    /// Event broadcast channel.
    event_tx: tokio::sync::broadcast::Sender<pwright_cdp::CdpEvent>,
    /// Recorded method calls (for optional verification).
    calls: Mutex<Vec<(String, Vec<Value>)>>,
}

impl FakeCdpClient {
    /// Create a fake browser from an HTML string.
    pub fn from_html(html: &str) -> Self {
        let dom = builder::parse_html(html);
        let (event_tx, _) = tokio::sync::broadcast::channel(1024);
        Self {
            dom: Mutex::new(dom),
            object_map: Mutex::new(HashMap::new()),
            next_object_id: Mutex::new(1),
            evaluate_response: Mutex::new(None),
            event_tx,
            calls: Mutex::new(Vec::new()),
        }
    }

    /// Set a custom response for `Runtime.evaluate`.
    pub fn set_evaluate_response(&self, response: Value) {
        *self.evaluate_response.lock().unwrap() = Some(response);
    }

    /// Inject a CDP event for testing network listeners.
    pub fn send_event(&self, event: pwright_cdp::CdpEvent) {
        let _ = self.event_tx.send(event);
    }

    /// Get recorded method calls (method, args).
    pub fn calls(&self) -> Vec<(String, Vec<Value>)> {
        self.calls.lock().unwrap().clone()
    }

    /// Get recorded method names.
    pub fn method_names(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .map(|(m, _)| m.clone())
            .collect()
    }

    fn record(&self, method: &str, args: Vec<Value>) {
        self.calls.lock().unwrap().push((method.to_string(), args));
    }

    fn find_node(&self, node_id: i64) -> Option<DomNode> {
        self.dom.lock().unwrap().find_by_node_id(node_id).cloned()
    }

    fn alloc_object_id(&self, node_id: i64) -> String {
        let mut next = self.next_object_id.lock().unwrap();
        let id = format!("fake-obj-{}", *next);
        *next += 1;
        self.object_map.lock().unwrap().insert(id.clone(), node_id);
        id
    }

    /// Evaluate JS property expressions on an element.
    ///
    /// Matches on the exact `pwright_js::element` constants rather than
    /// fragile substring checks, so JS formatting changes don't silently
    /// break the fake.
    fn eval_property_check(&self, object_id: &str, function_body: &str) -> Option<Value> {
        let node_id = self.object_map.lock().unwrap().get(object_id).copied()?;
        let node = self.find_node(node_id)?;

        if function_body == pwright_js::element::IS_CHECKED {
            return Some(serde_json::json!({"result": {"value": node.has_property("checked")}}));
        }
        if function_body == pwright_js::element::IS_DISABLED {
            return Some(serde_json::json!({"result": {"value": node.has_property("disabled")}}));
        }
        if function_body == pwright_js::element::GET_TEXT_CONTENT {
            return Some(serde_json::json!({"result": {"value": node.text_content()}}));
        }
        if function_body == pwright_js::element::GET_INNER_TEXT {
            return Some(serde_json::json!({"result": {"value": node.text_content()}}));
        }
        if function_body == pwright_js::element::GET_INPUT_VALUE {
            let val = node.value.as_deref().unwrap_or("");
            return Some(serde_json::json!({"result": {"value": val}}));
        }
        // blur/focus are called via various JS patterns — match on keyword
        if function_body.contains("blur()") || function_body.contains("focus()") {
            return Some(serde_json::json!({"result": {"value": null}}));
        }

        None
    }
}

#[async_trait]
impl CdpClient for FakeCdpClient {
    fn subscribe_events(&self) -> tokio::sync::broadcast::Receiver<pwright_cdp::CdpEvent> {
        self.event_tx.subscribe()
    }

    async fn browser_set_download_behavior(
        &self,
        behavior: pwright_cdp::DownloadBehavior,
        download_path: Option<&str>,
        events_enabled: bool,
    ) -> CdpResult<()> {
        self.record(
            "Browser.setDownloadBehavior",
            vec![serde_json::json!({
                "behavior": format!("{:?}", behavior),
                "downloadPath": download_path,
                "eventsEnabled": events_enabled,
            })],
        );
        Ok(())
    }

    async fn page_navigate(&self, url: &str) -> CdpResult<Value> {
        self.record("Page.navigate", vec![Value::String(url.to_string())]);
        Ok(serde_json::json!({"frameId": "fake-frame"}))
    }

    async fn page_enable(&self) -> CdpResult<()> {
        self.record("Page.enable", vec![]);
        Ok(())
    }

    async fn page_capture_screenshot(
        &self,
        format: &str,
        quality: Option<i32>,
        full_page: bool,
    ) -> CdpResult<String> {
        self.record(
            "Page.captureScreenshot",
            vec![serde_json::json!({"format": format, "quality": quality, "fullPage": full_page})],
        );
        Ok("bW9jaw==".to_string())
    }

    async fn page_print_to_pdf(&self, params: Value) -> CdpResult<String> {
        self.record("Page.printToPDF", vec![params]);
        // Return valid base64 (decodes to "%PDF-fake")
        Ok("JVBERi1mYWtl".to_string())
    }

    async fn page_add_script_on_new_document(&self, source: &str) -> CdpResult<String> {
        self.record(
            "Page.addScriptToEvaluateOnNewDocument",
            vec![Value::String(source.to_string())],
        );
        Ok("script-1".to_string())
    }

    async fn page_reload(&self) -> CdpResult<()> {
        self.record("Page.reload", vec![]);
        Ok(())
    }

    async fn page_get_navigation_history(&self) -> CdpResult<Value> {
        self.record("Page.getNavigationHistory", vec![]);
        Ok(serde_json::json!({
            "currentIndex": 0,
            "entries": [{"id": 0, "url": "about:blank", "title": ""}]
        }))
    }

    async fn page_navigate_to_history_entry(&self, entry_id: i64) -> CdpResult<()> {
        self.record(
            "Page.navigateToHistoryEntry",
            vec![serde_json::json!({"entryId": entry_id})],
        );
        Ok(())
    }

    async fn page_bring_to_front(&self) -> CdpResult<()> {
        self.record("Page.bringToFront", vec![]);
        Ok(())
    }

    async fn page_set_document_content(&self, frame_id: &str, html: &str) -> CdpResult<()> {
        self.record(
            "Page.setDocumentContent",
            vec![serde_json::json!({"frameId": frame_id, "html": html})],
        );
        // Update the in-memory DOM
        *self.dom.lock().unwrap() = builder::parse_html(html);
        Ok(())
    }

    // ── DOM domain (real behavior) ──

    async fn dom_focus(&self, node_id: i64) -> CdpResult<()> {
        self.record("DOM.focus", vec![serde_json::json!({"nodeId": node_id})]);
        Ok(())
    }

    async fn dom_scroll_into_view(&self, node_id: i64) -> CdpResult<()> {
        self.record(
            "DOM.scrollIntoViewIfNeeded",
            vec![serde_json::json!({"nodeId": node_id})],
        );
        Ok(())
    }

    async fn dom_get_box_model(&self, node_id: i64) -> CdpResult<Value> {
        self.record(
            "DOM.getBoxModel",
            vec![serde_json::json!({"nodeId": node_id})],
        );
        let node = self.find_node(node_id);
        match node {
            Some(n) if n.visible => Ok(serde_json::json!({
                "model": {
                    "content": [100.0, 200.0, 200.0, 200.0, 200.0, 300.0, 100.0, 300.0]
                }
            })),
            _ => Err(pwright_cdp::connection::CdpError::Other(
                "Could not compute box model".to_string(),
            )),
        }
    }

    async fn dom_resolve_node(&self, node_id: i64) -> CdpResult<Value> {
        self.record(
            "DOM.resolveNode",
            vec![serde_json::json!({"nodeId": node_id})],
        );
        let obj_id = self.alloc_object_id(node_id);
        Ok(serde_json::json!({"object": {"objectId": obj_id}}))
    }

    async fn dom_enable(&self) -> CdpResult<()> {
        self.record("DOM.enable", vec![]);
        Ok(())
    }

    async fn dom_get_document(&self) -> CdpResult<Value> {
        self.record("DOM.getDocument", vec![]);
        let root_id = self.dom.lock().unwrap().node_id;
        Ok(serde_json::json!({"root": {"nodeId": root_id}}))
    }

    async fn dom_query_selector(&self, _root_id: i64, css: &str) -> CdpResult<i64> {
        self.record(
            "DOM.querySelector",
            vec![serde_json::json!({"selector": css})],
        );
        let dom = self.dom.lock().unwrap();
        Ok(selector::query_selector(&dom, css).unwrap_or(0))
    }

    async fn dom_query_selector_all(&self, _root_id: i64, css: &str) -> CdpResult<Vec<i64>> {
        self.record(
            "DOM.querySelectorAll",
            vec![serde_json::json!({"selector": css})],
        );
        let dom = self.dom.lock().unwrap();
        Ok(selector::query_selector_all(&dom, css))
    }

    async fn dom_get_attributes(&self, node_id: i64) -> CdpResult<Vec<String>> {
        self.record(
            "DOM.getAttributes",
            vec![serde_json::json!({"nodeId": node_id})],
        );
        Ok(self
            .find_node(node_id)
            .map(|n| n.attributes_flat())
            .unwrap_or_default())
    }

    async fn dom_get_outer_html(&self, node_id: i64) -> CdpResult<String> {
        self.record(
            "DOM.getOuterHTML",
            vec![serde_json::json!({"nodeId": node_id})],
        );
        Ok(self
            .find_node(node_id)
            .map(|n| n.outer_html())
            .unwrap_or_default())
    }

    async fn dom_describe_node(&self, node_id: i64) -> CdpResult<Value> {
        self.record(
            "DOM.describeNode",
            vec![serde_json::json!({"nodeId": node_id})],
        );
        Ok(serde_json::json!({"node": {"nodeId": node_id}}))
    }

    async fn dom_set_file_input_files(&self, node_id: i64, files: &[String]) -> CdpResult<()> {
        self.record(
            "DOM.setFileInputFiles",
            vec![serde_json::json!({"nodeId": node_id, "files": files})],
        );
        Ok(())
    }

    async fn dom_request_node(&self, object_id: &str) -> CdpResult<i64> {
        self.record(
            "DOM.requestNode",
            vec![serde_json::json!({"objectId": object_id})],
        );
        Ok(self
            .object_map
            .lock()
            .unwrap()
            .get(object_id)
            .copied()
            .unwrap_or(0))
    }

    // ── Input domain (recording only) ──

    async fn input_dispatch_mouse_event(
        &self,
        event_type: MouseEventType,
        x: f64,
        y: f64,
        button: Option<MouseButton>,
        click_count: Option<i32>,
        buttons: Option<i32>,
    ) -> CdpResult<()> {
        self.record(
            "Input.dispatchMouseEvent",
            vec![serde_json::json!({
                "type": event_type.as_str(), "x": x, "y": y,
                "button": button.map(|b| b.as_str()), "clickCount": click_count, "buttons": buttons,
            })],
        );
        Ok(())
    }

    async fn input_dispatch_key_event(
        &self,
        event_type: KeyEventType,
        key: &str,
        code: &str,
        windows_virtual_key_code: Option<i64>,
    ) -> CdpResult<()> {
        self.record(
            "Input.dispatchKeyEvent",
            vec![serde_json::json!({
                "type": event_type.as_str(), "key": key, "code": code,
                "windowsVirtualKeyCode": windows_virtual_key_code,
            })],
        );
        Ok(())
    }

    async fn input_insert_text(&self, text: &str) -> CdpResult<()> {
        self.record("Input.insertText", vec![Value::String(text.to_string())]);
        Ok(())
    }

    async fn input_dispatch_touch_event(
        &self,
        event_type: TouchEventType,
        x: f64,
        y: f64,
    ) -> CdpResult<()> {
        self.record(
            "Input.dispatchTouchEvent",
            vec![serde_json::json!({"type": event_type.as_str(), "x": x, "y": y})],
        );
        Ok(())
    }

    // ── Runtime domain (hybrid: real for simple, configurable for complex) ──

    async fn runtime_evaluate(&self, expression: &str) -> CdpResult<Value> {
        self.record(
            "Runtime.evaluate",
            vec![Value::String(expression.to_string())],
        );
        if let Some(configured) = self.evaluate_response.lock().unwrap().clone() {
            return Ok(configured);
        }
        // Default: return empty result
        Ok(serde_json::json!({"result": {"value": ""}}))
    }

    async fn runtime_evaluate_as_object(&self, expression: &str) -> CdpResult<Value> {
        // Same as runtime_evaluate for the fake (no distinction needed)
        self.runtime_evaluate(expression).await
    }

    async fn runtime_call_function_on(
        &self,
        object_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
    ) -> CdpResult<Value> {
        self.record(
            "Runtime.callFunctionOn",
            vec![serde_json::json!({
                "objectId": object_id,
                "functionDeclaration": function_declaration,
                "arguments": arguments,
            })],
        );
        // Try to evaluate property checks against the DOM
        if let Some(result) = self.eval_property_check(object_id, function_declaration) {
            return Ok(result);
        }
        // Return bounding rect for functions that request element center coordinates
        if function_declaration.contains("getBoundingClientRect")
            || function_declaration.contains("getClientRects")
        {
            return Ok(serde_json::json!({
                "result": {"value": {"x": 150.0, "y": 250.0}}
            }));
        }
        // Fallback
        Ok(serde_json::json!({"result": {"value": null}}))
    }

    async fn runtime_evaluate_async(&self, expression: &str) -> CdpResult<Value> {
        self.record(
            "Runtime.evaluate(async)",
            vec![Value::String(expression.to_string())],
        );
        if let Some(configured) = self.evaluate_response.lock().unwrap().clone() {
            return Ok(configured);
        }
        Ok(serde_json::json!({"result": {"value": ""}}))
    }

    async fn runtime_enable(&self) -> CdpResult<()> {
        self.record("Runtime.enable", vec![]);
        Ok(())
    }

    // ── Accessibility domain ──

    async fn accessibility_enable(&self) -> CdpResult<()> {
        self.record("Accessibility.enable", vec![]);
        Ok(())
    }

    async fn accessibility_get_full_tree(&self) -> CdpResult<Vec<RawAXNode>> {
        self.record("Accessibility.getFullAXTree", vec![]);
        let dom = self.dom.lock().unwrap();
        let mut nodes = Vec::new();
        dom_to_ax_nodes(&dom, &mut nodes);
        Ok(nodes)
    }

    // ── Network domain ──

    async fn network_enable(&self) -> CdpResult<()> {
        self.record("Network.enable", vec![]);
        Ok(())
    }

    async fn network_set_blocked_urls(&self, patterns: &[String]) -> CdpResult<()> {
        self.record("Network.setBlockedURLs", vec![serde_json::json!(patterns)]);
        Ok(())
    }

    async fn network_get_cookies(&self) -> CdpResult<Vec<Cookie>> {
        self.record("Network.getCookies", vec![]);
        Ok(vec![])
    }

    async fn network_set_cookies(&self, cookies: &[Cookie]) -> CdpResult<()> {
        self.record("Network.setCookies", vec![serde_json::json!(cookies)]);
        Ok(())
    }

    async fn network_get_response_body(&self, request_id: &str) -> CdpResult<ResponseBody> {
        self.record(
            "Network.getResponseBody",
            vec![serde_json::json!({"requestId": request_id})],
        );
        Ok(ResponseBody {
            body: String::new(),
            base64_encoded: false,
        })
    }

    // ── Fetch domain ──

    async fn fetch_enable(&self) -> CdpResult<()> {
        self.record("Fetch.enable", vec![]);
        Ok(())
    }

    async fn fetch_disable(&self) -> CdpResult<()> {
        self.record("Fetch.disable", vec![]);
        Ok(())
    }

    async fn fetch_continue_request(&self, request_id: &str) -> CdpResult<()> {
        self.record(
            "Fetch.continueRequest",
            vec![Value::String(request_id.to_string())],
        );
        Ok(())
    }

    async fn fetch_fail_request(&self, request_id: &str, reason: &str) -> CdpResult<()> {
        self.record(
            "Fetch.failRequest",
            vec![
                Value::String(request_id.to_string()),
                Value::String(reason.to_string()),
            ],
        );
        Ok(())
    }

    // ── Target domain ──

    async fn target_create(&self, url: &str) -> CdpResult<String> {
        self.record("Target.createTarget", vec![Value::String(url.to_string())]);
        Ok("fake-target-1".to_string())
    }

    async fn target_close(&self, target_id: &str) -> CdpResult<()> {
        self.record(
            "Target.closeTarget",
            vec![Value::String(target_id.to_string())],
        );
        Ok(())
    }

    async fn target_get_targets(&self) -> CdpResult<Vec<TargetInfo>> {
        self.record("Target.getTargets", vec![]);
        Ok(vec![])
    }

    async fn target_attach(&self, target_id: &str) -> CdpResult<String> {
        self.record(
            "Target.attachToTarget",
            vec![Value::String(target_id.to_string())],
        );
        Ok("fake-session-1".to_string())
    }

    async fn target_detach(&self, session_id: &str) -> CdpResult<()> {
        self.record(
            "Target.detachFromTarget",
            vec![Value::String(session_id.to_string())],
        );
        Ok(())
    }
}

// ── DOM-to-AX tree conversion for fake accessibility support ──

/// Map an HTML tag to an approximate AX role.
fn tag_to_ax_role(tag: &str) -> &'static str {
    match tag {
        "html" => "document",
        "body" | "div" | "span" | "section" | "article" | "main" | "header" | "footer" | "nav"
        | "aside" => "generic",
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => "heading",
        "button" => "button",
        "a" => "link",
        "input" => "textbox",
        "textarea" => "textbox",
        "select" => "combobox",
        "option" => "option",
        "ul" | "ol" => "list",
        "li" => "listitem",
        "table" => "table",
        "tr" => "row",
        "td" | "th" => "cell",
        "img" => "image",
        "form" => "form",
        "label" => "LabelText",
        "p" => "paragraph",
        "#text" => "StaticText",
        _ => "generic",
    }
}

/// Recursively convert a DomNode tree into a flat list of RawAXNodes.
fn dom_to_ax_nodes(node: &DomNode, out: &mut Vec<RawAXNode>) {
    let mut ax_node_id_counter: u64 = 1;
    dom_to_ax_recursive(node, out, &mut ax_node_id_counter);
}

fn dom_to_ax_recursive(node: &DomNode, out: &mut Vec<RawAXNode>, counter: &mut u64) {
    let ax_id = format!("ax-{}", *counter);
    *counter += 1;

    let role = tag_to_ax_role(&node.tag);

    // Compute name from text content or attributes
    let name = if node.tag == "#text" {
        node.text.clone().unwrap_or_default()
    } else {
        // Use id, aria-label, or first text child as name
        node.attributes
            .get("aria-label")
            .cloned()
            .or_else(|| {
                // For simple elements, use direct text content
                node.children.iter().find_map(|c| {
                    if c.tag == "#text" {
                        c.text.clone()
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_default()
    };

    let value = node
        .value
        .clone()
        .or_else(|| node.attributes.get("value").cloned());

    let child_ids: Vec<String> = node
        .children
        .iter()
        .enumerate()
        .map(|(i, _)| format!("ax-{}", *counter + i as u64))
        .collect();

    let ax_node = RawAXNode {
        node_id: ax_id,
        ignored: false,
        role: Some(AXValue {
            value_type: "internalRole".to_string(),
            value: Value::String(role.to_string()),
        }),
        name: if name.is_empty() {
            None
        } else {
            Some(AXValue {
                value_type: "computedString".to_string(),
                value: Value::String(name),
            })
        },
        value: value.map(|v| AXValue {
            value_type: "string".to_string(),
            value: Value::String(v),
        }),
        properties: vec![],
        child_ids,
        backend_dom_node_id: node.node_id,
    };

    out.push(ax_node);

    for child in &node.children {
        dom_to_ax_recursive(child, out, counter);
    }
}
