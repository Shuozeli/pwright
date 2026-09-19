//! Native readiness/linkage, independent from discovery and snapshot visibility.
use super::*;
use crate::frame_metadata::CreationDelivery;
use crate::{CollectionView, FramePublications, LifecycleCause, LifecycleReservation};
use pwright_backend_cdp_topology::CdpTopologyEffect;

const GROUPS: usize = 32;

#[cfg(test)]
mod tests;

pub(super) struct ReadySource {
    reserved: Vec<LifecycleReservation>,
    _storage: Storage,
}

impl ReadySource {
    fn reserve(
        book: &FramePublications,
        factory: &FrameTreeFactory,
    ) -> Result<Box<Self>, HostError> {
        let storage = factory.reserve_collection_storage(
            std::mem::size_of::<Self>() + GROUPS * std::mem::size_of::<LifecycleReservation>(),
        )?;
        let mut source = Self {
            reserved: Vec::with_capacity(GROUPS),
            _storage: storage,
        };
        source.replenish(book, factory)?;
        Ok(Box::new(source))
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
        let before = self.len();
        while self.len() < GROUPS {
            match book.reserve_lifecycle(factory) {
                Ok(reserved) => self.reserved.push(reserved),
                Err(error) => {
                    self.truncate(before);
                    return Err(error);
                }
            }
        }
        Ok(())
    }
}

impl FrameTrees {
    pub(crate) fn with_ready_publication(self) -> Result<Self, HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed || !state.pages.is_empty() {
            return Err(HostError::InvalidArgument(
                "ready source must precede Page initialization".into(),
            ));
        }
        let collection = state.collections.as_mut().ok_or_else(|| {
            HostError::InvalidArgument("ready source requires collections".into())
        })?;
        if collection.external.is_none()
            || collection.ready.is_some()
            || collection.native.is_some()
        {
            return Err(HostError::InvalidArgument(
                "ready source requires unique pre-initialization close selection".into(),
            ));
        }
        collection.ready = Some(ReadySource::reserve(
            &self.urls.publications(),
            &self.factory,
        )?);
        drop(state);
        Ok(self)
    }

    pub(super) fn reserve_ready_delivery(
        &self,
        state: &State,
    ) -> Result<Option<CreationDelivery>, HostError> {
        state
            .collections
            .as_ref()
            .is_some_and(|collection| collection.ready.is_some())
            .then(|| CreationDelivery::reserve(&self.factory))
            .transpose()
    }

    pub(super) fn publish_ready_causes(
        &self,
        effects: &[CdpTopologyEffect],
    ) -> Result<(), HostError> {
        let result = self.stage_ready_causes(effects);
        if result.is_err() {
            self.state
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .closed = true;
        }
        result
    }

    fn stage_ready_causes(&self, effects: &[CdpTopologyEffect]) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state
            .collections
            .as_ref()
            .is_none_or(|collection| collection.ready.is_none())
        {
            return Ok(());
        }
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        for effect in effects {
            let collection = state.collections.as_ref().expect("selected collection");
            let before = collection.planned.clone().ok_or_else(|| {
                HostError::Backend("ready source has no planned projection".into())
            })?;
            let (page, cause) = match *effect {
                CdpTopologyEffect::PageReady { page, context } => {
                    if before.pages().iter().any(|entry| entry.id == page) {
                        continue;
                    }
                    let revision = collection.native.clone().ok_or_else(|| {
                        HostError::Backend("ready source has no native revision".into())
                    })?;
                    if !revision
                        .pages()
                        .iter()
                        .any(|entry| entry.id == page && entry.context == context)
                    {
                        return Err(HostError::Backend(
                            "native ready Page is missing from its Context projection".into(),
                        ));
                    }
                    (
                        page,
                        LifecycleCause::PageReady {
                            revision,
                            page,
                            opener: None,
                        },
                    )
                }
                CdpTopologyEffect::PopupLinked { opener, popup } => {
                    // Preserve native effect order, including intervening links.
                    // The Page's acknowledgement token carries the dependency
                    // across groups and across separate native transactions.
                    (popup, LifecycleCause::PopupLinked { opener, popup })
                }
                _ => continue,
            };
            let delivery = state
                .pages
                .iter()
                .find(|entry| entry.id == page)
                .and_then(|entry| entry.ready.clone())
                .ok_or_else(|| {
                    HostError::Backend("Page readiness acknowledgement was not admitted".into())
                })?;
            let reserved = state
                .collections
                .as_mut()
                .expect("selected collection")
                .ready
                .as_mut()
                .expect("selected source")
                .reserved
                .pop()
                .ok_or_else(|| {
                    HostError::ResourceExhausted(
                        "pre-admitted ready group capacity exhausted".into(),
                    )
                })?;
            let after: CollectionView = self.urls.publications().stage_lifecycle(
                reserved.track_creation(delivery),
                before,
                cause,
            )?;
            state
                .collections
                .as_mut()
                .expect("selected collection")
                .planned = Some(after);
        }
        Ok(())
    }
}
