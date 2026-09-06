use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use pwright_backend_cdp_topology::{
    CdpContextOwnership, CdpPageObjectIds, CdpPageTarget, CdpTopologyLimits,
    CdpTopologyProductionProfile, ConnectionEpoch, ProductionCdpTopology,
};
use pwright_browser_model::{
    BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND, BROWSER_OBJECT_KIND_CATALOG,
    FRAME_OBJECT_KIND, PAGE_OBJECT_KIND,
};
use pwright_cap_page_lifecycle::{PageClose, PageLifecycleErrorDetails, PageRunBeforeUnload};
use pwright_cdp::CdpClient;
use pwright_composition_chromium_page_lifecycle::{
    CdpPageCloseLimits, ChromiumPageLifecycleBindings, ChromiumPageLifecycleInstallError,
    ChromiumPageLifecycleInstallation, ChromiumPageLifecycleProfile, ChromiumPageRetirementPlan,
    ChromiumPageRetirementPlugin, ChromiumPageSessionResolver, ChromiumPageSessionRoute,
    FrameLimits, ImmediatePageClosePoll, PageLifecycleCodecLimits, PreparedChromiumPageRetirement,
};
use pwright_core::{
    CallId, CallMetadata, Command, ControlMessage, FailureKind, ObjectId, OperationError,
    ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceRequirements, ResourceScope,
    Service,
};
use pwright_fake::FakeCdpClient;
use pwright_runtime::{
    CleanupQueue, ControlledDispatcher, NoDeadlineScheduler, ObjectTopologyMode, RuntimeLimits,
};

type TestRuntime = ControlledDispatcher<(), (), NoDeadlineScheduler, CleanupQueue>;
type RecordedRetirementPlan = (ObjectId, String, String, Vec<ObjectId>);

struct RecordingResolver {
    client: Arc<FakeCdpClient>,
    routes: Mutex<Vec<(ObjectId, String, String)>>,
}

impl RecordingResolver {
    fn new(client: Arc<FakeCdpClient>) -> Self {
        Self {
            client,
            routes: Mutex::new(Vec::new()),
        }
    }
}

impl ChromiumPageSessionResolver for RecordingResolver {
    fn resolve(
        &self,
        route: ChromiumPageSessionRoute<'_>,
    ) -> Result<Arc<dyn CdpClient>, OperationError<PageLifecycleErrorDetails>> {
        self.routes
            .lock()
            .expect("resolver routes should remain available")
            .push((
                route.page_id(),
                route.target_id().to_owned(),
                route.session_id().to_owned(),
            ));
        Ok(self.client.clone())
    }
}

struct Fixture {
    runtime: TestRuntime,
    topology: Arc<Mutex<ProductionCdpTopology>>,
    browser: Arc<FakeCdpClient>,
    page: Arc<FakeCdpClient>,
    resolver: Arc<RecordingResolver>,
    resources: ProductionResourceLedger,
    profile: ChromiumPageLifecycleProfile,
}

fn exact_limits(requirements: ResourceRequirements) -> ResourceLimits {
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let required = requirements.class(class);
        limits = limits.with_class(class, required.global, required.per_scope);
    }
    limits
}

fn fixture() -> Fixture {
    let runtime_limits = RuntimeLimits::new(8, 2)
        .with_active_commands(1)
        .with_control_messages(8);
    let topology_profile = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(8, 2, 2, 4, 2, 128),
        ResourceScope::extension(93, 1),
    )
    .expect("topology profile should validate");
    let frames = FrameLimits::new(4096, 0, 0, 4096);
    let profile = ChromiumPageLifecycleProfile::new(
        CdpPageCloseLimits::new(128, 2),
        PageLifecycleCodecLimits::new(1024),
        frames,
        frames,
    );
    let requirements = topology_profile
        .requirements()
        .combine_concurrent(runtime_limits.resource_requirements())
        .combine_concurrent(profile.requirements());
    let resources = ProductionResourceLedger::new(exact_limits(requirements), requirements)
        .expect("exact Page-lifecycle resources should build");
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

    let epoch = ConnectionEpoch::new(1);
    let mut topology =
        ProductionCdpTopology::new(ObjectId::new(1), epoch, topology_profile, &resources)
            .expect("topology should install");
    topology
        .register_context(
            epoch,
            "context-a",
            ObjectId::new(2),
            CdpContextOwnership::Owned,
        )
        .expect("context should register");
    topology
        .attach_page(
            epoch,
            CdpPageTarget::new("target-a", "session-a", "context-a"),
            CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
        )
        .expect("Page should attach");

    let browser = Arc::new(FakeCdpClient::from_html(""));
    let page = Arc::new(FakeCdpClient::from_html(""));
    let resolver = Arc::new(RecordingResolver::new(page.clone()));
    Fixture {
        runtime,
        topology: Arc::new(Mutex::new(topology)),
        browser,
        page,
        resolver,
        resources,
        profile,
    }
}

