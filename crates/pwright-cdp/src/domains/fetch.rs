//! Fetch domain — request interception for redirect control.

use crate::connection::Result;
use crate::generated::fetch as cdp_gen;
use crate::session::CdpSession;

impl CdpSession {
    /// Enable the Fetch domain to intercept requests.
    pub async fn fetch_enable(&self) -> Result<()> {
        self.send(
            "Fetch.enable",
            serde_json::to_value(cdp_gen::EnableParams::default())?,
        )
        .await?;
        Ok(())
    }

    /// Disable the Fetch domain.
    pub async fn fetch_disable(&self) -> Result<()> {
        self.send("Fetch.disable", serde_json::json!({})).await?;
        Ok(())
    }

    /// Continue an intercepted request.
    // Uses json!() because generated ErrorReason enum doesn't match the string-based API.
    pub async fn fetch_continue_request(&self, request_id: &str) -> Result<()> {
        let params = serde_json::json!({ "requestId": request_id });
        self.send("Fetch.continueRequest", params).await?;
        Ok(())
    }

    /// Fail an intercepted request.
    // Uses json!() because generated ErrorReason enum doesn't match the string-based API.
    pub async fn fetch_fail_request(&self, request_id: &str, reason: &str) -> Result<()> {
        self.send(
            "Fetch.failRequest",
            serde_json::json!({ "requestId": request_id, "errorReason": reason }),
        )
        .await?;
        Ok(())
    }
}
