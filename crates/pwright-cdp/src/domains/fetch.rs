//! Fetch domain — request interception for redirect control.

use crate::connection::Result;
use crate::domains::network::ResponseBody;
use crate::generated::fetch as cdp_gen;
use crate::session::CdpSession;

impl CdpSession {
    pub async fn fetch_enable(&self) -> Result<()> {
        self.send(
            "Fetch.enable",
            serde_json::to_value(cdp_gen::EnableParams::default())?,
        )
        .await?;
        Ok(())
    }

    /// Pause matching requests after response headers arrive but before Chrome
    /// consumes or transforms the response body.
    pub async fn fetch_enable_response_stage(&self) -> Result<()> {
        let params = cdp_gen::EnableParams {
            patterns: Some(vec![cdp_gen::RequestPattern {
                url_pattern: Some("*".to_owned()),
                request_stage: Some(cdp_gen::RequestStage::Response),
                ..Default::default()
            }]),
            ..Default::default()
        };
        self.send("Fetch.enable", serde_json::to_value(params)?)
            .await?;
        Ok(())
    }

    pub async fn fetch_disable(&self) -> Result<()> {
        self.send("Fetch.disable", serde_json::json!({})).await?;
        Ok(())
    }

    // Uses json!() because generated ErrorReason enum doesn't match the string-based API.
    pub async fn fetch_continue_request(&self, request_id: &str) -> Result<()> {
        let params = serde_json::json!({ "requestId": request_id });
        self.send("Fetch.continueRequest", params).await?;
        Ok(())
    }

    /// Return the raw body for a request paused at the response stage.
    pub async fn fetch_get_response_body(&self, request_id: &str) -> Result<ResponseBody> {
        let params = cdp_gen::GetResponseBodyParams {
            request_id: request_id.to_owned(),
        };
        let result = self
            .send("Fetch.getResponseBody", serde_json::to_value(params)?)
            .await?;
        let response: cdp_gen::GetResponseBodyReturns = serde_json::from_value(result)?;
        Ok(ResponseBody {
            body: response.body,
            base64_encoded: response.base64_encoded,
        })
    }

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
