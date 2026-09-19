use super::*;
use pwright_core::ResourceMetrics;

fn released_ids(fixture: &Fixture) -> Vec<String> {
    fixture
        .primary
        .calls()
        .into_iter()
        .filter(|(method, _)| method == "Runtime.releaseObject")
        .map(|(_, arguments)| arguments[0].as_str().unwrap().to_owned())
        .collect()
}

fn assert_resources_at(fixture: &Fixture, expected: ResourceMetrics) {
    let current = fixture.resources.metrics().unwrap();
    assert_eq!(current.active_leases, expected.active_leases);
    for class in ResourceClass::ALL {
        assert_eq!(current.class(class).current, expected.class(class).current);
    }
}

fn assert_routes(fixture: &Fixture, current: usize, reserved: usize) {
    let metrics = fixture.installation.lifecycle().routes().metrics().unwrap();
    assert_eq!(metrics.current, current);
    assert_eq!(metrics.reserved, reserved);
}

async fn target(fixture: &Fixture) -> ObjectId {
    fixture.primary.set_evaluate_response(serde_json::json!({
        "result": {"type": "object", "objectId": "property-target"}
    }));
    let target = fixture
        .installation
        .creation_service()
        .call(Command::<EvaluateHandle>::new(
            CallId::new(91),
            ObjectId::new(4),
            CallMetadata::default(),
            EvaluateHandleParams::expression("({})"),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    while fixture.runtime.pop_control_message().unwrap().is_some() {}
    target.object_id
}

async fn dispose(fixture: &Fixture, target: ObjectId) {
    fixture
        .installation
        .lifecycle()
        .raw_dispose_service()
        .call(Command::<HandleDispose>::new(
            CallId::new(93),
            target,
            CallMetadata::default(),
            (),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
}

fn property(target: ObjectId) -> Command<HandleGetProperty> {
    Command::new(
        CallId::new(92),
        target,
        CallMetadata::default(),
        GetPropertyParams::new("promise"),
    )
}

#[tokio::test]
async fn abandoned_frame_acquisition_keeps_admission_through_malformed_result_cleanup() {
    // Arrange a malformed but releasable late result and two independent reply
    // gates: acquisition, then compensation of the acquired native identity.
    let fixture = fixture(1);
    let baseline = fixture.resources.metrics().unwrap();
    fixture.primary.set_evaluate_response(serde_json::json!({
        "result": {"objectId": "late-malformed-result"}
    }));
    let acquisition_gate = Arc::new(tokio::sync::Notify::new());
    let release_gate = Arc::new(tokio::sync::Notify::new());
    fixture
        .primary
        .set_evaluate_gate(Some(acquisition_gate.clone()));
    fixture
        .primary
        .set_release_object_gate(Some(release_gate.clone()));
    let continuation = CleanupQueue::new(1);
    let service = ContinuableHandleEvaluationService::new(
        fixture.installation.raw_service(),
        fixture.installation.lifecycle().dispose_service(),
        continuation.clone(),
    );
    let mut operation = Box::pin(service.call(Command::<EvaluateHandle>::new(
        CallId::new(90),
        ObjectId::new(4),
        CallMetadata::default(),
        EvaluateHandleParams::expression("({})"),
    )));
    assert!(poll_now(operation.as_mut()).is_pending());
    let admitted = fixture.resources.metrics().unwrap();
    assert_eq!(admitted.active_leases, baseline.active_leases + 2);
    assert_routes(&fixture, 0, 1);

    // Act: abandon acquisition, let the late response arrive, and keep the
    // continuation paused until its native cleanup acknowledgement arrives.
    drop(operation);
    let mut cleanup = continuation.try_take().unwrap().unwrap();
    assert!(poll_now(cleanup.as_mut()).is_pending());
    assert_resources_at(&fixture, admitted);
    fixture.primary.set_evaluate_gate(None);
    acquisition_gate.notify_waiters();
    assert!(poll_now(cleanup.as_mut()).is_pending());

    // Assert both the native-call lease and persistent result reservation still
    // exist while compensation is unresolved; no portable object was published.
    assert_resources_at(&fixture, admitted);
    assert_routes(&fixture, 0, 1);
    assert_eq!(released_ids(&fixture), ["late-malformed-result"]);
    assert!(!fixture.runtime.is_object_live(ObjectId::new(10)).unwrap());
    assert!(fixture.runtime.pop_control_message().unwrap().is_none());
    let health = continuation.metrics().unwrap();
    assert_eq!(health.accepted, 1);
    assert_eq!(health.completed, 0);
    assert_eq!(health.failed, 0);

    fixture.primary.set_release_object_gate(None);
    release_gate.notify_waiters();
    cleanup.await.unwrap();
    assert_resources_at(&fixture, baseline);
    assert_routes(&fixture, 0, 0);
    let health = continuation.metrics().unwrap();
    assert_eq!(health.completed, 1);
    assert_eq!(health.failed, 0);
}

#[tokio::test]
async fn abandoned_property_acquisition_cleanup_attempts_both_ids_and_reports_failure() {
    // Arrange a late carrier response whose result and thrown exception each
    // own an independent native ID. Every release refuses acknowledgement.
    let fixture = fixture(2);
    let target = target(&fixture).await;
    let baseline = fixture.resources.metrics().unwrap();
    fixture.primary.set_evaluate_response(serde_json::json!({
        "result": {"objectId": "malformed-carrier"},
        "exceptionDetails": {
            "text": "injected acquisition failure",
            "exception": {"type": "object", "objectId": "acquisition-exception"}
        }
    }));
    let acquisition_gate = Arc::new(tokio::sync::Notify::new());
    let release_gate = Arc::new(tokio::sync::Notify::new());
    fixture
        .primary
        .set_evaluate_gate(Some(acquisition_gate.clone()));
    fixture
        .primary
        .set_release_object_gate(Some(release_gate.clone()));
    fixture
        .primary
        .set_release_object_error(Some("injected release refusal".into()));
    let continuation = CleanupQueue::new(1);
    let service = ContinuableHandleEvaluationService::new(
        fixture.installation.raw_service(),
        fixture.installation.lifecycle().dispose_service(),
        continuation.clone(),
    );
    let mut operation = Box::pin(service.call(property(target)));
    assert!(poll_now(operation.as_mut()).is_pending());
    let admitted = fixture.resources.metrics().unwrap();
    assert_eq!(admitted.active_leases, baseline.active_leases + 2);

    // Act: transfer started acquisition to its reserved continuation, then
    // unblock the response while the first release acknowledgement is pending.
    drop(operation);
    fixture.primary.set_evaluate_gate(None);
    acquisition_gate.notify_waiters();
    let mut cleanup = continuation.try_take().unwrap().unwrap();
    assert!(poll_now(cleanup.as_mut()).is_pending());
    assert_resources_at(&fixture, admitted);
    assert_routes(&fixture, 1, 1);
    assert_eq!(released_ids(&fixture).len(), 1);
    fixture.primary.set_release_object_gate(None);
    release_gate.notify_waiters();
    let error = cleanup.await.unwrap_err();

    // Assert one failed release did not suppress the other ID; cleanup failure
    // reaches executor health rather than becoming an ordinary discarded error.
    assert!(error.message().contains("failed native compensation"));
    let mut released = released_ids(&fixture);
    released.sort();
    assert_eq!(released, ["acquisition-exception", "malformed-carrier"]);
    assert_resources_at(&fixture, baseline);
    assert_routes(&fixture, 1, 0);
    assert!(fixture.runtime.is_object_live(target).unwrap());
    assert!(!fixture.runtime.is_object_live(ObjectId::new(11)).unwrap());
    assert!(fixture.runtime.pop_control_message().unwrap().is_none());
    assert!(
        !fixture
            .primary
            .method_names()
            .iter()
            .any(|name| name == "Runtime.getProperties")
    );
    let health = continuation.metrics().unwrap();
    assert_eq!(health.accepted, 1);
    assert_eq!(health.completed, 0);
    assert_eq!(health.failed, 1);
    fixture.primary.set_release_object_error(None);
    dispose(&fixture, target).await;
}

#[tokio::test]
async fn abandoned_property_snapshot_release_preserves_admission_and_revalidates_source() {
    // Arrange a valid carrier and property, paused at carrier release rather
    // than acquisition. The source changes while no portable result exists.
    let fixture = fixture(2);
    let target = target(&fixture).await;
    let baseline = fixture.resources.metrics().unwrap();
    fixture.primary.set_evaluate_responses([
        serde_json::json!({
            "result": {"type": "object", "objectId": "property-carrier"}
        }),
        serde_json::json!({
            "result": [{"name": "promise", "enumerable": true, "value": {
                "type": "object", "subtype": "promise", "objectId": "property-value"
            }}]
        }),
    ]);
    let release_gate = Arc::new(tokio::sync::Notify::new());
    fixture
        .primary
        .set_release_object_gate(Some(release_gate.clone()));
    let continuation = CleanupQueue::new(1);
    let service = ContinuableHandleEvaluationService::new(
        fixture.installation.raw_service(),
        fixture.installation.lifecycle().dispose_service(),
        continuation.clone(),
    );
    let mut operation = Box::pin(service.call(property(target)));
    assert!(poll_now(operation.as_mut()).is_pending());
    let admitted = fixture.resources.metrics().unwrap();
    // The same evaluation lease now spans acquisition, property retrieval and
    // carrier release in addition to the persistent result route reservation.
    assert_eq!(admitted.active_leases, baseline.active_leases + 2);
    assert_eq!(released_ids(&fixture), ["property-carrier"]);

    // Act: abandon during cleanup and replace the Frame's source world before
    // resuming. Compensation must continue on the original native connection.
    drop(operation);
    let mut cleanup = continuation.try_take().unwrap().unwrap();
    assert!(poll_now(cleanup.as_mut()).is_pending());
    assert_resources_at(&fixture, admitted);
    assert_routes(&fixture, 1, 1);
    let routes = fixture.installation.routes();
    routes.unbind(ObjectId::new(4)).unwrap();
    routes
        .bind(
            ObjectId::new(4),
            ObjectId::new(3),
            fixture.secondary.clone(),
            201,
            "world-3",
        )
        .unwrap();
    fixture.primary.set_release_object_gate(None);
    release_gate.notify_waiters();
    cleanup.await.unwrap();

    // Assert both native objects were compensated on their original session;
    // the stale source cannot publish, and no resource ownership remains added.
    assert_eq!(
        released_ids(&fixture),
        ["property-carrier", "property-value"]
    );
    assert!(fixture.secondary.calls().is_empty());
    assert_resources_at(&fixture, baseline);
    assert_routes(&fixture, 1, 0);
    assert!(!fixture.runtime.is_object_live(ObjectId::new(11)).unwrap());
    assert!(fixture.runtime.pop_control_message().unwrap().is_none());
    let health = continuation.metrics().unwrap();
    assert_eq!(health.completed, 1);
    assert_eq!(health.failed, 0);
    dispose(&fixture, target).await;
}

#[tokio::test]
async fn abandoned_property_read_retains_pipeline_admission_until_every_failed_release_settles() {
    // Arrange: acquisition succeeds, but property retrieval and subsequent
    // cleanup have separate deterministic gates. Both response IDs are owned
    // even when JavaScript failure makes the response unpublishable.
    let fixture = fixture(2);
    let target = target(&fixture).await;
    let baseline = fixture.resources.metrics().unwrap();
    fixture.primary.set_evaluate_responses([
        serde_json::json!({"result": {"type": "object", "objectId": "read-carrier"}}),
        serde_json::json!({
            "result": [{"name": "promise", "enumerable": true,
                "value": {"type": "object", "objectId": "read-value"}}],
            "exceptionDetails": {"text": "property read failed", "exception": {
                "type": "object", "objectId": "read-exception"
            }}
        }),
    ]);
    let read_gate = Arc::new(tokio::sync::Notify::new());
    let release_gate = Arc::new(tokio::sync::Notify::new());
    fixture
        .primary
        .set_get_properties_gate(Some(read_gate.clone()));
    fixture
        .primary
        .set_release_object_gate(Some(release_gate.clone()));
    fixture
        .primary
        .set_release_object_error(Some("release failed".into()));
    let continuation = CleanupQueue::new(1);
    let service = ContinuableHandleEvaluationService::new(
        fixture.installation.raw_service(),
        fixture.installation.lifecycle().dispose_service(),
        continuation.clone(),
    );
    let mut operation = Box::pin(service.call(property(target)));
    assert!(poll_now(operation.as_mut()).is_pending());
    let admitted = fixture.resources.metrics().unwrap();
    assert_eq!(admitted.active_leases, baseline.active_leases + 2);
    assert_eq!(
        fixture.primary.calls().last().unwrap().0,
        "Runtime.getProperties"
    );

    // Act: abandonment transfers the whole in-flight property pipeline; neither
    // the result route slot nor evaluation allowance is returned while blocked.
    drop(operation);
    let mut cleanup = continuation.try_take().unwrap().unwrap();
    assert!(poll_now(cleanup.as_mut()).is_pending());
    assert_resources_at(&fixture, admitted);
    assert_routes(&fixture, 1, 1);
    fixture.primary.set_get_properties_gate(None);
    read_gate.notify_waiters();
    assert!(poll_now(cleanup.as_mut()).is_pending());
    assert_resources_at(&fixture, admitted);
    assert_eq!(released_ids(&fixture), ["read-value"]);
    fixture.primary.set_release_object_gate(None);
    release_gate.notify_waiters();
    let result = cleanup.await;

    // Assert: no early error skips later owners, no native result is published,
    // and failed compensation reaches continuation health rather than silent Ok.
    assert!(result.is_err());
    assert_eq!(
        released_ids(&fixture),
        ["read-value", "read-exception", "read-carrier"]
    );
    assert_resources_at(&fixture, baseline);
    assert_routes(&fixture, 1, 0);
    assert!(fixture.runtime.pop_control_message().unwrap().is_none());
    assert_eq!(continuation.metrics().unwrap().failed, 1);
    fixture.primary.set_release_object_error(None);
    dispose(&fixture, target).await;
}
