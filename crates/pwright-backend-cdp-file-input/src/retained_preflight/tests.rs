use super::*;
use pwright_core::ResourceLimits;
use pwright_fake::FakeCdpClient;
use serde_json::json;
use std::{
    pin::pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};

const FLAGS: FileInputPreflightFlags = FileInputPreflightFlags {
    multiple: false,
    directory_upload: false,
};

fn fixture() -> (
    CdpFileInputPreflightService,
    ProductionResourceLedger,
    FakeCdpClient,
) {
    let profile = RetainedPreflightProfile::new(ResourceScope::extension(178, 1), 1).unwrap();
    let needs = profile.requirements();
    let mut limits = ResourceLimits::deny_all(needs.scopes, needs.leases);
    for class in ResourceClass::ALL {
        let bound = needs.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    let resources = ProductionResourceLedger::new(limits, needs).unwrap();
    let service = CdpFileInputPreflightService::new(profile, &resources).unwrap();
    let client = FakeCdpClient::from_html("");
    (service, resources, client)
}

fn target(client: &FakeCdpClient) -> PayloadTarget<'_> {
    PayloadTarget::new(client, "exact-utility", "adopted-input", "owned-group").unwrap()
}

fn node(id: &str) -> Value {
    json!({"result":{"type":"object","subtype":"node","objectId":id}})
}

#[test]
fn profile_is_separate_and_charges_both_native_wrappers_before_io() {
    // Arrange
    let (service, resources, client) = fixture();

    // Act
    let admission = service.admit().unwrap();

    // Assert
    let metrics = resources.metrics().unwrap();
    assert_eq!(
        metrics.class(ResourceClass::RemoteHandle).current,
        EXTRA_NATIVE
    );
    assert_eq!(
        metrics.class(ResourceClass::BackendStateBytes).current,
        ADMISSION_BYTES
    );
    assert_eq!(metrics.active_leases, 1);
    assert!(client.calls().is_empty());
    assert!(admission.object_id().is_none());
    drop(admission);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn exhausted_admission_refuses_another_call_without_io() {
    // Arrange
    let (service, resources, client) = fixture();
    let held = service.admit().unwrap();
    let before = resources.metrics().unwrap();

    // Act
    let result = service.admit();

    // Assert
    assert!(matches!(result, Err(RetainedPreflightError::Budget(_))));
    let mut expected = before;
    expected.rejected_reservations += 1;
    assert_eq!(resources.metrics().unwrap(), expected);
    assert!(client.calls().is_empty());
    drop(held);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn successful_query_keeps_exact_node_and_native_allowance_without_releasing() {
    // Arrange
    let (service, resources, client) = fixture();
    client.set_evaluate_response(node("label-retargeted-input"));
    let mut admission = service.admit().unwrap();

    // Act
    let result = admission
        .validate(
            target(&client),
            FileInputPreflightFlags {
                multiple: true,
                directory_upload: true,
            },
        )
        .await;

    // Assert
    assert!(result.is_ok());
    assert_eq!(admission.object_id(), Some("label-retargeted-input"));
    let calls = client.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "Runtime.callFunctionOn");
    let params = &calls[0].1[0];
    assert_eq!(params["uniqueContextId"], "exact-utility");
    assert_eq!(params["objectGroup"], "owned-group");
    assert_eq!(params["returnByValue"], false);
    assert_eq!(params["awaitPromise"], false);
    assert_eq!(params["userGesture"], true);
    assert_eq!(
        params["arguments"],
        json!([
            {"objectId":"adopted-input"},
            {"value":{"multiple":true,"directoryUpload":true}},
        ])
    );
    assert!(!SCRIPT.contains("atob"));
    assert!(!SCRIPT.contains("getAttribute"));
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::RemoteHandle)
            .current,
        2
    );
    // The raw backend deliberately owns no group release authority.
    drop(admission);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn all_fixed_policy_statuses_map_without_admitting_a_result_id() {
    for code in 0..=4 {
        // Arrange
        let (service, resources, client) = fixture();
        client.set_evaluate_response(json!({"result":{"type":"number","value":code}}));
        let mut admission = service.admit().unwrap();

        // Act
        let result = admission.validate(target(&client), FLAGS).await;

        // Assert
        assert!(matches!(
            (code, result),
            (0, Err(RetainedPreflightError::Disconnected))
                | (
                    1,
                    Err(RetainedPreflightError::Policy(FileInputError::NotInput))
                )
                | (
                    2,
                    Err(RetainedPreflightError::Policy(
                        FileInputError::MultipleFiles
                    ))
                )
                | (
                    3,
                    Err(RetainedPreflightError::Policy(
                        FileInputError::DirectoryNotSupported
                    ))
                )
                | (
                    4,
                    Err(RetainedPreflightError::Policy(
                        FileInputError::DirectoryRequired
                    ))
                )
        ));
        assert!(admission.object_id().is_none());
        assert_eq!(client.calls().len(), 1);
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::RemoteHandle)
                .current,
            2
        );
    }
}

