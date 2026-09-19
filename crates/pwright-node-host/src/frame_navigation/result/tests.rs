use super::*;
use pwright_backend_cdp_navigation::frame_lifecycle::{
    LifecycleSnapshot, native::NativeLifecycleNotice, navigation::FrameNavigationWaitProfile,
};
use pwright_core::{ObjectId, ResourceLimits};
use std::time::Duration;

fn resources() -> ProductionResourceLedger {
    let req = requirements()
        .combine_concurrent(requirements())
        .combine_concurrent(profile().requirements());
    let mut limits = ResourceLimits::deny_all(req.scopes, req.leases);
    for class in ResourceClass::ALL {
        let bound = req.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    ProductionResourceLedger::new(limits, req).unwrap()
}
fn profile() -> FrameNavigationWaitProfile {
    FrameNavigationWaitProfile::new(ResourceScope::extension(127, 0), 4, 4, 32, 64, SOURCE_BYTES)
        .unwrap()
}
fn owner(resources: &ProductionResourceLedger) -> FrameNavigationWait {
    FrameNavigationWait::new(
        profile(),
        resources,
        ObjectId::new(9),
        pwright_cap_navigation::WaitUntil::Commit,
        None,
    )
    .unwrap()
}

#[test]
fn typed_output_keeps_its_lease_after_native_arena_and_installation_drop() {
    // Arrange
    let resources = resources();
    let ledger = resources.claim(requirements()).unwrap();
    let slots = ResultSlots::new(&resources).unwrap();
    let mut native = owner(&resources);
    native.receipt(Duration::ZERO, None).unwrap();
    native
        .observe(
            Duration::ZERO,
            &NativeLifecycleNotice::SameDocument {
                frame: ObjectId::new(9),
            },
        )
        .unwrap();
    native
        .poll(
            Duration::ZERO,
            Some(LifecycleSnapshot {
                document: None,
                commit: true,
                dom_content_loaded: false,
                load: false,
                network_idle: false,
            }),
        )
        .unwrap();
    let permit = slots.admit(&resources).unwrap();
    // Act
    let result = project(native, permit, 100).unwrap();
    drop(slots);
    // Assert
    assert!(
        ledger
            .scope_metrics(ResourceScope::extension(127, 0))
            .unwrap()
            .is_none()
    );
    assert_eq!(result.outcome().as_ref().unwrap().frame_id, 9);
    assert_eq!(result.value().unwrap().frame_id, 9);
    assert!(result.outcome().as_ref().unwrap().response.is_none());
    assert!(ledger.scope_metrics(RESULT_SCOPE).unwrap().is_some());
    assert!(ledger.scope_metrics(SLOTS_SCOPE).unwrap().is_some());
    drop(result);
    assert!(ledger.scope_metrics(RESULT_SCOPE).unwrap().is_none());
    assert!(ledger.scope_metrics(SLOTS_SCOPE).unwrap().is_none());
}

#[test]
fn foreign_error_projection_preserves_timeout_and_retains_the_result_owner() {
    // Arrange
    let resources = resources();
    let slots = ResultSlots::new(&resources).unwrap();
    let output = FrameGotoOutput {
        outcome: Err(wait_error(NavigationWaitError::Timeout, None, 123)),
        timeout_ms: 123,
        _permit: slots.admit(&resources).unwrap(),
    };
    // Act
    let error = output.value().unwrap_err();
    // Assert
    assert!(matches!(error, HostError::Timeout(123)));
    assert_eq!(slots.active.load(Ordering::Acquire), 1);
    assert_eq!(
        output.outcome().as_ref().unwrap_err().kind(),
        FailureKind::Timeout
    );
    drop(output);
    assert_eq!(slots.active.load(Ordering::Acquire), 0);
}

#[test]
fn result_admission_has_an_independent_ceiling_even_with_a_larger_shared_plan() {
    // Arrange
    let resources = resources();
    let slots = ResultSlots::new(&resources).unwrap();
    let mut permits: Vec<_> = (0..RESULTS)
        .map(|_| slots.admit(&resources).unwrap())
        .collect();
    // Act
    let excess = slots.admit(&resources);
    // Assert
    assert!(matches!(excess, Err(HostError::ResourceExhausted(_))));
    drop(permits.pop());
    assert!(slots.admit(&resources).is_ok());
}

#[test]
fn projection_preserves_native_status_url_escaped_strings_and_canonical_headers() {
    // Arrange
    let raw = br#"{"url":"https://example.test/\u0061","status":404,"statusText":"Not Found","headers":{"z":"line1\nline2","a":"value"},"ignored":{"nested":[1,2,3]}}"#;
    // Act
    let result = parse_response(raw).unwrap();
    // Assert
    assert_eq!(result.url, "https://example.test/a");
    assert_eq!(result.status, 404);
    assert_eq!(result.status_text, "Not Found");
    assert_eq!(
        result
            .headers
            .iter()
            .map(|h| h.name.as_str())
            .collect::<Vec<_>>(),
        ["a", "z"]
    );
    assert_eq!(result.headers[1].value, "line1\nline2");
}

#[test]
fn projection_rejects_invalid_fields_duplicates_and_nonstring_headers() {
    for raw in [
        r#"{"url":"x","status":200,"headers":{}}"#,
        r#"{"url":"x","url":"y","status":200,"statusText":"OK","headers":{}}"#,
        r#"{"url":"x","status":200.5,"statusText":"OK","headers":{}}"#,
        r#"{"url":"x","status":200,"statusText":"OK","headers":{"x":3}}"#,
    ] {
        // Arrange/Act
        let result = parse_response(raw.as_bytes());
        // Assert
        assert!(matches!(
            result.unwrap_err().details(),
            Some(NavigationErrorDetails::InvalidBackendResponse(_))
        ));
    }
}

#[test]
fn projection_enforces_source_url_status_and_header_count_limits() {
    // Arrange
    let cases = [
        vec![b' '; SOURCE_BYTES + 1],
        serde_json::to_vec(&serde_json::json!({"url":"x".repeat(URL_BYTES + 1),"status":200,"statusText":"OK","headers":{}})).unwrap(),
        serde_json::to_vec(&serde_json::json!({"url":"x","status":200,"statusText":"s".repeat(STATUS_BYTES + 1),"headers":{}})).unwrap(),
        serde_json::to_vec(&serde_json::json!({"url":"x","status":200,"statusText":"OK","headers":(0..=HEADERS).map(|i| (i.to_string(), "v")).collect::<std::collections::BTreeMap<_, _>>()})).unwrap(),
    ];
    for raw in cases {
        // Act
        let result = parse_response(&raw);
        // Assert
        assert!(result.is_err());
    }
}

#[test]
fn native_wait_failure_kinds_are_not_misreported_as_success() {
    for (error, expected) in [
        (NavigationWaitError::Timeout, FailureKind::Timeout),
        (NavigationWaitError::Detached, FailureKind::TargetClosed),
        (NavigationWaitError::Canceled, FailureKind::Cancelled),
        (
            NavigationWaitError::Capacity,
            FailureKind::ResourceExhausted,
        ),
        (
            NavigationWaitError::EventLoss,
            FailureKind::ProtocolViolation,
        ),
        (
            NavigationWaitError::Network { canceled: false },
            FailureKind::Backend,
        ),
    ] {
        // Arrange/Act
        let value = wait_error(error, Some("net::ERR_FAILED"), 123);
        // Assert
        assert_eq!(value.kind(), expected);
        if error == NavigationWaitError::Timeout {
            assert_eq!(value.message(), "Timeout 123ms exceeded");
        }
        if matches!(error, NavigationWaitError::Network { .. }) {
            assert_eq!(value.message(), "net::ERR_FAILED");
        }
    }
}

#[test]
fn pending_owner_is_rejected_and_releases_the_reserved_result_slot() {
    // Arrange
    let resources = resources();
    let slots = ResultSlots::new(&resources).unwrap();
    let native = owner(&resources);
    // Act
    let result = project(native, slots.admit(&resources).unwrap(), 100);
    // Assert
    assert!(result.is_err());
    assert_eq!(slots.active.load(Ordering::Acquire), 0);
}

#[test]
fn maximum_header_count_and_large_strings_fit_the_admitted_projection_budget() {
    // Arrange
    let headers: std::collections::BTreeMap<_, _> = (0..HEADERS)
        .map(|i| (format!("header-{i}"), "line\n".repeat(20)))
        .collect();
    let raw = serde_json::to_vec(&serde_json::json!({
        "url": "u".repeat(URL_BYTES), "status": 200,
        "statusText": "s".repeat(STATUS_BYTES), "headers": headers,
    }))
    .unwrap();
    assert!(raw.len() <= SOURCE_BYTES);
    // Act
    let result = parse_response(&raw).unwrap();
    // Assert
    let retained = std::mem::size_of::<FrameGotoOutput>()
        + result.url.capacity()
        + result.status_text.capacity()
        + result.headers.capacity() * std::mem::size_of::<Header>()
        + result
            .headers
            .iter()
            .map(|h| h.name.capacity() + h.value.capacity())
            .sum::<usize>();
    assert_eq!(result.headers.len(), HEADERS);
    assert!(retained <= RESULT_BYTES as usize);
}
