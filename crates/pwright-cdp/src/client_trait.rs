//! CdpClient trait — the dependency injection boundary for all CDP operations.
//!
//! All bridge functions accept `&dyn CdpClient` instead of `&CdpSession`,
//! enabling unit testing with `MockCdpClient`.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::connection::{CdpEventSubscriptionError, Result};
use crate::domains::accessibility::RawAXNode;
use crate::domains::browser::DownloadBehavior;
use crate::domains::input::{
    KeyEventType, KeyboardEventParams, MouseButton, MouseEventType, TouchEventType,
};
use crate::domains::network::{Cookie, ResponseBody};
use crate::domains::target::TargetInfo;
use crate::generated::{page, target};

/// Factory for creating per-tab CDP sessions from an attached session ID.
///
/// The real impl wraps `CdpConnection`; test fakes return mock clients.
pub trait SessionFactory: Send + Sync {
    fn create_session(&self, session_id: String, target_id: String) -> Arc<dyn CdpClient>;
}

/// Trait abstracting all CDP domain operations.
///
/// `CdpSession` implements this trait for real Chrome interaction.
/// Test code can use `MockCdpClient` to verify call sequences.
#[async_trait]
pub trait CdpClient: Send + Sync {
    // ── Browser domain ──
    async fn browser_set_download_behavior(
        &self,
        behavior: DownloadBehavior,
        download_path: Option<&str>,
        events_enabled: bool,
    ) -> Result<()>;

    // ── Page domain ──
    async fn page_navigate(&self, url: &str) -> Result<Value>;
    async fn page_navigate_with_referrer(&self, url: &str, referrer: Option<&str>)
    -> Result<Value>;
    async fn page_stop_loading(&self) -> Result<()>;
    async fn page_enable(&self) -> Result<()>;
    async fn page_set_lifecycle_events_enabled(&self, enabled: bool) -> Result<()>;
    async fn page_capture_screenshot(
        &self,
        params: page::CaptureScreenshotParams,
    ) -> Result<String>;
    async fn page_get_layout_metrics(&self) -> Result<page::GetLayoutMetricsReturns>;
    async fn page_print_to_pdf(&self, params: Value) -> Result<String>;
    async fn page_add_script_on_new_document(&self, source: &str) -> Result<String>;
    async fn page_add_script_on_new_document_in_world(
        &self,
        source: &str,
        world_name: &str,
    ) -> Result<String> {
        let _ = (source, world_name);
        Err(crate::connection::CdpError::Other(
            "Page.addScriptToEvaluateOnNewDocument with a world is not implemented by this CDP client"
                .to_string(),
        ))
    }
    async fn page_create_isolated_world(
        &self,
        frame_id: &str,
        world_name: &str,
        grant_universal_access: bool,
    ) -> Result<i64> {
        let _ = (frame_id, world_name, grant_universal_access);
        Err(crate::connection::CdpError::Other(
            "Page.createIsolatedWorld is not implemented by this CDP client".to_string(),
        ))
    }
    async fn page_reload(&self) -> Result<()>;
    async fn page_get_navigation_history(&self) -> Result<Value>;
    async fn page_navigate_to_history_entry(&self, entry_id: i64) -> Result<()>;
    async fn page_bring_to_front(&self) -> Result<()>;
    async fn page_set_document_content(&self, frame_id: &str, html: &str) -> Result<()>;
    async fn page_close(&self) -> Result<()>;
    async fn page_get_main_frame_id(&self) -> Result<String> {
        Err(crate::connection::CdpError::Other(
            "Page.getFrameTree is not implemented by this CDP client".to_string(),
        ))
    }
    async fn page_get_frame_tree(&self) -> Result<page::FrameTree> {
        Err(crate::connection::CdpError::Other(
            "Page.getFrameTree is not implemented by this CDP client".to_string(),
        ))
    }

