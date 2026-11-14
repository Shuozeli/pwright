use super::*;
use pwright_cap_handle_lifecycle::HandleLifecycleErrorDetails;
use pwright_composition_chromium_handle_lifecycle::{
    ChromiumHandleDisposer, ChromiumHandleRouteProfile, ChromiumHandleRouteSpec,
    ChromiumHandleRoutes,
};
use pwright_core::{FailureKind, ObjectKindTag, OperationError, ResourceLimits};
use pwright_runtime::{ControlledDispatcher, NoDeadlineScheduler, RuntimeLimits};
use std::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll, Waker},
};

struct Native {
    calls: AtomicUsize,
    failure: Option<FailureKind>,
}
impl ChromiumHandleDisposer for Native {
    fn dispose(
        &self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), OperationError<HandleLifecycleErrorDetails>>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if let Some(kind) = self.failure {
                Err(OperationError::new(kind, "release failed", None))
            } else {
                Ok(())
            }
        })
    }
}
struct Fixture {
    runtime: ControlledDispatcher<(), (), NoDeadlineScheduler>,
    releases: Arc<Releases>,
    native: Arc<Native>,
    routes: ChromiumHandleRoutes,
    resources: ProductionResourceLedger,
}
fn fixture(count: usize, fail: bool) -> Fixture {
    fixture_with_failure(count, fail.then_some(FailureKind::Backend))
}
fn fixture_with_failure(count: usize, failure: Option<FailureKind>) -> Fixture {
    let limits = RuntimeLimits::new(256, 16);
    let runtime_objects = std::num::NonZeroUsize::new(limits.live_objects).unwrap();
    let routes = ChromiumHandleRouteProfile::new(ResourceScope::extension(157, 3), count, 64, 64);
    let plan = limits
        .resource_requirements()
        .combine_concurrent(routes.requirements())
        .combine_concurrent(requirements(runtime_objects).unwrap());
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
    let routes = ChromiumHandleRoutes::new_with_production_resources(routes, &resources).unwrap();
    let native = Arc::new(Native {
        calls: AtomicUsize::new(0),
        failure,
    });
    for id in 10..10 + count {
        let id = ObjectId::new(id as u64);
        runtime
            .register_object(id, Some(ObjectId::new(1)), ObjectKindTag::new(2))
            .unwrap();
        routes
            .reserve()
            .unwrap()
            .publish(
                id,
                ChromiumHandleRouteSpec::new(
                    ObjectId::new(1),
                    ObjectId::new(2),
                    12,
                    0,
                    Some(native.clone()),
                ),
            )
            .unwrap();
    }
    let releases = Releases::new(
        ChromiumHandleDisposeCommandService::new(runtime.lifecycle_handle(), routes.clone()),
        &runtime.lifecycle_handle(),
        runtime_objects,
        &resources,
    )
    .unwrap();
    Fixture {
        runtime,
        releases,
        native,
        routes,
        resources,
    }
}
fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

#[test]
fn driver_commits_bound_native_completion_batches_without_a_lifecycle_gate() {
    // Arrange
    let fixture = fixture(9, false);
    let mut calls: Vec<_> = (10..19)
        .map(|id| Box::pin(fixture.releases.release(ObjectId::new(id), None)))
        .collect();

    // Act
    for call in &mut calls {
        assert!(poll_once(call.as_mut()).is_pending());
    }

    // Assert
    assert_eq!(fixture.native.calls.load(Ordering::SeqCst), 8);
    assert_eq!(fixture.routes.metrics().unwrap().current, 9);
    assert_eq!(fixture.runtime.lifecycle_transaction_active(), Ok(false));
    fixture.releases.finish_ready().unwrap();
    for call in &mut calls[..8] {
        assert!(matches!(poll_once(call.as_mut()), Poll::Ready(Ok(()))));
    }
    assert!(poll_once(calls[8].as_mut()).is_pending());
    assert_eq!(fixture.native.calls.load(Ordering::SeqCst), 9);
    fixture.releases.finish_ready().unwrap();
    assert!(matches!(poll_once(calls[8].as_mut()), Poll::Ready(Ok(()))));
    assert_eq!(fixture.routes.metrics().unwrap().current, 0);
    assert_eq!(
        fixture
            .resources
            .metrics()
            .unwrap()
            .class(ResourceClass::RemoteHandle)
            .current,
        0
    );
    assert_eq!(fixture.releases.capacity.pending(), 0);
}

#[test]
fn busy_commit_parks_without_self_waking_and_creation_completion_resumes_it() {
    // Arrange
    let fixture = fixture(1, false);
    let mut call = Box::pin(fixture.releases.release(ObjectId::new(10), None));
    assert!(poll_once(call.as_mut()).is_pending());
    let ordinary = fixture
        .runtime
        .prepare_object_disposal(ObjectId::new(1))
        .unwrap();
    let mut first_signal = Box::pin(fixture.releases.changed());
    assert!(poll_once(first_signal.as_mut()).is_ready());

    // Act
    fixture.releases.finish_ready().unwrap();

    // Assert
    let mut next_signal = Box::pin(fixture.releases.changed());
    assert!(poll_once(next_signal.as_mut()).is_pending());
    assert!(poll_once(call.as_mut()).is_pending());
    assert_eq!(fixture.native.calls.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.releases.capacity.pending(), 1);
    drop(ordinary);
    fixture.releases.creation_polled();
    assert!(poll_once(next_signal.as_mut()).is_ready());
    fixture.releases.finish_ready().unwrap();
    assert!(matches!(poll_once(call.as_mut()), Poll::Ready(Ok(()))));
    assert_eq!(fixture.native.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn terminal_coordinator_close_rejects_waiters_without_claiming_local_success() {
    // Arrange
    let fixture = fixture(1, false);
    let mut call = Box::pin(fixture.releases.release(ObjectId::new(10), None));
    assert!(poll_once(call.as_mut()).is_pending());

    // Act
    fixture.releases.close();

    // Assert
    assert!(matches!(poll_once(call.as_mut()), Poll::Ready(Err(_))));
    assert_eq!(fixture.releases.capacity.pending(), 0);
    assert!(fixture.routes.contains(ObjectId::new(10)).unwrap());
    let mut later = Box::pin(fixture.releases.release(ObjectId::new(10), None));
    assert!(matches!(poll_once(later.as_mut()), Poll::Ready(Err(_))));
    assert_eq!(fixture.native.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn native_failure_needs_no_driver_commit_and_keeps_the_route_owned() {
    // Arrange
    let fixture = fixture(1, true);
    let mut call = Box::pin(fixture.releases.release(ObjectId::new(10), None));

    // Act
    let result = poll_once(call.as_mut());

    // Assert
    assert!(matches!(result, Poll::Ready(Err(_))));
    assert_eq!(fixture.releases.capacity.pending(), 0);
    assert!(fixture.routes.contains(ObjectId::new(10)).unwrap());
    fixture.releases.finish_ready().unwrap();
    assert!(fixture.routes.contains(ObjectId::new(10)).unwrap());
}

#[test]
fn target_closed_without_native_retirement_evidence_remains_a_failure() {
    // Arrange
    let fixture = fixture_with_failure(1, Some(FailureKind::TargetClosed));
    let mut call = Box::pin(fixture.releases.release(ObjectId::new(10), None));

    // Act
    let result = poll_once(call.as_mut());

    // Assert
    assert!(matches!(result, Poll::Ready(Err(_))));
    assert_eq!(fixture.releases.capacity.pending(), 0);
    assert!(fixture.routes.contains(ObjectId::new(10)).unwrap());
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(10)), Ok(true));
}
