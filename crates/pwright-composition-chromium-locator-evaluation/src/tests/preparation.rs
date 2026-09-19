use super::*;
use crate::preparation::*;
use pwright_composition_chromium_locator_resolution::{
    ResolutionRoute, ResolutionWaitError, ResolutionWaitProfile, ResolutionWaitStop,
    StrictResolutionWaiter,
};
use pwright_runtime::DeadlineScheduler;
use std::{collections::VecDeque, sync::Mutex, time::Duration};

struct Pair {
    client: Arc<FakeCdpClient>,
    checks: AtomicUsize,
    retire_at: usize,
    main: &'static str,
}
impl PairedWorld for Pair {
    fn utility_route(&self) -> ResolutionRoute {
        ResolutionRoute {
            client: self.client.clone(),
            utility_world: "utility".into(),
            destruction: None,
        }
    }
    fn main_route(&self) -> MainWorldRoute {
        MainWorldRoute {
            client: self.client.clone(),
            execution_context_id: 29,
            unique_context_id: self.main.into(),
        }
    }
    fn is_current(&self) -> Result<bool, ResolutionWaitError> {
        Ok(self.checks.fetch_add(1, Ordering::AcqRel) + 1 < self.retire_at)
    }
}
struct Worlds {
    pairs: Mutex<VecDeque<Arc<Pair>>>,
    blocked: AtomicBool,
    requests: AtomicUsize,
}
impl PairedWorldSource for Worlds {
    type Pair = Pair;
    type Current<'a> =
        Pin<Box<dyn Future<Output = Result<Arc<Pair>, ResolutionWaitError>> + Send + 'a>>;
    fn current(&self) -> Self::Current<'_> {
        self.requests.fetch_add(1, Ordering::Relaxed);
        Box::pin(async {
            if self.blocked.load(Ordering::Acquire) {
                std::future::pending::<()>().await;
            }
            self.pairs
                .lock()
                .unwrap()
                .pop_front()
                .ok_or(ResolutionWaitStop::TargetClosed.into())
        })
    }
}
#[derive(Default)]
struct Delays {
    requested: Mutex<Vec<Duration>>,
    wake: Notify,
}
impl DeadlineScheduler for Delays {
    type Sleep<'a> = tokio::sync::futures::Notified<'a>;
    fn sleep(&self, delay: Duration) -> Option<Self::Sleep<'_>> {
        self.requested.lock().unwrap().push(delay);
        Some(self.wake.notified())
    }
}
struct Phase {
    base: Fixture,
    waiter: StrictResolutionWaiter,
    preparation: LocatorElementPreparationService,
    worlds: Worlds,
    delays: Delays,
}
impl Phase {
    fn new() -> Self {
        let wait = ResolutionWaitProfile::new(
            ResourceScope::extension(93, 1),
            1,
            [0, 20, 50, 100, 100, 500].map(Duration::from_millis),
        )
        .unwrap();
        let profile = PreparationProfile::new(
            ResourceScope::extension(93, 2),
            1,
            Duration::from_millis(20),
        )
        .unwrap();
        let base = Fixture::with_resources(
            4,
            wait.requirements()
                .combine_concurrent(profile.requirements()),
        );
        base.client.set_evaluate_response(
            json!({"result":{"type":"object","subtype":"node","objectId":"utility-node"}}),
        );
        Self {
            waiter: StrictResolutionWaiter::new(wait, &base.resources).unwrap(),
            preparation: LocatorElementPreparationService::new(profile, &base.resources).unwrap(),
            base,
            worlds: Worlds {
                pairs: Mutex::new(VecDeque::new()),
                blocked: AtomicBool::new(false),
                requests: AtomicUsize::new(0),
            },
            delays: Delays::default(),
        }
    }
    fn pair(&self, retire_at: usize) -> std::sync::Weak<Pair> {
        let pair = Arc::new(Pair {
            client: self.base.client.clone(),
            checks: AtomicUsize::new(0),
            retire_at,
            main: "main",
        });
        let weak = Arc::downgrade(&pair);
        self.worlds.pairs.lock().unwrap().push_back(pair);
        weak
    }
    fn inputs(
        &self,
    ) -> PreparationInputs<'_, CdpLocatorResolutionService, CleanupQueue, Worlds, Delays> {
        PreparationInputs {
            attempts: &self.base.resolve,
            waiter: &self.waiter,
            adoption: &self.base.service,
            worlds: &self.worlds,
            delays: &self.delays,
        }
    }
    fn idle(&self) {
        assert_eq!(self.base.resources.metrics().unwrap().active_leases, 3);
        assert_eq!(self.base.queue.metrics().unwrap().reserved, 0);
    }
}
fn selector() -> Selector {
    Selector::css("button").unwrap()
}
fn never() -> impl Future<Output = ResolutionWaitStop> {
    std::future::pending()
}

