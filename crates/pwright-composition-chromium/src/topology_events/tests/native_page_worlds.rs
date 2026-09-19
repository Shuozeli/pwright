use super::*;
use crate::{ChromiumFrameNavigationAccess, ChromiumFrameWorldAccess};
use pwright_backend_cdp::ConnectionEpoch;

const EPOCH: ConnectionEpoch = ConnectionEpoch::new(1);
const PAGE: ObjectId = ObjectId::new(3);
const MAIN: ObjectId = ObjectId::new(4);
const OOPIF: ObjectId = ObjectId::new(5);
const OTHER: ObjectId = ObjectId::new(9);

struct Worlds {
    driver: ChromiumTopologyEventDriver,
    runtime: DirectDispatcher<CdpPageServices>,
    topology: SharedChromiumTopology,
    access: ChromiumFrameWorldAccess,
    navigation: ChromiumFrameNavigationAccess,
    resources: ProductionResourceLedger,
    driver_resources: ProductionResourceLedger,
}
impl Worlds {
    fn new() -> Self {
        let f = crate::tests::close_route_retirement::Fixture::new();
        let connection = CdpConnectionProductionProfile::new(
            ResourceScope::Connection,
            ConnectionConfig {
                event_channel_capacity: 8,
                max_event_bytes: 4096,
                ..ConnectionConfig::default()
            },
        );
        let profile = ChromiumTopologyEventDriverProductionProfile::from_profiles(
            ResourceScope::extension(133, 1),
            &connection,
            f.topology.profile(),
        )
        .unwrap();
        let driver_resources = ProductionResourceLedger::new(
            exact_limits(profile.requirements()),
            profile.requirements(),
        )
        .unwrap();
        let topology = Arc::new(Mutex::new(f.topology));
        let (_sender, receiver) = broadcast::channel(8);
        let driver = ChromiumTopologyEventDriver::new_with_production_resources(
            receiver,
            Arc::new(FakeCdpClient::from_html("")),
            f.factory,
            topology.clone(),
            Arc::new(AtomicObjectIdSource::new(100).unwrap()),
            EPOCH,
            profile,
            None,
            None,
            &driver_resources,
        )
        .unwrap();
        Self {
            driver,
            runtime: DirectDispatcher::new(f.services, RuntimeLimits::new(32, 4)),
            topology,
            access: f.worlds,
            navigation: f.navigation,
            resources: f.resources,
            driver_resources,
        }
    }
    async fn event(
        &mut self,
        method: &str,
        params: serde_json::Value,
        session: Option<&str>,
    ) -> Result<ChromiumTopologyEventDriverOutcome, ChromiumTopologyEventDriverError> {
        // Exercise the actual decoded event branch without re-running fixture
        // session initialization. This is not a browser or raw transport oracle.
        self.driver
            .apply_topology_event(
                &self.runtime,
                &CdpEvent::new(method.into(), params, session.map(str::to_owned)),
            )
            .await
    }
    async fn destroyed(&mut self, target: &str) {
        self.event(
            "Target.targetDestroyed",
            serde_json::json!({"targetId": target}),
            None,
        )
        .await
        .unwrap();
    }
    fn finish(self) {
        drop((
            self.driver,
            self.runtime,
            self.topology,
            self.access,
            self.navigation,
        ));
        for resources in [self.resources, self.driver_resources] {
            let metrics = resources.metrics().unwrap();
            assert_eq!(metrics.active_leases, 0);
            for class in ResourceClass::ALL {
                assert_eq!(metrics.class(class).current, 0);
            }
        }
    }
}

#[tokio::test]
async fn native_page_target_destruction_marks_main_and_oopif_but_not_sibling() {
    // Arrange
    let mut f = Worlds::new();
    let main = f.access.snapshot(MAIN).await.unwrap();
    let child = f.access.snapshot(OOPIF).await.unwrap();
    let sibling = f.access.snapshot(OTHER).await.unwrap();
    let before = f.resources.metrics().unwrap();
    // Act
    f.destroyed("pa").await;
    // Assert
    assert!(main.native_lifetime().unwrap().both_worlds_destroyed());
    assert!(child.native_lifetime().unwrap().both_worlds_destroyed());
    assert!(!sibling.native_lifetime().unwrap().both_worlds_destroyed());
    assert_eq!(f.resources.metrics().unwrap(), before);
    assert_eq!(
        f.topology.lock().unwrap().topology().page_object_id("pa"),
        Some(PAGE)
    );
    drop((main, child, sibling));
    f.finish();
}

#[tokio::test]
async fn late_native_destruction_still_marks_snapshots_after_all_page_routes_are_gone() {
    // Arrange
    let mut f = Worlds::new();
    let child = f.access.snapshot(OOPIF).await.unwrap();
    {
        let mut topology = f.topology.lock().unwrap();
        let prepared = topology.prepare_page_close(PAGE).unwrap();
        let routes = f
            .runtime
            .service()
            .prepare_synchronization(prepared.topology())
            .unwrap();
        prepared.commit(&mut topology).unwrap();
        routes.commit();
    }
    assert!(!child.native_lifetime().unwrap().both_worlds_destroyed());
    // Act
    f.destroyed("pa").await;
    // Assert
    assert!(child.native_lifetime().unwrap().both_worlds_destroyed());
    assert_eq!(
        f.topology.lock().unwrap().topology().page_object_id("pa"),
        None
    );
    drop(child);
    f.finish();
}

