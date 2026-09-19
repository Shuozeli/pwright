use super::*;
use pwright_composition_chromium_locator_resolution::{
    CdpLocatorResolutionService, ContinuableResolutionService, ResolutionProfile,
    RetainedElementRequest,
};
use pwright_core::{AtomicObjectIdSource, ResourceLimits};
use pwright_fake::FakeCdpClient;
use pwright_runtime::{CleanupQueue, CleanupQueueReservation};
use serde_json::json;
use std::{sync::Weak, task::Waker};
use tokio::sync::Notify;

type Element = OwnedElement<CleanupQueueReservation>;
type Marker = OwnedTargetMarker<CleanupQueueReservation>;

fn production(requirements: ResourceRequirements) -> ProductionResourceLedger {
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let bound = requirements.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    ProductionResourceLedger::new(limits, requirements).unwrap()
}

struct Fixture {
    service: TargetMarkingOperationService<CleanupQueue>,
    resolver: ContinuableResolutionService<CdpLocatorResolutionService, CleanupQueue>,
    queue: CleanupQueue,
    client: Arc<FakeCdpClient>,
    native: ProductionResourceLedger,
    backend: ProductionResourceLedger,
    owners: ProductionResourceLedger,
}
impl Fixture {
    fn new(queue_capacity: usize, state_bytes: usize, call_bytes: usize) -> Self {
        Self::with_capacity(queue_capacity, state_bytes, call_bytes, 1, 2)
    }

    fn with_capacity(
        queue_capacity: usize,
        state_bytes: usize,
        call_bytes: usize,
        state_capacity: usize,
        call_capacity: usize,
    ) -> Self {
        let queue = CleanupQueue::new(queue_capacity);
        let profile = ResolutionProfile::new(ResourceScope::extension(182, 31), 8)
            .unwrap()
            .with_retained_reference()
            .unwrap();
        let native = production(profile.requirements());
        let resolver = ContinuableResolutionService::new(
            Arc::new(
                CdpLocatorResolutionService::new(
                    profile,
                    &native,
                    Arc::new(AtomicObjectIdSource::new(1).unwrap()),
                )
                .unwrap(),
            ),
            queue.clone(),
        );
        let profile = TargetMarkingProfile::new(
            ResourceScope::extension(182, 32),
            state_capacity,
            call_capacity,
        )
        .unwrap();
        let backend = production(profile.requirements());
        let raw = Arc::new(
            CdpTargetMarkingService::new(
                profile,
                &backend,
                Arc::new(AtomicObjectIdSource::new(1).unwrap()),
            )
            .unwrap(),
        );
        let states = UploadProfile::new(
            ResourceScope::extension(182, 33),
            state_capacity,
            state_bytes,
        )
        .unwrap();
        let calls =
            UploadProfile::new(ResourceScope::extension(182, 34), call_capacity, call_bytes)
                .unwrap();
        let owners = production(
            states
                .requirements()
                .combine_concurrent(calls.requirements()),
        );
        let service =
            TargetMarkingOperationService::new(states, calls, &owners, raw, queue.clone()).unwrap();
        let client = Arc::new(FakeCdpClient::from_html("<input type=file>"));
        client.set_object_backend_node("persistent-source", 17);
        client.set_adopted_node_response(
            17,
            23,
            json!({"object":{"type":"object","subtype":"node","objectId":"adopted"}}),
        );
        Self {
            service,
            resolver,
            queue,
            client,
            native,
            backend,
            owners,
        }
    }

    fn world(&self) -> MarkerWorld {
        MarkerWorld {
            client: self.client.clone(),
            utility_world: "utility-23".into(),
            destruction: None,
        }
    }

    async fn marker(&self) -> (Marker, Weak<u8>) {
        self.client.set_evaluate_response(json!({
            "result":{"type":"object","objectId":format!("private-marker-{}", self.service.active_states())}
        }));
        let route = Arc::new(23_u8);
        let weak = Arc::downgrade(&route);
        let marker = self
            .service
            .create(self.world(), Some(route))
            .unwrap()
            .await
            .unwrap();
        (marker, weak)
    }

