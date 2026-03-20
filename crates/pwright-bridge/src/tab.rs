use std::sync::Arc;
use std::time::Instant;

use pwright_cdp::CdpClient;
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
        let target_id = self.browser_client().target_create(nav_url).await?;
        debug!(target_id = target_id, url = nav_url, "created target");

        // Attach and create a per-tab session
        let session = self.attach_session(&target_id).await?;

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
            self.browser_client().target_close(&tab.target_id).await?;
            self.delete_ref_cache(tab_id).await;
            self.remove_tab_lock(tab_id);
            info!(tab_id = tab_id, "tab closed");
        }
        Ok(())
    }

    /// List all open page targets.
    pub async fn list_tabs(&self) -> CdpResult<Vec<TargetInfo>> {
        let all = self.browser_client().target_get_targets().await?;
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
        let session = self.attach_session(target_id).await?;

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
    use crate::test_utils::{FakeSessionFactory, MockCdpClient};

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

    #[tokio::test]
    async fn test_resolve_tab_by_id() {
        let mock = Arc::new(MockCdpClient::new());
        let browser = Browser::new_for_test(mock, Arc::new(FakeSessionFactory));
        browser
            .tabs()
            .write()
            .await
            .insert("tab_0".into(), make_tab("tab_0", "t-1"));

        let tab = browser.resolve_tab("tab_0").await.unwrap();
        assert_eq!(tab.tab_id, "tab_0");
        assert_eq!(tab.target_id, "t-1");
    }

    #[tokio::test]
    async fn test_resolve_tab_empty_returns_current() {
        let mock = Arc::new(MockCdpClient::new());
        let browser = Browser::new_for_test(mock, Arc::new(FakeSessionFactory));
        browser
            .tabs()
            .write()
            .await
            .insert("tab_0".into(), make_tab("tab_0", "t-1"));

        let tab = browser.resolve_tab("").await.unwrap();
        assert_eq!(tab.tab_id, "tab_0");
    }

    #[tokio::test]
    async fn test_resolve_tab_not_found() {
        let mock = Arc::new(MockCdpClient::new());
        let browser = Browser::new_for_test(mock, Arc::new(FakeSessionFactory));

        let result = browser.resolve_tab("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resolve_tab_empty_no_tabs() {
        let mock = Arc::new(MockCdpClient::new());
        let browser = Browser::new_for_test(mock, Arc::new(FakeSessionFactory));

        let result = browser.resolve_tab("").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_tab_calls_target_create_and_attach() {
        let mock = Arc::new(MockCdpClient::new());
        let browser = Browser::new_for_test(mock.clone(), Arc::new(FakeSessionFactory));

        let tab = browser.create_tab("https://example.com").await.unwrap();
        assert!(!tab.tab_id.is_empty());
        assert!(!tab.target_id.is_empty());

        // Verify CDP calls
        assert_eq!(mock.calls_for("Target.createTarget").len(), 1);
        assert_eq!(mock.calls_for("Target.attachToTarget").len(), 1);
    }

    #[tokio::test]
    async fn test_create_tab_inserts_into_tabs_map() {
        let mock = Arc::new(MockCdpClient::new());
        let browser = Browser::new_for_test(mock, Arc::new(FakeSessionFactory));

        let tab = browser.create_tab("about:blank").await.unwrap();
        let stored = browser.get_tab(&tab.tab_id).await;
        assert!(stored.is_some());
        assert_eq!(stored.unwrap().target_id, tab.target_id);
    }

    #[tokio::test]
    async fn test_close_tab_removes_from_map() {
        let mock = Arc::new(MockCdpClient::new());
        let browser = Browser::new_for_test(mock.clone(), Arc::new(FakeSessionFactory));

        // Pre-populate
        browser
            .tabs()
            .write()
            .await
            .insert("tab_0".into(), make_tab("tab_0", "t-close"));

        browser.close_tab("tab_0").await.unwrap();

        assert!(browser.get_tab("tab_0").await.is_none());
        assert_eq!(mock.calls_for("Target.closeTarget").len(), 1);
    }

    #[tokio::test]
    async fn test_close_nonexistent_tab_is_noop() {
        let mock = Arc::new(MockCdpClient::new());
        let browser = Browser::new_for_test(mock.clone(), Arc::new(FakeSessionFactory));

        browser.close_tab("nonexistent").await.unwrap();
        assert!(mock.calls_for("Target.closeTarget").is_empty());
    }

    #[tokio::test]
    async fn test_list_tabs_filters_pages() {
        let mock = Arc::new(MockCdpClient::new());
        use pwright_cdp::domains::target::TargetInfo;
        mock.set_targets_response(vec![
            TargetInfo {
                target_id: "t1".into(),
                target_type: "page".into(),
                title: "Page".into(),
                url: "https://a.com".into(),
                attached: false,
            },
            TargetInfo {
                target_id: "t2".into(),
                target_type: "background_page".into(),
                title: "BG".into(),
                url: "".into(),
                attached: false,
            },
        ]);
        let browser = Browser::new_for_test(mock, Arc::new(FakeSessionFactory));

        let tabs = browser.list_tabs().await.unwrap();
        assert_eq!(tabs.len(), 1);
        assert_eq!(tabs[0].target_id, "t1");
    }
}
