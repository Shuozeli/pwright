use super::*;
use crate::{
    context_metadata::ContextMetadataLimits,
    frame_metadata::{FrameMetadata, FrameMetadataLimits},
};

fn fixture(publications: usize) -> (Arc<FrameMetadata>, FrameTrees, ContextMetadata) {
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
    let contexts = ContextMetadata::new(ContextMetadataLimits {
        active_contexts: 16,
        states: 256,
        conversions: 128,
        base_url_bytes: 64,
        bytes: 8192,
    })
    .unwrap();
    (book, trees, contexts)
}

#[test]
fn selected_retirement_pool_is_preallocated_and_reclaimed_on_native_clear() {
    // Arrange
    let (book, trees, contexts) = fixture(64);
    let factory = trees.factory.clone();
    let before = factory.metrics().unwrap();
    let trees = trees.with_collections(contexts).unwrap();
    let selected = factory.metrics().unwrap();
    let reserved = trees.reserve_creation_collection().unwrap();
    reserved.commit().unwrap();
    // Act
    trees.clear();
    // Assert
    assert_eq!(
        selected.revisions,
        before.revisions + RETIREMENTS as u64 + 1
    );
    assert_eq!(factory.metrics().unwrap(), before);
    assert!(
        book.publications()
            .published_collections()
            .unwrap()
            .is_none()
    );
    println!(
        "Selected collection pool: bytes={}, revisions={}",
        selected.bytes - before.bytes,
        selected.revisions - before.revisions
    );
}

#[test]
fn selection_failure_releases_partial_retirement_admission() {
    // Arrange
    let (book, trees, contexts) = fixture(16);
    let factory = trees.factory.clone();
    let before = factory.metrics().unwrap();
    // Act
    let result = trees.with_collections(contexts);
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(factory.metrics().unwrap(), before);
    assert_eq!(book.metrics().unwrap().pending_publications, 0);
}

#[test]
fn creation_cannot_proceed_until_retirement_pool_can_be_replenished() {
    // Arrange
    let (book, trees, contexts) = fixture(33);
    let trees = trees.with_collections(contexts).unwrap();
    {
        let mut state = trees.state.lock().unwrap();
        let collection = state.collections.as_mut().unwrap();
        while let Some(reserved) = collection.retirements.pop() {
            book.publications().stage_collection(reserved).unwrap();
        }
    }
    let before = trees.factory.metrics().unwrap();
    // Act
    let result = trees.reserve_creation_collection();
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(trees.factory.metrics().unwrap(), before);
    assert_eq!(
        trees
            .state
            .lock()
            .unwrap()
            .collections
            .as_ref()
            .unwrap()
            .retirements
            .len(),
        0
    );
    book.publications().fence().unwrap().publish().unwrap();
    let before_creation = trees.factory.metrics().unwrap();
    let creation = trees.reserve_creation_collection().unwrap();
    assert!(creation.reserved.is_some());
    assert_eq!(
        trees
            .state
            .lock()
            .unwrap()
            .collections
            .as_ref()
            .unwrap()
            .retirements
            .len(),
        RETIREMENTS
    );
    drop(creation);
    assert_eq!(trees.factory.metrics().unwrap(), before_creation);
    assert_eq!(
        trees
            .state
            .lock()
            .unwrap()
            .collections
            .as_ref()
            .unwrap()
            .retirements
            .len(),
        0
    );
}

#[test]
fn terminal_selection_admits_every_group_before_initialization_and_releases_on_clear() {
    // Arrange
    let (book, trees, contexts) = fixture(96);
    let factory = trees.factory.clone();
    let baseline = factory.metrics().unwrap();
    let trees = trees.with_collections(contexts).unwrap();
    let collections_only = factory.metrics().unwrap();
    // Act
    let trees = trees.with_terminal_publication().unwrap();
    let selected = factory.metrics().unwrap();
    trees.clear();
    // Assert
    assert_eq!(selected.revisions - collections_only.revisions, 33);
    assert!(selected.bytes > collections_only.bytes);
    assert_eq!(factory.metrics().unwrap(), baseline);
    assert!(book.publications().is_drained().unwrap());
    eprintln!(
        "Selected terminal source: bytes={}, revisions={}",
        selected.bytes - collections_only.bytes,
        selected.revisions - collections_only.revisions
    );
}

#[test]
fn terminal_selection_failure_reclaims_partial_groups_and_collection_pools() {
    // Arrange
    let (book, trees, contexts) = fixture(33);
    let factory = trees.factory.clone();
    let baseline = factory.metrics().unwrap();
    let trees = trees.with_collections(contexts).unwrap();
    // Act
    let result = trees.with_terminal_publication();
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(factory.metrics().unwrap(), baseline);
    assert_eq!(book.metrics().unwrap().pending_publications, 0);
}
