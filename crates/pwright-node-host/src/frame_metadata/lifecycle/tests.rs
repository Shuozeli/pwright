use super::*;
use crate::{
    context_metadata::{ContextMetadata, ContextMetadataLimits, ContextSnapshot},
    frame_metadata::{FrameMetadataLimits, PublicationProgress},
    frame_tree::FrameTree,
    frame_trees::FrameTrees,
};
use pwright_composition_chromium::{ChromiumFrameOwnership, ChromiumFrameUrlObserver};

mod conversion;

const CONTEXT: ObjectId = ObjectId::new(1000);
const FIRST: ObjectId = ObjectId::new(1);
const SECOND: ObjectId = ObjectId::new(3);

struct Fixture {
    book: Arc<FrameMetadata>,
    _trees: FrameTrees,
    first: FrameTree,
    second: FrameTree,
    context: ContextSnapshot,
    initial: CollectionView,
}

impl Fixture {
    fn new() -> Self {
        let book = Arc::new(
            FrameMetadata::new(FrameMetadataLimits {
                entries: 8,
                url_bytes: 64,
                total_bytes: 8192,
                publications: 8,
            })
            .unwrap(),
        );
        let trees = FrameTrees::new(book.clone()).unwrap();
        trees
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
        book.update(ObjectId::new(2), "first", None).unwrap();
        let first = trees.get(FIRST).unwrap();
        let second = trees.get(SECOND).unwrap();
        let contexts = ContextMetadata::new(ContextMetadataLimits {
            active_contexts: 16,
            states: 256,
            conversions: 128,
            base_url_bytes: 64,
            bytes: 8192,
        })
        .unwrap();
        let context = contexts
            .reserve_creation(None)
            .unwrap()
            .commit(CONTEXT)
            .unwrap()
            .snapshot;
        let publications = book.publications();
        let mut reserved = publications
            .reserve_collection(first.lookup_resources())
            .unwrap();
        reserved.add_context(CONTEXT, context.clone()).unwrap();
        reserved.add_page(FIRST, CONTEXT, first.clone()).unwrap();
        publications.stage_collection(reserved).unwrap();
        publications.fence().unwrap().publish().unwrap();
        let initial = publications.published_collections().unwrap().unwrap();
        Self {
            book,
            _trees: trees,
            first,
            second,
            context,
            initial,
        }
    }

    fn reserve(&self) -> LifecycleReservation {
        self.book
            .publications()
            .reserve_lifecycle(self.first.lookup_resources())
            .unwrap()
    }

    fn popup(&self) -> CollectionView {
        let publications = self.book.publications();
        let mut reserved = publications
            .reserve_collection(self.first.lookup_resources())
            .unwrap();
        reserved.add_context(CONTEXT, self.context.clone()).unwrap();
        reserved
            .add_page(FIRST, CONTEXT, self.first.clone())
            .unwrap();
        reserved
            .add_page(SECOND, CONTEXT, self.second.clone())
            .unwrap();
        publications
            .stage_lifecycle(
                self.reserve(),
                self.initial.clone(),
                LifecycleCause::PageReady {
                    revision: reserved.into_revision(),
                    page: SECOND,
                    opener: Some(FIRST),
                },
            )
            .unwrap()
    }

    fn stage_close(
        &self,
        before: CollectionView,
        page: ObjectId,
        tree: &FrameTree,
    ) -> CollectionView {
        let admission = self.reserve();
        self.book
            .publications()
            .stage_lifecycle(
                admission,
                before,
                LifecycleCause::PageClosed {
                    page,
                    close: tree.close().unwrap(),
                },
            )
            .unwrap()
    }

    fn owner(&self) -> FramePublicationDrain {
        self.book
            .publications()
            .fence()
            .unwrap()
            .try_begin()
            .unwrap()
    }

    fn visible(&self) -> CollectionView {
        self.book
            .publications()
            .published_collections()
            .unwrap()
            .unwrap()
    }
}

#[test]
fn conversion_is_lock_free_and_precedes_atomic_membership_publication() {
    // Arrange
    let fixture = Fixture::new();
    fixture.popup();
    let mut owner = fixture.owner();
    // Act
    let notice = owner
        .with_next_notice(|view| {
            assert!(fixture.book.state.try_lock().is_ok());
            assert_eq!(fixture.visible().pages().len(), 1);
            assert_eq!(view.revision.pages().len(), 2);
            view.notice
        })
        .unwrap();
    let committed = owner.commit_notice().unwrap();
    // Assert
    assert_eq!(
        notice,
        LifecycleNotice::ContextPage {
            context: CONTEXT,
            page: SECOND
        }
    );
    assert_eq!(committed, notice);
    assert_eq!(fixture.visible().pages().len(), 2);
    assert_eq!(fixture.initial.pages().len(), 1);
    assert!(fixture.book.state.try_lock().is_ok());
    assert!(matches!(
        owner.publish_next(),
        Err(PublicationDrainError::AwaitingAcknowledgement)
    ));
}

