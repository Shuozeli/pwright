use super::*;
use pwright_backend_cdp_file_input::PayloadProfile;
use pwright_composition_chromium_locator_resolution::{
    CdpLocatorResolutionService, ContinuableResolutionService, ResolutionProfile, ResolutionRequest,
};
use pwright_core::{AtomicObjectIdSource, ResourceLimits, Service};
use pwright_fake::FakeCdpClient;
use pwright_file_input_model::FileInputLimits;
use pwright_file_input_source::{Content, PayloadSource};
use pwright_runtime::{CleanupQueue, CleanupQueueReservation};
use pwright_selector_model::Selector;
use serde_json::json;
use std::task::Waker;

fn production(requirements: ResourceRequirements) -> ProductionResourceLedger {
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let limit = requirements.class(class);
        limits = limits.with_class(class, limit.global, limit.per_scope);
    }
    ProductionResourceLedger::new(limits, requirements).unwrap()
}
struct Fixture {
    service: FilePayloadOperationService<CleanupQueue>,
    queue: CleanupQueue,
    resolver: ContinuableResolutionService<CdpLocatorResolutionService, CleanupQueue>,
    client: Arc<FakeCdpClient>,
    source: ResourceLedger,
    native: ProductionResourceLedger,
    backend: ProductionResourceLedger,
    owners: ProductionResourceLedger,
}
impl Fixture {
    fn new(queue_capacity: usize, owner_bytes: usize) -> Self {
        let queue = CleanupQueue::new(queue_capacity);
        let profile = ResolutionProfile::new(ResourceScope::extension(146, 1), 2).unwrap();
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
        let profile =
            PayloadProfile::new(ResourceScope::extension(146, 2), 1, 4, 4096, 1024).unwrap();
        let backend = production(profile.requirements().unwrap());
        let inner = Arc::new(CdpFilePayloadService::new(profile, &backend).unwrap());
        let profile = UploadProfile::new(ResourceScope::extension(146, 3), 1, owner_bytes).unwrap();
        let owners = production(profile.requirements());
        let service =
            FilePayloadOperationService::new(profile, &owners, inner, queue.clone()).unwrap();
        Self {
            service,
            queue,
            resolver,
            native,
            backend,
            owners,
            client: Arc::new(FakeCdpClient::from_html("<input type=file>")),
            source: ResourceLedger::new(
                ResourceLimits::deny_all(1, 4)
                    .with_class(ResourceClass::LiveObject, 4, 4)
                    .with_class(ResourceClass::ArtifactBytes, 64, 64)
                    .with_class(ResourceClass::BackendStateBytes, 8192, 8192),
            ),
        }
    }
    async fn element(&self) -> OwnedElement<CleanupQueueReservation> {
        self.client.set_evaluate_response(
            json!({"result":{"type":"object","subtype":"node","objectId":"input-1"}}),
        );
        let element = self
            .resolver
            .call(ResolutionRequest {
                client: self.client.clone(),
                utility_world: "exact-world".into(),
                selector: Selector::css("input").unwrap(),
                destruction: None,
            })
            .await
            .unwrap()
            .unwrap();
        self.client.set_evaluate_response(json!({"result":{"value":{"status":"applied","target":{"input":true,"multiple":false,"directory":false}}}}));
        element
    }
    fn payloads(&self) -> PreparedPayloads {
        PreparedPayloads::materialize(
            &self.source,
            ResourceScope::Connection,
            FileInputLimits::new(4, 512, 128, 128, 512).unwrap(),
            &mut [PayloadSource {
                name: "test.txt",
                mime_type: "text/plain",
                last_modified_ms: None,
                content: Content::Bytes(&[0, 255, 1]),
            }],
        )
        .unwrap()
    }
    fn mutations(&self) -> usize {
        self.client
            .calls()
            .iter()
            .filter(|(method, args)| {
                method == "Runtime.callFunctionOn" && args[0]["returnByValue"] == true
            })
            .count()
    }
    fn releases(&self) -> usize {
        self.client
            .calls()
            .iter()
            .filter(|(method, _)| method == "Runtime.releaseObjectGroup")
            .count()
    }
    fn assert_drained(&self) {
        assert_eq!(self.source.metrics().unwrap().active_leases, 0);
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
    }
}
fn poll_once<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
    Pin::new(future).poll(&mut Context::from_waker(Waker::noop()))
}

#[tokio::test]
async fn success_releases_before_publication_even_if_completed_waiter_is_retained() {
    // Arrange
    let fixture = Fixture::new(4, 4096);
    let element = fixture.element().await;
    let mut future = fixture
        .service
        .prepare(element, fixture.payloads())
        .unwrap();
    // Act
    let result = (&mut future).await;
    // Assert
    assert!(matches!(result, Ok(PayloadOutcome::Applied)));
    assert_eq!(fixture.mutations(), 1);
    assert_eq!(fixture.releases(), 1);
    fixture.assert_drained();
    assert!(fixture.service.is_cleanup_healthy());
}

