use super::*;
use pwright_cap_evaluation::EvaluationValueLimits;
use pwright_composition_chromium_locator_resolution::{
    CdpLocatorResolutionService, ContinuableResolutionService, ResolutionProfile, ResolutionRequest,
};
use pwright_core::{AtomicObjectIdSource, ResourceLimits};
use pwright_fake::FakeCdpClient;
use pwright_runtime::{CleanupQueue, CleanupQueueReservation};
use pwright_selector_model::Selector;
use serde_json::json;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use tokio::sync::Notify;

mod preparation;
mod retirement;

type Source = OwnedElement<CleanupQueueReservation>;
type Prepared = MainWorldElement<CleanupQueueReservation, CleanupQueueReservation>;
struct Fixture {
    client: Arc<FakeCdpClient>,
    resolve: ContinuableResolutionService<CdpLocatorResolutionService, CleanupQueue>,
    service: LocatorElementEvaluationService<CleanupQueue>,
    resources: ProductionResourceLedger,
    queue: CleanupQueue,
}
pub(super) fn limits() -> CdpEvaluationLimits {
    CdpEvaluationLimits::new(
        4096,
        128 * 1024,
        1024,
        EvaluationValueLimits::new(16, 256, 256, 8192, 8192).unwrap(),
    )
}
pub(super) fn resources(required: ResourceRequirements) -> ProductionResourceLedger {
    let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
    for class in ResourceClass::ALL {
        let bound = required.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    ProductionResourceLedger::new(limits, required).unwrap()
}
impl Fixture {
    fn new(queue_size: usize) -> Self {
        Self::with_resources(queue_size, ResourceRequirements::none())
    }
    fn with_resources(queue_size: usize, extra: ResourceRequirements) -> Self {
        let resolution = ResolutionProfile::new(ResourceScope::extension(91, 1), 2).unwrap();
        let evaluation =
            ElementEvaluationProfile::new(ResourceScope::extension(91, 2), 1, limits()).unwrap();
        let resources = resources(
            resolution
                .requirements()
                .combine_concurrent(evaluation.requirements())
                .combine_concurrent(extra),
        );
        let queue = CleanupQueue::new(queue_size);
        let raw = CdpLocatorResolutionService::new(
            resolution,
            &resources,
            Arc::new(AtomicObjectIdSource::new(1).unwrap()),
        )
        .unwrap();
        let service =
            LocatorElementEvaluationService::new(evaluation, &resources, queue.clone()).unwrap();
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_object_backend_node("utility-node", 71);
        client.set_adopted_node_response(
            71,
            29,
            json!({"object":{"type":"object","subtype":"node","objectId":"main-node"}}),
        );
        Self {
            client,
            resolve: ContinuableResolutionService::new(Arc::new(raw), queue.clone()),
            service,
            resources,
            queue,
        }
    }
    async fn source(&self) -> Source {
        self.client.set_evaluate_response(
            json!({"result":{"type":"object","subtype":"node","objectId":"utility-node"}}),
        );
        self.resolve
            .call(ResolutionRequest {
                client: self.client.clone(),
                utility_world: "utility".into(),
                selector: Selector::css("button").unwrap(),
                destruction: None,
            })
            .await
            .unwrap()
            .unwrap()
    }
    fn route(&self) -> MainWorldRoute {
        MainWorldRoute {
            client: self.client.clone(),
            execution_context_id: 29,
            unique_context_id: "main-unique".into(),
        }
    }
    async fn prepared(&self) -> Prepared {
        let source = self.source().await;
        self.service.adopt(source, self.route()).await.unwrap()
    }
    async fn drain(&self) {
        while let Some(job) = self.queue.try_take().unwrap() {
            job.await.unwrap();
        }
    }
    fn value(&self) {
        self.client.set_evaluate_response(
            json!({"result":{"value":{"ok":true,"value":{"t":"str","v":"done"}}}}),
        );
    }
    fn idle(&self) {
        assert_eq!(
            self.resources.metrics().unwrap().active_leases,
            1,
            "only installed admission state survives"
        );
        assert_eq!(self.queue.metrics().unwrap().reserved, 0);
        assert_eq!(self.queue.metrics().unwrap().queued, 0);
    }
}
fn poll<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}
fn params() -> EvaluateParams {
    EvaluateParams::function(
        "(element, suffix) => element.id + suffix",
        EvaluationValue::String("!".into()),
    )
}

