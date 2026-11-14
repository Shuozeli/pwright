use super::*;
use crate::frame_lifecycle::{
    FrameLifecycleClock, FrameLifecycleNoticeSink, PageLifecycleObservation,
    PageLifecycleObservationProfile,
};
use crate::{ChromiumFrameEventObserver, FrameObservationResult};
use pwright_backend_cdp_navigation::frame_lifecycle::{
    FrameLifecycleProfile,
    native::{CdpFrameLifecycleProfile, NativeLifecycleNotice},
};
use std::time::Duration;

#[derive(Default)]
struct Clock(AtomicUsize);
impl FrameLifecycleClock for Clock {
    fn now(&self) -> Duration {
        Duration::from_millis(self.0.load(Ordering::SeqCst) as u64)
    }
}
#[derive(Default)]
struct Sink {
    notices: AtomicUsize,
    fail: std::sync::atomic::AtomicBool,
}
impl FrameLifecycleNoticeSink for Sink {
    fn observe(&self, _: NativeLifecycleNotice<'_>) -> FrameObservationResult {
        self.notices.fetch_add(1, Ordering::SeqCst);
        if self.fail.load(Ordering::SeqCst) {
            return Err("selected sink failed".into());
        }
        Ok(())
    }
}
fn owner(
    page: u64,
) -> (
    Arc<PageLifecycleObservation>,
    Arc<Clock>,
    Arc<Sink>,
    ProductionResourceLedger,
) {
    let native = CdpFrameLifecycleProfile::new(
        FrameLifecycleProfile::new(ResourceScope::extension(92, 1), 8, 4).unwrap(),
        ResourceScope::extension(92, 2),
        64,
    )
    .unwrap();
    let profile = PageLifecycleObservationProfile::new(native, ResourceScope::extension(92, 3));
    let requirements = profile.requirements();
    let ledger = ProductionResourceLedger::new(exact_limits(requirements), requirements).unwrap();
    let clock = Arc::new(Clock::default());
    let sink = Arc::new(Sink::default());
    let observation = Arc::new(
        PageLifecycleObservation::new(
            profile,
            ObjectId::new(page),
            1,
            &ledger,
            clock.clone(),
            sink.clone(),
        )
        .unwrap(),
    );
    (observation, clock, sink, ledger)
}
fn selected_fixture(capacity: usize, observation: Arc<PageLifecycleObservation>) -> DriverFixture {
    let mut f = fixture(capacity);
    f.factory
        .client("target-a")
        .set_frame_tree(frame_tree(serde_json::json!({
            "frame": frame_value("target-a", None),
            "childFrames": [{"frame": frame_value("child", Some("target-a"))}]
        })));
    f.driver = f.driver.with_frame_event_observer(observation).unwrap();
    f
}
fn load(native: &str, loader: &str, session: &str) -> CdpEvent {
    CdpEvent::new(
        "Page.lifecycleEvent".into(),
        serde_json::json!({
            "frameId":native,"loaderId":loader,"name":"load","timestamp":0
        }),
        Some(session.into()),
    )
}

#[tokio::test]
async fn discovered_page_enables_lifecycle_before_debugger_resume() {
    // Arrange
    let (observation, _, _, _) = owner(3);
    let mut f = discovery_fixture(16);
    f.factory.set_frame_tree(
        "target-a",
        frame_tree(serde_json::json!({
            "frame": frame_value("target-a", None)
        })),
    );
    f.driver = f
        .driver
        .with_frame_event_observer(observation.clone())
        .unwrap();
    f.sender
        .send(attached_page_event("target-a", "session-a", None, None))
        .unwrap();
    // Act
    f.driver.receive_once(&f.runtime).await.unwrap();
    // Assert
    assert_eq!(
        f.topology
            .lock()
            .unwrap()
            .topology()
            .page_object_id("target-a"),
        Some(ObjectId::new(3))
    );
    assert!(
        observation
            .snapshot(ObjectId::new(4))
            .unwrap()
            .unwrap()
            .document
            .is_some()
    );
    let methods = f.factory.client("target-a").method_names();
    let index = |method: &str| methods.iter().position(|m| m == method).unwrap();
    assert!(index("Network.enable") < index("Runtime.runIfWaitingForDebugger"));
    assert!(index("Page.setLifecycleEventsEnabled") < index("Runtime.runIfWaitingForDebugger"));
}

