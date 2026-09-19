use super::*;
use crate::{
    ChromiumFrameNavigationAccess, ChromiumFrameNavigationProfile, FrameNavigationAccessError,
};
use std::num::NonZeroUsize;

const EPOCH: ConnectionEpoch = ConnectionEpoch::new(1);
const SCOPE: ResourceScope = ResourceScope::extension(99, 1);
fn selection(frames: usize, snapshots: usize) -> ChromiumFrameNavigationProfile {
    ChromiumFrameNavigationProfile::new(
        SCOPE,
        NonZeroUsize::new(frames).unwrap(),
        NonZeroUsize::new(snapshots).unwrap(),
        NonZeroUsize::new(128).unwrap(),
    )
    .unwrap()
}
struct Fixture {
    profile: ChromiumAttachProductionProfile,
    resources: ProductionResourceLedger,
    topology: ProductionCdpTopology,
    services: CdpPageServices,
    access: ChromiumFrameNavigationAccess,
    factory: Arc<RecordingSessionFactory>,
}
fn fixture(frames: usize, snapshots: usize) -> Fixture {
    fixture_with_profile(frames, snapshots, base_profile())
}
fn fixture_with_profile(
    frames: usize,
    snapshots: usize,
    base: ChromiumAttachProductionProfile,
) -> Fixture {
    let selected = selection(frames, snapshots);
    let profile = base
        .with_navigation(navigation_profile())
        .with_extension_requirements(selected.requirements());
    let resources =
        ProductionResourceLedger::new(exact_limits(profile.requirements()), profile.requirements())
            .unwrap();
    let mut topology =
        ProductionCdpTopology::new(ObjectId::new(1), EPOCH, profile.topology(), &resources)
            .unwrap();
    topology
        .register_context(EPOCH, "ctx", ObjectId::new(2), CdpContextOwnership::Owned)
        .unwrap();
    topology
        .attach_page(
            EPOCH,
            CdpPageTarget::new("page", "main-session", "ctx"),
            CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
        )
        .unwrap();
    for (binding, id) in [
        (CdpFrameAttachment::main("page", "main"), 4),
        (CdpFrameAttachment::local("page", "child", "main"), 5),
        (
            CdpFrameAttachment::remote("page", "remote", "main", "remote-session"),
            6,
        ),
        (CdpFrameAttachment::local("page", "nested", "remote"), 7),
    ] {
        topology
            .attach_frame(EPOCH, binding, ObjectId::new(id))
            .unwrap();
    }
    let factory = Arc::new(RecordingSessionFactory::default());
    let services =
        CdpPageServices::new_with_production_resources(factory.clone(), &profile, &resources)
            .unwrap();
    let access = ChromiumFrameNavigationAccess::new(&services, selected, &resources).unwrap();
    services.synchronize(&topology).unwrap();
    Fixture {
        profile,
        resources,
        topology,
        services,
        access,
        factory,
    }
}

#[test]
fn rejected_runtime_batch_does_not_revoke_prepared_navigation_routes() {
    for document_commit in [false, true] {
        // Arrange
        let base = base_profile_with_runtime(
            RuntimeLimits::new(16, 4)
                .with_active_commands(2)
                .with_control_messages(1),
        );
        let mut f = fixture_with_profile(4, 1, base);
        let prepared =
            PreparedChromiumAttach::with_production_resources(f.profile.clone(), &f.resources)
                .unwrap();
        let runtime = prepared
            .new_direct_runtime(f.services.clone(), ObjectId::new(1))
            .unwrap();
        runtime.pop_control_message().unwrap();
        for (id, parent, kind) in [
            (2, 1, BROWSER_CONTEXT_OBJECT_KIND.tag),
            (3, 2, PAGE_OBJECT_KIND.tag),
            (4, 3, FRAME_OBJECT_KIND.tag),
            (5, 4, FRAME_OBJECT_KIND.tag),
            (6, 4, FRAME_OBJECT_KIND.tag),
            (7, 6, FRAME_OBJECT_KIND.tag),
        ] {
            runtime
                .register_object(ObjectId::new(id), Some(ObjectId::new(parent)), kind)
                .unwrap();
            runtime.pop_control_message().unwrap();
        }
        let snapshot = f.access.snapshot(ObjectId::new(6)).unwrap();
        let baseline = bytes(&f);
        // Act: prepared routes remove both Frames, but two disposal messages cannot
        // fit the runtime's one-message outbox. Neither view may publish retirement.
        let outcome = transact_topology_update(&mut f.topology, &runtime, |candidate| {
            if document_commit {
                candidate.frame_document_committed(EPOCH, "main-session", "main")
            } else {
                candidate.detach_frame(EPOCH, "remote", CdpFrameDetachReason::Remove)
            }
        });
        // Assert
        assert!(matches!(outcome, Err(ChromiumCompositionError::Runtime(_))));
        assert!(snapshot.is_current());
        assert!(
            f.topology
                .topology()
                .frame_navigation_route(ObjectId::new(6))
                .is_some()
        );
        assert_eq!(runtime.is_object_live(ObjectId::new(7)), Ok(true));
        assert_eq!(bytes(&f), baseline);
    }
}
fn bytes(f: &Fixture) -> u64 {
    f.resources
        .claim(ResourceRequirements::none())
        .unwrap()
        .scope_metrics(SCOPE)
        .unwrap()
        .map_or(0, |m| m.units(ResourceClass::BackendStateBytes))
}

