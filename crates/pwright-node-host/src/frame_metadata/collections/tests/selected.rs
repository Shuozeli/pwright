use super::*;
use crate::{LifecycleCause, LifecycleDelivery, LifecycleNotice};

#[test]
fn masked_batch_retains_descriptors_without_publishing_page_membership() {
    // Arrange
    let (book, _trees, tree, context) = fixture(8);
    let reserved = populated(&book, &tree, context);
    // Act
    let view = book
        .publications()
        .begin_collection(reserved)
        .unwrap()
        .commit_view(|revision| {
            assert!(book.state.try_lock().is_ok());
            CollectionView::hidden(revision.clone())
                .with_visibility(CollectionMember::Context(ObjectId::new(1000)), true)
                .map_err(HostError::backend)
        })
        .unwrap();
    book.publications().fence().unwrap().publish().unwrap();
    // Assert
    let visible = book
        .publications()
        .published_collections()
        .unwrap()
        .unwrap();
    assert!(visible.same_state(&view));
    assert_eq!(visible.contexts().len(), 1);
    assert_eq!(visible.pages().len(), 0);
    assert_eq!(visible.revision().pages().len(), 1);
}

#[test]
fn masked_batch_is_a_valid_exact_predecessor_for_page_ready() {
    // Arrange
    let (book, _trees, tree, context) = fixture(8);
    let view = book
        .publications()
        .begin_collection(populated(&book, &tree, context))
        .unwrap()
        .commit_view(|revision| Ok(CollectionView::hidden(revision.clone())))
        .unwrap();
    book.publications().fence().unwrap().publish().unwrap();
    let group = book
        .publications()
        .reserve_lifecycle(tree.lookup_resources())
        .unwrap();
    book.publications()
        .stage_lifecycle(
            group,
            view.clone(),
            LifecycleCause::PageReady {
                revision: view.revision().clone(),
                page: ObjectId::new(1),
                opener: None,
            },
        )
        .unwrap();
    let mut drain = book.publications().fence().unwrap().try_begin().unwrap();
    // Act
    let notice = drain.commit_notice().unwrap();
    drain
        .acknowledge_notice(LifecycleDelivery::Delivered)
        .unwrap();
    // Assert
    assert_eq!(
        notice,
        LifecycleNotice::ContextPage {
            context: ObjectId::new(1000),
            page: ObjectId::new(1)
        }
    );
    assert_eq!(
        book.publications()
            .published_collections()
            .unwrap()
            .unwrap()
            .pages()
            .len(),
        1
    );
    assert!(view.pages().is_empty());
}

#[test]
fn batch_rejects_another_revision_from_the_same_resource_domain() {
    // Arrange
    let (book, _trees, tree, context) = fixture(8);
    let other = populated(&book, &tree, context.clone()).into_revision();
    let batch = book
        .publications()
        .begin_collection(populated(&book, &tree, context))
        .unwrap();
    // Act
    let result = batch.commit_view(|_| Ok(CollectionView::full(other)));
    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert!(
        book.publications()
            .published_collections()
            .unwrap()
            .is_none()
    );
    assert!(book.state.lock().unwrap().collection_batch.is_some());
}

#[test]
fn failed_view_selection_preserves_the_batch_gate_and_previous_public_state() {
    // Arrange
    let (book, _trees, tree, context) = fixture(8);
    let batch = book
        .publications()
        .begin_collection(populated(&book, &tree, context))
        .unwrap();
    book.update(ObjectId::new(2), "unpublished", None).unwrap();
    // Act
    let result = batch.commit_view(|_| Err(HostError::ResourceExhausted("selected view".into())));
    book.publications().fence().unwrap().publish().unwrap();
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(tree.bind(2).unwrap().node().url().unwrap(), "kept");
    assert!(
        book.publications()
            .published_collections()
            .unwrap()
            .is_none()
    );
}