    // ── DOM domain ──
    async fn dom_focus(&self, node_id: i64) -> Result<()>;
    async fn dom_scroll_into_view(&self, node_id: i64) -> Result<()>;
    async fn dom_get_box_model(&self, node_id: i64) -> Result<Value>;
    async fn dom_get_box_model_by_backend_node(&self, backend_node_id: i64) -> Result<Value> {
        let _ = backend_node_id;
        Err(crate::connection::CdpError::Other(
            "DOM.getBoxModel by backend node is not implemented by this CDP client".to_string(),
        ))
    }
    async fn dom_get_frame_owner(&self, frame_id: &str) -> Result<i64> {
        let _ = frame_id;
        Err(crate::connection::CdpError::Other(
            "DOM.getFrameOwner is not implemented by this CDP client".to_string(),
        ))
    }
    async fn dom_resolve_node(&self, node_id: i64) -> Result<Value>;
    async fn dom_describe_node_by_object_id(&self, object_id: &str) -> Result<i64> {
        let _ = object_id;
        Err(crate::connection::CdpError::Other(
            "DOM.describeNode by object ID is not implemented by this CDP client".to_string(),
        ))
    }
    async fn dom_resolve_backend_node_in_execution_context(
        &self,
        backend_node_id: i64,
        execution_context_id: i64,
    ) -> Result<Value> {
        let _ = (backend_node_id, execution_context_id);
        Err(crate::connection::CdpError::Other(
            "DOM.resolveNode by backend node and execution context is not implemented by this CDP client"
                .to_string(),
        ))
    }
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
    #[allow(clippy::too_many_arguments)]
    async fn input_dispatch_mouse_event(
        &self,
        event_type: MouseEventType,
        x: f64,
        y: f64,
        button: Option<MouseButton>,
        click_count: Option<i32>,
        buttons: Option<i32>,
        modifiers: Option<i32>,
    ) -> Result<()>;
    async fn input_dispatch_key_event(
        &self,
        event_type: KeyEventType,
        key: &str,
        code: &str,
        windows_virtual_key_code: Option<i64>,
    ) -> Result<()>;
    async fn input_dispatch_key_event_with_modifiers(
        &self,
        event_type: KeyEventType,
        key: &str,
        code: &str,
        windows_virtual_key_code: Option<i64>,
        modifiers: Option<i32>,
    ) -> Result<()> {
        let _ = modifiers;
        self.input_dispatch_key_event(event_type, key, code, windows_virtual_key_code)
            .await
    }
    async fn input_dispatch_keyboard_event(&self, params: KeyboardEventParams<'_>) -> Result<()> {
        self.input_dispatch_key_event_with_modifiers(
            params.event_type,
            params.key,
            params.code,
            Some(params.windows_virtual_key_code),
            Some(params.modifiers),
        )
        .await
    }
    async fn input_dispatch_text_key_event(
        &self,
        key: &str,
        code: &str,
        windows_virtual_key_code: Option<i64>,
        text: &str,
    ) -> Result<()>;
    async fn input_insert_text(&self, text: &str) -> Result<()>;
    async fn input_dispatch_touch_event(
        &self,
        event_type: TouchEventType,
        x: f64,
        y: f64,
    ) -> Result<()>;

    // ── Runtime domain ──
    async fn runtime_evaluate(&self, expression: &str) -> Result<Value>;
    async fn runtime_evaluate_in_unique_context(
        &self,
        expression: &str,
        unique_context_id: &str,
    ) -> Result<Value>;
    async fn runtime_evaluate_as_object(&self, expression: &str) -> Result<Value>;
    async fn runtime_evaluate_as_object_in_unique_context(
        &self,
        expression: &str,
        unique_context_id: &str,
    ) -> Result<Value>;
    async fn runtime_call_function_on(
        &self,
        object_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
    ) -> Result<Value>;
    async fn runtime_call_function_as_object_in_unique_context(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
    ) -> Result<Value> {
        let _ = (unique_context_id, function_declaration, arguments);
        Err(crate::connection::CdpError::Other(
            "Runtime.callFunctionOn in a unique context is not implemented by this CDP client"
                .to_string(),
        ))
    }
    async fn runtime_call_function_by_value_in_unique_context(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
    ) -> Result<Value> {
        let _ = (unique_context_id, function_declaration, arguments);
        Err(crate::connection::CdpError::Other(
            "Runtime.callFunctionOn by value in a unique context is not implemented by this CDP client"
                .to_string(),
        ))
    }
    async fn runtime_get_properties(&self, object_id: &str) -> Result<Value> {
        let _ = object_id;
        Err(crate::connection::CdpError::Other(
            "Runtime.getProperties is not implemented by this CDP client".to_string(),
        ))
    }
    async fn runtime_release_object(&self, object_id: &str) -> Result<()> {
        let _ = object_id;
        Err(crate::connection::CdpError::Other(
            "Runtime.releaseObject is not implemented by this CDP client".to_string(),
        ))
    }
    async fn runtime_enable(&self) -> Result<()>;
    async fn runtime_run_if_waiting_for_debugger(&self) -> Result<()> {
        Err(crate::connection::CdpError::Other(
            "Runtime.runIfWaitingForDebugger is not implemented by this CDP client".to_string(),
        ))
    }

