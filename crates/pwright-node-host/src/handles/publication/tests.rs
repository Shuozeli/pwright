use super::*;
use pwright_composition_chromium_handle_evaluation::{
    CdpEvaluationLimits, CdpEvaluationProductionProfile, CdpHandlePropertyLimits,
    ChromiumHandleEvaluationBackendProfile, ChromiumHandleEvaluationCommandService,
    ChromiumHandleEvaluationRoutes,
};
use pwright_composition_chromium_handle_lifecycle::{
    ChromiumHandleDisposeCommandService, ChromiumHandleRouteProfile, ChromiumHandleRoutes,
};
use pwright_core::{AtomicObjectIdSource, CallMetadata, ObjectId, ObjectKindTag, ResourceLimits};
use pwright_fake::FakeCdpClient;
use pwright_runtime::{
    ControlledDispatcher, NoDeadlineScheduler, ReservedCleanupExecutor, RuntimeLimits,
};
use pwright_runtime_tokio::TokioCleanupExecutor;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

struct Fixture {
    runtime: ControlledDispatcher<(), (), NoDeadlineScheduler>,
    service: ChromiumHandleEvaluationCommandService,
    frames: ChromiumHandleEvaluationRoutes,
    native: Arc<FakeCdpClient>,
    routes: ChromiumHandleRoutes,
    queue: Arc<Publications>,
    releases: Arc<Releases>,
    owners: Arc<Owners>,
    cleanup: TokioCleanupExecutor,
}
fn fixture() -> Fixture {
    let limits = RuntimeLimits::new(256, 16);
    let runtime_objects = std::num::NonZeroUsize::new(limits.live_objects).unwrap();
    let evaluation = CdpEvaluationProductionProfile::new(
        ResourceScope::extension(159, 3),
        CdpEvaluationLimits::new(4096, 32768, 4096, crate::evaluation::value_limits()),
        8,
    )
    .with_context_routes(1, 128);
    let routes = ChromiumHandleRouteProfile::new(ResourceScope::extension(159, 4), 64, 1024, 1024);
    let plan = limits
        .resource_requirements()
        .combine_concurrent(evaluation.requirements())
        .combine_concurrent(routes.requirements())
        .combine_concurrent(requirements())
        .combine_concurrent(super::super::pending::requirements())
        .combine_concurrent(super::super::release::requirements(runtime_objects).unwrap());
    let mut budget = ResourceLimits::deny_all(plan.scopes, plan.leases);
    for class in ResourceClass::ALL {
        let amount = plan.class(class);
        budget = budget.with_class(class, amount.global, amount.per_scope);
    }
    let resources = ProductionResourceLedger::new(budget, plan).unwrap();
    let runtime = ControlledDispatcher::new((), (), NoDeadlineScheduler, limits)
        .with_production_resources(&resources)
        .unwrap();
    runtime
        .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
        .unwrap();
    runtime
        .register_object(
            ObjectId::new(2),
            Some(ObjectId::new(1)),
            ObjectKindTag::new(2),
        )
        .unwrap();
    runtime
        .register_object(
            ObjectId::new(3),
            Some(ObjectId::new(1)),
            ObjectKindTag::new(2),
        )
        .unwrap();
    let frames =
        ChromiumHandleEvaluationRoutes::new_with_production_resources(evaluation, &resources)
            .unwrap();
    let native = Arc::new(FakeCdpClient::from_html(""));
    frames
        .bind(
            ObjectId::new(2),
            ObjectId::new(1),
            native.clone(),
            101,
            "queue-world",
        )
        .unwrap();
    let routes = ChromiumHandleRoutes::new_with_production_resources(routes, &resources).unwrap();
    let service = ChromiumHandleEvaluationCommandService::new(
        runtime.lifecycle_handle(),
        Arc::new(AtomicObjectIdSource::new(10).unwrap()),
        frames.clone(),
        routes.clone(),
        ChromiumHandleEvaluationBackendProfile::new(
            evaluation,
            1024,
            1024,
            CdpHandlePropertyLimits::new(4, 128).unwrap(),
        ),
    );
    let releases = Releases::new(
        ChromiumHandleDisposeCommandService::new(runtime.lifecycle_handle(), routes.clone()),
        &runtime.lifecycle_handle(),
        runtime_objects,
        &resources,
    )
    .unwrap();
    let owners = Owners::new(&resources).unwrap();
    let queue = Publications::new(&resources, owners.clone()).unwrap();
    Fixture {
        runtime,
        service,
        frames,
        native,
        routes,
        queue,
        releases,
        owners,
        cleanup: TokioCleanupExecutor::new(16),
    }
}
impl Fixture {
    async fn begin(&self) -> Pin<Box<dyn Future<Output = Outcome> + Send>> {
        let context = PublicationContext {
            admission: self.owners.admit().unwrap(),
            reservation: self.cleanup.try_reserve().unwrap(),
            lifetime: None,
            releases: self.releases.clone(),
        };
        self.native.set_evaluate_response(serde_json::json!({"result": {"type": "object", "objectId": "queued-native", "description": "queued"}}));
        let native = self
            .service
            .stage_evaluate_handle(
                ObjectId::new(2),
                CallMetadata::default(),
                pwright_cap_handle_evaluation::EvaluateHandleParams::expression("({})"),
            )
            .await
            .unwrap();
        let queue = self.queue.clone();
        Box::pin(async move { queue.publish(native, context).await })
    }
    async fn drain(&self) -> u64 {
        self.queue.close();
        self.owners.close();
        let waiting = self.owners.wait();
        tokio::pin!(waiting);
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                self.releases.finish_ready().unwrap();
                tokio::select! { biased; () = &mut waiting => break, () = tokio::task::yield_now() => {} }
            }
        }).await.unwrap();
        let report = self.cleanup.shutdown().await;
        assert_eq!(report.metrics.reserved, 0);
        assert_eq!(self.routes.metrics().unwrap().reserved, 0);
        assert_eq!(self.routes.metrics().unwrap().current, 0);
        report.metrics.failed
    }
    fn releases(&self) -> usize {
        self.native
            .calls()
            .iter()
            .filter(|(method, _)| method == "Runtime.releaseObject")
            .count()
    }
}
fn poll_once<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

