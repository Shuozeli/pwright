//! FakeCdpClient — in-memory browser simulation implementing `CdpClient`.
//!
//! Unlike `MockCdpClient` which records calls and returns canned responses,
//! `FakeCdpClient` holds an in-memory DOM tree that CDP operations work against.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

use async_trait::async_trait;
use pwright_cdp::connection::Result as CdpResult;
use pwright_cdp::domains::accessibility::{AXValue, RawAXNode};
use pwright_cdp::domains::network::{Cookie, ResponseBody};
use pwright_cdp::domains::target::TargetInfo;
use pwright_cdp::generated::{page as cdp_page, target as cdp_target};
use pwright_cdp::{
    CdpClient, KeyEventType, KeyboardEventParams, MouseButton, MouseEventType, TouchEventType,
};
use serde_json::Value;

use crate::builder;
use crate::call_log::{CdpCallLog, RecordedCdpCall};
use crate::dom::DomNode;
use crate::response_script::CdpResponseScript;
use crate::response_slot::CdpResponseSlot;
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
    /// Ordered errors/responses and persistent fallback for Runtime methods.
    evaluate: CdpResponseScript<Value>,
    evaluate_gate: Mutex<Option<Arc<tokio::sync::Notify>>>,
    get_properties_gate: Mutex<Option<Arc<tokio::sync::Notify>>>,
    release_object_error: Mutex<Option<String>>,
    release_object_gate: Mutex<Option<Arc<tokio::sync::Notify>>>,
    release_object_group_gate: Mutex<Option<Arc<tokio::sync::Notify>>>,
    /// Configurable response for Page.navigate.
    navigate_response: CdpResponseSlot<Value>,
    frame_navigate_gate: Mutex<Option<Arc<tokio::sync::Notify>>>,
    screenshot_response: Mutex<String>,
    screenshot_gate: Mutex<Option<Arc<tokio::sync::Notify>>>,
    frame_owner_responses: Mutex<HashMap<String, i64>>,
    backend_box_model_responses: Mutex<HashMap<i64, Value>>,
    object_backend_nodes: Mutex<HashMap<String, i64>>,
    object_node_descriptions: Mutex<HashMap<String, Value>>,
    describe_node_error: Mutex<Option<String>>,
    describe_node_gate: Mutex<Option<Arc<tokio::sync::Notify>>>,
    adopted_node_responses: Mutex<HashMap<(i64, i64), Value>>,
    adoption_gate: Mutex<Option<Arc<tokio::sync::Notify>>>,
    main_frame_id: Mutex<String>,
    frame_tree_response: Mutex<Option<cdp_page::FrameTree>>,
    frame_tree_responses: Mutex<VecDeque<cdp_page::FrameTree>>,
    create_isolated_world_errors: Mutex<VecDeque<Option<String>>>,
    create_isolated_world_gate: Mutex<Option<Arc<tokio::sync::Notify>>>,
    create_browser_context_response: Mutex<String>,
    create_browser_context_error: Mutex<Option<String>>,
    create_browser_context_gate: Mutex<Option<Arc<tokio::sync::Notify>>>,
    dispose_browser_context_error: Mutex<Option<String>>,
    dispose_browser_context_gate: Mutex<Option<Arc<tokio::sync::Notify>>>,
    target_create_gate: Mutex<Option<Arc<tokio::sync::Notify>>>,
    target_close_error: Mutex<Option<String>>,
    target_close_gate: Mutex<Option<Arc<tokio::sync::Notify>>>,
    target_detach_gate: Mutex<Option<Arc<tokio::sync::Notify>>>,
    target_detach_errors: Mutex<VecDeque<Option<String>>>,
    target_responses: Mutex<VecDeque<Vec<TargetInfo>>>,
    browser_contexts_response: Mutex<Vec<String>>,
    input_errors: Mutex<VecDeque<Option<String>>>,
    pointer_gates: Mutex<VecDeque<Option<Arc<tokio::sync::Notify>>>>,
    auto_attach_event_sink: Mutex<Option<tokio::sync::broadcast::Sender<pwright_cdp::CdpEvent>>>,
    auto_attach_events: Mutex<VecDeque<pwright_cdp::CdpEvent>>,
    page_enable_events: Mutex<VecDeque<Vec<pwright_cdp::CdpEvent>>>,
    page_enable_gate: Mutex<Option<Arc<tokio::sync::Notify>>>,
    reload_events: Mutex<Vec<pwright_cdp::CdpEvent>>,
    network_errors: Mutex<VecDeque<Option<String>>>,
    network_gate: Mutex<Option<Arc<tokio::sync::Notify>>>,
    /// Event broadcast channel.
    event_tx: tokio::sync::broadcast::Sender<pwright_cdp::CdpEvent>,
    /// Optional immutable session identity for full-path fake CDP tests.
    event_session_id: OnceLock<String>,
    /// Recorded method calls (for optional verification).
    calls: CdpCallLog,
}

