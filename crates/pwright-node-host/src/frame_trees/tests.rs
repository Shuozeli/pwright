use super::{FrameTrees, STATES};
use crate::context_metadata::{ContextMetadata, ContextMetadataLimits};
use crate::frame_metadata::{FrameMetadata, FrameMetadataLimits};
use crate::{CollectionMember, CollectionView, HostError};
use pwright_core::ObjectId;
use pwright_plugin_chromium_frame_observer::{ChromiumFrameOwnership, ChromiumFrameUrlObserver};
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

#[cfg(feature = "console-events")]
#[test]
fn console_profile_admits_three_hundred_one_pending_message_bodies() {
    // Arrange
    let urls = Arc::new(
        FrameMetadata::new(FrameMetadataLimits {
            entries: 128,
            url_bytes: 64,
            total_bytes: 2 * 1024 * 1024,
            publications: 512,
        })
        .unwrap(),
    );
    let trees = FrameTrees::new(urls.clone()).unwrap();
    let payload_bytes = crate::ConsoleMessage::storage_bytes(3, 3, 5, 0).unwrap();
    let baseline = trees.metrics().unwrap();

    // Act
    let reservations = (0..301)
        .map(|_| {
            urls.publications()
                .reserve_lifecycle_payload(&trees.factory, payload_bytes)
                .unwrap()
        })
        .collect::<Vec<_>>();

    // Assert
    assert_eq!(reservations.len(), 301);
    assert_eq!(trees.metrics().unwrap().revisions - baseline.revisions, 301);
    drop(reservations);
    assert_eq!(trees.metrics().unwrap(), baseline);
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

#[cfg(feature = "frame-events")]
#[test]
fn attached_child_is_published_after_tree_with_portable_identity() {
    use crate::frame_trees::close_events::tests::Fixture;
    use crate::{LifecycleDelivery, LifecycleNotice, PublicationDrainError, PublicationProgress};

    // Arrange
    let fixture = Fixture::external();
    let roots = [row(2, 1, None), row(4, 3, None)];
    let attached = [roots[0], row(5, 1, Some(2)), roots[1]];

    // Act
    fixture
        .owner
        .reconcile(attached.into_iter(), |_| true)
        .unwrap();
    let mut drain = fixture.book.fence().unwrap().try_begin().unwrap();

    // Assert
    assert_eq!(
        drain.publish_next().unwrap(),
        PublicationProgress::Published
    );
    assert_eq!(
        fixture
            .owner
            .get(ObjectId::new(1))
            .unwrap()
            .frames()
            .unwrap()
            .len(),
        2
    );
    assert!(matches!(
        drain.publish_next(),
        Err(PublicationDrainError::NoticePending)
    ));
    assert_eq!(
        drain.commit_notice().unwrap(),
        LifecycleNotice::PageFrameAttached {
            context: ObjectId::new(1000),
            page: ObjectId::new(1),
            frame: ObjectId::new(5),
        }
    );
    drain
        .acknowledge_notice(LifecycleDelivery::Delivered)
        .unwrap();
    assert_eq!(drain.publish_next().unwrap(), PublicationProgress::Complete);
}

#[cfg(feature = "frame-events")]
#[test]
fn repeated_projection_does_not_duplicate_frameattached() {
    use crate::frame_trees::close_events::tests::Fixture;

    // Arrange
    let fixture = Fixture::external();
    let attached = [row(2, 1, None), row(5, 1, Some(2)), row(4, 3, None)];
    fixture
        .owner
        .reconcile(attached.into_iter(), |_| true)
        .unwrap();
    assert_eq!(fixture.drain().len(), 1);

    // Act
    fixture
        .owner
        .reconcile(attached.into_iter(), |_| true)
        .unwrap();

    // Assert
    assert!(fixture.drain().is_empty());
}

#[cfg(feature = "frame-events")]
#[test]
fn navigation_notice_retains_the_attached_child_identity() {
    use crate::LifecycleNotice;
    use crate::frame_trees::close_events::tests::Fixture;

    // Arrange
    let fixture = Fixture::external();
    fixture
        .owner
        .reconcile(
            [row(2, 1, None), row(5, 1, Some(2)), row(4, 3, None)].into_iter(),
            |_| true,
        )
        .unwrap();
    assert_eq!(fixture.drain().len(), 1);

    // Act
    fixture
        .owner
        .publish_frame_navigated(ObjectId::new(1), ObjectId::new(5))
        .unwrap();
    let notices = fixture.drain();

    // Assert
    assert_eq!(
        notices,
        vec![LifecycleNotice::PageFrameNavigated {
            context: ObjectId::new(1000),
            page: ObjectId::new(1),
            frame: ObjectId::new(5),
        }]
    );
}

#[cfg(feature = "frame-events")]
#[test]
fn same_document_main_navigation_notice_retains_root_identity() {
    use crate::LifecycleNotice;
    use crate::frame_trees::close_events::tests::Fixture;

    // Arrange
    let fixture = Fixture::external();

    // Act
    fixture
        .owner
        .publish_frame_navigated(ObjectId::new(1), ObjectId::new(2))
        .unwrap();
    let notices = fixture.drain();

    // Assert
    assert_eq!(
        notices,
        vec![LifecycleNotice::PageFrameNavigated {
            context: ObjectId::new(1000),
            page: ObjectId::new(1),
            frame: ObjectId::new(2),
        }]
    );
}

#[cfg(feature = "frame-events")]
#[test]
fn child_reattachment_uses_a_new_frame_identity() {
    use crate::LifecycleNotice;
    use crate::frame_trees::close_events::tests::Fixture;

    // Arrange
    let fixture = Fixture::external();
    let roots = [row(2, 1, None), row(4, 3, None)];
    fixture
        .owner
        .reconcile([roots[0], row(5, 1, Some(2)), roots[1]].into_iter(), |_| {
            true
        })
        .unwrap();
    let first = fixture.drain();
    fixture
        .owner
        .reconcile(roots.into_iter(), |_| true)
        .unwrap();
    assert!(fixture.drain().is_empty());

    // Act
    fixture
        .owner
        .reconcile([roots[0], row(6, 1, Some(2)), roots[1]].into_iter(), |_| {
            true
        })
        .unwrap();
    let second = fixture.drain();

    // Assert
    assert!(matches!(
        first.as_slice(),
        [LifecycleNotice::PageFrameAttached { frame, .. }] if *frame == ObjectId::new(5)
    ));
    assert!(matches!(
        second.as_slice(),
        [LifecycleNotice::PageFrameAttached { frame, .. }] if *frame == ObjectId::new(6)
    ));
}

#[cfg(feature = "frame-events")]
#[test]
fn hidden_page_attachment_does_not_emit_public_notice() {
    use crate::frame_trees::close_events::tests::Fixture;

    // Arrange
    let hidden = Fixture::ready();

    // Act
    hidden
        .owner
        .reconcile(
            [row(2, 1, None), row(5, 1, Some(2)), row(4, 3, None)].into_iter(),
            |_| true,
        )
        .unwrap();

    // Assert
    assert!(hidden.drain().is_empty());
}

#[cfg(feature = "frame-events")]
#[test]
fn absent_frame_navigation_does_not_emit_public_notice() {
    use crate::frame_trees::close_events::tests::Fixture;

    // Arrange
    let visible = Fixture::external();

    // Act
    visible
        .owner
        .publish_frame_navigated(ObjectId::new(1), ObjectId::new(99))
        .unwrap();

    // Assert
    assert!(visible.drain().is_empty());
}

#[cfg(feature = "frame-events")]
#[test]
fn retired_child_navigation_does_not_replay_notice() {
    use crate::frame_trees::close_events::tests::Fixture;

    // Arrange
    let fixture = Fixture::external();
    let roots = [row(2, 1, None), row(4, 3, None)];
    fixture
        .owner
        .reconcile([roots[0], row(5, 1, Some(2)), roots[1]].into_iter(), |_| {
            true
        })
        .unwrap();
    assert_eq!(fixture.drain().len(), 1);
    fixture
        .owner
        .reconcile(roots.into_iter(), |_| true)
        .unwrap();
    assert!(fixture.drain().is_empty());

    // Act
    fixture
        .owner
        .publish_frame_navigated(ObjectId::new(1), ObjectId::new(5))
        .unwrap();

    // Assert
    assert!(fixture.drain().is_empty());
}

#[cfg(feature = "frame-detach-events")]
#[test]
fn selected_detach_publication_is_child_first_and_binds_retired_identity() {
    use crate::frame_trees::close_events::tests::Fixture;
    use crate::{LifecycleDelivery, LifecycleNotice, PublicationDrainError, PublicationProgress};
    use pwright_backend_cdp_topology::CdpTopologyEffect;

    // Arrange
    let fixture = Fixture::external();
    let roots = [row(2, 1, None), row(4, 3, None)];
    let attached = [roots[0], row(5, 1, Some(2)), row(6, 1, Some(5)), roots[1]];
    fixture
        .owner
        .reconcile(attached.into_iter(), |_| true)
        .unwrap();
    fixture.book.fence().unwrap().publish().unwrap();
    let tree = fixture.owner.get(ObjectId::new(1)).unwrap();
    let nodes = tree.frames().unwrap();
    let child = nodes
        .iter()
        .find(|node| node.id() == ObjectId::new(5))
        .unwrap()
        .clone();
    let grandchild = nodes
        .iter()
        .find(|node| node.id() == ObjectId::new(6))
        .unwrap()
        .clone();
    let effects = [
        CdpTopologyEffect::FrameRemoved {
            page: ObjectId::new(1),
            frame: ObjectId::new(6),
        },
        CdpTopologyEffect::FrameRemoved {
            page: ObjectId::new(1),
            frame: ObjectId::new(5),
        },
    ];
    let admitted = fixture.owner.metrics().unwrap();

    // Act
    fixture
        .owner
        .reconcile_with_effects(roots.into_iter(), |_| true, &effects)
        .unwrap();
    let staged = fixture.owner.metrics().unwrap();
    let mut drain = fixture.book.fence().unwrap().try_begin().unwrap();

    // Assert
    assert_eq!(admitted.revisions - staged.revisions, 1);
    assert!(!child.is_detached());
    assert!(!grandchild.is_detached());
    assert_eq!(
        drain.publish_next().unwrap(),
        PublicationProgress::Published
    );
    assert!(child.is_detached());
    assert!(grandchild.is_detached());
    assert!(tree.bind_retired(5).unwrap().node().same_identity(&child));
    assert!(
        tree.bind_retired(6)
            .unwrap()
            .node()
            .same_identity(&grandchild)
    );
    for frame in [ObjectId::new(6), ObjectId::new(5)] {
        assert!(matches!(
            drain.publish_next(),
            Err(PublicationDrainError::NoticePending)
        ));
        assert_eq!(
            drain.commit_notice().unwrap(),
            LifecycleNotice::PageFrameDetached {
                context: ObjectId::new(1000),
                page: ObjectId::new(1),
                frame,
            }
        );
        drain
            .acknowledge_notice(LifecycleDelivery::Delivered)
            .unwrap();
    }
    assert_eq!(drain.publish_next().unwrap(), PublicationProgress::Complete);
    assert_eq!(
        fixture
            .owner
            .get(ObjectId::new(1))
            .unwrap()
            .frames()
            .unwrap()
            .len(),
        1
    );
}

#[cfg(feature = "frame-detach-events")]
#[test]
fn detach_uses_reserved_tree_queue_and_binding_at_resource_capacity() {
    use crate::LifecycleNotice;
    use crate::frame_trees::close_events::tests::Fixture;
    use pwright_backend_cdp_topology::CdpTopologyEffect;

    // Arrange
    let fixture = Fixture::external();
    let roots = [row(2, 1, None), row(4, 3, None)];
    fixture
        .owner
        .reconcile([roots[0], row(5, 1, Some(2)), roots[1]].into_iter(), |_| {
            true
        })
        .unwrap();
    fixture.book.fence().unwrap().publish().unwrap();
    let tree = fixture.owner.get(ObjectId::new(1)).unwrap();
    let child = tree.frames().unwrap()[1].clone();
    let mut held_bindings = Vec::new();
    while let Ok(binding) = fixture.owner.factory.reserve_binding() {
        held_bindings.push(binding);
    }
    assert_eq!(fixture.owner.metrics().unwrap().states, STATES as u64);
    let mut held_queue = Vec::new();
    while let Ok(reservation) = fixture.book.reserve_lifecycle(&fixture.owner.factory) {
        held_queue.push(reservation);
    }
    let mut held_revisions = Vec::new();
    while let Ok(revision) = fixture.owner.factory.reserve_detach_revision() {
        held_revisions.push(revision);
    }
    assert_eq!(
        fixture.owner.metrics().unwrap().revisions,
        super::REVISIONS as u64
    );

    // Act
    fixture
        .owner
        .reconcile_with_effects(
            roots.into_iter(),
            |_| true,
            &[CdpTopologyEffect::FrameRemoved {
                page: ObjectId::new(1),
                frame: ObjectId::new(5),
            }],
        )
        .unwrap();
    let mut drain = fixture.book.fence().unwrap().try_begin().unwrap();
    assert_eq!(
        drain.publish_next().unwrap(),
        crate::PublicationProgress::Published
    );
    let binding = tree.bind_retired(5).unwrap();
    let notice = drain.commit_notice();

    // Assert
    assert!(binding.node().same_identity(&child));
    assert_eq!(
        notice.unwrap(),
        LifecycleNotice::PageFrameDetached {
            context: ObjectId::new(1000),
            page: ObjectId::new(1),
            frame: ObjectId::new(5),
        }
    );
    drain
        .acknowledge_notice(crate::LifecycleDelivery::Delivered)
        .unwrap();
    assert_eq!(
        drain.publish_next().unwrap(),
        crate::PublicationProgress::Complete
    );
    drop((held_bindings, held_queue, held_revisions, binding));
}

#[cfg(feature = "frame-detach-events")]
#[test]
fn opt_in_capacity_accounts_for_all_live_child_detach_reservations() {
    use crate::frame_metadata::FrameMetadataLimits;

    // Arrange
    assert_eq!(STATES, 383);
    assert_eq!(super::REVISIONS, 510 + super::FRAME_EVENT_RESERVATIONS);
    let urls = Arc::new(
        FrameMetadata::new(FrameMetadataLimits {
            entries: 128,
            url_bytes: 64,
            total_bytes: 2 * 1024 * 1024,
            publications: 256 + 2 * 127,
        })
        .unwrap(),
    );
    let trees = FrameTrees::new(urls.clone()).unwrap();
    project(&trees, &[row(2, 1, None)]);
    urls.publications().fence().unwrap().publish().unwrap();
    let before = trees.metrics().unwrap();
    let rows = std::iter::once(row(2, 1, None)).chain((3..=129).map(|id| row(id, 1, Some(2))));

    // Act
    trees.reconcile(rows, |_| true).unwrap();
    urls.publications().fence().unwrap().publish().unwrap();
    let after = trees.metrics().unwrap();
    let mut remaining = Vec::new();
    while let Ok(reservation) = urls.publications().reserve_lifecycle(&trees.factory) {
        remaining.push(reservation);
    }

    // Assert
    assert_eq!(after.states - before.states, 2 * 127);
    assert_eq!(after.revisions - before.revisions, 2 * 127);
    assert_eq!(remaining.len(), 255);
    assert_eq!(trees.metrics().unwrap().revisions, super::REVISIONS as u64);
}

#[cfg(feature = "frame-detach-events")]
#[test]
fn hidden_page_detach_retires_frame_without_a_public_notice() {
    use crate::frame_trees::close_events::tests::Fixture;
    use pwright_backend_cdp_topology::CdpTopologyEffect;

    // Arrange
    let fixture = Fixture::ready();
    let roots = [row(2, 1, None), row(4, 3, None)];
    fixture
        .owner
        .reconcile([roots[0], row(5, 1, Some(2)), roots[1]].into_iter(), |_| {
            true
        })
        .unwrap();
    fixture.book.fence().unwrap().publish().unwrap();
    let child = fixture
        .owner
        .get(ObjectId::new(1))
        .unwrap()
        .frames()
        .unwrap()[1]
        .clone();

    // Act
    fixture
        .owner
        .reconcile_with_effects(
            roots.into_iter(),
            |_| true,
            &[CdpTopologyEffect::FrameRemoved {
                page: ObjectId::new(1),
                frame: ObjectId::new(5),
            }],
        )
        .unwrap();

    // Assert
    assert!(fixture.drain().is_empty());
    assert!(child.is_detached());
    assert!(matches!(
        fixture.owner.get(ObjectId::new(1)).unwrap().bind_retired(5),
        Err(HostError::TargetClosed)
    ));
}

fn staged_page_admission_fixture(hidden_public_page: bool) -> (Arc<FrameMetadata>, FrameTrees) {
    let (urls, trees) = fixture();
    project(&trees, &[row(4, 3, None), row(7, 6, None)]);
    trees
        .state
        .lock()
        .unwrap()
        .pages
        .iter_mut()
        .find(|entry| entry.id == ObjectId::new(3))
        .unwrap()
        .internal = true;
    let contexts = ContextMetadata::new(ContextMetadataLimits {
        active_contexts: 4,
        states: 8,
        conversions: 4,
        base_url_bytes: 64,
        bytes: 4096,
    })
    .unwrap();
    let snapshot = contexts
        .reserve_creation(None)
        .unwrap()
        .commit(ObjectId::new(5))
        .unwrap()
        .snapshot;
    let visible_tree = trees.get(ObjectId::new(6)).unwrap();
    let mut reserved = urls
        .publications()
        .reserve_collection(visible_tree.lookup_resources())
        .unwrap();
    reserved.add_context(ObjectId::new(5), snapshot).unwrap();
    for page in [ObjectId::new(3), ObjectId::new(6)] {
        reserved
            .add_page(page, ObjectId::new(5), trees.get(page).unwrap())
            .unwrap();
    }
    if hidden_public_page {
        urls.publications()
            .begin_collection(reserved)
            .unwrap()
            .commit_view(|revision| {
                CollectionView::full(revision.clone())
                    .with_visibility(CollectionMember::Page(ObjectId::new(6)), false)
                    .map_err(HostError::backend)
            })
            .unwrap();
    } else {
        urls.publications().stage_collection(reserved).unwrap();
    }
    (urls, trees)
}

#[test]
fn indexed_page_is_not_public_before_collection_publication() {
    // Arrange
    let (_urls, trees) = staged_page_admission_fixture(false);

    // Act
    let visible = trees.is_published_public_page(ObjectId::new(6)).unwrap();

    // Assert
    assert!(!visible);
}

#[test]
fn internal_page_is_not_public_even_if_a_projection_exposes_its_id() {
    // Arrange
    let (urls, trees) = staged_page_admission_fixture(false);
    urls.publications().fence().unwrap().publish().unwrap();

    // Act
    let visible = trees.is_published_public_page(ObjectId::new(3)).unwrap();

    // Assert
    assert!(!visible);
}

#[test]
fn published_non_internal_page_is_public() {
    // Arrange
    let (urls, trees) = staged_page_admission_fixture(false);
    urls.publications().fence().unwrap().publish().unwrap();

    // Act
    let visible = trees.is_published_public_page(ObjectId::new(6)).unwrap();

    // Assert
    assert!(visible);
}

#[test]
fn hidden_page_in_published_projection_is_not_public() {
    // Arrange
    let (urls, trees) = staged_page_admission_fixture(true);
    urls.publications().fence().unwrap().publish().unwrap();

    // Act
    let visible = trees.is_published_public_page(ObjectId::new(6)).unwrap();

    // Assert
    assert!(!visible);
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
        .activate_page(reservation, ObjectId::new(1), ObjectId::new(2), false)
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
    let held: Vec<_> = (0..STATES - 3)
        .map(|_| trees.factory.reserve_index(1).unwrap())
        .collect();
    let before = trees.metrics().unwrap();
    assert_eq!(before.states, (STATES - 2) as u64);
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
    let _held: Vec<_> = (0..STATES - 5)
        .map(|_| trees.factory.reserve_index(1).unwrap())
        .collect();
    let reservation = trees
        .reserve_page(urls.reserve_new_page().unwrap())
        .unwrap();
    let before = trees.metrics().unwrap();
    assert_eq!(before.states, STATES as u64);
    // Act
    let (tree, binding, _tree_binding) = trees
        .activate_page(reservation, ObjectId::new(1), ObjectId::new(2), false)
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
    let _held: Vec<_> = (0..STATES - 4)
        .map(|_| trees.factory.reserve_index(1).unwrap())
        .collect();
    let before = trees.metrics().unwrap();
    assert_eq!(before.states, (STATES - 3) as u64);
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
