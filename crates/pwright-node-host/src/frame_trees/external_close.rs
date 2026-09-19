//! Pre-admitted external retirement source on the existing serial driver.
use super::close_events::{disposed, stage_notice};
use super::*;
use crate::{CollectionMember, FramePublications, LifecycleReservation};
use pwright_backend_cdp_topology::CdpTopologyEffect;

const NOTICES: usize = 32;

pub(super) struct ExternalClosePool {
    reserved: Vec<LifecycleReservation>,
    _storage: Storage,
}

impl ExternalClosePool {
    fn reserve(
        book: &FramePublications,
        factory: &FrameTreeFactory,
    ) -> Result<Box<Self>, HostError> {
        let storage = factory.reserve_collection_storage(
            std::mem::size_of::<Self>() + NOTICES * std::mem::size_of::<LifecycleReservation>(),
        )?;
        let mut pool = Self {
            reserved: Vec::with_capacity(NOTICES),
            _storage: storage,
        };
        pool.replenish(book, factory)?;
        Ok(Box::new(pool))
    }

    pub(super) fn len(&self) -> usize {
        self.reserved.len()
    }

    pub(super) fn truncate(&mut self, len: usize) {
        self.reserved.truncate(len);
    }

    pub(super) fn replenish(
        &mut self,
        book: &FramePublications,
        factory: &FrameTreeFactory,
    ) -> Result<(), HostError> {
        let before = self.reserved.len();
        while self.reserved.len() < NOTICES {
            match book.reserve_lifecycle(factory) {
                Ok(reserved) => self.reserved.push(reserved),
                Err(error) => {
                    self.reserved.truncate(before);
                    return Err(error);
                }
            }
        }
        Ok(())
    }

    fn take(&mut self) -> Result<LifecycleReservation, HostError> {
        self.reserved.pop().ok_or_else(|| {
            HostError::Backend(
                "external retirement capacity was not replenished before activation".into(),
            )
        })
    }
}

impl FrameTrees {
    pub(crate) fn with_external_close_publication(self) -> Result<Self, HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed || !state.pages.is_empty() {
            return Err(HostError::InvalidArgument(
                "external close must be selected before initialization".into(),
            ));
        }
        let collection = state.collections.as_mut().ok_or_else(|| {
            HostError::InvalidArgument("external close requires collections".into())
        })?;
        if !collection.close_events || collection.external.is_some() || collection.native.is_some()
        {
            return Err(HostError::InvalidArgument(
                "external close requires a unique pre-initialization close selection".into(),
            ));
        }
        collection.external = Some(ExternalClosePool::reserve(
            &self.urls.publications(),
            &self.factory,
        )?);
        drop(state);
        Ok(self)
    }

    pub(super) fn external_disposals(
        &self,
        effects: &[CdpTopologyEffect],
    ) -> Result<(), HostError> {
        let result = self.stage_external_disposals(effects);
        if result.is_err() {
            // The external native event already happened; no rollback or silent
            // omission is possible. Existing driver error handling retires it.
            self.state
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .closed = true;
        }
        result
    }

    fn stage_external_disposals(&self, effects: &[CdpTopologyEffect]) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        let Some(collection) = state
            .collections
            .as_ref()
            .filter(|value| value.external.is_some())
        else {
            return Ok(());
        };
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        let Some(view) = collection.planned.clone() else {
            return Ok(());
        };
        let needed = view
            .contexts()
            .iter()
            .filter(|context| disposed(effects, context.id))
            .count()
            + view
                .pages()
                .iter()
                .filter(|page| disposed(effects, page.id) || disposed(effects, page.context))
                .count();
        if collection.external.as_ref().expect("selected pool").len() < needed {
            return Err(HostError::Backend(
                "external close exceeds pre-admitted retirement capacity".into(),
            ));
        }
        let mut before = view.clone();
        for context in view.contexts() {
            for page in view.pages().iter().filter(|page| {
                page.context == context.id
                    && (disposed(effects, page.id) || disposed(effects, context.id))
            }) {
                let reserved = state
                    .collections
                    .as_mut()
                    .expect("selected collections")
                    .external
                    .as_mut()
                    .expect("selected pool")
                    .take()?;
                before = stage_notice(
                    self,
                    &mut state,
                    before,
                    CollectionMember::Page(page.id),
                    reserved,
                )?;
            }
            if disposed(effects, context.id) {
                let reserved = state
                    .collections
                    .as_mut()
                    .expect("selected collections")
                    .external
                    .as_mut()
                    .expect("selected pool")
                    .take()?;
                before = stage_notice(
                    self,
                    &mut state,
                    before,
                    CollectionMember::Context(context.id),
                    reserved,
                )?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
