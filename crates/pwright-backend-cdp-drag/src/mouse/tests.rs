use super::*;
use crate::{
    DragDataLimits, DragDataProfile, DragDataStore, DragEventClient, DragEventKind,
    DragInterceptionClient, DragListenerFactory, DragWorld,
};
use pwright_backend_cdp_input::{CdpInputCoordinator, CdpInputPolicy, MouseClickOptions};
use pwright_cdp::{CdpEvent, CdpEventReceiver};
use pwright_core::ResourceLimits;
use pwright_fake::FakeCdpClient;
use serde_json::{Value, json};
use std::{
    future::ready,
    num::NonZeroUsize,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    task::Waker,
};
use tokio::sync::Notify;

// The same fake object implements input and drag, like the production session.
// Drag commands are recorded as Runtime.evaluate JSON for test observation only;
// real native serialization is covered by the ignored Chrome suite.
impl DragInterceptionClient for FakeCdpClient {
    fn session_id(&self) -> Option<&str> {
        Some("mouse-test")
    }
    fn subscribe(&self) -> Result<CdpEventReceiver> {
        CdpClient::subscribe_composable_events(self).map_err(|_| error("subscription failed"))
    }
    fn set_interception(self: Arc<Self>, _: bool) -> DragToggleFuture {
        Box::pin(async { Ok(()) })
    }
}
impl DragEventClient for FakeCdpClient {
    fn dispatch_drag(
        self: Arc<Self>,
        kind: DragEventKind,
        position: DragPosition,
        data: Option<crate::AdmittedDragData>,
        modifiers: Option<i64>,
    ) -> DragToggleFuture {
        Box::pin(async move {
            let value = json!({"drag":format!("{kind:?}"),"x":position.x,"y":position.y,
                "data":data.as_ref().map(|data| data.data()),"modifiers":modifiers});
            self.runtime_evaluate(&value.to_string()).await?;
            Ok(())
        })
    }
}
struct Source {
    resources: Arc<ProductionResourceLedger>,
    captures: DragCaptureFactory,
    listeners: DragListenerFactory,
    world: Arc<FakeCdpClient>,
    input: Arc<FakeCdpClient>,
    drag: AtomicBool,
    starts: AtomicUsize,
    after_move: Mutex<Option<Arc<Notify>>>,
}
impl DragMoveSource for Source {
    fn captures(&self) -> &DragCaptureFactory {
        &self.captures
    }
    fn start(&self, movement: DragToggleFuture) -> Result<DragMove> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        let drag = self.drag.load(Ordering::SeqCst);
        self.world.set_evaluate_responses([
            json!({"result":{"value":true}}),
            json!({"result":{"value":drag}}),
        ]);
        let client = self.input.clone();
        let after_move = self.after_move.lock().unwrap().clone();
        DragMove::start(
            ResourceScope::extension(14, 63),
            &self.resources,
            &self.listeners,
            &self.captures,
            &[DragWorld {
                client: self.world.clone(),
                unique_context: "world",
            }],
            Box::pin(async move {
                movement.await?;
                if let Some(gate) = after_move {
                    gate.notified().await;
                }
                if drag {
                    client.send_event(CdpEvent::new("Input.dragIntercepted".into(), json!({"data":{
                    "items":[{"mimeType":"text/plain","data":"dragged"}],"dragOperationsMask":1
                }}), Some("mouse-test".into())));
                }
                Ok(())
            }),
        )
    }
}
struct Setup {
    input: CdpInputCoordinator,
    client: Arc<FakeCdpClient>,
    source: Arc<Source>,
    page: DragPage,
    dispatcher: Arc<MouseDragDispatcher>,
    driver: MouseDragRetirementDriver,
}
impl Setup {
    fn new(capacity: usize) -> Self {
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
            .combine_concurrent(DragCaptureFactory::requirements())
            .combine_concurrent(DragListenerFactory::requirements(1).unwrap())
            .combine_concurrent(DragMove::requirements(1).unwrap())
            .combine_concurrent(DragPage::requirements(4).unwrap())
            .combine_concurrent(MouseDragDispatcher::requirements(capacity).unwrap());
        let bytes = req.class(ResourceClass::BackendStateBytes);
        let resources = Arc::new(
            ProductionResourceLedger::new(
                ResourceLimits::deny_all(req.scopes, req.leases).with_class(
                    ResourceClass::BackendStateBytes,
                    bytes.global,
                    bytes.per_scope,
                ),
                req,
            )
            .unwrap(),
        );
        let client = Arc::new(FakeCdpClient::from_html(""));
        let world = Arc::new(FakeCdpClient::from_html(""));
        let store = DragDataStore::new(profile, &resources).unwrap();
        let captures = DragCaptureFactory::new(
            client.clone(),
            store,
            ResourceScope::extension(14, 62),
            &resources,
        )
        .unwrap();
        let listeners =
            DragListenerFactory::new("dispatch", ResourceScope::extension(14, 60), 1, &resources)
                .unwrap();
        let page = DragPage::new(
            client.clone(),
            captures.clone(),
            ResourceScope::extension(14, 64),
            4,
            &resources,
        )
        .unwrap();
        let source = Arc::new(Source {
            resources: resources.clone(),
            captures,
            listeners,
            world,
            input: client.clone(),
            drag: AtomicBool::new(false),
            starts: AtomicUsize::new(0),
            after_move: Mutex::new(None),
        });
        let (dispatcher, driver) = MouseDragDispatcher::new(
            client.clone(),
            page.clone(),
            source.clone(),
            ResourceScope::extension(14, 65),
            capacity,
            &resources,
        )
        .unwrap();
        Self {
            input: CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved),
            client,
            source,
            page,
            dispatcher: Arc::new(dispatcher),
            driver,
        }
    }
    async fn move_to(&self, x: f64, for_click: bool) -> Result<()> {
        let mut action = self
            .input
            .begin_transaction(self.client.clone(), None)
            .await
            .unwrap();
        let mut movement = action.raw_mouse_click_with_dispatcher(
            move_options(x, for_click),
            |_| ready(Ok(())),
            self.dispatcher.clone(),
        )?;
        let result = (&mut movement).await;
        movement.retire().await?;
        result
    }
    async fn down(&self, button: pwright_cdp::MouseButton) -> Result<()> {
        let mut action = self
            .input
            .begin_transaction(self.client.clone(), None)
            .await
            .unwrap();
        let mut down = action.raw_mouse_down_with_dispatcher(button, 1, self.dispatcher.clone())?;
        let result = (&mut down).await;
        down.retire().await?;
        result
    }
    async fn up(&self) -> Result<()> {
        let mut action = self
            .input
            .begin_transaction(self.client.clone(), None)
            .await
            .unwrap();
        let mut up = action.raw_mouse_up_with_dispatcher(
            pwright_cdp::MouseButton::Left,
            1,
            self.dispatcher.clone(),
        )?;
        let result = (&mut up).await;
        up.retire().await?;
        result
    }
    fn pointers(&self) -> Vec<Value> {
        self.client
            .calls()
            .into_iter()
            .filter(|(method, _)| method == "Input.dispatchMouseEvent")
            .map(|(_, args)| args[0].clone())
            .collect()
    }
    fn drags(&self) -> Vec<Value> {
        self.client
            .calls()
            .into_iter()
            .filter(|(method, _)| method == "Runtime.evaluate")
            .map(|(_, args)| serde_json::from_str(args[0].as_str().unwrap()).unwrap())
            .collect()
    }
    fn keys(&self) -> Vec<Value> {
        self.client
            .calls()
            .into_iter()
            .filter(|(method, _)| method == "Input.dispatchKeyEvent")
            .map(|(_, args)| args[0].clone())
            .collect()
    }
    async fn key_action(&self) -> pwright_backend_cdp_input::CdpInputTransaction {
        let mut action = self
            .input
            .begin_transaction(self.client.clone(), None)
            .await
            .unwrap();
        action
            .set_key_down_interceptor(self.dispatcher.clone())
            .unwrap();
        action
    }
}
fn move_options(x: f64, for_click: bool) -> MouseClickOptions {
    MouseClickOptions::new(x, 40.0)
        .unwrap()
        .with_count(0.0.try_into().unwrap())
        .with_for_click(for_click)
}
fn pending<F: Future>(future: F) {
    assert!(
        std::pin::pin!(future)
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
}

#[tokio::test]
async fn escape_without_drag_passes_keydown_and_keyup() {
    // Arrange
    let setup = Setup::new(4);
    let mut action = setup.key_action().await;
    // Act
    action.key_down_named("Escape", false).await.unwrap();
    action.key_up_named("Escape", false).await.unwrap();
    // Assert
    let keys = setup.keys();
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0]["type"], "rawKeyDown");
    assert_eq!(keys[1]["type"], "keyUp");
    assert!(setup.drags().is_empty());
    assert!(setup.dispatcher.is_drained());
}