#[tokio::test]
async fn adoption_joins_the_source_group_and_does_not_execute_user_code() {
    // Arrange
    let f = Fixture::new(4);
    let source = f.source().await;
    let group = source.element().object_group().to_owned();
    // Act
    let prepared = f.service.adopt(source, f.route()).await.unwrap();
    // Assert
    assert_eq!(
        f.client.method_names(),
        [
            "Runtime.callFunctionOn",
            "DOM.describeNode",
            "DOM.resolveNode"
        ]
    );
    assert_eq!(
        f.client.calls()[2].1[0],
        json!({"backendNodeId":71,"executionContextId":29,"objectGroup":group})
    );
    assert_eq!(f.resources.metrics().unwrap().active_leases, 3);
    prepared.release().await.unwrap();
    assert_eq!(
        f.client.method_names().last().unwrap(),
        "Runtime.releaseObjectGroup"
    );
    f.idle();
}

#[tokio::test]
async fn evaluation_uses_main_unique_world_and_native_receiver_without_a_portable_id() {
    // Arrange
    let f = Fixture::new(4);
    let prepared = f.prepared().await;
    f.value();
    // Act
    let value = prepared.evaluate(&params()).await.unwrap();
    // Assert
    assert_eq!(value, EvaluationValue::String("done".into()));
    let calls = f.client.calls();
    assert_eq!(calls.len(), 5);
    assert_eq!(calls[3].1[0]["uniqueContextId"], "main-unique");
    assert_eq!(
        calls[3].1[0]["arguments"][1],
        json!({"objectId":"main-node"})
    );
    assert_eq!(calls[3].1[0]["arguments"][0]["value"]["receiverSlot"], 0);
    assert_eq!(calls[4].0, "Runtime.releaseObjectGroup");
    f.idle();
}

#[tokio::test]
async fn invalid_main_route_releases_source_without_describe_or_resolve() {
    // Arrange
    let f = Fixture::new(4);
    let source = f.source().await;
    let mut route = f.route();
    route.execution_context_id = 0;
    // Act
    let result = f.service.adopt(source, route).await;
    // Assert
    assert!(matches!(result, Err(ElementEvaluationError::Setup(_))));
    assert_eq!(
        f.client.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
    );
    f.idle();
}

#[tokio::test]
async fn cross_session_route_is_rejected_before_native_adoption() {
    // Arrange
    let f = Fixture::new(4);
    let source = f.source().await;
    let mut route = f.route();
    route.client = Arc::new(FakeCdpClient::from_html(""));
    // Act
    let result = f.service.adopt(source, route).await;
    // Assert
    assert!(matches!(result, Err(ElementEvaluationError::Setup(_))));
    assert_eq!(f.client.calls().len(), 2);
    f.idle();
}

#[tokio::test]
async fn missing_continuation_capacity_never_starts_adoption() {
    // Arrange
    let f = Fixture::new(1);
    let source = f.source().await;
    // Act
    let result = f.service.adopt(source, f.route()).await;
    // Assert
    assert!(matches!(result, Err(ElementEvaluationError::Admission(_))));
    assert_eq!(
        f.client.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
    );
    f.idle();
}

#[tokio::test]
async fn cloned_installation_cannot_overbook_prepared_element_capacity() {
    // Arrange
    let f = Fixture::new(4);
    let first = f.prepared().await;
    let second = f.source().await;
    let before = f.client.calls().len();
    let clone = f.service.clone();
    // Act
    let result = clone.adopt(second, f.route()).await;
    // Assert
    assert!(matches!(result, Err(ElementEvaluationError::Admission(_))));
    assert_eq!(f.client.calls().len(), before + 1);
    first.release().await.unwrap();
    f.prepared().await.release().await.unwrap();
    f.idle();
}

#[tokio::test]
async fn unpolled_adoption_drop_cleans_only_the_already_selected_group() {
    // Arrange
    let f = Fixture::new(4);
    let source = f.source().await;
    let adoption = f.service.adopt(source, f.route());
    // Act
    drop(adoption);
    f.drain().await;
    // Assert
    assert_eq!(
        f.client.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
    );
    assert!(f.service.is_cleanup_healthy());
    f.idle();
}

