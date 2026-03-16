//! Target domain — create, close, list, and attach to targets (tabs).

use serde::{Deserialize, Serialize};

use crate::connection::Result;
use crate::generated::target as cdp_gen;
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
        let params = cdp_gen::CreateTargetParams {
            url: url.to_string(),
            ..Default::default()
        };
        let result = self
            .send("Target.createTarget", serde_json::to_value(&params)?)
            .await?;
        let returns: cdp_gen::CreateTargetReturns = serde_json::from_value(result)?;
        Ok(returns.target_id)
    }

    /// Close a target by ID.
    pub async fn target_close(&self, target_id: &str) -> Result<()> {
        let params = cdp_gen::CloseTargetParams {
            target_id: target_id.to_string(),
        };
        self.send("Target.closeTarget", serde_json::to_value(&params)?)
            .await?;
        Ok(())
    }

    /// List all targets, optionally filtered by type.
    pub async fn target_get_targets(&self) -> Result<Vec<TargetInfo>> {
        let result = self
            .send(
                "Target.getTargets",
                serde_json::to_value(cdp_gen::GetTargetsParams::default())?,
            )
            .await?;
        let targets: Vec<TargetInfo> =
            serde_json::from_value(result["targetInfos"].clone()).unwrap_or_default();
        Ok(targets)
    }

    /// Attach to a target, enabling session-scoped commands. Returns session ID.
    pub async fn target_attach(&self, target_id: &str) -> Result<String> {
        let params = cdp_gen::AttachToTargetParams {
            target_id: target_id.to_string(),
            flatten: Some(true),
        };
        let result = self
            .send("Target.attachToTarget", serde_json::to_value(&params)?)
            .await?;
        let returns: cdp_gen::AttachToTargetReturns = serde_json::from_value(result)?;
        Ok(returns.session_id)
    }

    /// Detach from a target session.
    pub async fn target_detach(&self, session_id: &str) -> Result<()> {
        let params = cdp_gen::DetachFromTargetParams {
            session_id: Some(session_id.to_string()),
            ..Default::default()
        };
        self.send("Target.detachFromTarget", serde_json::to_value(&params)?)
            .await?;
        Ok(())
    }
}
