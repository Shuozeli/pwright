use super::*;
use crate::frame_metadata::{FrameMetadata, FrameMetadataLimits};
use crate::frame_trees::FrameTrees;
use pwright_core::ObjectId;
use pwright_plugin_chromium_frame_observer::{ChromiumFrameOwnership, ChromiumFrameUrlObserver};
use std::sync::Arc;

fn book() -> Arc<FrameMetadata> {
    Arc::new(
        FrameMetadata::new(FrameMetadataLimits {
            entries: 8,
            url_bytes: 64,
            total_bytes: 1024,
            publications: 8,
        })
        .unwrap(),
    )
}

fn fixture() -> (Arc<FrameMetadata>, FrameTrees, crate::FrameUrlSnapshot) {
    let book = book();
    let trees = FrameTrees::new(book.clone()).unwrap();
    book.update(ObjectId::new(1), "initial", None).unwrap();
    let frame = book.get(ObjectId::new(1)).unwrap();
    (book, trees, frame)
}

#[test]
fn unadmitted_queue_cannot_acquire_owner() {
    // Arrange
    let book = book();
    // Act
    let result = book.publications().fence().unwrap().try_begin();
    // Assert
    assert!(matches!(result, Err(PublicationDrainError::NotSelected)));
}

#[test]
fn exclusive_claim_rejects_another_owner_without_publishing() {
    // Arrange
    let (book, _trees, frame) = fixture();
    book.update(ObjectId::new(1), "next", None).unwrap();
    let fence = book.publications().fence().unwrap();
    let _owner = fence.try_begin().unwrap();
    // Act
    let result = fence.try_begin();
    // Assert
    assert!(matches!(result, Err(PublicationDrainError::Busy)));
    assert_eq!(frame.published_url().unwrap(), "initial");
}

#[test]
fn legacy_whole_fence_cannot_bypass_owner() {
    // Arrange
    let (book, _trees, frame) = fixture();
    book.update(ObjectId::new(1), "next", None).unwrap();
    let fence = book.publications().fence().unwrap();
    let _owner = fence.try_begin().unwrap();
    // Act
    let result = fence.publish();
    // Assert
    assert!(matches!(result, Err(HostError::UnsupportedOperation(_))));
    assert_eq!(frame.published_url().unwrap(), "initial");
}

#[test]
fn one_advance_leaves_locks_free_and_later_changes_pending() {
    // Arrange
    let (book, _trees, frame) = fixture();
    book.update(ObjectId::new(1), "first", None).unwrap();
    book.update(ObjectId::new(1), "second", None).unwrap();
    let mut owner = book.publications().fence().unwrap().try_begin().unwrap();
    // Act
    let result = owner.publish_next().unwrap();
    // Assert
    assert_eq!(result, PublicationProgress::Published);
    assert!(book.state.try_lock().is_ok());
    assert!(frame.0.url.try_lock().is_ok());
    assert_eq!(frame.published_url().unwrap(), "first");
    assert_eq!(book.metrics().unwrap().pending_publications, 1);
}

#[test]
fn captured_watermark_does_not_consume_later_changes() {
    // Arrange
    let (book, _trees, frame) = fixture();
    book.update(ObjectId::new(1), "first", None).unwrap();
    let fence = book.publications().fence().unwrap();
    book.update(ObjectId::new(1), "second", None).unwrap();
    let mut owner = fence.try_begin().unwrap();
    owner.publish_next().unwrap();
    // Act
    let result = owner.publish_next().unwrap();
    // Assert
    assert_eq!(result, PublicationProgress::Complete);
    assert_eq!(frame.published_url().unwrap(), "first");
    assert_eq!(book.metrics().unwrap().pending_publications, 1);
    assert!(owner.book.is_none());
}

#[test]
fn drop_after_partial_drain_neither_consumes_nor_discards_remaining_work() {
    // Arrange
    let (book, _trees, frame) = fixture();
    book.update(ObjectId::new(1), "first", None).unwrap();
    book.update(ObjectId::new(1), "second", None).unwrap();
    let fence = book.publications().fence().unwrap();
    let mut owner = fence.try_begin().unwrap();
    owner.publish_next().unwrap();
    // Act
    drop(owner);
    // Assert
    assert_eq!(frame.published_url().unwrap(), "first");
    assert_eq!(book.metrics().unwrap().pending_publications, 1);
    let mut resumed = fence.try_begin().unwrap();
    assert_eq!(
        resumed.publish_next().unwrap(),
        PublicationProgress::Published
    );
    assert_eq!(frame.published_url().unwrap(), "second");
    assert_eq!(
        resumed.publish_next().unwrap(),
        PublicationProgress::Complete
    );
}

