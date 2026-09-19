use super::*;
use pwright_composition_chromium_handle_evaluation::native_creation::{
    NativeHandleCreationCommit, PendingNativeHandleCreation,
};

mod batch;
mod live;
mod target;

async fn staged(fixture: &Fixture) -> PendingNativeHandleCreation {
    fixture.primary.set_evaluate_response(serde_json::json!({
        "result": {"type": "object", "objectId": "staged-native", "description": "staged"}
    }));
    fixture
        .installation
        .raw_service()
        .stage_evaluate_handle(
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::expression("({})"),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn native_result_is_unpublished_until_explicit_local_commit() {
    // Arrange
    let fixture = fixture(1);
    let before = fixture.runtime.live_object_count().unwrap();
    let pending = staged(&fixture).await;
    assert_eq!(fixture.runtime.live_object_count().unwrap(), before);
    assert_eq!(
        fixture
            .installation
            .lifecycle()
            .routes()
            .metrics()
            .unwrap()
            .reserved,
        1
    );
    let calls = fixture.primary.calls().len();

    // Act
    let result = pending.commit_or_defer();

    // Assert
    let NativeHandleCreationCommit::Published(handle) = result else {
        panic!("creation did not publish")
    };
    assert_eq!(fixture.runtime.live_object_count().unwrap(), before + 1);
    assert!(fixture.runtime.is_object_live(handle.object_id).unwrap());
    assert_eq!(
        fixture
            .installation
            .lifecycle()
            .routes()
            .metrics()
            .unwrap()
            .current,
        1
    );
    assert_eq!(
        fixture
            .installation
            .lifecycle()
            .routes()
            .metrics()
            .unwrap()
            .reserved,
        0
    );
    assert_eq!(fixture.primary.calls().len(), calls);
}

#[tokio::test]
async fn busy_commit_retains_native_result_and_retries_without_browser_io() {
    // Arrange
    let fixture = fixture(1);
    let close = fixture
        .runtime
        .prepare_object_disposal(ObjectId::new(5))
        .unwrap();
    let pending = staged(&fixture).await;
    let before = fixture.primary.calls().len();
    let identity = pending.world_identity();

    // Act
    let result = pending.commit_or_defer();

    // Assert
    let NativeHandleCreationCommit::Deferred(pending) = result else {
        panic!("busy creation was not retained")
    };
    assert_eq!(pending.world_identity(), identity);
    assert_eq!(
        fixture
            .installation
            .lifecycle()
            .routes()
            .metrics()
            .unwrap()
            .reserved,
        1
    );
    assert_eq!(
        fixture
            .installation
            .lifecycle()
            .routes()
            .metrics()
            .unwrap()
            .current,
        0
    );
    assert_eq!(fixture.primary.calls().len(), before);
    drop(close);
    let result = pending.commit_or_defer();
    assert!(matches!(result, NativeHandleCreationCommit::Published(_)));
    assert_eq!(fixture.primary.calls().len(), before);
}

#[tokio::test]
async fn source_world_replacement_rejects_staged_result_then_releases_it() {
    // Arrange
    let fixture = fixture(1);
    let pending = staged(&fixture).await;
    let routes = fixture.installation.routes();
    routes.unbind(ObjectId::new(4)).unwrap();
    routes
        .bind(
            ObjectId::new(4),
            ObjectId::new(3),
            fixture.primary.clone(),
            202,
            "replacement",
        )
        .unwrap();
    let before = fixture.primary.calls().len();
    assert_eq!(pending.world_identity().unique_context_id(), "world-1");

    // Act
    let result = pending.commit_or_defer();

    // Assert
    let NativeHandleCreationCommit::Rejected(failure) = result else {
        panic!("old world was published")
    };
    assert_eq!(failure.error().kind(), FailureKind::TargetClosed);
    assert_eq!(fixture.primary.calls().len(), before);
    assert_eq!(failure.release().await.kind(), FailureKind::TargetClosed);
    assert_eq!(
        fixture.primary.calls().last().unwrap().0,
        "Runtime.releaseObject"
    );
    let metrics = fixture.installation.lifecycle().routes().metrics().unwrap();
    assert_eq!(metrics.current, 0);
    assert_eq!(metrics.reserved, 0);
}

#[tokio::test]
async fn explicit_abandonment_releases_without_publishing_a_portable_object() {
    // Arrange
    let fixture = fixture(1);
    let before = fixture.runtime.live_object_count().unwrap();
    let pending = staged(&fixture).await;

    // Act
    let result = pending.release().await;

    // Assert
    result.unwrap();
    assert_eq!(fixture.runtime.live_object_count().unwrap(), before);
    assert_eq!(
        fixture.primary.calls().last().unwrap().0,
        "Runtime.releaseObject"
    );
    assert_eq!(
        fixture
            .installation
            .lifecycle()
            .routes()
            .metrics()
            .unwrap()
            .reserved,
        0
    );
}

#[tokio::test]
async fn retained_unpublished_result_exhausts_route_capacity_before_second_script() {
    // Arrange
    let fixture = fixture(1);
    let pending = staged(&fixture).await;
    let before = fixture.primary.calls().len();

    // Act
    let result = fixture
        .installation
        .raw_service()
        .stage_evaluate_handle(
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::expression("({})"),
        )
        .await;

    // Assert
    let Err(error) = result else {
        panic!("capacity must reject the second creation")
    };
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert_eq!(fixture.primary.calls().len(), before);
    pending.release().await.unwrap();
    assert_eq!(
        fixture
            .installation
            .lifecycle()
            .routes()
            .metrics()
            .unwrap()
            .reserved,
        0
    );
}

#[tokio::test]
async fn primitive_abandonment_sends_no_native_release() {
    // Arrange
    let fixture = fixture(1);
    fixture
        .primary
        .set_evaluate_response(serde_json::json!({"result": {"type": "number", "value": 7}}));
    let pending = fixture
        .installation
        .raw_service()
        .stage_evaluate_handle(
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::expression("7"),
        )
        .await
        .unwrap();
    let before = fixture.primary.calls().len();

    // Act
    let result = pending.release().await;

    // Assert
    result.unwrap();
    assert_eq!(fixture.primary.calls().len(), before);
    assert_eq!(
        fixture
            .installation
            .lifecycle()
            .routes()
            .metrics()
            .unwrap()
            .reserved,
        0
    );
}

#[tokio::test]
async fn committed_unrelated_page_close_does_not_invalidate_staged_creation() {
    // Arrange
    let fixture = fixture(1);
    let pending = staged(&fixture).await;
    fixture
        .runtime
        .prepare_object_disposal(ObjectId::new(5))
        .unwrap()
        .commit()
        .unwrap();
    let before = fixture.primary.calls().len();

    // Act
    let result = pending.commit_or_defer();

    // Assert
    let NativeHandleCreationCommit::Published(handle) = result else {
        panic!("unrelated close rejected creation")
    };
    assert!(fixture.runtime.is_object_live(handle.object_id).unwrap());
    assert!(!fixture.runtime.is_object_live(ObjectId::new(5)).unwrap());
    assert_eq!(fixture.primary.calls().len(), before);
}

#[tokio::test]
async fn transport_close_is_terminal_not_a_deferred_creation() {
    // Arrange
    let fixture = fixture(1);
    let pending = staged(&fixture).await;
    fixture.runtime.close_transport().unwrap();

    // Act
    let result = pending.commit_or_defer();

    // Assert
    let NativeHandleCreationCommit::Rejected(failure) = result else {
        panic!("closed transport must reject publication")
    };
    assert_eq!(failure.error().kind(), FailureKind::TransportClosed);
    assert_eq!(failure.release().await.kind(), FailureKind::TransportClosed);
    assert_eq!(
        fixture
            .installation
            .lifecycle()
            .routes()
            .metrics()
            .unwrap()
            .reserved,
        0
    );
}

#[tokio::test]
async fn rejected_publication_preserves_both_source_failure_and_cleanup_failure() {
    // Arrange
    let fixture = fixture(1);
    let pending = staged(&fixture).await;
    fixture
        .installation
        .routes()
        .unbind(ObjectId::new(4))
        .unwrap();
    fixture
        .primary
        .set_release_object_error(Some("injected native release failure".into()));
    let NativeHandleCreationCommit::Rejected(failure) = pending.commit_or_defer() else {
        panic!("missing source must reject publication")
    };

    // Act
    let error = failure.release().await;

    // Assert
    assert_eq!(error.kind(), FailureKind::Cleanup);
    assert!(
        error
            .message()
            .contains("no active Handle evaluation route")
    );
    assert!(error.message().contains("injected native release failure"));
    assert_eq!(
        fixture
            .installation
            .lifecycle()
            .routes()
            .metrics()
            .unwrap()
            .current,
        0
    );
}
