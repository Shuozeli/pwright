//! Browser domain — configure global/session-level browser settings like downloads.

use crate::connection::Result;
use crate::generated::browser as cdp_gen;
use crate::session::CdpSession;

impl CdpSession {
    /// Configure the download behavior for the current target/session.
    pub async fn browser_set_download_behavior(
        &self,
        behavior: &str,
        download_path: Option<&str>,
        events_enabled: bool,
    ) -> Result<()> {
        let params = cdp_gen::SetDownloadBehaviorParams {
            behavior: behavior.to_string(),
            download_path: download_path.map(String::from),
            events_enabled: Some(events_enabled),
            ..Default::default()
        };
        self.send(
            "Browser.setDownloadBehavior",
            serde_json::to_value(&params)?,
        )
        .await?;
        Ok(())
    }
}
