//! Local commit integration with real routes, runtime, Handles/worlds and
//! network adapters. Native proof validation is exercised by scheduler tests;
//! these tests invoke the production local phase with explicit native keys.
use super::*;
use crate::close_capacity::tests::fixture::Observer;
use crate::native_close_authority::scheduler::NativeCloseCommitTarget;
use crate::native_close_retirement::{retire_local, retire_network};
use crate::{
    ChromiumCloseCapacityBundle, ChromiumCloseTarget, ChromiumNetworkEventDriverShared,
    ChromiumNetworkEventDriverState, ChromiumNetworkPageRegistrationHandle,
    register_exact_network_page,
};
use pwright_backend_cdp_topology::transition_reservations::{
    TransitionReservationProfile, TransitionReservations,
};
use pwright_core::{ObjectKindTag, OperationError};
use pwright_runtime::disposal_reservations::{
    ControlOutboxStorageProfile, DisposalReservationProfile, DisposalReservations,
};
use std::{num::NonZeroUsize, sync::atomic::Ordering};

const PAGE: ChromiumCloseTarget = ChromiumCloseTarget::Page(ObjectId::new(3));
const CONTEXT: ChromiumCloseTarget = ChromiumCloseTarget::BrowserContext(ObjectId::new(2));
struct Fixture {
    routes: close_route_retirement::Fixture,
    runtime: ControlledDispatcher<(), (), NoDeadlineScheduler>,
    topology_pool: TransitionReservations,
    runtime_pool: DisposalReservations,
    observer: Observer,
    resources: ProductionResourceLedger,
    network: Arc<ChromiumNetworkEventDriverShared>,
}
impl Fixture {
    fn new() -> Self {
        let routes = close_route_retirement::Fixture::new();
        let transition = TransitionReservationProfile::new(
            routes.topology.profile(),
            ResourceScope::extension(137, 1),
            NonZeroUsize::new(2).unwrap(),
        )
        .unwrap();
        let limits = RuntimeLimits::new(64, 2).with_control_messages(256);
        let disposal = DisposalReservationProfile::new(
            NonZeroUsize::new(64).unwrap(),
            NonZeroUsize::new(2).unwrap(),
            ResourceScope::extension(137, 2),
        )
        .unwrap()
        .with_deferred_commit(NonZeroUsize::new(2).unwrap())
        .unwrap();
        let outbox = ControlOutboxStorageProfile::new(
            NonZeroUsize::new(256).unwrap(),
            ResourceScope::extension(137, 3),
        )
        .unwrap();
        let plan = transition
            .requirements()
            .combine_concurrent(limits.resource_requirements())
            .combine_concurrent(disposal.requirements())
            .combine_concurrent(outbox.requirements())
            .combine_concurrent(Observer::requirements(2));
        let resources = ProductionResourceLedger::new(exact_limits(plan), plan).unwrap();
        let topology_pool = transition.bind(&routes.topology, &resources).unwrap();
        let runtime = ControlledDispatcher::new((), (), NoDeadlineScheduler, limits)
            .with_production_resources(&resources)
            .unwrap();
        outbox
            .install(&runtime.lifecycle_handle(), &resources)
            .unwrap();
        for (id, parent) in [
            (1, None),
            (2, Some(1)),
            (3, Some(2)),
            (4, Some(3)),
            (5, Some(4)),
            (7, Some(1)),
            (8, Some(7)),
            (9, Some(8)),
        ] {
            runtime
                .register_object(
                    ObjectId::new(id),
                    parent.map(ObjectId::new),
                    ObjectKindTag::new(1),
                )
                .unwrap();
        }
        while runtime.pop_control_message().unwrap().is_some() {}
        let runtime_pool = disposal
            .bind(&runtime.lifecycle_handle(), &resources)
            .unwrap();
        let observer = Observer::new(2, &resources);
        let profile = all_capabilities_profile(all_capabilities_handshake_profile());
        let selected = profile.network_events.unwrap();
        let network = Arc::new(ChromiumNetworkEventDriverShared {
            state: Mutex::new(ChromiumNetworkEventDriverState {
                adapters: BTreeMap::new(),
                related_sessions: BTreeMap::new(),
                pending_page_registration: None,
                pending_page_replacement: None,
                unpublished_batch: None,
            }),
            subscriptions: routes.services.network_subscription_registry().unwrap(),
            object_ids: Arc::new(AtomicObjectIdSource::new(20).unwrap()),
            adapter_profile: selected.adapter(),
            resources: routes.resources.clone(),
            max_page_lanes: 2,
            max_related_sessions: 0,
        });
        for (page, context, session) in [(3, 2, "sa"), (8, 7, "sb")] {
            // The shared route fixture installed registry-only lanes. Replace
            // those with this fixture's fully owned driver adapter admission.
            network.subscriptions.forget_page(ObjectId::new(page));
            register_exact_network_page(
                &network,
                &mut network.state.lock().unwrap(),
                ObjectId::new(page),
                ObjectId::new(context),
                session.into(),
            )
            .unwrap();
        }
        Self {
            routes,
            runtime,
            topology_pool,
            runtime_pool,
            observer,
            resources,
            network,
        }
    }
    fn reserve(&self, target: ChromiumCloseTarget) -> ChromiumCloseCapacityBundle {
        ChromiumCloseCapacityBundle::reserve(
            target,
            &self.routes.topology,
            &self.topology_pool,
            &self.runtime_pool,
            &self.observer,
        )
        .unwrap()
    }
    fn commit(
        &mut self,
        capacity: ChromiumCloseCapacityBundle,
        native: NativeCloseCommitTarget<'_>,
        scratch: &mut [ObjectId],
    ) -> Result<(), OperationError<()>> {
        let registration = ChromiumNetworkPageRegistrationHandle {
            shared: Arc::downgrade(&self.network),
        };
        retire_network(
            &self.routes.services,
            Some(&registration),
            capacity.target(),
            || {
                retire_local(
                    native,
                    &mut self.routes.topology,
                    capacity,
                    self.runtime.lifecycle_handle(),
                    &self.routes.services,
                    scratch,
                )
            },
        )?
        .notify()
    }
    fn finish(self) {
        assert_eq!(self.runtime_pool.pending(), 0);
        assert_eq!(self.topology_pool.pending(), 0);
        assert_eq!(self.observer.0.pending.load(Ordering::Acquire), 0);
        drop((
            self.network,
            self.runtime_pool,
            self.topology_pool,
            self.observer,
            self.runtime,
        ));
        self.routes.finish();
        let metrics = self.resources.metrics().unwrap();
        assert_eq!(metrics.active_leases, 0);
        for class in ResourceClass::ALL {
            assert_eq!(metrics.class(class).current, 0);
        }
    }
}
fn page() -> NativeCloseCommitTarget<'static> {
    NativeCloseCommitTarget::Page {
        object: ObjectId::new(3),
        target_id: "pa",
        session_id: "sa",
    }
}
fn context() -> NativeCloseCommitTarget<'static> {
    NativeCloseCommitTarget::Context {
        object: ObjectId::new(2),
        browser_context_id: "a",
    }
}