#[tokio::test]
async fn preparation_returns_one_adopted_owner_without_executing_user_code() {
    // Arrange
    let f = Phase::new();
    let retained = f.pair(usize::MAX);
    let inputs = f.inputs();
    // Act
    let prepared = f
        .preparation
        .prepare(selector(), &inputs, never())
        .await
        .unwrap();
    // Assert
    assert_eq!(
        f.base.client.method_names(),
        [
            "Runtime.callFunctionOn",
            "DOM.describeNode",
            "DOM.resolveNode"
        ]
    );
    assert!(
        retained.upgrade().is_some(),
        "returned owner retains paired route after phase returns"
    );
    prepared.release().await.unwrap();
    assert!(retained.upgrade().is_none());
    f.idle();
}

#[test]
fn preparation_admits_unpolled_work_across_clones_and_returns_capacity_on_drop() {
    // Arrange
    let f = Phase::new();
    let inputs = f.inputs();
    let first = f.preparation.prepare(selector(), &inputs, never());
    let clone = f.preparation.clone();
    // Act
    let mut rejected = Box::pin(clone.prepare(selector(), &inputs, never()));
    let result = poll(rejected.as_mut());
    // Assert
    assert!(matches!(
        result,
        Poll::Ready(Err(PreparationError::Adoption(
            ElementEvaluationError::Admission(_)
        )))
    ));
    assert!(
        std::mem::size_of_val(&first) < 4096,
        "preparation future is {} bytes; prepared owner is {} bytes",
        std::mem::size_of_val(&first),
        std::mem::size_of::<Prepared>()
    );
    assert!(f.base.client.calls().is_empty());
    drop(first);
    f.idle();
}

#[tokio::test]
async fn already_expired_stop_wins_before_route_or_native_work() {
    // Arrange
    let f = Phase::new();
    let inputs = f.inputs();
    // Act
    let result = f
        .preparation
        .prepare(
            selector(),
            &inputs,
            std::future::ready(ResolutionWaitStop::Timeout),
        )
        .await;
    // Assert
    assert!(matches!(
        result,
        Err(PreparationError::Waiting(ResolutionWaitError::Stopped(
            ResolutionWaitStop::Timeout
        )))
    ));
    assert!(f.base.client.calls().is_empty());
    assert_eq!(f.worlds.requests.load(Ordering::Relaxed), 0);
    f.idle();
}

#[tokio::test]
async fn original_stop_terminates_pending_paired_world_readiness() {
    // Arrange
    let f = Phase::new();
    f.worlds.blocked.store(true, Ordering::Release);
    let stop = Notify::new();
    let inputs = f.inputs();
    let mut future = Box::pin(f.preparation.prepare(selector(), &inputs, async {
        stop.notified().await;
        ResolutionWaitStop::TargetClosed
    }));
    assert!(poll(future.as_mut()).is_pending());
    // Act
    stop.notify_waiters();
    let result = future.await;
    // Assert
    assert!(matches!(
        result,
        Err(PreparationError::Waiting(ResolutionWaitError::Stopped(
            ResolutionWaitStop::TargetClosed
        )))
    ));
    assert!(f.base.client.calls().is_empty());
    f.idle();
}

