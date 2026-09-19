use super::*;
use pwright_backend_cdp_locator_resolution::{
    CdpLocatorResolutionService, ResolutionProfile, ResolutionRequest,
};
use pwright_core::{AtomicObjectIdSource, ResourceLimits, Service};
use pwright_fake::FakeCdpClient;
use pwright_file_input_model::FileInputLimits;
use pwright_file_input_source::{Content, PayloadSource};
use pwright_selector_model::Selector;
use serde_json::json;
use std::sync::Arc;

fn production(requirements: ResourceRequirements) -> ProductionResourceLedger {
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let bound = requirements.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    ProductionResourceLedger::new(limits, requirements).unwrap()
}
async fn element() -> (
    ResolvedElement,
    Arc<FakeCdpClient>,
    ProductionResourceLedger,
) {
    let profile = ResolutionProfile::new(ResourceScope::Connection, 1).unwrap();
    let resources = production(profile.requirements());
    let resolver = CdpLocatorResolutionService::new(
        profile,
        &resources,
        Arc::new(AtomicObjectIdSource::new(1).unwrap()),
    )
    .unwrap();
    let client = Arc::new(FakeCdpClient::from_html("<input type=file>"));
    client.set_evaluate_response(
        json!({"result":{"type":"object","subtype":"node","objectId":"input-1"}}),
    );
    let element = resolver
        .call(ResolutionRequest {
            client: client.clone(),
            utility_world: "exact-utility".into(),
            selector: Selector::css("input").unwrap(),
            destruction: None,
        })
        .await
        .unwrap()
        .unwrap();
    (element, client, resources)
}
fn payloads() -> (PreparedPayloads, ResourceLedger) {
    payloads_with_mime("text/plain")
}
fn payloads_with_mime(mime: &str) -> (PreparedPayloads, ResourceLedger) {
    let ledger = ResourceLedger::new(
        ResourceLimits::deny_all(1, 4)
            .with_class(ResourceClass::LiveObject, 1, 1)
            .with_class(ResourceClass::ArtifactBytes, 16, 16)
            .with_class(ResourceClass::BackendStateBytes, 8192, 8192),
    );
    let value = PreparedPayloads::materialize(
        &ledger,
        ResourceScope::Connection,
        FileInputLimits::new(4, 512, 128, 128, 512).unwrap(),
        &mut [PayloadSource {
            name: "test.txt",
            mime_type: mime,
            last_modified_ms: Some(1234.5),
            content: Content::Bytes(&[0, 255, 1]),
        }],
    )
    .unwrap();
    (value, ledger)
}
fn backend(bytes: u64) -> (CdpFilePayloadService, ProductionResourceLedger) {
    let profile = PayloadProfile::new(ResourceScope::Connection, 1, 4, bytes, 512).unwrap();
    let resources = production(profile.requirements().unwrap());
    (
        CdpFilePayloadService::new(profile, &resources).unwrap(),
        resources,
    )
}
fn response(status: &str, input: bool, multiple: bool, directory: bool) -> serde_json::Value {
    json!({"result":{"value":{"status":status,"target":{"input":input,"multiple":multiple,"directory":directory}}}})
}

#[tokio::test]
async fn borrowed_target_dispatches_without_selector_resolution_or_native_release() {
    // Arrange: no Locator, resolver or owned resolution group is constructed.
    let client = FakeCdpClient::from_html("<input type=file>");
    client.set_evaluate_response(response("applied", true, false, false));
    let target =
        PayloadTarget::new(&client, "retained-world", "retained-node", "upload-group").unwrap();
    let (payloads, source) = payloads();
    let (backend, resources) = backend(32);
    // Act
    let result = backend.apply_on_target(target, &payloads).await;
    // Assert: one exact mutation; caller still owns source and native lifetime.
    assert_eq!(result.unwrap(), PayloadOutcome::Applied);
    let calls = client.calls();
    assert_eq!(calls.len(), 1);
    let request = &calls[0].1[0];
    assert_eq!(request["uniqueContextId"], "retained-world");
    assert_eq!(request["objectGroup"], "upload-group");
    assert_eq!(request["arguments"][0], json!({"objectId":"retained-node"}));
    assert_eq!(request["arguments"][1]["value"][0]["buffer"], "AP8B");
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    assert_eq!(
        source
            .metrics()
            .unwrap()
            .class(ResourceClass::ArtifactBytes)
            .current,
        3
    );
    drop(payloads);
    assert_eq!(source.metrics().unwrap().active_leases, 0);
}

