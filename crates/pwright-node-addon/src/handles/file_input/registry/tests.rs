use super::*;
use pwright_node_host::ffi_budget::{FfiLimits, FfiMetrics};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

fn budget(commands: usize) -> FfiBudget {
    FfiBudget::new(FfiLimits {
        connects: 0,
        commands,
        closes: 1,
        watches: 0,
        reserved_bytes: CALL_BYTES * (commands as u64 + 1),
        future_bytes: [1024; 4],
    })
    .unwrap()
}

#[cfg(feature = "retained-target-marking")]
#[test]
fn protocol_call_ids_are_monotonic_per_session_scope() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let first = registry.scope().unwrap();
    let sibling = registry.scope().unwrap();

    // Act
    let first_ids = [
        first.next_protocol_call().unwrap(),
        first.next_protocol_call().unwrap(),
    ];
    let sibling_id = sibling.next_protocol_call().unwrap();

    // Assert
    assert_eq!(first_ids, [1, 2]);
    assert_eq!(sibling_id, 1);
}

#[cfg(feature = "retained-target-marking")]
#[test]
fn protocol_call_id_exhaustion_and_closed_scope_fail_explicitly() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let exhausted = registry.scope().unwrap();
    exhausted.0.next_protocol_call.set(u64::MAX);
    let closed = registry.scope().unwrap();
    closed.close();

    // Act
    let overflow = exhausted.next_protocol_call();
    let after_close = closed.next_protocol_call();

    // Assert
    assert!(matches!(overflow, Err(HostError::ResourceExhausted(_))));
    assert!(matches!(after_close, Err(HostError::TargetClosed)));
}

struct Probe {
    label: usize,
    drops: Rc<Cell<usize>>,
    on_drop: Option<Box<dyn FnOnce()>>,
}

impl Probe {
    fn new(label: usize, drops: &Rc<Cell<usize>>) -> Self {
        Self {
            label,
            drops: drops.clone(),
            on_drop: None,
        }
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
        if let Some(callback) = self.on_drop.take() {
            callback();
        }
    }
}

fn poll_now<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

#[test]
fn failed_admission_prevents_payload_creation() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(0);
    let created = Cell::new(false);
    let drops = Rc::new(Cell::new(0));

    // Act
    let result = scope.admit(&budget).inspect(|ticket| {
        created.set(true);
        ticket.populate(Probe::new(1, &drops)).unwrap();
    });

    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert!(!created.get());
    assert_eq!(drops.get(), 0);
    assert_eq!(registry.metrics(), (0, 0));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn populated_entry_is_charged_until_explicit_abort_without_gc() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    ticket.populate(Probe::new(7, &drops)).unwrap();
    assert_eq!(registry.metrics(), (1, 1));
    assert_eq!(budget.metrics().unwrap().reserved_bytes, CALL_BYTES);
    assert_eq!(budget.metrics().unwrap().running, 0);
    assert_eq!(scope.inspect(&ticket, |value| value.label).unwrap(), 7);

    // Act
    let aborted = ticket.abort();

    // Assert
    assert!(aborted);
    assert_eq!(drops.get(), 1);
    assert!(!ticket.is_pending());
    assert_eq!(registry.metrics(), (0, 0));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    drop(ticket);
    assert_eq!(drops.get(), 1);
}

#[test]
fn dropping_ticket_releases_payload_and_admission_synchronously() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    ticket.populate(Probe::new(1, &drops)).unwrap();

    // Act
    drop(ticket);

    // Assert
    assert_eq!(drops.get(), 1);
    assert_eq!(registry.metrics(), (0, 0));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn closing_one_scope_preserves_independent_sibling_entries() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let first = registry.scope().unwrap();
    let sibling = registry.scope().unwrap();
    let budget = budget(2);
    let first_ticket = first.admit(&budget).unwrap();
    let sibling_ticket = sibling.admit(&budget).unwrap();
    let first_drops = Rc::new(Cell::new(0));
    let sibling_drops = Rc::new(Cell::new(0));
    first_ticket.populate(Probe::new(1, &first_drops)).unwrap();
    sibling_ticket
        .populate(Probe::new(2, &sibling_drops))
        .unwrap();

    // Act
    first.close();

    // Assert
    assert_eq!(first_drops.get(), 1);
    assert_eq!(sibling_drops.get(), 0);
    assert!(!first_ticket.is_pending());
    assert!(sibling_ticket.is_pending());
    assert_eq!(
        sibling
            .inspect(&sibling_ticket, |value| value.label)
            .unwrap(),
        2
    );
    assert_eq!(registry.metrics(), (1, 1));
    assert_eq!(budget.metrics().unwrap().commands, 1);
    assert!(matches!(first.admit(&budget), Err(HostError::TargetClosed)));
    sibling.close();
    assert_eq!(sibling_drops.get(), 1);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn sibling_scope_cannot_inspect_an_exact_receipt_owned_elsewhere() {
    // Arrange
    let registry = Registry::<usize>::new().unwrap();
    let origin = registry.scope().unwrap();
    let sibling = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = origin.admit(&budget).unwrap();
    ticket.populate(41).unwrap();
    let inspected = Cell::new(false);

    // Act
    let result = sibling.inspect(&ticket, |_| inspected.set(true));

    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert!(!inspected.get());
    assert_eq!(origin.inspect(&ticket, |value| *value).unwrap(), 41);
    assert_eq!(registry.metrics(), (1, 1));
}