#[tokio::test]
async fn malformed_or_oversized_remote_results_fail_without_copying_a_native_id() {
    for response in [
        json!({}),
        json!({"result":{"type":"number","value":5}}),
        json!({"result":{"type":"number","value":1,"objectId":"unexpected"}}),
        json!({"result":{"type":"object","subtype":"array","objectId":"array"}}),
        json!({"result":{"type":"object","subtype":"node"}}),
        node(""),
        node(&"界".repeat(MAX_NATIVE_ID / 3 + 1)),
    ] {
        // Arrange
        let (service, resources, client) = fixture();
        client.set_evaluate_response(response);
        let mut admission = service.admit().unwrap();

        // Act
        let result = admission.validate(target(&client), FLAGS).await;

        // Assert
        assert!(matches!(result, Err(RetainedPreflightError::Protocol(_))));
        assert!(admission.object_id().is_none());
        assert_eq!(resources.metrics().unwrap().active_leases, 1);
    }
}

#[tokio::test]
async fn native_exception_keeps_group_allowance_and_bounds_utf8_before_copy() {
    // Arrange
    let (service, resources, client) = fixture();
    client.set_evaluate_response(json!({
        "result":{"type":"object","subtype":"node","objectId":"result"},
        "exceptionDetails":{"exception":{"objectId":"exception", "description":"界".repeat(4096)}},
    }));
    let mut admission = service.admit().unwrap();

    // Act
    let result = admission.validate(target(&client), FLAGS).await;

    // Assert
    let Err(RetainedPreflightError::JavaScript(message)) = result else {
        panic!("expected native exception")
    };
    assert!(message.len() <= DIAGNOSTIC_BYTES);
    assert_eq!(message.len() % 3, 0);
    assert!(admission.object_id().is_none());
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::RemoteHandle)
            .current,
        2
    );
    assert_eq!(client.calls().len(), 1);
}

#[tokio::test]
async fn a_successful_admission_cannot_issue_a_second_retained_call() {
    // Arrange
    let (service, _, client) = fixture();
    client.set_evaluate_response(node("retargeted"));
    let mut admission = service.admit().unwrap();
    admission.validate(target(&client), FLAGS).await.unwrap();

    // Act
    let result = admission.validate(target(&client), FLAGS).await;

    // Assert
    assert!(matches!(result, Err(RetainedPreflightError::AlreadyUsed)));
    assert_eq!(admission.object_id(), Some("retargeted"));
    assert_eq!(client.calls().len(), 1);
}

#[tokio::test]
async fn a_failed_admission_cannot_retry_a_retained_call() {
    // Arrange
    let (service, _, client) = fixture();
    client.set_evaluate_response(json!({}));
    let mut admission = service.admit().unwrap();
    assert!(admission.validate(target(&client), FLAGS).await.is_err());

    // Act
    let result = admission.validate(target(&client), FLAGS).await;

    // Assert
    assert!(matches!(result, Err(RetainedPreflightError::AlreadyUsed)));
    assert_eq!(client.calls().len(), 1);
}

