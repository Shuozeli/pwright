use super::FrameTrees;
use crate::HostError;
use crate::frame_metadata::{FrameMetadata, FrameMetadataLimits};
use pwright_composition_chromium::{ChromiumFrameOwnership, ChromiumFrameUrlObserver};
use pwright_core::ObjectId;
use std::sync::Arc;

fn fixture() -> (Arc<FrameMetadata>, FrameTrees) {
    let urls = Arc::new(
        FrameMetadata::new(FrameMetadataLimits {
            entries: 128,
            url_bytes: 64,
            total_bytes: 8192,
            publications: 256,
        })
        .unwrap(),
    );
    let trees = FrameTrees::new(urls.clone()).unwrap();
    (urls, trees)
}
fn row(id: u64, page: u64, parent: Option<u64>) -> ChromiumFrameOwnership {
    ChromiumFrameOwnership {
        frame: ObjectId::new(id),
        page: ObjectId::new(page),
        parent: parent.map(ObjectId::new),
    }
}
fn project(trees: &FrameTrees, rows: &[ChromiumFrameOwnership]) {
    trees
        .reconcile(rows.iter().copied(), |page| {
            rows.iter().any(|row| row.page == page)
        })
        .unwrap();
}

#[test]
fn projection_initializes_root_and_publishes_child_with_its_url() {
    // Arrange
    let (urls, trees) = fixture();
    trees.update(ObjectId::new(2), "root", None).unwrap();
    trees
        .update(ObjectId::new(3), "child", Some("#fragment"))
        .unwrap();
    // Act
    project(&trees, &[row(3, 1, Some(2)), row(2, 1, None)]);
    // Assert
    let tree = trees.get(ObjectId::new(1)).unwrap();
    assert_eq!(tree.frames().unwrap().len(), 1);
    urls.publications().fence().unwrap().publish().unwrap();
    let frames = tree.frames().unwrap();
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].id(), ObjectId::new(2));
    assert_eq!(frames[1].url().unwrap(), "child#fragment");
    assert_eq!(frames[1].parent_id(), Some(frames[0].id()));
}

#[test]
fn unchanged_projection_does_not_spend_revision_or_queue_admission() {
    // Arrange
    let (urls, trees) = fixture();
    let rows = [row(2, 1, None), row(3, 1, Some(2))];
    project(&trees, &rows);
    urls.publications().fence().unwrap().publish().unwrap();
    let before = trees.metrics().unwrap();
    // Act
    project(&trees, &rows);
    // Assert
    assert_eq!(trees.metrics().unwrap(), before);
    assert_eq!(urls.metrics().unwrap().pending_publications, 0);
}

#[test]
fn attachment_before_navigation_starts_at_empty_url() {
    // Arrange
    let (urls, trees) = fixture();
    // Act
    project(&trees, &[row(2, 1, None), row(3, 1, Some(2))]);
    // Assert
    urls.publications().fence().unwrap().publish().unwrap();
    let child = trees.get(ObjectId::new(1)).unwrap().frames().unwrap()[1].clone();
    assert_eq!(child.url().unwrap(), "");
    trees.update(child.id(), "navigated", None).unwrap();
    assert_eq!(child.url().unwrap(), "");
    urls.publications().fence().unwrap().publish().unwrap();
    assert_eq!(child.url().unwrap(), "navigated");
}

#[test]
fn live_page_child_removal_publishes_detach_but_preserves_old_metadata() {
    // Arrange
    let (urls, trees) = fixture();
    project(&trees, &[row(2, 1, None), row(3, 1, Some(2))]);
    urls.publications().fence().unwrap().publish().unwrap();
    let tree = trees.get(ObjectId::new(1)).unwrap();
    let child = tree.frames().unwrap()[1].clone();
    // Act
    project(&trees, &[row(2, 1, None)]);
    // Assert
    assert!(!child.is_detached());
    urls.publications().fence().unwrap().publish().unwrap();
    assert!(child.is_detached());
    assert_eq!(child.parent_id(), Some(ObjectId::new(2)));
    assert_eq!(tree.frames().unwrap().len(), 1);
}

