//! Page domain — navigation, screenshots, PDF.

use serde_json::Value;

use crate::connection::Result;
use crate::generated::page as cdp_gen;
use crate::session::CdpSession;

impl CdpSession {
    /// Reads `Page.getFrameTree` and returns the current root frame identity.
    pub async fn page_get_main_frame_id(&self) -> Result<String> {
        let frame_tree = self.page_get_frame_tree().await?;
        if frame_tree.frame.parent_id.is_some() {
            return Err(crate::connection::CdpError::Other(
                "Page.getFrameTree returned a root frame with a parent".to_string(),
            ));
        }
        Ok(frame_tree.frame.id)
    }

    /// Reads the complete current frame tree for target initialization.
    pub async fn page_get_frame_tree(&self) -> Result<cdp_gen::FrameTree> {
        let result = self
            .send("Page.getFrameTree", serde_json::json!({}))
            .await?;
        let returns: cdp_gen::GetFrameTreeReturns = serde_json::from_value(result)?;
        Ok(returns.frame_tree)
    }

    /// Returns frameId, loaderId, errorText.
    pub async fn page_navigate(&self, url: &str) -> Result<Value> {
        self.page_navigate_with_referrer(url, None).await
    }

    pub async fn page_navigate_with_referrer(
        &self,
        url: &str,
        referrer: Option<&str>,
    ) -> Result<Value> {
        let params = cdp_gen::NavigateParams {
            url: url.to_string(),
            referrer: referrer.map(ToOwned::to_owned),
            ..Default::default()
        };
        self.send("Page.navigate", serde_json::to_value(&params)?)
            .await
    }

    /// Initiates navigation in an explicit Frame, preserving Page/sibling targets.
    pub async fn page_navigate_in_frame(
        &self,
        frame_id: &str,
        url: &str,
        referrer: Option<&str>,
    ) -> Result<Value> {
        let params = frame_navigation_params(frame_id, url, referrer);
        self.send("Page.navigate", serde_json::to_value(&params)?)
            .await
    }

    /// Stops loading the page and its pending resources.
    pub async fn page_stop_loading(&self) -> Result<()> {
        self.send("Page.stopLoading", serde_json::json!({})).await?;
        Ok(())
    }

    pub async fn page_enable(&self) -> Result<()> {
        self.send("Page.enable", serde_json::json!({})).await?;
        Ok(())
    }

    pub async fn page_set_lifecycle_events_enabled(&self, enabled: bool) -> Result<()> {
        let params = cdp_gen::SetLifecycleEventsEnabledParams { enabled };
        self.send(
            "Page.setLifecycleEventsEnabled",
            serde_json::to_value(params)?,
        )
        .await?;
        Ok(())
    }

    /// Returns base64-encoded image data in the requested format.
    pub async fn page_capture_screenshot(
        &self,
        params: cdp_gen::CaptureScreenshotParams,
    ) -> Result<String> {
        let result = self
            .send("Page.captureScreenshot", serde_json::to_value(&params)?)
            .await?;
        let returns: cdp_gen::CaptureScreenshotReturns = serde_json::from_value(result)?;
        Ok(returns.data)
    }

    pub async fn page_get_layout_metrics(&self) -> Result<cdp_gen::GetLayoutMetricsReturns> {
        let result = self
            .send("Page.getLayoutMetrics", serde_json::json!({}))
            .await?;
        Ok(serde_json::from_value(result)?)
    }

    /// Returns base64 PDF data.
    pub async fn page_print_to_pdf(&self, params: Value) -> Result<String> {
        let result = self.send("Page.printToPDF", params).await?;
        let returns: cdp_gen::PrintToPDFReturns = serde_json::from_value(result)?;
        Ok(returns.data)
    }

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