#[tokio::test]
async fn consumed_escape_keeps_logical_state_and_never_consumes_keyup() {
    // Arrange
    let setup = Setup::new(4);
    setup.down(pwright_cdp::MouseButton::Left).await.unwrap();
    setup.source.drag.store(true, Ordering::SeqCst);
    setup.move_to(70.0, false).await.unwrap();
    let mut action = setup.key_action().await;
    // Act
    action.key_down_named("Escape", false).await.unwrap();
    // Assert
    assert!(setup.keys().is_empty());
    assert_eq!(setup.drags().last().unwrap()["drag"], "Cancel");
    assert!(!setup.page.is_dragging());
    action.key_down_named("Escape", false).await.unwrap();
    action.key_up_named("Escape", false).await.unwrap();
    let keys = setup.keys();
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0]["autoRepeat"], true);
    assert_eq!(keys[1]["type"], "keyUp");
    assert!(setup.dispatcher.is_drained());
}

#[tokio::test]
async fn dropped_escape_keeps_native_cancel_in_page_quarantine_until_reply() {
    // Arrange
    let mut setup = Setup::new(4);
    setup.down(pwright_cdp::MouseButton::Left).await.unwrap();
    setup.source.drag.store(true, Ordering::SeqCst);
    setup.move_to(70.0, false).await.unwrap();
    let gate = Arc::new(Notify::new());
    setup.client.set_evaluate_gate(Some(gate.clone()));
    let mut action = setup.key_action().await;
    let mut down = Box::pin(action.key_down_named("Escape", false));
    pending(&mut down);
    // Act
    drop(down);
    // Assert
    assert!(setup.page.is_dragging());
    assert_eq!(setup.dispatcher.quarantined_operations(), 1);
    pending(setup.driver.retire_quarantined());
    gate.notify_one();
    setup.driver.retire_quarantined().await.unwrap();
    assert!(!setup.page.is_dragging());
    assert_eq!(setup.page.active_operations(), 0);
    assert!(setup.dispatcher.is_drained());
    assert!(setup.keys().is_empty());
    assert_eq!(
        setup
            .drags()
            .iter()
            .filter(|event| event["drag"] == "Cancel")
            .count(),
        1
    );
}