#[test]
fn page_retirement_preserves_its_collection_and_is_not_frame_detach() {
    // Arrange
    let (urls, trees) = fixture();
    project(&trees, &[row(2, 1, None), row(3, 1, Some(2))]);
    urls.publications().fence().unwrap().publish().unwrap();
    let tree = trees.get(ObjectId::new(1)).unwrap();
    // Act
    project(&trees, &[]);
    // Assert
    assert!(trees.get(ObjectId::new(1)).is_err());
    let frames = tree.frames().unwrap();
    assert_eq!(frames.len(), 2);
    assert!(frames.iter().all(|node| !node.is_detached()));
    assert!(matches!(tree.stage(&frames), Err(HostError::TargetClosed)));
}

#[test]
fn projection_preserves_node_identity_across_repeated_renderer_observations() {
    // Arrange
    let (urls, trees) = fixture();
    let rows = [row(2, 1, None), row(3, 1, Some(2))];
    project(&trees, &rows);
    urls.publications().fence().unwrap().publish().unwrap();
    let tree = trees.get(ObjectId::new(1)).unwrap();
    let child = tree.frames().unwrap()[1].clone();
    trees
        .update(child.id(), "replacement renderer", None)
        .unwrap();
    // Act
    project(&trees, &rows);
    // Assert
    urls.publications().fence().unwrap().publish().unwrap();
    assert!(tree.frames().unwrap()[1].same_identity(&child));
    assert!(!child.is_detached());
    assert_eq!(child.url().unwrap(), "replacement renderer");
}

#[test]
fn page_reservation_releases_without_creating_an_index_entry() {
    // Arrange
    let (urls, trees) = fixture();
    let before = trees.metrics().unwrap();
    let reservation = trees
        .reserve_page(urls.reserve_new_page().unwrap())
        .unwrap();
    assert_eq!(trees.metrics().unwrap().states, before.states + 4);
    // Act
    drop(reservation);
    // Assert
    assert_eq!(trees.metrics().unwrap(), before);
    assert_eq!(urls.metrics().unwrap().entries, 0);
}

#[test]
fn page_activation_consumes_existing_admission_without_double_charging() {
    // Arrange
    let (urls, trees) = fixture();
    let url = urls.reserve_new_page().unwrap();
    let reservation = trees.reserve_page(url.clone()).unwrap();
    let before = trees.metrics().unwrap();
    // Act
    let (tree, binding, _tree_binding) = trees
        .activate_page(reservation, ObjectId::new(1), ObjectId::new(2))
        .unwrap();
    // Assert
    assert_eq!(trees.metrics().unwrap(), before);
    assert_eq!(tree.frames().unwrap()[0].url().unwrap(), "about:blank");
    assert!(binding.node().same_identity(&tree.frames().unwrap()[0]));
    urls.activate_new_page(ObjectId::new(2), url).unwrap();
    project(&trees, &[row(2, 1, None)]);
    assert_eq!(trees.metrics().unwrap(), before);
}

#[test]
fn root_binding_exhaustion_rolls_back_before_any_page_activation() {
    // Arrange
    let (urls, trees) = fixture();
    let held: Vec<_> = (0..253)
        .map(|_| trees.factory.reserve_index(1).unwrap())
        .collect();
    let before = trees.metrics().unwrap();
    assert_eq!(before.states, 254);
    // Act
    let result = trees.reserve_page(urls.reserve_new_page().unwrap());
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(trees.metrics().unwrap(), before);
    assert_eq!(urls.metrics().unwrap().entries, 0);
    assert!(trees.get(ObjectId::new(1)).is_err());
    drop(held);
    assert!(trees.reserve_page(urls.reserve_new_page().unwrap()).is_ok());
}

