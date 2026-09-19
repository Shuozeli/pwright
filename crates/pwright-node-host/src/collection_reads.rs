//! Selected foreign-runtime reads use published metadata, never native routes.
//! The addon retains its existing conversion guard through actual N-API output.
use crate::{ContextDescriptor, HostError, HostSession, PageDescriptor, PageSnapshot};

impl HostSession {
    pub fn published_contexts(&self) -> Result<Vec<ContextDescriptor>, HostError> {
        let Some(view) = self.frame_publications().published_collections()? else {
            return Ok(Vec::new());
        };
        Ok(view
            .contexts()
            .iter()
            .map(|context| ContextDescriptor {
                id: context.id.get().to_string(),
                settings: context.settings.clone(),
                settings_binding: None,
            })
            .collect())
    }

    pub fn published_pages(&self, context_id: &str) -> Result<Vec<PageDescriptor>, HostError> {
        Ok(self.published_page_snapshot(context_id, &[])?.new_pages)
    }

    pub fn published_page_snapshot(
        &self,
        context_id: &str,
        known: &[String],
    ) -> Result<PageSnapshot, HostError> {
        if known.len() > 256 || known.iter().any(|id| id.len() > 20) {
            return Err(HostError::InvalidArgument(
                "Known Page IDs exceed snapshot bounds".into(),
            ));
        }
        let mut result = PageSnapshot {
            ids: Vec::new(),
            new_pages: Vec::new(),
        };
        let Some(view) = self.frame_publications().published_collections()? else {
            return Ok(result);
        };
        for page in view
            .pages()
            .iter()
            .filter(|page| page.context.get().to_string() == context_id)
        {
            let id = page.id.get().to_string();
            let cached = known.contains(&id);
            result.ids.push(id.clone());
            if cached {
                continue;
            }
            result.new_pages.push(PageDescriptor {
                id,
                context_id: context_id.to_owned(),
                main_frame_id: page.main_frame.id().get().to_string(),
                main_frame_metadata: page.main_frame.metadata_snapshot(),
                frame_tree: page.tree.clone(),
                main_frame_binding: None,
                frame_tree_binding: page.tree.bind_tree()?,
            });
        }
        Ok(result)
    }
}
