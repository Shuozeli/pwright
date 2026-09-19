use super::*;
use crate::context_metadata::{ContextMetadata, ContextMetadataLimits, ContextSnapshot};
use crate::frame_metadata::FrameMetadata;
use crate::frame_metadata::collections::tests::{fixture, populated};
use crate::frame_tree::FrameTree;
use pwright_core::ObjectId;
use std::sync::Arc;

fn revision(
    book: &Arc<FrameMetadata>,
    tree: &FrameTree,
    settings: &ContextSnapshot,
    contexts: &[u64],
    pages: &[(u64, u64, FrameTree)],
) -> CollectionRevision {
    let mut reserved = book
        .publications()
        .reserve_collection(tree.lookup_resources())
        .unwrap();
    for id in contexts {
        reserved
            .add_context(ObjectId::new(*id), settings.clone())
            .unwrap();
    }
    for (id, context, page) in pages {
        reserved
            .add_page(ObjectId::new(*id), ObjectId::new(*context), page.clone())
            .unwrap();
    }
    reserved.revision
}

fn page(book: &Arc<FrameMetadata>, tree: &FrameTree, id: u64) -> FrameTree {
    let root = tree
        .lookup_resources()
        .frame(
            ObjectId::new(id + 10000),
            None,
            book.get(ObjectId::new(2)).unwrap(),
        )
        .unwrap();
    tree.lookup_resources().page(root).unwrap()
}

fn context_metadata() -> ContextMetadata {
    ContextMetadata::new(ContextMetadataLimits {
        active_contexts: 16,
        states: 256,
        conversions: 128,
        base_url_bytes: 64,
        bytes: 8192,
    })
    .unwrap()
}

#[test]
fn separately_admitted_equal_settings_in_one_ledger_cannot_replace_each_other() {
    // Arrange
    let (book, _trees, tree, _settings) = fixture(8);
    let metadata = context_metadata();
    let first = metadata
        .reserve_creation(Some("https://example.test/"))
        .unwrap()
        .commit(ObjectId::new(1000))
        .unwrap();
    let second = metadata
        .reserve_creation(Some("https://example.test/"))
        .unwrap()
        .commit(ObjectId::new(2000))
        .unwrap();
    assert_eq!(first.snapshot, second.snapshot);
    let source = CollectionView::hidden(revision(&book, &tree, &first.snapshot, &[1000], &[]));
    let target = revision(&book, &tree, &second.snapshot, &[1000], &[]);
    // Act
    let result = source.try_rebase(&target);
    // Assert
    assert!(matches!(
        result,
        Err(CollectionRebaseError::IdentityChanged(
            CollectionMember::Context(_)
        ))
    ));
}

#[test]
fn shared_unconfigured_borrowed_settings_can_rebase() {
    // Arrange
    let (book, _trees, tree, _settings) = fixture(8);
    let metadata = context_metadata();
    let first = metadata
        .visible(ObjectId::new(1000), false)
        .unwrap()
        .unwrap();
    let second = metadata
        .visible(ObjectId::new(1000), false)
        .unwrap()
        .unwrap();
    let source = CollectionView::full(revision(&book, &tree, &first, &[1000], &[]));
    let target = revision(&book, &tree, &second, &[1000], &[]);
    // Act
    let next = source.try_rebase(&target).unwrap();
    // Assert
    assert_eq!(next.contexts().len(), 1);
    assert_eq!(next.contexts()[0].settings.base_url(), None);
}

#[test]
fn unconfigured_settings_cannot_become_configured_during_rebase() {
    // Arrange
    let (book, _trees, tree, _settings) = fixture(8);
    let metadata = context_metadata();
    let empty = metadata
        .visible(ObjectId::new(1000), false)
        .unwrap()
        .unwrap();
    let configured = metadata
        .reserve_creation(Some("https://example.test/"))
        .unwrap()
        .commit(ObjectId::new(1000))
        .unwrap();
    let source = CollectionView::hidden(revision(&book, &tree, &empty, &[1000], &[]));
    let target = revision(&book, &tree, &configured.snapshot, &[1000], &[]);
    // Act
    let result = source.try_rebase(&target);
    // Assert
    assert!(matches!(
        result,
        Err(CollectionRebaseError::IdentityChanged(
            CollectionMember::Context(_)
        ))
    ));
}

