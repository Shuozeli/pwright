use super::{CdpHandlePropertiesService, CdpHandlePropertyLimits};
use crate::{CdpEvaluatedHandle, CdpHandleEvaluationLimits, CdpHandleKind};
use pwright_backend_cdp_evaluation::{CdpEvaluationLimits, CdpEvaluationService};
use pwright_cap_handle_evaluation::HandleEvaluationErrorDetails;
use pwright_core::{FailureKind, ObjectId};
use pwright_fake::FakeCdpClient;
use serde_json::{Value, json};
use std::sync::Arc;

fn handle(fake: &Arc<FakeCdpClient>, remote: Value) -> CdpEvaluatedHandle {
    let client: Arc<dyn pwright_cdp::CdpClient> = fake.clone();
    crate::handle_evaluation::parse_response(
        client,
        json!({ "result": remote }),
        CdpHandleEvaluationLimits::default(),
    )
    .unwrap()
}

fn service(fake: &Arc<FakeCdpClient>) -> CdpHandlePropertiesService {
    let client: Arc<dyn pwright_cdp::CdpClient> = fake.clone();
    CdpHandlePropertiesService::from_evaluation(
        CdpEvaluationService::with_limits_in_unique_context(
            client,
            "property-world",
            CdpEvaluationLimits::default(),
        ),
        4096,
        4096,
        CdpHandlePropertyLimits::new(2, 32).unwrap(),
    )
}

fn arrange_responses(fake: &FakeCdpClient, descriptors: Value) {
    fake.set_evaluate_responses([
        json!({ "result": { "type": "object", "objectId": "snapshot", "description": "Object" } }),
        json!({ "result": descriptors }),
    ]);
}

fn releases(fake: &FakeCdpClient) -> Vec<String> {
    fake.calls()
        .into_iter()
        .filter(|(method, _)| method == "Runtime.releaseObject")
        .map(|(_, arguments)| arguments[0].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn property_promise_keeps_its_native_identity_instead_of_returning_its_fulfillment() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    arrange_responses(
        &fake,
        json!([{
            "name": "promise",
            "enumerable": true,
            "value": { "type": "object", "subtype": "promise", "objectId": "promise-value", "description": "Promise" }
        }]),
    );
    let target = handle(&fake, json!({ "type": "object", "objectId": "target" }));
    let service = service(&fake);

    // Act
    let property = service
        .call_property_on_handle(ObjectId::new(9), target, "promise")
        .await
        .unwrap();

    // Assert
    assert_eq!(property.object_id(), Some("promise-value"));
    assert_eq!(property.kind(), CdpHandleKind::Js);
    assert_eq!(property.preview(), "Promise");
    assert_eq!(
        property.call_argument(),
        json!({ "objectId": "promise-value" })
    );
    assert_eq!(releases(&fake), ["snapshot"]);
    let calls = fake.calls();
    assert_eq!(calls[0].0, "Runtime.callFunctionOn");
    assert_eq!(calls[0].1[0]["returnByValue"], false);
    let source = calls[0].1[0]["arguments"][0]["value"]["source"]
        .as_str()
        .unwrap();
    assert!(source.contains("{ __proto__: null }"));
    assert!(source.contains("target[name]"));
    assert_ne!(source.trim(), "(target, name) => target[name]");
    assert_eq!(calls[1].0, "Runtime.getProperties");
    assert_eq!(calls[1].1[0], "snapshot");
}

#[tokio::test]
async fn function_property_keeps_the_returned_function_as_a_native_handle() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    arrange_responses(
        &fake,
        json!([{
            "name": "method",
            "enumerable": true,
            "value": { "type": "function", "objectId": "then-function", "description": "function then() {}" }
        }]),
    );
    let target = handle(&fake, json!({ "type": "object", "objectId": "thenable" }));
    let service = service(&fake);

    // Act
    let property = service
        .call_property_on_handle(ObjectId::new(9), target, "method")
        .await
        .unwrap();

    // Assert
    assert_eq!(property.object_id(), Some("then-function"));
    assert_eq!(property.preview(), "function then() {}");
    assert_eq!(releases(&fake), ["snapshot"]);
    assert_eq!(fake.method_names().len(), 3);
}

#[tokio::test]
async fn primitive_target_property_is_evaluated_instead_of_short_circuiting_to_empty() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    arrange_responses(
        &fake,
        json!([{ "name": "length", "enumerable": true, "value": { "type": "number", "value": 3 } }]),
    );
    let target = handle(&fake, json!({ "type": "string", "value": "abc" }));
    let service = service(&fake);

    // Act
    let property = service
        .call_property_on_handle(ObjectId::new(9), target, "length")
        .await
        .unwrap();

    // Assert
    assert_eq!(property.object_id(), None);
    assert_eq!(property.call_argument(), json!({ "value": 3 }));
    assert_eq!(fake.calls()[0].1[0]["arguments"][1]["value"], "abc");
    assert_eq!(releases(&fake), ["snapshot"]);
}

