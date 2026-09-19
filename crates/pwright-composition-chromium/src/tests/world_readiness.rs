use super::*;
use crate::{ChromiumExecutionWorldReadinessProfile, ChromiumFrameWorldAccessProfile};
use pwright_runtime::DeadlineScheduler;
use std::num::NonZeroUsize;

mod frame_world_access;
mod page_close_native_receipt;

#[derive(Clone, Default)]
struct ManualDeadline {
    budgets: Arc<Mutex<Vec<std::time::Duration>>>,
    expired: Arc<tokio::sync::Notify>,
}

impl DeadlineScheduler for ManualDeadline {
    type Sleep<'a> = tokio::sync::futures::Notified<'a>;
    fn sleep(&self, budget: std::time::Duration) -> Option<Self::Sleep<'_>> {
        self.budgets.lock().unwrap().push(budget);
        Some(self.expired.notified())
    }
}

struct Fixture {
    prepared: PreparedChromiumAttach,
    topology: ProductionCdpTopology,
    services: CdpPageServices,
    factory: Arc<RecordingSessionFactory>,
}

fn runtime<D: DeadlineScheduler>(
    fixture: &Fixture,
    deadlines: D,
) -> ControlledDispatcher<CdpPageServices, CdpPageServices, D, CleanupQueue> {
    let runtime = fixture
        .prepared
        .new_controlled_runtime(
            fixture.services.clone(),
            fixture.services.clone(),
            deadlines,
            CleanupQueue::new(4),
            ObjectId::new(1),
        )
        .unwrap();
    for (object, parent, kind) in [
        (2, 1, BROWSER_CONTEXT_OBJECT_KIND.tag),
        (3, 2, PAGE_OBJECT_KIND.tag),
        (4, 3, FRAME_OBJECT_KIND.tag),
        (5, 4, FRAME_OBJECT_KIND.tag),
    ] {
        runtime
            .register_object(ObjectId::new(object), Some(ObjectId::new(parent)), kind)
            .unwrap();
    }
    runtime
}

fn fixture(enabled: bool) -> Fixture {
    fixture_with_world_access(enabled, None)
}

fn fixture_with_world_access(
    enabled: bool,
    access: Option<ChromiumFrameWorldAccessProfile>,
) -> Fixture {
    let base = base_profile()
        .with_dom_content(dom_content_profile())
        .with_locator(locator_profile())
        .with_evaluation(evaluation_profile());
    let events = ChromiumTopologyEventDriverProductionProfile::from_profiles(
        ResourceScope::extension(98, 1),
        base.connection(),
        base.topology(),
    )
    .unwrap();
    let readiness = ChromiumExecutionWorldReadinessProfile::from_topology(
        ResourceScope::extension(98, 2),
        base.topology(),
        NonZeroUsize::new(2).unwrap(),
    );
    let mut profile = base
        .with_topology_events(events)
        .with_utility_world(ChromiumUtilityWorldProfile::playwright_compatible());
    if enabled {
        profile = profile.with_execution_world_readiness(readiness);
    }
    if let Some(access) = access {
        profile = profile.with_extension_requirements(access.requirements());
    }
    let prepared =
        PreparedChromiumAttach::new(profile.clone(), exact_limits(profile.requirements())).unwrap();
    let epoch = ConnectionEpoch::new(1);
    let mut topology = ProductionCdpTopology::new(
        ObjectId::new(1),
        epoch,
        profile.topology(),
        prepared.resources(),
    )
    .unwrap();
    topology
        .register_context(
            epoch,
            "context-a",
            ObjectId::new(2),
            CdpContextOwnership::External,
        )
        .unwrap();
    topology
        .attach_page(
            epoch,
            CdpPageTarget::new("target-a", "session-a", "context-a"),
            CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
        )
        .unwrap();
    topology
        .attach_frame(
            epoch,
            CdpFrameAttachment::main("target-a", "frame-a"),
            ObjectId::new(4),
        )
        .unwrap();
    topology
        .attach_frame(
            epoch,
            CdpFrameAttachment::local("target-a", "child", "frame-a"),
            ObjectId::new(5),
        )
        .unwrap();
    let factory = Arc::new(RecordingSessionFactory::default());
    let services = CdpPageServices::new_with_production_resources(
        factory.clone(),
        &profile,
        prepared.resources(),
    )
    .unwrap();
    services.synchronize(&topology).unwrap();
    Fixture {
        prepared,
        topology,
        services,
        factory,
    }
}

