//! Pre-admitted, metadata-only collection revisions on the existing watermark.
//! Native topology producers must reserve before activation and stage a complete
//! coherent projection. This module neither discovers nor owns browser objects.
use super::{
    FrameChange, FrameChangeValue, FrameMetadata, FramePublications, HostError, ResourceClass,
    ResourceLease, SCOPE,
};
use crate::{
    context_metadata::ContextSnapshot,
    frame_tree::{FrameNode, FrameTree, FrameTreeFactory, Storage},
};
use pwright_core::ObjectId;
use std::sync::{Arc, Weak};

pub(super) const CONTEXTS: usize = 16;
pub(super) const PAGES: usize = 16;

mod visibility;
pub use visibility::{
    CollectionMember, CollectionMembers, CollectionRebaseError, CollectionView,
    CollectionVisibilityError, VisibleMembers,
};

pub struct CollectionContext {
    pub id: ObjectId,
    pub settings: ContextSnapshot,
}

pub struct CollectionPage {
    pub id: ObjectId,
    pub context: ObjectId,
    pub main_frame: FrameNode,
    pub tree: FrameTree,
}

impl CollectionPage {
    /// Bind the admitted root descriptor, even before Page-ready visibility is
    /// committed or after native retirement. Never consult live host routes.
    pub fn bind_main_frame(&self) -> Result<crate::frame_tree::FrameBinding, HostError> {
        Ok(self
            .tree
            .lookup_resources()
            .reserve_binding()?
            .activate(self.main_frame.clone()))
    }
}

struct Projection {
    // Metadata-only identity of the publication domain; never retain its queue,
    // runtime or transport. Cloning this ledger performs no new allocation.
    domain: pwright_core::ResourceLedger,
    contexts: Vec<CollectionContext>,
    pages: Vec<CollectionPage>,
    _storage: Storage,
}

/// An immutable membership view; retained objects keep their existing metadata
/// publication semantics. Clones retain already charged metadata, never
/// the publication book, host, driver, runtime or browser transport.
#[derive(Clone)]
pub struct CollectionRevision(Arc<Projection>);

impl CollectionRevision {
    pub fn contexts(&self) -> &[CollectionContext] {
        &self.0.contexts
    }

    pub fn pages(&self) -> &[CollectionPage] {
        &self.0.pages
    }
}

/// The complete queue slot, both fixed vectors and Arc body exist before native
/// creation. Filling/staging an admitted reservation makes no new allocation.
pub struct CollectionReservation {
    book: Weak<FrameMetadata>,
    event: ResourceLease,
    revision: CollectionRevision,
}

const STORAGE_BYTES: usize = std::mem::size_of::<Projection>()
    + 2 * std::mem::size_of::<usize>()
    + CONTEXTS * std::mem::size_of::<CollectionContext>()
    + PAGES * std::mem::size_of::<CollectionPage>()
    // Conservatively retain the reservation's body allowance until the final
    // revision reader releases it, including producer/publication overlap.
    + std::mem::size_of::<CollectionReservation>()
    + std::mem::size_of::<CollectionBatch>()
    + std::mem::size_of::<CollectionView>();

/// A synchronous lifecycle projection transaction. On failure the caller must
/// retire/discard the session: dropping an uncommitted batch keeps its changes
/// unpublished until terminal teardown rather than exposing half an update.
pub struct CollectionBatch {
    book: FramePublications,
    start: u64,
    reserved: CollectionReservation,
}

impl CollectionBatch {
    pub fn reservation(&mut self) -> &mut CollectionReservation {
        &mut self.reserved
    }

    pub fn commit(self) -> Result<CollectionRevision, HostError> {
        self.commit_view(|revision| Ok(CollectionView::full(revision.clone())))
            .map(|view| view.revision().clone())
    }

    /// Commit an exact visibility projection of this admitted revision. The
    /// selector runs outside the queue mutex while the batch publication gate
    /// remains closed. Failure leaves the batch unpublished for Session teardown.
    pub fn commit_view(
        self,
        select: impl FnOnce(&CollectionRevision) -> Result<CollectionView, HostError>,
    ) -> Result<CollectionView, HostError> {
        let view = select(&self.reserved.revision)?;
        if !Arc::ptr_eq(&view.revision().0, &self.reserved.revision.0) {
            return Err(HostError::InvalidArgument(
                "collection batch must publish its own admitted revision".into(),
            ));
        }
        let mut state = self.book.0.state.lock().map_err(HostError::backend)?;
        if state.discarded || state.sequence == u64::MAX {
            return Err(HostError::TargetClosed);
        }
        if state.collection_batch != Some(self.start) {
            return Err(HostError::Backend(
                "collection batch ownership changed".into(),
            ));
        }
        let sequence = state.next_sequence()?;
        state.pending.push_back(FrameChange {
            sequence,
            value: FrameChangeValue::Collection(view.clone()),
            _event: self.reserved.event,
        });
        state.sequence = sequence;
        state.collection_batch = None;
        self.book.0.changed.notify_one();
        Ok(view)
    }
}

impl CollectionReservation {
    /// Retain the admitted native descriptors without enqueuing a full-visible
    /// replacement. A lifecycle producer can instead stage incremental groups.
    /// This releases the unused queue-slot lease; the projection stays charged.
    pub fn into_revision(self) -> CollectionRevision {
        self.revision
    }