#[tokio::test]
async fn failed_native_escape_cancel_preserves_drag_and_does_not_send_keydown() {
    // Arrange
    let setup = Setup::new(4);
    setup.down(pwright_cdp::MouseButton::Left).await.unwrap();
    setup.source.drag.store(true, Ordering::SeqCst);
    setup.move_to(70.0, false).await.unwrap();
    setup.client.set_evaluate_errors(["cancel failed".into()]);
    let mut action = setup.key_action().await;
    // Act
    let failed = action.key_down_named("Escape", false).await;
    // Assert
    assert!(failed.is_err());
    assert!(setup.page.is_dragging());
    assert!(setup.keys().is_empty());
    assert!(setup.dispatcher.is_drained());
    action.key_down_named("Escape", false).await.unwrap();
    assert!(!setup.page.is_dragging());
}

#[tokio::test]
async fn unpolled_escape_interception_drops_without_native_cancel() {
    // Arrange
    let setup = Setup::new(4);
    setup.down(pwright_cdp::MouseButton::Left).await.unwrap();
    setup.source.drag.store(true, Ordering::SeqCst);
    setup.move_to(70.0, false).await.unwrap();
    let baseline = setup.source.resources.metrics().unwrap().active_leases;
    let operation = setup
        .dispatcher
        .clone()
        .intercept(setup.client.clone(), "Escape")
        .unwrap();
    // Act
    drop(operation);
    // Assert
    assert!(setup.page.is_dragging());
    assert_eq!(setup.drags().len(), 1);
    assert!(setup.dispatcher.is_drained());
    assert_eq!(
        setup.source.resources.metrics().unwrap().active_leases,
        baseline
    );
}