    // ── Accessibility domain ──
    async fn accessibility_enable(&self) -> Result<()>;
    async fn accessibility_get_full_tree(&self) -> Result<Vec<RawAXNode>>;

    // ── Network domain ──
    async fn network_enable(&self) -> Result<()>;
    async fn network_disable(&self) -> Result<()>;
    async fn network_set_blocked_urls(&self, patterns: &[String]) -> Result<()>;
    async fn network_get_cookies(&self) -> Result<Vec<Cookie>>;
    async fn network_set_cookies(&self, cookies: &[Cookie]) -> Result<()>;
    async fn network_get_response_body(&self, request_id: &str) -> Result<ResponseBody>;

    // ── Fetch domain ──
    async fn fetch_enable(&self) -> Result<()>;
    async fn fetch_enable_response_stage(&self) -> Result<()>;
    async fn fetch_disable(&self) -> Result<()>;
    async fn fetch_continue_request(&self, request_id: &str) -> Result<()>;
    async fn fetch_get_response_body(&self, request_id: &str) -> Result<ResponseBody>;
    async fn fetch_fail_request(&self, request_id: &str, reason: &str) -> Result<()>;

    // ── Target domain ──
    async fn target_create_browser_context(
        &self,
        proxy_server: Option<&str>,
        proxy_bypass_list: Option<&str>,
    ) -> Result<String>;
    async fn target_create(&self, url: &str) -> Result<String>;
    async fn target_create_in_context(&self, url: &str, browser_context_id: &str)
    -> Result<String>;
    async fn target_dispose_browser_context(&self, browser_context_id: &str) -> Result<()>;
    async fn target_close(&self, target_id: &str) -> Result<()>;
    async fn target_get_targets(&self) -> Result<Vec<TargetInfo>>;
    async fn target_get_browser_contexts(&self) -> Result<Vec<String>> {
        Err(crate::connection::CdpError::Other(
            "Target.getBrowserContexts is not implemented by this CDP client".to_string(),
        ))
    }
    async fn target_get_target_info(&self) -> Result<target::TargetInfo> {
        Err(crate::connection::CdpError::Other(
            "Target.getTargetInfo is not implemented by this CDP client".to_string(),
        ))
    }
    async fn target_attach(&self, target_id: &str) -> Result<String>;
    async fn target_detach(&self, session_id: &str) -> Result<()>;
    async fn target_set_auto_attach(
        &self,
        auto_attach: bool,
        wait_for_debugger_on_start: bool,
        flatten: bool,
    ) -> Result<()>;

    // ── Events ──
    fn event_session_id(&self) -> Option<&str>;
    fn subscribe_events(&self) -> tokio::sync::broadcast::Receiver<CdpEvent>;
    fn subscribe_composable_events(
        &self,
    ) -> std::result::Result<CdpEventReceiver, CdpEventSubscriptionError> {
        Ok(self.subscribe_events().into())
    }
}

// ── CdpSession implements CdpClient ──
// Every operation remains bound to the concrete connection and native session
// that created this object. Native transport loss is terminal for the object.

use crate::events::{CdpEvent, CdpEventReceiver};
use crate::session::CdpSession;

