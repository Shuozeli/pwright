use super::{FrameNode, FrameTree, FrameTreeFactory, FrameTreeLimits, FrameTreeMetrics};
use crate::HostError;
use crate::frame_metadata::{FrameMetadata, FrameMetadataLimits};
use pwright_composition_chromium::ChromiumFrameUrlObserver;
use pwright_core::ObjectId;

fn limits() -> FrameTreeLimits {
    FrameTreeLimits {
        states: 16,
        frames_per_page: 8,
        revisions: 8,
        bytes: 32 * 1024,
    }
}

struct Fixture {
    factory: FrameTreeFactory,
    urls: FrameMetadata,
}

impl Fixture {
    fn new(limits: FrameTreeLimits) -> Self {
        Self {
            factory: FrameTreeFactory::new(limits).unwrap(),
            urls: FrameMetadata::new(FrameMetadataLimits {
                entries: 16,
                url_bytes: 64,
                total_bytes: 1024,
                publications: 0,
            })
            .unwrap(),
        }
    }

    fn node(&self, id: u64, parent: Option<u64>) -> FrameNode {
        let id = ObjectId::new(id);
        self.urls
            .update(id, "https://fixture.test/frame", None)
            .unwrap();
        self.factory
            .frame(id, parent.map(ObjectId::new), self.urls.get(id).unwrap())
            .unwrap()
    }

    fn page_with_child(&self) -> (FrameTree, FrameNode, FrameNode) {
        let root = self.node(1, None);
        let child = self.node(2, Some(1));
        let tree = self.factory.page(root.clone()).unwrap();
        tree.stage(&[root.clone(), child.clone()])
            .unwrap()
            .publish()
            .unwrap();
        (tree, root, child)
    }
}

fn ids(tree: &FrameTree) -> Vec<ObjectId> {
    tree.frames().unwrap().iter().map(FrameNode::id).collect()
}

#[test]
fn staged_attachment_is_invisible_until_publication() {
    // Arrange
    let fixture = Fixture::new(limits());
    let root = fixture.node(1, None);
    let child = fixture.node(2, Some(1));
    let tree = fixture.factory.page(root.clone()).unwrap();
    // Act
    let publication = tree.stage(&[root.clone(), child.clone()]).unwrap();
    // Assert
    assert_eq!(ids(&tree), vec![root.id()]);
    assert!(!child.is_detached());
    publication.publish().unwrap();
    assert_eq!(ids(&tree), vec![root.id(), child.id()]);
    assert!(tree.frames().unwrap()[1].same_identity(&child));
}

#[test]
fn detach_publication_preserves_parent_identity_and_url() {
    // Arrange
    let fixture = Fixture::new(limits());
    let (tree, root, child) = fixture.page_with_child();
    // Act
    let publication = tree.stage(std::slice::from_ref(&root)).unwrap();
    // Assert
    assert_eq!(ids(&tree), vec![root.id(), child.id()]);
    assert!(!child.is_detached());
    publication.publish().unwrap();
    assert_eq!(ids(&tree), vec![root.id()]);
    assert!(child.is_detached());
    assert_eq!(child.parent_id(), Some(root.id()));
    assert_eq!(child.url().unwrap(), "https://fixture.test/frame");
    assert!(!root.is_detached());
}

#[test]
fn newer_publication_detaches_even_an_unpublished_child() {
    // Arrange
    let fixture = Fixture::new(limits());
    let root = fixture.node(1, None);
    let child = fixture.node(2, Some(1));
    let tree = fixture.factory.page(root.clone()).unwrap();
    let old = tree.stage(&[root.clone(), child.clone()]).unwrap();
    let new = tree.stage(std::slice::from_ref(&root)).unwrap();
    // Act
    new.publish().unwrap();
    // Assert
    assert!(child.is_detached());
    assert_eq!(ids(&tree), vec![root.id()]);
    old.publish().unwrap();
    assert_eq!(ids(&tree), vec![root.id()]);
    assert!(child.is_detached());
}

#[test]
fn skipped_removal_publication_is_carried_to_the_next_revision() {
    // Arrange
    let fixture = Fixture::new(limits());
    let (tree, root, child) = fixture.page_with_child();
    let removal = tree.stage(std::slice::from_ref(&root)).unwrap();
    drop(removal);
    let replacement = fixture.node(3, Some(1));
    let latest = tree.stage(&[root.clone(), replacement.clone()]).unwrap();
    // Act
    latest.publish().unwrap();
    // Assert
    assert!(child.is_detached());
    assert!(!replacement.is_detached());
    assert_eq!(ids(&tree), vec![root.id(), replacement.id()]);
}