#[test]
fn equal_numeric_ids_from_distinct_registries_do_not_authorize_inspection() {
    // Arrange
    let origin = Registry::<usize>::new().unwrap();
    let foreign = Registry::<usize>::new().unwrap();
    let origin_scope = origin.scope().unwrap();
    let foreign_scope = foreign.scope().unwrap();
    let budget = budget(2);
    let original = origin_scope.admit(&budget).unwrap();
    let other = foreign_scope.admit(&budget).unwrap();
    original.populate(1).unwrap();
    other.populate(2).unwrap();
    assert_eq!(original.id, other.id);
    assert_eq!(origin_scope.0.id, foreign_scope.0.id);

    // Act
    let result = foreign_scope.inspect(&original, |value| *value);

    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert_eq!(origin_scope.inspect(&original, |value| *value).unwrap(), 1);
    assert_eq!(foreign_scope.inspect(&other, |value| *value).unwrap(), 2);
}

#[test]
fn duplicate_population_drops_only_the_rejected_new_payload() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let old_drops = Rc::new(Cell::new(0));
    let new_drops = Rc::new(Cell::new(0));
    ticket.populate(Probe::new(1, &old_drops)).unwrap();

    // Act
    let result = ticket.populate(Probe::new(2, &new_drops));

    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert_eq!(new_drops.get(), 1);
    assert_eq!(old_drops.get(), 0);
    assert_eq!(scope.inspect(&ticket, |value| value.label).unwrap(), 1);
    assert_eq!(registry.metrics(), (1, 1));
}

#[test]
fn duplicate_abort_cannot_remove_a_new_entry_in_the_reused_slot() {
    // Arrange
    let registry = Registry::<usize>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let old = scope.admit(&budget).unwrap();
    assert!(old.abort());
    let replacement = scope.admit(&budget).unwrap();
    replacement.populate(9).unwrap();
    assert_ne!(old.id, replacement.id);

    // Act
    let aborted = old.abort();

    // Assert
    assert!(!aborted);
    assert!(!old.is_pending());
    assert!(replacement.is_pending());
    assert_eq!(scope.inspect(&replacement, |value| *value).unwrap(), 9);
    assert_eq!(registry.metrics(), (1, 1));
}

#[test]
fn late_population_after_scope_close_drops_payload_without_reopening_admission() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    scope.close();

    // Act
    let result = ticket.populate(Probe::new(1, &drops));

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(drops.get(), 1);
    assert!(!ticket.is_pending());
    assert_eq!(registry.metrics(), (0, 0));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn held_ticket_does_not_keep_its_scope_or_payload_alive() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let observed = Rc::downgrade(&scope.0);
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    ticket.populate(Probe::new(1, &drops)).unwrap();

    // Act
    drop(scope);

    // Assert
    assert!(observed.upgrade().is_none());
    assert_eq!(drops.get(), 1);
    assert!(!ticket.is_pending());
    assert_eq!(registry.metrics(), (0, 0));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn held_scope_and_ticket_do_not_keep_the_environment_registry_alive() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let observed = Rc::downgrade(&registry.0);
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    ticket.populate(Probe::new(1, &drops)).unwrap();

    // Act
    drop(registry);

    // Assert
    assert!(observed.upgrade().is_none());
    assert_eq!(drops.get(), 1);
    assert!(!ticket.is_pending());
    assert!(matches!(scope.admit(&budget), Err(HostError::TargetClosed)));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn scope_capacity_reuses_a_closed_slot_without_reusing_its_identity() {
    // Arrange
    let registry = Registry::<()>::new().unwrap();
    let scopes: Vec<_> = (0..SESSIONS).map(|_| registry.scope().unwrap()).collect();
    assert!(matches!(
        registry.scope(),
        Err(HostError::ResourceExhausted(_))
    ));
    let retired_id = scopes[0].0.id;
    scopes[0].close();

    // Act
    let replacement = registry.scope().unwrap();

    // Assert
    assert_ne!(replacement.0.id, retired_id);
    assert!(scopes[0].0.closed.get());
    assert!(!replacement.0.closed.get());
    assert!(matches!(
        registry.scope(),
        Err(HostError::ResourceExhausted(_))
    ));
}

