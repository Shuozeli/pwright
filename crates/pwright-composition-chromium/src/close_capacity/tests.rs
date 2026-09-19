use super::*;
use crate::{ChromiumClosePlan, ChromiumCloseReservation};
use pwright_backend_cdp_topology::{
    CdpContextOwnership, CdpPageObjectIds, CdpPageTarget, CdpTopologyEffect, CdpTopologyLimits,
    CdpTopologyProductionProfile, ConnectionEpoch,
    transition_reservations::TransitionReservationProfile,
};
use pwright_core::{
    FailureKind, ObjectId, ObjectKindTag, ProductionResourceLedger, ResourceClass, ResourceLease,
    ResourceLedger, ResourceLimits, ResourceRequirements, ResourceScope,
};
use pwright_runtime::{
    ControlledDispatcher, NoDeadlineScheduler, RuntimeLimits,
    disposal_reservations::{ControlOutboxStorageProfile, DisposalReservationProfile},
};
use std::{
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

pub(crate) mod fixture;
use fixture::*;

#[test]
fn waiting_bundle_owns_all_capacity_without_holding_lifecycle_gates() {
    // Arrange
    let mut f = Fixture::new(2);
    // Act
    let bundle = f.reserve(PAGE).unwrap();
    // Assert
    assert_eq!(bundle.target(), PAGE);
    assert_eq!(f.pending(), (1, 1, 1));
    assert_eq!(f.runtime.lifecycle_transaction_active(), Ok(false));
    f.topology
        .register_context(EPOCH, "late", ObjectId::new(9), CdpContextOwnership::Owned)
        .unwrap();
    f.runtime
        .register_object(
            ObjectId::new(9),
            Some(ObjectId::new(1)),
            ObjectKindTag::new(1),
        )
        .unwrap();
    assert_eq!(f.observer.0.prepared.load(Ordering::Acquire), 0);
    drop(bundle);
    f.finish();
}

#[test]
fn topology_exhaustion_prevents_runtime_and_publication_admission() {
    // Arrange
    let f = Fixture::new(1);
    let held = f.topology_pool.reserve(&f.topology).unwrap();
    // Act
    let result = f.reserve(PAGE);
    // Assert
    assert!(matches!(
        result,
        Err(ChromiumCloseCapacityBundleError::Topology(_))
    ));
    assert_eq!(f.pending(), (1, 0, 0));
    assert_eq!(f.observer.0.attempts.load(Ordering::Acquire), 0);
    drop(held);
    f.finish();
}

#[test]
fn runtime_exhaustion_returns_the_earlier_topology_capacity() {
    // Arrange
    let f = Fixture::new(1);
    let held = f.runtime_pool.reserve().unwrap();
    // Act
    let result = f.reserve(PAGE);
    // Assert
    assert!(matches!(
        result,
        Err(ChromiumCloseCapacityBundleError::Runtime(_))
    ));
    assert_eq!(f.pending(), (0, 1, 0));
    assert_eq!(f.observer.0.attempts.load(Ordering::Acquire), 0);
    drop(held);
    f.finish();
}

#[test]
fn publication_exhaustion_rolls_back_both_native_and_runtime_capacity() {
    // Arrange
    let f = Fixture::new(1);
    let held = f.observer.reserve_capacity(PAGE).unwrap();
    let before = f.current();
    // Act
    let result = f.reserve(CONTEXT);
    // Assert
    assert!(
        matches!(result, Err(ChromiumCloseCapacityBundleError::Publication(error)) if error.kind() == FailureKind::ResourceExhausted)
    );
    assert_eq!(f.pending(), (0, 0, 1));
    assert_eq!(f.current(), before);
    drop(held);
    f.finish();
}

#[test]
fn extracting_parts_moves_reservations_without_reacquiring_them() {
    // Arrange
    let f = Fixture::new(1);
    let bundle = f.reserve(CONTEXT).unwrap();
    let before = f.resources.metrics().unwrap();
    // Act
    let parts = bundle.into_parts();
    // Assert
    assert_eq!(parts.target, CONTEXT);
    assert_eq!(f.pending(), (1, 1, 1));
    assert_eq!(f.resources.metrics().unwrap(), before);
    drop(parts);
    assert_eq!(f.pending(), (0, 0, 0));
    f.finish();
}

#[test]
fn closed_runtime_reclaims_topology_before_observer_is_called() {
    // Arrange
    let f = Fixture::new(1);
    f.runtime.close_transport().unwrap();
    // Act
    let result = f.reserve(CONTEXT);
    // Assert
    assert!(matches!(
        result,
        Err(ChromiumCloseCapacityBundleError::Runtime(_))
    ));
    assert_eq!(f.pending(), (0, 0, 0));
    assert_eq!(f.observer.0.attempts.load(Ordering::Acquire), 0);
    f.finish();
}

#[test]
fn foreign_topology_owner_is_rejected_before_later_admission() {
    // Arrange
    let f = Fixture::new(1);
    let other = Fixture::new(1);
    // Act
    let result = ChromiumCloseCapacityBundle::reserve(
        PAGE,
        &other.topology,
        &f.topology_pool,
        &f.runtime_pool,
        &f.observer,
    );
    // Assert
    assert!(matches!(
        result,
        Err(ChromiumCloseCapacityBundleError::Topology(_))
    ));
    assert_eq!(f.pending(), (0, 0, 0));
    assert_eq!(f.observer.0.attempts.load(Ordering::Acquire), 0);
    f.finish();
    other.finish();
}

#[test]
fn context_bundle_prepares_latest_children_then_commits_runtime_and_publication() {
    // Arrange
    let mut f = Fixture::new(1);
    let bundle = f.reserve(CONTEXT).unwrap();
    f.add_page("late", "late-session", 5, 6);
    f.topology
        .register_context(
            EPOCH,
            "sibling",
            ObjectId::new(9),
            CdpContextOwnership::Owned,
        )
        .unwrap();
    f.runtime
        .register_object(
            ObjectId::new(9),
            Some(ObjectId::new(1)),
            ObjectKindTag::new(1),
        )
        .unwrap();
    f.drain();
    // Act
    let removed = f.commit(bundle);
    // Assert
    assert_eq!(removed, [4, 3, 6, 5, 2]);
    assert_eq!(f.topology.context_object_id("context-a"), None);
    assert_eq!(
        f.topology.context_object_id("sibling"),
        Some(ObjectId::new(9))
    );
    assert_eq!(f.runtime.is_object_live(ObjectId::new(9)), Ok(true));
    assert_eq!(f.observer.0.committed.load(Ordering::Acquire), 1);
    assert_eq!(f.pending(), (0, 0, 0));
    f.finish();
}

#[test]
fn page_bundle_retires_its_runtime_subtree_but_preserves_context() {
    // Arrange
    let mut f = Fixture::new(1);
    let bundle = f.reserve(PAGE).unwrap();
    // Act
    let removed = f.commit(bundle);
    // Assert
    assert_eq!(removed, [4, 3]);
    assert_eq!(f.topology.page_object_id("target-a"), None);
    assert_eq!(
        f.topology.context_object_id("context-a"),
        Some(ObjectId::new(2))
    );
    assert_eq!(f.runtime.is_object_live(ObjectId::new(2)), Ok(true));
    assert_eq!(f.observer.0.committed.load(Ordering::Acquire), 1);
    f.finish();
}

#[test]
fn late_publication_preparation_failure_leaves_both_live_states_unchanged() {
    // Arrange
    let mut f = Fixture::new(1);
    let parts = f.reserve(PAGE).unwrap().into_parts();
    let runtime = parts
        .runtime
        .prepare(&f.runtime.lifecycle_handle(), ObjectId::new(3))
        .unwrap();
    let topology = parts
        .topology
        .prepare(&mut f.topology, |next| {
            next.detach_target(EPOCH, "session-a", Some("target-a"))
        })
        .unwrap();
    f.observer.0.reject_prepare.store(true, Ordering::Release);
    // Act
    let result = parts
        .publication
        .prepare(ChromiumClosePlan::new(PAGE, topology.output().effects()));
    // Assert
    assert!(result.is_err());
    drop((runtime, topology));
    assert_eq!(f.runtime.is_object_live(ObjectId::new(3)), Ok(true));
    assert_eq!(
        f.topology.page_object_id("target-a"),
        Some(ObjectId::new(3))
    );
    assert_eq!(f.pending(), (0, 0, 0));
    assert_eq!(f.observer.0.committed.load(Ordering::Acquire), 0);
    f.finish();
}
