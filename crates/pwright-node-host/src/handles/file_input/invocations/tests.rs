use super::*;
use crate::ffi_budget::{FfiBudget, FfiLane, FfiLimits, FfiMetrics};
use pwright_core::ResourceLimits;
use std::{
    future::Future,
    panic::{AssertUnwindSafe, catch_unwind},
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll, Waker},
};

fn registry() -> (Invocations, ProductionResourceLedger) {
    let requirements = Invocations::requirements();
    let limits = ResourceLimits::deny_all(1, 1).with_class(
        ResourceClass::BackendStateBytes,
        STORAGE_BYTES,
        STORAGE_BYTES,
    );
    let resources = ProductionResourceLedger::new(limits, requirements).unwrap();
    (Invocations::new(&resources).unwrap(), resources)
}

fn budget() -> FfiBudget {
    FfiBudget::new(FfiLimits {
        connects: 1,
        commands: 9,
        closes: 1,
        watches: 1,
        reserved_bytes: 4096,
        future_bytes: [512; 4],
    })
    .unwrap()
}

// This observes active admission only, not Arc tombstone heap reclamation.
fn assert_storage(resources: &ProductionResourceLedger, leases: usize) {
    let metrics = resources.metrics().unwrap();
    assert_eq!(metrics.active_leases, leases);
    assert_eq!(metrics.live_scopes, leases);
    assert_eq!(
        metrics.class(ResourceClass::BackendStateBytes).current,
        STORAGE_BYTES * leases as u64
    );
    assert_eq!(metrics.class(ResourceClass::PendingCall).current, 0);
    assert_eq!(metrics.class(ResourceClass::RemoteHandle).current, 0);
}

fn poll_now<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

#[test]
fn requirements_cover_fixed_index_and_no_native_or_task_capacity() {
    // Arrange
    let requirements = Invocations::requirements();

    // Act
    let (registry, resources) = registry();

    // Assert
    assert_eq!(requirements.scopes, 1);
    assert_eq!(requirements.leases, 1);
    for class in ResourceClass::ALL {
        let expected = if class == ResourceClass::BackendStateBytes {
            STORAGE_BYTES
        } else {
            0
        };
        assert_eq!(requirements.class(class).global, expected);
        assert_eq!(requirements.class(class).per_scope, expected);
    }
    assert_storage(&resources, 1);
    drop(registry);
    assert_storage(&resources, 0);
}

#[test]
fn omitted_resource_plan_rejects_before_index_allocation() {
    // Arrange
    let resources =
        ProductionResourceLedger::new(ResourceLimits::deny_all(1, 1), ResourceRequirements::none())
            .unwrap();

    // Act
    let result = Invocations::new(&resources);

    // Assert
    assert!(result.is_err());
    assert_storage(&resources, 0);
}

#[test]
fn eight_hooks_are_admitted_and_ninth_refusal_does_not_promote_or_revoke() {
    // Arrange
    let (registry, resources) = registry();
    let budget = budget();
    let preparations: [_; 9] =
        std::array::from_fn(|_| budget.prepare::<()>(FfiLane::Command, 16).unwrap());
    let registrations: [_; CAPACITY] =
        std::array::from_fn(|index| registry.register(preparations[index].revoker()).unwrap());
    let before = budget.metrics().unwrap();

    // Act
    let refused = registry.register(preparations[8].revoker());

    // Assert
    assert!(matches!(refused, Err(HostError::ResourceExhausted(_))));
    assert_eq!(budget.metrics().unwrap(), before);
    assert_eq!(before.running, 0);
    assert_eq!(before.pending, 9);
    for receipt in &registrations {
        registry.check(receipt).unwrap();
    }
    assert_storage(&resources, 1);
    drop(registrations);
    drop(preparations);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    drop(registry);
    assert_storage(&resources, 0);
}

#[test]
fn receipt_drop_reuses_one_slot_without_cancelling_its_ffi_entry() {
    // Arrange
    let (registry, resources) = registry();
    let budget = budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, 16).unwrap();
    let original = registry.register(preparation.revoker()).unwrap();
    let slot = original.slot;
    let generation = original.generation;

    // Act
    drop(original);
    let replacement = registry.register(preparation.revoker()).unwrap();

    // Assert
    assert_eq!(replacement.slot, slot);
    assert!(replacement.generation > generation);
    registry.check(&replacement).unwrap();
    assert_eq!(budget.metrics().unwrap().pending, 1);
    drop((replacement, preparation, registry));
    assert_storage(&resources, 0);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn stale_generation_cannot_check_or_remove_a_reused_slot() {
    // Arrange: duplicate only through private test access; public receipts are move-only.
    let (registry, resources) = registry();
    let budget = budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, 16).unwrap();
    let original = registry.register(preparation.revoker()).unwrap();
    let stale = Registration {
        origin: original.origin.clone(),
        slot: original.slot,
        generation: original.generation,
    };
    drop(original);
    let replacement = registry.register(preparation.revoker()).unwrap();

    // Act
    let result = registry.check(&stale);
    drop(stale);

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    registry.check(&replacement).unwrap();
    drop((replacement, preparation, registry));
    assert_storage(&resources, 0);
}

