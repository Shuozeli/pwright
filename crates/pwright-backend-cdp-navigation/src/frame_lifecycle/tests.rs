use super::*;
use pwright_core::ResourceLimits;

fn resources(profile: FrameLifecycleProfile) -> ProductionResourceLedger {
    let plan = profile.requirements();
    ProductionResourceLedger::new(
        ResourceLimits::deny_all(plan.scopes, plan.leases).with_class(
            ResourceClass::BackendStateBytes,
            profile.bytes,
            profile.bytes,
        ),
        plan,
    )
    .unwrap()
}
fn fixture(
    frames: usize,
    requests: usize,
) -> (
    FrameLifecycleStore,
    ProductionResourceLedger,
    FrameLifecycleProfile,
) {
    let profile = FrameLifecycleProfile::new(ResourceScope::Connection, frames, requests).unwrap();
    let resources = resources(profile);
    let store = FrameLifecycleStore::new(profile, &resources).unwrap();
    (store, resources, profile)
}
fn apply(store: &mut FrameLifecycleStore, input: LifecycleInput) {
    store.apply(store.now, input).unwrap();
}
fn attach(store: &mut FrameLifecycleStore, frame: u64, parent: Option<u64>) {
    apply(
        store,
        LifecycleInput::Attach {
            frame: ObjectId::new(frame),
            parent: parent.map(ObjectId::new),
        },
    );
}
fn commit(store: &mut FrameLifecycleStore, frame: u64, document: u64) {
    apply(
        store,
        LifecycleInput::Commit {
            frame: ObjectId::new(frame),
            document: DocumentGeneration(document),
            request: None,
        },
    );
}
fn snapshot(store: &FrameLifecycleStore, frame: u64) -> LifecycleSnapshot {
    store.snapshot(ObjectId::new(frame)).unwrap().unwrap()
}

#[test]
fn persistent_document_lifecycle_ignores_old_generations() {
    // Arrange
    let (mut store, _, _) = fixture(2, 2);
    attach(&mut store, 1, None);
    commit(&mut store, 1, 1);
    apply(
        &mut store,
        LifecycleInput::DocumentEvent {
            frame: ObjectId::new(1),
            document: DocumentGeneration(1),
            event: DocumentLifecycle::Load,
        },
    );
    assert!(snapshot(&store, 1).load);
    commit(&mut store, 1, 2);
    // Act
    apply(
        &mut store,
        LifecycleInput::DocumentEvent {
            frame: ObjectId::new(1),
            document: DocumentGeneration(1),
            event: DocumentLifecycle::Load,
        },
    );
    // Assert
    assert_eq!(snapshot(&store, 1).document, Some(DocumentGeneration(2)));
    assert!(!snapshot(&store, 1).load);
    assert!(snapshot(&store, 1).commit);
}

#[test]
fn busy_sibling_does_not_block_the_selected_subtree() {
    // Arrange
    let (mut store, _, _) = fixture(4, 4);
    attach(&mut store, 1, None);
    attach(&mut store, 2, Some(1));
    attach(&mut store, 3, Some(1));
    attach(&mut store, 4, Some(2));
    apply(
        &mut store,
        LifecycleInput::RequestStarted {
            frame: ObjectId::new(3),
            request: ObjectId::new(10),
            favicon: false,
        },
    );
    // Act
    store.advance(Duration::from_millis(500)).unwrap();
    // Assert
    assert!(snapshot(&store, 2).network_idle);
    assert!(snapshot(&store, 4).network_idle);
    assert!(!snapshot(&store, 1).network_idle);
    assert!(!snapshot(&store, 3).network_idle);
}

#[test]
fn active_descendant_blocks_ancestor_until_its_last_request_finishes() {
    // Arrange
    let (mut store, _, _) = fixture(3, 4);
    attach(&mut store, 1, None);
    attach(&mut store, 2, Some(1));
    apply(
        &mut store,
        LifecycleInput::RequestStarted {
            frame: ObjectId::new(2),
            request: ObjectId::new(10),
            favicon: false,
        },
    );
    store.advance(Duration::from_millis(500)).unwrap();
    assert!(!snapshot(&store, 1).network_idle);
    apply(
        &mut store,
        LifecycleInput::RequestFinished {
            request: ObjectId::new(10),
        },
    );
    store.advance(Duration::from_millis(999)).unwrap();
    assert!(!snapshot(&store, 1).network_idle);
    // Act
    store.advance(Duration::from_millis(1000)).unwrap();
    // Assert
    assert!(snapshot(&store, 1).network_idle);
    assert!(snapshot(&store, 2).network_idle);
}

#[test]
fn commit_retains_only_its_document_request_and_retires_child_requests() {
    // Arrange
    let (mut store, _, _) = fixture(3, 4);
    attach(&mut store, 1, None);
    attach(&mut store, 2, Some(1));
    for (frame, request) in [(1, 10), (1, 11), (2, 12)] {
        apply(
            &mut store,
            LifecycleInput::RequestStarted {
                frame: ObjectId::new(frame),
                request: ObjectId::new(request),
                favicon: false,
            },
        );
    }
    // Act
    apply(
        &mut store,
        LifecycleInput::Commit {
            frame: ObjectId::new(1),
            document: DocumentGeneration(1),
            request: Some(ObjectId::new(10)),
        },
    );
    // Assert
    assert_eq!(store.snapshot(ObjectId::new(2)).unwrap(), None);
    assert_eq!(store.next_deadline().unwrap(), None);
    apply(
        &mut store,
        LifecycleInput::RequestFinished {
            request: ObjectId::new(11),
        },
    );
    apply(
        &mut store,
        LifecycleInput::RequestFinished {
            request: ObjectId::new(12),
        },
    );
    apply(
        &mut store,
        LifecycleInput::RequestFinished {
            request: ObjectId::new(10),
        },
    );
    store.advance(Duration::from_millis(500)).unwrap();
    assert!(snapshot(&store, 1).network_idle);
}

