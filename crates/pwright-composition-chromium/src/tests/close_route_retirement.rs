use super::*;
use crate::close_route_retirement::CloseRouteRetirementError;
use crate::{
    CdpServiceRoutes, ChromiumExecutionWorldReadinessProfile, ChromiumFrameNavigationAccess,
    ChromiumFrameNavigationProfile, ChromiumFrameWorldAccess, ChromiumFrameWorldAccessProfile,
    ChromiumNetworkContextTransitionState,
};
use std::num::NonZeroUsize;

const EPOCH: ConnectionEpoch = ConnectionEpoch::new(1);
const PAGE: ObjectId = ObjectId::new(3);
const FRAME: ObjectId = ObjectId::new(4);
const OTHER: ObjectId = ObjectId::new(8);
const PAGE_IDS: [ObjectId; 3] = [ObjectId::new(5), FRAME, PAGE];
const CONTEXT_IDS: [ObjectId; 4] = [ObjectId::new(5), FRAME, PAGE, ObjectId::new(2)];

pub(crate) struct Fixture {
    pub(crate) topology: ProductionCdpTopology,
    pub(crate) services: CdpPageServices,
    pub(crate) navigation: ChromiumFrameNavigationAccess,
    pub(crate) worlds: ChromiumFrameWorldAccess,
    pub(crate) factory: Arc<RecordingSessionFactory>,
    pub(crate) resources: ProductionResourceLedger,
}
impl Fixture {
    pub(crate) fn new() -> Self {
        let base = all_capabilities_profile(all_capabilities_handshake_profile())
            .with_utility_world(ChromiumUtilityWorldProfile::playwright_compatible());
        let navigation = ChromiumFrameNavigationProfile::new(
            ResourceScope::extension(130, 1),
            NonZeroUsize::new(8).unwrap(),
            NonZeroUsize::new(4).unwrap(),
            NonZeroUsize::new(128).unwrap(),
        )
        .unwrap();
        let world_access = ChromiumFrameWorldAccessProfile::new(
            ResourceScope::extension(130, 2),
            NonZeroUsize::new(4).unwrap(),
        )
        .unwrap()
        .with_native_retirement()
        .unwrap();
        let readiness = ChromiumExecutionWorldReadinessProfile::from_topology(
            ResourceScope::extension(130, 3),
            base.topology(),
            NonZeroUsize::new(2).unwrap(),
        );
        let profile = base
            .with_execution_world_readiness(readiness)
            .with_extension_requirements(navigation.requirements())
            .with_extension_requirements(world_access.requirements());
        let resources = ProductionResourceLedger::new(
            exact_limits(profile.requirements()),
            profile.requirements(),
        )
        .unwrap();
        let mut topology =
            ProductionCdpTopology::new(ObjectId::new(1), EPOCH, profile.topology(), &resources)
                .unwrap();
        for (context, target, session, page, frame, native_frame) in
            [("a", "pa", "sa", 3, 4, "fa"), ("b", "pb", "sb", 8, 9, "fb")]
        {
            let context_id = if context == "a" { 2 } else { 7 };
            topology
                .register_context(
                    EPOCH,
                    context,
                    ObjectId::new(context_id),
                    CdpContextOwnership::Owned,
                )
                .unwrap();
            topology
                .attach_page(
                    EPOCH,
                    CdpPageTarget::new(target, session, context),
                    CdpPageObjectIds::new(ObjectId::new(page), ObjectId::new(frame)),
                )
                .unwrap();
            topology
                .attach_frame(
                    EPOCH,
                    CdpFrameAttachment::main(target, native_frame),
                    ObjectId::new(frame),
                )
                .unwrap();
        }
        topology
            .attach_frame(
                EPOCH,
                CdpFrameAttachment::remote("pa", "remote", "fa", "sr"),
                ObjectId::new(5),
            )
            .unwrap();
        for (session, frame, id) in [("sa", "fa", 10), ("sr", "remote", 20), ("sb", "fb", 30)] {
            for (offset, kind) in [(0, CdpWorldKind::Main), (1, CdpWorldKind::Utility)] {
                topology
                    .attach_world(
                        EPOCH,
                        CdpWorldAttachment::new(
                            session,
                            id + offset,
                            format!("world-{}", id + offset),
                            frame,
                            kind,
                        ),
                    )
                    .unwrap();
            }
        }
        let factory = Arc::new(RecordingSessionFactory::default());
        let services =
            CdpPageServices::new_with_production_resources(factory.clone(), &profile, &resources)
                .unwrap();
        let navigation =
            ChromiumFrameNavigationAccess::new(&services, navigation, &resources).unwrap();
        let worlds = ChromiumFrameWorldAccess::new(&services, world_access, &resources).unwrap();
        services.synchronize(&topology).unwrap();
        let registry = services.network_subscription_registry().unwrap();
        for (page, context, session) in [(3, 2, "sa"), (8, 7, "sb")] {
            registry
                .register_page_lane(ObjectId::new(page), ObjectId::new(context), session.into())
                .unwrap();
        }
        Self {
            topology,
            services,
            navigation,
            worlds,
            factory,
            resources,
        }
    }
    pub(crate) fn finish(self) {
        drop((
            self.topology,
            self.services,
            self.navigation,
            self.worlds,
            self.factory,
        ));
        let metrics = self.resources.metrics().unwrap();
        assert_eq!(metrics.active_leases, 0);
        for class in ResourceClass::ALL {
            assert_eq!(metrics.class(class).current, 0, "{class:?}");
        }
    }
}
fn shape(routes: &CdpServiceRoutes) -> Vec<Vec<(ObjectId, usize)>> {
    fn ids<T>(map: &BTreeMap<ObjectId, Arc<T>>) -> Vec<(ObjectId, usize)> {
        map.iter()
            .map(|(id, value)| (*id, Arc::as_ptr(value) as usize))
            .collect()
    }
    vec![
        ids(&routes.pages),
        ids(&routes.frame_evaluations),
        ids(&routes.frame_handle_evaluations),
        ids(&routes.frame_contexts),
        // Ordinary synchronization intentionally copies this outer wrapper
        // while rebuilding clip translators. Compare its retained service,
        // and separately assert deletion-only commit preserves the wrapper.
        routes
            .locator_screenshot_contexts
            .iter()
            .map(|(id, entry)| (*id, Arc::as_ptr(&entry.locator) as usize))
            .collect(),
    ]
}