    /// Registers a script in a named isolated world for every future document.
    pub async fn page_add_script_on_new_document_in_world(
        &self,
        source: &str,
        world_name: &str,
    ) -> Result<String> {
        let params = cdp_gen::AddScriptToEvaluateOnNewDocumentParams {
            source: source.to_string(),
            world_name: Some(world_name.to_string()),
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

    /// Creates a named isolated world for one already-existing frame.
    pub async fn page_create_isolated_world(
        &self,
        frame_id: &str,
        world_name: &str,
        grant_universal_access: bool,
    ) -> Result<i64> {
        let params = cdp_gen::CreateIsolatedWorldParams {
            frame_id: frame_id.to_string(),
            world_name: Some(world_name.to_string()),
            grant_univeral_access: Some(grant_universal_access),
        };
        let result = self
            .send("Page.createIsolatedWorld", serde_json::to_value(&params)?)
            .await?;
        let returns: cdp_gen::CreateIsolatedWorldReturns = serde_json::from_value(result)?;
        Ok(returns.execution_context_id)
    }

    pub async fn page_reload(&self) -> Result<()> {
        self.send(
            "Page.reload",
            serde_json::to_value(cdp_gen::ReloadParams::default())?,
        )
        .await?;
        Ok(())
    }

    pub async fn page_get_navigation_history(&self) -> Result<Value> {
        self.send("Page.getNavigationHistory", serde_json::json!({}))
            .await
    }

    pub async fn page_navigate_to_history_entry(&self, entry_id: i64) -> Result<()> {
        let params = cdp_gen::NavigateToHistoryEntryParams { entry_id };
        self.send(
            "Page.navigateToHistoryEntry",
            serde_json::to_value(&params)?,
        )
        .await?;
        Ok(())
    }

    pub async fn page_bring_to_front(&self) -> Result<()> {
        self.send("Page.bringToFront", serde_json::json!({}))
            .await?;
        Ok(())
    }

    pub async fn page_set_document_content(&self, frame_id: &str, html: &str) -> Result<()> {
        let params = cdp_gen::SetDocumentContentParams {
            frame_id: frame_id.to_string(),
            html: html.to_string(),
        };
        self.send("Page.setDocumentContent", serde_json::to_value(&params)?)
            .await?;
        Ok(())
    }

    /// Initiates Page closure while running before-unload handlers.
    ///
    /// CDP completion does not prove that the target closed: a before-unload
    /// dialog can keep it alive.
    pub async fn page_close(&self) -> Result<()> {
        self.send("Page.close", serde_json::json!({})).await?;
        Ok(())
    }
}

fn frame_navigation_params(
    frame_id: &str,
    url: &str,
    referrer: Option<&str>,
) -> cdp_gen::NavigateParams {
    cdp_gen::NavigateParams {
        url: url.to_owned(),
        referrer: referrer.map(ToOwned::to_owned),
        frame_id: Some(frame_id.to_owned()),
        // Matches the pinned Playwright Chromium navigateFrame boundary.
        referrer_policy: Some(cdp_gen::ReferrerPolicy::UnsafeUrl),
        ..Default::default()
    }
}

#[cfg(test)]
mod frame_navigation_tests {
    use super::*;

    #[test]
    fn frame_navigation_serializes_explicit_target_and_referrer_policy() {
        // Arrange
        let params = frame_navigation_params(
            "child",
            "https://example.test/next",
            Some("https://referrer.test/path?q=1"),
        );
        // Act
        let encoded = serde_json::to_value(params).unwrap();
        // Assert
        assert_eq!(
            encoded,
            serde_json::json!({"frameId": "child", "url": "https://example.test/next",
            "referrer": "https://referrer.test/path?q=1", "referrerPolicy": "unsafeUrl"})
        );
    }

    #[test]
    fn omitted_referrer_does_not_omit_the_frame_target() {
        // Arrange
        let params = frame_navigation_params("child", "about:blank", None);
        // Act
        let encoded = serde_json::to_value(params).unwrap();
        // Assert
        assert_eq!(
            encoded,
            serde_json::json!({"frameId": "child", "url": "about:blank", "referrerPolicy": "unsafeUrl"})
        );
    }
}