#[tokio::test]
async fn local_and_remote_frames_initiate_on_exact_sessions_without_worlds() {
    // Arrange
    let f = fixture(4, 4);
    assert_eq!(f.topology.topology().frame_execution_routes().count(), 0);
    assert!(f.services.world_readiness.is_none());
    let routes: Vec<_> = (4..=7)
        .map(|id| f.access.snapshot(ObjectId::new(id)).unwrap())
        .collect();
    let expected = [
        ("main", "page", "main-session"),
        ("child", "page", "main-session"),
        ("remote", "remote", "remote-session"),
        ("nested", "remote", "remote-session"),
    ];
    // Act
    for (snapshot, (frame, target, session)) in routes.iter().zip(expected) {
        f.factory
            .client(target)
            .set_navigate_response(serde_json::json!({"frameId":frame,"loaderId":"doc"}));
        let receipt = snapshot
            .begin_navigation(BeginNavigationParams::new("about:blank"))
            .await
            .unwrap();
        // Assert
        assert_eq!(receipt.frame_id, frame);
        assert_eq!(
            (
                snapshot.frame_id(),
                snapshot.target_id(),
                snapshot.session_id()
            ),
            (frame, target, session)
        );
        assert_eq!(snapshot.page(), ObjectId::new(3));
    }
    assert_eq!(f.factory.created.lock().unwrap().len(), 2);
    assert_eq!(
        f.factory.client("page").method_names(),
        ["Page.navigate", "Page.navigate"]
    );
    assert_eq!(
        f.factory.client("remote").method_names(),
        ["Page.navigate", "Page.navigate"]
    );
}

#[test]
fn snapshots_share_admission_and_hold_budget_after_route_retirement() {
    // Arrange
    let mut f = fixture(4, 1);
    let baseline = bytes(&f);
    let snapshot = f.access.snapshot(ObjectId::new(7)).unwrap();
    let retained = bytes(&f);
    let cloned = f.access.clone();
    // Act
    f.topology
        .detach_frame(EPOCH, "remote", CdpFrameDetachReason::Remove)
        .unwrap();
    f.services.synchronize(&f.topology).unwrap();
    let excess = cloned.snapshot(ObjectId::new(4));
    // Assert
    assert!(!snapshot.is_current());
    assert!(matches!(excess, Err(FrameNavigationAccessError::Capacity)));
    assert!(retained > baseline);
    assert_eq!(bytes(&f), retained);
    drop(snapshot);
    assert_eq!(bytes(&f), baseline);
    assert!(cloned.snapshot(ObjectId::new(4)).is_ok());
}

#[test]
fn committed_swap_gap_permanently_retires_snapshot_even_if_native_ids_return() {
    // Arrange
    let mut f = fixture(4, 2);
    let old = f.access.snapshot(ObjectId::new(6)).unwrap();
    let root = f.access.snapshot(ObjectId::new(4)).unwrap();
    // Act
    f.topology
        .detach_target(EPOCH, "remote-session", Some("remote"))
        .unwrap();
    f.services.synchronize(&f.topology).unwrap();
    f.topology
        .attach_frame(
            EPOCH,
            CdpFrameAttachment::remote("page", "remote", "main", "remote-session"),
            ObjectId::new(99),
        )
        .unwrap();
    f.services.synchronize(&f.topology).unwrap();
    // Assert: never queried is_current during the gap.
    assert!(!old.is_current());
    assert!(root.is_current());
    drop(root);
    let replacement = f.access.snapshot(ObjectId::new(6)).unwrap();
    assert!(replacement.is_current());
    assert_eq!(replacement.session_id(), old.session_id());
}

