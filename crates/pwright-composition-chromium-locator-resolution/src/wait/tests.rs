use super::*;
use pwright_core::{AtomicObjectIdSource, ResourceLimits};
use pwright_fake::FakeCdpClient;
use pwright_runtime::{CleanupQueue, NoDeadlineScheduler};
use serde_json::json;
use std::{
    pin::Pin,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Waker},
};
use tokio::sync::Notify;

type Attempts = ContinuableResolutionService<crate::CdpLocatorResolutionService, CleanupQueue>;

struct Routes {
    route: Mutex<ResolutionRoute>,
    closed: AtomicBool,
    blocked: AtomicBool,
    ready: Notify,
    reads: AtomicUsize,
}
impl ResolutionRouteSource for Routes {
    type Current<'a> =
        Pin<Box<dyn Future<Output = Result<ResolutionRoute, ResolutionWaitError>> + Send + 'a>>;
    fn current(&self) -> Self::Current<'_> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        Box::pin(async {
            if self.blocked.load(Ordering::Acquire) {
                self.ready.notified().await;
            }
            if self.closed.load(Ordering::Acquire) {
                return Err(ResolutionWaitStop::TargetClosed.into());
            }
            Ok(self.route.lock().unwrap().clone())
        })
    }
    fn is_current(&self, route: &ResolutionRoute) -> Result<bool, ResolutionWaitError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(ResolutionWaitStop::TargetClosed.into());
        }
        let current = self.route.lock().unwrap();
        Ok(Arc::ptr_eq(&current.client, &route.client)
            && current.utility_world == route.utility_world)
    }
}

#[derive(Default)]
struct Delays {
    calls: Mutex<Vec<Duration>>,
    ready: Notify,
}
impl DeadlineScheduler for Delays {
    type Sleep<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
    fn sleep(&self, delay: Duration) -> Option<Self::Sleep<'_>> {
        self.calls.lock().unwrap().push(delay);
        Some(Box::pin(self.ready.notified()))
    }
}

fn exact_resources(requirements: ResourceRequirements) -> ProductionResourceLedger {
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let bound = requirements.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    ProductionResourceLedger::new(limits, requirements).unwrap()
}
fn profile(capacity: usize) -> Result<ResolutionWaitProfile, ResolutionError> {
    ResolutionWaitProfile::new(
        ResourceScope::extension(89, 2),
        capacity,
        [0, 20, 50, 100, 100, 500].map(Duration::from_millis),
    )
}
struct Fixture {
    attempts: Attempts,
    waiter: StrictResolutionWaiter,
    client: Arc<FakeCdpClient>,
    routes: Routes,
    delays: Delays,
    queue: CleanupQueue,
    resources: ProductionResourceLedger,
}
impl Fixture {
    fn new() -> Self {
        let resolution = crate::ResolutionProfile::new(ResourceScope::extension(89, 1), 1).unwrap();
        let waiting = profile(1).unwrap();
        let resources = exact_resources(
            resolution
                .requirements()
                .combine_concurrent(waiting.requirements()),
        );
        let raw = crate::CdpLocatorResolutionService::new(
            resolution,
            &resources,
            Arc::new(AtomicObjectIdSource::new(1).unwrap()),
        )
        .unwrap();
        let queue = CleanupQueue::new(1);
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(node("selected"));
        let routes = Routes {
            route: Mutex::new(ResolutionRoute {
                client: client.clone(),
                utility_world: "utility-1".into(),
                destruction: None,
            }),
            closed: AtomicBool::new(false),
            blocked: AtomicBool::new(false),
            ready: Notify::new(),
            reads: AtomicUsize::new(0),
        };
        Self {
            attempts: ContinuableResolutionService::new(Arc::new(raw), queue.clone()),
            waiter: StrictResolutionWaiter::new(waiting, &resources).unwrap(),
            client,
            routes,
            delays: Delays::default(),
            queue,
            resources,
        }
    }
    fn wait<'a>(
        &'a self,
        stop: impl Future<Output = ResolutionWaitStop> + Send + 'a,
    ) -> impl Future<
        Output = Result<
            OwnedElement<pwright_runtime::CleanupQueueReservation>,
            ResolutionWaitError,
        >,
    > + Send {
        self.waiter.wait(
            Selector::css("button").unwrap(),
            &self.attempts,
            &self.routes,
            &self.delays,
            stop,
        )
    }
    fn replace_world(&self) {
        self.routes.route.lock().unwrap().utility_world = "utility-2".into();
    }
    fn active_leases(&self) -> usize {
        // The installed waiter's fixed admission counter keeps one state lease.
        self.resources.metrics().unwrap().active_leases - 1
    }
}
fn node(id: &str) -> serde_json::Value {
    json!({"result":{"type":"object","subtype":"node","objectId":id}})
}
fn missing() -> serde_json::Value {
    json!({"result":{"type":"number","value":0}})
}
fn poll<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}
fn never() -> impl Future<Output = ResolutionWaitStop> {
    std::future::pending()
}

