//! Browser domain — configure global/session-level browser settings like downloads.

use crate::connection::Result;
use crate::session::CdpSession;
use serde_json::json;

impl CdpSession {
    /// Configure the download behavior for the current target/session.
    pub async fn browser_set_download_behavior(
        &self,
        behavior: &str,
        download_path: Option<&str>,
        events_enabled: bool,
    ) -> Result<()> {
        let mut params = json!({
            "behavior": behavior,
            "eventsEnabled": events_enabled,
        });

        if let Some(path) = download_path {
            params["downloadPath"] = json!(path);
        }

        self.send("Browser.setDownloadBehavior", params).await?;
        Ok(())
    }
}
