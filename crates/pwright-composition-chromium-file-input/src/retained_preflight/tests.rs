use super::*;
use pwright_backend_cdp_file_input::RetainedPreflightProfile;
use pwright_composition_chromium_locator_resolution::{
    CdpLocatorResolutionService, ContinuableResolutionService, ResolutionProfile,
    ResolutionRequest, RetainedElementRequest,
};
use pwright_core::{AtomicObjectIdSource, ResourceLimits, Service};
use pwright_fake::FakeCdpClient;
use pwright_runtime::{CleanupQueue, CleanupQueueReservation};
use pwright_selector_model::Selector;
use serde_json::json;
use std::{sync::Weak, task::Waker};
use tokio::sync::Notify;

type Element = OwnedElement<CleanupQueueReservation>;

fn production(requirements: ResourceRequirements) -> ProductionResourceLedger {
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let bound = requirements.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    ProductionResourceLedger::new(limits, requirements).unwrap()
}

fn flags() -> FileInputPreflightFlags {
    FileInputPreflightFlags {
        multiple: false,
        directory_upload: false,
    }
}

struct Fixture {
    service: FileInputPreflightOperationService<CleanupQueue>,
    queue: CleanupQueue,
    resolver: ContinuableResolutionService<CdpLocatorResolutionService, CleanupQueue>,
    raw_resolver: Arc<CdpLocatorResolutionService>,
    client: Arc<FakeCdpClient>,
    native: ProductionResourceLedger,
    backend: ProductionResourceLedger,
    owners: ProductionResourceLedger,
}

impl Fixture {
    fn new(queue_capacity: usize, capacity: usize, owner_bytes: usize) -> Self {
        let queue = CleanupQueue::new(queue_capacity);
        let profile = ResolutionProfile::new(ResourceScope::extension(178, 11), 8)
            .unwrap()
            .with_retained_reference()
            .unwrap();
        let native = production(profile.requirements());
        let raw_resolver = Arc::new(
            CdpLocatorResolutionService::new(
                profile,
                &native,
                Arc::new(AtomicObjectIdSource::new(1).unwrap()),
            )
            .unwrap(),
        );
        let resolver = ContinuableResolutionService::new(raw_resolver.clone(), queue.clone());
        let profile =
            RetainedPreflightProfile::new(ResourceScope::extension(178, 12), capacity).unwrap();
        let backend = production(profile.requirements());
        let raw = Arc::new(CdpFileInputPreflightService::new(profile, &backend).unwrap());
        let profile =
            UploadProfile::new(ResourceScope::extension(178, 13), capacity, owner_bytes).unwrap();
        let owners = production(profile.requirements());
        let service =
            FileInputPreflightOperationService::new(profile, &owners, raw, queue.clone()).unwrap();
        let client = Arc::new(FakeCdpClient::from_html("<input type=file>"));
        client.set_object_backend_node("borrowed-source", 17);
        client.set_adopted_node_response(
            17,
            23,
            json!({"object":{"type":"object","subtype":"node","objectId":"adopted-original"}}),
        );
        Self {
            service,
            queue,
            resolver,
            raw_resolver,
            client,
            native,
            backend,
            owners,
        }
    }

    async fn element(&self) -> (Element, Weak<u32>) {
        self.client
            .set_evaluate_response(json!({"result":{"type":"boolean","value":true}}));
        let route = Arc::new(23_u32);
        let weak = Arc::downgrade(&route);
        let element = self
            .resolver
            .call_retained_retaining(
                RetainedElementRequest {
                    source_client: self.client.clone(),
                    source_object_id: "borrowed-source".into(),
                    client: self.client.clone(),
                    utility_world: "utility-23".into(),
                    utility_execution_context_id: 23,
                    destruction: None,
                },
                Some(route),
            )
            .await
            .unwrap();
        self.client.set_evaluate_response(json!({
            "result":{"type":"object","subtype":"node","objectId":"retargeted-input"}
        }));
        (element, weak)
    }