#[test]
fn page_and_root_binding_activate_at_the_exact_state_limit() {
    // Arrange
    let (urls, trees) = fixture();
    let _held: Vec<_> = (0..251)
        .map(|_| trees.factory.reserve_index(1).unwrap())
        .collect();
    let reservation = trees
        .reserve_page(urls.reserve_new_page().unwrap())
        .unwrap();
    let before = trees.metrics().unwrap();
    assert_eq!(before.states, 256);
    // Act
    let (tree, binding, _tree_binding) = trees
        .activate_page(reservation, ObjectId::new(1), ObjectId::new(2))
        .unwrap();
    // Assert
    assert_eq!(trees.metrics().unwrap(), before);
    assert!(binding.node().same_identity(&tree.frames().unwrap()[0]));
}

#[test]
fn retained_views_release_every_node_after_session_index_teardown() {
    // Arrange
    let (urls, trees) = fixture();
    let empty = trees.metrics().unwrap();
    project(&trees, &[row(2, 1, None), row(3, 1, Some(2))]);
    urls.publications().fence().unwrap().publish().unwrap();
    let retained = trees.get(ObjectId::new(1)).unwrap();
    trees.clear();
    urls.clear();
    assert_eq!(retained.frames().unwrap().len(), 2);
    assert_eq!(urls.metrics().unwrap().entries, 2);
    // Act
    drop(retained);
    urls.publications().fence().unwrap().publish().unwrap();
    // Assert
    assert_eq!(trees.metrics().unwrap(), empty);
    assert_eq!(urls.metrics().unwrap().entries, 0);
    assert!(matches!(
        trees.reconcile(std::iter::empty(), |_| false),
        Err(HostError::TargetClosed)
    ));
}

#[test]
fn tree_wrapper_exhaustion_rolls_back_all_four_page_reservations() {
    // Arrange
    let (urls, trees) = fixture();
    let _held: Vec<_> = (0..252)
        .map(|_| trees.factory.reserve_index(1).unwrap())
        .collect();
    let before = trees.metrics().unwrap();
    assert_eq!(before.states, 253);
    // Act
    let result = trees.reserve_page(urls.reserve_new_page().unwrap());
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(trees.metrics().unwrap(), before);
    assert_eq!(urls.metrics().unwrap().entries, 0);
    assert!(trees.get(ObjectId::new(1)).is_err());
}

#[test]
fn oversized_projection_fails_before_mutating_existing_views() {
    // Arrange
    let (urls, trees) = fixture();
    project(&trees, &[row(2, 1, None)]);
    let before = trees.metrics().unwrap();
    let rows = (0..129).map(|index| row(index + 2, 1, if index == 0 { None } else { Some(2) }));
    // Act
    let result = trees.reconcile(rows, |_| true);
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(trees.metrics().unwrap(), before);
    assert_eq!(
        trees.get(ObjectId::new(1)).unwrap().frames().unwrap().len(),
        1
    );
    assert_eq!(urls.metrics().unwrap().pending_publications, 0);
}

#[test]
fn native_page_retirement_is_visible_only_at_its_publication_fence() {
    // Arrange
    let (urls, trees) = fixture();
    project(&trees, &[row(2, 1, None)]);
    let tree = trees.get(ObjectId::new(1)).unwrap();
    let old = urls.publications().fence().unwrap();
    // Act
    project(&trees, &[]);
    old.publish().unwrap();
    let staged = tree.stage(&tree.frames().unwrap());
    // Assert
    assert!(!tree.is_closed().unwrap());
    assert!(matches!(staged, Err(HostError::TargetClosed)));
    urls.publications().fence().unwrap().publish().unwrap();
    assert!(tree.is_closed().unwrap());
    assert!(!tree.frames().unwrap()[0].is_detached());
}

#[test]
fn an_older_fence_cannot_reopen_a_published_page() {
    // Arrange
    let (urls, trees) = fixture();
    project(&trees, &[row(2, 1, None)]);
    let tree = trees.get(ObjectId::new(1)).unwrap();
    let old = urls.publications().fence().unwrap();
    project(&trees, &[]);
    urls.publications().fence().unwrap().publish().unwrap();
    // Act
    old.publish().unwrap();
    // Assert
    assert!(tree.is_closed().unwrap());
}

