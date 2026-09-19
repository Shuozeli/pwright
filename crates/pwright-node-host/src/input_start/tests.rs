use super::*;
use pwright_core::ResourceLimits;
use std::{
    future::{pending, ready},
    sync::atomic::{AtomicUsize, Ordering},
    task::Wake,
};

fn setup() -> (ProductionResourceLedger, Order) {
    let required = requirements();
    let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
    for class in ResourceClass::ALL {
        limits = limits.with_class(
            class,
            required.class(class).global,
            required.class(class).per_scope,
        );
    }
    let resources = ProductionResourceLedger::new(limits, required).unwrap();
    let order = Order::new(&resources).unwrap();
    (resources, order)
}
struct Counter(AtomicUsize);
impl Wake for Counter {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn reverse_executor_polls_preserve_submission_prefix_without_waiting_for_results() {
    // Arrange
    let (resources, order) = setup();
    let first = order.admit(None).unwrap();
    let second = order.admit(None).unwrap();
    let seen = Mutex::new(Vec::new());
    let mut first = Box::pin(first.run(async {
        seen.lock().unwrap().push(1);
        pending::<Result<(), HostError>>().await
    }));
    let mut second = Box::pin(second.run(async {
        seen.lock().unwrap().push(2);
        Ok(())
    }));
    let mut cx = Context::from_waker(Waker::noop());
    // Act
    assert!(second.as_mut().poll(&mut cx).is_pending());
    assert!(first.as_mut().poll(&mut cx).is_pending());
    let result = second.as_mut().poll(&mut cx);
    // Assert
    assert!(matches!(result, Poll::Ready(Ok(()))));
    assert_eq!(*seen.lock().unwrap(), [1, 2]);
    drop(first);
    drop(second);
    drop(order);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn dropping_an_unstarted_head_wakes_the_waiting_successor() {
    // Arrange
    let (_, order) = setup();
    let first = order.admit(None).unwrap();
    let second = order.admit(None).unwrap();
    let counter = Arc::new(Counter(AtomicUsize::new(0)));
    let waker = Waker::from(counter.clone());
    let mut cx = Context::from_waker(&waker);
    let mut second = Box::pin(second.run(ready(Ok(()))));
    assert!(second.as_mut().poll(&mut cx).is_pending());
    // Act
    drop(first);
    // Assert
    assert_eq!(counter.0.load(Ordering::SeqCst), 1);
    assert!(matches!(second.as_mut().poll(&mut cx), Poll::Ready(Ok(()))));
}

#[test]
fn close_wakes_every_queued_call_without_polling_its_body() {
    // Arrange
    let (_, order) = setup();
    let first = order.admit(None).unwrap();
    let second = order.admit(None).unwrap();
    let third = order.admit(None).unwrap();
    let counter = Arc::new(Counter(AtomicUsize::new(0)));
    let waker = Waker::from(counter.clone());
    let mut second = Box::pin(second.run::<()>(async {
        panic!("closed input body must not run");
    }));
    let mut third = Box::pin(third.run::<()>(async {
        panic!("closed input body must not run");
    }));
    let mut cx = Context::from_waker(&waker);
    assert!(second.as_mut().poll(&mut cx).is_pending());
    assert!(third.as_mut().poll(&mut cx).is_pending());
    // Act
    order.close();
    // Assert
    assert_eq!(counter.0.load(Ordering::SeqCst), 2);
    assert!(matches!(
        second.as_mut().poll(&mut cx),
        Poll::Ready(Err(HostError::TargetClosed))
    ));
    assert!(matches!(order.admit(None), Err(HostError::TargetClosed)));
    assert!(matches!(
        third.as_mut().poll(&mut cx),
        Poll::Ready(Err(HostError::TargetClosed))
    ));
    drop(first);
}

#[test]
fn capacity_bounds_unpolled_submissions_and_reclaims_dropped_slots() {
    // Arrange
    let (_, order) = setup();
    let mut tickets = (0..CAPACITY)
        .map(|_| order.admit(None).unwrap())
        .collect::<Vec<_>>();
    // Act
    let overflow = order.admit(None);
    // Assert
    assert!(matches!(overflow, Err(HostError::ResourceExhausted(_))));
    tickets.pop();
    let replacement = order.admit(None).unwrap();
    assert_eq!(replacement.id, CAPACITY as u64 + 1);
}

#[test]
fn capacity_lease_outlives_the_order_owner_while_a_ticket_is_retained() {
    // Arrange
    let (resources, order) = setup();
    let ticket = order.admit(None).unwrap();
    // Act
    drop(order);
    // Assert
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    drop(ticket);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn pending_first_call_releases_its_shutdown_admission_after_the_first_poll() {
    // Arrange
    let (_, order) = setup();
    let tracker = tokio_util::task::TaskTracker::new();
    let ticket = order.admit(Some(tracker.token())).unwrap();
    let mut call = Box::pin(ticket.run(pending::<Result<(), HostError>>()));
    assert_eq!(tracker.len(), 1);
    // Act
    let result = call.as_mut().poll(&mut Context::from_waker(Waker::noop()));
    // Assert
    assert!(result.is_pending());
    assert!(tracker.is_empty());
}

#[test]
fn dropping_the_installation_wakes_queued_tickets_but_retains_their_storage() {
    // Arrange
    let (resources, order) = setup();
    let first = order.admit(None).unwrap();
    let second = order.admit(None).unwrap();
    let mut second = Box::pin(second.run::<()>(async {
        panic!("owner is gone");
    }));
    let counter = Arc::new(Counter(AtomicUsize::new(0)));
    let waker = Waker::from(counter.clone());
    let mut cx = Context::from_waker(&waker);
    assert!(second.as_mut().poll(&mut cx).is_pending());
    // Act
    drop(order);
    // Assert
    assert_eq!(counter.0.load(Ordering::SeqCst), 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    assert!(matches!(
        second.as_mut().poll(&mut cx),
        Poll::Ready(Err(HostError::TargetClosed))
    ));
    drop(first);
    drop(second);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}
