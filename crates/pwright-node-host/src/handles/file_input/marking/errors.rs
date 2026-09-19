use super::*;
use pwright_cdp::connection::CdpError;
use pwright_core::ResourceError;

#[test]
fn creation_capacity_errors_keep_their_typed_payload_and_resource_classification() {
    // Arrange
    let causes = [
        MarkerOperationError::Owner(UploadError::Capacity),
        MarkerOperationError::Backend(TargetMarkingError::Capacity),
    ];

    for cause in causes {
        let original = Arc::new(cause);

        // Act
        let result = cache_error(MarkerCacheError::Creation(original.clone()));

        // Assert
        assert_eq!(result.kind(), FailureKind::ResourceExhausted);
        assert!(matches!(
            result.details(),
            Some(TargetMarkPhaseError::Cache(MarkerCacheError::Creation(retained)))
                if Arc::ptr_eq(retained, &original)
        ));
    }
}

#[test]
fn creation_storage_and_resource_errors_use_the_same_marker_classification() {
    // Arrange
    let causes = [
        MarkerOperationError::Owner(UploadError::Storage),
        MarkerOperationError::Owner(UploadError::Resource(
            ResourceError::LeaseCapacityExceeded { limit: 8 },
        )),
        MarkerOperationError::Backend(TargetMarkingError::Budget(
            ResourceError::LeaseCapacityExceeded { limit: 72 },
        )),
    ];

    for cause in causes {
        let original = Arc::new(cause);

        // Act
        let result = cache_error(MarkerCacheError::Creation(original.clone()));

        // Assert
        assert_eq!(result.kind(), FailureKind::ResourceExhausted);
        assert!(matches!(
            result.details(),
            Some(TargetMarkPhaseError::Cache(MarkerCacheError::Creation(retained)))
                if Arc::ptr_eq(retained, &original)
        ));
    }
}

#[test]
fn creation_cleanup_failure_is_not_hidden_by_a_capacity_primary() {
    // Arrange
    let causes = [
        MarkerOperationError::Cleanup {
            primary: Some(Box::new(MarkerOperationError::Owner(UploadError::Capacity))),
            cleanup: ResolutionError::Backend(CdpError::SessionDetached),
        },
        MarkerOperationError::Backend(TargetMarkingError::Cleanup {
            primary: Some(Box::new(TargetMarkingError::Capacity)),
            source: CdpError::SessionDetached,
        }),
    ];

    for cause in causes {
        let original = Arc::new(cause);

        // Act
        let result = cache_error(MarkerCacheError::Creation(original.clone()));

        // Assert
        assert_eq!(result.kind(), FailureKind::Cleanup);
        assert!(matches!(
            result.details(),
            Some(TargetMarkPhaseError::Cache(MarkerCacheError::Creation(retained)))
                if Arc::ptr_eq(retained, &original)
        ));
    }
}