#[test]
fn configured_settings_cannot_become_unconfigured_during_rebase() {
    // Arrange
    let (book, _trees, tree, _settings) = fixture(8);
    let metadata = context_metadata();
    let empty = metadata
        .visible(ObjectId::new(1000), false)
        .unwrap()
        .unwrap();
    let configured = metadata
        .reserve_creation(Some("https://example.test/"))
        .unwrap()
        .commit(ObjectId::new(1000))
        .unwrap();
    let source = CollectionView::hidden(revision(&book, &tree, &configured.snapshot, &[1000], &[]));
    let target = revision(&book, &tree, &empty, &[1000], &[]);
    // Act
    let result = source.try_rebase(&target);
    // Assert
    assert!(matches!(
        result,
        Err(CollectionRebaseError::IdentityChanged(
            CollectionMember::Context(_)
        ))
    ));
}

#[test]
fn rebasing_to_the_same_revision_preserves_a_partial_view() {
    // Arrange
    let (book, _trees, tree, settings) = fixture(8);
    let source = CollectionView::full(populated(&book, &tree, settings).revision)
        .with_visibility(CollectionMember::Page(ObjectId::new(1)), false)
        .unwrap();
    // Act
    let next = source.try_rebase(&source.revision).unwrap();
    // Assert
    assert_eq!(next.contexts().len(), 1);
    assert!(next.pages().is_empty());
    assert!(source.pages().is_empty());
}

#[test]
fn additions_stay_hidden_and_surviving_page_identity_is_retained() {
    // Arrange
    let (book, _trees, tree, settings) = fixture(8);
    let source = CollectionView::full(populated(&book, &tree, settings.clone()).revision);
    let added = page(&book, &tree, 3);
    let target = revision(
        &book,
        &tree,
        &settings,
        &[1000, 2000],
        &[(1, 1000, tree.clone()), (3, 2000, added)],
    );
    let budget = tree.lookup_resources().metrics().unwrap();
    // Act
    let next = source.try_rebase(&target).unwrap();
    // Assert
    assert_eq!(next.contexts().len(), 1);
    assert_eq!(next.pages().len(), 1);
    assert!(
        next.pages()[0]
            .main_frame
            .same_identity(&source.pages()[0].main_frame)
    );
    assert_eq!(tree.lookup_resources().metrics().unwrap(), budget);
    assert!(matches!(
        next.with_visibility(CollectionMember::Page(ObjectId::new(3)), true),
        Err(super::super::super::CollectionVisibilityError::ParentNotVisible(_))
    ));
    assert_eq!(source.pages().len(), 1);
}

#[test]
fn visible_page_cannot_disappear_at_a_revision_boundary() {
    // Arrange
    let (book, _trees, tree, settings) = fixture(8);
    let source = CollectionView::full(populated(&book, &tree, settings.clone()).revision);
    let target = revision(&book, &tree, &settings, &[1000], &[]);
    // Act
    let result = source.try_rebase(&target);
    // Assert
    assert!(
        matches!(result, Err(CollectionRebaseError::VisibleMemberMissing(
        CollectionMember::Page(id))) if id == ObjectId::new(1))
    );
    assert_eq!(source.pages().len(), 1);
}

#[test]
fn visible_context_cannot_disappear_at_a_revision_boundary() {
    // Arrange
    let (book, _trees, tree, settings) = fixture(8);
    let source = CollectionView::full(revision(&book, &tree, &settings, &[1000], &[]));
    let target = revision(&book, &tree, &settings, &[], &[]);
    // Act
    let result = source.try_rebase(&target);
    // Assert
    assert!(
        matches!(result, Err(CollectionRebaseError::VisibleMemberMissing(
        CollectionMember::Context(id))) if id == ObjectId::new(1000))
    );
}