#[tokio::test]
async fn dropping_an_unpolled_upload_never_dispatches_a_mutation() {
    // Arrange
    let fixture = Fixture::new(4, 4096);
    let element = fixture.element().await;
    let future = fixture
        .service
        .prepare(element, fixture.payloads())
        .unwrap();
    // Act
    drop(future);
    // Assert
    assert_eq!(fixture.mutations(), 0);
    assert_eq!(fixture.source.metrics().unwrap().active_leases, 0);
    assert_eq!(fixture.service.active_operations(), 0);
    fixture.queue.try_take().unwrap().unwrap().await.unwrap();
    assert_eq!(fixture.mutations(), 0);
    fixture.assert_drained();
    assert!(fixture.service.is_cleanup_healthy());
}

#[tokio::test]
async fn dropping_a_started_waiter_retains_native_source_and_admission_until_reply() {
    // Arrange
    let fixture = Fixture::new(4, 4096);
    let element = fixture.element().await;
    let gate = Arc::new(tokio::sync::Notify::new());
    fixture.client.set_evaluate_gate(Some(gate.clone()));
    let mut future = fixture
        .service
        .prepare(element, fixture.payloads())
        .unwrap();
    assert!(poll_once(&mut future).is_pending());
    // Act
    drop(future);
    // Assert
    assert_eq!(fixture.mutations(), 1);
    assert_eq!(fixture.releases(), 0);
    assert_eq!(fixture.service.active_operations(), 1);
    assert_eq!(
        fixture
            .source
            .metrics()
            .unwrap()
            .class(ResourceClass::ArtifactBytes)
            .current,
        3
    );
    assert_eq!(
        fixture
            .backend
            .metrics()
            .unwrap()
            .class(ResourceClass::ActiveCommand)
            .current,
        1
    );
    let mut job = fixture.queue.try_take().unwrap().unwrap();
    assert!(poll_once(&mut job).is_pending());
    assert_eq!(fixture.mutations(), 1);
    gate.notify_one();
    job.await.unwrap();
    assert_eq!(fixture.mutations(), 1);
    assert_eq!(fixture.releases(), 1);
    fixture.assert_drained();
}

#[tokio::test]
async fn caller_source_admission_survives_waiter_drop_until_group_release() {
    // Arrange
    let fixture = Fixture::new(4, 4096);
    let element = fixture.element().await;
    let native = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    fixture.client.set_evaluate_gate(Some(native.clone()));
    fixture
        .client
        .set_release_object_group_gate(Some(release.clone()));
    let admission = Arc::new(());
    let observed = Arc::downgrade(&admission);
    let mut future = fixture
        .service
        .prepare_retaining(element, fixture.payloads(), admission)
        .unwrap();
    assert!(poll_once(&mut future).is_pending());
    // Act
    drop(future);
    // Assert: first native response and then group cleanup are separate gates.
    assert!(observed.upgrade().is_some());
    let mut job = fixture.queue.try_take().unwrap().unwrap();
    assert!(poll_once(&mut job).is_pending());
    native.notify_one();
    assert!(poll_once(&mut job).is_pending());
    assert!(observed.upgrade().is_some());
    release.notify_one();
    job.await.unwrap();
    assert!(observed.upgrade().is_none());
    fixture.assert_drained();
}

#[tokio::test]
async fn unpolled_upload_returns_caller_source_admission_without_mutating() {
    // Arrange
    let fixture = Fixture::new(4, 4096);
    let element = fixture.element().await;
    let admission = Arc::new(());
    let observed = Arc::downgrade(&admission);
    let future = fixture
        .service
        .prepare_retaining(element, fixture.payloads(), admission)
        .unwrap();
    // Act
    drop(future);
    // Assert
    assert!(observed.upgrade().is_none());
    assert_eq!(fixture.mutations(), 0);
    fixture.queue.try_take().unwrap().unwrap().await.unwrap();
    fixture.assert_drained();
}

#[tokio::test]
async fn dropping_during_group_release_retains_source_and_never_reissues_release() {
    // Arrange
    let fixture = Fixture::new(4, 4096);
    let element = fixture.element().await;
    let gate = Arc::new(tokio::sync::Notify::new());
    fixture
        .client
        .set_release_object_group_gate(Some(gate.clone()));
    let mut future = fixture
        .service
        .prepare(element, fixture.payloads())
        .unwrap();
    assert!(poll_once(&mut future).is_pending());
    // Act
    drop(future);
    // Assert
    assert_eq!(fixture.releases(), 1);
    assert_eq!(
        fixture
            .source
            .metrics()
            .unwrap()
            .class(ResourceClass::ArtifactBytes)
            .current,
        3
    );
    let mut job = fixture.queue.try_take().unwrap().unwrap();
    assert!(poll_once(&mut job).is_pending());
    gate.notify_one();
    job.await.unwrap();
    assert_eq!(fixture.releases(), 1);
    fixture.assert_drained();
}