    fn queries(&self) -> usize {
        self.client
            .calls()
            .iter()
            .filter(|(method, args)| {
                method == "Runtime.callFunctionOn"
                    && args[0]["returnByValue"] == false
                    && args[0]["awaitPromise"] == false
                    && args[0]["arguments"].is_array()
            })
            .count()
    }

    fn releases(&self) -> usize {
        self.client
            .method_names()
            .iter()
            .filter(|name| *name == "Runtime.releaseObjectGroup")
            .count()
    }

    fn assert_derived_admission(&self) {
        assert_eq!(
            self.backend
                .metrics()
                .unwrap()
                .class(ResourceClass::RemoteHandle)
                .current,
            2
        );
        assert!(
            self.backend
                .metrics()
                .unwrap()
                .class(ResourceClass::BackendStateBytes)
                .current
                > 0
        );
        assert_eq!(self.service.active_operations(), 1);
        assert_eq!(
            self.owners
                .metrics()
                .unwrap()
                .class(ResourceClass::LiveObject)
                .current,
            1
        );
    }

    // These are local lease/counter observations, not proof that a refused or
    // forcibly abandoned native cleanup actually released remote objects.
    fn assert_local_owners_drained(&self) {
        assert_eq!(self.native.metrics().unwrap().active_leases, 0);
        assert_eq!(self.backend.metrics().unwrap().active_leases, 0);
        assert_eq!(
            self.owners
                .metrics()
                .unwrap()
                .class(ResourceClass::LiveObject)
                .current,
            0
        );
        assert_eq!(self.service.active_operations(), 0);
        assert_eq!(self.queue.metrics().unwrap().reserved, 0);
        assert_eq!(self.queue.metrics().unwrap().queued, 0);
        assert!(
            !self.client.method_names().iter().any(
                |method| method == "Runtime.releaseObject" || method == "DOM.setFileInputFiles"
            )
        );
        for (method, args) in self.client.calls() {
            if method == "Runtime.callFunctionOn"
                && args[0]["awaitPromise"] == false
                && args[0]["arguments"].is_array()
            {
                assert_eq!(args[0]["returnByValue"], false);
                assert_eq!(args[0]["arguments"].as_array().unwrap().len(), 2);
                assert_eq!(
                    args[0]["arguments"][1],
                    json!({"value":{"multiple":false,"directoryUpload":false}})
                );
                let script = args[0]["functionDeclaration"].as_str().unwrap();
                assert!(!script.contains("DataTransfer"));
                assert!(!script.contains("atob("));
                assert!(!script.contains("dispatchEvent"));
            }
        }
    }
}

fn poll_once<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
    Pin::new(future).poll(&mut Context::from_waker(Waker::noop()))
}

#[tokio::test]
async fn successful_query_transfers_exact_target_route_and_admission_to_ready_owner() {
    // Arrange
    let fixture = Fixture::new(8, 1, 8192);
    let (element, route) = fixture.element().await;
    let group = element.element().object_group().to_owned();
    let mut future = fixture.service.prepare(element, flags()).unwrap();

    // Act
    let ready = (&mut future).await.unwrap();

    // Assert: retaining the completed waiter must not retain another reservation.
    assert_eq!(ready.object_id(), "retargeted-input");
    assert_eq!(ready.element().object_id(), "adopted-original");
    assert_eq!(ready.element().object_group(), group);
    assert!(route.upgrade().is_some());
    assert_eq!(fixture.queries(), 1);
    assert_eq!(fixture.releases(), 0);
    assert_eq!(fixture.queue.metrics().unwrap().reserved, 2);
    fixture.assert_derived_admission();
    ready.release().await.unwrap();
    assert!(route.upgrade().is_none());
    fixture.assert_local_owners_drained();
    assert!(fixture.service.is_cleanup_healthy());
}

