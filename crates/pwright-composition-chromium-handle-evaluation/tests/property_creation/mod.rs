use super::*;

async fn target(fixture: &Fixture) -> ObjectId {
    fixture.primary.set_evaluate_response(serde_json::json!({
        "result": {"type": "object", "objectId": "property-target", "description": "Object"}
    }));
    fixture
        .installation
        .raw_service()
        .call(Command::<EvaluateHandle>::new(
            CallId::new(71),
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::expression("({})"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap()
        .object_id
}

fn property_response(fixture: &Fixture) {
    fixture.primary.set_evaluate_responses([
        serde_json::json!({
            "result": {"type": "object", "objectId": "property-snapshot", "description": "Object"}
        }),
        serde_json::json!({
            "result": [{"name": "promise", "enumerable": true, "value": {
                "type": "object", "subtype": "promise", "objectId": "property-promise", "description": "Promise"
            }}]
        }),
    ]);
}

fn command(target: ObjectId) -> Command<HandleGetProperty> {
    Command::new(
        CallId::new(72),
        target,
        CallMetadata::default(),
        GetPropertyParams::new("promise"),
    )
}

async fn dispose(fixture: &Fixture, target: ObjectId) {
    fixture
        .installation
        .lifecycle()
        .raw_dispose_service()
        .call(Command::<HandleDispose>::new(
            CallId::new(73),
            target,
            CallMetadata::default(),
            (),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
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

#[tokio::test]
async fn portable_property_uses_a_snapshot_and_publishes_only_the_requested_result() {
    // Arrange
    let fixture = fixture(2);
    let target = target(&fixture).await;
    property_response(&fixture);
    let before = fixture.runtime.live_object_count().unwrap();

    // Act
    let property = fixture
        .installation
        .raw_service()
        .call(command(target))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();

    // Assert
    assert_eq!(property.preview, "Promise");
    assert_eq!(property.parent_id, ObjectId::new(4));
    assert_eq!(fixture.runtime.live_object_count().unwrap(), before + 1);
    let calls = fixture.primary.calls();
    let source = calls[1].1[0]["arguments"][0]["value"]["source"]
        .as_str()
        .unwrap();
    assert!(source.contains("{ __proto__: null }"));
    assert!(source.contains("result[name] = target[name]"));
    assert_eq!(calls[2].0, "Runtime.getProperties");
    assert_eq!(releases(&fixture), ["property-snapshot"]);
    let metrics = fixture.installation.lifecycle().routes().metrics().unwrap();
    assert_eq!(metrics.current, 2);
    assert_eq!(metrics.reserved, 0);
    dispose(&fixture, property.object_id).await;
    dispose(&fixture, target).await;
}

#[tokio::test]
async fn portable_property_retains_original_native_identity_for_later_commands() {
    // Arrange
    let fixture = fixture(2);
    let target = target(&fixture).await;
    property_response(&fixture);
    let service = fixture.installation.raw_service();
    let property = service
        .call(command(target))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    fixture.primary.set_evaluate_response(serde_json::json!({
        "result": {"value": {"ok": true, "value": {"t": "bool", "v": true}}}
    }));

    // Act
    let value = service
        .call(Command::<HandleEvaluate>::new(
            CallId::new(74),
            property.object_id,
            CallMetadata::default(),
            HandleEvaluateParams::function_without_argument("value => value instanceof Promise"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();

    // Assert
    assert_eq!(value.value, EvaluationValue::Bool(true));
    let calls = fixture.primary.calls();
    let request = &calls.last().unwrap().1[0];
    assert_eq!(request["uniqueContextId"], "world-1");
    assert_eq!(request["arguments"][1]["objectId"], "property-promise");
    dispose(&fixture, property.object_id).await;
    dispose(&fixture, target).await;
}

#[tokio::test]
async fn portable_property_result_lifetime_is_independent_after_target_disposal() {
    // Arrange
    let fixture = fixture(2);
    let target = target(&fixture).await;
    property_response(&fixture);
    let property = fixture
        .installation
        .raw_service()
        .call(command(target))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();

    // Act
    dispose(&fixture, target).await;

    // Assert
    assert!(!fixture.runtime.is_object_live(target).unwrap());
    assert!(fixture.runtime.is_object_live(property.object_id).unwrap());
    assert_eq!(releases(&fixture), ["property-snapshot", "property-target"]);
    assert_eq!(
        fixture
            .installation
            .lifecycle()
            .routes()
            .metrics()
            .unwrap()
            .current,
        1
    );
    dispose(&fixture, property.object_id).await;
    assert_eq!(releases(&fixture).last().unwrap(), "property-promise");
}

#[tokio::test]
async fn portable_property_reserves_result_capacity_before_any_snapshot_io() {
    // Arrange
    let fixture = fixture(1);
    let target = target(&fixture).await;
    property_response(&fixture);
    let before = fixture.primary.calls().len();

    // Act
    let error = fixture
        .installation
        .raw_service()
        .call(command(target))
        .await
        .unwrap()
        .into_outcome()
        .unwrap_err();

    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert_eq!(fixture.primary.calls().len(), before);
    assert_eq!(
        fixture
            .installation
            .lifecycle()
            .routes()
            .metrics()
            .unwrap()
            .reserved,
        0
    );
    dispose(&fixture, target).await;
}

#[tokio::test]
async fn portable_property_world_replacement_compensates_result_without_publication() {
    // Arrange
    let fixture = fixture(2);
    let target = target(&fixture).await;
    property_response(&fixture);
    let gate = Arc::new(tokio::sync::Notify::new());
    fixture.primary.set_evaluate_gate(Some(gate.clone()));
    let service = fixture.installation.raw_service();
    let mut operation = Box::pin(service.call(command(target)));
    assert!(poll_now(operation.as_mut()).is_pending());

    // Act
    let routes = fixture.installation.routes();
    routes.unbind(ObjectId::new(4)).unwrap();
    routes
        .bind(
            ObjectId::new(4),
            ObjectId::new(3),
            fixture.secondary.clone(),
            204,
            "replacement-world",
        )
        .unwrap();
    fixture.primary.set_evaluate_gate(None);
    gate.notify_waiters();
    let error = operation.await.unwrap().into_outcome().unwrap_err();

    // Assert
    assert_eq!(error.kind(), FailureKind::TargetClosed);
    assert_eq!(
        releases(&fixture),
        ["property-snapshot", "property-promise"]
    );
    assert!(!fixture.runtime.is_object_live(ObjectId::new(11)).unwrap());
    assert!(fixture.secondary.calls().is_empty());
    let metrics = fixture.installation.lifecycle().routes().metrics().unwrap();
    assert_eq!(metrics.current, 1);
    assert_eq!(metrics.reserved, 0);
    dispose(&fixture, target).await;
}

#[tokio::test]
async fn portable_property_propagates_backend_shape_error_and_returns_admission() {
    // Arrange
    let fixture = fixture(2);
    let target = target(&fixture).await;
    fixture.primary.set_evaluate_responses([
        serde_json::json!({"result": {"type": "object", "objectId": "property-snapshot"}}),
        serde_json::json!({"result": []}),
    ]);

    // Act
    let error = fixture
        .installation
        .raw_service()
        .call(command(target))
        .await
        .unwrap()
        .into_outcome()
        .unwrap_err();

    // Assert
    assert_eq!(error.kind(), FailureKind::ProtocolViolation);
    assert!(error.message().contains("exactly the requested property"));
    assert_eq!(releases(&fixture), ["property-snapshot"]);
    let metrics = fixture.installation.lifecycle().routes().metrics().unwrap();
    assert_eq!(metrics.current, 1);
    assert_eq!(metrics.reserved, 0);
    dispose(&fixture, target).await;
}

#[tokio::test]
async fn abandoned_portable_property_finishes_then_disposes_its_unobserved_result() {
    // Arrange
    let fixture = fixture(2);
    let target = target(&fixture).await;
    property_response(&fixture);
    let gate = Arc::new(tokio::sync::Notify::new());
    fixture.primary.set_evaluate_gate(Some(gate.clone()));
    let continuation = CleanupQueue::new(1);
    let service = ContinuableHandleEvaluationService::new(
        fixture.installation.raw_service(),
        fixture.installation.lifecycle().dispose_service(),
        continuation.clone(),
    );
    let mut operation = Box::pin(service.call(command(target)));
    assert!(poll_now(operation.as_mut()).is_pending());

    // Act
    drop(operation);
    fixture.primary.set_evaluate_gate(None);
    gate.notify_waiters();
    let cleanup = continuation.try_take().unwrap().unwrap();
    let result = cleanup.await;

    // Assert
    result.unwrap();
    assert_eq!(
        releases(&fixture),
        ["property-snapshot", "property-promise"]
    );
    assert!(!fixture.runtime.is_object_live(ObjectId::new(11)).unwrap());
    assert!(fixture.runtime.is_object_live(target).unwrap());
    let metrics = fixture.installation.lifecycle().routes().metrics().unwrap();
    assert_eq!(metrics.current, 1);
    assert_eq!(metrics.reserved, 0);
    dispose(&fixture, target).await;
}

#[tokio::test]
async fn abandoned_portable_property_reports_refused_native_compensation() {
    // Arrange
    let fixture = fixture(2);
    let target = target(&fixture).await;
    property_response(&fixture);
    let gate = Arc::new(tokio::sync::Notify::new());
    fixture.primary.set_evaluate_gate(Some(gate.clone()));
    fixture
        .primary
        .set_release_object_error(Some("injected release refusal".into()));
    let continuation = CleanupQueue::new(1);
    let service = ContinuableHandleEvaluationService::new(
        fixture.installation.raw_service(),
        fixture.installation.lifecycle().dispose_service(),
        continuation.clone(),
    );
    let mut operation = Box::pin(service.call(command(target)));
    assert!(poll_now(operation.as_mut()).is_pending());

    // Act
    drop(operation);
    fixture.primary.set_evaluate_gate(None);
    gate.notify_waiters();
    let cleanup = continuation.try_take().unwrap().unwrap();
    let error = cleanup.await.unwrap_err();

    // Assert
    assert!(error.to_string().contains("failed native compensation"));
    assert_eq!(
        releases(&fixture),
        ["property-snapshot", "property-promise"]
    );
    let metrics = fixture.installation.lifecycle().routes().metrics().unwrap();
    assert_eq!(metrics.current, 1);
    assert_eq!(metrics.reserved, 0);
    fixture.primary.set_release_object_error(None);
    dispose(&fixture, target).await;
}