#[tokio::test]
async fn immediate_selection_finishes_wait_admission_but_retains_exact_owner() {
    // Arrange
    let f = Fixture::new();
    // Act
    let element = f.wait(never()).await.unwrap();
    // Assert
    assert_eq!(element.element().object_id(), "selected");
    assert_eq!(f.client.method_names(), ["Runtime.callFunctionOn"]);
    assert!(f.delays.calls.lock().unwrap().is_empty());
    assert_eq!(f.active_leases(), 1);
    element.release().await.unwrap();
    assert_eq!(f.active_leases(), 0);
}

#[tokio::test]
async fn retries_use_injected_backoff_and_release_every_missing_group_before_delay() {
    // Arrange
    let f = Fixture::new();
    f.client
        .set_evaluate_responses(std::iter::repeat_n(missing(), 7).chain([node("later")]));
    let mut waiting = Box::pin(f.wait(never()));
    // Act
    for index in 0..6 {
        assert!(poll(waiting.as_mut()).is_pending());
        assert_eq!(
            f.client.method_names().last().unwrap(),
            "Runtime.releaseObjectGroup"
        );
        assert_eq!(f.queue.metrics().unwrap().reserved, 0);
        assert_eq!(f.active_leases(), 1);
        assert_eq!(f.delays.calls.lock().unwrap().len(), index + 1);
        f.delays.ready.notify_waiters();
    }
    let element = waiting.await.unwrap();
    // Assert
    assert_eq!(
        *f.delays.calls.lock().unwrap(),
        [20, 50, 100, 100, 500, 500].map(Duration::from_millis)
    );
    assert_eq!(element.element().object_id(), "later");
    assert_eq!(f.routes.reads.load(Ordering::Relaxed), 8);
    element.release().await.unwrap();
    assert_eq!(f.active_leases(), 0);
}

#[tokio::test]
async fn already_expired_deadline_wins_before_any_route_or_browser_work() {
    // Arrange
    let f = Fixture::new();
    // Act
    let result = f
        .wait(std::future::ready(ResolutionWaitStop::Timeout))
        .await;
    // Assert
    assert!(matches!(
        result,
        Err(ResolutionWaitError::Stopped(ResolutionWaitStop::Timeout))
    ));
    assert_eq!(f.routes.reads.load(Ordering::Relaxed), 0);
    assert!(f.client.calls().is_empty());
    assert_eq!(f.active_leases(), 0);
}

#[tokio::test]
async fn one_deadline_survives_retries_and_cancels_pending_pacing() {
    // Arrange
    let f = Fixture::new();
    f.client.set_evaluate_response(missing());
    let deadline = Notify::new();
    let mut waiting = Box::pin(f.wait(async {
        deadline.notified().await;
        ResolutionWaitStop::Timeout
    }));
    assert!(poll(waiting.as_mut()).is_pending());
    f.delays.ready.notify_waiters();
    assert!(poll(waiting.as_mut()).is_pending());
    let before = f.client.calls().len();
    // Act
    deadline.notify_waiters();
    let result = waiting.await;
    // Assert
    assert!(matches!(
        result,
        Err(ResolutionWaitError::Stopped(ResolutionWaitStop::Timeout))
    ));
    assert_eq!(f.client.calls().len(), before);
    assert_eq!(
        *f.delays.calls.lock().unwrap(),
        [20, 50].map(Duration::from_millis)
    );
    assert_eq!(f.active_leases(), 0);
    assert_eq!(f.queue.metrics().unwrap().queued, 0);
}

#[tokio::test]
async fn stop_while_route_is_unavailable_never_dispatches_a_query() {
    // Arrange
    let f = Fixture::new();
    f.routes.blocked.store(true, Ordering::Release);
    let closed = Notify::new();
    let mut waiting = Box::pin(f.wait(async {
        closed.notified().await;
        ResolutionWaitStop::TargetClosed
    }));
    assert!(poll(waiting.as_mut()).is_pending());
    // Act
    closed.notify_waiters();
    let result = waiting.await;
    // Assert
    assert!(matches!(
        result,
        Err(ResolutionWaitError::Stopped(
            ResolutionWaitStop::TargetClosed
        ))
    ));
    assert!(f.client.calls().is_empty());
    assert_eq!(f.active_leases(), 0);
}