#[test]
fn borrowed_target_rejects_empty_identifiers_without_native_io() {
    // Arrange
    let client = FakeCdpClient::from_html("");
    for ids in [
        ["", "node", "group"],
        ["world", "", "group"],
        ["world", "node", ""],
    ] {
        // Act
        let result = PayloadTarget::new(&client, ids[0], ids[1], ids[2]);
        // Assert
        assert!(matches!(result, Err(PayloadError::InvalidTarget)));
        assert!(client.calls().is_empty());
    }
}

#[test]
fn borrowed_target_checks_utf8_bytes_without_copying_native_identifiers() {
    // Arrange
    let client = FakeCdpClient::from_html("");
    let overlong = "界".repeat(MAX_NATIVE_ID / 3 + 1);
    for ids in [
        [overlong.as_str(), "node", "group"],
        ["world", &overlong, "group"],
        ["world", "node", &overlong],
    ] {
        // Act
        let result = PayloadTarget::new(&client, ids[0], ids[1], ids[2]);
        // Assert
        assert!(matches!(result, Err(PayloadError::Capacity)));
        assert!(client.calls().is_empty());
    }
}

#[tokio::test]
async fn borrowed_target_uses_the_same_payload_capacity_admission() {
    // Arrange
    let client = FakeCdpClient::from_html("");
    let target = PayloadTarget::new(&client, "world", "node", "group").unwrap();
    let (payloads, _) = payloads();
    let (backend, resources) = backend(3);
    // Act
    let result = backend.apply_on_target(target, &payloads).await;
    // Assert
    assert!(matches!(result, Err(PayloadError::Capacity)));
    assert!(client.calls().is_empty());
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn payload_dispatch_uses_exact_world_original_object_and_binary_encoding() {
    // Arrange
    let (element, client, native) = element().await;
    let (payloads, source) = payloads();
    let (backend, resources) = backend(32);
    client.set_evaluate_response(response("applied", true, false, false));
    // Act
    let result = backend.apply(&element, &payloads).await.unwrap();
    // Assert
    assert_eq!(result, PayloadOutcome::Applied);
    let calls = client.calls();
    let (_, arguments) = calls.last().unwrap();
    let request = &arguments[0];
    assert_eq!(request["uniqueContextId"], "exact-utility");
    assert_eq!(request["objectGroup"], element.object_group());
    assert_eq!(request["returnByValue"], true);
    assert_eq!(request["arguments"][0], json!({"objectId":"input-1"}));
    assert_eq!(
        request["arguments"][1]["value"][0],
        json!({"name":"test.txt","mimeType":"text/plain","buffer":"AP8B","lastModifiedMs":1234.5})
    );
    assert!(
        request["functionDeclaration"]
            .as_str()
            .unwrap()
            .contains("new DataTransfer")
    );
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    assert_eq!(
        source
            .metrics()
            .unwrap()
            .class(ResourceClass::ArtifactBytes)
            .current,
        3
    );
    element.release().await.unwrap();
    drop(payloads);
    assert_eq!(native.metrics().unwrap().active_leases, 0);
    assert_eq!(source.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn backend_capacity_rejects_before_a_mutation_command() {
    // Arrange
    let (element, client, _) = element().await;
    let (payloads, _) = payloads();
    let (backend, resources) = backend(3);
    let before = client.calls().len();
    // Act
    let result = backend.apply(&element, &payloads).await;
    // Assert
    assert!(matches!(result, Err(PayloadError::Capacity)));
    assert_eq!(client.calls().len(), before);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    element.release().await.unwrap();
}

#[tokio::test]
async fn budget_exhaustion_rejects_before_a_mutation_command() {
    // Arrange
    let (element, client, _) = element().await;
    let (payloads, _) = payloads();
    let (backend, resources) = backend(32);
    let ledger = resources
        .claim(
            PayloadProfile::new(ResourceScope::Connection, 1, 4, 32, 512)
                .unwrap()
                .requirements()
                .unwrap(),
        )
        .unwrap();
    let occupied = ledger
        .reserve(ResourceScope::Connection, ResourceClass::ActiveCommand, 1)
        .unwrap();
    let before = client.calls().len();
    // Act
    let result = backend.apply(&element, &payloads).await;
    // Assert
    assert!(matches!(result, Err(PayloadError::Budget(_))));
    assert_eq!(client.calls().len(), before);
    drop(occupied);
    element.release().await.unwrap();
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn rejected_target_is_classified_using_the_rust_policy() {
    // Arrange
    let (element, client, _) = element().await;
    let (payloads, _) = payloads();
    let (backend, _) = backend(32);
    client.set_evaluate_response(response("rejected", true, false, true));
    // Act
    let result = backend.apply(&element, &payloads).await;
    // Assert
    assert!(matches!(
        result,
        Err(PayloadError::Policy(FileInputError::DirectoryRequired))
    ));
    element.release().await.unwrap();
}

#[tokio::test]
async fn text_input_noop_is_preserved() {
    // Arrange
    let (element, client, _) = element().await;
    let (payloads, _) = payloads();
    let (backend, _) = backend(32);
    client.set_evaluate_response(response("noop", true, false, false));
    // Act
    let result = backend.apply(&element, &payloads).await.unwrap();
    // Assert
    assert_eq!(result, PayloadOutcome::Noop);
    element.release().await.unwrap();
}

#[tokio::test]
async fn inconsistent_success_snapshot_is_a_protocol_error() {
    // Arrange
    let (element, client, _) = element().await;
    let (payloads, _) = payloads();
    let (backend, _) = backend(32);
    client.set_evaluate_response(response("applied", false, false, false));
    // Act
    let result = backend.apply(&element, &payloads).await;
    // Assert
    assert!(matches!(result, Err(PayloadError::Protocol)));
    element.release().await.unwrap();
}

#[tokio::test]
async fn large_unrecognized_status_is_rejected_without_echoing_browser_text() {
    // Arrange
    let (element, client, _) = element().await;
    let (payloads, _) = payloads();
    let (backend, _) = backend(32);
    client.set_evaluate_response(response(&"x".repeat(8192), true, false, false));
    // Act
    let result = backend.apply(&element, &payloads).await;
    // Assert
    assert!(matches!(result, Err(PayloadError::Protocol)));
    element.release().await.unwrap();
}

#[tokio::test]
async fn nonfinite_modification_time_rejects_before_dispatch() {
    // Arrange
    let (element, client, _) = element().await;
    let (mut payloads, _) = payloads();
    let (backend, _) = backend(32);
    payloads.set_last_modified_ms(0, Some(f64::NAN)).unwrap();
    let before = client.calls().len();
    // Act
    let result = backend.apply(&element, &payloads).await;
    // Assert
    assert!(matches!(result, Err(PayloadError::InvalidModifiedTime)));
    assert_eq!(client.calls().len(), before);
    element.release().await.unwrap();
}

#[tokio::test]
async fn backend_error_is_preserved_and_releases_scratch_admission() {
    // Arrange
    let (element, client, _) = element().await;
    let (payloads, _) = payloads();
    let (backend, resources) = backend(32);
    client.set_evaluate_errors(["native failure".to_string()]);
    // Act
    let result = backend.apply(&element, &payloads).await;
    // Assert
    assert!(matches!(
        result,
        Err(PayloadError::Backend(CdpError::Protocol {
            code: -32000,
            ..
        }))
    ));
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    element.release().await.unwrap();
}

#[test]
fn overflowing_profile_rejects_without_allocating() {
    // Arrange
    let scope = ResourceScope::Connection;
    // Act
    let result = PayloadProfile::new(scope, 1, 1, u64::MAX, 1);
    // Assert
    assert!(matches!(result, Err(PayloadError::InvalidProfile)));
}

#[tokio::test]
async fn unresolved_mime_rejects_before_browser_dispatch() {
    // Arrange
    let (element, client, _) = element().await;
    let (payloads, _) = payloads_with_mime("");
    let (backend, _) = backend(32);
    let before = client.calls().len();
    // Act
    let result = backend.apply(&element, &payloads).await;
    // Assert
    assert!(matches!(result, Err(PayloadError::UnresolvedMime)));
    assert_eq!(client.calls().len(), before);
    element.release().await.unwrap();
}

#[tokio::test]
async fn pending_mutation_retains_scratch_and_rejects_an_overlapping_borrowed_target() {
    use std::future::Future;
    // Arrange
    let (element, client, _) = element().await;
    let (payloads, source) = payloads();
    let (backend, resources) = backend(32);
    client.set_evaluate_response(response("applied", true, false, false));
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_evaluate_gate(Some(gate.clone()));
    let mut pending = Box::pin(backend.apply(&element, &payloads));
    assert!(
        pending
            .as_mut()
            .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
            .is_pending()
    );
    let before = client.calls().len();
    // Act
    let target = PayloadTarget::new(
        client.as_ref(),
        "same-world",
        "another-node",
        "another-group",
    )
    .unwrap();
    let rejected = backend.apply_on_target(target, &payloads).await;
    // Assert
    assert!(matches!(rejected, Err(PayloadError::Budget(_))));
    assert_eq!(client.calls().len(), before);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::ActiveCommand)
            .current,
        1
    );
    assert_eq!(
        source
            .metrics()
            .unwrap()
            .class(ResourceClass::ArtifactBytes)
            .current,
        3
    );
    gate.notify_waiters();
    assert_eq!(pending.await.unwrap(), PayloadOutcome::Applied);
    client.set_evaluate_gate(None);
    element.release().await.unwrap();
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}