#[test]
fn delivered_creation_continues_to_its_dependent_popup_once() {
    // Arrange
    let fixture = Fixture::new();
    fixture.popup();
    let mut owner = fixture.owner();
    owner.commit_notice().unwrap();
    // Act
    owner
        .acknowledge_notice(LifecycleDelivery::Delivered)
        .unwrap();
    let popup = owner.commit_notice().unwrap();
    owner
        .acknowledge_notice(LifecycleDelivery::Delivered)
        .unwrap();
    // Assert
    assert_eq!(
        popup,
        LifecycleNotice::Popup {
            opener: FIRST,
            popup: SECOND
        }
    );
    assert_eq!(owner.publish_next().unwrap(), PublicationProgress::Complete);
    assert!(fixture.book.publications().is_drained().unwrap());
}

#[test]
fn listener_throw_skips_dependent_popup_but_preserves_independent_page_close() {
    // Arrange
    let fixture = Fixture::new();
    let after = fixture.popup();
    fixture.stage_close(after, FIRST, &fixture.first);
    let mut owner = fixture.owner();
    owner.commit_notice().unwrap();
    // Act
    owner
        .acknowledge_notice(LifecycleDelivery::ListenerThrew)
        .unwrap();
    let next = owner.commit_notice().unwrap();
    // Assert
    assert_eq!(
        next,
        LifecycleNotice::PageClose {
            context: CONTEXT,
            page: FIRST
        }
    );
    assert!(fixture.first.is_closed().unwrap());
    assert_eq!(fixture.visible().pages()[0].id, SECOND);
}

#[test]
fn close_groups_publish_incrementally_and_do_not_share_failure_scope() {
    // Arrange
    let fixture = Fixture::new();
    let after = fixture.popup();
    let mut creation = fixture.owner();
    creation.commit_notice().unwrap();
    creation
        .acknowledge_notice(LifecycleDelivery::ListenerThrew)
        .unwrap();
    creation.publish_next().unwrap();
    let after = fixture.stage_close(after, FIRST, &fixture.first);
    let after = fixture.stage_close(after, SECOND, &fixture.second);
    fixture
        .book
        .publications()
        .stage_lifecycle(
            fixture.reserve(),
            after,
            LifecycleCause::ContextClosed { context: CONTEXT },
        )
        .unwrap();
    let mut owner = fixture.owner();
    // Act
    let first = owner.commit_notice().unwrap();
    assert!(fixture.first.is_closed().unwrap());
    assert!(!fixture.second.is_closed().unwrap());
    assert_eq!(fixture.visible().pages().len(), 1);
    assert_eq!(fixture.visible().contexts().len(), 1);
    owner
        .acknowledge_notice(LifecycleDelivery::ListenerThrew)
        .unwrap();
    let second = owner.commit_notice().unwrap();
    owner
        .acknowledge_notice(LifecycleDelivery::Delivered)
        .unwrap();
    let context = owner.commit_notice().unwrap();
    owner
        .acknowledge_notice(LifecycleDelivery::Delivered)
        .unwrap();
    // Assert
    assert_eq!(
        first,
        LifecycleNotice::PageClose {
            context: CONTEXT,
            page: FIRST
        }
    );
    assert_eq!(
        second,
        LifecycleNotice::PageClose {
            context: CONTEXT,
            page: SECOND
        }
    );
    assert_eq!(context, LifecycleNotice::ContextClose { context: CONTEXT });
    assert!(fixture.visible().contexts().is_empty());
    assert!(fixture.second.is_closed().unwrap());
    assert_eq!(owner.publish_next().unwrap(), PublicationProgress::Complete);
}

#[test]
fn failed_conversion_and_uncommitted_guard_drop_preserve_the_notice() {
    // Arrange
    let fixture = Fixture::new();
    fixture.popup();
    let owner = fixture.owner();
    // Act
    let result = owner
        .with_next_notice(|_| Err::<(), _>("conversion capacity"))
        .unwrap();
    drop(owner);
    // Assert
    assert_eq!(result, Err("conversion capacity"));
    assert_eq!(fixture.visible().pages().len(), 1);
    let mut retry = fixture.owner();
    assert_eq!(
        retry.commit_notice().unwrap(),
        LifecycleNotice::ContextPage {
            context: CONTEXT,
            page: SECOND
        }
    );
}