#[test]
fn request_deduplication_and_favicon_exclusion_do_not_consume_extra_capacity() {
    // Arrange
    let (mut store, _, _) = fixture(1, 1);
    attach(&mut store, 1, None);
    let start = LifecycleInput::RequestStarted {
        frame: ObjectId::new(1),
        request: ObjectId::new(10),
        favicon: false,
    };
    apply(&mut store, start);
    apply(&mut store, start);
    apply(
        &mut store,
        LifecycleInput::RequestStarted {
            frame: ObjectId::new(1),
            request: ObjectId::new(11),
            favicon: true,
        },
    );
    // Act
    apply(
        &mut store,
        LifecycleInput::RequestFinished {
            request: ObjectId::new(10),
        },
    );
    store.advance(Duration::from_millis(500)).unwrap();
    // Assert
    assert!(snapshot(&store, 1).network_idle);
}

#[test]
fn overflow_and_event_loss_seal_snapshots_until_the_owner_drops_the_store() {
    for input in [
        LifecycleInput::Attach {
            frame: ObjectId::new(2),
            parent: Some(ObjectId::new(1)),
        },
        LifecycleInput::RequestStarted {
            frame: ObjectId::new(1),
            request: ObjectId::new(11),
            favicon: false,
        },
    ] {
        // Arrange
        let (mut store, _, _) = fixture(1, 1);
        attach(&mut store, 1, None);
        apply(
            &mut store,
            LifecycleInput::RequestStarted {
                frame: ObjectId::new(1),
                request: ObjectId::new(10),
                favicon: false,
            },
        );
        // Act
        let result = store.apply(Duration::ZERO, input);
        // Assert
        assert!(matches!(
            result,
            Err(LifecycleError::FrameCapacity | LifecycleError::RequestCapacity)
        ));
        assert_eq!(
            store.snapshot(ObjectId::new(1)),
            Err(LifecycleError::Invalidated)
        );
        assert_eq!(store.next_deadline(), Err(LifecycleError::Invalidated));
    }
    let (mut store, _, _) = fixture(1, 1);
    store.invalidate();
    assert_eq!(
        store.advance(Duration::ZERO),
        Err(LifecycleError::Invalidated)
    );
}

#[test]
fn production_storage_is_admitted_before_construction_and_released_on_drop() {
    // Arrange
    let (store, resources, profile) = fixture(4, 8);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        profile.retained_bytes()
    );
    // Act
    let second = FrameLifecycleStore::new(profile, &resources);
    // Assert
    assert!(matches!(second, Err(LifecycleError::Resources)));
    drop(store);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        0
    );
    assert!(FrameLifecycleStore::new(profile, &resources).is_ok());
}

#[test]
fn invalid_time_and_profile_overflow_fail_explicitly() {
    // Arrange
    let (mut store, _, _) = fixture(1, 1);
    store.advance(Duration::from_millis(10)).unwrap();
    // Act
    let error = store.advance(Duration::ZERO);
    // Assert
    assert_eq!(error, Err(LifecycleError::TimeWentBackwards));
    assert!(matches!(
        FrameLifecycleProfile::new(ResourceScope::Connection, usize::MAX, 1),
        Err(LifecycleError::SizeOverflow)
    ));
    assert!(matches!(
        FrameLifecycleProfile::new(ResourceScope::Connection, 0, 1),
        Err(LifecycleError::InvalidProfile)
    ));
}

#[test]
fn lifecycle_waits_remain_distinct_and_already_observed_load_is_reusable() {
    use pwright_cap_navigation::WaitUntil;
    // Arrange
    let (mut store, _, _) = fixture(1, 1);
    attach(&mut store, 1, None);
    commit(&mut store, 1, 1);
    apply(
        &mut store,
        LifecycleInput::DocumentEvent {
            frame: ObjectId::new(1),
            document: DocumentGeneration(1),
            event: DocumentLifecycle::DomContentLoaded,
        },
    );
    // Act
    let state = snapshot(&store, 1);
    // Assert
    assert!(state.reached(WaitUntil::Commit));
    assert!(state.reached(WaitUntil::DomContentLoaded));
    assert!(!state.reached(WaitUntil::Load));
    assert!(!state.reached(WaitUntil::NetworkIdle));
    apply(
        &mut store,
        LifecycleInput::DocumentEvent {
            frame: ObjectId::new(1),
            document: DocumentGeneration(1),
            event: DocumentLifecycle::Load,
        },
    );
    assert!(snapshot(&store, 1).reached(WaitUntil::Load));
    assert!(snapshot(&store, 1).reached(WaitUntil::Load));
}

#[test]
fn stream_identity_conflicts_do_not_leave_queryable_success_state() {
    // Arrange
    let (mut store, _, _) = fixture(3, 3);
    attach(&mut store, 1, None);
    attach(&mut store, 2, Some(1));
    apply(
        &mut store,
        LifecycleInput::RequestStarted {
            frame: ObjectId::new(1),
            request: ObjectId::new(10),
            favicon: false,
        },
    );
    // Act
    let result = store.apply(
        Duration::ZERO,
        LifecycleInput::RequestStarted {
            frame: ObjectId::new(2),
            request: ObjectId::new(10),
            favicon: false,
        },
    );
    // Assert
    assert_eq!(result, Err(LifecycleError::ConflictingIdentity));
    assert_eq!(
        store.snapshot(ObjectId::new(1)),
        Err(LifecycleError::Invalidated)
    );
}