#[tokio::test]
async fn future_created_before_detach_does_not_dispatch_on_first_poll_after_detach() {
    // Arrange
    let mut f = fixture(4, 1);
    let snapshot = f.access.snapshot(ObjectId::new(5)).unwrap();
    let future = snapshot.begin_navigation(BeginNavigationParams::new("about:blank"));
    f.topology
        .detach_frame(EPOCH, "child", CdpFrameDetachReason::Remove)
        .unwrap();
    f.services.synchronize(&f.topology).unwrap();
    // Act
    let outcome = future.await;
    // Assert
    assert_eq!(outcome.unwrap_err().kind(), FailureKind::TargetClosed);
    assert!(f.factory.client("page").calls().is_empty());
}

#[test]
fn failed_preparation_preserves_published_routes_and_admission() {
    // Arrange
    let mut f = fixture(4, 1);
    let snapshot = f.access.snapshot(ObjectId::new(5)).unwrap();
    let baseline = bytes(&f);
    f.topology
        .attach_frame(
            EPOCH,
            CdpFrameAttachment::local("page", "extra", "main"),
            ObjectId::new(8),
        )
        .unwrap();
    // Act
    let rejected = f.services.synchronize(&f.topology);
    // Assert
    assert!(matches!(
        rejected,
        Err(CdpPageRouteError::FrameNavigation(
            FrameNavigationAccessError::Capacity
        ))
    ));
    assert!(snapshot.is_current());
    assert_eq!(bytes(&f), baseline);
}

#[test]
fn unrelated_publication_does_not_replace_route_but_crash_and_reconnect_do() {
    for reconnect in [false, true] {
        // Arrange
        let mut f = fixture(4, 1);
        let snapshot = f.access.snapshot(ObjectId::new(4)).unwrap();
        f.topology
            .attach_world(
                EPOCH,
                CdpWorldAttachment::new("main-session", 9, "world", "main", CdpWorldKind::Main),
            )
            .unwrap();
        f.services.synchronize(&f.topology).unwrap();
        assert!(snapshot.is_current());
        // Act
        if reconnect {
            f.topology.begin_reconnect(ConnectionEpoch::new(2)).unwrap();
        } else {
            f.topology
                .crash_target(EPOCH, "page", "crashed", 1)
                .unwrap();
        }
        f.services.synchronize(&f.topology).unwrap();
        // Assert
        assert!(!snapshot.is_current());
        assert!(matches!(
            f.access.snapshot(ObjectId::new(4)),
            Err(FrameNavigationAccessError::Unavailable)
        ));
    }
}

#[test]
fn owner_teardown_retires_escaped_snapshot_and_lease_outlives_service_table() {
    // Arrange
    let f = fixture(4, 1);
    let snapshot = f.access.snapshot(ObjectId::new(4)).unwrap();
    let ledger = f.resources.claim(ResourceRequirements::none()).unwrap();
    // Act
    drop(f);
    // Assert
    assert!(!snapshot.is_current());
    assert!(ledger.scope_metrics(SCOPE).unwrap().is_some());
    drop(snapshot);
    assert!(ledger.scope_metrics(SCOPE).unwrap().is_none());
}

#[test]
fn no_page_fallback_duplicate_installation_or_unbounded_profile() {
    // Arrange
    let f = fixture(4, 1);
    // Act
    let page = f.access.snapshot(ObjectId::new(3));
    let unknown = f.access.snapshot(ObjectId::new(99));
    let duplicate = ChromiumFrameNavigationAccess::new(&f.services, selection(4, 1), &f.resources);
    let overflow = ChromiumFrameNavigationProfile::new(
        SCOPE,
        NonZeroUsize::new(usize::MAX).unwrap(),
        NonZeroUsize::new(1).unwrap(),
        NonZeroUsize::new(128).unwrap(),
    );
    // Assert
    assert!(matches!(page, Err(FrameNavigationAccessError::Unavailable)));
    assert!(matches!(
        unknown,
        Err(FrameNavigationAccessError::Unavailable)
    ));
    assert!(matches!(
        duplicate,
        Err(FrameNavigationAccessError::Setup(_))
    ));
    assert!(matches!(
        overflow,
        Err(FrameNavigationAccessError::Setup(_))
    ));
}

