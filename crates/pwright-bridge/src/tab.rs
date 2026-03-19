use std::sync::Arc;
use std::time::Instant;

use pwright_cdp::CdpClient;
use pwright_cdp::CdpSession;
use pwright_cdp::connection::Result as CdpResult;
use pwright_cdp::domains::target::TargetInfo;
use tracing::{debug, info};

use crate::browser::Browser;

/// A single browser tab with its CDP session.
#[derive(Clone)]
pub struct Tab {
    pub session: Arc<dyn CdpClient>,
    pub tab_id: String,
    pub target_id: String,
    pub created_at: Instant,
    pub last_used: Instant,
}

impl Browser {
    /// Create a new tab and navigate to the URL.
    pub async fn create_tab(self: &Arc<Self>, url: &str) -> CdpResult<Tab> {
        let nav_url = if url.is_empty() { "about:blank" } else { url };

        // Create target via browser session
        let target_id = self.browser_session().target_create(nav_url).await?;
        debug!(target_id = target_id, url = nav_url, "created target");

        // Attach to the target to get a session
        let session_id = self.browser_session().target_attach(&target_id).await?;
        debug!(
            session_id = session_id,
            target_id = target_id,
            "attached to target"
        );

        // Create a session scoped to this tab
        let session: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
            self.connection().clone(),
            session_id,
            target_id.clone(),
        ));

        // Enable necessary domains
        session.page_enable().await?;
        session.runtime_enable().await?;

        let tab_id = self.next_tab_id();
        let now = Instant::now();

        let tab = Tab {
            session,
            tab_id: tab_id.clone(),
            target_id,
            created_at: now,
            last_used: now,
        };

        self.tabs()
            .write()
            .await
            .insert(tab_id.clone(), tab.clone());
        info!(tab_id = tab_id, url = nav_url, "tab created");
        Ok(tab)
    }

    /// Close a tab by tab ID.
    pub async fn close_tab(&self, tab_id: &str) -> CdpResult<()> {
        let tab = {
            let mut tabs = self.tabs().write().await;
            tabs.remove(tab_id)
        };

        if let Some(tab) = tab {
            self.browser_session().target_close(&tab.target_id).await?;
            self.delete_ref_cache(tab_id).await;
            self.remove_tab_lock(tab_id);
            info!(tab_id = tab_id, "tab closed");
        }
        Ok(())
    }

    /// List all open page targets.
    pub async fn list_tabs(&self) -> CdpResult<Vec<TargetInfo>> {
        let all = self.browser_session().target_get_targets().await?;
        Ok(all
            .into_iter()
            .filter(|t| t.target_type == "page")
            .collect())
    }

    /// Get a tab by ID.
    pub async fn get_tab(&self, tab_id: &str) -> Option<Tab> {
        self.tabs().read().await.get(tab_id).cloned()
    }

    /// Get the most recently used tab, or None.
    pub async fn current_tab(&self) -> Option<Tab> {
        let tabs = self.tabs().read().await;
        tabs.values().max_by_key(|t| t.last_used).cloned()
    }

    /// Resolve a tab — if tab_id is empty, use the current tab.
    pub async fn resolve_tab(&self, tab_id: &str) -> CdpResult<Tab> {
        if tab_id.is_empty() {
            self.current_tab()
                .await
                .ok_or_else(|| pwright_cdp::connection::CdpError::Other("no tabs open".to_string()))
        } else {
            self.get_tab(tab_id).await.ok_or_else(|| {
                pwright_cdp::connection::CdpError::Other(format!("tab {} not found", tab_id))
            })
        }
    }

    /// Re-attach to an existing tab by target_id.
    /// This is used by the CLI to resume a session across invocations.
    pub async fn reattach_tab(self: &Arc<Self>, target_id: &str, tab_id: &str) -> CdpResult<Tab> {
        // Attach to the existing target
        let session_id = self.browser_session().target_attach(target_id).await?;
        let session: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
            self.connection().clone(),
            session_id,
            target_id.to_string(),
        ));

        // Enable necessary domains
        session.page_enable().await?;
        session.runtime_enable().await?;

        let now = Instant::now();
        let tab = Tab {
            session,
            tab_id: tab_id.to_string(),
            target_id: target_id.to_string(),
            created_at: now,
            last_used: now,
        };

        self.tabs()
            .write()
            .await
            .insert(tab_id.to_string(), tab.clone());
        info!(tab_id = tab_id, target_id = target_id, "tab re-attached");
        Ok(tab)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockCdpClient;

    fn make_tab(tab_id: &str, target_id: &str) -> Tab {
        Tab {
            session: Arc::new(MockCdpClient::new()),
            tab_id: tab_id.to_string(),
            target_id: target_id.to_string(),
            created_at: Instant::now(),
            last_used: Instant::now(),
        }
    }

    #[test]
    fn tab_fields() {
        let tab = make_tab("tab_0", "target-abc");
        assert_eq!(tab.tab_id, "tab_0");
        assert_eq!(tab.target_id, "target-abc");
    }

    #[test]
    fn tab_clone_preserves_ids() {
        let tab = make_tab("tab_1", "target-xyz");
        let cloned = tab.clone();
        assert_eq!(cloned.tab_id, "tab_1");
        assert_eq!(cloned.target_id, "target-xyz");
    }
}
