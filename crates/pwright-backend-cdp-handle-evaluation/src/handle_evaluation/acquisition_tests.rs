//! Native acquisition failures exercised through public service entrypoints.

use super::{CdpHandleEvaluationLimits, CdpHandleEvaluationService};
use crate::{CdpHandlePropertiesService, CdpHandlePropertyLimits};
use pwright_backend_cdp_evaluation::{CdpEvaluationLimits, CdpEvaluationService};
use pwright_cap_handle_evaluation::{EvaluateHandleParams, HandleEvaluationErrorDetails};
use pwright_core::{FailureKind, ObjectId, OperationError, Service};
use pwright_fake::FakeCdpClient;
use serde_json::{Value, json};
use std::sync::Arc;

const NATIVE_ID_BYTES: usize = 32;
const ERROR_BYTES: usize = 256;
const PREVIEW_BYTES: usize = 64;

fn limits() -> CdpEvaluationLimits {
    CdpEvaluationLimits {
        error_bytes: ERROR_BYTES,
        ..CdpEvaluationLimits::default()
    }
}
fn evaluation(fake: &Arc<FakeCdpClient>) -> CdpEvaluationService {
    CdpEvaluationService::with_limits_in_unique_context(fake.clone(), "acquisition-world", limits())
}
fn service(fake: &Arc<FakeCdpClient>) -> CdpHandleEvaluationService {
    CdpHandleEvaluationService::from_evaluation(evaluation(fake), NATIVE_ID_BYTES, PREVIEW_BYTES)
}
fn properties(fake: &Arc<FakeCdpClient>) -> CdpHandlePropertiesService {
    CdpHandlePropertiesService::from_evaluation(
        evaluation(fake),
        NATIVE_ID_BYTES,
        PREVIEW_BYTES,
        CdpHandlePropertyLimits::new(2, 32).unwrap(),
    )
}
fn releases(fake: &FakeCdpClient) -> Vec<String> {
    fake.calls()
        .into_iter()
        .filter(|(method, _)| method == "Runtime.releaseObject")
        .map(|(_, arguments)| arguments[0].as_str().unwrap().to_owned())
        .collect()
}
fn assert_bounded(error: &OperationError<HandleEvaluationErrorDetails>) {
    assert!(error.message().len() <= ERROR_BYTES);
    match error.details() {
        None => {}
        Some(HandleEvaluationErrorDetails::Javascript {
            name,
            message,
            stack,
        }) => {
            assert!(name.len() <= ERROR_BYTES);
            assert!(message.len() <= ERROR_BYTES);
            assert!(
                stack
                    .as_ref()
                    .is_none_or(|value| value.len() <= ERROR_BYTES)
            );
        }
        Some(
            HandleEvaluationErrorDetails::InvalidValue(value)
            | HandleEvaluationErrorDetails::InvalidBackendResponse(value)
            | HandleEvaluationErrorDetails::Backend(value),
        ) => assert!(value.len() <= ERROR_BYTES),
        Some(HandleEvaluationErrorDetails::Cleanup { primary, cleanup }) => {
            assert!(primary.len() <= ERROR_BYTES);
            assert!(cleanup.len() <= ERROR_BYTES);
        }
    }
}
fn exception_response(result_id: &str, exception_id: &str) -> Value {
    json!({
        "result": { "type": "object", "objectId": result_id },
        "exceptionDetails": {
            "text": "Uncaught",
            "exception": { "type": "object", "subtype": "error", "objectId": exception_id,
                "description": "Error: primary sentinel" }
        }
    })
}

#[tokio::test]
async fn missing_type_releases_the_acquired_result_exactly_once() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    fake.set_evaluate_response(json!({ "result": { "objectId": "untyped-result" } }));
    let service = CdpHandleEvaluationService::with_limits(
        fake.clone(),
        CdpHandleEvaluationLimits::new(limits(), NATIVE_ID_BYTES, PREVIEW_BYTES),
    );
    // Act
    let error = service
        .call(EvaluateHandleParams::expression("({})"))
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert!(error.message().contains("type"));
    assert_eq!(releases(&fake), ["untyped-result"]);
    assert_eq!(
        fake.method_names(),
        ["Runtime.evaluate", "Runtime.releaseObject"]
    );
    assert_bounded(&error);
}

#[tokio::test]
async fn exception_releases_both_distinct_native_result_and_exception_ids() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    fake.set_evaluate_response(exception_response("result-object", "exception-object"));
    let service = service(&fake);
    // Act
    let error = service
        .call(EvaluateHandleParams::expression(
            "throw new Error('primary sentinel')",
        ))
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::Backend);
    assert!(
        matches!(error.details(), Some(HandleEvaluationErrorDetails::Javascript { message, .. }) if message.contains("primary sentinel"))
    );
    let mut released = releases(&fake);
    released.sort();
    assert_eq!(released, ["exception-object", "result-object"]);
    assert_eq!(fake.method_names()[0], "Runtime.callFunctionOn");
    assert_bounded(&error);
}

