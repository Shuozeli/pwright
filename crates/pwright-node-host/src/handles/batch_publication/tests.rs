use super::*;
use crate::handles::{
    PendingHandle,
    publication::{PublicationContext, Publications},
};
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
use serde_json::json;
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
    queue: Arc<BatchPublications>,
    releases: Arc<Releases>,
    owners: Arc<Owners>,
    source: PendingHandle,
    cleanup: TokioCleanupExecutor,
    resources: ProductionResourceLedger,
}

async fn fixture() -> Fixture {
    let limits = RuntimeLimits::new(256, 16);
    let evaluation = CdpEvaluationProductionProfile::new(
        ResourceScope::extension(166, 3),
        CdpEvaluationLimits::new(4096, 32768, 4096, crate::evaluation::value_limits()),
        8,
    )
    .with_context_routes(1, 128);
    let routes = ChromiumHandleRouteProfile::new(ResourceScope::extension(166, 4), 64, 1024, 1024);
    let plan = limits
        .resource_requirements()
        .combine_concurrent(evaluation.requirements())
        .combine_concurrent(routes.requirements())
        .combine_concurrent(requirements())
        .combine_concurrent(crate::handles::publication::requirements())
        .combine_concurrent(crate::handles::pending::requirements())
        .combine_concurrent(crate::handles::release::requirements().unwrap());
    let mut budget = ResourceLimits::deny_all(plan.scopes, plan.leases);
    for class in ResourceClass::ALL {
        let amount = plan.class(class);
        budget = budget.with_class(class, amount.global, amount.per_scope);
    }
    let resources = ProductionResourceLedger::new(budget, plan).unwrap();
    let runtime = ControlledDispatcher::new((), (), NoDeadlineScheduler, limits)
        .with_production_resources(&resources)
        .unwrap();
    for (object, parent, kind) in [(1, None, 1), (2, Some(1), 2), (3, Some(1), 2)] {
        runtime
            .register_object(
                ObjectId::new(object),
                parent.map(ObjectId::new),
                ObjectKindTag::new(kind),
            )
            .unwrap();
    }
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
            "batch-world",
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
        &resources,
    )
    .unwrap();
    let owners = Owners::new(&resources).unwrap();
    let queue = BatchPublications::new(&resources, owners.clone()).unwrap();
    let cleanup = TokioCleanupExecutor::new(16);
    let scalar = Publications::new(&resources, owners.clone()).unwrap();
    native.set_evaluate_response(json!({"result": {
        "type": "object", "objectId": "source-native", "description": "source"
    }}));
    let source_native = service
        .stage_evaluate_handle(
            ObjectId::new(2),
            CallMetadata::default(),
            pwright_cap_handle_evaluation::EvaluateHandleParams::expression("({})"),
        )
        .await
        .unwrap();
    let context = PublicationContext {
        admission: owners.admit().unwrap(),
        reservation: cleanup.try_reserve().unwrap(),
        lifetime: None,
        releases: releases.clone(),
    };
    let mut source = Box::pin(scalar.publish(source_native, context));
    assert!(poll_once(source.as_mut()).is_pending());
    scalar.finish_ready().unwrap();
    let source = source.await.unwrap();
    drop(scalar);
    Fixture {
        runtime,
        service,
        frames,
        native,
        routes,
        queue,
        releases,
        owners,
        source,
        cleanup,
        resources,
    }
}

impl Fixture {
    async fn begin(&self, names: &[&str]) -> Pin<Box<dyn Future<Output = Outcome> + Send>> {
        let mut admission = self.owners.admit_batch().unwrap();
        // This fixture selects four backend result slots, while the real host
        // installs a profile matching its complete available owner admission.
        admission.entries.truncate(4);
        let reservations = (0..admission.entries.len())
            .map(|_| self.cleanup.try_reserve().unwrap())
            .collect();
        let context = BatchContext {
            admission,
            reservations,
            recovery: self.cleanup.try_reserve().unwrap(),
            lifetime: None,
            releases: self.releases.clone(),
        };
        let descriptors = names.iter().map(|name| json!({
            "name": name, "enumerable": true,
            "value": {"type": "object", "objectId": format!("native-{name}"), "description": name}
        })).collect::<Vec<_>>();
        self.native.set_evaluate_responses([
            json!({"result": {"type": "object", "objectId": "carrier"}}),
            json!({"result": descriptors}),
        ]);
        let native = self
            .service
            .stage_get_properties(
                self.source.initializer().object_id,
                CallMetadata::default(),
                context.admission.entries.len(),
            )
            .await
            .unwrap();
        let queue = self.queue.clone();
        Box::pin(async move { queue.publish(native, context).await })
    }