#[tokio::test]
async fn timeout_during_query_continues_only_that_query_and_releases_its_late_object() {
    // Arrange
    let f = Fixture::new();
    let native = Arc::new(Notify::new());
    f.client.set_evaluate_gate(Some(native.clone()));
    let deadline = Notify::new();
    let mut waiting = Box::pin(f.wait(async {
        deadline.notified().await;
        ResolutionWaitStop::Timeout
    }));
    assert!(poll(waiting.as_mut()).is_pending());
    // Act
    deadline.notify_waiters();
    let result = waiting.await;
    native.notify_waiters();
    f.queue.try_take().unwrap().unwrap().await.unwrap();
    // Assert
    assert!(matches!(
        result,
        Err(ResolutionWaitError::Stopped(ResolutionWaitStop::Timeout))
    ));
    assert_eq!(
        f.client.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
    );
    assert_eq!(f.routes.reads.load(Ordering::Relaxed), 1);
    assert_eq!(f.active_leases(), 0);
}

#[tokio::test]
async fn dropping_a_started_wait_has_the_same_exact_attempt_cleanup() {
    // Arrange
    let f = Fixture::new();
    let native = Arc::new(Notify::new());
    f.client.set_evaluate_gate(Some(native.clone()));
    let mut waiting = Box::pin(f.wait(never()));
    assert!(poll(waiting.as_mut()).is_pending());
    // Act
    drop(waiting);
    native.notify_waiters();
    f.queue.try_take().unwrap().unwrap().await.unwrap();
    // Assert
    assert_eq!(
        f.client.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
    );
    assert_eq!(f.active_leases(), 0);
}

#[test]
fn dropping_an_unpolled_wait_returns_admission_without_route_or_cleanup_work() {
    // Arrange
    let f = Fixture::new();
    let waiting = f.wait(never());
    assert!(std::mem::size_of_val(&waiting) as u64 <= WAITER_BYTES);
    assert_eq!(f.active_leases(), 1);
    // Act
    drop(waiting);
    // Assert
    assert!(f.client.calls().is_empty());
    assert_eq!(f.routes.reads.load(Ordering::Relaxed), 0);
    assert_eq!(f.queue.metrics().unwrap().queued, 0);
    assert_eq!(f.active_leases(), 0);
}

#[tokio::test]
async fn second_wait_is_rejected_before_route_lookup_at_exact_capacity() {
    // Arrange
    let f = Fixture::new();
    let first = f.wait(never());
    // Act
    let clone = f.waiter.clone();
    let second = clone
        .wait(
            Selector::css("button").unwrap(),
            &f.attempts,
            &f.routes,
            &f.delays,
            never(),
        )
        .await;
    // Assert
    assert!(matches!(
        second,
        Err(ResolutionWaitError::Resolution(ResolutionError::Admission(
            _
        )))
    ));
    assert!(f.client.calls().is_empty());
    assert_eq!(f.routes.reads.load(Ordering::Relaxed), 0);
    drop(first);
    assert_eq!(f.active_leases(), 0);
    f.wait(never()).await.unwrap().release().await.unwrap();
}

#[tokio::test]
async fn strict_violation_is_terminal_even_when_a_new_world_is_available() {
    // Arrange
    let f = Fixture::new();
    f.client
        .set_evaluate_response(json!({"result":{"type":"number","value":2}}));
    let native = Arc::new(Notify::new());
    f.client.set_evaluate_gate(Some(native.clone()));
    let mut waiting = Box::pin(f.wait(never()));
    assert!(poll(waiting.as_mut()).is_pending());
    f.replace_world();
    // Act
    native.notify_waiters();
    let result = waiting.await;
    // Assert
    assert!(matches!(
        result,
        Err(ResolutionWaitError::Resolution(
            ResolutionError::StrictViolation { matches: 2 }
        ))
    ));
    assert_eq!(
        f.client.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
    );
    assert_eq!(f.active_leases(), 0);
}

