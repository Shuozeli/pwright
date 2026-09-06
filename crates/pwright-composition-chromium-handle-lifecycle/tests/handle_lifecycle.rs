use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};

use pwright_browser_model::{
    BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND, BROWSER_OBJECT_KIND_CATALOG,
    FRAME_OBJECT_KIND, JS_HANDLE_OBJECT_KIND, PAGE_OBJECT_KIND,
};
use pwright_cap_handle_lifecycle::{HandleDispose, HandleLifecycleErrorDetails};
use pwright_composition_chromium_handle_lifecycle::{
    CdpHandleDisposeLimits, ChromiumHandleDisposer, ChromiumHandleLifecycleBindings,
    ChromiumHandleLifecycleInstallation, ChromiumHandleLifecycleProfile, ChromiumHandleRouteError,
    ChromiumHandleRouteProfile, ChromiumHandleRouteSpec, ChromiumHandleRoutes, FrameLimits,
    HandleLifecycleCodecLimits,
};
use pwright_core::{
    CallId, CallMetadata, Command, ControlMessage, FailureKind, ObjectId, OperationError,
    ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceRequirements, ResourceScope,
    Service,
};
use pwright_fake::FakeCdpClient;
use pwright_runtime::{
    CleanupQueue, ControlledDispatcher, NoDeadlineScheduler, ObjectCreationSpec,
    ObjectTopologyMode, RuntimeLimits,
};

type TestRuntime = ControlledDispatcher<(), (), NoDeadlineScheduler, CleanupQueue>;

struct Fixture {
    runtime: TestRuntime,
    resources: ProductionResourceLedger,
    profile: ChromiumHandleLifecycleProfile,
}

fn exact_limits(requirements: ResourceRequirements) -> ResourceLimits {
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let required = requirements.class(class);
        limits = limits.with_class(class, required.global, required.per_scope);
    }
    limits
}

fn fixture(max_handles: usize) -> Fixture {
    let runtime_limits = RuntimeLimits::new(8, 2)
        .with_active_commands(1)
        .with_control_messages(8);
    let frames = FrameLimits::new(4096, 0, 0, 4096);
    let profile = ChromiumHandleLifecycleProfile::new(
        ChromiumHandleRouteProfile::new(ResourceScope::extension(97, 1), max_handles, 64, 128),
        HandleLifecycleCodecLimits::new(1024),
        frames,
        frames,
    );
    let requirements = runtime_limits
        .resource_requirements()
        .combine_concurrent(profile.requirements());
    let resources = ProductionResourceLedger::new(exact_limits(requirements), requirements)
        .expect("exact Handle-lifecycle resources should build");
    let runtime = ControlledDispatcher::with_object_kinds_and_topology(
        (),
        (),
        NoDeadlineScheduler,
        runtime_limits,
        BROWSER_OBJECT_KIND_CATALOG,
        ObjectTopologyMode::Complete,
    )
    .expect("browser object catalog should validate")
    .with_production_resources(&resources)
    .expect("runtime resources should fit");
    for (object_id, parent, kind) in [
        (ObjectId::new(1), None, BROWSER_OBJECT_KIND.tag),
        (
            ObjectId::new(2),
            Some(ObjectId::new(1)),
            BROWSER_CONTEXT_OBJECT_KIND.tag,
        ),
        (
            ObjectId::new(3),
            Some(ObjectId::new(2)),
            PAGE_OBJECT_KIND.tag,
        ),
        (
            ObjectId::new(4),
            Some(ObjectId::new(3)),
            FRAME_OBJECT_KIND.tag,
        ),
        (
            ObjectId::new(5),
            Some(ObjectId::new(4)),
            JS_HANDLE_OBJECT_KIND.tag,
        ),
    ] {
        runtime
            .register_object(object_id, parent, kind)
            .expect("test object should register");
    }
    while runtime
        .pop_control_message()
        .expect("control queue should load")
        .is_some()
    {}
    Fixture {
        runtime,
        resources,
        profile,
    }
}

fn install(
    fixture: &Fixture,
    continuation: CleanupQueue,
) -> ChromiumHandleLifecycleInstallation<CleanupQueue> {
    ChromiumHandleLifecycleInstallation::install(
        ChromiumHandleLifecycleBindings {
            runtime: fixture.runtime.lifecycle_handle(),
            continuation,
        },
        fixture.profile,
        &fixture.resources,
    )
    .expect("Handle lifecycle should install")
}