    async fn complete<F: Future>(&self, future: F) -> F::Output {
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            tokio::pin!(future);
            loop {
                self.releases.finish_ready().unwrap();
                tokio::select! {
                    biased;
                    result = &mut future => break result,
                    () = tokio::task::yield_now() => {},
                }
            }
        })
        .await
        .expect("controlled batch cleanup must settle")
    }

    async fn drain(&self) -> u64 {
        self.queue.close();
        self.owners.close();
        self.complete(self.owners.wait()).await;
        let report = self.cleanup.shutdown().await;
        assert_eq!(report.metrics.reserved, 0);
        assert_eq!(self.routes.metrics().unwrap().reserved, 0);
        assert_eq!(self.routes.metrics().unwrap().current, 0);
        report.metrics.failed
    }

    fn released(&self, native_id: &str) -> usize {
        self.native
            .calls()
            .iter()
            .filter(|(method, arguments)| {
                method == "Runtime.releaseObject" && arguments[0].as_str() == Some(native_id)
            })
            .count()
    }
}

fn poll_once<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

#[tokio::test]
async fn every_sibling_keeps_source_world_metadata_without_retaining_native_ownership() {
    // Arrange
    let fixture = fixture().await;
    let native = Arc::downgrade(&fixture.native);
    let mut result = fixture.begin(&["first", "second", "third"]).await;
    assert!(poll_once(result.as_mut()).is_pending());
    fixture.queue.finish_ready().unwrap();
    let handles = result
        .await
        .unwrap()
        .into_iter()
        .map(NamedPendingHandle::into_handle)
        .collect::<Vec<_>>();
    let source = fixture.source.source_world().unwrap().clone();

    // Act: both parts of the live Frame world change before owner shutdown.
    fixture.frames.unbind(ObjectId::new(2)).unwrap();
    fixture
        .frames
        .bind(
            ObjectId::new(2),
            ObjectId::new(1),
            fixture.native.clone(),
            202,
            "replacement-world",
        )
        .unwrap();
    let failed = fixture.drain().await;
    let released = ["native-first", "native-second", "native-third"]
        .map(|native_id| fixture.released(native_id));
    drop(fixture);

    // Assert: scalar source and every batch result carry the same immutable
    // identity; sibling descriptors do not retain any released native service.
    assert_eq!(source.page_id(), ObjectId::new(1));
    assert_eq!(source.execution_context_id(), 101);
    assert_eq!(source.unique_context_id(), "batch-world");
    assert_eq!(handles.len(), 3);
    assert!(
        handles
            .iter()
            .all(|handle| handle.source_world() == Some(&source))
    );
    #[cfg(feature = "retained-file-input")]
    assert!(
        handles.iter().all(|handle| {
            handle.source_identity().creation_page_id() == Some(ObjectId::new(1))
        })
    );
    assert_eq!(failed, 0);
    assert_eq!(released, [1, 1, 1]);
    assert!(native.upgrade().is_none());
    for handle in &handles {
        handle.dispose().await.unwrap();
    }
}

#[tokio::test]
async fn sibling_publication_preserves_order_and_independent_native_lifetimes() {
    // Arrange
    let fixture = fixture().await;
    let mut result = fixture.begin(&["zeta", "alpha"]).await;
    assert!(poll_once(result.as_mut()).is_pending());
    // Act
    fixture.queue.finish_ready().unwrap();
    let named = result.await.unwrap();
    // Assert
    assert_eq!(
        named
            .iter()
            .map(NamedPendingHandle::name)
            .collect::<Vec<_>>(),
        ["zeta", "alpha"]
    );
    let mut handles = named.into_iter().map(NamedPendingHandle::into_handle);
    let first = handles.next().unwrap();
    let second = handles.next().unwrap();
    assert!(handles.next().is_none());
    assert_ne!(
        first.initializer().object_id,
        second.initializer().object_id
    );
    assert_eq!(first.initializer().parent_id, ObjectId::new(2));
    assert_eq!(second.initializer().parent_id, ObjectId::new(2));
    fixture.complete(first.dispose()).await.unwrap();
    assert_eq!(
        fixture
            .runtime
            .is_object_live(first.initializer().object_id),
        Ok(false)
    );
    assert_eq!(
        fixture
            .runtime
            .is_object_live(second.initializer().object_id),
        Ok(true)
    );
    assert_eq!(
        fixture
            .runtime
            .is_object_live(fixture.source.initializer().object_id),
        Ok(true)
    );
    assert_eq!(fixture.released("native-zeta"), 1);
    assert_eq!(fixture.released("native-alpha"), 0);
    drop((first, second, handles));
    assert_eq!(fixture.drain().await, 0);
    assert_eq!(fixture.released("native-alpha"), 1);
    assert_eq!(fixture.released("source-native"), 1);
}