    async fn element(&self, world: &str) -> Element {
        self.client
            .set_evaluate_response(json!({"result":{"type":"boolean","value":true}}));
        let element = self
            .resolver
            .call_retained_retaining(
                RetainedElementRequest {
                    source_client: self.client.clone(),
                    source_object_id: "persistent-source".into(),
                    client: self.client.clone(),
                    utility_world: world.into(),
                    utility_execution_context_id: 23,
                    destruction: None,
                },
                None,
            )
            .await
            .unwrap();
        self.client
            .set_evaluate_response(json!({"result":{"type":"undefined"}}));
        element
    }

    fn releases(&self) -> Vec<String> {
        self.client
            .calls()
            .into_iter()
            .filter_map(|(method, args)| {
                (method == "Runtime.releaseObjectGroup")
                    .then(|| args[0].as_str().unwrap().to_owned())
            })
            .collect()
    }

    fn assert_native_units(&self, units: u64) {
        assert_eq!(
            self.backend
                .metrics()
                .unwrap()
                .class(ResourceClass::RemoteHandle)
                .current,
            units
        );
    }

    fn assert_drained(&self) {
        assert_eq!(self.service.active_states(), 0);
        assert_eq!(self.service.active_calls(), 0);
        self.assert_native_units(0);
        assert_eq!(self.native.metrics().unwrap().active_leases, 0);
        assert_eq!(
            self.owners
                .metrics()
                .unwrap()
                .class(ResourceClass::LiveObject)
                .current,
            0
        );
        assert_eq!(self.queue.metrics().unwrap().reserved, 0);
        assert_eq!(self.queue.metrics().unwrap().queued, 0);
        assert!(
            !self
                .client
                .method_names()
                .iter()
                .any(|method| method == "Runtime.releaseObject")
        );
    }
}

fn poll_once<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
    Pin::new(future).poll(&mut Context::from_waker(Waker::noop()))
}

#[tokio::test]
async fn same_world_state_survives_multiple_target_group_releases() {
    // Arrange
    let fixture = Fixture::new(8, 16384, 16384);
    let (marker, route) = fixture.marker().await;
    let first = fixture.element("utility-23").await;
    let first_group = first.element().object_group().to_owned();

    // Act
    let marked = fixture
        .service
        .mark(&marker, first, "call@1".into())
        .unwrap()
        .await
        .unwrap();

    // Assert
    assert_eq!(marked.element().object_id(), "adopted");
    fixture.assert_native_units(4);
    marked.release().await.unwrap();
    assert_eq!(fixture.releases(), [first_group]);
    fixture.assert_native_units(2);
    assert!(route.upgrade().is_some());
    let second = fixture.element("utility-23").await;
    let second_group = second.element().object_group().to_owned();
    fixture
        .service
        .mark(&marker, second, "call@2".into())
        .unwrap()
        .await
        .unwrap()
        .release()
        .await
        .unwrap();
    assert_eq!(fixture.releases()[1], second_group);
    assert_eq!(fixture.service.active_states(), 1);
    marker
        .release()
        .ok()
        .expect("no active mark")
        .await
        .unwrap();
    assert_eq!(fixture.releases().len(), 3);
    assert!(route.upgrade().is_none());
    fixture.assert_drained();
    assert!(fixture.service.is_cleanup_healthy());
}

#[test]
fn unpolled_creation_has_no_io_and_returns_all_dynamic_admission() {
    // Arrange
    let fixture = Fixture::new(8, 16384, 16384);
    let future = fixture.service.create(fixture.world(), None).unwrap();

    // Act
    drop(future);

    // Assert
    assert!(fixture.client.calls().is_empty());
    fixture.assert_drained();
    assert!(fixture.service.is_cleanup_healthy());
}

