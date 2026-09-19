//! Cache lifecycle contracts through the real marker composition and FakeCdpClient.
//! Counted route/proof owners stand in for independently admitted snapshots, not
//! fabricated browser worlds. These tests do not establish assembled HostSession
//! routing, source-Handle retention, Node promotion, DOM events or upload parity.

use super::*;
use pwright_backend_cdp_file_input::{CdpTargetMarkingService, MarkerWorld, TargetMarkingProfile};
use pwright_composition_chromium_file_input::{TargetMarkingOperationService, UploadProfile};
use pwright_composition_chromium_locator_resolution::{
    CdpLocatorResolutionService, ContinuableResolutionService, ObjectGroupDestructionProof,
    ResolutionProfile, RetainedElementRequest,
};
use pwright_core::{
    AtomicObjectIdSource, ObjectId, ProductionResourceLedger, ResourceClass, ResourceLimits,
    ResourceRequirements, ResourceScope,
};
use pwright_fake::FakeCdpClient;
use pwright_runtime_tokio::TokioCleanupExecutor;
use serde_json::json;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
    time::Duration,
};
use tokio::sync::Notify;

struct Route {
    current: AtomicBool,
}
impl Route {
    fn new() -> Self {
        Self {
            current: AtomicBool::new(true),
        }
    }
}
impl MarkerRoute for Route {
    fn is_current(&self) -> bool {
        self.current.load(Ordering::Acquire)
    }
}

fn production(requirements: ResourceRequirements) -> ProductionResourceLedger {
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let bound = requirements.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    ProductionResourceLedger::new(limits, requirements).unwrap()
}

struct Fixture {
    cache: MarkerCache,
    service: Arc<TargetMarkingOperationService<TokioCleanupExecutor>>,
    creation: TokioCleanupExecutor,
    cleanup: TokioCleanupExecutor,
    resources: ProductionResourceLedger,
    client: Arc<FakeCdpClient>,
    factories: AtomicUsize,
}

impl Fixture {
    fn new() -> Self {
        let backend =
            TargetMarkingProfile::new(ResourceScope::extension(183, 91), WORLD_SLOTS, CALLS)
                .unwrap();
        let states =
            UploadProfile::new(ResourceScope::extension(183, 92), WORLD_SLOTS, 16 * 1024).unwrap();
        let calls =
            UploadProfile::new(ResourceScope::extension(183, 93), CALLS, 16 * 1024).unwrap();
        let resources = production(
            MarkerCache::requirements()
                .combine_concurrent(backend.requirements())
                .combine_concurrent(states.requirements())
                .combine_concurrent(calls.requirements()),
        );
        let raw = Arc::new(
            CdpTargetMarkingService::new(
                backend,
                &resources,
                Arc::new(AtomicObjectIdSource::new(1).unwrap()),
            )
            .unwrap(),
        );
        let cleanup = TokioCleanupExecutor::new(WORLD_SLOTS + CALLS);
        let creation = TokioCleanupExecutor::new(WORLD_SLOTS);
        let service = Arc::new(
            TargetMarkingOperationService::new(states, calls, &resources, raw, cleanup.clone())
                .unwrap(),
        );
        let cache = MarkerCache::new(&resources, service.clone(), creation.clone()).unwrap();
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(json!({
            "result":{"type":"object","objectId":"private-marker"}
        }));
        Self {
            cache,
            service,
            creation,
            cleanup,
            resources,
            client,
            factories: AtomicUsize::new(0),
        }
    }

    fn key(&self, frame: u64, unique: &str) -> MarkerKey {
        MarkerKey::new(
            ObjectId::new(2),
            ObjectId::new(frame),
            self.client.clone(),
            Arc::from(unique),
        )
        .unwrap()
    }

    fn acquire(&self, frame: u64, unique: &str) -> Result<MarkerLookup, MarkerCacheError> {
        self.acquire_with_route(frame, unique, Arc::new(Route::new()))
    }

    fn acquire_with_route(
        &self,
        frame: u64,
        unique: &str,
        route: Arc<Route>,
    ) -> Result<MarkerLookup, MarkerCacheError> {
        self.cache.acquire(self.key(frame, unique), || {
            self.factories.fetch_add(1, Ordering::AcqRel);
            Ok((
                MarkerWorld {
                    client: self.client.clone(),
                    utility_world: Arc::from(unique),
                    destruction: None,
                },
                route as Arc<dyn MarkerRoute>,
            ))
        })
    }

    fn creations(&self) -> usize {
        self.client
            .calls()
            .into_iter()
            .filter(|(method, params)| {
                method == "Runtime.callFunctionOn"
                    && params[0]["objectGroup"]
                        .as_str()
                        .is_some_and(|group| group.starts_with("pwright.target-marker."))
            })
            .count()
    }