#[test]
fn page_retirement_matches_full_synchronization_and_preserves_all_surviving_arcs() {
    // Arrange
    let mut f = Fixture::new();
    let before = f.factory.created.lock().unwrap().len();
    let screenshot_survivor = f
        .services
        .routes
        .lock()
        .unwrap()
        .locator_screenshot_contexts
        .get(&ObjectId::new(9))
        .unwrap()
        .clone();
    let candidate = f.topology.prepare_page_close(PAGE).unwrap();
    let expected = shape(
        &f.services
            .prepare_synchronization(candidate.topology())
            .unwrap()
            .next,
    );
    let baseline = f.resources.metrics().unwrap();
    // Act
    let prepared = f
        .services
        .prepare_close_route_retirement(candidate.topology(), &PAGE_IDS)
        .unwrap();
    let wake = prepared.commit();
    candidate.commit(&mut f.topology).unwrap();
    wake.notify();
    // Assert
    assert_eq!(shape(&f.services.routes.lock().unwrap()), expected);
    assert!(Arc::ptr_eq(
        &screenshot_survivor,
        f.services
            .routes
            .lock()
            .unwrap()
            .locator_screenshot_contexts
            .get(&ObjectId::new(9))
            .unwrap()
    ));
    assert_eq!(f.factory.created.lock().unwrap().len(), before);
    assert_eq!(
        f.services.routes.lock().unwrap().readiness_frames,
        [(ObjectId::new(9), OTHER)]
    );
    assert!(
        f.services
            .routes
            .lock()
            .unwrap()
            .browser_contexts
            .contains(&ObjectId::new(2))
    );
    assert!(f.resources.metrics().unwrap().active_leases <= baseline.active_leases);
    drop(screenshot_survivor);
    f.finish();
}

#[test]
fn prepare_and_drop_preserve_every_route_navigation_and_network_lane() {
    // Arrange
    let mut f = Fixture::new();
    let before = shape(&f.services.routes.lock().unwrap());
    let frame = f.navigation.snapshot(ObjectId::new(5)).unwrap();
    let candidate = f.topology.prepare_page_close(PAGE).unwrap();
    let metrics = f.resources.metrics().unwrap();
    // Act
    drop(
        f.services
            .prepare_close_route_retirement(candidate.topology(), &PAGE_IDS)
            .unwrap(),
    );
    // Assert
    assert_eq!(shape(&f.services.routes.lock().unwrap()), before);
    assert!(frame.is_current());
    let registry = f.services.network_subscription_registry().unwrap();
    assert_eq!(registry.shared.state.lock().unwrap().lanes.len(), 2);
    assert_eq!(f.resources.metrics().unwrap(), metrics);
    drop((registry, frame, candidate));
    f.finish();
}

