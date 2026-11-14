//! Headless `Page.pause` timeout scope.
//!
//! Official Playwright does not open Inspector for a headless browser. It
//! temporarily disables the Context's operation/navigation timeouts while the
//! protocol pause round-trip is in flight, then restores both settings. This
//! leaf preserves that scoped timeout ownership in Rust. Headed Inspector UI
//! remains explicitly unsupported.

use crate::{HostError, HostSession};

impl HostSession {
    pub async fn page_pause(&self, page_id: &str) -> Result<(), HostError> {
        let page = Self::parse_timeout_object(page_id, "Page")?;
        self.require_visible_page_id(page)?;
        let headful = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state.as_ref().ok_or(HostError::TargetClosed)?.headful
        };
        if headful {
            return Err(HostError::UnsupportedOperation(
                "headed Page.pause Inspector capability is not installed".into(),
            ));
        }
        let (installation, contexts, pages) = self.timeout_snapshot()?;
        let context = pages
            .iter()
            .flatten()
            .find_map(|(candidate, context)| (*candidate == page).then_some(*context))
            .ok_or(HostError::TargetClosed)?;
        let _reset = installation.begin_headless_pause(page, context, &contexts, &pages)?;
        tokio::select! {
            biased;
            () = self.stop.cancelled() => Err(HostError::TargetClosed),
            () = self.disconnected.cancelled() => Err(HostError::TargetClosed),
            () = tokio::task::yield_now() => Ok(()),
        }
    }
}