    fn releases(&self) -> Vec<String> {
        self.client
            .calls()
            .into_iter()
            .filter_map(|(method, params)| {
                (method == "Runtime.releaseObjectGroup")
                    .then(|| params[0].as_str().unwrap().to_owned())
            })
            .collect()
    }

    async fn finish(self) {
        self.cache.seal();
        self.cache.close();
        let creation = bounded(self.creation.shutdown()).await;
        let cleanup = bounded(self.cleanup.shutdown()).await;
        for report in [creation, cleanup] {
            assert_eq!(report.metrics.failed, 0);
            assert_eq!(report.metrics.timed_out, 0);
            assert_eq!(report.metrics.reserved, 0);
            assert_eq!(report.metrics.in_flight, 0);
        }
        assert!(self.cache.is_cleanup_healthy());
        assert!(self.service.is_cleanup_healthy());
        assert_eq!(self.service.active_states(), 0);
        assert_eq!(self.service.active_calls(), 0);
        let metrics = self.cache.metrics();
        assert_eq!(metrics.current, 0);
        assert_eq!(metrics.creating, 0);
        assert_eq!(metrics.retiring, 0);
        assert_eq!(metrics.waiters, 0);
        assert_eq!(
            self.resources
                .metrics()
                .unwrap()
                .class(ResourceClass::RemoteHandle)
                .current,
            0
        );
        let resources = self.resources.clone();
        drop(self);
        let metrics = resources.metrics().unwrap();
        assert_eq!(metrics.active_leases, 0);
        for class in ResourceClass::ALL {
            assert_eq!(metrics.class(class).current, 0, "unreturned {class:?}");
        }
    }
}

async fn bounded<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(5), future)
        .await
        .expect("diagnostic cache-test deadline; not a semantic rejection")
}

