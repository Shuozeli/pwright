use super::*;
use crate::frame_lifecycle::DocumentGeneration;
use pwright_core::ResourceLimits;
use serde_json::json;

const FRAME: ObjectId = ObjectId::new(2);
const NOW: Duration = Duration::ZERO;

fn fixture(wait: WaitUntil) -> (FrameNavigationWait, ProductionResourceLedger) {
    let profile =
        FrameNavigationWaitProfile::new(ResourceScope::Connection, 4, 4, 32, 64, 128).unwrap();
    let plan = profile.requirements();
    let resources = ProductionResourceLedger::new(
        ResourceLimits::deny_all(plan.scopes, plan.leases).with_class(
            ResourceClass::BackendStateBytes,
            profile.bytes,
            profile.bytes,
        ),
        plan,
    )
    .unwrap();
    let operation = FrameNavigationWait::new(
        profile,
        &resources,
        FRAME,
        wait,
        Some(Duration::from_secs(1)),
    )
    .unwrap();
    (operation, resources)
}
fn matched_fixture(wait: WaitUntil) -> (FrameNavigationWait, ProductionResourceLedger) {
    let profile = FrameNavigationWaitProfile::new(ResourceScope::Connection, 4, 4, 32, 64, 128)
        .unwrap()
        .with_url_bytes(128)
        .unwrap();
    let plan = profile.requirements();
    let resources = ProductionResourceLedger::new(
        ResourceLimits::deny_all(plan.scopes, plan.leases).with_class(
            ResourceClass::BackendStateBytes,
            profile.bytes,
            profile.bytes,
        ),
        plan,
    )
    .unwrap();
    let operation = FrameNavigationWait::new(
        profile,
        &resources,
        FRAME,
        wait,
        Some(Duration::from_secs(1)),
    )
    .unwrap();
    (operation, resources)
}
fn snapshot(load: bool) -> Option<LifecycleSnapshot> {
    Some(LifecycleSnapshot {
        document: Some(DocumentGeneration(1)),
        commit: true,
        dom_content_loaded: load,
        load,
        network_idle: load,
    })
}
fn start(
    operation: &mut FrameNavigationWait,
    request: u64,
    loader: &str,
    redirected_from: Option<u64>,
) {
    operation
        .observe(
            NOW,
            &NativeLifecycleNotice::RequestStarted {
                frame: FRAME,
                request: ObjectId::new(request),
                document_loader: Some(loader),
                redirected_from: redirected_from.map(ObjectId::new),
            },
        )
        .unwrap();
}
fn commit(operation: &mut FrameNavigationWait, loader: &str) -> Result<()> {
    operation.observe(
        NOW,
        &NativeLifecycleNotice::Committed {
            frame: FRAME,
            loader,
            document: DocumentGeneration(1),
            url: None,
        },
    )
}
fn commit_url(operation: &mut FrameNavigationWait, loader: &str, url: &str) -> Result<()> {
    operation.observe(
        NOW,
        &NativeLifecycleNotice::Committed {
            frame: FRAME,
            loader,
            document: DocumentGeneration(1),
            url: Some(url),
        },
    )
}
fn fail(operation: &mut FrameNavigationWait, request: u64, canceled: bool) -> Result<()> {
    operation.observe(
        NOW,
        &NativeLifecycleNotice::RequestFailed {
            frame: FRAME,
            request: ObjectId::new(request),
            error_text: "net::ERR_ABORTED",
            canceled,
        },
    )
}
fn respond(operation: &mut FrameNavigationWait, request: u64, status: u16) {
    operation
        .observe(
            NOW,
            &NativeLifecycleNotice::Response {
                frame: FRAME,
                request: ObjectId::new(request),
                response: &json!({"status":status}),
            },
        )
        .unwrap();
}
fn response(operation: &mut FrameNavigationWait) -> serde_json::Value {
    match operation.poll(NOW, snapshot(true)).unwrap() {
        NavigationPoll::Pending => panic!("navigation did not settle"),
        NavigationPoll::Complete {
            frame,
            response_json,
        } => {
            assert_eq!(frame, FRAME);
            response_json
                .map(|bytes| serde_json::from_slice(bytes).unwrap())
                .unwrap_or(serde_json::Value::Null)
        }
    }
}