#[tokio::test]
async fn dropped_started_adoption_continues_the_same_native_call_then_releases_group() {
    // Arrange
    let f = Fixture::new(4);
    let source = f.source().await;
    let gate = Arc::new(Notify::new());
    f.client.set_adoption_gate(Some(gate.clone()));
    let mut adoption = Box::pin(f.service.adopt(source, f.route()));
    assert!(poll(adoption.as_mut()).is_pending());
    // Act
    drop(adoption);
    gate.notify_waiters();
    f.drain().await;
    // Assert
    assert_eq!(
        f.client.method_names(),
        [
            "Runtime.callFunctionOn",
            "DOM.describeNode",
            "DOM.resolveNode",
            "Runtime.releaseObjectGroup"
        ]
    );
    assert!(f.service.is_cleanup_healthy());
    f.idle();
}

#[tokio::test]
async fn dropping_the_prepared_owner_releases_both_worlds_without_runtime_evaluation() {
    // Arrange
    let f = Fixture::new(4);
    let prepared = f.prepared().await;
    // Act
    drop(prepared);
    f.drain().await;
    // Assert
    assert_eq!(
        f.client.method_names(),
        [
            "Runtime.callFunctionOn",
            "DOM.describeNode",
            "DOM.resolveNode",
            "Runtime.releaseObjectGroup"
        ]
    );
    f.idle();
}

#[tokio::test]
async fn unpolled_evaluation_drop_never_dispatches_the_user_function() {
    // Arrange
    let f = Fixture::new(4);
    let evaluation = f.prepared().await.evaluate(&params());
    // Act
    drop(evaluation);
    f.drain().await;
    // Assert
    assert_eq!(
        f.client
            .method_names()
            .iter()
            .filter(|method| *method == "Runtime.callFunctionOn")
            .count(),
        1
    );
    f.idle();
}

#[tokio::test]
async fn dropped_started_evaluation_finishes_once_without_requery_or_replay() {
    // Arrange
    let f = Fixture::new(4);
    let prepared = f.prepared().await;
    f.value();
    let gate = Arc::new(Notify::new());
    f.client.set_evaluate_gate(Some(gate.clone()));
    let mut evaluation = Box::pin(prepared.evaluate(&params()));
    assert!(poll(evaluation.as_mut()).is_pending());
    // Act
    drop(evaluation);
    gate.notify_waiters();
    f.drain().await;
    // Assert
    assert_eq!(
        f.client.method_names(),
        [
            "Runtime.callFunctionOn",
            "DOM.describeNode",
            "DOM.resolveNode",
            "Runtime.callFunctionOn",
            "Runtime.releaseObjectGroup"
        ]
    );
    f.idle();
}

#[tokio::test]
async fn cancelled_evaluation_retains_ownership_until_the_native_wait_settles() {
    // Arrange
    let f = Fixture::new(4);
    let prepared = f.prepared().await;
    f.value();
    let gate = Arc::new(Notify::new());
    f.client.set_evaluate_gate(Some(gate.clone()));
    let mut evaluation = Box::pin(prepared.evaluate(&params()));
    assert!(poll(evaluation.as_mut()).is_pending());
    let retained = f.resources.metrics().unwrap().active_leases;

    // Act
    drop(evaluation);
    let mut cleanup = f.queue.try_take().unwrap().unwrap();

    // Assert: cancellation cannot silently return native capacity or report a
    // group release while the same native command is still outstanding.
    assert!(poll(Pin::new(&mut cleanup)).is_pending());
    assert_eq!(f.resources.metrics().unwrap().active_leases, retained);
    assert!(
        !f.client
            .method_names()
            .iter()
            .any(|method| method == "Runtime.releaseObjectGroup")
    );
    gate.notify_waiters();
    cleanup.await.unwrap();
    assert!(f.service.is_cleanup_healthy());
    f.idle();
}

