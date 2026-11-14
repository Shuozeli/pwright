use super::*;
use pwright_composition_chromium_handle_evaluation::native_creation::NativeHandleBatchCreationCommit;

async fn target(fixture: &Fixture) -> ObjectId {
    let NativeHandleCreationCommit::Published(handle) = staged(fixture).await.commit_or_defer()
    else {
        panic!("target did not publish")
    };
    while fixture.runtime.pop_control_message().unwrap().is_some() {}
    handle.object_id
}

fn response(fixture: &Fixture, count: usize) {
    fixture.primary.set_evaluate_responses([
        serde_json::json!({"result": {"type": "object", "objectId": "batch-carrier"}}),
        serde_json::json!({"result": (0..count).map(|index| serde_json::json!({
            "name": format!("property-{index}"), "enumerable": true,
            "value": {"type": "object", "objectId": format!("value-{index}")}
        })).collect::<Vec<_>>()}),
    ]);
}

fn releases(fixture: &Fixture) -> Vec<String> {
    fixture
        .primary
        .calls()
        .into_iter()
        .filter(|(method, _)| method == "Runtime.releaseObject")
        .map(|(_, arguments)| arguments[0].as_str().unwrap().to_owned())
        .collect()
}

fn assert_routes(fixture: &Fixture, current: usize, reserved: usize) {
    let metrics = fixture.installation.lifecycle().routes().metrics().unwrap();
    assert_eq!(metrics.current, current);
    assert_eq!(metrics.reserved, reserved);
}

#[tokio::test]
async fn emitted_remote_objects_stage_and_publish_without_browser_io() {
    // Arrange
    let fixture = fixture(3);
    let service = fixture.installation.raw_service();
    let remotes = [
        serde_json::json!({"type": "string", "value": "hello"}),
        serde_json::json!({"type": "number", "value": 5}),
        serde_json::json!({
            "type": "object",
            "description": "Object",
            "objectId": "console-object-1"
        }),
    ];
    let before = fixture.primary.calls().len();

    // Act
    let pending = service
        .stage_remote_objects(ObjectId::new(4), &remotes)
        .expect("event RemoteObjects should stage");
    let result = pending.commit_or_defer();

    // Assert
    let NativeHandleBatchCreationCommit::Published(result) = result else {
        panic!("event Handle batch did not publish")
    };
    assert_eq!(fixture.primary.calls().len(), before);
    assert_eq!(result.properties.len(), 3);
    assert_eq!(result.properties[0].name, "0");
    assert_eq!(result.properties[0].handle.preview, "hello");
    assert_eq!(result.properties[1].name, "1");
    assert_eq!(result.properties[1].handle.preview, "5");
    assert_eq!(result.properties[2].name, "2");
    assert_eq!(result.properties[2].handle.preview, "Object");
    assert!(
        result
            .properties
            .iter()
            .all(|property| property.handle.parent_id == ObjectId::new(4))
    );
    assert_routes(&fixture, 3, 0);
    for property in result.properties {
        super::target::dispose(&fixture, property.handle.object_id).await;
    }
    assert_routes(&fixture, 0, 0);
}

#[test]
fn malformed_emitted_remote_batch_releases_all_local_route_admission() {
    // Arrange
    let fixture = fixture(2);
    let service = fixture.installation.raw_service();
    let remotes = [
        serde_json::json!({
            "type": "object",
            "description": "Object",
            "objectId": "console-object-1"
        }),
        serde_json::json!({"value": "missing type"}),
    ];
    let before = fixture.primary.calls().len();

    // Act
    let error = match service.stage_remote_objects(ObjectId::new(4), &remotes) {
        Ok(_) => panic!("malformed RemoteObject must be rejected"),
        Err(error) => error,
    };

    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert_eq!(fixture.primary.calls().len(), before);
    assert_routes(&fixture, 0, 0);
}

#[tokio::test]
async fn batch_capacity_is_admitted_before_browser_io() {
    // Arrange
    let fixture = fixture(3);
    let target = target(&fixture).await;
    response(&fixture, 2);
    let before = fixture.primary.calls().len();

    // Act
    let result = fixture
        .installation
        .raw_service()
        .stage_get_properties(target, CallMetadata::default(), 3)
        .await;

    // Assert
    assert_eq!(result.err().unwrap().kind(), FailureKind::ResourceExhausted);
    assert_eq!(fixture.primary.calls().len(), before);
    assert_routes(&fixture, 1, 0);
    super::target::dispose(&fixture, target).await;
}

#[tokio::test]
async fn nonprimitive_zero_capacity_is_rejected_before_browser_io() {
    // Arrange
    let fixture = fixture(1);
    let target = target(&fixture).await;
    let before = fixture.primary.calls().len();

    // Act
    let result = fixture
        .installation
        .raw_service()
        .stage_get_properties(target, CallMetadata::default(), 0)
        .await;

    // Assert
    assert_eq!(result.err().unwrap().kind(), FailureKind::ResourceExhausted);
    assert_eq!(fixture.primary.calls().len(), before);
    assert_routes(&fixture, 1, 0);
    super::target::dispose(&fixture, target).await;
}