#[test]
fn completed_response_exposes_its_bounded_document_request_identity() {
    // Arrange
    let (mut operation, _) = fixture(WaitUntil::Load);
    start(&mut operation, 1, "document-request", None);
    respond(&mut operation, 1, 200);
    commit(&mut operation, "document-request").unwrap();
    operation.receipt(NOW, Some("document-request")).unwrap();

    // Act
    let outcome = operation.poll(NOW, snapshot(true)).unwrap();

    // Assert
    assert!(matches!(outcome, NavigationPoll::Complete { .. }));
    assert_eq!(operation.response_request_id(), Some("document-request"));
}

#[test]
fn commit_and_response_before_receipt_remain_owned_without_body_completion() {
    // Arrange
    let (mut operation, resources) = fixture(WaitUntil::Load);
    start(&mut operation, 1, "doc", None);
    respond(&mut operation, 1, 500);
    commit(&mut operation, "doc").unwrap();
    // Act
    operation.receipt(NOW, Some("doc")).unwrap();
    // Assert
    assert_eq!(
        operation.poll(NOW, snapshot(false)).unwrap(),
        NavigationPoll::Pending
    );
    assert_eq!(response(&mut operation), json!({"status":500}));
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    drop(operation);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn wait_for_next_navigation_selects_the_first_document_without_a_command_receipt() {
    // Arrange
    let (mut operation, _) = fixture(WaitUntil::Load);
    operation.wait_for_next_navigation(NOW).unwrap();
    start(&mut operation, 1, "next", None);
    respond(&mut operation, 1, 201);

    // Act
    commit(&mut operation, "next").unwrap();

    // Assert
    assert_eq!(response(&mut operation), json!({"status":201}));
}

#[test]
fn wait_for_next_navigation_accepts_a_same_document_navigation() {
    // Arrange
    let (mut operation, _) = fixture(WaitUntil::Commit);
    operation.wait_for_next_navigation(NOW).unwrap();

    // Act
    operation
        .observe(
            NOW,
            &NativeLifecycleNotice::SameDocument {
                frame: FRAME,
                url: None,
            },
        )
        .unwrap();

    // Assert
    assert_eq!(response(&mut operation), serde_json::Value::Null);
}

#[test]
fn matched_registration_selects_later_same_document_event_without_reregistering() {
    // Arrange
    let (mut operation, _) = matched_fixture(WaitUntil::Load);
    for url in [
        "https://test/first",
        "https://test/second",
        "https://test/third",
    ] {
        operation
            .observe(
                NOW,
                &NativeLifecycleNotice::SameDocument {
                    frame: FRAME,
                    url: Some(url),
                },
            )
            .unwrap();
    }

    // Act
    assert_eq!(
        operation.candidate(0).unwrap(),
        Some(NavigationCandidate::Url("https://test/first"))
    );
    assert_eq!(
        operation.candidate(1).unwrap(),
        Some(NavigationCandidate::Url("https://test/second"))
    );
    assert_eq!(
        operation.candidate(2).unwrap(),
        Some(NavigationCandidate::Url("https://test/third"))
    );
    operation.select_candidate(NOW, 2).unwrap();

    // Assert
    assert_eq!(response(&mut operation), serde_json::Value::Null);
}

#[test]
fn matched_registration_preserves_selected_document_response() {
    // Arrange
    let (mut operation, _) = matched_fixture(WaitUntil::Load);
    start(&mut operation, 1, "first", None);
    respond(&mut operation, 1, 201);
    commit_url(&mut operation, "first", "https://test/first").unwrap();
    start(&mut operation, 2, "second", None);
    respond(&mut operation, 2, 202);
    commit_url(&mut operation, "second", "https://test/second").unwrap();

    // Act
    operation.select_candidate(NOW, 1).unwrap();

    // Assert
    assert_eq!(response(&mut operation), json!({"status": 202}));
    assert_eq!(operation.response_request_id(), Some("second"));
}

#[test]
fn matched_registration_keeps_original_deadline_after_nonmatching_event() {
    // Arrange
    let (mut operation, _) = matched_fixture(WaitUntil::Load);
    commit_url(&mut operation, "first", "https://test/first").unwrap();

    // Act
    let result = operation.select_candidate(Duration::from_secs(1), 0);

    // Assert
    assert_eq!(result, Err(NavigationWaitError::Timeout));
    assert_eq!(
        operation.poll(NOW, snapshot(true)),
        Err(NavigationWaitError::Timeout)
    );
}

#[test]
fn concurrent_matchers_select_different_events_without_shared_cursor() {
    // Arrange
    let profile = FrameNavigationWaitProfile::new(ResourceScope::Connection, 4, 1, 32, 64, 128)
        .unwrap()
        .with_url_bytes(128)
        .unwrap();
    let plan = profile
        .requirements()
        .combine_concurrent(profile.requirements());
    let resources = ProductionResourceLedger::new(
        ResourceLimits::deny_all(plan.scopes, plan.leases).with_class(
            ResourceClass::BackendStateBytes,
            profile.bytes * 2,
            profile.bytes * 2,
        ),
        plan,
    )
    .unwrap();
    let mut first =
        FrameNavigationWait::new(profile, &resources, FRAME, WaitUntil::Load, None).unwrap();
    let mut second =
        FrameNavigationWait::new(profile, &resources, FRAME, WaitUntil::Load, None).unwrap();
    for url in ["https://test/first", "https://test/second"] {
        let notice = NativeLifecycleNotice::SameDocument {
            frame: FRAME,
            url: Some(url),
        };
        first.observe(NOW, &notice).unwrap();
        second.observe(NOW, &notice).unwrap();
    }

    // Act
    first.select_candidate(NOW, 0).unwrap();
    second.select_candidate(NOW, 1).unwrap();

    // Assert
    assert_eq!(response(&mut first), serde_json::Value::Null);
    assert_eq!(response(&mut second), serde_json::Value::Null);
    assert_eq!(resources.metrics().unwrap().active_leases, 2);
    drop(first);
    drop(second);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn dropping_unselected_matcher_reclaims_bounded_event_arena() {
    // Arrange
    let (mut operation, resources) = matched_fixture(WaitUntil::Load);
    commit_url(&mut operation, "first", "https://test/first").unwrap();
    assert_eq!(
        operation.candidate(0).unwrap(),
        Some(NavigationCandidate::Url("https://test/first"))
    );

    // Act
    drop(operation);

    // Assert
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn first_successful_foreign_commit_interrupts_even_if_expected_commit_follows() {
    // Arrange
    let (mut operation, _) = fixture(WaitUntil::Load);
    commit(&mut operation, "other").unwrap();
    commit(&mut operation, "expected").unwrap();
    // Act
    let result = operation.receipt(NOW, Some("expected"));
    // Assert
    assert_eq!(result, Err(NavigationWaitError::Interrupted));
    assert_eq!(
        operation.poll(NOW, snapshot(true)),
        Err(NavigationWaitError::Interrupted)
    );
}

#[test]
fn matching_commit_first_is_not_retroactively_interrupted() {
    // Arrange
    let (mut operation, _) = fixture(WaitUntil::Load);
    commit(&mut operation, "expected").unwrap();
    commit(&mut operation, "other").unwrap();
    // Act
    operation.receipt(NOW, Some("expected")).unwrap();
    // Assert
    assert_eq!(
        operation.poll(NOW, snapshot(false)).unwrap(),
        NavigationPoll::Pending
    );
    assert_eq!(response(&mut operation), serde_json::Value::Null);
}

#[test]
fn unrelated_failed_document_does_not_interrupt_expected_navigation() {
    // Arrange
    let (mut operation, _) = fixture(WaitUntil::Commit);
    start(&mut operation, 1, "other", None);
    fail(&mut operation, 1, false).unwrap();
    commit(&mut operation, "expected").unwrap();
    // Act
    operation.receipt(NOW, Some("expected")).unwrap();
    // Assert
    assert_eq!(response(&mut operation), serde_json::Value::Null);
}

#[test]
fn failed_pending_document_keeps_error_until_native_receipt_matches() {
    // Arrange
    let (mut operation, _) = fixture(WaitUntil::Load);
    start(&mut operation, 1, "expected", None);
    fail(&mut operation, 1, true).unwrap();
    // Act
    let result = operation.receipt(NOW, Some("expected"));
    // Assert
    assert_eq!(result, Err(NavigationWaitError::Network { canceled: true }));
    assert_eq!(operation.failure_text(), Some("net::ERR_ABORTED"));
}

#[test]
fn old_request_failure_cannot_abort_the_new_pending_document() {
    // Arrange
    let (mut operation, _) = fixture(WaitUntil::Load);
    start(&mut operation, 1, "old", None);
    start(&mut operation, 2, "new", None);
    fail(&mut operation, 1, true).unwrap();
    commit(&mut operation, "new").unwrap();
    respond(&mut operation, 2, 200);
    // Act
    operation.receipt(NOW, Some("new")).unwrap();
    // Assert
    assert_eq!(response(&mut operation), json!({"status":200}));
}

#[test]
fn hash_requires_same_document_event_and_requested_existing_lifecycle() {
    // Arrange
    let (mut operation, _) = fixture(WaitUntil::NetworkIdle);
    operation.receipt(NOW, None).unwrap();
    assert_eq!(
        operation.poll(NOW, snapshot(true)).unwrap(),
        NavigationPoll::Pending
    );
    // Act
    operation
        .observe(
            NOW,
            &NativeLifecycleNotice::SameDocument {
                frame: FRAME,
                url: None,
            },
        )
        .unwrap();
    // Assert
    assert_eq!(
        operation.poll(NOW, snapshot(false)).unwrap(),
        NavigationPoll::Pending
    );
    assert_eq!(response(&mut operation), serde_json::Value::Null);
}

#[test]
fn hash_ignores_new_document_success_and_failure_before_its_event() {
    // Arrange
    let (mut operation, _) = fixture(WaitUntil::Load);
    start(&mut operation, 1, "other", None);
    fail(&mut operation, 1, false).unwrap();
    commit(&mut operation, "new").unwrap();
    operation
        .observe(
            NOW,
            &NativeLifecycleNotice::SameDocument {
                frame: FRAME,
                url: None,
            },
        )
        .unwrap();
    // Act
    operation.receipt(NOW, None).unwrap();
    // Assert
    assert_eq!(response(&mut operation), serde_json::Value::Null);
}

#[test]
fn redirect_legs_reuse_slot_and_return_only_final_response() {
    // Arrange
    let (mut operation, _) = fixture(WaitUntil::Load);
    start(&mut operation, 1, "doc", None);
    respond(&mut operation, 1, 302);
    for i in 2..=20 {
        start(&mut operation, i, "doc", Some(i - 1));
    }
    respond(&mut operation, 1, 301); // old normalized request is no longer authoritative
    respond(&mut operation, 20, 404);
    commit(&mut operation, "doc").unwrap();
    // Act
    operation.receipt(NOW, Some("doc")).unwrap();
    // Assert
    assert_eq!(response(&mut operation), json!({"status":404}));
}

#[test]
fn lifecycle_completion_waits_for_response_but_not_request_finished() {
    // Arrange
    let (mut operation, _) = fixture(WaitUntil::Load);
    start(&mut operation, 1, "doc", None);
    commit(&mut operation, "doc").unwrap();
    operation.receipt(NOW, Some("doc")).unwrap();
    assert_eq!(
        operation.poll(NOW, snapshot(true)).unwrap(),
        NavigationPoll::Pending
    );
    // Act
    respond(&mut operation, 1, 200);
    // Assert
    assert_eq!(response(&mut operation), json!({"status":200}));
}

#[test]
fn failed_body_after_commit_does_not_turn_navigation_into_abort() {
    // Arrange
    let (mut operation, _) = fixture(WaitUntil::Commit);
    start(&mut operation, 1, "doc", None);
    respond(&mut operation, 1, 200);
    commit(&mut operation, "doc").unwrap();
    operation.receipt(NOW, Some("doc")).unwrap();
    // Act
    fail(&mut operation, 1, false).unwrap();
    // Assert
    assert_eq!(response(&mut operation), json!({"status":200}));
}

#[test]
fn unrelated_frame_events_do_not_consume_slots_or_complete_wait() {
    // Arrange
    let (mut operation, _) = fixture(WaitUntil::Commit);
    operation.receipt(NOW, Some("doc")).unwrap();
    // Act
    for _ in 0..100 {
        operation
            .observe(
                NOW,
                &NativeLifecycleNotice::Committed {
                    frame: ObjectId::new(3),
                    document: DocumentGeneration(1),
                    loader: "doc",
                    url: None,
                },
            )
            .unwrap();
    }
    // Assert
    assert_eq!(
        operation.poll(NOW, snapshot(true)).unwrap(),
        NavigationPoll::Pending
    );
}

#[test]
fn exact_deadline_seals_without_any_native_side_effect() {
    // Arrange
    let (mut operation, _) = fixture(WaitUntil::Commit);
    commit(&mut operation, "doc").unwrap();
    // Act
    let result = operation.receipt(Duration::from_secs(1), Some("doc"));
    // Assert
    assert_eq!(result, Err(NavigationWaitError::Timeout));
    assert_eq!(
        operation.poll(NOW, snapshot(true)),
        Err(NavigationWaitError::Timeout)
    );
}

#[test]
fn missing_frame_and_external_terminal_reasons_never_report_success() {
    // Arrange
    let reasons = [
        NavigationWaitError::Detached,
        NavigationWaitError::PageClosed,
        NavigationWaitError::Disconnected,
        NavigationWaitError::EventLoss,
        NavigationWaitError::Canceled,
        NavigationWaitError::Download,
    ];
    for reason in reasons {
        let (mut operation, _) = fixture(WaitUntil::Load);
        // Act
        assert_eq!(operation.abort(reason), Err(reason));
        // Assert
        assert_eq!(operation.poll(NOW, snapshot(true)), Err(reason));
    }
    let (mut operation, _) = fixture(WaitUntil::Load);
    assert_eq!(
        operation.poll(NOW, None),
        Err(NavigationWaitError::Detached)
    );
}

#[test]
fn pre_reply_event_overflow_is_terminal_and_releases_lease_on_drop() {
    // Arrange
    let (mut operation, resources) = fixture(WaitUntil::Load);
    for _ in 0..4 {
        commit(&mut operation, "doc").unwrap();
    }
    // Act
    let result = commit(&mut operation, "doc");
    // Assert
    assert_eq!(result, Err(NavigationWaitError::Capacity));
    assert_eq!(
        operation.receipt(NOW, Some("doc")),
        Err(NavigationWaitError::Capacity)
    );
    drop(operation);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn oversized_response_seals_instead_of_truncating_success() {
    // Arrange
    let (mut operation, _) = fixture(WaitUntil::Load);
    start(&mut operation, 1, "doc", None);
    let value = json!({"data":"x".repeat(129)});
    // Act
    let result = operation.observe(
        NOW,
        &NativeLifecycleNotice::Response {
            frame: FRAME,
            request: ObjectId::new(1),
            response: &value,
        },
    );
    // Assert
    assert_eq!(result, Err(NavigationWaitError::Capacity));
    assert_eq!(
        operation.poll(NOW, snapshot(true)),
        Err(NavigationWaitError::Capacity)
    );
}

#[test]
fn admission_precedes_allocation_and_rejects_a_second_owner() {
    // Arrange
    let (operation, resources) = fixture(WaitUntil::Load);
    let profile = operation.profile;
    // Act
    let result = FrameNavigationWait::new(profile, &resources, FRAME, WaitUntil::Load, None);
    // Assert
    assert!(matches!(result, Err(NavigationWaitError::Resources)));
    drop(operation);
    let admitted =
        FrameNavigationWait::new(profile, &resources, FRAME, WaitUntil::Load, None).unwrap();
    drop(admitted);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn invalid_profiles_and_backward_time_are_rejected() {
    // Arrange
    let (mut operation, _) = fixture(WaitUntil::Load);
    operation
        .poll(Duration::from_millis(1), snapshot(true))
        .unwrap();
    // Act
    let result = operation.poll(NOW, snapshot(true));
    // Assert
    assert_eq!(result, Err(NavigationWaitError::TimeWentBackwards));
    assert!(matches!(
        FrameNavigationWaitProfile::new(ResourceScope::Connection, 0, 1, 1, 1, 1),
        Err(NavigationWaitError::InvalidProfile)
    ));
    assert!(matches!(
        FrameNavigationWaitProfile::new(ResourceScope::Connection, usize::MAX, 1, 1, 1, 1),
        Err(NavigationWaitError::SizeOverflow)
    ));
}

#[test]
fn distinct_document_request_capacity_does_not_grow() {
    // Arrange
    let (mut operation, _) = fixture(WaitUntil::Load);
    for id in 1..=4 {
        start(&mut operation, id, "doc", None);
    }
    // Act
    let result = operation.observe(
        NOW,
        &NativeLifecycleNotice::RequestStarted {
            frame: FRAME,
            request: ObjectId::new(5),
            document_loader: Some("doc"),
            redirected_from: None,
        },
    );
    // Assert
    assert_eq!(result, Err(NavigationWaitError::Capacity));
    assert_eq!(
        operation.poll(NOW, snapshot(true)),
        Err(NavigationWaitError::Capacity)
    );
}

#[test]
fn long_failure_text_is_never_silently_truncated() {
    // Arrange
    let (mut operation, _) = fixture(WaitUntil::Load);
    start(&mut operation, 1, "doc", None);
    // Act
    let result = operation.observe(
        NOW,
        &NativeLifecycleNotice::RequestFailed {
            frame: FRAME,
            request: ObjectId::new(1),
            error_text: &"x".repeat(65),
            canceled: false,
        },
    );
    // Assert
    assert_eq!(result, Err(NavigationWaitError::Capacity));
    assert_eq!(
        operation.receipt(NOW, Some("doc")),
        Err(NavigationWaitError::Capacity)
    );
}

#[test]
fn redirect_cannot_change_document_identity_under_a_retained_slot() {
    // Arrange
    let (mut operation, _) = fixture(WaitUntil::Load);
    start(&mut operation, 1, "doc", None);
    // Act
    let result = operation.observe(
        NOW,
        &NativeLifecycleNotice::RequestStarted {
            frame: FRAME,
            request: ObjectId::new(2),
            document_loader: Some("different"),
            redirected_from: Some(ObjectId::new(1)),
        },
    );
    // Assert
    assert_eq!(result, Err(NavigationWaitError::InvalidIdentity));
}

#[test]
fn settled_success_keeps_response_immutable_after_close_or_late_network_events() {
    // Arrange
    let (mut operation, _) = fixture(WaitUntil::Commit);
    start(&mut operation, 1, "doc", None);
    respond(&mut operation, 1, 200);
    commit(&mut operation, "doc").unwrap();
    operation.receipt(NOW, Some("doc")).unwrap();
    assert_eq!(response(&mut operation), json!({"status":200}));
    // Act
    operation.abort(NavigationWaitError::PageClosed).unwrap();
    respond(&mut operation, 1, 500);
    // Assert
    assert_eq!(response(&mut operation), json!({"status":200}));
}