#[test]
fn interrupted_committed_dispatch_drops_only_its_group_without_replay() {
    // Arrange
    let fixture = Fixture::new();
    let after = fixture.popup();
    fixture.stage_close(after, FIRST, &fixture.first);
    let mut owner = fixture.owner();
    owner.commit_notice().unwrap();
    // Act
    drop(owner);
    // Assert
    let mut retry = fixture.owner();
    assert_eq!(
        retry.commit_notice().unwrap(),
        LifecycleNotice::PageClose {
            context: CONTEXT,
            page: FIRST
        }
    );
}

#[test]
fn dropping_after_acknowledgement_preserves_the_uncommitted_popup() {
    // Arrange
    let fixture = Fixture::new();
    fixture.popup();
    let mut owner = fixture.owner();
    owner.commit_notice().unwrap();
    owner
        .acknowledge_notice(LifecycleDelivery::Delivered)
        .unwrap();
    // Act
    drop(owner);
    // Assert
    let mut retry = fixture.owner();
    assert_eq!(
        retry.commit_notice().unwrap(),
        LifecycleNotice::Popup {
            opener: FIRST,
            popup: SECOND
        }
    );
}

#[test]
fn legacy_fence_refuses_without_consuming_a_group() {
    // Arrange
    let fixture = Fixture::new();
    fixture.popup();
    // Act
    let result = fixture.book.publications().fence().unwrap().publish();
    // Assert
    assert!(matches!(result, Err(HostError::UnsupportedOperation(_))));
    assert_eq!(fixture.book.metrics().unwrap().pending_publications, 1);
    assert_eq!(fixture.visible().pages().len(), 1);
    assert!(matches!(
        fixture.owner().publish_next(),
        Err(PublicationDrainError::NoticePending)
    ));
}

#[test]
fn an_old_watermark_cannot_consume_a_later_group() {
    // Arrange
    let fixture = Fixture::new();
    let old = fixture.book.publications().fence().unwrap();
    fixture.popup();
    let mut owner = old.try_begin().unwrap();
    // Act
    let result = owner.publish_next().unwrap();
    // Assert
    assert_eq!(result, PublicationProgress::Complete);
    assert_eq!(fixture.book.metrics().unwrap().pending_publications, 1);
    assert_eq!(fixture.visible().pages().len(), 1);
}

#[test]
fn acknowledgement_before_commit_and_double_commit_are_rejected() {
    // Arrange
    let fixture = Fixture::new();
    fixture.popup();
    let mut owner = fixture.owner();
    // Act
    let premature = owner.acknowledge_notice(LifecycleDelivery::Delivered);
    owner.commit_notice().unwrap();
    let duplicate = owner.commit_notice();
    // Assert
    assert!(matches!(
        premature,
        Err(PublicationDrainError::NotCommitted)
    ));
    assert!(matches!(
        duplicate,
        Err(PublicationDrainError::AwaitingAcknowledgement)
    ));
}

#[test]
fn stale_predecessor_fails_closed_instead_of_regressing_public_state() {
    // Arrange
    let fixture = Fixture::new();
    fixture.popup();
    fixture.stage_close(fixture.initial.clone(), FIRST, &fixture.first);
    let mut owner = fixture.owner();
    owner.commit_notice().unwrap();
    owner
        .acknowledge_notice(LifecycleDelivery::ListenerThrew)
        .unwrap();
    // Act
    let result = owner.commit_notice();
    // Assert
    assert!(matches!(
        result,
        Err(PublicationDrainError::Backend(HostError::Backend(_)))
    ));
    assert!(matches!(
        fixture.book.publications().fence().unwrap().try_begin(),
        Err(PublicationDrainError::Discarded)
    ));
    assert!(!fixture.first.is_closed().unwrap());
}

#[test]
fn foreign_queue_reservation_is_rejected_and_releases_its_storage() {
    // Arrange
    let fixture = Fixture::new();
    let foreign = Fixture::new();
    let metrics = foreign.first.lookup_resources().metrics().unwrap();
    let reserved = foreign.reserve();
    // Act
    let result = fixture.book.publications().stage_lifecycle(
        reserved,
        fixture.initial.clone(),
        LifecycleCause::ContextClosed { context: CONTEXT },
    );
    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert_eq!(foreign.first.lookup_resources().metrics().unwrap(), metrics);
    assert_eq!(fixture.book.metrics().unwrap().pending_publications, 0);
}

