use super::*;
use pwright_cdp::CdpEvent;
use pwright_fake::FakeCdpClient;
use std::{
    future::Future,
    task::{Context, Poll, Waker},
};

fn event(method: &str, target: &str) -> CdpEvent {
    CdpEvent::new(
        method.into(),
        serde_json::json!({"targetId":target,"sessionId":"inspector"}),
        None,
    )
}

#[test]
fn pending_admission_cannot_borrow_another_consumers_unused_budget() {
    // Arrange
    let pending = AtomicUsize::new(1);
    // Act
    let result = admit_pending(&pending, 1);
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::ResourceExhausted);
    assert_eq!(pending.load(Ordering::Acquire), 1);
}

#[test]
fn reused_pending_capacity_admits_exactly_one_owner() {
    // Arrange
    let pending = AtomicUsize::new(0);
    // Act
    let result = admit_pending(&pending, 1);
    // Assert
    assert!(result.is_ok());
    assert_eq!(pending.load(Ordering::Acquire), 1);
}

#[test]
fn measured_installation_bytes_are_part_of_the_declared_plan() {
    // Arrange
    let base = NativePageCloseProfile::new(ResourceScope::Connection, 1, 128).unwrap();
    let measured = base.with_installation_owner_bytes(9876).unwrap();
    // Act
    let requirement = measured
        .requirements()
        .class(ResourceClass::BackendStateBytes)
        .global;
    // Assert
    assert_eq!(
        requirement,
        base.requirements()
            .class(ResourceClass::BackendStateBytes)
            .global
            + 9876
    );
    assert_eq!(measured.event_subscribers(), 1);
}

#[tokio::test]
async fn owner_termination_ends_receipt_wait_without_claiming_target_death() {
    // Arrange
    let browser = FakeCdpClient::from_html("");
    let (_sender, receiver) = tokio::sync::broadcast::channel(4);
    let mut receiver = CdpEventReceiver::from(receiver);
    // Act
    let result = close_with_receipt_and_interruption(
        &browser,
        "page",
        &mut receiver,
        Some(Box::pin(async {})),
    )
    .await;
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::Cancelled);
    assert_eq!(browser.method_names(), ["Target.closeTarget"]);
}

#[tokio::test]
async fn exact_native_destruction_confirms_close_without_absence_polling() {
    // Arrange
    let browser = FakeCdpClient::from_html("");
    let (sender, receiver) = tokio::sync::broadcast::channel(4);
    let mut receiver = CdpEventReceiver::from(receiver);
    sender
        .send(event("Target.targetDestroyed", "page"))
        .unwrap();
    // Act
    let result = close_with_receipt(&browser, "page", &mut receiver).await;
    // Assert
    assert!(result.is_ok());
    assert_eq!(browser.method_names(), ["Target.closeTarget"]);
}

#[tokio::test]
async fn acknowledgement_detach_and_unrelated_receipts_do_not_confirm_close() {
    // Arrange
    let browser = FakeCdpClient::from_html("");
    let (sender, receiver) = tokio::sync::broadcast::channel(64);
    let mut receiver = CdpEventReceiver::from(receiver);
    sender
        .send(event("Target.detachedFromTarget", "page"))
        .unwrap();
    for _ in 0..32 {
        sender
            .send(event("Target.targetDestroyed", "other"))
            .unwrap();
    }
    let mut future = Box::pin(close_with_receipt(&browser, "page", &mut receiver));
    let mut context = Context::from_waker(Waker::noop());
    // Act
    let status = future.as_mut().poll(&mut context);
    // Assert
    assert!(
        status.is_pending(),
        "more than 20 unrelated receipts is not a close deadline"
    );
    assert_eq!(browser.method_names(), ["Target.closeTarget"]);
}

#[tokio::test]
async fn receipt_arriving_after_the_command_completes_the_wait() {
    // Arrange
    let browser = FakeCdpClient::from_html("");
    let (sender, receiver) = tokio::sync::broadcast::channel(4);
    let mut receiver = CdpEventReceiver::from(receiver);
    let mut future = Box::pin(close_with_receipt(&browser, "page", &mut receiver));
    let mut context = Context::from_waker(Waker::noop());
    assert!(future.as_mut().poll(&mut context).is_pending());
    // Act
    sender
        .send(event("Target.targetDestroyed", "page"))
        .unwrap();
    let result = future.await;
    // Assert
    assert!(result.is_ok());
}

#[tokio::test]
async fn receipt_stream_loss_is_not_a_successful_native_close() {
    // Arrange
    let browser = FakeCdpClient::from_html("");
    let (sender, receiver) = tokio::sync::broadcast::channel::<CdpEvent>(4);
    let mut receiver = CdpEventReceiver::from(receiver);
    drop(sender);
    // Act
    let result = close_with_receipt(&browser, "page", &mut receiver).await;
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::TargetClosed);
}

#[tokio::test]
async fn lost_receipts_fail_instead_of_accepting_a_later_event() {
    // Arrange
    let browser = FakeCdpClient::from_html("");
    let (sender, receiver) = tokio::sync::broadcast::channel(1);
    let mut receiver = CdpEventReceiver::from(receiver);
    sender
        .send(event("Target.targetDestroyed", "other"))
        .unwrap();
    sender
        .send(event("Target.targetDestroyed", "page"))
        .unwrap();
    // Act
    let result = close_with_receipt(&browser, "page", &mut receiver).await;
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::Backend);
}

#[tokio::test]
async fn malformed_receipt_is_an_explicit_failure() {
    // Arrange
    let browser = FakeCdpClient::from_html("");
    let (sender, receiver) = tokio::sync::broadcast::channel(1);
    let mut receiver = CdpEventReceiver::from(receiver);
    sender
        .send(CdpEvent::new(
            "Target.targetDestroyed".into(),
            serde_json::json!({}),
            None,
        ))
        .unwrap();
    // Act
    let result = close_with_receipt(&browser, "page", &mut receiver).await;
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::Backend);
}

#[tokio::test]
async fn scoped_receipt_cannot_replace_browser_target_destruction() {
    // Arrange
    let browser = FakeCdpClient::from_html("");
    let (sender, receiver) = tokio::sync::broadcast::channel(1);
    let mut receiver = CdpEventReceiver::from(receiver);
    sender
        .send(CdpEvent::new(
            "Target.targetDestroyed".into(),
            serde_json::json!({"targetId":"page"}),
            Some("child".into()),
        ))
        .unwrap();
    let mut future = Box::pin(close_with_receipt(&browser, "page", &mut receiver));
    // Act
    let status = future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()));
    // Assert
    assert!(matches!(status, Poll::Pending));
}
