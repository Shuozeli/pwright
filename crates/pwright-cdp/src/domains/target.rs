//! Target domain — create, close, list, and attach to targets (tabs).

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::connection::Result;
use crate::session::CdpSession;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetInfo {
    /// CDP returns `targetId`; Chrome HTTP endpoints return `id`.
    #[serde(alias = "id")]
    pub target_id: String,
    #[serde(rename = "type")]
    pub target_type: String,
    pub title: String,
    pub url: String,
    /// Present in CDP responses but absent from Chrome HTTP endpoints.
    #[serde(default)]
    pub attached: bool,
}

impl CdpSession {
    /// Create a new browser target (tab) at the given URL.
    pub async fn target_create(&self, url: &str) -> Result<String> {
        let result = self
            .send("Target.createTarget", json!({ "url": url }))
            .await?;
        Ok(result["targetId"].as_str().unwrap_or_default().to_string())
    }

    /// Close a target by ID.
    pub async fn target_close(&self, target_id: &str) -> Result<()> {
        self.send("Target.closeTarget", json!({ "targetId": target_id }))
            .await?;
        Ok(())
    }

    /// List all targets, optionally filtered by type.
    pub async fn target_get_targets(&self) -> Result<Vec<TargetInfo>> {
        let result = self.send("Target.getTargets", json!({})).await?;
        let targets: Vec<TargetInfo> =
            serde_json::from_value(result["targetInfos"].clone()).unwrap_or_default();
        Ok(targets)
    }

    /// Attach to a target, enabling session-scoped commands. Returns session ID.
    pub async fn target_attach(&self, target_id: &str) -> Result<String> {
        let result = self
            .send(
                "Target.attachToTarget",
                json!({ "targetId": target_id, "flatten": true }),
            )
            .await?;
        Ok(result["sessionId"].as_str().unwrap_or_default().to_string())
    }

    /// Detach from a target session.
    pub async fn target_detach(&self, session_id: &str) -> Result<()> {
        self.send(
            "Target.detachFromTarget",
            json!({ "sessionId": session_id }),
        )
        .await?;
        Ok(())
    }
}
