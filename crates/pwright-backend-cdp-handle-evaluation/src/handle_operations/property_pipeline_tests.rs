use super::{CdpHandlePropertiesService, CdpHandlePropertyLimits};
use crate::{CdpEvaluatedHandle, CdpHandleEvaluationLimits};
use pwright_backend_cdp_evaluation::{CdpEvaluationLimits, CdpEvaluationService};
use pwright_cap_evaluation::EvaluationValueLimits;
use pwright_core::{FailureKind, ObjectId};
use pwright_fake::FakeCdpClient;
use serde_json::{Value, json};
use std::sync::Arc;

fn evaluation_limits() -> CdpEvaluationLimits {
    CdpEvaluationLimits::new(
        4096,
        64 * 1024,
        256,
        EvaluationValueLimits::new(16, 128, 128, 16 * 1024, 16 * 1024).unwrap(),
    )
}

fn service(fake: &Arc<FakeCdpClient>) -> CdpHandlePropertiesService {
    service_with_limits(fake, evaluation_limits())
}

fn service_with_limits(
    fake: &Arc<FakeCdpClient>,
    limits: CdpEvaluationLimits,
) -> CdpHandlePropertiesService {
    CdpHandlePropertiesService::from_evaluation(
        CdpEvaluationService::with_limits_in_unique_context(fake.clone(), "property-world", limits),
        64,
        128,
        CdpHandlePropertyLimits::new(8, 64).unwrap(),
    )
}

fn target(fake: &Arc<FakeCdpClient>) -> CdpEvaluatedHandle {
    crate::handle_evaluation::parse_response(
        fake.clone(),
        json!({"result": {"type": "object", "objectId": "target"}}),
        CdpHandleEvaluationLimits::new(evaluation_limits(), 64, 128),
    )
    .unwrap()
}

fn arrange(fake: &FakeCdpClient, properties: Value) {
    fake.set_evaluate_responses([
        json!({"result": {"type": "object", "objectId": "carrier"}}),
        properties,
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
async fn property_exception_attempts_descriptor_exception_and_carrier_after_release_failure() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    arrange(
        &fake,
        json!({
            "result": [{"name": "child", "enumerable": true,
                "value": {"type": "object", "objectId": "child-owner"}}],
            "exceptionDetails": {"text": "Uncaught", "exception": {
                "type": "object", "objectId": "exception-owner", "description": "Error: property-marker"
            }}
        }),
    );
    fake.set_release_object_error(Some("native release failed".into()));
    let service = service(&fake);
    let target = target(&fake);
    // Act
    let error = service
        .call_on_handle(ObjectId::new(9), target)
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::Cleanup);
    assert!(error.message().contains("property-marker"));
    assert!(error.message().len() <= evaluation_limits().error_bytes);
    assert_eq!(
        releases(&fake),
        ["child-owner", "exception-owner", "carrier"]
    );
}

#[tokio::test]
async fn duplicate_property_native_ids_are_rejected_and_released_once() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    arrange(
        &fake,
        json!({"result": [
            {"name": "first", "enumerable": true,
                "value": {"type": "object", "objectId": "shared-owner"}},
            {"name": "second", "enumerable": true,
                "value": {"type": "object", "objectId": "shared-owner"}}
        ]}),
    );
    let service = service(&fake);
    let target = target(&fake);
    // Act
    let error = service
        .call_on_handle(ObjectId::new(9), target)
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert!(error.message().contains("repeats a native objectId"));
    assert_eq!(releases(&fake), ["shared-owner", "carrier"]);
}

#[tokio::test]
async fn a_property_aliasing_its_carrier_is_rejected_and_released_only_once() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    arrange(
        &fake,
        json!({"result": [{"name": "alias", "enumerable": true,
            "value": {"type": "object", "objectId": "carrier"}}]}),
    );
    let service = service(&fake);
    let target = target(&fake);
    // Act
    let error = service
        .call_on_handle(ObjectId::new(9), target)
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert_eq!(releases(&fake), ["carrier"]);
}

async fn assert_unresolved_property_keeps_cleanup_failure(remote: Value) {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    arrange(
        &fake,
        json!({"result": [
            {"name": "unresolved", "enumerable": true, "value": remote},
            {"name": "valid", "enumerable": true,
                "value": {"type": "object", "objectId": "valid-owner"}}
        ]}),
    );
    let service = service(&fake);
    let target = target(&fake);
    // Act
    let error = service
        .call_on_handle(ObjectId::new(9), target)
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::Cleanup);
    assert!(error.message().contains("unresolved native ownership"));
    assert!(error.message().len() <= evaluation_limits().error_bytes);
    assert_eq!(releases(&fake), ["valid-owner", "carrier"]);
}

