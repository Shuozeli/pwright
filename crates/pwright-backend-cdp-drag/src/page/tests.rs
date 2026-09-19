use super::*;
use crate::{DragDataLimits, DragDataProfile, DragDataStore, DragListenerFactory, DragWorld};
use pwright_cdp::{CdpEvent, CdpEventReceiver, event_fanout::ReclaimingEventFanout};
use pwright_core::ResourceLimits;
use pwright_fake::FakeCdpClient;
use serde_json::json;
use std::{num::NonZeroUsize, task::Waker};
use tokio::sync::Notify;

#[derive(Debug)]
struct Sent {
    kind: DragEventKind,
    position: DragPosition,
    text: Option<String>,
    modifiers: Option<i64>,
}
struct Session {
    events: ReclaimingEventFanout<CdpEvent>,
    calls: Mutex<Vec<Sent>>,
    gate: Mutex<Option<Arc<Notify>>>,
    fail: Mutex<bool>,
}
impl Session {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            events: ReclaimingEventFanout::new(8, 1).unwrap(),
            calls: Mutex::new(Vec::new()),
            gate: Mutex::new(None),
            fail: Mutex::new(false),
        })
    }
}
impl DragInterceptionClient for Session {
    fn session_id(&self) -> Option<&str> {
        Some("s1")
    }
    fn subscribe(&self) -> Result<CdpEventReceiver> {
        self.events
            .subscribe()
            .map(Into::into)
            .map_err(|_| error("subscription full"))
    }
    fn set_interception(self: Arc<Self>, _: bool) -> DragToggleFuture {
        Box::pin(async { Ok(()) })
    }
}
impl DragEventClient for Session {
    fn dispatch_drag(
        self: Arc<Self>,
        kind: DragEventKind,
        position: DragPosition,
        data: Option<AdmittedDragData>,
        modifiers: Option<i64>,
    ) -> DragToggleFuture {
        Box::pin(async move {
            self.calls.lock().unwrap().push(Sent {
                kind,
                position,
                modifiers,
                text: data.as_ref().map(|data| data.data().items[0].data.clone()),
            });
            let gate = self.gate.lock().unwrap().clone();
            if let Some(gate) = gate {
                gate.notified().await;
            }
            if *self.fail.lock().unwrap() {
                Err(error("native reply failed"))
            } else {
                Ok(())
            }
        })
    }
}
struct Setup {
    resources: ProductionResourceLedger,
    session: Arc<Session>,
    captures: DragCaptureFactory,
    store: DragDataStore,
    listeners: DragListenerFactory,
    page: DragPage,
}
impl Setup {
    fn new() -> Self {
        Self::with_capacity(2)
    }
    fn with_capacity(capacity: usize) -> Self {
        let profile = DragDataProfile::new(
            ResourceScope::extension(14, 61),
            DragDataLimits {
                items: 2,
                files: 0,
                text_bytes: 128,
            },
            NonZeroUsize::new(2).unwrap(),
        )
        .unwrap();
        let req = profile
            .requirements()
            .combine_concurrent(DragListenerFactory::requirements(1).unwrap())
            .combine_concurrent(DragCaptureFactory::requirements())
            .combine_concurrent(DragMove::requirements(1).unwrap())
            .combine_concurrent(DragPage::requirements(capacity).unwrap());
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
        let session = Session::new();
        let captures = DragCaptureFactory::new(
            session.clone(),
            store.clone(),
            ResourceScope::extension(14, 62),
            &resources,
        )
        .unwrap();
        let listeners =
            DragListenerFactory::new("page", ResourceScope::extension(14, 60), 1, &resources)
                .unwrap();
        let page = DragPage::new(
            session.clone(),
            captures.clone(),
            ResourceScope::extension(14, 64),
            capacity,
            &resources,
        )
        .unwrap();
        Self {
            resources,
            session,
            captures,
            store,
            listeners,
            page,
        }
    }
    async fn captured(&self) -> (DragMove, AdmittedDragData) {
        let world = Arc::new(FakeCdpClient::from_html(""));
        world.set_evaluate_response(json!({"result":{"value":true}}));
        let session = self.session.clone();
        let mut movement = DragMove::start(
            ResourceScope::extension(14, 63),
            &self.resources,
            &self.listeners,
            &self.captures,
            &[DragWorld {
                client: world,
                unique_context: "world",
            }],
            Box::pin(async move {
                session.events.send(CdpEvent::new("Input.dragIntercepted".into(), json!({"data":{
                    "items":[{"mimeType":"text/plain","data":"captured"}],"dragOperationsMask":1
                }}), Some("s1".into()))).unwrap();
                Ok(())
            }),
        )
        .unwrap();
        let data = movement.result().await.unwrap().unwrap();
        (movement, data)
    }
    async fn dragging(&self) {
        let (movement, data) = self.captured().await;
        self.page
            .record_position(DragPosition { x: 10.0, y: 20.0 })
            .unwrap();
        let mut enter = self
            .page
            .enter(&movement, data, DragPosition { x: 10.0, y: 20.0 }, 0)
            .unwrap();
        assert!((&mut enter).await.unwrap());
    }
}
fn pending<F: Future>(future: F) {
    assert!(
        std::pin::pin!(future)
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
}

#[tokio::test]
async fn enter_installs_shared_state_on_first_poll_before_native_reply() {
    // Arrange
    let setup = Setup::new();
    let (movement, data) = setup.captured().await;
    let gate = Arc::new(Notify::new());
    *setup.session.gate.lock().unwrap() = Some(gate.clone());
    let mut enter = setup
        .page
        .enter(&movement, data, DragPosition { x: 1.0, y: 2.0 }, 8)
        .unwrap();
    assert!(!setup.page.is_dragging());
    assert!(setup.session.calls.lock().unwrap().is_empty());
    // Act
    pending(&mut enter);
    // Assert
    assert!(setup.page.clone().is_dragging());
    assert!(!enter.is_retired());
    {
        let calls = setup.session.calls.lock().unwrap();
        assert_eq!(calls[0].kind, DragEventKind::Enter);
        assert_eq!(calls[0].text.as_deref(), Some("captured"));
        assert_eq!(calls[0].modifiers, Some(8));
    }
    gate.notify_one();
    assert!((&mut enter).await.unwrap());
    assert!(enter.is_retired());
}

#[tokio::test]
async fn unpolled_enter_retirement_does_not_install_state_or_send_input() {
    // Arrange
    let setup = Setup::new();
    let (movement, data) = setup.captured().await;
    let mut enter = setup
        .page
        .enter(&movement, data, DragPosition { x: 1.0, y: 2.0 }, 0)
        .unwrap();
    // Act
    enter.retire().await.unwrap();
    // Assert
    assert!(!setup.page.is_dragging());
    assert!(enter.is_retired());
    assert!(setup.session.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn captured_data_requires_exact_factory_not_matching_session_text() {
    // Arrange
    let source = Setup::new();
    let other = Setup::new();
    let (movement, data) = source.captured().await;
    // Act
    let failure = other
        .page
        .enter(&movement, data, DragPosition { x: 1.0, y: 2.0 }, 0);
    // Assert
    assert!(failure.is_err());
    assert!(!other.page.is_dragging());
    assert_eq!(other.page.active_operations(), 0);
    assert!(other.session.calls.lock().unwrap().is_empty());
}

#[test]
fn page_constructor_requires_same_native_binding_as_capture() {
    // Arrange
    let setup = Setup::new();
    // Act
    let failure = DragPage::new(
        Session::new(),
        setup.captures.clone(),
        ResourceScope::extension(14, 64),
        2,
        &setup.resources,
    );
    // Assert
    assert!(failure.is_err());
}

#[tokio::test]
async fn successful_drop_releases_page_data_only_after_native_reply() {
    // Arrange
    let setup = Setup::new();
    setup.dragging().await;
    let gate = Arc::new(Notify::new());
    *setup.session.gate.lock().unwrap() = Some(gate.clone());
    let mut drop = setup
        .page
        .drop_at(DragPosition { x: 3.0, y: 4.0 }, 2)
        .unwrap();
    pending(&mut drop);
    assert!(setup.page.is_dragging());
    // Act
    gate.notify_one();
    assert!((&mut drop).await.unwrap());
    // Assert
    assert!(!setup.page.is_dragging());
    assert!(drop.is_retired());
    assert_eq!(setup.store.active_records(), 0);
}

#[tokio::test]
async fn abandoned_drop_result_drains_receipt_without_running_success_continuation() {
    // Arrange
    let setup = Setup::new();
    setup.dragging().await;
    let gate = Arc::new(Notify::new());
    *setup.session.gate.lock().unwrap() = Some(gate.clone());
    let mut drop = setup
        .page
        .drop_at(DragPosition { x: 3.0, y: 4.0 }, 0)
        .unwrap();
    pending(&mut drop);
    // Act
    pending(drop.retire());
    gate.notify_one();
    drop.retire().await.unwrap();
    // Assert
    assert!(drop.is_retired());
    assert!(setup.page.is_dragging());
    assert_eq!(setup.session.calls.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn cancel_uses_last_ordinary_move_position_and_no_data_or_modifiers() {
    // Arrange
    let setup = Setup::new();
    setup.dragging().await;
    setup
        .page
        .over(DragPosition { x: 30.0, y: 40.0 }, 4)
        .unwrap()
        .await
        .unwrap();
    // Act
    let result = setup.page.cancel().unwrap().await.unwrap();
    // Assert
    assert!(result);
    assert!(!setup.page.is_dragging());
    let calls = setup.session.calls.lock().unwrap();
    assert_eq!(calls[1].kind, DragEventKind::Over);
    assert_eq!(calls[1].modifiers, Some(4));
    assert_eq!(calls[2].kind, DragEventKind::Cancel);
    assert_eq!(calls[2].position, DragPosition { x: 30.0, y: 40.0 });
    assert_eq!(calls[2].modifiers, None);
    assert_eq!(calls[2].text, None);
}

#[tokio::test]
async fn abandoned_cancel_still_clears_after_native_reply_like_upstream_cancel_drag() {
    // Arrange
    let setup = Setup::new();
    setup.dragging().await;
    let gate = Arc::new(Notify::new());
    *setup.session.gate.lock().unwrap() = Some(gate.clone());
    let mut cancel = setup.page.cancel().unwrap();
    pending(&mut cancel);
    // Act
    gate.notify_one();
    cancel.retire().await.unwrap();
    // Assert
    assert!(cancel.is_retired());
    assert!(!setup.page.is_dragging());
}

#[tokio::test]
async fn cancel_without_active_drag_is_noop_false() {
    // Arrange
    let setup = Setup::new();
    // Act
    let cancelled = setup.page.cancel().unwrap().await.unwrap();
    // Assert
    assert!(!cancelled);
    assert!(setup.session.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn failed_native_drop_preserves_drag_data_for_later_cancel() {
    // Arrange
    let setup = Setup::new();
    setup.dragging().await;
    *setup.session.fail.lock().unwrap() = true;
    // Act
    let failure = setup
        .page
        .drop_at(DragPosition { x: 1.0, y: 2.0 }, 0)
        .unwrap()
        .await;
    // Assert
    assert!(failure.is_err());
    assert!(setup.page.is_dragging());
    *setup.session.fail.lock().unwrap() = false;
    setup.page.cancel().unwrap().await.unwrap();
    assert!(!setup.page.is_dragging());
}

#[tokio::test]
async fn capacity_is_held_until_completed_operation_owner_is_dropped() {
    // Arrange
    let setup = Setup::new();
    let mut first = setup.page.cancel().unwrap();
    let second = setup.page.cancel().unwrap();
    (&mut first).await.unwrap();
    // Act
    let rejected = setup.page.cancel();
    // Assert
    assert!(rejected.is_err());
    assert_eq!(setup.page.active_operations(), 2);
    drop(first);
    assert!(setup.page.cancel().is_ok());
    drop(second);
    assert_eq!(setup.page.active_operations(), 0);
}

#[test]
fn expanded_page_capacity_is_admitted_before_unpolled_owners_and_fully_released() {
    // Arrange
    let setup = Setup::with_capacity(256);
    let baseline = setup.resources.metrics().unwrap().active_leases;
    let operations: Vec<_> = (0..256).map(|_| setup.page.cancel().unwrap()).collect();
    // Act
    let rejected = setup.page.cancel();
    // Assert
    assert!(rejected.is_err());
    assert_eq!(setup.page.active_operations(), 256);
    assert_eq!(
        setup.resources.metrics().unwrap().active_leases,
        baseline + 256
    );
    assert!(setup.session.calls.lock().unwrap().is_empty());
    drop(operations);
    assert_eq!(setup.page.active_operations(), 0);
    assert_eq!(setup.resources.metrics().unwrap().active_leases, baseline);
    assert!(DragPage::requirements(0).is_err());
    assert!(DragPage::requirements(1025).is_err());
}

#[tokio::test]
async fn undrained_operation_drop_poisoned_page_cannot_dispatch_again() {
    // Arrange
    let setup = Setup::new();
    setup.dragging().await;
    *setup.session.gate.lock().unwrap() = Some(Arc::new(Notify::new()));
    let mut operation = setup.page.cancel().unwrap();
    pending(&mut operation);
    // Act
    drop(operation);
    // Assert
    assert!(setup.page.cancel().is_err());
    assert!(setup.page.is_dragging());
}

#[test]
fn nonfinite_coordinates_reject_before_operation_admission() {
    // Arrange
    let setup = Setup::new();
    // Act
    let failure = setup.page.over(
        DragPosition {
            x: f64::NAN,
            y: 1.0,
        },
        0,
    );
    // Assert
    assert!(failure.is_err());
    assert_eq!(setup.page.active_operations(), 0);
    assert!(setup.session.calls.lock().unwrap().is_empty());
}