#[tokio::test]
async fn cancelled_native_query_retains_pair_until_original_query_and_cleanup_finish() {
    // Arrange
    let f = Phase::new();
    let retained = f.pair(usize::MAX);
    let inputs = f.inputs();
    let query = Arc::new(Notify::new());
    f.base.client.set_evaluate_gate(Some(query.clone()));
    let mut future = Box::pin(f.preparation.prepare(selector(), &inputs, never()));
    assert!(poll(future.as_mut()).is_pending());
    // Act
    drop(future);
    // Assert
    assert!(retained.upgrade().is_some());
    query.notify_waiters();
    f.base.drain().await;
    assert!(retained.upgrade().is_none());
    f.idle();
}

#[tokio::test]
async fn original_stop_terminates_adoption_and_retains_pair_through_its_cleanup() {
    // Arrange
    let f = Phase::new();
    let retained = f.pair(usize::MAX);
    let inputs = f.inputs();
    let gate = Arc::new(Notify::new());
    f.base.client.set_adoption_gate(Some(gate.clone()));
    let stop = Notify::new();
    let mut future = Box::pin(f.preparation.prepare(selector(), &inputs, async {
        stop.notified().await;
        ResolutionWaitStop::Timeout
    }));
    assert!(poll(future.as_mut()).is_pending());
    // Act
    stop.notify_waiters();
    let result = future.await;
    // Assert
    assert!(matches!(
        result,
        Err(PreparationError::Waiting(ResolutionWaitError::Stopped(
            ResolutionWaitStop::Timeout
        )))
    ));
    assert!(retained.upgrade().is_some());
    gate.notify_waiters();
    f.base.drain().await;
    assert!(retained.upgrade().is_none());
    f.idle();
}

#[tokio::test]
async fn stale_successful_adoption_releases_before_retrying_and_never_executes_user_code() {
    // Arrange
    let f = Phase::new();
    let old = f.pair(4);
    let next = f.pair(usize::MAX);
    let inputs = f.inputs();
    let mut future = Box::pin(f.preparation.prepare(selector(), &inputs, never()));
    // Act
    assert!(poll(future.as_mut()).is_pending());
    // Assert
    assert_eq!(
        f.base.client.method_names(),
        [
            "Runtime.callFunctionOn",
            "DOM.describeNode",
            "DOM.resolveNode",
            "Runtime.releaseObjectGroup"
        ]
    );
    assert!(old.upgrade().is_none());
    assert_eq!(
        *f.delays.requested.lock().unwrap(),
        [Duration::from_millis(20)]
    );
    f.delays.wake.notify_waiters();
    let prepared = future.await.unwrap();
    assert_eq!(f.worlds.requests.load(Ordering::Relaxed), 2);
    prepared.release().await.unwrap();
    assert!(next.upgrade().is_none());
    f.idle();
}

#[tokio::test]
async fn stale_selected_route_is_released_without_adoption_and_original_deadline_is_not_reset() {
    // Arrange
    let f = Phase::new();
    f.pair(3);
    let inputs = f.inputs();
    let stop = Notify::new();
    let mut future = Box::pin(f.preparation.prepare(selector(), &inputs, async {
        stop.notified().await;
        ResolutionWaitStop::Timeout
    }));
    assert!(poll(future.as_mut()).is_pending());
    // Act
    stop.notify_waiters();
    let result = future.await;
    // Assert
    assert!(matches!(
        result,
        Err(PreparationError::Waiting(ResolutionWaitError::Stopped(
            ResolutionWaitStop::Timeout
        )))
    ));
    assert_eq!(
        f.base.client.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
    );
    f.idle();
}