#[tokio::test]
async fn invalid_property_native_id_cannot_hide_other_owners_or_report_clean_cleanup() {
    assert_unresolved_property_keeps_cleanup_failure(json!({"type": "object", "objectId": 7}))
        .await;
}

#[tokio::test]
async fn oversized_property_native_id_cannot_hide_other_owners_or_report_clean_cleanup() {
    assert_unresolved_property_keeps_cleanup_failure(
        json!({"type": "object", "objectId": "x".repeat(65)}),
    )
    .await;
}

#[tokio::test]
async fn missing_property_native_id_cannot_hide_other_owners_or_report_clean_cleanup() {
    assert_unresolved_property_keeps_cleanup_failure(json!({"type": "object"})).await;
}

#[tokio::test]
async fn accessor_internal_and_private_owners_are_rejected_and_all_compensated() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    arrange(
        &fake,
        json!({
            "result": [{"name": "child", "enumerable": true,
                "value": {"type": "object", "objectId": "child-owner"},
                "get": {"type": "function", "objectId": "getter-owner"},
                "set": {"type": "function", "objectId": "setter-owner"},
                "symbol": {"type": "symbol", "objectId": "symbol-owner"}}],
            "internalProperties": [{"name": "[[Prototype]]",
                "value": {"type": "object", "objectId": "internal-owner"}}],
            "privateProperties": [{"name": "#private",
                "value": {"type": "object", "objectId": "private-owner"}}]
        }),
    );
    let service = service(&fake);
    let target = target(&fake);
    // Act
    let error = service
        .call_on_handle(ObjectId::new(9), target)
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert!(error.message().contains("unexpected retained owners"));
    assert_eq!(
        releases(&fake),
        [
            "child-owner",
            "getter-owner",
            "setter-owner",
            "symbol-owner",
            "internal-owner",
            "private-owner",
            "carrier"
        ]
    );
}

#[tokio::test]
async fn oversized_aggregate_property_staging_rejects_before_copying_and_compensates_ids() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    let oversized =
        usize::try_from(evaluation_limits().maximum_retained_bytes_per_operation() / 16).unwrap()
            + 1;
    arrange(
        &fake,
        json!({
            "result": [
                {"name": "first", "enumerable": true,
                    "value": {"type": "object", "objectId": "first-owner"}},
                {"name": "second", "enumerable": true,
                    "value": {"type": "object", "objectId": "second-owner"}}
            ],
            "diagnosticPadding": "x".repeat(oversized)
        }),
    );
    let service = service(&fake);
    let target = target(&fake);
    // Act
    let error = service
        .call_on_handle(ObjectId::new(9), target)
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert!(error.message().contains("property staging"));
    assert_eq!(releases(&fake), ["first-owner", "second-owner", "carrier"]);
}

#[tokio::test]
async fn deeply_nested_property_response_is_rejected_with_all_valid_owners_compensated() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    let mut padding = Value::Null;
    for _ in 0..129 {
        padding = json!({"x": padding});
    }
    arrange(
        &fake,
        json!({
            "result": [
                {"name": "first", "enumerable": true,
                    "value": {"type": "object", "objectId": "first-owner"}},
                {"name": "second", "enumerable": true,
                    "value": {"type": "object", "objectId": "second-owner"}}
            ],
            "diagnosticPadding": padding
        }),
    );
    // Keep byte capacity above these small map nodes so this exercises the
    // independent depth boundary, not the aggregate byte-bound regression.
    let mut limits = evaluation_limits();
    limits.generated_script_bytes = 128 * 1024;
    let service = service_with_limits(&fake, limits);
    let target = target(&fake);
    // Act
    let error = service
        .call_on_handle(ObjectId::new(9), target)
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert!(error.message().contains("property staging"));
    assert_eq!(releases(&fake), ["first-owner", "second-owner", "carrier"]);
}

#[tokio::test]
async fn malformed_nonarray_property_result_still_compensates_its_native_owner() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    arrange(
        &fake,
        json!({"result": {"type": "object", "objectId": "malformed-owner"}}),
    );
    let service = service(&fake);
    let target = target(&fake);
    // Act
    let error = service
        .call_on_handle(ObjectId::new(9), target)
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert_eq!(releases(&fake), ["malformed-owner", "carrier"]);
}