/// Generate a `CdpClient` delegation impl. Each entry produces
/// `async fn method(&self, params) -> Ret { CdpSession::method(self, params).await }`.
macro_rules! cdp_delegate_impl {
    (
        $(
            async fn $method:ident(&self $(, $arg:ident : $ty:ty)*) -> $ret:ty;
        )*
    ) => {
        #[async_trait]
        impl CdpClient for CdpSession {
            fn event_session_id(&self) -> Option<&str> {
                self.session_id()
            }

            fn subscribe_events(&self) -> tokio::sync::broadcast::Receiver<CdpEvent> {
                self.connection().subscribe_events()
            }

            fn subscribe_composable_events(
                &self,
            ) -> std::result::Result<CdpEventReceiver, CdpEventSubscriptionError> {
                self.connection().subscribe_composable_events()
            }

            $(
                async fn $method(&self $(, $arg: $ty)*) -> $ret {
                    CdpSession::$method(self $(, $arg)*).await
                }
            )*
        }
    };
}

cdp_delegate_impl! {
    async fn browser_set_download_behavior(&self, behavior: DownloadBehavior, download_path: Option<&str>, events_enabled: bool) -> Result<()>;
    async fn page_navigate(&self, url: &str) -> Result<Value>;
    async fn page_navigate_with_referrer(&self, url: &str, referrer: Option<&str>) -> Result<Value>;
    async fn page_stop_loading(&self) -> Result<()>;
    async fn page_enable(&self) -> Result<()>;
    async fn page_set_lifecycle_events_enabled(&self, enabled: bool) -> Result<()>;
    async fn page_capture_screenshot(&self, params: page::CaptureScreenshotParams) -> Result<String>;
    async fn page_get_layout_metrics(&self) -> Result<page::GetLayoutMetricsReturns>;
    async fn page_print_to_pdf(&self, params: Value) -> Result<String>;
    async fn page_add_script_on_new_document(&self, source: &str) -> Result<String>;
    async fn page_add_script_on_new_document_in_world(&self, source: &str, world_name: &str) -> Result<String>;
    async fn page_create_isolated_world(&self, frame_id: &str, world_name: &str, grant_universal_access: bool) -> Result<i64>;
    async fn page_reload(&self) -> Result<()>;
    async fn page_get_navigation_history(&self) -> Result<Value>;
    async fn page_navigate_to_history_entry(&self, entry_id: i64) -> Result<()>;
    async fn page_bring_to_front(&self) -> Result<()>;
    async fn page_set_document_content(&self, frame_id: &str, html: &str) -> Result<()>;
    async fn page_close(&self) -> Result<()>;
    async fn page_get_main_frame_id(&self) -> Result<String>;
    async fn page_get_frame_tree(&self) -> Result<page::FrameTree>;
    async fn dom_focus(&self, node_id: i64) -> Result<()>;
    async fn dom_scroll_into_view(&self, node_id: i64) -> Result<()>;
    async fn dom_get_box_model(&self, node_id: i64) -> Result<Value>;
    async fn dom_get_box_model_by_backend_node(&self, backend_node_id: i64) -> Result<Value>;
    async fn dom_get_frame_owner(&self, frame_id: &str) -> Result<i64>;
    async fn dom_resolve_node(&self, node_id: i64) -> Result<Value>;
    async fn dom_describe_node_by_object_id(&self, object_id: &str) -> Result<i64>;
    async fn dom_resolve_backend_node_in_execution_context(&self, backend_node_id: i64, execution_context_id: i64) -> Result<Value>;
    async fn dom_enable(&self) -> Result<()>;
    async fn dom_get_document(&self) -> Result<Value>;
    async fn dom_query_selector(&self, node_id: i64, selector: &str) -> Result<i64>;
    async fn dom_query_selector_all(&self, node_id: i64, selector: &str) -> Result<Vec<i64>>;
    async fn dom_get_attributes(&self, node_id: i64) -> Result<Vec<String>>;
    async fn dom_get_outer_html(&self, node_id: i64) -> Result<String>;
    async fn dom_describe_node(&self, backend_node_id: i64) -> Result<Value>;
    async fn dom_set_file_input_files(&self, node_id: i64, files: &[String]) -> Result<()>;
    async fn dom_request_node(&self, object_id: &str) -> Result<i64>;
    async fn input_dispatch_mouse_event(&self, event_type: MouseEventType, x: f64, y: f64, button: Option<MouseButton>, click_count: Option<i32>, buttons: Option<i32>, modifiers: Option<i32>) -> Result<()>;
    async fn input_dispatch_key_event(&self, event_type: KeyEventType, key: &str, code: &str, windows_virtual_key_code: Option<i64>) -> Result<()>;
    async fn input_dispatch_key_event_with_modifiers(&self, event_type: KeyEventType, key: &str, code: &str, windows_virtual_key_code: Option<i64>, modifiers: Option<i32>) -> Result<()>;
    async fn input_dispatch_keyboard_event(&self, params: KeyboardEventParams<'_>) -> Result<()>;
    async fn input_dispatch_text_key_event(&self, key: &str, code: &str, windows_virtual_key_code: Option<i64>, text: &str) -> Result<()>;
    async fn input_insert_text(&self, text: &str) -> Result<()>;
    async fn input_dispatch_touch_event(&self, event_type: TouchEventType, x: f64, y: f64) -> Result<()>;
    async fn runtime_evaluate(&self, expression: &str) -> Result<Value>;
    async fn runtime_evaluate_in_unique_context(
        &self,
        expression: &str,
        unique_context_id: &str
    ) -> Result<Value>;
    async fn runtime_evaluate_as_object(&self, expression: &str) -> Result<Value>;
    async fn runtime_evaluate_as_object_in_unique_context(&self, expression: &str, unique_context_id: &str) -> Result<Value>;
    async fn runtime_call_function_on(&self, object_id: &str, function_declaration: &str, arguments: Vec<Value>) -> Result<Value>;
    async fn runtime_call_function_as_object_in_unique_context(&self, unique_context_id: &str, function_declaration: &str, arguments: Vec<Value>) -> Result<Value>;
    async fn runtime_call_function_by_value_in_unique_context(&self, unique_context_id: &str, function_declaration: &str, arguments: Vec<Value>) -> Result<Value>;
    async fn runtime_get_properties(&self, object_id: &str) -> Result<Value>;
    async fn runtime_release_object(&self, object_id: &str) -> Result<()>;
    async fn runtime_enable(&self) -> Result<()>;
    async fn runtime_run_if_waiting_for_debugger(&self) -> Result<()>;
    async fn accessibility_enable(&self) -> Result<()>;
    async fn accessibility_get_full_tree(&self) -> Result<Vec<RawAXNode>>;
    async fn network_enable(&self) -> Result<()>;
    async fn network_disable(&self) -> Result<()>;
    async fn network_set_blocked_urls(&self, patterns: &[String]) -> Result<()>;
    async fn network_get_cookies(&self) -> Result<Vec<Cookie>>;
    async fn network_set_cookies(&self, cookies: &[Cookie]) -> Result<()>;
    async fn network_get_response_body(&self, request_id: &str) -> Result<ResponseBody>;
    async fn fetch_enable(&self) -> Result<()>;
    async fn fetch_enable_response_stage(&self) -> Result<()>;
    async fn fetch_disable(&self) -> Result<()>;
    async fn fetch_continue_request(&self, request_id: &str) -> Result<()>;
    async fn fetch_get_response_body(&self, request_id: &str) -> Result<ResponseBody>;
    async fn fetch_fail_request(&self, request_id: &str, reason: &str) -> Result<()>;
    async fn target_create_browser_context(&self, proxy_server: Option<&str>, proxy_bypass_list: Option<&str>) -> Result<String>;
    async fn target_create(&self, url: &str) -> Result<String>;
    async fn target_create_in_context(&self, url: &str, browser_context_id: &str) -> Result<String>;
    async fn target_dispose_browser_context(&self, browser_context_id: &str) -> Result<()>;
    async fn target_close(&self, target_id: &str) -> Result<()>;
    async fn target_get_targets(&self) -> Result<Vec<TargetInfo>>;
    async fn target_get_browser_contexts(&self) -> Result<Vec<String>>;
    async fn target_get_target_info(&self) -> Result<target::TargetInfo>;
    async fn target_attach(&self, target_id: &str) -> Result<String>;
    async fn target_detach(&self, session_id: &str) -> Result<()>;
    async fn target_set_auto_attach(&self, auto_attach: bool, wait_for_debugger_on_start: bool, flatten: bool) -> Result<()>;
}