#[test]
fn retired_page_can_be_dropped_without_changing_an_older_reader() {
    // Arrange
    let (book, _trees, tree, settings) = fixture(8);
    let retained = CollectionView::full(populated(&book, &tree, settings.clone()).revision);
    let source = retained
        .with_visibility(CollectionMember::Page(ObjectId::new(1)), false)
        .unwrap();
    let target = revision(&book, &tree, &settings, &[1000], &[]);
    // Act
    let next = source.try_rebase(&target).unwrap();
    // Assert
    assert!(next.pages().is_empty());
    assert_eq!(next.contexts().len(), 1);
    assert_eq!(retained.pages().len(), 1);
    assert!(!retained.pages()[0].tree.is_closed().unwrap());
}

#[test]
fn visible_context_order_cannot_change() {
    // Arrange
    let (book, _trees, tree, settings) = fixture(8);
    let source = CollectionView::full(revision(&book, &tree, &settings, &[1000, 2000], &[]));
    let target = revision(&book, &tree, &settings, &[2000, 1000], &[]);
    // Act
    let result = source.try_rebase(&target);
    // Assert
    assert!(matches!(
        result,
        Err(CollectionRebaseError::VisibleOrderChanged(
            CollectionMember::Context(_)
        ))
    ));
    assert_eq!(source.contexts()[0].id, ObjectId::new(1000));
}

#[test]
fn visible_page_order_cannot_change() {
    // Arrange
    let (book, _trees, tree, settings) = fixture(8);
    let second = page(&book, &tree, 3);
    let source = CollectionView::full(revision(
        &book,
        &tree,
        &settings,
        &[1000],
        &[(1, 1000, tree.clone()), (3, 1000, second.clone())],
    ));
    let target = revision(
        &book,
        &tree,
        &settings,
        &[1000],
        &[(3, 1000, second), (1, 1000, tree.clone())],
    );
    // Act
    let result = source.try_rebase(&target);
    // Assert
    assert!(matches!(
        result,
        Err(CollectionRebaseError::VisibleOrderChanged(
            CollectionMember::Page(_)
        ))
    ));
    assert_eq!(source.pages()[0].id, ObjectId::new(1));
}

#[test]
fn hidden_members_do_not_constrain_visible_order() {
    // Arrange
    let (book, _trees, tree, settings) = fixture(8);
    let source = CollectionView::full(revision(&book, &tree, &settings, &[1000, 2000], &[]))
        .with_visibility(CollectionMember::Context(ObjectId::new(1000)), false)
        .unwrap();
    let target = revision(&book, &tree, &settings, &[2000, 1000], &[]);
    // Act
    let next = source.try_rebase(&target).unwrap();
    // Assert
    assert_eq!(next.contexts().len(), 1);
    assert_eq!(next.contexts()[0].id, ObjectId::new(2000));
}

#[test]
fn foreign_empty_revision_is_rejected_even_with_no_visible_members() {
    // Arrange
    let (book, _trees, tree, settings) = fixture(8);
    let (foreign, _other_trees, other_tree, other_settings) = fixture(8);
    let source = CollectionView::hidden(revision(&book, &tree, &settings, &[], &[]));
    let target = revision(&foreign, &other_tree, &other_settings, &[], &[]);
    // Act
    let result = source.try_rebase(&target);
    // Assert
    assert!(matches!(
        result,
        Err(CollectionRebaseError::ForeignRevision)
    ));
}

#[test]
fn the_same_numeric_page_id_cannot_replace_its_main_frame() {
    // Arrange
    let (book, _trees, tree, settings) = fixture(8);
    let source = CollectionView::hidden(populated(&book, &tree, settings.clone()).revision);
    let target = revision(
        &book,
        &tree,
        &settings,
        &[1000],
        &[(1, 1000, page(&book, &tree, 1))],
    );
    // Act
    let result = source.try_rebase(&target);
    // Assert
    assert!(matches!(
        result,
        Err(CollectionRebaseError::IdentityChanged(
            CollectionMember::Page(_)
        ))
    ));
}

#[test]
fn an_existing_page_cannot_move_to_a_different_context() {
    // Arrange
    let (book, _trees, tree, settings) = fixture(8);
    let source = CollectionView::hidden(populated(&book, &tree, settings.clone()).revision);
    let target = revision(&book, &tree, &settings, &[2000], &[(1, 2000, tree.clone())]);
    // Act
    let result = source.try_rebase(&target);
    // Assert
    assert!(matches!(
        result,
        Err(CollectionRebaseError::IdentityChanged(
            CollectionMember::Page(_)
        ))
    ));
}