#[tokio::test]
async fn canceled_creation_waits_for_late_result_then_group_ack() {
    // Arrange
    let fixture = Fixture::new(8, 16384, 16384);
    fixture
        .client
        .set_evaluate_response(json!({"result":{"type":"object","objectId":"private-marker"}}));
    let query = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    fixture.client.set_evaluate_gate(Some(query.clone()));
    fixture
        .client
        .set_release_object_group_gate(Some(release.clone()));
    let route = Arc::new(23_u8);
    let weak = Arc::downgrade(&route);
    let mut future = fixture
        .service
        .create(fixture.world(), Some(route))
        .unwrap();
    assert!(poll_once(&mut future).is_pending());

    // Act
    drop(future);

    // Assert
    let mut cleanup = fixture.queue.try_take().unwrap().unwrap();
    assert!(poll_once(&mut cleanup).is_pending());
    assert!(fixture.releases().is_empty());
    fixture.assert_native_units(2);
    assert!(weak.upgrade().is_some());
    query.notify_one();
    assert!(poll_once(&mut cleanup).is_pending());
    assert_eq!(fixture.releases().len(), 1);
    fixture.assert_native_units(2);
    release.notify_one();
    cleanup.await.unwrap();
    assert!(weak.upgrade().is_none());
    fixture.assert_drained();
}

#[tokio::test]
async fn canceled_mark_keeps_persistent_state_but_cleans_target_after_settlement() {
    // Arrange
    let fixture = Fixture::new(8, 16384, 16384);
    let (marker, route) = fixture.marker().await;
    let element = fixture.element("utility-23").await;
    let group = element.element().object_group().to_owned();
    let query = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    fixture.client.set_evaluate_gate(Some(query.clone()));
    fixture
        .client
        .set_release_object_group_gate(Some(release.clone()));
    let mut future = fixture
        .service
        .mark(&marker, element, "call@1".into())
        .unwrap();
    assert!(poll_once(&mut future).is_pending());

    // Act
    drop(future);

    // Assert
    let mut cleanup = fixture.queue.try_take().unwrap().unwrap();
    assert!(poll_once(&mut cleanup).is_pending());
    fixture.assert_native_units(4);
    assert!(fixture.releases().is_empty());
    query.notify_one();
    assert!(poll_once(&mut cleanup).is_pending());
    assert_eq!(fixture.releases(), [group]);
    fixture.assert_native_units(4);
    release.notify_one();
    cleanup.await.unwrap();
    fixture.assert_native_units(2);
    assert!(route.upgrade().is_some());
    assert_eq!(fixture.service.active_calls(), 0);
    fixture.client.set_release_object_group_gate(None);
    marker.release().ok().expect("mark settled").await.unwrap();
    fixture.assert_drained();
    assert!(fixture.service.is_cleanup_healthy());
}

#[tokio::test]
async fn dropping_marker_during_mark_defers_private_group_release() {
    // Arrange
    let fixture = Fixture::new(8, 16384, 16384);
    let (marker, route) = fixture.marker().await;
    let element = fixture.element("utility-23").await;
    let query = Arc::new(Notify::new());
    fixture.client.set_evaluate_gate(Some(query.clone()));
    let mut future = fixture
        .service
        .mark(&marker, element, "call@1".into())
        .unwrap();
    assert!(poll_once(&mut future).is_pending());
    let marker = marker.release().err().expect("running mark retains state");

    // Act
    drop(marker);
    drop(future);

    // Assert
    assert!(fixture.releases().is_empty());
    assert!(route.upgrade().is_some());
    let cleanup = fixture.queue.try_take().unwrap().unwrap();
    query.notify_one();
    cleanup.await.unwrap();
    // Final state cleanup is independently pre-admitted, not another new slot.
    fixture.queue.try_take().unwrap().unwrap().await.unwrap();
    assert_eq!(fixture.releases().len(), 2);
    assert!(route.upgrade().is_none());
    fixture.assert_drained();
}

#[tokio::test]
async fn dropped_ready_target_keeps_extra_native_units_until_release_ack() {
    // Arrange
    let fixture = Fixture::new(8, 16384, 16384);
    let (marker, _) = fixture.marker().await;
    let element = fixture.element("utility-23").await;
    let marked = fixture
        .service
        .mark(&marker, element, "call@1".into())
        .unwrap()
        .await
        .unwrap();
    let release = Arc::new(Notify::new());
    fixture
        .client
        .set_release_object_group_gate(Some(release.clone()));

    // Act
    drop(marked);

    // Assert
    let mut cleanup = fixture.queue.try_take().unwrap().unwrap();
    assert!(poll_once(&mut cleanup).is_pending());
    fixture.assert_native_units(4);
    assert_eq!(fixture.service.active_calls(), 1);
    release.notify_one();
    cleanup.await.unwrap();
    fixture.assert_native_units(2);
    fixture.client.set_release_object_group_gate(None);
    marker.release().ok().expect("mark settled").await.unwrap();
    fixture.assert_drained();
}

