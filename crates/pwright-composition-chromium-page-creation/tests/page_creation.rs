use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use pwright_backend_cdp_topology::{
    CdpContextOwnership, CdpTopologyLimits, CdpTopologyProductionProfile, ConnectionEpoch,
    ProductionCdpTopology,
};
use pwright_browser_model::{
    BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND, BROWSER_OBJECT_KIND_CATALOG,
};
use pwright_cap_page_creation::{BrowserContextNewPage, PageCreationErrorDetails, PageInitializer};
use pwright_cap_page_lifecycle::PageClose;
use pwright_cdp::{CdpClient, SessionFactory};
use pwright_composition_chromium_page_creation::{
    AtomicObjectIdSource, CdpPageCloseLimits, CdpPageCreationLimits, ChromiumCreatedPageRoute,
    ChromiumPageContinuations, ChromiumPageCoreBindings, ChromiumPageCreationInstallation,
    ChromiumPageCreationProfile, ChromiumPageLifecycleProfile, ChromiumPagePlugins,
    ChromiumPagePublicationPlan, ChromiumPagePublicationPlugin, FrameLimits,
    ImmediatePageClosePoll, NoopChromiumPageRetirement, PageCreationCodecLimits,
    PageLifecycleCodecLimits, PreparedChromiumPagePublication, SessionFactoryChromiumPageResolver,
};
use pwright_core::{
    CallId, CallMetadata, Command, FailureKind, ObjectId, OperationError, ProductionResourceLedger,
    ResourceClass, ResourceLimits, ResourceRequirements, ResourceScope, Service,
};
use pwright_fake::FakeCdpClient;
use pwright_runtime::{
    CleanupQueue, ControlledDispatcher, NoDeadlineScheduler, ObjectTopologyMode, RuntimeLimits,
};

type TestRuntime = ControlledDispatcher<(), (), NoDeadlineScheduler, CleanupQueue>;
type BoundRoute = (String, String, String);

struct FakeFactory {
    client: Arc<FakeCdpClient>,
}

impl SessionFactory for FakeFactory {
    fn create_session(&self, _session_id: String, _target_id: String) -> Arc<dyn CdpClient> {
        self.client.clone()
    }
}

struct Fixture {
    runtime: TestRuntime,
    topology: Arc<Mutex<ProductionCdpTopology>>,
    browser: Arc<FakeCdpClient>,
    factory: Arc<FakeFactory>,
    resources: ProductionResourceLedger,
    profile: ChromiumPageCreationProfile,
}

fn exact_limits(requirements: ResourceRequirements) -> ResourceLimits {
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let required = requirements.class(class);
        limits = limits.with_class(class, required.global, required.per_scope);
    }
    limits
}

fn fixture(page_capacity: usize) -> Fixture {
    let runtime_limits = RuntimeLimits::new(8, 2)
        .with_active_commands(1)
        .with_control_messages(8);
    let topology_profile = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(8, 2, page_capacity, 4, 2, 128),
        ResourceScope::extension(95, 1),
    )
    .expect("topology profile should validate");
    let frames = FrameLimits::new(4096, 0, 0, 4096);
    let lifecycle = ChromiumPageLifecycleProfile::new(
        CdpPageCloseLimits::new(128, 2),
        PageLifecycleCodecLimits::new(1024),
        frames,
        frames,
    );
    let profile = ChromiumPageCreationProfile::new(
        CdpPageCreationLimits::new(128).with_cleanup_confirmation_attempts(2),
        PageCreationCodecLimits::new(1024),
        frames,
        frames,
        lifecycle,
    );
    let requirements = topology_profile
        .requirements()
        .combine_concurrent(runtime_limits.resource_requirements())
        .combine_concurrent(profile.requirements());
    let resources = ProductionResourceLedger::new(exact_limits(requirements), requirements)
        .expect("exact Page-creation resources should build");
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
    runtime
        .register_object(ObjectId::new(1), None, BROWSER_OBJECT_KIND.tag)
        .expect("Browser should register");
    runtime
        .register_object(
            ObjectId::new(2),
            Some(ObjectId::new(1)),
            BROWSER_CONTEXT_OBJECT_KIND.tag,
        )
        .expect("Context should register");
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
        .expect("Context should register");
    let browser = Arc::new(FakeCdpClient::from_html(""));
    let page = Arc::new(FakeCdpClient::from_html(""));
    page.set_main_frame_id("frame-created");
    Fixture {
        runtime,
        topology: Arc::new(Mutex::new(topology)),
        browser,
        factory: Arc::new(FakeFactory { client: page }),
        resources,
        profile,
    }
}

fn bindings(fixture: &Fixture) -> ChromiumPageCoreBindings {
    ChromiumPageCoreBindings {
        browser_client: fixture.browser.clone(),
        session_factory: fixture.factory.clone(),
        session_resolver: Arc::new(SessionFactoryChromiumPageResolver::new(
            fixture.factory.clone(),
        )),
        topology: Arc::clone(&fixture.topology),
        runtime: fixture.runtime.lifecycle_handle(),
        object_ids: Arc::new(AtomicObjectIdSource::new(3).expect("IDs should start at three")),
    }
}