#[test]
fn page_commit_retires_topology_runtime_routes_and_exact_network_adapter() {
    // Arrange
    let mut f = Fixture::new();
    let capacity = f.reserve(PAGE);
    let survivor = f.routes.services.routes.lock().unwrap().pages[&ObjectId::new(8)].clone();
    let created = f.routes.factory.created.lock().unwrap().len();
    // Act
    f.commit(capacity, page(), &mut [ObjectId::new(1); 64])
        .unwrap();
    // Assert
    assert_eq!(f.routes.topology.page_object_id("pa"), None);
    assert_eq!(f.runtime.is_object_live(ObjectId::new(5)), Ok(false));
    assert_eq!(f.runtime.is_object_live(ObjectId::new(2)), Ok(true));
    assert!(Arc::ptr_eq(
        &survivor,
        &f.routes.services.routes.lock().unwrap().pages[&ObjectId::new(8)]
    ));
    assert_eq!(f.routes.factory.created.lock().unwrap().len(), created);
    assert_eq!(f.network.state.lock().unwrap().adapters.len(), 1);
    assert_eq!(f.observer.0.committed.load(Ordering::Acquire), 1);
    drop(survivor);
    f.finish();
}

#[test]
fn parent_first_commit_allows_an_already_admitted_child_without_duplicate_dispose() {
    // Arrange
    let mut f = Fixture::new();
    let child = f.reserve(PAGE);
    let parent = f.reserve(CONTEXT);
    f.commit(parent, context(), &mut [ObjectId::new(1); 64])
        .unwrap();
    while f.runtime.pop_control_message().unwrap().is_some() {}
    // Act
    f.commit(child, page(), &mut [ObjectId::new(1); 64])
        .unwrap();
    // Assert
    assert_eq!(f.runtime.pop_control_message().unwrap(), None);
    assert_eq!(f.network.state.lock().unwrap().adapters.len(), 1);
    assert_eq!(f.runtime.is_object_live(ObjectId::new(9)), Ok(true));
    assert_eq!(f.observer.0.committed.load(Ordering::Acquire), 2);
    f.finish();
}

