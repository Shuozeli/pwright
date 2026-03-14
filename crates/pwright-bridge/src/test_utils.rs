//! Test utilities — MockCdpClient for unit testing bridge functions.
//!
//! Records all CDP method calls for assertion, returns configurable responses.

use std::sync::Mutex;

use async_trait::async_trait;
use pwright_cdp::CdpClient;
use pwright_cdp::connection::Result as CdpResult;
use pwright_cdp::domains::accessibility::RawAXNode;
use pwright_cdp::domains::network::{Cookie, ResponseBody};
use pwright_cdp::domains::target::TargetInfo;
use serde_json::Value;

/// A recorded CDP method call.
#[derive(Debug, Clone)]
pub struct CdpCall {
    pub method: String,
    pub args: Vec<Value>,
}

/// Mock implementation of CdpClient for testing.
///
/// Records all calls and returns configurable responses.
pub struct MockCdpClient {
    calls: Mutex<Vec<CdpCall>>,
    event_tx: tokio::sync::broadcast::Sender<pwright_cdp::CdpEvent>,
    box_model_response: Mutex<Option<Value>>,
    box_model_error: Mutex<bool>,
    strict: bool,
    resolve_node_response: Mutex<Option<Value>>,
    runtime_evaluate_response: Mutex<Option<Value>>,
    call_function_response: Mutex<Option<Value>>,
    navigate_response: Mutex<Option<Value>>,
    screenshot_response: Mutex<Option<String>>,
    pdf_response: Mutex<Option<String>>,
    cookies_response: Mutex<Option<Vec<Cookie>>>,
    ax_tree_response: Mutex<Option<Vec<RawAXNode>>>,
    targets_response: Mutex<Option<Vec<TargetInfo>>>,
    query_selector_response: Mutex<Option<i64>>,
    query_selector_all_response: Mutex<Option<Vec<i64>>>,
    get_attributes_response: Mutex<Option<Vec<String>>>,
    get_outer_html_response: Mutex<Option<String>>,
    describe_node_response: Mutex<Option<Value>>,
    navigation_history_response: Mutex<Option<Value>>,
}

impl Default for MockCdpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockCdpClient {
    pub fn new() -> Self {
        let (event_tx, _) = tokio::sync::broadcast::channel(1024);
        Self {
            calls: Mutex::new(Vec::new()),
            event_tx,
            box_model_response: Mutex::new(None),
            box_model_error: Mutex::new(false),
            strict: false,
            resolve_node_response: Mutex::new(None),
            runtime_evaluate_response: Mutex::new(None),
            call_function_response: Mutex::new(None),
            navigate_response: Mutex::new(None),
            screenshot_response: Mutex::new(None),
            pdf_response: Mutex::new(None),
            cookies_response: Mutex::new(None),
            ax_tree_response: Mutex::new(None),
            targets_response: Mutex::new(None),
            query_selector_response: Mutex::new(None),
            query_selector_all_response: Mutex::new(None),
            get_attributes_response: Mutex::new(None),
            get_outer_html_response: Mutex::new(None),
            describe_node_response: Mutex::new(None),
            navigation_history_response: Mutex::new(None),
        }
    }

    /// Record a method call.
    fn record(&self, method: &str, args: Vec<Value>) {
        self.calls.lock().unwrap().push(CdpCall {
            method: method.to_string(),
            args,
        });
    }

    /// Check strict mode and return error if response is not configured.
    fn check_strict(&self, method: &str) -> CdpResult<()> {
        if self.strict {
            Err(pwright_cdp::connection::CdpError::Other(format!(
                "unexpected call: {method}"
            )))
        } else {
            Ok(())
        }
    }

    /// Get all recorded calls.
    pub fn calls(&self) -> Vec<CdpCall> {
        self.calls.lock().unwrap().clone()
    }