fn attach_world(
    topology: &mut ProductionCdpTopology,
    id: i64,
    name: &str,
    frame: &str,
    kind: CdpWorldKind,
) {
    topology
        .attach_world(
            ConnectionEpoch::new(1),
            CdpWorldAttachment::new("session-a", id, name, frame, kind),
        )
        .unwrap();
}

#[tokio::test]
async fn default_profile_still_rejects_missing_world_without_waiting() {
    // Arrange
    let fixture = fixture(false);
    // Act
    let result = <CdpPageServices as TargetedService<TitleParams>>::call_for(
        &fixture.services,
        ObjectId::new(4),
        TitleParams,
    )
    .await;
    // Assert
    assert_eq!(
        result.unwrap_err().kind(),
        FailureKind::UnsupportedCapability
    );
    assert!(fixture.services.world_readiness.is_none());
    assert!(
        fixture
            .services
            .routes
            .lock()
            .unwrap()
            .readiness_frames
            .is_empty()
    );
}

#[tokio::test]
async fn default_route_snapshot_is_taken_before_the_command_future_is_polled() {
    // Arrange
    let mut fixture = fixture(false);
    let client = fixture.factory.client("target-a");
    let title = <CdpPageServices as TargetedService<TitleParams>>::call_for(
        &fixture.services,
        ObjectId::new(4),
        TitleParams,
    );
    attach_world(
        &mut fixture.topology,
        11,
        "later-utility",
        "frame-a",
        CdpWorldKind::Utility,
    );
    fixture.services.synchronize(&fixture.topology).unwrap();
    // Act
    let result = title.await;
    // Assert
    assert_eq!(
        result.unwrap_err().kind(),
        FailureKind::UnsupportedCapability
    );
    assert!(client.calls().is_empty());
}

#[tokio::test]
async fn title_waits_for_the_exact_replacement_world_before_any_cdp_read() {
    // Arrange
    let mut fixture = fixture(true);
    let client = fixture.factory.client("target-a");
    client.set_evaluate_response(serde_json::json!({"result":{"value":"new title"}}));
    let mut title = Box::pin(<CdpPageServices as TargetedService<TitleParams>>::call_for(
        &fixture.services,
        ObjectId::new(4),
        TitleParams,
    ));
    let mut task = Context::from_waker(Waker::noop());
    assert!(title.as_mut().poll(&mut task).is_pending());
    assert!(client.calls().is_empty());
    // Act
    attach_world(
        &mut fixture.topology,
        11,
        "new-utility",
        "frame-a",
        CdpWorldKind::Utility,
    );
    fixture.services.synchronize(&fixture.topology).unwrap();
    let result = title.await.unwrap();
    // Assert
    assert_eq!(result.as_str(), "new title");
    assert_eq!(client.calls().len(), 1);
    assert_eq!(client.calls()[0].1[0]["uniqueContextId"], "new-utility");
}

#[tokio::test]
async fn child_locator_ownership_survives_a_world_gap_and_selects_only_the_latest_generation() {
    // Arrange
    let mut fixture = fixture(true);
    let mut locator = Box::pin(fixture.services.ready_locator_route(ObjectId::new(5)));
    let mut task = Context::from_waker(Waker::noop());
    assert!(locator.as_mut().poll(&mut task).is_pending());
    // Act
    attach_world(
        &mut fixture.topology,
        11,
        "retired-child",
        "child",
        CdpWorldKind::Utility,
    );
    fixture.services.synchronize(&fixture.topology).unwrap();
    fixture
        .topology
        .destroy_world(ConnectionEpoch::new(1), "session-a", 11)
        .unwrap();
    fixture.services.synchronize(&fixture.topology).unwrap();
    attach_world(
        &mut fixture.topology,
        12,
        "latest-child",
        "child",
        CdpWorldKind::Utility,
    );
    fixture.services.synchronize(&fixture.topology).unwrap();
    let result = locator.await.unwrap().unwrap();
    // Assert
    assert_eq!(result.unique_context_id(), Some("latest-child"));
    assert!(fixture.factory.client("target-a").calls().is_empty());
}

#[tokio::test]
async fn a_main_world_does_not_satisfy_a_utility_wait_or_vice_versa() {
    // Arrange
    let mut fixture = fixture(true);
    let mut utility = Box::pin(fixture.services.ready_dom_content_route(ObjectId::new(4)));
    let mut main = Box::pin(fixture.services.ready_evaluation_route(ObjectId::new(4)));
    let mut task = Context::from_waker(Waker::noop());
    assert!(utility.as_mut().poll(&mut task).is_pending());
    assert!(main.as_mut().poll(&mut task).is_pending());
    // Act
    attach_world(
        &mut fixture.topology,
        11,
        "main-only",
        "frame-a",
        CdpWorldKind::Main,
    );
    fixture.services.synchronize(&fixture.topology).unwrap();
    let main_result = main.await.unwrap();
    let utility_state = utility.as_mut().poll(&mut task);
    // Assert
    assert!(main_result.is_some());
    assert!(utility_state.is_pending());
}