#[test]
fn committed_notification_owner_leaves_outer_session_lock_before_publication() {
    // Arrange
    let mut f = Fixture::new();
    let capacity = f.reserve(PAGE);
    let outer_session = Mutex::new(());
    let guard = outer_session.lock().unwrap();
    let mut scratch = [ObjectId::new(1); 64];
    let registration = ChromiumNetworkPageRegistrationHandle {
        shared: Arc::downgrade(&f.network),
    };
    // Act
    let committed = retire_network(&f.routes.services, Some(&registration), PAGE, || {
        retire_local(
            page(),
            &mut f.routes.topology,
            capacity,
            f.runtime.lifecycle_handle(),
            &f.routes.services,
            &mut scratch,
        )
    })
    .unwrap();
    // Assert
    assert_eq!(f.runtime.is_object_live(ObjectId::new(3)), Ok(false));
    assert_eq!(f.observer.0.committed.load(Ordering::Acquire), 0);
    assert_eq!(f.runtime_pool.pending(), 1);
    assert!(f.network.state.try_lock().is_ok());
    assert!(f.routes.services.routes.try_lock().is_ok());
    drop(guard);
    assert!(outer_session.try_lock().is_ok());
    committed.notify().unwrap();
    assert_eq!(f.observer.0.committed.load(Ordering::Acquire), 1);
    f.finish();
}

#[test]
fn undersized_scratch_preserves_all_local_state_and_returns_capacity() {
    // Arrange
    let mut f = Fixture::new();
    let capacity = f.reserve(CONTEXT);
    // Act
    let result = f.commit(capacity, context(), &mut []);
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::ResourceExhausted);
    assert_eq!(
        f.routes.topology.page_object_id("pa"),
        Some(ObjectId::new(3))
    );
    assert_eq!(f.runtime.is_object_live(ObjectId::new(5)), Ok(true));
    assert_eq!(f.network.state.lock().unwrap().adapters.len(), 2);
    assert_eq!(f.runtime.lifecycle_transaction_active(), Ok(false));
    f.finish();
}

#[test]
fn wrong_native_target_does_not_prepare_or_retire_the_admitted_intent() {
    // Arrange
    let mut f = Fixture::new();
    let capacity = f.reserve(CONTEXT);
    // Act
    let result = f.commit(capacity, page(), &mut [ObjectId::new(1); 64]);
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::ProtocolViolation);
    assert_eq!(f.observer.0.prepared.load(Ordering::Acquire), 0);
    assert_eq!(f.runtime.is_object_live(ObjectId::new(3)), Ok(true));
    f.finish();
}

#[test]
fn publication_preflight_failure_preserves_routes_and_network_adapters() {
    // Arrange
    let mut f = Fixture::new();
    let capacity = f.reserve(PAGE);
    f.observer.0.reject_prepare.store(true, Ordering::Release);
    // Act
    let result = f.commit(capacity, page(), &mut [ObjectId::new(1); 64]);
    // Assert
    assert!(result.is_err());
    assert_eq!(
        f.routes.topology.page_object_id("pa"),
        Some(ObjectId::new(3))
    );
    assert_eq!(f.network.state.lock().unwrap().adapters.len(), 2);
    assert_eq!(f.runtime.is_object_live(ObjectId::new(3)), Ok(true));
    f.finish();
}

#[test]
fn unpublished_network_batch_rejects_before_local_preparation() {
    // Arrange
    let mut f = Fixture::new();
    let capacity = f.reserve(PAGE);
    f.network.state.lock().unwrap().unpublished_batch = Some(ObjectId::new(3));
    // Act
    let result = f.commit(capacity, page(), &mut [ObjectId::new(1); 64]);
    // Assert
    assert!(result.is_err());
    assert_eq!(f.observer.0.prepared.load(Ordering::Acquire), 0);
    assert_eq!(f.runtime.is_object_live(ObjectId::new(3)), Ok(true));
    f.network.state.lock().unwrap().unpublished_batch = None;
    f.finish();
}

#[test]
fn closed_runtime_preserves_transport_failure_kind_without_local_publication() {
    // Arrange
    let mut f = Fixture::new();
    let capacity = f.reserve(CONTEXT);
    f.runtime.close_transport().unwrap();
    // Act
    let result = f.commit(capacity, context(), &mut [ObjectId::new(1); 64]);
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::TransportClosed);
    assert_eq!(
        f.routes.topology.page_object_id("pa"),
        Some(ObjectId::new(3))
    );
    assert_eq!(f.observer.0.committed.load(Ordering::Acquire), 0);
    f.finish();
}

#[test]
fn foreign_network_registration_rejects_before_invoking_local_publication() {
    // Arrange
    let f = Fixture::new();
    let foreign = Fixture::new();
    let registration = ChromiumNetworkPageRegistrationHandle {
        shared: Arc::downgrade(&foreign.network),
    };
    let mut called = false;
    // Act
    let result = retire_network(&f.routes.services, Some(&registration), PAGE, || {
        called = true;
        Ok(())
    });
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::ProtocolViolation);
    assert!(!called);
    f.finish();
    foreign.finish();
}

