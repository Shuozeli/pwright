use super::{CdpHandleOwnerFrameService, DOCUMENT_ELEMENT_SOURCE};
use crate::{CdpEvaluatedHandle, CdpHandleEvaluationLimits};
use pwright_backend_cdp_evaluation::{
    CdpEvaluationLimits, CdpEvaluationProductionProfile, CdpEvaluationService,
    CdpEvaluationServiceFactory,
};
use pwright_cap_evaluation::EvaluationValueLimits;
use pwright_cap_handle_evaluation::HandleEvaluationErrorDetails;
use pwright_core::{
    FailureKind, OperationError, ProductionResourceLedger, ResourceClass, ResourceLimits,
    ResourceScope,
};
use pwright_fake::FakeCdpClient;
use serde_json::{Value, json};
use std::{
    future::Future,
    sync::Arc,
    task::{Context, Waker},
};
use tokio::sync::Notify;

const NATIVE_BYTES: usize = 64;

fn limits(error_bytes: usize) -> CdpEvaluationLimits {
    CdpEvaluationLimits::new(
        4096,
        32 * 1024,
        error_bytes,
        EvaluationValueLimits::new(8, 64, 64, 1024, 1024).unwrap(),
    )
}

fn service(fake: &Arc<FakeCdpClient>, error_bytes: usize) -> CdpHandleOwnerFrameService {
    CdpHandleOwnerFrameService::new(
        CdpEvaluationService::with_limits_in_unique_context(
            fake.clone(),
            "original-world",
            limits(error_bytes),
        ),
        NATIVE_BYTES,
        128,
    )
}

fn target(fake: &Arc<FakeCdpClient>, remote: Value) -> CdpEvaluatedHandle {
    crate::handle_evaluation::parse_response(
        fake.clone(),
        json!({"result": remote}),
        CdpHandleEvaluationLimits::new(limits(256), NATIVE_BYTES, 128),
    )
    .unwrap()
}

fn element(fake: &Arc<FakeCdpClient>) -> CdpEvaluatedHandle {
    target(
        fake,
        json!({"type":"object", "subtype":"node", "objectId":"source-element"}),
    )
}

fn arrange_document(fake: &FakeCdpClient, description: Value) {
    fake.set_evaluate_response(json!({"result": {
        "type":"object", "subtype":"node", "objectId":"temporary-document-element"
    }}));
    fake.set_object_node_description("temporary-document-element", description);
}

fn releases(fake: &FakeCdpClient) -> Vec<String> {
    fake.calls()
        .into_iter()
        .filter(|(method, _)| method == "Runtime.releaseObject")
        .map(|(_, args)| args[0].as_str().unwrap().to_owned())
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

#[tokio::test]
async fn returns_current_owner_frame_via_exact_receiver_and_releases_only_temporary() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    let service = service(&fake, 256);
    let source = element(&fake);
    arrange_document(&fake, json!({"node":{"frameId":"current-owner"}}));

    // Act
    let result = service.owner_frame(&source).await.unwrap();

    // Assert
    assert_eq!(result.as_deref(), Some("current-owner"));
    let calls = fake.calls();
    assert_eq!(
        fake.method_names(),
        [
            "Runtime.callFunctionOn",
            "DOM.describeNode",
            "Runtime.releaseObject"
        ]
    );
    assert_eq!(calls[0].1[0]["uniqueContextId"], "original-world");
    assert_eq!(
        calls[0].1[0]["arguments"][0]["value"]["source"],
        DOCUMENT_ELEMENT_SOURCE
    );
    assert_eq!(calls[0].1[0]["arguments"][1]["objectId"], "source-element");
    assert_eq!(
        calls[1].1,
        [json!({"objectId":"temporary-document-element"})]
    );
    assert_eq!(releases(&fake), ["temporary-document-element"]);
    assert!(DOCUMENT_ELEMENT_SOURCE.contains("doc.documentElement.ownerDocument === doc"));
    assert!(DOCUMENT_ELEMENT_SOURCE.contains("node.ownerDocument.documentElement"));
    assert!(!DOCUMENT_ELEMENT_SOURCE.contains("isConnected"));
}

#[tokio::test]
async fn null_document_element_returns_none_without_describe_or_release() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    let service = service(&fake, 256);
    let source = element(&fake);
    fake.set_evaluate_response(json!({"result":{"type":"object", "subtype":"null", "value":null}}));

    // Act
    let result = service.owner_frame(&source).await.unwrap();

    // Assert
    assert_eq!(result, None);
    assert_eq!(fake.method_names(), ["Runtime.callFunctionOn"]);
}

#[tokio::test]
async fn absent_non_string_or_empty_frame_id_returns_none_after_release() {
    for description in [
        json!({"node":{}}),
        json!({"node":{"frameId":17}}),
        json!({"node":{"frameId":""}}),
    ] {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let service = service(&fake, 256);
        let source = element(&fake);
        arrange_document(&fake, description);

        // Act
        let result = service.owner_frame(&source).await.unwrap();

        // Assert
        assert_eq!(result, None);
        assert_eq!(releases(&fake), ["temporary-document-element"]);
    }
}