#[test]
fn equal_slot_and_generation_from_a_foreign_registry_are_rejected() {
    // Arrange
    let (registry, resources) = registry();
    let (foreign, foreign_resources) = self::registry();
    let budget = budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, 16).unwrap();
    let local = registry.register(preparation.revoker()).unwrap();
    let other = foreign.register(preparation.revoker()).unwrap();
    assert_eq!(
        (local.slot, local.generation),
        (other.slot, other.generation)
    );

    // Act
    let rejected = registry.check(&other);

    // Assert
    assert!(matches!(rejected, Err(HostError::InvalidArgument(_))));
    registry.check(&local).unwrap();
    foreign.check(&other).unwrap();
    drop((local, other, preparation, registry, foreign));
    assert_storage(&resources, 0);
    assert_storage(&foreign_resources, 0);
}

#[test]
fn generation_exhaustion_rejects_without_wrap_or_partial_insertion() {
    // Arrange
    let (registry, resources) = registry();
    let budget = budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, 16).unwrap();
    registry.0.state.lock().unwrap().next_generation = u64::MAX;
    let before = budget.metrics().unwrap();

    // Act
    let result = registry.register(preparation.revoker());

    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    let state = registry.0.state.lock().unwrap();
    assert_eq!(state.next_generation, u64::MAX);
    assert!(state.entries.iter().all(Option::is_none));
    drop(state);
    assert_eq!(budget.metrics().unwrap(), before);
    drop((preparation, registry));
    assert_storage(&resources, 0);
}

#[test]
fn taking_only_seals_then_explicit_revoke_reclaims_fresh_preparation() {
    // Arrange
    let (registry, resources) = registry();
    let budget = budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, 16).unwrap();
    let registration = registry.register(preparation.revoker()).unwrap();

    // Act
    let batch = registry.seal_and_take();

    // Assert: no cancel callout occurred during the synchronous take.
    assert_eq!(budget.metrics().unwrap().pending, 1);
    assert_eq!(budget.metrics().unwrap().running, 0);
    assert!(matches!(
        registry.check(&registration),
        Err(HostError::TargetClosed)
    ));
    assert!(matches!(
        registry.register(preparation.revoker()),
        Err(HostError::TargetClosed)
    ));
    batch.revoke();
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    assert!(matches!(
        preparation.start(&budget),
        Err(HostError::TargetClosed)
    ));
    registry.seal_and_take().revoke();
    drop((registration, registry));
    assert_storage(&resources, 0);
}

#[test]
fn dropping_a_detached_batch_revokes_and_holds_admission_until_callout_finishes() {
    // Arrange
    let (registry, resources) = registry();
    let budget = budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, 16).unwrap();
    let registration = registry.register(preparation.revoker()).unwrap();
    let batch = registry.seal_and_take();
    drop(registry);
    assert_storage(&resources, 1);

    // Act
    drop(batch);

    // Assert
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    assert_storage(&resources, 0);
    assert!(registration.origin.upgrade().is_none());
    drop((registration, preparation));
}

struct ObservedOwner {
    registry: Weak<Inner>,
    resources: ProductionResourceLedger,
    budget: FfiBudget,
    drops: Arc<AtomicUsize>,
}