#[tokio::test]
async fn unused_upfront_sibling_admission_is_trimmed_before_publication() {
    // Arrange
    let fixture = fixture().await;
    let mut result = fixture.begin(&["first", "second"]).await;
    let before = fixture.resources.metrics().unwrap();
    assert_eq!(fixture.cleanup.metrics().reserved, 6);
    // Act
    assert!(poll_once(result.as_mut()).is_pending());
    let queued = fixture.resources.metrics().unwrap();
    let queued_cleanup = fixture.cleanup.metrics();
    fixture.queue.finish_ready().unwrap();
    let named = result.await.unwrap();
    // Assert
    assert_eq!(before.active_leases - queued.active_leases, 2);
    assert_eq!(queued_cleanup.reserved, 4);
    assert_eq!(fixture.cleanup.metrics().reserved, 3);
    assert_eq!(fixture.routes.metrics().unwrap().reserved, 0);
    assert_eq!(fixture.routes.metrics().unwrap().current, 3);
    assert_eq!(named.len(), 2);
    drop(named);
    assert_eq!(fixture.drain().await, 0);
}

#[tokio::test]
async fn receiver_loss_before_commit_compensates_all_siblings_without_new_objects() {
    // Arrange
    let fixture = fixture().await;
    let before = fixture.runtime.live_object_count().unwrap();
    let mut result = fixture.begin(&["first", "second"]).await;
    assert!(poll_once(result.as_mut()).is_pending());
    // Act
    drop(result);
    fixture.queue.finish_ready().unwrap();
    // Assert
    assert_eq!(fixture.runtime.live_object_count().unwrap(), before);
    assert_eq!(fixture.routes.metrics().unwrap().current, 1);
    assert_eq!(fixture.drain().await, 0);
    assert_eq!(fixture.released("native-first"), 1);
    assert_eq!(fixture.released("native-second"), 1);
}

#[tokio::test]
async fn losing_an_unread_published_batch_releases_every_registered_sibling() {
    // Arrange
    let fixture = fixture().await;
    let mut result = fixture.begin(&["first", "second"]).await;
    assert!(poll_once(result.as_mut()).is_pending());
    fixture.queue.finish_ready().unwrap();
    assert_eq!(fixture.routes.metrics().unwrap().current, 3);
    // Act
    drop(result);
    // Assert
    assert_eq!(fixture.drain().await, 0);
    assert_eq!(fixture.released("native-first"), 1);
    assert_eq!(fixture.released("native-second"), 1);
    assert_eq!(fixture.runtime.live_object_count().unwrap(), 3);
}

#[tokio::test]
async fn busy_lifecycle_defers_the_whole_batch_without_repeating_native_io() {
    // Arrange
    let fixture = fixture().await;
    let barrier = fixture
        .runtime
        .prepare_object_disposal(ObjectId::new(3))
        .unwrap();
    let mut result = fixture.begin(&["first", "second"]).await;
    assert!(poll_once(result.as_mut()).is_pending());
    assert!(poll_once(Box::pin(fixture.queue.changed()).as_mut()).is_ready());
    let native_calls = fixture.native.calls().len();
    // Act
    fixture.queue.finish_ready().unwrap();
    // Assert
    assert!(poll_once(result.as_mut()).is_pending());
    assert!(poll_once(Box::pin(fixture.queue.changed()).as_mut()).is_pending());
    assert_eq!(fixture.native.calls().len(), native_calls);
    assert_eq!(fixture.routes.metrics().unwrap().current, 1);
    drop(barrier);
    fixture.queue.finish_ready().unwrap();
    let named = result.await.unwrap();
    assert_eq!(fixture.native.calls().len(), native_calls);
    assert_eq!(named.len(), 2);
    drop(named);
    assert_eq!(fixture.drain().await, 0);
}