#[tokio::test]
async fn failed_stale_group_cleanup_is_terminal_and_never_retries() {
    // Arrange
    let f = Phase::new();
    f.pair(4);
    let inputs = f.inputs();
    f.base
        .client
        .set_release_object_error(Some("native release failed".into()));
    // Act
    let result = f.preparation.prepare(selector(), &inputs, never()).await;
    // Assert
    assert!(matches!(
        result,
        Err(PreparationError::Adoption(
            ElementEvaluationError::Cleanup { .. }
        ))
    ));
    assert_eq!(f.worlds.requests.load(Ordering::Relaxed), 1);
    assert!(f.delays.requested.lock().unwrap().is_empty());
    assert!(!f.base.service.is_cleanup_healthy());
    f.idle();
}

#[tokio::test]
async fn strict_selection_error_is_terminal_before_adoption() {
    // Arrange
    let f = Phase::new();
    f.pair(usize::MAX);
    let inputs = f.inputs();
    f.base
        .client
        .set_evaluate_response(json!({"result":{"type":"number","value":2}}));
    // Act
    let result = f.preparation.prepare(selector(), &inputs, never()).await;
    // Assert
    assert!(result.is_err());
    assert_eq!(
        f.base.client.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
    );
    assert!(f.delays.requested.lock().unwrap().is_empty());
    f.idle();
}

#[tokio::test]
async fn route_validation_rejects_identical_worlds_before_native_work() {
    // Arrange
    let f = Phase::new();
    let inputs = f.inputs();
    f.worlds.pairs.lock().unwrap().push_back(Arc::new(Pair {
        client: f.base.client.clone(),
        checks: AtomicUsize::new(0),
        retire_at: usize::MAX,
        main: "utility",
    }));
    // Act
    let result = f.preparation.prepare(selector(), &inputs, never()).await;
    // Assert
    assert!(matches!(
        result,
        Err(PreparationError::Waiting(ResolutionWaitError::Resolution(
            ResolutionError::Setup(_)
        )))
    ));
    assert!(f.base.client.calls().is_empty());
    f.idle();
}

#[tokio::test]
async fn evaluation_occurs_after_preparation_deadline_and_does_not_reselect() {
    // Arrange
    let f = Phase::new();
    let retained = f.pair(usize::MAX);
    let inputs = f.inputs();
    let stop = Notify::new();
    let prepared = f
        .preparation
        .prepare(selector(), &inputs, async {
            stop.notified().await;
            ResolutionWaitStop::Timeout
        })
        .await
        .unwrap();
    stop.notify_waiters();
    f.base.value();
    // Act
    let value = prepared.evaluate(&params()).await.unwrap();
    // Assert
    assert_eq!(value, EvaluationValue::String("done".into()));
    assert_eq!(f.worlds.requests.load(Ordering::Relaxed), 1);
    assert!(retained.upgrade().is_none());
    f.idle();
}

#[test]
fn invalid_preparation_profiles_and_undeclared_resources_reject() {
    // Arrange
    let scope = ResourceScope::extension(93, 2);
    let profile = PreparationProfile::new(scope, 1, Duration::from_millis(20)).unwrap();
    // Act
    let invalid = [
        PreparationProfile::new(scope, 0, Duration::from_millis(20)),
        PreparationProfile::new(scope, 1, Duration::ZERO),
        PreparationProfile::new(scope, usize::MAX, Duration::from_millis(20)),
    ];
    let undeclared =
        LocatorElementPreparationService::new(profile, &resources(ResourceRequirements::none()));
    // Assert
    assert!(invalid.iter().all(Result::is_err));
    assert!(undeclared.is_err());
}

struct OversizedStop([u8; 16 * 1024]);
impl Future for OversizedStop {
    type Output = ResolutionWaitStop;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        std::hint::black_box(&self.0);
        Poll::Pending
    }
}

