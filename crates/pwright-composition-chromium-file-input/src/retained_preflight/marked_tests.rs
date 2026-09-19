use super::*;
use pwright_backend_cdp_file_input::{RetainedPreflightProfile, TargetMarkingProfile};
use pwright_composition_chromium_locator_resolution::{
    CdpLocatorResolutionService, ContinuableResolutionService, ResolutionProfile,
    RetainedElementRequest,
};
use pwright_core::{AtomicObjectIdSource, ResourceLimits};
use pwright_fake::FakeCdpClient;
use pwright_runtime::CleanupQueue;
use serde_json::json;

fn production(requirements: ResourceRequirements) -> ProductionResourceLedger {
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let bound = requirements.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    ProductionResourceLedger::new(limits, requirements).unwrap()
}

#[tokio::test]
async fn prevalidation_retains_the_exact_mark_call_until_validated_owner_release() {
    // Arrange
    let queue = CleanupQueue::new(8);
    let client = Arc::new(FakeCdpClient::from_html("<input type=file>"));
    client.set_object_backend_node("source", 17);
    client.set_adopted_node_response(
        17,
        23,
        json!({"object":{"type":"object","subtype":"node","objectId":"adopted"}}),
    );

    let resolution = ResolutionProfile::new(ResourceScope::extension(187, 11), 8)
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
        TargetMarkingProfile::new(ResourceScope::extension(187, 12), 1, 1).unwrap();
    let marker_backend_resources = production(marker_profile.requirements());
    let marker_backend = Arc::new(
        CdpTargetMarkingService::new(
            marker_profile,
            &marker_backend_resources,
            Arc::new(AtomicObjectIdSource::new(1).unwrap()),
        )
        .unwrap(),
    );
    let marker_state = UploadProfile::new(ResourceScope::extension(187, 13), 1, 16384).unwrap();
    let marker_call = UploadProfile::new(ResourceScope::extension(187, 14), 1, 16384).unwrap();
    let marker_owner_resources = production(
        marker_state
            .requirements()
            .combine_concurrent(marker_call.requirements()),
    );
    let marker_service = TargetMarkingOperationService::new(
        marker_state,
        marker_call,
        &marker_owner_resources,
        marker_backend,
        queue.clone(),
    )
    .unwrap();

    let preflight_profile =
        RetainedPreflightProfile::new(ResourceScope::extension(187, 15), 1).unwrap();
    let preflight_backend_resources = production(preflight_profile.requirements());
    let preflight_backend = Arc::new(
        CdpFileInputPreflightService::new(preflight_profile, &preflight_backend_resources).unwrap(),
    );
    let preflight_owner = UploadProfile::new(ResourceScope::extension(187, 16), 1, 16384).unwrap();
    let preflight_owner_resources = production(preflight_owner.requirements());
    let preflight_service = FileInputPreflightOperationService::new(
        preflight_owner,
        &preflight_owner_resources,
        preflight_backend,
        queue.clone(),
    )
    .unwrap();

    client.set_evaluate_response(json!({"result":{"type":"object","objectId":"private-marker"}}));
    let marker = marker_service
        .create(
            MarkerWorld {
                client: client.clone(),
                utility_world: "utility-23".into(),
                destruction: None,
            },
            None,
        )
        .unwrap()
        .await
        .unwrap();
    client.set_evaluate_response(json!({"result":{"type":"boolean","value":true}}));
    let element = resolver
        .call_retained_retaining(
            RetainedElementRequest {
                source_client: client.clone(),
                source_object_id: "source".into(),
                client: client.clone(),
                utility_world: "utility-23".into(),
                utility_execution_context_id: 23,
                destruction: None,
            },
            None,
        )
        .await
        .unwrap();
    client.set_evaluate_response(json!({"result":{"type":"undefined"}}));
    let marked = marker_service
        .mark(&marker, element, "call@1".into())
        .unwrap()
        .await
        .unwrap();
    assert_eq!(marker_service.active_calls(), 1);

    // Act
    client.set_evaluate_response(json!({
        "result":{"type":"object","subtype":"node","objectId":"retargeted-input"}
    }));
    let validated = preflight_service
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

    // Assert
    assert_eq!(validated.object_id(), "retargeted-input");
    assert_eq!(validated.element().object_id(), "adopted");
    assert_eq!(marker_service.active_calls(), 1);
    assert_eq!(preflight_service.active_operations(), 1);
    validated.release().await.unwrap();
    assert_eq!(marker_service.active_calls(), 0);
    assert_eq!(preflight_service.active_operations(), 0);
    assert_eq!(resolution_resources.metrics().unwrap().active_leases, 0);
    marker
        .release()
        .ok()
        .expect("validated owner released its mark call")
        .await
        .unwrap();
    assert_eq!(marker_service.active_states(), 0);
    assert_eq!(queue.metrics().unwrap().reserved, 0);
    assert_eq!(queue.metrics().unwrap().queued, 0);
}