#[tokio::test]
async fn unpolled_prevalidation_drops_unused_derived_admission_without_querying() {
    // Arrange
    let fixture = Fixture::new(8, 1, 8192);
    let (element, route) = fixture.element().await;
    let release_gate = Arc::new(Notify::new());
    fixture
        .client
        .set_release_object_group_gate(Some(release_gate.clone()));
    let future = fixture.service.prepare(element, flags()).unwrap();

    // Act
    drop(future);

    // Assert: only the previously acquired original group needs compensation.
    assert_eq!(fixture.queries(), 0);
    assert_eq!(fixture.backend.metrics().unwrap().active_leases, 0);
    assert_eq!(fixture.service.active_operations(), 0);
    assert!(route.upgrade().is_some());
    let mut cleanup = fixture.queue.try_take().unwrap().unwrap();
    assert!(poll_once(&mut cleanup).is_pending());
    assert!(route.upgrade().is_some());
    release_gate.notify_one();
    cleanup.await.unwrap();
    assert!(route.upgrade().is_none());
    fixture.assert_local_owners_drained();
    assert!(fixture.service.is_cleanup_healthy());
}

#[tokio::test]
async fn cancelled_query_retains_extra_native_units_and_route_through_late_group_ack() {
    // Arrange
    let fixture = Fixture::new(8, 1, 8192);
    let (element, route) = fixture.element().await;
    let query_gate = Arc::new(Notify::new());
    let release_gate = Arc::new(Notify::new());
    fixture.client.set_evaluate_gate(Some(query_gate.clone()));
    fixture
        .client
        .set_release_object_group_gate(Some(release_gate.clone()));
    let mut future = fixture.service.prepare(element, flags()).unwrap();
    assert!(poll_once(&mut future).is_pending());

    // Act
    drop(future);

    // Assert: cleanup must await the existing query, not release its group early.
    let mut cleanup = fixture.queue.try_take().unwrap().unwrap();
    assert!(poll_once(&mut cleanup).is_pending());
    assert_eq!(fixture.queries(), 1);
    assert_eq!(fixture.releases(), 0);
    fixture.assert_derived_admission();
    assert!(route.upgrade().is_some());
    query_gate.notify_one();
    assert!(poll_once(&mut cleanup).is_pending());
    assert_eq!(fixture.queries(), 1);
    assert_eq!(fixture.releases(), 1);
    fixture.assert_derived_admission();
    assert!(route.upgrade().is_some());
    release_gate.notify_one();
    cleanup.await.unwrap();
    assert!(route.upgrade().is_none());
    fixture.assert_local_owners_drained();
}

#[tokio::test]
async fn cancelled_late_result_and_exception_are_compensated_by_the_same_group() {
    // Arrange
    let fixture = Fixture::new(8, 1, 8192);
    let (element, route) = fixture.element().await;
    let group = element.element().object_group().to_owned();
    fixture.client.set_evaluate_response(json!({
        "result":{"type":"object","subtype":"node","objectId":"late-result"},
        "exceptionDetails":{"exception":{"type":"object","objectId":"late-exception","description":"failure"}}
    }));
    let query_gate = Arc::new(Notify::new());
    let release_gate = Arc::new(Notify::new());
    fixture.client.set_evaluate_gate(Some(query_gate.clone()));
    fixture
        .client
        .set_release_object_group_gate(Some(release_gate.clone()));
    let mut future = fixture.service.prepare(element, flags()).unwrap();
    assert!(poll_once(&mut future).is_pending());

    // Act
    drop(future);

    // Assert: both wrappers stay covered even though no successful owner appears.
    let mut cleanup = fixture.queue.try_take().unwrap().unwrap();
    query_gate.notify_one();
    assert!(poll_once(&mut cleanup).is_pending());
    assert_eq!(fixture.client.calls().last().unwrap().1, [json!(group)]);
    fixture.assert_derived_admission();
    assert!(route.upgrade().is_some());
    release_gate.notify_one();
    cleanup.await.unwrap();
    assert_eq!(fixture.releases(), 1);
    assert!(route.upgrade().is_none());
    assert!(fixture.service.is_cleanup_healthy());
    fixture.assert_local_owners_drained();
}

