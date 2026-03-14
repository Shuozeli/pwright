//! CdpClient trait — the dependency injection boundary for all CDP operations.
//!
//! All bridge functions accept `&dyn CdpClient` instead of `&CdpSession`,
//! enabling unit testing with `MockCdpClient`.

use async_trait::async_trait;
use serde_json::Value;

use crate::connection::Result;
use crate::domains::accessibility::RawAXNode;
use crate::domains::network::{Cookie, ResponseBody};
use crate::domains::target::TargetInfo;

/// Trait abstracting all CDP domain operations.
///
/// `CdpSession` implements this trait for real Chrome interaction.
/// Test code can use `MockCdpClient` to verify call sequences.
#[async_trait]
pub trait CdpClient: Send + Sync {
    // ── Browser domain ──
    async fn browser_set_download_behavior(
        &self,
        behavior: &str,
        download_path: Option<&str>,
        events_enabled: bool,
    ) -> Result<()>;

    // ── Page domain ──
    async fn page_navigate(&self, url: &str) -> Result<Value>;
    async fn page_enable(&self) -> Result<()>;
    async fn page_capture_screenshot(
        &self,
        format: &str,
        quality: Option<i32>,
        full_page: bool,
    ) -> Result<String>;
    async fn page_print_to_pdf(&self, params: Value) -> Result<String>;
    async fn page_add_script_on_new_document(&self, source: &str) -> Result<String>;
    async fn page_reload(&self) -> Result<()>;
    async fn page_get_navigation_history(&self) -> Result<Value>;
    async fn page_navigate_to_history_entry(&self, entry_id: i64) -> Result<()>;
    async fn page_bring_to_front(&self) -> Result<()>;
    async fn page_set_document_content(&self, frame_id: &str, html: &str) -> Result<()>;

    // ── DOM domain ──
    async fn dom_focus(&self, node_id: i64) -> Result<()>;
    async fn dom_scroll_into_view(&self, node_id: i64) -> Result<()>;
    async fn dom_get_box_model(&self, node_id: i64) -> Result<Value>;
    async fn dom_resolve_node(&self, node_id: i64) -> Result<Value>;
    async fn dom_enable(&self) -> Result<()>;
    async fn dom_get_document(&self) -> Result<Value>;
    async fn dom_query_selector(&self, node_id: i64, selector: &str) -> Result<i64>;
    async fn dom_query_selector_all(&self, node_id: i64, selector: &str) -> Result<Vec<i64>>;
    async fn dom_get_attributes(&self, node_id: i64) -> Result<Vec<String>>;
    async fn dom_get_outer_html(&self, node_id: i64) -> Result<String>;
    async fn dom_describe_node(&self, backend_node_id: i64) -> Result<Value>;
    async fn dom_set_file_input_files(&self, node_id: i64, files: &[String]) -> Result<()>;
    async fn dom_request_node(&self, object_id: &str) -> Result<i64>;

    // ── Input domain ──
    async fn input_dispatch_mouse_event(
        &self,
        event_type: &str,
        x: f64,
        y: f64,
        button: Option<&str>,
        click_count: Option<i32>,
        buttons: Option<i32>,
    ) -> Result<()>;
    async fn input_dispatch_key_event(
        &self,
        event_type: &str,
        key: &str,
        code: &str,
        windows_virtual_key_code: Option<i64>,
    ) -> Result<()>;
    async fn input_insert_text(&self, text: &str) -> Result<()>;
    async fn input_dispatch_touch_event(&self, event_type: &str, x: f64, y: f64) -> Result<()>;

    // ── Runtime domain ──
    async fn runtime_evaluate(&self, expression: &str) -> Result<Value>;
    async fn runtime_evaluate_as_object(&self, expression: &str) -> Result<Value>;
    async fn runtime_call_function_on(
        &self,
        object_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
    ) -> Result<Value>;
    async fn runtime_evaluate_async(&self, expression: &str) -> Result<Value>;
    async fn runtime_enable(&self) -> Result<()>;

    // ── Accessibility domain ──
    async fn accessibility_enable(&self) -> Result<()>;
    async fn accessibility_get_full_tree(&self) -> Result<Vec<RawAXNode>>;

    // ── Network domain ──
    async fn network_enable(&self) -> Result<()>;
    async fn network_set_blocked_urls(&self, patterns: &[String]) -> Result<()>;
    async fn network_get_cookies(&self) -> Result<Vec<Cookie>>;
    async fn network_set_cookies(&self, cookies: Vec<Value>) -> Result<()>;
    async fn network_get_response_body(&self, request_id: &str) -> Result<ResponseBody>;

