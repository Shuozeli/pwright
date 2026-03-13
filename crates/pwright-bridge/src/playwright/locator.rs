//! Locator — Playwright-compatible element selection and interaction.
//!
//! A `Locator` represents a CSS selector query. Actions are performed
//! by resolving the selector to a DOM node, then delegating to bridge functions.

use std::sync::Arc;

use pwright_cdp::CdpClient;
use pwright_cdp::connection::CdpError;
use pwright_cdp::connection::Result as CdpResult;
use serde_json::json;

use super::selectors;

/// A Playwright-compatible Locator. Lazily resolves on each action call.
///
/// ```rust,ignore
/// let loc = page.locator("button.submit");
/// loc.click(None).await?;
/// loc.fill("hello").await?;
/// ```
pub struct Locator {
    session: Arc<dyn CdpClient>,
    selector: String,
}

impl Locator {
    pub(crate) fn new(session: Arc<dyn CdpClient>, selector: impl Into<String>) -> Self {
        Self {
            session,
            selector: selector.into(),
        }
    }

    /// Get the selector string.
    pub fn selector(&self) -> &str {
        &self.selector
    }

    // ── Actions ──

    /// Click the element.
    pub async fn click(&self) -> CdpResult<()> {
        let node = self.resolve_one().await?;
        crate::actions::click_by_node_id(&*self.session, node.node_id).await
    }

    /// Fill an input element with a value (clears existing value first).
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

    /// Set files on a file input element.
    pub async fn set_input_files(&self, files: &[String]) -> CdpResult<()> {
        let node = self.resolve_one().await?;
        self.session
            .dom_set_file_input_files(node.node_id, files)
            .await
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
        Ok(result
            .get("result")
            .and_then(|r| r.get("value"))
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
        Ok(result
            .get("result")
            .and_then(|r| r.get("value"))
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
        Ok(result
            .get("result")
            .and_then(|r| r.get("value"))
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

    /// Check if element is disabled. AX tree — no JS.
    pub async fn is_disabled(&self) -> CdpResult<bool> {
        let node = self.resolve_one().await?;
        let attrs = self.session.dom_get_attributes(node.node_id).await?;
        let mut iter = attrs.iter();
        while let Some(key) = iter.next() {
            if let Some(_val) = iter.next()
                && key == "disabled"
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Check if a checkbox/radio is checked. Attribute check — no JS.
    pub async fn is_checked(&self) -> CdpResult<bool> {
        let node = self.resolve_one().await?;
        let attrs = self.session.dom_get_attributes(node.node_id).await?;
        let mut iter = attrs.iter();
        while let Some(key) = iter.next() {
            if let Some(_val) = iter.next()
                && key == "checked"
            {
                return Ok(true);
            }
        }
        Ok(false)
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

    /// Wait for the selector to match at least one element.
    pub async fn wait_for(&self, timeout_ms: u64) -> CdpResult<()> {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(200));

        loop {
            interval.tick().await;
            if tokio::time::Instant::now() > deadline {
                return Err(CdpError::Timeout);
            }
            if selectors::resolve_selector(&*self.session, &self.selector)
                .await?
                .is_some()
            {
                return Ok(());
            }
        }
    }

    // ── Composition ──

    /// Return a Locator matching the first element.
    pub fn first(&self) -> Locator {
        // CSS :first-of-type or we can use nth(0) internally
        Locator::new(
            self.session.clone(),
            format!("{}:first-of-type", self.selector),
        )
    }

    /// Return a Locator matching the last element.
    pub fn last(&self) -> Locator {
        Locator::new(
            self.session.clone(),
            format!("{}:last-of-type", self.selector),
        )
    }

    /// Return a Locator scoped to a sub-selector.
    pub fn locator(&self, sub_selector: &str) -> Locator {
        Locator::new(
            self.session.clone(),
            format!("{} {}", self.selector, sub_selector),
        )
    }

    /// Filter matched elements by text content.
    ///
    /// Returns a new Locator that uses JS-based text filtering.
    pub fn filter_by_text(&self, text: &str) -> Locator {
        // Use a special selector: __pw_filter_text=<base_selector>|<text>
        Locator::new(
            self.session.clone(),
            format!("__pw_filter_text={}|{}", self.selector, text),
        )
    }

    /// Combine with another locator using AND (intersection).
    /// The resulting locator matches elements that satisfy both selectors.
    pub fn and(&self, other: &Locator) -> Locator {
        // For CSS selectors, combining them as "selectorA selectorB" is scope,
        // but for AND on the same element, use :is()
        Locator::new(
            self.session.clone(),
            format!("{}:is({})", self.selector, other.selector),
        )
    }

    /// Combine with another locator using OR (union).
    /// The resulting locator matches elements that satisfy either selector.
    pub fn or(&self, other: &Locator) -> Locator {
        Locator::new(
            self.session.clone(),
            format!("{}, {}", self.selector, other.selector),
        )
    }

    // ── Internal ──

    async fn resolve_one(&self) -> CdpResult<selectors::ResolvedElement> {
        selectors::resolve_selector(&*self.session, &self.selector)
            .await?
            .ok_or_else(|| {
                CdpError::Other(format!("No element found for selector: {}", self.selector))
            })
    }

    async fn resolve_object_id(&self, node_id: i64) -> CdpResult<String> {
        selectors::resolve_object_id(&*self.session, node_id)
            .await?
            .ok_or_else(|| CdpError::Other("Could not resolve object ID".to_string()))
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
fn strip_outer_tag(html: &str) -> String {
    // Find the end of the opening tag
    if let Some(open_end) = html.find('>') {
        // Find the start of the closing tag
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
        assert_eq!(child.selector(), "ul li");

        let first = loc.first();
        assert_eq!(first.selector(), "ul:first-of-type");
    }

    #[test]
    fn test_strip_outer_tag() {
        assert_eq!(strip_outer_tag("<div>hello</div>"), "hello");
        assert_eq!(strip_outer_tag(r#"<div class="x">inner</div>"#), "inner");
        assert_eq!(strip_outer_tag("<span></span>"), "");
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