#[tokio::test]
async fn ready_owner_drop_keeps_derived_admission_until_group_release_acknowledges() {
    // Arrange
    let fixture = Fixture::new(8, 1, 8192);
    let (element, route) = fixture.element().await;
    let ready = fixture
        .service
        .prepare(element, flags())
        .unwrap()
        .await
        .unwrap();
    let release_gate = Arc::new(Notify::new());
    fixture
        .client
        .set_release_object_group_gate(Some(release_gate.clone()));

    // Act
    drop(ready);

    // Assert: queued cleanup, not Rust field order, retains the native allowance.
    fixture.assert_derived_admission();
    assert!(route.upgrade().is_some());
    let mut cleanup = fixture.queue.try_take().unwrap().unwrap();
    assert!(poll_once(&mut cleanup).is_pending());
    assert_eq!(fixture.releases(), 1);
    fixture.assert_derived_admission();
    release_gate.notify_one();
    cleanup.await.unwrap();
    assert!(route.upgrade().is_none());
    fixture.assert_local_owners_drained();
}

#[tokio::test]
async fn dropping_unpolled_explicit_release_transfers_the_entire_ready_owner() {
    // Arrange
    let fixture = Fixture::new(8, 1, 8192);
    let (element, route) = fixture.element().await;
    let ready = fixture
        .service
        .prepare(element, flags())
        .unwrap()
        .await
        .unwrap();
    let release = ready.release();

    // Act
    drop(release);

    // Assert
    assert_eq!(fixture.releases(), 0);
    fixture.assert_derived_admission();
    assert!(route.upgrade().is_some());
    fixture.queue.try_take().unwrap().unwrap().await.unwrap();
    assert_eq!(fixture.releases(), 1);
    assert!(route.upgrade().is_none());
    fixture.assert_local_owners_drained();
}

#[tokio::test]
async fn dropping_started_explicit_release_does_not_reissue_the_group_command() {
    // Arrange
    let fixture = Fixture::new(8, 1, 8192);
    let (element, route) = fixture.element().await;
    let ready = fixture
        .service
        .prepare(element, flags())
        .unwrap()
        .await
        .unwrap();
    let gate = Arc::new(Notify::new());
    fixture
        .client
        .set_release_object_group_gate(Some(gate.clone()));
    let mut release = ready.release();
    assert!(poll_once(&mut release).is_pending());

    // Act
    drop(release);

    // Assert
    let mut cleanup = fixture.queue.try_take().unwrap().unwrap();
    assert!(poll_once(&mut cleanup).is_pending());
    assert_eq!(fixture.releases(), 1);
    fixture.assert_derived_admission();
    assert!(route.upgrade().is_some());
    gate.notify_one();
    cleanup.await.unwrap();
    assert_eq!(fixture.releases(), 1);
    assert!(route.upgrade().is_none());
    fixture.assert_local_owners_drained();
}

#[tokio::test]
async fn failed_group_cleanup_preserves_primary_error_and_closes_further_admission() {
    // Arrange
    let fixture = Fixture::new(8, 2, 8192);
    let (element, route) = fixture.element().await;
    let (later, later_route) = fixture.element().await;
    fixture.client.set_evaluate_response(json!({
        "exceptionDetails":{"exception":{"type":"object","objectId":"exception","description":"primary"}}
    }));
    fixture
        .client
        .set_release_object_error(Some("group refused".into()));
    let future = fixture.service.prepare(element, flags()).unwrap();

    // Act
    let result = future.await;

    // Assert
    assert!(
        matches!(result, Err(PrevalidationError::Cleanup { primary: Some(ref primary), .. })
        if matches!(primary.as_ref(), PrevalidationError::Backend(RetainedPreflightError::JavaScript(message)) if message.as_ref() == "primary"))
    );
    assert!(!fixture.service.is_cleanup_healthy());
    assert!(!fixture.raw_resolver.is_cleanup_healthy());
    assert!(route.upgrade().is_none());
    let rejected = fixture.service.prepare(later, flags());
    assert!(matches!(
        rejected,
        Err(PrevalidationError::Owner(UploadError::Unhealthy))
    ));
    assert_eq!(fixture.queries(), 1);
    fixture.client.set_release_object_error(None);
    fixture.queue.try_take().unwrap().unwrap().await.unwrap();
    assert!(later_route.upgrade().is_none());
    fixture.assert_local_owners_drained();
}

