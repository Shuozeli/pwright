use super::*;
use pwright_browser_model::ELEMENT_HANDLE_OBJECT_KIND;
use serde_json::json;

fn configure_owner(fixture: &Fixture, object_id: &str) {
    fixture.primary.set_frame_owner("child-frame", 91);
    fixture.primary.set_adopted_node_response(
        91,
        101,
        json!({"object": {"type":"object", "subtype":"node", "objectId":object_id}}),
    );
}

#[tokio::test]
async fn frame_owner_is_published_in_the_exact_parent_main_world() {
    // Arrange
    let fixture = fixture(1);
    configure_owner(&fixture, "iframe-owner");
    let service = fixture.installation.raw_service();

    // Act
    let pending = service
        .stage_frame_owner_element(
            ObjectId::new(4),
            "child-frame",
            fixture.primary.clone(),
            101,
            "world-1",
        )
        .await
        .unwrap();
    let identity = pending.world_identity();
    let outcome = pending.commit_or_defer();

    // Assert
    let NativeHandleCreationCommit::Published(handle) = outcome else {
        panic!("frame owner did not publish")
    };
    assert_eq!(handle.object_kind, ELEMENT_HANDLE_OBJECT_KIND.tag);
    assert_eq!(identity.page_id(), ObjectId::new(3));
    assert_eq!(identity.execution_context_id(), 101);
    assert_eq!(identity.unique_context_id(), "world-1");
    assert_eq!(
        fixture.primary.method_names(),
        ["DOM.getFrameOwner", "DOM.resolveNode"]
    );
    assert!(fixture.secondary.calls().is_empty());
    assert_eq!(fixture.primary.calls()[1].1[0]["executionContextId"], 101);
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
}

#[tokio::test]
async fn frame_owner_rejects_a_different_parent_session_before_cdp() {
    // Arrange
    let fixture = fixture(1);
    configure_owner(&fixture, "iframe-owner");
    let service = fixture.installation.raw_service();

    // Act
    let result = service
        .stage_frame_owner_element(
            ObjectId::new(4),
            "child-frame",
            fixture.secondary.clone(),
            101,
            "world-1",
        )
        .await;

    // Assert
    assert!(matches!(result, Err(error) if error.kind() == FailureKind::TargetClosed));
    assert!(fixture.primary.calls().is_empty());
    assert!(fixture.secondary.calls().is_empty());
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
async fn frame_owner_rejects_a_replaced_parent_main_world_before_cdp() {
    // Arrange
    let fixture = fixture(1);
    configure_owner(&fixture, "iframe-owner");
    let service = fixture.installation.raw_service();

    // Act
    let result = service
        .stage_frame_owner_element(
            ObjectId::new(4),
            "child-frame",
            fixture.primary.clone(),
            101,
            "replacement-world",
        )
        .await;

    // Assert
    assert!(matches!(result, Err(error) if error.kind() == FailureKind::TargetClosed));
    assert!(fixture.primary.calls().is_empty());
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
async fn abandoned_frame_owner_releases_native_object_without_publishing_a_route() {
    // Arrange
    let fixture = fixture(1);
    configure_owner(&fixture, "unpublished-owner");
    let service = fixture.installation.raw_service();
    let pending = service
        .stage_frame_owner_element(
            ObjectId::new(4),
            "child-frame",
            fixture.primary.clone(),
            101,
            "world-1",
        )
        .await
        .unwrap();

    // Act
    pending.release_native().await.unwrap();

    // Assert
    assert_eq!(
        fixture.primary.method_names(),
        [
            "DOM.getFrameOwner",
            "DOM.resolveNode",
            "Runtime.releaseObject"
        ]
    );
    let metrics = fixture.installation.lifecycle().routes().metrics().unwrap();
    assert_eq!(metrics.current, 0);
    assert_eq!(metrics.reserved, 0);
}