#[tokio::test]
async fn oopif_target_id_is_not_the_owning_page_target() {
    // Arrange
    let mut f = Worlds::new();
    let child = f.access.snapshot(OOPIF).await.unwrap();
    // Act
    f.destroyed("remote").await;
    // Assert
    assert!(!child.native_lifetime().unwrap().both_worlds_destroyed());
    drop(child);
    f.finish();
}

#[tokio::test]
async fn unknown_target_destruction_does_not_mark_any_world() {
    // Arrange
    let mut f = Worlds::new();
    let main = f.access.snapshot(MAIN).await.unwrap();
    // Act
    f.destroyed("unknown").await;
    // Assert
    assert!(!main.native_lifetime().unwrap().both_worlds_destroyed());
    drop(main);
    f.finish();
}

#[tokio::test]
async fn malformed_target_destruction_is_rejected_without_native_marking() {
    // Arrange
    let mut f = Worlds::new();
    let main = f.access.snapshot(MAIN).await.unwrap();
    // Act
    let result = f
        .event(
            "Target.targetDestroyed",
            serde_json::json!({"targetId": 7}),
            None,
        )
        .await;
    // Assert
    assert!(result.is_err());
    assert!(!main.native_lifetime().unwrap().both_worlds_destroyed());
    drop(main);
    f.finish();
}

#[tokio::test]
async fn unselected_native_registry_keeps_ignored_event_behavior() {
    // Arrange
    let mut f = fixture(8);
    let event = CdpEvent::new(
        "Target.targetDestroyed".into(),
        serde_json::json!({"targetId": 7}),
        None,
    );
    // Act
    let result = f.driver.apply_topology_event(&f.runtime, &event).await;
    // Assert
    assert!(matches!(
        result,
        Ok(ChromiumTopologyEventDriverOutcome::Ignored)
    ));
}

#[tokio::test]
async fn stale_driver_epoch_cannot_mark_new_or_retained_worlds() {
    // Arrange
    let mut f = Worlds::new();
    let main = f.access.snapshot(MAIN).await.unwrap();
    f.topology
        .lock()
        .unwrap()
        .begin_reconnect(ConnectionEpoch::new(2))
        .unwrap();
    // Act
    let result = f
        .event(
            "Target.targetDestroyed",
            serde_json::json!({"targetId": "pa"}),
            None,
        )
        .await;
    // Assert
    assert!(result.is_err());
    assert!(!main.native_lifetime().unwrap().both_worlds_destroyed());
    drop(main);
    f.finish();
}

#[tokio::test]
async fn detached_session_is_not_native_page_destruction() {
    // Arrange
    let mut f = Worlds::new();
    let main = f.access.snapshot(MAIN).await.unwrap();
    // Act
    f.event(
        "Target.detachedFromTarget",
        serde_json::json!({"sessionId": "sa", "targetId": "pa"}),
        None,
    )
    .await
    .unwrap();
    // Assert
    assert!(!main.native_lifetime().unwrap().both_worlds_destroyed());
    assert_eq!(
        f.topology.lock().unwrap().topology().page_object_id("pa"),
        None
    );
    drop(main);
    f.finish();
}

#[tokio::test]
async fn duplicate_native_page_destruction_is_monotonic_without_new_leases() {
    // Arrange
    let mut f = Worlds::new();
    let main = f.access.snapshot(MAIN).await.unwrap();
    f.destroyed("pa").await;
    let before = f.resources.metrics().unwrap();
    // Act
    f.destroyed("pa").await;
    // Assert
    assert!(main.native_lifetime().unwrap().both_worlds_destroyed());
    assert_eq!(f.resources.metrics().unwrap(), before);
    drop(main);
    f.finish();
}

#[tokio::test]
async fn oversized_owning_page_target_is_rejected_before_paired_snapshot_storage() {
    // Arrange
    let f = Worlds::new();
    {
        let mut routes = f.runtime.service().routes.lock().unwrap();
        Arc::make_mut(routes.pages.get_mut(&PAGE).unwrap()).target_id = "x".repeat(1025);
    }
    let before = f.resources.metrics().unwrap();
    // Act
    let result = f.access.snapshot(MAIN).await;
    // Assert
    assert!(result.is_err());
    assert_eq!(
        f.resources.metrics().unwrap().active_leases,
        before.active_leases
    );
    f.finish();
}

#[test]
fn oversized_owning_page_target_is_rejected_before_utility_snapshot_storage() {
    // Arrange
    let f = Worlds::new();
    {
        let mut routes = f.runtime.service().routes.lock().unwrap();
        Arc::make_mut(routes.pages.get_mut(&PAGE).unwrap()).target_id = "x".repeat(1025);
    }
    let before = f.resources.metrics().unwrap();
    // Act
    let result = f.access.existing_page_utility_worlds(PAGE, 4);
    // Assert
    assert!(result.is_err());
    assert_eq!(f.resources.metrics().unwrap(), before);
    f.finish();
}

#[tokio::test]
async fn page_destruction_marks_utility_only_snapshots_including_oopif() {
    // Arrange
    let mut f = Worlds::new();
    let worlds = f.access.existing_page_utility_worlds(PAGE, 4).unwrap();
    assert_eq!(worlds.worlds().len(), 2);
    let sibling = f.access.snapshot(OTHER).await.unwrap();
    // Act
    f.destroyed("pa").await;
    // Assert
    assert!(
        worlds
            .worlds()
            .iter()
            .all(|world| world.native_lifetime().unwrap().world_destroyed())
    );
    assert!(!sibling.native_lifetime().unwrap().both_worlds_destroyed());
    drop((worlds, sibling));
    f.finish();
}
