use super::*;
use crate::{DragDataLimits, DragDataProfile, DragDataStore};
use pwright_cdp::{CdpEvent, CdpEventReceiver, event_fanout::ReclaimingEventFanout};
use pwright_core::ResourceLimits;
use pwright_fake::FakeCdpClient;
use serde_json::json;
use std::{
    num::NonZeroUsize,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    task::Waker,
};
use tokio::sync::Notify;

struct Session {
    events: ReclaimingEventFanout<CdpEvent>,
    toggles: Mutex<Vec<bool>>,
}
impl DragInterceptionClient for Session {
    fn session_id(&self) -> Option<&str> {
        Some("session")
    }
    fn subscribe(&self) -> Result<CdpEventReceiver> {
        self.events
            .subscribe()
            .map(Into::into)
            .map_err(|_| error("subscription full"))
    }
    fn set_interception(self: Arc<Self>, enabled: bool) -> DragToggleFuture {
        Box::pin(async move {
            self.toggles.lock().unwrap().push(enabled);
            Ok(())
        })
    }
}
struct Setup {
    session: Arc<Session>,
    resources: ProductionResourceLedger,
    listeners: DragListenerFactory,
    captures: DragCaptureFactory,
    worlds: Vec<Arc<FakeCdpClient>>,
}
impl Setup {
    fn new(worlds: usize) -> Self {
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
            .combine_concurrent(DragListenerFactory::requirements(worlds.max(1)).unwrap())
            .combine_concurrent(DragCaptureFactory::requirements())
            .combine_concurrent(DragMove::requirements(worlds).unwrap());
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
        let session = Arc::new(Session {
            events: ReclaimingEventFanout::new(8, 1).unwrap(),
            toggles: Mutex::new(Vec::new()),
        });
        let store = DragDataStore::new(profile, &resources).unwrap();
        let captures = DragCaptureFactory::new(
            session.clone(),
            store,
            ResourceScope::extension(14, 62),
            &resources,
        )
        .unwrap();
        let listeners = DragListenerFactory::new(
            "composition",
            ResourceScope::extension(14, 60),
            worlds.max(1),
            &resources,
        )
        .unwrap();
        let worlds = (0..worlds)
            .map(|_| {
                let client = Arc::new(FakeCdpClient::from_html(""));
                client.set_evaluate_responses([
                    json!({"result":{"value":true}}),
                    json!({"result":{"value":false}}),
                ]);
                client
            })
            .collect();
        Self {
            session,
            resources,
            listeners,
            captures,
            worlds,
        }
    }
    fn start(&self, movement: DragToggleFuture) -> DragMove {
        let worlds: Vec<_> = self
            .worlds
            .iter()
            .map(|client| DragWorld {
                client: client.clone(),
                unique_context: "world",
            })
            .collect();
        DragMove::start(
            ResourceScope::extension(14, 63),
            &self.resources,
            &self.listeners,
            &self.captures,
            &worlds,
            movement,
        )
        .unwrap()
    }
}
fn pending<F: Future>(future: F) {
    assert!(poll_borrowed(future, &mut Context::from_waker(Waker::noop())).is_pending());
}

struct WorldProof {
    client: Arc<dyn CdpClient>,
    destroyed: std::sync::atomic::AtomicBool,
}
impl DragWorldLifetime for WorldProof {
    fn world_destroyed(&self, client: &Arc<dyn CdpClient>, unique: &str) -> bool {
        unique == "world"
            && Arc::ptr_eq(&self.client, client)
            && self.destroyed.load(Ordering::Acquire)
    }
}
fn with_proof(setup: &Setup, proof: Arc<WorldProof>) -> DragMove {
    let worlds: Vec<_> = setup
        .worlds
        .iter()
        .map(|client| DragWorld {
            client: client.clone(),
            unique_context: "world",
        })
        .collect();
    DragMove::start_with_world_lifetime(
        ResourceScope::extension(14, 63),
        &setup.resources,
        &setup.listeners,
        &setup.captures,
        &worlds,
        Box::pin(async { Ok(()) }),
        Some(proof),
    )
    .unwrap()
}

