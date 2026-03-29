//! Page — Playwright-compatible page interface.
//!
//! Wraps a CDP session with a high-level API matching Playwright's Page class.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use pwright_cdp::CdpClient;
use pwright_cdp::connection::Result as CdpResult;
use serde_json::Value;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use super::keyboard::Keyboard;
use super::locator::Locator;
use super::mouse::Mouse;
use super::network::{self, NetworkRequest, NetworkResponse};
use super::selectors::{SelectorKind, root_node_id};
use super::touchscreen::Touchscreen;

use crate::content::ScreenshotFormat;
use crate::navigate::WaitStrategy;

/// Options for `page.goto()`.
#[derive(Debug, Clone, Default)]
pub struct GotoOptions {
    /// Wait strategy for navigation.
    pub wait_until: WaitStrategy,
    /// Timeout in milliseconds.
    pub timeout_ms: Option<u64>,
}

/// Options for screenshots.
#[derive(Debug, Clone, Default)]
pub struct ScreenshotOptions {
    /// Screenshot format (PNG, JPEG with quality, WebP with quality).
    pub format: ScreenshotFormat,
    /// Capture the full page, not just the viewport.
    pub full_page: bool,
}

/// Playwright-compatible Page.
///
/// ```rust,ignore
/// let page = Page::new(session);
/// page.goto("https://example.com", None).await?;
/// page.locator("button").click().await?;
/// page.keyboard().press("Enter").await?;
/// ```
pub struct Page {
    session: Arc<dyn CdpClient>,
    target_id: Option<String>,
    closed: AtomicBool,
    /// Track spawned network listener tasks so they can be aborted on close/drop.
    listener_handles: Mutex<Vec<JoinHandle<()>>>,
}

impl Drop for Page {
    fn drop(&mut self) {
        // Abort any spawned listener tasks to prevent leaks.
        // try_lock avoids panic if Mutex is poisoned during unwinding.
        if let Ok(handles) = self.listener_handles.try_lock() {
            for handle in handles.iter() {
                handle.abort();
            }
        }
    }
}

impl Page {
    /// Create a new Page wrapping a CDP session.
    pub fn new(session: Arc<dyn CdpClient>) -> Self {
        Self {
            session,
            target_id: None,
            closed: AtomicBool::new(false),
            listener_handles: Mutex::new(Vec::new()),
        }
    }

    /// Create a Page wrapping a CDP session with a known target ID.
    /// Used by `TabHandle::page()` so `close()` can close the tab.
    pub fn with_tab(session: Arc<dyn CdpClient>, target_id: String) -> Self {
        Self {
            session,
            target_id: Some(target_id),
            closed: AtomicBool::new(false),
            listener_handles: Mutex::new(Vec::new()),
        }
    }

    /// The CDP target ID, if this page is backed by a tab.
    pub fn target_id(&self) -> Option<&str> {
        self.target_id.as_deref()
    }

    /// Close the page (tab). If created with `with_tab`, closes the underlying target.
    /// Also aborts any spawned network listener tasks. Thread-safe.
    pub async fn close(&self) -> CdpResult<()> {
        if self.closed.swap(true, Ordering::SeqCst) {
            return Ok(()); // already closed
        }
        for handle in self.listener_handles.lock().await.drain(..) {
            handle.abort();
        }
        if let Some(ref target_id) = self.target_id {
            self.session.target_close(target_id).await?;
        }
        Ok(())
    }