#[tokio::test]
async fn cancelled_started_group_release_is_not_reissued_and_retains_native_budget() {
    // Arrange
    let f = Fixture::new(4);
    let prepared = f.prepared().await;
    let gate = Arc::new(Notify::new());
    f.client.set_release_object_group_gate(Some(gate.clone()));
    let mut release = Box::pin(prepared.release());
    assert!(poll(release.as_mut()).is_pending());
    assert_eq!(f.resources.metrics().unwrap().active_leases, 3);
    // Act
    drop(release);
    gate.notify_waiters();
    f.drain().await;
    // Assert
    assert_eq!(
        f.client
            .method_names()
            .iter()
            .filter(|method| *method == "Runtime.releaseObjectGroup")
            .count(),
        1
    );
    f.idle();
}

#[tokio::test]
async fn malformed_adoption_without_an_id_still_releases_the_whole_group() {
    // Arrange
    let f = Fixture::new(4);
    let source = f.source().await;
    f.client.set_adopted_node_response(
        71,
        29,
        json!({"object":{"type":"object","subtype":"node"}}),
    );
    // Act
    let result = f.service.adopt(source, f.route()).await;
    // Assert
    assert!(matches!(result, Err(ElementEvaluationError::Backend(_))));
    assert_eq!(
        f.client.method_names().last().unwrap(),
        "Runtime.releaseObjectGroup"
    );
    assert!(f.service.is_cleanup_healthy());
    f.idle();
}

#[tokio::test]
async fn javascript_failure_keeps_structured_details_after_successful_cleanup() {
    // Arrange
    let f = Fixture::new(4);
    let prepared = f.prepared().await;
    f.client.set_evaluate_response(json!({"result":{"value":{"ok":false,"kind":"javascript","error":{"name":"TypeError","message":"user failure","stack":"browser stack"}}}}));
    // Act
    let error = prepared.evaluate(&params()).await.unwrap_err();
    // Assert
    assert!(
        matches!(error, ElementEvaluationError::Backend(error) if matches!(error.details(), Some(HandleEvaluationErrorDetails::Javascript {name, message, stack: Some(stack)}) if name == "TypeError" && message == "user failure" && stack == "browser stack"))
    );
    assert!(f.service.is_cleanup_healthy());
    f.idle();
}

#[tokio::test]
async fn cleanup_failure_retains_primary_error_and_closes_admission() {
    // Arrange
    let f = Fixture::new(4);
    let prepared = f.prepared().await;
    f.client.set_evaluate_errors(["evaluation failed".into()]);
    f.client
        .set_release_object_error(Some("cleanup failed".into()));
    // Act
    let error = prepared.evaluate(&params()).await.unwrap_err();
    // Assert
    assert!(
        matches!(error, ElementEvaluationError::Cleanup { primary: Some(primary), .. } if matches!(*primary, ElementEvaluationError::Backend(_)))
    );
    assert!(!f.service.is_cleanup_healthy());
    assert!(!f.service.clone().is_cleanup_healthy());
    f.idle();
}

#[tokio::test]
async fn abandoned_failed_cleanup_is_reported_to_the_executor() {
    // Arrange
    let f = Fixture::new(4);
    let prepared = f.prepared().await;
    f.client
        .set_release_object_error(Some("release failed".into()));
    // Act
    drop(prepared);
    let result = f.queue.try_take().unwrap().unwrap().await;
    // Assert
    assert!(result.is_err());
    assert_eq!(f.queue.metrics().unwrap().failed, 1);
    assert!(!f.service.is_cleanup_healthy());
    f.idle();
}

#[tokio::test]
async fn discarded_executor_job_fails_closed_even_if_original_group_cleanup_later_succeeds() {
    // Arrange
    let f = Fixture::new(4);
    drop(f.prepared().await);
    let job = f.queue.try_take().unwrap().unwrap();
    // Act
    drop(job);
    f.drain().await;
    // Assert
    assert!(!f.service.is_cleanup_healthy());
    f.idle();
}

#[tokio::test]
async fn oversized_source_is_rejected_before_any_user_runtime_call() {
    // Arrange
    let f = Fixture::new(4);
    let prepared = f.prepared().await;
    let params = EvaluateParams::expression("a".repeat(limits().source_bytes + 1));
    // Act
    let result = prepared.evaluate(&params).await;
    // Assert
    assert!(matches!(result, Err(ElementEvaluationError::Admission(_))));
    assert_eq!(f.client.calls().len(), 4);
    f.idle();
}

