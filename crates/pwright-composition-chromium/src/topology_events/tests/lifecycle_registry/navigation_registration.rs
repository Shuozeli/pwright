use super::*;
use crate::frame_lifecycle::{
    NavigationRegistrationProfile, NavigationWaitHandle, PageLifecycleHandle,
};
use pwright_backend_cdp_navigation::frame_lifecycle::navigation::{
    FrameNavigationWaitProfile, NavigationPoll, NavigationWaitError,
};
use pwright_cap_navigation::WaitUntil;
use std::sync::atomic::AtomicU64;

#[derive(Default)]
struct TestClock(AtomicU64);
impl FrameLifecycleClock for TestClock {
    fn now(&self) -> Duration {
        Duration::from_millis(self.0.load(Ordering::SeqCst))
    }
}
struct Setup {
    driver: DriverFixture,
    registry: Arc<FrameLifecycleRegistry>,
    ledger: ProductionResourceLedger,
    clock: Arc<TestClock>,
    registration: NavigationRegistrationProfile,
    page: PageLifecycleHandle,
}
impl Setup {
    async fn new(slots: usize) -> Self {
        let profile = FrameLifecycleRegistryProfile::new(96, 2, 3, 8, 8, 64)
            .unwrap()
            .with_navigation_slots(slots)
            .unwrap();
        let registration = NavigationRegistrationProfile::new(
            FrameNavigationWaitProfile::new(ResourceScope::extension(97, 0), 4, 4, 64, 128, 256)
                .unwrap(),
            ResourceScope::extension(97, 1),
        );
        let mut requirements = profile.requirements();
        for _ in 0..8 {
            requirements = requirements.combine_concurrent(registration.requirements());
        }
        let ledger =
            ProductionResourceLedger::new(exact_limits(requirements), requirements).unwrap();
        let clock = Arc::new(TestClock::default());
        let registry = Arc::new(
            FrameLifecycleRegistry::new(profile, 1, &ledger, clock.clone(), Arc::new(Sink))
                .unwrap(),
        );
        let driver = selected(registry.clone()).await;
        let page = registry.page(ObjectId::new(3)).unwrap().unwrap();
        Self {
            driver,
            registry,
            ledger,
            clock,
            registration,
            page,
        }
    }
    fn register(&self, wait: WaitUntil, deadline: Option<Duration>) -> NavigationWaitHandle {
        self.page
            .register_navigation(
                self.registration,
                &self.ledger,
                ObjectId::new(4),
                wait,
                deadline,
            )
            .unwrap()
    }
    async fn event(&mut self, method: &str, params: serde_json::Value) {
        self.driver
            .sender
            .send(CdpEvent::new(
                method.into(),
                params,
                Some("session-a".into()),
            ))
            .unwrap();
        self.driver
            .driver
            .receive_once(&self.driver.runtime)
            .await
            .unwrap();
    }
    async fn commit(&mut self, loader: &str) {
        let mut frame = frame_value("target-a", None);
        frame["loaderId"] = loader.into();
        self.event(
            "Page.frameNavigated",
            serde_json::json!({"frame":frame, "type":"Navigation"}),
        )
        .await;
    }
    async fn load(&mut self, loader: &str) {
        self.event("Page.lifecycleEvent", serde_json::json!({"frameId":"target-a", "loaderId":loader, "name":"load", "timestamp":0})).await;
    }
    async fn hash(&mut self) {
        self.event("Page.navigatedWithinDocument", serde_json::json!({"frameId":"target-a", "url":"about:blank#hash", "navigationType":"fragment"})).await;
    }
}