#[tokio::test]
async fn an_unpolled_validation_still_consumes_its_one_shot_authority() {
    // Arrange
    let (service, _, client) = fixture();
    let mut admission = service.admit().unwrap();
    drop(admission.validate(target(&client), FLAGS));

    // Act
    let result = admission.validate(target(&client), FLAGS).await;

    // Assert
    assert!(matches!(result, Err(RetainedPreflightError::AlreadyUsed)));
    assert!(client.calls().is_empty());
}

#[tokio::test]
async fn raw_cancellation_cannot_reuse_the_same_wrapper_allowance() {
    // Arrange: raw-call one-shot semantics only; the composition separately
    // proves continuation/cleanup of the retained native request.
    let (service, resources, client) = fixture();
    client.set_evaluate_gate(Some(Arc::new(tokio::sync::Notify::new())));
    let mut admission = service.admit().unwrap();
    {
        let mut first = pin!(admission.validate(target(&client), FLAGS));
        assert!(matches!(
            first.as_mut().poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        ));
    }

    // Act
    let result = admission.validate(target(&client), FLAGS).await;

    // Assert
    assert!(matches!(result, Err(RetainedPreflightError::AlreadyUsed)));
    assert_eq!(client.calls().len(), 1);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::RemoteHandle)
            .current,
        2
    );
}

#[test]
fn terminal_timeout_and_resource_errors_preserve_exact_typed_data() {
    for error in [
        CdpError::Closed,
        CdpError::SessionDetached,
        CdpError::ChannelDropped,
        CdpError::Timeout,
        CdpError::PageClosed,
        CdpError::IdentifierExhausted,
        CdpError::InvalidConfiguration("invalid configuration sentinel"),
        CdpError::LimitExceeded {
            resource: "pending command",
            actual: 17,
            limit: 16,
        },
        CdpError::Resource(ResourceError::ScopeLimitExceeded {
            scope: ResourceScope::extension(178, 2),
            class: ResourceClass::RemoteHandle,
            current: 2,
            requested: 1,
            limit: 2,
        }),
    ] {
        // Arrange
        let discriminant = std::mem::discriminant(&error);
        let expected = format!("{error:?}");

        // Act
        let result = bounded_error(error);

        // Assert
        assert_eq!(std::mem::discriminant(&result), discriminant);
        assert_eq!(format!("{result:?}"), expected);
    }
}

#[test]
fn string_error_variants_preserve_kind_and_protocol_code_with_bounded_utf8_storage() {
    for variant in 0..7 {
        // Arrange
        let message = "界é".repeat(DIAGNOSTIC_BYTES);
        let input = match variant {
            0 => CdpError::WebSocket(message),
            1 => CdpError::Protocol {
                code: -32007,
                message,
            },
            2 => CdpError::ElementNotFound { selector: message },
            3 => CdpError::TabNotFound(message),
            4 => CdpError::HttpFailed(message),
            5 => CdpError::JsException(message),
            _ => CdpError::Other(message),
        };
        let discriminant = std::mem::discriminant(&input);

        // Act
        let result = bounded_error(input);

        // Assert
        assert_eq!(std::mem::discriminant(&result), discriminant);
        let text = match result {
            CdpError::WebSocket(text)
            | CdpError::TabNotFound(text)
            | CdpError::HttpFailed(text)
            | CdpError::JsException(text)
            | CdpError::Other(text) => text,
            CdpError::ElementNotFound { selector } => selector,
            CdpError::Protocol { code, message } => {
                assert_eq!(code, -32007);
                message
            }
            other => panic!("unexpected error: {other:?}"),
        };
        assert_eq!(text.len(), DIAGNOSTIC_BYTES / 5 * 5 + 3);
        assert!(text.capacity() <= DIAGNOSTIC_BYTES);
        assert!("界é".repeat(DIAGNOSTIC_BYTES).starts_with(&text));
    }
}

