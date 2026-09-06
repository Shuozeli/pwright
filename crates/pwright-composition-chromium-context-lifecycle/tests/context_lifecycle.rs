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
use pwright_cap_context_lifecycle::{BrowserContextClose, ContextLifecycleErrorDetails};
use pwright_composition_chromium_context_lifecycle::{
    ChromiumContextLifecycleInstallation, ChromiumContextLifecycleProfile,
    ChromiumContextRetirementPlan, ChromiumContextRetirementPlugin, FrameLimits,
    PreparedChromiumContextRetirement,
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

struct Fixture {
    runtime: TestRuntime,
    topology: Arc<Mutex<ProductionCdpTopology>>,
    cdp: Arc<FakeCdpClient>,
    resources: ProductionResourceLedger,
    profile: ChromiumContextLifecycleProfile,
}

fn exact_limits(requirements: ResourceRequirements) -> ResourceLimits {
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let required = requirements.class(class);
        limits = limits.with_class(class, required.global, required.per_scope);
    }
    limits
}

fn fixture(ownership: CdpContextOwnership) -> Fixture {
    let runtime_limits = RuntimeLimits::new(8, 2)
        .with_active_commands(1)
        .with_control_messages(8);
    let topology_profile = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(8, 2, 2, 4, 2, 128),
        ResourceScope::extension(91, 1),
    )
    .expect("topology profile should validate");
    let frames = FrameLimits::new(4096, 0, 0, 4096);
    let profile = ChromiumContextLifecycleProfile::new(
        pwright_composition_chromium_context_lifecycle::ContextLifecycleCodecLimits::new(1024),
        frames,
        frames,
    );
    let requirements = topology_profile
        .requirements()
        .combine_concurrent(runtime_limits.resource_requirements())
        .combine_concurrent(profile.requirements());
    let resources = ProductionResourceLedger::new(exact_limits(requirements), requirements)
        .expect("exact Context-lifecycle resources should build");
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
        .register_context(epoch, "context-a", ObjectId::new(2), ownership)
        .expect("context should register");
    topology
        .attach_page(
            epoch,
            CdpPageTarget::new("target-a", "session-a", "context-a"),
            CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
        )
        .expect("Page should attach");

    Fixture {
        runtime,
        topology: Arc::new(Mutex::new(topology)),
        cdp: Arc::new(FakeCdpClient::from_html("")),
        resources,
        profile,
    }
}

#[derive(Default)]
struct RecordingRetirement {
    prepared: AtomicUsize,
    committed: Arc<AtomicUsize>,
    plans: Mutex<Vec<(ObjectId, Vec<ObjectId>)>>,
}

struct PreparedRecordingRetirement {
    committed: Arc<AtomicUsize>,
}

impl PreparedChromiumContextRetirement for PreparedRecordingRetirement {
    fn commit(self: Box<Self>) {
        self.committed.fetch_add(1, Ordering::SeqCst);
    }
}

impl ChromiumContextRetirementPlugin for RecordingRetirement {
    fn prepare(
        &self,
        plan: ChromiumContextRetirementPlan<'_>,
    ) -> Result<
        Box<dyn PreparedChromiumContextRetirement>,
        OperationError<ContextLifecycleErrorDetails>,
    > {
        self.prepared.fetch_add(1, Ordering::SeqCst);
        self.plans
            .lock()
            .expect("recording plugin should remain available")
            .push((plan.context_id(), plan.disposal_order().to_vec()));
        assert_eq!(
            plan.topology().context_object_id("context-a"),
            None,
            "plugin must observe the staged post-close topology"
        );
        Ok(Box::new(PreparedRecordingRetirement {
            committed: Arc::clone(&self.committed),
        }))
    }
}

fn close_command() -> Command<BrowserContextClose> {
    Command::new(
        CallId::new(9),
        ObjectId::new(2),
        CallMetadata::default(),
        (),
    )
}

