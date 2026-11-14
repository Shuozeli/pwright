use super::*;
use pwright_core::ResourceLimits;
use serde_json::{Value, json};

struct Routes {
    left_session: &'static str,
}
impl CdpLifecycleRoutes for Routes {
    fn contains_session(&self, session: &str) -> bool {
        ["main", "child", "replacement", "worker"].contains(&session)
    }
    fn event_frame(&self, session: &str, native: &str) -> Option<ObjectId> {
        let frame = match (session, native) {
            ("main", "root") => 1,
            ("main", "right") => 3,
            ("child", "nested") => 4,
            (s, "left") if s == self.left_session => 2,
            _ => return None,
        };
        Some(ObjectId::new(frame))
    }
    fn request_frame(&self, session: &str, native: Option<&str>) -> Option<ObjectId> {
        if session == "worker" && native.is_none() {
            Some(ObjectId::new(2))
        } else {
            native.and_then(|native| self.event_frame(session, native))
        }
    }
    fn accepts_document_handoff(&self, frame: ObjectId, session: &str) -> bool {
        frame == ObjectId::new(2) && session == self.left_session
    }
}
fn profile(requests: usize) -> CdpFrameLifecycleProfile {
    CdpFrameLifecycleProfile::new(
        FrameLifecycleProfile::new(ResourceScope::extension(91, 1), 4, requests).unwrap(),
        ResourceScope::extension(91, 2),
        64,
    )
    .unwrap()
}
fn resources(p: CdpFrameLifecycleProfile) -> ProductionResourceLedger {
    let requirements = p.requirements();
    let bytes = requirements.class(ResourceClass::BackendStateBytes);
    ProductionResourceLedger::new(
        ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
            ResourceClass::BackendStateBytes,
            bytes.global,
            bytes.per_scope,
        ),
        requirements,
    )
    .unwrap()
}
fn event(session: &str, method: &str, params: Value) -> CdpEvent {
    CdpEvent::new(method.into(), params, Some(session.into()))
}
fn apply(
    observer: &mut CdpFrameLifecycle,
    routes: &Routes,
    ms: u64,
    session: &str,
    method: &str,
    params: Value,
) {
    observer
        .apply(
            Duration::from_millis(ms),
            1,
            &event(session, method, params),
            routes,
        )
        .unwrap();
}
fn commit(
    observer: &mut CdpFrameLifecycle,
    routes: &Routes,
    session: &str,
    frame: &str,
    loader: &str,
) {
    apply(
        observer,
        routes,
        0,
        session,
        "Page.frameNavigated",
        json!({"frame":{"id":frame,"loaderId":loader}}),
    );
}
fn request(frame: Option<&str>, id: &str, loader: &str, kind: &str, url: &str) -> Value {
    let mut value = json!({"requestId":id,"loaderId":loader,"type":kind,"request":{"url":url}});
    if let Some(frame) = frame {
        value["frameId"] = frame.into();
    }
    value
}
fn fixture(requests: usize) -> (CdpFrameLifecycle, Routes, ProductionResourceLedger) {
    let p = profile(requests);
    let resources = resources(p);
    let mut observer = CdpFrameLifecycle::new(p, 1, &resources).unwrap();
    let routes = Routes {
        left_session: "main",
    };
    for (frame, parent, native, session) in [
        (1, None, "root", "main"),
        (2, Some(1), "left", "main"),
        (4, Some(2), "nested", "child"),
        (3, Some(1), "right", "main"),
    ] {
        observer
            .attach(
                Duration::ZERO,
                ObjectId::new(frame),
                parent.map(ObjectId::new),
            )
            .unwrap();
        commit(
            &mut observer,
            &routes,
            session,
            native,
            &format!("document-{frame}"),
        );
    }
    (observer, routes, resources)
}
fn state(observer: &CdpFrameLifecycle, frame: u64) -> LifecycleSnapshot {
    observer.snapshot(ObjectId::new(frame)).unwrap().unwrap()
}

