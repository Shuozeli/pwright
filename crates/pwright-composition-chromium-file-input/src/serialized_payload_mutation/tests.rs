use super::*;
use crate::{
    CdpFileInputPreflightService, CdpTargetMarkingService, FileInputPreflightFlags,
    FileInputPreflightOperationService, MarkerWorld, OwnedTargetMarker,
    TargetMarkingOperationService,
};
use pwright_backend_cdp_file_input::{
    RetainedPreflightProfile, SerializedPayloadMutationProfile, TargetMarkingProfile,
};
use pwright_composition_chromium_locator_resolution::{
    CdpLocatorResolutionService, ContinuableResolutionService, ResolutionProfile,
    RetainedElementRequest,
};
use pwright_core::{AtomicObjectIdSource, ResourceLimits, ResourceRequirements, ResourceScope};
use pwright_fake::FakeCdpClient;
use pwright_runtime::{CleanupQueue, CleanupQueueReservation};
use serde_json::json;
use std::task::Waker;

type Validated = OwnedValidatedMarkedFileInput<
    CleanupQueueReservation,
    CleanupQueueReservation,
    CleanupQueueReservation,
>;

fn production(requirements: ResourceRequirements) -> ProductionResourceLedger {
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let bound = requirements.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    ProductionResourceLedger::new(limits, requirements).unwrap()
}

fn graph() -> Value {
    json!({
        "a": [{
            "o": [
                {"k":"name","v":"one.txt"},
                {"k":"mimeType","v":"text/plain"},
                {"k":"buffer","v":"AQI="},
                {"k":"lastModifiedMs","v":{"v":"undefined"}}
            ],
            "id": 2
        }],
        "id": 1
    })
}

struct Fixture {
    queue: CleanupQueue,
    client: Arc<FakeCdpClient>,
    resolver: ContinuableResolutionService<CdpLocatorResolutionService, CleanupQueue>,
    marker: TargetMarkingOperationService<CleanupQueue>,
    preflight: FileInputPreflightOperationService<CleanupQueue>,
    mutation: SerializedPayloadMutationOperationService<CleanupQueue>,
    resolution_resources: ProductionResourceLedger,
    marker_backend_resources: ProductionResourceLedger,
    marker_owner_resources: ProductionResourceLedger,
    preflight_backend_resources: ProductionResourceLedger,
    preflight_owner_resources: ProductionResourceLedger,
    mutation_backend_resources: ProductionResourceLedger,
    mutation_owner_resources: ProductionResourceLedger,
    baseline_leases: [usize; 7],
}

impl Fixture {
    fn new() -> Self {
        let queue = CleanupQueue::new(16);
        let client = Arc::new(FakeCdpClient::from_html("<input type=file>"));
        client.set_object_backend_node("source", 17);
        client.set_adopted_node_response(
            17,
            23,
            json!({"object":{"type":"object","subtype":"node","objectId":"adopted"}}),
        );

        let resolution = ResolutionProfile::new(ResourceScope::extension(191, 11), 8)
            .unwrap()
            .with_retained_reference()
            .unwrap();
        let resolution_resources = production(resolution.requirements());
        let resolver = ContinuableResolutionService::new(
            Arc::new(
                CdpLocatorResolutionService::new(
                    resolution,
                    &resolution_resources,
                    Arc::new(AtomicObjectIdSource::new(1).unwrap()),
                )
                .unwrap(),
            ),
            queue.clone(),
        );

        let marker_profile =
            TargetMarkingProfile::new(ResourceScope::extension(191, 12), 1, 1).unwrap();
        let marker_backend_resources = production(marker_profile.requirements());
        let marker_backend = Arc::new(
            CdpTargetMarkingService::new(
                marker_profile,
                &marker_backend_resources,
                Arc::new(AtomicObjectIdSource::new(1).unwrap()),
            )
            .unwrap(),
        );
        let marker_state = UploadProfile::new(ResourceScope::extension(191, 13), 1, 16384).unwrap();
        let marker_call = UploadProfile::new(ResourceScope::extension(191, 14), 1, 16384).unwrap();
        let marker_owner_resources = production(
            marker_state
                .requirements()
                .combine_concurrent(marker_call.requirements()),
        );
        let marker = TargetMarkingOperationService::new(
            marker_state,
            marker_call,
            &marker_owner_resources,
            marker_backend,
            queue.clone(),
        )
        .unwrap();

        let preflight_profile =
            RetainedPreflightProfile::new(ResourceScope::extension(191, 15), 1).unwrap();
        let preflight_backend_resources = production(preflight_profile.requirements());
        let preflight_backend = Arc::new(
            CdpFileInputPreflightService::new(preflight_profile, &preflight_backend_resources)
                .unwrap(),
        );
        let preflight_owner =
            UploadProfile::new(ResourceScope::extension(191, 16), 1, 16384).unwrap();
        let preflight_owner_resources = production(preflight_owner.requirements());
        let preflight = FileInputPreflightOperationService::new(
            preflight_owner,
            &preflight_owner_resources,
            preflight_backend,
            queue.clone(),
        )
        .unwrap();

        let mutation_profile =
            SerializedPayloadMutationProfile::new(ResourceScope::extension(191, 17), 1).unwrap();
        let mutation_backend_resources = production(mutation_profile.requirements().unwrap());
        let mutation_backend = Arc::new(
            CdpSerializedPayloadMutationService::new(mutation_profile, &mutation_backend_resources)
                .unwrap(),
        );
        let mutation_owner =
            UploadProfile::new(ResourceScope::extension(191, 18), 1, 16384).unwrap();
        let mutation_owner_resources = production(mutation_owner.requirements());
        let mutation = SerializedPayloadMutationOperationService::new(
            mutation_owner,
            &mutation_owner_resources,
            mutation_backend,
            queue.clone(),
        )
        .unwrap();
        let baseline_leases = [
            resolution_resources.metrics().unwrap().active_leases,
            marker_backend_resources.metrics().unwrap().active_leases,
            marker_owner_resources.metrics().unwrap().active_leases,
            preflight_backend_resources.metrics().unwrap().active_leases,
            preflight_owner_resources.metrics().unwrap().active_leases,
            mutation_backend_resources.metrics().unwrap().active_leases,
            mutation_owner_resources.metrics().unwrap().active_leases,
        ];

        Self {
            queue,
            client,
            resolver,
            marker,
            preflight,
            mutation,
            resolution_resources,
            marker_backend_resources,
            marker_owner_resources,
            preflight_backend_resources,
            preflight_owner_resources,
            mutation_backend_resources,
            mutation_owner_resources,
            baseline_leases,
        }
    }

