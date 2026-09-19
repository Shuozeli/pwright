use super::*;
use pwright_browser_model::JS_HANDLE_OBJECT_KIND;
use pwright_core::{ObjectId, ResourceLimits};
use pwright_runtime::ReservedCleanupExecutor;
use pwright_runtime_tokio::TokioCleanupExecutor;

mod argument_tests;
mod batch_tests;
mod call_retention_tests;
mod close_wake_tests;
#[cfg(feature = "retained-file-input")]
mod invocation_registration_tests;
#[cfg(feature = "retained-file-input")]
mod source_identity_tests;
#[cfg(feature = "retained-file-input")]
mod tracked_call_tests;
mod tracked_read_tests;

fn owners() -> (Arc<Owners>, ProductionResourceLedger) {
    let needs = requirements();
    let mut limits = ResourceLimits::deny_all(needs.scopes, needs.leases);
    for class in ResourceClass::ALL {
        let bound = needs.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    let resources = ProductionResourceLedger::new(limits, needs).unwrap();
    (Owners::new(&resources).unwrap(), resources)
}

fn ticket(
    id: u64,
    admission: Admission,
    cleanup: &TokioCleanupExecutor,
    released: Arc<AtomicUsize>,
) -> (HandleInitializer, Ticket, Arc<Allowance>) {
    let Admission { owner, call, work } = admission;
    drop(call);
    let retained = owner.clone();
    let job = Box::pin(async move {
        let _owner = retained;
        let _work = work;
        released.fetch_add(1, Ordering::AcqRel);
        Ok(())
    });
    (
        HandleInitializer::new(
            ObjectId::new(id),
            ObjectId::new(1),
            JS_HANDLE_OBJECT_KIND.tag,
            "object",
        ),
        Ticket {
            reservation: cleanup.try_reserve().unwrap(),
            job,
        },
        owner,
    )
}

#[tokio::test]
async fn close_drains_cleanup_while_returned_descriptor_is_still_held() {
    // Arrange
    let (owners, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let released = Arc::new(AtomicUsize::new(0));
    let (initializer, ticket, admission) =
        ticket(2, owners.admit().unwrap(), &cleanup, released.clone());
    let pending = owners.retain(initializer, ticket, admission).unwrap();

    // Act
    owners.close();
    owners.wait().await;
    let report = cleanup.shutdown().await;

    // Assert
    assert_eq!(released.load(Ordering::Acquire), 1);
    assert_eq!(report.metrics.reserved, 0);
    assert_eq!(report.metrics.failed, 0);
    assert_eq!(pending.initializer().object_id, ObjectId::new(2));
    pending.dispose().await.unwrap();
    assert!(matches!(owners.admit(), Err(HostError::TargetClosed)));
    // Descriptor storage is still charged, unlike native cleanup work.
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        STATE_BYTES + OWNER_BYTES
    );
    drop(pending);
    owners.close();
    assert_eq!(released.load(Ordering::Acquire), 1);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        STATE_BYTES
    );
}

#[tokio::test]
async fn dropped_ready_result_submits_exactly_one_cleanup() {
    // Arrange
    let (owners, _) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let released = Arc::new(AtomicUsize::new(0));
    let (initializer, ticket, admission) =
        ticket(2, owners.admit().unwrap(), &cleanup, released.clone());
    let pending = owners.retain(initializer, ticket, admission).unwrap();

    // Act
    drop(pending);
    let report = cleanup.shutdown().await;

    // Assert
    owners.close();
    owners.wait().await;
    assert_eq!(released.load(Ordering::Acquire), 1);
    assert_eq!(report.metrics.completed, 1);
    assert_eq!(report.metrics.reserved, 0);
}

#[tokio::test]
async fn result_finishing_after_owner_close_is_rejected_but_cleaned() {
    // Arrange
    let (owners, _) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let released = Arc::new(AtomicUsize::new(0));
    let (initializer, ticket, admission) =
        ticket(2, owners.admit().unwrap(), &cleanup, released.clone());
    owners.close();

    // Act
    let result = owners.retain(initializer, ticket, admission);
    owners.wait().await;
    let report = cleanup.shutdown().await;

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(released.load(Ordering::Acquire), 1);
    assert_eq!(report.metrics.reserved, 0);
}

