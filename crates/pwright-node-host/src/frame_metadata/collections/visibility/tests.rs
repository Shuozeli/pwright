use super::super::tests::{fixture, populated};
use super::*;

fn view(pages: usize, contexts: usize) -> CollectionView {
    let (book, _trees, tree, context) = fixture(8);
    let mut reserved = populated(&book, &tree, context.clone());
    for index in 1..contexts {
        reserved
            .add_context(ObjectId::new(1000 + index as u64), context.clone())
            .unwrap();
    }
    for index in 1..pages {
        let root = tree
            .lookup_resources()
            .frame(
                ObjectId::new(100 + index as u64),
                None,
                book.get(ObjectId::new(2)).unwrap(),
            )
            .unwrap();
        let page = tree.lookup_resources().page(root).unwrap();
        reserved
            .add_page(ObjectId::new(9 + index as u64), ObjectId::new(1000), page)
            .unwrap();
    }
    CollectionView::full(reserved.revision)
}

#[test]
fn a_new_visibility_step_preserves_old_membership_and_object_identity() {
    // Arrange
    let original = view(2, 1);
    let retained = original.clone();
    // Act
    let next = original
        .with_visibility(CollectionMember::Page(ObjectId::new(1)), false)
        .unwrap();
    // Assert
    assert_eq!(original.pages().len(), 2);
    assert_eq!(retained.pages().len(), 2);
    assert_eq!(next.pages().len(), 1);
    assert_eq!(next.pages()[0].id, ObjectId::new(10));
    assert!(std::ptr::eq(&next.pages()[0], &original.pages()[1]));
    assert!(
        !original.pages()[0].tree.is_closed().unwrap(),
        "visibility does not close native/public metadata"
    );
}

#[test]
fn a_context_with_visible_pages_cannot_be_hidden() {
    // Arrange
    let original = view(1, 1);
    // Act
    let result = original.with_visibility(CollectionMember::Context(ObjectId::new(1000)), false);
    // Assert
    assert!(
        matches!(result, Err(CollectionVisibilityError::ContextHasVisiblePages(id)) if id == ObjectId::new(1000))
    );
    assert_eq!(original.contexts().len(), 1);
    assert_eq!(original.pages().len(), 1);
}

#[test]
fn page_removals_precede_context_removal_without_mutating_prior_steps() {
    // Arrange
    let full = view(2, 1);
    let one = full
        .with_visibility(CollectionMember::Page(ObjectId::new(1)), false)
        .unwrap();
    let empty = one
        .with_visibility(CollectionMember::Page(ObjectId::new(10)), false)
        .unwrap();
    // Act
    let removed = empty
        .with_visibility(CollectionMember::Context(ObjectId::new(1000)), false)
        .unwrap();
    // Assert
    assert_eq!(full.pages().len(), 2);
    assert_eq!(one.pages().len(), 1);
    assert!(empty.pages().is_empty());
    assert_eq!(empty.contexts().len(), 1);
    assert!(removed.contexts().is_empty());
    assert!(removed.pages().is_empty());
}

#[test]
fn a_hidden_parent_prevents_page_visibility() {
    // Arrange
    let hidden = CollectionView::hidden(view(1, 1).revision);
    // Act
    let result = hidden.with_visibility(CollectionMember::Page(ObjectId::new(1)), true);
    // Assert
    assert!(
        matches!(result, Err(CollectionVisibilityError::ParentNotVisible(id)) if id == ObjectId::new(1000))
    );
    assert!(hidden.contexts().is_empty());
    assert!(hidden.pages().is_empty());
}

#[test]
fn context_visibility_can_precede_page_visibility() {
    // Arrange
    let hidden = CollectionView::hidden(view(1, 1).revision);
    let context = hidden
        .with_visibility(CollectionMember::Context(ObjectId::new(1000)), true)
        .unwrap();
    // Act
    let page = context
        .with_visibility(CollectionMember::Page(ObjectId::new(1)), true)
        .unwrap();
    // Assert
    assert!(hidden.contexts().is_empty());
    assert_eq!(context.contexts().len(), 1);
    assert!(context.pages().is_empty());
    assert_eq!(page.pages()[0].context, page.contexts()[0].id);
}

#[test]
fn unknown_members_fail_without_changing_the_view_or_its_budget() {
    // Arrange
    let original = view(1, 1);
    let factory = original.pages()[0].tree.lookup_resources().clone();
    let before = factory.metrics().unwrap();
    let unknown = CollectionMember::Page(ObjectId::new(999));
    // Act
    let result = original.with_visibility(unknown, false);
    // Assert
    assert!(matches!(result, Err(CollectionVisibilityError::Unknown(member)) if member == unknown));
    assert_eq!(original.pages().len(), 1);
    assert_eq!(factory.metrics().unwrap(), before);
}

#[test]
fn a_second_show_rejects_duplicate_transition() {
    // Arrange
    let original = view(1, 1);
    let page = CollectionMember::Page(ObjectId::new(1));
    // Act
    let result = original.with_visibility(page, true);
    // Assert
    assert!(
        matches!(result, Err(CollectionVisibilityError::AlreadyVisible(member)) if member == page)
    );
    assert_eq!(original.pages().len(), 1);
}

