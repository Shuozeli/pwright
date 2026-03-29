//! Locator — Playwright-compatible element selection and interaction.
//!
//! A `Locator` represents a CSS selector query. Actions are performed
//! by resolving the selector to a DOM node, then delegating to bridge functions.

use std::sync::Arc;

use pwright_cdp::CdpClient;
use pwright_cdp::connection::CdpError;
use pwright_cdp::connection::Result as CdpResult;
use serde_json::{Value, json};

use crate::clock::{Clock, TokioClock};

use super::selectors;
use super::selectors::SelectorKind;

/// State to wait for in `Locator::wait_for`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WaitState {
    /// Wait for element to be attached to the DOM.
    Attached,
    /// Wait for element to be visible (has layout / box model). This is the default.
    #[default]
    Visible,
    /// Wait for element to be hidden (no layout) or detached.
    Hidden,
    /// Wait for element to be detached from the DOM.
    Detached,
}

/// A Playwright-compatible Locator. Lazily resolves on each action call.
///
/// ```rust,ignore
/// let loc = page.locator("button.submit");
/// loc.click(None).await?;
/// loc.fill("hello").await?;
/// ```
pub struct Locator {
    session: Arc<dyn CdpClient>,
    selector: SelectorKind,
    clock: Arc<dyn Clock>,
}

impl Locator {
    /// Create a locator from a CSS selector string (backwards-compatible convenience).
    pub(crate) fn new(session: Arc<dyn CdpClient>, selector: impl Into<String>) -> Self {
        Self {
            session,
            selector: SelectorKind::Css(selector.into()),
            clock: Arc::new(TokioClock::new()),
        }
    }

    /// Create a locator from a typed `SelectorKind`.
    pub(crate) fn new_with_kind(session: Arc<dyn CdpClient>, selector: SelectorKind) -> Self {
        Self {
            session,
            selector,
            clock: Arc::new(TokioClock::new()),
        }
    }

