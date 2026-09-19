//! Terminal publications are admitted before native Page creation, not during
//! shutdown. They share the existing queue and its fixed publication watermarks.
use super::{
    FrameChange, FrameChangeValue, FrameMetadata, FramePublications, FrameTree, HostError,
    ResourceClass, ResourceLease, SCOPE, Storage,
};
use std::sync::Arc;
use std::sync::Weak;

pub(crate) struct FrameClosePermit {
    book: Weak<FrameMetadata>,
    pub(super) event: ResourceLease,
}

impl FrameMetadata {
    pub(crate) fn publication_storage_bytes(&self) -> Result<usize, HostError> {
        self.limits
            .publications
            .checked_mul(std::mem::size_of::<FrameChange>())
            .and_then(|bytes| {
                bytes.checked_add(
                    std::mem::size_of::<Storage>()
                        + 2 * std::mem::size_of::<usize>()
                        + std::mem::size_of::<Option<super::CollectionView>>()
                        + std::mem::size_of::<Option<u64>>()
                        + std::mem::size_of::<super::drain::DrainOwnership>()
                        + std::mem::size_of::<super::FramePublicationDrain>(),
                )
            })
            .ok_or_else(|| HostError::ResourceExhausted("metadata queue storage overflow".into()))
    }

    pub(crate) fn admit_page_closure(&self, storage: Arc<Storage>) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if self.limits.publications == 0
            || state.queue_storage.is_some()
            || state.pending.capacity() != 0
            || state.discarded
        {
            return Err(HostError::InvalidArgument(
                "Page closure requires a fresh bounded publication queue".into(),
            ));
        }
        state
            .pending
            .try_reserve_exact(self.limits.publications)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        state.queue_storage = Some(storage);
        Ok(())
    }
}

impl FramePublications {
    pub(crate) fn reserve_close(&self) -> Result<FrameClosePermit, HostError> {
        let state = self.0.state.lock().map_err(HostError::backend)?;
        if state.discarded || state.sequence == u64::MAX {
            return Err(HostError::TargetClosed);
        }
        if state.queue_storage.is_none() {
            return Err(HostError::UnsupportedOperation(
                "Page close publication is not selected".into(),
            ));
        }
        let event = self
            .0
            .resources
            .reserve(SCOPE, ResourceClass::EventItem, 1)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        Ok(FrameClosePermit {
            book: Arc::downgrade(&self.0),
            event,
        })
    }

    pub(crate) fn stage_close(
        &self,
        tree: &FrameTree,
        permit: &mut Option<FrameClosePermit>,
    ) -> Result<(), HostError> {
        let mut state = self.0.state.lock().map_err(HostError::backend)?;
        if state.discarded {
            return Err(HostError::TargetClosed);
        }
        let reserved = permit
            .as_ref()
            .ok_or_else(|| HostError::Backend("Page close permit already consumed".into()))?;
        if !reserved.book.ptr_eq(&Arc::downgrade(&self.0)) {
            return Err(HostError::InvalidArgument(
                "Page close permit belongs to another publication queue".into(),
            ));
        }
        let sequence = state.next_sequence()?;
        let publication = tree.close()?;
        let reserved = permit.take().expect("validated close permit");
        state.pending.push_back(FrameChange {
            sequence,
            value: FrameChangeValue::Close(publication),
            _event: reserved.event,
        });
        state.sequence = sequence;
        self.0.changed.notify_one();
        Ok(())
    }

    /// Called once the serial lifecycle driver can publish no more Pages.
    /// All queue slots and bytes were admitted at Page creation. MAX cannot be
    /// observed by an earlier fence; publish the complete batch before signalling
    /// disconnect. Environment destruction discards it without foreign callbacks.
    pub(crate) fn close_for_teardown<'a>(
        &self,
        pages: impl Iterator<Item = (&'a FrameTree, &'a mut Option<FrameClosePermit>)>,
    ) {
        let mut state = self
            .0
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.sequence = u64::MAX;
        state.collection_batch = None;
        for (tree, permit) in pages {
            let publication = tree.close_for_teardown();
            if let Some(reserved) = permit.take()
                && !state.discarded
            {
                state.pending.push_back(FrameChange {
                    sequence: u64::MAX,
                    value: FrameChangeValue::Close(publication),
                    _event: reserved.event,
                });
            }
        }
        self.0.changed.notify_one();
    }
}

#[cfg(test)]
mod tests;