impl FakeCdpClient {
    async fn resolve_node_for_adoption(
        &self,
        backend_node_id: i64,
        execution_context_id: i64,
        object_group: Option<&str>,
    ) -> CdpResult<Value> {
        let mut params = serde_json::json!({
            "backendNodeId": backend_node_id,
            "executionContextId": execution_context_id,
        });
        if let Some(group) = object_group {
            params["objectGroup"] = group.into();
        }
        self.record("DOM.resolveNode", vec![params]);
        let gate = self.adoption_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.notified().await;
        }
        self.adopted_node_responses.lock().unwrap()
            .get(&(backend_node_id, execution_context_id)).cloned()
            .ok_or_else(|| pwright_cdp::connection::CdpError::Other(format!(
                "No adopted node configured for backend node {backend_node_id} in context {execution_context_id}"
            )))
    }

    /// Create a fake browser from an HTML string.
    pub fn from_html(html: &str) -> Self {
        let dom = builder::parse_html(html);
        let (event_tx, _) = tokio::sync::broadcast::channel(1024);
        Self {
            dom: Mutex::new(dom),
            object_map: Mutex::new(HashMap::new()),
            next_object_id: Mutex::new(1),
            evaluate: CdpResponseScript::new(),
            evaluate_gate: Mutex::new(None),
            get_properties_gate: Mutex::new(None),
            release_object_error: Mutex::new(None),
            release_object_gate: Mutex::new(None),
            release_object_group_gate: Mutex::new(None),
            navigate_response: CdpResponseSlot::new(),
            frame_navigate_gate: Mutex::new(None),
            screenshot_response: Mutex::new("bW9jaw==".to_owned()),
            screenshot_gate: Mutex::new(None),
            frame_owner_responses: Mutex::new(HashMap::new()),
            backend_box_model_responses: Mutex::new(HashMap::new()),
            object_backend_nodes: Mutex::new(HashMap::new()),
            object_node_descriptions: Mutex::new(HashMap::new()),
            describe_node_error: Mutex::new(None),
            describe_node_gate: Mutex::new(None),
            adopted_node_responses: Mutex::new(HashMap::new()),
            adoption_gate: Mutex::new(None),
            main_frame_id: Mutex::new("fake-frame-1".to_string()),
            frame_tree_response: Mutex::new(None),
            frame_tree_responses: Mutex::new(VecDeque::new()),
            create_isolated_world_errors: Mutex::new(VecDeque::new()),
            create_isolated_world_gate: Mutex::new(None),
            create_browser_context_response: Mutex::new("fake-browser-context-1".to_string()),
            create_browser_context_error: Mutex::new(None),
            create_browser_context_gate: Mutex::new(None),
            dispose_browser_context_error: Mutex::new(None),
            dispose_browser_context_gate: Mutex::new(None),
            target_create_gate: Mutex::new(None),
            target_close_error: Mutex::new(None),
            target_close_gate: Mutex::new(None),
            target_detach_gate: Mutex::new(None),
            target_detach_errors: Mutex::new(VecDeque::new()),
            target_responses: Mutex::new(VecDeque::new()),
            browser_contexts_response: Mutex::new(Vec::new()),
            input_errors: Mutex::new(VecDeque::new()),
            pointer_gates: Mutex::new(VecDeque::new()),
            auto_attach_event_sink: Mutex::new(None),
            auto_attach_events: Mutex::new(VecDeque::new()),
            page_enable_events: Mutex::new(VecDeque::new()),
            page_enable_gate: Mutex::new(None),
            reload_events: Mutex::new(Vec::new()),
            network_errors: Mutex::new(VecDeque::new()),
            network_gate: Mutex::new(None),
            event_tx,
            event_session_id: OnceLock::new(),
            calls: CdpCallLog::new(),
        }
    }

    /// Select one stable event session identity before exercising this fake.
    pub fn set_event_session_id(&self, session_id: impl Into<String>) -> Result<(), String> {
        self.event_session_id.set(session_id.into())
    }

    /// Set a custom response for `Runtime.evaluate`.
    pub fn set_evaluate_response(&self, response: Value) {
        self.evaluate.set_fallback(response);
    }

    /// Set ordered responses consumed by successive `Runtime.evaluate` calls.
    pub fn set_evaluate_responses(&self, responses: impl IntoIterator<Item = Value>) {
        self.evaluate.set_responses(responses);
    }

    /// Configure ordered Runtime.evaluate protocol failures.
    pub fn set_evaluate_errors(&self, errors: impl IntoIterator<Item = String>) {
        self.evaluate.set_errors(errors);
    }

    /// Configure `Runtime.releaseObject` to fail with a deterministic CDP
    /// protocol error. Passing `None` restores normal success.
    pub fn set_release_object_error(&self, message: Option<String>) {
        *self.release_object_error.lock().unwrap() = message;
    }

    pub fn set_release_object_gate(&self, gate: Option<Arc<tokio::sync::Notify>>) {
        *self.release_object_gate.lock().unwrap() = gate;
    }

    pub fn set_release_object_group_gate(&self, gate: Option<Arc<tokio::sync::Notify>>) {
        *self.release_object_group_gate.lock().unwrap() = gate;
    }

    /// Blocks Runtime.evaluate calls until notified. Clearing the gate restores
    /// immediate fake responses.
    pub fn set_evaluate_gate(&self, gate: Option<Arc<tokio::sync::Notify>>) {
        *self.evaluate_gate.lock().unwrap() = gate;
    }

    /// Pause property reads after recording the exact native command attempt.
    pub fn set_get_properties_gate(&self, gate: Option<Arc<tokio::sync::Notify>>) {
        *self.get_properties_gate.lock().unwrap() = gate;
    }

    /// Set a custom response for `Page.navigate`.
    pub fn set_navigate_response(&self, response: Value) {
        self.navigate_response.set(response);
    }

    /// Hold a Frame-targeted navigation reply after recording its dispatch.
    pub fn set_frame_navigate_gate(&self, gate: Option<Arc<tokio::sync::Notify>>) {
        *self.frame_navigate_gate.lock().unwrap() = gate;
    }

    /// Hold `Page.enable` after recording it so popup-startup ordering can be
    /// tested without a browser process.
    pub fn set_page_enable_gate(&self, gate: Option<Arc<tokio::sync::Notify>>) {
        *self.page_enable_gate.lock().unwrap() = gate;
    }

    pub fn set_screenshot_response(&self, response: impl Into<String>) {
        *self.screenshot_response.lock().unwrap() = response.into();
    }

    pub fn set_screenshot_gate(&self, gate: Option<Arc<tokio::sync::Notify>>) {
        *self.screenshot_gate.lock().unwrap() = gate;
    }

    pub fn set_frame_owner(&self, frame_id: impl Into<String>, backend_node_id: i64) {
        self.frame_owner_responses
            .lock()
            .unwrap()
            .insert(frame_id.into(), backend_node_id);
    }

    pub fn set_backend_box_model(&self, backend_node_id: i64, response: Value) {
        self.backend_box_model_responses
            .lock()
            .unwrap()
            .insert(backend_node_id, response);
    }

    pub fn set_object_backend_node(&self, object_id: impl Into<String>, backend_node_id: i64) {
        self.object_backend_nodes
            .lock()
            .unwrap()
            .insert(object_id.into(), backend_node_id);
    }

    /// Configure an unmodified raw `DOM.describeNode` result for one object ID.
    pub fn set_object_node_description(&self, object_id: impl Into<String>, response: Value) {
        self.object_node_descriptions
            .lock()
            .unwrap()
            .insert(object_id.into(), response);
    }

    pub fn set_describe_node_error(&self, message: Option<String>) {
        *self.describe_node_error.lock().unwrap() = message;
    }

    /// Blocks raw object-ID descriptions after recording the exact request.
    pub fn set_describe_node_gate(&self, gate: Option<Arc<tokio::sync::Notify>>) {
        *self.describe_node_gate.lock().unwrap() = gate;
    }

    pub fn set_adopted_node_response(
        &self,
        backend_node_id: i64,
        execution_context_id: i64,
        response: Value,
    ) {
        self.adopted_node_responses
            .lock()
            .unwrap()
            .insert((backend_node_id, execution_context_id), response);
    }

    /// Blocks adopted `DOM.resolveNode` calls after recording the request and
    /// before returning the configured RemoteObject.
    pub fn set_adoption_gate(&self, gate: Option<Arc<tokio::sync::Notify>>) {
        *self.adoption_gate.lock().unwrap() = gate;
    }

    pub fn set_main_frame_id(&self, frame_id: impl Into<String>) {
        *self.main_frame_id.lock().unwrap() = frame_id.into();
        *self.frame_tree_response.lock().unwrap() = None;
    }

    pub fn set_frame_tree(&self, frame_tree: cdp_page::FrameTree) {
        *self.main_frame_id.lock().unwrap() = frame_tree.frame.id.clone();
        *self.frame_tree_response.lock().unwrap() = Some(frame_tree);
    }

    /// Emit these native lifecycle events during the next Page.reload call.
    pub fn set_reload_events(&self, events: impl IntoIterator<Item = pwright_cdp::CdpEvent>) {
        *self.reload_events.lock().unwrap() = events.into_iter().collect();
    }

    /// Set ordered frame-tree snapshots consumed by successive reads.
    pub fn set_frame_trees(&self, frame_trees: impl IntoIterator<Item = cdp_page::FrameTree>) {
        *self.frame_tree_responses.lock().unwrap() = frame_trees.into_iter().collect();
    }

    /// Configure ordered isolated-world outcomes. `None` succeeds; `Some`
    /// returns a deterministic CDP protocol error.
    pub fn set_create_isolated_world_errors(
        &self,
        errors: impl IntoIterator<Item = Option<String>>,
    ) {
        *self.create_isolated_world_errors.lock().unwrap() = errors.into_iter().collect();
    }

    pub fn set_create_isolated_world_gate(&self, gate: Option<Arc<tokio::sync::Notify>>) {
        *self.create_isolated_world_gate.lock().unwrap() = gate;
    }

    async fn finish_target_detach(&self) -> CdpResult<()> {
        let gate = self.target_detach_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.notified().await;
        }
        if let Some(message) = self
            .target_detach_errors
            .lock()
            .unwrap()
            .pop_front()
            .flatten()
        {
            return Err(pwright_cdp::connection::CdpError::Protocol {
                code: -32000,
                message,
            });
        }
        Ok(())
    }

    pub fn set_create_browser_context_response(&self, response: impl Into<String>) {
        *self.create_browser_context_response.lock().unwrap() = response.into();
    }

    pub fn set_browser_contexts_response(&self, context_ids: Vec<String>) {
        *self.browser_contexts_response.lock().unwrap() = context_ids;
    }

    /// Configure ordered input-dispatch outcomes. `None` succeeds; `Some`
    /// returns a deterministic CDP protocol error after recording the call.
    pub fn set_input_errors(&self, errors: impl IntoIterator<Item = Option<String>>) {
        *self.input_errors.lock().unwrap() = errors.into_iter().collect();
    }

    /// Ordered native pointer reply gates. Calls record/capture their error at
    /// submission; each optional gate delays only that call's acknowledgement.
    pub fn set_pointer_gates(
        &self,
        gates: impl IntoIterator<Item = Option<Arc<tokio::sync::Notify>>>,
    ) {
        *self.pointer_gates.lock().unwrap() = gates.into_iter().collect();
    }

    /// Configure ordered Network-domain outcomes. `None` succeeds; `Some`
    /// returns a deterministic CDP protocol error after recording the call.
    pub fn set_network_errors(&self, errors: impl IntoIterator<Item = Option<String>>) {
        *self.network_errors.lock().unwrap() = errors.into_iter().collect();
    }
    pub fn set_target_detach_gate(&self, gate: Option<Arc<tokio::sync::Notify>>) {
        *self.target_detach_gate.lock().unwrap() = gate;
    }
    pub fn set_target_detach_errors(&self, errors: impl IntoIterator<Item = Option<String>>) {
        *self.target_detach_errors.lock().unwrap() = errors.into_iter().collect();
    }

    /// Blocks Network enable/disable calls until notified. Clearing the gate
    /// restores immediate fake responses.
    pub fn set_network_gate(&self, gate: Option<Arc<tokio::sync::Notify>>) {
        *self.network_gate.lock().unwrap() = gate;
    }

    fn take_input_error(&self) -> CdpResult<()> {
        match self.input_errors.lock().unwrap().pop_front().flatten() {
            Some(message) => Err(pwright_cdp::connection::CdpError::Protocol {
                code: -32000,
                message,
            }),
            None => Ok(()),
        }
    }

    fn take_network_error(&self) -> CdpResult<()> {
        match self.network_errors.lock().unwrap().pop_front().flatten() {
            Some(message) => Err(pwright_cdp::connection::CdpError::Protocol {
                code: -32000,
                message,
            }),
            None => Ok(()),
        }
    }

    /// Emit the supplied events through `sink` the next time auto-attach is
    /// enabled. This models Chromium re-attaching every existing target after
    /// auto-attach was disabled for an authoritative topology reconstruction.
    pub fn set_auto_attach_events(
        &self,
        sink: tokio::sync::broadcast::Sender<pwright_cdp::CdpEvent>,
        events: impl IntoIterator<Item = pwright_cdp::CdpEvent>,
    ) {
        *self.auto_attach_event_sink.lock().unwrap() = Some(sink);
        *self.auto_attach_events.lock().unwrap() = events.into_iter().collect();
    }

    pub fn set_create_browser_context_error(&self, message: Option<String>) {
        *self.create_browser_context_error.lock().unwrap() = message;
    }

    pub fn pause_create_browser_context(&self) {
        *self.create_browser_context_gate.lock().unwrap() =
            Some(Arc::new(tokio::sync::Notify::new()));
    }

    pub fn resume_create_browser_context(&self) {
        if let Some(gate) = self.create_browser_context_gate.lock().unwrap().take() {
            gate.notify_waiters();
        }
    }

    /// Configure the next and subsequent browser-context disposal calls to
    /// fail with a deterministic CDP protocol error. Passing `None` restores
    /// normal success.
    pub fn set_dispose_browser_context_error(&self, message: Option<String>) {
        *self.dispose_browser_context_error.lock().unwrap() = message;
    }

    /// Pause browser-context disposal after the native method is recorded but
    /// before its response completes. Tests must call
    /// `resume_dispose_browser_context` after observing the pending future.
    pub fn pause_dispose_browser_context(&self) {
        *self.dispose_browser_context_gate.lock().unwrap() =
            Some(Arc::new(tokio::sync::Notify::new()));
    }

    pub fn resume_dispose_browser_context(&self) {
        if let Some(gate) = self.dispose_browser_context_gate.lock().unwrap().take() {
            gate.notify_waiters();
        }
    }

    pub fn set_target_close_error(&self, message: Option<String>) {
        *self.target_close_error.lock().unwrap() = message;
    }

    pub fn pause_target_create(&self) {
        *self.target_create_gate.lock().unwrap() = Some(Arc::new(tokio::sync::Notify::new()));
    }

    pub fn resume_target_create(&self) {
        if let Some(gate) = self.target_create_gate.lock().unwrap().take() {
            gate.notify_waiters();
        }
    }

    pub fn pause_target_close(&self) {
        *self.target_close_gate.lock().unwrap() = Some(Arc::new(tokio::sync::Notify::new()));
    }

    pub fn resume_target_close(&self) {
        if let Some(gate) = self.target_close_gate.lock().unwrap().take() {
            gate.notify_waiters();
        }
    }

    pub fn set_target_responses(&self, responses: impl IntoIterator<Item = Vec<TargetInfo>>) {
        *self.target_responses.lock().unwrap() = responses.into_iter().collect();
    }

    /// Inject a CDP event for testing network listeners.
    pub fn send_event(&self, event: pwright_cdp::CdpEvent) {
        let _ = self.event_tx.send(event);
    }

    /// Queue event batches emitted immediately before successive
    /// `Page.enable` responses. Empty batches can reserve earlier calls.
    pub fn set_page_enable_events(
        &self,
        batches: impl IntoIterator<Item = Vec<pwright_cdp::CdpEvent>>,
    ) {
        *self.page_enable_events.lock().unwrap() = batches.into_iter().collect();
    }

    /// Get recorded method calls (method, args).
    pub fn calls(&self) -> Vec<RecordedCdpCall> {
        self.calls.calls()
    }

    /// Get recorded method names.
    pub fn method_names(&self) -> Vec<String> {
        self.calls.method_names()
    }

    fn record(&self, method: &str, args: Vec<Value>) {
        self.calls.record(method, args);
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

    async fn scripted_runtime_result(
        &self,
        gate: &Mutex<Option<Arc<tokio::sync::Notify>>>,
        default: Value,
    ) -> CdpResult<Value> {
        let gate = gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.notified().await;
        }
        match self.evaluate.resolve() {
            Ok(Some(response)) => Ok(response),
            Ok(None) => Ok(default),
            Err(message) => Err(pwright_cdp::connection::CdpError::Protocol {
                code: -32000,
                message,
            }),
        }
    }
}