#[test]
fn context_retirement_removes_only_its_routes_and_network_ownership() {
    // Arrange
    let mut f = Fixture::new();
    let candidate = f
        .topology
        .prepare_browser_context_close(ObjectId::new(2))
        .unwrap();
    let registry = f.services.network_subscription_registry().unwrap();
    let sibling = f
        .services
        .routes
        .lock()
        .unwrap()
        .pages
        .get(&OTHER)
        .unwrap()
        .clone();
    // Act
    let wake = f
        .services
        .prepare_close_route_retirement(candidate.topology(), &CONTEXT_IDS)
        .unwrap()
        .commit();
    candidate.commit(&mut f.topology).unwrap();
    wake.notify();
    // Assert
    assert_eq!(registry.shared.state.lock().unwrap().contexts.len(), 1);
    assert_eq!(registry.shared.state.lock().unwrap().lanes.len(), 1);
    assert!(Arc::ptr_eq(
        f.services.routes.lock().unwrap().pages.get(&OTHER).unwrap(),
        &sibling
    ));
    drop((registry, sibling));
    f.finish();
}

#[test]
fn removed_oopif_snapshot_is_revoked_but_unrelated_navigation_survives() {
    // Arrange
    let mut f = Fixture::new();
    let removed = f.navigation.snapshot(ObjectId::new(5)).unwrap();
    let survivor = f.navigation.snapshot(ObjectId::new(9)).unwrap();
    let candidate = f.topology.prepare_page_close(PAGE).unwrap();
    // Act
    let wake = f
        .services
        .prepare_close_route_retirement(candidate.topology(), &PAGE_IDS)
        .unwrap()
        .commit();
    candidate.commit(&mut f.topology).unwrap();
    wake.notify();
    // Assert
    assert!(!removed.is_current());
    assert!(survivor.is_current());
    drop((removed, survivor));
    f.finish();
}

#[tokio::test]
async fn local_route_retirement_does_not_fabricate_native_world_destruction() {
    // Arrange
    let mut f = Fixture::new();
    let frame = f.worlds.snapshot(ObjectId::new(5)).await.unwrap();
    let registry = f
        .services
        .routes
        .lock()
        .unwrap()
        .native_world_retirement
        .clone()
        .unwrap();
    let candidate = f.topology.prepare_page_close(PAGE).unwrap();
    // Act
    let wake = f
        .services
        .prepare_close_route_retirement(candidate.topology(), &PAGE_IDS)
        .unwrap()
        .commit();
    candidate.commit(&mut f.topology).unwrap();
    wake.notify();
    // Assert
    assert!(!frame.native_lifetime().unwrap().both_worlds_destroyed());
    assert!(!frame.native_lifetime().unwrap().native_objects_retired());
    assert!(Arc::ptr_eq(
        &registry,
        f.services
            .routes
            .lock()
            .unwrap()
            .native_world_retirement
            .as_ref()
            .unwrap()
    ));
    drop((frame, registry));
    f.finish();
}

#[test]
fn incomplete_disposal_ids_are_rejected_without_hiding_child_routes() {
    // Arrange
    let mut f = Fixture::new();
    let candidate = f.topology.prepare_page_close(PAGE).unwrap();
    let before = shape(&f.services.routes.lock().unwrap());
    let ids = [PAGE];
    // Act
    let result = f
        .services
        .prepare_close_route_retirement(candidate.topology(), &ids);
    // Assert
    assert!(matches!(
        result,
        Err(CloseRouteRetirementError::NotDeletionOnly)
    ));
    drop(result);
    assert_eq!(shape(&f.services.routes.lock().unwrap()), before);
    drop(candidate);
    f.finish();
}

#[test]
fn disposed_id_cannot_hide_an_object_present_in_the_candidate() {
    // Arrange
    let f = Fixture::new();
    let ids = [OTHER];
    // Act
    let result = f
        .services
        .prepare_close_route_retirement(f.topology.topology(), &ids);
    // Assert
    assert!(matches!(
        result,
        Err(CloseRouteRetirementError::NotDeletionOnly)
    ));
    drop(result);
    f.finish();
}

#[test]
fn new_context_in_candidate_is_not_a_deletion_only_change() {
    // Arrange
    let mut f = Fixture::new();
    f.topology
        .register_context(EPOCH, "new", ObjectId::new(12), CdpContextOwnership::Owned)
        .unwrap();
    let before = shape(&f.services.routes.lock().unwrap());
    // Act
    let result = f
        .services
        .prepare_close_route_retirement(f.topology.topology(), &[]);
    // Assert
    assert!(matches!(
        result,
        Err(CloseRouteRetirementError::NotDeletionOnly)
    ));
    drop(result);
    assert_eq!(shape(&f.services.routes.lock().unwrap()), before);
    f.finish();
}

