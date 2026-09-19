use super::*;
use crate::handles::file_input::check_dispatch_registration;
use pwright_browser_model::ELEMENT_HANDLE_OBJECT_KIND;

fn source(owners: &Arc<Owners>, cleanup: &TokioCleanupExecutor, element: bool) -> PendingHandle {
    let (mut initializer, ticket, admission) = ticket(
        2,
        owners.admit().unwrap(),
        cleanup,
        Arc::new(AtomicUsize::new(0)),
    );
    if element {
        initializer.object_kind = ELEMENT_HANDLE_OBJECT_KIND.tag;
    }
    owners.retain(initializer, ticket, admission).unwrap()
}

#[tokio::test]
async fn identity_is_copy_only_metadata_with_no_new_native_or_work_admission() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let source = source(&owners, &cleanup, true);
    let before = resources.metrics().unwrap();
    let work = owners.work.len();
    let strong = Arc::strong_count(&owners);

    // Act
    let identity = source.source_identity();
    let validated = owners.check_file_input_source(&identity);

    // Assert
    assert!(validated.is_ok());
    assert_eq!(identity.target(), ObjectId::new(2));
    assert_eq!(identity.creation_frame_id(), ObjectId::new(1));
    assert_eq!(identity.creation_page_id(), None);
    assert!(identity.is_element());
    assert_eq!(Arc::strong_count(&owners), strong);
    assert_eq!(owners.work.len(), work);
    assert_eq!(resources.metrics().unwrap(), before);
    source.dispose().await.unwrap();
    assert_eq!(cleanup.shutdown().await.metrics.failed, 0);
}

#[tokio::test]
async fn identical_portable_ids_do_not_authorize_a_foreign_session() {
    // Arrange
    let (origin, _) = owners();
    let (foreign, _) = owners();
    let origin_cleanup = TokioCleanupExecutor::new(1);
    let foreign_cleanup = TokioCleanupExecutor::new(1);
    let original = source(&origin, &origin_cleanup, true);
    let other = source(&foreign, &foreign_cleanup, true);
    let identity = original.source_identity();
    assert_eq!(identity.target(), other.initializer().object_id);
    assert_eq!(identity.creation_frame_id(), other.initializer().parent_id);

    // Act
    let result = foreign.check_file_input_source(&identity);

    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    original.dispose().await.unwrap();
    other.dispose().await.unwrap();
    assert_eq!(origin_cleanup.shutdown().await.metrics.failed, 0);
    assert_eq!(foreign_cleanup.shutdown().await.metrics.failed, 0);
}

#[tokio::test]
async fn disposed_descriptor_does_not_prematurely_fail_preparation_identity() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let source = source(&owners, &cleanup, true);
    let identity = source.source_identity();
    source.dispose().await.unwrap();
    assert!(owners.check_arguments([&source]).is_err());
    drop(source);
    let before = resources.metrics().unwrap();

    // Act
    let result = owners.check_file_input_source(&identity);

    // Assert: only provenance is accepted; normal native-use checks above fail.
    assert!(result.is_ok());
    assert_eq!(resources.metrics().unwrap(), before);
    assert_eq!(
        before.class(ResourceClass::BackendStateBytes).current,
        STATE_BYTES
    );
    assert_eq!(before.active_leases, 1);
    assert_eq!(cleanup.shutdown().await.metrics.failed, 0);
}

#[tokio::test]
async fn identity_can_be_captured_after_explicit_disposal_without_a_new_lease() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let source = source(&owners, &cleanup, true);
    source.dispose().await.unwrap();
    let before = resources.metrics().unwrap();

    // Act
    let identity = source.source_identity();
    let result = owners.check_file_input_source(&identity);

    // Assert
    assert!(result.is_ok());
    assert_eq!(resources.metrics().unwrap(), before);
    assert_eq!(cleanup.shutdown().await.metrics.failed, 0);
}