#[tokio::test]
async fn key_interception_rejects_foreign_native_client_and_double_selection() {
    // Arrange
    let setup = Setup::new(4);
    let other = Arc::new(FakeCdpClient::from_html(""));
    let mut action = setup
        .input
        .begin_transaction(other.clone(), None)
        .await
        .unwrap();
    action
        .set_key_down_interceptor(setup.dispatcher.clone())
        .unwrap();
    // Act
    let failed = action.key_down_named("Escape", false).await;
    // Assert
    assert!(failed.is_err());
    assert!(other.calls().is_empty());
    assert!(setup.client.calls().is_empty());
    assert!(
        action
            .set_key_down_interceptor(setup.dispatcher.clone())
            .is_err()
    );
}

#[tokio::test]
async fn ordinary_key_does_not_acquire_drag_admission() {
    // Arrange
    let setup = Setup::new(1);
    let held = setup
        .dispatcher
        .clone()
        .intercept(setup.client.clone(), "Escape")
        .unwrap();
    let mut action = setup.key_action().await;
    // Act
    action.key_down_named("KeyA", false).await.unwrap();
    // Assert
    assert_eq!(setup.keys().len(), 1);
    assert_eq!(setup.keys()[0]["code"], "KeyA");
    drop(held);
    assert!(setup.dispatcher.is_drained());
}

#[tokio::test]
async fn legacy_exclusive_transaction_rejects_interception_without_native_effects() {
    // Arrange
    let setup = Setup::new(1);
    let legacy = CdpInputCoordinator::default();
    let mut action = legacy
        .begin_transaction(setup.client.clone(), None)
        .await
        .unwrap();
    // Act
    let rejected = action.set_key_down_interceptor(setup.dispatcher.clone());
    // Assert
    assert!(rejected.is_err());
    assert!(setup.client.calls().is_empty());
    assert!(setup.dispatcher.is_drained());
}

#[test]
fn slot_masks_keep_word_boundaries_independent() {
    // Arrange
    let mut mask = SlotMask::default();
    let slots = [0, 31, 32, 63, 64, 127, 128, 511, 512, 519, 1023];
    // Act
    for slot in slots {
        mask.insert(slot);
    }
    mask.remove(64);
    // Assert
    assert_eq!(mask.count(), slots.len() - 1);
    assert!(!mask.contains(64));
    for slot in slots.into_iter().filter(|slot| *slot != 64) {
        assert!(mask.contains(slot));
    }
    assert!(!mask.is_empty());
}

#[test]
fn dispatcher_capacity_rejects_outside_fixed_mask_before_allocation() {
    // Arrange
    let capacities = [0, MAX_OPERATIONS + 1];
    // Act
    let results = capacities.map(MouseDragDispatcher::requirements);
    // Assert
    assert!(results.into_iter().all(|result| result.is_err()));
    assert!(MouseDragDispatcher::requirements(MAX_OPERATIONS).is_ok());
}

#[tokio::test]
async fn maximum_click_batch_dispatches_all_65_children_without_capture() {
    // Arrange
    let setup = Setup::new(65);
    let mut action = setup
        .input
        .begin_transaction(setup.client.clone(), None)
        .await
        .unwrap();
    let mut click = action
        .raw_mouse_click_with_dispatcher(
            MouseClickOptions::new(80.0, 40.0)
                .unwrap()
                .with_count(32.0.try_into().unwrap()),
            |_| ready(Ok(())),
            setup.dispatcher.clone(),
        )
        .unwrap();
    // Act
    let result = (&mut click).await;
    click.retire().await.unwrap();
    // Assert
    result.unwrap();
    let events = setup.pointers();
    assert_eq!(events.len(), 65);
    assert_eq!(events[0]["type"], "mouseMoved");
    assert_eq!(events[63]["clickCount"], 32);
    assert_eq!(events[64]["clickCount"], 32);
    assert_eq!(setup.source.starts.load(Ordering::SeqCst), 0);
    assert!(setup.dispatcher.is_drained());
}

