//! Page — Playwright-compatible page interface.
//!
//! Wraps a CDP session with a high-level API matching Playwright's Page class.

use std::sync::Arc;

use pwright_cdp::CdpClient;
use pwright_cdp::connection::Result as CdpResult;
use serde_json::Value;

use super::keyboard::Keyboard;
use super::locator::Locator;
use super::mouse::Mouse;
use super::touchscreen::Touchscreen;

/// Options for `page.goto()`.
#[derive(Debug, Clone, Default)]
pub struct GotoOptions {
    /// Wait strategy: "load", "domcontentloaded", "networkidle".
    pub wait_until: Option<String>,
    /// Timeout in milliseconds.
    pub timeout_ms: Option<u64>,
}

/// Options for screenshots.
#[derive(Debug, Clone, Default)]
pub struct ScreenshotOptions {
    /// Image format: "png" (default) or "jpeg".
    pub format: Option<String>,
    /// JPEG quality (0-100).
    pub quality: Option<i32>,
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
    closed: bool,
}

impl Page {
    /// Create a new Page wrapping a CDP session.
    pub fn new(session: Arc<dyn CdpClient>) -> Self {
        Self {
            session,
            closed: false,
        }
    }

    // ── Navigation ──

    /// Navigate to a URL.
    pub async fn goto(&self, url: &str, options: Option<GotoOptions>) -> CdpResult<()> {
        let opts = options.unwrap_or_default();
        let timeout_ms = opts.timeout_ms.unwrap_or(30_000);

        let wait_for = match opts.wait_until.as_deref() {
            Some("domcontentloaded") => crate::navigate::WaitStrategy::Dom,
            Some("networkidle") => crate::navigate::WaitStrategy::NetworkIdle,
            _ => crate::navigate::WaitStrategy::None,
        };

        let nav_opts = crate::navigate::NavigateOptions {
            wait_for,
            timeout: std::time::Duration::from_millis(timeout_ms),
            block_images: false,
            block_media: false,
        };

        crate::navigate::navigate(&*self.session, "", url, &nav_opts).await?;
        Ok(())
    }

    /// Reload the current page.
    pub async fn reload(&self) -> CdpResult<()> {
        self.session.page_reload().await
    }

    /// Navigate back in history.
    pub async fn go_back(&self) -> CdpResult<()> {
        let history = self.session.page_get_navigation_history().await?;
        let current_index = history
            .get("currentIndex")
            .and_then(|i| i.as_i64())
            .unwrap_or(0);

        if current_index > 0 {
            let entries = history
                .get("entries")
                .and_then(|e| e.as_array())
                .cloned()
                .unwrap_or_default();
            if let Some(entry) = entries.get((current_index - 1) as usize) {
                let entry_id = entry.get("id").and_then(|id| id.as_i64()).unwrap_or(0);
                self.session
                    .page_navigate_to_history_entry(entry_id)
                    .await?;
            }
        }
        Ok(())
    }

    /// Navigate forward in history.
    pub async fn go_forward(&self) -> CdpResult<()> {
        let history = self.session.page_get_navigation_history().await?;
        let current_index = history
            .get("currentIndex")
            .and_then(|i| i.as_i64())
            .unwrap_or(0);
        let entries = history
            .get("entries")
            .and_then(|e| e.as_array())
            .cloned()
            .unwrap_or_default();

        if (current_index + 1) < entries.len() as i64
            && let Some(entry) = entries.get((current_index + 1) as usize)
        {
            let entry_id = entry.get("id").and_then(|id| id.as_i64()).unwrap_or(0);
            self.session
                .page_navigate_to_history_entry(entry_id)
                .await?;
        }
        Ok(())
    }

    // ── Content & State ──

    /// Get the current page URL.
    pub async fn url(&self) -> CdpResult<String> {
        let result = self
            .session
            .runtime_evaluate(pwright_js::page::GET_LOCATION_HREF)
            .await?;
        Ok(result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string())
    }

    /// Get the page title.
    pub async fn title(&self) -> CdpResult<String> {
        let result = self
            .session
            .runtime_evaluate(pwright_js::page::GET_TITLE)
            .await?;
        Ok(result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string())
    }

    /// Get the full page HTML.
    pub async fn content(&self) -> CdpResult<String> {
        let result = self
            .session
            .runtime_evaluate(pwright_js::page::GET_DOCUMENT_HTML)
            .await?;
        Ok(result
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string())
    }

    /// Check if the page has been closed.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// Bring the page to front (activate tab).
    pub async fn bring_to_front(&self) -> CdpResult<()> {
        self.session.page_bring_to_front().await
    }

