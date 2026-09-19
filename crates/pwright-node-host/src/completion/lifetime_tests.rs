use super::*;
use std::future::pending;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll, Waker};

#[tokio::test]
async fn closed_lifetime_never_reenters_publication() {
    // Arrange
    let publication = async { panic!("closed Page cannot enqueue a publication fence") };
    // Act
    let result = publish_lifetime_result(Ok(42), true, publication, pending(), || true).await;
    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
}

#[tokio::test]
async fn page_closed_during_publication_overrides_prior_network_error() {
    // Arrange
    let closed = AtomicBool::new(false);
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let publication = async {
        receiver.await.unwrap();
        Ok(())
    };
    let result = Err::<(), _>(HostError::Backend("net::ERR_ABORTED".into()));
    let completed = publish_lifetime_result(result, true, publication, pending(), || {
        closed.load(Ordering::Acquire)
    });
    let mut completed = std::pin::pin!(completed);
    assert!(
        completed
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    closed.store(true, Ordering::Release);
    sender.send(()).unwrap();
    let result = completed.await;
    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
}

#[tokio::test]
async fn page_closed_during_publication_overrides_prior_success() {
    // Arrange
    let closed = AtomicBool::new(false);
    let publication = async {
        closed.store(true, Ordering::Release);
        Ok(())
    };
    // Act
    let result = publish_lifetime_result(Ok(42), true, publication, pending(), || {
        closed.load(Ordering::Acquire)
    })
    .await;
    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
}

#[tokio::test]
async fn terminal_lifetime_wins_over_fence_failure() {
    // Arrange
    let closed = AtomicBool::new(false);
    let publication = async {
        closed.store(true, Ordering::Release);
        Err(HostError::ResourceExhausted("fence full".into()))
    };
    // Act
    let result = publish_lifetime_result(Ok(42), true, publication, pending(), || {
        closed.load(Ordering::Acquire)
    })
    .await;
    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
}

#[tokio::test]
async fn live_page_keeps_its_original_navigation_error() {
    // Arrange
    let result = Err::<(), _>(HostError::Backend("net::ERR_ABORTED".into()));
    // Act
    let result = publish_lifetime_result(result, true, async { Ok(()) }, pending(), || false).await;
    // Assert
    assert!(matches!(result, Err(HostError::Backend(message)) if message == "net::ERR_ABORTED"));
}

#[tokio::test]
async fn live_page_keeps_publication_capacity_failure() {
    // Arrange
    let publication = async { Err(HostError::ResourceExhausted("fence full".into())) };
    // Act
    let result = publish_lifetime_result(Ok(42), true, publication, pending(), || false).await;
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
}

#[test]
fn live_success_finishes_when_publication_is_ready() {
    // Arrange
    let mut result = std::pin::pin!(publish_lifetime_result(
        Ok(42),
        true,
        async { Ok(()) },
        pending(),
        || false
    ));
    // Act
    let settled = result
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()));
    // Assert
    assert!(matches!(settled, Poll::Ready(Ok(42))));
}

#[tokio::test]
async fn page_close_interrupts_a_publication_that_never_finishes() {
    // Arrange
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let closed = async {
        receiver.await.unwrap();
    };
    let result = Err::<(), _>(HostError::Backend("net::ERR_ABORTED".into()));
    let completed = publish_lifetime_result(result, true, pending(), closed, || false);
    let mut completed = std::pin::pin!(completed);
    assert!(
        completed
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    sender.send(()).unwrap();
    let result = completed.await;
    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
}

#[tokio::test]
async fn simultaneously_ready_page_close_precedes_publication_success() {
    // Arrange
    let publication = async { Ok(()) };
    // Act
    let result = publish_lifetime_result(Ok(42), true, publication, async {}, || false).await;
    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
}