#[tokio::test]
async fn unpolled_explicit_marker_release_keeps_route_and_native_lease() {
    // Arrange
    let fixture = Fixture::new(8, 16384, 16384);
    let (marker, route) = fixture.marker().await;
    let release = marker.release().ok().expect("idle marker");

    // Act
    drop(release);

    // Assert
    fixture.assert_native_units(2);
    assert!(route.upgrade().is_some());
    assert!(fixture.releases().is_empty());
    fixture.queue.try_take().unwrap().unwrap().await.unwrap();
    assert!(route.upgrade().is_none());
    fixture.assert_drained();
}

#[tokio::test]
async fn foreign_world_rejects_before_mark_io_and_cleans_only_target() {
    // Arrange
    let fixture = Fixture::new(8, 16384, 16384);
    let (marker, _) = fixture.marker().await;
    let element = fixture.element("another-utility").await;
    let before = fixture.client.calls().len();

    // Act
    let result = fixture
        .service
        .mark(&marker, element, "call@1".into())
        .unwrap()
        .await;

    // Assert
    assert!(matches!(
        result,
        Err(MarkerOperationError::Backend(
            TargetMarkingError::InvalidTarget
        ))
    ));
    assert_eq!(fixture.client.calls().len(), before + 1);
    assert_eq!(fixture.releases().len(), 1);
    fixture.assert_native_units(2);
    marker.release().ok().expect("mark settled").await.unwrap();
    fixture.assert_drained();
}

#[tokio::test]
async fn javascript_failure_releases_target_and_preserves_marker_state() {
    // Arrange
    let fixture = Fixture::new(8, 16384, 16384);
    let (marker, _) = fixture.marker().await;
    let element = fixture.element("utility-23").await;
    fixture.client.set_evaluate_response(json!({
        "result":{"type":"undefined"},
        "exceptionDetails":{"text":"marker event failed","exception":{"type":"object","objectId":"exception","description":"marker event failed"}}
    }));

    // Act
    let result = fixture
        .service
        .mark(&marker, element, "call@1".into())
        .unwrap()
        .await;

    // Assert
    assert!(matches!(
        result,
        Err(MarkerOperationError::Backend(
            TargetMarkingError::JavaScript(_)
        ))
    ));
    assert_eq!(fixture.releases().len(), 1);
    fixture.assert_native_units(2);
    assert!(fixture.service.is_cleanup_healthy());
    marker.release().ok().expect("mark settled").await.unwrap();
    fixture.assert_drained();
}

#[tokio::test]
async fn target_cleanup_failure_is_reported_and_closes_new_admission() {
    // Arrange
    let fixture = Fixture::new(8, 16384, 16384);
    let (marker, _) = fixture.marker().await;
    let element = fixture.element("utility-23").await;
    let marked = fixture
        .service
        .mark(&marker, element, "call@1".into())
        .unwrap()
        .await
        .unwrap();
    fixture
        .client
        .set_release_object_error(Some("release failed".into()));

    // Act
    let result = marked.release().await;

    // Assert
    assert!(matches!(result, Err(MarkerOperationError::Cleanup { .. })));
    assert!(!fixture.service.is_cleanup_healthy());
    assert!(matches!(
        fixture.service.create(fixture.world(), None),
        Err(MarkerOperationError::Owner(UploadError::Unhealthy))
    ));
    fixture.client.set_release_object_error(None);
    marker.release().ok().expect("mark settled").await.unwrap();
    fixture.assert_drained();
    // Local zero leases do not claim the failed target group was reclaimed.
}