#[test]
fn equal_context_settings_values_do_not_prove_shared_metadata() {
    // Arrange
    let (book, _trees, tree, settings) = fixture(8);
    let (_other_book, _other_trees, _other_tree, other_settings) = fixture(8);
    assert_eq!(settings, other_settings);
    let source = CollectionView::hidden(populated(&book, &tree, settings).revision);
    let target = revision(&book, &tree, &other_settings, &[1000], &[]);
    // Act
    let result = source.try_rebase(&target);
    // Assert
    assert!(matches!(
        result,
        Err(CollectionRebaseError::IdentityChanged(
            CollectionMember::Context(_)
        ))
    ));
}

#[test]
fn an_old_page_id_cannot_be_reused_as_a_context() {
    // Arrange
    let (book, _trees, tree, settings) = fixture(8);
    let source = CollectionView::hidden(populated(&book, &tree, settings.clone()).revision);
    let target = revision(&book, &tree, &settings, &[1], &[]);
    // Act
    let result = source.try_rebase(&target);
    // Assert
    assert!(matches!(
        result,
        Err(CollectionRebaseError::IdentityChanged(
            CollectionMember::Page(_)
        ))
    ));
}

#[test]
fn an_old_context_id_cannot_be_reused_as_a_page() {
    // Arrange
    let (book, _trees, tree, settings) = fixture(8);
    let source = CollectionView::hidden(populated(&book, &tree, settings.clone()).revision);
    let target = revision(
        &book,
        &tree,
        &settings,
        &[2000],
        &[(1000, 2000, page(&book, &tree, 1000))],
    );
    // Act
    let result = source.try_rebase(&target);
    // Assert
    assert!(matches!(
        result,
        Err(CollectionRebaseError::IdentityChanged(
            CollectionMember::Context(_)
        ))
    ));
}

#[test]
fn sixteen_disjoint_replacements_need_no_union_revision_or_new_admission() {
    // Arrange
    let (book, _trees, tree, settings) = fixture(8);
    let old_pages: Vec<_> = (1..=16)
        .map(|id| (id, 1000, page(&book, &tree, id)))
        .collect();
    let new_pages: Vec<_> = (101..=116)
        .map(|id| (id, 2000, page(&book, &tree, id)))
        .collect();
    let retained = CollectionView::full(revision(&book, &tree, &settings, &[1000], &old_pages));
    let mut source = retained.clone();
    for id in 1..=16 {
        source = source
            .with_visibility(CollectionMember::Page(ObjectId::new(id)), false)
            .unwrap();
    }
    source = source
        .with_visibility(CollectionMember::Context(ObjectId::new(1000)), false)
        .unwrap();
    let target = revision(&book, &tree, &settings, &[2000], &new_pages);
    let before = tree.lookup_resources().metrics().unwrap();
    // Act
    let next = source.try_rebase(&target).unwrap();
    // Assert
    assert_eq!(tree.lookup_resources().metrics().unwrap(), before);
    assert!(next.contexts().is_empty());
    assert!(next.pages().is_empty());
    assert_eq!(retained.pages().len(), 16);
    assert_eq!(next.revision.pages().len(), 16);
}

#[test]
fn retained_views_release_both_revisions_without_publication_queue_ownership() {
    // Arrange
    let (source, target, factory, weak_book) = {
        let (book, _trees, tree, settings) = fixture(8);
        let source = CollectionView::hidden(populated(&book, &tree, settings.clone()).revision);
        let target = revision(&book, &tree, &settings, &[], &[]);
        (
            source,
            target,
            tree.lookup_resources().clone(),
            Arc::downgrade(&book),
        )
    };
    assert!(weak_book.upgrade().is_none());
    let next = source.try_rebase(&target).unwrap();
    drop(target);
    drop(source);
    assert_eq!(factory.metrics().unwrap().states, 0);
    assert_eq!(factory.metrics().unwrap().revisions, 1);
    // Act
    drop(next);
    // Assert
    assert_eq!(factory.metrics().unwrap().revisions, 0);
    assert_eq!(factory.metrics().unwrap().bytes, 0);
}