#[test]
fn closing_page_preserves_published_frames_without_detaching() {
    // Arrange
    let fixture = Fixture::new(limits());
    let (tree, root, child) = fixture.page_with_child();
    // Act
    tree.close().unwrap();
    // Assert
    assert_eq!(ids(&tree), vec![root.id(), child.id()]);
    assert!(!root.is_detached());
    assert!(!child.is_detached());
    assert_eq!(child.parent_id(), Some(root.id()));
    let result = tree.stage(std::slice::from_ref(&root));
    assert!(matches!(result, Err(HostError::TargetClosed)));
}

#[test]
fn already_staged_removal_can_publish_after_page_close() {
    // Arrange
    let fixture = Fixture::new(limits());
    let (tree, root, child) = fixture.page_with_child();
    let removal = tree.stage(std::slice::from_ref(&root)).unwrap();
    tree.close().unwrap();
    // Act
    removal.publish().unwrap();
    // Assert
    assert!(child.is_detached());
    assert!(!root.is_detached());
    assert_eq!(ids(&tree), vec![root.id()]);
}

#[test]
fn foreign_factory_root_is_rejected_without_spending_budget() {
    // Arrange
    let fixture = Fixture::new(limits());
    let foreign = Fixture::new(limits());
    let root = foreign.node(1, None);
    let before = fixture.factory.metrics().unwrap();
    // Act
    let result = fixture.factory.page(root);
    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert_eq!(fixture.factory.metrics().unwrap(), before);
}

#[test]
fn foreign_factory_child_is_rejected_without_mutating_the_tree() {
    // Arrange
    let fixture = Fixture::new(limits());
    let foreign = Fixture::new(limits());
    let root = fixture.node(1, None);
    let tree = fixture.factory.page(root.clone()).unwrap();
    let child = foreign.node(2, Some(1));
    let before = fixture.factory.metrics().unwrap();
    // Act
    let result = tree.stage(&[root.clone(), child.clone()]);
    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert_eq!(ids(&tree), vec![root.id()]);
    assert_eq!(fixture.factory.metrics().unwrap(), before);
}

#[test]
fn a_main_frame_cannot_back_two_page_trees() {
    // Arrange
    let fixture = Fixture::new(limits());
    let root = fixture.node(1, None);
    let _tree = fixture.factory.page(root.clone()).unwrap();
    let before = fixture.factory.metrics().unwrap();
    // Act
    let result = fixture.factory.page(root);
    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert_eq!(fixture.factory.metrics().unwrap(), before);
}

#[test]
fn invalid_parent_graphs_reject_without_admission_or_publication() {
    // Arrange
    for edges in [
        vec![(2, None)],
        vec![(2, Some(9))],
        vec![(2, Some(3)), (3, Some(2))],
    ] {
        let fixture = Fixture::new(limits());
        let root = fixture.node(1, None);
        let tree = fixture.factory.page(root.clone()).unwrap();
        let mut frames = vec![root.clone()];
        frames.extend(
            edges
                .into_iter()
                .map(|(id, parent)| fixture.node(id, parent)),
        );
        let before = fixture.factory.metrics().unwrap();
        // Act
        let result = tree.stage(&frames);
        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
        assert_eq!(ids(&tree), vec![root.id()]);
        assert_eq!(fixture.factory.metrics().unwrap(), before);
    }
}

#[test]
fn duplicate_frame_is_rejected() {
    // Arrange
    let fixture = Fixture::new(limits());
    let (tree, root, child) = fixture.page_with_child();
    let before = fixture.factory.metrics().unwrap();
    // Act
    let result = tree.stage(&[root, child.clone(), child]);
    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert_eq!(fixture.factory.metrics().unwrap(), before);
}

#[test]
fn retained_frame_id_cannot_be_rebound_to_a_different_node() {
    // Arrange
    let fixture = Fixture::new(limits());
    let (tree, root, child) = fixture.page_with_child();
    let rebound = fixture.node(2, Some(1));
    // Act
    let result = tree.stage(&[root, rebound]);
    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert!(tree.frames().unwrap()[1].same_identity(&child));
}

