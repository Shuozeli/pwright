use super::*;
use serde_json::{Value, json};
use std::task::{Context, Waker};
mod wire;
use wire::Fixture;

#[tokio::test]
async fn oversized_validator_is_rejected_before_command_allocation_or_write() {
    // Arrange
    let fixture = Fixture::new(1).await;
    let before = fixture.bytes();
    let payload = [0_u8; CALL_BYTES as usize];
    // Act
    let result = fixture
        .source
        .prepare_command_with_validation("page", move || {
            std::hint::black_box(&payload);
            Ok(())
        });
    // Assert
    assert!(matches!(result, Err(error) if error.kind() == FailureKind::ResourceExhausted));
    assert_eq!(fixture.bytes(), before);
    assert_eq!(fixture.source.0.pending.load(Ordering::Acquire), 0);
    assert!(fixture.peer.no_commands());
    fixture.finish().await;
}

#[tokio::test]
async fn rejecting_validator_prevents_discovery_and_close_commands() {
    // Arrange
    let fixture = Fixture::new(1).await;
    let prepared = fixture
        .source
        .prepare_command_with_validation("page", || {
            Err(failure(FailureKind::TargetClosed, "route invalidated"))
        })
        .unwrap();
    // Act
    let result = prepared.start().await;
    // Assert
    assert!(matches!(result, Err(error) if error.kind() == FailureKind::TargetClosed));
    assert!(fixture.peer.no_commands());
    assert_eq!(fixture.source.0.pending.load(Ordering::Acquire), 0);
    fixture.finish().await;
}

#[tokio::test]
async fn peer_loss_retires_a_written_command_without_acknowledgement() {
    // Arrange
    let mut fixture = Fixture::new(1).await;
    let mut command = fixture.source.prepare_command("page").unwrap().start();
    close_packet(&mut fixture, &mut command).await;
    assert!(pending(&mut command));
    // Act
    fixture.peer.disconnect().await;
    let result = command.await;
    // Assert
    assert!(result.is_err());
    assert_eq!(fixture.source.0.pending.load(Ordering::Acquire), 0);
    fixture.finish().await;
}

#[tokio::test]
async fn peer_loss_after_acknowledgement_is_not_a_destruction_receipt() {
    // Arrange
    let mut fixture = Fixture::new(1).await;
    let mut command = fixture.source.prepare_command("page").unwrap().start();
    let packet = close_packet(&mut fixture, &mut command).await;
    fixture.peer.reply(&packet, json!({"success": true})).await;
    let acknowledged = command.await.unwrap();
    // Act
    fixture.peer.disconnect().await;
    let result = acknowledged.wait_for_destruction().await;
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::TargetClosed);
    assert_eq!(fixture.source.0.pending.load(Ordering::Acquire), 0);
    fixture.finish().await;
}

#[tokio::test]
async fn stopped_during_discovery_retires_that_reply_without_sending_target_close() {
    // Arrange
    let mut fixture = Fixture::new(1).await;
    let mut command = fixture.source.prepare_command("page").unwrap().start();
    let packet = tokio::select! {
        _ = command.as_mut() => panic!("discovery was not acknowledged"),
        packet = fixture.peer.command() => packet,
    };
    assert_eq!(packet["method"], "Target.setDiscoverTargets");
    fixture.stop.stop();
    assert!(pending(&mut command));
    // Act
    fixture.peer.reply(&packet, json!({})).await;
    let result = command.await;
    // Assert
    assert!(matches!(result, Err(error) if error.kind() == FailureKind::Cancelled));
    assert!(fixture.peer.no_commands());
    assert_eq!(fixture.source.0.pending.load(Ordering::Acquire), 0);
    fixture.finish().await;
}