#[tokio::test]
async fn identical_result_and_exception_ids_are_released_once() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    fake.set_evaluate_response(exception_response("shared-object", "shared-object"));
    let service = service(&fake);
    // Act
    let error = service
        .call(EvaluateHandleParams::expression(
            "throw new Error('primary sentinel')",
        ))
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::Backend);
    assert_eq!(releases(&fake), ["shared-object"]);
    assert_bounded(&error);
}

#[tokio::test]
async fn primitive_exception_is_reported_without_inventing_a_native_release() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    fake.set_evaluate_response(json!({
        "result": { "type": "undefined" },
        "exceptionDetails": { "text": "Uncaught",
            "exception": { "type": "string", "value": "primitive sentinel" } }
    }));
    let service = service(&fake);
    // Act
    let error = service
        .call(EvaluateHandleParams::expression(
            "throw 'primitive sentinel'",
        ))
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::Backend);
    assert!(
        matches!(error.details(), Some(HandleEvaluationErrorDetails::Javascript { message, .. }) if message.contains("primitive sentinel"))
    );
    assert!(releases(&fake).is_empty());
    assert_bounded(&error);
}

async fn assert_unresolved_identity(remote: Value) {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    fake.set_evaluate_response(json!({ "result": remote }));
    let service = service(&fake);
    // Act
    let error = service
        .call(EvaluateHandleParams::expression("({})"))
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::Cleanup);
    assert!(matches!(
        error.details(),
        Some(HandleEvaluationErrorDetails::Cleanup { .. })
    ));
    assert!(
        releases(&fake).is_empty(),
        "an unusable native ID must never be submitted for release"
    );
    assert_eq!(fake.method_names(), ["Runtime.callFunctionOn"]);
    assert_bounded(&error);
}

#[tokio::test]
async fn missing_object_id_reports_unresolved_native_ownership() {
    assert_unresolved_identity(json!({ "type": "object", "subtype": "node" })).await;
}
#[tokio::test]
async fn missing_function_id_reports_unresolved_native_ownership() {
    assert_unresolved_identity(
        json!({ "type": "function", "description": "function retained() {}" }),
    )
    .await;
}
#[tokio::test]
async fn empty_native_id_reports_cleanup_without_invalid_release() {
    assert_unresolved_identity(json!({ "type": "object", "objectId": "" })).await;
}
#[tokio::test]
async fn non_string_native_id_reports_cleanup_without_invalid_release() {
    assert_unresolved_identity(json!({ "type": "object", "objectId": 17 })).await;
}

#[tokio::test]
async fn oversized_native_id_is_not_copied_into_cleanup_commands_or_diagnostics() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    let native_id = "untrusted-native-é".repeat(512);
    fake.set_evaluate_response(json!({ "result": { "type": "object", "objectId": native_id } }));
    let service = service(&fake);
    // Act
    let error = service
        .call(EvaluateHandleParams::expression("({})"))
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::Cleanup);
    assert!(releases(&fake).is_empty());
    assert!(!format!("{error:?}").contains(&native_id));
    assert_bounded(&error);
}

#[tokio::test]
async fn unresolved_exception_id_does_not_prevent_releasing_the_valid_result() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    let mut response = exception_response("valid-result", "replace-me");
    response["exceptionDetails"]["exception"]["objectId"] = json!(17);
    fake.set_evaluate_response(response);
    let service = service(&fake);
    // Act
    let error = service
        .call(EvaluateHandleParams::expression(
            "throw new Error('primary sentinel')",
        ))
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::Cleanup);
    assert_eq!(releases(&fake), ["valid-result"]);
    assert_bounded(&error);
}

#[tokio::test]
async fn cleanup_failures_preserve_the_primary_exception_and_attempt_every_id() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    fake.set_evaluate_response(exception_response("result-object", "exception-object"));
    fake.set_release_object_error(Some("release denial sentinel".to_owned()));
    let service = service(&fake);
    // Act
    let error = service
        .call(EvaluateHandleParams::expression(
            "throw new Error('primary sentinel')",
        ))
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::Cleanup);
    let Some(HandleEvaluationErrorDetails::Cleanup { primary, cleanup }) = error.details() else {
        panic!("failed compensation must retain primary and cleanup diagnostics");
    };
    assert!(primary.contains("primary sentinel"));
    assert!(cleanup.contains("release denial sentinel"));
    let mut released = releases(&fake);
    released.sort();
    assert_eq!(released, ["exception-object", "result-object"]);
    assert_bounded(&error);
}

#[tokio::test]
async fn oversized_utf8_release_errors_are_bounded_without_losing_the_parse_failure() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    fake.set_evaluate_response(json!({ "result": { "objectId": "untyped-result" } }));
    let release_message = format!("release sentinel {}", "é".repeat(ERROR_BYTES * 16));
    fake.set_release_object_error(Some(release_message.clone()));
    let service = service(&fake);
    // Act
    let error = service
        .call(EvaluateHandleParams::expression("({})"))
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::Cleanup);
    let Some(HandleEvaluationErrorDetails::Cleanup { primary, cleanup }) = error.details() else {
        panic!("failed compensation must retain primary and cleanup diagnostics");
    };
    assert!(primary.contains("type"));
    assert!(cleanup.contains("release sentinel"));
    assert!(!cleanup.contains(&release_message));
    assert_eq!(releases(&fake), ["untyped-result"]);
    assert_bounded(&error);
}