#[tokio::test]
async fn retained_identity_does_not_keep_the_owner_installation_or_leases_alive() {
    // Arrange
    let (owners, resources) = owners();
    let observed = Arc::downgrade(&owners);
    let cleanup = TokioCleanupExecutor::new(1);
    let source = source(&owners, &cleanup, true);
    let identity = source.source_identity();
    let duplicate = identity.clone();
    source.dispose().await.unwrap();
    drop(source);
    owners.close();
    owners.wait().await;
    assert_eq!(cleanup.shutdown().await.metrics.failed, 0);

    // Act
    drop(owners);

    // Assert
    assert!(observed.upgrade().is_none());
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    assert_eq!(identity.target(), duplicate.target());
    assert_eq!(identity.creation_frame_id(), duplicate.creation_frame_id());
    assert!(identity.is_element());
}

#[tokio::test]
async fn ordinary_js_handle_identity_cannot_authorize_element_preparation() {
    // Arrange
    let (owners, _) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let source = source(&owners, &cleanup, false);
    let identity = source.source_identity();

    // Act
    let result = owners.check_file_input_source(&identity);

    // Assert
    assert!(!identity.is_element());
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    source.dispose().await.unwrap();
    assert_eq!(cleanup.shutdown().await.metrics.failed, 0);
}

#[tokio::test]
async fn closed_owner_rejects_preparation_without_waiting_for_descriptor_drop() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let source = source(&owners, &cleanup, true);
    let identity = source.source_identity();
    owners.close();
    owners.wait().await;
    let before = resources.metrics().unwrap();

    // Act
    let result = owners.check_file_input_source(&identity);

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(resources.metrics().unwrap(), before);
    assert_eq!(cleanup.shutdown().await.metrics.failed, 0);
}

#[tokio::test]
async fn registered_element_dispatch_check_does_not_admit_resources_or_work() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let source = source(&owners, &cleanup, true);
    let identity = source.source_identity();
    let before = resources.metrics().unwrap();
    let work = owners.work.len();
    let strong = Arc::strong_count(&owners);

    // Act
    let result = owners.check_file_input_dispatch_source(&identity);

    // Assert
    assert!(result.is_ok());
    assert_eq!(resources.metrics().unwrap(), before);
    assert_eq!(owners.work.len(), work);
    assert_eq!(Arc::strong_count(&owners), strong);
    source.dispose().await.unwrap();
    assert_eq!(cleanup.shutdown().await.metrics.failed, 0);
}

#[tokio::test]
async fn requested_disposal_unregisters_dispatch_before_release_completion() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let source = source(&owners, &cleanup, true);
    let identity = source.source_identity();
    let release = source.request_dispose();
    // This synchronous removal precedes polling the accepted cleanup job.
    let before = resources.metrics().unwrap();
    let work = owners.work.len();

    // Act
    let result = owners.check_file_input_dispatch_source(&identity);

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(resources.metrics().unwrap(), before);
    assert_eq!(owners.work.len(), work);
    release.wait().await.unwrap();
    assert_eq!(cleanup.shutdown().await.metrics.failed, 0);
}

#[tokio::test]
async fn disposed_source_remains_preparation_metadata_but_cannot_dispatch() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let source = source(&owners, &cleanup, true);
    let identity = source.source_identity();
    source.dispose().await.unwrap();
    drop(source);
    let before = resources.metrics().unwrap();

    // Act
    let preparation = owners.check_file_input_source(&identity);
    let dispatch = owners.check_file_input_dispatch_source(&identity);

    // Assert
    assert!(preparation.is_ok());
    assert!(matches!(dispatch, Err(HostError::TargetClosed)));
    assert_eq!(resources.metrics().unwrap(), before);
    assert_eq!(before.active_leases, 1);
    assert_eq!(cleanup.shutdown().await.metrics.failed, 0);
}