#[tokio::test]
async fn canceled_failed_target_cleanup_reaches_executor_health() {
    // Arrange
    let fixture = Fixture::new(8, 16384, 16384);
    let (marker, _) = fixture.marker().await;
    let element = fixture.element("utility-23").await;
    let query = Arc::new(Notify::new());
    fixture.client.set_evaluate_gate(Some(query.clone()));
    fixture
        .client
        .set_release_object_error(Some("release failed".into()));
    let mut future = fixture
        .service
        .mark(&marker, element, "call@1".into())
        .unwrap();
    assert!(poll_once(&mut future).is_pending());

    // Act
    drop(future);

    // Assert
    let cleanup = fixture.queue.try_take().unwrap().unwrap();
    query.notify_one();
    assert!(cleanup.await.is_err());
    assert!(!fixture.service.is_cleanup_healthy());
    fixture.client.set_release_object_error(None);
    marker.release().ok().expect("mark settled").await.unwrap();
    fixture.assert_drained();
}

#[test]
fn exhausted_cleanup_refuses_creation_before_native_io() {
    // Arrange
    let fixture = Fixture::new(1, 16384, 16384);
    let held = fixture.queue.try_reserve().unwrap();

    // Act
    let result = fixture.service.create(fixture.world(), None);

    // Assert
    assert!(matches!(
        result,
        Err(MarkerOperationError::Owner(UploadError::Admission(_)))
    ));
    assert!(fixture.client.calls().is_empty());
    assert_eq!(fixture.service.active_states(), 0);
    fixture.assert_native_units(0);
    drop(held);
    fixture.assert_drained();
}

#[test]
fn insufficient_creation_storage_refuses_before_native_io() {
    // Arrange
    let fixture = Fixture::new(8, 1, 16384);

    // Act
    let result = fixture.service.create(fixture.world(), None);

    // Assert
    assert!(matches!(
        result,
        Err(MarkerOperationError::Owner(UploadError::Storage))
    ));
    assert!(fixture.client.calls().is_empty());
    fixture.assert_drained();
}

#[tokio::test]
async fn insufficient_call_storage_releases_existing_target_without_marking() {
    // Arrange
    let fixture = Fixture::new(8, 16384, 1);
    let (marker, _) = fixture.marker().await;
    let element = fixture.element("utility-23").await;
    let before = fixture.client.calls().len();

    // Act
    let result = fixture.service.mark(&marker, element, "call@1".into());

    // Assert
    assert!(matches!(
        result,
        Err(MarkerOperationError::Owner(UploadError::Storage))
    ));
    assert_eq!(fixture.client.calls().len(), before);
    fixture.queue.try_take().unwrap().unwrap().await.unwrap();
    marker
        .release()
        .ok()
        .expect("no mark started")
        .await
        .unwrap();
    fixture.assert_drained();
    assert!(fixture.service.is_cleanup_healthy());
}

#[tokio::test]
async fn discarded_cleanup_poison_does_not_claim_native_reclamation() {
    // Arrange
    let fixture = Fixture::new(8, 16384, 16384);
    let (marker, _) = fixture.marker().await;
    drop(marker);
    let cleanup = fixture.queue.try_take().unwrap().unwrap();

    // Act
    drop(cleanup);

    // Assert
    assert!(!fixture.service.is_cleanup_healthy());
    assert!(fixture.releases().is_empty());
    fixture.assert_drained();
}

#[tokio::test]
async fn foreign_service_marker_rejects_before_admission_even_on_same_client_and_world() {
    // Arrange: the native client/world match, but the composition authority does not.
    let origin = Fixture::new(8, 16384, 16384);
    let (marker, route) = origin.marker().await;
    let mut receiver = Fixture::new(8, 16384, 16384);
    receiver.client = origin.client.clone();
    let element = receiver.element("utility-23").await;
    let before = origin.client.calls().len();
    let receiver_budget = receiver.backend.metrics().unwrap();

    // Act
    let result = receiver.service.mark(&marker, element, "call@1".into());

    // Assert: only the consumed target's existing reservation owes cleanup.
    assert!(matches!(result, Err(MarkerOperationError::ForeignMarker)));
    assert_eq!(origin.client.calls().len(), before);
    assert_eq!(receiver.service.active_calls(), 0);
    assert_eq!(receiver.backend.metrics().unwrap(), receiver_budget);
    assert_eq!(origin.service.active_states(), 1);
    assert!(route.upgrade().is_some());
    receiver.queue.try_take().unwrap().unwrap().await.unwrap();
    receiver.assert_drained();
    marker
        .release()
        .ok()
        .expect("foreign call never started")
        .await
        .unwrap();
    origin.assert_drained();
}

