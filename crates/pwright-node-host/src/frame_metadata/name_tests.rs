use super::*;

const ID: ObjectId = ObjectId::new(1);

fn book(publications: usize, url_bytes: u64, name_bytes: u64) -> Arc<FrameMetadata> {
    Arc::new(
        FrameMetadata::new(FrameMetadataLimits {
            entries: 2,
            url_bytes: 32,
            total_bytes: url_bytes,
            publications,
        })
        .unwrap()
        .with_names(FrameNameLimits {
            name_bytes: 16,
            total_bytes: name_bytes,
        })
        .unwrap(),
    )
}

fn name(frame: &FrameUrlSnapshot) -> String {
    frame.with_published_name(str::to_owned).unwrap()
}

#[test]
fn navigation_publishes_url_and_name_at_the_same_watermark() {
    // Arrange
    let book = book(4, 128, 4096);
    book.update_navigation(ID, "old", None, "original").unwrap();
    let frame = book.get(ID).unwrap();
    book.update_navigation(ID, "first", None, "one").unwrap();
    let fence = book.publications().fence().unwrap();
    book.update_navigation(ID, "second", None, "two").unwrap();
    assert_eq!(name(&frame), "original");
    // Act
    fence.publish().unwrap();
    // Assert
    assert_eq!(frame.published_url().unwrap(), "first");
    assert_eq!(name(&frame), "one");
    assert_eq!(book.metrics().unwrap().pending_publications, 1);
}

#[test]
fn same_document_navigation_reuses_the_name_allocation() {
    // Arrange
    let book = book(4, 128, 4096);
    book.update_navigation(ID, "old", None, "你好").unwrap();
    let before = book.metrics().unwrap().name_bytes;
    // Act
    book.update(ID, "old#hash", None).unwrap();
    // Assert
    book.publications().fence().unwrap().publish().unwrap();
    assert_eq!(name(&book.get(ID).unwrap()), "你好");
    assert_eq!(book.metrics().unwrap().name_bytes, before);
}

#[test]
fn name_only_navigation_is_not_mistaken_for_an_identical_url() {
    // Arrange
    let book = book(4, 128, 4096);
    book.update_navigation(ID, "same", None, "before").unwrap();
    // Act
    book.update_navigation(ID, "same", None, "after").unwrap();
    // Assert
    let frame = book.get(ID).unwrap();
    assert!(frame.has_unpublished_url().unwrap());
    assert_eq!(name(&frame), "before");
    book.publications().fence().unwrap().publish().unwrap();
    assert_eq!(name(&frame), "after");
    assert_eq!(frame.published_url().unwrap(), "same");
}

#[test]
fn identical_navigation_does_not_reserve_another_revision() {
    // Arrange
    let book = book(1, 4, 4096);
    book.update_navigation(ID, "same", None, "name").unwrap();
    let before = book.metrics().unwrap();
    // Act
    book.update_navigation(ID, "same", None, "name").unwrap();
    // Assert
    assert_eq!(book.metrics().unwrap(), before);
}

#[test]
fn empty_navigation_name_clears_the_previous_name_without_name_storage() {
    // Arrange
    let book = book(1, 128, 4096);
    book.update_navigation(ID, "old", None, "named").unwrap();
    // Act
    book.update_navigation(ID, "new", None, "").unwrap();
    // Assert
    book.publications().fence().unwrap().publish().unwrap();
    assert_eq!(name(&book.get(ID).unwrap()), "");
    assert_eq!(book.metrics().unwrap().name_bytes, 0);
}

#[test]
fn attachment_and_new_page_have_empty_names_before_navigation() {
    // Arrange
    let book = book(1, 128, 4096);
    // Act
    let attached = book.ensure_attached(ID).unwrap();
    let new_page = book.reserve_new_page().unwrap();
    // Assert
    assert_eq!(name(&attached), "");
    assert_eq!(name(&new_page), "");
    assert_eq!(book.metrics().unwrap().name_bytes, 0);
}

#[test]
fn name_capacity_failure_rolls_back_event_and_both_views() {
    // Arrange
    let book = book(1, 128, 1);
    book.update_navigation(ID, "old", None, "").unwrap();
    let before = book.metrics().unwrap();
    // Act
    let result = book.update_navigation(ID, "new", None, "x");
    // Assert
    assert!(
        result
            .unwrap_err()
            .downcast_ref::<HostError>()
            .is_some_and(|error| matches!(error, HostError::ResourceExhausted(_)))
    );
    let frame = book.get(ID).unwrap();
    assert_eq!(frame.url().unwrap(), "old");
    assert_eq!(name(&frame), "");
    assert_eq!(book.metrics().unwrap(), before);
}