#[tokio::test]
async fn eight_maximum_batches_retain_and_fairly_drain_520_quarantined_children() {
    // Arrange
    let mut setup = Setup::new(520);
    let baseline = setup.source.resources.metrics().unwrap().active_leases;
    let gates: Vec<_> = (0..520).map(|_| Arc::new(Notify::new())).collect();
    setup
        .client
        .set_pointer_gates(gates.iter().cloned().map(Some));
    let mut action = setup
        .input
        .begin_transaction(setup.client.clone(), None)
        .await
        .unwrap();
    let mut clicks: Vec<_> = (0..8)
        .map(|_| {
            action
                .raw_mouse_click_with_dispatcher(
                    MouseClickOptions::new(80.0, 40.0)
                        .unwrap()
                        .with_count(32.0.try_into().unwrap()),
                    |_| ready(Ok(())),
                    setup.dispatcher.clone(),
                )
                .unwrap()
        })
        .collect();
    for click in &mut clicks {
        pending(click);
    }
    assert_eq!(setup.pointers().len(), 520);
    // Act
    drop(clicks);
    // Assert
    assert_eq!(setup.dispatcher.quarantined_operations(), 520);
    assert_eq!(
        setup.source.resources.metrics().unwrap().active_leases,
        baseline + 520
    );
    pending(setup.driver.retire_quarantined());
    for gate in &gates[..519] {
        gate.notify_one();
    }
    pending(setup.driver.retire_quarantined());
    assert_eq!(setup.dispatcher.quarantined_operations(), 1);
    assert!(!setup.dispatcher.is_drained());
    assert_eq!(
        setup.source.resources.metrics().unwrap().active_leases,
        baseline + 1
    );
    gates[519].notify_one();
    setup.driver.retire_quarantined().await.unwrap();
    assert!(setup.dispatcher.is_drained());
    assert_eq!(
        setup.source.resources.metrics().unwrap().active_leases,
        baseline
    );
    assert_eq!(setup.pointers().len(), 520);
}

#[tokio::test]
async fn ordinary_nonleft_move_is_direct_without_capture() {
    // Arrange
    let setup = Setup::new(4);
    // Act
    setup.move_to(50.0, false).await.unwrap();
    // Assert
    assert_eq!(setup.source.starts.load(Ordering::SeqCst), 0);
    assert_eq!(setup.pointers()[0]["x"], 50.0);
    assert!(setup.dispatcher.is_drained());
}

#[tokio::test]
async fn held_left_move_uses_capture_and_installs_active_page_drag() {
    // Arrange
    let setup = Setup::new(4);
    setup.down(pwright_cdp::MouseButton::Left).await.unwrap();
    setup.source.drag.store(true, Ordering::SeqCst);
    // Act
    setup.move_to(70.0, false).await.unwrap();
    // Assert
    assert_eq!(setup.source.starts.load(Ordering::SeqCst), 1);
    assert!(setup.page.is_dragging());
    assert_eq!(setup.drags()[0]["drag"], "Enter");
    assert!(setup.dispatcher.is_drained());
}