    async fn validated(&self) -> (Validated, OwnedTargetMarker<CleanupQueueReservation>) {
        self.client
            .set_evaluate_response(json!({"result":{"type":"object","objectId":"private-marker"}}));
        let marker = self
            .marker
            .create(
                MarkerWorld {
                    client: self.client.clone(),
                    utility_world: "utility-23".into(),
                    destruction: None,
                },
                None,
            )
            .unwrap()
            .await
            .unwrap();
        self.client
            .set_evaluate_response(json!({"result":{"type":"boolean","value":true}}));
        let element = self
            .resolver
            .call_retained_retaining(
                RetainedElementRequest {
                    source_client: self.client.clone(),
                    source_object_id: "source".into(),
                    client: self.client.clone(),
                    utility_world: "utility-23".into(),
                    utility_execution_context_id: 23,
                    destruction: None,
                },
                None,
            )
            .await
            .unwrap();
        self.client
            .set_evaluate_response(json!({"result":{"type":"undefined"}}));
        let marked = self
            .marker
            .mark(&marker, element, "call@1".into())
            .unwrap()
            .await
            .unwrap();
        self.client.set_evaluate_response(json!({
            "result":{"type":"object","subtype":"node","objectId":"retargeted-input"}
        }));
        let validated = self
            .preflight
            .prepare_marked(
                marked,
                FileInputPreflightFlags {
                    multiple: false,
                    directory_upload: false,
                },
            )
            .unwrap()
            .await
            .unwrap();
        (validated, marker)
    }

    fn mutations(&self) -> usize {
        self.client
            .calls()
            .iter()
            .filter(|(method, args)| {
                method == "Runtime.callFunctionOn"
                    && args[0]["functionDeclaration"]
                        .as_str()
                        .is_some_and(|source| source.contains("new DataTransfer"))
            })
            .count()
    }

    fn target_releases(&self) -> usize {
        self.client
            .method_names()
            .iter()
            .filter(|method| *method == "Runtime.releaseObjectGroup")
            .count()
    }

    async fn finish_marker(&self, marker: OwnedTargetMarker<CleanupQueueReservation>) {
        marker
            .release()
            .ok()
            .expect("validated owner released its mark call")
            .await
            .unwrap();
        let current = [
            self.resolution_resources.metrics().unwrap().active_leases,
            self.marker_backend_resources
                .metrics()
                .unwrap()
                .active_leases,
            self.marker_owner_resources.metrics().unwrap().active_leases,
            self.preflight_backend_resources
                .metrics()
                .unwrap()
                .active_leases,
            self.preflight_owner_resources
                .metrics()
                .unwrap()
                .active_leases,
            self.mutation_backend_resources
                .metrics()
                .unwrap()
                .active_leases,
            self.mutation_owner_resources
                .metrics()
                .unwrap()
                .active_leases,
        ];
        assert_eq!(current, self.baseline_leases);
        assert_eq!(self.queue.metrics().unwrap().reserved, 0);
        assert_eq!(self.queue.metrics().unwrap().queued, 0);
    }
}