    /// Get calls filtered by method name.
    pub fn calls_for(&self, method: &str) -> Vec<CdpCall> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c.method == method)
            .cloned()
            .collect()
    }

    /// Get the ordered list of method names called.
    pub fn method_names(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .map(|c| c.method.clone())
            .collect()
    }

    /// Set the response for DOM.getBoxModel.
    pub fn set_box_model(&self, response: Value) {
        *self.box_model_response.lock().unwrap() = Some(response);
    }

    /// Set the response for DOM.resolveNode.
    pub fn set_resolve_node(&self, response: Value) {
        *self.resolve_node_response.lock().unwrap() = Some(response);
    }

    /// Set the response for Runtime.evaluate.
    pub fn set_evaluate_response(&self, response: Value) {
        *self.runtime_evaluate_response.lock().unwrap() = Some(response);
    }

    /// Set the response for Runtime.callFunctionOn.
    pub fn set_call_function_response(&self, response: Value) {
        *self.call_function_response.lock().unwrap() = Some(response);
    }

    /// Set the response for Page.navigate.
    pub fn set_navigate_response(&self, response: Value) {
        *self.navigate_response.lock().unwrap() = Some(response);
    }

    /// Set the response for Page.captureScreenshot.
    pub fn set_screenshot_response(&self, response: String) {
        *self.screenshot_response.lock().unwrap() = Some(response);
    }

    /// Set the response for Page.printToPDF.
    pub fn set_pdf_response(&self, response: String) {
        *self.pdf_response.lock().unwrap() = Some(response);
    }

    /// Set the response for Network.getCookies.
    pub fn set_cookies_response(&self, cookies: Vec<Cookie>) {
        *self.cookies_response.lock().unwrap() = Some(cookies);
    }

    /// Set the response for Accessibility.getFullAXTree.
    pub fn set_ax_tree_response(&self, nodes: Vec<RawAXNode>) {
        *self.ax_tree_response.lock().unwrap() = Some(nodes);
    }

    /// Set the response for DOM.querySelector.
    pub fn set_query_selector_response(&self, node_id: i64) {
        *self.query_selector_response.lock().unwrap() = Some(node_id);
    }

    /// Set the response for DOM.querySelectorAll.
    pub fn set_query_selector_all_response(&self, node_ids: Vec<i64>) {
        *self.query_selector_all_response.lock().unwrap() = Some(node_ids);
    }

    /// Set the response for DOM.getAttributes.
    pub fn set_get_attributes_response(&self, attrs: Vec<String>) {
        *self.get_attributes_response.lock().unwrap() = Some(attrs);
    }

    /// Set the response for DOM.getOuterHTML.
    pub fn set_get_outer_html_response(&self, html: String) {
        *self.get_outer_html_response.lock().unwrap() = Some(html);
    }

    /// Set the response for Page.getNavigationHistory.
    pub fn set_navigation_history_response(&self, response: Value) {
        *self.navigation_history_response.lock().unwrap() = Some(response);
    }

    /// Make DOM.getBoxModel return an error (element not visible).
    pub fn set_box_model_error(&self, should_error: bool) {
        *self.box_model_error.lock().unwrap() = should_error;
    }

    /// Enable strict mode: unconfigured calls return an error instead of a default.
    pub fn strict(mut self) -> Self {
        self.strict = true;
        self
    }

    /// Inject a CDP event into the event stream for testing.
    pub fn send_event(&self, event: pwright_cdp::CdpEvent) {
        let _ = self.event_tx.send(event);
    }
}

#[async_trait]
impl CdpClient for MockCdpClient {
    fn subscribe_events(&self) -> tokio::sync::broadcast::Receiver<pwright_cdp::CdpEvent> {
        self.event_tx.subscribe()
    }

    async fn browser_set_download_behavior(
        &self,
        behavior: &str,
        download_path: Option<&str>,
        events_enabled: bool,
    ) -> CdpResult<()> {
        self.record(
            "Browser.setDownloadBehavior",
            vec![serde_json::json!({
                "behavior": behavior,
                "downloadPath": download_path,
                "eventsEnabled": events_enabled,
            })],
        );
        Ok(())
    }