#[tokio::test]
async fn retained_world_owner_survives_result_and_retirement_until_move_drop() {
    // Arrange
    let setup = Setup::new(1);
    let proof = Arc::new(WorldProof {
        client: setup.worlds[0].clone(),
        destroyed: false.into(),
    });
    let weak = Arc::downgrade(&proof);
    let mut movement = with_proof(&setup, proof);
    // Act
    movement.result().await.unwrap();
    movement.retire().await.unwrap();
    // Assert
    assert!(movement.is_retired());
    assert!(weak.upgrade().is_some());
    drop(movement);
    assert!(weak.upgrade().is_none());
}

#[tokio::test]
async fn destruction_during_setup_drains_original_receipt_without_new_cleanup() {
    // Arrange
    let setup = Setup::new(1);
    let gate = Arc::new(Notify::new());
    setup.worlds[0].set_evaluate_gate(Some(gate.clone()));
    let proof = Arc::new(WorldProof {
        client: setup.worlds[0].clone(),
        destroyed: false.into(),
    });
    let mut movement = with_proof(&setup, proof.clone());
    pending(movement.result());
    proof.destroyed.store(true, Ordering::Release);
    // Act
    pending(movement.retire());
    // Assert
    assert!(!movement.is_retired());
    assert_eq!(setup.worlds[0].calls().len(), 1);
    setup.worlds[0].set_evaluate_gate(None);
    gate.notify_one();
    movement.retire().await.unwrap();
    assert!(movement.is_retired());
    assert_eq!(setup.worlds[0].calls().len(), 1);
    assert!(setup.session.toggles.lock().unwrap().is_empty());
}

#[tokio::test]
async fn foreign_client_proof_cannot_skip_listener_cleanup() {
    // Arrange
    let setup = Setup::new(1);
    let gate = Arc::new(Notify::new());
    setup.worlds[0].set_evaluate_gate(Some(gate.clone()));
    let proof = Arc::new(WorldProof {
        client: Arc::new(FakeCdpClient::from_html("")),
        destroyed: true.into(),
    });
    let mut movement = with_proof(&setup, proof);
    pending(movement.result());
    setup.worlds[0].set_evaluate_gate(None);
    gate.notify_one();
    // Act
    movement.retire().await.unwrap();
    // Assert
    assert!(movement.is_retired());
    assert_eq!(setup.worlds[0].calls().len(), 2);
}

#[tokio::test]
async fn one_destroyed_world_does_not_skip_independent_live_world_cleanup() {
    // Arrange
    let setup = Setup::new(2);
    let gate = Arc::new(Notify::new());
    setup.worlds[0].set_evaluate_gate(Some(gate.clone()));
    let proof = Arc::new(WorldProof {
        client: setup.worlds[0].clone(),
        destroyed: true.into(),
    });
    let mut movement = with_proof(&setup, proof);
    pending(movement.result());
    // Act
    pending(movement.retire());
    // Assert
    assert_eq!(setup.worlds[1].calls().len(), 2);
    assert_eq!(setup.worlds[0].calls().len(), 1);
    setup.worlds[0].set_evaluate_gate(None);
    gate.notify_one();
    movement.retire().await.unwrap();
    assert!(movement.is_retired());
    assert_eq!(setup.worlds[0].calls().len(), 1);
}

#[tokio::test]
async fn no_drag_runs_native_move_after_setup_and_enable_then_cleans() {
    // Arrange
    let setup = Setup::new(1);
    let world = setup.worlds[0].clone();
    let session = setup.session.clone();
    let count = Arc::new(AtomicUsize::new(0));
    let seen = count.clone();
    let mut movement = setup.start(Box::pin(async move {
        assert_eq!(world.calls().len(), 1);
        assert_eq!(*session.toggles.lock().unwrap(), [true]);
        seen.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }));
    // Act
    let data = movement.result().await.unwrap();
    // Assert
    assert!(data.is_none());
    assert!(movement.is_retired());
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(setup.worlds[0].calls().len(), 2);
    assert_eq!(*setup.session.toggles.lock().unwrap(), [true, false]);
    assert_eq!(setup.session.events.metrics().subscribers, 0);
}