#[tokio::test]
async fn discovered_page_queues_lifecycle_subscription_while_network_enable_is_pending() {
    // Arrange
    let (observation, _, _, _) = owner(3);
    let mut f = discovery_fixture(16);
    f.factory.set_frame_tree(
        "target-a",
        frame_tree(serde_json::json!({
            "frame": frame_value("target-a", None)
        })),
    );
    f.driver = f.driver.with_frame_event_observer(observation).unwrap();
    f.factory
        .create_session("session-a".into(), "target-a".into());
    let client = f.factory.client("target-a");
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_network_gate(Some(Arc::clone(&gate)));
    f.sender
        .send(attached_page_event("target-a", "session-a", None, None))
        .unwrap();
    let mut receive = Box::pin(f.driver.receive_once(&f.runtime));

    // Act
    assert!(matches!(
        receive
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop())),
        Poll::Pending
    ));

    // Assert
    assert_eq!(
        client.method_names(),
        ["Network.enable", "Page.setLifecycleEventsEnabled"]
    );
    gate.notify_one();
    receive.await.unwrap();
}

#[tokio::test]
async fn recursive_popup_sends_page_observation_and_resume_while_lifecycle_ack_is_pending() {
    // Arrange
    let (observation, _, _, _) = owner(3);
    let mut f = discovery_fixture_with_startup(16, false, true);
    f.factory.set_frame_tree(
        "target-a",
        frame_tree(serde_json::json!({
            "frame": frame_value("target-a", None)
        })),
    );
    f.driver = f.driver.with_frame_event_observer(observation).unwrap();
    f.factory
        .create_session("session-a".into(), "target-a".into());
    let client = f.factory.client("target-a");
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_network_gate(Some(Arc::clone(&gate)));
    f.sender
        .send(attached_page_event("target-a", "session-a", None, None))
        .unwrap();
    let mut receive = Box::pin(f.driver.receive_once(&f.runtime));

    // Act
    assert!(matches!(
        receive
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop())),
        Poll::Pending
    ));

    // Assert
    let methods = client.method_names();
    let index = |method: &str| {
        methods
            .iter()
            .position(|candidate| candidate == method)
            .unwrap()
    };
    assert!(index("Page.enable") < index("Runtime.runIfWaitingForDebugger"));
    assert!(index("Network.enable") < index("Runtime.runIfWaitingForDebugger"));
    assert!(index("Page.setLifecycleEventsEnabled") < index("Runtime.runIfWaitingForDebugger"));
    gate.notify_one();
    receive.await.unwrap();
}