fn bindings(
    fixture: &Fixture,
    retirement: Arc<dyn ChromiumPageRetirementPlugin>,
    continuation: CleanupQueue,
) -> ChromiumPageLifecycleBindings<CleanupQueue> {
    ChromiumPageLifecycleBindings {
        browser_client: fixture.browser.clone(),
        session_resolver: fixture.resolver.clone(),
        topology: Arc::clone(&fixture.topology),
        runtime: fixture.runtime.lifecycle_handle(),
        retirement,
        close_poll: Arc::new(ImmediatePageClosePoll),
        continuation,
    }
}

#[derive(Default)]
struct RecordingRetirement {
    prepared: AtomicUsize,
    committed: Arc<AtomicUsize>,
    plans: Mutex<Vec<RecordedRetirementPlan>>,
}

struct PreparedRecordingRetirement {
    committed: Arc<AtomicUsize>,
}

impl PreparedChromiumPageRetirement for PreparedRecordingRetirement {
    fn commit(self: Box<Self>) {
        self.committed.fetch_add(1, Ordering::SeqCst);
    }
}

impl ChromiumPageRetirementPlugin for RecordingRetirement {
    fn prepare(
        &self,
        plan: ChromiumPageRetirementPlan<'_>,
    ) -> Result<Box<dyn PreparedChromiumPageRetirement>, OperationError<PageLifecycleErrorDetails>>
    {
        self.prepared.fetch_add(1, Ordering::SeqCst);
        self.plans
            .lock()
            .expect("plugin plans should remain available")
            .push((
                plan.page_id(),
                plan.target_id().to_owned(),
                plan.session_id().to_owned(),
                plan.disposal_order().to_vec(),
            ));
        assert_eq!(
            plan.topology().page_object_id("target-a"),
            None,
            "plugin must observe staged post-close topology"
        );
        Ok(Box::new(PreparedRecordingRetirement {
            committed: Arc::clone(&self.committed),
        }))
    }
}

struct RejectingRetirement;

impl ChromiumPageRetirementPlugin for RejectingRetirement {
    fn prepare(
        &self,
        _plan: ChromiumPageRetirementPlan<'_>,
    ) -> Result<Box<dyn PreparedChromiumPageRetirement>, OperationError<PageLifecycleErrorDetails>>
    {
        Err(OperationError::new(
            FailureKind::ResourceExhausted,
            "route capacity unavailable",
            None,
        ))
    }
}

fn close_command() -> Command<PageClose> {
    Command::new(
        CallId::new(9),
        ObjectId::new(3),
        CallMetadata::default(),
        (),
    )
}