    async fn page_navigate(&self, url: &str) -> CdpResult<Value> {
        self.record("Page.navigate", vec![Value::String(url.to_string())]);
        Ok(self
            .navigate_response
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| serde_json::json!({"frameId": "mock"})))
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
            vec![serde_json::json!({
                "format": format,
                "quality": quality,
                "fullPage": full_page,
            })],
        );
        Ok(self
            .screenshot_response
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| "bW9jaw==".to_string()))
    }

    async fn page_print_to_pdf(&self, params: Value) -> CdpResult<String> {
        self.record("Page.printToPDF", vec![params]);
        Ok(self
            .pdf_response
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| "JVBER".to_string()))
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
        Ok(self
            .navigation_history_response
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| {
                serde_json::json!({
                    "currentIndex": 0,
                    "entries": [{"id": 0, "url": "about:blank", "title": ""}]
                })
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
        Ok(())
    }

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
        if *self.box_model_error.lock().unwrap() {
            return Err(pwright_cdp::connection::CdpError::Other(
                "Could not compute box model".to_string(),
            ));
        }
        Ok(self
            .box_model_response
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| {
                serde_json::json!({
                    "model": {
                        "content": [100.0, 200.0, 200.0, 200.0, 200.0, 300.0, 100.0, 300.0]
                    }
                })
            }))
    }

    async fn dom_resolve_node(&self, node_id: i64) -> CdpResult<Value> {
        self.record(
            "DOM.resolveNode",
            vec![serde_json::json!({"nodeId": node_id})],
        );
        Ok(self
            .resolve_node_response
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| {
                serde_json::json!({
                    "object": {"objectId": "mock-obj-1"}
                })
            }))
    }

    async fn dom_enable(&self) -> CdpResult<()> {
        self.record("DOM.enable", vec![]);
        Ok(())
    }

    async fn dom_get_document(&self) -> CdpResult<Value> {
        self.record("DOM.getDocument", vec![]);
        Ok(serde_json::json!({"root": {"nodeId": 1}}))
    }

    async fn dom_query_selector(&self, node_id: i64, selector: &str) -> CdpResult<i64> {
        self.record(
            "DOM.querySelector",
            vec![serde_json::json!({"nodeId": node_id, "selector": selector})],
        );
        match *self.query_selector_response.lock().unwrap() {
            Some(v) => Ok(v),
            None => {
                self.check_strict("DOM.querySelector")?;
                Ok(0)
            }
        }
    }

    async fn dom_query_selector_all(&self, node_id: i64, selector: &str) -> CdpResult<Vec<i64>> {
        self.record(
            "DOM.querySelectorAll",
            vec![serde_json::json!({"nodeId": node_id, "selector": selector})],
        );
        Ok(self
            .query_selector_all_response
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_default())
    }

    async fn dom_get_attributes(&self, node_id: i64) -> CdpResult<Vec<String>> {
        self.record(
            "DOM.getAttributes",
            vec![serde_json::json!({"nodeId": node_id})],
        );
        Ok(self
            .get_attributes_response
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_default())
    }

    async fn dom_get_outer_html(&self, node_id: i64) -> CdpResult<String> {
        self.record(
            "DOM.getOuterHTML",
            vec![serde_json::json!({"nodeId": node_id})],
        );
        Ok(self
            .get_outer_html_response
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_default())
    }

    async fn dom_describe_node(&self, backend_node_id: i64) -> CdpResult<Value> {
        self.record(
            "DOM.describeNode",
            vec![serde_json::json!({"backendNodeId": backend_node_id})],
        );
        Ok(self
            .describe_node_response
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| serde_json::json!({"node": {"nodeId": 10}})))
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
        // Return the query_selector_response if set, otherwise default to 42
        Ok(self.query_selector_response.lock().unwrap().unwrap_or(42))
    }

    async fn input_dispatch_mouse_event(
        &self,
        event_type: &str,
        x: f64,
        y: f64,
        button: Option<&str>,
        click_count: Option<i32>,
        buttons: Option<i32>,
    ) -> CdpResult<()> {
        self.record(
            "Input.dispatchMouseEvent",
            vec![serde_json::json!({
                "type": event_type,
                "x": x,
                "y": y,
                "button": button,
                "clickCount": click_count,
                "buttons": buttons,
            })],
        );
        Ok(())
    }

    async fn input_dispatch_key_event(
        &self,
        event_type: &str,
        key: &str,
        code: &str,
        windows_virtual_key_code: Option<i64>,
    ) -> CdpResult<()> {
        self.record(
            "Input.dispatchKeyEvent",
            vec![serde_json::json!({
                "type": event_type,
                "key": key,
                "code": code,
                "windowsVirtualKeyCode": windows_virtual_key_code,
            })],
        );
        Ok(())
    }

    async fn input_insert_text(&self, text: &str) -> CdpResult<()> {
        self.record("Input.insertText", vec![Value::String(text.to_string())]);
        Ok(())
    }

    async fn input_dispatch_touch_event(&self, event_type: &str, x: f64, y: f64) -> CdpResult<()> {
        self.record(
            "Input.dispatchTouchEvent",
            vec![serde_json::json!({"type": event_type, "x": x, "y": y})],
        );
        Ok(())
    }

    async fn runtime_evaluate(&self, expression: &str) -> CdpResult<Value> {
        self.record(
            "Runtime.evaluate",
            vec![Value::String(expression.to_string())],
        );
        match self.runtime_evaluate_response.lock().unwrap().clone() {
            Some(v) => Ok(v),
            None => {
                self.check_strict("Runtime.evaluate")?;
                Ok(serde_json::json!({"result": {"value": ""}}))
            }
        }
    }

    async fn runtime_evaluate_as_object(&self, expression: &str) -> CdpResult<Value> {
        self.record(
            "Runtime.evaluate(object)",
            vec![Value::String(expression.to_string())],
        );
        match self.runtime_evaluate_response.lock().unwrap().clone() {
            Some(v) => Ok(v),
            None => {
                self.check_strict("Runtime.evaluate(object)")?;
                Ok(serde_json::json!({"result": {"value": ""}}))
            }
        }
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
        match self.call_function_response.lock().unwrap().clone() {
            Some(v) => Ok(v),
            None => {
                self.check_strict("Runtime.callFunctionOn")?;
                Ok(serde_json::json!({"result": {"value": null}}))
            }
        }
    }

    async fn runtime_evaluate_async(&self, expression: &str) -> CdpResult<Value> {
        self.record(
            "Runtime.evaluate(async)",
            vec![Value::String(expression.to_string())],
        );
        match self.runtime_evaluate_response.lock().unwrap().clone() {
            Some(v) => Ok(v),
            None => {
                self.check_strict("Runtime.evaluate(async)")?;
                Ok(serde_json::json!({"result": {"value": ""}}))
            }
        }
    }

    async fn runtime_enable(&self) -> CdpResult<()> {
        self.record("Runtime.enable", vec![]);
        Ok(())
    }

    async fn accessibility_enable(&self) -> CdpResult<()> {
        self.record("Accessibility.enable", vec![]);
        Ok(())
    }

    async fn accessibility_get_full_tree(&self) -> CdpResult<Vec<RawAXNode>> {
        self.record("Accessibility.getFullAXTree", vec![]);
        Ok(self
            .ax_tree_response
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_default())
    }

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
        Ok(self
            .cookies_response
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_default())
    }

    async fn network_set_cookies(&self, cookies: Vec<Value>) -> CdpResult<()> {
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

    async fn target_create(&self, url: &str) -> CdpResult<String> {
        self.record("Target.createTarget", vec![Value::String(url.to_string())]);
        Ok("target-1".to_string())
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
        Ok(self
            .targets_response
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_default())
    }

    async fn target_attach(&self, target_id: &str) -> CdpResult<String> {
        self.record(
            "Target.attachToTarget",
            vec![Value::String(target_id.to_string())],
        );
        Ok("session-1".to_string())
    }

    async fn target_detach(&self, session_id: &str) -> CdpResult<()> {
        self.record(
            "Target.detachFromTarget",
            vec![Value::String(session_id.to_string())],
        );
        Ok(())
    }
}
