use super::*;
use pwright_fake::FakeCdpClient;
use serde_json::json;

#[tokio::test]
async fn initiation_targets_only_the_supplied_frame_and_preserves_receipt() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    let service = CdpNavigationService::new(fake.clone());
    // Act
    let result = service
        .begin_frame_navigation(
            "child",
            BeginNavigationParams::new("https://example.test/next")
                .with_referrer("https://referrer.test/path?q=1"),
        )
        .await
        .unwrap();
    // Assert
    assert_eq!(result.frame_id, "child");
    assert_eq!(result.loader_id.as_deref(), Some("frame-loader"));
    assert!(!result.same_document);
    assert!(!result.is_download);
    assert_eq!(
        fake.calls(),
        [(
            "Page.navigate".into(),
            vec![json!({"frameId": "child",
        "url": "https://example.test/next", "referrer": "https://referrer.test/path?q=1", "referrerPolicy": "unsafeUrl"})]
        )]
    );
}

#[tokio::test]
async fn empty_or_oversized_frame_rejects_before_native_dispatch() {
    for frame in ["", "12345"] {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let service = CdpNavigationService::with_limits(
            fake.clone(),
            CdpNavigationLimits::new(128, 4, 4, 4, 128, 32),
        );
        // Act
        let result = service
            .begin_frame_navigation(frame, BeginNavigationParams::new("about:blank"))
            .await;
        // Assert
        assert!(result.is_err());
        assert!(fake.calls().is_empty());
    }
}

#[tokio::test]
async fn a_wrong_native_frame_identity_cannot_be_reported_as_success() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    fake.set_navigate_response(json!({"frameId": "root", "loaderId": "loader"}));
    let service = CdpNavigationService::new(fake.clone());
    // Act
    let error = service
        .begin_frame_navigation("child", BeginNavigationParams::new("about:blank"))
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert!(error.message().contains("different Frame"));
    assert_eq!(fake.method_names(), ["Page.navigate"]);
}

#[tokio::test]
async fn same_document_and_download_receipts_remain_distinct_from_completion() {
    for download in [false, true] {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html(""));
        fake.set_navigate_response(json!({"frameId": "child", "isDownload": download}));
        let service = CdpNavigationService::new(fake.clone());
        // Act
        let result = service
            .begin_frame_navigation("child", BeginNavigationParams::new("about:blank#hash"))
            .await
            .unwrap();
        // Assert
        assert!(result.same_document);
        assert_eq!(result.is_download, download);
        assert_eq!(result.loader_id, None);
        assert_eq!(fake.method_names(), ["Page.navigate"]);
    }
}

#[tokio::test]
async fn network_failure_propagates_without_retrying_or_stopping_other_frames() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    fake.set_navigate_response(
        json!({"frameId": "child", "errorText": "net::ERR_NAME_NOT_RESOLVED"}),
    );
    let service = CdpNavigationService::new(fake.clone());
    // Act
    let error = service
        .begin_frame_navigation("child", BeginNavigationParams::new("https://missing.test"))
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::Backend);
    assert!(error.message().contains("ERR_NAME_NOT_RESOLVED"));
    assert_eq!(fake.method_names(), ["Page.navigate"]);
}

#[tokio::test]
async fn frame_dispatch_does_not_turn_a_native_http_failure_into_success() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    fake.set_navigate_response(json!({"frameId": "child", "errorText": HTTP_STATUS_FAILURE}));
    let service = CdpNavigationService::new(fake.clone());
    // Act
    let error = service
        .begin_frame_navigation("child", BeginNavigationParams::new("https://example.test"))
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::Backend);
    assert!(error.message().contains(HTTP_STATUS_FAILURE));
    assert_eq!(fake.method_names(), ["Page.navigate"]);
}

#[tokio::test]
async fn frame_initiation_shares_admission_with_page_operations_and_releases_on_drop() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    let profile = CdpNavigationProductionProfile::new(
        ResourceScope::Connection,
        CdpNavigationLimits::new(128, 32, 4, 4, 128, 32),
        1,
    );
    let requirements = profile.requirements();
    let bytes = requirements.class(ResourceClass::BackendStateBytes);
    let limits = pwright_core::ResourceLimits::deny_all(requirements.scopes, requirements.leases)
        .with_class(
            ResourceClass::BackendStateBytes,
            bytes.global,
            bytes.per_scope,
        );
    let resources = ProductionResourceLedger::new(limits, requirements).unwrap();
    let service =
        CdpNavigationService::new_with_production_resources(fake.clone(), profile, &resources)
            .unwrap();
    let held = service.call(BeginNavigationParams::new("about:blank"));
    assert!(fake.calls().is_empty());
    // Act
    let denied = service
        .begin_frame_navigation("child", BeginNavigationParams::new("about:blank"))
        .await
        .unwrap_err();
    // Assert
    assert_eq!(denied.kind(), FailureKind::ResourceExhausted);
    assert!(fake.calls().is_empty());
    drop(held);
    let unpolled =
        service.begin_frame_navigation("child", BeginNavigationParams::new("about:blank"));
    assert!(fake.calls().is_empty());
    drop(unpolled);
    let result = service
        .begin_frame_navigation("child", BeginNavigationParams::new("about:blank"))
        .await
        .unwrap();
    assert_eq!(result.frame_id, "child");
    assert_eq!(fake.method_names(), ["Page.navigate"]);
}