#[tokio::test]
async fn ordered_driver_captures_load_before_later_document_erases_it() {
    // Arrange
    let mut f = Setup::new(2).await;
    let mut ticket = f.register(WaitUntil::Load, None);
    ticket.receipt(Some("first")).unwrap();
    f.commit("first").await;
    // Act
    f.load("first").await;
    f.commit("second").await;
    // Assert: latest readiness is false, but the pending wait already settled.
    assert!(!f.page.snapshot(ObjectId::new(4)).unwrap().unwrap().load);
    let result = ticket.try_take().unwrap().unwrap();
    assert_eq!(
        result.outcome().unwrap().unwrap(),
        NavigationPoll::Complete {
            frame: ObjectId::new(4),
            response_json: None
        }
    );
    assert!(ticket.try_take().is_err());
    assert!(ticket.receipt(None).is_err());
    assert!(!f.page.is_sealed());
}

#[tokio::test]
async fn native_response_envelope_is_retained_before_receipt_and_before_finish() {
    // Arrange
    let mut f = Setup::new(1).await;
    let mut ticket = f.register(WaitUntil::Load, None);
    f.event("Network.requestWillBeSent", serde_json::json!({"frameId":"target-a", "requestId":"doc", "loaderId":"doc", "type":"Document", "request":{"url":"http://example.test/a"}})).await;
    f.event("Network.responseReceived", serde_json::json!({"requestId":"doc", "response":{"status":404,"url":"http://example.test/a"}})).await;
    f.commit("doc").await;
    f.load("doc").await;
    // Act
    ticket.receipt(Some("doc")).unwrap();
    // Assert
    let result = ticket.try_take().unwrap().unwrap();
    let NavigationPoll::Complete {
        response_json: Some(bytes),
        ..
    } = result.outcome().unwrap().unwrap()
    else {
        panic!("missing response");
    };
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(bytes).unwrap()["status"],
        404
    );
}

#[tokio::test]
async fn receipt_uses_current_snapshot_not_earlier_pre_reply_load() {
    // Arrange
    let mut f = Setup::new(1).await;
    let mut ticket = f.register(WaitUntil::Load, None);
    f.commit("first").await;
    f.load("first").await;
    f.commit("second").await;
    // Act
    ticket.receipt(Some("first")).unwrap();
    // Assert: pinned gotoImpl starts the lifecycle wait after the receipt.
    assert!(ticket.try_take().unwrap().is_none());
    f.load("second").await;
    assert!(
        ticket
            .try_take()
            .unwrap()
            .unwrap()
            .outcome()
            .unwrap()
            .is_ok()
    );
}

#[tokio::test]
async fn elapsed_idle_is_observed_before_a_following_commit_clears_it() {
    // Arrange
    let mut f = Setup::new(1).await;
    let mut ticket = f.register(WaitUntil::NetworkIdle, None);
    ticket.receipt(None).unwrap();
    f.hash().await;
    assert_eq!(
        ticket.next_deadline().unwrap(),
        Some(Duration::from_millis(500))
    );
    // Act
    f.clock.0.store(500, Ordering::SeqCst);
    f.commit("later").await;
    // Assert
    assert!(
        !f.page
            .snapshot(ObjectId::new(4))
            .unwrap()
            .unwrap()
            .network_idle
    );
    assert!(
        ticket
            .try_take()
            .unwrap()
            .unwrap()
            .outcome()
            .unwrap()
            .is_ok()
    );
}

#[tokio::test]
async fn timeout_checkpoint_settles_only_its_own_navigation() {
    // Arrange
    let f = Setup::new(2).await;
    let mut expired = f.register(WaitUntil::Load, Some(Duration::from_millis(100)));
    let mut active = f.register(WaitUntil::Load, Some(Duration::from_millis(1000)));
    // Act
    f.clock.0.store(100, Ordering::SeqCst);
    let result = expired.try_take().unwrap().unwrap();
    // Assert
    assert_eq!(result.outcome().unwrap(), Err(NavigationWaitError::Timeout));
    assert!(active.try_take().unwrap().is_none());
    assert!(!f.page.is_sealed());
}

