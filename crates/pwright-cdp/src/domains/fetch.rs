//! Fetch domain — request interception for redirect control.

use serde_json::json;

use crate::connection::Result;
use crate::session::CdpSession;

impl CdpSession {
    /// Enable the Fetch domain to intercept requests.
    pub async fn fetch_enable(&self) -> Result<()> {
        self.send("Fetch.enable", json!({})).await?;
        Ok(())
    }

    /// Disable the Fetch domain.
    pub async fn fetch_disable(&self) -> Result<()> {
        self.send("Fetch.disable", json!({})).await?;
        Ok(())
    }

    /// Continue an intercepted request.
    pub async fn fetch_continue_request(&self, request_id: &str) -> Result<()> {
        self.send("Fetch.continueRequest", json!({ "requestId": request_id }))
            .await?;
        Ok(())
    }

    /// Fail an intercepted request.
    pub async fn fetch_fail_request(&self, request_id: &str, reason: &str) -> Result<()> {
        self.send(
            "Fetch.failRequest",
            json!({ "requestId": request_id, "errorReason": reason }),
        )
        .await?;
        Ok(())
    }
}
