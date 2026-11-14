//! Selected Node collection lifecycle ownership. Native admission stays in the
//! serial topology driver; metadata publication uses its existing queue only.
use super::*;
use crate::context_metadata::{ContextMetadata, ContextSnapshot};
use crate::{CollectionReservation, CollectionRevision};

// At most 16 Contexts and 16 Pages can retire without an intervening creation.
// Creation replenishes these admitted revisions before native activation.
const RETIREMENTS: usize = 32;

#[cfg(test)]
mod tests;

pub(super) struct CollectionState {
    contexts: ContextMetadata,
    pub(super) native: Option<CollectionRevision>,
    pub(super) planned: Option<crate::CollectionView>,
    pub(super) close_events: bool,
    pub(super) external: Option<Box<super::external_close::ExternalClosePool>>,
    pub(super) ready: Option<Box<super::ready::ReadySource>>,
    creation: Option<CollectionReservation>,
    retirements: Vec<CollectionReservation>,
    pub(super) terminal: Option<Box<crate::frame_metadata::TerminalLifecycle>>,
    _storage: Storage,
}

/// Roll back newly replenished retirement capacity if any later creation
/// admission/native step fails. Only successful native registration commits it.
pub(crate) struct CollectionCreation<'a> {
    owner: &'a FrameTrees,
    reserved: Option<CollectionReservation>,
    prior: usize,
    prior_external: usize,
    prior_ready: usize,
    committed: bool,
}

const _: () = assert!(
    std::mem::size_of::<CollectionCreation<'_>>()
        <= std::mem::size_of::<CollectionReservation>()
            + std::mem::size_of::<crate::CollectionBatch>()
);

impl CollectionCreation<'_> {
    pub(crate) fn commit(mut self) -> Result<(), HostError> {
        self.owner
            .install_creation_collection(self.reserved.take())?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for CollectionCreation<'_> {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        let mut state = self
            .owner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(collection) = state.collections.as_mut() {
            collection.retirements.truncate(self.prior);
            if let Some(external) = collection.external.as_mut() {
                external.truncate(self.prior_external);
            }
            if let Some(ready) = collection.ready.as_mut() {
                ready.truncate(self.prior_ready);
            }
        }
    }
}

fn settings(
    metadata: &ContextMetadata,
    view: ChromiumFrameOwnershipView<'_>,
    id: ObjectId,
) -> Result<Option<ContextSnapshot>, HostError> {
    let Some(context) = view.collection_contexts().find(|context| context.id == id) else {
        return Ok(None);
    };
    metadata.visible(id, context.owned)
}

impl FrameTrees {
    pub fn with_collections(self, contexts: ContextMetadata) -> Result<Self, HostError> {
        let storage = self.factory.reserve_collection_storage(
            RETIREMENTS * std::mem::size_of::<CollectionReservation>(),
        )?;
        let mut collection = CollectionState {
            contexts,
            native: None,
            planned: None,
            close_events: false,
            external: None,
            ready: None,
            creation: None,
            retirements: Vec::with_capacity(RETIREMENTS),
            terminal: None,
            _storage: storage,
        };
        self.replenish_retirements(&mut collection)?;
        self.state.lock().map_err(HostError::backend)?.collections = Some(collection);
        Ok(self)
    }

