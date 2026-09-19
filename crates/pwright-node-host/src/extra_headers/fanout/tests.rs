use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::oneshot;

#[tokio::test]
async fn sequential_group_waits_for_every_current_member_before_starting() {
    // Arrange
    let (first, first_done) = oneshot::channel();
    let (second, second_done) = oneshot::channel();
    let next_started = Arc::new(AtomicUsize::new(0));
    let next = next_started.clone();
    let mut calls = Fanout::new();
    calls.push(async { first_done.await.unwrap() }).unwrap();
    calls.push(async { second_done.await.unwrap() }).unwrap();
    calls.end_group().unwrap();
    calls
        .push(async move {
            next.fetch_add(1, Ordering::AcqRel);
            Ok(())
        })
        .unwrap();
    calls.end_group().unwrap();
    let mut result = std::pin::pin!(calls.first_result());
    poll_fn(|cx| {
        assert!(result.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    first.send(Ok(())).unwrap();
    poll_fn(|cx| {
        assert!(result.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    assert_eq!(next_started.load(Ordering::Acquire), 0);
    // Act
    second.send(Ok(())).unwrap();
    result.await.unwrap();
    // Assert
    assert_eq!(next_started.load(Ordering::Acquire), 1);
}

#[tokio::test]
async fn group_failure_drains_started_siblings_without_starting_later_groups() {
    // Arrange
    let (first, first_done) = oneshot::channel();
    let next_started = Arc::new(AtomicUsize::new(0));
    let next = next_started.clone();
    let mut calls = Fanout::new();
    calls.push(async { first_done.await.unwrap() }).unwrap();
    calls
        .push(async { Err(HostError::Backend("current Page".into())) })
        .unwrap();
    calls.end_group().unwrap();
    calls
        .push(async move {
            next.fetch_add(1, Ordering::AcqRel);
            Ok(())
        })
        .unwrap();
    calls.end_group().unwrap();
    // Act
    let result = calls.first_result().await;
    first.send(Ok(())).unwrap();
    calls.drain().await;
    // Assert
    assert!(matches!(result, Err(HostError::Backend(message)) if message == "current Page"));
    assert_eq!(next_started.load(Ordering::Acquire), 0);
    assert!(calls.pending.is_empty());
    assert_eq!(calls.unstarted.len(), 1);
}

#[tokio::test]
async fn immediate_failure_starts_every_sibling_and_drain_keeps_pending_calls_alive() {
    // Arrange
    let started = Arc::new(AtomicUsize::new(0));
    let finished = Arc::new(AtomicUsize::new(0));
    let (complete, completion) = oneshot::channel();
    let mut calls = Fanout::new();
    calls
        .push(async { Err(HostError::Backend("primary".into())) })
        .unwrap();
    let seen = started.clone();
    let done = finished.clone();
    calls
        .push(async move {
            seen.fetch_add(1, Ordering::AcqRel);
            completion.await.unwrap();
            done.fetch_add(1, Ordering::AcqRel);
            Ok(())
        })
        .unwrap();
    // Act
    let primary = calls.first_result().await;
    // Assert: failure is observable but the pending sibling is not canceled.
    assert!(matches!(primary, Err(HostError::Backend(message)) if message == "primary"));
    assert_eq!(started.load(Ordering::Acquire), 1);
    assert_eq!(finished.load(Ordering::Acquire), 0);
    complete.send(()).unwrap();
    calls.drain().await;
    assert_eq!(finished.load(Ordering::Acquire), 1);
}

#[tokio::test]
async fn later_native_failure_is_observed_without_waiting_for_an_earlier_member() {
    // Arrange
    let (first, first_done) = oneshot::channel();
    let (second, second_done) = oneshot::channel();
    let mut calls = Fanout::new();
    calls.push(async { first_done.await.unwrap() }).unwrap();
    calls.push(async { second_done.await.unwrap() }).unwrap();
    calls.start().await;
    second
        .send(Err(HostError::Backend("second".into())))
        .unwrap();
    // Act
    let result = calls.first_result().await;
    // Assert
    assert!(matches!(result, Err(HostError::Backend(message)) if message == "second"));
    first.send(Ok(())).unwrap();
    calls.drain().await;
    assert!(calls.pending.is_empty());
}

#[test]
fn fanout_count_and_future_body_are_checked_before_boxing() {
    // Arrange
    let mut calls = Fanout::new();
    for _ in 0..super::super::MAX_ROUTES {
        calls.push(std::future::pending()).unwrap();
    }
    // Act
    let full = calls.push(std::future::pending());
    let mut empty = Fanout::new();
    let large = [0u8; FUTURE_BYTES + 1];
    let oversized = empty.push(async move {
        std::hint::black_box(large);
        Ok(())
    });
    // Assert
    assert!(matches!(full, Err(HostError::ResourceExhausted(_))));
    assert!(matches!(oversized, Err(HostError::ResourceExhausted(_))));
    assert!(empty.unstarted.is_empty());
}