#[test]
fn wrong_tree_close_token_cannot_publish_another_pages_closed_flag() {
    // Arrange
    let fixture = Fixture::new();
    let reserved = fixture.reserve();
    let close = fixture.second.close().unwrap();
    // Act
    let result = fixture.book.publications().stage_lifecycle(
        reserved,
        fixture.initial.clone(),
        LifecycleCause::PageClosed { page: FIRST, close },
    );
    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert!(!fixture.first.is_closed().unwrap());
    assert!(!fixture.second.is_closed().unwrap());
    assert_eq!(fixture.book.metrics().unwrap().pending_publications, 0);
}

#[test]
fn discard_releases_group_bodies_even_while_the_guard_is_retained() {
    // Arrange
    let fixture = Fixture::new();
    let baseline = fixture.first.lookup_resources().metrics().unwrap();
    fixture.popup();
    let mut owner = fixture.owner();
    // Act
    fixture.book.publications().discard();
    // Assert
    assert_eq!(
        fixture.first.lookup_resources().metrics().unwrap(),
        baseline
    );
    assert!(matches!(
        owner.commit_notice(),
        Err(PublicationDrainError::Discarded)
    ));
    assert!(fixture.book.publications().is_drained().unwrap());
}

#[test]
fn saturated_queue_rejects_admission_without_allocating_group_storage() {
    // Arrange
    let fixture = Fixture::new();
    let metrics = fixture.first.lookup_resources().metrics().unwrap();
    let mut held = Vec::new();
    while let Ok(reserved) = fixture
        .book
        .publications()
        .reserve_lifecycle(fixture.first.lookup_resources())
    {
        held.push(reserved);
    }
    let full = fixture.first.lookup_resources().metrics().unwrap();
    // Act
    let result = fixture
        .book
        .publications()
        .reserve_lifecycle(fixture.first.lookup_resources());
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(fixture.first.lookup_resources().metrics().unwrap(), full);
    assert!(!held.is_empty());
    assert_eq!(
        full.bytes - metrics.bytes,
        held.len() as u64 * STORAGE_BYTES as u64
    );
    drop(held);
    assert_eq!(fixture.first.lookup_resources().metrics().unwrap(), metrics);
}

#[test]
fn a_deferred_link_publishes_popup_without_replaying_context_page() {
    // Arrange
    let fixture = Fixture::new();
    let after = fixture.popup();
    let mut creation = fixture.owner();
    creation.commit_notice().unwrap();
    creation
        .acknowledge_notice(LifecycleDelivery::ListenerThrew)
        .unwrap();
    creation.publish_next().unwrap();
    fixture
        .book
        .publications()
        .stage_lifecycle(
            fixture.reserve(),
            after,
            LifecycleCause::PopupLinked {
                opener: FIRST,
                popup: SECOND,
            },
        )
        .unwrap();
    let mut owner = fixture.owner();
    // Act
    let notice = owner.commit_notice().unwrap();
    owner
        .acknowledge_notice(LifecycleDelivery::Delivered)
        .unwrap();
    // Assert
    assert_eq!(
        notice,
        LifecycleNotice::Popup {
            opener: FIRST,
            popup: SECOND
        }
    );
    assert_eq!(owner.publish_next().unwrap(), PublicationProgress::Complete);
    assert_eq!(fixture.visible().pages().len(), 2);
}

#[test]
fn context_cannot_close_while_its_page_is_still_visible() {
    // Arrange
    let fixture = Fixture::new();
    let baseline = fixture.first.lookup_resources().metrics().unwrap();
    // Act
    let result = fixture.book.publications().stage_lifecycle(
        fixture.reserve(),
        fixture.initial.clone(),
        LifecycleCause::ContextClosed { context: CONTEXT },
    );
    // Assert
    assert!(result.is_err());
    assert_eq!(fixture.visible().contexts().len(), 1);
    assert_eq!(fixture.book.metrics().unwrap().pending_publications, 0);
    assert_eq!(
        fixture.first.lookup_resources().metrics().unwrap(),
        baseline
    );
}