    /// Return an error if the page has been closed.
    fn ensure_open(&self) -> CdpResult<()> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(pwright_cdp::connection::CdpError::PageClosed);
        }
        Ok(())
    }

    // ── Navigation ──

    /// Navigate to a URL.
    pub async fn goto(&self, url: &str, options: Option<GotoOptions>) -> CdpResult<()> {
        self.ensure_open()?;
        let opts = options.unwrap_or_default();
        let timeout_ms = opts.timeout_ms.unwrap_or(30_000);

        let nav_opts = crate::navigate::NavigateOptions {
            wait_for: opts.wait_until,
            timeout: std::time::Duration::from_millis(timeout_ms),
            block_images: false,
            block_media: false,
        };

        let tab_id = self.target_id.as_deref().unwrap_or_default();
        crate::navigate::navigate(&*self.session, tab_id, url, &nav_opts).await?;
        Ok(())
    }

    /// Reload the current page.
    pub async fn reload(&self) -> CdpResult<()> {
        self.ensure_open()?;
        self.session.page_reload().await
    }

    /// Navigate back in history. Returns Ok(()) even if already at the
    /// beginning (no-op, matches Playwright behavior).
    pub async fn go_back(&self) -> CdpResult<()> {
        self.navigate_history(-1).await
    }

    /// Navigate forward in history. Returns Ok(()) even if already at the
    /// end (no-op, matches Playwright behavior).
    pub async fn go_forward(&self) -> CdpResult<()> {
        self.navigate_history(1).await
    }

    /// Navigate to a history entry by offset (-1 = back, +1 = forward).
    async fn navigate_history(&self, offset: i64) -> CdpResult<()> {
        self.ensure_open()?;
        let history = self.session.page_get_navigation_history().await?;
        let current = history
            .get("currentIndex")
            .and_then(|i| i.as_i64())
            .unwrap_or(0);
        let target = current + offset;
        if target < 0 {
            return Ok(());
        }
        let entries = history
            .get("entries")
            .and_then(|e| e.as_array())
            .cloned()
            .unwrap_or_default();
        if let Some(entry) = entries.get(target as usize) {
            let entry_id = entry.get("id").and_then(|id| id.as_i64()).unwrap_or(0);
            self.session
                .page_navigate_to_history_entry(entry_id)
                .await?;
        }
        Ok(())
    }

    // ── Content & State ──

    /// Evaluate a JS expression and extract the string result value.
    async fn eval_page_string(&self, js: &str) -> CdpResult<String> {
        self.ensure_open()?;
        let result = self.session.runtime_evaluate(js).await?;
        Ok(crate::evaluate::extract_result_value(&result)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string())
    }

    /// Get the current page URL.
    pub async fn url(&self) -> CdpResult<String> {
        self.eval_page_string(pwright_js::page::GET_LOCATION_HREF)
            .await
    }

    /// Get the page title.
    pub async fn title(&self) -> CdpResult<String> {
        self.eval_page_string(pwright_js::page::GET_TITLE).await
    }

    /// Get the full page HTML.
    pub async fn content(&self) -> CdpResult<String> {
        self.eval_page_string(pwright_js::page::GET_DOCUMENT_HTML)
            .await
    }

    /// Check if the page has been closed.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    /// Bring the page to front (activate tab).
    pub async fn bring_to_front(&self) -> CdpResult<()> {
        self.ensure_open()?;
        self.session.page_bring_to_front().await
    }

    // ── Evaluation ──

    /// Evaluate a JavaScript expression and return the result.
    pub async fn evaluate(&self, expression: &str) -> CdpResult<Value> {
        self.ensure_open()?;
        crate::evaluate::evaluate(&*self.session, expression).await
    }

    /// Evaluate a JavaScript expression and convert the result to a typed value.
    ///
    /// ```rust,ignore
    /// let title: String = page.evaluate_into("document.title").await?;
    /// let count: i64 = page.evaluate_into("document.querySelectorAll('a').length").await?;
    /// let ready: bool = page.evaluate_into("!!document.querySelector('.loaded')").await?;
    ///
    /// // For JSON.stringify results into arbitrary types:
    /// use pwright_bridge::evaluate::FromEvalJson;
    /// let items: FromEvalJson<Vec<Item>> = page.evaluate_into("JSON.stringify([...])").await?;
    /// ```
    pub async fn evaluate_into<T: crate::evaluate::FromEvalResult>(
        &self,
        expression: &str,
    ) -> CdpResult<T> {
        self.ensure_open()?;
        crate::evaluate::evaluate_into(&*self.session, expression).await
    }

    /// Evaluate a JS expression, awaiting any returned Promise.
    ///
    /// Use this for async JS like `fetch(...).then(r => r.text())`.
    pub async fn evaluate_async(&self, expression: &str) -> CdpResult<Value> {
        self.ensure_open()?;
        crate::evaluate::evaluate_async(&*self.session, expression).await
    }

    /// Evaluate a JS expression (Promise-aware) and convert to a typed value.
    ///
    /// ```rust,ignore
    /// let text: String = page.evaluate_async_into("fetch('/api').then(r => r.text())").await?;
    /// ```
    pub async fn evaluate_async_into<T: crate::evaluate::FromEvalResult>(
        &self,
        expression: &str,
    ) -> CdpResult<T> {
        self.ensure_open()?;
        crate::evaluate::evaluate_async_into(&*self.session, expression).await
    }

    // ── Screenshots & PDF ──

    /// Take a screenshot. Returns base64-encoded image data.
    pub async fn screenshot(&self, options: Option<ScreenshotOptions>) -> CdpResult<String> {
        self.ensure_open()?;
        let opts = options.unwrap_or_default();
        crate::content::take_screenshot(&*self.session, &opts.format, opts.full_page).await
    }

    /// Generate a PDF. Returns base64-encoded PDF data.
    pub async fn pdf(&self) -> CdpResult<String> {
        self.ensure_open()?;
        crate::content::get_pdf(&*self.session).await
    }

    /// Get the full page's visible text (`document.body.innerText`).
    pub async fn body_text(&self) -> CdpResult<String> {
        self.ensure_open()?;
        crate::content::get_text(&*self.session).await
    }

    // ── Locators ──

    /// Create a Locator for a CSS selector.
    pub fn locator(&self, selector: &str) -> Locator {
        Locator::new(self.session.clone(), selector)
    }

    // ── Selector-based convenience methods (Playwright Page API) ──

    /// Get the text content of an element matched by selector.
    pub async fn text_content(&self, selector: &str) -> CdpResult<Option<String>> {
        self.locator(selector).text_content().await
    }

    /// Click an element matched by selector.
    pub async fn click(&self, selector: &str) -> CdpResult<()> {
        self.locator(selector).click().await
    }

    /// Fill an input or textarea matched by selector.
    ///
    /// Does **not** work on `contenteditable` elements. See
    /// [`Locator::fill`] for details and the `keyboard().type_text()` alternative.
    pub async fn fill(&self, selector: &str, value: &str) -> CdpResult<()> {
        self.locator(selector).fill(value).await
    }

    /// Type text into an element matched by selector.
    pub async fn type_text(&self, selector: &str, text: &str) -> CdpResult<()> {
        self.locator(selector).type_text(text).await
    }

    /// Press a key on an element matched by selector.
    pub async fn press(&self, selector: &str, key: &str) -> CdpResult<()> {
        self.locator(selector).press(key).await
    }

    /// Hover over an element matched by selector.
    pub async fn hover(&self, selector: &str) -> CdpResult<()> {
        self.locator(selector).hover().await
    }

    /// Focus an element matched by selector.
    pub async fn focus(&self, selector: &str) -> CdpResult<()> {
        self.locator(selector).focus().await
    }

    /// Check if an element matched by selector is visible.
    pub async fn is_visible(&self, selector: &str) -> CdpResult<bool> {
        self.locator(selector).is_visible().await
    }

    /// Get an attribute value of an element matched by selector.
    pub async fn get_attribute(&self, selector: &str, name: &str) -> CdpResult<Option<String>> {
        self.locator(selector).get_attribute(name).await
    }

    /// Get the inner HTML of an element matched by selector.
    pub async fn inner_html(&self, selector: &str) -> CdpResult<String> {
        self.locator(selector).inner_html().await
    }

    /// Get the inner text of an element matched by selector.
    pub async fn inner_text(&self, selector: &str) -> CdpResult<String> {
        self.locator(selector).inner_text().await
    }

    /// Check if an element matched by selector is hidden.
    pub async fn is_hidden(&self, selector: &str) -> CdpResult<bool> {
        self.locator(selector).is_hidden().await
    }

    /// Check if an element matched by selector is checked.
    pub async fn is_checked(&self, selector: &str) -> CdpResult<bool> {
        self.locator(selector).is_checked().await
    }

    /// Check if an element matched by selector is disabled.
    pub async fn is_disabled(&self, selector: &str) -> CdpResult<bool> {
        self.locator(selector).is_disabled().await
    }

    /// Check if an element matched by selector is enabled.
    pub async fn is_enabled(&self, selector: &str) -> CdpResult<bool> {
        self.locator(selector).is_enabled().await
    }

    /// Get the input value of an element matched by selector.
    pub async fn input_value(&self, selector: &str) -> CdpResult<String> {
        self.locator(selector).input_value().await
    }

    /// Check a checkbox matched by selector.
    pub async fn check(&self, selector: &str) -> CdpResult<()> {
        self.locator(selector).check().await
    }

    /// Uncheck a checkbox matched by selector.
    pub async fn uncheck(&self, selector: &str) -> CdpResult<()> {
        self.locator(selector).uncheck().await
    }

    /// Select an option by value on a `<select>` element matched by selector.
    pub async fn select_option(&self, selector: &str, value: &str) -> CdpResult<()> {
        self.locator(selector).select_option(value).await
    }

    /// Double-click an element matched by selector.
    pub async fn dblclick(&self, selector: &str) -> CdpResult<()> {
        self.locator(selector).dblclick().await
    }

    /// Dispatch a custom event on an element matched by selector.
    pub async fn dispatch_event(&self, selector: &str, event_type: &str) -> CdpResult<()> {
        self.locator(selector).dispatch_event(event_type).await
    }

    /// Tap an element matched by selector (touchscreen).
    pub async fn tap(&self, selector: &str) -> CdpResult<()> {
        let loc = self.locator(selector);
        let bbox = loc.bounding_box().await?.ok_or_else(|| {
            pwright_cdp::connection::CdpError::ElementNotFound {
                selector: selector.to_string(),
            }
        })?;
        let x = bbox.x + bbox.width / 2.0;
        let y = bbox.y + bbox.height / 2.0;
        self.touchscreen().tap(x, y).await
    }

    /// Wait for a selector to appear in the DOM.
    pub async fn wait_for_selector(&self, selector: &str, timeout_ms: u64) -> CdpResult<()> {
        self.locator(selector)
            .wait_for(timeout_ms, super::locator::WaitState::Attached)
            .await
            .map_err(|e| match e {
                pwright_cdp::connection::CdpError::Timeout => {
                    pwright_cdp::connection::CdpError::Other(format!(
                        "Timeout waiting for selector '{selector}' ({timeout_ms}ms)"
                    ))
                }
                other => other,
            })
    }

    /// Set files on a file `<input>` element matched by CSS selector.
    ///
    /// See [`Locator::set_input_files`] for important notes about remote Chrome.
    pub async fn set_input_files(&self, selector: &str, files: &[String]) -> CdpResult<()> {
        self.locator(selector).set_input_files(files).await
    }

    /// Find element by test ID attribute (`[data-testid="..."]`).
    pub fn get_by_test_id(&self, test_id: &str) -> Locator {
        let escaped = super::selectors::css_escape_attr(test_id);
        Locator::new(
            self.session.clone(),
            format!(r#"[data-testid="{escaped}"]"#),
        )
    }

    /// Find element by placeholder attribute.
    pub fn get_by_placeholder(&self, placeholder: &str) -> Locator {
        let escaped = super::selectors::css_escape_attr(placeholder);
        Locator::new(
            self.session.clone(),
            format!(r#"[placeholder="{escaped}"]"#),
        )
    }

    /// Find element by alt text attribute.
    pub fn get_by_alt_text(&self, alt: &str) -> Locator {
        let escaped = super::selectors::css_escape_attr(alt);
        Locator::new(self.session.clone(), format!(r#"[alt="{escaped}"]"#))
    }

    /// Find element by title attribute.
    pub fn get_by_title(&self, title: &str) -> Locator {
        let escaped = super::selectors::css_escape_attr(title);
        Locator::new(self.session.clone(), format!(r#"[title="{escaped}"]"#))
    }

    /// Find element by text content (substring match).
    pub fn get_by_text(&self, text: &str, exact: bool) -> Locator {
        let kind = if exact {
            SelectorKind::TextExact(text.to_string())
        } else {
            SelectorKind::Text(text.to_string())
        };
        Locator::new_with_kind(self.session.clone(), kind)
    }

    /// Find element by label text (via `<label>` for/wrapping or `aria-label`).
    pub fn get_by_label(&self, text: &str) -> Locator {
        Locator::new_with_kind(self.session.clone(), SelectorKind::Label(text.to_string()))
    }

    /// Find element by ARIA role, with optional accessible name filter.
    pub fn get_by_role(&self, role: &str, name: Option<&str>) -> Locator {
        Locator::new_with_kind(
            self.session.clone(),
            SelectorKind::Role {
                role: role.to_string(),
                name: name.map(|n| n.to_string()),
            },
        )
    }

    // ── Input devices ──

    /// Get the Keyboard interface.
    pub fn keyboard(&self) -> Keyboard {
        Keyboard::new(self.session.clone())
    }

    /// Get the Mouse interface.
    pub fn mouse(&self) -> Mouse {
        Mouse::new(self.session.clone())
    }

    /// Get the Touchscreen interface.
    pub fn touchscreen(&self) -> Touchscreen {
        Touchscreen::new(self.session.clone())
    }

    /// Subscribe to a network event, enabling the Network domain and returning a
    /// channel that receives parsed values for each matching CDP event.
    ///
    /// The listener task is tracked and will be aborted when `close()` is called.
    async fn subscribe_network_event<T: Send + 'static>(
        &self,
        event_method: &str,
        parser: fn(&serde_json::Value) -> Option<T>,
    ) -> CdpResult<tokio::sync::mpsc::Receiver<T>> {
        self.ensure_open()?;
        // Subscribe BEFORE enabling so no events are missed
        let mut event_rx = self.session.subscribe_events();
        self.session.network_enable().await?;
        let (tx, rx) = tokio::sync::mpsc::channel(256);
        let method = event_method.to_string();

        let handle = tokio::spawn(async move {
            loop {
                match event_rx.recv().await {
                    Ok(event) => {
                        if event.method == method
                            && let Some(parsed) = parser(&event.params)
                            && tx.send(parsed).await.is_err()
                        {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        self.listener_handles.lock().await.push(handle);

        Ok(rx)
    }

    /// Subscribe to network responses. Enables the Network domain and returns
    /// a channel that receives `NetworkResponse` for each `Network.responseReceived` event.
    ///
    /// The listener task is tracked and will be aborted when `close()` is called.
    pub async fn on_response(&self) -> CdpResult<tokio::sync::mpsc::Receiver<NetworkResponse>> {
        self.subscribe_network_event("Network.responseReceived", network::parse_network_response)
            .await
    }

    /// Subscribe to network requests. Enables the Network domain and returns
    /// a channel that receives `NetworkRequest` for each `Network.requestWillBeSent` event.
    ///
    /// The listener task is tracked and will be aborted when `close()` is called.
    pub async fn on_request(&self) -> CdpResult<tokio::sync::mpsc::Receiver<NetworkRequest>> {
        self.subscribe_network_event("Network.requestWillBeSent", network::parse_network_request)
            .await
    }

    /// Get the response body for a captured network response.
    ///
    /// Use with `on_response()` -- the `request_id` comes from `NetworkResponse`.
    ///
    /// ```rust,ignore
    /// let mut rx = page.on_response().await?;
    /// page.goto(url, None).await?;
    /// while let Some(resp) = rx.recv().await {
    ///     if resp.url.contains("/api/data") {
    ///         let body = page.response_body(&resp.request_id).await?;
    ///         println!("{}", body.body);
    ///     }
    /// }
    /// ```
    pub async fn response_body(
        &self,
        request_id: &str,
    ) -> CdpResult<pwright_cdp::domains::network::ResponseBody> {
        self.ensure_open()?;
        self.session.network_get_response_body(request_id).await
    }

    /// Wait for a network response matching a predicate.
    ///
    /// Enables the Network domain, listens for responses, and returns the first
    /// one where `predicate` returns true. Times out with `CdpError::Timeout`.
    ///
    /// ```rust,ignore
    /// page.goto(url, None).await?;
    /// let resp = page.wait_for_response(
    ///     |r| r.url.contains("/api/search"),
    ///     30_000,
    /// ).await?;
    /// let body = page.response_body(&resp.request_id).await?;
    /// ```
    pub async fn wait_for_response<F>(
        &self,
        predicate: F,
        timeout_ms: u64,
    ) -> CdpResult<NetworkResponse>
    where
        F: Fn(&NetworkResponse) -> bool,
    {
        self.ensure_open()?;
        let mut rx = self.on_response().await?;
        tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), async move {
            while let Some(resp) = rx.recv().await {
                if predicate(&resp) {
                    return Ok(resp);
                }
            }
            Err(pwright_cdp::connection::CdpError::Other(
                "channel closed before matching response".to_string(),
            ))
        })
        .await
        .map_err(|_| {
            pwright_cdp::connection::CdpError::Other(format!(
                "timeout waiting for response ({timeout_ms}ms)"
            ))
        })?
    }

    /// Wait for a network request matching a predicate.
    ///
    /// Enables the Network domain, listens for requests, and returns the first
    /// one where `predicate` returns true. Times out with `CdpError::Timeout`.
    pub async fn wait_for_request<F>(
        &self,
        predicate: F,
        timeout_ms: u64,
    ) -> CdpResult<NetworkRequest>
    where
        F: Fn(&NetworkRequest) -> bool,
    {
        self.ensure_open()?;
        let mut rx = self.on_request().await?;
        tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), async move {
            while let Some(req) = rx.recv().await {
                if predicate(&req) {
                    return Ok(req);
                }
            }
            Err(pwright_cdp::connection::CdpError::Other(
                "channel closed before matching request".to_string(),
            ))
        })
        .await
        .map_err(|_| {
            pwright_cdp::connection::CdpError::Other(format!(
                "timeout waiting for request ({timeout_ms}ms)"
            ))
        })?
    }

    /// Evaluate a JavaScript function with a serialized argument.
    ///
    /// Unlike `evaluate()` which requires string interpolation, this passes
    /// the argument via CDP serialization -- no escaping needed, no injection risk.
    ///
    /// ```rust,ignore
    /// let result = page.evaluate_with_arg(
    ///     "function(url) { return fetch(url).then(r => r.text()); }",
    ///     &serde_json::json!("https://example.com/api"),
    /// ).await?;
    /// ```
    pub async fn evaluate_with_arg(&self, function_body: &str, arg: &Value) -> CdpResult<Value> {
        self.ensure_open()?;
        // Get a DOM node objectId to anchor callFunctionOn.
        // We use DOM.getDocument + DOM.resolveNode which returns an objectId
        // without trying to serialize the entire global scope.
        let doc = self.session.dom_get_document().await?;
        let root_id = root_node_id(&doc)?;
        let resolved = self.session.dom_resolve_node(root_id).await?;
        let object_id = resolved
            .get("object")
            .and_then(|o| o.get("objectId"))
            .and_then(|id| id.as_str())
            .ok_or_else(|| {
                pwright_cdp::connection::CdpError::Other(
                    "could not resolve document objectId".to_string(),
                )
            })?;
        let result = self
            .session
            .runtime_call_function_on(
                object_id,
                function_body,
                vec![serde_json::json!({"value": arg})],
            )
            .await?;
        Ok(result.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Wait for a specified duration.
    pub async fn wait_for_timeout(&self, ms: u64) -> CdpResult<()> {
        tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        Ok(())
    }

    /// Wait until the page body contains the given text (substring match).
    ///
    /// Polls `document.body.innerText` every 200ms until the text is found
    /// or the timeout expires.
    ///
    /// ```rust,ignore
    /// page.goto("https://example.com", None).await?;
    /// page.wait_for_text("Results", 30_000).await?;
    /// ```
    pub async fn wait_for_text(&self, text: &str, timeout_ms: u64) -> CdpResult<()> {
        self.ensure_open()?;
        const POLL_MS: u64 = 200;
        let js = format!(
            "document.body && document.body.innerText.includes({})",
            serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string())
        );
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(POLL_MS)).await;

            if let Ok(result) = self.session.runtime_evaluate(&js).await
                && result
                    .get("result")
                    .and_then(|r| r.get("value"))
                    .and_then(|v| v.as_bool())
                    == Some(true)
            {
                return Ok(());
            }

            if tokio::time::Instant::now() > deadline {
                return Err(pwright_cdp::connection::CdpError::Other(format!(
                    "Timeout waiting for text '{}' ({timeout_ms}ms)",
                    text
                )));
            }
        }
    }

    /// Wait until an element's text contains the given substring.
    ///
    /// Like [`wait_for_text`](Self::wait_for_text) but scoped to the element
    /// matched by `selector` instead of the full page body.
    ///
    /// ```rust,ignore
    /// page.wait_for_text_in(".response", "complete", 30_000).await?;
    /// ```
    pub async fn wait_for_text_in(
        &self,
        selector: &str,
        text: &str,
        timeout_ms: u64,
    ) -> CdpResult<()> {
        self.ensure_open()?;
        const POLL_MS: u64 = 200;
        let js = format!(
            "(function() {{ var el = document.querySelector({}); return el && el.innerText.includes({}); }})()",
            serde_json::to_string(selector).unwrap_or_else(|_| "\"\"".to_string()),
            serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string())
        );
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(POLL_MS)).await;

            if let Ok(result) = self.session.runtime_evaluate(&js).await
                && result
                    .get("result")
                    .and_then(|r| r.get("value"))
                    .and_then(|v| v.as_bool())
                    == Some(true)
            {
                return Ok(());
            }

            if tokio::time::Instant::now() > deadline {
                return Err(pwright_cdp::connection::CdpError::Other(format!(
                    "Timeout waiting for text '{}' in '{}' ({timeout_ms}ms)",
                    text, selector
                )));
            }
        }
    }

    /// Wait until a JavaScript expression returns a truthy value.
    ///
    /// Polls the expression every 200ms until it returns truthy or the
    /// timeout expires.
    ///
    /// ```rust,ignore
    /// page.wait_until("document.title !== ''", 10_000).await?;
    /// page.wait_until("document.querySelectorAll('.item').length > 5", 30_000).await?;
    /// ```
    pub async fn wait_until(&self, expression: &str, timeout_ms: u64) -> CdpResult<()> {
        self.ensure_open()?;
        const POLL_MS: u64 = 200;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);

        loop {
            tokio::time::sleep(std::time::Duration::from_millis(POLL_MS)).await;

            if let Ok(result) = self.session.runtime_evaluate(expression).await {
                let value = result.get("result").and_then(|r| r.get("value"));
                // Truthy: true, non-zero number, non-empty string
                let is_truthy = match value {
                    Some(v) if v.is_boolean() => v.as_bool() == Some(true),
                    Some(v) if v.is_number() => v.as_f64() != Some(0.0),
                    Some(v) if v.is_string() => !v.as_str().unwrap_or("").is_empty(),
                    Some(v) if v.is_null() => false,
                    Some(_) => true, // objects, arrays
                    None => false,
                };
                if is_truthy {
                    return Ok(());
                }
            }

            if tokio::time::Instant::now() > deadline {
                return Err(pwright_cdp::connection::CdpError::Other(format!(
                    "Timeout waiting for expression to be truthy ({timeout_ms}ms): {expression}"
                )));
            }
        }
    }

    /// Wait for a download to complete while executing the given action.
    ///
    /// The download is saved to the system's temporary directory.
    /// Returns the absolute path to the downloaded file.
    pub async fn expect_download<F, Fut>(&self, action: F) -> CdpResult<String>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = CdpResult<()>>,
    {
        self.ensure_open()?;

        // 1. Subscribe to events before allowing downloads to avoid race conditions.
        let mut rx = self.session.subscribe_events();

        // 2. Enable downloads globally to temp dir.
        let temp_dir = std::env::temp_dir().to_string_lossy().to_string();
        self.session
            .browser_set_download_behavior(
                pwright_cdp::DownloadBehavior::AllowAndName,
                Some(&temp_dir),
                true,
            )
            .await?;

        // 3. Execute the action that triggers the download.
        action().await?;

        // 4. Wait for the download to complete.
        loop {
            // Give it up to 30 seconds for a download to complete (or at least start).
            match tokio::time::timeout(std::time::Duration::from_secs(30), rx.recv()).await {
                Ok(Ok(event)) => {
                    if event.method == "Browser.downloadProgress" {
                        if event.params["state"] == "completed" {
                            if let Some(file_path) = event.params["filePath"].as_str() {
                                return Ok(file_path.to_string());
                            }
                        } else if event.params["state"] == "canceled" {
                            return Err(pwright_cdp::connection::CdpError::Other(
                                "Download canceled".to_string(),
                            ));
                        }
                    }
                }
                Ok(Err(_)) => {
                    return Err(pwright_cdp::connection::CdpError::Other(
                        "Event stream closed".to_string(),
                    ));
                }
                Err(_) => {
                    return Err(pwright_cdp::connection::CdpError::Timeout);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockCdpClient;

    #[tokio::test]
    async fn test_page_reload() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock.clone());
        page.reload().await.unwrap();

        assert_eq!(mock.method_names(), vec!["Page.reload"]);
    }

    #[tokio::test]
    async fn test_page_go_back() {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_navigation_history_response(serde_json::json!({
            "currentIndex": 1,
            "entries": [
                {"id": 0, "url": "https://a.com", "title": "A"},
                {"id": 1, "url": "https://b.com", "title": "B"},
            ]
        }));

        let page = Page::new(mock.clone());
        page.go_back().await.unwrap();

        let methods = mock.method_names();
        assert!(methods.contains(&"Page.getNavigationHistory".to_string()));
        assert!(methods.contains(&"Page.navigateToHistoryEntry".to_string()));
    }

    #[tokio::test]
    async fn test_page_go_back_at_start_is_noop() {
        let mock = Arc::new(MockCdpClient::new());
        // currentIndex is 0, so go_back should be a no-op
        let page = Page::new(mock.clone());
        page.go_back().await.unwrap();

        let methods = mock.method_names();
        assert!(methods.contains(&"Page.getNavigationHistory".to_string()));
        // Should NOT have navigated
        assert!(!methods.contains(&"Page.navigateToHistoryEntry".to_string()));
    }

    #[tokio::test]
    async fn test_page_url() {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_evaluate_response(serde_json::json!({
            "result": {"value": "https://example.com/page"}
        }));

        let page = Page::new(mock.clone());
        let url = page.url().await.unwrap();
        assert_eq!(url, "https://example.com/page");
    }

    #[tokio::test]
    async fn test_page_title() {
        let mock = Arc::new(MockCdpClient::new());
        mock.set_evaluate_response(serde_json::json!({
            "result": {"value": "My Page Title"}
        }));

        let page = Page::new(mock.clone());
        let title = page.title().await.unwrap();
        assert_eq!(title, "My Page Title");
    }

    #[tokio::test]
    async fn test_page_bring_to_front() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock.clone());
        page.bring_to_front().await.unwrap();

        assert_eq!(mock.method_names(), vec!["Page.bringToFront"]);
    }

    #[tokio::test]
    async fn test_page_is_closed_default() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock);
        assert!(!page.is_closed());
    }

    #[tokio::test]
    async fn test_page_locator_creates_selector() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock);
        let loc = page.locator("button.submit");
        assert_eq!(loc.selector().to_string(), "button.submit");
    }

    #[tokio::test]
    async fn test_page_get_by_test_id() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock);
        let loc = page.get_by_test_id("login-btn");
        assert_eq!(loc.selector().to_string(), r#"[data-testid="login-btn"]"#);
    }

    #[tokio::test]
    async fn test_page_get_by_placeholder() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock);
        let loc = page.get_by_placeholder("Enter email");
        assert_eq!(loc.selector().to_string(), r#"[placeholder="Enter email"]"#);
    }

    #[tokio::test]
    async fn test_page_close_with_target_id() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::with_tab(mock.clone(), "target-123".to_string());
        page.close().await.unwrap();

        let methods = mock.method_names();
        assert!(methods.contains(&"Target.closeTarget".to_string()));
        assert!(page.is_closed());
    }

    #[tokio::test]
    async fn test_page_close_without_target_id_is_noop() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock.clone());
        page.close().await.unwrap();

        // Should not call Target.closeTarget
        assert!(mock.method_names().is_empty());
        assert!(page.is_closed());
    }

    #[tokio::test]
    async fn test_page_close_idempotent() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::with_tab(mock.clone(), "target-123".to_string());
        page.close().await.unwrap();
        page.close().await.unwrap();

        // Should only call close once
        let close_calls = mock.calls_for("Target.closeTarget");
        assert_eq!(close_calls.len(), 1);
    }

    #[tokio::test]
    async fn test_page_on_response() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock.clone());
        let mut rx = page.on_response().await.unwrap();

        // Verify Network.enable was called
        assert!(mock.method_names().contains(&"Network.enable".to_string()));

        // Inject a response event
        mock.send_event(pwright_cdp::CdpEvent {
            method: "Network.responseReceived".to_string(),
            params: serde_json::json!({
                "requestId": "req-1",
                "response": {
                    "url": "https://example.com/api",
                    "status": 200,
                    "statusText": "OK",
                    "headers": {},
                    "mimeType": "text/html"
                }
            }),
            session_id: None,
        });

        let resp = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(resp.request_id, "req-1");
        assert_eq!(resp.url, "https://example.com/api");
        assert_eq!(resp.status, 200);
    }

    #[tokio::test]
    async fn test_page_on_request() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock.clone());
        let mut rx = page.on_request().await.unwrap();

        mock.send_event(pwright_cdp::CdpEvent {
            method: "Network.requestWillBeSent".to_string(),
            params: serde_json::json!({
                "requestId": "req-3",
                "request": {
                    "url": "https://example.com/submit",
                    "method": "POST",
                    "headers": {}
                },
                "type": "Fetch"
            }),
            session_id: None,
        });

        let req = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(req.request_id, "req-3");
        assert_eq!(req.method, "POST");
        assert_eq!(req.resource_type, "Fetch");
    }

    #[tokio::test]
    async fn test_closed_page_rejects_operations() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock.clone());
        page.close().await.unwrap();

        assert!(page.goto("https://example.com", None).await.is_err());
        assert!(page.reload().await.is_err());
        assert!(page.url().await.is_err());
        assert!(page.evaluate("1+1").await.is_err());
        assert!(page.screenshot(None).await.is_err());

        // No CDP calls should have been made
        assert!(mock.method_names().is_empty());
    }

    #[tokio::test]
    async fn test_response_body() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock.clone());

        let body = page.response_body("req-42").await.unwrap();

        let calls = mock.calls_for("Network.getResponseBody");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].args[0]["requestId"], "req-42");
        // MockCdpClient returns empty body by default
        assert_eq!(body.body, "");
    }

    #[tokio::test]
    async fn test_wait_for_response_matches() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock.clone());

        // Inject a matching response event after a short delay
        let mock2 = mock.clone();
        tokio::spawn(async move {
            tokio::task::yield_now().await;
            mock2.send_event(pwright_cdp::CdpEvent {
                method: "Network.responseReceived".to_string(),
                params: serde_json::json!({
                    "requestId": "req-api",
                    "response": {
                        "url": "https://example.com/api/search",
                        "status": 200,
                        "statusText": "OK",
                        "headers": {},
                        "mimeType": "application/json"
                    }
                }),
                session_id: None,
            });
        });

        let resp = page
            .wait_for_response(|r| r.url.contains("/api/search"), 5000)
            .await
            .unwrap();

        assert_eq!(resp.request_id, "req-api");
        assert_eq!(resp.status, 200);
    }

    #[tokio::test]
    async fn test_wait_for_response_skips_non_matching() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock.clone());

        let mock2 = mock.clone();
        tokio::spawn(async move {
            tokio::task::yield_now().await;
            // First: non-matching
            mock2.send_event(pwright_cdp::CdpEvent {
                method: "Network.responseReceived".to_string(),
                params: serde_json::json!({
                    "requestId": "req-css",
                    "response": {
                        "url": "https://example.com/style.css",
                        "status": 200,
                        "statusText": "OK",
                        "headers": {},
                        "mimeType": "text/css"
                    }
                }),
                session_id: None,
            });
            // Second: matching
            mock2.send_event(pwright_cdp::CdpEvent {
                method: "Network.responseReceived".to_string(),
                params: serde_json::json!({
                    "requestId": "req-api",
                    "response": {
                        "url": "https://example.com/api/data",
                        "status": 200,
                        "statusText": "OK",
                        "headers": {},
                        "mimeType": "application/json"
                    }
                }),
                session_id: None,
            });
        });

        let resp = page
            .wait_for_response(|r| r.url.contains("/api/"), 5000)
            .await
            .unwrap();

        // Should have skipped the CSS response
        assert_eq!(resp.request_id, "req-api");
    }

    #[tokio::test]
    async fn test_wait_for_response_timeout() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock.clone());

        // No events injected, should timeout
        let result = page
            .wait_for_response(|r| r.url.contains("/never"), 100)
            .await;

        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("timeout"), "Error: {err}");
    }

    #[tokio::test]
    async fn test_wait_for_request_matches() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock.clone());

        let mock2 = mock.clone();
        tokio::spawn(async move {
            tokio::task::yield_now().await;
            mock2.send_event(pwright_cdp::CdpEvent {
                method: "Network.requestWillBeSent".to_string(),
                params: serde_json::json!({
                    "requestId": "req-post",
                    "request": {
                        "url": "https://example.com/api/submit",
                        "method": "POST",
                        "headers": {}
                    },
                    "type": "XHR"
                }),
                session_id: None,
            });
        });

        let req = page
            .wait_for_request(|r| r.method == "POST", 5000)
            .await
            .unwrap();

        assert_eq!(req.request_id, "req-post");
        assert_eq!(req.url, "https://example.com/api/submit");
    }

    #[tokio::test]
    async fn test_evaluate_with_arg() {
        let mock = Arc::new(MockCdpClient::new());
        // callFunctionOn returns the result
        mock.set_call_function_response(serde_json::json!({
            "result": {"type": "string", "value": "hello world"}
        }));

        let page = Page::new(mock.clone());
        let result = page
            .evaluate_with_arg(
                "function(name) { return 'hello ' + name; }",
                &serde_json::json!("world"),
            )
            .await
            .unwrap();

        assert_eq!(result["value"], "hello world");

        // Verify callFunctionOn was called with an objectId from DOM.resolveNode
        let cf_calls = mock.calls_for("Runtime.callFunctionOn");
        assert_eq!(cf_calls.len(), 1);
        // The objectId comes from mock's dom_resolve_node default ("mock-obj-1")
        assert_eq!(cf_calls[0].args[0]["objectId"], "mock-obj-1");
        assert_eq!(
            cf_calls[0].args[0]["functionDeclaration"],
            "function(name) { return 'hello ' + name; }"
        );
        // Arg should be serialized
        let args = cf_calls[0].args[0]["arguments"].as_array().unwrap();
        assert_eq!(args[0]["value"], "world");
    }

    #[test]
    fn test_page_target_id_with_tab() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::with_tab(mock, "target-123".to_string());
        assert_eq!(page.target_id(), Some("target-123"));
    }

    #[test]
    fn test_page_target_id_without_tab() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock);
        assert_eq!(page.target_id(), None);
    }

    #[tokio::test]
    async fn test_evaluate_with_arg_closed_page() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock.clone());
        page.close().await.unwrap();

        let result = page
            .evaluate_with_arg("function(x) { return x; }", &serde_json::json!(1))
            .await;
        assert!(result.is_err());
    }
}
