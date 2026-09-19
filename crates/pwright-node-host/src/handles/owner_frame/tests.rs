use super::*;

fn cleanup_failure() -> Result<(), OperationError<HostError>> {
    Err(OperationError::new(
        FailureKind::Cleanup,
        "temporary receiver release failed",
        Some(HostError::Backend("native compensation failed".into())),
    ))
}

#[tokio::test]
async fn cleanup_failure_survives_a_resource_exhausted_publication_fence() {
    // Arrange
    let mut attempted = false;
    let publication = async {
        attempted = true;
        Err(HostError::ResourceExhausted("driver queue full".into()))
    };

    // Act
    let result = publish_owner_frame_result(cleanup_failure(), true, publication, || {
        panic!("cleanup must not be reclassified as navigation")
    })
    .await;

    // Assert
    assert!(attempted);
    assert!(
        matches!(result, Err(HostError::Backend(message)) if message == "native compensation failed")
    );
}

#[tokio::test]
async fn cleanup_failure_survives_a_closed_publication_fence() {
    // Arrange
    let mut attempted = false;
    let publication = async {
        attempted = true;
        Err(HostError::TargetClosed)
    };

    // Act
    let result = publish_owner_frame_result(cleanup_failure(), true, publication, || {
        panic!("cleanup must not be reclassified as navigation")
    })
    .await;

    // Assert
    assert!(attempted);
    assert!(
        matches!(result, Err(HostError::Backend(message)) if message == "native compensation failed")
    );
}

#[tokio::test]
async fn successful_native_read_still_reports_publication_failure() {
    // Arrange
    let mut attempted = false;
    let publication = async {
        attempted = true;
        Err(HostError::ResourceExhausted("driver queue full".into()))
    };

    // Act
    let result = publish_owner_frame_result(Ok(Some("42".to_owned())), true, publication, || {
        panic!("successful read must not ask for error refinement")
    })
    .await;

    // Assert
    assert!(attempted);
    assert!(
        matches!(result, Err(HostError::ResourceExhausted(message)) if message == "driver queue full")
    );
}

#[tokio::test]
async fn ordinary_target_closed_does_not_reenter_the_publication_queue() {
    // Arrange
    let outcome = Err::<(), _>(OperationError::new(
        FailureKind::TargetClosed,
        "source Page closed",
        Some(HostError::TargetClosed),
    ));
    let mut attempted = false;
    let publication = async {
        attempted = true;
        Err(HostError::ResourceExhausted("driver queue full".into()))
    };

    // Act
    let result = publish_owner_frame_result(outcome, true, publication, || false).await;

    // Assert
    assert!(!attempted);
    assert!(matches!(result, Err(HostError::TargetClosed)));
}

#[tokio::test]
async fn typed_world_replacement_refines_only_an_ordinary_closed_result() {
    // Arrange
    let outcome = Err::<(), _>(OperationError::new(
        FailureKind::TargetClosed,
        "source Handle retired",
        Some(HostError::TargetClosed),
    ));
    let mut attempted = false;
    let publication = async {
        attempted = true;
        Ok(())
    };

    // Act
    let result = publish_owner_frame_result(outcome, true, publication, || true).await;

    // Assert
    assert!(attempted);
    assert!(matches!(result, Err(HostError::Backend(message))
        if message == "Execution context was destroyed, most likely because of a navigation"));
}