#[tokio::test]
async fn rolled_back_route_preparation_does_not_publish_an_uncommitted_world() {
    // Arrange
    let mut fixture = fixture(true);
    let mut waiting = Box::pin(fixture.services.ready_dom_content_route(ObjectId::new(4)));
    let mut task = Context::from_waker(Waker::noop());
    assert!(waiting.as_mut().poll(&mut task).is_pending());
    attach_world(
        &mut fixture.topology,
        11,
        "not-committed",
        "frame-a",
        CdpWorldKind::Utility,
    );
    // Act
    drop(
        fixture
            .services
            .prepare_synchronization(fixture.topology.topology())
            .unwrap(),
    );
    let state = waiting.as_mut().poll(&mut task);
    // Assert
    assert!(state.is_pending());
    assert!(
        fixture
            .services
            .dom_content_route(ObjectId::new(4))
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn closed_child_and_crashed_page_interrupt_waits_without_native_commands() {
    // Arrange
    let mut fixture = fixture(true);
    let mut child = Box::pin(fixture.services.ready_dom_content_route(ObjectId::new(5)));
    let mut main = Box::pin(fixture.services.ready_dom_content_route(ObjectId::new(4)));
    let mut task = Context::from_waker(Waker::noop());
    assert!(child.as_mut().poll(&mut task).is_pending());
    assert!(main.as_mut().poll(&mut task).is_pending());
    // Act
    fixture
        .topology
        .detach_frame(
            ConnectionEpoch::new(1),
            "child",
            CdpFrameDetachReason::Remove,
        )
        .unwrap();
    fixture
        .topology
        .crash_target(ConnectionEpoch::new(1), "target-a", "crash", 1)
        .unwrap();
    fixture.services.synchronize(&fixture.topology).unwrap();
    let child_result = child.await;
    let main_result = main.await;
    // Assert
    assert!(matches!(
        child_result,
        Err(CdpPageRouteError::WorldTargetClosed { .. })
    ));
    assert!(matches!(
        main_result,
        Err(CdpPageRouteError::PageCrashed { .. })
    ));
    assert!(fixture.factory.client("target-a").calls().is_empty());
}

#[test]
fn selection_declares_exact_incremental_resources_and_requires_a_matching_event_driver() {
    // Arrange
    let base = base_profile().with_evaluation(evaluation_profile());
    let addition = ChromiumExecutionWorldReadinessProfile::from_topology(
        ResourceScope::extension(98, 2),
        base.topology(),
        NonZeroUsize::new(2).unwrap(),
    );
    let expected = base
        .requirements()
        .combine_concurrent(addition.requirements());
    // Act
    let configured = base.with_execution_world_readiness(addition);
    let requirements = configured.requirements();
    let rejected = PreparedChromiumAttach::new(configured, exact_limits(requirements));
    // Assert
    assert_eq!(requirements, expected);
    let error = match rejected {
        Ok(_) => panic!("missing event driver must reject readiness selection"),
        Err(error) => error,
    };
    assert!(
        matches!(
            error,
            ChromiumCompositionError::InvalidWorldReadinessProfile
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn the_original_command_deadline_covers_world_waiting_without_rearming() {
    // Arrange
    let fixture = fixture(true);
    let deadlines = ManualDeadline::default();
    let runtime = runtime(&fixture, deadlines.clone());
    let baseline = fixture
        .prepared
        .resources()
        .metrics()
        .unwrap()
        .class(ResourceClass::BackendStateBytes)
        .current;
    let budget = std::time::Duration::from_millis(25);
    let mut command = Box::pin(runtime.call(Command::<FrameTitle>::new(
        CallId::new(1),
        ObjectId::new(4),
        CallMetadata::new(Some(budget), None),
        TitleParams,
    )));
    let mut task = Context::from_waker(Waker::noop());
    assert!(command.as_mut().poll(&mut task).is_pending());
    assert_eq!(runtime.pending_call_count().unwrap(), 1);
    // Act
    for _ in 0..3 {
        fixture.services.synchronize(&fixture.topology).unwrap();
        assert!(command.as_mut().poll(&mut task).is_pending());
    }
    deadlines.expired.notify_waiters();
    let outcome = command.await.unwrap().into_outcome();
    // Assert
    assert_eq!(outcome.unwrap_err().kind(), FailureKind::Timeout);
    assert_eq!(*deadlines.budgets.lock().unwrap(), [budget]);
    assert_eq!(runtime.pending_call_count().unwrap(), 0);
    assert_eq!(
        fixture
            .prepared
            .resources()
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        baseline
    );
    assert!(fixture.factory.client("target-a").calls().is_empty());
}

#[tokio::test]
async fn context_close_cancels_a_world_wait_before_any_native_read() {
    // Arrange
    let fixture = fixture(true);
    let runtime = runtime(&fixture, NoDeadlineScheduler);
    let baseline = fixture
        .prepared
        .resources()
        .metrics()
        .unwrap()
        .class(ResourceClass::BackendStateBytes)
        .current;
    let mut command = Box::pin(runtime.call(Command::<FrameTitle>::new(
        CallId::new(1),
        ObjectId::new(5),
        CallMetadata::default(),
        TitleParams,
    )));
    assert!(
        command
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    runtime.dispose_object(ObjectId::new(2)).unwrap();
    let result = command.await.unwrap().into_outcome();
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::TargetClosed);
    assert_eq!(runtime.pending_call_count().unwrap(), 0);
    assert_eq!(
        fixture
            .prepared
            .resources()
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        baseline
    );
    assert!(fixture.factory.client("target-a").calls().is_empty());
}

#[tokio::test]
async fn world_waits_remain_inside_the_original_pending_call_limit() {
    // Arrange
    let fixture = fixture(true);
    let runtime = runtime(&fixture, NoDeadlineScheduler);
    let mut calls = (1..=4)
        .map(|id| {
            Box::pin(runtime.call(Command::<FrameTitle>::new(
                CallId::new(id),
                ObjectId::new(4),
                CallMetadata::default(),
                TitleParams,
            )))
        })
        .collect::<Vec<_>>();
    let mut task = Context::from_waker(Waker::noop());
    for call in &mut calls {
        assert!(call.as_mut().poll(&mut task).is_pending());
    }
    // Act
    let excess = runtime
        .call(Command::<FrameTitle>::new(
            CallId::new(5),
            ObjectId::new(4),
            CallMetadata::default(),
            TitleParams,
        ))
        .await
        .unwrap()
        .into_outcome();
    runtime.dispose_object(ObjectId::new(3)).unwrap();
    for call in calls {
        assert_eq!(
            call.await.unwrap().into_outcome().unwrap_err().kind(),
            FailureKind::TargetClosed
        );
    }
    // Assert
    assert_eq!(excess.unwrap_err().kind(), FailureKind::ResourceExhausted);
    assert_eq!(runtime.pending_call_count().unwrap(), 0);
    assert!(fixture.factory.client("target-a").calls().is_empty());
}

#[tokio::test]
async fn transport_close_interrupts_world_waiting_without_waiting_for_another_event() {
    // Arrange
    let fixture = fixture(true);
    let runtime = runtime(&fixture, NoDeadlineScheduler);
    let mut command = Box::pin(runtime.call(Command::<FrameTitle>::new(
        CallId::new(1),
        ObjectId::new(4),
        CallMetadata::default(),
        TitleParams,
    )));
    assert!(
        command
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    runtime.close_transport().unwrap();
    let result = command.await.unwrap().into_outcome();
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::TransportClosed);
    assert_eq!(runtime.pending_call_count().unwrap(), 0);
    assert!(fixture.factory.client("target-a").calls().is_empty());
}

#[tokio::test]
async fn abandoned_world_wait_releases_admission_through_existing_cleanup_executor() {
    // Arrange
    let fixture = fixture(true);
    let runtime = runtime(&fixture, NoDeadlineScheduler);
    let baseline = fixture
        .prepared
        .resources()
        .metrics()
        .unwrap()
        .class(ResourceClass::BackendStateBytes)
        .current;
    let mut command = Box::pin(runtime.call(Command::<FrameTitle>::new(
        CallId::new(1),
        ObjectId::new(4),
        CallMetadata::default(),
        TitleParams,
    )));
    assert!(
        command
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    drop(command);
    while let Some(cleanup) = runtime.cleanup_executor().try_take().unwrap() {
        cleanup.await.unwrap();
    }
    // Assert
    assert_eq!(runtime.pending_call_count().unwrap(), 0);
    assert_eq!(
        fixture
            .prepared
            .resources()
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        baseline
    );
    assert!(fixture.factory.client("target-a").calls().is_empty());
}