#[tokio::test]
async fn pending_commit_notifies_consumer_to_schedule_new_earlier_idle_deadline() {
    // Arrange
    let mut f = Setup::new(1).await;
    f.clock.0.store(500, Ordering::SeqCst);
    assert!(
        f.page
            .snapshot(ObjectId::new(4))
            .unwrap()
            .unwrap()
            .network_idle
    );
    let mut ticket = f.register(WaitUntil::NetworkIdle, Some(Duration::from_secs(10)));
    ticket.receipt(Some("doc")).unwrap();
    assert_eq!(
        ticket.next_deadline().unwrap(),
        Some(Duration::from_secs(10))
    );
    // Act
    f.clock.0.store(600, Ordering::SeqCst);
    f.commit("doc").await;
    // Assert: pending is still a change the consumer must wake to observe.
    tokio::time::timeout(Duration::from_secs(1), ticket.changed())
        .await
        .unwrap()
        .unwrap();
    assert!(ticket.try_take().unwrap().is_none());
    assert_eq!(
        ticket.next_deadline().unwrap(),
        Some(Duration::from_millis(1100))
    );
    // Snapshot reads did not generate another notification permit.
    let notified = ticket.changed();
    tokio::pin!(notified);
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(notified.as_mut().poll(cx)))
            .await
            .is_pending()
    );
}

#[tokio::test]
async fn admission_failure_does_not_seal_page_and_drop_reclaims_slot() {
    // Arrange
    let f = Setup::new(1).await;
    let ticket = f.register(WaitUntil::Load, None);
    // Act
    let exhausted = f.page.register_navigation(
        f.registration,
        &f.ledger,
        ObjectId::new(4),
        WaitUntil::Load,
        None,
    );
    // Assert
    assert!(exhausted.is_err());
    assert!(!f.page.is_sealed());
    drop(ticket);
    let replacement = f.register(WaitUntil::Load, None);
    assert!(!f.page.is_sealed());
    drop(replacement);
}

#[tokio::test]
async fn completion_notifies_even_when_consumer_registers_after_event() {
    // Arrange
    let mut f = Setup::new(1).await;
    let mut ticket = f.register(WaitUntil::Commit, None);
    ticket.receipt(Some("doc")).unwrap();
    // Act
    f.commit("doc").await;
    // Assert
    tokio::time::timeout(Duration::from_secs(1), ticket.changed())
        .await
        .unwrap()
        .unwrap();
    assert!(ticket.try_take().unwrap().is_some());
}

#[tokio::test]
async fn closing_page_wakes_pending_consumer_without_closing_other_pages() {
    // Arrange
    let mut f = Setup::new(1).await;
    add_second_page(&f.driver);
    f.driver
        .driver
        .initialize_snapshot(&f.driver.runtime, 0)
        .await
        .unwrap();
    let second = f.registry.page(ObjectId::new(12)).unwrap().unwrap();
    let mut ticket = f.register(WaitUntil::Load, None);
    let wait = ticket.changed();
    tokio::pin!(wait);
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(wait.as_mut().poll(cx)))
            .await
            .is_pending()
    );
    // Act
    close_first(&mut f.driver).await;
    // Assert
    assert!(
        tokio::time::timeout(Duration::from_secs(1), wait)
            .await
            .unwrap()
            .is_err()
    );
    assert!(!second.is_sealed());
}

#[tokio::test]
async fn moved_result_keeps_only_core_lease_after_page_and_ticket_drop() {
    // Arrange
    let mut f = Setup::new(1).await;
    let mut ticket = f.register(WaitUntil::Commit, None);
    ticket.receipt(None).unwrap();
    f.hash().await;
    let result = ticket.try_take().unwrap().unwrap();
    let ledger = f.ledger.clone();
    // Act
    drop(ticket);
    drop(f);
    // Assert
    assert_eq!(ledger.metrics().unwrap().active_leases, 1);
    assert!(result.outcome().unwrap().is_ok());
    drop(result);
    assert_eq!(ledger.metrics().unwrap().active_leases, 0);
}