#[tokio::test]
async fn result_capacity_is_retained_through_cleanup_and_descriptor_lifetime() {
    // Arrange
    let (owners, _) = owners();
    let cleanup = TokioCleanupExecutor::new(MAX_HANDLES);
    let released = Arc::new(AtomicUsize::new(0));
    let mut pending = Vec::new();
    for id in 2..2 + MAX_HANDLES as u64 {
        let (initializer, ticket, admission) =
            ticket(id, owners.admit().unwrap(), &cleanup, released.clone());
        pending.push(owners.retain(initializer, ticket, admission).unwrap());
    }

    // Act
    let overflow = owners.admit();

    // Assert
    assert!(matches!(overflow, Err(HostError::ResourceExhausted(_))));
    owners.close();
    owners.wait().await;
    let report = cleanup.shutdown().await;
    assert_eq!(released.load(Ordering::Acquire), MAX_HANDLES);
    assert_eq!(report.metrics.reserved, 0);
    assert_eq!(pending.len(), MAX_HANDLES);
}

#[test]
fn in_flight_capacity_is_independent_of_ready_result_capacity() {
    // Arrange
    let (owners, resources) = owners();
    let admissions = (0..MAX_CALLS)
        .map(|_| owners.admit().unwrap())
        .collect::<Vec<_>>();

    // Act
    let overflow = owners.admit();

    // Assert
    assert!(matches!(overflow, Err(HostError::ResourceExhausted(_))));
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        STATE_BYTES + (CALL_BYTES + OWNER_BYTES) * MAX_CALLS as u64
    );
    drop(admissions);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        STATE_BYTES
    );
}

#[tokio::test]
async fn concurrent_dispose_waits_for_one_actual_cleanup_completion() {
    // Arrange
    let (owners, _) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let released = Arc::new(AtomicUsize::new(0));
    let (initializer, mut ticket, admission) =
        ticket(2, owners.admit().unwrap(), &cleanup, released.clone());
    let (finish, blocked) = tokio::sync::oneshot::channel();
    let job = ticket.job;
    ticket.job = Box::pin(async move {
        blocked.await.unwrap();
        job.await
    });
    let pending = owners.retain(initializer, ticket, admission).unwrap();
    let first = pending.dispose();
    let second = pending.dispose();
    tokio::pin!(first, second);

    // Act
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    assert!(std::future::Future::poll(first.as_mut(), &mut context).is_pending());
    assert!(std::future::Future::poll(second.as_mut(), &mut context).is_pending());
    finish.send(()).unwrap();
    let (first, second) = tokio::join!(first, second);

    // Assert
    first.unwrap();
    second.unwrap();
    pending.dispose().await.unwrap();
    assert_eq!(released.load(Ordering::Acquire), 1);
    let report = cleanup.shutdown().await;
    assert_eq!(report.metrics.completed, 1);
    assert_eq!(report.metrics.failed, 0);
}

#[tokio::test]
async fn cancelled_dispose_waiter_does_not_abandon_the_release_or_its_result() {
    // Arrange
    let (owners, _) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let released = Arc::new(AtomicUsize::new(0));
    let (initializer, mut ticket, admission) =
        ticket(2, owners.admit().unwrap(), &cleanup, released.clone());
    let (finish, blocked) = tokio::sync::oneshot::channel();
    let job = ticket.job;
    ticket.job = Box::pin(async move {
        blocked.await.unwrap();
        job.await
    });
    let pending = owners.retain(initializer, ticket, admission).unwrap();

    // Act
    {
        let wait = pending.dispose();
        tokio::pin!(wait);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(std::future::Future::poll(wait.as_mut(), &mut context).is_pending());
    }
    owners.close();
    finish.send(()).unwrap();
    pending.dispose().await.unwrap();

    // Assert
    owners.wait().await;
    let report = cleanup.shutdown().await;
    assert_eq!(released.load(Ordering::Acquire), 1);
    assert_eq!(report.metrics.completed, 1);
}

#[tokio::test]
async fn dispose_and_shutdown_both_report_failed_cleanup() {
    // Arrange
    let (owners, _) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let released = Arc::new(AtomicUsize::new(0));
    let (initializer, mut ticket, admission) =
        ticket(2, owners.admit().unwrap(), &cleanup, released.clone());
    let job = ticket.job;
    ticket.job = Box::pin(async move {
        job.await?;
        Err(CleanupFailure::new("native release refused"))
    });
    let pending = owners.retain(initializer, ticket, admission).unwrap();

    // Act
    let result = pending.dispose().await;

    // Assert
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("native release refused")
    );
    assert!(
        pending
            .dispose()
            .await
            .unwrap_err()
            .to_string()
            .contains("native release refused")
    );
    let report = cleanup.shutdown().await;
    assert_eq!(report.metrics.failed, 1);
    assert_eq!(released.load(Ordering::Acquire), 1);
}

