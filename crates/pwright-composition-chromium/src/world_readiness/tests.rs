use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use super::*;
use pwright_backend_cdp::CdpTopologyLimits;
use pwright_core::ResourceLimits;

fn fixture(max_waiters: usize) -> (ExecutionWorldReadiness, ProductionResourceLedger) {
    let topology = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(16, 4, 4, 8, 8, 128),
        ResourceScope::extension(98, 1),
    )
    .unwrap();
    let profile = ChromiumExecutionWorldReadinessProfile::from_topology(
        ResourceScope::extension(98, 2),
        topology,
        NonZeroUsize::new(max_waiters).unwrap(),
    );
    let requirements = profile.requirements();
    let bytes = requirements.class(ResourceClass::BackendStateBytes);
    let resources = ProductionResourceLedger::new(
        ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
            ResourceClass::BackendStateBytes,
            bytes.global,
            bytes.per_scope,
        ),
        requirements,
    )
    .unwrap();
    (
        ExecutionWorldReadiness::new(profile, &resources).unwrap(),
        resources,
    )
}

fn poll<T>(future: Pin<&mut impl Future<Output = T>>) -> Poll<T> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

#[test]
fn wait_capacity_and_retained_bytes_return_on_future_drop() {
    // Arrange
    let (readiness, resources) = fixture(2);
    let baseline = resources.metrics().unwrap().active_leases;
    let mut first = Box::pin(readiness.resolve(|| Ok::<Option<()>, _>(None)));
    let mut second = Box::pin(readiness.resolve(|| Ok::<Option<()>, _>(None)));
    assert!(poll(first.as_mut()).is_pending());
    assert!(poll(second.as_mut()).is_pending());
    let mut excess = Box::pin(readiness.resolve(|| Ok::<Option<()>, _>(None)));
    // Act
    let rejected = poll(excess.as_mut());
    drop(first);
    let mut replacement = Box::pin(readiness.resolve(|| Ok::<Option<()>, _>(None)));
    let admitted = poll(replacement.as_mut());
    drop((second, replacement));
    // Assert
    assert!(matches!(
        rejected,
        Poll::Ready(Err(CdpPageRouteError::WorldReadinessCapacityExceeded {
            limit: 2
        }))
    ));
    assert!(admitted.is_pending());
    assert_eq!(readiness.waiters.load(Ordering::Acquire), 0);
    assert_eq!(resources.metrics().unwrap().active_leases, baseline);
}

#[test]
fn ready_routes_do_not_need_wait_admission() {
    // Arrange
    let (readiness, _) = fixture(1);
    let mut blocked = Box::pin(readiness.resolve(|| Ok::<Option<()>, _>(None)));
    assert!(poll(blocked.as_mut()).is_pending());
    let mut ready = Box::pin(readiness.resolve(|| Ok(Some(42))));
    // Act
    let result = poll(ready.as_mut());
    // Assert
    assert!(matches!(result, Poll::Ready(Ok(42))));
    assert_eq!(readiness.waiters.load(Ordering::Acquire), 1);
}

#[test]
fn a_commit_between_readiness_check_and_await_cannot_be_lost() {
    // Arrange
    let (readiness, _) = fixture(1);
    let checks = Cell::new(0);
    let mut future = Box::pin(readiness.resolve(|| {
        checks.set(checks.get() + 1);
        match checks.get() {
            1 => Ok(None),
            2 => {
                readiness.committed();
                Ok(None)
            }
            _ => Ok(Some(42)),
        }
    }));
    // Act
    let result = poll(future.as_mut());
    // Assert
    assert!(matches!(result, Poll::Ready(Ok(42))));
    assert_eq!(readiness.waiters.load(Ordering::Acquire), 0);
}

#[test]
fn notifications_recheck_the_latest_world_instead_of_caching_a_retired_world() {
    // Arrange
    let (readiness, _) = fixture(1);
    let world = Cell::new(None);
    let mut future = Box::pin(readiness.resolve(|| Ok(world.get())));
    assert!(poll(future.as_mut()).is_pending());
    readiness.committed();
    assert!(
        poll(future.as_mut()).is_pending(),
        "notification is not readiness"
    );
    // Act
    world.set(Some(1));
    readiness.committed();
    world.set(None);
    readiness.committed();
    world.set(Some(2));
    readiness.committed();
    let result = poll(future.as_mut());
    // Assert
    assert!(matches!(result, Poll::Ready(Ok(2))));
}

#[test]
fn terminal_owner_error_releases_the_waiter_without_a_world() {
    // Arrange
    let (readiness, resources) = fixture(1);
    let baseline = resources.metrics().unwrap().active_leases;
    let closed = Cell::new(false);
    let mut future = Box::pin(readiness.resolve(|| {
        if closed.get() {
            Err(CdpPageRouteError::WorldTargetClosed {
                object_id: ObjectId::new(4),
            })
        } else {
            Ok::<Option<()>, _>(None)
        }
    }));
    assert!(poll(future.as_mut()).is_pending());
    // Act
    closed.set(true);
    readiness.committed();
    let result = poll(future.as_mut());
    // Assert
    assert!(matches!(
        result,
        Poll::Ready(Err(CdpPageRouteError::WorldTargetClosed { .. }))
    ));
    assert_eq!(resources.metrics().unwrap().active_leases, baseline);
}

#[test]
fn profile_covers_two_route_tables_and_each_waiter() {
    // Arrange
    let (readiness, _) = fixture(2);
    let required = readiness.profile.requirements();
    let notification = readiness.changed.notified();
    // Act
    let bytes = required.class(ResourceClass::BackendStateBytes);
    // Assert
    assert_eq!(bytes.global, 256 + 8 * 128 * 2 + 2 * 512);
    assert_eq!(required.leases, 3);
    assert!(
        std::mem::size_of::<ExecutionWorldReadiness>() <= usize::try_from(FIXED_BYTES).unwrap()
    );
    assert!(
        std::mem::size_of_val(&notification) + std::mem::size_of::<WorldWaiter<'_>>()
            <= usize::try_from(WAITER_BYTES).unwrap()
    );
}