#[tokio::test]
async fn primitive_zero_capacity_preserves_world_and_performs_no_browser_io() {
    // Arrange
    let fixture = fixture(1);
    fixture
        .primary
        .set_evaluate_response(serde_json::json!({"result": {"type": "number", "value": 7}}));
    let service = fixture.installation.raw_service();
    let pending = service
        .stage_evaluate_handle(
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::expression("7"),
        )
        .await
        .unwrap();
    let NativeHandleCreationCommit::Published(target) = pending.commit_or_defer() else {
        panic!("primitive did not publish")
    };
    let before = fixture.primary.calls().len();

    // Act
    let pending = service
        .stage_get_properties(target.object_id, CallMetadata::default(), 0)
        .await
        .unwrap();

    // Assert
    assert!(pending.is_empty());
    assert_eq!(pending.len(), 0);
    assert_eq!(pending.world_identity().unique_context_id(), "world-1");
    assert_eq!(fixture.primary.calls().len(), before);
    let NativeHandleBatchCreationCommit::Published(result) = pending.commit_or_defer() else {
        panic!("primitive batch did not publish")
    };
    assert!(result.properties.is_empty());
    assert_routes(&fixture, 1, 0);
    super::target::dispose(&fixture, target.object_id).await;
}

#[tokio::test]
async fn empty_object_batch_releases_carrier_and_all_reserved_routes() {
    // Arrange
    let fixture = fixture(3);
    let target = target(&fixture).await;
    response(&fixture, 0);
    let before = fixture.runtime.live_object_count().unwrap();

    // Act
    let pending = fixture
        .installation
        .raw_service()
        .stage_get_properties(target, CallMetadata::default(), 2)
        .await
        .unwrap();

    // Assert
    assert!(pending.is_empty());
    assert_routes(&fixture, 1, 0);
    assert_eq!(releases(&fixture), ["batch-carrier"]);
    let NativeHandleBatchCreationCommit::Published(result) = pending.commit_or_defer() else {
        panic!("empty batch did not publish")
    };
    assert!(result.properties.is_empty());
    assert_eq!(fixture.runtime.live_object_count().unwrap(), before);
    super::target::dispose(&fixture, target).await;
}

#[tokio::test]
async fn batch_trims_unused_admission_and_publishes_independent_sibling_identities() {
    // Arrange
    let fixture = fixture(4);
    let target = target(&fixture).await;
    response(&fixture, 2);
    let before = fixture.runtime.live_object_count().unwrap();
    let pending = fixture
        .installation
        .raw_service()
        .stage_get_properties(target, CallMetadata::default(), 3)
        .await
        .unwrap();
    assert_eq!(pending.len(), 2);
    assert_routes(&fixture, 1, 2);
    assert_eq!(fixture.runtime.live_object_count().unwrap(), before);
    let calls = fixture.primary.calls();
    let acquisition = &calls
        .iter()
        .rev()
        .find(|(method, _)| method == "Runtime.callFunctionOn")
        .unwrap()
        .1[0];
    assert_eq!(acquisition["uniqueContextId"], "world-1");
    assert_eq!(acquisition["arguments"][1]["objectId"], "staged-native");
    assert_eq!(acquisition["arguments"][0]["value"]["argument"]["v"], 3.0);
    let call_count = calls.len();

    // Act
    let result = pending.commit_or_defer();

    // Assert
    let NativeHandleBatchCreationCommit::Published(result) = result else {
        panic!("batch did not publish")
    };
    assert_eq!(fixture.primary.calls().len(), call_count);
    assert_routes(&fixture, 3, 0);
    assert_eq!(fixture.runtime.live_object_count().unwrap(), before + 2);
    assert_eq!(result.properties[0].name, "property-0");
    assert_eq!(result.properties[1].name, "property-1");
    assert_ne!(
        result.properties[0].handle.object_id,
        result.properties[1].handle.object_id
    );
    for property in &result.properties {
        assert_eq!(property.handle.parent_id, ObjectId::new(4));
        assert_ne!(property.handle.object_id, target);
    }
    super::target::dispose(&fixture, target).await;
    for property in result.properties {
        assert!(
            fixture
                .runtime
                .is_object_live(property.handle.object_id)
                .unwrap()
        );
        super::target::dispose(&fixture, property.handle.object_id).await;
    }
    assert_routes(&fixture, 0, 0);
}

