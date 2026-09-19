//! Production registration/FFI/Owners contracts, not an assembled HostSession,
//! Node callback, browser simulation or native object-group cleanup proof.
use super::*;
use crate::{
    ffi_budget::{FfiBudget, FfiLane, FfiLimits, FfiMetrics},
    handles::file_input::RetainedUploads,
};
use pwright_browser_model::ELEMENT_HANDLE_OBJECT_KIND;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

const FFI_BYTES: u64 = 256;

struct RetainedFixture {
    service: RetainedUploads,
    resources: ProductionResourceLedger,
}

impl RetainedFixture {
    fn new() -> Self {
        // Separate admitted production registry storage from the Owners ledger.
        let requirements = RetainedUploads::requirements().unwrap();
        let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let required = requirements.class(class);
            limits = limits.with_class(class, required.global, required.per_scope);
        }
        let resources = ProductionResourceLedger::new(limits, requirements).unwrap();
        let service = RetainedUploads::new(&resources).unwrap();
        Self { service, resources }
    }

    async fn finish(self) {
        self.service.shutdown().await.unwrap();
        drop(self.service);
        let metrics = self.resources.metrics().unwrap();
        assert_eq!(metrics.active_leases, 0);
        for class in ResourceClass::ALL {
            assert_eq!(metrics.class(class).current, 0);
        }
    }
}

fn ffi_budget() -> FfiBudget {
    FfiBudget::new(FfiLimits {
        connects: 1,
        commands: 2,
        closes: 1,
        watches: 1,
        reserved_bytes: 16 * 1024,
        future_bytes: [4096; 4],
    })
    .unwrap()
}

fn element(owners: &Arc<Owners>, cleanup: &TokioCleanupExecutor, id: u64) -> PendingHandle {
    let (mut initializer, ticket, allowance) = ticket(
        id,
        owners.admit().unwrap(),
        cleanup,
        Arc::new(AtomicUsize::new(0)),
    );
    initializer.object_kind = ELEMENT_HANDLE_OBJECT_KIND.tag;
    owners.retain(initializer, ticket, allowance).unwrap()
}

async fn finish_owners(
    owners: Arc<Owners>,
    cleanup: TokioCleanupExecutor,
    resources: ProductionResourceLedger,
) {
    owners.close();
    owners.wait().await;
    let report = cleanup.shutdown().await;
    assert_eq!(report.metrics.failed, 0);
    assert_eq!(report.metrics.timed_out, 0);
    assert_eq!(report.metrics.reserved, 0);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 0);
    assert_eq!(owners.capacity.owners.load(Ordering::Acquire), 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::RemoteHandle)
            .peak,
        0
    );
    drop(owners);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

fn poll_now<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

#[tokio::test]
async fn equal_portable_ids_from_foreign_owners_reject_without_revoking_their_preparation() {
    // Arrange
    let (local, local_resources) = owners();
    let (foreign, foreign_resources) = owners();
    let local_cleanup = TokioCleanupExecutor::new(1);
    let foreign_cleanup = TokioCleanupExecutor::new(1);
    let local_handle = element(&local, &local_cleanup, 2);
    let foreign_handle = element(&foreign, &foreign_cleanup, 2);
    assert_eq!(
        local_handle.initializer().object_id,
        foreign_handle.initializer().object_id
    );
    let fixture = RetainedFixture::new();
    let budget = ffi_budget();
    let preparation = budget.prepare::<u8>(FfiLane::Command, FFI_BYTES).unwrap();
    let ffi_before = budget.metrics().unwrap();
    let registry_before = fixture.resources.metrics().unwrap();
    let owners_before = local_resources.metrics().unwrap();

    // Act
    let rejected = fixture.service.register_invocation(
        &local,
        &foreign_handle.source_identity(),
        preparation.revoker(),
    );

    // Assert: receiving a weak cancellation receipt does not grant ownership.
    assert!(matches!(rejected, Err(HostError::InvalidArgument(_))));
    assert_eq!(budget.metrics().unwrap(), ffi_before);
    assert_eq!(fixture.resources.metrics().unwrap(), registry_before);
    assert_eq!(local_resources.metrics().unwrap(), owners_before);
    let mut execution = preparation.start(&budget).unwrap();
    let future = async { Ok(7) };
    execution
        .reserve_future(std::mem::size_of_val(&future))
        .unwrap();
    let ticket = execution.run(future).await;
    assert_eq!(ticket.deliver(Result::unwrap).unwrap(), 7);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    local_handle.dispose().await.unwrap();
    foreign_handle.dispose().await.unwrap();
    drop((local_handle, foreign_handle));
    fixture.finish().await;
    finish_owners(local, local_cleanup, local_resources).await;
    finish_owners(foreign, foreign_cleanup, foreign_resources).await;
}