#[tokio::test]
async fn ordinary_close_commits_native_topology_runtime_and_plugin() {
    // Arrange
    let fixture = fixture();
    let plugin = Arc::new(RecordingRetirement::default());
    let installation = ChromiumPageLifecycleInstallation::install(
        bindings(&fixture, plugin.clone(), CleanupQueue::new(1)),
        fixture.profile,
        &fixture.resources,
    )
    .expect("Page-lifecycle composition should install");

    // Act
    let outcome = installation
        .close_service()
        .call(close_command())
        .await
        .expect("command service is infallible")
        .into_outcome();
    let _handlers = installation.handlers();

    // Assert
    assert_eq!(outcome, Ok(()));
    assert_eq!(
        fixture.browser.method_names(),
        ["Target.closeTarget", "Target.getTargets"]
    );
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(2)), Ok(true));
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(3)), Ok(false));
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(4)), Ok(false));
    assert_eq!(
        fixture
            .topology
            .lock()
            .expect("topology should remain available")
            .page_object_id("target-a"),
        None
    );
    assert_eq!(plugin.prepared.load(Ordering::SeqCst), 1);
    assert_eq!(plugin.committed.load(Ordering::SeqCst), 1);
    assert_eq!(
        plugin
            .plans
            .lock()
            .expect("plugin plans should remain available")
            .as_slice(),
        [(
            ObjectId::new(3),
            "target-a".to_string(),
            "session-a".to_string(),
            vec![ObjectId::new(4), ObjectId::new(3)]
        )]
    );
    assert_eq!(
        fixture.runtime.pop_control_message(),
        Ok(Some(ControlMessage::Dispose {
            object_id: ObjectId::new(4)
        }))
    );
}

#[tokio::test]
async fn missing_native_absence_proof_rolls_back_every_prepared_owner() {
    // Arrange
    let fixture = fixture();
    fixture
        .browser
        .set_target_responses([vec![target("target-a")], vec![target("target-a")]]);
    let plugin = Arc::new(RecordingRetirement::default());
    let installation = ChromiumPageLifecycleInstallation::install(
        bindings(&fixture, plugin.clone(), CleanupQueue::new(1)),
        fixture.profile,
        &fixture.resources,
    )
    .expect("Page-lifecycle composition should install");

    // Act
    let error = installation
        .close_service()
        .call(close_command())
        .await
        .expect("command service is infallible")
        .into_outcome()
        .expect_err("target still present must fail");

    // Assert
    assert_eq!(error.kind(), FailureKind::Backend);
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(3)), Ok(true));
    assert_eq!(
        fixture
            .topology
            .lock()
            .expect("topology should remain available")
            .page_object_id("target-a"),
        Some(ObjectId::new(3))
    );
    assert_eq!(plugin.prepared.load(Ordering::SeqCst), 1);
    assert_eq!(plugin.committed.load(Ordering::SeqCst), 0);
    assert_eq!(fixture.runtime.lifecycle_transaction_active(), Ok(false));
    assert!(
        !fixture
            .topology
            .lock()
            .expect("topology should remain available")
            .transaction_active()
    );
}

#[tokio::test]
async fn retirement_rejection_happens_before_any_browser_interaction() {
    // Arrange
    let fixture = fixture();
    let installation = ChromiumPageLifecycleInstallation::install(
        bindings(
            &fixture,
            Arc::new(RejectingRetirement),
            CleanupQueue::new(1),
        ),
        fixture.profile,
        &fixture.resources,
    )
    .expect("Page-lifecycle composition should install");

    // Act
    let error = installation
        .close_service()
        .call(close_command())
        .await
        .expect("command service is infallible")
        .into_outcome()
        .expect_err("plugin rejection should surface");

    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert!(fixture.browser.method_names().is_empty());
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(3)), Ok(true));
    assert_eq!(fixture.runtime.lifecycle_transaction_active(), Ok(false));
    assert!(
        !fixture
            .topology
            .lock()
            .expect("topology should remain available")
            .transaction_active()
    );
}

#[tokio::test]
async fn before_unload_uses_exact_page_session_without_retiring_the_page() {
    // Arrange
    let fixture = fixture();
    let installation = ChromiumPageLifecycleInstallation::install(
        bindings(
            &fixture,
            Arc::new(RecordingRetirement::default()),
            CleanupQueue::new(1),
        ),
        fixture.profile,
        &fixture.resources,
    )
    .expect("Page-lifecycle composition should install");
    let command = Command::<PageRunBeforeUnload>::new(
        CallId::new(10),
        ObjectId::new(3),
        CallMetadata::default(),
        (),
    );

    // Act
    let outcome = installation
        .run_before_unload_service()
        .call(command)
        .await
        .expect("command service is infallible")
        .into_outcome();

    // Assert
    assert_eq!(outcome, Ok(()));
    assert_eq!(fixture.page.method_names(), ["Page.close"]);
    assert_eq!(
        fixture
            .resolver
            .routes
            .lock()
            .expect("resolver routes should remain available")
            .as_slice(),
        [(
            ObjectId::new(3),
            "target-a".to_string(),
            "session-a".to_string()
        )]
    );
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(3)), Ok(true));
    assert_eq!(
        fixture
            .topology
            .lock()
            .expect("topology should remain available")
            .page_object_id("target-a"),
        Some(ObjectId::new(3))
    );
}

