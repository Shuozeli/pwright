use super::*;
use crate::frame_metadata::FrameMetadataLimits;
use crate::frame_trees::FrameTrees;
use pwright_composition_chromium::{ChromiumFrameOwnership, ChromiumFrameUrlObserver};
use pwright_core::ObjectId;

fn fixture() -> (Arc<FrameMetadata>, FrameTrees, FrameTree) {
    let book = Arc::new(
        FrameMetadata::new(FrameMetadataLimits {
            entries: 8,
            url_bytes: 64,
            total_bytes: 1024,
            publications: 4,
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
    let tree = trees.get(ObjectId::new(1)).unwrap();
    (book, trees, tree)
}

#[test]
fn closure_selection_rejects_a_previously_allocated_unadmitted_queue() {
    // Arrange: a legacy growable queue may retain more capacity than its limit.
    let book = Arc::new(
        FrameMetadata::new(FrameMetadataLimits {
            entries: 8,
            url_bytes: 64,
            total_bytes: 1024,
            publications: 3,
        })
        .unwrap(),
    );
    book.update(ObjectId::new(2), "first", None).unwrap();
    book.update(ObjectId::new(2), "second", None).unwrap();
    book.publications().fence().unwrap().publish().unwrap();
    assert!(book.state.lock().unwrap().pending.capacity() > 0);
    // Act
    let result = FrameTrees::new(book.clone());
    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert!(book.state.lock().unwrap().queue_storage.is_none());
}

#[test]
fn exhausted_normal_sequence_preserves_close_permit_for_terminal_teardown() {
    // Arrange: exercise the last normal sequence without trillions of events.
    let (book, trees, tree) = fixture();
    book.state.lock().unwrap().sequence = u64::MAX - 1;
    let old = book.publications().fence().unwrap();
    let failed = trees.reconcile(std::iter::empty(), |_| false);
    assert!(matches!(failed, Err(HostError::ResourceExhausted(_))));
    assert!(!tree.is_closed().unwrap());
    assert_eq!(book.metrics().unwrap().pending_publications, 0);
    // Act
    trees.clear();
    old.publish().unwrap();
    // Assert
    assert!(!tree.is_closed().unwrap());
    assert_eq!(book.metrics().unwrap().pending_publications, 1);
    book.publications().fence().unwrap().publish().unwrap();
    let refused = book.publications().reserve_close();
    assert!(tree.is_closed().unwrap());
    assert!(matches!(refused, Err(HostError::TargetClosed)));
}

#[test]
fn foreign_queue_permit_cannot_close_a_page_or_spend_local_admission() {
    // Arrange
    let (book, _trees, tree) = fixture();
    let (foreign, _foreign_trees, _foreign_tree) = fixture();
    let mut permit = Some(foreign.publications().reserve_close().unwrap());
    // Act
    let result = book.publications().stage_close(&tree, &mut permit);
    let subsequent = tree.stage(&tree.frames().unwrap());
    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert!(permit.is_some());
    assert!(!tree.is_closed().unwrap());
    assert_eq!(book.metrics().unwrap().pending_publications, 0);
    assert!(subsequent.is_ok());
}

#[test]
fn teardown_uses_only_preallocated_queue_storage() {
    // Arrange
    let (book, trees, tree) = fixture();
    let capacity = book.state.lock().unwrap().pending.capacity();
    let permits: Vec<_> = (0..3)
        .map(|_| book.publications().reserve_close().unwrap())
        .collect();
    let refused = book.publications().reserve_close();
    assert!(matches!(refused, Err(HostError::ResourceExhausted(_))));
    // Act
    trees.clear();
    // Assert
    assert_eq!(book.state.lock().unwrap().pending.capacity(), capacity);
    assert_eq!(book.metrics().unwrap().pending_publications, 1);
    assert!(!tree.is_closed().unwrap());
    drop(permits);
    book.publications().fence().unwrap().publish().unwrap();
    assert!(tree.is_closed().unwrap());
}

#[tokio::test]
async fn a_close_only_publication_wakes_the_existing_metadata_watch() {
    // Arrange
    let (book, trees, tree) = fixture();
    // Act
    trees.reconcile(std::iter::empty(), |_| false).unwrap();
    let fence = book.publications().next().await.unwrap();
    fence.publish().unwrap();
    // Assert
    assert!(tree.is_closed().unwrap());
    assert!(book.publications().is_drained().unwrap());
}