#[tokio::test]
async fn bounded_input_is_snapshotted_before_the_returned_future_is_polled() {
    // Arrange
    let f = Fixture::new(4);
    let prepared = f.prepared().await;
    f.value();
    let mut argument = params();
    // Act
    let evaluation = prepared.evaluate(&argument);
    argument = EvaluateParams::expression("replacement");
    evaluation.await.unwrap();
    // Assert
    assert_eq!(argument.source(), "replacement");
    assert_eq!(
        f.client.calls()[3].1[0]["arguments"][0]["value"]["source"],
        "(element, suffix) => element.id + suffix"
    );
    f.idle();
}

#[test]
fn profiles_reject_zero_overflow_and_undeclared_resources() {
    // Arrange
    let scope = ResourceScope::extension(91, 2);
    // Act
    let zero = ElementEvaluationProfile::new(scope, 0, limits());
    let overflow = ElementEvaluationProfile::new(scope, usize::MAX, limits());
    let unplanned = LocatorElementEvaluationService::new(
        ElementEvaluationProfile::new(scope, 1, limits()).unwrap(),
        &resources(ResourceRequirements::none()),
        CleanupQueue::new(2),
    );
    // Assert
    assert!(matches!(zero, Err(ElementEvaluationError::Setup(_))));
    assert!(matches!(overflow, Err(ElementEvaluationError::Setup(_))));
    assert!(matches!(
        unplanned,
        Err(ElementEvaluationError::Production(_))
    ));
}

#[tokio::test]
async fn oversized_argument_is_rejected_without_cloning_or_user_runtime_io() {
    // Arrange
    let f = Fixture::new(4);
    let prepared = f.prepared().await;
    let params = EvaluateParams::function(
        "element => element.id",
        EvaluationValue::String("x".repeat(limits().value.max_string_bytes() + 1)),
    );
    // Act
    let result = prepared.evaluate(&params).await;
    // Assert
    assert!(matches!(result, Err(ElementEvaluationError::Admission(_))));
    assert_eq!(f.client.calls().len(), 4);
    f.idle();
}

#[tokio::test]
async fn abandoned_adoption_cleanup_failure_is_not_swallowed() {
    // Arrange
    let f = Fixture::new(4);
    let source = f.source().await;
    let gate = Arc::new(Notify::new());
    f.client.set_adoption_gate(Some(gate.clone()));
    f.client.set_adopted_node_response(71, 29, json!({}));
    let mut adoption = Box::pin(f.service.adopt(source, f.route()));
    assert!(poll(adoption.as_mut()).is_pending());
    f.client
        .set_release_object_error(Some("group release failed".into()));
    // Act
    drop(adoption);
    gate.notify_waiters();
    let result = f.queue.try_take().unwrap().unwrap().await;
    // Assert
    assert!(result.is_err());
    assert!(!f.service.is_cleanup_healthy());
    assert_eq!(f.queue.metrics().unwrap().failed, 1);
    f.idle();
}

#[tokio::test]
async fn raw_group_request_validation_occurs_before_any_describe_or_resolve() {
    // Arrange
    let f = Fixture::new(4);
    let source = f.source().await;
    let request = CdpElementHandleAdoptionRequest::from_element_reference(
        source.element().client(),
        source.element().object_id(),
        f.client.clone(),
        29,
        NATIVE_BYTES,
    )
    .unwrap()
    .with_object_group(Arc::from("g".repeat(NATIVE_BYTES + 1)));
    let service = CdpElementHandleAdoptionService::new(CdpHandleEvaluationLimits::new(
        limits(),
        NATIVE_BYTES,
        0,
    ));
    // Act
    let result = service.call(request).await;
    // Assert
    assert!(result.is_err());
    assert_eq!(f.client.calls().len(), 1);
    source.release().await.unwrap();
    f.idle();
}

#[tokio::test]
async fn all_owned_state_is_reclaimed_after_the_last_installation_clone() {
    // Arrange
    let f = Fixture::new(4);
    let resources = f.resources.clone();
    let clone = f.service.clone();
    f.prepared().await.release().await.unwrap();
    // Act
    drop(f);
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    drop(clone);
    // Assert
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}