#[test]
fn initial_document_seed_accepts_queued_load_and_same_loader_snapshot_preserves_it() {
    // Arrange
    let p = profile(2);
    let ledger = resources(p);
    let mut observer = CdpFrameLifecycle::new(p, 1, &ledger).unwrap();
    let routes = Routes {
        left_session: "main",
    };
    observer
        .initialize_frame(Duration::ZERO, ObjectId::new(1), None, "seed")
        .unwrap();
    observer
        .initialize_frame(
            Duration::ZERO,
            ObjectId::new(2),
            Some(ObjectId::new(1)),
            "child-seed",
        )
        .unwrap();
    apply(
        &mut observer,
        &routes,
        0,
        "main",
        "Page.lifecycleEvent",
        json!({"frameId":"root","loaderId":"seed","name":"load"}),
    );
    let before = state(&observer, 1);
    // Act
    observer
        .initialize_frame(Duration::ZERO, ObjectId::new(1), None, "seed")
        .unwrap();
    // Assert
    assert!(before.load);
    assert_eq!(state(&observer, 1), before);
    assert!(observer.snapshot(ObjectId::new(2)).unwrap().is_some());
}

#[test]
fn changed_current_renderer_snapshot_commits_without_a_later_native_commit_event() {
    // Arrange
    let (mut observer, routes, _) = fixture(2);
    let before = state(&observer, 2).document;
    // Act
    let notice = observer
        .initialize_frame(
            Duration::ZERO,
            ObjectId::new(2),
            Some(ObjectId::new(1)),
            "replacement-document",
        )
        .unwrap();
    // Assert
    assert!(
        matches!(notice, NativeLifecycleNotice::Committed { frame, loader: "replacement-document", .. } if frame == ObjectId::new(2))
    );
    assert_ne!(state(&observer, 2).document, before);
    assert!(!state(&observer, 2).load);
    assert!(observer.snapshot(ObjectId::new(4)).unwrap().is_none());
    apply(
        &mut observer,
        &routes,
        0,
        "main",
        "Page.lifecycleEvent",
        json!({"frameId":"left","loaderId":"replacement-document","name":"load"}),
    );
    assert!(state(&observer, 2).load);
    let current = state(&observer, 2);
    commit(
        &mut observer,
        &routes,
        "main",
        "left",
        "replacement-document",
    );
    assert_eq!(state(&observer, 2), current);
}

#[test]
fn explicit_document_reset_reuses_loader_for_following_lifecycle_events() {
    // Arrange
    let (mut observer, routes, _) = fixture(2);
    apply(
        &mut observer,
        &routes,
        0,
        "main",
        "Page.lifecycleEvent",
        json!({"frameId":"left","loaderId":"document-2","name":"load"}),
    );
    let before = state(&observer, 2).document;

    // Act
    observer
        .reset_document(Duration::from_millis(1), ObjectId::new(2))
        .unwrap();

    // Assert
    assert_ne!(state(&observer, 2).document, before);
    assert!(!state(&observer, 2).load);
    assert!(observer.snapshot(ObjectId::new(4)).unwrap().is_none());
    apply(
        &mut observer,
        &routes,
        2,
        "main",
        "Page.lifecycleEvent",
        json!({"frameId":"left","loaderId":"document-2","name":"load"}),
    );
    assert!(state(&observer, 2).load);
}

#[test]
fn malformed_initial_document_seals_the_native_owner() {
    // Arrange
    let p = profile(2);
    let ledger = resources(p);
    let mut observer = CdpFrameLifecycle::new(p, 1, &ledger).unwrap();
    let loader = "x".repeat(65);
    // Act
    let result = observer.initialize_frame(Duration::ZERO, ObjectId::new(1), None, &loader);
    // Assert
    assert!(result.is_err());
    assert!(observer.snapshot(ObjectId::new(1)).is_err());
}