#[tokio::test]
async fn foreign_element_with_identical_ids_cannot_authorize_dispatch() {
    // Arrange
    let (origin, _) = owners();
    let (foreign, _) = owners();
    let origin_cleanup = TokioCleanupExecutor::new(1);
    let foreign_cleanup = TokioCleanupExecutor::new(1);
    let original = source(&origin, &origin_cleanup, true);
    let other = source(&foreign, &foreign_cleanup, true);
    let identity = original.source_identity();
    assert_eq!(identity.target(), other.initializer().object_id);
    assert_eq!(identity.creation_frame_id(), other.initializer().parent_id);

    // Act
    let result = foreign.check_file_input_dispatch_source(&identity);

    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    original.dispose().await.unwrap();
    other.dispose().await.unwrap();
    assert_eq!(origin_cleanup.shutdown().await.metrics.failed, 0);
    assert_eq!(foreign_cleanup.shutdown().await.metrics.failed, 0);
}

#[tokio::test]
async fn registered_generic_handle_cannot_authorize_element_dispatch() {
    // Arrange
    let (owners, _) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let source = source(&owners, &cleanup, false);
    let identity = source.source_identity();

    // Act
    let result = owners.check_file_input_dispatch_source(&identity);

    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    source.dispose().await.unwrap();
    assert_eq!(cleanup.shutdown().await.metrics.failed, 0);
}

#[tokio::test]
async fn closed_owner_rejects_dispatch_even_while_source_descriptor_survives() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let source = source(&owners, &cleanup, true);
    let identity = source.source_identity();
    owners.close();
    owners.wait().await;
    let before = resources.metrics().unwrap();

    // Act
    let result = owners.check_file_input_dispatch_source(&identity);

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(resources.metrics().unwrap(), before);
    assert_eq!(cleanup.shutdown().await.metrics.failed, 0);
}

#[tokio::test]
async fn dispatch_registration_does_not_require_a_native_world_receipt() {
    // Arrange: this fixture installs only an ownership ticket, with no runtime,
    // browser, native client, Frame registry or current-world route at all.
    let (owners, _) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let source = source(&owners, &cleanup, true);
    let identity = source.source_identity();
    assert!(source.source_world().is_none());
    assert_eq!(identity.creation_page_id(), None);

    // Act
    let result = owners.check_file_input_dispatch_source(&identity);

    // Assert: registration proves no native-use or current-world authority.
    assert!(result.is_ok());
    source.dispose().await.unwrap();
    assert_eq!(cleanup.shutdown().await.metrics.failed, 0);
}

#[tokio::test]
async fn a_different_registered_ticket_does_not_keep_disposed_source_dispatchable() {
    // Arrange
    let (owners, _) = owners();
    let cleanup = TokioCleanupExecutor::new(2);
    let original = source(&owners, &cleanup, true);
    let identity = original.source_identity();
    let (mut initializer, ticket, admission) = ticket(
        3,
        owners.admit().unwrap(),
        &cleanup,
        Arc::new(AtomicUsize::new(0)),
    );
    initializer.object_kind = ELEMENT_HANDLE_OBJECT_KIND.tag;
    let sibling = owners.retain(initializer, ticket, admission).unwrap();
    original.dispose().await.unwrap();
    assert_eq!(
        owners
            .state
            .lock()
            .unwrap()
            .tickets
            .iter()
            .flatten()
            .count(),
        1
    );

    // Act
    let result = owners.check_file_input_dispatch_source(&identity);

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    sibling.dispose().await.unwrap();
    assert_eq!(cleanup.shutdown().await.metrics.failed, 0);
}

#[tokio::test]
async fn dispatch_gate_queries_only_the_exact_creation_page_without_new_admission() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let source = source(&owners, &cleanup, true);
    let mut identity = source.source_identity();
    let creation_page = ObjectId::new(41);
    identity.creation_page = Some(creation_page);
    let observed = std::cell::Cell::new(None);
    let before = resources.metrics().unwrap();
    let work = owners.work.len();
    let strong = Arc::strong_count(&owners);
    assert_ne!(creation_page, identity.target());
    assert_ne!(creation_page, identity.creation_frame_id());

    // Act
    let result = check_dispatch_registration(&owners, &identity, |id| {
        observed.set(Some(id));
        Ok(id == creation_page)
    });

    // Assert
    assert!(result.is_ok());
    assert_eq!(observed.get(), Some(creation_page));
    assert_eq!(resources.metrics().unwrap(), before);
    assert_eq!(owners.work.len(), work);
    assert_eq!(Arc::strong_count(&owners), strong);
    source.dispose().await.unwrap();
    assert_eq!(cleanup.shutdown().await.metrics.failed, 0);
}