#[tokio::test]
async fn native_event_is_captured_when_any_world_observes_drag() {
    // Arrange
    let setup = Setup::new(2);
    setup.worlds[1].set_evaluate_response(json!({"result":{"value":true}}));
    let session = setup.session.clone();
    let mut movement = setup.start(Box::pin(async move {
        session
            .events
            .send(CdpEvent::new(
                "Input.dragIntercepted".into(),
                json!({"data":{
                    "items":[{"mimeType":"text/plain","data":"native"}],"dragOperationsMask":1
                }}),
                Some("session".into()),
            ))
            .unwrap();
        Ok(())
    }));
    // Act
    let data = movement.result().await.unwrap().unwrap();
    // Assert
    assert_eq!(data.data().items[0].data, "native");
    assert!(movement.is_retired());
    assert_eq!(setup.worlds[0].calls().len(), 2);
    assert_eq!(setup.worlds[1].calls().len(), 2);
    assert_eq!(*setup.session.toggles.lock().unwrap(), [true, false]);
}

#[tokio::test]
async fn independent_world_setups_are_polled_without_serial_waiting() {
    // Arrange
    let setup = Setup::new(2);
    let gate = Arc::new(Notify::new());
    setup.worlds[0].set_evaluate_gate(Some(gate.clone()));
    let mut movement = setup.start(Box::pin(async { Ok(()) }));
    // Act
    pending(movement.result());
    // Assert
    assert_eq!(setup.worlds[0].calls().len(), 1);
    assert_eq!(setup.worlds[1].calls().len(), 1);
    assert!(setup.session.toggles.lock().unwrap().is_empty());
    setup.worlds[0].set_evaluate_gate(None);
    gate.notify_one();
    assert!(movement.result().await.unwrap().is_none());
    assert!(movement.is_retired());
}

#[tokio::test]
async fn unpolled_cancellation_never_starts_any_child() {
    // Arrange
    let setup = Setup::new(1);
    let mut movement = setup.start(Box::pin(async { panic!("cancelled input must not run") }));
    // Act
    movement.retire().await.unwrap();
    // Assert
    assert!(movement.is_retired());
    assert!(setup.worlds[0].calls().is_empty());
    assert!(setup.session.toggles.lock().unwrap().is_empty());
    assert!(movement.result().await.is_err());
}

#[tokio::test]
async fn cancellation_during_setup_drains_listener_without_enabling_or_moving() {
    // Arrange
    let setup = Setup::new(1);
    let gate = Arc::new(Notify::new());
    setup.worlds[0].set_evaluate_gate(Some(gate.clone()));
    let mut movement = setup.start(Box::pin(async { panic!("cancelled move") }));
    pending(movement.result());
    // Act
    pending(movement.retire());
    setup.worlds[0].set_evaluate_gate(None);
    gate.notify_one();
    movement.retire().await.unwrap();
    // Assert
    assert!(movement.is_retired());
    assert_eq!(setup.worlds[0].calls().len(), 2);
    assert!(setup.session.toggles.lock().unwrap().is_empty());
}

#[tokio::test]
async fn cancellation_during_input_drains_before_disabling_interception() {
    // Arrange
    let setup = Setup::new(1);
    let gate = Arc::new(Notify::new());
    let release = gate.clone();
    let mut movement = setup.start(Box::pin(async move {
        release.notified().await;
        Ok(())
    }));
    pending(movement.result());
    // Act
    pending(movement.retire());
    // Assert
    assert!(!movement.is_retired());
    assert_eq!(*setup.session.toggles.lock().unwrap(), [true]);
    gate.notify_one();
    movement.retire().await.unwrap();
    assert!(movement.is_retired());
    assert_eq!(*setup.session.toggles.lock().unwrap(), [true, false]);
}

#[tokio::test]
async fn missing_expected_event_is_cancellable_without_waiting_for_data() {
    // Arrange
    let setup = Setup::new(1);
    setup.worlds[0].set_evaluate_responses([
        json!({"result":{"value":true}}),
        json!({"result":{"value":true}}),
    ]);
    let mut movement = setup.start(Box::pin(async { Ok(()) }));
    pending(movement.result());
    // Act
    movement.retire().await.unwrap();
    // Assert
    assert!(movement.is_retired());
    assert_eq!(*setup.session.toggles.lock().unwrap(), [true, false]);
    assert_eq!(setup.session.events.metrics().subscribers, 0);
}

#[tokio::test]
async fn failed_movement_still_cleans_listeners_and_disables() {
    // Arrange
    let setup = Setup::new(1);
    let mut movement = setup.start(Box::pin(async { Err(error("input reply failed")) }));
    // Act
    let failure = movement.result().await;
    // Assert
    assert!(
        failure
            .err()
            .unwrap()
            .to_string()
            .contains("input reply failed")
    );
    assert!(!movement.is_retired());
    movement.retire().await.unwrap();
    assert!(movement.is_retired());
    assert_eq!(setup.worlds[0].calls().len(), 2);
    assert_eq!(*setup.session.toggles.lock().unwrap(), [true, false]);
}