#[test]
fn retired_identity_cannot_reappear_before_detach_is_published() {
    // Arrange
    let fixture = Fixture::new(limits());
    let (tree, root, child) = fixture.page_with_child();
    let pending = tree.stage(std::slice::from_ref(&root)).unwrap();
    // Act
    let result = tree.stage(&[root, child.clone()]);
    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert!(!child.is_detached());
    pending.publish().unwrap();
    assert!(child.is_detached());
}

#[test]
fn detached_node_cannot_be_reattached() {
    // Arrange
    let fixture = Fixture::new(limits());
    let (tree, root, child) = fixture.page_with_child();
    tree.stage(std::slice::from_ref(&root))
        .unwrap()
        .publish()
        .unwrap();
    // Act
    let result = tree.stage(&[root.clone(), child]);
    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert_eq!(ids(&tree), vec![root.id()]);
}

#[test]
fn removal_of_subtree_detaches_descendants_at_the_same_publication() {
    // Arrange
    let fixture = Fixture::new(limits());
    let (tree, root, child) = fixture.page_with_child();
    let nested = fixture.node(3, Some(2));
    tree.stage(&[root.clone(), child.clone(), nested.clone()])
        .unwrap()
        .publish()
        .unwrap();
    // Act
    tree.stage(std::slice::from_ref(&root))
        .unwrap()
        .publish()
        .unwrap();
    // Assert
    assert!(child.is_detached());
    assert!(nested.is_detached());
    assert_eq!(nested.parent_id(), Some(child.id()));
    assert_eq!(ids(&tree), vec![root.id()]);
}

#[test]
fn state_exhaustion_releases_failed_node_reservations() {
    // Arrange
    let fixture = Fixture::new(FrameTreeLimits {
        states: 2,
        ..limits()
    });
    let root = fixture.node(1, None);
    let tree = fixture.factory.page(root.clone()).unwrap();
    let url = fixture.urls.get(root.id()).unwrap();
    let before = fixture.factory.metrics().unwrap();
    // Act
    let result = fixture
        .factory
        .frame(ObjectId::new(2), Some(root.id()), url);
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(fixture.factory.metrics().unwrap(), before);
    assert_eq!(ids(&tree), vec![root.id()]);
}

#[test]
fn revision_exhaustion_rolls_back_without_detach_or_tree_change() {
    // Arrange
    let fixture = Fixture::new(FrameTreeLimits {
        revisions: 2,
        ..limits()
    });
    let (tree, root, child) = fixture.page_with_child();
    let pending = tree.stage(std::slice::from_ref(&root)).unwrap();
    let before = fixture.factory.metrics().unwrap();
    // Act
    let result = tree.stage(std::slice::from_ref(&root));
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(fixture.factory.metrics().unwrap(), before);
    assert_eq!(ids(&tree), vec![root.id(), child.id()]);
    assert!(!child.is_detached());
    pending.publish().unwrap();
    assert!(child.is_detached());
    tree.stage(std::slice::from_ref(&root))
        .unwrap()
        .publish()
        .unwrap();
}

#[test]
fn byte_exhaustion_releases_the_revision_slot() {
    // Arrange: measure admitted storage through the public metrics contract.
    let generous = Fixture::new(limits());
    let (_tree, _, _) = generous.page_with_child();
    let bytes = generous.factory.metrics().unwrap().bytes;
    let fixture = Fixture::new(FrameTreeLimits { bytes, ..limits() });
    let root = fixture.node(1, None);
    let child = fixture.node(2, Some(1));
    let tree = fixture.factory.page(root.clone()).unwrap();
    let before = fixture.factory.metrics().unwrap();
    // Act
    let result = tree.stage(&[root.clone(), child.clone()]);
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(fixture.factory.metrics().unwrap(), before);
    assert_eq!(ids(&tree), vec![root.id()]);
}

#[test]
fn exact_state_and_revision_limits_need_no_extra_scratch_lease() {
    // Arrange
    let fixture = Fixture::new(FrameTreeLimits {
        states: 3,
        revisions: 2,
        ..limits()
    });
    let (tree, root, child) = fixture.page_with_child();
    // Act
    let pending = tree.stage(std::slice::from_ref(&root)).unwrap();
    // Assert
    let metrics = fixture.factory.metrics().unwrap();
    assert_eq!(metrics.states, 3);
    assert_eq!(metrics.revisions, 2);
    pending.publish().unwrap();
    assert!(child.is_detached());
}