#[test]
fn route_registry_installs_without_protocol_or_runtime_resources() {
    // Arrange
    let profile = ChromiumHandleRouteProfile::new(ResourceScope::extension(97, 2), 1, 64, 128);
    let requirements = profile.requirements();
    let resources = ProductionResourceLedger::new(exact_limits(requirements), requirements)
        .expect("route-only resources should build");

    // Act
    let routes = ChromiumHandleRoutes::new_with_production_resources(profile, &resources)
        .expect("route owner should install independently");

    // Assert
    assert_eq!(routes.profile(), profile);
    assert_eq!(routes.metrics().unwrap().current, 0);
}

#[test]
fn committed_parent_retirement_releases_only_the_runtime_subtree_selection() {
    // Arrange
    let profile = ChromiumHandleRouteProfile::new(ResourceScope::extension(97, 3), 2, 64, 128);
    let requirements = profile.requirements();
    let resources = ProductionResourceLedger::new(exact_limits(requirements), requirements)
        .expect("parent-retirement resources should build");
    let routes = ChromiumHandleRoutes::new_with_production_resources(profile, &resources)
        .expect("route owner should install");
    for (object_id, frame_id) in [(ObjectId::new(5), 4), (ObjectId::new(6), 7)] {
        routes
            .reserve()
            .expect("route should reserve")
            .publish(
                object_id,
                ChromiumHandleRouteSpec::new(ObjectId::new(frame_id), ObjectId::new(3), 0, 4, None),
            )
            .expect("route should publish");
    }
    let retirement = routes
        .prepare_parent_retirement(&[ObjectId::new(4), ObjectId::new(5)])
        .expect("parent retirement should prepare");

    // Act
    let retired = retirement.commit();

    // Assert
    assert_eq!(retired, 1);
    assert!(!routes.contains(ObjectId::new(5)).unwrap());
    assert!(routes.contains(ObjectId::new(6)).unwrap());
    let metrics = routes.metrics().unwrap();
    assert_eq!(metrics.current, 1);
    assert_eq!(metrics.parent_retired, 1);
}