#[tokio::test]
async fn stale_renderer_snapshot_cannot_replace_the_current_document() {
    // Arrange
    let (observation, _, sink, _) = owner(3);
    let mut f = selected_fixture(16, observation.clone());
    f.driver.initialize_snapshot(&f.runtime, 0).await.unwrap();
    let before = observation.snapshot(ObjectId::new(4)).unwrap();
    let mut frame = frame_tree(serde_json::json!({"frame":frame_value("target-a", None)})).frame;
    frame.loader_id = "stale-document".into();
    // Act
    observation
        .initialize_frame(
            f.topology.lock().unwrap().topology(),
            "retired-session",
            &frame,
        )
        .unwrap();
    // Assert
    assert_eq!(observation.snapshot(ObjectId::new(4)).unwrap(), before);
    assert_eq!(sink.notices.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn initial_tree_seeds_documents_and_domains_before_snapshot() {
    // Arrange
    let (observation, _, sink, _) = owner(3);
    let mut f = selected_fixture(16, observation.clone());
    // Act
    f.driver.initialize_snapshot(&f.runtime, 0).await.unwrap();
    f.sender
        .send(load("child", "loader-child", "session-a"))
        .unwrap();
    f.driver.receive_once(&f.runtime).await.unwrap();
    // Assert
    let child = f
        .topology
        .lock()
        .unwrap()
        .topology()
        .frame_object_id("child")
        .unwrap();
    assert!(observation.snapshot(child).unwrap().unwrap().load);
    assert!(
        !observation
            .snapshot(ObjectId::new(4))
            .unwrap()
            .unwrap()
            .load
    );
    assert_eq!(sink.notices.load(Ordering::SeqCst), 3);
    let methods = f.factory.client("target-a").method_names();
    let index = |method: &str| methods.iter().position(|m| m == method).unwrap();
    assert!(index("Network.enable") < index("Page.getFrameTree"));
    assert!(index("Page.setLifecycleEventsEnabled") < index("Page.getFrameTree"));
}

#[tokio::test]
async fn another_page_does_not_select_domains_or_receive_frames() {
    // Arrange
    let (observation, _, sink, _) = owner(99);
    let mut f = selected_fixture(16, observation.clone());
    // Act
    f.driver.initialize_snapshot(&f.runtime, 0).await.unwrap();
    f.sender
        .send(load("target-a", "loader-target-a", "session-a"))
        .unwrap();
    f.driver.receive_once(&f.runtime).await.unwrap();
    // Assert
    assert!(observation.snapshot(ObjectId::new(4)).unwrap().is_none());
    assert_eq!(sink.notices.load(Ordering::SeqCst), 0);
    assert!(
        !f.factory
            .client("target-a")
            .method_names()
            .iter()
            .any(|m| m == "Network.enable")
    );
}

#[tokio::test]
async fn stale_loader_and_session_do_not_publish_load() {
    // Arrange
    let (observation, _, sink, _) = owner(3);
    let mut f = selected_fixture(16, observation.clone());
    f.driver.initialize_snapshot(&f.runtime, 0).await.unwrap();
    // Act
    for event in [
        load("target-a", "old", "session-a"),
        load("target-a", "loader-target-a", "old-session"),
    ] {
        f.sender.send(event).unwrap();
        f.driver.receive_once(&f.runtime).await.unwrap();
    }
    // Assert
    assert!(
        !observation
            .snapshot(ObjectId::new(4))
            .unwrap()
            .unwrap()
            .load
    );
    assert_eq!(sink.notices.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn sink_failure_seals_retained_queries() {
    // Arrange
    let (observation, _, sink, _) = owner(3);
    let mut f = selected_fixture(16, observation.clone());
    f.driver.initialize_snapshot(&f.runtime, 0).await.unwrap();
    sink.fail.store(true, Ordering::SeqCst);
    f.sender
        .send(load("target-a", "loader-target-a", "session-a"))
        .unwrap();
    // Act
    let result = f.driver.receive_once(&f.runtime).await;
    // Assert
    assert!(result.is_err());
    assert!(observation.is_sealed());
    assert!(observation.snapshot(ObjectId::new(4)).is_err());
    assert!(observation.next_deadline().is_err());
}

#[tokio::test]
async fn cancelling_pending_driver_receive_seals_the_owner() {
    // Arrange
    let (observation, _, _, _) = owner(3);
    let mut f = selected_fixture(16, observation.clone());
    f.driver.initialize_snapshot(&f.runtime, 0).await.unwrap();
    let mut receive = Box::pin(f.driver.receive_once(&f.runtime));
    assert!(
        receive
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    drop(receive);
    // Assert
    assert!(observation.is_sealed());
    assert!(observation.snapshot(ObjectId::new(4)).is_err());
}

#[tokio::test]
async fn event_loss_seals_even_when_topology_can_resynchronize() {
    // Arrange
    let (observation, _, _, _) = owner(3);
    let mut f = selected_fixture(1, observation.clone());
    f.driver.initialize_snapshot(&f.runtime, 0).await.unwrap();
    for _ in 0..3 {
        f.sender
            .send(load("target-a", "loader-target-a", "session-a"))
            .unwrap();
    }
    // Act
    let _result = f.driver.receive_once(&f.runtime).await;
    // Assert
    assert!(observation.is_sealed());
    assert!(observation.snapshot(ObjectId::new(4)).is_err());
}

#[tokio::test]
async fn driver_drop_seals_but_storage_remains_admitted_until_last_owner_drop() {
    // Arrange
    let (observation, _, _, ledger) = owner(3);
    let mut f = selected_fixture(16, observation.clone());
    f.driver.initialize_snapshot(&f.runtime, 0).await.unwrap();
    // Act
    drop(f.driver);
    // Assert
    assert!(observation.is_sealed());
    assert!(
        ledger
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current
            > 0
    );
    drop(observation);
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
async fn normal_page_close_seals_owner_without_failing_driver() {
    // Arrange
    let (observation, _, _, _) = owner(3);
    let mut f = selected_fixture(16, observation.clone());
    f.driver.initialize_snapshot(&f.runtime, 0).await.unwrap();
    f.sender
        .send(CdpEvent::new(
            "Target.detachedFromTarget".into(),
            serde_json::json!({"targetId":"target-a", "sessionId":"session-a"}),
            None,
        ))
        .unwrap();
    // Act
    let result = f.driver.receive_once(&f.runtime).await;
    // Assert
    assert!(result.is_ok(), "{result:?}");
    assert!(observation.is_sealed());
    assert!(observation.snapshot(ObjectId::new(4)).is_err());
}

#[tokio::test]
async fn selecting_observer_after_initialization_is_rejected() {
    // Arrange
    let (observation, _, _, _) = owner(3);
    let mut f = fixture(16);
    f.driver.initialize_snapshot(&f.runtime, 0).await.unwrap();
    // Act
    let result = f.driver.with_frame_event_observer(observation);
    // Assert
    assert!(matches!(
        result,
        Err(ChromiumTopologyEventDriverError::AlreadyInstalled)
    ));
}