#[tokio::test]
async fn independent_command_can_acknowledge_while_an_earlier_receipt_is_pending() {
    // Arrange
    let mut fixture = Fixture::new(2).await;
    let before = fixture.bytes();
    let mut first = fixture.source.prepare_command("first").unwrap().start();
    let packet = close_packet(&mut fixture, &mut first).await;
    fixture.peer.reply(&packet, json!({"success": true})).await;
    let mut first_receipt = first.await.unwrap().wait_for_destruction();
    let mut second = fixture.source.prepare_command("second").unwrap().start();
    let packet = close_packet(&mut fixture, &mut second).await;
    assert_eq!(packet["params"]["targetId"], "second");
    assert_eq!(fixture.source.0.pending.load(Ordering::Acquire), 2);
    assert_eq!(fixture.bytes(), before + 2 * CALL_BYTES);
    // Act
    fixture.peer.reply(&packet, json!({"success": true})).await;
    let second_receipt = second.await.unwrap();
    fixture.peer.destroy("second").await;
    let result = second_receipt.wait_for_destruction().await;
    // Assert
    result.unwrap();
    assert!(
        first_receipt
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    assert_eq!(fixture.source.0.pending.load(Ordering::Acquire), 1);
    fixture.peer.destroy("first").await;
    first_receipt.await.unwrap();
    assert_eq!(fixture.bytes(), before);
    fixture.finish().await;
}

#[tokio::test]
async fn maximum_native_identity_fits_but_oversized_identity_is_rejected_before_admission() {
    // Arrange
    let fixture = Fixture::new(1).await;
    let before = fixture.bytes();
    let maximum = "x".repeat(1024);
    let over = "x".repeat(1025);
    // Act
    let admitted = fixture.source.prepare_command(&maximum);
    // Assert
    assert!(admitted.is_ok());
    drop(admitted);
    assert!(fixture.source.prepare_command(&over).is_err());
    assert!(fixture.source.prepare_command("").is_err());
    assert_eq!(fixture.bytes(), before);
    assert_eq!(fixture.source.0.pending.load(Ordering::Acquire), 0);
    assert!(fixture.peer.no_commands());
    fixture.finish().await;
}

#[tokio::test]
async fn unsuccessful_or_malformed_native_response_cannot_create_acknowledgement() {
    for result in [json!({"success": false}), json!({})] {
        // Arrange
        let mut fixture = Fixture::new(1).await;
        let mut command = fixture.source.prepare_command("page").unwrap().start();
        let packet = close_packet(&mut fixture, &mut command).await;
        // Act
        fixture.peer.reply(&packet, result).await;
        let outcome = command.await;
        // Assert
        assert!(outcome.is_err());
        assert_eq!(fixture.source.0.pending.load(Ordering::Acquire), 0);
        fixture.finish().await;
    }
}

fn pending(future: &mut NativePageCloseCommandFuture) -> bool {
    future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
        .is_pending()
}

async fn close_packet(fixture: &mut Fixture, command: &mut NativePageCloseCommandFuture) -> Value {
    loop {
        let packet = tokio::select! {
            _ = command.as_mut() => panic!("command settled before its reply"),
            packet = fixture.peer.command() => packet,
        };
        match packet["method"].as_str().unwrap() {
            "Target.setDiscoverTargets" => fixture.peer.reply(&packet, json!({})).await,
            "Target.closeTarget" => return packet,
            other => panic!("unexpected command {other}"),
        }
    }
}

#[tokio::test]
async fn unstarted_split_owner_reserves_and_releases_exact_call_capacity() {
    // Arrange
    let fixture = Fixture::new(1).await;
    let before = fixture.bytes();
    let command = fixture.source.prepare_command("page").unwrap();
    assert_eq!(fixture.source.0.pending.load(Ordering::Acquire), 1);
    assert_eq!(fixture.bytes(), before + CALL_BYTES);
    assert!(fixture.source.prepare_command("second").is_err());
    // Act
    drop(command);
    // Assert
    assert_eq!(fixture.source.0.pending.load(Ordering::Acquire), 0);
    assert_eq!(fixture.bytes(), before);
    assert!(fixture.peer.no_commands());
    fixture.finish().await;
}

#[tokio::test]
async fn acknowledgement_retains_lease_and_cursor_until_exact_destruction() {
    // Arrange
    let mut fixture = Fixture::new(1).await;
    let before = fixture.bytes();
    let mut command = fixture.source.prepare_command("page").unwrap().start();
    let packet = close_packet(&mut fixture, &mut command).await;
    assert_eq!(packet["params"]["targetId"], "page");
    assert!(pending(&mut command));
    fixture.peer.reply(&packet, json!({"success": true})).await;
    let acknowledged = command.await.unwrap();
    assert_eq!(acknowledged.target_id(), "page");
    assert!(acknowledged.belongs_to(&fixture.source.clone()));
    assert_eq!(fixture.bytes(), before + CALL_BYTES);
    assert_eq!(fixture.source.0.pending.load(Ordering::Acquire), 1);
    let mut receipt = acknowledged.wait_for_destruction();
    assert!(
        receipt
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    fixture.peer.destroy("page").await;
    let result = receipt.await;
    // Assert
    result.unwrap();
    assert_eq!(fixture.bytes(), before);
    assert_eq!(fixture.source.0.pending.load(Ordering::Acquire), 0);
    fixture.finish().await;
}

#[tokio::test]
async fn early_destruction_does_not_skip_command_reply_and_is_retained_for_receipt() {
    // Arrange
    let mut fixture = Fixture::new(1).await;
    let mut command = fixture.source.prepare_command("page").unwrap().start();
    let packet = close_packet(&mut fixture, &mut command).await;
    fixture.peer.destroy("page").await;
    assert!(pending(&mut command));
    // Act
    fixture.peer.reply(&packet, json!({"success": true})).await;
    let acknowledged = command.await.unwrap();
    let result = acknowledged.wait_for_destruction().await;
    // Assert
    result.unwrap();
    assert_eq!(fixture.source.0.pending.load(Ordering::Acquire), 0);
    fixture.finish().await;
}

#[tokio::test]
async fn owner_stop_after_write_does_not_discard_the_command_reply_owner() {
    // Arrange
    let mut fixture = Fixture::new(1).await;
    let mut command = fixture.source.prepare_command("page").unwrap().start();
    let packet = close_packet(&mut fixture, &mut command).await;
    fixture.stop.stop();
    assert!(pending(&mut command));
    // Act
    fixture.peer.reply(&packet, json!({"success": true})).await;
    let acknowledged = command.await.unwrap();
    let result = acknowledged.wait_for_destruction().await;
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::Cancelled);
    assert_eq!(fixture.source.0.pending.load(Ordering::Acquire), 0);
    fixture.finish().await;
}

#[tokio::test]
async fn native_error_cannot_create_an_acknowledgement_even_with_early_destruction() {
    // Arrange
    let mut fixture = Fixture::new(1).await;
    let before = fixture.bytes();
    let mut command = fixture.source.prepare_command("page").unwrap().start();
    let packet = close_packet(&mut fixture, &mut command).await;
    fixture.peer.destroy("page").await;
    // Act
    fixture.peer.error(&packet, "native close denied").await;
    let result = command.await;
    // Assert
    assert!(matches!(result, Err(error) if error.kind() == FailureKind::Backend));
    assert_eq!(fixture.source.0.pending.load(Ordering::Acquire), 0);
    assert_eq!(fixture.bytes(), before);
    fixture.finish().await;
}

#[tokio::test]
async fn stopped_unstarted_command_does_not_send_discovery_or_close() {
    // Arrange
    let fixture = Fixture::new(1).await;
    let prepared = fixture.source.prepare_command("page").unwrap();
    fixture.stop.stop();
    // Act
    let result = prepared.start().await;
    // Assert
    assert!(matches!(result, Err(error) if error.kind() == FailureKind::Cancelled));
    assert!(fixture.peer.no_commands());
    assert_eq!(fixture.source.0.pending.load(Ordering::Acquire), 0);
    fixture.finish().await;
}

#[tokio::test]
async fn failed_discovery_never_sends_target_close() {
    // Arrange
    let mut fixture = Fixture::new(1).await;
    let mut command = fixture.source.prepare_command("page").unwrap().start();
    let packet = tokio::select! {
        _ = command.as_mut() => panic!("discovery was not acknowledged"),
        packet = fixture.peer.command() => packet,
    };
    assert_eq!(packet["method"], "Target.setDiscoverTargets");
    // Act
    fixture.peer.error(&packet, "discovery denied").await;
    let result = command.await;
    // Assert
    assert!(result.is_err());
    assert!(!fixture.source.0.discovery_enabled.load(Ordering::Acquire));
    assert_eq!(fixture.source.0.pending.load(Ordering::Acquire), 0);
    assert!(fixture.peer.no_commands());
    fixture.finish().await;
}

#[tokio::test]
async fn same_target_on_another_source_is_not_the_acknowledgement_owner() {
    // Arrange
    let mut fixture = Fixture::new(1).await;
    let other = Fixture::new(1).await;
    let mut command = fixture.source.prepare_command("page").unwrap().start();
    let packet = close_packet(&mut fixture, &mut command).await;
    fixture.peer.reply(&packet, json!({"success": true})).await;
    let acknowledged = command.await.unwrap();
    // Act
    let belongs = acknowledged.belongs_to(&other.source);
    // Assert
    assert!(!belongs);
    drop(acknowledged);
    assert_eq!(fixture.source.0.pending.load(Ordering::Acquire), 0);
    other.finish().await;
    fixture.finish().await;
}

#[tokio::test]
async fn acknowledged_drop_reclaims_wait_ownership_without_an_extra_command() {
    // Arrange
    let mut fixture = Fixture::new(1).await;
    let before = fixture.bytes();
    let mut command = fixture.source.prepare_command("page").unwrap().start();
    let packet = close_packet(&mut fixture, &mut command).await;
    fixture.peer.reply(&packet, json!({"success": true})).await;
    let acknowledged = command.await.unwrap();
    // Act
    drop(acknowledged);
    // Assert
    assert_eq!(fixture.bytes(), before);
    assert_eq!(fixture.source.0.pending.load(Ordering::Acquire), 0);
    assert!(fixture.peer.no_commands());
    drop(fixture.source.prepare_command("replacement").unwrap());
    fixture.finish().await;
}
