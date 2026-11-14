use super::*;
use crate::{
    context_metadata::{ContextMetadata, ContextMetadataLimits},
    frame_metadata::FrameMetadataLimits,
    frame_tree::FrameTreeLimits,
    frame_trees::FrameTrees,
};
use pwright_plugin_chromium_frame_observer::{ChromiumFrameOwnership, ChromiumFrameUrlObserver};

mod selected;

pub(super) fn fixture(
    publications: usize,
) -> (Arc<FrameMetadata>, FrameTrees, FrameTree, ContextSnapshot) {
    let book = Arc::new(
        FrameMetadata::new(FrameMetadataLimits {
            entries: 128,
            url_bytes: 64,
            total_bytes: 8192,
            publications,
        })
        .unwrap(),
    );
    let trees = FrameTrees::new(book.clone()).unwrap();
    trees
        .reconcile(
            std::iter::once(ChromiumFrameOwnership {
                frame: ObjectId::new(2),
                page: ObjectId::new(1),
                parent: None,
            }),
            |_| true,
        )
        .unwrap();
    book.update(ObjectId::new(2), "kept", None).unwrap();
    book.publications().fence().unwrap().publish().unwrap();
    let tree = trees.get(ObjectId::new(1)).unwrap();
    let contexts = ContextMetadata::new(ContextMetadataLimits {
        active_contexts: 16,
        states: 256,
        conversions: 128,
        base_url_bytes: 64,
        bytes: 8192,
    })
    .unwrap();
    let created = contexts
        .reserve_creation(Some("https://example.test/"))
        .unwrap()
        .commit(ObjectId::new(1000))
        .unwrap();
    (book, trees, tree, created.snapshot)
}

pub(super) fn populated(
    book: &Arc<FrameMetadata>,
    tree: &FrameTree,
    context: ContextSnapshot,
) -> CollectionReservation {
    let mut reservation = book
        .publications()
        .reserve_collection(tree.lookup_resources())
        .unwrap();
    reservation
        .add_context(ObjectId::new(1000), context)
        .unwrap();
    reservation
        .add_page(ObjectId::new(1), ObjectId::new(1000), tree.clone())
        .unwrap();
    reservation
}

#[test]
fn collection_is_invisible_until_its_fixed_fence_and_old_fences_cannot_regress() {
    // Arrange
    let (book, _trees, tree, context) = fixture(8);
    let old = book.publications().fence().unwrap();
    book.publications()
        .stage_collection(populated(&book, &tree, context))
        .unwrap();
    let first = book.publications().fence().unwrap();
    let empty = book
        .publications()
        .reserve_collection(tree.lookup_resources())
        .unwrap();
    book.publications().stage_collection(empty).unwrap();
    assert!(
        book.publications()
            .published_collections()
            .unwrap()
            .is_none()
    );
    // Act
    first.publish().unwrap();
    old.publish().unwrap();
    // Assert
    let visible = book
        .publications()
        .published_collections()
        .unwrap()
        .unwrap();
    assert_eq!(visible.contexts().len(), 1);
    assert_eq!(visible.pages().len(), 1);
    assert_eq!(visible.pages()[0].id, ObjectId::new(1));
    assert_eq!(book.metrics().unwrap().pending_publications, 1);
}

#[test]
fn old_revision_keeps_its_bytes_charged_after_replacement_until_last_reader_drops() {
    // Arrange
    let (book, _trees, tree, context) = fixture(8);
    let factory = tree.lookup_resources();
    let before = factory.metrics().unwrap();
    book.publications()
        .stage_collection(populated(&book, &tree, context))
        .unwrap();
    book.publications().fence().unwrap().publish().unwrap();
    let retained = book
        .publications()
        .published_collections()
        .unwrap()
        .unwrap();
    let empty = book.publications().reserve_collection(factory).unwrap();
    book.publications().stage_collection(empty).unwrap();
    // Act
    book.publications().fence().unwrap().publish().unwrap();
    let overlapping = factory.metrics().unwrap();
    drop(retained);
    // Assert
    assert_eq!(overlapping.bytes, before.bytes + 2 * STORAGE_BYTES as u64);
    assert_eq!(overlapping.revisions, before.revisions + 2);
    assert_eq!(
        factory.metrics().unwrap().bytes,
        before.bytes + STORAGE_BYTES as u64
    );
    assert_eq!(factory.metrics().unwrap().revisions, before.revisions + 1);
    assert!(
        book.publications()
            .published_collections()
            .unwrap()
            .unwrap()
            .pages()
            .is_empty()
    );
}

