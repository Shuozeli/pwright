use super::CdpHandleEvaluationService;
use pwright_backend_cdp_evaluation::{CdpEvaluationLimits, CdpEvaluationService};
use pwright_cap_handle_evaluation::{EvaluateHandleParams, HandleEvaluationErrorDetails};
use pwright_core::{FailureKind, OperationError, Service};
use pwright_fake::FakeCdpClient;
use serde_json::{Value, json};
use std::sync::Arc;

fn service(fake: &Arc<FakeCdpClient>, error_bytes: usize) -> CdpHandleEvaluationService {
    let defaults = CdpEvaluationLimits::default();
    CdpHandleEvaluationService::from_evaluation(
        CdpEvaluationService::with_limits_in_unique_context(
            fake.clone(),
            "tiny-error-world",
            CdpEvaluationLimits::new(
                defaults.source_bytes,
                defaults.generated_script_bytes,
                error_bytes,
                defaults.value,
            ),
        ),
        32,
        64,
    )
}

fn releases(fake: &FakeCdpClient) -> Vec<String> {
    fake.calls()
        .into_iter()
        .filter(|(method, _)| method == "Runtime.releaseObject")
        .map(|(_, arguments)| arguments[0].as_str().unwrap().to_owned())
        .collect()
}

fn assert_bounded(error: &OperationError<HandleEvaluationErrorDetails>, limit: usize) {
    assert!(error.message().len() <= limit);
    match error.details() {
        None => {}
        Some(HandleEvaluationErrorDetails::Javascript {
            name,
            message,
            stack,
        }) => {
            assert!(name.len() <= limit);
            assert!(message.len() <= limit);
            assert!(stack.as_ref().is_none_or(|value| value.len() <= limit));
        }
        Some(
            HandleEvaluationErrorDetails::InvalidValue(value)
            | HandleEvaluationErrorDetails::InvalidBackendResponse(value)
            | HandleEvaluationErrorDetails::Backend(value),
        ) => assert!(value.len() <= limit),
        Some(HandleEvaluationErrorDetails::Cleanup { primary, cleanup }) => {
            assert!(primary.len() <= limit);
            assert!(cleanup.len() <= limit);
        }
    }
}

fn oversized_exception() -> Value {
    json!({
        "result": {"type": "object", "objectId": "acquired-result"},
        "exceptionDetails": {
            "text": "é漢".repeat(8),
            "exception": {
                "type": "object", "objectId": "acquired-exception",
                "description": "é漢".repeat(8)
            }
        }
    })
}

#[tokio::test]
async fn malformed_exception_diagnostics_obey_even_zero_byte_limit_after_cleanup() {
    for limit in [0, 1, 4, 8] {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html(""));
        fake.set_evaluate_response(json!({
            "result": {"type": "object", "objectId": "acquired-result"},
            "exceptionDetails": null
        }));
        let service = service(&fake, limit);

        // Act
        let error = service
            .call(EvaluateHandleParams::expression("({})"))
            .await
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert_bounded(&error, limit);
        assert_eq!(releases(&fake), ["acquired-result"]);
    }
}

#[tokio::test]
async fn oversized_utf8_exception_obeys_tiny_limits_and_releases_both_ids() {
    for limit in [0, 1, 4, 8] {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html(""));
        fake.set_evaluate_response(oversized_exception());
        let service = service(&fake, limit);

        // Act
        let error = service
            .call(EvaluateHandleParams::expression(
                "throw new Error('failed')",
            ))
            .await
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert_bounded(&error, limit);
        assert_eq!(releases(&fake), ["acquired-result", "acquired-exception"]);
    }
}

#[tokio::test]
async fn javascript_error_name_obeys_tiny_limits_without_changing_failure_kind() {
    for limit in [0, 1, 4, 8] {
        // Arrange an empty diagnostic so the typed JavaScript error, rather
        // than the oversized-diagnostic protocol branch, is exercised.
        let fake = Arc::new(FakeCdpClient::from_html(""));
        fake.set_evaluate_response(json!({
            "result": {"type": "object", "objectId": "acquired-result"},
            "exceptionDetails": {"text": ""}
        }));
        let service = service(&fake, limit);

        // Act
        let error = service
            .call(EvaluateHandleParams::expression("throw undefined"))
            .await
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert!(matches!(
            error.details(),
            Some(HandleEvaluationErrorDetails::Javascript { .. })
        ));
        assert_bounded(&error, limit);
        assert_eq!(releases(&fake), ["acquired-result"]);
    }
}

#[tokio::test]
async fn unresolved_identity_keeps_cleanup_classification_with_tiny_diagnostics() {
    for limit in [0, 1, 4, 8] {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html(""));
        fake.set_evaluate_response(json!({"result": {"type": "object", "objectId": ""}}));
        let service = service(&fake, limit);

        // Act
        let error = service
            .call(EvaluateHandleParams::expression("({})"))
            .await
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::Cleanup);
        assert_bounded(&error, limit);
        assert!(releases(&fake).is_empty());
    }
}

#[tokio::test]
async fn failed_releases_keep_cleanup_classification_and_all_attempts_with_tiny_diagnostics() {
    for limit in [0, 1, 4, 8] {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html(""));
        fake.set_evaluate_response(oversized_exception());
        fake.set_release_object_error(Some("é漢".repeat(8)));
        let service = service(&fake, limit);

        // Act
        let error = service
            .call(EvaluateHandleParams::expression(
                "throw new Error('failed')",
            ))
            .await
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::Cleanup);
        assert_bounded(&error, limit);
        assert_eq!(releases(&fake), ["acquired-result", "acquired-exception"]);
    }
}