#[tokio::test]
async fn oversized_utf8_exception_errors_still_release_every_bounded_native_id() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    let mut response = exception_response("result-object", "exception-object");
    let description = format!("exception sentinel {}", "é".repeat(ERROR_BYTES * 16));
    response["exceptionDetails"]["exception"]["description"] = json!(description);
    fake.set_evaluate_response(response);
    let service = service(&fake);
    // Act
    let error = service
        .call(EvaluateHandleParams::expression(
            "throw new Error('exception sentinel')",
        ))
        .await
        .unwrap_err();
    // Assert
    let mut released = releases(&fake);
    released.sort();
    assert_eq!(released, ["exception-object", "result-object"]);
    assert!(!format!("{error:?}").contains(&description));
    assert_bounded(&error);
}

#[tokio::test]
async fn malformed_primitive_is_a_protocol_error_without_native_cleanup() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    fake.set_evaluate_response(
        json!({ "result": { "type": "number", "unserializableValue": false } }),
    );
    let service = service(&fake);
    // Act
    let error = service
        .call(EvaluateHandleParams::expression("42"))
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert!(releases(&fake).is_empty());
    assert_bounded(&error);
}

#[tokio::test]
async fn accepted_object_is_not_released_before_transfer_to_its_caller() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    fake.set_evaluate_response(
        json!({ "result": { "type": "object", "objectId": "accepted-result" } }),
    );
    let service = service(&fake);
    // Act
    let handle = service
        .call(EvaluateHandleParams::expression("({})"))
        .await
        .unwrap();
    // Assert
    assert_eq!(handle.object_id(), Some("accepted-result"));
    assert!(releases(&fake).is_empty());
    assert_eq!(fake.method_names(), ["Runtime.callFunctionOn"]);
    // Cleanup: the caller now owns exactly one explicit native release.
    handle.release().await.unwrap();
    assert_eq!(releases(&fake), ["accepted-result"]);
}

#[tokio::test]
async fn accepted_primitive_has_no_native_release_on_acquisition_or_disposal() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    fake.set_evaluate_response(json!({ "result": { "type": "number", "value": 42 } }));
    let service = service(&fake);
    // Act
    let handle = service
        .call(EvaluateHandleParams::expression("42"))
        .await
        .unwrap();
    // Assert
    assert_eq!(handle.object_id(), None);
    assert_eq!(handle.call_argument(), json!({ "value": 42 }));
    assert!(releases(&fake).is_empty());
    // Cleanup
    handle.release().await.unwrap();
    assert!(releases(&fake).is_empty());
}

#[tokio::test]
async fn null_primitive_is_not_mistaken_for_an_object_with_unresolved_identity() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    fake.set_evaluate_response(
        json!({ "result": { "type": "object", "subtype": "null", "value": null } }),
    );
    let service = service(&fake);
    // Act
    let handle = service
        .call(EvaluateHandleParams::expression("null"))
        .await
        .unwrap();
    // Assert
    assert_eq!(handle.object_id(), None);
    assert_eq!(handle.call_argument(), json!({ "value": null }));
    assert!(releases(&fake).is_empty());
}

#[tokio::test]
async fn malformed_property_carrier_is_compensated_once_without_reading_properties() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    fake.set_evaluate_responses([
        json!({ "result": { "type": "object", "objectId": "target" } }),
        json!({ "result": { "objectId": "untyped-carrier" } }),
    ]);
    let target = service(&fake)
        .call(EvaluateHandleParams::expression("({ value: 42 })"))
        .await
        .unwrap();
    let properties = properties(&fake);
    // Act
    let error = properties
        .call_property_on_handle(ObjectId::new(9), target.clone(), "value")
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert_eq!(releases(&fake), ["untyped-carrier"]);
    assert!(
        !fake
            .method_names()
            .iter()
            .any(|method| method == "Runtime.getProperties")
    );
    assert_bounded(&error);
    // Cleanup: the failed property read must not have released its source.
    target.release().await.unwrap();
    assert_eq!(releases(&fake), ["untyped-carrier", "target"]);
}

#[tokio::test]
async fn exceptional_property_carrier_deduplicates_acquisition_and_outer_compensation() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    fake.set_evaluate_responses([
        json!({ "result": { "type": "object", "objectId": "target" } }),
        exception_response("exceptional-carrier", "exceptional-carrier"),
    ]);
    let target = service(&fake)
        .call(EvaluateHandleParams::expression("({ value: 42 })"))
        .await
        .unwrap();
    let properties = properties(&fake);
    // Act
    let error = properties
        .call_property_on_handle(ObjectId::new(9), target.clone(), "value")
        .await
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::Backend);
    assert_eq!(releases(&fake), ["exceptional-carrier"]);
    assert!(
        !fake
            .method_names()
            .iter()
            .any(|method| method == "Runtime.getProperties")
    );
    assert_bounded(&error);
    // Cleanup
    target.release().await.unwrap();
    assert_eq!(releases(&fake), ["exceptional-carrier", "target"]);
}