#[test]
fn reservation_admits_the_queue_slot_before_native_activation() {
    // Arrange
    let (book, _trees, tree, context) = fixture(2);
    let reservation = populated(&book, &tree, context);
    let capacity = book.state.lock().unwrap().pending.capacity();
    let overflow = book.update(ObjectId::new(2), "later", None);
    assert!(overflow.is_err());
    // Act
    book.publications().stage_collection(reservation).unwrap();
    // Assert
    assert_eq!(book.metrics().unwrap().pending_publications, 1);
    assert_eq!(book.state.lock().unwrap().pending.capacity(), capacity);
    assert_eq!(book.get(ObjectId::new(2)).unwrap().url().unwrap(), "kept");
}

#[test]
fn failed_byte_admission_returns_the_reserved_queue_slot() {
    // Arrange
    let (book, _trees, _tree, _context) = fixture(2);
    let tiny = FrameTreeFactory::new(FrameTreeLimits {
        states: 1,
        frames_per_page: 1,
        revisions: 1,
        bytes: 1,
    })
    .unwrap();
    // Act
    let result = book.publications().reserve_collection(&tiny);
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(tiny.metrics().unwrap().revisions, 0);
    assert_eq!(tiny.metrics().unwrap().bytes, 0);
    book.update(ObjectId::new(2), "later", None).unwrap();
}

#[test]
fn abandoning_a_reservation_releases_vectors_bytes_and_queue_admission() {
    // Arrange
    let (book, _trees, tree, context) = fixture(2);
    let before = tree.lookup_resources().metrics().unwrap();
    let reservation = populated(&book, &tree, context);
    // Act
    drop(reservation);
    // Assert
    assert_eq!(tree.lookup_resources().metrics().unwrap(), before);
    assert_eq!(book.metrics().unwrap().pending_publications, 0);
    book.update(ObjectId::new(2), "later", None).unwrap();
}

#[test]
fn foreign_queue_cannot_consume_a_collection_reservation() {
    // Arrange
    let (book, _trees, tree, context) = fixture(2);
    let (other, _other_trees, _other_tree, _other_context) = fixture(2);
    let before = tree.lookup_resources().metrics().unwrap();
    let reservation = populated(&book, &tree, context);
    // Act
    let result = other.publications().stage_collection(reservation);
    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert_eq!(tree.lookup_resources().metrics().unwrap(), before);
    assert_eq!(other.metrics().unwrap().pending_publications, 0);
    book.update(ObjectId::new(2), "later", None).unwrap();
}

#[test]
fn native_teardown_keeps_uncached_metadata_until_terminal_publication() {
    // Arrange
    let (book, trees, tree, context) = fixture(3);
    book.publications()
        .stage_collection(populated(&book, &tree, context))
        .unwrap();
    book.publications().fence().unwrap().publish().unwrap();
    // Act
    trees.clear();
    book.clear();
    // Assert
    let visible = book
        .publications()
        .published_collections()
        .unwrap()
        .unwrap();
    assert_eq!(
        visible.contexts()[0].settings.base_url(),
        Some("https://example.test/")
    );
    assert_eq!(visible.pages()[0].main_frame.url().unwrap(), "kept");
    assert!(!visible.pages()[0].tree.is_closed().unwrap());
    assert_eq!(book.metrics().unwrap().active_frames, 0);
    assert!(trees.get(ObjectId::new(1)).is_err());
}