#[test]
fn topology_retention_removes_descendants_and_releases_request_capacity() {
    // Arrange
    let (mut observer, routes, _) = fixture(1);
    apply(
        &mut observer,
        &routes,
        0,
        "child",
        "Network.requestWillBeSent",
        request(
            Some("nested"),
            "old",
            "document-4",
            "Fetch",
            "https://a.test",
        ),
    );
    // Act
    observer
        .retain_frames(Duration::ZERO, |frame| frame != ObjectId::new(2))
        .unwrap();
    // Assert
    assert!(observer.snapshot(ObjectId::new(2)).unwrap().is_none());
    assert!(observer.snapshot(ObjectId::new(4)).unwrap().is_none());
    assert!(observer.snapshot(ObjectId::new(3)).unwrap().is_some());
    let next = event(
        "main",
        "Network.requestWillBeSent",
        request(
            Some("right"),
            "new",
            "document-3",
            "Fetch",
            "https://a.test",
        ),
    );
    assert!(matches!(
        observer.apply(Duration::ZERO, 1, &next, &routes).unwrap(),
        NativeLifecycleNotice::RequestStarted { .. }
    ));
}

#[test]
fn exact_epoch_session_and_loader_are_required_for_document_lifecycle() {
    // Arrange
    let (mut observer, routes, _) = fixture(4);
    commit(&mut observer, &routes, "main", "left", "new-document");
    let load = event(
        "main",
        "Page.lifecycleEvent",
        json!({"frameId":"left","loaderId":"new-document","name":"load"}),
    );
    // Act
    observer.apply(Duration::ZERO, 2, &load, &routes).unwrap();
    apply(
        &mut observer,
        &routes,
        0,
        "child",
        "Page.lifecycleEvent",
        load.params.clone(),
    );
    apply(
        &mut observer,
        &routes,
        0,
        "main",
        "Page.lifecycleEvent",
        json!({"frameId":"left","loaderId":"document-2","name":"load"}),
    );
    // Assert
    assert!(!state(&observer, 2).load);
    observer.apply(Duration::ZERO, 1, &load, &routes).unwrap();
    assert!(state(&observer, 2).load);
    assert!(!state(&observer, 1).load);
}

#[test]
fn document_redirect_reuses_native_id_but_gets_a_distinct_normalized_request() {
    // Arrange
    let (mut observer, routes, _) = fixture(1);
    let first = event(
        "main",
        "Network.requestWillBeSent",
        request(
            Some("left"),
            "navigation",
            "navigation",
            "Document",
            "https://a.test",
        ),
    );
    let NativeLifecycleNotice::RequestStarted { request: old, .. } =
        observer.apply(Duration::ZERO, 1, &first, &routes).unwrap()
    else {
        panic!("missing start");
    };
    let mut params = first.params.clone();
    params["redirectResponse"] = json!({"status":302});
    params["request"]["url"] = "https://b.test".into();
    let redirect = event("main", "Network.requestWillBeSent", params);
    // Act
    let notice = observer
        .apply(Duration::ZERO, 1, &redirect, &routes)
        .unwrap();
    // Assert
    let NativeLifecycleNotice::RequestStarted {
        request: new,
        redirected_from,
        document_loader,
        ..
    } = notice
    else {
        panic!("missing redirect");
    };
    assert_ne!(new, old);
    assert_eq!(redirected_from, Some(old));
    assert_eq!(document_loader, Some("navigation"));
    commit(&mut observer, &routes, "main", "left", "navigation");
    observer.advance(Duration::from_millis(500)).unwrap();
    assert!(!state(&observer, 2).network_idle);
    apply(
        &mut observer,
        &routes,
        500,
        "main",
        "Network.loadingFinished",
        json!({"requestId":"navigation"}),
    );
    observer.advance(Duration::from_millis(1000)).unwrap();
    assert!(state(&observer, 2).network_idle);
}

#[test]
fn document_completion_moves_sessions_only_with_explicit_current_frame_proof() {
    // Arrange
    let (mut observer, mut routes, _) = fixture(2);
    apply(
        &mut observer,
        &routes,
        0,
        "main",
        "Network.requestWillBeSent",
        request(
            Some("left"),
            "navigation",
            "navigation",
            "Document",
            "https://a.test",
        ),
    );
    commit(&mut observer, &routes, "main", "left", "navigation");
    apply(
        &mut observer,
        &routes,
        0,
        "replacement",
        "Network.loadingFinished",
        json!({"requestId":"navigation"}),
    );
    observer.advance(Duration::from_millis(500)).unwrap();
    assert!(!state(&observer, 2).network_idle);
    routes.left_session = "replacement";
    // Act
    apply(
        &mut observer,
        &routes,
        500,
        "replacement",
        "Network.loadingFinished",
        json!({"requestId":"navigation"}),
    );
    observer.advance(Duration::from_millis(1000)).unwrap();
    // Assert
    assert!(state(&observer, 2).network_idle);
}