impl Drop for ObservedOwner {
    fn drop(&mut self) {
        let registry = self
            .registry
            .upgrade()
            .expect("batch retains index allowance through owner Drop");
        assert!(
            registry.state.try_lock().is_ok(),
            "owner Drop cannot run under registry lock"
        );
        assert_storage(&self.resources, 1);
        assert_eq!(self.budget.metrics().unwrap().pending, 1);
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn parked_result_is_revoked_outside_registry_lock_with_both_allowances_held() {
    // Arrange
    let (registry, resources) = registry();
    let budget = budget();
    let drops = Arc::new(AtomicUsize::new(0));
    let owner = ObservedOwner {
        registry: Arc::downgrade(&registry.0),
        resources: resources.clone(),
        budget: budget.clone(),
        drops: drops.clone(),
    };
    let preparation = budget
        .prepare::<ObservedOwner>(FfiLane::Command, 128)
        .unwrap();
    let registration = registry.register(preparation.revoker()).unwrap();
    let mut execution = preparation.start(&budget).unwrap();
    let future = async move { Ok(owner) };
    execution
        .reserve_future(std::mem::size_of_val(&future))
        .unwrap();
    let parked = execution.run(future).await.park(&budget).unwrap();
    assert_eq!(budget.metrics().unwrap().running, 0);
    assert_eq!(budget.metrics().unwrap().queued, 0);
    let batch = registry.seal_and_take();
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(registry);

    // Act
    batch.revoke();

    // Assert
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    assert_storage(&resources, 0);
    assert!(matches!(
        parked.resume(&budget),
        Err(HostError::TargetClosed)
    ));
    drop(registration);
}

struct PendingOwner(ObservedOwner);
impl Future for PendingOwner {
    type Output = Result<(), HostError>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        assert_eq!(self.0.budget.metrics().unwrap().running, 1);
        Poll::Pending
    }
}

#[test]
fn running_revoke_keeps_ffi_admission_until_actual_future_is_dropped() {
    // Arrange
    let (registry, resources) = registry();
    let budget = budget();
    let drops = Arc::new(AtomicUsize::new(0));
    let preparation = budget.prepare::<()>(FfiLane::Command, 128).unwrap();
    let registration = registry.register(preparation.revoker()).unwrap();
    let mut execution = preparation.start(&budget).unwrap();
    let future = PendingOwner(ObservedOwner {
        registry: Arc::downgrade(&registry.0),
        resources: resources.clone(),
        budget: budget.clone(),
        drops: drops.clone(),
    });
    execution
        .reserve_future(std::mem::size_of_val(&future))
        .unwrap();
    let mut running = Box::pin(execution.run(future));
    assert!(poll_now(running.as_mut()).is_pending());
    let before = budget.metrics().unwrap();

    // Act
    registry.seal_and_take().revoke();

    // Assert: revocation only wakes Running, which still owns its live future.
    assert_eq!(budget.metrics().unwrap(), before);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    let Poll::Ready(ticket) = poll_now(running.as_mut()) else {
        panic!("cancellation must settle the next poll")
    };
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    assert!(matches!(
        ticket.deliver(|_| ()),
        Err(HostError::TargetClosed)
    ));
    drop((running, registration, registry));
    assert_storage(&resources, 0);
}

#[test]
fn weak_receipt_does_not_retain_active_registry_lease_or_cancel_ffi_owner() {
    // Arrange
    let (registry, resources) = registry();
    let budget = budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, 16).unwrap();
    let registration = registry.register(preparation.revoker()).unwrap();

    // Act
    drop(registry);

    // Assert: dropping Inner returns its lease but does not cancel FFI work.
    // The stale Weak still retains dead Arc backing, not a live Inner/lease;
    // neither failed upgrade nor zero leases proves that backing was freed.
    assert!(registration.origin.upgrade().is_none());
    assert_storage(&resources, 0);
    assert_eq!(budget.metrics().unwrap().pending, 1);
    drop((registration, preparation));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[tokio::test]
async fn registered_weak_revoker_does_not_retain_a_dropped_parked_result() {
    // Arrange
    let (registry, resources) = registry();
    let budget = budget();
    let owned = Arc::new(());
    let weak = Arc::downgrade(&owned);
    let preparation = budget.prepare::<Arc<()>>(FfiLane::Command, 16).unwrap();
    let registration = registry.register(preparation.revoker()).unwrap();
    let parked = preparation
        .start(&budget)
        .unwrap()
        .run(async move { Ok(owned) })
        .await
        .park(&budget)
        .unwrap();
    assert!(weak.upgrade().is_some());

    // Act
    drop(parked);

    // Assert: the registry's current receipt is not FFI/native-use authority.
    registry.check(&registration).unwrap();
    assert!(weak.upgrade().is_none());
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    registry.seal_and_take().revoke();
    drop((registration, registry));
    assert_storage(&resources, 0);
}

#[test]
fn poisoned_admission_fails_closed_but_cleanup_recovers_and_revokes() {
    // Arrange
    let (registry, resources) = registry();
    let budget = budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, 16).unwrap();
    let registration = registry.register(preparation.revoker()).unwrap();
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let _guard = registry.0.state.lock().unwrap();
            panic!("injected registry poison");
        }))
        .is_err()
    );
    assert!(registry.register(preparation.revoker()).is_err());
    assert!(registry.check(&registration).is_err());

    // Act
    registry.seal_and_take().revoke();

    // Assert
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    drop((registration, preparation, registry));
    assert_storage(&resources, 0);
}

#[test]
fn registration_drop_recovers_poison_and_removes_only_its_slot() {
    // Arrange
    let (registry, resources) = registry();
    let budget = budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, 16).unwrap();
    let registration = registry.register(preparation.revoker()).unwrap();
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let _guard = registry.0.state.lock().unwrap();
            panic!("injected registry poison");
        }))
        .is_err()
    );

    // Act
    drop(registration);

    // Assert
    assert!(
        registry
            .0
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .entries
            .iter()
            .all(Option::is_none)
    );
    assert_eq!(budget.metrics().unwrap().pending, 1);
    drop((preparation, registry));
    assert_storage(&resources, 0);
}