#[tokio::test]
async fn source_world_metadata_survives_replacement_and_shutdown_without_native_ownership() {
    // Arrange
    let fixture = fixture();
    let native = Arc::downgrade(&fixture.native);
    let mut result = fixture.begin().await;
    assert!(poll_once(result.as_mut()).is_pending());
    fixture.queue.finish_ready().unwrap();
    let handle = result.await.unwrap();

    // Act: reuse the numeric context ID, but replace its unique world identity.
    fixture.frames.unbind(ObjectId::new(2)).unwrap();
    fixture
        .frames
        .bind(
            ObjectId::new(2),
            ObjectId::new(1),
            fixture.native.clone(),
            101,
            "replacement-world",
        )
        .unwrap();
    let failed = fixture.drain().await;
    let released = fixture.releases();
    drop(fixture);

    // Assert: retained descriptors preserve only bounded source metadata. They
    // neither delay cleanup nor keep a native client alive after owner shutdown.
    let source = handle.source_world().expect("published source identity");
    assert_eq!(source.page_id(), ObjectId::new(1));
    #[cfg(feature = "retained-file-input")]
    assert_eq!(
        handle.source_identity().creation_page_id(),
        Some(ObjectId::new(1))
    );
    assert_eq!(source.execution_context_id(), 101);
    assert_eq!(source.unique_context_id(), "queue-world");
    assert_eq!(failed, 0);
    assert_eq!(released, 1);
    assert!(native.upgrade().is_none());
    handle.dispose().await.unwrap();
}

#[tokio::test]
async fn dropping_queued_receiver_compensates_without_portable_publication() {
    // Arrange
    let fixture = fixture();
    let mut result = fixture.begin().await;
    assert!(poll_once(result.as_mut()).is_pending());
    let before = fixture.runtime.live_object_count().unwrap();
    // Act
    drop(result);
    fixture.queue.finish_ready().unwrap();
    // Assert
    assert_eq!(fixture.drain().await, 0);
    assert_eq!(fixture.runtime.live_object_count().unwrap(), before);
    assert_eq!(fixture.releases(), 1);
}