#[tokio::test]
async fn same_owners_wrong_source_cannot_reuse_a_registration_or_acquire_strong_ownership() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(2);
    let first = element(&owners, &cleanup, 2);
    let other = element(&owners, &cleanup, 3);
    let first_source = first.source_identity();
    let fixture = RetainedFixture::new();
    let budget = ffi_budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, FFI_BYTES).unwrap();
    let strong_before = Arc::strong_count(&owners);
    let owner_metrics = resources.metrics().unwrap();
    let work_before = owners.work.len();
    let registration = fixture
        .service
        .register_invocation(&owners, &first_source, preparation.revoker())
        .unwrap();

    // Act
    let wrong = fixture
        .service
        .check_invocation(&owners, &other.source_identity(), &registration);
    let correct = fixture
        .service
        .check_invocation(&owners, &first_source, &registration);

    // Assert
    assert!(matches!(wrong, Err(HostError::InvalidArgument(_))));
    assert!(correct.is_ok());
    assert_eq!(Arc::strong_count(&owners), strong_before);
    assert_eq!(resources.metrics().unwrap(), owner_metrics);
    assert_eq!(owners.work.len(), work_before);
    assert_eq!(budget.metrics().unwrap().running, 0);
    drop((registration, preparation));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    first.dispose().await.unwrap();
    other.dispose().await.unwrap();
    drop((first, other));
    fixture.finish().await;
    finish_owners(owners, cleanup, resources).await;
}

#[tokio::test]
async fn another_retained_registry_cannot_accept_a_same_source_receipt() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let source = element(&owners, &cleanup, 2);
    let identity = source.source_identity();
    let origin = RetainedFixture::new();
    let foreign = RetainedFixture::new();
    let budget = ffi_budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, FFI_BYTES).unwrap();
    let registration = origin
        .service
        .register_invocation(&owners, &identity, preparation.revoker())
        .unwrap();
    let before = budget.metrics().unwrap();

    // Act
    let result = foreign
        .service
        .check_invocation(&owners, &identity, &registration);

    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert!(
        origin
            .service
            .check_invocation(&owners, &identity, &registration)
            .is_ok()
    );
    assert_eq!(budget.metrics().unwrap(), before);
    drop((registration, preparation));
    source.dispose().await.unwrap();
    drop(source);
    origin.finish().await;
    foreign.finish().await;
    finish_owners(owners, cleanup, resources).await;
}

#[tokio::test]
async fn disposed_source_metadata_can_register_without_reauthorizing_native_use() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let source = element(&owners, &cleanup, 2);
    let identity = source.source_identity();
    source.dispose().await.unwrap();
    assert!(owners.check_arguments([&source]).is_err());
    drop(source);
    let fixture = RetainedFixture::new();
    let budget = ffi_budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, FFI_BYTES).unwrap();
    let before = resources.metrics().unwrap();
    let strong_before = Arc::strong_count(&owners);

    // Act
    let registration = fixture
        .service
        .register_invocation(&owners, &identity, preparation.revoker())
        .unwrap();
    let checked = fixture
        .service
        .check_invocation(&owners, &identity, &registration);

    // Assert: cancellation bookkeeping is deliberately not another dispatcher check.
    assert!(checked.is_ok());
    assert_eq!(Arc::strong_count(&owners), strong_before);
    assert_eq!(resources.metrics().unwrap(), before);
    assert_eq!(owners.work.len(), 0);
    assert_eq!(budget.metrics().unwrap().running, 0);
    drop((registration, preparation));
    fixture.finish().await;
    finish_owners(owners, cleanup, resources).await;
}

struct OwnedCall {
    _tracked: TrackedCallAdmission,
    drops: Arc<AtomicUsize>,
}

impl Drop for OwnedCall {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::AcqRel);
    }
}

#[tokio::test]
async fn session_revocation_drops_parked_actual_owner_work_without_a_node_callback() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let source = element(&owners, &cleanup, 2);
    let identity = source.source_identity();
    let drops = Arc::new(AtomicUsize::new(0));
    let call = OwnedCall {
        _tracked: owners.admit_tracked_call(&source).unwrap(),
        drops: drops.clone(),
    };
    source.dispose().await.unwrap();
    drop(source);
    let fixture = RetainedFixture::new();
    let budget = ffi_budget();
    let preparation = budget
        .prepare::<OwnedCall>(FfiLane::Command, FFI_BYTES)
        .unwrap();
    let registration = fixture
        .service
        .register_invocation(&owners, &identity, preparation.revoker())
        .unwrap();
    let mut execution = preparation.start(&budget).unwrap();
    let future = async move { Ok(call) };
    execution
        .reserve_future(std::mem::size_of_val(&future))
        .unwrap();
    let parked = execution.run(future).await.park(&budget).unwrap();
    assert_eq!(budget.metrics().unwrap().running, 0);
    assert_eq!(owners.work.len(), 1);
    owners.close();
    let mut waiting = Box::pin(owners.wait());
    assert!(poll_now(waiting.as_mut()).is_pending());

    // Act: real session-registry cancellation, while the parked receipt is held.
    fixture.service.seal_invocations().revoke();

    // Assert: actual Owners work/call admission drains, not simulated browser cleanup.
    assert_eq!(drops.load(Ordering::Acquire), 1);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 0);
    assert!(poll_now(waiting.as_mut()).is_ready());
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    assert!(matches!(
        parked.inspect_parked(&budget, |_| ()),
        Err(HostError::TargetClosed)
    ));
    drop((waiting, parked, registration));
    fixture.finish().await;
    finish_owners(owners, cleanup, resources).await;
}

