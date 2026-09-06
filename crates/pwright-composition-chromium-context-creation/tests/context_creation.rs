use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use pwright_backend_cdp_topology::{
    CdpTopologyLimits, CdpTopologyProductionProfile, ConnectionEpoch, ProductionCdpTopology,
};
use pwright_browser_model::{BROWSER_OBJECT_KIND, BROWSER_OBJECT_KIND_CATALOG};
use pwright_cap_context_creation::{BrowserNewContext, ContextCreationErrorDetails};
use pwright_cap_context_lifecycle::BrowserContextClose;
use pwright_cdp::CdpClient;
use pwright_composition_chromium_context_creation::{
    AtomicObjectIdSource, ChromiumContextContinuations, ChromiumContextCoreBindings,
    ChromiumContextCreationInstallation, ChromiumContextCreationProfile,
    ChromiumContextLifecycleProfile, ChromiumContextPlugins, ChromiumContextPublicationPlugin,
    ContextCreationCodecLimits, ContextLifecycleCodecLimits, FrameLimits,
    NoopChromiumContextRetirement, PreparedChromiumContextPublication,
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

struct Fixture {
    runtime: TestRuntime,
    topology: Arc<Mutex<ProductionCdpTopology>>,
    cdp: Arc<FakeCdpClient>,
    resources: ProductionResourceLedger,
    profile: ChromiumContextCreationProfile,
}

fn exact_limits(requirements: ResourceRequirements) -> ResourceLimits {
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let required = requirements.class(class);
        limits = limits.with_class(class, required.global, required.per_scope);
    }
    limits
}

fn fixture(context_capacity: usize) -> Fixture {
    let runtime_limits = RuntimeLimits::new(8, 2)
        .with_active_commands(1)
        .with_control_messages(8);
    let topology_profile = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(8, context_capacity, 2, 4, 2, 128),
        ResourceScope::extension(93, 1),
    )
    .expect("topology profile should validate");
    let frames = FrameLimits::new(4096, 0, 0, 4096);
    let lifecycle = ChromiumContextLifecycleProfile::new(
        ContextLifecycleCodecLimits::new(1024),
        frames,
        frames,
    );
    let profile = ChromiumContextCreationProfile::new(
        pwright_composition_chromium_context_creation::CdpContextCreationLimits::new(128),
        ContextCreationCodecLimits::new(1024),
        frames,
        frames,
        lifecycle,
    );
    let requirements = topology_profile
        .requirements()
        .combine_concurrent(runtime_limits.resource_requirements())
        .combine_concurrent(profile.requirements());
    let resources = ProductionResourceLedger::new(exact_limits(requirements), requirements)
        .expect("exact Context-creation resources should build");
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
    assert!(
        runtime
            .pop_control_message()
            .expect("control queue should load")
            .is_some()
    );
    let topology = ProductionCdpTopology::new(
        ObjectId::new(1),
        ConnectionEpoch::new(1),
        topology_profile,
        &resources,
    )
    .expect("topology should install");
    let cdp = Arc::new(FakeCdpClient::from_html(""));
    cdp.set_create_browser_context_response("context-created");
    Fixture {
        runtime,
        topology: Arc::new(Mutex::new(topology)),
        cdp,
        resources,
        profile,
    }
}

#[derive(Default)]
struct RecordingPublication {
    prepared: AtomicUsize,
    committed: Arc<AtomicUsize>,
    identities: Mutex<Vec<(ObjectId, ObjectId)>>,
}

struct PreparedRecordingPublication {
    committed: Arc<AtomicUsize>,
}

impl PreparedChromiumContextPublication for PreparedRecordingPublication {
    fn commit(self: Box<Self>) {
        self.committed.fetch_add(1, Ordering::SeqCst);
    }
}

impl ChromiumContextPublicationPlugin for RecordingPublication {
    fn prepare(
        &self,
        context_id: ObjectId,
        browser_id: ObjectId,
    ) -> Result<
        Box<dyn PreparedChromiumContextPublication>,
        OperationError<ContextCreationErrorDetails>,
    > {
        self.prepared.fetch_add(1, Ordering::SeqCst);
        self.identities
            .lock()
            .expect("recording plugin should remain available")
            .push((context_id, browser_id));
        Ok(Box::new(PreparedRecordingPublication {
            committed: Arc::clone(&self.committed),
        }))
    }
}

struct RejectingPublication;