#[test]
fn exhausted_identity_counter_rejects_entries_before_ffi_admission() {
    // Arrange
    let registry = Registry::<()>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    registry.0.borrow_mut().next_id = u64::MAX;

    // Act
    let result = scope.admit(&budget);

    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(registry.0.borrow().next_id, u64::MAX);
    assert_eq!(registry.metrics(), (0, 0));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn exhausted_identity_counter_rejects_scopes_without_wrapping() {
    // Arrange
    let registry = Registry::<()>::new().unwrap();
    registry.0.borrow_mut().next_id = u64::MAX;

    // Act
    let result = registry.scope();

    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    let state = registry.0.borrow();
    assert_eq!(state.next_id, u64::MAX);
    assert!(state.scopes.iter().all(Option::is_none));
}

#[test]
fn fixed_entry_table_rejects_overflow_even_when_ffi_has_spare_capacity() {
    // Arrange
    let registry = Registry::<()>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(CALLS + 1);
    let tickets: Vec<_> = (0..CALLS).map(|_| scope.admit(&budget).unwrap()).collect();

    // Act
    let result = scope.admit(&budget);

    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(registry.metrics(), (CALLS as u32, 0));
    assert_eq!(budget.metrics().unwrap().commands, CALLS as u32);
    assert_eq!(
        budget.metrics().unwrap().reserved_bytes,
        CALL_BYTES * CALLS as u64
    );
    assert_eq!(budget.metrics().unwrap().running, 0);
    drop(tickets);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn preparation_and_ordinary_execution_share_the_existing_ffi_lane() {
    // Arrange
    let registry = Registry::<()>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(2);
    let first = scope.admit(&budget).unwrap();
    let ordinary = budget.admit::<()>(FfiLane::Command, 1).unwrap();

    // Act
    let result = scope.admit(&budget);

    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(registry.metrics(), (1, 0));
    assert_eq!(budget.metrics().unwrap().commands, 2);
    assert_eq!(budget.metrics().unwrap().running, 1);
    drop((first, ordinary));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[tokio::test]
async fn shutdown_releases_roots_before_budget_drain_while_tickets_remain_held() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    ticket.populate(Probe::new(1, &drops)).unwrap();

    // Act
    registry.shutdown();
    let mut shutdown = Box::pin(budget.shutdown());
    let result = poll_now(shutdown.as_mut());

    // Assert
    assert!(matches!(result, Poll::Ready(Ok(()))));
    assert_eq!(drops.get(), 1);
    assert!(!ticket.is_pending());
    assert_eq!(registry.metrics(), (0, 0));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    assert!(matches!(registry.scope(), Err(HostError::TargetClosed)));
    assert!(matches!(scope.admit(&budget), Err(HostError::TargetClosed)));
}

fn reentrant_probe(
    registry: &Registry<Probe>,
    drops: &Rc<Cell<usize>>,
    reentered: &Rc<Cell<bool>>,
) -> Probe {
    let weak = Rc::downgrade(&registry.0);
    let reentered = reentered.clone();
    Probe {
        label: 1,
        drops: drops.clone(),
        on_drop: Some(Box::new(move || {
            let registry = Registry(weak.upgrade().expect("outer registry is still held"));
            assert_eq!(registry.metrics(), (0, 0));
            registry.shutdown();
            reentered.set(true);
        })),
    }
}

#[test]
fn abort_drops_payload_outside_registry_borrow() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    let reentered = Rc::new(Cell::new(false));
    ticket
        .populate(reentrant_probe(&registry, &drops, &reentered))
        .unwrap();

    // Act
    let aborted = ticket.abort();

    // Assert
    assert!(aborted);
    assert!(reentered.get());
    assert_eq!(drops.get(), 1);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn scope_close_drops_payload_outside_registry_borrow() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    let reentered = Rc::new(Cell::new(false));
    ticket
        .populate(reentrant_probe(&registry, &drops, &reentered))
        .unwrap();

    // Act
    scope.close();

    // Assert
    assert!(reentered.get());
    assert_eq!(drops.get(), 1);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn shutdown_drops_payload_outside_registry_borrow() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    let reentered = Rc::new(Cell::new(false));
    ticket
        .populate(reentrant_probe(&registry, &drops, &reentered))
        .unwrap();

    // Act
    registry.shutdown();

    // Assert
    assert!(reentered.get());
    assert_eq!(drops.get(), 1);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn rejected_duplicate_payload_can_reenter_and_revoke_the_original() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let old_drops = Rc::new(Cell::new(0));
    let new_drops = Rc::new(Cell::new(0));
    let reentered = Rc::new(Cell::new(false));
    ticket.populate(Probe::new(1, &old_drops)).unwrap();
    let weak = Rc::downgrade(&registry.0);
    let observed = reentered.clone();
    let rejected = Probe {
        label: 2,
        drops: new_drops.clone(),
        on_drop: Some(Box::new(move || {
            let registry = Registry(weak.upgrade().unwrap());
            assert_eq!(registry.metrics(), (1, 1));
            registry.shutdown();
            observed.set(true);
        })),
    };

    // Act
    let result = ticket.populate(rejected);

    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert!(reentered.get());
    assert_eq!(old_drops.get(), 1);
    assert_eq!(new_drops.get(), 1);
    assert_eq!(registry.metrics(), (0, 0));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn scope_finalization_drops_payload_outside_registry_borrow() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    let reentered = Rc::new(Cell::new(false));
    ticket
        .populate(reentrant_probe(&registry, &drops, &reentered))
        .unwrap();

    // Act
    drop(scope);

    // Assert
    assert!(reentered.get());
    assert_eq!(drops.get(), 1);
    assert!(!ticket.is_pending());
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn unpopulated_receipt_cannot_expose_a_value() {
    // Arrange
    let registry = Registry::<()>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let inspected = Cell::new(false);

    // Act
    let result = scope.inspect(&ticket, |_| inspected.set(true));

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert!(!inspected.get());
    assert!(ticket.is_pending());
    assert_eq!(registry.metrics(), (1, 0));
}

#[test]
fn oversized_inline_entry_is_rejected_before_fixed_registry_storage_allocation() {
    // Arrange
    type Oversized = [u8; ADMIN_BYTES];

    // Act
    let result = Registry::<Oversized>::new();

    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
}

#[test]
fn dropping_one_scope_clone_preserves_the_other_owners_entries() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let alias = scope.clone();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    ticket.populate(Probe::new(7, &drops)).unwrap();

    // Act
    drop(scope);

    // Assert
    assert_eq!(drops.get(), 0);
    assert!(ticket.is_pending());
    assert_eq!(alias.inspect(&ticket, |value| value.label).unwrap(), 7);
    assert_eq!(registry.metrics(), (1, 1));
    assert_eq!(budget.metrics().unwrap().commands, 1);
    assert_eq!(budget.metrics().unwrap().reserved_bytes, CALL_BYTES);
}

#[test]
fn closing_a_scope_clone_revokes_entries_and_admission_through_every_alias() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let alias = scope.clone();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    ticket.populate(Probe::new(7, &drops)).unwrap();

    // Act
    alias.close();

    // Assert
    assert_eq!(drops.get(), 1);
    assert!(!ticket.is_pending());
    assert!(matches!(
        scope.inspect(&ticket, |value| value.label),
        Err(HostError::TargetClosed)
    ));
    assert!(matches!(
        alias.inspect(&ticket, |value| value.label),
        Err(HostError::TargetClosed)
    ));
    assert!(matches!(scope.admit(&budget), Err(HostError::TargetClosed)));
    assert!(matches!(alias.admit(&budget), Err(HostError::TargetClosed)));
    assert_eq!(registry.metrics(), (0, 0));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn payload_destruction_remains_charged_until_its_destructor_returns() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    let checked = Rc::new(Cell::new(false));
    let during_drop = budget.clone();
    let observed = checked.clone();
    ticket
        .populate(Probe {
            label: 1,
            drops: drops.clone(),
            on_drop: Some(Box::new(move || {
                let metrics = during_drop.metrics().unwrap();
                assert_eq!(metrics.pending, 1);
                assert_eq!(metrics.commands, 1);
                assert_eq!(metrics.reserved_bytes, CALL_BYTES);
                assert_eq!(metrics.running, 0);
                observed.set(true);
            })),
        })
        .unwrap();

    // Act
    let aborted = ticket.abort();

    // Assert
    assert!(aborted);
    assert!(checked.get());
    assert_eq!(drops.get(), 1);
    assert_eq!(registry.metrics(), (0, 0));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn closing_a_scope_removes_every_owned_entry_but_preserves_all_sibling_entries() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let sibling = registry.scope().unwrap();
    let budget = budget(5);
    let drops = Rc::new(Cell::new(0));
    let sibling_drops = Rc::new(Cell::new(0));
    let mut owned = Vec::new();
    let mut independent = Vec::new();
    // Interleave entries to exercise removal throughout the fixed table.
    for label in 0..3 {
        let ticket = scope.admit(&budget).unwrap();
        ticket.populate(Probe::new(label, &drops)).unwrap();
        owned.push(ticket);
        if label < 2 {
            let ticket = sibling.admit(&budget).unwrap();
            ticket
                .populate(Probe::new(label + 10, &sibling_drops))
                .unwrap();
            independent.push(ticket);
        }
    }

    // Act
    scope.close();

    // Assert
    assert_eq!(drops.get(), 3);
    assert_eq!(sibling_drops.get(), 0);
    assert!(owned.iter().all(|ticket| !ticket.is_pending()));
    assert!(independent.iter().all(Ticket::is_pending));
    for (index, ticket) in independent.iter().enumerate() {
        assert_eq!(
            sibling.inspect(ticket, |value| value.label).unwrap(),
            index + 10
        );
    }
    assert_eq!(registry.metrics(), (2, 2));
    assert_eq!(budget.metrics().unwrap().commands, 2);
    assert_eq!(budget.metrics().unwrap().reserved_bytes, 2 * CALL_BYTES);
}