    // ── Evaluation ──

    /// Evaluate a JavaScript expression and return the result.
    pub async fn evaluate(&self, expression: &str) -> CdpResult<Value> {
        crate::evaluate::evaluate(&*self.session, expression).await
    }

    // ── Screenshots & PDF ──

    /// Take a screenshot. Returns base64-encoded image data.
    pub async fn screenshot(&self, options: Option<ScreenshotOptions>) -> CdpResult<String> {
        let opts = options.unwrap_or_default();
        let format = match opts.format.as_deref() {
            Some("jpeg") => crate::content::ScreenshotFormat::Jpeg(opts.quality.unwrap_or(80)),
            Some("webp") => crate::content::ScreenshotFormat::Webp(opts.quality.unwrap_or(80)),
            _ => crate::content::ScreenshotFormat::Png,
        };
        crate::content::take_screenshot(&*self.session, &format, opts.full_page).await
    }

    /// Generate a PDF. Returns base64-encoded PDF data.
    pub async fn pdf(&self) -> CdpResult<String> {
        crate::content::get_pdf(&*self.session).await
    }

    /// Get visible text content.
    pub async fn text_content(&self) -> CdpResult<String> {
        crate::content::get_text(&*self.session).await
    }

    // ── Locators ──

    /// Create a Locator for a CSS selector.
    pub fn locator(&self, selector: &str) -> Locator {
        Locator::new(self.session.clone(), selector)
    }

    /// Set files on a file input element matched by CSS selector.
    pub async fn set_input_files(&self, selector: &str, files: &[String]) -> CdpResult<()> {
        self.locator(selector).set_input_files(files).await
    }

    /// Find element by test ID attribute (`[data-testid="..."]`).
    pub fn get_by_test_id(&self, test_id: &str) -> Locator {
        Locator::new(
            self.session.clone(),
            format!(r#"[data-testid="{}"]"#, test_id),
        )
    }

    /// Find element by placeholder attribute.
    pub fn get_by_placeholder(&self, placeholder: &str) -> Locator {
        Locator::new(
            self.session.clone(),
            format!(r#"[placeholder="{}"]"#, placeholder),
        )
    }

    /// Find element by alt text attribute.
    pub fn get_by_alt_text(&self, alt: &str) -> Locator {
        Locator::new(self.session.clone(), format!(r#"[alt="{}"]"#, alt))
    }

    /// Find element by title attribute.
    pub fn get_by_title(&self, title: &str) -> Locator {
        Locator::new(self.session.clone(), format!(r#"[title="{}"]"#, title))
    }

    /// Find element by text content (substring match).
    pub fn get_by_text(&self, text: &str, exact: bool) -> Locator {
        if exact {
            Locator::new(self.session.clone(), format!("__pw_text_exact={}", text))
        } else {
            Locator::new(self.session.clone(), format!("__pw_text={}", text))
        }
    }

    /// Find element by label text (via `<label>` for/wrapping or `aria-label`).
    pub fn get_by_label(&self, text: &str) -> Locator {
        Locator::new(self.session.clone(), format!("__pw_label={}", text))
    }

    /// Find element by ARIA role, with optional accessible name filter.
    pub fn get_by_role(&self, role: &str, name: Option<&str>) -> Locator {
        match name {
            Some(n) => Locator::new(self.session.clone(), format!("__pw_role={}|{}", role, n)),
            None => Locator::new(self.session.clone(), format!("__pw_role={}", role)),
        }
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

    /// Wait for a specified duration.
    pub async fn wait_for_timeout(&self, ms: u64) -> CdpResult<()> {
        tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        Ok(())
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
        // 1. Subscribe to events before allowing downloads to avoid race conditions.
        let mut rx = self.session.subscribe_events();

        // 2. Enable downloads globally to temp dir.
        let temp_dir = std::env::temp_dir().to_string_lossy().to_string();
        self.session
            .browser_set_download_behavior("allowAndName", Some(&temp_dir), true)
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
        assert_eq!(loc.selector(), "button.submit");
    }

    #[tokio::test]
    async fn test_page_get_by_test_id() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock);
        let loc = page.get_by_test_id("login-btn");
        assert_eq!(loc.selector(), r#"[data-testid="login-btn"]"#);
    }

    #[tokio::test]
    async fn test_page_get_by_placeholder() {
        let mock = Arc::new(MockCdpClient::new());
        let page = Page::new(mock);
        let loc = page.get_by_placeholder("Enter email");
        assert_eq!(loc.selector(), r#"[placeholder="Enter email"]"#);
    }
}