#[async_trait]
impl CdpClient for FakeCdpClient {
    fn event_session_id(&self) -> Option<&str> {
        self.event_session_id.get().map(String::as_str)
    }

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
        Ok(self
            .navigate_response
            .get()
            .unwrap_or_else(|| serde_json::json!({"frameId": "fake-frame"})))
    }

    async fn page_navigate_with_referrer(
        &self,
        url: &str,
        referrer: Option<&str>,
    ) -> CdpResult<Value> {
        self.record(
            "Page.navigate",
            vec![serde_json::json!({"url": url, "referrer": referrer})],
        );
        Ok(self
            .navigate_response
            .get()
            .unwrap_or_else(|| serde_json::json!({"frameId": "fake-frame"})))
    }

    async fn page_stop_loading(&self) -> CdpResult<()> {
        self.record("Page.stopLoading", vec![]);
        Ok(())
    }

    async fn page_navigate_in_frame(
        &self,
        frame_id: &str,
        url: &str,
        referrer: Option<&str>,
    ) -> CdpResult<Value> {
        self.record(
            "Page.navigate",
            vec![serde_json::json!({"frameId": frame_id,
            "url": url, "referrer": referrer, "referrerPolicy": "unsafeUrl"})],
        );
        let gate = self.frame_navigate_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.notified().await;
        }
        Ok(self.navigate_response.get().unwrap_or_else(
            || serde_json::json!({"frameId": frame_id, "loaderId": "frame-loader"}),
        ))
    }

    async fn page_enable(&self) -> CdpResult<()> {
        self.record("Page.enable", vec![]);
        let gate = self.page_enable_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.notified().await;
        }
        if let Some(events) = self.page_enable_events.lock().unwrap().pop_front() {
            for event in events {
                let _ = self.event_tx.send(event);
            }
        }
        Ok(())
    }

    async fn page_set_lifecycle_events_enabled(&self, enabled: bool) -> CdpResult<()> {
        self.record("Page.setLifecycleEventsEnabled", vec![Value::Bool(enabled)]);
        Ok(())
    }

    async fn page_capture_screenshot(
        &self,
        params: pwright_cdp::generated::page::CaptureScreenshotParams,
    ) -> CdpResult<String> {
        self.record(
            "Page.captureScreenshot",
            vec![serde_json::to_value(params).expect("screenshot params should serialize")],
        );
        let gate = self.screenshot_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.notified().await;
        }
        Ok(self.screenshot_response.lock().unwrap().clone())
    }

    async fn page_get_layout_metrics(
        &self,
    ) -> CdpResult<pwright_cdp::generated::page::GetLayoutMetricsReturns> {
        self.record("Page.getLayoutMetrics", vec![]);
        serde_json::from_value(serde_json::json!({
            "layoutViewport": {"pageX": 0, "pageY": 0, "clientWidth": 800, "clientHeight": 600},
            "visualViewport": {"offsetX": 0, "offsetY": 0, "pageX": 0, "pageY": 0, "clientWidth": 800, "clientHeight": 600, "scale": 1},
            "contentSize": {"x": 0, "y": 0, "width": 800, "height": 1200},
            "cssLayoutViewport": {"pageX": 0, "pageY": 0, "clientWidth": 800, "clientHeight": 600},
            "cssVisualViewport": {"offsetX": 0, "offsetY": 0, "pageX": 0, "pageY": 0, "clientWidth": 800, "clientHeight": 600, "scale": 1},
            "cssContentSize": {"x": 0, "y": 0, "width": 800, "height": 1200}
        }))
        .map_err(Into::into)
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

    async fn page_add_script_on_new_document_in_world(
        &self,
        source: &str,
        world_name: &str,
    ) -> CdpResult<String> {
        self.record(
            "Page.addScriptToEvaluateOnNewDocument",
            vec![serde_json::json!({"source": source, "worldName": world_name})],
        );
        Ok("world-script-1".to_string())
    }

    async fn page_create_isolated_world(
        &self,
        frame_id: &str,
        world_name: &str,
        grant_universal_access: bool,
    ) -> CdpResult<i64> {
        self.record(
            "Page.createIsolatedWorld",
            vec![serde_json::json!({
                "frameId": frame_id,
                "worldName": world_name,
                "grantUniveralAccess": grant_universal_access,
            })],
        );
        let gate = self.create_isolated_world_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.notified().await;
        }
        if let Some(Some(message)) = self
            .create_isolated_world_errors
            .lock()
            .unwrap()
            .pop_front()
        {
            return Err(pwright_cdp::connection::CdpError::Protocol {
                code: -32000,
                message,
            });
        }
        Ok(17)
    }

    async fn page_reload(&self) -> CdpResult<()> {
        self.record("Page.reload", vec![]);
        for event in self.reload_events.lock().unwrap().drain(..) {
            let _ = self.event_tx.send(event);
        }
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

    async fn page_handle_java_script_dialog(
        &self,
        accept: bool,
        prompt_text: Option<&str>,
    ) -> CdpResult<()> {
        self.record(
            "Page.handleJavaScriptDialog",
            vec![serde_json::json!({
                "accept": accept,
                "promptText": prompt_text,
            })],
        );
        Ok(())
    }

    async fn page_close(&self) -> CdpResult<()> {
        self.record("Page.close", vec![]);
        Ok(())
    }

    async fn page_get_main_frame_id(&self) -> CdpResult<String> {
        self.record("Page.getFrameTree", vec![]);
        Ok(self.main_frame_id.lock().unwrap().clone())
    }

    async fn page_get_frame_tree(&self) -> CdpResult<cdp_page::FrameTree> {
        self.record("Page.getFrameTree", vec![]);
        if let Some(frame_tree) = self.frame_tree_responses.lock().unwrap().pop_front() {
            return Ok(frame_tree);
        }
        if let Some(frame_tree) = self.frame_tree_response.lock().unwrap().clone() {
            return Ok(frame_tree);
        }
        Ok(fake_frame_tree(&self.main_frame_id.lock().unwrap()))
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

    async fn dom_get_box_model_by_backend_node(&self, backend_node_id: i64) -> CdpResult<Value> {
        self.record(
            "DOM.getBoxModel",
            vec![serde_json::json!({"backendNodeId": backend_node_id})],
        );
        self.backend_box_model_responses
            .lock()
            .unwrap()
            .get(&backend_node_id)
            .cloned()
            .ok_or_else(|| {
                pwright_cdp::connection::CdpError::Other(format!(
                    "No box model configured for backend node {backend_node_id}"
                ))
            })
    }

    async fn dom_get_frame_owner(&self, frame_id: &str) -> CdpResult<i64> {
        self.record(
            "DOM.getFrameOwner",
            vec![serde_json::json!({"frameId": frame_id})],
        );
        self.frame_owner_responses
            .lock()
            .unwrap()
            .get(frame_id)
            .copied()
            .ok_or_else(|| {
                pwright_cdp::connection::CdpError::Other(format!(
                    "No frame owner configured for frame {frame_id}"
                ))
            })
    }

    async fn dom_resolve_node(&self, node_id: i64) -> CdpResult<Value> {
        self.record(
            "DOM.resolveNode",
            vec![serde_json::json!({"nodeId": node_id})],
        );
        let obj_id = self.alloc_object_id(node_id);
        Ok(serde_json::json!({"object": {"objectId": obj_id}}))
    }

    async fn dom_describe_node_by_object_id(&self, object_id: &str) -> CdpResult<i64> {
        self.record(
            "DOM.describeNode",
            vec![serde_json::json!({"objectId": object_id})],
        );
        self.object_backend_nodes
            .lock()
            .unwrap()
            .get(object_id)
            .copied()
            .ok_or_else(|| {
                pwright_cdp::connection::CdpError::Other(format!(
                    "No backend node configured for object {object_id}"
                ))
            })
    }

    async fn dom_describe_node_by_object_id_raw(&self, object_id: &str) -> CdpResult<Value> {
        self.record(
            "DOM.describeNode",
            vec![serde_json::json!({"objectId": object_id})],
        );
        let gate = self.describe_node_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.notified().await;
        }
        if let Some(message) = self.describe_node_error.lock().unwrap().clone() {
            return Err(pwright_cdp::connection::CdpError::Protocol {
                code: -32000,
                message,
            });
        }
        self.object_node_descriptions
            .lock()
            .unwrap()
            .get(object_id)
            .cloned()
            .ok_or_else(|| {
                pwright_cdp::connection::CdpError::Other(format!(
                    "No raw node description configured for object {object_id}"
                ))
            })
    }

    async fn dom_resolve_backend_node_in_execution_context(
        &self,
        backend_node_id: i64,
        execution_context_id: i64,
    ) -> CdpResult<Value> {
        self.resolve_node_for_adoption(backend_node_id, execution_context_id, None)
            .await
    }

    async fn dom_resolve_backend_node_in_execution_context_group(
        &self,
        backend_node_id: i64,
        execution_context_id: i64,
        object_group: &str,
    ) -> CdpResult<Value> {
        self.resolve_node_for_adoption(backend_node_id, execution_context_id, Some(object_group))
            .await
    }

    async fn dom_resolve_node_in_group_to_settlement(
        &self,
        backend_node_id: i64,
        execution_context_id: i64,
        object_group: &str,
    ) -> CdpResult<Value> {
        // The adoption gate records before blocking and retains the raw fixture;
        // it does not synthesize success or parse away malformed native owners.
        self.resolve_node_for_adoption(backend_node_id, execution_context_id, Some(object_group))
            .await
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

    async fn dom_set_file_input_files_by_object_id(
        &self,
        object_id: &str,
        files: &[String],
    ) -> CdpResult<()> {
        self.record(
            "DOM.setFileInputFiles",
            vec![serde_json::json!({
                "objectId": object_id,
                "files": files,
            })],
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

    async fn input_dispatch_pointer_event(
        &self,
        params: pwright_cdp::generated::input::DispatchMouseEventParams,
    ) -> CdpResult<()> {
        self.record(
            "Input.dispatchMouseEvent",
            vec![serde_json::to_value(params)?],
        );
        let result = self.take_input_error();
        let gate = self.pointer_gates.lock().unwrap().pop_front().flatten();
        if let Some(gate) = gate {
            gate.notified().await;
        }
        result
    }

    async fn input_dispatch_mouse_event(
        &self,
        event_type: MouseEventType,
        x: f64,
        y: f64,
        button: Option<MouseButton>,
        click_count: Option<i32>,
        buttons: Option<i32>,
        modifiers: Option<i32>,
    ) -> CdpResult<()> {
        self.record(
            "Input.dispatchMouseEvent",
            vec![serde_json::json!({
                "type": event_type.as_str(), "x": x, "y": y,
                "button": button.map(|b| b.as_str()), "clickCount": click_count, "buttons": buttons,
                "modifiers": modifiers,
            })],
        );
        self.take_input_error()
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
        self.take_input_error()
    }

    async fn input_dispatch_key_event_with_modifiers(
        &self,
        event_type: KeyEventType,
        key: &str,
        code: &str,
        windows_virtual_key_code: Option<i64>,
        modifiers: Option<i32>,
    ) -> CdpResult<()> {
        self.record(
            "Input.dispatchKeyEvent",
            vec![serde_json::json!({
                "type": event_type.as_str(), "key": key, "code": code,
                "windowsVirtualKeyCode": windows_virtual_key_code,
                "modifiers": modifiers,
            })],
        );
        self.take_input_error()
    }

    async fn input_dispatch_keyboard_event(
        &self,
        params: KeyboardEventParams<'_>,
    ) -> CdpResult<()> {
        self.record(
            "Input.dispatchKeyEvent",
            vec![serde_json::json!({
                "type": params.event_type.as_str(),
                "modifiers": params.modifiers,
                "key": params.key,
                "code": params.code,
                "windowsVirtualKeyCode": params.windows_virtual_key_code,
                "text": params.text,
                "unmodifiedText": params.unmodified_text,
                "autoRepeat": params.auto_repeat,
                "location": params.location,
                "isKeypad": params.is_keypad,
                "commands": params.commands,
            })],
        );
        self.take_input_error()
    }

    async fn input_dispatch_text_key_event(
        &self,
        key: &str,
        code: &str,
        windows_virtual_key_code: Option<i64>,
        text: &str,
    ) -> CdpResult<()> {
        self.record(
            "Input.dispatchKeyEvent",
            vec![serde_json::json!({
                "type": "keyDown", "key": key, "code": code,
                "windowsVirtualKeyCode": windows_virtual_key_code,
                "text": text, "unmodifiedText": text,
            })],
        );
        self.take_input_error()
    }

    async fn input_insert_text(&self, text: &str) -> CdpResult<()> {
        self.record("Input.insertText", vec![Value::String(text.to_string())]);
        self.take_input_error()
    }

    async fn input_dispatch_touch_event(
        &self,
        event_type: TouchEventType,
        x: f64,
        y: f64,
    ) -> CdpResult<()> {
        self.input_dispatch_touch_event_with_modifiers(event_type, x, y, None)
            .await
    }

    async fn input_dispatch_touch_event_with_modifiers(
        &self,
        event_type: TouchEventType,
        x: f64,
        y: f64,
        modifiers: Option<i32>,
    ) -> CdpResult<()> {
        let touch_points = match event_type {
            TouchEventType::Start => serde_json::json!([{ "x": x, "y": y }]),
            TouchEventType::End => serde_json::json!([]),
        };
        self.record(
            "Input.dispatchTouchEvent",
            vec![serde_json::json!({
                "type": event_type.as_str(),
                "touchPoints": touch_points,
                "modifiers": modifiers,
            })],
        );
        let result = self.take_input_error();
        let gate = self.pointer_gates.lock().unwrap().pop_front().flatten();
        if let Some(gate) = gate {
            gate.notified().await;
        }
        result
    }

    // ── Runtime domain (hybrid: real for simple, configurable for complex) ──

    async fn runtime_ensure_binding(&self, name: &'static str) -> CdpResult<()> {
        self.record(
            "Runtime.addBinding",
            vec![serde_json::json!({ "name": name })],
        );
        Ok(())
    }

    async fn runtime_evaluate(&self, expression: &str) -> CdpResult<Value> {
        self.record(
            "Runtime.evaluate",
            vec![Value::String(expression.to_string())],
        );
        self.scripted_runtime_result(
            &self.evaluate_gate,
            serde_json::json!({"result": {"value": ""}}),
        )
        .await
    }

    async fn runtime_evaluate_in_unique_context(
        &self,
        expression: &str,
        unique_context_id: &str,
    ) -> CdpResult<Value> {
        self.record(
            "Runtime.evaluate",
            vec![serde_json::json!({
                "expression": expression,
                "uniqueContextId": unique_context_id,
            })],
        );
        self.scripted_runtime_result(
            &self.evaluate_gate,
            serde_json::json!({"result": {"value": ""}}),
        )
        .await
    }

    async fn runtime_evaluate_as_object(&self, expression: &str) -> CdpResult<Value> {
        // Same as runtime_evaluate for the fake (no distinction needed)
        self.runtime_evaluate(expression).await
    }

    async fn runtime_evaluate_as_object_to_settlement(&self, expression: &str) -> CdpResult<Value> {
        // The fake has no command timer. Keep the same recorded wire operation
        // and unfiltered response fixtures; transport settlement is tested in CDP.
        self.runtime_evaluate_as_object(expression).await
    }

    async fn runtime_evaluate_with_user_gesture(
        &self,
        expression: &str,
        unique_context_id: Option<&str>,
    ) -> CdpResult<Value> {
        self.record(
            "Runtime.evaluate",
            vec![serde_json::json!({
                "expression": expression, "uniqueContextId": unique_context_id, "userGesture": true,
            })],
        );
        self.scripted_runtime_result(
            &self.evaluate_gate,
            serde_json::json!({"result": {"value": ""}}),
        )
        .await
    }

    async fn runtime_evaluate_as_object_in_unique_context(
        &self,
        expression: &str,
        unique_context_id: &str,
    ) -> CdpResult<Value> {
        self.runtime_evaluate_in_unique_context(expression, unique_context_id)
            .await
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
        if function_declaration.contains("__pwrightChunkedString")
            || function_declaration.contains("const value=this.value.v")
        {
            return self
                .scripted_runtime_result(
                    &self.evaluate_gate,
                    serde_json::json!({"result": {"value": null}}),
                )
                .await;
        }
        // Fallback
        Ok(serde_json::json!({"result": {"value": null}}))
    }

    async fn runtime_call_function_as_object_in_unique_context(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
    ) -> CdpResult<Value> {
        self.record(
            "Runtime.callFunctionOn",
            vec![serde_json::json!({
                "uniqueContextId": unique_context_id,
                "functionDeclaration": function_declaration,
                "arguments": arguments,
                "returnByValue": false,
                "awaitPromise": true,
            })],
        );
        self.scripted_runtime_result(
            &self.evaluate_gate,
            serde_json::json!({"result": {"type": "undefined"}}),
        )
        .await
    }

    async fn runtime_call_function_as_object_in_unique_context_to_settlement(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
    ) -> CdpResult<Value> {
        self.runtime_call_function_as_object_in_unique_context(
            unique_context_id,
            function_declaration,
            arguments,
        )
        .await
    }

    async fn runtime_call_function_as_object_in_group_to_settlement(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
        object_group: &str,
        user_gesture: bool,
    ) -> CdpResult<Value> {
        self.record(
            "Runtime.callFunctionOn",
            vec![serde_json::json!({
                "uniqueContextId": unique_context_id,
                "functionDeclaration": function_declaration,
                "arguments": arguments,
                "returnByValue": false,
                "awaitPromise": false,
                "objectGroup": object_group,
                "userGesture": user_gesture,
            })],
        );
        self.scripted_runtime_result(
            &self.evaluate_gate,
            serde_json::json!({"result": {"type": "undefined"}}),
        )
        .await
    }

    async fn runtime_call_function_as_object_awaited_in_group_to_settlement(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
        object_group: &str,
        user_gesture: bool,
    ) -> CdpResult<Value> {
        self.record(
            "Runtime.callFunctionOn",
            vec![serde_json::json!({
                "uniqueContextId": unique_context_id,
                "functionDeclaration": function_declaration,
                "arguments": arguments,
                "returnByValue": false,
                "awaitPromise": true,
                "objectGroup": object_group,
                "userGesture": user_gesture,
            })],
        );
        self.scripted_runtime_result(
            &self.evaluate_gate,
            serde_json::json!({"result": {"type": "undefined"}}),
        )
        .await
    }

    async fn runtime_call_function_raw_in_group_to_settlement(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
        object_group: &str,
        user_gesture: bool,
    ) -> CdpResult<Value> {
        self.record(
            "Runtime.callFunctionOn",
            vec![serde_json::json!({
                "uniqueContextId": unique_context_id,
                "functionDeclaration": function_declaration,
                "arguments": arguments,
                "returnByValue": true,
                "awaitPromise": true,
                "objectGroup": object_group,
                "userGesture": user_gesture,
            })],
        );
        self.scripted_runtime_result(
            &self.evaluate_gate,
            serde_json::json!({"result": {"type": "undefined"}}),
        )
        .await
    }

    async fn runtime_call_function_by_value_in_unique_context(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
    ) -> CdpResult<Value> {
        self.record(
            "Runtime.callFunctionOn",
            vec![serde_json::json!({
                "uniqueContextId": unique_context_id,
                "functionDeclaration": function_declaration,
                "arguments": arguments,
                "returnByValue": true,
                "awaitPromise": true,
            })],
        );
        self.scripted_runtime_result(
            &self.evaluate_gate,
            serde_json::json!({"result": {"value": {"ok": true, "value": {"t": "u"}}}}),
        )
        .await
    }

    async fn runtime_call_function_by_value_in_group_to_settlement(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
        object_group: &str,
    ) -> CdpResult<Value> {
        self.record(
            "Runtime.callFunctionOn",
            vec![serde_json::json!({
                "uniqueContextId": unique_context_id,
                "functionDeclaration": function_declaration,
                "arguments": arguments,
                "returnByValue": true,
                "awaitPromise": true,
                "objectGroup": object_group,
            })],
        );
        self.scripted_runtime_result(
            &self.evaluate_gate,
            serde_json::json!({"result": {"value": {"ok": true, "value": {"t": "u"}}}}),
        )
        .await
    }

    async fn runtime_call_function_by_value_in_large_group_to_settlement(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
        object_group: &str,
    ) -> CdpResult<Value> {
        self.runtime_call_function_by_value_in_group_to_settlement(
            unique_context_id,
            function_declaration,
            arguments,
            object_group,
        )
        .await
    }

    async fn runtime_call_function_by_value_in_large_group_with_user_gesture_to_settlement(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
        object_group: &str,
        user_gesture: bool,
    ) -> CdpResult<Value> {
        self.record(
            "Runtime.callFunctionOn",
            vec![serde_json::json!({
                "uniqueContextId": unique_context_id,
                "functionDeclaration": function_declaration,
                "arguments": arguments,
                "returnByValue": true,
                "awaitPromise": true,
                "objectGroup": object_group,
                "userGesture": user_gesture,
            })],
        );
        self.scripted_runtime_result(&self.evaluate_gate, serde_json::json!({"result": {}}))
            .await
    }

    async fn runtime_get_properties(&self, object_id: &str) -> CdpResult<Value> {
        self.record(
            "Runtime.getProperties",
            vec![Value::String(object_id.to_owned())],
        );
        self.scripted_runtime_result(&self.get_properties_gate, serde_json::json!({"result": []}))
            .await
    }

    async fn runtime_get_properties_to_settlement(&self, object_id: &str) -> CdpResult<Value> {
        self.runtime_get_properties(object_id).await
    }

    async fn runtime_release_object(&self, object_id: &str) -> CdpResult<()> {
        self.record(
            "Runtime.releaseObject",
            vec![Value::String(object_id.to_string())],
        );
        let gate = self.release_object_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.notified().await;
        }
        if let Some(message) = self.release_object_error.lock().unwrap().clone() {
            return Err(pwright_cdp::connection::CdpError::Protocol {
                code: -32000,
                message,
            });
        }
        Ok(())
    }

    async fn runtime_release_object_to_settlement(&self, object_id: &str) -> CdpResult<()> {
        self.runtime_release_object(object_id).await
    }

    async fn runtime_query_object_in_group(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        object_group: &str,
    ) -> CdpResult<Value> {
        self.record(
            "Runtime.callFunctionOn",
            vec![serde_json::json!({
                "uniqueContextId": unique_context_id, "functionDeclaration": function_declaration,
                "objectGroup": object_group, "returnByValue": false, "awaitPromise": false,
            })],
        );
        self.scripted_runtime_result(
            &self.evaluate_gate,
            serde_json::json!({"result": {"type": "number", "value": 0}}),
        )
        .await
    }

    async fn runtime_release_object_group(&self, object_group: &str) -> CdpResult<()> {
        self.record(
            "Runtime.releaseObjectGroup",
            vec![Value::String(object_group.to_owned())],
        );
        let gate = self.release_object_group_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.notified().await;
        }
        if let Some(message) = self.release_object_error.lock().unwrap().clone() {
            return Err(pwright_cdp::connection::CdpError::Protocol {
                code: -32000,
                message,
            });
        }
        Ok(())
    }

    async fn runtime_release_object_group_to_settlement(
        &self,
        object_group: &str,
    ) -> CdpResult<()> {
        self.runtime_release_object_group(object_group).await
    }

    async fn runtime_enable(&self) -> CdpResult<()> {
        self.record("Runtime.enable", vec![]);
        Ok(())
    }

    async fn runtime_run_if_waiting_for_debugger(&self) -> CdpResult<()> {
        self.record("Runtime.runIfWaitingForDebugger", vec![]);
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
        let gate = self.network_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.notified().await;
        }
        self.take_network_error()
    }

    async fn network_disable(&self) -> CdpResult<()> {
        self.record("Network.disable", vec![]);
        let gate = self.network_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.notified().await;
        }
        self.take_network_error()
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

    async fn fetch_enable_response_stage(&self) -> CdpResult<()> {
        self.record("Fetch.enableResponseStage", vec![]);
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

    async fn fetch_get_response_body(&self, request_id: &str) -> CdpResult<ResponseBody> {
        self.record(
            "Fetch.getResponseBody",
            vec![Value::String(request_id.to_string())],
        );
        Ok(ResponseBody {
            body: String::new(),
            base64_encoded: false,
        })
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

    async fn target_create_browser_context(
        &self,
        proxy_server: Option<&str>,
        proxy_bypass_list: Option<&str>,
    ) -> CdpResult<String> {
        self.record(
            "Target.createBrowserContext",
            vec![serde_json::json!({
                "disposeOnDetach": true,
                "proxyServer": proxy_server,
                "proxyBypassList": proxy_bypass_list,
            })],
        );
        let gate = self.create_browser_context_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.notified().await;
        }
        if let Some(message) = self.create_browser_context_error.lock().unwrap().clone() {
            return Err(pwright_cdp::connection::CdpError::Protocol {
                code: -32000,
                message,
            });
        }
        Ok(self.create_browser_context_response.lock().unwrap().clone())
    }

    async fn target_create(&self, url: &str) -> CdpResult<String> {
        self.record("Target.createTarget", vec![Value::String(url.to_string())]);
        let gate = self.target_create_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.notified().await;
        }
        Ok("fake-target-1".to_string())
    }

    async fn target_create_in_context(
        &self,
        url: &str,
        browser_context_id: &str,
    ) -> CdpResult<String> {
        self.record(
            "Target.createTarget",
            vec![serde_json::json!({
                "url": url,
                "browserContextId": browser_context_id,
            })],
        );
        let gate = self.target_create_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.notified().await;
        }
        Ok("fake-target-1".to_string())
    }

    async fn target_dispose_browser_context(&self, browser_context_id: &str) -> CdpResult<()> {
        self.record(
            "Target.disposeBrowserContext",
            vec![Value::String(browser_context_id.to_string())],
        );
        let gate = self.dispose_browser_context_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.notified().await;
        }
        if let Some(message) = self.dispose_browser_context_error.lock().unwrap().clone() {
            return Err(pwright_cdp::connection::CdpError::Protocol {
                code: -32000,
                message,
            });
        }
        Ok(())
    }

    async fn target_close(&self, target_id: &str) -> CdpResult<()> {
        self.record(
            "Target.closeTarget",
            vec![Value::String(target_id.to_string())],
        );
        let gate = self.target_close_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.notified().await;
        }
        if let Some(message) = self.target_close_error.lock().unwrap().clone() {
            return Err(pwright_cdp::connection::CdpError::Protocol {
                code: -32000,
                message,
            });
        }
        Ok(())
    }

    async fn target_get_targets(&self) -> CdpResult<Vec<TargetInfo>> {
        self.record("Target.getTargets", vec![]);
        Ok(self
            .target_responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_default())
    }

    async fn target_get_browser_contexts(&self) -> CdpResult<Vec<String>> {
        self.record("Target.getBrowserContexts", vec![]);
        Ok(self.browser_contexts_response.lock().unwrap().clone())
    }

    async fn target_get_target_info(&self) -> CdpResult<cdp_target::TargetInfo> {
        self.record("Target.getTargetInfo", vec![]);
        Ok(fake_target_info("fake-browser", "browser", None))
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
        self.finish_target_detach().await
    }

    async fn target_detach_in_parent(
        &self,
        parent_session_id: Option<&str>,
        session_id: &str,
    ) -> CdpResult<()> {
        if parent_session_id.is_none() {
            return self.target_detach(session_id).await;
        }
        // Record the explicit port arguments, not fabricated wire parameters.
        self.record(
            "Target.detachFromTargetInParent",
            vec![
                Value::String(parent_session_id.expect("checked parent").to_owned()),
                Value::String(session_id.to_owned()),
            ],
        );
        self.finish_target_detach().await
    }

    async fn target_set_auto_attach(
        &self,
        auto_attach: bool,
        wait_for_debugger_on_start: bool,
        flatten: bool,
    ) -> CdpResult<()> {
        self.record(
            "Target.setAutoAttach",
            vec![serde_json::json!({
                "autoAttach": auto_attach,
                "waitForDebuggerOnStart": wait_for_debugger_on_start,
                "flatten": flatten,
            })],
        );
        if auto_attach {
            let sink = self.auto_attach_event_sink.lock().unwrap().take();
            let events = std::mem::take(&mut *self.auto_attach_events.lock().unwrap());
            if let Some(sink) = sink {
                for event in events {
                    let _ = sink.send(event);
                }
            }
        }
        Ok(())
    }

    async fn target_set_auto_attach_types(
        &self,
        auto_attach: bool,
        wait_for_debugger_on_start: bool,
        flatten: bool,
        included_target_types: &[&str],
    ) -> CdpResult<()> {
        self.record(
            "Target.setAutoAttach",
            vec![serde_json::json!({
                "autoAttach": auto_attach,
                "waitForDebuggerOnStart": wait_for_debugger_on_start,
                "flatten": flatten,
                "includedTargetTypes": included_target_types,
            })],
        );
        if auto_attach {
            let sink = self.auto_attach_event_sink.lock().unwrap().take();
            let events = std::mem::take(&mut *self.auto_attach_events.lock().unwrap());
            if let Some(sink) = sink {
                for event in events {
                    let _ = sink.send(event);
                }
            }
        }
        Ok(())
    }
}

fn fake_frame_tree(frame_id: &str) -> cdp_page::FrameTree {
    serde_json::from_value(serde_json::json!({
        "frame": {
            "id": frame_id,
            "loaderId": "fake-loader",
            "url": "about:blank",
            "domainAndRegistry": "",
            "securityOrigin": "://",
            "mimeType": "text/html",
            "secureContextType": "InsecureScheme",
            "crossOriginIsolatedContextType": "NotIsolated",
            "gatedAPIFeatures": []
        }
    }))
    .expect("built-in fake Page.frameTree must match generated CDP types")
}

fn fake_target_info(
    target_id: &str,
    target_type: &str,
    browser_context_id: Option<&str>,
) -> cdp_target::TargetInfo {
    serde_json::from_value(serde_json::json!({
        "targetId": target_id,
        "type": target_type,
        "title": "",
        "url": "",
        "attached": true,
        "canAccessOpener": false,
        "browserContextId": browser_context_id
    }))
    .expect("built-in fake Target.targetInfo must match generated CDP types")
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
