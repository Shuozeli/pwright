use super::*;
mod deferred_tests;
use crate::{
    LifecycleDelivery, LifecycleNotice, PublicationDrainError, PublicationProgress,
    context_metadata::{ContextMetadata, ContextMetadataLimits},
    frame_metadata::{FrameMetadataLimits, FramePublications},
};

const CONTEXT: ObjectId = ObjectId::new(1000);
const FIRST: ObjectId = ObjectId::new(1);
const SECOND: ObjectId = ObjectId::new(3);

pub(in crate::frame_trees) struct Fixture {
    pub(in crate::frame_trees) owner: Arc<FrameTrees>,
    observer: Observer,
    pub(in crate::frame_trees) book: FramePublications,
}
fn effects(ids: &[ObjectId]) -> Vec<CdpTopologyEffect> {
    ids.iter()
        .map(|id| CdpTopologyEffect::Lifecycle(ControlMessage::Dispose { object_id: *id }))
        .collect()
}
impl Fixture {
    fn new() -> Self {
        Self::selected(false, false)
    }

    pub(in crate::frame_trees) fn external() -> Self {
        Self::selected(true, false)
    }

    pub(in crate::frame_trees) fn ready() -> Self {
        Self::selected(true, true)
    }

    pub(in crate::frame_trees) fn explicit_close(&self, ids: &[ObjectId]) {
        let native = effects(ids);
        Box::new(self.observer.prepare_close(&native).unwrap())
            .committed(&native)
            .unwrap();
    }

    fn selected(external: bool, ready: bool) -> Self {
        let metadata = Arc::new(
            FrameMetadata::new(FrameMetadataLimits {
                entries: 128,
                url_bytes: 64,
                total_bytes: 8192,
                publications: if external { 256 } else { 100 },
            })
            .unwrap(),
        );
        let contexts = ContextMetadata::new(ContextMetadataLimits {
            active_contexts: 16,
            states: 256,
            conversions: 128,
            base_url_bytes: 64,
            bytes: 8192,
        })
        .unwrap();
        let snapshot = contexts
            .reserve_creation(None)
            .unwrap()
            .commit(CONTEXT)
            .unwrap()
            .snapshot;
        let owner = FrameTrees::new(metadata.clone())
            .unwrap()
            .with_collections(contexts)
            .unwrap()
            .with_terminal_publication()
            .unwrap()
            .with_explicit_close_publication()
            .unwrap();
        let owner = if external {
            owner.with_external_close_publication().unwrap()
        } else {
            owner
        };
        let owner = Arc::new(if ready {
            owner.with_ready_publication().unwrap()
        } else {
            owner
        });
        owner
            .reconcile(
                [FIRST, SECOND]
                    .map(|page| ChromiumFrameOwnership {
                        frame: ObjectId::new(page.get() + 1),
                        page,
                        parent: None,
                    })
                    .into_iter(),
                |_| true,
            )
            .unwrap();
        let book = metadata.publications();
        let mut reserved = book.reserve_collection(&owner.factory).unwrap();
        reserved.add_context(CONTEXT, snapshot).unwrap();
        for page in [FIRST, SECOND] {
            reserved
                .add_page(page, CONTEXT, owner.get(page).unwrap())
                .unwrap();
        }
        let planned = book
            .begin_collection(reserved)
            .unwrap()
            .commit_view(|revision| {
                if ready {
                    CollectionView::hidden(revision.clone())
                        .with_visibility(CollectionMember::Context(CONTEXT), true)
                        .map_err(HostError::backend)
                } else {
                    Ok(CollectionView::full(revision.clone()))
                }
            })
            .unwrap();
        let revision = planned.revision().clone();
        let mut state = owner.state.lock().unwrap();
        let collection = state.collections.as_mut().unwrap();
        collection.native = Some(revision.clone());
        collection.planned = Some(planned);
        drop(state);
        book.fence().unwrap().publish().unwrap();
        let storage = owner
            .factory
            .reserve_collection_storage(std::mem::size_of::<Observer>())
            .unwrap();
        let observer = Observer {
            owner: owner.clone(),
            _storage: storage,
        };
        Self {
            owner,
            observer,
            book,
        }
    }

    pub(in crate::frame_trees) fn drain(&self) -> Vec<LifecycleNotice> {
        let mut drain = self.book.fence().unwrap().try_begin().unwrap();
        let mut notices = Vec::new();
        loop {
            match drain.publish_next() {
                Ok(PublicationProgress::Published) => {}
                Ok(PublicationProgress::Complete) => break,
                Err(PublicationDrainError::NoticePending) => {
                    notices.push(drain.commit_notice().unwrap());
                    drain
                        .acknowledge_notice(LifecycleDelivery::Delivered)
                        .unwrap();
                }
                other => panic!("unexpected drain result {other:?}"),
            }
        }
        notices
    }
}

#[test]
fn abandoned_close_admission_releases_all_storage_without_publishing() {
    // Arrange
    let fixture = Fixture::new();
    let before = fixture.owner.factory.metrics().unwrap();
    let receipt = fixture
        .observer
        .prepare_close(&effects(&[FIRST, SECOND, CONTEXT]))
        .unwrap();
    // Act
    drop(receipt);
    // Assert
    assert_eq!(fixture.owner.factory.metrics().unwrap(), before);
    assert!(!fixture.owner.get(FIRST).unwrap().is_closed().unwrap());
    assert_eq!(
        fixture.owner.urls.metrics().unwrap().pending_publications,
        0
    );
}