#[tokio::test]
async fn busy_batch_commit_preserves_all_results_without_repeating_browser_work() {
    // Arrange
    let fixture = fixture(3);
    let target = target(&fixture).await;
    response(&fixture, 2);
    let pending = fixture
        .installation
        .raw_service()
        .stage_get_properties(target, CallMetadata::default(), 2)
        .await
        .unwrap();
    let gate = fixture
        .runtime
        .prepare_object_disposal(ObjectId::new(5))
        .unwrap();
    let before = fixture.primary.calls().len();

    // Act
    let result = pending.commit_or_defer();

    // Assert
    let NativeHandleBatchCreationCommit::Deferred(pending) = result else {
        panic!("busy batch was not deferred")
    };
    assert_eq!(pending.len(), 2);
    assert_routes(&fixture, 1, 2);
    assert_eq!(fixture.primary.calls().len(), before);
    drop(gate);
    let NativeHandleBatchCreationCommit::Published(result) = pending.commit_or_defer() else {
        panic!("batch did not publish after lifecycle gate")
    };
    assert_eq!(fixture.primary.calls().len(), before);
    for property in result.properties {
        super::target::dispose(&fixture, property.handle.object_id).await;
    }
    super::target::dispose(&fixture, target).await;
}

#[tokio::test]
async fn disposed_source_rejects_entire_batch_and_compensation_attempts_every_sibling() {
    // Arrange
    let fixture = fixture(3);
    let target = target(&fixture).await;
    response(&fixture, 2);
    let pending = fixture
        .installation
        .raw_service()
        .stage_get_properties(target, CallMetadata::default(), 2)
        .await
        .unwrap();
    super::target::dispose(&fixture, target).await;
    fixture
        .primary
        .set_release_object_error(Some("release failure".into()));
    let before = fixture.runtime.live_object_count().unwrap();

    // Act
    let result = pending.commit_or_defer();

    // Assert
    let NativeHandleBatchCreationCommit::Rejected(failure) = result else {
        panic!("disposed source published siblings")
    };
    assert_eq!(failure.error().kind(), FailureKind::TargetClosed);
    assert_eq!(failure.release().await.kind(), FailureKind::Cleanup);
    assert_eq!(
        releases(&fixture),
        ["batch-carrier", "staged-native", "value-0", "value-1"]
    );
    assert_eq!(fixture.runtime.live_object_count().unwrap(), before);
    assert_routes(&fixture, 0, 0);
}

#[tokio::test]
async fn overflow_sentinel_is_compensated_without_publishing_any_sibling() {
    // Arrange
    let fixture = fixture(3);
    let target = target(&fixture).await;
    response(&fixture, 3);
    let before = fixture.runtime.live_object_count().unwrap();

    // Act
    let result = fixture
        .installation
        .raw_service()
        .stage_get_properties(target, CallMetadata::default(), 2)
        .await;

    // Assert
    assert_eq!(result.err().unwrap().kind(), FailureKind::ResourceExhausted);
    assert_eq!(
        releases(&fixture),
        ["value-0", "value-1", "value-2", "batch-carrier"]
    );
    assert_eq!(fixture.runtime.live_object_count().unwrap(), before);
    assert_routes(&fixture, 1, 0);
    super::target::dispose(&fixture, target).await;
}

#[tokio::test]
async fn property_read_and_carrier_release_keep_one_evaluation_lease_and_maximum_route_admission() {
    // Arrange
    let fixture = fixture(4);
    let target = target(&fixture).await;
    response(&fixture, 2);
    let baseline = fixture.resources.metrics().unwrap();
    let read_gate = Arc::new(tokio::sync::Notify::new());
    let release_gate = Arc::new(tokio::sync::Notify::new());
    fixture
        .primary
        .set_get_properties_gate(Some(read_gate.clone()));
    fixture
        .primary
        .set_release_object_gate(Some(release_gate.clone()));
    let service = fixture.installation.raw_service();
    let mut operation = Box::pin(service.stage_get_properties(target, CallMetadata::default(), 3));

    // Act: caller retains its raw creation future through both settlement gates.
    assert!(poll_now(operation.as_mut()).is_pending());
    let blocked_read = fixture.resources.metrics().unwrap();
    assert_eq!(blocked_read.active_leases, baseline.active_leases + 4);
    assert_routes(&fixture, 1, 3);
    assert_eq!(
        fixture.primary.calls().last().unwrap().0,
        "Runtime.getProperties"
    );
    fixture.primary.set_get_properties_gate(None);
    read_gate.notify_waiters();
    assert!(poll_now(operation.as_mut()).is_pending());

    // Assert: release is still inside the SAME evaluation admission; no new
    // queue or lease appears, and results are not portable until local commit.
    let blocked_release = fixture.resources.metrics().unwrap();
    assert_eq!(blocked_release.active_leases, blocked_read.active_leases);
    for class in ResourceClass::ALL {
        assert_eq!(
            blocked_release.class(class).current,
            blocked_read.class(class).current
        );
    }
    assert_routes(&fixture, 1, 3);
    assert_eq!(releases(&fixture), ["batch-carrier"]);
    fixture.primary.set_release_object_gate(None);
    release_gate.notify_waiters();
    let pending = operation.await.unwrap();
    assert_routes(&fixture, 1, 2);
    pending.release().await.unwrap();
    assert_routes(&fixture, 1, 0);
    assert_eq!(
        fixture.resources.metrics().unwrap().active_leases,
        baseline.active_leases
    );
    super::target::dispose(&fixture, target).await;
}
