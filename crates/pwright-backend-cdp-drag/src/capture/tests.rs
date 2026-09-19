use super::*;
use crate::{DragDataLimits, DragDataProfile};
use pwright_cdp::event_fanout::ReclaimingEventFanout;
use pwright_core::ResourceLimits;
use serde_json::json;
use std::{collections::VecDeque, num::NonZeroUsize, task::Waker};
use tokio::sync::Notify;

struct FakeSession {
    events: ReclaimingEventFanout<CdpEvent>,
    calls: Mutex<Vec<bool>>,
    gates: Mutex<VecDeque<Option<Arc<Notify>>>>,
    failures: Mutex<VecDeque<bool>>,
}
impl FakeSession {
    fn new(capacity: usize) -> Arc<Self> {
        Arc::new(Self {
            events: ReclaimingEventFanout::new(capacity, 1).unwrap(),
            calls: Mutex::new(Vec::new()),
            gates: Mutex::new(VecDeque::new()),
            failures: Mutex::new(VecDeque::new()),
        })
    }
    fn send(&self, method: &str, session: &str, text: &str) {
        self.events
            .send(CdpEvent::new(
                method.into(),
                json!({"data":{
                    "items":[{"mimeType":"text/plain","data":text}],"dragOperationsMask":1
                }}),
                Some(session.into()),
            ))
            .unwrap();
    }
}
impl DragInterceptionClient for FakeSession {
    fn session_id(&self) -> Option<&str> {
        Some("s1")
    }
    fn subscribe(&self) -> Result<CdpEventReceiver> {
        self.events
            .subscribe()
            .map(Into::into)
            .map_err(|_| error("subscription failed"))
    }
    fn set_interception(self: Arc<Self>, enabled: bool) -> DragToggleFuture {
        Box::pin(async move {
            // The real native enable must also have an already installed cursor.
            assert_eq!(self.events.metrics().subscribers, usize::from(enabled));
            self.calls.lock().unwrap().push(enabled);
            let gate = self.gates.lock().unwrap().pop_front().flatten();
            let fail = self.failures.lock().unwrap().pop_front().unwrap_or(false);
            if let Some(gate) = gate {
                gate.notified().await;
            }
            if fail {
                Err(error("native toggle reply lost"))
            } else {
                Ok(())
            }
        })
    }
}
fn setup(client: Arc<FakeSession>) -> (DragCaptureFactory, ProductionResourceLedger) {
    let profile = DragDataProfile::new(
        ResourceScope::extension(14, 61),
        DragDataLimits {
            items: 2,
            files: 0,
            text_bytes: 128,
        },
        NonZeroUsize::new(1).unwrap(),
    )
    .unwrap();
    let req = profile
        .requirements()
        .combine_concurrent(DragCaptureFactory::requirements());
    let bytes = req.class(ResourceClass::BackendStateBytes);
    let resources = ProductionResourceLedger::new(
        ResourceLimits::deny_all(req.scopes, req.leases).with_class(
            ResourceClass::BackendStateBytes,
            bytes.global,
            bytes.per_scope,
        ),
        req,
    )
    .unwrap();
    let store = DragDataStore::new(profile, &resources).unwrap();
    let factory =
        DragCaptureFactory::new(client, store, ResourceScope::extension(14, 62), &resources)
            .unwrap();
    (factory, resources)
}
fn pending<F: Future>(future: Pin<&mut F>) {
    assert!(
        future
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
}

#[tokio::test]
async fn unpolled_capture_has_no_subscription_or_io() {
    // Arrange
    let client = FakeSession::new(4);
    let (factory, resources) = setup(client.clone());
    let mut capture = factory.start().unwrap();
    // Act
    capture.retire().await.unwrap();
    // Assert
    assert!(capture.is_retired());
    assert_eq!(client.events.metrics().subscribers, 0);
    assert!(client.calls.lock().unwrap().is_empty());
    drop(capture);
    drop(factory);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn first_matching_event_wins_and_retained_data_outlives_capture() {
    // Arrange
    let client = FakeSession::new(8);
    let (factory, resources) = setup(client.clone());
    let mut capture = factory.start().unwrap();
    capture.enable().await.unwrap();
    client.send("Runtime.consoleAPICalled", "s1", "irrelevant");
    client.send("Input.dragIntercepted", "another-session", "wrong");
    client.send("Input.dragIntercepted", "s1", "first");
    client.send("Input.dragIntercepted", "s1", "second");
    // Act
    let data = capture.receive().await.unwrap();
    let again = capture.receive().await.unwrap();
    capture.retire().await.unwrap();
    // Assert
    assert_eq!(data.data().items[0].data, "first");
    assert!(data.shares_payload_with(&again));
    assert_eq!(*client.calls.lock().unwrap(), [true, false]);
    assert_eq!(client.events.metrics().subscribers, 0);
    assert_eq!(client.events.metrics().queued_events, 0);
    drop(again);
    drop(capture);
    drop(factory);
    assert_eq!(resources.metrics().unwrap().active_leases, 2);
    drop(data);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn cancelled_enable_drains_same_command_then_disables() {
    // Arrange
    let client = FakeSession::new(4);
    let gate = Arc::new(Notify::new());
    client.gates.lock().unwrap().push_back(Some(gate.clone()));
    let (factory, _) = setup(client.clone());
    let mut capture = factory.start().unwrap();
    let mut enabling = Box::pin(capture.enable());
    pending(enabling.as_mut());
    drop(enabling);
    // Act
    let mut retirement = Box::pin(capture.retire());
    pending(retirement.as_mut());
    drop(retirement);
    // Assert
    assert_eq!(client.events.metrics().subscribers, 0);
    assert_eq!(*client.calls.lock().unwrap(), [true]);
    assert!(capture.enable().await.is_err());
    assert!(capture.receive().await.is_err());
    gate.notify_one();
    capture.retire().await.unwrap();
    assert!(capture.is_retired());
    assert_eq!(*client.calls.lock().unwrap(), [true, false]);
}

#[tokio::test]
async fn failed_enable_still_requires_disable() {
    // Arrange
    let client = FakeSession::new(4);
    client.failures.lock().unwrap().push_back(true);
    let (factory, _) = setup(client.clone());
    let mut capture = factory.start().unwrap();
    // Act
    let failure = capture.enable().await;
    // Assert
    assert!(failure.is_err());
    assert!(!capture.is_retired());
    capture.retire().await.unwrap();
    assert_eq!(*client.calls.lock().unwrap(), [true, false]);
    assert!(capture.is_retired());
}

#[tokio::test]
async fn failed_disable_keeps_exclusivity_until_explicit_retry_and_drop() {
    // Arrange
    let client = FakeSession::new(4);
    client.failures.lock().unwrap().extend([false, true]);
    let (factory, resources) = setup(client.clone());
    let mut capture = factory.start().unwrap();
    capture.enable().await.unwrap();
    // Act
    let failure = capture.retire().await;
    // Assert
    assert!(failure.is_err());
    assert!(!capture.is_retired());
    assert!(factory.clone().start().is_err());
    assert_eq!(resources.metrics().unwrap().active_leases, 3);
    capture.retire().await.unwrap();
    assert!(factory.start().is_err());
    drop(capture);
    let next = factory.start().unwrap();
    assert_eq!(next.epoch(), 2);
    assert_eq!(*client.calls.lock().unwrap(), [true, false, false]);
}

#[tokio::test]
async fn abandoned_disable_future_is_resumed_without_resubmission() {
    // Arrange
    let client = FakeSession::new(4);
    let gate = Arc::new(Notify::new());
    client
        .gates
        .lock()
        .unwrap()
        .extend([None, Some(gate.clone())]);
    let (factory, _) = setup(client.clone());
    let mut capture = factory.start().unwrap();
    capture.enable().await.unwrap();
    // Act
    let mut retirement = Box::pin(capture.retire());
    pending(retirement.as_mut());
    drop(retirement);
    gate.notify_one();
    capture.retire().await.unwrap();
    // Assert
    assert!(capture.is_retired());
    assert_eq!(*client.calls.lock().unwrap(), [true, false]);
}

#[tokio::test]
async fn abandoned_receive_keeps_cursor_and_recovers_delivered_event() {
    // Arrange
    let client = FakeSession::new(4);
    let (factory, _) = setup(client.clone());
    let mut capture = factory.start().unwrap();
    capture.enable().await.unwrap();
    // Act
    let mut receiving = Box::pin(capture.receive());
    pending(receiving.as_mut());
    drop(receiving);
    client.send("Input.dragIntercepted", "s1", "after cancel");
    let data = capture.receive().await.unwrap();
    // Assert
    assert_eq!(data.data().items[0].data, "after cancel");
    capture.retire().await.unwrap();
}

#[tokio::test]
async fn lag_is_terminal_capture_failure_not_silent_latest_event_success() {
    // Arrange
    let client = FakeSession::new(1);
    let (factory, _) = setup(client.clone());
    let mut capture = factory.start().unwrap();
    capture.enable().await.unwrap();
    client.send("Input.dragIntercepted", "s1", "lost");
    client.send("Input.dragIntercepted", "s1", "retained");
    // Act
    let failure = capture.receive().await;
    // Assert
    assert!(failure.err().unwrap().to_string().contains("lagged"));
    assert!(capture.receive().await.is_err());
    assert!(!capture.is_retired());
    assert_eq!(client.events.metrics().subscribers, 0);
    capture.retire().await.unwrap();
}

#[tokio::test]
async fn closed_stream_and_malformed_payload_fail_without_claiming_cleanup() {
    for closed in [false, true] {
        // Arrange
        let client = FakeSession::new(4);
        let (factory, _) = setup(client.clone());
        let mut capture = factory.start().unwrap();
        capture.enable().await.unwrap();
        if closed {
            client.events.close();
        } else {
            client
                .events
                .send(CdpEvent::new(
                    "Input.dragIntercepted".into(),
                    json!({}),
                    Some("s1".into()),
                ))
                .unwrap();
        }
        // Act
        let failure = capture.receive().await;
        // Assert
        assert!(failure.is_err());
        assert!(!capture.is_retired());
        assert!(capture.receive().await.is_err());
        capture.retire().await.unwrap();
    }
}

#[tokio::test]
async fn subscription_failure_never_toggles_native_interception() {
    // Arrange
    let client = FakeSession::new(4);
    let existing = client.events.subscribe().unwrap();
    let (factory, _) = setup(client.clone());
    let mut capture = factory.start().unwrap();
    // Act
    let failure = capture.enable().await;
    // Assert
    assert!(failure.is_err());
    assert!(capture.is_retired());
    assert!(client.calls.lock().unwrap().is_empty());
    drop(existing);
}

#[tokio::test]
async fn fresh_subscription_does_not_replay_previous_interval_events() {
    // Arrange
    let client = FakeSession::new(4);
    let (factory, _) = setup(client.clone());
    let mut first = factory.start().unwrap();
    first.enable().await.unwrap();
    client.send("Input.dragIntercepted", "s1", "old");
    first.retire().await.unwrap();
    drop(first);
    let mut next = factory.start().unwrap();
    next.enable().await.unwrap();
    client.send("Input.dragIntercepted", "s1", "new");
    // Act
    let data = next.receive().await.unwrap();
    // Assert
    assert_eq!(next.epoch(), 2);
    assert_eq!(data.data().items[0].data, "new");
    next.retire().await.unwrap();
}

#[tokio::test]
async fn dropping_dirty_owner_poisoned_factory_cannot_reenable() {
    // Arrange
    let client = FakeSession::new(4);
    let (factory, _) = setup(client.clone());
    let mut capture = factory.start().unwrap();
    capture.enable().await.unwrap();
    // Act
    drop(capture);
    // Assert
    assert!(factory.start().is_err());
    assert_eq!(*client.calls.lock().unwrap(), [true]);
    assert_eq!(client.events.metrics().subscribers, 0);
}

#[tokio::test]
async fn destruction_ack_rejects_replacement_binding_with_same_session_name() {
    // Arrange
    let client = FakeSession::new(4);
    let (factory, _) = setup(client.clone());
    let mut capture = factory.start().unwrap();
    capture.enable().await.unwrap();
    let replacement: Arc<dyn DragInterceptionClient> = FakeSession::new(4);
    // Act
    let failure = capture.acknowledge_destroyed_session(&replacement);
    // Assert
    assert!(failure.is_err());
    assert!(!capture.is_retired());
    let exact: Arc<dyn DragInterceptionClient> = client;
    capture.acknowledge_destroyed_session(&exact).unwrap();
    assert!(capture.is_retired());
}

#[tokio::test]
async fn bounded_receive_poll_yields_and_resumes_without_dropping_events() {
    // Arrange
    let client = FakeSession::new(64);
    let (factory, _) = setup(client.clone());
    let mut capture = factory.start().unwrap();
    capture.enable().await.unwrap();
    for _ in 0..40 {
        client.send("Runtime.consoleAPICalled", "s1", "other");
    }
    client.send("Input.dragIntercepted", "s1", "drag");
    // Act
    let mut receiving = Box::pin(capture.receive());
    pending(receiving.as_mut());
    let data = receiving.await.unwrap();
    // Assert
    assert_eq!(data.data().items[0].data, "drag");
    capture.retire().await.unwrap();
}

#[test]
fn epoch_exhaustion_rejects_before_subscription_or_io() {
    // Arrange
    let client = FakeSession::new(4);
    let (factory, _) = setup(client.clone());
    factory.0.state.lock().unwrap().next = u64::MAX;
    // Act
    let failure = factory.start();
    // Assert
    assert!(failure.is_err());
    assert!(client.calls.lock().unwrap().is_empty());
    assert_eq!(client.events.metrics().subscribers, 0);
}

#[tokio::test]
async fn destruction_ack_cannot_discard_pending_native_receipt() {
    // Arrange
    let client = FakeSession::new(4);
    let gate = Arc::new(Notify::new());
    client.gates.lock().unwrap().push_back(Some(gate.clone()));
    let (factory, _) = setup(client.clone());
    let mut capture = factory.start().unwrap();
    let mut enabling = Box::pin(capture.enable());
    pending(enabling.as_mut());
    drop(enabling);
    let exact: Arc<dyn DragInterceptionClient> = client.clone();
    // Act
    let failure = capture.acknowledge_destroyed_session(&exact);
    // Assert
    assert!(failure.is_err());
    assert!(!capture.is_retired());
    gate.notify_one();
    capture.retire().await.unwrap();
    assert_eq!(*client.calls.lock().unwrap(), [true, false]);
}

#[tokio::test]
async fn retire_without_receiving_reclaims_unread_cursor_payloads() {
    // Arrange
    let client = FakeSession::new(4);
    let (factory, _) = setup(client.clone());
    let mut capture = factory.start().unwrap();
    capture.enable().await.unwrap();
    client.send("Input.dragIntercepted", "s1", "discarded on cancellation");
    // Act
    capture.retire().await.unwrap();
    // Assert
    assert_eq!(client.events.metrics().subscribers, 0);
    assert_eq!(client.events.metrics().queued_events, 0);
    assert_eq!(factory.0.store.active_records(), 0);
    assert!(capture.receive().await.is_err());
}
