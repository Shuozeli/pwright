//! Target domain — create, close, list, and attach to targets (tabs).

use serde::{Deserialize, Serialize};

use crate::connection::{CdpError, Result};
use crate::generated::target as cdp_gen;
use crate::session::CdpSession;

#[derive(Serialize)]
struct ResumePausedTargetParams {}

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
    /// The child ID belongs in the command parameters; its announcing parent
    /// belongs in the flattened CDP envelope. No synthetic target binding.
    pub async fn target_detach_in_parent(
        &self,
        parent_session_id: Option<&str>,
        session_id: &str,
    ) -> Result<()> {
        // Chromium does not reliably resume a paused target on detach. Match
        // Playwright's child-session detach sequence before releasing ownership.
        let resumed = self
            .connection()
            .send_command_with_session(
                "Runtime.runIfWaitingForDebugger",
                serde_json::to_value(ResumePausedTargetParams {})?,
                Some(session_id),
            )
            .await;
        match resumed {
            Ok(_) => {}
            Err(CdpError::SessionDetached) => return Ok(()),
            // The exact child is already gone; no detach command can release
            // it further. Other errors (including connection loss) still fail.
            Err(CdpError::Protocol {
                code: -32001,
                message,
            }) if message == "Session with given id not found." => return Ok(()),
            Err(error) => return Err(error),
        }
        let params = cdp_gen::DetachFromTargetParams {
            session_id: Some(session_id.to_owned()),
            target_id: None,
        };
        let detached = self
            .connection()
            .send_command_with_session(
                "Target.detachFromTarget",
                serde_json::to_value(params)?,
                parent_session_id,
            )
            .await;
        match detached {
            Ok(_) => Ok(()),
            // Resume succeeded, but the child can disappear before this second
            // command reaches its parent. This exact command/receipt confirms
            // that no child attachment remains to release. It is not a general
            // invalid-parameters or connection-loss success policy.
            Err(CdpError::Protocol {
                code: -32602,
                message,
            }) if message == "No session with given id" => Ok(()),
            Err(error) => Err(error),
        }
    }
    pub async fn target_create_browser_context(
        &self,
        proxy_server: Option<&str>,
        proxy_bypass_list: Option<&str>,
    ) -> Result<String> {
        let params = cdp_gen::CreateBrowserContextParams {
            dispose_on_detach: Some(true),
            proxy_server: proxy_server.map(str::to_owned),
            proxy_bypass_list: proxy_bypass_list.map(str::to_owned),
            ..Default::default()
        };
        let result = self
            .send(
                "Target.createBrowserContext",
                serde_json::to_value(&params)?,
            )
            .await?;
        let returns: cdp_gen::CreateBrowserContextReturns = serde_json::from_value(result)?;
        Ok(returns.browser_context_id)
    }

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

    pub async fn target_create_in_context(
        &self,
        url: &str,
        browser_context_id: &str,
    ) -> Result<String> {
        let params = target_creation_params(url, browser_context_id);
        let result = self
            .send("Target.createTarget", serde_json::to_value(&params)?)
            .await?;
        let returns: cdp_gen::CreateTargetReturns = serde_json::from_value(result)?;
        Ok(returns.target_id)
    }

    pub async fn target_dispose_browser_context(&self, browser_context_id: &str) -> Result<()> {
        let params = cdp_gen::DisposeBrowserContextParams {
            browser_context_id: browser_context_id.to_owned(),
        };
        self.send(
            "Target.disposeBrowserContext",
            serde_json::to_value(&params)?,
        )
        .await?;
        Ok(())
    }

    pub async fn target_close(&self, target_id: &str) -> Result<()> {
        let params = cdp_gen::CloseTargetParams {
            target_id: target_id.to_string(),
        };
        let result = self
            .send("Target.closeTarget", serde_json::to_value(&params)?)
            .await?;
        let returns: cdp_gen::CloseTargetReturns = serde_json::from_value(result)?;
        if !returns.success {
            return Err(crate::connection::CdpError::Other(format!(
                "Target.closeTarget reported failure for target {target_id}"
            )));
        }
        Ok(())
    }

    pub async fn target_get_targets(&self) -> Result<Vec<TargetInfo>> {
        let mut result = self
            .send(
                "Target.getTargets",
                serde_json::to_value(cdp_gen::GetTargetsParams::default())?,
            )
            .await?;
        let targets: Vec<TargetInfo> = serde_json::from_value(result["targetInfos"].take())
            .map_err(crate::connection::CdpError::Json)?;
        Ok(targets)
    }

    /// Lists non-default browser contexts created through CDP.
    pub async fn target_get_browser_contexts(&self) -> Result<Vec<String>> {
        let result = self
            .send("Target.getBrowserContexts", serde_json::json!({}))
            .await?;
        let returns: cdp_gen::GetBrowserContextsReturns = serde_json::from_value(result)?;
        Ok(returns.browser_context_ids)
    }

    /// Returns one target as a browser-root ordering barrier after
    /// `Target.setAutoAttach`, matching Chromium's attach initialization.
    pub async fn target_get_target_info(&self) -> Result<cdp_gen::TargetInfo> {
        let result = self
            .send(
                "Target.getTargetInfo",
                serde_json::to_value(cdp_gen::GetTargetInfoParams::default())?,
            )
            .await?;
        let returns: cdp_gen::GetTargetInfoReturns = serde_json::from_value(result)?;
        Ok(returns.target_info)
    }

    /// Returns session ID.
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

    pub async fn target_detach(&self, session_id: &str) -> Result<()> {
        let params = cdp_gen::DetachFromTargetParams {
            session_id: Some(session_id.to_string()),
            ..Default::default()
        };
        self.send("Target.detachFromTarget", serde_json::to_value(&params)?)
            .await?;
        Ok(())
    }

    /// Recursively observe targets related to this target-local session.
    pub async fn target_set_auto_attach(
        &self,
        auto_attach: bool,
        wait_for_debugger_on_start: bool,
        flatten: bool,
    ) -> Result<()> {
        let params = cdp_gen::SetAutoAttachParams {
            auto_attach,
            wait_for_debugger_on_start,
            flatten: Some(flatten),
            filter: None,
        };
        self.send("Target.setAutoAttach", serde_json::to_value(params)?)
            .await?;
        Ok(())
    }
}

fn target_creation_params(url: &str, browser_context_id: &str) -> cdp_gen::CreateTargetParams {
    cdp_gen::CreateTargetParams {
        url: url.to_owned(),
        // The topology uses an empty key for the browser's default Context.
        // CDP requires omission here: an explicitly empty ID is not a Context.
        browser_context_id: (!browser_context_id.is_empty()).then(|| browser_context_id.to_owned()),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::target_creation_params;

    #[test]
    fn default_context_creation_omits_the_native_context_id() {
        // Arrange
        let params = target_creation_params("about:blank", "");
        // Act
        let encoded = serde_json::to_value(params).unwrap();
        // Assert
        assert_eq!(encoded["url"], "about:blank");
        assert!(encoded.get("browserContextId").is_none());
    }

    #[test]
    fn explicit_context_creation_preserves_the_exact_native_id() {
        // Arrange
        let params = target_creation_params("about:blank", "context-a");
        // Act
        let encoded = serde_json::to_value(params).unwrap();
        // Assert
        assert_eq!(encoded["browserContextId"], "context-a");
    }
}
