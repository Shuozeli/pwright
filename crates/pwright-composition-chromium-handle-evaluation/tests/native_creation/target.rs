use super::*;
use pwright_composition_chromium_handle_evaluation::CdpHandleEvaluationProgram;

async fn target(fixture: &Fixture) -> ObjectId {
    let NativeHandleCreationCommit::Published(handle) = staged(fixture).await.commit_or_defer()
    else {
        panic!("target did not publish")
    };
    fixture.primary.set_evaluate_response(serde_json::json!({
        "result": {"type": "object", "objectId": "result-native", "description": "result"}
    }));
    handle.object_id
}

pub(super) async fn dispose(fixture: &Fixture, object: ObjectId) {
    fixture
        .installation
        .lifecycle()
        .raw_dispose_service()
        .call(Command::<HandleDispose>::new(
            CallId::new(81),
            object,
            CallMetadata::default(),
            (),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
}

#[tokio::test]
async fn staged_handle_program_uses_exact_native_target_and_source_world() {
    // Arrange
    let fixture = fixture(2);
    let target = target(&fixture).await;
    let before = fixture.runtime.live_object_count().unwrap();

    // Act
    let pending = fixture
        .installation
        .raw_service()
        .stage_evaluate_program_for_handle(
            target,
            CallMetadata::default(),
            EvaluateHandleParams::function(
                "(target, alias) => ({ same: target === alias })",
                HandleEvaluationValue::Handle(target),
            )
            .into(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(fixture.runtime.live_object_count().unwrap(), before);
    assert_eq!(pending.world_identity().unique_context_id(), "world-1");
    let calls = fixture.primary.calls();
    let request = &calls.last().unwrap().1[0];
    assert_eq!(request["uniqueContextId"], "world-1");
    assert_eq!(request["arguments"][0]["value"]["receiverSlot"], 0);
    assert_eq!(request["arguments"][1]["objectId"], "staged-native");
    assert_eq!(request["arguments"][2]["objectId"], "staged-native");
    assert!(fixture.secondary.calls().is_empty());
    let NativeHandleCreationCommit::Published(result) = pending.commit_or_defer() else {
        panic!("live target result did not publish")
    };
    assert_ne!(result.object_id, target);
    assert_eq!(result.parent_id, ObjectId::new(4));
    dispose(&fixture, result.object_id).await;
    dispose(&fixture, target).await;
    let metrics = fixture.installation.lifecycle().routes().metrics().unwrap();
    assert_eq!(metrics.current, 0);
    assert_eq!(metrics.reserved, 0);
}

#[tokio::test]
async fn disposing_target_rejects_staged_expression_result_before_publication() {
    // Arrange
    let fixture = fixture(2);
    let target = target(&fixture).await;
    let pending = fixture
        .installation
        .raw_service()
        .stage_evaluate_program_for_handle(
            target,
            CallMetadata::default(),
            CdpHandleEvaluationProgram::expression(
                "({ independent: true })".into(),
                HandleEvaluationValue::Undefined,
            ),
        )
        .await
        .unwrap();
    dispose(&fixture, target).await;
    let before = fixture.primary.calls().len();

    // Act
    let result = pending.commit_or_defer();

    // Assert
    let NativeHandleCreationCommit::Rejected(failure) = result else {
        panic!("disposed target result was published")
    };
    assert_eq!(failure.error().kind(), FailureKind::TargetClosed);
    assert_eq!(fixture.primary.calls().len(), before);
    assert_eq!(failure.release().await.kind(), FailureKind::TargetClosed);
    let calls = fixture.primary.calls();
    assert_eq!(calls.last().unwrap().0, "Runtime.releaseObject");
    assert_eq!(calls.last().unwrap().1[0], "result-native");
    let metrics = fixture.installation.lifecycle().routes().metrics().unwrap();
    assert_eq!(metrics.current, 0);
    assert_eq!(metrics.reserved, 0);
}

#[tokio::test]
async fn disposing_argument_rejects_staged_result_while_target_stays_live() {
    // Arrange
    let fixture = fixture(3);
    let target = target(&fixture).await;
    fixture.primary.set_evaluate_response(serde_json::json!({
        "result": {"type": "object", "objectId": "argument-native", "description": "argument"}
    }));
    let argument = fixture
        .installation
        .raw_service()
        .stage_evaluate_handle(
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::expression("({})"),
        )
        .await
        .unwrap();
    let NativeHandleCreationCommit::Published(argument) = argument.commit_or_defer() else {
        panic!("argument did not publish")
    };
    fixture.primary.set_evaluate_response(serde_json::json!({
        "result": {"type": "object", "objectId": "result-native", "description": "result"}
    }));
    let pending = fixture
        .installation
        .raw_service()
        .stage_evaluate_program_for_handle(
            target,
            CallMetadata::default(),
            EvaluateHandleParams::function(
                "(target, argument) => ({ target, argument })",
                HandleEvaluationValue::Handle(argument.object_id),
            )
            .into(),
        )
        .await
        .unwrap();
    dispose(&fixture, argument.object_id).await;

    // Act
    let result = pending.commit_or_defer();

    // Assert
    let NativeHandleCreationCommit::Rejected(failure) = result else {
        panic!("disposed argument result was published")
    };
    assert_eq!(failure.release().await.kind(), FailureKind::TargetClosed);
    assert!(fixture.runtime.is_object_live(target).unwrap());
    let metrics = fixture.installation.lifecycle().routes().metrics().unwrap();
    assert_eq!(metrics.current, 1);
    assert_eq!(metrics.reserved, 0);
    dispose(&fixture, target).await;
}

#[tokio::test]
async fn staged_property_owns_result_independently_of_its_target_after_publication() {
    // Arrange
    let fixture = fixture(2);
    let target = target(&fixture).await;
    let before = fixture.runtime.live_object_count().unwrap();
    property_response(&fixture);

    // Act
    let pending = fixture
        .installation
        .raw_service()
        .stage_get_property(
            target,
            CallMetadata::default(),
            GetPropertyParams::new("nested"),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(fixture.runtime.live_object_count().unwrap(), before);
    let calls = fixture.primary.calls();
    let request = &calls[1].1[0];
    let source = request["arguments"][0]["value"]["source"].as_str().unwrap();
    assert!(source.contains("{ __proto__: null }"));
    assert!(source.contains("result[name] = target[name]"));
    assert_eq!(request["arguments"][0]["value"]["argument"]["v"], "nested");
    assert_eq!(request["arguments"][1]["objectId"], "staged-native");
    assert_eq!(calls[2].0, "Runtime.getProperties");
    assert_eq!(calls[3].0, "Runtime.releaseObject");
    assert_eq!(calls[3].1[0], "property-snapshot");
    let NativeHandleCreationCommit::Published(property) = pending.commit_or_defer() else {
        panic!("property did not publish")
    };
    assert_eq!(property.parent_id, ObjectId::new(4));
    assert_ne!(property.object_id, target);
    dispose(&fixture, target).await;
    assert!(fixture.runtime.is_object_live(property.object_id).unwrap());
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
    let metrics = fixture.installation.lifecycle().routes().metrics().unwrap();
    assert_eq!(metrics.current, 0);
    assert_eq!(metrics.reserved, 0);
    assert_eq!(
        fixture.primary.calls().last().unwrap().1[0],
        "result-native"
    );
}

fn property_response(fixture: &Fixture) {
    fixture.primary.set_evaluate_responses([
        serde_json::json!({
            "result": {"type": "object", "objectId": "property-snapshot", "description": "Object"}
        }),
        serde_json::json!({
            "result": [{"name": "nested", "enumerable": true, "value": {
                "type": "object", "objectId": "result-native", "description": "result"
            }}]
        }),
    ]);
}

#[tokio::test]
async fn disposing_property_target_before_publication_compensates_only_the_result() {
    // Arrange
    let fixture = fixture(2);
    let target = target(&fixture).await;
    property_response(&fixture);
    let pending = fixture
        .installation
        .raw_service()
        .stage_get_property(
            target,
            CallMetadata::default(),
            GetPropertyParams::new("nested"),
        )
        .await
        .unwrap();
    dispose(&fixture, target).await;
    let before = fixture.primary.calls().len();

    // Act
    let result = pending.commit_or_defer();

    // Assert
    let NativeHandleCreationCommit::Rejected(failure) = result else {
        panic!("disposed property target result was published")
    };
    assert_eq!(failure.error().kind(), FailureKind::TargetClosed);
    assert_eq!(fixture.primary.calls().len(), before);
    assert_eq!(failure.release().await.kind(), FailureKind::TargetClosed);
    let calls = fixture.primary.calls();
    assert_eq!(calls.len(), before + 1);
    assert_eq!(calls.last().unwrap().1[0], "result-native");
    let metrics = fixture.installation.lifecycle().routes().metrics().unwrap();
    assert_eq!(metrics.current, 0);
    assert_eq!(metrics.reserved, 0);
}

#[tokio::test]
async fn staged_property_rejects_oversized_name_before_native_admission_or_script() {
    // Arrange
    let fixture = fixture(2);
    let target = target(&fixture).await;
    let before = fixture.primary.calls().len();

    // Act
    let result = fixture
        .installation
        .raw_service()
        .stage_get_property(
            target,
            CallMetadata::default(),
            GetPropertyParams::new("x".repeat(65)),
        )
        .await;

    // Assert
    let Err(error) = result else {
        panic!("oversized property name was admitted")
    };
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert_eq!(fixture.primary.calls().len(), before);
    let metrics = fixture.installation.lifecycle().routes().metrics().unwrap();
    assert_eq!(metrics.current, 1);
    assert_eq!(metrics.reserved, 0);
    dispose(&fixture, target).await;
}

#[tokio::test]
async fn handle_result_route_capacity_is_admitted_before_native_script() {
    // Arrange
    let fixture = fixture(1);
    let target = target(&fixture).await;
    let before = fixture.primary.calls().len();

    // Act
    let result = fixture
        .installation
        .raw_service()
        .stage_evaluate_program_for_handle(
            target,
            CallMetadata::default(),
            EvaluateHandleParams::function_without_argument("target => target").into(),
        )
        .await;

    // Assert
    let Err(error) = result else {
        panic!("native result was created without route capacity")
    };
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert_eq!(fixture.primary.calls().len(), before);
    dispose(&fixture, target).await;
}
