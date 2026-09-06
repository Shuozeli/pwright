//! Browser domain — configure global/session-level browser settings like downloads.

use crate::connection::Result;
use crate::generated::browser as cdp_gen;
use crate::session::CdpSession;

/// CDP `Browser.setDownloadBehavior` behavior parameter.
///
/// See <https://chromedevtools.github.io/devtools-protocol/tot/Browser/#method-setDownloadBehavior>.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadBehavior {
    /// Deny all download requests.
    Deny,
    /// Allow all download requests.
    Allow,
    /// Allow downloads and name files according to their download GUIDs.
    AllowAndName,
    /// Use default Chrome behavior (if available, otherwise deny).
    Default,
}

impl DownloadBehavior {
    /// Returns the CDP wire-format string for this variant.
    fn as_str(&self) -> &'static str {
        match self {
            DownloadBehavior::Deny => "deny",
            DownloadBehavior::Allow => "allow",
            DownloadBehavior::AllowAndName => "allowAndName",
            DownloadBehavior::Default => "default",
        }
    }
}

impl CdpSession {
    /// Requests graceful shutdown through CDP `Browser.close`.
    ///
    /// The browser may close the transport before the command response is
    /// observed. Callers that own the browser process must therefore observe
    /// that process independently rather than treating acknowledgement as
    /// proof of termination.
    pub async fn browser_close(&self) -> Result<()> {
        self.send("Browser.close", serde_json::json!({})).await?;
        Ok(())
    }

    pub async fn browser_set_download_behavior(
        &self,
        behavior: DownloadBehavior,
        download_path: Option<&str>,
        events_enabled: bool,
    ) -> Result<()> {
        let params = cdp_gen::SetDownloadBehaviorParams {
            behavior: behavior.as_str().to_string(),
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