#[tokio::test]
async fn abandoned_query_reports_failed_cleanup_to_the_executor() {
    // Arrange
    let fixture = Fixture::new(8, 1, 8192);
    let (element, route) = fixture.element().await;
    let query_gate = Arc::new(Notify::new());
    fixture.client.set_evaluate_gate(Some(query_gate.clone()));
    fixture
        .client
        .set_release_object_error(Some("refused".repeat(4096)));
    let mut future = fixture.service.prepare(element, flags()).unwrap();
    assert!(poll_once(&mut future).is_pending());

    // Act
    drop(future);

    // Assert
    query_gate.notify_one();
    let error = fixture
        .queue
        .try_take()
        .unwrap()
        .unwrap()
        .await
        .unwrap_err();
    assert_eq!(
        error.message(),
        "retained file-input prevalidation cleanup failed; admission closed"
    );
    assert_eq!(fixture.queue.metrics().unwrap().failed, 1);
    assert!(!fixture.service.is_cleanup_healthy());
    assert!(!fixture.raw_resolver.is_cleanup_healthy());
    assert!(route.upgrade().is_none());
    fixture.assert_local_owners_drained();
}

#[tokio::test]
async fn missing_extra_cleanup_slot_rejects_before_native_prevalidation() {
    // Arrange: the original OwnedElement already occupies the only slot.
    let fixture = Fixture::new(1, 1, 8192);
    let (element, route) = fixture.element().await;

    // Act
    let result = fixture.service.prepare(element, flags());

    // Assert
    assert!(matches!(
        result,
        Err(PrevalidationError::Owner(UploadError::Admission(_)))
    ));
    assert_eq!(fixture.queries(), 0);
    assert_eq!(fixture.backend.metrics().unwrap().active_leases, 0);
    assert_eq!(fixture.service.active_operations(), 0);
    assert!(route.upgrade().is_some());
    fixture.queue.try_take().unwrap().unwrap().await.unwrap();
    assert!(route.upgrade().is_none());
    fixture.assert_local_owners_drained();
    assert!(fixture.service.is_cleanup_healthy());
}

#[tokio::test]
async fn parked_ready_owner_keeps_capacity_occupied_until_released() {
    // Arrange
    let fixture = Fixture::new(8, 1, 8192);
    let (first, first_route) = fixture.element().await;
    let ready = fixture
        .service
        .prepare(first, flags())
        .unwrap()
        .await
        .unwrap();
    let (second, second_route) = fixture.element().await;

    // Act
    let result = fixture.service.prepare(second, flags());

    // Assert
    assert!(matches!(
        result,
        Err(PrevalidationError::Owner(UploadError::Capacity))
    ));
    assert_eq!(fixture.queries(), 1);
    fixture.assert_derived_admission();
    fixture.queue.try_take().unwrap().unwrap().await.unwrap();
    assert!(second_route.upgrade().is_none());
    assert!(first_route.upgrade().is_some());
    ready.release().await.unwrap();
    assert!(first_route.upgrade().is_none());
    fixture.assert_local_owners_drained();
}

#[tokio::test]
async fn insufficient_owner_storage_rejects_before_native_prevalidation() {
    // Arrange
    let fixture = Fixture::new(8, 1, 1);
    let (element, route) = fixture.element().await;

    // Act
    let result = fixture.service.prepare(element, flags());

    // Assert
    assert!(matches!(
        result,
        Err(PrevalidationError::Owner(UploadError::Storage))
    ));
    assert_eq!(fixture.queries(), 0);
    assert_eq!(fixture.backend.metrics().unwrap().active_leases, 0);
    assert_eq!(fixture.service.active_operations(), 0);
    fixture.queue.try_take().unwrap().unwrap().await.unwrap();
    assert!(route.upgrade().is_none());
    fixture.assert_local_owners_drained();
}