#[tokio::test]
async fn owned_close_commits_native_topology_runtime_and_plugin() {
    // Arrange
    let fixture = fixture(CdpContextOwnership::Owned);
    let plugin = Arc::new(RecordingRetirement::default());
    let continuation = CleanupQueue::new(1);
    let installation = ChromiumContextLifecycleInstallation::install(
        fixture.cdp.clone(),
        Arc::clone(&fixture.topology),
        fixture.runtime.lifecycle_handle(),
        plugin.clone(),
        continuation,
        fixture.profile,
        &fixture.resources,
    )
    .expect("Context-lifecycle composition should install");

    // Act
    let outcome = installation
        .service()
        .call(close_command())
        .await
        .expect("command service is infallible")
        .into_outcome();
    let _handlers = installation.handlers();

    // Assert
    assert_eq!(outcome, Ok(()));
    assert_eq!(fixture.cdp.method_names(), ["Target.disposeBrowserContext"]);
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(1)), Ok(true));
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(2)), Ok(false));
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(3)), Ok(false));
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(4)), Ok(false));
    assert_eq!(
        fixture
            .topology
            .lock()
            .expect("topology should remain available")
            .context_object_id("context-a"),
        None
    );
    assert_eq!(plugin.prepared.load(Ordering::SeqCst), 1);
    assert_eq!(plugin.committed.load(Ordering::SeqCst), 1);
    assert_eq!(
        plugin
            .plans
            .lock()
            .expect("recorded plans should remain available")
            .as_slice(),
        [(
            ObjectId::new(2),
            vec![ObjectId::new(4), ObjectId::new(3), ObjectId::new(2)]
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
async fn backend_failure_rolls_back_every_prepared_owner() {
    // Arrange
    let fixture = fixture(CdpContextOwnership::Owned);
    fixture
        .cdp
        .set_dispose_browser_context_error(Some("close refused".to_string()));
    let plugin = Arc::new(RecordingRetirement::default());
    let installation = ChromiumContextLifecycleInstallation::install(
        fixture.cdp.clone(),
        Arc::clone(&fixture.topology),
        fixture.runtime.lifecycle_handle(),
        plugin.clone(),
        CleanupQueue::new(1),
        fixture.profile,
        &fixture.resources,
    )
    .expect("Context-lifecycle composition should install");

    // Act
    let error = installation
        .service()
        .call(close_command())
        .await
        .expect("command service is infallible")
        .into_outcome()
        .expect_err("native failure should remain visible");

    // Assert
    assert_eq!(error.kind(), FailureKind::Backend);
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(2)), Ok(true));
    assert_eq!(
        fixture
            .topology
            .lock()
            .expect("topology should remain available")
            .context_object_id("context-a"),
        Some(ObjectId::new(2))
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
async fn unowned_context_is_rejected_before_plugin_and_cdp() {
    // Arrange
    let fixture = fixture(CdpContextOwnership::External);
    let plugin = Arc::new(RecordingRetirement::default());
    let installation = ChromiumContextLifecycleInstallation::install(
        fixture.cdp.clone(),
        Arc::clone(&fixture.topology),
        fixture.runtime.lifecycle_handle(),
        plugin.clone(),
        CleanupQueue::new(1),
        fixture.profile,
        &fixture.resources,
    )
    .expect("Context-lifecycle composition should install");

    // Act
    let error = installation
        .service()
        .call(close_command())
        .await
        .expect("command service is infallible")
        .into_outcome()
        .expect_err("external Context must not close");

    // Assert
    assert_eq!(error.kind(), FailureKind::UnsupportedCapability);
    assert!(fixture.cdp.method_names().is_empty());
    assert_eq!(plugin.prepared.load(Ordering::SeqCst), 0);
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(2)), Ok(true));
    assert_eq!(
        fixture
            .topology
            .lock()
            .expect("topology should remain available")
            .context_object_id("context-a"),
        Some(ObjectId::new(2))
    );
}

fn poll_now<F: std::future::Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    let mut context = Context::from_waker(Waker::noop());
    future.poll(&mut context)
}

#[test]
fn dropped_started_close_transfers_exact_transaction_to_caller_queue() {
    // Arrange
    let fixture = fixture(CdpContextOwnership::Owned);
    fixture.cdp.pause_dispose_browser_context();
    let continuation = CleanupQueue::new(1);
    let installation = ChromiumContextLifecycleInstallation::install_without_retirement_plugins(
        fixture.cdp.clone(),
        Arc::clone(&fixture.topology),
        fixture.runtime.lifecycle_handle(),
        continuation.clone(),
        fixture.profile,
        &fixture.resources,
    )
    .expect("Context-lifecycle composition should install");
    let service = installation.service();
    let mut close = Box::pin(service.call(close_command()));

    // Act
    assert!(poll_now(close.as_mut()).is_pending());
    drop(close);
    fixture.cdp.resume_dispose_browser_context();
    let mut job = continuation
        .try_take()
        .expect("continuation queue should load")
        .expect("started close should transfer its future");
    let completed = poll_now(job.as_mut());

    // Assert
    assert!(matches!(completed, Poll::Ready(Ok(()))));
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(2)), Ok(false));
    assert_eq!(
        fixture
            .topology
            .lock()
            .expect("topology should remain available")
            .context_object_id("context-a"),
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
    let fixture = fixture(CdpContextOwnership::Owned);
    let installation = ChromiumContextLifecycleInstallation::install_without_retirement_plugins(
        fixture.cdp.clone(),
        Arc::clone(&fixture.topology),
        fixture.runtime.lifecycle_handle(),
        CleanupQueue::new(0),
        fixture.profile,
        &fixture.resources,
    )
    .expect("Context-lifecycle composition should install");
    let service = installation.service();
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
    assert!(fixture.cdp.method_names().is_empty());
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(2)), Ok(true));
    assert_eq!(fixture.runtime.lifecycle_transaction_active(), Ok(false));
    assert!(
        !fixture
            .topology
            .lock()
            .expect("topology should remain available")
            .transaction_active()
    );
}