    // ── Fetch domain ──
    async fn fetch_enable(&self) -> Result<()>;
    async fn fetch_disable(&self) -> Result<()>;
    async fn fetch_continue_request(&self, request_id: &str) -> Result<()>;
    async fn fetch_fail_request(&self, request_id: &str, reason: &str) -> Result<()>;

    // ── Target domain ──
    async fn target_create(&self, url: &str) -> Result<String>;
    async fn target_close(&self, target_id: &str) -> Result<()>;
    async fn target_get_targets(&self) -> Result<Vec<TargetInfo>>;
    async fn target_attach(&self, target_id: &str) -> Result<String>;
    async fn target_detach(&self, session_id: &str) -> Result<()>;

    // ── Events ──
    fn subscribe_events(&self) -> tokio::sync::broadcast::Receiver<CdpEvent>;
}

// ── CdpSession implements CdpClient ──
// The existing impl methods on CdpSession already have the right signatures,
// so we just delegate.

use crate::events::CdpEvent;
use crate::session::CdpSession;

#[async_trait]
impl CdpClient for CdpSession {
    fn subscribe_events(&self) -> tokio::sync::broadcast::Receiver<CdpEvent> {
        self.connection().subscribe_events()
    }

    async fn browser_set_download_behavior(
        &self,
        behavior: &str,
        download_path: Option<&str>,
        events_enabled: bool,
    ) -> Result<()> {
        CdpSession::browser_set_download_behavior(self, behavior, download_path, events_enabled)
            .await
    }