#[test]
fn old_fence_after_new_publication_cannot_replay_or_regress() {
    // Arrange
    let (book, _trees, frame) = fixture();
    book.update(ObjectId::new(1), "first", None).unwrap();
    let old = book.publications().fence().unwrap();
    book.update(ObjectId::new(1), "second", None).unwrap();
    book.publications().fence().unwrap().publish().unwrap();
    let mut owner = old.try_begin().unwrap();
    // Act
    let result = owner.publish_next().unwrap();
    // Assert
    assert_eq!(result, PublicationProgress::Complete);
    assert_eq!(frame.published_url().unwrap(), "second");
}

#[test]
fn finished_owner_cannot_advance_work_belonging_to_new_owner() {
    // Arrange
    let (book, _trees, frame) = fixture();
    let mut old = book.publications().fence().unwrap().try_begin().unwrap();
    old.publish_next().unwrap();
    book.update(ObjectId::new(1), "next", None).unwrap();
    let _new = book.publications().fence().unwrap().try_begin().unwrap();
    // Act
    let result = old.publish_next();
    // Assert
    assert!(matches!(result, Err(PublicationDrainError::Finished)));
    assert_eq!(frame.published_url().unwrap(), "initial");
}

#[test]
fn discard_revokes_owner_without_foreign_publication() {
    // Arrange
    let (book, _trees, frame) = fixture();
    book.update(ObjectId::new(1), "next", None).unwrap();
    let fence = book.publications().fence().unwrap();
    let mut owner = fence.try_begin().unwrap();
    book.publications().discard();
    // Act
    let result = owner.publish_next();
    // Assert
    assert!(matches!(result, Err(PublicationDrainError::Discarded)));
    assert!(matches!(
        fence.try_begin(),
        Err(PublicationDrainError::Discarded)
    ));
    assert_eq!(frame.published_url().unwrap(), "initial");
    assert!(owner.book.is_none());
    assert!(book.publications().is_drained().unwrap());
    fence.publish().unwrap(); // Preserve legacy discard completion behavior.
}

#[test]
fn owner_sequence_exhaustion_never_wraps_or_claims() {
    // Arrange
    let (book, _trees, _frame) = fixture();
    book.state.lock().unwrap().drain.issued = u64::MAX;
    // Act
    let result = book.publications().fence().unwrap().try_begin();
    // Assert
    assert!(matches!(
        result,
        Err(PublicationDrainError::OwnerSequenceExhausted)
    ));
    assert!(book.state.lock().unwrap().drain.active.is_none());
}

#[test]
fn terminal_readiness_requires_explicit_completion() {
    // Arrange
    let (book, trees, frame) = fixture();
    book.update(ObjectId::new(1), "last", None).unwrap();
    trees.clear();
    let mut owner = book.publications().fence().unwrap().try_begin().unwrap();
    owner.publish_next().unwrap();
    assert_eq!(frame.published_url().unwrap(), "last");
    assert!(book.publications().publish_disconnect().is_err());
    // Act
    let result = owner.publish_next().unwrap();
    // Assert
    assert_eq!(result, PublicationProgress::Complete);
    assert!(book.publications().connection_is_published_open().unwrap());
    assert!(book.publications().publish_disconnect().unwrap());
    assert!(!book.publications().publish_disconnect().unwrap());
}

#[test]
fn active_guard_keeps_metadata_and_storage_alive_but_completion_releases_them() {
    // Arrange
    let (book, trees, frame) = fixture();
    let weak = Arc::downgrade(&book);
    let storage = Arc::downgrade(book.state.lock().unwrap().queue_storage.as_ref().unwrap());
    let mut owner = book.publications().fence().unwrap().try_begin().unwrap();
    drop(trees);
    drop(frame);
    drop(book);
    assert!(weak.upgrade().is_some());
    assert!(storage.upgrade().is_some());
    // Act
    let result = owner.publish_next().unwrap();
    // Assert
    assert_eq!(result, PublicationProgress::Complete);
    assert!(weak.upgrade().is_none());
    assert!(storage.upgrade().is_none());
}

