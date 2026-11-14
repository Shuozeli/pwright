use super::{FrameMetadata, FrameMetadataLimits};
use crate::HostError;
use crate::frame_tree::{FrameNode, FrameTree, FrameTreeFactory, FrameTreeLimits};
use pwright_core::ObjectId;
use pwright_plugin_chromium_frame_observer::ChromiumFrameUrlObserver;
use std::sync::Arc;

struct Fixture {
    book: Arc<FrameMetadata>,
    factory: FrameTreeFactory,
    tree: FrameTree,
    root: FrameNode,
    child: FrameNode,
}

impl Fixture {
    fn new(publications: usize, revisions: usize) -> Self {
        let book = Arc::new(
            FrameMetadata::new(FrameMetadataLimits {
                entries: 4,
                url_bytes: 64,
                total_bytes: 512,
                publications,
            })
            .unwrap(),
        );
        let factory = FrameTreeFactory::new(FrameTreeLimits {
            states: 8,
            frames_per_page: 4,
            revisions,
            bytes: 16 * 1024,
        })
        .unwrap();
        book.update(ObjectId::new(1), "root", None).unwrap();
        book.update(ObjectId::new(2), "child", None).unwrap();
        let root = factory
            .frame(ObjectId::new(1), None, book.get(ObjectId::new(1)).unwrap())
            .unwrap();
        let child = factory
            .frame(
                ObjectId::new(2),
                Some(root.id()),
                book.get(ObjectId::new(2)).unwrap(),
            )
            .unwrap();
        let tree = factory.page(root.clone()).unwrap();
        Self {
            book,
            factory,
            tree,
            root,
            child,
        }
    }
}

#[test]
fn one_watermark_publishes_url_and_tree_without_exposing_later_updates() {
    // Arrange
    let f = Fixture::new(8, 4);
    let publications = f.book.publications();
    f.book.update(f.child.id(), "attached", None).unwrap();
    publications
        .stage_tree(&f.tree, &[f.root.clone(), f.child.clone()])
        .unwrap();
    let fence = publications.fence().unwrap();
    f.book.update(f.child.id(), "later", None).unwrap();
    publications
        .stage_tree(&f.tree, std::slice::from_ref(&f.root))
        .unwrap();
    assert_eq!(f.tree.frames().unwrap().len(), 1);
    assert_eq!(f.child.url().unwrap(), "child");
    // Act
    fence.publish().unwrap();
    // Assert
    assert_eq!(f.tree.frames().unwrap().len(), 2);
    assert_eq!(f.child.url().unwrap(), "attached");
    assert!(!f.child.is_detached());
    assert_eq!(f.book.metrics().unwrap().pending_publications, 2);
}

#[test]
fn a_later_fence_drains_both_kinds_and_an_older_fence_cannot_regress() {
    // Arrange
    let f = Fixture::new(8, 4);
    let publications = f.book.publications();
    publications
        .stage_tree(&f.tree, &[f.root.clone(), f.child.clone()])
        .unwrap();
    let old = publications.fence().unwrap();
    f.book.update(f.child.id(), "last", None).unwrap();
    publications
        .stage_tree(&f.tree, std::slice::from_ref(&f.root))
        .unwrap();
    publications.fence().unwrap().publish().unwrap();
    // Act
    old.publish().unwrap();
    // Assert
    assert_eq!(f.tree.frames().unwrap().len(), 1);
    assert_eq!(f.child.url().unwrap(), "last");
    assert!(f.child.is_detached());
    assert!(publications.is_drained().unwrap());
}

#[test]
fn full_mixed_queue_rejects_before_staging_native_tree() {
    // Arrange
    let f = Fixture::new(1, 4);
    let publications = f.book.publications();
    f.book.update(f.child.id(), "queued", None).unwrap();
    let before = f.factory.metrics().unwrap();
    // Act
    let result = publications.stage_tree(&f.tree, &[f.root.clone(), f.child.clone()]);
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(f.factory.metrics().unwrap(), before);
    assert_eq!(f.tree.frames().unwrap().len(), 1);
    publications.fence().unwrap().publish().unwrap();
    // If the rejected attach had changed native state, this root-only revision
    // would retire child. It must not do so.
    publications
        .stage_tree(&f.tree, std::slice::from_ref(&f.root))
        .unwrap();
    publications.fence().unwrap().publish().unwrap();
    assert!(!f.child.is_detached());
}

#[test]
fn failed_revision_admission_returns_the_shared_queue_slot() {
    // Arrange
    let f = Fixture::new(1, 1);
    let publications = f.book.publications();
    let before = f.factory.metrics().unwrap();
    // Act
    let result = publications.stage_tree(&f.tree, &[f.root.clone(), f.child.clone()]);
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(f.factory.metrics().unwrap(), before);
    assert!(publications.is_drained().unwrap());
    f.book
        .update(f.child.id(), "url still works", None)
        .unwrap();
    publications.fence().unwrap().publish().unwrap();
    assert_eq!(f.child.url().unwrap(), "url still works");
}

#[test]
fn discard_releases_queued_revisions_without_advancing_visible_metadata() {
    // Arrange
    let f = Fixture::new(4, 4);
    let publications = f.book.publications();
    publications
        .stage_tree(&f.tree, &[f.root.clone(), f.child.clone()])
        .unwrap();
    f.book.update(f.child.id(), "new", None).unwrap();
    let fence = publications.fence().unwrap();
    // Act
    publications.discard();
    // Assert
    fence.publish().unwrap();
    assert!(publications.is_drained().unwrap());
    assert_eq!(f.tree.frames().unwrap().len(), 1);
    assert_eq!(f.child.url().unwrap(), "child");
    assert!(!f.child.is_detached());
    let before = f.factory.metrics().unwrap();
    let result = publications.stage_tree(&f.tree, std::slice::from_ref(&f.root));
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(f.factory.metrics().unwrap(), before);
}

#[tokio::test]
async fn tree_only_update_wakes_existing_publication_endpoint() {
    // Arrange
    let f = Fixture::new(4, 4);
    let publications = f.book.publications();
    publications
        .stage_tree(&f.tree, &[f.root.clone(), f.child.clone()])
        .unwrap();
    // Act
    let fence = publications.next().await.unwrap();
    // Assert
    fence.publish().unwrap();
    assert_eq!(f.tree.frames().unwrap().len(), 2);
    assert!(publications.is_drained().unwrap());
}
