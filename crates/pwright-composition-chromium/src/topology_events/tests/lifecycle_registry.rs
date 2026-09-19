use super::*;
use crate::{
    ChromiumFrameEventObserver, FrameObservationResult,
    frame_lifecycle::{
        FrameLifecycleClock, FrameLifecycleNoticeSink, FrameLifecycleRegistry,
        FrameLifecycleRegistryProfile,
    },
};
use pwright_backend_cdp::CdpTopology;
use pwright_backend_cdp_navigation::frame_lifecycle::native::NativeLifecycleNotice;
use std::time::Duration;

mod discovery_retirement;
mod lifetime_wait;
mod navigation_registration;

struct Clock;
impl FrameLifecycleClock for Clock {
    fn now(&self) -> Duration {
        Duration::ZERO
    }
}
struct Sink;
impl FrameLifecycleNoticeSink for Sink {
    fn observe(&self, _: NativeLifecycleNotice<'_>) -> FrameObservationResult {
        Ok(())
    }
}
fn registry(
    pages: usize,
    owners: usize,
) -> (Arc<FrameLifecycleRegistry>, ProductionResourceLedger) {
    let profile = FrameLifecycleRegistryProfile::new(96, pages, owners, 8, 4, 64).unwrap();
    let requirements = profile.requirements();
    let ledger = ProductionResourceLedger::new(exact_limits(requirements), requirements).unwrap();
    let registry = Arc::new(
        FrameLifecycleRegistry::new(profile, 1, &ledger, Arc::new(Clock), Arc::new(Sink)).unwrap(),
    );
    (registry, ledger)
}
async fn selected(registry: Arc<FrameLifecycleRegistry>) -> DriverFixture {
    let mut f = fixture(16);
    f.factory.client("target-a").set_frame_tree(frame_tree(
        serde_json::json!({"frame":frame_value("target-a", None)}),
    ));
    f.driver = f.driver.with_frame_event_observer(registry).unwrap();
    f.driver.initialize_snapshot(&f.runtime, 0).await.unwrap();
    f
}
fn add_second_page(f: &DriverFixture) {
    f.topology
        .lock()
        .unwrap()
        .attach_page(
            ConnectionEpoch::new(1),
            CdpPageTarget::new("target-b", "session-b", "context-a"),
            CdpPageObjectIds::new(ObjectId::new(12), ObjectId::new(13)),
        )
        .unwrap();
    f.runtime
        .register_object(
            ObjectId::new(12),
            Some(ObjectId::new(2)),
            PAGE_OBJECT_KIND.tag,
        )
        .unwrap();
    f.runtime
        .register_object(
            ObjectId::new(13),
            Some(ObjectId::new(12)),
            FRAME_OBJECT_KIND.tag,
        )
        .unwrap();
    f.factory.set_frame_tree(
        "target-b",
        frame_tree(serde_json::json!({"frame":frame_value("target-b", None)})),
    );
}
async fn close_first(f: &mut DriverFixture) {
    f.sender
        .send(CdpEvent::new(
            "Target.detachedFromTarget".into(),
            serde_json::json!({"sessionId":"session-a", "targetId":"target-a"}),
            None,
        ))
        .unwrap();
    f.driver.receive_once(&f.runtime).await.unwrap();
}

#[tokio::test]
async fn distinct_pages_keep_independent_readiness_and_retirement() {
    // Arrange
    let (registry, _) = registry(2, 3);
    let mut f = selected(registry.clone()).await;
    add_second_page(&f);
    f.driver.initialize_snapshot(&f.runtime, 0).await.unwrap();
    let first = registry.page(ObjectId::new(3)).unwrap().unwrap();
    let second = registry.page(ObjectId::new(12)).unwrap().unwrap();
    f.sender
        .send(CdpEvent::new(
            "Page.lifecycleEvent".into(),
            serde_json::json!({
                "frameId":"target-a", "loaderId":"loader-target-a", "name":"load", "timestamp":0
            }),
            Some("session-a".into()),
        ))
        .unwrap();
    // Act
    f.driver.receive_once(&f.runtime).await.unwrap();
    // Assert
    assert!(first.snapshot(ObjectId::new(4)).unwrap().unwrap().load);
    assert!(!second.snapshot(ObjectId::new(13)).unwrap().unwrap().load);
    assert!(first.snapshot(ObjectId::new(13)).unwrap().is_none());
    close_first(&mut f).await;
    assert!(first.is_sealed());
    assert!(!second.is_sealed());
    assert!(registry.page(ObjectId::new(3)).unwrap().is_none());
    assert!(second.snapshot(ObjectId::new(13)).unwrap().is_some());
}

#[tokio::test]
async fn active_page_capacity_rejects_before_enabling_network_domains() {
    // Arrange
    let (registry, _) = registry(1, 2);
    let mut f = selected(registry.clone()).await;
    let retained = registry.page(ObjectId::new(3)).unwrap().unwrap();
    add_second_page(&f);
    // Act
    let result = f.driver.initialize_snapshot(&f.runtime, 0).await;
    // Assert
    assert!(result.is_err());
    assert!(
        !f.factory
            .client("target-b")
            .method_names()
            .iter()
            .any(|m| m == "Network.enable")
    );
    assert!(retained.is_sealed());
    assert!(retained.snapshot(ObjectId::new(4)).is_err());
}