#[test]
fn terminal_fence_clears_collections_with_full_ordinary_admission_and_cannot_reopen() {
    // Arrange
    let (book, trees, tree, context) = fixture(3);
    book.publications()
        .stage_collection(populated(&book, &tree, context))
        .unwrap();
    book.publications().fence().unwrap().publish().unwrap();
    book.update(ObjectId::new(2), "first", None).unwrap();
    book.update(ObjectId::new(2), "second", None).unwrap();
    let old = book.publications().fence().unwrap();
    assert!(book.update(ObjectId::new(2), "overflow", None).is_err());
    let capacity = book.state.lock().unwrap().pending.capacity();
    trees.clear();
    // Act
    book.publications().fence().unwrap().publish().unwrap();
    old.publish().unwrap();
    // Assert
    assert!(
        book.publications()
            .published_collections()
            .unwrap()
            .is_none()
    );
    assert!(tree.is_closed().unwrap());
    assert_eq!(book.metrics().unwrap().pending_publications, 0);
    assert_eq!(book.state.lock().unwrap().pending.capacity(), capacity);
    assert!(book.publications().publish_disconnect().unwrap());
}

#[test]
fn discard_drops_pending_and_current_revisions_but_not_caller_owned_metadata() {
    // Arrange
    let (book, _trees, tree, context) = fixture(4);
    let before = tree.lookup_resources().metrics().unwrap();
    book.publications()
        .stage_collection(populated(&book, &tree, context.clone()))
        .unwrap();
    book.publications().fence().unwrap().publish().unwrap();
    let retained = book
        .publications()
        .published_collections()
        .unwrap()
        .unwrap();
    book.publications()
        .stage_collection(populated(&book, &tree, context))
        .unwrap();
    let late = book.publications().fence().unwrap();
    // Act
    book.publications().discard();
    late.publish().unwrap();
    // Assert
    assert!(
        book.publications()
            .published_collections()
            .unwrap()
            .is_none()
    );
    assert_eq!(book.metrics().unwrap().pending_publications, 0);
    assert_eq!(
        tree.lookup_resources().metrics().unwrap().bytes,
        before.bytes + STORAGE_BYTES as u64
    );
    assert!(!retained.pages()[0].tree.is_closed().unwrap());
    drop(retained);
    assert_eq!(tree.lookup_resources().metrics().unwrap(), before);
}

#[test]
fn retained_revision_has_no_strong_path_back_to_publication_book() {
    // Arrange
    let (book, trees, tree, context) = fixture(3);
    book.publications()
        .stage_collection(populated(&book, &tree, context))
        .unwrap();
    book.publications().fence().unwrap().publish().unwrap();
    let retained = book
        .publications()
        .published_collections()
        .unwrap()
        .unwrap();
    let weak = Arc::downgrade(&book);
    // Act
    drop(trees);
    drop(tree);
    drop(book);
    // Assert
    assert!(weak.upgrade().is_none());
    assert_eq!(retained.pages()[0].main_frame.url().unwrap(), "kept");
}

#[test]
fn invalid_identity_and_missing_parent_do_not_mutate_an_admitted_projection() {
    // Arrange
    let (book, _trees, tree, context) = fixture(3);
    let mut reserved = populated(&book, &tree, context.clone());
    // Act
    let duplicate = reserved.add_context(ObjectId::new(1000), context);
    let missing = reserved.add_page(ObjectId::new(3), ObjectId::new(999), tree.clone());
    let reused_root = reserved.add_page(ObjectId::new(3), ObjectId::new(1000), tree);
    // Assert
    assert!(matches!(duplicate, Err(HostError::InvalidArgument(_))));
    assert!(matches!(missing, Err(HostError::InvalidArgument(_))));
    assert!(matches!(reused_root, Err(HostError::InvalidArgument(_))));
    assert_eq!(reserved.revision.contexts().len(), 1);
    assert_eq!(reserved.revision.pages().len(), 1);
}

#[test]
fn context_capacity_is_rejected_without_growing_the_admitted_vector() {
    // Arrange
    let (book, _trees, tree, context) = fixture(3);
    let mut reserved = book
        .publications()
        .reserve_collection(tree.lookup_resources())
        .unwrap();
    for id in 1000..1016 {
        reserved
            .add_context(ObjectId::new(id), context.clone())
            .unwrap();
    }
    let before = tree.lookup_resources().metrics().unwrap();
    // Act
    let result = reserved.add_context(ObjectId::new(1016), context);
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(reserved.revision.contexts().len(), CONTEXTS);
    assert_eq!(reserved.revision.0.contexts.capacity(), CONTEXTS);
    assert_eq!(tree.lookup_resources().metrics().unwrap(), before);
}