#[tokio::test]
async fn explicit_dispose_releases_exact_native_object_and_retires_both_owners() {
    // Arrange
    let fixture = fixture(1);
    let installation = install(&fixture, CleanupQueue::new(1));
    let client = Arc::new(FakeCdpClient::from_html(""));
    let spec = ChromiumHandleRouteSpec::from_cdp(
        ObjectId::new(4),
        ObjectId::new(3),
        client.clone(),
        Some("remote-object-5"),
        7,
        CdpHandleDisposeLimits::new(64),
    )
    .expect("native route should validate");
    installation
        .routes()
        .reserve()
        .expect("route capacity should admit")
        .publish(ObjectId::new(5), spec)
        .expect("route should publish");

    // Act
    let result = installation
        .dispose_service()
        .call(Command::<HandleDispose>::new(
            CallId::new(1),
            ObjectId::new(5),
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("infallible service should respond");

    // Assert
    result.outcome().as_ref().expect("dispose should succeed");
    assert_eq!(client.method_names(), ["Runtime.releaseObject"]);
    assert!(
        !fixture
            .runtime
            .is_object_live(ObjectId::new(5))
            .expect("runtime should remain readable")
    );
    assert!(
        !installation
            .routes()
            .contains(ObjectId::new(5))
            .expect("routes should remain readable")
    );
    let metrics = installation.routes().metrics().unwrap();
    assert_eq!(metrics.current, 0);
    assert_eq!(metrics.explicitly_disposed, 1);
}

#[tokio::test]
async fn primitive_handle_disposal_performs_no_native_io() {
    // Arrange
    let fixture = fixture(1);
    let installation = install(&fixture, CleanupQueue::new(1));
    installation
        .routes()
        .reserve()
        .unwrap()
        .publish(
            ObjectId::new(5),
            ChromiumHandleRouteSpec::new(ObjectId::new(4), ObjectId::new(3), 0, 2, None),
        )
        .unwrap();

    // Act
    let result = installation
        .dispose_service()
        .call(Command::<HandleDispose>::new(
            CallId::new(2),
            ObjectId::new(5),
            CallMetadata::default(),
            (),
        ))
        .await
        .unwrap();

    // Assert
    result.outcome().as_ref().expect("dispose should succeed");
    assert!(!installation.routes().contains(ObjectId::new(5)).unwrap());
}

#[tokio::test]
async fn native_failure_preserves_runtime_and_route_for_visible_retry() {
    // Arrange
    let fixture = fixture(1);
    let installation = install(&fixture, CleanupQueue::new(1));
    let client = Arc::new(FakeCdpClient::from_html(""));
    client.set_release_object_error(Some("renderer refused release".to_owned()));
    let spec = ChromiumHandleRouteSpec::from_cdp(
        ObjectId::new(4),
        ObjectId::new(3),
        client,
        Some("remote-object-5"),
        0,
        CdpHandleDisposeLimits::new(64),
    )
    .unwrap();
    installation
        .routes()
        .reserve()
        .unwrap()
        .publish(ObjectId::new(5), spec)
        .unwrap();

    // Act
    let result = installation
        .dispose_service()
        .call(Command::<HandleDispose>::new(
            CallId::new(3),
            ObjectId::new(5),
            CallMetadata::default(),
            (),
        ))
        .await
        .unwrap();
    let error = result
        .outcome()
        .as_ref()
        .expect_err("native failure must remain visible");

    // Assert
    assert_eq!(error.kind(), FailureKind::Backend);
    assert!(fixture.runtime.is_object_live(ObjectId::new(5)).unwrap());
    assert!(installation.routes().contains(ObjectId::new(5)).unwrap());
}

#[test]
fn route_capacity_includes_in_flight_producer_reservations() {
    // Arrange
    let fixture = fixture(1);
    let installation = install(&fixture, CleanupQueue::new(1));
    let first = installation
        .routes()
        .reserve()
        .expect("first producer should reserve");

    // Act
    let second = installation.routes().reserve();

    // Assert
    assert!(matches!(
        second,
        Err(ChromiumHandleRouteError::CapacityExceeded { limit: 1 })
    ));
    assert_eq!(installation.routes().metrics().unwrap().reserved, 1);
    drop(first);
    assert_eq!(installation.routes().metrics().unwrap().reserved, 0);
}

#[test]
fn dropped_prepared_connection_retirement_preserves_every_route() {
    // Arrange
    let fixture = fixture(1);
    let installation = install(&fixture, CleanupQueue::new(1));
    let routes = installation.routes();
    routes
        .reserve()
        .expect("route should reserve")
        .publish(
            ObjectId::new(5),
            ChromiumHandleRouteSpec::new(ObjectId::new(4), ObjectId::new(3), 0, 4, None),
        )
        .expect("route should publish");
    let prepared = routes
        .prepare_connection_retirement()
        .expect("connection retirement should prepare");

    // Act
    drop(prepared);

    // Assert
    assert!(routes.contains(ObjectId::new(5)).unwrap());
    let metrics = routes.metrics().unwrap();
    assert_eq!(metrics.current, 1);
    assert_eq!(metrics.connection_retired, 0);
}

#[test]
fn committed_prepared_connection_retirement_releases_exact_snapshot() {
    // Arrange
    let fixture = fixture(1);
    let installation = install(&fixture, CleanupQueue::new(1));
    let routes = installation.routes();
    routes
        .reserve()
        .expect("route should reserve")
        .publish(
            ObjectId::new(5),
            ChromiumHandleRouteSpec::new(ObjectId::new(4), ObjectId::new(3), 0, 4, None),
        )
        .expect("route should publish");
    let prepared = routes
        .prepare_connection_retirement()
        .expect("connection retirement should prepare");

    // Act
    let retired = prepared.commit();

    // Assert
    assert_eq!(retired, 1);
    assert!(!routes.contains(ObjectId::new(5)).unwrap());
    let metrics = routes.metrics().unwrap();
    assert_eq!(metrics.current, 0);
    assert_eq!(metrics.connection_retired, 1);
}

#[test]
fn owner_retirement_selects_invalid_routes_and_includes_runtime_removals() {
    // Arrange
    let fixture = fixture(2);
    let installation = install(&fixture, CleanupQueue::new(1));
    let routes = installation.routes();
    for (object_id, owner_frame, owner_page) in [
        (ObjectId::new(5), ObjectId::new(4), ObjectId::new(3)),
        (ObjectId::new(6), ObjectId::new(8), ObjectId::new(7)),
    ] {
        routes
            .reserve()
            .expect("route should reserve")
            .publish(
                object_id,
                ChromiumHandleRouteSpec::new(owner_frame, owner_page, 0, 4, None),
            )
            .expect("route should publish");
    }
    let mut prepared = routes
        .prepare_owner_retirement(&[ObjectId::new(4)], &[], false)
        .expect("owner retirement should prepare");
    let messages = prepared.lifecycle_messages().collect::<Vec<_>>();

    // Act
    prepared.include_removed(&[ObjectId::new(6)]);
    let retired = prepared.commit();

    // Assert
    assert_eq!(
        messages,
        vec![ControlMessage::Dispose {
            object_id: ObjectId::new(5),
        }]
    );
    assert_eq!(retired, 2);
    assert!(!routes.contains(ObjectId::new(5)).unwrap());
    assert!(!routes.contains(ObjectId::new(6)).unwrap());
    let metrics = routes.metrics().unwrap();
    assert_eq!(metrics.current, 0);
    assert_eq!(metrics.topology_retired, 2);
}

#[test]
fn failed_publication_releases_its_route_and_resource_reservation() {
    // Arrange
    let fixture = fixture(1);
    let installation = install(&fixture, CleanupQueue::new(1));
    let reservation = installation
        .routes()
        .reserve()
        .expect("producer should reserve");

    // Act
    let result = reservation.publish(
        ObjectId::new(5),
        ChromiumHandleRouteSpec::new(ObjectId::new(4), ObjectId::new(3), 65, 0, None),
    );

    // Assert
    assert!(matches!(
        result,
        Err(ChromiumHandleRouteError::NativeIdTooLong {
            actual: 65,
            limit: 64
        })
    ));
    let metrics = installation.routes().metrics().unwrap();
    assert_eq!(metrics.reserved, 0);
    assert_eq!(metrics.current, 0);
    assert!(installation.routes().reserve().is_ok());
}

#[test]
fn batch_publication_commits_runtime_and_all_routes_under_one_admission() {
    // Arrange
    let fixture = fixture(2);
    let installation = install(&fixture, CleanupQueue::new(1));
    let prepared = fixture
        .runtime
        .lifecycle_handle()
        .prepare_object_creation_batch(&[
            ObjectCreationSpec::new(
                ObjectId::new(6),
                Some(ObjectId::new(4)),
                JS_HANDLE_OBJECT_KIND.tag,
            ),
            ObjectCreationSpec::new(
                ObjectId::new(7),
                Some(ObjectId::new(4)),
                JS_HANDLE_OBJECT_KIND.tag,
            ),
        ])
        .expect("runtime batch should prepare");
    let entries = [ObjectId::new(6), ObjectId::new(7)]
        .into_iter()
        .map(|object_id| {
            (
                object_id,
                ChromiumHandleRouteSpec::new(ObjectId::new(4), ObjectId::new(3), 0, 4, None),
            )
        })
        .collect();

    // Act
    installation
        .routes()
        .reserve_batch(2)
        .expect("whole batch should reserve")
        .publish_with(entries, || prepared.commit())
        .expect("runtime and routes should publish");

    // Assert
    for object_id in [ObjectId::new(6), ObjectId::new(7)] {
        assert!(fixture.runtime.is_object_live(object_id).unwrap());
        assert!(installation.routes().contains(object_id).unwrap());
    }
    let metrics = installation.routes().metrics().unwrap();
    assert_eq!(metrics.current, 2);
    assert_eq!(metrics.reserved, 0);
    assert_eq!(metrics.created, 2);
}

#[test]
fn failed_batch_commit_publishes_no_route_and_releases_every_reservation() {
    // Arrange
    let fixture = fixture(2);
    let installation = install(&fixture, CleanupQueue::new(1));
    let entries = [ObjectId::new(6), ObjectId::new(7)]
        .into_iter()
        .map(|object_id| {
            (
                object_id,
                ChromiumHandleRouteSpec::new(ObjectId::new(4), ObjectId::new(3), 0, 4, None),
            )
        })
        .collect();

    // Act
    let result = installation
        .routes()
        .reserve_batch(2)
        .unwrap()
        .publish_with(entries, || Err("injected runtime failure"));

    // Assert
    assert!(matches!(
        result,
        Err(
            pwright_composition_chromium_handle_lifecycle::ChromiumHandlePublicationError::Commit(
                "injected runtime failure"
            )
        )
    ));
    assert!(!installation.routes().contains(ObjectId::new(6)).unwrap());
    assert!(!installation.routes().contains(ObjectId::new(7)).unwrap());
    let metrics = installation.routes().metrics().unwrap();
    assert_eq!(metrics.current, 0);
    assert_eq!(metrics.reserved, 0);
}

struct GateState {
    open: AtomicBool,
    calls: AtomicUsize,
    waker: Mutex<Option<Waker>>,
}

struct GateDisposer {
    state: Arc<GateState>,
}

impl ChromiumHandleDisposer for GateDisposer {
    fn dispose(
        &self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), OperationError<HandleLifecycleErrorDetails>>>
                + Send
                + '_,
        >,
    > {
        self.state.calls.fetch_add(1, Ordering::SeqCst);
        let state = Arc::clone(&self.state);
        Box::pin(std::future::poll_fn(move |context| {
            if state.open.load(Ordering::SeqCst) {
                Poll::Ready(Ok(()))
            } else {
                *state
                    .waker
                    .lock()
                    .expect("gate waker should remain available") = Some(context.waker().clone());
                Poll::Pending
            }
        }))
    }
}

#[tokio::test]
async fn dropped_started_dispose_transfers_the_exact_transaction_to_caller_queue() {
    // Arrange
    let fixture = fixture(1);
    let continuation = CleanupQueue::new(1);
    let installation = install(&fixture, continuation.clone());
    let gate = Arc::new(GateState {
        open: AtomicBool::new(false),
        calls: AtomicUsize::new(0),
        waker: Mutex::new(None),
    });
    installation
        .routes()
        .reserve()
        .unwrap()
        .publish(
            ObjectId::new(5),
            ChromiumHandleRouteSpec::new(
                ObjectId::new(4),
                ObjectId::new(3),
                8,
                0,
                Some(Arc::new(GateDisposer {
                    state: Arc::clone(&gate),
                })),
            ),
        )
        .unwrap();
    let mut future = Box::pin(
        installation
            .dispose_service()
            .call(Command::<HandleDispose>::new(
                CallId::new(4),
                ObjectId::new(5),
                CallMetadata::default(),
                (),
            )),
    );
    std::future::poll_fn(|context| {
        assert!(future.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;

    // Act
    drop(future);
    let job = continuation
        .try_take()
        .expect("queue should remain available")
        .expect("started operation should transfer");
    gate.open.store(true, Ordering::SeqCst);
    if let Some(waker) = gate.waker.lock().unwrap().take() {
        waker.wake();
    }
    job.await.expect("continued dispose should finish");

    // Assert
    assert_eq!(gate.calls.load(Ordering::SeqCst), 1);
    assert!(!fixture.runtime.is_object_live(ObjectId::new(5)).unwrap());
    assert!(!installation.routes().contains(ObjectId::new(5)).unwrap());
    assert_eq!(continuation.metrics().unwrap().completed, 1);
}

#[tokio::test]
async fn missing_continuation_capacity_rejects_before_native_disposal() {
    // Arrange
    let fixture = fixture(1);
    let installation = install(&fixture, CleanupQueue::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(GateState {
        open: AtomicBool::new(true),
        calls: AtomicUsize::new(0),
        waker: Mutex::new(None),
    });
    installation
        .routes()
        .reserve()
        .unwrap()
        .publish(
            ObjectId::new(5),
            ChromiumHandleRouteSpec::new(
                ObjectId::new(4),
                ObjectId::new(3),
                8,
                0,
                Some(Arc::new(GateDisposer {
                    state: Arc::clone(&gate),
                })),
            ),
        )
        .unwrap();

    // Act
    let result = installation
        .dispose_service()
        .call(Command::<HandleDispose>::new(
            CallId::new(5),
            ObjectId::new(5),
            CallMetadata::default(),
            (),
        ))
        .await
        .unwrap();
    let error = result
        .outcome()
        .as_ref()
        .expect_err("saturated continuation must reject");
    calls.store(gate.calls.load(Ordering::SeqCst), Ordering::SeqCst);

    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(fixture.runtime.is_object_live(ObjectId::new(5)).unwrap());
    assert!(installation.routes().contains(ObjectId::new(5)).unwrap());
}