#[tokio::test]
async fn full_state_pool_refuses_creation_without_consuming_call_capacity() {
    // Arrange
    let fixture = Fixture::new(8, 16384, 16384);
    let (marker, _) = fixture.marker().await;
    let before = fixture.client.calls().len();
    let backend = fixture.backend.metrics().unwrap();
    let owners = fixture.owners.metrics().unwrap();
    let queue = fixture.queue.metrics().unwrap();

    // Act
    let result = fixture.service.create(fixture.world(), None);

    // Assert
    assert!(matches!(
        result,
        Err(MarkerOperationError::Owner(UploadError::Capacity))
    ));
    assert_eq!(fixture.client.calls().len(), before);
    assert_eq!(fixture.service.active_states(), 1);
    assert_eq!(fixture.service.active_calls(), 0);
    assert_eq!(fixture.backend.metrics().unwrap(), backend);
    assert_eq!(fixture.owners.metrics().unwrap(), owners);
    assert_eq!(fixture.queue.metrics().unwrap(), queue);
    marker
        .release()
        .ok()
        .expect("no mark started")
        .await
        .unwrap();
    fixture.assert_drained();
}

#[tokio::test]
async fn full_call_pool_refuses_before_io_and_preserves_persistent_state() {
    // Arrange: unpolled accepted calls still occupy composition capacity.
    let fixture = Fixture::new(8, 16384, 16384);
    let (marker, _) = fixture.marker().await;
    let first = fixture.element("utility-23").await;
    let second = fixture.element("utility-23").await;
    let third = fixture.element("utility-23").await;
    let held_first = fixture
        .service
        .mark(&marker, first, "call@1".into())
        .unwrap();
    let held_second = fixture
        .service
        .mark(&marker, second, "call@2".into())
        .unwrap();
    let before = fixture.client.calls().len();
    let backend = fixture.backend.metrics().unwrap();
    let owners = fixture.owners.metrics().unwrap();

    // Act
    let result = fixture.service.mark(&marker, third, "call@3".into());

    // Assert
    assert!(matches!(
        result,
        Err(MarkerOperationError::Owner(UploadError::Capacity))
    ));
    assert_eq!(fixture.client.calls().len(), before);
    assert_eq!(fixture.service.active_states(), 1);
    assert_eq!(fixture.service.active_calls(), 2);
    assert_eq!(fixture.backend.metrics().unwrap(), backend);
    assert_eq!(fixture.owners.metrics().unwrap(), owners);
    drop(held_first);
    drop(held_second);
    for _ in 0..3 {
        fixture.queue.try_take().unwrap().unwrap().await.unwrap();
    }
    assert_eq!(fixture.service.active_calls(), 0);
    marker
        .release()
        .ok()
        .expect("unpolled calls dropped")
        .await
        .unwrap();
    fixture.assert_drained();
    assert!(fixture.service.is_cleanup_healthy());
}

#[tokio::test]
async fn occupied_call_pool_does_not_consume_the_next_world_slot() {
    // Arrange: successful call ownership, not just a running query, fills the call pool.
    let fixture = Fixture::with_capacity(8, 16384, 16384, 2, 1);
    let (first_marker, _) = fixture.marker().await;
    let element = fixture.element("utility-23").await;
    let marked = fixture
        .service
        .mark(&first_marker, element, "call@1".into())
        .unwrap()
        .await
        .unwrap();
    fixture
        .client
        .set_evaluate_response(json!({"result":{"type":"object","objectId":"second-marker"}}));
    let mut second_world = fixture.world();
    second_world.utility_world = "utility-24".into();

    // Act
    let second_marker = fixture
        .service
        .create(second_world, None)
        .unwrap()
        .await
        .unwrap();

    // Assert
    assert_eq!(fixture.service.active_states(), 2);
    assert_eq!(fixture.service.active_calls(), 1);
    fixture.assert_native_units(6);
    second_marker
        .release()
        .ok()
        .expect("second marker idle")
        .await
        .unwrap();
    marked.release().await.unwrap();
    first_marker
        .release()
        .ok()
        .expect("first marker idle")
        .await
        .unwrap();
    fixture.assert_drained();
}