struct WaitingNative {
    _call: OwnedCall,
    budget: FfiBudget,
    observations: Arc<Mutex<Vec<FfiMetrics>>>,
}

impl Future for WaitingNative {
    type Output = Result<OwnedCall, HostError>;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Pending
    }
}

impl Drop for WaitingNative {
    fn drop(&mut self) {
        self.observations
            .lock()
            .unwrap()
            .push(self.budget.metrics().unwrap());
    }
}

#[tokio::test]
async fn running_session_revocation_preserves_future_and_owner_work_until_next_poll() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let source = element(&owners, &cleanup, 2);
    let identity = source.source_identity();
    let drops = Arc::new(AtomicUsize::new(0));
    let call = OwnedCall {
        _tracked: owners.admit_tracked_call(&source).unwrap(),
        drops: drops.clone(),
    };
    source.dispose().await.unwrap();
    drop(source);
    let fixture = RetainedFixture::new();
    let budget = ffi_budget();
    let preparation = budget
        .prepare::<OwnedCall>(FfiLane::Command, FFI_BYTES)
        .unwrap();
    let registration = fixture
        .service
        .register_invocation(&owners, &identity, preparation.revoker())
        .unwrap();
    let observations = Arc::new(Mutex::new(Vec::new()));
    let future = WaitingNative {
        _call: call,
        budget: budget.clone(),
        observations: observations.clone(),
    };
    let mut execution = preparation.start(&budget).unwrap();
    execution
        .reserve_future(std::mem::size_of_val(&future))
        .unwrap();
    let mut running = Box::pin(execution.run(future));
    assert!(poll_now(running.as_mut()).is_pending());
    let before = budget.metrics().unwrap();
    owners.close();
    let mut waiting = Box::pin(owners.wait());

    // Act
    fixture.service.seal_invocations().revoke();

    // Assert: revocation signals, but cannot destroy a future on another stack.
    assert_eq!(drops.load(Ordering::Acquire), 0);
    assert!(observations.lock().unwrap().is_empty());
    assert_eq!(budget.metrics().unwrap(), before);
    assert_eq!(owners.work.len(), 1);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 1);
    assert!(poll_now(waiting.as_mut()).is_pending());
    let Poll::Ready(ticket) = poll_now(running.as_mut()) else {
        panic!("revoked native phase must settle when polled");
    };
    assert_eq!(*observations.lock().unwrap(), [before]);
    assert_eq!(drops.load(Ordering::Acquire), 1);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    assert!(poll_now(waiting.as_mut()).is_ready());
    assert!(matches!(
        ticket.deliver(|_| ()),
        Err(HostError::TargetClosed)
    ));
    drop((waiting, running, registration));
    fixture.finish().await;
    finish_owners(owners, cleanup, resources).await;
}

#[tokio::test]
async fn previously_successful_receipt_rejects_after_registry_seal() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let source = element(&owners, &cleanup, 2);
    let identity = source.source_identity();
    let fixture = RetainedFixture::new();
    let budget = ffi_budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, FFI_BYTES).unwrap();
    let registration = fixture
        .service
        .register_invocation(&owners, &identity, preparation.revoker())
        .unwrap();
    assert!(
        fixture
            .service
            .check_invocation(&owners, &identity, &registration)
            .is_ok()
    );

    // Act
    fixture.service.seal_invocations().revoke();
    let checked = fixture
        .service
        .check_invocation(&owners, &identity, &registration);

    // Assert
    assert!(matches!(checked, Err(HostError::TargetClosed)));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    drop((registration, preparation));
    source.dispose().await.unwrap();
    drop(source);
    fixture.finish().await;
    finish_owners(owners, cleanup, resources).await;
}

#[tokio::test]
async fn closed_owners_reject_receipt_checks_without_revoking_the_registered_entry() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let source = element(&owners, &cleanup, 2);
    let identity = source.source_identity();
    let fixture = RetainedFixture::new();
    let budget = ffi_budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, FFI_BYTES).unwrap();
    let registration = fixture
        .service
        .register_invocation(&owners, &identity, preparation.revoker())
        .unwrap();
    source.dispose().await.unwrap();
    drop(source);
    owners.close();
    let before = budget.metrics().unwrap();

    // Act
    let result = fixture
        .service
        .check_invocation(&owners, &identity, &registration);

    // Assert: a failed point check is not authority to revoke an FFI entry.
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(budget.metrics().unwrap(), before);
    fixture.service.seal_invocations().revoke();
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    drop((registration, preparation));
    fixture.finish().await;
    finish_owners(owners, cleanup, resources).await;
}