fn target(target_id: &str) -> pwright_cdp::domains::target::TargetInfo {
    pwright_cdp::domains::target::TargetInfo {
        target_id: target_id.to_string(),
        target_type: "page".to_string(),
        title: String::new(),
        url: "about:blank".to_string(),
        attached: true,
    }
}

fn poll_now<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    let mut context = Context::from_waker(Waker::noop());
    future.poll(&mut context)
}

use std::future::Future;

#[test]
fn dropped_started_close_transfers_exact_transaction_to_caller_queue() {
    // Arrange
    let fixture = fixture();
    fixture.browser.pause_target_close();
    let continuation = CleanupQueue::new(1);
    let installation = ChromiumPageLifecycleInstallation::install(
        bindings(
            &fixture,
            Arc::new(RecordingRetirement::default()),
            continuation.clone(),
        ),
        fixture.profile,
        &fixture.resources,
    )
    .expect("Page-lifecycle composition should install");
    let service = installation.close_service();
    let mut close = Box::pin(service.call(close_command()));

    // Act
    assert!(poll_now(close.as_mut()).is_pending());
    drop(close);
    fixture.browser.resume_target_close();
    let mut job = continuation
        .try_take()
        .expect("continuation queue should load")
        .expect("started close should transfer its future");
    let completed = poll_now(job.as_mut());

    // Assert
    assert!(matches!(completed, Poll::Ready(Ok(()))));
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(3)), Ok(false));
    assert_eq!(
        fixture
            .topology
            .lock()
            .expect("topology should remain available")
            .page_object_id("target-a"),
        None
    );
    let metrics = continuation.metrics().expect("queue metrics should load");
    assert_eq!(metrics.reserved, 0);
    assert_eq!(metrics.queued, 0);
    assert_eq!(metrics.completed, 1);
}

#[test]
fn missing_continuation_capacity_rejects_before_any_owner_changes() {
    // Arrange
    let fixture = fixture();
    let installation = ChromiumPageLifecycleInstallation::install(
        bindings(
            &fixture,
            Arc::new(RecordingRetirement::default()),
            CleanupQueue::new(0),
        ),
        fixture.profile,
        &fixture.resources,
    )
    .expect("Page-lifecycle composition should install");
    let service = installation.close_service();
    let mut close = Box::pin(service.call(close_command()));

    // Act
    let Poll::Ready(Ok(result)) = poll_now(close.as_mut()) else {
        panic!("missing continuation capacity should reject synchronously");
    };
    let error = result
        .into_outcome()
        .expect_err("close should reject before native work");

    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert!(fixture.browser.method_names().is_empty());
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(3)), Ok(true));
    assert_eq!(fixture.runtime.lifecycle_transaction_active(), Ok(false));
    assert!(
        !fixture
            .topology
            .lock()
            .expect("topology should remain available")
            .transaction_active()
    );
}

#[test]
fn invalid_backend_profile_fails_during_installation() {
    // Arrange
    let fixture = fixture();
    let invalid = ChromiumPageLifecycleProfile::new(
        CdpPageCloseLimits::new(0, 1),
        fixture.profile.codec(),
        fixture.profile.send_limits(),
        fixture.profile.receive_limits(),
    );

    // Act
    let result = ChromiumPageLifecycleInstallation::install(
        bindings(
            &fixture,
            Arc::new(RecordingRetirement::default()),
            CleanupQueue::new(1),
        ),
        invalid,
        &fixture.resources,
    );

    // Assert
    assert!(matches!(
        result,
        Err(ChromiumPageLifecycleInstallError::Backend(_))
    ));
    assert!(fixture.browser.method_names().is_empty());
}
