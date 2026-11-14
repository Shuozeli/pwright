use super::*;
use pwright_fake::FakeCdpClient;
use serde_json::json;

fn fixture() -> (Arc<FakeCdpClient>, CdpFrameOwnerElementService) {
    let fake = Arc::new(FakeCdpClient::from_html(""));
    let service = CdpFrameOwnerElementService::new(CdpHandleEvaluationLimits::default());
    (fake, service)
}

#[tokio::test]
async fn resolves_frame_owner_in_the_supplied_parent_main_world() {
    // Arrange
    let (fake, service) = fixture();
    fake.set_frame_owner("child-frame", 91);
    fake.set_adopted_node_response(
        91,
        17,
        json!({"object": {"type":"object", "subtype":"node", "objectId":"owner-17"}}),
    );

    // Act
    let result = service
        .resolve(fake.clone(), "child-frame", 17, "frame-owner-test-1")
        .await
        .unwrap();

    // Assert
    assert_eq!(result.kind(), CdpHandleKind::Element);
    assert_eq!(result.object_id(), Some("owner-17"));
    assert_eq!(
        fake.method_names(),
        ["DOM.getFrameOwner", "DOM.resolveNode"]
    );
    let calls = fake.calls();
    assert_eq!(calls[0].1, [json!({"frameId":"child-frame"})]);
    assert_eq!(calls[1].1[0]["backendNodeId"], 91);
    assert_eq!(calls[1].1[0]["executionContextId"], 17);
    assert_eq!(calls[1].1[0]["objectGroup"], "frame-owner-test-1");
}

#[tokio::test]
async fn malformed_resolve_releases_the_exact_owner_group() {
    // Arrange
    let (fake, service) = fixture();
    fake.set_frame_owner("child-frame", 91);
    fake.set_adopted_node_response(91, 17, json!({"object":{"type":"object"}}));

    // Act
    let result = service
        .resolve(fake.clone(), "child-frame", 17, "frame-owner-test-2")
        .await;

    // Assert
    assert!(matches!(result, Err(error) if error.kind() == FailureKind::ProtocolViolation));
    assert_eq!(
        fake.method_names(),
        [
            "DOM.getFrameOwner",
            "DOM.resolveNode",
            "Runtime.releaseObjectGroup"
        ]
    );
    assert_eq!(fake.calls()[2].1, [json!("frame-owner-test-2")]);
}

#[tokio::test]
async fn a_missing_frame_owner_reports_detachment_before_native_acquisition() {
    // Arrange
    let (fake, service) = fixture();

    // Act
    let result = service
        .resolve(fake.clone(), "child-frame", 17, "frame-owner-test-3")
        .await;

    // Assert
    assert!(result.is_err());
    assert_eq!(fake.method_names(), ["DOM.getFrameOwner"]);
}

#[test]
fn chromium_detached_frame_error_has_the_playwright_message_and_target_closed_kind() {
    // Arrange
    let error_bytes = 4096;

    // Act
    let error = detached(error_bytes);

    // Assert
    assert_eq!(error.kind(), FailureKind::TargetClosed);
    assert_eq!(error.message(), "Frame has been detached.");
}

#[tokio::test]
async fn rejected_route_does_not_touch_the_browser() {
    // Arrange
    let (fake, service) = fixture();

    // Act
    let result = service
        .resolve(fake.clone(), "child-frame", 0, "owner")
        .await;

    // Assert
    assert!(result.is_err());
    assert!(fake.calls().is_empty());
}