#[tokio::test]
async fn malformed_or_oversized_describe_response_is_bounded_and_releases_temporary() {
    for error_bytes in [0, 1, 4, 8, 256] {
        for description in [
            json!({}),
            json!({"node":null}),
            json!({"node":{"frameId":"é".repeat(NATIVE_BYTES)}}),
        ] {
            // Arrange
            let fake = Arc::new(FakeCdpClient::from_html(""));
            let service = service(&fake, error_bytes);
            let source = element(&fake);
            arrange_document(&fake, description);

            // Act
            let error = service.owner_frame(&source).await.unwrap_err();

            // Assert
            assert_eq!(error.kind(), FailureKind::ProtocolViolation);
            assert_bounded(&error, error_bytes);
            assert_eq!(releases(&fake), ["temporary-document-element"]);
        }
    }
}

#[tokio::test]
async fn describe_failure_preserves_primary_classification_after_release() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    let service = service(&fake, 256);
    let source = element(&fake);
    arrange_document(&fake, json!({"node":{}}));
    fake.set_describe_node_error(Some("target closed".into()));

    // Act
    let error = service.owner_frame(&source).await.unwrap_err();

    // Assert
    assert_eq!(error.kind(), FailureKind::TargetClosed);
    assert_eq!(releases(&fake), ["temporary-document-element"]);
}

#[tokio::test]
async fn describe_and_release_errors_remain_bounded_cleanup_including_tiny_utf8_limits() {
    for error_bytes in [0, 1, 4, 8, 256] {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let service = service(&fake, error_bytes);
        let source = element(&fake);
        arrange_document(&fake, json!({"node":{}}));
        fake.set_describe_node_error(Some("é漢".repeat(100)));
        fake.set_release_object_error(Some("target closed é漢".repeat(100)));

        // Act
        let error = service.owner_frame(&source).await.unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::Cleanup);
        assert_bounded(&error, error_bytes);
        assert_eq!(releases(&fake), ["temporary-document-element"]);
    }
}

#[tokio::test]
async fn successful_describe_does_not_hide_failed_terminal_release() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    let service = service(&fake, 256);
    let source = element(&fake);
    arrange_document(&fake, json!({"node":{"frameId":"owner"}}));
    fake.set_release_object_error(Some("release sentinel".into()));

    // Act
    let error = service.owner_frame(&source).await.unwrap_err();

    // Assert
    assert_eq!(error.kind(), FailureKind::Cleanup);
    assert!(
        matches!(error.details(), Some(HandleEvaluationErrorDetails::Cleanup { cleanup, .. }) if cleanup.contains("release sentinel"))
    );
    assert_eq!(releases(&fake), ["temporary-document-element"]);
}

#[tokio::test]
async fn acquisition_exception_compensates_both_ids_without_describe() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    let service = service(&fake, 256);
    let source = element(&fake);
    fake.set_evaluate_response(json!({
        "result":{"type":"object", "objectId":"acquired-result"},
        "exceptionDetails":{"text":"Uncaught", "exception":{
            "type":"object", "objectId":"acquired-exception", "description":"Error: owner lookup failed"
        }}
    }));

    // Act
    let error = service.owner_frame(&source).await.unwrap_err();

    // Assert
    assert_eq!(error.kind(), FailureKind::Backend);
    assert_eq!(releases(&fake), ["acquired-result", "acquired-exception"]);
    assert!(
        !fake
            .method_names()
            .iter()
            .any(|method| method == "DOM.describeNode")
    );
}

#[tokio::test]
async fn non_element_or_foreign_client_or_unbound_world_fails_before_io() {
    for mode in 0..3 {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let other = Arc::new(FakeCdpClient::from_html(""));
        let source = match mode {
            0 => target(&fake, json!({"type":"object", "objectId":"js-target"})),
            1 => element(&other),
            _ => element(&fake),
        };
        let service = if mode == 2 {
            CdpHandleOwnerFrameService::new(
                CdpEvaluationService::with_limits(fake.clone(), limits(256)),
                NATIVE_BYTES,
                128,
            )
        } else {
            service(&fake, 256)
        };

        // Act
        let error = service.owner_frame(&source).await.unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert!(fake.calls().is_empty());
        assert!(other.calls().is_empty());
    }
}

#[tokio::test]
async fn corrupt_temporary_alias_reports_cleanup_without_releasing_borrowed_source() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    let service = service(&fake, 256);
    let source = element(&fake);
    fake.set_evaluate_response(
        json!({"result":{"type":"object", "subtype":"node", "objectId":"source-element"}}),
    );

    // Act
    let error = service.owner_frame(&source).await.unwrap_err();

    // Assert
    assert_eq!(error.kind(), FailureKind::Cleanup);
    assert_eq!(fake.method_names(), ["Runtime.callFunctionOn"]);
    assert!(releases(&fake).is_empty());
}