#[tokio::test]
async fn missing_creation_page_registration_returns_structured_target_closed() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let source = source(&owners, &cleanup, true);
    let mut identity = source.source_identity();
    let creation_page = ObjectId::new(41);
    identity.creation_page = Some(creation_page);
    let observed = std::cell::Cell::new(None);
    let before = resources.metrics().unwrap();

    // Act
    let result = check_dispatch_registration(&owners, &identity, |id| {
        observed.set(Some(id));
        Ok(false)
    });

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(observed.get(), Some(creation_page));
    assert_eq!(resources.metrics().unwrap(), before);
    source.dispose().await.unwrap();
    assert_eq!(cleanup.shutdown().await.metrics.failed, 0);
}

#[tokio::test]
async fn missing_creation_page_proof_fails_without_querying_runtime_objects() {
    // Arrange
    let (owners, _) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let source = source(&owners, &cleanup, true);
    let identity = source.source_identity();
    let called = std::cell::Cell::new(false);
    assert_eq!(identity.creation_page_id(), None);

    // Act
    let result = check_dispatch_registration(&owners, &identity, |_| {
        called.set(true);
        Ok(true)
    });

    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert!(!called.get());
    source.dispose().await.unwrap();
    assert_eq!(cleanup.shutdown().await.metrics.failed, 0);
}

#[tokio::test]
async fn disposed_or_foreign_source_fails_before_the_creation_page_query() {
    for disposed in [true, false] {
        // Arrange
        let (origin, origin_resources) = owners();
        let (foreign, foreign_resources) = owners();
        let origin_cleanup = TokioCleanupExecutor::new(1);
        let foreign_cleanup = TokioCleanupExecutor::new(1);
        let original = source(&origin, &origin_cleanup, true);
        let other = source(&foreign, &foreign_cleanup, true);
        let mut identity = original.source_identity();
        identity.creation_page = Some(ObjectId::new(41));
        assert_eq!(identity.target(), other.initializer().object_id);
        if disposed {
            original.dispose().await.unwrap();
        }
        let target = if disposed { &origin } else { &foreign };
        let called = std::cell::Cell::new(false);
        let before_origin = origin_resources.metrics().unwrap();
        let before_foreign = foreign_resources.metrics().unwrap();

        // Act
        let result = check_dispatch_registration(target, &identity, |_| {
            called.set(true);
            Ok(true)
        });

        // Assert
        if disposed {
            assert!(matches!(result, Err(HostError::TargetClosed)));
        } else {
            assert!(matches!(result, Err(HostError::InvalidArgument(_))));
        }
        assert!(!called.get());
        assert_eq!(origin_resources.metrics().unwrap(), before_origin);
        assert_eq!(foreign_resources.metrics().unwrap(), before_foreign);
        original.dispose().await.unwrap();
        other.dispose().await.unwrap();
        assert_eq!(origin_cleanup.shutdown().await.metrics.failed, 0);
        assert_eq!(foreign_cleanup.shutdown().await.metrics.failed, 0);
    }
}

#[tokio::test]
async fn creation_page_lookup_error_preserves_its_structured_variant_and_message() {
    // Arrange
    let (owners, _) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let source = source(&owners, &cleanup, true);
    let mut identity = source.source_identity();
    identity.creation_page = Some(ObjectId::new(41));

    // Act
    let result = check_dispatch_registration(&owners, &identity, |_| {
        Err(HostError::ResourceExhausted(
            "page registration lookup refused".into(),
        ))
    });

    // Assert
    assert!(matches!(
        result,
        Err(HostError::ResourceExhausted(message))
            if message == "page registration lookup refused"
    ));
    source.dispose().await.unwrap();
    assert_eq!(cleanup.shutdown().await.metrics.failed, 0);
}