#[test]
fn a_reservation_cannot_be_staged_after_terminal_shutdown() {
    // Arrange
    let (book, trees, tree, context) = fixture(3);
    let reserved = populated(&book, &tree, context);
    trees.clear();
    // Act
    let result = book.publications().stage_collection(reserved);
    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert!(
        book.publications()
            .published_collections()
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        book.publications()
            .reserve_collection(tree.lookup_resources()),
        Err(HostError::TargetClosed)
    ));
}

#[test]
fn page_capacity_is_rejected_without_growing_the_admitted_vector() {
    // Arrange
    let (book, _trees, tree, context) = fixture(3);
    let factory = tree.lookup_resources();
    let mut reserved = populated(&book, &tree, context);
    for id in 10..25 {
        let root = factory
            .frame(
                ObjectId::new(id + 100),
                None,
                book.get(ObjectId::new(2)).unwrap(),
            )
            .unwrap();
        let page = factory.page(root).unwrap();
        reserved
            .add_page(ObjectId::new(id), ObjectId::new(1000), page)
            .unwrap();
    }
    let root = factory
        .frame(
            ObjectId::new(125),
            None,
            book.get(ObjectId::new(2)).unwrap(),
        )
        .unwrap();
    let overflow = factory.page(root).unwrap();
    let before = factory.metrics().unwrap();
    // Act
    let result = reserved.add_page(ObjectId::new(25), ObjectId::new(1000), overflow.clone());
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(reserved.revision.pages().len(), PAGES);
    assert_eq!(reserved.revision.0.pages.capacity(), PAGES);
    assert_eq!(factory.metrics().unwrap(), before);
}

#[test]
fn revision_count_admission_is_shared_with_frame_metadata_work() {
    // Arrange
    let (book, _trees, _tree, _context) = fixture(4);
    let factory = FrameTreeFactory::new(FrameTreeLimits {
        states: 1,
        frames_per_page: 1,
        revisions: 1,
        bytes: 65536,
    })
    .unwrap();
    let held = factory.reserve_conversion(1).unwrap();
    // Act
    let result = book.publications().reserve_collection(&factory);
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(factory.metrics().unwrap().revisions, 1);
    drop(held);
    assert!(book.publications().reserve_collection(&factory).is_ok());
}

#[test]
fn collection_requires_explicit_preallocated_queue_selection() {
    // Arrange
    let book = Arc::new(
        FrameMetadata::new(FrameMetadataLimits {
            entries: 8,
            url_bytes: 64,
            total_bytes: 1024,
            publications: 4,
        })
        .unwrap(),
    );
    let factory = FrameTreeFactory::new(FrameTreeLimits {
        states: 1,
        frames_per_page: 1,
        revisions: 1,
        bytes: 65536,
    })
    .unwrap();
    // Act
    let result = book.publications().reserve_collection(&factory);
    // Assert
    assert!(matches!(result, Err(HostError::UnsupportedOperation(_))));
    assert_eq!(factory.metrics().unwrap().revisions, 0);
    assert_eq!(factory.metrics().unwrap().bytes, 0);
}

#[test]
fn discarded_reservation_cannot_publish_or_hold_queue_capacity_forever() {
    // Arrange
    let (book, _trees, tree, context) = fixture(2);
    let before = tree.lookup_resources().metrics().unwrap();
    let reserved = populated(&book, &tree, context);
    book.publications().discard();
    // Act
    let result = book.publications().stage_collection(reserved);
    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(tree.lookup_resources().metrics().unwrap(), before);
    assert_eq!(book.metrics().unwrap().pending_publications, 0);
}