#[tokio::test]
async fn malformed_result_alias_is_not_released_during_acquisition_compensation() {
    for error_bytes in [0, 1, 8, 256] {
        // Arrange: no type field, so the shared pure parser would fail.
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let service = service(&fake, error_bytes);
        let source = element(&fake);
        fake.set_evaluate_response(json!({"result":{"objectId":"source-element"}}));

        // Act
        let error = service.owner_frame(&source).await.unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::Cleanup);
        assert_bounded(&error, error_bytes);
        assert_eq!(fake.method_names(), ["Runtime.callFunctionOn"]);
        assert!(releases(&fake).is_empty());
    }
}

#[tokio::test]
async fn exception_alias_is_not_released_and_both_source_aliases_are_deduplicated() {
    for result in [
        json!({"type":"undefined"}),
        json!({"type":"object", "objectId":"source-element"}),
    ] {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let service = service(&fake, 256);
        let source = element(&fake);
        fake.set_evaluate_response(json!({
            "result":result,
            "exceptionDetails":{"text":"Uncaught", "exception":{
                "type":"object", "objectId":"source-element"
            }}
        }));

        // Act
        let error = service.owner_frame(&source).await.unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::Cleanup);
        assert_bounded(&error, 256);
        assert_eq!(fake.method_names(), ["Runtime.callFunctionOn"]);
        assert!(releases(&fake).is_empty());
    }
}

#[tokio::test]
async fn protected_result_or_exception_still_compensates_every_distinct_new_owner() {
    for source_is_result in [true, false] {
        for release_fails in [true, false] {
            // Arrange: the result has no type, and either raw identity may be
            // the protected source. The OTHER acquired owner remains ours.
            let fake = Arc::new(FakeCdpClient::from_html(""));
            let service = service(&fake, 256);
            let source = element(&fake);
            let (result, exception) = if source_is_result {
                ("source-element", "new-exception")
            } else {
                ("new-result", "source-element")
            };
            fake.set_evaluate_response(json!({
                "result":{"objectId":result},
                "exceptionDetails":{"text":"Uncaught", "exception":{
                    "type":"object", "objectId":exception
                }}
            }));
            if release_fails {
                fake.set_release_object_error(Some("release failed".into()));
            }

            // Act
            let error = service.owner_frame(&source).await.unwrap_err();

            // Assert
            assert_eq!(error.kind(), FailureKind::Cleanup);
            assert_bounded(&error, 256);
            assert_eq!(
                releases(&fake),
                [if source_is_result {
                    "new-exception"
                } else {
                    "new-result"
                }]
            );
            assert_eq!(
                fake.method_names(),
                ["Runtime.callFunctionOn", "Runtime.releaseObject"]
            );
        }
    }
}

#[tokio::test]
async fn impossible_native_staging_bound_rejects_before_io() {
    // Arrange
    let fake = Arc::new(FakeCdpClient::from_html(""));
    let source = element(&fake);
    let service = CdpHandleOwnerFrameService::new(
        CdpEvaluationService::with_limits_in_unique_context(fake.clone(), "world", limits(256)),
        usize::MAX,
        usize::MAX,
    );

    // Act
    let error = service.owner_frame(&source).await.unwrap_err();

    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert!(fake.calls().is_empty());
}

#[tokio::test]
async fn evaluation_admission_remains_held_during_describe_and_terminal_release() {
    for block_describe in [true, false] {
        // Arrange
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let source = element(&fake);
        arrange_document(&fake, json!({"node":{"frameId":"owner"}}));
        let profile =
            CdpEvaluationProductionProfile::new(ResourceScope::extension(167, 1), limits(256), 1);
        let bytes = profile
            .requirements()
            .class(ResourceClass::BackendStateBytes);
        let resources = ProductionResourceLedger::new(
            ResourceLimits::deny_all(1, 1).with_class(
                ResourceClass::BackendStateBytes,
                bytes.global,
                bytes.per_scope,
            ),
            profile.requirements(),
        )
        .unwrap();
        let factory =
            CdpEvaluationServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();
        let service = CdpHandleOwnerFrameService::new(
            factory.bind_unique_context(fake.clone(), "world".into()),
            NATIVE_BYTES,
            128,
        );
        let gate = Arc::new(Notify::new());
        if block_describe {
            fake.set_describe_node_gate(Some(gate.clone()));
        } else {
            fake.set_release_object_gate(Some(gate.clone()));
        }

        // Act: manually poll to the deterministic transport gate, then attempt
        // one competing call under the same single-operation admission.
        let mut operation = Box::pin(service.owner_frame(&source));
        let pending = operation
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()));
        let rejected = service.owner_frame(&source).await.unwrap_err();
        let held = resources.metrics().unwrap();
        gate.notify_one();
        let result = operation.await.unwrap();

        // Assert
        assert!(pending.is_pending());
        assert_eq!(rejected.kind(), FailureKind::ResourceExhausted);
        assert_eq!(held.active_leases, 1);
        assert_eq!(
            held.class(ResourceClass::BackendStateBytes).current,
            profile.maximum_retained_bytes_per_operation()
        );
        assert_eq!(result.as_deref(), Some("owner"));
        assert_eq!(releases(&fake), ["temporary-document-element"]);
        assert_eq!(resources.metrics().unwrap().active_leases, 0);
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::BackendStateBytes)
                .current,
            0
        );
    }
}