impl ChromiumContextPublicationPlugin for RejectingPublication {
    fn prepare(
        &self,
        _context_id: ObjectId,
        _browser_id: ObjectId,
    ) -> Result<
        Box<dyn PreparedChromiumContextPublication>,
        OperationError<ContextCreationErrorDetails>,
    > {
        Err(OperationError::new(
            FailureKind::ResourceExhausted,
            "publication plugin is full",
            Some(ContextCreationErrorDetails::Publication(
                "plugin capacity exhausted".to_string(),
            )),
        ))
    }
}

fn bindings(fixture: &Fixture) -> ChromiumContextCoreBindings {
    let browser_client: Arc<dyn CdpClient> = fixture.cdp.clone();
    ChromiumContextCoreBindings {
        browser_client,
        topology: Arc::clone(&fixture.topology),
        runtime: fixture.runtime.lifecycle_handle(),
        object_ids: Arc::new(AtomicObjectIdSource::new(2).expect("object IDs should start at two")),
        browser_object_id: ObjectId::new(1),
    }
}

fn continuations(
    creation: usize,
    close: usize,
) -> ChromiumContextContinuations<CleanupQueue, CleanupQueue> {
    ChromiumContextContinuations {
        creation: CleanupQueue::new(creation),
        close: CleanupQueue::new(close),
    }
}

fn create_command() -> Command<BrowserNewContext> {
    Command::new(
        CallId::new(1),
        ObjectId::new(1),
        CallMetadata::default(),
        (),
    )
}

#[tokio::test]
async fn paired_installation_creates_and_closes_the_same_context() {
    // Arrange
    let fixture = fixture(2);
    let publication = Arc::new(RecordingPublication::default());
    let installation = ChromiumContextCreationInstallation::install(
        bindings(&fixture),
        ChromiumContextPlugins {
            publication: publication.clone(),
            retirement: Arc::new(NoopChromiumContextRetirement),
        },
        continuations(1, 1),
        fixture.profile,
        &fixture.resources,
    )
    .expect("paired Context composition should install");

    // Act
    let initializer = installation
        .creation_service()
        .call(create_command())
        .await
        .expect("creation service is infallible")
        .into_outcome()
        .expect("Context should be created");
    let _handlers = installation.handlers();
    installation
        .lifecycle()
        .service()
        .call(Command::<BrowserContextClose>::new(
            CallId::new(2),
            initializer.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .expect("lifecycle service is infallible")
        .into_outcome()
        .expect("created Context should close through the paired lifecycle");

    // Assert
    assert_eq!(initializer.object_id, ObjectId::new(2));
    assert_eq!(initializer.parent_id, ObjectId::new(1));
    assert_eq!(publication.prepared.load(Ordering::SeqCst), 1);
    assert_eq!(publication.committed.load(Ordering::SeqCst), 1);
    assert_eq!(
        publication
            .identities
            .lock()
            .expect("recorded identities should remain available")
            .as_slice(),
        [(ObjectId::new(2), ObjectId::new(1))]
    );
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(2)), Ok(false));
    assert_eq!(
        fixture.cdp.method_names(),
        [
            "Target.createBrowserContext",
            "Target.disposeBrowserContext"
        ]
    );
}

#[tokio::test]
async fn cdp_failure_rolls_back_runtime_and_publication() {
    // Arrange
    let fixture = fixture(2);
    fixture
        .cdp
        .set_create_browser_context_error(Some("creation rejected".to_string()));
    let publication = Arc::new(RecordingPublication::default());
    let installation = ChromiumContextCreationInstallation::install(
        bindings(&fixture),
        ChromiumContextPlugins {
            publication: publication.clone(),
            retirement: Arc::new(NoopChromiumContextRetirement),
        },
        continuations(1, 1),
        fixture.profile,
        &fixture.resources,
    )
    .expect("paired Context composition should install");

    // Act
    let error = installation
        .creation_service()
        .call(create_command())
        .await
        .expect("creation service is infallible")
        .into_outcome()
        .expect_err("CDP failure should surface");

    // Assert
    assert_eq!(error.kind(), FailureKind::Backend);
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(2)), Ok(false));
    assert_eq!(publication.prepared.load(Ordering::SeqCst), 1);
    assert_eq!(publication.committed.load(Ordering::SeqCst), 0);
    assert_eq!(fixture.runtime.lifecycle_transaction_active(), Ok(false));
    assert_eq!(fixture.cdp.method_names(), ["Target.createBrowserContext"]);
}