#[tokio::test]
async fn ordinary_selector_group_is_rejected_before_derived_native_acquisition() {
    // Arrange: ordinary resolution legitimately records historical SelectorBytes.
    let fixture = Fixture::new(8, 1, 8192);
    fixture.client.set_evaluate_response(json!({
        "result":{"type":"object","subtype":"node","objectId":"ordinary-input"}
    }));
    let element = fixture
        .resolver
        .call(ResolutionRequest {
            client: fixture.client.clone(),
            utility_world: "utility-23".into(),
            selector: Selector::css("input").unwrap(),
            destruction: None,
        })
        .await
        .unwrap()
        .unwrap();
    assert!(!element.element().uses_settlement_release());
    let before = fixture.queries();

    // Act
    let result = fixture.service.prepare(element, flags());

    // Assert: current owners drain; historical selector allocation is permitted.
    assert!(matches!(result, Err(PrevalidationError::OrdinaryGroup)));
    assert_eq!(fixture.queries(), before);
    assert_eq!(fixture.backend.metrics().unwrap().active_leases, 0);
    fixture.queue.try_take().unwrap().unwrap().await.unwrap();
    fixture.assert_local_owners_drained();
}

#[tokio::test]
async fn discarded_executor_continuation_poison_is_not_reported_as_native_cleanup_success() {
    // Arrange
    let fixture = Fixture::new(8, 1, 8192);
    let (element, route) = fixture.element().await;
    fixture
        .client
        .set_evaluate_gate(Some(Arc::new(Notify::new())));
    let mut future = fixture.service.prepare(element, flags()).unwrap();
    assert!(poll_once(&mut future).is_pending());
    drop(future);
    let cleanup = fixture.queue.try_take().unwrap().unwrap();

    // Act: deliberately violate the admitted executor's drain contract.
    drop(cleanup);

    // Assert: no impossible guarantee is made about the abandoned raw query.
    assert!(!fixture.service.is_cleanup_healthy());
    assert_eq!(fixture.queue.metrics().unwrap().completed, 0);
    assert_eq!(fixture.queries(), 1);
    assert!(route.upgrade().is_some());
    // OwnedElement independently schedules best-effort original-group cleanup.
    fixture.queue.try_take().unwrap().unwrap().await.unwrap();
    assert!(route.upgrade().is_none());
    fixture.assert_local_owners_drained();
    assert!(!fixture.service.is_cleanup_healthy());
}

#[tokio::test]
async fn dropping_service_does_not_drop_ready_owner_storage_or_cleanup_authority() {
    // Arrange
    let fixture = Fixture::new(8, 1, 8192);
    let (element, route) = fixture.element().await;
    let ready = fixture
        .service
        .prepare(element, flags())
        .unwrap()
        .await
        .unwrap();
    let native = fixture.native.clone();
    let backend = fixture.backend.clone();
    let owners = fixture.owners.clone();

    // Act
    drop(fixture);

    // Assert: the ready receipt owns its route, group and all selected backing.
    assert!(route.upgrade().is_some());
    assert!(native.metrics().unwrap().active_leases > 0);
    assert_eq!(
        backend
            .metrics()
            .unwrap()
            .class(ResourceClass::RemoteHandle)
            .current,
        2
    );
    assert_eq!(
        owners
            .metrics()
            .unwrap()
            .class(ResourceClass::LiveObject)
            .current,
        1
    );
    ready.release().await.unwrap();
    assert!(route.upgrade().is_none());
    assert_eq!(native.metrics().unwrap().active_leases, 0);
    assert_eq!(backend.metrics().unwrap().active_leases, 0);
    assert_eq!(owners.metrics().unwrap().active_leases, 0);
}