fn poll_once<F: Future + Unpin>(future: &mut F) -> Poll<F::Output> {
    Pin::new(future).poll(&mut Context::from_waker(Waker::noop()))
}

#[tokio::test]
async fn successful_mutation_releases_the_exact_target_before_publication() {
    // Arrange
    let fixture = Fixture::new();
    let (validated, marker) = fixture.validated().await;
    let retained = Arc::new(());
    let observed = Arc::downgrade(&retained);
    fixture
        .client
        .set_evaluate_response(json!({"result":{"type":"undefined"}}));
    let future = fixture
        .mutation
        .prepare_retaining(validated, graph(), retained)
        .unwrap();

    // Act
    let result = future.await;

    // Assert
    result.unwrap();
    assert_eq!(fixture.mutations(), 1);
    assert_eq!(fixture.target_releases(), 1);
    assert!(observed.upgrade().is_none());
    assert_eq!(fixture.mutation.active_operations(), 0);
    assert_eq!(fixture.preflight.active_operations(), 0);
    assert_eq!(fixture.marker.active_calls(), 0);
    fixture.finish_marker(marker).await;
}

#[tokio::test]
async fn dropping_an_unpolled_mutation_sends_no_dom_command() {
    // Arrange
    let fixture = Fixture::new();
    let (validated, marker) = fixture.validated().await;
    let retained = Arc::new(());
    let observed = Arc::downgrade(&retained);
    let future = fixture
        .mutation
        .prepare_retaining(validated, graph(), retained)
        .unwrap();

    // Act
    drop(future);

    // Assert
    assert_eq!(fixture.mutations(), 0);
    assert!(observed.upgrade().is_none());
    assert_eq!(fixture.mutation.active_operations(), 0);
    fixture.queue.try_take().unwrap().unwrap().await.unwrap();
    assert_eq!(fixture.target_releases(), 1);
    fixture.finish_marker(marker).await;
}

#[tokio::test]
async fn dropping_a_started_waiter_retains_graph_owner_until_mutation_and_cleanup_settle() {
    // Arrange
    let fixture = Fixture::new();
    let (validated, marker) = fixture.validated().await;
    let gate = Arc::new(tokio::sync::Notify::new());
    fixture.client.set_evaluate_gate(Some(gate.clone()));
    let retained = Arc::new(());
    let observed = Arc::downgrade(&retained);
    let mut future = fixture
        .mutation
        .prepare_retaining(validated, graph(), retained)
        .unwrap();
    assert!(poll_once(&mut future).is_pending());

    // Act
    drop(future);

    // Assert
    assert_eq!(fixture.mutations(), 1);
    assert_eq!(fixture.target_releases(), 0);
    assert!(observed.upgrade().is_some());
    assert_eq!(fixture.mutation.active_operations(), 1);
    let mut cleanup = fixture.queue.try_take().unwrap().unwrap();
    assert!(poll_once(&mut cleanup).is_pending());
    gate.notify_one();
    cleanup.await.unwrap();
    assert_eq!(fixture.mutations(), 1);
    assert_eq!(fixture.target_releases(), 1);
    assert!(observed.upgrade().is_none());
    assert_eq!(fixture.mutation.active_operations(), 0);
    fixture.finish_marker(marker).await;
}

#[tokio::test]
async fn failed_group_cleanup_poisons_later_mutation_admission() {
    // Arrange
    let fixture = Fixture::new();
    let (validated, marker) = fixture.validated().await;
    fixture
        .client
        .set_evaluate_response(json!({"result":{"type":"undefined"}}));
    fixture
        .client
        .set_release_object_error(Some("release refused".into()));

    // Act
    let result = fixture
        .mutation
        .prepare_retaining(validated, graph(), Arc::new(()))
        .unwrap()
        .await;

    // Assert
    assert!(matches!(
        result,
        Err(SerializedPayloadOperationError::Cleanup { .. })
    ));
    assert!(!fixture.mutation.is_cleanup_healthy());
    fixture.client.set_release_object_error(None);

    let fresh = Fixture::new();
    let (candidate, candidate_marker) = fresh.validated().await;
    let retry = fixture
        .mutation
        .prepare_retaining(candidate, graph(), Arc::new(()));
    assert!(matches!(
        retry,
        Err(SerializedPayloadOperationError::Unhealthy)
    ));
    fresh.queue.try_take().unwrap().unwrap().await.unwrap();
    fresh.finish_marker(candidate_marker).await;
    fixture.finish_marker(marker).await;
}
