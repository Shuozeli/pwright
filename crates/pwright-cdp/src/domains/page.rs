//! Page domain — navigation, screenshots, PDF.

use serde_json::{Value, json};

use crate::connection::Result;
use crate::session::CdpSession;

impl CdpSession {
    /// Navigate to a URL. Returns frameId, loaderId, errorText.
    pub async fn page_navigate(&self, url: &str) -> Result<Value> {
        self.send("Page.navigate", json!({ "url": url })).await
    }

    /// Enable Page domain events.
    pub async fn page_enable(&self) -> Result<()> {
        self.send("Page.enable", json!({})).await?;
        Ok(())
    }

    /// Capture a screenshot. Returns base64 PNG data.
    pub async fn page_capture_screenshot(
        &self,
        format: &str,
        quality: Option<i32>,
        full_page: bool,
    ) -> Result<String> {
        let mut params = json!({ "format": format });
        if let Some(q) = quality {
            params["quality"] = json!(q);
        }
        if full_page {
            params["captureBeyondViewport"] = json!(true);
        }
        let result = self.send("Page.captureScreenshot", params).await?;
        Ok(result["data"].as_str().unwrap_or_default().to_string())
    }

    /// Print the page to PDF. Returns base64 PDF data.
    pub async fn page_print_to_pdf(&self, params: Value) -> Result<String> {
        let result = self.send("Page.printToPDF", params).await?;
        Ok(result["data"].as_str().unwrap_or_default().to_string())
    }

    /// Add a script to evaluate on every new document.
    pub async fn page_add_script_on_new_document(&self, source: &str) -> Result<String> {
        let result = self
            .send(
                "Page.addScriptToEvaluateOnNewDocument",
                json!({ "source": source }),
            )
            .await?;
        Ok(result["identifier"]
            .as_str()
            .unwrap_or_default()
            .to_string())
    }

    /// Reload the current page.
    pub async fn page_reload(&self) -> Result<()> {
        self.send("Page.reload", json!({})).await?;
        Ok(())
    }

    /// Get navigation history entries and current index.
    pub async fn page_get_navigation_history(&self) -> Result<Value> {
        self.send("Page.getNavigationHistory", json!({})).await
    }

    /// Navigate to a specific history entry.
    pub async fn page_navigate_to_history_entry(&self, entry_id: i64) -> Result<()> {
        self.send(
            "Page.navigateToHistoryEntry",
            json!({ "entryId": entry_id }),
        )
        .await?;
        Ok(())
    }

    /// Bring the page to front (activate tab).
    pub async fn page_bring_to_front(&self) -> Result<()> {
        self.send("Page.bringToFront", json!({})).await?;
        Ok(())
    }

    /// Set the document content (replaces current HTML).
    pub async fn page_set_document_content(&self, frame_id: &str, html: &str) -> Result<()> {
        self.send(
            "Page.setDocumentContent",
            json!({ "frameId": frame_id, "html": html }),
        )
        .await?;
        Ok(())
    }
}