#[test]
fn full_metadata_admission_cannot_evict_the_reserved_page_close() {
    // Arrange
    let (urls, trees) = fixture();
    project(&trees, &[row(2, 1, None)]);
    let tree = trees.get(ObjectId::new(1)).unwrap();
    for index in 0..255 {
        trees
            .update(ObjectId::new(2), &index.to_string(), None)
            .unwrap();
    }
    assert_eq!(urls.metrics().unwrap().pending_publications, 255);
    let before = trees.metrics().unwrap();
    let refused = trees.reserve_page(urls.reserve_new_page().unwrap());
    assert!(matches!(refused, Err(HostError::ResourceExhausted(_))));
    assert_eq!(trees.metrics().unwrap(), before);
    // Act
    project(&trees, &[]);
    // Assert
    assert_eq!(urls.metrics().unwrap().pending_publications, 256);
    assert!(!tree.is_closed().unwrap());
    urls.publications().fence().unwrap().publish().unwrap();
    assert!(tree.is_closed().unwrap());
    assert_eq!(tree.frames().unwrap()[0].url().unwrap(), "254");
}

#[test]
fn saturated_teardown_closes_all_pages_after_the_terminal_fence() {
    // Arrange
    let (urls, trees) = fixture();
    project(&trees, &[row(2, 1, None), row(4, 3, None)]);
    let first = trees.get(ObjectId::new(1)).unwrap();
    let second = trees.get(ObjectId::new(3)).unwrap();
    for index in 0..254 {
        trees
            .update(ObjectId::new(2), &index.to_string(), None)
            .unwrap();
    }
    let old = urls.publications().fence().unwrap();
    // Act
    trees.clear();
    trees.clear();
    old.publish().unwrap();
    // Assert
    assert_eq!(urls.metrics().unwrap().pending_publications, 2);
    assert!(!first.is_closed().unwrap());
    assert!(!second.is_closed().unwrap());
    urls.publications().fence().unwrap().publish().unwrap();
    assert!(first.is_closed().unwrap());
    assert!(second.is_closed().unwrap());
    assert!(urls.publications().is_drained().unwrap());
}

#[test]
fn environment_discard_releases_terminal_storage_without_publishing() {
    // Arrange
    let (urls, trees) = fixture();
    let empty = trees.metrics().unwrap();
    project(&trees, &[row(2, 1, None)]);
    let tree = trees.get(ObjectId::new(1)).unwrap();
    trees.clear();
    urls.clear();
    let fence = urls.publications().fence().unwrap();
    // Act
    urls.publications().discard();
    fence.publish().unwrap();
    // Assert
    assert!(!tree.is_closed().unwrap());
    assert!(urls.publications().is_drained().unwrap());
    drop(tree);
    assert_eq!(trees.metrics().unwrap(), empty);
    assert_eq!(urls.metrics().unwrap().entries, 0);
}

#[test]
fn discarded_environment_still_retires_native_trees_without_queueing() {
    // Arrange
    let (urls, trees) = fixture();
    project(&trees, &[row(2, 1, None)]);
    let tree = trees.get(ObjectId::new(1)).unwrap();
    urls.publications().discard();
    // Act
    trees.clear();
    let staged = tree.stage(&tree.frames().unwrap());
    // Assert
    assert!(!tree.is_closed().unwrap());
    assert_eq!(urls.metrics().unwrap().pending_publications, 0);
    assert!(matches!(staged, Err(HostError::TargetClosed)));
}

#[test]
fn pending_close_owns_its_metadata_leases_until_delivery() {
    // Arrange
    let (urls, trees) = fixture();
    let empty = trees.metrics().unwrap();
    project(&trees, &[row(2, 1, None)]);
    trees.clear();
    urls.clear();
    assert_eq!(urls.metrics().unwrap().entries, 1);
    assert!(trees.metrics().unwrap().states > empty.states);
    // Act
    urls.publications().fence().unwrap().publish().unwrap();
    // Assert
    assert_eq!(trees.metrics().unwrap(), empty);
    assert_eq!(urls.metrics().unwrap().entries, 0);
}