    pub fn add_context(
        &mut self,
        id: ObjectId,
        settings: ContextSnapshot,
    ) -> Result<(), HostError> {
        let projection = Arc::get_mut(&mut self.revision.0)
            .expect("an unpublished reservation owns its projection exclusively");
        if id.get() == 0
            || projection.contexts.iter().any(|context| context.id == id)
            || projection
                .pages
                .iter()
                .any(|page| page.id == id || page.main_frame.id() == id)
        {
            return Err(HostError::InvalidArgument(
                "invalid or duplicate collection Context identity".into(),
            ));
        }
        if projection.contexts.len() == CONTEXTS {
            return Err(HostError::ResourceExhausted(
                "published Context capacity exceeded (16)".into(),
            ));
        }
        projection.contexts.push(CollectionContext { id, settings });
        Ok(())
    }

    pub fn add_page(
        &mut self,
        id: ObjectId,
        context: ObjectId,
        tree: FrameTree,
    ) -> Result<(), HostError> {
        let projection = Arc::get_mut(&mut self.revision.0)
            .expect("an unpublished reservation owns its projection exclusively");
        if id.get() == 0
            || projection.pages.iter().any(|page| page.id == id)
            || projection.contexts.iter().any(|entry| entry.id == id)
            || !projection.contexts.iter().any(|entry| entry.id == context)
        {
            return Err(HostError::InvalidArgument(
                "invalid collection Page identity or missing Context".into(),
            ));
        }
        if projection.pages.len() == PAGES {
            return Err(HostError::ResourceExhausted(
                "published Page capacity exceeded (16)".into(),
            ));
        }
        let main_frame = tree.main_frame_snapshot()?;
        if main_frame.id().get() == 0
            || main_frame.id() == id
            || projection
                .contexts
                .iter()
                .any(|entry| entry.id == main_frame.id())
            || projection.pages.iter().any(|page| {
                page.main_frame.id() == main_frame.id()
                    || page.id == main_frame.id()
                    || page.main_frame.id() == id
            })
        {
            return Err(HostError::InvalidArgument(
                "collection main Frame already belongs to a Page".into(),
            ));
        }
        projection.pages.push(CollectionPage {
            id,
            context,
            main_frame,
            tree,
        });
        Ok(())
    }
}

impl FramePublications {
    pub fn begin_collection(
        &self,
        reserved: CollectionReservation,
    ) -> Result<CollectionBatch, HostError> {
        let mut state = self.0.state.lock().map_err(HostError::backend)?;
        if !reserved.book.ptr_eq(&Arc::downgrade(&self.0)) {
            return Err(HostError::InvalidArgument(
                "collection reservation belongs to another publication queue".into(),
            ));
        }
        if state.discarded || state.sequence == u64::MAX {
            return Err(HostError::TargetClosed);
        }
        if state.collection_batch.is_some() {
            return Err(HostError::Backend(
                "collection batch is already active".into(),
            ));
        }
        let start = state.sequence;
        state.collection_batch = Some(start);
        Ok(CollectionBatch {
            book: self.clone(),
            start,
            reserved,
        })
    }

    pub fn reserve_collection(
        &self,
        factory: &FrameTreeFactory,
    ) -> Result<CollectionReservation, HostError> {
        let state = self.0.state.lock().map_err(HostError::backend)?;
        if state.discarded || state.sequence == u64::MAX {
            return Err(HostError::TargetClosed);
        }
        if state.queue_storage.is_none() {
            return Err(HostError::UnsupportedOperation(
                "collection publication requires a preallocated metadata queue".into(),
            ));
        }
        let event = self
            .0
            .resources
            .reserve(SCOPE, ResourceClass::EventItem, 1)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let storage = factory.reserve_collection_storage(STORAGE_BYTES)?;
        let mut contexts = Vec::new();
        contexts
            .try_reserve_exact(CONTEXTS)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let mut pages = Vec::new();
        pages
            .try_reserve_exact(PAGES)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        Ok(CollectionReservation {
            book: Arc::downgrade(&self.0),
            event,
            revision: CollectionRevision(Arc::new(Projection {
                domain: self.0.resources.clone(),
                contexts,
                pages,
                _storage: storage,
            })),
        })
    }

    pub fn stage_collection(&self, reserved: CollectionReservation) -> Result<(), HostError> {
        let mut state = self.0.state.lock().map_err(HostError::backend)?;
        if !reserved.book.ptr_eq(&Arc::downgrade(&self.0)) {
            return Err(HostError::InvalidArgument(
                "collection reservation belongs to another publication queue".into(),
            ));
        }
        if state.discarded || state.sequence == u64::MAX {
            return Err(HostError::TargetClosed);
        }
        if state.collection_batch.is_some() {
            return Err(HostError::Backend(
                "collection batch must commit its own revision".into(),
            ));
        }
        let sequence = state.next_sequence()?;
        state.pending.push_back(FrameChange {
            sequence,
            value: FrameChangeValue::Collection(CollectionView::full(reserved.revision)),
            _event: reserved.event,
        });
        state.sequence = sequence;
        self.0.changed.notify_one();
        Ok(())
    }

    /// No topology lock or allocation. A retained revision keeps its explicit
    /// budget lease, even if a later fence replaces the current projection.
    pub fn published_collections(&self) -> Result<Option<CollectionView>, HostError> {
        Ok(self
            .0
            .state
            .lock()
            .map_err(HostError::backend)?
            .collections
            .clone())
    }
}

#[cfg(test)]
mod tests;
