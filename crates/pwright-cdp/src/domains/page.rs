//! Page domain — navigation, screenshots, PDF.

use serde_json::Value;

use crate::connection::Result;
use crate::generated::page as cdp_gen;
use crate::session::CdpSession;

impl CdpSession {
    /// Navigate to a URL. Returns frameId, loaderId, errorText.
    pub async fn page_navigate(&self, url: &str) -> Result<Value> {
        let params = cdp_gen::NavigateParams {
            url: url.to_string(),
            ..Default::default()
        };
        self.send("Page.navigate", serde_json::to_value(&params)?)
            .await
    }

    /// Enable Page domain events.
    pub async fn page_enable(&self) -> Result<()> {
        self.send("Page.enable", serde_json::json!({})).await?;
        Ok(())
    }

    /// Capture a screenshot. Returns base64 PNG data.
    pub async fn page_capture_screenshot(
        &self,
        format: &str,
        quality: Option<i32>,
        full_page: bool,
    ) -> Result<String> {
        let params = cdp_gen::CaptureScreenshotParams {
            format: Some(format.to_string()),
            quality: quality.map(i64::from),
            capture_beyond_viewport: if full_page { Some(true) } else { None },
            ..Default::default()
        };
        let result = self
            .send("Page.captureScreenshot", serde_json::to_value(&params)?)
            .await?;
        let returns: cdp_gen::CaptureScreenshotReturns = serde_json::from_value(result)?;
        Ok(returns.data)
    }

    /// Print the page to PDF. Returns base64 PDF data.
    pub async fn page_print_to_pdf(&self, params: Value) -> Result<String> {
        let result = self.send("Page.printToPDF", params).await?;
        let returns: cdp_gen::PrintToPDFReturns = serde_json::from_value(result)?;
        Ok(returns.data)
    }

    /// Add a script to evaluate on every new document.
    pub async fn page_add_script_on_new_document(&self, source: &str) -> Result<String> {
        let params = cdp_gen::AddScriptToEvaluateOnNewDocumentParams {
            source: source.to_string(),
            ..Default::default()
        };
        let result = self
            .send(
                "Page.addScriptToEvaluateOnNewDocument",
                serde_json::to_value(&params)?,
            )
            .await?;
        let returns: cdp_gen::AddScriptToEvaluateOnNewDocumentReturns =
            serde_json::from_value(result)?;
        Ok(returns.identifier)
    }

    /// Reload the current page.
    pub async fn page_reload(&self) -> Result<()> {
        self.send(
            "Page.reload",
            serde_json::to_value(cdp_gen::ReloadParams::default())?,
        )
        .await?;
        Ok(())
    }

    /// Get navigation history entries and current index.
    pub async fn page_get_navigation_history(&self) -> Result<Value> {
        self.send("Page.getNavigationHistory", serde_json::json!({}))
            .await
    }

    /// Navigate to a specific history entry.
    pub async fn page_navigate_to_history_entry(&self, entry_id: i64) -> Result<()> {
        let params = cdp_gen::NavigateToHistoryEntryParams { entry_id };
        self.send(
            "Page.navigateToHistoryEntry",
            serde_json::to_value(&params)?,
        )
        .await?;
        Ok(())
    }

    /// Bring the page to front (activate tab).
    pub async fn page_bring_to_front(&self) -> Result<()> {
        self.send("Page.bringToFront", serde_json::json!({}))
            .await?;
        Ok(())
    }

    /// Set the document content (replaces current HTML).
    pub async fn page_set_document_content(&self, frame_id: &str, html: &str) -> Result<()> {
        let params = cdp_gen::SetDocumentContentParams {
            frame_id: frame_id.to_string(),
            html: html.to_string(),
        };
        self.send("Page.setDocumentContent", serde_json::to_value(&params)?)
            .await?;
        Ok(())
    }
}