#[test]
fn worker_and_eventsource_requests_block_their_frame_but_busy_siblings_do_not() {
    // Arrange
    let (mut observer, routes, _) = fixture(4);
    apply(
        &mut observer,
        &routes,
        0,
        "main",
        "Network.requestWillBeSent",
        request(
            Some("right"),
            "busy",
            "document-3",
            "Fetch",
            "https://a.test/stream",
        ),
    );
    apply(
        &mut observer,
        &routes,
        0,
        "worker",
        "Network.requestWillBeSent",
        request(
            None,
            "worker-stream",
            "",
            "EventSource",
            "https://a.test/events",
        ),
    );
    observer.advance(Duration::from_millis(500)).unwrap();
    assert!(!state(&observer, 2).network_idle);
    // Act
    apply(
        &mut observer,
        &routes,
        500,
        "worker",
        "Network.loadingFinished",
        json!({"requestId":"worker-stream"}),
    );
    observer.advance(Duration::from_millis(1000)).unwrap();
    // Assert
    assert!(state(&observer, 2).network_idle);
    assert!(!state(&observer, 3).network_idle);
    assert!(!state(&observer, 1).network_idle);
}

#[test]
fn equal_request_ids_in_different_sessions_do_not_finish_each_other() {
    // Arrange
    let (mut observer, routes, _) = fixture(4);
    apply(
        &mut observer,
        &routes,
        0,
        "main",
        "Network.requestWillBeSent",
        request(
            Some("right"),
            "same",
            "document-3",
            "Fetch",
            "https://a.test",
        ),
    );
    apply(
        &mut observer,
        &routes,
        0,
        "child",
        "Network.requestWillBeSent",
        request(
            Some("nested"),
            "same",
            "document-4",
            "Fetch",
            "https://a.test",
        ),
    );
    // Act
    apply(
        &mut observer,
        &routes,
        0,
        "main",
        "Network.loadingFinished",
        json!({"requestId":"same"}),
    );
    observer.advance(Duration::from_millis(500)).unwrap();
    // Assert
    assert!(state(&observer, 3).network_idle);
    assert!(!state(&observer, 4).network_idle);
    assert!(!state(&observer, 2).network_idle);
}

#[test]
fn favicon_redirects_remain_excluded_and_data_urls_need_no_request_slot() {
    // Arrange
    let (mut observer, routes, _) = fixture(1);
    apply(
        &mut observer,
        &routes,
        0,
        "main",
        "Network.requestWillBeSent",
        request(
            Some("left"),
            "icon",
            "document-2",
            "Other",
            "https://a.test/favicon.ico",
        ),
    );
    let mut redirect = request(
        Some("left"),
        "icon",
        "document-2",
        "Other",
        "https://a.test/icon.png",
    );
    redirect["redirectResponse"] = json!({"status":302});
    // Act
    apply(
        &mut observer,
        &routes,
        0,
        "main",
        "Network.requestWillBeSent",
        redirect,
    );
    apply(
        &mut observer,
        &routes,
        0,
        "main",
        "Network.requestWillBeSent",
        request(
            Some("left"),
            "data",
            "document-2",
            "Fetch",
            "data:text/plain,hello",
        ),
    );
    observer.advance(Duration::from_millis(500)).unwrap();
    // Assert
    assert!(state(&observer, 2).network_idle);
}