#[tokio::test]
async fn oversized_concrete_future_rejects_before_boxing_or_route_work() {
    // Arrange
    let f = Phase::new();
    let inputs = f.inputs();
    // Act
    let result = f
        .preparation
        .prepare(selector(), &inputs, OversizedStop([0; 16 * 1024]))
        .await;
    // Assert
    assert!(matches!(
        result,
        Err(PreparationError::Adoption(
            ElementEvaluationError::Admission(
                "concrete preparation future exceeds its declared storage bound"
            )
        ))
    ));
    assert_eq!(f.worlds.requests.load(Ordering::Relaxed), 0);
    assert!(f.base.client.calls().is_empty());
    f.idle();
}

#[tokio::test]
async fn cancellation_during_stale_group_release_retains_pair_and_never_starts_another_query() {
    // Arrange
    let f = Phase::new();
    let retained = f.pair(4);
    let inputs = f.inputs();
    let release = Arc::new(Notify::new());
    f.base
        .client
        .set_release_object_group_gate(Some(release.clone()));
    let stop = Notify::new();
    let mut future = Box::pin(f.preparation.prepare(selector(), &inputs, async {
        stop.notified().await;
        ResolutionWaitStop::Cancelled
    }));
    assert!(poll(future.as_mut()).is_pending());
    // Act
    stop.notify_waiters();
    let result = future.await;
    // Assert
    assert!(matches!(
        result,
        Err(PreparationError::Waiting(ResolutionWaitError::Stopped(
            ResolutionWaitStop::Cancelled
        )))
    ));
    assert!(retained.upgrade().is_some());
    release.notify_waiters();
    f.base.drain().await;
    assert!(retained.upgrade().is_none());
    assert_eq!(f.worlds.requests.load(Ordering::Relaxed), 1);
    f.idle();
}

#[tokio::test]
async fn route_owner_survives_cancelled_user_evaluation_until_native_group_release() {
    // Arrange
    let f = Phase::new();
    let retained = f.pair(usize::MAX);
    let inputs = f.inputs();
    let prepared = f
        .preparation
        .prepare(selector(), &inputs, never())
        .await
        .unwrap();
    f.base.value();
    let gate = Arc::new(Notify::new());
    f.base.client.set_evaluate_gate(Some(gate.clone()));
    let mut evaluation = Box::pin(prepared.evaluate(&params()));
    assert!(poll(evaluation.as_mut()).is_pending());
    // Act
    drop(evaluation);
    // Assert
    assert!(retained.upgrade().is_some());
    gate.notify_waiters();
    f.base.drain().await;
    assert!(retained.upgrade().is_none());
    assert_eq!(f.worlds.requests.load(Ordering::Relaxed), 1);
    f.idle();
}

struct OversizedWorldFuture([u8; 4096]);
impl Future for OversizedWorldFuture {
    type Output = Result<Arc<Pair>, ResolutionWaitError>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        std::hint::black_box(&self.0);
        Poll::Pending
    }
}
struct OversizedWorlds(AtomicUsize);
impl PairedWorldSource for OversizedWorlds {
    type Pair = Pair;
    type Current<'a> = OversizedWorldFuture;
    fn current(&self) -> Self::Current<'_> {
        self.0.fetch_add(1, Ordering::Relaxed);
        OversizedWorldFuture([0; 4096])
    }
}

#[tokio::test]
async fn oversized_route_future_rejects_before_allocating_or_invoking_its_source() {
    // Arrange
    let f = Phase::new();
    let worlds = OversizedWorlds(AtomicUsize::new(0));
    let inputs = PreparationInputs {
        attempts: &f.base.resolve,
        waiter: &f.waiter,
        adoption: &f.base.service,
        worlds: &worlds,
        delays: &f.delays,
    };
    // Act
    let result = f.preparation.prepare(selector(), &inputs, never()).await;
    // Assert
    assert!(matches!(
        result,
        Err(PreparationError::Waiting(ResolutionWaitError::Resolution(
            ResolutionError::Setup(
                "concrete paired route future exceeds its declared storage bound"
            )
        )))
    ));
    assert_eq!(worlds.0.load(Ordering::Relaxed), 0);
    assert!(f.base.client.calls().is_empty());
    f.idle();
}
