use super::*;
use pwright_core::ResourceLimits;
use pwright_fake::FakeCdpClient;
use serde_json::json;

fn graph() -> Value {
    json!({
        "a": [{
            "o": [
                {"k":"name","v":"one.txt"},
                {"k":"mimeType","v":"text/plain"},
                {"k":"buffer","v":"AQI="},
                {"k":"lastModifiedMs","v":{"v":"undefined"}}
            ],
            "id": 2
        }],
        "id": 1
    })
}

fn service() -> (
    CdpSerializedPayloadMutationService,
    ProductionResourceLedger,
) {
    let profile = SerializedPayloadMutationProfile::new(ResourceScope::Connection, 1).unwrap();
    let required = profile.requirements().unwrap();
    let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
    for class in ResourceClass::ALL {
        let amount = required.class(class);
        limits = limits.with_class(class, amount.global, amount.per_scope);
    }
    let resources = ProductionResourceLedger::new(limits, required).unwrap();
    let service = CdpSerializedPayloadMutationService::new(profile, &resources).unwrap();
    (service, resources)
}

#[tokio::test]
async fn dispatches_one_large_user_gesture_call_on_the_exact_retained_target() {
    // Arrange
    let client = FakeCdpClient::from_html("");
    client.set_evaluate_response(json!({"result":{"type":"undefined"}}));
    let target = PayloadTarget::new(&client, "utility-world", "retargeted-input", "group").unwrap();
    let (service, resources) = service();

    // Act
    let result = service.apply_on_target(target, graph()).await;

    // Assert
    result.unwrap();
    let calls = client.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "Runtime.callFunctionOn");
    let request = &calls[0].1[0];
    assert_eq!(request["uniqueContextId"], "utility-world");
    assert_eq!(request["objectGroup"], "group");
    assert_eq!(request["userGesture"], true);
    assert_eq!(
        request["arguments"][0],
        json!({"objectId":"retargeted-input"})
    );
    assert_eq!(request["arguments"][1]["value"], graph());
    let source = request["functionDeclaration"].as_str().unwrap();
    assert!(!source.contains("retarget("));
    assert!(!source.contains("DOM.setFileInputFiles"));
    assert!(source.contains("new DataTransfer"));
    assert!(source.contains("new Event('input', { bubbles: true, composed: true })"));
    assert!(source.contains("new Event('change', { bubbles: true })"));
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn ignores_the_injected_helpers_late_non_file_return_value() {
    // Arrange
    let client = FakeCdpClient::from_html("");
    client.set_evaluate_response(json!({"result":{"value":"Not an input[type=file] element"}}));
    let target = PayloadTarget::new(&client, "world", "input", "group").unwrap();
    let (service, resources) = service();

    // Act
    let result = service.apply_on_target(target, graph()).await;

    // Assert
    assert!(result.is_ok());
    assert_eq!(client.calls().len(), 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn accepts_integral_float_ids_created_by_the_napi_json_import_boundary() {
    // Arrange
    let client = FakeCdpClient::from_html("");
    client.set_evaluate_response(json!({"result":{"type":"undefined"}}));
    let target = PayloadTarget::new(&client, "world", "input", "group").unwrap();
    let (service, resources) = service();
    let mut imported = graph();
    imported["id"] = json!(1.0);
    imported["a"][0]["id"] = json!(2.0);

    // Act
    let result = service.apply_on_target(target, imported).await;

    // Assert
    result.unwrap();
    assert_eq!(client.calls().len(), 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn rejects_malformed_graph_before_native_io() {
    // Arrange
    let client = FakeCdpClient::from_html("");
    let target = PayloadTarget::new(&client, "world", "input", "group").unwrap();
    let (service, resources) = service();
    let mut malformed = graph();
    malformed["a"][0]["o"][2]["v"] = json!("not base64");

    // Act
    let result = service.apply_on_target(target, malformed).await;

    // Assert
    assert!(matches!(
        result,
        Err(SerializedPayloadMutationError::Capacity)
    ));
    assert!(client.calls().is_empty());
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn propagates_native_failure_and_releases_operation_admission() {
    // Arrange
    let client = FakeCdpClient::from_html("");
    client.set_evaluate_errors(["utility execution failed".to_owned()]);
    let target = PayloadTarget::new(&client, "world", "input", "group").unwrap();
    let (service, resources) = service();

    // Act
    let result = service.apply_on_target(target, graph()).await;

    // Assert
    assert!(matches!(
        result,
        Err(SerializedPayloadMutationError::Backend(_))
    ));
    assert_eq!(client.calls().len(), 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}