    async fn page_navigate(&self, url: &str) -> Result<Value> {
        CdpSession::page_navigate(self, url).await
    }
    async fn page_enable(&self) -> Result<()> {
        CdpSession::page_enable(self).await
    }
    async fn page_capture_screenshot(
        &self,
        format: &str,
        quality: Option<i32>,
        full_page: bool,
    ) -> Result<String> {
        CdpSession::page_capture_screenshot(self, format, quality, full_page).await
    }
    async fn page_print_to_pdf(&self, params: Value) -> Result<String> {
        CdpSession::page_print_to_pdf(self, params).await
    }
    async fn page_add_script_on_new_document(&self, source: &str) -> Result<String> {
        CdpSession::page_add_script_on_new_document(self, source).await
    }
    async fn page_reload(&self) -> Result<()> {
        CdpSession::page_reload(self).await
    }
    async fn page_get_navigation_history(&self) -> Result<Value> {
        CdpSession::page_get_navigation_history(self).await
    }
    async fn page_navigate_to_history_entry(&self, entry_id: i64) -> Result<()> {
        CdpSession::page_navigate_to_history_entry(self, entry_id).await
    }
    async fn page_bring_to_front(&self) -> Result<()> {
        CdpSession::page_bring_to_front(self).await
    }
    async fn page_set_document_content(&self, frame_id: &str, html: &str) -> Result<()> {
        CdpSession::page_set_document_content(self, frame_id, html).await
    }
    async fn dom_focus(&self, node_id: i64) -> Result<()> {
        CdpSession::dom_focus(self, node_id).await
    }
    async fn dom_scroll_into_view(&self, node_id: i64) -> Result<()> {
        CdpSession::dom_scroll_into_view(self, node_id).await
    }
    async fn dom_get_box_model(&self, node_id: i64) -> Result<Value> {
        CdpSession::dom_get_box_model(self, node_id).await
    }
    async fn dom_resolve_node(&self, node_id: i64) -> Result<Value> {
        CdpSession::dom_resolve_node(self, node_id).await
    }
    async fn dom_enable(&self) -> Result<()> {
        CdpSession::dom_enable(self).await
    }
    async fn dom_get_document(&self) -> Result<Value> {
        CdpSession::dom_get_document(self).await
    }
    async fn dom_query_selector(&self, node_id: i64, selector: &str) -> Result<i64> {
        CdpSession::dom_query_selector(self, node_id, selector).await
    }
    async fn dom_query_selector_all(&self, node_id: i64, selector: &str) -> Result<Vec<i64>> {
        CdpSession::dom_query_selector_all(self, node_id, selector).await
    }
    async fn dom_get_attributes(&self, node_id: i64) -> Result<Vec<String>> {
        CdpSession::dom_get_attributes(self, node_id).await
    }
    async fn dom_get_outer_html(&self, node_id: i64) -> Result<String> {
        CdpSession::dom_get_outer_html(self, node_id).await
    }
    async fn dom_describe_node(&self, backend_node_id: i64) -> Result<Value> {
        CdpSession::dom_describe_node(self, backend_node_id).await
    }
    async fn dom_set_file_input_files(&self, node_id: i64, files: &[String]) -> Result<()> {
        CdpSession::dom_set_file_input_files(self, node_id, files).await
    }
    async fn dom_request_node(&self, object_id: &str) -> Result<i64> {
        CdpSession::dom_request_node(self, object_id).await
    }
    async fn input_dispatch_mouse_event(
        &self,
        event_type: &str,
        x: f64,
        y: f64,
        button: Option<&str>,
        click_count: Option<i32>,
        buttons: Option<i32>,
    ) -> Result<()> {
        CdpSession::input_dispatch_mouse_event(self, event_type, x, y, button, click_count, buttons)
            .await
    }
    async fn input_dispatch_key_event(
        &self,
        event_type: &str,
        key: &str,
        code: &str,
        windows_virtual_key_code: Option<i64>,
    ) -> Result<()> {
        CdpSession::input_dispatch_key_event(self, event_type, key, code, windows_virtual_key_code)
            .await
    }
    async fn input_insert_text(&self, text: &str) -> Result<()> {
        CdpSession::input_insert_text(self, text).await
    }
    async fn input_dispatch_touch_event(&self, event_type: &str, x: f64, y: f64) -> Result<()> {
        CdpSession::input_dispatch_touch_event(self, event_type, x, y).await
    }
    async fn runtime_evaluate(&self, expression: &str) -> Result<Value> {
        CdpSession::runtime_evaluate(self, expression).await
    }
    async fn runtime_evaluate_as_object(&self, expression: &str) -> Result<Value> {
        CdpSession::runtime_evaluate_as_object(self, expression).await
    }
    async fn runtime_call_function_on(
        &self,
        object_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
    ) -> Result<Value> {
        CdpSession::runtime_call_function_on(self, object_id, function_declaration, arguments).await
    }
    async fn runtime_evaluate_async(&self, expression: &str) -> Result<Value> {
        CdpSession::runtime_evaluate_async(self, expression).await
    }
    async fn runtime_enable(&self) -> Result<()> {
        CdpSession::runtime_enable(self).await
    }
    async fn accessibility_enable(&self) -> Result<()> {
        CdpSession::accessibility_enable(self).await
    }
    async fn accessibility_get_full_tree(&self) -> Result<Vec<RawAXNode>> {
        CdpSession::accessibility_get_full_tree(self).await
    }
    async fn network_enable(&self) -> Result<()> {
        CdpSession::network_enable(self).await
    }
    async fn network_set_blocked_urls(&self, patterns: &[String]) -> Result<()> {
        CdpSession::network_set_blocked_urls(self, patterns).await
    }
    async fn network_get_cookies(&self) -> Result<Vec<Cookie>> {
        CdpSession::network_get_cookies(self).await
    }
    async fn network_set_cookies(&self, cookies: Vec<Value>) -> Result<()> {
        CdpSession::network_set_cookies(self, cookies).await
    }
    async fn network_get_response_body(&self, request_id: &str) -> Result<ResponseBody> {
        CdpSession::network_get_response_body(self, request_id).await
    }
    async fn fetch_enable(&self) -> Result<()> {
        CdpSession::fetch_enable(self).await
    }
    async fn fetch_disable(&self) -> Result<()> {
        CdpSession::fetch_disable(self).await
    }
    async fn fetch_continue_request(&self, request_id: &str) -> Result<()> {
        CdpSession::fetch_continue_request(self, request_id).await
    }
    async fn fetch_fail_request(&self, request_id: &str, reason: &str) -> Result<()> {
        CdpSession::fetch_fail_request(self, request_id, reason).await
    }
    async fn target_create(&self, url: &str) -> Result<String> {
        CdpSession::target_create(self, url).await
    }
    async fn target_close(&self, target_id: &str) -> Result<()> {
        CdpSession::target_close(self, target_id).await
    }
    async fn target_get_targets(&self) -> Result<Vec<TargetInfo>> {
        CdpSession::target_get_targets(self).await
    }
    async fn target_attach(&self, target_id: &str) -> Result<String> {
        CdpSession::target_attach(self, target_id).await
    }
    async fn target_detach(&self, session_id: &str) -> Result<()> {
        CdpSession::target_detach(self, session_id).await
    }
}