fn continuations(
    creation: usize,
    close: usize,
) -> ChromiumPageContinuations<CleanupQueue, CleanupQueue> {
    ChromiumPageContinuations {
        creation: CleanupQueue::new(creation),
        close: CleanupQueue::new(close),
    }
}

#[derive(Default)]
struct RecordingPublication {
    prepared: AtomicUsize,
    bound: Arc<Mutex<Vec<BoundRoute>>>,
    committed: Arc<AtomicUsize>,
}

struct PreparedRecordingPublication {
    bound: Arc<Mutex<Vec<BoundRoute>>>,
    committed: Arc<AtomicUsize>,
}

impl PreparedChromiumPagePublication for PreparedRecordingPublication {
    fn bind(
        &mut self,
        route: ChromiumCreatedPageRoute<'_>,
    ) -> Result<(), OperationError<PageCreationErrorDetails>> {
        self.bound
            .lock()
            .expect("bound routes should remain available")
            .push((
                route.target_id().to_owned(),
                route.session_id().to_owned(),
                route.main_frame_id().to_owned(),
            ));
        Ok(())
    }

    fn commit(self: Box<Self>) {
        self.committed.fetch_add(1, Ordering::SeqCst);
    }
}

impl ChromiumPagePublicationPlugin for RecordingPublication {
    fn prepare(
        &self,
        plan: ChromiumPagePublicationPlan,
    ) -> Result<Box<dyn PreparedChromiumPagePublication>, OperationError<PageCreationErrorDetails>>
    {
        assert_eq!(plan.page_id(), ObjectId::new(3));
        assert_eq!(plan.main_frame_id(), ObjectId::new(4));
        assert_eq!(plan.context_id(), ObjectId::new(2));
        self.prepared.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(PreparedRecordingPublication {
            bound: Arc::clone(&self.bound),
            committed: Arc::clone(&self.committed),
        }))
    }
}

struct RejectingPublication;

impl ChromiumPagePublicationPlugin for RejectingPublication {
    fn prepare(
        &self,
        _plan: ChromiumPagePublicationPlan,
    ) -> Result<Box<dyn PreparedChromiumPagePublication>, OperationError<PageCreationErrorDetails>>
    {
        Err(OperationError::new(
            FailureKind::ResourceExhausted,
            "publication capacity unavailable",
            None,
        ))
    }
}

struct RejectingBindPublication;

struct PreparedRejectingBindPublication;

impl PreparedChromiumPagePublication for PreparedRejectingBindPublication {
    fn bind(
        &mut self,
        _route: ChromiumCreatedPageRoute<'_>,
    ) -> Result<(), OperationError<PageCreationErrorDetails>> {
        Err(OperationError::new(
            FailureKind::ResourceExhausted,
            "route binding unavailable",
            None,
        ))
    }

    fn commit(self: Box<Self>) {
        panic!("rejected binding must not commit");
    }
}

impl ChromiumPagePublicationPlugin for RejectingBindPublication {
    fn prepare(
        &self,
        _plan: ChromiumPagePublicationPlan,
    ) -> Result<Box<dyn PreparedChromiumPagePublication>, OperationError<PageCreationErrorDetails>>
    {
        Ok(Box::new(PreparedRejectingBindPublication))
    }
}

fn create_command() -> Command<BrowserContextNewPage> {
    Command::new(
        CallId::new(1),
        ObjectId::new(2),
        CallMetadata::default(),
        (),
    )
}

fn install(
    fixture: &Fixture,
    publication: Arc<dyn ChromiumPagePublicationPlugin>,
    continuations: ChromiumPageContinuations<CleanupQueue, CleanupQueue>,
) -> ChromiumPageCreationInstallation<CleanupQueue, CleanupQueue> {
    ChromiumPageCreationInstallation::install(
        bindings(fixture),
        ChromiumPagePlugins {
            publication,
            retirement: Arc::new(NoopChromiumPageRetirement),
        },
        continuations,
        Arc::new(ImmediatePageClosePoll),
        fixture.profile,
        &fixture.resources,
    )
    .expect("paired Page composition should install")
}

#[tokio::test]
async fn paired_installation_creates_and_closes_the_same_page() {
    // Arrange
    let fixture = fixture(2);
    let publication = Arc::new(RecordingPublication::default());
    let installation = install(&fixture, publication.clone(), continuations(1, 1));

    // Act
    let initializer = installation
        .creation_service()
        .call(create_command())
        .await
        .expect("creation service is infallible")
        .into_outcome()
        .expect("Page should create");
    let _handlers = installation.handlers();
    installation
        .lifecycle()
        .close_service()
        .call(Command::<PageClose>::new(
            CallId::new(2),
            initializer.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("close service is infallible")
        .into_outcome()
        .expect("created Page should close");

    // Assert
    assert_eq!(
        initializer,
        PageInitializer::new(
            ObjectId::new(3),
            ObjectId::new(2),
            pwright_browser_model::PAGE_OBJECT_KIND.tag,
            ObjectId::new(4),
            ObjectId::new(3),
            pwright_browser_model::FRAME_OBJECT_KIND.tag,
        )
    );
    assert_eq!(publication.prepared.load(Ordering::SeqCst), 1);
    assert_eq!(publication.committed.load(Ordering::SeqCst), 1);
    assert_eq!(
        publication
            .bound
            .lock()
            .expect("bound routes should remain available")
            .as_slice(),
        [(
            "fake-target-1".to_string(),
            "fake-session-1".to_string(),
            "frame-created".to_string()
        )]
    );
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(3)), Ok(false));
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(4)), Ok(false));
    assert_eq!(
        fixture.browser.method_names(),
        [
            "Target.createTarget",
            "Target.attachToTarget",
            "Target.closeTarget",
            "Target.getTargets"
        ]
    );
}