#[tokio::test]
async fn topology_failure_compensates_the_exact_native_context() {
    // Arrange
    let fixture = fixture(0);
    let installation = ChromiumContextCreationInstallation::install_without_plugins(
        bindings(&fixture),
        continuations(1, 1),
        fixture.profile,
        &fixture.resources,
    )
    .expect("paired Context composition should install");

    // Act
    let error = installation
        .creation_service()
        .call(create_command())
        .await
        .expect("creation service is infallible")
        .into_outcome()
        .expect_err("full topology should reject publication");

    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(2)), Ok(false));
    assert_eq!(
        fixture.cdp.method_names(),
        [
            "Target.createBrowserContext",
            "Target.disposeBrowserContext"
        ]
    );
    assert_eq!(
        fixture.cdp.calls()[1].1,
        [serde_json::Value::String("context-created".to_string())]
    );
}

#[test]
fn publication_capacity_rejects_before_runtime_and_cdp() {
    // Arrange
    let fixture = fixture(2);
    let installation = ChromiumContextCreationInstallation::install(
        bindings(&fixture),
        ChromiumContextPlugins {
            publication: Arc::new(RejectingPublication),
            retirement: Arc::new(NoopChromiumContextRetirement),
        },
        continuations(1, 1),
        fixture.profile,
        &fixture.resources,
    )
    .expect("paired Context composition should install");

    // Act
    let mut creation = Box::pin(installation.creation_service().call(create_command()));
    let Poll::Ready(Ok(result)) = poll_now(creation.as_mut()) else {
        panic!("synchronous plugin rejection should be ready");
    };
    let error = result
        .into_outcome()
        .expect_err("plugin capacity should reject creation");

    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert!(fixture.cdp.method_names().is_empty());
    assert_eq!(fixture.runtime.lifecycle_transaction_active(), Ok(false));
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(2)), Ok(false));
}

fn poll_now<F: std::future::Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    let mut context = Context::from_waker(Waker::noop());
    future.poll(&mut context)
}

#[test]
fn dropped_started_creation_transfers_the_exact_transaction() {
    // Arrange
    let fixture = fixture(2);
    fixture.cdp.pause_create_browser_context();
    let creation_queue = CleanupQueue::new(1);
    let installation = ChromiumContextCreationInstallation::install_without_plugins(
        bindings(&fixture),
        ChromiumContextContinuations {
            creation: creation_queue.clone(),
            close: CleanupQueue::new(1),
        },
        fixture.profile,
        &fixture.resources,
    )
    .expect("paired Context composition should install");
    let service = installation.creation_service();
    let mut creation = Box::pin(service.call(create_command()));

    // Act
    assert!(poll_now(creation.as_mut()).is_pending());
    drop(creation);
    fixture.cdp.resume_create_browser_context();
    let mut job = creation_queue
        .try_take()
        .expect("continuation queue should load")
        .expect("started creation should transfer");
    let completed = poll_now(job.as_mut());

    // Assert
    assert!(matches!(completed, Poll::Ready(Ok(()))));
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(2)), Ok(true));
    assert_eq!(
        fixture
            .topology
            .lock()
            .expect("topology should remain available")
            .context_object_id("context-created"),
        Some(ObjectId::new(2))
    );
    assert_eq!(
        creation_queue
            .metrics()
            .expect("metrics should load")
            .completed,
        1
    );
}

#[test]
fn missing_creation_continuation_rejects_before_object_id_or_cdp() {
    // Arrange
    let fixture = fixture(2);
    let installation = ChromiumContextCreationInstallation::install_without_plugins(
        bindings(&fixture),
        continuations(0, 1),
        fixture.profile,
        &fixture.resources,
    )
    .expect("paired Context composition should install");
    let service = installation.creation_service();
    let mut creation = Box::pin(service.call(create_command()));

    // Act
    let Poll::Ready(Ok(result)) = poll_now(creation.as_mut()) else {
        panic!("missing continuation capacity should reject synchronously");
    };
    let error = result
        .into_outcome()
        .expect_err("creation should reject before CDP");

    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert!(fixture.cdp.method_names().is_empty());
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(2)), Ok(false));
    assert_eq!(fixture.runtime.lifecycle_transaction_active(), Ok(false));
}