async fn settle_until(mut condition: impl FnMut() -> bool) {
    // Bounded executor turns observe an explicit state, never a time-based sleep.
    for _ in 0..256 {
        if condition() {
            return;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        condition(),
        "cache did not reach the expected settled state"
    );
}

fn poll_now<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

#[tokio::test]
async fn exact_world_waiters_share_one_factory_and_one_native_initialization() {
    // Arrange
    let fixture = Fixture::new();
    let gate = Arc::new(Notify::new());
    fixture.client.set_evaluate_gate(Some(gate.clone()));
    let first = fixture.acquire(4, "utility-a").unwrap();
    let second = fixture.acquire(4, "utility-a").unwrap();
    settle_until(|| fixture.creations() == 1).await;

    // Act
    gate.notify_waiters();
    let first = bounded(first).await.unwrap();
    let second = bounded(second).await.unwrap();

    // Assert
    assert_eq!(fixture.factories.load(Ordering::Acquire), 1);
    assert_eq!(fixture.creations(), 1);
    assert_eq!(fixture.service.active_states(), 1);
    assert_eq!(fixture.cache.metrics().current, 1);
    drop((first, second));
    assert_eq!(fixture.cache.metrics().waiters, 0);
    assert_eq!(fixture.service.active_states(), 1);
    assert!(fixture.releases().is_empty());
    fixture.finish().await;
}

#[tokio::test]
async fn cancelling_one_waiter_does_not_cancel_another_waiters_shared_initialization() {
    // Arrange
    let fixture = Fixture::new();
    let gate = Arc::new(Notify::new());
    fixture.client.set_evaluate_gate(Some(gate.clone()));
    let first = fixture.acquire(4, "utility-a").unwrap();
    let second = fixture.acquire(4, "utility-a").unwrap();
    settle_until(|| fixture.creations() == 1).await;

    // Act
    drop(first);
    gate.notify_waiters();
    let survivor = bounded(second).await.unwrap();

    // Assert
    assert_eq!(fixture.creations(), 1);
    assert_eq!(fixture.cache.metrics().current, 1);
    assert_eq!(fixture.cache.metrics().retiring, 0);
    assert!(fixture.releases().is_empty());
    drop(survivor);
    fixture.finish().await;
}

#[tokio::test]
async fn cancelling_the_last_creating_waiter_cleans_a_late_native_result() {
    // Arrange
    let fixture = Fixture::new();
    let gate = Arc::new(Notify::new());
    fixture.client.set_evaluate_gate(Some(gate.clone()));
    let lookup = fixture.acquire(4, "utility-a").unwrap();
    settle_until(|| fixture.creations() == 1).await;

    // Act
    drop(lookup);
    gate.notify_waiters();

    // Assert
    settle_until(|| fixture.service.active_states() == 0).await;
    assert_eq!(fixture.releases().len(), 1);
    assert_eq!(fixture.cache.metrics().current, 0);
    assert_eq!(fixture.cache.metrics().creating, 0);
    assert_eq!(fixture.cache.metrics().retiring, 0);
    assert_eq!(fixture.cache.metrics().waiters, 0);
    fixture.finish().await;
}

#[tokio::test]
async fn seal_rejects_new_acquisition_and_close_compensates_late_initialization() {
    // Arrange
    let fixture = Fixture::new();
    let gate = Arc::new(Notify::new());
    fixture.client.set_evaluate_gate(Some(gate.clone()));
    let lookup = fixture.acquire(4, "utility-a").unwrap();
    settle_until(|| fixture.creations() == 1).await;

    // Act
    fixture.cache.seal();
    let refused = fixture.acquire(5, "utility-b");
    fixture.cache.close();
    gate.notify_waiters();
    let late = bounded(lookup).await;

    // Assert
    assert!(refused.is_err());
    assert!(late.is_err());
    assert_eq!(fixture.factories.load(Ordering::Acquire), 1);
    settle_until(|| fixture.service.active_states() == 0).await;
    assert_eq!(fixture.releases().len(), 1);
    fixture.finish().await;
}

#[tokio::test]
async fn current_world_capacity_refuses_before_factory_or_native_io() {
    // Arrange
    let fixture = Fixture::new();
    for index in 0..CURRENT_WORLDS {
        drop(
            bounded(fixture.acquire(4 + index as u64, "utility-a").unwrap())
                .await
                .unwrap(),
        );
    }
    let before_calls = fixture.client.calls().len();
    let before_factories = fixture.factories.load(Ordering::Acquire);

    // Act
    let refused = fixture.acquire(4 + CURRENT_WORLDS as u64, "utility-a");

    // Assert
    assert!(refused.is_err());
    assert_eq!(fixture.factories.load(Ordering::Acquire), before_factories);
    assert_eq!(fixture.client.calls().len(), before_calls);
    assert_eq!(fixture.cache.metrics().current, CURRENT_WORLDS);
    assert_eq!(fixture.cache.metrics().retiring, 0);
    assert_eq!(fixture.service.active_states(), CURRENT_WORLDS);
    fixture.finish().await;
}

#[tokio::test]
async fn same_world_lookup_and_lease_capacity_is_bounded_independently_of_world_slots() {
    // Arrange
    let fixture = Fixture::new();
    drop(
        bounded(fixture.acquire(4, "utility-a").unwrap())
            .await
            .unwrap(),
    );
    let mut leases = Vec::new();
    for _ in 0..CALLS {
        leases.push(
            bounded(fixture.acquire(4, "utility-a").unwrap())
                .await
                .unwrap(),
        );
    }

    // Act
    let refused = fixture.acquire(4, "utility-a");

    // Assert
    assert!(refused.is_err());
    assert_eq!(fixture.factories.load(Ordering::Acquire), 1);
    assert_eq!(fixture.creations(), 1);
    assert_eq!(fixture.cache.metrics().waiters, CALLS);
    leases.pop();
    let reused = bounded(fixture.acquire(4, "utility-a").unwrap())
        .await
        .unwrap();
    assert_eq!(fixture.factories.load(Ordering::Acquire), 1);
    drop((leases, reused));
    fixture.finish().await;
}

#[tokio::test]
async fn repeated_world_replacement_cannot_overflow_retiring_capacity() {
    // Arrange
    let fixture = Fixture::new();
    drop(
        bounded(fixture.acquire(4, "utility-0").unwrap())
            .await
            .unwrap(),
    );
    let gate = Arc::new(Notify::new());
    fixture
        .client
        .set_release_object_group_gate(Some(gate.clone()));
    for generation in 1..=RETIRING_WORLDS {
        drop(
            bounded(
                fixture
                    .acquire(4, &format!("utility-{generation}"))
                    .unwrap(),
            )
            .await
            .unwrap(),
        );
    }
    settle_until(|| fixture.releases().len() == RETIRING_WORLDS).await;
    let before_factories = fixture.factories.load(Ordering::Acquire);
    let before_calls = fixture.client.calls().len();

    // Act
    let refused = fixture.acquire(4, "utility-overflow");

    // Assert
    assert!(refused.is_err());
    assert_eq!(fixture.factories.load(Ordering::Acquire), before_factories);
    assert_eq!(fixture.client.calls().len(), before_calls);
    assert_eq!(fixture.cache.metrics().current, 1);
    assert_eq!(fixture.cache.metrics().retiring, RETIRING_WORLDS);
    fixture.client.set_release_object_group_gate(None);
    gate.notify_waiters();
    settle_until(|| fixture.cache.metrics().retiring == 0).await;
    drop(
        bounded(fixture.acquire(4, "utility-after-retirement").unwrap())
            .await
            .unwrap(),
    );
    fixture.finish().await;
}

#[tokio::test]
async fn a_pending_lookup_registers_for_completion_without_starting_duplicate_io() {
    // Arrange
    let fixture = Fixture::new();
    let gate = Arc::new(Notify::new());
    fixture.client.set_evaluate_gate(Some(gate.clone()));
    let mut lookup = Box::pin(fixture.acquire(4, "utility-a").unwrap());
    settle_until(|| fixture.creations() == 1).await;

    // Act
    let first = poll_now(lookup.as_mut());
    let second = poll_now(lookup.as_mut());

    // Assert
    assert!(first.is_pending());
    assert!(second.is_pending());
    assert_eq!(fixture.creations(), 1);
    gate.notify_waiters();
    drop(bounded(lookup).await.unwrap());
    fixture.finish().await;
}

struct IndependentProof;
impl ObjectGroupDestructionProof for IndependentProof {
    fn all_group_worlds_destroyed(&self) -> bool {
        false
    }
}

#[tokio::test]
async fn independent_route_and_destruction_receipt_survive_until_group_acknowledgement() {
    // Arrange: neither owner contains a Handle, task token or source descriptor.
    let fixture = Fixture::new();
    let route = Arc::new(Route::new());
    let route_weak = Arc::downgrade(&route);
    let proof = Arc::new(IndependentProof);
    let proof_weak = Arc::downgrade(&proof);
    let lookup = fixture
        .cache
        .acquire(fixture.key(4, "utility-a"), || {
            Ok((
                MarkerWorld {
                    client: fixture.client.clone(),
                    utility_world: "utility-a".into(),
                    destruction: Some(proof),
                },
                route as Arc<dyn MarkerRoute>,
            ))
        })
        .unwrap();
    drop(bounded(lookup).await.unwrap());
    let gate = Arc::new(Notify::new());
    fixture
        .client
        .set_release_object_group_gate(Some(gate.clone()));

    // Act
    fixture.cache.seal();
    fixture.cache.close();
    settle_until(|| fixture.releases().len() == 1).await;

    // Assert
    assert!(route_weak.upgrade().is_some());
    assert!(proof_weak.upgrade().is_some());
    assert_eq!(fixture.cache.metrics().retiring, 1);
    gate.notify_waiters();
    settle_until(|| route_weak.upgrade().is_none() && proof_weak.upgrade().is_none()).await;
    assert_eq!(fixture.cache.metrics().retiring, 0);
    fixture.finish().await;
}

#[tokio::test]
async fn failed_route_factory_returns_admission_without_submitting_native_work() {
    // Arrange
    let fixture = Fixture::new();

    // Act
    let rejected = fixture
        .cache
        .acquire(fixture.key(4, "utility-a"), || Err(HostError::TargetClosed));

    // Assert
    assert!(rejected.is_err());
    assert!(fixture.client.calls().is_empty());
    assert_eq!(fixture.cache.metrics().current, 0);
    assert_eq!(fixture.cache.metrics().creating, 0);
    assert_eq!(fixture.cache.metrics().retiring, 0);
    assert_eq!(fixture.cache.metrics().waiters, 0);
    drop(
        bounded(fixture.acquire(4, "utility-a").unwrap())
            .await
            .unwrap(),
    );
    fixture.finish().await;
}

#[tokio::test]
async fn factory_can_synchronously_seal_cache_and_prevent_native_creation() {
    // Arrange
    let fixture = Fixture::new();
    let baseline = fixture.resources.metrics().unwrap();
    let route = Arc::new(Route::new());
    let route_weak = Arc::downgrade(&route);
    let callbacks = AtomicUsize::new(0);

    // Act: actual synchronous reentry, not a timer-based deadlock assertion.
    let rejected = fixture.cache.acquire(fixture.key(4, "utility-a"), || {
        // Observe only: fail deterministically before seal if a regression holds
        // the lock here, rather than hanging the test process indefinitely.
        assert!(fixture.cache.0.state.try_lock().is_ok());
        callbacks.fetch_add(1, Ordering::AcqRel);
        fixture.cache.seal();
        Ok((
            MarkerWorld {
                client: fixture.client.clone(),
                utility_world: "utility-a".into(),
                destruction: None,
            },
            route as Arc<dyn MarkerRoute>,
        ))
    });

    // Assert: post-callback validation sees sealing before any native admission.
    assert!(rejected.is_err());
    assert_eq!(callbacks.load(Ordering::Acquire), 1);
    assert!(fixture.client.calls().is_empty());
    assert!(route_weak.upgrade().is_none());
    assert_eq!(fixture.cache.metrics().current, 0);
    assert_eq!(fixture.cache.metrics().creating, 0);
    assert_eq!(fixture.cache.metrics().retiring, 0);
    assert_eq!(fixture.cache.metrics().waiters, 0);
    assert_eq!(fixture.creation.metrics().reserved, 0);
    assert_eq!(fixture.creation.metrics().in_flight, 0);
    assert_eq!(fixture.cleanup.metrics().reserved, 0);
    assert_eq!(fixture.service.active_states(), 0);
    let after = fixture.resources.metrics().unwrap();
    assert_eq!(after.active_leases, baseline.active_leases);
    for class in ResourceClass::ALL {
        assert_eq!(after.class(class).current, baseline.class(class).current);
    }
    fixture.finish().await;
}

#[tokio::test]
async fn factory_world_with_a_foreign_client_rejects_before_native_io() {
    // Arrange
    let fixture = Fixture::new();
    let foreign = Arc::new(FakeCdpClient::from_html(""));
    let route = Arc::new(Route::new());
    let weak = Arc::downgrade(&route);

    // Act
    let rejected = fixture.cache.acquire(fixture.key(4, "utility-a"), || {
        Ok((
            MarkerWorld {
                client: foreign.clone(),
                utility_world: "utility-a".into(),
                destruction: None,
            },
            route as Arc<dyn MarkerRoute>,
        ))
    });

    // Assert
    assert!(matches!(rejected, Err(MarkerCacheError::Host(_))));
    assert!(fixture.client.calls().is_empty());
    assert!(foreign.calls().is_empty());
    assert!(weak.upgrade().is_none());
    assert_eq!(fixture.cache.metrics().current, 0);
    assert_eq!(fixture.cache.metrics().creating, 0);
    assert_eq!(fixture.cache.metrics().waiters, 0);
    drop(
        bounded(fixture.acquire(4, "utility-a").unwrap())
            .await
            .unwrap(),
    );
    fixture.finish().await;
}

#[tokio::test]
async fn factory_world_with_a_different_unique_id_rejects_before_native_io() {
    // Arrange
    let fixture = Fixture::new();
    let route = Arc::new(Route::new());
    let weak = Arc::downgrade(&route);

    // Act
    let rejected = fixture.cache.acquire(fixture.key(4, "utility-a"), || {
        Ok((
            MarkerWorld {
                client: fixture.client.clone(),
                utility_world: "utility-b".into(),
                destruction: None,
            },
            route as Arc<dyn MarkerRoute>,
        ))
    });

    // Assert
    assert!(matches!(rejected, Err(MarkerCacheError::Host(_))));
    assert!(fixture.client.calls().is_empty());
    assert!(weak.upgrade().is_none());
    assert_eq!(fixture.cache.metrics().current, 0);
    assert_eq!(fixture.cache.metrics().creating, 0);
    assert_eq!(fixture.cache.metrics().waiters, 0);
    drop(
        bounded(fixture.acquire(4, "utility-a").unwrap())
            .await
            .unwrap(),
    );
    fixture.finish().await;
}

#[tokio::test]
async fn malformed_native_creation_is_compensated_before_its_lookup_fails() {
    // Arrange
    let fixture = Fixture::new();
    fixture
        .client
        .set_evaluate_response(json!({"result":{"type":"undefined"}}));
    let lookup = fixture.acquire(4, "utility-a").unwrap();

    // Act
    let rejected = bounded(lookup).await;

    // Assert
    assert!(rejected.is_err());
    assert_eq!(fixture.creations(), 1);
    assert_eq!(fixture.releases().len(), 1);
    settle_until(|| fixture.cache.metrics().retiring == 0).await;
    assert_eq!(fixture.service.active_states(), 0);
    assert!(fixture.cache.is_cleanup_healthy());
    fixture.finish().await;
}

#[tokio::test]
async fn failed_group_cleanup_closes_future_admission_even_after_capacity_returns() {
    // Arrange
    let fixture = Fixture::new();
    drop(
        bounded(fixture.acquire(4, "utility-a").unwrap())
            .await
            .unwrap(),
    );
    fixture
        .client
        .set_release_object_error(Some("marker group release failed".into()));
    let release_gate = Arc::new(Notify::new());
    fixture
        .client
        .set_release_object_group_gate(Some(release_gate.clone()));
    drop(
        bounded(fixture.acquire(4, "utility-b").unwrap())
            .await
            .unwrap(),
    );
    settle_until(|| fixture.releases().len() == 1).await;
    fixture.client.set_release_object_group_gate(None);
    release_gate.notify_waiters();
    settle_until(|| fixture.cleanup.metrics().failed == 1).await;
    let before_factories = fixture.factories.load(Ordering::Acquire);
    let before_calls = fixture.client.calls().len();

    // Act
    let refused = fixture.acquire(5, "utility-c");

    // Assert: no ignored error or retry restores native ownership authority.
    assert!(refused.is_err());
    assert!(!fixture.cache.is_cleanup_healthy());
    assert!(!fixture.service.is_cleanup_healthy());
    // Failed ownership allowance A has returned, but that is not a native ACK
    // and cannot reopen admission. Only current owner B still holds two units.
    assert_eq!(fixture.service.active_states(), 1);
    assert_eq!(
        fixture
            .resources
            .metrics()
            .unwrap()
            .class(ResourceClass::RemoteHandle)
            .current,
        2
    );
    assert_eq!(fixture.factories.load(Ordering::Acquire), before_factories);
    assert_eq!(fixture.client.calls().len(), before_calls);
    fixture.client.set_release_object_error(None);
    fixture.cache.seal();
    fixture.cache.close();
    let creation = bounded(fixture.creation.shutdown()).await;
    let cleanup = bounded(fixture.cleanup.shutdown()).await;
    assert_eq!(creation.metrics.failed, 0);
    assert_eq!(cleanup.metrics.failed, 1);
    assert_eq!(cleanup.metrics.timed_out, 0);
    assert_eq!(cleanup.metrics.reserved, 0);
    assert_eq!(cleanup.metrics.in_flight, 0);
    assert_eq!(fixture.service.active_states(), 0);
    assert!(!fixture.cache.is_cleanup_healthy());
    let resources = fixture.resources.clone();
    drop(fixture);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn close_does_not_release_a_state_while_a_previously_admitted_lease_owns_it() {
    // Arrange
    let fixture = Fixture::new();
    let lease = bounded(fixture.acquire(4, "utility-a").unwrap())
        .await
        .unwrap();

    // Act
    fixture.cache.seal();
    fixture.cache.close();

    // Assert
    assert_eq!(fixture.service.active_states(), 1);
    assert!(fixture.releases().is_empty());
    assert!(fixture.acquire(4, "utility-a").is_err());
    drop(lease);
    settle_until(|| fixture.service.active_states() == 0).await;
    assert_eq!(fixture.releases().len(), 1);
    fixture.finish().await;
}

#[tokio::test]
async fn identical_world_text_on_a_distinct_client_never_coalesces_native_owners() {
    // Arrange
    let fixture = Fixture::new();
    let first = bounded(fixture.acquire(4, "utility-a").unwrap())
        .await
        .unwrap();
    let other = Arc::new(FakeCdpClient::from_html(""));
    other.set_evaluate_response(json!({
        "result":{"type":"object","objectId":"other-private-marker"}
    }));
    let key = MarkerKey::new(
        ObjectId::new(2),
        ObjectId::new(4),
        other.clone(),
        "utility-a".into(),
    )
    .unwrap();

    // Act
    let replacement = bounded(
        fixture
            .cache
            .acquire(key, || {
                Ok((
                    MarkerWorld {
                        client: other.clone(),
                        utility_world: "utility-a".into(),
                        destruction: None,
                    },
                    Arc::new(Route::new()) as Arc<dyn MarkerRoute>,
                ))
            })
            .unwrap(),
    )
    .await
    .unwrap();

    // Assert
    assert_eq!(fixture.creations(), 1);
    assert_eq!(other.method_names(), ["Runtime.callFunctionOn"]);
    assert_eq!(fixture.service.active_states(), 2);
    drop((first, replacement));
    fixture.finish().await;
    assert_eq!(
        other.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
    );
}

#[tokio::test]
async fn same_world_text_in_distinct_frames_keeps_separate_current_entries() {
    // Arrange
    let fixture = Fixture::new();
    let first = bounded(fixture.acquire(4, "utility-a").unwrap())
        .await
        .unwrap();

    // Act
    let second = bounded(fixture.acquire(5, "utility-a").unwrap())
        .await
        .unwrap();

    // Assert
    assert_eq!(fixture.factories.load(Ordering::Acquire), 2);
    assert_eq!(fixture.creations(), 2);
    assert_eq!(fixture.cache.metrics().current, 2);
    assert_eq!(fixture.cache.metrics().retiring, 0);
    drop((first, second));
    fixture.finish().await;
}

#[tokio::test]
async fn sequential_distinct_frame_retirement_reclaims_historical_current_capacity() {
    // Arrange: this is the route-liveness port, not fabricated DOM topology.
    let fixture = Fixture::new();
    let iterations = CURRENT_WORLDS + RETIRING_WORLDS + 1;
    let mut previous: Option<std::sync::Weak<Route>> = None;

    // Act: more than a full cache's history, but only one current Frame at a time.
    for index in 0..iterations {
        if let Some(previous) = previous.take() {
            previous
                .upgrade()
                .expect("cached route still owned before retirement")
                .current
                .store(false, Ordering::Release);
        }
        let route = Arc::new(Route::new());
        previous = Some(Arc::downgrade(&route));
        drop(
            bounded(
                fixture
                    .acquire_with_route(4 + index as u64, "utility-a", route)
                    .unwrap(),
            )
            .await
            .unwrap(),
        );
        settle_until(|| fixture.cache.metrics().retiring == 0).await;
        assert_eq!(fixture.cache.metrics().current, 1);
        assert_eq!(fixture.service.active_states(), 1);
    }

    // Assert: old different-Frame keys do not consume permanent historical slots.
    assert_eq!(fixture.factories.load(Ordering::Acquire), iterations);
    assert_eq!(fixture.creations(), iterations);
    assert_eq!(fixture.releases().len(), iterations - 1);
    let last = previous.unwrap();
    fixture.finish().await;
    assert!(last.upgrade().is_none());
}

#[tokio::test]
async fn a_retiring_world_cannot_revive_and_its_stale_receipt_cannot_clear_a_reused_slot() {
    // Arrange: A1 remains at the raw reply boundary while B publishes.
    let fixture = Fixture::new();
    let gate = Arc::new(Notify::new());
    fixture.client.set_evaluate_gate(Some(gate.clone()));
    let old_a = fixture.acquire(4, "utility-a").unwrap();
    settle_until(|| fixture.creations() == 1).await;
    fixture.client.set_evaluate_gate(None);
    drop(
        bounded(fixture.acquire(4, "utility-b").unwrap())
            .await
            .unwrap(),
    );
    let before_factories = fixture.factories.load(Ordering::Acquire);

    // Act
    let premature_a = fixture.acquire(4, "utility-a");

    // Assert: one exact world cannot own two independent marker states.
    assert!(premature_a.is_err());
    assert_eq!(fixture.factories.load(Ordering::Acquire), before_factories);
    assert_eq!(fixture.creations(), 2);
    gate.notify_waiters();
    settle_until(|| fixture.cache.metrics().retiring == 0).await;
    assert_eq!(fixture.releases().len(), 1);
    // Keep A1's now-failed lookup receipt alive until A2 occupies its free slot.
    let new_a = bounded(fixture.acquire(4, "utility-a").unwrap())
        .await
        .unwrap();
    let old_outcome = bounded(old_a).await;
    assert!(old_outcome.is_err());
    settle_until(|| fixture.cache.metrics().retiring == 0).await;
    assert_eq!(fixture.cache.metrics().current, 1);
    assert_eq!(fixture.cache.metrics().waiters, 1);
    assert_eq!(fixture.service.active_states(), 1);
    assert_eq!(fixture.releases().len(), 2);
    let again = bounded(fixture.acquire(4, "utility-a").unwrap())
        .await
        .unwrap();
    assert_eq!(fixture.factories.load(Ordering::Acquire), 3);
    assert_eq!(fixture.creations(), 3);
    drop((new_a, again));
    fixture.finish().await;
}

struct LockCheckingWake {
    inner: std::sync::Weak<Inner>,
    wakes: AtomicUsize,
    lock_available: AtomicBool,
}
impl Wake for LockCheckingWake {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        // Diagnostic access observes the production lock; it never changes state.
        let available = self
            .inner
            .upgrade()
            .is_some_and(|inner| inner.state.try_lock().is_ok());
        self.lock_available.fetch_and(available, Ordering::AcqRel);
        self.wakes.fetch_add(1, Ordering::AcqRel);
    }
}

#[tokio::test]
async fn host_prune_without_new_acquisition_signals_stale_retirement_and_ack_outside_lock() {
    // Arrange
    let fixture = Fixture::new();
    let route = Arc::new(Route::new());
    drop(
        bounded(
            fixture
                .acquire_with_route(4, "utility-a", route.clone())
                .unwrap(),
        )
        .await
        .unwrap(),
    );
    let release_gate = Arc::new(Notify::new());
    fixture
        .client
        .set_release_object_group_gate(Some(release_gate.clone()));
    let observer = Arc::new(LockCheckingWake {
        inner: Arc::downgrade(&fixture.cache.0),
        wakes: AtomicUsize::new(0),
        lock_available: AtomicBool::new(true),
    });
    let waker = Waker::from(observer.clone());
    let mut changed = Box::pin(fixture.cache.changed());
    assert!(
        changed
            .as_mut()
            .poll(&mut Context::from_waker(&waker))
            .is_pending()
    );
    route.current.store(false, Ordering::Release);

    // Act: no next acquire is needed to remove a closed/removed route.
    fixture.cache.prune();

    // Assert: both the prune event and native ACK notify outside the cache lock.
    assert_eq!(fixture.factories.load(Ordering::Acquire), 1);
    assert_eq!(fixture.creations(), 1);
    assert_eq!(fixture.cache.metrics().current, 0);
    assert_eq!(fixture.cache.metrics().retiring, 1);
    assert_eq!(observer.wakes.load(Ordering::Acquire), 1);
    assert!(observer.lock_available.load(Ordering::Acquire));
    assert!(poll_now(changed.as_mut()).is_ready());
    drop(changed);
    settle_until(|| fixture.releases().len() == 1).await;
    let mut acknowledged = Box::pin(fixture.cache.changed());
    assert!(
        acknowledged
            .as_mut()
            .poll(&mut Context::from_waker(&waker))
            .is_pending()
    );
    release_gate.notify_waiters();
    settle_until(|| fixture.cache.metrics().retiring == 0).await;
    assert_eq!(observer.wakes.load(Ordering::Acquire), 2);
    assert!(observer.lock_available.load(Ordering::Acquire));
    assert!(poll_now(acknowledged.as_mut()).is_ready());
    drop((acknowledged, route, waker, observer));
    fixture.finish().await;
}

#[tokio::test]
async fn held_ready_lease_cannot_mark_after_host_prune_but_keeps_native_state_until_drop() {
    // Arrange: both marker and adopted target use real composition services.
    let fixture = Fixture::new();
    let route = Arc::new(Route::new());
    let lease = bounded(
        fixture
            .acquire_with_route(4, "utility-a", route.clone())
            .unwrap(),
    )
    .await
    .unwrap();
    let profile = ResolutionProfile::new(ResourceScope::extension(183, 94), 2)
        .unwrap()
        .with_retained_reference()
        .unwrap();
    let resources = production(profile.requirements());
    let cleanup = TokioCleanupExecutor::new(1);
    let raw = Arc::new(
        CdpLocatorResolutionService::new(
            profile,
            &resources,
            Arc::new(AtomicObjectIdSource::new(1).unwrap()),
        )
        .unwrap(),
    );
    let resolver = ContinuableResolutionService::new(raw.clone(), cleanup.clone());
    fixture.client.set_object_backend_node("source-element", 17);
    fixture.client.set_adopted_node_response(
        17,
        23,
        json!({"object":{"type":"object","subtype":"node","objectId":"adopted-target"}}),
    );
    fixture
        .client
        .set_evaluate_response(json!({"result":{"type":"boolean","value":true}}));
    let element = bounded(resolver.call_retained_retaining(
        RetainedElementRequest {
            source_client: fixture.client.clone(),
            source_object_id: "source-element".into(),
            client: fixture.client.clone(),
            utility_world: "utility-a".into(),
            utility_execution_context_id: 23,
            destruction: None,
        },
        None,
    ))
    .await
    .unwrap();
    let group = element.element().object_group().to_owned();
    let calls_before = fixture.client.calls().len();
    route.current.store(false, Ordering::Release);
    fixture.cache.prune();
    assert_eq!(fixture.cache.metrics().retiring, 1);
    assert_eq!(fixture.cache.metrics().waiters, 1);
    assert_eq!(fixture.service.active_states(), 1);
    assert!(fixture.releases().is_empty());

    // Act
    let rejected = lease.mark(element, "call@1".into());

    // Assert: refusal performs no mark; target cleanup still settles independently.
    assert!(matches!(rejected, Err(MarkerCacheError::Host(_))));
    assert_eq!(fixture.client.calls().len(), calls_before);
    let report = bounded(cleanup.shutdown()).await;
    assert_eq!(report.metrics.failed, 0);
    assert_eq!(report.metrics.timed_out, 0);
    assert_eq!(report.metrics.reserved, 0);
    assert!(raw.is_cleanup_healthy());
    assert_eq!(fixture.releases(), [group]);
    assert_eq!(fixture.service.active_states(), 1);
    assert_eq!(fixture.cache.metrics().waiters, 1);
    drop((lease, route, resolver, raw));
    settle_until(|| fixture.service.active_states() == 0).await;
    assert_eq!(fixture.releases().len(), 2);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
    fixture.finish().await;
}

#[tokio::test]
async fn initialization_that_finishes_after_route_staleness_cannot_publish_without_host_prune() {
    // Arrange: keep a real raw creation outstanding, then invalidate its route.
    let fixture = Fixture::new();
    let route = Arc::new(Route::new());
    let gate = Arc::new(Notify::new());
    fixture.client.set_evaluate_gate(Some(gate.clone()));
    let lookup = fixture
        .acquire_with_route(4, "utility-a", route.clone())
        .unwrap();
    settle_until(|| fixture.creations() == 1).await;
    route.current.store(false, Ordering::Release);

    // Act: deliberately no prune/acquire between invalidation and native reply.
    gate.notify_waiters();
    let result = bounded(lookup).await;

    // Assert: a valid raw RemoteObject is not permission to publish a stale world.
    assert!(result.is_err());
    settle_until(|| fixture.service.active_states() == 0).await;
    assert_eq!(fixture.creations(), 1);
    assert_eq!(fixture.releases().len(), 1);
    assert_eq!(fixture.cache.metrics().current, 0);
    assert_eq!(fixture.cache.metrics().creating, 0);
    assert_eq!(fixture.cache.metrics().retiring, 0);
    assert_eq!(fixture.cache.metrics().waiters, 0);
    drop(route);
    fixture.finish().await;
}