#[tokio::test]
async fn stale_success_is_released_before_selecting_in_the_replacement_world() {
    // Arrange
    let f = Fixture::new();
    f.client.set_evaluate_responses([node("old"), node("new")]);
    let native = Arc::new(Notify::new());
    f.client.set_evaluate_gate(Some(native.clone()));
    let mut waiting = Box::pin(f.wait(never()));
    assert!(poll(waiting.as_mut()).is_pending());
    f.replace_world();
    f.client.set_evaluate_gate(None);
    // Act
    native.notify_waiters();
    let element = waiting.await.unwrap();
    // Assert
    assert_eq!(element.element().object_id(), "new");
    assert_eq!(element.element().utility_world(), "utility-2");
    assert_eq!(
        f.client.method_names(),
        [
            "Runtime.callFunctionOn",
            "Runtime.releaseObjectGroup",
            "Runtime.callFunctionOn"
        ]
    );
    element.release().await.unwrap();
    assert_eq!(f.active_leases(), 0);
}

#[tokio::test]
async fn backend_error_retries_only_after_authoritative_world_invalidation() {
    // Arrange
    let f = Fixture::new();
    f.client
        .set_evaluate_errors(["Execution context was destroyed".into()]);
    let native = Arc::new(Notify::new());
    f.client.set_evaluate_gate(Some(native.clone()));
    let mut waiting = Box::pin(f.wait(never()));
    assert!(poll(waiting.as_mut()).is_pending());
    f.replace_world();
    f.client.set_evaluate_gate(None);
    // Act
    native.notify_waiters();
    let element = waiting.await.unwrap();
    // Assert
    assert_eq!(element.element().utility_world(), "utility-2");
    assert_eq!(f.routes.reads.load(Ordering::Relaxed), 2);
    element.release().await.unwrap();
    assert_eq!(f.active_leases(), 0);
}

#[tokio::test]
async fn backend_failure_in_current_world_is_not_silently_retried() {
    // Arrange
    let f = Fixture::new();
    f.client.set_evaluate_errors(["backend failed".into()]);
    // Act
    let result = f.wait(never()).await;
    // Assert
    assert!(matches!(
        result,
        Err(ResolutionWaitError::Resolution(ResolutionError::Backend(_)))
    ));
    assert_eq!(f.routes.reads.load(Ordering::Relaxed), 1);
    assert_eq!(f.active_leases(), 0);
}

#[tokio::test]
async fn cleanup_failure_on_stale_success_is_terminal_and_never_requeries() {
    // Arrange
    let f = Fixture::new();
    let native = Arc::new(Notify::new());
    f.client.set_evaluate_gate(Some(native.clone()));
    f.client
        .set_release_object_error(Some("failed release".into()));
    let mut waiting = Box::pin(f.wait(never()));
    assert!(poll(waiting.as_mut()).is_pending());
    f.replace_world();
    // Act
    native.notify_waiters();
    let result = waiting.await;
    // Assert
    assert!(matches!(
        result,
        Err(ResolutionWaitError::Resolution(
            ResolutionError::Cleanup { .. }
        ))
    ));
    assert_eq!(f.routes.reads.load(Ordering::Relaxed), 1);
    assert_eq!(f.active_leases(), 0);
}

#[tokio::test]
async fn closure_after_native_success_releases_the_unpublished_element() {
    // Arrange
    let f = Fixture::new();
    let native = Arc::new(Notify::new());
    f.client.set_evaluate_gate(Some(native.clone()));
    let mut waiting = Box::pin(f.wait(never()));
    assert!(poll(waiting.as_mut()).is_pending());
    f.routes.closed.store(true, Ordering::Release);
    // Act
    native.notify_waiters();
    let result = waiting.await;
    f.queue.try_take().unwrap().unwrap().await.unwrap();
    // Assert
    assert!(matches!(
        result,
        Err(ResolutionWaitError::Stopped(
            ResolutionWaitStop::TargetClosed
        ))
    ));
    assert_eq!(
        f.client.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
    );
    assert_eq!(f.active_leases(), 0);
}

#[tokio::test]
async fn expired_resolution_signal_does_not_revoke_the_returned_element() {
    // Arrange
    let f = Fixture::new();
    let deadline = Notify::new();
    let mut stop = std::pin::pin!(async {
        deadline.notified().await;
        ResolutionWaitStop::Timeout
    });
    let element = f.wait(stop.as_mut()).await.unwrap();
    // Act
    deadline.notify_waiters();
    f.replace_world();
    let reason = stop.await;
    // Assert
    assert_eq!(element.element().object_id(), "selected");
    assert_eq!(reason, ResolutionWaitStop::Timeout);
    assert_eq!(element.element().utility_world(), "utility-1");
    assert_eq!(f.client.calls().len(), 1);
    assert_eq!(f.active_leases(), 1);
    element.release().await.unwrap();
}