#[test]
fn selected_storage_layout_has_exact_logical_count_and_byte_deltas() {
    // Arrange
    let (book, trees, tree, context) = fixture(256);
    let factory = tree.lookup_resources().clone();
    let before = factory.metrics().unwrap();
    // Act
    let reserved = populated(&book, &tree, context);
    let after = factory.metrics().unwrap();
    // Assert
    assert_eq!(after.states, before.states);
    assert_eq!(after.revisions, before.revisions + 1);
    assert_eq!(after.bytes, before.bytes + STORAGE_BYTES as u64);
    assert_eq!(reserved.revision.0.contexts.capacity(), CONTEXTS);
    assert_eq!(reserved.revision.0.pages.capacity(), PAGES);
    drop(reserved);
    trees.clear();
    book.clear();
    book.publications().fence().unwrap().publish().unwrap();
    drop(tree);
    let fixed = factory.metrics().unwrap();
    assert_eq!(fixed.states, 1);
    assert_eq!(fixed.revisions, 0);
    println!(
        "Collection storage: revision_bytes={STORAGE_BYTES}, revision_items=1, fixed_bytes={}, fixed_states={}",
        fixed.bytes, fixed.states
    );
}

#[test]
fn collection_batch_holds_page_close_until_membership_is_committed() {
    // Arrange
    let (book, trees, tree, context) = fixture(4);
    book.publications()
        .stage_collection(populated(&book, &tree, context))
        .unwrap();
    book.publications().fence().unwrap().publish().unwrap();
    let reserved = book
        .publications()
        .reserve_collection(tree.lookup_resources())
        .unwrap();
    let batch = book.publications().begin_collection(reserved).unwrap();
    trees.reconcile(std::iter::empty(), |_| false).unwrap();
    // Act
    book.publications().fence().unwrap().publish().unwrap();
    // Assert
    assert!(!tree.is_closed().unwrap());
    assert_eq!(
        book.publications()
            .published_collections()
            .unwrap()
            .unwrap()
            .pages()
            .len(),
        1
    );
    batch.commit().unwrap();
    book.publications().fence().unwrap().publish().unwrap();
    assert!(tree.is_closed().unwrap());
    assert!(
        book.publications()
            .published_collections()
            .unwrap()
            .unwrap()
            .pages()
            .is_empty()
    );
}

#[test]
fn incomplete_batch_does_not_spin_the_publication_watch() {
    use std::{
        future::Future,
        task::{Context, Poll, Waker},
    };
    // Arrange
    let (book, trees, tree, _context) = fixture(4);
    let reserved = book
        .publications()
        .reserve_collection(tree.lookup_resources())
        .unwrap();
    let batch = book.publications().begin_collection(reserved).unwrap();
    trees.reconcile(std::iter::empty(), |_| false).unwrap();
    let publications = book.publications();
    let mut pending = Box::pin(publications.next());
    let mut cx = Context::from_waker(Waker::noop());
    // Act
    let outcome = pending.as_mut().poll(&mut cx);
    // Assert
    assert!(matches!(outcome, Poll::Pending));
    batch.commit().unwrap();
    assert!(matches!(pending.as_mut().poll(&mut cx), Poll::Ready(Ok(_))));
}

#[test]
fn abandoned_batch_is_published_only_by_terminal_teardown() {
    // Arrange
    let (book, trees, tree, context) = fixture(4);
    book.publications()
        .stage_collection(populated(&book, &tree, context))
        .unwrap();
    book.publications().fence().unwrap().publish().unwrap();
    let reserved = book
        .publications()
        .reserve_collection(tree.lookup_resources())
        .unwrap();
    let batch = book.publications().begin_collection(reserved).unwrap();
    trees.reconcile(std::iter::empty(), |_| false).unwrap();
    drop(batch);
    book.publications().fence().unwrap().publish().unwrap();
    assert!(!tree.is_closed().unwrap());
    // Act
    trees.clear();
    book.publications().fence().unwrap().publish().unwrap();
    // Assert
    assert!(tree.is_closed().unwrap());
    assert!(
        book.publications()
            .published_collections()
            .unwrap()
            .is_none()
    );
}

#[test]
fn discard_prevents_a_previously_started_batch_from_committing() {
    // Arrange
    let (book, _trees, tree, _context) = fixture(4);
    let before = tree.lookup_resources().metrics().unwrap();
    let reserved = book
        .publications()
        .reserve_collection(tree.lookup_resources())
        .unwrap();
    let batch = book.publications().begin_collection(reserved).unwrap();
    book.publications().discard();
    // Act
    let result = batch.commit();
    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(tree.lookup_resources().metrics().unwrap(), before);
    assert!(
        book.publications()
            .published_collections()
            .unwrap()
            .is_none()
    );
}