#[test]
fn selected_network_registry_cannot_be_silently_skipped() {
    // Arrange
    let f = Fixture::new();
    // Act
    let result: Result<(), OperationError<()>> =
        retire_network(&f.routes.services, None, PAGE, || {
            panic!("must reject before publication")
        });
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::ProtocolViolation);
    f.finish();
}

#[test]
fn active_network_request_is_disposed_once_by_runtime_before_adapter_removal() {
    // Arrange
    let mut f = Fixture::new();
    let batch = f.network.state.lock().unwrap().adapters.get_mut(&ObjectId::new(3)).unwrap().map_event(&pwright_cdp::CdpEvent::new(
        "Network.requestWillBeSent".into(),
        serde_json::json!({"requestId": "pending", "type": "Fetch", "request": {"url": "https://example.test/", "method": "GET", "headers": {}}}),
        Some("sa".into()),
    )).unwrap();
    f.runtime.apply_lifecycle_batch(batch.lifecycle()).unwrap();
    drop(batch);
    assert_eq!(f.runtime.is_object_live(ObjectId::new(20)), Ok(true));
    while f.runtime.pop_control_message().unwrap().is_some() {}
    let capacity = f.reserve(PAGE);
    // Act
    f.commit(capacity, page(), &mut [ObjectId::new(1); 64])
        .unwrap();
    // Assert
    assert_eq!(f.runtime.is_object_live(ObjectId::new(20)), Ok(false));
    let mut request_disposals = 0;
    while let Some(message) = f.runtime.pop_control_message().unwrap() {
        if matches!(message, ControlMessage::Dispose { object_id } if object_id == ObjectId::new(20))
        {
            request_disposals += 1;
        }
    }
    assert_eq!(request_disposals, 1);
    assert!(
        !f.network
            .state
            .lock()
            .unwrap()
            .adapters
            .contains_key(&ObjectId::new(3))
    );
    f.finish();
}

#[test]
fn borrowed_handle_leaf_retirement_reclaims_only_handles_in_the_runtime_subtree() {
    use pwright_composition_chromium_handle_evaluation::{
        ChromiumHandleRouteProfile, ChromiumHandleRouteSpec, ChromiumHandleRoutes,
    };
    // Arrange
    let mut f = Fixture::new();
    let profile = ChromiumHandleRouteProfile::new(ResourceScope::extension(137, 4), 2, 64, 128);
    let resources =
        ProductionResourceLedger::new(exact_limits(profile.requirements()), profile.requirements())
            .unwrap();
    let handles = ChromiumHandleRoutes::new_with_production_resources(profile, &resources).unwrap();
    for (id, frame, page) in [(10, 4, 3), (11, 9, 8)] {
        handles
            .reserve()
            .unwrap()
            .publish(
                ObjectId::new(id),
                ChromiumHandleRouteSpec::new(ObjectId::new(frame), ObjectId::new(page), 0, 4, None),
            )
            .unwrap();
        f.runtime
            .register_object(
                ObjectId::new(id),
                Some(ObjectId::new(frame)),
                ObjectKindTag::new(1),
            )
            .unwrap();
    }
    f.routes.services = f
        .routes
        .services
        .clone()
        .with_handle_leaf_routes(handles.clone());
    let capacity = f.reserve(PAGE);
    // Act
    f.commit(capacity, page(), &mut [ObjectId::new(1); 64])
        .unwrap();
    // Assert
    assert!(!handles.contains(ObjectId::new(10)).unwrap());
    assert!(handles.contains(ObjectId::new(11)).unwrap());
    assert_eq!(handles.metrics().unwrap().parent_retired, 1);
    drop(handles);
    f.finish();
    let metrics = resources.metrics().unwrap();
    assert_eq!(metrics.active_leases, 0);
    for class in ResourceClass::ALL {
        assert_eq!(metrics.class(class).current, 0);
    }
}

#[tokio::test]
async fn native_context_proof_marks_retained_worlds_and_revokes_navigation() {
    // Arrange
    let mut f = Fixture::new();
    let worlds = f.routes.worlds.snapshot(ObjectId::new(5)).await.unwrap();
    let removed = f.routes.navigation.snapshot(ObjectId::new(5)).unwrap();
    let survivor = f.routes.navigation.snapshot(ObjectId::new(9)).unwrap();
    let capacity = f.reserve(CONTEXT);
    // Act
    f.commit(capacity, context(), &mut [ObjectId::new(1); 64])
        .unwrap();
    // Assert
    assert!(worlds.native_lifetime().unwrap().both_worlds_destroyed());
    assert!(!removed.is_current());
    assert!(survivor.is_current());
    drop((worlds, removed, survivor));
    f.finish();
}