#[tokio::test]
async fn stale_source_world_rejects_and_compensates_all_native_siblings() {
    // Arrange
    let fixture = fixture().await;
    let mut result = fixture.begin(&["first", "second"]).await;
    assert!(poll_once(result.as_mut()).is_pending());
    fixture.frames.unbind(ObjectId::new(2)).unwrap();
    // Act
    fixture.queue.finish_ready().unwrap();
    let error = match result.await {
        Err(error) => error,
        Ok(_) => panic!("stale batch source was published"),
    };
    // Assert
    assert_eq!(error.kind(), FailureKind::TargetClosed);
    assert_eq!(fixture.released("native-first"), 1);
    assert_eq!(fixture.released("native-second"), 1);
    assert_eq!(fixture.routes.metrics().unwrap().current, 1);
    assert_eq!(fixture.drain().await, 0);
}

#[tokio::test]
async fn disposed_source_handle_rejects_and_compensates_all_native_siblings() {
    // Arrange
    let fixture = fixture().await;
    let mut result = fixture.begin(&["first", "second"]).await;
    assert!(poll_once(result.as_mut()).is_pending());
    fixture.complete(fixture.source.dispose()).await.unwrap();
    // Act
    fixture.queue.finish_ready().unwrap();
    let error = match result.await {
        Err(error) => error,
        Ok(_) => panic!("disposed batch source was published"),
    };
    // Assert
    assert_eq!(error.kind(), FailureKind::TargetClosed);
    assert_eq!(fixture.released("native-first"), 1);
    assert_eq!(fixture.released("native-second"), 1);
    assert_eq!(fixture.routes.metrics().unwrap().current, 0);
    assert_eq!(fixture.drain().await, 0);
}

#[tokio::test]
async fn refused_abandoned_compensation_attempts_every_sibling_and_reports_failure() {
    // Arrange
    let fixture = fixture().await;
    let mut result = fixture.begin(&["first", "second"]).await;
    assert!(poll_once(result.as_mut()).is_pending());
    fixture
        .native
        .set_release_object_error(Some("batch compensation failed".into()));
    // Act
    drop(result);
    fixture.queue.finish_ready().unwrap();
    fixture
        .complete(async {
            while fixture.cleanup.metrics().failed == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await;
    // Assert
    assert_eq!(fixture.cleanup.metrics().failed, 1);
    assert_eq!(fixture.released("native-first"), 1);
    assert_eq!(fixture.released("native-second"), 1);
    // The independent source owner is not part of failed batch compensation.
    fixture.native.set_release_object_error(None);
    assert_eq!(fixture.drain().await, 1);
}

#[tokio::test]
async fn owner_close_before_retention_still_submits_every_committed_sibling_ticket() {
    // Arrange
    let fixture = fixture().await;
    let mut result = fixture.begin(&["first", "second"]).await;
    assert!(poll_once(result.as_mut()).is_pending());
    // Act: do not yield between close and commit, so source cleanup cannot
    // replace this retain-failure case with a stale-source rejection.
    fixture.owners.close();
    fixture.queue.finish_ready().unwrap();
    let locally_published = fixture.routes.metrics().unwrap().current;
    let accepted = fixture.cleanup.metrics().accepted;
    let error = match result.await {
        Err(error) => error,
        Ok(_) => panic!("closed owners accepted a batch"),
    };
    // Assert
    // Host preparation uses the same Backend envelope as scalar publication;
    // the typed detail must still preserve the precise closed-owner outcome.
    assert_eq!(error.kind(), FailureKind::Backend);
    assert!(matches!(error.details(), Some(HostError::TargetClosed)));
    assert_eq!(locally_published, 3);
    assert_eq!(accepted, 3);
    assert_eq!(fixture.drain().await, 0);
    assert_eq!(fixture.released("native-first"), 1);
    assert_eq!(fixture.released("native-second"), 1);
}

#[tokio::test]
async fn empty_batch_returns_all_upfront_tickets_without_publishing_objects() {
    // Arrange
    let fixture = fixture().await;
    let before = fixture.runtime.live_object_count().unwrap();
    let mut result = fixture.begin(&[]).await;
    assert!(poll_once(result.as_mut()).is_pending());
    // Act
    fixture.queue.finish_ready().unwrap();
    let named = result.await.unwrap();
    // Assert
    assert!(named.is_empty());
    assert_eq!(fixture.runtime.live_object_count().unwrap(), before);
    assert_eq!(fixture.routes.metrics().unwrap().current, 1);
    assert_eq!(fixture.cleanup.metrics().reserved, 1);
    assert_eq!(fixture.released("carrier"), 1);
    assert_eq!(fixture.drain().await, 0);
}