#[tokio::test]
async fn dropping_unread_published_reply_runs_registered_handle_cleanup() {
    // Arrange
    let fixture = fixture();
    let mut result = fixture.begin().await;
    assert!(poll_once(result.as_mut()).is_pending());
    fixture.queue.finish_ready().unwrap();
    assert_eq!(fixture.routes.metrics().unwrap().current, 1);
    // Act
    drop(result);
    // Assert
    assert_eq!(fixture.drain().await, 0);
    assert_eq!(fixture.releases(), 1);
}

#[tokio::test]
async fn busy_publication_parks_without_self_waking_then_publishes_once() {
    // Arrange
    let fixture = fixture();
    let close = fixture
        .runtime
        .prepare_object_disposal(ObjectId::new(3))
        .unwrap();
    let mut result = fixture.begin().await;
    assert!(poll_once(result.as_mut()).is_pending());
    assert!(poll_once(Box::pin(fixture.queue.changed()).as_mut()).is_ready());
    let before = fixture.native.calls().len();
    // Act
    fixture.queue.finish_ready().unwrap();
    // Assert
    assert!(poll_once(Box::pin(fixture.queue.changed()).as_mut()).is_pending());
    assert!(poll_once(result.as_mut()).is_pending());
    assert_eq!(fixture.native.calls().len(), before);
    drop(close);
    fixture.queue.finish_ready().unwrap();
    let handle = result.await.unwrap();
    assert_eq!(fixture.native.calls().len(), before);
    drop(handle);
    assert_eq!(fixture.drain().await, 0);
}

#[tokio::test]
async fn queue_close_compensates_pending_and_late_arrival_results() {
    // Arrange
    let fixture = fixture();
    let mut result = fixture.begin().await;
    assert!(poll_once(result.as_mut()).is_pending());
    let late = fixture.begin().await;
    // Act
    fixture.queue.close();
    // Assert
    assert!(result.await.is_err());
    assert!(late.await.is_err());
    assert_eq!(fixture.drain().await, 0);
    assert_eq!(fixture.releases(), 2);
}

#[tokio::test]
async fn stale_source_rejection_finishes_compensation_before_reporting_failure() {
    // Arrange
    let fixture = fixture();
    let mut result = fixture.begin().await;
    assert!(poll_once(result.as_mut()).is_pending());
    fixture.frames.unbind(ObjectId::new(2)).unwrap();
    // Act
    fixture.queue.finish_ready().unwrap();
    // Assert
    let error = match result.await {
        Err(error) => error,
        Ok(_) => panic!("stale source was published"),
    };
    assert_eq!(error.kind(), FailureKind::TargetClosed);
    assert_eq!(fixture.releases(), 1);
    assert_eq!(fixture.drain().await, 0);
}

#[tokio::test]
async fn abandoned_result_cleanup_failure_reaches_executor_report() {
    // Arrange
    let fixture = fixture();
    let mut result = fixture.begin().await;
    assert!(poll_once(result.as_mut()).is_pending());
    fixture
        .native
        .set_release_object_error(Some("native compensation failed".into()));
    // Act
    drop(result);
    fixture.queue.finish_ready().unwrap();
    // Assert
    assert_eq!(fixture.drain().await, 1);
    assert_eq!(fixture.releases(), 1);
}

#[tokio::test]
async fn rejected_publication_reports_both_primary_and_compensation_failures() {
    // Arrange
    let fixture = fixture();
    let mut result = fixture.begin().await;
    assert!(poll_once(result.as_mut()).is_pending());
    fixture.frames.unbind(ObjectId::new(2)).unwrap();
    fixture
        .native
        .set_release_object_error(Some("native compensation failed".into()));
    // Act
    fixture.queue.finish_ready().unwrap();
    // Assert
    let error = match result.await {
        Err(error) => error,
        Ok(_) => panic!("stale source was published"),
    };
    assert_eq!(error.kind(), FailureKind::Cleanup);
    let details = error.details().unwrap().to_string();
    assert!(details.contains("no active Handle evaluation route"));
    assert!(details.contains("native compensation failed"));
    assert_eq!(fixture.drain().await, 1);
}
