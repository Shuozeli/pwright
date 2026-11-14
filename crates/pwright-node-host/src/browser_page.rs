//! Rust-owned `Browser.newPage` convenience ownership.

use std::sync::Mutex;

use pwright_core::ObjectId;

use crate::{BrowserPageDescriptor, HostError, HostSession};

const MAX_BROWSER_PAGES: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entry {
    context: ObjectId,
    page: Option<ObjectId>,
}

pub(crate) struct BrowserPages {
    entries: Mutex<[Option<Entry>; MAX_BROWSER_PAGES]>,
}

impl BrowserPages {
    pub(crate) const fn new() -> Self {
        Self {
            entries: Mutex::new([None; MAX_BROWSER_PAGES]),
        }
    }

    fn parse(id: &str, kind: &str) -> Result<ObjectId, HostError> {
        id.parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument(format!("invalid portable {kind} ID")))
    }

    pub(crate) fn reserve(&self, context_id: &str) -> Result<(), HostError> {
        let context = Self::parse(context_id, "Context")?;
        let mut entries = self.entries.lock().map_err(HostError::backend)?;
        if entries
            .iter()
            .flatten()
            .any(|entry| entry.context == context)
        {
            return Err(HostError::Backend(
                "Browser.new_page Context is already reserved".into(),
            ));
        }
        let slot = entries
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or_else(|| {
                HostError::ResourceExhausted(format!(
                    "Browser.new_page ownership capacity exceeded ({MAX_BROWSER_PAGES})"
                ))
            })?;
        *slot = Some(Entry {
            context,
            page: None,
        });
        Ok(())
    }

    pub(crate) fn commit(&self, context_id: &str, page_id: &str) -> Result<(), HostError> {
        let context = Self::parse(context_id, "Context")?;
        let page = Self::parse(page_id, "Page")?;
        let mut entries = self.entries.lock().map_err(HostError::backend)?;
        let entry = entries
            .iter_mut()
            .flatten()
            .find(|entry| entry.context == context)
            .ok_or_else(|| HostError::Backend("Browser.new_page reservation is missing".into()))?;
        if entry.page.replace(page).is_some() {
            return Err(HostError::Backend(
                "Browser.new_page reservation was already committed".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn owns_context(&self, context_id: &str) -> Result<bool, HostError> {
        let context = Self::parse(context_id, "Context")?;
        let entries = self.entries.lock().map_err(HostError::backend)?;
        Ok(entries
            .iter()
            .flatten()
            .any(|entry| entry.context == context))
    }

    pub(crate) fn context_for_page(&self, page_id: &str) -> Result<Option<ObjectId>, HostError> {
        let page = Self::parse(page_id, "Page")?;
        let entries = self.entries.lock().map_err(HostError::backend)?;
        Ok(entries
            .iter()
            .flatten()
            .find(|entry| entry.page == Some(page))
            .map(|entry| entry.context))
    }

    pub(crate) fn remove_context(&self, context_id: &str) -> Result<(), HostError> {
        let context = Self::parse(context_id, "Context")?;
        let mut entries = self.entries.lock().map_err(HostError::backend)?;
        if let Some(slot) = entries
            .iter_mut()
            .find(|slot| slot.is_some_and(|entry| entry.context == context))
        {
            *slot = None;
        }
        Ok(())
    }

    pub(crate) fn prune(&self, mut is_live: impl FnMut(ObjectId) -> bool) -> Result<(), HostError> {
        let mut entries = self.entries.lock().map_err(HostError::backend)?;
        for slot in &mut *entries {
            if slot.is_some_and(|entry| !is_live(entry.context)) {
                *slot = None;
            }
        }
        Ok(())
    }
}

impl HostSession {
    pub async fn new_browser_page(
        &self,
        base_url: Option<&str>,
    ) -> Result<BrowserPageDescriptor, HostError> {
        self.new_browser_page_with_downloads(base_url, false).await
    }

    pub async fn new_browser_page_with_downloads(
        &self,
        base_url: Option<&str>,
        accept_downloads: bool,
    ) -> Result<BrowserPageDescriptor, HostError> {
        #[cfg(not(feature = "download-events"))]
        if accept_downloads {
            return Err(HostError::UnsupportedOperation(
                "download event capability is not installed".into(),
            ));
        }
        #[cfg(feature = "browser-cdp-session")]
        let target_fence = self.cdp_browser_target_fence()?;
        let context = self.new_context_with_base_url(base_url).await?;
        if let Err(primary) = self.browser_pages.reserve(&context.id) {
            return Err(self
                .compensate_browser_page_context(&context.id, primary)
                .await);
        }
        #[cfg(feature = "download-events")]
        if accept_downloads && let Err(primary) = self.enable_download_context(&context.id).await {
            return Err(self
                .compensate_browser_page_context(&context.id, primary)
                .await);
        }
        let page = match self.new_page_inner(&context.id).await {
            Ok(page) => page,
            Err(primary) => {
                return Err(self
                    .compensate_browser_page_context(&context.id, primary)
                    .await);
            }
        };
        #[cfg(feature = "browser-cdp-session")]
        if let Err(primary) = self
            .cdp_wait_browser_target_fence(&page.id, &target_fence)
            .await
        {
            return Err(self
                .compensate_browser_page_context(&context.id, primary)
                .await);
        }
        if let Err(primary) = self.browser_pages.commit(&context.id, &page.id) {
            return Err(self
                .compensate_browser_page_context(&context.id, primary)
                .await);
        }
        Ok(BrowserPageDescriptor { context, page })
    }

    async fn compensate_browser_page_context(
        &self,
        context_id: &str,
        primary: HostError,
    ) -> HostError {
        match self.close_context(context_id).await {
            Ok(()) => primary,
            Err(cleanup) => HostError::Backend(format!(
                "Browser.new_page failed: {primary}; Context cleanup failed: {cleanup}"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_page_owns_exact_context_until_removal() {
        // Arrange
        let pages = BrowserPages::new();
        pages.reserve("10").unwrap();
        pages.commit("10", "20").unwrap();

        // Act
        let owner = pages.context_for_page("20").unwrap();
        pages.remove_context("10").unwrap();

        // Assert
        assert_eq!(owner, Some(ObjectId::new(10)));
        assert!(!pages.owns_context("10").unwrap());
        assert_eq!(pages.context_for_page("20").unwrap(), None);
    }

    #[test]
    fn pending_context_is_already_exclusive() {
        // Arrange
        let pages = BrowserPages::new();

        // Act
        pages.reserve("10").unwrap();

        // Assert
        assert!(pages.owns_context("10").unwrap());
        assert!(pages.reserve("10").is_err());
    }

    #[test]
    fn prune_reclaims_only_closed_contexts() {
        // Arrange
        let pages = BrowserPages::new();
        pages.reserve("10").unwrap();
        pages.reserve("11").unwrap();

        // Act
        pages.prune(|context| context == ObjectId::new(11)).unwrap();

        // Assert
        assert!(!pages.owns_context("10").unwrap());
        assert!(pages.owns_context("11").unwrap());
    }
}