#[test]
fn context_close_commits_each_page_flag_before_its_own_notice() {
    // Arrange
    let fixture = Fixture::new();
    let native = effects(&[CONTEXT]);
    let receipt = fixture.observer.prepare_close(&native).unwrap();
    let first = fixture.owner.get(FIRST).unwrap();
    let second = fixture.owner.get(SECOND).unwrap();
    // Act
    Box::new(receipt).committed(&native).unwrap();
    // Assert
    assert!(!first.is_closed().unwrap());
    assert!(!second.is_closed().unwrap());
    let mut drain = fixture.book.fence().unwrap().try_begin().unwrap();
    assert_eq!(
        drain.commit_notice().unwrap(),
        LifecycleNotice::PageClose {
            context: CONTEXT,
            page: FIRST
        }
    );
    assert!(first.is_closed().unwrap());
    assert!(!second.is_closed().unwrap());
    drain
        .acknowledge_notice(LifecycleDelivery::ListenerThrew)
        .unwrap();
    assert_eq!(
        drain.commit_notice().unwrap(),
        LifecycleNotice::PageClose {
            context: CONTEXT,
            page: SECOND
        }
    );
    assert!(second.is_closed().unwrap());
    drain
        .acknowledge_notice(LifecycleDelivery::Delivered)
        .unwrap();
    assert_eq!(
        drain.commit_notice().unwrap(),
        LifecycleNotice::ContextClose { context: CONTEXT }
    );
    drain
        .acknowledge_notice(LifecycleDelivery::Delivered)
        .unwrap();
    assert_eq!(drain.publish_next().unwrap(), PublicationProgress::Complete);
    assert!(
        fixture
            .book
            .published_collections()
            .unwrap()
            .unwrap()
            .contexts()
            .is_empty()
    );
}

#[test]
fn context_disposal_expands_its_coherent_page_subtree_before_native_close() {
    // Arrange
    let fixture = Fixture::new();
    let before = fixture.owner.factory.metrics().unwrap();
    // Act
    let result = fixture
        .observer
        .prepare_close(&effects(&[CONTEXT]))
        .unwrap();
    // Assert
    assert_eq!(result.notices.len(), 3);
    assert!(!fixture.owner.get(FIRST).unwrap().is_closed().unwrap());
    drop(result);
    assert_eq!(fixture.owner.factory.metrics().unwrap(), before);
    assert!(!fixture.owner.state.lock().unwrap().closed);
}

#[test]
fn partial_group_admission_failure_rolls_back_earlier_groups() {
    // Arrange
    let fixture = Fixture::new();
    let mut held = Vec::new();
    while let Ok(reserved) = fixture.book.reserve_lifecycle(&fixture.owner.factory) {
        held.push(reserved);
    }
    assert!(held.len() > 2);
    held.pop();
    held.pop(); // only two of the three required notices can fit
    let before = fixture.owner.factory.metrics().unwrap();
    // Act
    let result = fixture
        .observer
        .prepare_close(&effects(&[FIRST, SECOND, CONTEXT]));
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(fixture.owner.factory.metrics().unwrap(), before);
    assert!(!fixture.owner.get(FIRST).unwrap().is_closed().unwrap());
}

#[test]
fn mismatched_committed_effects_fail_closed_and_terminal_can_finish() {
    // Arrange
    let fixture = Fixture::new();
    let receipt = fixture.observer.prepare_close(&effects(&[FIRST])).unwrap();
    // Act
    let result = Box::new(receipt).committed(&effects(&[SECOND]));
    // Assert
    assert!(result.is_err());
    assert!(fixture.owner.state.lock().unwrap().closed);
    fixture.owner.clear();
    assert_eq!(fixture.drain().len(), 3);
}

#[test]
fn reconciliation_does_not_duplicate_an_observed_page_close() {
    // Arrange
    let fixture = Fixture::new();
    let native = effects(&[FIRST]);
    let receipt = fixture.observer.prepare_close(&native).unwrap();
    Box::new(receipt).committed(&native).unwrap();
    // Act
    let result = fixture.owner.reconcile(
        [ChromiumFrameOwnership {
            frame: ObjectId::new(4),
            page: SECOND,
            parent: None,
        }]
        .into_iter(),
        |page| page == SECOND,
    );
    // Assert
    result.unwrap();
    assert_eq!(
        fixture.drain(),
        vec![LifecycleNotice::PageClose {
            context: CONTEXT,
            page: FIRST
        }]
    );
}

#[test]
fn terminal_handoff_uses_exact_planned_close_masks() {
    // Arrange
    let fixture = Fixture::new();
    let native = effects(&[FIRST]);
    let receipt = fixture.observer.prepare_close(&native).unwrap();
    Box::new(receipt).committed(&native).unwrap();
    // Act
    fixture.owner.clear();
    // Assert
    assert_eq!(
        fixture.drain(),
        vec![
            LifecycleNotice::PageClose {
                context: CONTEXT,
                page: FIRST
            },
            LifecycleNotice::PageClose {
                context: CONTEXT,
                page: SECOND
            },
            LifecycleNotice::ContextClose { context: CONTEXT },
        ]
    );
}
