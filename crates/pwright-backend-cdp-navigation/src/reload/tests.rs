use super::*;
use pwright_fake::FakeCdpClient;
use serde_json::json;

#[tokio::test]
async fn typed_reload_cancellation_stops_native_loading() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    let service = CdpNavigationService::new(fake.clone());
    let context = OperationContext::new(
        pwright_core::CallId::new(1),
        pwright_core::ObjectId::new(7),
        pwright_cap_navigation::RELOAD_METHOD,
        None,
    );
    // Act
    <CdpNavigationService as CommandCancellation<Reload>>::cancel(
        &service,
        context,
        CancellationReason::DeadlineExceeded,
    )
    .await
    .unwrap();
    // Assert
    assert_eq!(fake.method_names(), ["Page.stopLoading"]);
}

fn event(method: &str, params: serde_json::Value) -> CdpEvent {
    CdpEvent::new(method.to_owned(), params, None)
}

fn commit(loader: &str, url: &str) -> CdpEvent {
    event(
        "Page.frameNavigated",
        json!({"frame": {
            "id": "fake-frame-1", "loaderId": loader, "url": url,
        }}),
    )
}

fn lifecycle(name: &str) -> CdpEvent {
    event(
        "Page.lifecycleEvent",
        json!({
            "frameId": "fake-frame-1", "loaderId": "new-loader", "name": name,
        }),
    )
}

fn http_events(status: i64) -> Vec<CdpEvent> {
    vec![
        event(
            "Network.requestWillBeSent",
            json!({
                "frameId": "fake-frame-1", "loaderId": "new-loader", "requestId": "request",
                "type": "Document", "request": {"url": "https://example.test/reload"},
            }),
        ),
        event(
            "Network.responseReceived",
            json!({
                "frameId": "fake-frame-1", "loaderId": "new-loader", "requestId": "request",
                "type": "Document", "response": {"url": "https://example.test/reload",
                    "status": status, "statusText": "response", "headers": {"x-reload": "yes"}},
            }),
        ),
        commit("new-loader", "https://example.test/reload"),
        lifecycle("DOMContentLoaded"),
        lifecycle("load"),
        event("Network.loadingFinished", json!({"requestId": "request"})),
    ]
}

#[tokio::test(start_paused = true)]
async fn reload_preserves_http_response_for_each_lifecycle_policy() {
    for wait in [
        WaitUntil::Commit,
        WaitUntil::DomContentLoaded,
        WaitUntil::Load,
        WaitUntil::NetworkIdle,
    ] {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html(""));
        fake.set_reload_events(http_events(404));
        let service = CdpNavigationService::new(fake.clone());
        // Act
        let result = service.reload(wait).await.unwrap();
        // Assert
        assert_eq!(result.loader_id.as_deref(), Some("new-loader"));
        assert!(!result.same_document);
        assert_eq!(result.response.unwrap().status, 404);
        assert_eq!(
            fake.method_names(),
            [
                "Page.getFrameTree",
                "Page.enable",
                "Page.setLifecycleEventsEnabled",
                "Network.enable",
                "Page.reload"
            ]
        );
    }
}

#[tokio::test]
async fn old_same_document_foreign_session_and_child_events_cannot_finish_reload() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    let foreign = CdpEvent::new(
        "Page.frameNavigated".to_owned(),
        json!({"frame": {
            "id": "fake-frame-1", "loaderId": "foreign", "url": "about:blank",
        }}),
        Some("other-session".to_owned()),
    );
    let child = event(
        "Page.frameNavigated",
        json!({"frame": {
            "id": "child", "loaderId": "child-loader", "url": "about:blank",
        }}),
    );
    fake.set_reload_events([
        commit("fake-loader", "about:blank"),
        foreign,
        child,
        event(
            "Page.navigatedWithinDocument",
            json!({"frameId": "fake-frame-1", "url": "about:blank#stray"}),
        ),
        commit("new-loader", "data:text/html,new"),
        lifecycle("load"),
    ]);
    let service = CdpNavigationService::new(fake);
    // Act
    let result = service.reload(WaitUntil::Load).await.unwrap();
    // Assert
    assert_eq!(result.loader_id.as_deref(), Some("new-loader"));
    assert_eq!(result.url, "data:text/html,new");
    assert!(result.response.is_none());
}

#[tokio::test]
async fn load_reload_remains_pending_after_commit_until_matching_load() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    fake.set_reload_events([commit("new-loader", "about:blank")]);
    let service = CdpNavigationService::new(fake.clone());
    let mut operation = Box::pin(service.reload(WaitUntil::Load));
    let pending =
        std::future::poll_fn(|cx| std::task::Poll::Ready(operation.as_mut().poll(cx))).await;
    assert!(pending.is_pending());
    // Act
    fake.send_event(lifecycle("load"));
    let result = operation.await.unwrap();
    // Assert
    assert_eq!(result.url, "about:blank");
    assert!(result.response.is_none());
}

#[tokio::test]
async fn document_network_failure_rejects_before_commit() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    let request = http_events(200).remove(0);
    fake.set_reload_events([
        request,
        event(
            "Network.loadingFailed",
            json!({
                "requestId": "request", "errorText": "net::ERR_CONNECTION_RESET",
            }),
        ),
    ]);
    let service = CdpNavigationService::new(fake);
    // Act
    let error = service.reload(WaitUntil::Load).await.unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::Backend);
    assert!(error.message().contains("ERR_CONNECTION_RESET"));
}

#[tokio::test]
async fn oversized_new_loader_is_rejected_before_retention() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    fake.set_reload_events([commit(&"x".repeat(33), "about:blank")]);
    let service =
        CdpNavigationService::with_limits(fake, CdpNavigationLimits::new(128, 32, 4, 4, 128, 32));
    // Act
    let error = service.reload(WaitUntil::Commit).await.unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
}

#[tokio::test]
async fn lagged_reload_stream_fails_instead_of_reconstructing_missing_navigation() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    fake.set_reload_events((0..1025).map(|_| lifecycle("load")));
    let service = CdpNavigationService::new(fake);
    // Act
    let error = service.reload(WaitUntil::Load).await.unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert!(error.message().contains("lagged"));
}

#[tokio::test]
async fn dropping_reload_releases_the_shared_operation_and_byte_admission() {
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
    let mut held = Box::pin(service.reload(WaitUntil::Load));
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(held.as_mut().poll(cx)))
            .await
            .is_pending()
    );
    let denied = service.reload(WaitUntil::Commit).await.unwrap_err();
    assert_eq!(denied.kind(), FailureKind::ResourceExhausted);
    // Act
    drop(held);
    fake.set_reload_events([commit("new-loader", "about:blank")]);
    let result = service.reload(WaitUntil::Commit).await.unwrap();
    // Assert
    assert_eq!(result.url, "about:blank");
    assert_eq!(
        fake.method_names()
            .iter()
            .filter(|name| name.as_str() == "Page.reload")
            .count(),
        2
    );
}