#[test]
fn url_capacity_failure_releases_the_new_name_and_event() {
    // Arrange
    let book = book(1, 3, 4096);
    book.update_navigation(ID, "old", None, "old-name").unwrap();
    let before = book.metrics().unwrap();
    // Act
    let result = book.update_navigation(ID, "new", None, "new-name");
    // Assert
    assert!(result.is_err());
    assert_eq!(book.metrics().unwrap(), before);
    assert_eq!(name(&book.get(ID).unwrap()), "old-name");
}

#[test]
fn replacing_a_name_requires_space_for_old_and_new_allocations() {
    // Arrange
    let one_name =
        u64::try_from(std::mem::size_of::<BudgetedName>() + 2 * std::mem::size_of::<usize>() + 3)
            .unwrap();
    let book = book(1, 128, one_name);
    book.update_navigation(ID, "old", None, "old").unwrap();
    let before = book.metrics().unwrap();
    // Act
    let result = book.update_navigation(ID, "new", None, "new");
    // Assert
    assert!(result.is_err());
    assert_eq!(name(&book.get(ID).unwrap()), "old");
    assert_eq!(book.metrics().unwrap(), before);
}

#[test]
fn publication_capacity_rejects_before_allocating_a_new_name() {
    // Arrange
    let book = book(1, 128, 4096);
    book.update_navigation(ID, "old", None, "old").unwrap();
    book.update_navigation(ID, "one", None, "one").unwrap();
    let before = book.metrics().unwrap();
    // Act
    let result = book.update_navigation(ID, "two", None, "two");
    // Assert
    assert!(result.is_err());
    assert_eq!(name(&book.get(ID).unwrap()), "old");
    assert_eq!(book.get(ID).unwrap().url().unwrap(), "one");
    assert_eq!(book.metrics().unwrap(), before);
}

#[test]
fn oversized_unicode_name_rejects_initial_entry_without_truncation() {
    // Arrange
    let book = book(1, 128, 4096);
    let before = book.metrics().unwrap();
    // Act
    let result = book.update_navigation(ID, "new", None, "你好你好你好");
    // Assert
    assert!(result.is_err());
    assert_eq!(book.metrics().unwrap(), before);
}

#[test]
fn retained_name_survives_book_drop_without_retaining_the_book() {
    // Arrange
    let book = book(1, 128, 4096);
    book.update_navigation(ID, "old", None, "retained").unwrap();
    let frame = book.get(ID).unwrap();
    let weak = Arc::downgrade(&book);
    // Act
    drop(book);
    // Assert
    assert!(weak.upgrade().is_none());
    assert_eq!(name(&frame), "retained");
}

#[test]
fn discarded_publications_free_intermediate_names_and_last_reference_releases_all() {
    // Arrange
    let book = book(4, 128, 4096);
    book.update_navigation(ID, "old", None, "old").unwrap();
    let first_bytes = book.metrics().unwrap().name_bytes;
    let frame = book.get(ID).unwrap();
    book.update_navigation(ID, "one", None, "one").unwrap();
    book.update_navigation(ID, "two", None, "two").unwrap();
    book.clear();
    // Act
    book.publications().discard();
    // Assert
    assert_eq!(name(&frame), "old");
    assert_eq!(book.metrics().unwrap().name_bytes, 2 * first_bytes);
    drop(frame);
    assert_eq!(book.metrics().unwrap().name_bytes, 0);
    assert_eq!(book.metrics().unwrap().entries, 0);
}

#[test]
fn disabled_names_do_not_retain_or_validate_navigation_names() {
    // Arrange
    let book = FrameMetadata::new(FrameMetadataLimits {
        entries: 1,
        url_bytes: 32,
        total_bytes: 32,
        publications: 0,
    })
    .unwrap();
    // Act
    book.update_navigation(ID, "url", None, "unselected name")
        .unwrap();
    // Assert
    assert!(book.names.is_none());
    assert_eq!(book.metrics().unwrap().name_bytes, 0);
    assert!(matches!(
        book.get(ID).unwrap().with_published_name(str::to_owned),
        Err(HostError::UnsupportedOperation(_))
    ));
}

#[test]
fn borrowed_name_conversion_runs_outside_the_metadata_lock() {
    // Arrange
    let book = book(0, 128, 4096);
    book.update_navigation(ID, "old", None, "old").unwrap();
    let frame = book.get(ID).unwrap();
    // Act
    let converted = frame
        .with_published_name(|borrowed| {
            // A synchronous publication during conversion must neither deadlock
            // nor invalidate the charged string that the converter is borrowing.
            book.update_navigation(ID, "new", None, "new").unwrap();
            borrowed.to_owned()
        })
        .unwrap();
    // Assert
    assert_eq!(converted, "old");
    assert_eq!(name(&frame), "new");
}