#[test]
fn active_phase_rejects_same_ticket_reentry_without_removing_its_payload() {
    // Arrange
    let registry = Registry::<usize>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    ticket.populate(7).unwrap();
    let phase = ticket.phase().unwrap();

    // Act
    let reentry = ticket.phase();

    // Assert
    assert!(matches!(reentry, Err(HostError::InvalidArgument(_))));
    assert_eq!(phase.inspect(|value| *value).unwrap(), 7);
    assert!(ticket.is_pending());
    assert_eq!(registry.metrics(), (1, 1));
    assert_eq!(budget.metrics().unwrap().reserved_bytes, CALL_BYTES);
}

#[test]
fn phase_checkout_requires_a_populated_entry_without_poisoning_future_population() {
    // Arrange
    let registry = Registry::<usize>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();

    // Act
    let result = ticket.phase();

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert!(ticket.is_pending());
    assert_eq!(registry.metrics(), (1, 0));
    assert!(
        !registry
            .0
            .borrow()
            .entries
            .iter()
            .flatten()
            .next()
            .unwrap()
            .busy
    );
}

#[test]
fn successful_phase_replacement_preserves_one_admission_and_allows_the_next_phase() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let old_drops = Rc::new(Cell::new(0));
    let new_drops = Rc::new(Cell::new(0));
    ticket.populate(Probe::new(1, &old_drops)).unwrap();
    let phase = ticket.phase().unwrap();

    // Act
    let result = phase.replace(Probe::new(2, &new_drops));

    // Assert
    assert!(result.is_ok());
    assert_eq!(old_drops.get(), 1);
    assert_eq!(new_drops.get(), 0);
    assert_eq!(registry.metrics(), (1, 1));
    assert_eq!(budget.metrics().unwrap().pending, 1);
    assert_eq!(budget.metrics().unwrap().reserved_bytes, CALL_BYTES);
    let next = ticket.phase().unwrap();
    assert_eq!(next.inspect(|value| value.label).unwrap(), 2);
}