#[test]
fn claims_and_advances_do_not_allocate_additional_queue_storage() {
    // Arrange
    let (book, trees, _frame) = fixture();
    book.update(ObjectId::new(1), "next", None).unwrap();
    let before = trees.metrics().unwrap();
    let capacity = book.state.lock().unwrap().pending.capacity();
    // Act
    let mut owner = book.publications().fence().unwrap().try_begin().unwrap();
    owner.publish_next().unwrap();
    owner.publish_next().unwrap();
    // Assert
    assert_eq!(trees.metrics().unwrap(), before);
    assert_eq!(book.state.lock().unwrap().pending.capacity(), capacity);
    assert!(book.publications().is_drained().unwrap());
}

#[test]
fn terminal_page_close_is_published_before_disconnect_readiness() {
    // Arrange
    let (book, trees, _frame) = fixture();
    trees
        .reconcile(
            std::iter::once(ChromiumFrameOwnership {
                frame: ObjectId::new(2),
                page: ObjectId::new(3),
                parent: None,
            }),
            |_| true,
        )
        .unwrap();
    let tree = trees.get(ObjectId::new(3)).unwrap();
    book.publications().fence().unwrap().publish().unwrap();
    trees.clear();
    let mut owner = book.publications().fence().unwrap().try_begin().unwrap();
    assert!(!tree.is_closed().unwrap());
    // Act
    let result = owner.publish_next().unwrap();
    // Assert
    assert_eq!(result, PublicationProgress::Published);
    assert!(tree.is_closed().unwrap());
    assert!(book.publications().publish_disconnect().is_err());
    assert_eq!(owner.publish_next().unwrap(), PublicationProgress::Complete);
    assert!(book.publications().publish_disconnect().unwrap());
}

#[test]
fn owner_captured_during_collection_batch_cannot_publish_its_partial_changes() {
    // Arrange
    let (book, trees, frame) = fixture();
    trees
        .reconcile(
            std::iter::once(ChromiumFrameOwnership {
                frame: ObjectId::new(2),
                page: ObjectId::new(3),
                parent: None,
            }),
            |_| true,
        )
        .unwrap();
    let tree = trees.get(ObjectId::new(3)).unwrap();
    let publications = book.publications();
    publications.fence().unwrap().publish().unwrap();
    let reserved = publications
        .reserve_collection(tree.lookup_resources())
        .unwrap();
    let batch = publications.begin_collection(reserved).unwrap();
    book.update(ObjectId::new(1), "next", None).unwrap();
    let fence = publications.fence().unwrap();
    batch.commit().unwrap();
    let mut owner = fence.try_begin().unwrap();
    // Act
    let result = owner.publish_next().unwrap();
    // Assert
    assert_eq!(result, PublicationProgress::Complete);
    assert_eq!(frame.published_url().unwrap(), "initial");
    assert_eq!(book.metrics().unwrap().pending_publications, 2);
    assert!(publications.published_collections().unwrap().is_none());
    publications.fence().unwrap().publish().unwrap();
    assert_eq!(frame.published_url().unwrap(), "next");
    assert!(
        publications
            .published_collections()
            .unwrap()
            .unwrap()
            .pages()
            .is_empty()
    );
}

#[test]
fn application_failure_discards_endpoint_and_cannot_retry_later_changes() {
    // Arrange
    let (book, _trees, frame) = fixture();
    book.update(ObjectId::new(1), "first", None).unwrap();
    book.update(ObjectId::new(1), "second", None).unwrap();
    let mut owner = book.publications().fence().unwrap().try_begin().unwrap();
    let poisoned = frame.clone();
    assert!(
        std::thread::spawn(move || {
            let _lock = poisoned.0.url.lock().unwrap();
            panic!("exercise publication target poisoning");
        })
        .join()
        .is_err()
    );
    // Act
    let result = owner.publish_next();
    // Assert
    assert!(matches!(result, Err(PublicationDrainError::Backend(_))));
    assert!(book.publications().is_drained().unwrap());
    assert!(matches!(
        owner.publish_next(),
        Err(PublicationDrainError::Finished)
    ));
    assert!(matches!(
        book.publications().fence().unwrap().try_begin(),
        Err(PublicationDrainError::Discarded)
    ));
}