#[test]
fn changed_surviving_world_cannot_reuse_an_old_provider() {
    // Arrange
    let mut f = Fixture::new();
    f.topology
        .attach_world(
            EPOCH,
            CdpWorldAttachment::new("sb", 32, "replacement", "fb", CdpWorldKind::Main),
        )
        .unwrap();
    let candidate = f.topology.prepare_page_close(PAGE).unwrap();
    // Act
    let result = f
        .services
        .prepare_close_route_retirement(candidate.topology(), &PAGE_IDS);
    // Assert
    assert!(matches!(
        result,
        Err(CloseRouteRetirementError::NotDeletionOnly)
    ));
    drop((result, candidate));
    f.finish();
}

#[test]
fn epoch_mismatch_rejects_before_local_mutation() {
    // Arrange
    let mut f = Fixture::new();
    f.topology.begin_reconnect(ConnectionEpoch::new(2)).unwrap();
    // Act
    let result = f
        .services
        .prepare_close_route_retirement(f.topology.topology(), &[]);
    // Assert
    assert!(matches!(
        result,
        Err(CloseRouteRetirementError::EpochMismatch)
    ));
    drop(result);
    f.finish();
}

#[test]
fn pending_context_network_transition_is_rejected_before_commit() {
    // Arrange
    let mut f = Fixture::new();
    let registry = f.services.network_subscription_registry().unwrap();
    registry
        .shared
        .state
        .lock()
        .unwrap()
        .pending_context_transition = Some(ChromiumNetworkContextTransitionState {
        browser_context: ObjectId::new(7),
        bit: 1,
        enabled: true,
        pages: vec![],
    });
    let candidate = f.topology.prepare_page_close(PAGE).unwrap();
    // Act
    let result = f
        .services
        .prepare_close_route_retirement(candidate.topology(), &PAGE_IDS);
    // Assert
    assert!(matches!(
        result,
        Err(CloseRouteRetirementError::Network(
            ChromiumNetworkSubscriptionStateError::BrowserContextTransitionInProgress { .. }
        ))
    ));
    drop((result, candidate));
    registry
        .shared
        .state
        .lock()
        .unwrap()
        .pending_context_transition = None;
    drop(registry);
    f.finish();
}

#[test]
fn missing_surviving_network_lane_is_not_silently_repaired_during_close() {
    // Arrange
    let mut f = Fixture::new();
    let registry = f.services.network_subscription_registry().unwrap();
    {
        let mut state = registry.shared.state.lock().unwrap();
        state.contexts.get_mut(&ObjectId::new(7)).unwrap().interests = 1;
        state.lanes.remove(&OTHER);
    }
    let candidate = f.topology.prepare_page_close(PAGE).unwrap();
    // Act
    let result = f
        .services
        .prepare_close_route_retirement(candidate.topology(), &PAGE_IDS);
    // Assert
    assert!(matches!(
        result,
        Err(CloseRouteRetirementError::Network(
            ChromiumNetworkSubscriptionStateError::InheritedPageLaneUnavailable { .. }
        ))
    ));
    drop((result, candidate, registry));
    f.finish();
}

struct LockCheckingWake {
    services: CdpPageServices,
    wakes: AtomicUsize,
}
impl std::task::Wake for LockCheckingWake {
    fn wake(self: Arc<Self>) {
        assert!(self.services.routes.try_lock().is_ok());
        assert!(
            self.services
                .network_subscriptions
                .as_ref()
                .unwrap()
                .shared
                .state
                .try_lock()
                .is_ok()
        );
        self.wakes.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn readiness_notification_is_explicit_and_runs_after_route_network_unlock() {
    // Arrange
    let mut f = Fixture::new();
    f.topology.destroy_world(EPOCH, "sa", 10).unwrap();
    f.services.synchronize(&f.topology).unwrap();
    let owner = Arc::new(LockCheckingWake {
        services: f.services.clone(),
        wakes: AtomicUsize::new(0),
    });
    let waker = Waker::from(owner.clone());
    let mut waiter = Box::pin(f.services.ready_evaluation_route(FRAME));
    assert!(
        waiter
            .as_mut()
            .poll(&mut Context::from_waker(&waker))
            .is_pending()
    );
    let candidate = f.topology.prepare_page_close(PAGE).unwrap();
    let wake = f
        .services
        .prepare_close_route_retirement(candidate.topology(), &PAGE_IDS)
        .unwrap()
        .commit();
    candidate.commit(&mut f.topology).unwrap();
    assert_eq!(owner.wakes.load(Ordering::SeqCst), 0);
    // Act
    wake.notify();
    // Assert
    assert_eq!(owner.wakes.load(Ordering::SeqCst), 1);
    assert!(matches!(
        waiter.await,
        Err(CdpPageRouteError::WorldTargetClosed { .. })
    ));
    drop((waker, owner));
    f.finish();
}