#[test]
fn retained_frame_keeps_only_its_node_and_url_until_last_reference_drops() {
    // Arrange
    let fixture = Fixture::new(limits());
    let (tree, root, child) = fixture.page_with_child();
    tree.stage(std::slice::from_ref(&root))
        .unwrap()
        .publish()
        .unwrap();
    let retained = child.clone();
    fixture.urls.clear();
    drop(tree);
    drop(root);
    drop(child);
    assert_eq!(fixture.factory.metrics().unwrap().states, 1);
    assert_eq!(fixture.factory.metrics().unwrap().revisions, 0);
    assert_eq!(fixture.urls.metrics().unwrap().entries, 1);
    assert_eq!(retained.url().unwrap(), "https://fixture.test/frame");
    // Act
    drop(retained);
    // Assert
    assert_eq!(
        fixture.factory.metrics().unwrap(),
        FrameTreeMetrics {
            states: 0,
            revisions: 0,
            bytes: 0
        }
    );
    assert_eq!(fixture.urls.metrics().unwrap().entries, 0);
    assert_eq!(fixture.urls.metrics().unwrap().url_bytes, 0);
}

#[test]
fn dropped_publication_releases_tree_when_no_other_owner_remains() {
    // Arrange
    let fixture = Fixture::new(limits());
    let (tree, root, child) = fixture.page_with_child();
    let pending = tree.stage(std::slice::from_ref(&root)).unwrap();
    drop(tree);
    drop(root);
    drop(child);
    fixture.urls.clear();
    // Act
    drop(pending);
    // Assert
    assert_eq!(
        fixture.factory.metrics().unwrap(),
        FrameTreeMetrics {
            states: 0,
            revisions: 0,
            bytes: 0
        }
    );
    assert_eq!(fixture.urls.metrics().unwrap().entries, 0);
}

#[test]
fn read_holds_the_published_collection_and_conversion_charge_until_drop() {
    // Arrange
    let fixture = Fixture::new(limits());
    let (tree, root, child) = fixture.page_with_child();
    let before = fixture.factory.metrics().unwrap();
    // Act
    let read = tree.read(128).unwrap();
    // Assert
    let during = fixture.factory.metrics().unwrap();
    assert_eq!(during.revisions, before.revisions + 1);
    assert_eq!(
        during.bytes,
        before.bytes + 256 + std::mem::size_of::<super::FrameTreeRead>() as u64
    );
    tree.stage(std::slice::from_ref(&root))
        .unwrap()
        .publish()
        .unwrap();
    assert_eq!(read.frames().len(), 2);
    assert!(read.frames()[1].same_identity(&child));
    assert_eq!(ids(&tree), vec![root.id()]);
    drop(read);
    assert_eq!(fixture.factory.metrics().unwrap().revisions, 1);
}

#[test]
fn read_exhaustion_and_overflow_do_not_change_publication_or_leak_count() {
    // Arrange
    let fixture = Fixture::new(limits());
    let (tree, root, child) = fixture.page_with_child();
    let before = fixture.factory.metrics().unwrap();
    // Act
    let exhausted = tree.read(32 * 1024);
    let overflow = tree.read(usize::MAX);
    // Assert
    assert!(matches!(exhausted, Err(HostError::ResourceExhausted(_))));
    assert!(matches!(overflow, Err(HostError::InvalidArgument(_))));
    assert_eq!(fixture.factory.metrics().unwrap(), before);
    assert_eq!(ids(&tree), vec![root.id(), child.id()]);
}

#[test]
fn read_capacity_is_shared_with_revisions_and_released_for_retry() {
    // Arrange
    let fixture = Fixture::new(FrameTreeLimits {
        revisions: 2,
        ..limits()
    });
    let (tree, _, _) = fixture.page_with_child();
    let first = tree.read(0).unwrap();
    // Act
    let exhausted = tree.read(0);
    // Assert
    assert!(matches!(exhausted, Err(HostError::ResourceExhausted(_))));
    drop(first);
    assert!(tree.read(0).is_ok());
}