#[test]
fn a_second_hide_rejects_duplicate_transition() {
    // Arrange
    let page = CollectionMember::Page(ObjectId::new(1));
    let hidden = view(1, 1).with_visibility(page, false).unwrap();
    // Act
    let result = hidden.with_visibility(page, false);
    // Assert
    assert!(matches!(result, Err(CollectionVisibilityError::NotVisible(member)) if member == page));
    assert!(hidden.pages().is_empty());
}

#[test]
fn all_sixteen_bits_preserve_order_and_exact_bidirectional_iteration() {
    // Arrange
    let full = view(16, 16);
    let masked = full
        .with_visibility(CollectionMember::Page(ObjectId::new(10)), false)
        .unwrap();
    let mut entries = masked.pages().into_iter();
    // Act
    let first = entries.next().unwrap().id;
    let last = entries.next_back().unwrap().id;
    let middle: Vec<_> = entries.by_ref().map(|page| page.id).collect();
    // Assert
    assert_eq!(full.contexts().len(), 16);
    assert_eq!(full.pages().len(), 16);
    assert_eq!(masked.pages().len(), 15);
    assert_eq!(first, ObjectId::new(1));
    assert_eq!(last, ObjectId::new(24));
    assert_eq!(middle, (11..24).map(ObjectId::new).collect::<Vec<_>>());
    assert_eq!(entries.len(), 0);
    assert_eq!(entries.size_hint(), (0, Some(0)));
    assert!(entries.next().is_none());
    assert!(entries.next_back().is_none());
    assert!(masked.pages().get(15).is_none());
}

#[test]
fn an_empty_revision_has_no_shift_overflow_or_visible_members() {
    // Arrange
    let (book, _trees, tree, _context) = fixture(8);
    let reserved = book
        .publications()
        .reserve_collection(tree.lookup_resources())
        .unwrap();
    // Act
    let empty = CollectionView::full(reserved.revision);
    // Assert
    assert!(empty.contexts().is_empty());
    assert!(empty.pages().is_empty());
    assert_eq!(empty.contexts().iter().len(), 0);
    assert!(empty.pages().get(0).is_none());
}

#[test]
fn hidden_metadata_remains_charged_until_the_last_view_drops() {
    // Arrange
    let original = view(1, 1);
    let factory = original.pages()[0].tree.lookup_resources().clone();
    let before = factory.metrics().unwrap();
    let hidden = CollectionView::hidden(original.revision.clone());
    assert_eq!(factory.metrics().unwrap(), before);
    drop(original);
    assert_eq!(factory.metrics().unwrap(), before);
    // Act
    drop(hidden);
    // Assert
    let final_metrics = factory.metrics().unwrap();
    assert_eq!(final_metrics.states, 0);
    assert_eq!(final_metrics.revisions, 0);
    assert_eq!(final_metrics.bytes, 0);
}

#[test]
fn successful_steps_allocate_no_additional_logical_storage() {
    // Arrange
    let original = view(1, 2);
    let factory = original.pages()[0].tree.lookup_resources().clone();
    let before = factory.metrics().unwrap();
    // Act
    let next = original
        .with_visibility(CollectionMember::Context(ObjectId::new(1001)), false)
        .unwrap();
    // Assert
    assert_eq!(next.contexts().len(), 1);
    assert_eq!(factory.metrics().unwrap(), before);
    println!(
        "Collection view: bytes={}, member_bytes={}, iterator_bytes={}",
        size_of::<CollectionView>(),
        size_of::<CollectionMembers<'_, CollectionPage>>(),
        size_of::<VisibleMembers<'_, CollectionPage>>()
    );
}

#[test]
fn the_sixteenth_context_can_be_hidden_and_restored_in_place() {
    // Arrange
    let original = view(1, 16);
    let last = CollectionMember::Context(ObjectId::new(1015));
    let hidden = original.with_visibility(last, false).unwrap();
    // Act
    let restored = hidden.with_visibility(last, true).unwrap();
    // Assert
    assert_eq!(hidden.contexts().len(), 15);
    assert_eq!(hidden.contexts()[14].id, ObjectId::new(1014));
    assert_eq!(restored.contexts().len(), 16);
    assert_eq!(restored.contexts()[15].id, ObjectId::new(1015));
    assert!(std::ptr::eq(
        &restored.contexts()[15],
        &original.contexts()[15]
    ));
}

#[test]
fn an_unknown_context_cannot_be_made_visible() {
    // Arrange
    let original = view(1, 1);
    let unknown = CollectionMember::Context(ObjectId::new(0));
    // Act
    let result = original.with_visibility(unknown, true);
    // Assert
    assert!(matches!(result, Err(CollectionVisibilityError::Unknown(member)) if member == unknown));
    assert_eq!(original.contexts().len(), 1);
    assert_eq!(original.pages().len(), 1);
}