#[test]
fn same_ids_from_a_foreign_view_do_not_authorize_a_transition() {
    // Arrange
    let fixture = Fixture::new();
    let foreign = Fixture::new();
    // Act
    let result = fixture.book.publications().stage_lifecycle(
        fixture.reserve(),
        foreign.initial,
        LifecycleCause::PageClosed {
            page: FIRST,
            close: foreign.first.close().unwrap(),
        },
    );
    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert_eq!(fixture.book.metrics().unwrap().pending_publications, 0);
    assert!(!fixture.first.is_closed().unwrap());
}

#[test]
fn discarding_during_conversion_prevents_later_commit() {
    // Arrange
    let fixture = Fixture::new();
    let baseline = fixture.first.lookup_resources().metrics().unwrap();
    fixture.popup();
    let admitted = fixture.first.lookup_resources().metrics().unwrap();
    let mut owner = fixture.owner();
    // Act
    owner
        .with_next_notice(|view| {
            fixture.book.publications().discard();
            assert_eq!(view.revision.pages().len(), 2);
            assert_eq!(
                fixture.first.lookup_resources().metrics().unwrap(),
                admitted
            );
        })
        .unwrap();
    let result = owner.commit_notice();
    // Assert
    assert!(matches!(result, Err(PublicationDrainError::Discarded)));
    assert!(fixture.book.publications().is_drained().unwrap());
    assert_eq!(
        fixture.first.lookup_resources().metrics().unwrap(),
        baseline
    );
}

#[test]
fn metadata_and_lifecycle_groups_share_one_ordered_watermark() {
    // Arrange
    let fixture = Fixture::new();
    fixture
        .book
        .update(ObjectId::new(2), "before-notice", None)
        .unwrap();
    fixture.popup();
    fixture
        .book
        .update(ObjectId::new(2), "after-notice", None)
        .unwrap();
    let frame = fixture.book.get(ObjectId::new(2)).unwrap();
    let mut owner = fixture.owner();
    // Act
    let first = owner.publish_next().unwrap();
    owner.commit_notice().unwrap();
    assert_eq!(frame.published_url().unwrap(), "before-notice");
    owner
        .acknowledge_notice(LifecycleDelivery::ListenerThrew)
        .unwrap();
    let last = owner.publish_next().unwrap();
    // Assert
    assert_eq!(first, PublicationProgress::Published);
    assert_eq!(last, PublicationProgress::Published);
    assert_eq!(frame.published_url().unwrap(), "after-notice");
    assert_eq!(owner.publish_next().unwrap(), PublicationProgress::Complete);
}

#[test]
fn pending_acknowledgement_keeps_the_slot_and_body_charged() {
    // Arrange
    let fixture = Fixture::new();
    fixture.stage_close(fixture.initial.clone(), FIRST, &fixture.first);
    let admitted = fixture.first.lookup_resources().metrics().unwrap();
    let mut owner = fixture.owner();
    // Act
    owner.commit_notice().unwrap();
    // Assert
    assert!(!fixture.book.publications().is_drained().unwrap());
    assert_eq!(fixture.book.metrics().unwrap().pending_publications, 1);
    assert_eq!(
        fixture.first.lookup_resources().metrics().unwrap(),
        admitted
    );
    owner
        .acknowledge_notice(LifecycleDelivery::Delivered)
        .unwrap();
    assert_eq!(
        fixture.first.lookup_resources().metrics().unwrap().bytes,
        admitted.bytes - STORAGE_BYTES as u64
    );
}

#[test]
fn reservation_cannot_be_staged_after_terminal_teardown() {
    // Arrange
    let fixture = Fixture::new();
    let reserved = fixture.reserve();
    fixture
        .book
        .publications()
        .close_for_teardown(std::iter::empty());
    // Act
    let result = fixture.book.publications().stage_lifecycle(
        reserved,
        fixture.initial.clone(),
        LifecycleCause::PageClosed {
            page: FIRST,
            close: fixture.first.close().unwrap(),
        },
    );
    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert!(!fixture.first.is_closed().unwrap());
}

#[test]
fn group_layout_is_explicit_and_does_not_expand_every_queue_slot() {
    // Arrange
    let fixture = Fixture::new();
    // Act
    let queue_bytes = fixture.book.publication_storage_bytes().unwrap();
    // Assert
    assert_eq!(
        std::mem::size_of::<FrameChangeValue>(),
        3 * std::mem::size_of::<usize>()
    );
    assert_eq!(
        std::mem::size_of::<FramePublicationDrain>(),
        3 * std::mem::size_of::<usize>()
    );
    eprintln!("lifecycle group admission bytes={STORAGE_BYTES}, queue(8) bytes={queue_bytes}");
}