#[tokio::test]
async fn missing_retry_scheduler_fails_explicitly_after_cleaned_attempts() {
    // Arrange
    let f = Fixture::new();
    f.client.set_evaluate_response(missing());
    // Act
    let result = f
        .waiter
        .wait(
            Selector::css("button").unwrap(),
            &f.attempts,
            &f.routes,
            &NoDeadlineScheduler,
            never(),
        )
        .await;
    // Assert
    assert!(matches!(
        result,
        Err(ResolutionWaitError::Resolution(ResolutionError::Setup(_)))
    ));
    assert_eq!(f.client.calls().len(), 4);
    assert_eq!(f.active_leases(), 0);
}

#[test]
fn invalid_profiles_and_undeclared_installation_are_rejected() {
    // Arrange
    let undeclared = exact_resources(ResourceRequirements::none());
    // Act
    let zero = profile(0);
    let overflow = profile(usize::MAX);
    let spinning =
        ResolutionWaitProfile::new(ResourceScope::extension(89, 2), 1, [Duration::ZERO; 6]);
    let unplanned = StrictResolutionWaiter::new(profile(1).unwrap(), &undeclared);
    // Assert
    assert!(matches!(zero, Err(ResolutionError::Setup(_))));
    assert!(matches!(overflow, Err(ResolutionError::Setup(_))));
    assert!(matches!(spinning, Err(ResolutionError::Setup(_))));
    assert!(matches!(unplanned, Err(ResolutionError::Production(_))));
}

#[tokio::test]
async fn oversized_selector_rejects_before_waiter_admission_or_native_io() {
    // Arrange
    let f = Fixture::new();
    let selector = Selector::css("a".repeat(MAX_SELECTOR_STRING_BYTES + 1)).unwrap();
    // Act
    let result = f
        .waiter
        .wait(selector, &f.attempts, &f.routes, &f.delays, never())
        .await;
    // Assert
    assert!(matches!(
        result,
        Err(ResolutionWaitError::Resolution(ResolutionError::Selector(
            _
        )))
    ));
    assert!(f.client.calls().is_empty());
    assert_eq!(f.active_leases(), 0);
}

#[tokio::test]
async fn retained_string_capacity_is_bounded_even_when_the_selector_text_is_small() {
    // Arrange
    let f = Fixture::new();
    let mut text = String::with_capacity(MAX_SELECTOR_STRING_BYTES + 1);
    text.push_str("button");
    let selector = Selector::css(text).unwrap();
    // Act
    let result = f
        .waiter
        .wait(selector, &f.attempts, &f.routes, &f.delays, never())
        .await;
    // Assert
    assert!(matches!(
        result,
        Err(ResolutionWaitError::Resolution(ResolutionError::Admission(
            _
        )))
    ));
    assert!(f.client.calls().is_empty());
    assert_eq!(f.active_leases(), 0);
}

#[tokio::test]
async fn timeout_during_stale_owner_release_continues_that_release_without_requery() {
    // Arrange
    let f = Fixture::new();
    let native = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let deadline = Notify::new();
    f.client.set_evaluate_gate(Some(native.clone()));
    f.client
        .set_release_object_group_gate(Some(release.clone()));
    let mut waiting = Box::pin(f.wait(async {
        deadline.notified().await;
        ResolutionWaitStop::Timeout
    }));
    assert!(poll(waiting.as_mut()).is_pending());
    f.replace_world();
    native.notify_waiters();
    assert!(poll(waiting.as_mut()).is_pending());
    // Act
    deadline.notify_waiters();
    let result = waiting.await;
    release.notify_waiters();
    f.queue.try_take().unwrap().unwrap().await.unwrap();
    // Assert
    assert!(matches!(
        result,
        Err(ResolutionWaitError::Stopped(ResolutionWaitStop::Timeout))
    ));
    assert_eq!(
        f.client.method_names(),
        ["Runtime.callFunctionOn", "Runtime.releaseObjectGroup"]
    );
    assert_eq!(f.active_leases(), 0);
    assert_eq!(f.queue.metrics().unwrap().completed, 1);
}

#[test]
fn installed_state_is_shared_by_clones_and_reclaimed_after_the_last_owner() {
    // Arrange
    let f = Fixture::new();
    let resources = f.resources.clone();
    let clone = f.waiter.clone();
    assert!(
        std::mem::size_of::<WaitSlots>()
            + 2 * std::mem::size_of::<usize>()
            + std::mem::size_of::<ResourceRequest>()
            <= STATE_BYTES as usize
    );
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    // Act
    drop(f);
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    drop(clone);
    // Assert
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}
