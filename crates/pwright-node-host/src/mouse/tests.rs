use super::*;
#[test]
fn selected_fixed_storage_and_delay_frame_fit_the_declared_allowance() {
    // Arrange
    let fixed = std::mem::size_of::<Installation>()
        + std::mem::size_of::<Shared>()
        + 4 * std::mem::size_of::<usize>()
        + 4096;
    let native =
        std::mem::size_of::<pwright_cdp::CdpSession>() + 2 * 256 + 2 * std::mem::size_of::<usize>();
    let invocation = std::mem::size_of::<Invocation<MouseClick>>()
        + std::mem::size_of::<CallLease>()
        + 4 * std::mem::size_of::<usize>();
    let delay = Arc::new(HostDelay).wait(Duration::from_secs(1));
    // Act
    let delay_bytes = std::mem::size_of_val(delay.as_ref().get_ref());
    // Assert
    assert!(fixed < STATE_BYTES as usize);
    assert!(native + invocation + delay_bytes < CALL_BYTES as usize / 2);
    assert!(delay_bytes < PROVIDER_FRAME_BYTES);
}

#[tokio::test]
async fn installation_drop_signals_workers_without_discarding_started_input() {
    // Arrange
    let resources = resources();
    let installation = Installation::new(&resources).unwrap();
    let tracker = installation.shared.tasks.clone();
    let client = Arc::new(FakeCdpClient::from_html(""));
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_pointer_gates([Some(gate.clone())]);
    let command = down(1);
    let invocation = installation
        .bind(client.clone(), input(), &command)
        .unwrap();
    let (_, page, _, params) = command.into_parts();
    let mut operation = invocation.call_for(page, params);
    assert!(poll(&mut operation).is_pending());
    // Act
    drop(operation);
    drop(invocation);
    drop(installation);
    tracker.close();
    let mut finished = Box::pin(tracker.wait());
    assert!(poll(&mut finished).is_pending());
    gate.notify_one();
    finished.await;
    // Assert
    assert_eq!(client.calls().len(), 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}
use pwright_core::{CallId, ResourceLimits};
use pwright_fake::FakeCdpClient;
use std::{
    future::Future,
    task::{Context, Poll, Waker},
};

fn resources() -> ProductionResourceLedger {
    let required = requirements().unwrap();
    let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
    for class in ResourceClass::ALL {
        let bound = required.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    ProductionResourceLedger::new(limits, required).unwrap()
}
fn input() -> CdpInputCoordinator {
    CdpInputCoordinator::with_policy(pwright_composition_chromium::CdpInputPolicy::Interleaved)
}
fn down(id: u64) -> Command<MouseDown> {
    Command::new(
        CallId::new(id),
        ObjectId::new(7),
        CallMetadata::default(),
        DownParams {
            button: MouseButton::Left,
            click_count: 1,
        },
    )
}
fn click() -> Command<MouseClick> {
    Command::new(
        CallId::new(1),
        ObjectId::new(7),
        CallMetadata::default(),
        ClickParams {
            x: MouseNumber::try_from(20.0).unwrap(),
            y: MouseNumber::try_from(30.0).unwrap(),
            button: MouseButton::Left,
            click_count: MouseClickCount::ONE,
            delay: None,
        },
    )
}
fn poll<T: Future + Unpin>(future: &mut T) -> Poll<T::Output> {
    Pin::new(future).poll(&mut Context::from_waker(Waker::noop()))
}
use std::pin::Pin;

#[tokio::test]
async fn unpolled_invocation_is_retired_without_native_input() {
    // Arrange
    let resources = resources();
    let installation = Installation::new(&resources).unwrap();
    let client = Arc::new(FakeCdpClient::from_html(""));
    let invocation = installation
        .bind(client.clone(), input(), &down(1))
        .unwrap();
    // Act
    drop(invocation);
    installation.shutdown().await.unwrap();
    // Assert
    assert!(client.calls().is_empty());
    assert_eq!(installation.metrics().unwrap().retirement_tasks, 0);
    assert_eq!(installation.metrics().unwrap().retained_invocations, 0);
    drop(installation);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn early_error_keeps_admission_until_independent_worker_retires_tail() {
    // Arrange
    let resources = resources();
    let installation = Installation::new(&resources).unwrap();
    let client = Arc::new(FakeCdpClient::from_html(""));
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_input_errors([Some("move failed".into()), None, None]);
    client.set_pointer_gates([None, Some(gate.clone()), None]);
    let command = click();
    let invocation = installation
        .bind(client.clone(), input(), &command)
        .unwrap();
    let (_, page, _, params) = command.into_parts();
    // Act
    let error = invocation.call_for(page, params).await.unwrap_err();
    drop(invocation);
    // Assert
    assert_eq!(error.kind(), pwright_core::FailureKind::Backend);
    assert_eq!(client.calls().len(), 3);
    assert_eq!(installation.metrics().unwrap().retained_invocations, 1);
    let mut close = Box::pin(installation.shutdown());
    assert!(poll(&mut close).is_pending());
    gate.notify_one();
    close.await.unwrap();
    assert_eq!(installation.metrics().unwrap().completed_receipts, 1);
    assert_eq!(installation.metrics().unwrap().retained_invocations, 0);
}

#[tokio::test]
async fn cleanup_future_and_receipt_worker_make_progress_independently() {
    // Arrange
    let resources = resources();
    let installation = Installation::new(&resources).unwrap();
    let client = Arc::new(FakeCdpClient::from_html(""));
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_pointer_gates([Some(gate.clone())]);
    let command = down(1);
    let invocation = installation
        .bind(client.clone(), input(), &command)
        .unwrap();
    let context = invocation.inner.context();
    let (_, page, _, params) = command.into_parts();
    let mut operation = invocation.call_for(page, params);
    assert!(poll(&mut operation).is_pending());
    drop(operation);
    // Act
    let mut cleanup = invocation.cancel(context, CancellationReason::DeadlineExceeded);
    assert!(poll(&mut cleanup).is_pending());
    gate.notify_one();
    cleanup.await.unwrap();
    drop(invocation);
    installation.shutdown().await.unwrap();
    // Assert
    assert_eq!(installation.metrics().unwrap().completed_receipts, 1);
    assert_eq!(installation.metrics().unwrap().cleanup_failures, 0);
    assert_eq!(client.calls().len(), 1);
}

#[tokio::test]
async fn driver_failure_is_retained_and_reported_at_shutdown() {
    // Arrange
    let resources = resources();
    let installation = Installation::new(&resources).unwrap();
    let client = Arc::new(FakeCdpClient::from_html(""));
    let gate = Arc::new(tokio::sync::Notify::new());
    client.set_input_errors([Some("native reply failed".into())]);
    client.set_pointer_gates([Some(gate.clone())]);
    let command = down(1);
    let invocation = installation.bind(client, input(), &command).unwrap();
    let (_, page, _, params) = command.into_parts();
    let mut operation = invocation.call_for(page, params);
    assert!(poll(&mut operation).is_pending());
    drop(operation);
    drop(invocation);
    // Act
    gate.notify_one();
    let error = installation.shutdown().await.unwrap_err();
    // Assert
    assert!(error.to_string().contains("native reply failed"));
    assert_eq!(installation.metrics().unwrap().cleanup_failures, 1);
    assert_eq!(installation.metrics().unwrap().retirement_tasks, 0);
    assert_eq!(installation.metrics().unwrap().retained_invocations, 0);
}

#[tokio::test]
async fn retained_limit_precedes_new_route_and_worker_allocation() {
    // Arrange
    let resources = resources();
    let installation = Installation::new(&resources).unwrap();
    let client = Arc::new(FakeCdpClient::from_html(""));
    let input = input();
    let invocations: Vec<_> = (0..RETAINED)
        .map(|n| {
            installation
                .bind(client.clone(), input.clone(), &down(n as u64 + 1))
                .unwrap()
        })
        .collect();
    // Act
    let snapshots = AtomicUsize::new(0);
    let error = installation
        .bind_with(&down(100), || {
            snapshots.fetch_add(1, Ordering::Relaxed);
            Ok((client.clone(), input))
        })
        .err()
        .unwrap();
    // Assert
    assert!(matches!(error, HostError::ResourceExhausted(_)));
    assert_eq!(snapshots.load(Ordering::Relaxed), 0);
    assert_eq!(installation.metrics().unwrap().retirement_tasks, RETAINED);
    assert!(client.calls().is_empty());
    drop(invocations);
    installation.shutdown().await.unwrap();
    assert_eq!(installation.metrics().unwrap().retained_invocations, 0);
}

#[tokio::test]
async fn shutdown_rejects_new_bindings_before_native_input() {
    // Arrange
    let resources = resources();
    let installation = Installation::new(&resources).unwrap();
    let client = Arc::new(FakeCdpClient::from_html(""));
    installation.shutdown().await.unwrap();
    // Act
    let error = installation
        .bind(client.clone(), input(), &down(1))
        .err()
        .unwrap();
    // Assert
    assert!(matches!(error, HostError::TargetClosed));
    assert!(client.calls().is_empty());
    assert_eq!(installation.metrics().unwrap().retirement_tasks, 0);
}

#[tokio::test]
async fn rejected_binding_rolls_back_host_storage_and_count() {
    // Arrange
    let resources = resources();
    let installation = Installation::new(&resources).unwrap();
    let baseline = resources.metrics().unwrap().active_leases;
    let client = Arc::new(FakeCdpClient::from_html(""));
    let exclusive =
        CdpInputCoordinator::with_policy(pwright_composition_chromium::CdpInputPolicy::Exclusive);
    // Act
    let result = installation.bind(client, exclusive, &down(1));
    // Assert
    assert!(result.is_err());
    assert_eq!(installation.metrics().unwrap().retained_invocations, 0);
    assert_eq!(installation.metrics().unwrap().retirement_tasks, 0);
    assert_eq!(resources.metrics().unwrap().active_leases, baseline);
}
