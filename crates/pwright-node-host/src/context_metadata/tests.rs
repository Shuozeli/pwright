use super::*;

fn limits() -> ContextMetadataLimits {
    ContextMetadataLimits {
        active_contexts: 2,
        states: 8,
        conversions: 8,
        base_url_bytes: 128,
        bytes: 64 * 1024,
    }
}

#[test]
fn owned_context_is_hidden_until_settings_commit() {
    // Arrange
    let book = ContextMetadata::new(limits()).unwrap();
    let reservation = book.reserve_creation(Some("https://a.test/root/")).unwrap();
    assert!(book.visible(ObjectId::new(1), true).unwrap().is_none());
    // Act
    let created = reservation.commit(ObjectId::new(1)).unwrap();
    // Assert
    let visible = book.visible(ObjectId::new(1), true).unwrap().unwrap();
    assert_eq!(visible.base_url(), Some("https://a.test/root/"));
    assert_eq!(created.binding.snapshot(), &visible);
    assert_eq!(book.metrics().unwrap().pending_creations, 0);
}

#[test]
fn absent_and_empty_base_urls_remain_distinct() {
    // Arrange
    let book = ContextMetadata::new(limits()).unwrap();
    // Act
    let missing = book
        .reserve_creation(None)
        .unwrap()
        .commit(ObjectId::new(1))
        .unwrap();
    let empty = book
        .reserve_creation(Some(""))
        .unwrap()
        .commit(ObjectId::new(2))
        .unwrap();
    // Assert
    assert_eq!(missing.snapshot.base_url(), None);
    assert_eq!(empty.snapshot.base_url(), Some(""));
    assert_ne!(missing.snapshot, empty.snapshot);
}

#[test]
fn abandoned_creation_releases_every_admission() {
    // Arrange
    let book = ContextMetadata::new(limits()).unwrap();
    let before = book.metrics().unwrap();
    let reservation = book.reserve_creation(Some("https://a.test/")).unwrap();
    // Act
    drop(reservation);
    // Assert
    assert_eq!(book.metrics().unwrap(), before);
}

#[test]
fn binding_exhaustion_rolls_back_settings_and_registry_before_creation() {
    // Arrange
    let book = ContextMetadata::new(ContextMetadataLimits {
        states: 1,
        ..limits()
    })
    .unwrap();
    let before = book.metrics().unwrap();
    // Act
    let result = book.reserve_creation(Some("https://a.test/"));
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(book.metrics().unwrap(), before);
}

#[test]
fn pending_creations_share_the_active_registry_limit() {
    // Arrange
    let book = ContextMetadata::new(ContextMetadataLimits {
        active_contexts: 1,
        ..limits()
    })
    .unwrap();
    let pending = book.reserve_creation(None).unwrap();
    let before = book.metrics().unwrap();
    // Act
    let result = book.reserve_creation(None);
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(book.metrics().unwrap(), before);
    drop(pending);
    assert_eq!(book.metrics().unwrap().pending_creations, 0);
}

#[test]
fn retained_binding_survives_registry_and_connection_owner_drop() {
    // Arrange
    let book = ContextMetadata::new(limits()).unwrap();
    let created = book
        .reserve_creation(Some("https://a.test/"))
        .unwrap()
        .commit(ObjectId::new(1))
        .unwrap();
    let resources = created.snapshot.resources.clone();
    // Act
    book.clear();
    drop(book);
    drop(created.snapshot);
    // Assert
    assert_eq!(
        created.binding.snapshot().base_url(),
        Some("https://a.test/")
    );
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::LiveObject)
            .current,
        2
    );
    drop(created.binding);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::LiveObject)
            .current,
        0
    );
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        0
    );
}

#[test]
fn shutdown_rejects_late_commit_and_releases_pending_creation() {
    // Arrange
    let book = ContextMetadata::new(limits()).unwrap();
    let before = book.metrics().unwrap();
    let pending = book.reserve_creation(Some("https://a.test/")).unwrap();
    book.clear();
    // Act
    let result = pending.commit(ObjectId::new(1));
    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(book.metrics().unwrap(), before);
}

#[test]
fn duplicate_commit_preserves_original_settings_and_rolls_back_new_ones() {
    // Arrange
    let book = ContextMetadata::new(limits()).unwrap();
    let original = book
        .reserve_creation(Some("https://a.test/"))
        .unwrap()
        .commit(ObjectId::new(1))
        .unwrap();
    let before = book.metrics().unwrap();
    let pending = book.reserve_creation(Some("https://b.test/")).unwrap();
    // Act
    let result = pending.commit(ObjectId::new(1));
    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert_eq!(book.metrics().unwrap(), before);
    assert_eq!(
        book.visible(ObjectId::new(1), true).unwrap(),
        Some(original.snapshot)
    );
}

#[test]
fn external_and_default_contexts_need_no_registry_entry() {
    // Arrange
    let book = ContextMetadata::new(limits()).unwrap();
    let before = book.metrics().unwrap();
    // Act
    let snapshot = book.visible(ObjectId::new(99), false).unwrap().unwrap();
    // Assert
    assert_eq!(snapshot.base_url(), None);
    assert_eq!(book.metrics().unwrap(), before);
}

#[test]
fn pruning_native_closed_context_keeps_retained_settings() {
    // Arrange
    let book = ContextMetadata::new(limits()).unwrap();
    let created = book
        .reserve_creation(Some("https://a.test/"))
        .unwrap()
        .commit(ObjectId::new(1))
        .unwrap();
    // Act
    book.retain_live(|_| false).unwrap();
    // Assert
    assert_eq!(book.metrics().unwrap().active_contexts, 0);
    assert!(book.visible(ObjectId::new(1), true).unwrap().is_none());
    assert_eq!(created.snapshot.base_url(), Some("https://a.test/"));
}

#[test]
fn oversized_base_url_rejects_without_retaining_input_or_leases() {
    // Arrange
    let book = ContextMetadata::new(limits()).unwrap();
    let before = book.metrics().unwrap();
    let value = "x".repeat(129);
    // Act
    let result = book.reserve_creation(Some(&value));
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(book.metrics().unwrap(), before);
}

#[test]
fn conversion_byte_failure_rolls_back_its_count_slot() {
    // Arrange
    let book = ContextMetadata::new(limits()).unwrap();
    let snapshot = book.empty();
    let before = book.metrics().unwrap();
    // Act
    let result = snapshot.reserve_conversion(64 * 1024);
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(book.metrics().unwrap(), before);
}