#[test]
fn terminal_event_loss_fence_cannot_be_reopened_by_later_publication() {
    // Arrange
    let f = fixture(4, 1);
    let snapshot = f.access.snapshot(ObjectId::new(4)).unwrap();
    let baseline = bytes(&f);
    // Act
    f.access.seal().unwrap();
    f.services.synchronize(&f.topology).unwrap();
    // Assert
    assert!(!snapshot.is_current());
    assert!(matches!(
        f.access.snapshot(ObjectId::new(4)),
        Err(FrameNavigationAccessError::Unavailable)
    ));
    assert_eq!(bytes(&f), baseline);
}

#[tokio::test]
async fn successful_native_reply_after_route_revocation_is_not_reported_as_success() {
    // Arrange
    let mut f = fixture(4, 1);
    let snapshot = f.access.snapshot(ObjectId::new(5)).unwrap();
    let client = f.factory.client("page");
    client.set_navigate_response(serde_json::json!({"frameId":"child","loaderId":"document"}));
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_frame_navigate_gate(Some(gate.clone()));
    let mut future = Box::pin(snapshot.begin_navigation(BeginNavigationParams::new("about:blank")));
    assert!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    f.topology
        .detach_frame(EPOCH, "child", CdpFrameDetachReason::Remove)
        .unwrap();
    f.services.synchronize(&f.topology).unwrap();
    gate.notify_one();
    let outcome = future.await;
    // Assert
    assert_eq!(outcome.unwrap_err().kind(), FailureKind::TargetClosed);
    assert_eq!(client.method_names(), ["Page.navigate"]);
}

#[tokio::test]
async fn dispatched_receipt_survives_renderer_handoff_without_retrying() {
    // Arrange
    let mut f = fixture(4, 1);
    let snapshot = f.access.snapshot(ObjectId::new(6)).unwrap();
    let client = f.factory.client("remote");
    client
        .set_navigate_response(serde_json::json!({"frameId":"remote","loaderId":"next-document"}));
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_frame_navigate_gate(Some(gate.clone()));
    let mut future =
        Box::pin(snapshot.dispatch_navigation(BeginNavigationParams::new("about:blank")));
    assert!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act: the portable Frame survives its old renderer route.
    f.topology
        .detach_target(EPOCH, "remote-session", Some("remote"))
        .unwrap();
    f.services.synchronize(&f.topology).unwrap();
    gate.notify_one();
    let receipt = future.await.unwrap();
    // Assert: only the executor may interpret this as navigation completion.
    assert!(!snapshot.is_current());
    assert_eq!(receipt.loader_id.as_deref(), Some("next-document"));
    assert_eq!(client.method_names(), ["Page.navigate"]);
}

#[tokio::test]
async fn receipt_dispatch_rejects_route_retired_before_first_poll() {
    // Arrange
    let mut f = fixture(4, 1);
    let snapshot = f.access.snapshot(ObjectId::new(5)).unwrap();
    let future = snapshot.dispatch_navigation(BeginNavigationParams::new("about:blank"));
    f.topology
        .detach_frame(EPOCH, "child", CdpFrameDetachReason::Remove)
        .unwrap();
    f.services.synchronize(&f.topology).unwrap();
    // Act
    let result = future.await;
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::TargetClosed);
    assert!(f.factory.client("page").calls().is_empty());
}

#[tokio::test]
async fn dropping_dispatched_navigation_releases_its_operation_without_stopping_page_loading() {
    // Arrange
    let f = fixture(4, 1);
    let snapshot = f.access.snapshot(ObjectId::new(5)).unwrap();
    let client = f.factory.client("page");
    client.set_frame_navigate_gate(Some(Arc::new(tokio::sync::Notify::new())));
    let before = f.resources.metrics().unwrap();
    let mut future =
        Box::pin(snapshot.dispatch_navigation(BeginNavigationParams::new("about:blank")));
    assert!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    assert!(
        f.resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current
            > before.class(ResourceClass::BackendStateBytes).current
    );
    // Act
    drop(future);
    // Assert
    assert_eq!(
        f.resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        before.class(ResourceClass::BackendStateBytes).current
    );
    assert_eq!(client.method_names(), ["Page.navigate"]);
    assert!(snapshot.is_current());
}