#[tokio::test]
async fn oversized_utf8_property_name_rejects_before_any_native_io() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    let target = handle(&fake, json!({ "type": "object", "objectId": "target" }));
    let service = service(&fake);
    let name = "é".repeat(17);

    // Act
    let result = service
        .call_property_on_handle(ObjectId::new(9), target, &name)
        .await;

    // Assert
    assert!(result.is_err());
    assert!(fake.calls().is_empty());
}

#[tokio::test]
async fn mismatched_property_name_compensates_the_native_property_and_snapshot() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    arrange_responses(
        &fake,
        json!([{ "name": "unexpected", "enumerable": true, "value": { "type": "object", "objectId": "unpublished" } }]),
    );
    let target = handle(&fake, json!({ "type": "object", "objectId": "target" }));
    let service = service(&fake);

    // Act
    let error = service
        .call_property_on_handle(ObjectId::new(9), target, "expected")
        .await
        .unwrap_err();

    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert_eq!(releases(&fake), ["unpublished", "snapshot"]);
}

#[tokio::test]
async fn malformed_property_descriptor_compensates_its_raw_object_and_snapshot() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    arrange_responses(
        &fake,
        json!([{ "enumerable": true, "value": { "type": "object", "objectId": "malformed-property" } }]),
    );
    let target = handle(&fake, json!({ "type": "object", "objectId": "target" }));
    let service = service(&fake);

    // Act
    let error = service
        .call_property_on_handle(ObjectId::new(9), target, "expected")
        .await
        .unwrap_err();

    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert_eq!(releases(&fake), ["malformed-property", "snapshot"]);
}

#[tokio::test]
async fn extra_property_descriptor_compensates_all_raw_results_and_snapshot() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    arrange_responses(
        &fake,
        json!([
            { "name": "expected", "enumerable": true, "value": { "type": "object", "objectId": "first" } },
            { "name": "extra", "enumerable": true, "value": { "type": "object", "objectId": "second" } }
        ]),
    );
    let target = handle(&fake, json!({ "type": "object", "objectId": "target" }));
    let service = service(&fake);

    // Act
    let result = service
        .call_property_on_handle(ObjectId::new(9), target, "expected")
        .await;

    // Assert
    assert!(result.is_err());
    assert_eq!(releases(&fake), ["first", "second", "snapshot"]);
}

#[tokio::test]
async fn missing_property_descriptor_rejects_and_releases_the_snapshot() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    arrange_responses(&fake, json!([]));
    let target = handle(&fake, json!({ "type": "object", "objectId": "target" }));
    let service = service(&fake);

    // Act
    let error = service
        .call_property_on_handle(ObjectId::new(9), target, "expected")
        .await
        .unwrap_err();

    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert_eq!(releases(&fake), ["snapshot"]);
}

#[tokio::test]
async fn snapshot_release_failure_compensates_the_property_and_preserves_cleanup_failure() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    arrange_responses(
        &fake,
        json!([{ "name": "expected", "enumerable": true, "value": { "type": "object", "objectId": "unpublished" } }]),
    );
    fake.set_release_object_error(Some("injected native release failure".to_owned()));
    let target = handle(&fake, json!({ "type": "object", "objectId": "target" }));
    let service = service(&fake);

    // Act
    let error = service
        .call_property_on_handle(ObjectId::new(9), target, "expected")
        .await
        .unwrap_err();

    // Assert
    assert_eq!(releases(&fake), ["snapshot", "unpublished"]);
    assert_eq!(error.kind(), FailureKind::Cleanup);
    assert!(matches!(
        error.details(),
        Some(HandleEvaluationErrorDetails::Cleanup { primary, cleanup })
            if primary.contains("injected native release failure")
                && cleanup.contains("injected native release failure")
    ));
}

#[tokio::test]
async fn primitive_property_does_not_hide_a_failed_snapshot_release() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    arrange_responses(
        &fake,
        json!([{ "name": "expected", "enumerable": true, "value": { "type": "number", "value": 7 } }]),
    );
    fake.set_release_object_error(Some("injected snapshot release failure".to_owned()));
    let target = handle(&fake, json!({ "type": "object", "objectId": "target" }));
    let service = service(&fake);

    // Act
    let error = service
        .call_property_on_handle(ObjectId::new(9), target, "expected")
        .await
        .unwrap_err();

    // Assert
    assert_eq!(releases(&fake), ["snapshot"]);
    assert_eq!(error.kind(), FailureKind::Cleanup);
    assert!(
        error
            .message()
            .contains("injected snapshot release failure")
    );
}