#[test]
fn sibling_preparation_can_finish_a_phase_while_another_is_busy() {
    // Arrange
    let registry = Registry::<usize>::new().unwrap();
    let scope = registry.scope().unwrap();
    let sibling = registry.scope().unwrap();
    let budget = budget(2);
    let first = scope.admit(&budget).unwrap();
    let other = sibling.admit(&budget).unwrap();
    first.populate(1).unwrap();
    other.populate(2).unwrap();
    let first_phase = first.phase().unwrap();
    let other_phase = other.phase().unwrap();

    // Act
    let result = other_phase.replace(3);

    // Assert
    assert!(result.is_ok());
    assert_eq!(first_phase.inspect(|value| *value).unwrap(), 1);
    assert_eq!(sibling.inspect(&other, |value| *value).unwrap(), 3);
    assert!(matches!(first.phase(), Err(HostError::InvalidArgument(_))));
    assert_eq!(registry.metrics(), (2, 2));
}

#[test]
fn explicit_abort_revokes_payload_and_admission_while_phase_remains_held() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    ticket.populate(Probe::new(1, &drops)).unwrap();
    let phase = ticket.phase().unwrap();

    // Act
    let aborted = ticket.abort();

    // Assert
    assert!(aborted);
    assert_eq!(drops.get(), 1);
    assert!(!ticket.is_pending());
    assert!(matches!(
        phase.inspect(|value| value.label),
        Err(HostError::TargetClosed)
    ));
    assert_eq!(registry.metrics(), (0, 0));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn scope_close_revokes_payload_and_admission_while_phase_remains_held() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    ticket.populate(Probe::new(1, &drops)).unwrap();
    let phase = ticket.phase().unwrap();

    // Act
    scope.close();

    // Assert
    assert_eq!(drops.get(), 1);
    assert!(!ticket.is_pending());
    assert!(matches!(
        phase.inspect(|value| value.label),
        Err(HostError::TargetClosed)
    ));
    assert_eq!(registry.metrics(), (0, 0));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[tokio::test]
async fn shutdown_revokes_a_held_phase_without_waiting_for_user_code() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    ticket.populate(Probe::new(1, &drops)).unwrap();
    let phase = ticket.phase().unwrap();

    // Act
    registry.shutdown();
    let mut shutdown = Box::pin(budget.shutdown());
    let result = poll_now(shutdown.as_mut());

    // Assert
    assert!(matches!(result, Poll::Ready(Ok(()))));
    assert_eq!(drops.get(), 1);
    assert!(matches!(
        phase.inspect(|value| value.label),
        Err(HostError::TargetClosed)
    ));
    assert_eq!(registry.metrics(), (0, 0));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn aborted_phase_replacement_cannot_resurrect_or_remove_a_new_slot_owner() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let original = scope.admit(&budget).unwrap();
    let old_drops = Rc::new(Cell::new(0));
    let rejected_drops = Rc::new(Cell::new(0));
    let replacement_drops = Rc::new(Cell::new(0));
    original.populate(Probe::new(1, &old_drops)).unwrap();
    let phase = original.phase().unwrap();
    assert!(original.abort());
    let replacement = scope.admit(&budget).unwrap();
    replacement
        .populate(Probe::new(2, &replacement_drops))
        .unwrap();

    // Act
    let result = phase.replace(Probe::new(3, &rejected_drops));

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(old_drops.get(), 1);
    assert_eq!(rejected_drops.get(), 1);
    assert_eq!(replacement_drops.get(), 0);
    assert!(replacement.is_pending());
    assert_eq!(scope.inspect(&replacement, |value| value.label).unwrap(), 2);
    assert_eq!(registry.metrics(), (1, 1));
}

#[test]
fn rejected_phase_replacement_drops_its_payload_outside_registry_borrow() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let old_drops = Rc::new(Cell::new(0));
    let rejected_drops = Rc::new(Cell::new(0));
    let reentered = Rc::new(Cell::new(false));
    ticket.populate(Probe::new(1, &old_drops)).unwrap();
    let phase = ticket.phase().unwrap();
    assert!(ticket.abort());
    let rejected = reentrant_probe(&registry, &rejected_drops, &reentered);

    // Act
    let result = phase.replace(rejected);

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert!(reentered.get());
    assert_eq!(old_drops.get(), 1);
    assert_eq!(rejected_drops.get(), 1);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn replaced_payload_can_reenter_a_new_phase_without_completed_guard_aborting_it() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = Rc::new(scope.admit(&budget).unwrap());
    let observed_ticket = Rc::downgrade(&ticket);
    let next_phase = Rc::new(RefCell::new(None));
    let observed_phase = next_phase.clone();
    let old_drops = Rc::new(Cell::new(0));
    let new_drops = Rc::new(Cell::new(0));
    let during_drop = budget.clone();
    ticket
        .populate(Probe {
            label: 1,
            drops: old_drops.clone(),
            on_drop: Some(Box::new(move || {
                assert_eq!(during_drop.metrics().unwrap().reserved_bytes, CALL_BYTES);
                let ticket = observed_ticket.upgrade().unwrap();
                *observed_phase.borrow_mut() = Some(ticket.phase().unwrap());
            })),
        })
        .unwrap();
    let phase = ticket.phase().unwrap();

    // Act
    let result = phase.replace(Probe::new(2, &new_drops));

    // Assert
    assert!(result.is_ok());
    assert_eq!(old_drops.get(), 1);
    assert_eq!(new_drops.get(), 0);
    let next = next_phase.borrow_mut().take().unwrap();
    assert_eq!(next.inspect(|value| value.label).unwrap(), 2);
    assert!(ticket.is_pending());
    assert!(matches!(ticket.phase(), Err(HostError::InvalidArgument(_))));
    assert_eq!(budget.metrics().unwrap().reserved_bytes, CALL_BYTES);
}