    /// Select terminal lifecycle production before any native Page is indexed.
    /// Ordinary native source groups are a separate, still pending integration.
    pub(crate) fn with_terminal_publication(self) -> Result<Self, HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed || !state.pages.is_empty() {
            return Err(HostError::InvalidArgument(
                "terminal publication must be selected before initialization".into(),
            ));
        }
        let collection = state.collections.as_mut().ok_or_else(|| {
            HostError::InvalidArgument("terminal publication requires collection projection".into())
        })?;
        if collection.terminal.is_some() {
            return Err(HostError::InvalidArgument(
                "terminal publication already selected".into(),
            ));
        }
        collection.terminal = Some(crate::frame_metadata::TerminalLifecycle::reserve(
            &self.urls.publications(),
            &self.factory,
        )?);
        drop(state);
        Ok(self)
    }

    pub(super) fn terminal_publication(
        state: &mut State,
    ) -> Option<(
        Box<crate::frame_metadata::TerminalLifecycle>,
        crate::CollectionView,
    )> {
        let collection = state.collections.as_mut()?;
        let terminal = collection.terminal.take()?;
        // Preserve staged close masks even before snapshot reconciliation.
        // The fallback serves initial collection-only producers; future ready
        // sources must likewise keep the exact planned visibility here.
        let view = collection
            .planned
            .clone()
            .or_else(|| collection.native.clone().map(crate::CollectionView::full))?;
        Some((terminal, view))
    }

    fn replenish_retirements(&self, state: &mut CollectionState) -> Result<(), HostError> {
        let before = state.retirements.len();
        let external_before = state.external.as_ref().map_or(0, |value| value.len());
        while state.retirements.len() < RETIREMENTS {
            match self.urls.publications().reserve_collection(&self.factory) {
                Ok(reserved) => state.retirements.push(reserved),
                Err(error) => {
                    state.retirements.truncate(before);
                    return Err(error);
                }
            }
        }
        if let Some(external) = state.external.as_mut()
            && let Err(error) = external.replenish(&self.urls.publications(), &self.factory)
        {
            state.retirements.truncate(before);
            return Err(error);
        }
        if let Some(ready) = state.ready.as_mut()
            && let Err(error) = ready.replenish(&self.urls.publications(), &self.factory)
        {
            state.retirements.truncate(before);
            if let Some(external) = state.external.as_mut() {
                external.truncate(external_before);
            }
            return Err(error);
        }
        Ok(())
    }

    pub(crate) fn reserve_creation_collection(&self) -> Result<CollectionCreation<'_>, HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        let Some(collection) = state.collections.as_mut() else {
            return Ok(CollectionCreation {
                owner: self,
                reserved: None,
                prior: 0,
                prior_external: 0,
                prior_ready: 0,
                committed: false,
            });
        };
        if collection.creation.is_some() {
            return Err(HostError::Backend(
                "previous creation collection is not reconciled".into(),
            ));
        }
        let reserved = self.urls.publications().reserve_collection(&self.factory)?;
        let prior = collection.retirements.len();
        let prior_external = collection.external.as_ref().map_or(0, |value| value.len());
        let prior_ready = collection.ready.as_ref().map_or(0, |value| value.len());
        self.replenish_retirements(collection)?;
        Ok(CollectionCreation {
            owner: self,
            reserved: Some(reserved),
            prior,
            prior_external,
            prior_ready,
            committed: false,
        })
    }

    fn install_creation_collection(
        &self,
        reserved: Option<CollectionReservation>,
    ) -> Result<(), HostError> {
        let Some(reserved) = reserved else {
            return Ok(());
        };
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        let collection = state
            .collections
            .as_mut()
            .ok_or_else(|| HostError::Backend("collection projection is not installed".into()))?;
        if collection.creation.is_some() {
            return Err(HostError::Backend(
                "creation projection already reserved".into(),
            ));
        }
        collection.creation = Some(reserved);
        Ok(())
    }

    pub(super) fn reconcile_collections(
        &self,
        view: ChromiumFrameOwnershipView<'_>,
        effects: &[CdpTopologyEffect],
    ) -> Result<(), HostError> {
        let projection = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .collections
                .as_ref()
                .map(|value| (value.contexts.clone(), value.native.clone()))
        };
        let Some((metadata, previous)) = projection else {
            return self.reconcile_with_effects(
                view.frames(),
                |page| view.contains_page(page),
                effects,
            );
        };
        let mut same = previous.is_some();
        let mut additions = false;
        let mut context_count = 0;
        for context in view.collection_contexts() {
            if metadata.visible(context.id, context.owned)?.is_none() {
                continue;
            }
            let prior = previous
                .as_ref()
                .map(|value| value.contexts())
                .unwrap_or(&[]);
            same &= prior
                .get(context_count)
                .is_some_and(|entry| entry.id == context.id);
            additions |= !prior.iter().any(|entry| entry.id == context.id);
            context_count += 1;
        }
        let mut page_count = 0;
        for page in view.collection_pages() {
            if settings(&metadata, view, page.context)?.is_none() {
                continue;
            }
            let prior = previous.as_ref().map(|value| value.pages()).unwrap_or(&[]);
            same &= prior
                .get(page_count)
                .is_some_and(|entry| entry.id == page.id && entry.context == page.context);
            additions |= !prior
                .iter()
                .any(|entry| entry.id == page.id && entry.context == page.context);
            page_count += 1;
        }
        let prior_count = previous
            .as_ref()
            .map_or(0, |value| value.contexts().len() + value.pages().len());
        same &= previous.as_ref().is_some_and(|value| {
            value.contexts().len() == context_count && value.pages().len() == page_count
        });
        if same {
            return self.reconcile_with_effects(
                view.frames(),
                |page| view.contains_page(page),
                effects,
            );
        }
        let reserved = {
            let mut state = self.state.lock().map_err(HostError::backend)?;
            let collection = state
                .collections
                .as_mut()
                .expect("selected collection owner");
            if let Some(reserved) = collection.creation.take() {
                reserved
            } else if additions || context_count + page_count >= prior_count {
                let reserved = self.urls.publications().reserve_collection(&self.factory)?;
                if additions {
                    self.replenish_retirements(collection)?;
                }
                reserved
            } else {
                collection.retirements.pop().ok_or_else(|| {
                    HostError::Backend(
                        "pre-admitted collection retirement capacity exhausted".into(),
                    )
                })?
            }
        };
        let mut batch = self.urls.publications().begin_collection(reserved)?;
        self.reconcile_with_effects(view.frames(), |page| view.contains_page(page), effects)?;
        for context in view.collection_contexts() {
            if let Some(snapshot) = metadata.visible(context.id, context.owned)? {
                batch.reservation().add_context(context.id, snapshot)?;
            }
        }
        for page in view.collection_pages() {
            if settings(&metadata, view, page.context)?.is_some() {
                batch
                    .reservation()
                    .add_page(page.id, page.context, self.get(page.id)?)?;
            }
        }
        let (ready_selected, previous_view, internal_pages) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let collection = state.collections.as_ref().expect("selected collection");
            (
                collection.ready.is_some(),
                collection.planned.clone(),
                state
                    .pages
                    .iter()
                    .filter(|entry| entry.internal)
                    .map(|entry| entry.id)
                    .collect::<Vec<_>>(),
            )
        };
        let planned = batch.commit_view(|revision| {
            if !ready_selected {
                let mut visible = crate::CollectionView::full(revision.clone());
                for page in revision.pages() {
                    if internal_pages.contains(&page.id) {
                        visible = visible
                            .with_visibility(crate::CollectionMember::Page(page.id), false)
                            .map_err(HostError::backend)?;
                    }
                }
                return Ok(visible);
            }
            let mut visible = previous_view
                .as_ref()
                .map(|previous| previous.try_rebase(revision).map_err(HostError::backend))
                .transpose()?
                .unwrap_or_else(|| crate::CollectionView::hidden(revision.clone()));
            // Context creation has no Context-page event until a Page is ready.
            // Preserve known Pages; new Page descriptors remain hidden.
            for context in revision.contexts() {
                if !visible
                    .contexts()
                    .iter()
                    .any(|entry| entry.id == context.id)
                {
                    visible = visible
                        .with_visibility(crate::CollectionMember::Context(context.id), true)
                        .map_err(HostError::backend)?;
                }
            }
            for page in revision.pages() {
                if internal_pages.contains(&page.id)
                    && visible.pages().iter().any(|entry| entry.id == page.id)
                {
                    visible = visible
                        .with_visibility(crate::CollectionMember::Page(page.id), false)
                        .map_err(HostError::backend)?;
                }
            }
            Ok(visible)
        })?;
        let revision = planned.revision().clone();
        let mut state = self.state.lock().map_err(HostError::backend)?;
        let collection = state
            .collections
            .as_mut()
            .expect("selected collection owner");
        collection.planned = Some(planned);
        collection.native = Some(revision);
        Ok(())
    }
}