#[tokio::test(start_paused = true)]
async fn external_deadline_expires_without_releasing_the_native_continuation() {
    // Arrange
    let fixture = Fixture::new(4, 4096);
    let element = fixture.element().await;
    let gate = Arc::new(tokio::sync::Notify::new());
    fixture.client.set_evaluate_gate(Some(gate.clone()));
    let future = fixture
        .service
        .prepare(element, fixture.payloads())
        .unwrap();
    let mut timed = Box::pin(tokio::time::timeout(
        std::time::Duration::from_secs(1),
        future,
    ));
    assert!(poll_once(&mut timed).is_pending());
    // Act
    tokio::time::advance(std::time::Duration::from_secs(2)).await;
    let result = timed.await;
    // Assert
    assert!(result.is_err());
    assert_eq!(fixture.service.active_operations(), 1);
    assert_eq!(fixture.releases(), 0);
    gate.notify_one();
    fixture.queue.try_take().unwrap().unwrap().await.unwrap();
    fixture.assert_drained();
}

#[tokio::test]
async fn cleanup_slot_rejection_cannot_dispatch_or_leak_input_owners() {
    // Arrange
    let fixture = Fixture::new(1, 4096);
    let element = fixture.element().await;
    // Act
    let result = fixture.service.prepare(element, fixture.payloads());
    // Assert
    assert!(matches!(result, Err(UploadError::Admission(_))));
    assert_eq!(fixture.mutations(), 0);
    fixture.queue.try_take().unwrap().unwrap().await.unwrap();
    fixture.assert_drained();
}

#[tokio::test]
async fn owner_storage_rejection_occurs_before_mutation() {
    // Arrange
    let fixture = Fixture::new(4, 1);
    let element = fixture.element().await;
    // Act
    let result = fixture.service.prepare(element, fixture.payloads());
    // Assert
    assert!(matches!(result, Err(UploadError::Storage)));
    assert_eq!(fixture.mutations(), 0);
    fixture.queue.try_take().unwrap().unwrap().await.unwrap();
    fixture.assert_drained();
}

#[tokio::test]
async fn capacity_includes_an_unpolled_owned_operation() {
    // Arrange
    let fixture = Fixture::new(4, 4096);
    let first = fixture.element().await;
    let second = fixture.element().await;
    let future = fixture.service.prepare(first, fixture.payloads()).unwrap();
    // Act
    let result = fixture.service.prepare(second, fixture.payloads());
    // Assert
    assert!(matches!(result, Err(UploadError::Capacity)));
    assert_eq!(fixture.service.active_operations(), 1);
    assert_eq!(fixture.mutations(), 0);
    drop(future);
    while let Some(job) = fixture.queue.try_take().unwrap() {
        job.await.unwrap();
    }
    fixture.assert_drained();
}

#[tokio::test]
async fn backend_failure_is_preserved_after_successful_cleanup() {
    // Arrange
    let fixture = Fixture::new(4, 4096);
    let element = fixture.element().await;
    fixture
        .client
        .set_evaluate_response(json!({"result":{"value":null}}));
    // Act
    let result = fixture
        .service
        .prepare(element, fixture.payloads())
        .unwrap()
        .await;
    // Assert
    assert!(matches!(
        result,
        Err(UploadError::Backend(PayloadError::Protocol))
    ));
    assert_eq!(fixture.releases(), 1);
    assert!(fixture.service.is_cleanup_healthy());
    fixture.assert_drained();
}

#[tokio::test]
async fn cleanup_failure_preserves_primary_and_closes_new_admission() {
    // Arrange
    let fixture = Fixture::new(4, 4096);
    let first = fixture.element().await;
    let second = fixture.element().await;
    fixture
        .client
        .set_evaluate_response(json!({"result":{"value":null}}));
    fixture
        .client
        .set_release_object_error(Some("release failed".into()));
    // Act
    let result = fixture
        .service
        .prepare(first, fixture.payloads())
        .unwrap()
        .await;
    // Assert
    assert!(
        matches!(result, Err(UploadError::Cleanup { primary: Some(ref primary), .. })
        if matches!(primary.as_ref(), UploadError::Backend(PayloadError::Protocol)))
    );
    assert!(!fixture.service.is_cleanup_healthy());
    assert!(matches!(
        fixture.service.prepare(second, fixture.payloads()),
        Err(UploadError::Unhealthy)
    ));
    assert_eq!(fixture.mutations(), 1);
    assert!(fixture.queue.try_take().unwrap().unwrap().await.is_err());
    fixture.assert_drained();
}