#[test]
fn dropping_unfinished_phase_keeps_allowance_until_payload_destruction_returns() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let _other_phases = reserve_other_phase_storage(&registry);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    let checked = Rc::new(Cell::new(false));
    let observed = checked.clone();
    let during_drop = budget.clone();
    let weak_registry = Rc::downgrade(&registry.0);
    ticket
        .populate(Probe {
            label: 1,
            drops: drops.clone(),
            on_drop: Some(Box::new(move || {
                assert_eq!(during_drop.metrics().unwrap().reserved_bytes, CALL_BYTES);
                assert_eq!(during_drop.metrics().unwrap().pending, 1);
                let registry = Registry(weak_registry.upgrade().unwrap());
                assert_eq!(registry.metrics(), (0, 0));
                assert!(matches!(
                    registry.0.borrow().phase_budget.reserve(),
                    Err(HostError::ResourceExhausted(_))
                ));
                registry.shutdown();
                observed.set(true);
            })),
        })
        .unwrap();
    let phase = ticket.phase().unwrap();

    // Act
    drop(phase);

    // Assert
    assert!(checked.get());
    assert_eq!(drops.get(), 1);
    assert!(!ticket.is_pending());
    assert_eq!(registry.metrics(), (0, 0));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    assert!(registry.0.borrow().phase_budget.reserve().is_ok());
}

#[test]
fn held_phase_does_not_keep_the_registry_or_its_payload_alive() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    ticket.populate(Probe::new(1, &drops)).unwrap();
    let observed = Rc::downgrade(&registry.0);
    let registry_owners = Rc::strong_count(&registry.0);
    let scope_owners = Rc::strong_count(&scope.0);
    let phase = ticket.phase().unwrap();
    assert_eq!(Rc::strong_count(&registry.0), registry_owners);
    assert_eq!(Rc::strong_count(&scope.0), scope_owners);

    // Act
    drop(registry);

    // Assert
    assert!(observed.upgrade().is_none());
    assert_eq!(drops.get(), 1);
    assert!(matches!(
        phase.inspect(|value| value.label),
        Err(HostError::TargetClosed)
    ));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn held_phase_does_not_keep_its_scope_or_entry_alive() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    ticket.populate(Probe::new(1, &drops)).unwrap();
    let observed = Rc::downgrade(&scope.0);
    let phase = ticket.phase().unwrap();

    // Act
    drop(scope);

    // Assert
    assert!(observed.upgrade().is_none());
    assert_eq!(drops.get(), 1);
    assert!(matches!(
        phase.inspect(|value| value.label),
        Err(HostError::TargetClosed)
    ));
    assert_eq!(registry.metrics(), (0, 0));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn root_metrics_count_only_populated_entries_including_an_active_phase() {
    // Arrange
    let registry = Registry::<u32>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(3);
    let first = scope.admit(&budget).unwrap();
    let second = scope.admit(&budget).unwrap();
    let _unpopulated = scope.admit(&budget).unwrap();
    first.populate(3).unwrap();
    second.populate(7).unwrap();
    let _phase = second.phase().unwrap();
    let mut inspected = 0;

    // Act
    let result = registry.rooted_values(|count| {
        inspected += 1;
        *count
    });

    // Assert
    assert_eq!(result.unwrap(), 10);
    assert_eq!(inspected, 2);
    assert_eq!(registry.metrics(), (3, 2));
    assert_eq!(budget.metrics().unwrap().commands, 3);
}

#[test]
fn root_metrics_observe_the_replaced_payload_without_changing_entry_counts() {
    // Arrange
    let registry = Registry::<u32>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    ticket.populate(3).unwrap();
    ticket.phase().unwrap().replace(6).unwrap();

    // Act
    let count = registry.rooted_values(|value| *value);

    // Assert
    assert_eq!(count.unwrap(), 6);
    assert_eq!(registry.metrics(), (1, 1));
    assert_eq!(budget.metrics().unwrap().reserved_bytes, CALL_BYTES);
}

#[test]
fn root_metrics_reject_counter_overflow_without_mutating_entries() {
    // Arrange
    let registry = Registry::<u32>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(2);
    let first = scope.admit(&budget).unwrap();
    let second = scope.admit(&budget).unwrap();
    first.populate(u32::MAX).unwrap();
    second.populate(1).unwrap();

    // Act
    let result = registry.rooted_values(|value| *value);

    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert!(first.is_pending());
    assert!(second.is_pending());
    assert_eq!(registry.metrics(), (2, 2));
    assert_eq!(budget.metrics().unwrap().commands, 2);
}