#[test]
fn small_error_messages_discard_large_original_backing_capacity() {
    // Arrange
    let mut message = String::with_capacity(DIAGNOSTIC_BYTES * 64);
    message.push_str("Target closed");

    // Act
    let result = bounded_error(CdpError::Other(message));

    // Assert: text alone does not classify a terminal error.
    let CdpError::Other(message) = result else {
        panic!("arbitrary text was reclassified")
    };
    assert_eq!(message, "Target closed");
    assert!(message.capacity() <= DIAGNOSTIC_BYTES);
}

#[test]
fn navigation_diagnostics_share_one_total_utf8_storage_budget() {
    // Arrange
    let error = CdpError::NavigationFailed {
        url: "界".repeat(DIAGNOSTIC_BYTES),
        reason: "é".repeat(DIAGNOSTIC_BYTES),
    };

    // Act
    let result = bounded_error(error);

    // Assert
    let CdpError::NavigationFailed { url, reason } = result else {
        panic!("navigation kind changed")
    };
    assert_eq!(url.len(), DIAGNOSTIC_BYTES / 2 / 3 * 3);
    assert_eq!(reason.len(), DIAGNOSTIC_BYTES / 2);
    assert!(url.capacity() + reason.capacity() <= DIAGNOSTIC_BYTES);
}

#[test]
fn compound_errors_normalize_without_retaining_or_formatting_the_owned_tree() {
    // Arrange
    let compound = CdpError::Compound {
        source: Box::new(CdpError::Other("x".repeat(4096))),
        system: Box::new(CdpError::Compound {
            source: Box::new(CdpError::SessionDetached),
            system: Box::new(CdpError::Closed),
        }),
    };

    // Act
    let result = bounded_error(compound);

    // Assert
    let CdpError::Other(message) = result else {
        panic!("recursive storage survived")
    };
    assert_eq!(
        message,
        "compound CDP error during retained file-input prevalidation"
    );
    assert!(message.capacity() <= DIAGNOSTIC_BYTES);
}

#[test]
fn opaque_json_errors_normalize_without_retaining_large_owned_storage() {
    // Arrange
    let error = CdpError::Json(serde_json::Error::io(std::io::Error::other(
        "界é".repeat(DIAGNOSTIC_BYTES),
    )));

    // Act
    let result = bounded_error(error);

    // Assert
    let CdpError::Other(message) = result else {
        panic!("opaque JSON storage survived")
    };
    assert!(message.starts_with("JSON error: "));
    assert!(message.len() <= DIAGNOSTIC_BYTES);
    assert!(message.capacity() <= DIAGNOSTIC_BYTES);
}

#[tokio::test]
async fn native_protocol_failure_keeps_code_without_forgiving_group_cleanup() {
    // Arrange
    let (service, resources, client) = fixture();
    client.set_evaluate_errors(["Target closed".into()]);
    let mut admission = service.admit().unwrap();

    // Act
    let result = admission.validate(target(&client), FLAGS).await;

    // Assert
    let Err(RetainedPreflightError::Backend(CdpError::Protocol { code, message })) = result else {
        panic!("protocol error was flattened or reclassified")
    };
    assert_eq!(code, -32000);
    assert_eq!(message, "Target closed");
    assert!(admission.object_id().is_none());
    assert_eq!(client.calls().len(), 1);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::RemoteHandle)
            .current,
        EXTRA_NATIVE
    );
}

#[test]
fn zero_and_overflow_capacity_profiles_are_rejected() {
    // Arrange
    let scope = ResourceScope::extension(178, 1);

    // Act
    let zero = RetainedPreflightProfile::new(scope, 0);
    let overflow = RetainedPreflightProfile::new(scope, usize::MAX);

    // Assert
    assert!(matches!(zero, Err(RetainedPreflightError::InvalidProfile)));
    assert!(matches!(
        overflow,
        Err(RetainedPreflightError::InvalidProfile)
    ));
}