#[tokio::test]
async fn active_drag_moves_over_suppresses_down_and_drops_on_up() {
    // Arrange
    let setup = Setup::new(4);
    setup.down(pwright_cdp::MouseButton::Left).await.unwrap();
    setup.source.drag.store(true, Ordering::SeqCst);
    setup.move_to(70.0, false).await.unwrap();
    let pointers = setup.pointers().len();
    // Act
    setup.move_to(220.0, false).await.unwrap();
    setup.down(pwright_cdp::MouseButton::Right).await.unwrap();
    setup.up().await.unwrap();
    // Assert
    assert_eq!(setup.source.starts.load(Ordering::SeqCst), 1);
    assert_eq!(setup.pointers().len(), pointers);
    assert_eq!(
        setup
            .drags()
            .iter()
            .map(|v| v["drag"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["Enter", "Over", "Drop"]
    );
    assert!(!setup.page.is_dragging());
    // The suppressed down still changed the independent logical Mouse state.
    setup.move_to(230.0, true).await.unwrap();
    assert_eq!(setup.pointers().last().unwrap()["buttons"], 2);
}

#[tokio::test]
async fn for_click_bypasses_drag_and_does_not_rewind_cancel_position() {
    // Arrange
    let setup = Setup::new(4);
    setup.down(pwright_cdp::MouseButton::Left).await.unwrap();
    setup.source.drag.store(true, Ordering::SeqCst);
    setup.move_to(70.0, false).await.unwrap();
    // Act
    setup.move_to(500.0, true).await.unwrap();
    setup.page.cancel().unwrap().await.unwrap();
    // Assert
    assert_eq!(setup.source.starts.load(Ordering::SeqCst), 1);
    assert_eq!(setup.pointers().last().unwrap()["x"], 500.0);
    assert_eq!(setup.drags().last().unwrap()["x"], 70.0);
}

#[tokio::test]
async fn deferred_native_move_reads_live_buttons_modifiers_and_force() {
    // Arrange
    let setup = Setup::new(4);
    setup.down(pwright_cdp::MouseButton::Left).await.unwrap();
    let gate = Arc::new(Notify::new());
    setup.source.world.set_evaluate_gate(Some(gate.clone()));
    let mut action = setup
        .input
        .begin_transaction(setup.client.clone(), None)
        .await
        .unwrap();
    let mut movement = action
        .raw_mouse_click_with_dispatcher(
            move_options(80.0, false),
            |_| ready(Ok(())),
            setup.dispatcher.clone(),
        )
        .unwrap();
    pending(&mut movement);
    setup.up().await.unwrap();
    action.key_down_named("Shift", false).await.unwrap();
    // Act
    setup.source.world.set_evaluate_gate(None);
    gate.notify_one();
    (&mut movement).await.unwrap();
    movement.retire().await.unwrap();
    // Assert
    let events = setup.pointers();
    let moved = events.last().unwrap();
    assert_eq!(moved["type"], "mouseMoved");
    assert_eq!(moved["button"], "left");
    assert_eq!(moved["buttons"], 0);
    assert_eq!(moved["modifiers"], 8);
    assert_eq!(moved["force"], 0.0);
}

#[tokio::test]
async fn click_bypass_while_left_held_never_installs_capture() {
    // Arrange
    let setup = Setup::new(4);
    setup.down(pwright_cdp::MouseButton::Left).await.unwrap();
    // Act
    setup.move_to(80.0, true).await.unwrap();
    // Assert
    assert_eq!(setup.source.starts.load(Ordering::SeqCst), 0);
    assert_eq!(setup.pointers().last().unwrap()["buttons"], 1);
}

#[tokio::test]
async fn stopped_dispatcher_rejects_before_native_work() {
    // Arrange
    let setup = Setup::new(4);
    setup.dispatcher.stop();
    // Act
    let result = setup.move_to(80.0, false).await;
    // Assert
    assert!(result.is_err());
    assert!(setup.client.calls().is_empty());
    assert!(setup.dispatcher.is_drained());
}

#[tokio::test]
async fn dropped_pending_input_is_retained_by_page_quarantine_until_drained() {
    // Arrange
    let mut setup = Setup::new(4);
    let gate = Arc::new(Notify::new());
    setup.client.set_pointer_gates([Some(gate.clone())]);
    let mut action = setup
        .input
        .begin_transaction(setup.client.clone(), None)
        .await
        .unwrap();
    let mut movement = action
        .raw_mouse_click_with_dispatcher(
            move_options(80.0, true),
            |_| ready(Ok(())),
            setup.dispatcher.clone(),
        )
        .unwrap();
    pending(&mut movement);
    // Act
    drop(movement);
    // Assert
    assert_eq!(setup.dispatcher.quarantined_operations(), 1);
    assert!(!setup.dispatcher.is_drained());
    assert!(setup.move_to(90.0, true).await.is_err());
    pending(setup.driver.retire_quarantined());
    assert_eq!(setup.dispatcher.quarantined_operations(), 1);
    gate.notify_one();
    setup.driver.retire_quarantined().await.unwrap();
    assert!(setup.dispatcher.is_drained());
    assert_eq!(setup.pointers().len(), 1);
}

#[tokio::test]
async fn failed_move_cleanup_transfers_owner_before_terminal_retirement_error() {
    // Arrange
    let mut setup = Setup::new(4);
    setup.down(pwright_cdp::MouseButton::Left).await.unwrap();
    let gate = Arc::new(Notify::new());
    setup.client.set_pointer_gates([Some(gate.clone())]);
    let mut action = setup
        .input
        .begin_transaction(setup.client.clone(), None)
        .await
        .unwrap();
    let mut movement = action
        .raw_mouse_click_with_dispatcher(
            move_options(80.0, false),
            |_| ready(Ok(())),
            setup.dispatcher.clone(),
        )
        .unwrap();
    pending(&mut movement);
    setup
        .source
        .world
        .set_evaluate_errors(["cleanup failed".into()]);
    gate.notify_one();
    // Act
    let failure = movement.retire().await;
    // Assert
    assert!(failure.is_err());
    assert_eq!(setup.dispatcher.quarantined_operations(), 1);
    assert_eq!(setup.source.listeners.active_listeners(), 1);
    setup.driver.retire_quarantined().await.unwrap();
    assert_eq!(setup.source.listeners.active_listeners(), 0);
    assert!(setup.dispatcher.is_drained());
    assert_eq!(setup.source.starts.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn foreign_client_is_rejected_even_with_shared_input_coordinator() {
    // Arrange
    let setup = Setup::new(4);
    let other = Arc::new(FakeCdpClient::from_html(""));
    let mut action = setup
        .input
        .begin_transaction(other.clone(), None)
        .await
        .unwrap();
    let mut movement = action
        .raw_mouse_click_with_dispatcher(
            move_options(80.0, true),
            |_| ready(Ok(())),
            setup.dispatcher.clone(),
        )
        .unwrap();
    // Act
    let result = (&mut movement).await;
    movement.retire().await.unwrap();
    // Assert
    assert!(result.is_err());
    assert!(other.calls().is_empty());
    assert!(setup.client.calls().is_empty());
}

#[tokio::test]
async fn drag_enter_reads_modifiers_after_intervening_input() {
    // Arrange
    let setup = Setup::new(4);
    setup.down(pwright_cdp::MouseButton::Left).await.unwrap();
    setup.source.drag.store(true, Ordering::SeqCst);
    let gate = Arc::new(Notify::new());
    *setup.source.after_move.lock().unwrap() = Some(gate.clone());
    let mut action = setup
        .input
        .begin_transaction(setup.client.clone(), None)
        .await
        .unwrap();
    let mut movement = action
        .raw_mouse_click_with_dispatcher(
            move_options(80.0, false),
            |_| ready(Ok(())),
            setup.dispatcher.clone(),
        )
        .unwrap();
    pending(&mut movement);
    assert_eq!(setup.pointers().last().unwrap()["modifiers"], 0);
    // Act
    action.key_down_named("Shift", false).await.unwrap();
    gate.notify_one();
    (&mut movement).await.unwrap();
    movement.retire().await.unwrap();
    // Assert
    assert_eq!(setup.drags()[0]["modifiers"], 8);
}

#[tokio::test]
async fn capacity_bounds_outstanding_dispatch_and_quarantine_preserves_both_slots() {
    // Arrange
    let mut setup = Setup::new(2);
    let first_gate = Arc::new(Notify::new());
    let second_gate = Arc::new(Notify::new());
    setup
        .client
        .set_pointer_gates([Some(first_gate.clone()), Some(second_gate.clone())]);
    let mut action = setup
        .input
        .begin_transaction(setup.client.clone(), None)
        .await
        .unwrap();
    let mut first = action
        .raw_mouse_click_with_dispatcher(
            move_options(80.0, true),
            |_| ready(Ok(())),
            setup.dispatcher.clone(),
        )
        .unwrap();
    let mut second = action
        .raw_mouse_click_with_dispatcher(
            move_options(90.0, true),
            |_| ready(Ok(())),
            setup.dispatcher.clone(),
        )
        .unwrap();
    pending(&mut first);
    pending(&mut second);
    // Act
    let rejected = setup.move_to(100.0, true).await;
    drop(first);
    drop(second);
    // Assert
    assert!(rejected.is_err());
    assert_eq!(setup.pointers().len(), 2);
    assert_eq!(setup.dispatcher.quarantined_operations(), 2);
    first_gate.notify_one();
    second_gate.notify_one();
    setup.driver.retire_quarantined().await.unwrap();
    assert!(setup.dispatcher.is_drained());
}