fn reserve_other_phase_storage<T>(registry: &Registry<T>) -> Vec<PublicationPhaseStorage> {
    (0..31)
        .map(|_| registry.0.borrow().phase_budget.reserve().unwrap())
        .collect()
}

#[test]
fn creation_phase_populates_once_and_releases_synchronous_scratch() {
    // Arrange
    let registry = Registry::<usize>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let _other_phases = reserve_other_phase_storage(&registry);
    let creation = ticket.creation_phase().unwrap();
    assert!(matches!(
        registry.0.borrow().phase_budget.reserve(),
        Err(HostError::ResourceExhausted(_))
    ));

    // Act
    let result = creation.replace(7);

    // Assert
    assert!(result.is_ok());
    assert_eq!(scope.inspect(&ticket, |value| *value).unwrap(), 7);
    assert_eq!(registry.metrics(), (1, 1));
    assert_eq!(budget.metrics().unwrap().reserved_bytes, CALL_BYTES);
    assert!(registry.0.borrow().phase_budget.reserve().is_ok());
    assert!(matches!(
        ticket.creation_phase(),
        Err(HostError::InvalidArgument(_))
    ));
}

#[test]
fn creation_phase_excludes_reentrant_creation_and_fixture_population() {
    // Arrange
    let registry = Registry::<usize>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let _creation = ticket.creation_phase().unwrap();

    // Act
    let duplicate = ticket.creation_phase();
    let populated = ticket.populate(7);

    // Assert
    assert!(matches!(duplicate, Err(HostError::InvalidArgument(_))));
    assert!(matches!(populated, Err(HostError::InvalidArgument(_))));
    assert_eq!(registry.metrics(), (1, 0));
    assert!(ticket.is_pending());
}

#[test]
fn dropping_unfinished_creation_returns_both_ffi_admission_and_phase_scratch() {
    // Arrange
    let registry = Registry::<usize>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let _other_phases = reserve_other_phase_storage(&registry);
    let ticket = scope.admit(&budget).unwrap();
    let creation = ticket.creation_phase().unwrap();
    assert_eq!(registry.metrics(), (1, 0));
    assert!(matches!(
        registry.0.borrow().phase_budget.reserve(),
        Err(HostError::ResourceExhausted(_))
    ));

    // Act
    drop(creation);

    // Assert
    assert!(!ticket.is_pending());
    assert_eq!(registry.metrics(), (0, 0));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    assert!(registry.0.borrow().phase_budget.reserve().is_ok());
}

#[test]
fn aborted_creation_rejects_late_payload_without_restoring_ffi_admission() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let creation = ticket.creation_phase().unwrap();
    assert!(ticket.abort());
    let drops = Rc::new(Cell::new(0));

    // Act
    let result = creation.replace(Probe::new(1, &drops));

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(drops.get(), 1);
    assert_eq!(registry.metrics(), (0, 0));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn revoked_entries_cannot_reuse_the_shared_thirty_two_phase_storage_slots() {
    // Arrange
    let registry = Registry::<usize>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let mut phases = Vec::new();
    for value in 0..32 {
        let ticket = scope.admit(&budget).unwrap();
        ticket.populate(value).unwrap();
        phases.push(ticket.phase().unwrap());
        assert!(ticket.abort());
    }
    assert_eq!(registry.metrics(), (0, 0));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    let later = scope.admit(&budget).unwrap();
    later.populate(99).unwrap();

    // Act
    let refused = later.phase();

    // Assert: all earlier entries/FFI tasks are gone, but synchronous scratch
    // stays owned by their still-held phases. Refusal must not mark later busy.
    assert!(matches!(refused, Err(HostError::ResourceExhausted(_))));
    assert!(later.is_pending());
    assert_eq!(registry.metrics(), (1, 1));
    assert_eq!(budget.metrics().unwrap().commands, 1);
    assert_eq!(budget.metrics().unwrap().running, 0);
    drop(phases.pop());
    let admitted = later.phase().unwrap();
    assert_eq!(admitted.inspect(|value| *value).unwrap(), 99);
    assert!(matches!(
        registry.0.borrow().phase_budget.reserve(),
        Err(HostError::ResourceExhausted(_))
    ));
    drop(admitted);
    drop(phases);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn phase_storage_is_shared_across_scopes_and_initial_construction() {
    // Arrange
    let registry = Registry::<usize>::new().unwrap();
    let first = registry.scope().unwrap();
    let sibling = registry.scope().unwrap();
    let budget = budget(2);
    let _other_phases = reserve_other_phase_storage(&registry);
    let existing = first.admit(&budget).unwrap();
    existing.populate(1).unwrap();
    let phase = existing.phase().unwrap();
    let constructing = sibling.admit(&budget).unwrap();

    // Act
    let refused = constructing.creation_phase();

    // Assert
    assert!(matches!(refused, Err(HostError::ResourceExhausted(_))));
    assert_eq!(registry.metrics(), (2, 1));
    drop(phase);
    let creation = constructing.creation_phase().unwrap();
    creation.replace(2).unwrap();
    assert_eq!(sibling.inspect(&constructing, |value| *value).unwrap(), 2);
    assert_eq!(registry.metrics(), (1, 1));
}

#[test]
fn synchronous_scratch_survives_scope_revocation_without_retaining_ffi_work() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let _other_phases = reserve_other_phase_storage(&registry);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    let checked = Rc::new(Cell::new(false));
    let observed = checked.clone();
    let weak = Rc::downgrade(&registry.0);
    ticket
        .populate(Probe {
            label: 1,
            drops: drops.clone(),
            on_drop: Some(Box::new(move || {
                let registry = Registry(weak.upgrade().unwrap());
                assert!(matches!(
                    registry.0.borrow().phase_budget.reserve(),
                    Err(HostError::ResourceExhausted(_))
                ));
                observed.set(true);
            })),
        })
        .unwrap();
    let phase = ticket.phase().unwrap();

    // Act
    scope.close();

    // Assert
    assert!(checked.get());
    assert_eq!(drops.get(), 1);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    assert!(matches!(
        registry.0.borrow().phase_budget.reserve(),
        Err(HostError::ResourceExhausted(_))
    ));
    drop(phase);
    assert!(registry.0.borrow().phase_budget.reserve().is_ok());
}