#[tokio::test]
async fn reads_reject_foreign_and_disposing_handles_before_admission() {
    // Arrange
    let (owner, resources) = owners();
    let (foreign, _) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let (initializer, ticket, admission) = ticket(
        2,
        owner.admit().unwrap(),
        &cleanup,
        Arc::new(AtomicUsize::new(0)),
    );
    let pending = owner.retain(initializer, ticket, admission).unwrap();
    let baseline = resources.metrics().unwrap();

    // Act
    let foreign_result = foreign.admit_read(&pending);
    owner.abandon(pending.initializer.object_id);
    let disposed_result = owner.admit_read(&pending);

    // Assert
    assert!(
        matches!(foreign_result, Err(HostError::InvalidArgument(message)) if message.contains("another session"))
    );
    assert!(
        matches!(disposed_result, Err(HostError::InvalidArgument(message)) if message.contains("disposed"))
    );
    assert_eq!(resources.metrics().unwrap(), baseline);
    pending.dispose().await.unwrap();
    assert_eq!(cleanup.shutdown().await.metrics.completed, 1);
}

#[tokio::test(start_paused = true)]
async fn dispose_reports_missing_acknowledgement_when_cleanup_deadline_drops_job() {
    // Arrange
    let (owner, _) = owners();
    let cleanup =
        TokioCleanupExecutor::new_with_job_timeout(1, std::time::Duration::from_secs(5)).unwrap();
    let (initializer, mut ticket, admission) = ticket(
        2,
        owner.admit().unwrap(),
        &cleanup,
        Arc::new(AtomicUsize::new(0)),
    );
    let job = ticket.job;
    ticket.job = Box::pin(async move {
        std::future::pending::<()>().await;
        job.await
    });
    let pending = owner.retain(initializer, ticket, admission).unwrap();

    // Act: paused Tokio time advances to the admitted executor deadline.
    let result = pending.dispose().await;

    // Assert
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("without a completion acknowledgement")
    );
    assert!(pending.dispose().await.is_err());
    let report = cleanup.shutdown().await;
    assert_eq!(report.metrics.timed_out, 1);
    assert_eq!(report.metrics.failed, 0);
    assert_eq!(report.metrics.reserved, 0);
    owner.close();
    owner.wait().await;
}

#[tokio::test]
async fn read_calls_share_the_bounded_creation_call_allowance() {
    // Arrange
    let (owner, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let (initializer, ticket, admission) = ticket(
        2,
        owner.admit().unwrap(),
        &cleanup,
        Arc::new(AtomicUsize::new(0)),
    );
    let pending = owner.retain(initializer, ticket, admission).unwrap();
    let calls = (0..MAX_CALLS)
        .map(|_| owner.admit_read(&pending).unwrap())
        .collect::<Vec<_>>();

    // Act
    let read_overflow = owner.admit_read(&pending);
    let creation_overflow = owner.admit();

    // Assert
    assert!(matches!(
        read_overflow,
        Err(HostError::ResourceExhausted(_))
    ));
    assert!(matches!(
        creation_overflow,
        Err(HostError::ResourceExhausted(_))
    ));
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        STATE_BYTES + OWNER_BYTES + CALL_BYTES * MAX_CALLS as u64
    );
    drop(calls);
    pending.dispose().await.unwrap();
    assert_eq!(cleanup.shutdown().await.metrics.completed, 1);
}

#[tokio::test]
async fn completion_receipt_does_not_retain_descriptor_or_owner_admission() {
    // Arrange
    let (owner, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let (initializer, ticket, admission) = ticket(
        2,
        owner.admit().unwrap(),
        &cleanup,
        Arc::new(AtomicUsize::new(0)),
    );
    let pending = owner.retain(initializer, ticket, admission).unwrap();

    // Act
    let release = pending.request_dispose();
    drop(pending);
    release.wait().await.unwrap();
    owner.close();
    owner.wait().await;
    drop(owner);

    // Assert: receipt is still usable with all service admission returned.
    release.wait().await.unwrap();
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        0
    );
    assert_eq!(cleanup.shutdown().await.metrics.completed, 1);
}