#[tokio::test]
async fn abandoned_waiter_reports_cleanup_failure_to_the_executor() {
    // Arrange
    let fixture = Fixture::new(4, 4096);
    let element = fixture.element().await;
    let gate = Arc::new(tokio::sync::Notify::new());
    fixture.client.set_evaluate_gate(Some(gate.clone()));
    fixture
        .client
        .set_release_object_error(Some("release failed".repeat(4096)));
    let mut future = fixture
        .service
        .prepare(element, fixture.payloads())
        .unwrap();
    assert!(poll_once(&mut future).is_pending());
    // Act
    drop(future);
    // Assert
    gate.notify_one();
    let error = fixture
        .queue
        .try_take()
        .unwrap()
        .unwrap()
        .await
        .unwrap_err();
    assert_eq!(
        error.message(),
        "file-input group cleanup failed; service admission closed"
    );
    assert_eq!(fixture.queue.metrics().unwrap().failed, 1);
    assert!(!fixture.service.is_cleanup_healthy());
    fixture.assert_drained();
}

#[tokio::test]
async fn service_drop_does_not_drop_started_work_or_shared_state_admission() {
    // Arrange
    let fixture = Fixture::new(4, 4096);
    let element = fixture.element().await;
    let gate = Arc::new(tokio::sync::Notify::new());
    fixture.client.set_evaluate_gate(Some(gate.clone()));
    let mut future = fixture
        .service
        .prepare(element, fixture.payloads())
        .unwrap();
    assert!(poll_once(&mut future).is_pending());
    // Act
    drop(fixture.service);
    drop(future);
    // Assert
    assert_eq!(fixture.owners.metrics().unwrap().active_leases, 2);
    gate.notify_one();
    fixture.queue.try_take().unwrap().unwrap().await.unwrap();
    assert_eq!(fixture.owners.metrics().unwrap().active_leases, 0);
    assert_eq!(fixture.source.metrics().unwrap().active_leases, 0);
    assert_eq!(fixture.native.metrics().unwrap().active_leases, 0);
}

#[test]
fn profile_rejects_zero_and_overflowing_storage() {
    // Arrange
    let scope = ResourceScope::Connection;
    // Act
    let invalid = [(0, 4096), (1, 0), (usize::MAX, 4096), (2, usize::MAX)]
        .map(|(capacity, bytes)| UploadProfile::new(scope, capacity, bytes));
    // Assert
    assert!(invalid.iter().all(Result::is_err));
}

#[tokio::test]
async fn exhausted_owner_backing_rolls_back_the_admission_count_before_dispatch() {
    // Arrange
    let fixture = Fixture::new(4, 4096);
    let element = fixture.element().await;
    let profile = UploadProfile::new(ResourceScope::extension(146, 3), 1, 4096).unwrap();
    let ledger = fixture.owners.claim(profile.requirements()).unwrap();
    let held = ledger
        .reserve(
            ResourceScope::extension(146, 3),
            ResourceClass::BackendStateBytes,
            4096,
        )
        .unwrap();
    // Act
    let result = fixture.service.prepare(element, fixture.payloads());
    // Assert
    assert!(matches!(result, Err(UploadError::Resource(_))));
    assert_eq!(fixture.service.active_operations(), 0);
    assert_eq!(fixture.mutations(), 0);
    drop(held);
    fixture.queue.try_take().unwrap().unwrap().await.unwrap();
    fixture.assert_drained();
}

#[tokio::test]
async fn executor_discarding_started_work_poisons_admission_instead_of_claiming_cleanup() {
    // Arrange
    let fixture = Fixture::new(4, 4096);
    let element = fixture.element().await;
    fixture
        .client
        .set_evaluate_gate(Some(Arc::new(tokio::sync::Notify::new())));
    let mut future = fixture
        .service
        .prepare(element, fixture.payloads())
        .unwrap();
    assert!(poll_once(&mut future).is_pending());
    drop(future);
    let job = fixture.queue.try_take().unwrap().unwrap();
    // Act: deliberately violate the executor's drain contract.
    drop(job);
    // Assert: zero Rust owners is not misreported as proven native cleanup.
    assert!(!fixture.service.is_cleanup_healthy());
    assert_eq!(fixture.queue.metrics().unwrap().completed, 0);
    fixture.queue.try_take().unwrap().unwrap().await.unwrap();
    assert_eq!(fixture.mutations(), 1);
    fixture.assert_drained();
}
