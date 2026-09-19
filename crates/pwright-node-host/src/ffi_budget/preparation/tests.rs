use super::*;
use crate::ffi_budget::{FfiLimits, FfiMetrics, Reclaim};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

fn budget() -> FfiBudget {
    FfiBudget::new(FfiLimits {
        connects: 1,
        commands: 2,
        closes: 1,
        watches: 1,
        reserved_bytes: 32,
        future_bytes: [16; 4],
    })
    .unwrap()
}

fn poll_now<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

#[test]
fn preparation_reserves_slot_and_bytes_without_running_or_queued_work() {
    // Arrange
    let budget = budget();

    // Act
    let preparation = budget.prepare::<()>(FfiLane::Command, 16).unwrap();

    // Assert
    assert_eq!(
        budget.metrics().unwrap(),
        FfiMetrics {
            pending: 1,
            reserved_bytes: 16,
            commands: 1,
            ..FfiMetrics::default()
        }
    );
    drop(preparation);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn preparation_and_execution_share_the_same_finite_command_capacity() {
    // Arrange
    let budget = budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, 8).unwrap();
    let execution = budget.admit::<()>(FfiLane::Command, 8).unwrap();

    // Act
    let extra_preparation = budget.prepare::<()>(FfiLane::Command, 1);
    let extra_execution = budget.admit::<()>(FfiLane::Command, 1);

    // Assert
    assert!(matches!(
        extra_preparation,
        Err(HostError::ResourceExhausted(_))
    ));
    assert!(matches!(
        extra_execution,
        Err(HostError::ResourceExhausted(_))
    ));
    assert_eq!(budget.metrics().unwrap().commands, 2);
    assert_eq!(budget.metrics().unwrap().reserved_bytes, 16);
    assert_eq!(budget.metrics().unwrap().running, 1);
    drop((preparation, execution));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn byte_rejection_leaves_no_preparation_slot_or_task() {
    // Arrange
    let budget = budget();

    // Act
    let result = budget.prepare::<()>(FfiLane::Command, 33);

    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn dropped_preparation_returns_capacity_for_existing_eager_admission() {
    // Arrange
    let budget = budget();
    let first = budget.prepare::<()>(FfiLane::Command, 16).unwrap();
    let second = budget.prepare::<()>(FfiLane::Command, 16).unwrap();

    // Act
    drop(first);
    let replacement = budget.admit::<()>(FfiLane::Command, 16);

    // Assert
    assert!(replacement.is_ok());
    assert_eq!(budget.metrics().unwrap().commands, 2);
    assert_eq!(budget.metrics().unwrap().reserved_bytes, 32);
    drop((replacement, second));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn saturated_preparations_do_not_block_the_independent_close_lane() {
    // Arrange
    let budget = budget();
    let first = budget.prepare::<()>(FfiLane::Command, 1).unwrap();
    let second = budget.prepare::<()>(FfiLane::Command, 1).unwrap();

    // Act
    let close = budget.admit::<()>(FfiLane::Close, 1);

    // Assert
    assert!(close.is_ok());
    drop((first, second, close));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[tokio::test]
async fn held_preparation_does_not_delay_shutdown_and_cannot_start_afterward() {
    // Arrange
    let budget = budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, 16).unwrap();
    let mut shutdown = Box::pin(budget.shutdown());

    // Act: one poll must finish; no timer or foreign callback can release this
    // still-held preparation on behalf of shutdown.
    let result = poll_now(shutdown.as_mut());

    // Assert
    assert!(matches!(result, Poll::Ready(Ok(()))));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    assert!(matches!(
        preparation.start(&budget),
        Err(HostError::TargetClosed)
    ));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[tokio::test]
async fn shutdown_reclaims_preparation_even_while_its_empty_receipt_remains_held() {
    // Arrange
    let budget = budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, 16).unwrap();

    // Act
    budget.shutdown().await.unwrap();

    // Assert
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    assert!(!preparation.ticket.entry.is_active());
    drop(preparation);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[test]
fn equal_limits_do_not_authorize_promotion_into_a_foreign_budget() {
    // Arrange
    let origin = budget();
    let foreign = budget();
    let preparation = origin.prepare::<()>(FfiLane::Command, 16).unwrap();

    // Act
    let result = preparation.start(&foreign);

    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert_eq!(origin.metrics().unwrap(), FfiMetrics::default());
    assert_eq!(foreign.metrics().unwrap(), FfiMetrics::default());
}

#[tokio::test]
async fn revoked_receipt_does_not_keep_the_budget_alive() {
    // Arrange
    let budget = budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, 16).unwrap();
    budget.shutdown().await.unwrap();

    // Act
    drop(budget);

    // Assert: the held receipt retains only an empty cell and weak identity,
    // not the registry, its ledger, or a strong environment owner.
    assert!(preparation.origin.upgrade().is_none());
    assert!(!preparation.ticket.entry.is_active());
}

#[test]
fn explicitly_revoked_permit_cannot_be_promoted_while_budget_is_open() {
    // Arrange
    let budget = budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, 16).unwrap();
    preparation.ticket.entry.reclaim();

    // Act
    let result = preparation.start(&budget);

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[tokio::test]
async fn promotion_preserves_one_permit_and_tracks_until_execution_settles() {
    // Arrange
    let budget = budget();
    let preparation = budget.prepare::<u8>(FfiLane::Command, 16).unwrap();
    let alias = budget.clone();

    // Act
    let mut execution = preparation.start(&alias).unwrap();

    // Assert: an exact budget clone is valid; promotion cannot reserve twice.
    assert_eq!(budget.metrics().unwrap().pending, 1);
    assert_eq!(budget.metrics().unwrap().commands, 1);
    assert_eq!(budget.metrics().unwrap().reserved_bytes, 16);
    assert_eq!(budget.metrics().unwrap().running, 1);
    execution.reserve_future(8).unwrap();
    let ticket = execution.run(async { Ok(7) }).await;
    assert_eq!(budget.metrics().unwrap().running, 0);
    assert_eq!(budget.metrics().unwrap().queued_commands, 1);
    assert_eq!(budget.metrics().unwrap().reserved_bytes, 24);
    assert_eq!(ticket.deliver(|value| value.unwrap()).unwrap(), 7);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[tokio::test]
async fn promotion_before_shutdown_is_tracked_before_closed_empty_can_be_observed() {
    // Arrange
    let budget = budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, 16).unwrap();
    let execution = preparation.start(&budget).unwrap();
    let mut shutdown = Box::pin(budget.shutdown());

    // Act
    let before_drop = poll_now(shutdown.as_mut());
    drop(execution);
    let after_drop = poll_now(shutdown.as_mut());

    // Assert: eager execution semantics remain unchanged after promotion.
    assert!(before_drop.is_pending());
    assert!(matches!(after_drop, Poll::Ready(Ok(()))));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[tokio::test]
async fn shutdown_before_promotion_rejects_start_without_creating_late_work() {
    // Arrange
    let budget = budget();
    let preparation = budget.prepare::<()>(FfiLane::Command, 16).unwrap();
    let existing = budget.admit::<()>(FfiLane::Close, 1).unwrap();
    let mut shutdown = Box::pin(budget.shutdown());
    assert!(poll_now(shutdown.as_mut()).is_pending());

    // Act
    let result = preparation.start(&budget);

    // Assert: shutdown has closed the gate but is still joining earlier work.
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(budget.metrics().unwrap().running, 1);
    assert_eq!(budget.metrics().unwrap().commands, 0);
    drop(existing);
    assert!(matches!(poll_now(shutdown.as_mut()), Poll::Ready(Ok(()))));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[tokio::test]
async fn closed_budget_rejects_preparation_before_any_new_reservation() {
    // Arrange
    let budget = budget();
    budget.shutdown().await.unwrap();

    // Act
    let result = budget.prepare::<()>(FfiLane::Command, 16);

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}