    /// Create a locator from a `SelectorKind` with a custom clock (for testing).
    pub(crate) fn with_clock_and_kind(
        session: Arc<dyn CdpClient>,
        selector: SelectorKind,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            session,
            selector,
            clock,
        }
    }

    /// Get the selector kind.
    pub fn selector(&self) -> &SelectorKind {
        &self.selector
    }

    // ── Actions ──

    /// Click the element.
    pub async fn click(&self) -> CdpResult<()> {
        let node = self.resolve_one().await?;
        crate::actions::click_by_node_id(&*self.session, node.node_id).await
    }

    /// Double-click the element.
    pub async fn dblclick(&self) -> CdpResult<()> {
        let node = self.resolve_one().await?;
        crate::actions::dblclick_by_node_id(&*self.session, node.node_id).await
    }

    /// Fill an input or textarea with a value (clears existing value first).
    ///
    /// This sets `element.value` and dispatches input/change events. It does
    /// **not** work on `contenteditable` elements (Quill, ProseMirror, TipTap,
    /// Slate, etc.) because they lack a `.value` property. For contenteditable,
    /// focus the element and use [`super::keyboard::Keyboard::type_text`] instead:
    /// ```rust,ignore
    /// page.click(".ql-editor").await?;
    /// page.keyboard().type_text("hello").await?;
    /// ```
    pub async fn fill(&self, value: &str) -> CdpResult<()> {
        let node = self.resolve_one().await?;
        crate::actions::fill_by_node_id(&*self.session, node.node_id, value).await
    }

    /// Type text character by character.
    pub async fn type_text(&self, text: &str) -> CdpResult<()> {
        let node = self.resolve_one().await?;
        crate::actions::type_by_node_id(&*self.session, node.node_id, text).await
    }

    /// Press a key on the element.
    pub async fn press(&self, key: &str) -> CdpResult<()> {
        let node = self.resolve_one().await?;
        self.session.dom_focus(node.node_id).await?;
        crate::actions::press_key(&*self.session, key).await
    }

    /// Hover over the element.
    pub async fn hover(&self) -> CdpResult<()> {
        let node = self.resolve_one().await?;
        crate::actions::hover_by_node_id(&*self.session, node.node_id).await
    }

    /// Focus the element.
    pub async fn focus(&self) -> CdpResult<()> {
        let node = self.resolve_one().await?;
        self.session.dom_focus(node.node_id).await
    }

    /// Blur (unfocus) the element.
    pub async fn blur(&self) -> CdpResult<()> {
        let node = self.resolve_one().await?;
        let obj_id = self.resolve_object_id(node.node_id).await?;
        self.session
            .runtime_call_function_on(&obj_id, pwright_js::element::BLUR, vec![])
            .await?;
        Ok(())
    }

    /// Check a checkbox (no-op if already checked).
    pub async fn check(&self) -> CdpResult<()> {
        if !self.is_checked().await? {
            self.click().await?;
        }
        Ok(())
    }

    /// Uncheck a checkbox (no-op if already unchecked).
    pub async fn uncheck(&self) -> CdpResult<()> {
        if self.is_checked().await? {
            self.click().await?;
        }
        Ok(())
    }

    /// Select an option by value.
    pub async fn select_option(&self, value: &str) -> CdpResult<()> {
        let node = self.resolve_one().await?;
        crate::actions::select_by_node_id(&*self.session, node.node_id, value).await
    }

    /// Scroll the element into view.
    pub async fn scroll_into_view(&self) -> CdpResult<()> {
        let node = self.resolve_one().await?;
        self.session.dom_scroll_into_view(node.node_id).await
    }

    /// Set files on a file `<input>` element.
    ///
    /// Unlike Playwright (Node/Python), this sends file **paths** to Chrome via
    /// CDP `DOM.setFileInputFiles`. The files must exist on Chrome's filesystem,
    /// not the machine running this code. For remote Chrome instances, copy files
    /// to the remote host first (e.g. via `rsync`).
    pub async fn set_input_files(&self, files: &[String]) -> CdpResult<()> {
        let node = self.resolve_one().await?;
        self.session
            .dom_set_file_input_files(node.node_id, files)
            .await
    }

    /// Evaluate a JavaScript function on the element.
    ///
    /// The function receives the element as `this`. Returns the raw CDP result value.
    ///
    /// ```rust,ignore
    /// let result = locator.evaluate("function() { return this.offsetHeight; }").await?;
    /// ```
    pub async fn evaluate(&self, function_body: &str) -> CdpResult<Value> {
        let node = self.resolve_one().await?;
        let obj_id = self.resolve_object_id(node.node_id).await?;
        let result = self
            .session
            .runtime_call_function_on(&obj_id, function_body, vec![])
            .await?;
        Ok(result.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Evaluate a JavaScript function on the element and convert to a typed value.
    ///
    /// ```rust,ignore
    /// let height: i64 = locator.evaluate_into("function() { return this.offsetHeight; }").await?;
    /// let visible: bool = locator.evaluate_into("function() { return this.checkVisibility(); }").await?;
    /// ```
    pub async fn evaluate_into<T: crate::evaluate::FromEvalResult>(
        &self,
        function_body: &str,
    ) -> CdpResult<T> {
        let remote_object = self.evaluate(function_body).await?;
        T::from_eval_result(&remote_object)
    }

    /// Dispatch a custom event on the element.
    pub async fn dispatch_event(&self, event_type: &str) -> CdpResult<()> {
        let node = self.resolve_one().await?;
        let obj_id = self.resolve_object_id(node.node_id).await?;
        self.session
            .runtime_call_function_on(
                &obj_id,
                pwright_js::element::DISPATCH_EVENT,
                vec![json!({"value": event_type})],
            )
            .await?;
        Ok(())
    }

    // ── Queries ──

    /// Get the text content of the element.
    pub async fn text_content(&self) -> CdpResult<Option<String>> {
        let node = self.resolve_one().await?;
        let obj_id = self.resolve_object_id(node.node_id).await?;
        let result = self
            .session
            .runtime_call_function_on(&obj_id, pwright_js::element::GET_TEXT_CONTENT, vec![])
            .await?;
        Ok(crate::evaluate::extract_result_value(&result)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()))
    }

    /// Get the inner text of the element (layout-aware).
    pub async fn inner_text(&self) -> CdpResult<String> {
        let node = self.resolve_one().await?;
        let obj_id = self.resolve_object_id(node.node_id).await?;
        let result = self
            .session
            .runtime_call_function_on(&obj_id, pwright_js::element::GET_INNER_TEXT, vec![])
            .await?;
        Ok(crate::evaluate::extract_result_value(&result)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string())
    }

    /// Get the inner HTML.
    pub async fn inner_html(&self) -> CdpResult<String> {
        let node = self.resolve_one().await?;
        let html = self.session.dom_get_outer_html(node.node_id).await?;
        // Strip outer tag to get innerHTML
        Ok(strip_outer_tag(&html))
    }

    /// Get an attribute value by name. Pure CDP — no JS evaluation.
    pub async fn get_attribute(&self, name: &str) -> CdpResult<Option<String>> {
        let node = self.resolve_one().await?;
        let attrs = self.session.dom_get_attributes(node.node_id).await?;
        // Attributes come as flat [name1, val1, name2, val2, ...]
        let mut iter = attrs.iter();
        while let Some(key) = iter.next() {
            if let Some(val) = iter.next()
                && key == name
            {
                return Ok(Some(val.clone()));
            }
        }
        Ok(None)
    }

    /// Get the input value. Requires JS.
    pub async fn input_value(&self) -> CdpResult<String> {
        let node = self.resolve_one().await?;
        let obj_id = self.resolve_object_id(node.node_id).await?;
        let result = self
            .session
            .runtime_call_function_on(&obj_id, pwright_js::element::GET_INPUT_VALUE, vec![])
            .await?;
        Ok(crate::evaluate::extract_result_value(&result)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string())
    }

    /// Check if element is visible. Pure CDP — no JS (uses DOM.getBoxModel).
    pub async fn is_visible(&self) -> CdpResult<bool> {
        let node = self.resolve_one().await?;
        match self.session.dom_get_box_model(node.node_id).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Check if element is hidden. Inverse of `is_visible`.
    pub async fn is_hidden(&self) -> CdpResult<bool> {
        Ok(!self.is_visible().await?)
    }

    /// Check if element is enabled. AX tree — no JS.
    pub async fn is_enabled(&self) -> CdpResult<bool> {
        // If AX tree query fails, fall back to true (most elements are enabled)
        Ok(!self.is_disabled().await?)
    }

    /// Check if element is disabled. Uses JS DOM property for accuracy.
    pub async fn is_disabled(&self) -> CdpResult<bool> {
        let node = self.resolve_one().await?;
        let obj_id = self.resolve_object_id(node.node_id).await?;
        let result = self
            .session
            .runtime_call_function_on(&obj_id, pwright_js::element::IS_DISABLED, vec![])
            .await?;
        Ok(crate::evaluate::extract_result_value(&result)
            .and_then(|v| v.as_bool())
            .unwrap_or(false))
    }

    /// Check if a checkbox/radio is checked. Uses JS DOM property for accuracy.
    pub async fn is_checked(&self) -> CdpResult<bool> {
        let node = self.resolve_one().await?;
        let obj_id = self.resolve_object_id(node.node_id).await?;
        let result = self
            .session
            .runtime_call_function_on(&obj_id, pwright_js::element::IS_CHECKED, vec![])
            .await?;
        Ok(crate::evaluate::extract_result_value(&result)
            .and_then(|v| v.as_bool())
            .unwrap_or(false))
    }

    /// Get the bounding box of the element. Pure CDP.
    pub async fn bounding_box(&self) -> CdpResult<Option<BoundingBox>> {
        let node = self.resolve_one().await?;
        match self.session.dom_get_box_model(node.node_id).await {
            Ok(model) => {
                let content = model
                    .get("model")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_array());
                if let Some(pts) = content {
                    let x = pts[0].as_f64().unwrap_or(0.0);
                    let y = pts[1].as_f64().unwrap_or(0.0);
                    let width = pts[2].as_f64().unwrap_or(0.0) - x;
                    let height = pts[5].as_f64().unwrap_or(0.0) - y;
                    Ok(Some(BoundingBox {
                        x,
                        y,
                        width,
                        height,
                    }))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Ok(None),
        }
    }

    /// Count matching elements.
    pub async fn count(&self) -> CdpResult<usize> {
        let elements = selectors::resolve_selector_all(&*self.session, &self.selector).await?;
        Ok(elements.len())
    }

    /// Wait for the selector to reach the desired state.
    pub async fn wait_for(&self, timeout_ms: u64, state: WaitState) -> CdpResult<()> {
        const WAIT_POLL_MS: u64 = 200;
        let poll_interval = std::time::Duration::from_millis(WAIT_POLL_MS);
        let deadline = self
            .clock
            .deadline_from_now(std::time::Duration::from_millis(timeout_ms));

        loop {
            self.clock.sleep(poll_interval).await;

            let resolved = selectors::resolve_selector(&*self.session, &self.selector).await?;

            match state {
                WaitState::Attached => {
                    if resolved.is_some() {
                        return Ok(());
                    }
                }
                WaitState::Visible => {
                    if let Some(elem) = resolved
                        && self.session.dom_get_box_model(elem.node_id).await.is_ok()
                    {
                        return Ok(());
                    }
                }
                WaitState::Hidden => match resolved {
                    None => return Ok(()),
                    Some(elem) => {
                        if self.session.dom_get_box_model(elem.node_id).await.is_err() {
                            return Ok(());
                        }
                    }
                },
                WaitState::Detached => {
                    if resolved.is_none() {
                        return Ok(());
                    }
                }
            }

            if self.clock.is_past(deadline) {
                return Err(CdpError::Timeout);
            }
        }
    }

    // ── Composition ──

    /// Create a derived locator from a `SelectorKind`, inheriting this locator's clock.
    fn derive_kind(&self, selector: SelectorKind) -> Locator {
        Locator::with_clock_and_kind(self.session.clone(), selector, self.clock.clone())
    }

    /// Return a Locator matching the first element.
    pub fn first(&self) -> Locator {
        self.derive_kind(SelectorKind::Nth {
            base: Box::new(self.selector.clone()),
            index: 0,
        })
    }

    /// Return a Locator matching the last element.
    pub fn last(&self) -> Locator {
        self.derive_kind(SelectorKind::Nth {
            base: Box::new(self.selector.clone()),
            index: -1,
        })
    }

    /// Return a Locator matching the nth element (0-based).
    pub fn nth(&self, index: i64) -> Locator {
        self.derive_kind(SelectorKind::Nth {
            base: Box::new(self.selector.clone()),
            index,
        })
    }

    /// Return a Locator scoped to a sub-selector.
    ///
    /// For CSS base selectors, combines as `"base sub"`. For non-CSS bases,
    /// uses the sub-selector alone since CSS descendant chaining is meaningless.
    pub fn locator(&self, sub_selector: &str) -> Locator {
        let combined = match &self.selector {
            SelectorKind::Css(css) => SelectorKind::Css(format!("{css} {sub_selector}")),
            _ => SelectorKind::Css(sub_selector.to_string()),
        };
        self.derive_kind(combined)
    }

    /// Filter matched elements by text content.
    ///
    /// Returns a new Locator that uses JS-based text filtering.
    pub fn filter_by_text(&self, text: &str) -> Locator {
        self.derive_kind(SelectorKind::FilterText {
            base: Box::new(self.selector.clone()),
            text: text.to_string(),
        })
    }

    /// Combine with another locator using AND (intersection).
    /// The resulting locator matches elements that satisfy both selectors.
    pub fn and(&self, other: &Locator) -> Locator {
        self.derive_kind(SelectorKind::Css(format!(
            "{}:is({})",
            self.selector, other.selector
        )))
    }

    /// Combine with another locator using OR (union).
    /// The resulting locator matches elements that satisfy either selector.
    pub fn or(&self, other: &Locator) -> Locator {
        self.derive_kind(SelectorKind::Css(format!(
            "{}, {}",
            self.selector, other.selector
        )))
    }

    // ── Internal ──

    async fn resolve_one(&self) -> CdpResult<selectors::ResolvedElement> {
        selectors::resolve_selector(&*self.session, &self.selector)
            .await?
            .ok_or_else(|| CdpError::ElementNotFound {
                selector: self.selector.to_string(),
            })
    }

    async fn resolve_object_id(&self, node_id: i64) -> CdpResult<String> {
        crate::actions::resolve_to_object_id(&*self.session, node_id).await
    }
}

/// Bounding box of an element.
#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Strip the outer HTML tag to get innerHTML.
///
/// Quote-aware: handles `>` inside attribute values like `data-expr="a>b"`.
fn strip_outer_tag(html: &str) -> String {
    // Find the end of the opening tag, respecting quotes
    let mut in_quote: Option<char> = None;
    let mut open_end = None;
    for (i, ch) in html.char_indices() {
        match in_quote {
            Some(q) if ch == q => in_quote = None,
            Some(_) => {}
            None => match ch {
                '"' | '\'' => in_quote = Some(ch),
                '>' => {
                    open_end = Some(i);
                    break;
                }
                _ => {}
            },
        }
    }
    if let Some(open_end) = open_end {
        // Find the start of the closing tag (last '<')
        if let Some(close_start) = html.rfind('<')
            && close_start > open_end
        {
            return html[open_end + 1..close_start].to_string();
        }
    }
    html.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockCdpClient;

    fn mock_with_element() -> Arc<MockCdpClient> {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_query_selector_response(42);
        mock
    }

    #[tokio::test]
    async fn test_locator_click() {
        let mock = mock_with_element();
        // Need resolve_node + box_model for click path
        mock.set_resolve_node(serde_json::json!({"object": {"objectId": "obj-1"}}));
        mock.set_call_function_response(serde_json::json!({
            "result": {"value": {"x": 100.0, "y": 200.0}}
        }));

        let loc = Locator::new(mock.clone(), "button");
        loc.click().await.unwrap();

        let methods = mock.method_names();
        assert!(methods.contains(&"DOM.getDocument".to_string()));
        assert!(methods.contains(&"DOM.querySelector".to_string()));
    }

    #[tokio::test]
    async fn test_locator_get_attribute_no_js() {
        let mock = mock_with_element();
        mock.set_get_attributes_response(vec![
            "href".to_string(),
            "https://example.com".to_string(),
            "class".to_string(),
            "btn".to_string(),
        ]);

        let loc = Locator::new(mock.clone(), "a");
        let href = loc.get_attribute("href").await.unwrap();
        assert_eq!(href, Some("https://example.com".to_string()));

        let class = loc.get_attribute("class").await.unwrap();
        assert_eq!(class, Some("btn".to_string()));

        let missing = loc.get_attribute("data-x").await.unwrap();
        assert_eq!(missing, None);

        // Verify no Runtime.evaluate or callFunctionOn was called
        let methods = mock.method_names();
        assert!(!methods.contains(&"Runtime.evaluate".to_string()));
        assert!(!methods.contains(&"Runtime.callFunctionOn".to_string()));
    }

    #[tokio::test]
    async fn test_locator_count() {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_query_selector_all_response(vec![1, 2, 3, 4, 5]);

        let loc = Locator::new(mock.clone(), "li");
        assert_eq!(loc.count().await.unwrap(), 5);
    }

    #[tokio::test]
    async fn test_locator_not_found() {
        let mock = Arc::new(MockCdpClient::new());
        // query_selector returns 0 = not found

        let loc = Locator::new(mock.clone(), ".nonexistent");
        let result = loc.click().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_locator_composition() {
        let mock = mock_with_element();
        let loc = Locator::new(mock.clone(), "ul");

        let child = loc.locator("li");
        assert_eq!(child.selector().to_string(), "ul li");

        let first = loc.first();
        assert_eq!(first.selector().to_string(), "ul.nth(0)");

        let last = loc.last();
        assert_eq!(last.selector().to_string(), "ul.nth(-1)");

        let nth = loc.nth(2);
        assert_eq!(nth.selector().to_string(), "ul.nth(2)");
    }

    #[tokio::test]
    async fn test_first_resolves_to_first_element() {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_query_selector_all_response(vec![10, 20, 30]);

        let loc = Locator::new(mock.clone(), "li");
        let first = loc.first();
        let count_result = first.count().await.unwrap();
        // first() should resolve to exactly 1 element
        assert_eq!(count_result, 1);
    }

    #[tokio::test]
    async fn test_last_resolves_to_last_element() {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_query_selector_all_response(vec![10, 20, 30]);
        mock.set_resolve_node(serde_json::json!({"object": {"objectId": "obj-1"}}));
        mock.set_call_function_response(serde_json::json!({
            "result": {"value": "third"}
        }));

        let loc = Locator::new(mock.clone(), "li");
        // last() should pick node_id 30 (index -1)
        let last = loc.last();
        let text = last.text_content().await.unwrap();
        assert_eq!(text, Some("third".to_string()));

        // Verify it resolved node 30
        let resolve_calls = mock.calls_for("DOM.resolveNode");
        let resolved_id = resolve_calls.last().unwrap().args[0]["nodeId"]
            .as_i64()
            .unwrap();
        assert_eq!(resolved_id, 30);
    }

    #[tokio::test]
    async fn test_first_empty_returns_element_not_found() {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_query_selector_all_response(vec![]);

        let loc = Locator::new(mock.clone(), "li");
        let result = loc.first().click().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_is_checked_uses_js_property() {
        let mock = mock_with_element();
        mock.set_call_function_response(serde_json::json!({
            "result": {"value": true}
        }));

        let loc = Locator::new(mock.clone(), "input[type=checkbox]");
        let checked = loc.is_checked().await.unwrap();
        assert!(checked);

        // Verify Runtime.callFunctionOn IS called (not DOM.getAttributes)
        let methods = mock.method_names();
        assert!(methods.contains(&"Runtime.callFunctionOn".to_string()));
    }

    #[tokio::test]
    async fn test_is_disabled_uses_js_property() {
        let mock = mock_with_element();
        mock.set_call_function_response(serde_json::json!({
            "result": {"value": true}
        }));

        let loc = Locator::new(mock.clone(), "button");
        let disabled = loc.is_disabled().await.unwrap();
        assert!(disabled);

        let methods = mock.method_names();
        assert!(methods.contains(&"Runtime.callFunctionOn".to_string()));
        // Should NOT use DOM.getAttributes
        assert!(!methods.contains(&"DOM.getAttributes".to_string()));
    }

    #[tokio::test]
    async fn test_wait_for_visible_succeeds_with_box_model() {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_query_selector_response(42);
        // box_model succeeds by default = visible

        let loc = Locator::new(mock.clone(), "div");
        loc.wait_for(1000, WaitState::Visible).await.unwrap();
    }

    #[tokio::test]
    async fn test_wait_for_visible_times_out_without_box_model() {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_query_selector_response(42);
        mock.set_box_model_error(true);

        let loc = Locator::new(mock.clone(), "div");
        let result = loc.wait_for(300, WaitState::Visible).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_wait_for_attached_succeeds_when_found() {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_query_selector_response(42);

        let loc = Locator::new(mock.clone(), "div");
        loc.wait_for(1000, WaitState::Attached).await.unwrap();
    }

    #[tokio::test]
    async fn test_wait_for_detached_succeeds_when_not_found() {
        let mock = Arc::new(MockCdpClient::new());
        // query_selector returns 0 = not found

        let loc = Locator::new(mock.clone(), "div");
        loc.wait_for(1000, WaitState::Detached).await.unwrap();
    }

    #[tokio::test]
    async fn test_locator_evaluate() {
        let mock = mock_with_element();
        mock.set_call_function_response(serde_json::json!({
            "result": {"value": 42}
        }));

        let loc = Locator::new(mock.clone(), "div");
        let result = loc
            .evaluate("function() { return this.offsetHeight; }")
            .await
            .unwrap();
        assert_eq!(result.get("value").and_then(|v| v.as_i64()), Some(42));

        // Verify correct objectId and function body passed
        let cf_calls = mock.calls_for("Runtime.callFunctionOn");
        let last = cf_calls.last().unwrap();
        assert_eq!(
            last.args[0]["functionDeclaration"].as_str().unwrap(),
            "function() { return this.offsetHeight; }"
        );
        assert_eq!(last.args[0]["objectId"].as_str().unwrap(), "mock-obj-1");
    }

    #[tokio::test]
    async fn test_wait_for_hidden_succeeds_when_no_box_model() {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_query_selector_response(42);
        mock.set_box_model_error(true);

        let loc = Locator::new(mock.clone(), "div");
        loc.wait_for(1000, WaitState::Hidden).await.unwrap();
    }

    #[test]
    fn test_strip_outer_tag() {
        assert_eq!(strip_outer_tag("<div>hello</div>"), "hello");
        assert_eq!(strip_outer_tag(r#"<div class="x">inner</div>"#), "inner");
        assert_eq!(strip_outer_tag("<span></span>"), "");
    }

    #[test]
    fn test_strip_outer_tag_with_gt_in_attribute() {
        // Double quotes
        assert_eq!(
            strip_outer_tag(r#"<div data-expr="a>b">content</div>"#),
            "content"
        );
        // Single quotes
        assert_eq!(
            strip_outer_tag("<div data-expr='a>b'>content</div>"),
            "content"
        );
    }

    #[tokio::test]
    async fn test_element_not_found_error_variant() {
        let mock = Arc::new(MockCdpClient::new());
        let loc = Locator::new(mock.clone(), ".missing");
        let result = loc.click().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        match &err {
            CdpError::ElementNotFound { selector } => {
                assert_eq!(selector, ".missing");
            }
            other => panic!("expected ElementNotFound, got: {other}"),
        }
    }

    #[tokio::test]
    async fn test_locator_text_content() {
        let mock = mock_with_element();
        mock.set_resolve_node(serde_json::json!({"object": {"objectId": "obj-1"}}));
        mock.set_call_function_response(serde_json::json!({
            "result": {"value": "hello world"}
        }));

        let loc = Locator::new(mock.clone(), "p");
        let text = loc.text_content().await.unwrap();
        assert_eq!(text, Some("hello world".to_string()));
    }
}