#[tokio::test]
async fn failed_cleanup_does_not_abandon_other_world_and_can_be_retried() {
    // Arrange
    let setup = Setup::new(2);
    let gate = Arc::new(Notify::new());
    let release = gate.clone();
    let mut movement = setup.start(Box::pin(async move {
        release.notified().await;
        Ok(())
    }));
    pending(movement.result());
    setup.worlds[0].set_evaluate_errors(["cleanup failed".into()]);
    gate.notify_one();
    // Act
    let failure = movement.retire().await;
    // Assert
    assert!(failure.is_err());
    assert!(!movement.is_retired());
    assert_eq!(setup.worlds[1].calls().len(), 2);
    assert_eq!(*setup.session.toggles.lock().unwrap(), [true, false]);
    movement.retire().await.unwrap();
    assert!(movement.is_retired());
    assert_eq!(setup.worlds[0].calls().len(), 3);
    assert_eq!(setup.worlds[1].calls().len(), 2);
}

#[tokio::test]
async fn pending_world_cleanup_does_not_block_other_world_cleanup() {
    // Arrange
    let setup = Setup::new(2);
    let input_gate = Arc::new(Notify::new());
    let release = input_gate.clone();
    let mut movement = setup.start(Box::pin(async move {
        release.notified().await;
        Ok(())
    }));
    pending(movement.result());
    let cleanup_gate = Arc::new(Notify::new());
    setup.worlds[0].set_evaluate_gate(Some(cleanup_gate.clone()));
    input_gate.notify_one();
    // Act
    pending(movement.retire());
    // Assert
    assert_eq!(setup.worlds[0].calls().len(), 2);
    assert_eq!(setup.worlds[1].calls().len(), 2);
    setup.worlds[0].set_evaluate_gate(None);
    cleanup_gate.notify_one();
    movement.retire().await.unwrap();
    assert!(movement.is_retired());
}

#[tokio::test]
async fn failed_setup_does_not_send_input_or_enable_interception() {
    // Arrange
    let setup = Setup::new(1);
    setup.worlds[0].set_evaluate_errors(["setup failed".into()]);
    let mut movement = setup.start(Box::pin(async { panic!("no movement after failed setup") }));
    // Act
    let failure = movement.result().await;
    // Assert
    assert!(failure.is_err());
    movement.retire().await.unwrap();
    assert!(movement.is_retired());
    assert!(setup.session.toggles.lock().unwrap().is_empty());
}

#[test]
fn invalid_world_construction_releases_fresh_children_without_io() {
    // Arrange
    let setup = Setup::new(1);
    let before = setup.resources.metrics().unwrap().active_leases;
    let worlds = [DragWorld {
        client: setup.worlds[0].clone(),
        unique_context: "",
    }];
    // Act
    let failure = DragMove::start(
        ResourceScope::extension(14, 63),
        &setup.resources,
        &setup.listeners,
        &setup.captures,
        &worlds,
        Box::pin(async { Ok(()) }),
    );
    // Assert
    assert!(failure.is_err());
    assert_eq!(setup.resources.metrics().unwrap().active_leases, before);
    assert_eq!(setup.listeners.active_listeners(), 0);
    assert!(setup.captures.start().is_ok());
    assert!(setup.worlds[0].calls().is_empty());
    assert!(setup.session.toggles.lock().unwrap().is_empty());
}

#[tokio::test]
async fn destruction_ack_requires_exact_world_binding_and_stops_new_input() {
    // Arrange
    let setup = Setup::new(1);
    let mut movement = setup.start(Box::pin(async {
        panic!("no movement after world destruction")
    }));
    let wrong: Arc<dyn CdpClient> = Arc::new(FakeCdpClient::from_html(""));
    let exact: Arc<dyn CdpClient> = setup.worlds[0].clone();
    // Act
    let rejected = movement.acknowledge_destroyed_world(&wrong, "world");
    movement
        .acknowledge_destroyed_world(&exact, "world")
        .unwrap();
    // Assert
    assert!(rejected.is_err());
    assert!(movement.result().await.is_err());
    movement.retire().await.unwrap();
    assert!(movement.is_retired());
    assert!(setup.worlds[0].calls().is_empty());
}