#[tokio::test]
async fn publication_rejection_happens_before_native_creation() {
    // Arrange
    let fixture = fixture(2);
    let installation = install(
        &fixture,
        Arc::new(RejectingPublication),
        continuations(1, 1),
    );

    // Act
    let error = installation
        .creation_service()
        .call(create_command())
        .await
        .expect("creation service is infallible")
        .into_outcome()
        .expect_err("publication capacity must reject");

    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert!(fixture.browser.method_names().is_empty());
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(3)), Ok(false));
}

#[tokio::test]
async fn native_route_binding_failure_compensates_the_exact_target() {
    // Arrange
    let fixture = fixture(2);
    let installation = install(
        &fixture,
        Arc::new(RejectingBindPublication),
        continuations(1, 1),
    );

    // Act
    let error = installation
        .creation_service()
        .call(create_command())
        .await
        .expect("creation service is infallible")
        .into_outcome()
        .expect_err("route binding should fail");

    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert_eq!(
        fixture.browser.method_names(),
        [
            "Target.createTarget",
            "Target.attachToTarget",
            "Target.closeTarget",
            "Target.getTargets"
        ]
    );
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(3)), Ok(false));
    assert_eq!(
        fixture
            .topology
            .lock()
            .expect("topology should remain available")
            .page_object_id("fake-target-1"),
        None
    );
}

#[tokio::test]
async fn topology_capacity_failure_compensates_without_publishing_runtime() {
    // Arrange
    let fixture = fixture(0);
    let publication = Arc::new(RecordingPublication::default());
    let installation = install(&fixture, publication.clone(), continuations(1, 1));

    // Act
    let error = installation
        .creation_service()
        .call(create_command())
        .await
        .expect("creation service is infallible")
        .into_outcome()
        .expect_err("zero Page capacity should reject publication");

    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert_eq!(publication.prepared.load(Ordering::SeqCst), 1);
    assert_eq!(publication.committed.load(Ordering::SeqCst), 0);
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(3)), Ok(false));
    assert_eq!(fixture.runtime.lifecycle_transaction_active(), Ok(false));
    assert_eq!(
        fixture.browser.method_names(),
        [
            "Target.createTarget",
            "Target.attachToTarget",
            "Target.closeTarget",
            "Target.getTargets"
        ]
    );
}

fn poll_now<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    let mut context = Context::from_waker(Waker::noop());
    future.poll(&mut context)
}

#[test]
fn dropped_started_creation_transfers_exact_transaction_to_caller_queue() {
    // Arrange
    let fixture = fixture(2);
    fixture.browser.pause_target_create();
    let creation_queue = CleanupQueue::new(1);
    let installation = install(
        &fixture,
        Arc::new(RecordingPublication::default()),
        ChromiumPageContinuations {
            creation: creation_queue.clone(),
            close: CleanupQueue::new(1),
        },
    );
    let service = installation.creation_service();
    let mut creation = Box::pin(service.call(create_command()));

    // Act
    assert!(poll_now(creation.as_mut()).is_pending());
    drop(creation);
    fixture.browser.resume_target_create();
    let mut job = creation_queue
        .try_take()
        .expect("continuation queue should load")
        .expect("started creation should transfer");
    let completed = poll_now(job.as_mut());

    // Assert
    assert!(matches!(completed, Poll::Ready(Ok(()))));
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(3)), Ok(true));
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(4)), Ok(true));
    assert_eq!(
        fixture
            .topology
            .lock()
            .expect("topology should remain available")
            .page_object_id("fake-target-1"),
        Some(ObjectId::new(3))
    );
}

#[test]
fn missing_creation_continuation_rejects_before_any_owner_changes() {
    // Arrange
    let fixture = fixture(2);
    let installation = install(
        &fixture,
        Arc::new(RecordingPublication::default()),
        continuations(0, 1),
    );
    let service = installation.creation_service();
    let mut creation = Box::pin(service.call(create_command()));

    // Act
    let Poll::Ready(Ok(result)) = poll_now(creation.as_mut()) else {
        panic!("missing continuation capacity should reject synchronously");
    };
    let error = result
        .into_outcome()
        .expect_err("creation should reject before native work");

    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert!(fixture.browser.method_names().is_empty());
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(3)), Ok(false));
    assert_eq!(fixture.runtime.lifecycle_transaction_active(), Ok(false));
}