#[test]
fn replaced_payload_drops_before_its_phase_scratch_is_released() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let _other_phases = reserve_other_phase_storage(&registry);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    let replacement_drops = Rc::new(Cell::new(0));
    let checked = Rc::new(Cell::new(false));
    let observed = checked.clone();
    let weak = Rc::downgrade(&registry.0);
    ticket
        .populate(Probe {
            label: 1,
            drops: drops.clone(),
            on_drop: Some(Box::new(move || {
                let registry = Registry(weak.upgrade().unwrap());
                assert!(matches!(
                    registry.0.borrow().phase_budget.reserve(),
                    Err(HostError::ResourceExhausted(_))
                ));
                assert_eq!(registry.metrics(), (1, 1));
                observed.set(true);
            })),
        })
        .unwrap();
    let phase = ticket.phase().unwrap();

    // Act
    let result = phase.replace(Probe::new(2, &replacement_drops));

    // Assert
    assert!(result.is_ok());
    assert!(checked.get());
    assert_eq!(drops.get(), 1);
    assert_eq!(replacement_drops.get(), 0);
    assert!(registry.0.borrow().phase_budget.reserve().is_ok());
}

#[test]
fn rejected_payload_drops_before_its_revoked_phase_scratch_is_released() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let _other_phases = reserve_other_phase_storage(&registry);
    let ticket = scope.admit(&budget).unwrap();
    let original_drops = Rc::new(Cell::new(0));
    ticket.populate(Probe::new(1, &original_drops)).unwrap();
    let phase = ticket.phase().unwrap();
    assert!(ticket.abort());
    let rejected_drops = Rc::new(Cell::new(0));
    let checked = Rc::new(Cell::new(false));
    let observed = checked.clone();
    let weak = Rc::downgrade(&registry.0);
    let during_drop = budget.clone();
    let rejected = Probe {
        label: 2,
        drops: rejected_drops.clone(),
        on_drop: Some(Box::new(move || {
            let registry = Registry(weak.upgrade().unwrap());
            assert!(matches!(
                registry.0.borrow().phase_budget.reserve(),
                Err(HostError::ResourceExhausted(_))
            ));
            assert_eq!(during_drop.metrics().unwrap(), FfiMetrics::default());
            registry.shutdown();
            observed.set(true);
        })),
    };

    // Act
    let result = phase.replace(rejected);

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert!(checked.get());
    assert_eq!(original_drops.get(), 1);
    assert_eq!(rejected_drops.get(), 1);
    assert_eq!(registry.metrics(), (0, 0));
    assert!(registry.0.borrow().phase_budget.reserve().is_ok());
}

#[test]
fn terminal_commit_removes_exact_entry_and_drops_payload_after_registry_unlock() {
    // Arrange
    let registry = Registry::<Probe>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let ticket = scope.admit(&budget).unwrap();
    let drops = Rc::new(Cell::new(0));
    let observed = Rc::new(Cell::new(false));
    let weak = Rc::downgrade(&registry.0);
    let reentered = observed.clone();
    ticket
        .populate(Probe {
            label: 1,
            drops: drops.clone(),
            on_drop: Some(Box::new(move || {
                let registry = Registry(weak.upgrade().unwrap());
                assert_eq!(registry.metrics(), (0, 0));
                reentered.set(true);
            })),
        })
        .unwrap();
    let phase = ticket.phase().unwrap();

    // Act
    let result = phase.finish_terminal(|value| {
        value.label = 2;
        Ok(())
    });

    // Assert
    assert!(result.is_ok());
    assert!(observed.get());
    assert_eq!(drops.get(), 1);
    assert!(!ticket.is_pending());
    assert_eq!(registry.metrics(), (0, 0));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn revoked_terminal_guard_cannot_remove_a_later_entry_in_the_reused_slot() {
    // Arrange
    let registry = Registry::<usize>::new().unwrap();
    let scope = registry.scope().unwrap();
    let budget = budget(1);
    let original = scope.admit(&budget).unwrap();
    original.populate(1).unwrap();
    let phase = original.phase().unwrap();
    assert!(original.abort());
    let replacement = scope.admit(&budget).unwrap();
    replacement.populate(2).unwrap();

    // Act
    let result = phase.finish_terminal(|value| {
        *value = 3;
        Ok(())
    });

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert!(replacement.is_pending());
    assert_eq!(scope.inspect(&replacement, |value| *value).unwrap(), 2);
    assert_eq!(registry.metrics(), (1, 1));
    assert_eq!(budget.metrics().unwrap().commands, 1);
}