#[test]
fn hash_keeps_document_lifecycle_and_new_commit_removes_child_requests() {
    // Arrange
    let (mut observer, routes, _) = fixture(2);
    apply(
        &mut observer,
        &routes,
        0,
        "main",
        "Page.lifecycleEvent",
        json!({"frameId":"left","loaderId":"document-2","name":"load"}),
    );
    let old = state(&observer, 2);
    apply(
        &mut observer,
        &routes,
        0,
        "main",
        "Page.navigatedWithinDocument",
        json!({"frameId":"left","url":"https://a.test/#hash"}),
    );
    assert_eq!(state(&observer, 2), old);
    apply(
        &mut observer,
        &routes,
        0,
        "child",
        "Network.requestWillBeSent",
        request(
            Some("nested"),
            "old",
            "document-4",
            "Fetch",
            "https://a.test",
        ),
    );
    // Act
    commit(&mut observer, &routes, "main", "left", "new");
    apply(
        &mut observer,
        &routes,
        0,
        "child",
        "Network.loadingFinished",
        json!({"requestId":"old"}),
    );
    observer.advance(Duration::from_millis(500)).unwrap();
    // Assert
    assert!(!state(&observer, 2).load);
    assert!(state(&observer, 2).network_idle);
    assert!(observer.snapshot(ObjectId::new(4)).unwrap().is_none());
}

#[test]
fn response_and_failure_notices_borrow_the_envelope_instead_of_retaining_bodies() {
    // Arrange
    let (mut observer, routes, _) = fixture(1);
    apply(
        &mut observer,
        &routes,
        0,
        "main",
        "Network.requestWillBeSent",
        request(
            Some("left"),
            "fetch",
            "document-2",
            "Fetch",
            "https://a.test",
        ),
    );
    let response = event(
        "main",
        "Network.responseReceived",
        json!({"requestId":"fetch","response":{"status":500,"headers":{"x":"y"}}}),
    );
    let failure = event(
        "main",
        "Network.loadingFailed",
        json!({"requestId":"fetch","errorText":"net::ERR_ABORTED","canceled":true}),
    );
    // Act
    let received = observer
        .apply(Duration::ZERO, 1, &response, &routes)
        .unwrap();
    let failed = observer
        .apply(Duration::ZERO, 1, &failure, &routes)
        .unwrap();
    // Assert
    let NativeLifecycleNotice::Response {
        response: borrowed, ..
    } = received
    else {
        panic!("missing response");
    };
    assert!(std::ptr::eq(borrowed, &response.params["response"]));
    assert!(
        matches!(failed,NativeLifecycleNotice::RequestFailed { frame,error_text:"net::ERR_ABORTED",canceled:true,.. } if frame == ObjectId::new(2))
    );
}

#[test]
fn capacity_or_malformed_native_events_seal_readiness_until_owner_drop() {
    for malformed in [false, true] {
        // Arrange
        let (mut observer, routes, _) = fixture(1);
        apply(
            &mut observer,
            &routes,
            0,
            "main",
            "Network.requestWillBeSent",
            request(Some("left"), "one", "document-2", "Fetch", "https://a.test"),
        );
        let params = if malformed {
            json!({"frame":{"id":"left","loaderId":"x".repeat(65)}})
        } else {
            request(Some("left"), "two", "document-2", "Fetch", "https://a.test")
        };
        let method = if malformed {
            "Page.frameNavigated"
        } else {
            "Network.requestWillBeSent"
        };
        // Act
        let input = event("main", method, params);
        let outcome = observer.apply(Duration::ZERO, 1, &input, &routes);
        // Assert
        assert!(outcome.is_err());
        assert_eq!(
            observer.snapshot(ObjectId::new(2)),
            Err(NativeLifecycleError::Lifecycle(LifecycleError::Invalidated))
        );
        assert!(observer.next_deadline().is_err());
    }
}

#[test]
fn full_native_storage_is_admitted_before_construction_and_released_on_drop() {
    // Arrange
    let p = profile(2);
    let ledger = resources(p);
    let observer = CdpFrameLifecycle::new(p, 1, &ledger).unwrap();
    let bytes = ledger
        .metrics()
        .unwrap()
        .class(ResourceClass::BackendStateBytes)
        .current;
    // Act
    let second = CdpFrameLifecycle::new(p, 1, &ledger);
    drop(observer);
    // Assert
    assert!(matches!(
        second,
        Err(NativeLifecycleError::Lifecycle(LifecycleError::Resources))
    ));
    assert!(bytes > 0);
    assert_eq!(ledger.metrics().unwrap().active_leases, 0);
    assert_eq!(
        ledger
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        0
    );
    assert!(CdpFrameLifecycle::new(p, 1, &ledger).is_ok());
}