#[test]
fn native_binding_keeps_only_its_node_and_wrapper_storage_after_close() {
    // Arrange
    let fixture = Fixture::new(limits());
    let (tree, root, child) = fixture.page_with_child();
    let before = fixture.factory.metrics().unwrap();
    let binding = tree.bind(2).unwrap();
    assert_eq!(fixture.factory.metrics().unwrap().states, before.states + 1);
    // Act
    tree.close().unwrap();
    drop(tree);
    drop(root);
    drop(child);
    fixture.urls.clear();
    // Assert
    assert!(!binding.node().is_detached());
    assert_eq!(binding.node().parent_id(), Some(ObjectId::new(1)));
    assert_eq!(binding.node().url().unwrap(), "https://fixture.test/frame");
    assert_eq!(fixture.factory.metrics().unwrap().states, 2);
    assert_eq!(fixture.factory.metrics().unwrap().revisions, 0);
    drop(binding);
    assert_eq!(fixture.factory.metrics().unwrap().bytes, 0);
    assert_eq!(fixture.urls.metrics().unwrap().entries, 0);
}

#[test]
fn native_binding_refuses_unpublished_or_retired_nodes() {
    // Arrange
    let fixture = Fixture::new(limits());
    let (tree, root, child) = fixture.page_with_child();
    let retained = tree.bind(2).unwrap();
    let replacement = fixture.node(3, Some(1));
    let pending = tree.stage(&[root, replacement]).unwrap();
    // Act
    let unpublished = tree.bind(3);
    pending.publish().unwrap();
    let retired = tree.bind(2);
    // Assert
    assert!(matches!(unpublished, Err(HostError::TargetClosed)));
    assert!(matches!(retired, Err(HostError::TargetClosed)));
    assert!(retained.node().same_identity(&child));
    assert!(retained.node().is_detached());
}

#[test]
fn wrapper_exhaustion_is_recoverable_without_losing_tree_identity() {
    // Arrange
    let fixture = Fixture::new(FrameTreeLimits {
        states: 4,
        ..limits()
    });
    let (tree, _, child) = fixture.page_with_child();
    let binding = tree.bind(2).unwrap();
    let before = fixture.factory.metrics().unwrap();
    // Act
    let result = tree.bind(2);
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(fixture.factory.metrics().unwrap(), before);
    drop(binding);
    assert!(tree.bind(2).unwrap().node().same_identity(&child));
}

#[test]
fn tree_wrapper_retains_only_admitted_metadata_after_tree_owner_drops() {
    // Arrange
    let fixture = Fixture::new(limits());
    let (tree, root, child) = fixture.page_with_child();
    let before = fixture.factory.metrics().unwrap();
    let binding = tree.bind_tree().unwrap();
    assert_eq!(fixture.factory.metrics().unwrap().states, before.states + 1);
    drop(tree);
    drop(root);
    drop(child);
    fixture.urls.clear();
    assert_eq!(binding.tree().frames().unwrap().len(), 2);
    // Act
    drop(binding);
    // Assert
    assert_eq!(fixture.factory.metrics().unwrap().bytes, 0);
    assert_eq!(fixture.urls.metrics().unwrap().entries, 0);
}

#[test]
fn tree_wrapper_admission_failure_preserves_collection_and_budget() {
    // Arrange
    let fixture = Fixture::new(FrameTreeLimits {
        states: 3,
        ..limits()
    });
    let (tree, root, child) = fixture.page_with_child();
    let before = fixture.factory.metrics().unwrap();
    // Act
    let result = tree.bind_tree();
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(fixture.factory.metrics().unwrap(), before);
    assert_eq!(ids(&tree), vec![root.id(), child.id()]);
}

#[test]
fn descriptor_conversion_guard_charges_before_copy_and_releases_on_finish() {
    // Arrange
    let fixture = Fixture::new(limits());
    let before = fixture.factory.metrics().unwrap();
    // Act
    let guard = fixture.factory.reserve_conversion(1024).unwrap();
    // Assert
    let during = fixture.factory.metrics().unwrap();
    assert_eq!(during.states, before.states);
    assert_eq!(during.revisions, before.revisions + 1);
    assert_eq!(
        during.bytes,
        before.bytes + 1024 + std::mem::size_of::<super::FrameConversion>() as u64
    );
    guard.finish();
    assert_eq!(fixture.factory.metrics().unwrap(), before);
}

#[test]
fn descriptor_conversion_errors_release_partial_admission() {
    // Arrange
    let fixture = Fixture::new(limits());
    let before = fixture.factory.metrics().unwrap();
    // Act
    let exhausted = fixture.factory.reserve_conversion(32 * 1024);
    let overflow = fixture.factory.reserve_conversion(usize::MAX);
    // Assert
    assert!(matches!(exhausted, Err(HostError::ResourceExhausted(_))));
    assert!(matches!(overflow, Err(HostError::InvalidArgument(_))));
    assert_eq!(fixture.factory.metrics().unwrap(), before);
}