#[tokio::test]
async fn escaped_retired_owner_prevents_slot_reuse_even_after_page_close() {
    // Arrange
    let (registry, ledger) = registry(1, 1);
    let mut f = selected(registry.clone()).await;
    let retained = registry.page(ObjectId::new(3)).unwrap().unwrap();
    close_first(&mut f).await;
    let bytes = ledger
        .metrics()
        .unwrap()
        .class(ResourceClass::BackendStateBytes)
        .current;
    add_second_page(&f);
    // Act
    let result = f.driver.initialize_snapshot(&f.runtime, 0).await;
    // Assert
    assert!(result.is_err());
    assert!(bytes > 0);
    assert!(retained.is_sealed());
    assert!(
        !f.factory
            .client("target-b")
            .method_names()
            .iter()
            .any(|m| m == "Network.enable")
    );
}

#[tokio::test]
async fn dropping_last_retired_handle_allows_slot_reuse_at_next_checkpoint() {
    // Arrange
    let (registry, _) = registry(1, 1);
    let mut f = selected(registry.clone()).await;
    let retained = registry.page(ObjectId::new(3)).unwrap().unwrap();
    close_first(&mut f).await;
    drop(retained);
    add_second_page(&f);
    // Act
    f.driver.initialize_snapshot(&f.runtime, 0).await.unwrap();
    // Assert
    assert!(
        registry
            .page(ObjectId::new(12))
            .unwrap()
            .unwrap()
            .snapshot(ObjectId::new(13))
            .unwrap()
            .is_some()
    );
    assert!(registry.page(ObjectId::new(3)).unwrap().is_none());
}

#[tokio::test]
async fn escaped_handle_keeps_its_storage_and_terminal_allowance_until_drop() {
    // Arrange
    let (registry, ledger) = registry(1, 1);
    let f = selected(registry.clone()).await;
    let retained = registry.page(ObjectId::new(3)).unwrap().unwrap();
    // Act
    drop(f.driver);
    drop(registry);
    // Assert
    assert!(retained.is_sealed());
    assert!(retained.next_deadline().is_err());
    assert!(ledger.metrics().unwrap().active_leases > 0);
    drop(retained);
    assert_eq!(ledger.metrics().unwrap().active_leases, 0);
    assert_eq!(
        ledger
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        0
    );
}

#[tokio::test]
async fn changed_connection_epoch_seals_every_escaped_page_handle() {
    // Arrange
    let (registry, _) = registry(1, 1);
    let f = selected(registry.clone()).await;
    let retained = registry.page(ObjectId::new(3)).unwrap().unwrap();
    let other = CdpTopology::new(
        ObjectId::new(1),
        ConnectionEpoch::new(2),
        CdpTopologyLimits::new(16, 1, 1, 4, 4, 64),
    )
    .unwrap();
    // Act
    let result = registry.reconcile(&other);
    // Assert
    assert!(result.is_err());
    assert!(retained.is_sealed());
    assert_eq!(f.runtime.is_object_live(ObjectId::new(4)), Ok(true));
}

#[test]
fn profile_rejects_invalid_capacity_and_scope_arithmetic_overflow() {
    // Arrange
    let invalid = [(0, 1), (2, 1), (1, usize::MAX)];
    // Act
    let results = invalid
        .map(|(pages, owners)| FrameLifecycleRegistryProfile::new(96, pages, owners, 8, 4, 64));
    // Assert
    assert!(results.iter().all(Result::is_err));
}

#[tokio::test]
async fn shutdown_rejects_escaped_reads_while_an_observation_callback_holds_the_lock() {
    struct PausedSink {
        entered: std::sync::mpsc::SyncSender<()>,
        release: Mutex<std::sync::mpsc::Receiver<()>>,
    }
    impl FrameLifecycleNoticeSink for PausedSink {
        fn observe(&self, notice: NativeLifecycleNotice<'_>) -> FrameObservationResult {
            if matches!(notice, NativeLifecycleNotice::Lifecycle { .. }) {
                self.entered.send(())?;
                self.release
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(5))?;
            }
            Ok(())
        }
    }
    // Arrange
    let profile = FrameLifecycleRegistryProfile::new(96, 1, 1, 8, 4, 64).unwrap();
    let requirements = profile.requirements();
    let ledger = ProductionResourceLedger::new(exact_limits(requirements), requirements).unwrap();
    let (entered, waiting) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let registry = Arc::new(
        FrameLifecycleRegistry::new(
            profile,
            1,
            &ledger,
            Arc::new(Clock),
            Arc::new(PausedSink {
                entered,
                release: Mutex::new(released),
            }),
        )
        .unwrap(),
    );
    let f = selected(registry.clone()).await;
    let retained = registry.page(ObjectId::new(3)).unwrap().unwrap();
    let worker_registry = registry.clone();
    let topology = f.topology.clone();
    let worker =
        std::thread::spawn(move || {
            worker_registry.event(topology.lock().unwrap().topology(),
        &CdpEvent::new("Page.lifecycleEvent".into(), serde_json::json!({
            "frameId":"target-a", "loaderId":"loader-target-a", "name":"load", "timestamp":0
        }), Some("session-a".into())))
        });
    waiting.recv_timeout(Duration::from_secs(5)).unwrap();
    // Act: seal cannot wait for the borrowed-notice callback to return.
    registry.seal();
    let result = retained.snapshot(ObjectId::new(4));
    release.send(()).unwrap();
    // Assert
    assert!(result.is_err());
    assert!(retained.is_sealed());
    assert!(worker.join().unwrap().is_err());
}
