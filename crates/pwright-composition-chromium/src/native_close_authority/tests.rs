use super::*;
use serde_json::{Value, json};
use std::task::{Context, Waker};
mod wire;
use wire::Fixture;
const EPOCH: ConnectionEpoch = ConnectionEpoch::new(1);
const CONTEXT: ObjectId = ObjectId::new(2);
const PAGE: ObjectId = ObjectId::new(3);

#[tokio::test]
async fn epoch_change_during_native_wait_prevents_receipt_creation() {
    // Arrange
    let mut f = Fixture::new(1).await;
    let mut future = f.authority.prepare_context(CONTEXT).unwrap().start();
    let packet = packet(&mut f, &mut future).await;
    f.authority
        .0
        .topology
        .lock()
        .unwrap()
        .begin_reconnect(ConnectionEpoch::new(2))
        .unwrap();
    // Act
    f.peer.reply(&packet, json!({})).await;
    let result = future.await;
    // Assert
    assert!(result.is_err());
    assert_eq!(f.authority.0.pending.load(Ordering::Acquire), 0);
    f.finish().await;
}

#[tokio::test]
async fn peer_loss_retires_the_written_command_without_minting_authority() {
    // Arrange
    let mut f = Fixture::new(1).await;
    let mut future = f.authority.prepare_context(CONTEXT).unwrap().start();
    packet(&mut f, &mut future).await;
    // Act
    f.peer.disconnect().await;
    let result = future.await;
    // Assert
    assert!(result.is_err());
    assert_eq!(f.authority.0.pending.load(Ordering::Acquire), 0);
    f.finish().await;
}

#[tokio::test]
async fn maximum_declared_native_identities_fit_and_certify_without_new_storage() {
    // Arrange
    let mut f = Fixture::with_id_bytes(2, 1024).await;
    let context_id = ObjectId::new(10);
    let page_id = ObjectId::new(11);
    let native = "c".repeat(1024);
    {
        let mut topology = f.authority.0.topology.lock().unwrap();
        topology
            .register_context(EPOCH, &native, context_id, CdpContextOwnership::Owned)
            .unwrap();
        topology
            .attach_page(
                EPOCH,
                pwright_backend_cdp::CdpPageTarget::new(
                    "p".repeat(1024),
                    "s".repeat(1024),
                    &native,
                ),
                pwright_backend_cdp::CdpPageObjectIds::new(page_id, ObjectId::new(12)),
            )
            .unwrap();
    }
    let page = f.authority.bind_page(page_id).unwrap();
    let mut future = f.authority.prepare_context(context_id).unwrap().start();
    let packet = packet(&mut f, &mut future).await;
    f.peer.reply(&packet, json!({})).await;
    let receipt = future.await.unwrap();
    let before = f.bytes();
    // Act
    let proof = receipt.certify_page(&page).unwrap();
    // Assert
    assert_eq!(proof.page(), page_id);
    assert_eq!(f.bytes(), before);
    drop(page);
    drop(receipt);
    f.finish().await;
}

#[tokio::test]
async fn route_identity_over_selection_limit_is_rejected_before_admission() {
    // Arrange
    let f = Fixture::with_id_bytes(1, 4).await;
    let before = f.bytes();
    // Act
    let result = f.authority.prepare_context(CONTEXT);
    // Assert
    assert!(matches!(result, Err(error) if error.kind() == FailureKind::ProtocolViolation));
    assert!(f.authority.bind_page(PAGE).is_err());
    assert_eq!(f.bytes(), before);
    assert_eq!(f.authority.0.pending.load(Ordering::Acquire), 0);
    assert!(f.peer.no_commands());
    f.finish().await;
}

#[tokio::test]
async fn context_native_id_rebinding_rejects_the_old_receipt() {
    // Arrange
    let mut f = Fixture::new(2).await;
    let page = f.authority.bind_page(PAGE).unwrap();
    let receipt = close(&mut f).await;
    {
        let mut topology = f.authority.0.topology.lock().unwrap();
        topology.close_context(EPOCH, "context-a").unwrap();
        topology
            .register_context(
                EPOCH,
                "context-a",
                ObjectId::new(10),
                CdpContextOwnership::Owned,
            )
            .unwrap();
    }
    // Act
    let result = receipt.certify_page(&page);
    // Assert
    assert!(result.is_err());
    drop(page);
    drop(receipt);
    f.finish().await;
}

#[tokio::test]
async fn page_target_rebinding_to_another_portable_id_rejects_old_ancestry() {
    // Arrange
    let mut f = Fixture::new(2).await;
    let page = f.authority.bind_page(PAGE).unwrap();
    let receipt = close(&mut f).await;
    {
        let mut topology = f.authority.0.topology.lock().unwrap();
        topology
            .detach_target(EPOCH, "session-a", Some("page-a"))
            .unwrap();
        topology
            .attach_page(
                EPOCH,
                pwright_backend_cdp::CdpPageTarget::new("page-a", "session-new", "context-b"),
                pwright_backend_cdp::CdpPageObjectIds::new(ObjectId::new(10), ObjectId::new(11)),
            )
            .unwrap();
    }
    // Act
    let result = receipt.certify_page(&page);
    // Assert
    assert!(result.is_err());
    drop(page);
    drop(receipt);
    f.finish().await;
}

#[tokio::test]
async fn binding_generation_exhaustion_releases_reserved_capacity() {
    // Arrange
    let f = Fixture::new(1).await;
    let before = f.bytes();
    f.authority.0.sequence.store(u64::MAX, Ordering::Release);
    // Act
    let result = f.authority.bind_page(PAGE);
    // Assert
    assert!(matches!(result, Err(error) if error.kind() == FailureKind::ResourceExhausted));
    assert_eq!(f.bytes(), before);
    assert_eq!(f.authority.0.pending.load(Ordering::Acquire), 0);
    f.finish().await;
}

#[tokio::test]
async fn byte_admission_failure_rolls_back_the_pending_slot() {
    // Arrange
    let f = Fixture::new(1).await;
    let state = &f.authority.0;
    let free = state
        .ledger
        .limits()
        .class(ResourceClass::BackendStateBytes)
        .global
        - f.bytes();
    let held = state
        .ledger
        .reserve(state.profile.scope, ResourceClass::BackendStateBytes, free)
        .unwrap();
    let before = f.bytes();
    // Act
    let result = f.authority.bind_page(PAGE);
    // Assert
    assert!(matches!(result, Err(error) if error.kind() == FailureKind::ResourceExhausted));
    assert_eq!(f.bytes(), before);
    assert_eq!(state.pending.load(Ordering::Acquire), 0);
    assert!(f.peer.no_commands());
    drop(held);
    f.finish().await;
}

#[tokio::test]
async fn terminated_owner_precedes_capacity_failure_even_when_all_slots_are_held() {
    // Arrange
    let f = Fixture::new(1).await;
    let page = f.authority.bind_page(PAGE).unwrap();
    f.stop.store(true, Ordering::Release);
    // Act
    let result = f.authority.prepare_context(CONTEXT);
    // Assert
    assert!(matches!(result, Err(error) if error.kind() == FailureKind::TargetClosed));
    drop(page);
    f.finish().await;
}

#[tokio::test]
async fn default_and_external_contexts_do_not_admit_native_disposal() {
    // Arrange
    let f = Fixture::new(2).await;
    {
        let mut topology = f.authority.0.topology.lock().unwrap();
        topology
            .register_default_context(EPOCH, ObjectId::new(8))
            .unwrap();
        topology
            .register_context(
                EPOCH,
                "external",
                ObjectId::new(9),
                CdpContextOwnership::External,
            )
            .unwrap();
    }
    // Act
    let outcomes = [
        f.authority.prepare_context(ObjectId::new(8)),
        f.authority.prepare_context(ObjectId::new(9)),
    ];
    // Assert
    for outcome in outcomes {
        assert!(
            matches!(outcome, Err(error) if error.kind() == FailureKind::UnsupportedCapability)
        );
    }
    assert_eq!(f.authority.0.pending.load(Ordering::Acquire), 0);
    assert!(f.peer.no_commands());
    f.finish().await;
}

async fn packet(f: &mut Fixture, future: &mut NativeContextCloseFuture) -> Value {
    let packet = tokio::select! {
        _ = future.as_mut() => panic!("Context command completed before peer reply"),
        packet = f.peer.command() => packet,
    };
    assert_eq!(packet["method"], "Target.disposeBrowserContext");
    packet
}
async fn close(f: &mut Fixture) -> NativeContextCloseReceipt {
    let mut future = f.authority.prepare_context(CONTEXT).unwrap().start();
    let packet = packet(f, &mut future).await;
    f.peer.reply(&packet, json!({})).await;
    future.await.unwrap()
}

#[tokio::test]
async fn native_context_ack_certifies_exact_preexisting_page_without_holding_mutation_gate() {
    // Arrange
    let mut f = Fixture::new(2).await;
    let page = f.authority.bind_page(PAGE).unwrap();
    let mut future = f.authority.prepare_context(CONTEXT).unwrap().start();
    let packet = packet(&mut f, &mut future).await;
    assert_eq!(packet["params"]["browserContextId"], "context-a");
    assert!(!f.authority.0.topology.lock().unwrap().transaction_active());
    assert!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    f.peer.reply(&packet, json!({})).await;
    let receipt = future.await.unwrap();
    let proof = receipt.certify_page(&page).unwrap();
    // Assert
    assert_eq!(proof.page(), PAGE);
    assert_eq!(proof.context(), CONTEXT);
    assert_eq!(receipt.epoch(), EPOCH);
    drop(page);
    drop(receipt);
    f.finish().await;
}

#[tokio::test]
async fn same_native_strings_on_another_attachment_are_not_ancestor_authority() {
    // Arrange
    let mut f = Fixture::new(2).await;
    let other = Fixture::new(1).await;
    let page = other.authority.bind_page(PAGE).unwrap();
    let receipt = close(&mut f).await;
    // Act
    let result = receipt.certify_page(&page);
    // Assert
    assert!(result.is_err());
    drop(receipt);
    drop(page);
    f.finish().await;
    other.finish().await;
}

#[tokio::test]
async fn another_context_on_the_same_connection_is_not_covered() {
    // Arrange
    let mut f = Fixture::new(2).await;
    let page = f.authority.bind_page(ObjectId::new(6)).unwrap();
    let receipt = close(&mut f).await;
    // Act
    let result = receipt.certify_page(&page);
    // Assert
    assert!(result.is_err());
    drop(page);
    drop(receipt);
    f.finish().await;
}

#[tokio::test]
async fn a_binding_admitted_after_receipt_cannot_reuse_that_receipt() {
    // Arrange
    let mut f = Fixture::new(2).await;
    let receipt = close(&mut f).await;
    let page = f.authority.bind_page(PAGE).unwrap();
    // Act
    let result = receipt.certify_page(&page);
    // Assert
    assert!(result.is_err());
    drop(page);
    drop(receipt);
    f.finish().await;
}

#[tokio::test]
async fn topology_retirement_does_not_erase_already_captured_native_ancestry() {
    // Arrange
    let mut f = Fixture::new(2).await;
    let page = f.authority.bind_page(PAGE).unwrap();
    let mut future = f.authority.prepare_context(CONTEXT).unwrap().start();
    let packet = packet(&mut f, &mut future).await;
    f.authority
        .0
        .topology
        .lock()
        .unwrap()
        .close_context(EPOCH, "context-a")
        .unwrap();
    // Act
    f.peer.reply(&packet, json!({})).await;
    let receipt = future.await.unwrap();
    let result = receipt.certify_page(&page);
    // Assert
    assert!(result.is_ok());
    drop(page);
    drop(receipt);
    f.finish().await;
}

#[tokio::test]
async fn epoch_change_invalidates_existing_receipts_and_further_bindings() {
    // Arrange
    let mut f = Fixture::new(3).await;
    let page = f.authority.bind_page(PAGE).unwrap();
    let receipt = close(&mut f).await;
    f.authority
        .0
        .topology
        .lock()
        .unwrap()
        .begin_reconnect(ConnectionEpoch::new(2))
        .unwrap();
    // Act
    let result = receipt.certify_page(&page);
    // Assert
    assert!(result.is_err());
    assert!(f.authority.bind_page(PAGE).is_err());
    assert!(f.authority.prepare_context(CONTEXT).is_err());
    drop(page);
    drop(receipt);
    f.finish().await;
}

#[tokio::test]
async fn already_closed_native_error_is_not_a_successful_ancestor_receipt() {
    // Arrange
    let mut f = Fixture::new(1).await;
    let mut future = f.authority.prepare_context(CONTEXT).unwrap().start();
    let packet = packet(&mut f, &mut future).await;
    // Act
    f.peer
        .error(&packet, "Failed to find context with id context-a")
        .await;
    let result = future.await;
    // Assert
    assert!(matches!(result, Err(error) if error.kind() == FailureKind::TargetClosed));
    assert_eq!(f.authority.0.pending.load(Ordering::Acquire), 0);
    f.finish().await;
}

#[tokio::test]
async fn interruption_after_write_preserves_reply_ownership_but_mints_no_receipt() {
    // Arrange
    let mut f = Fixture::new(1).await;
    let mut future = f.authority.prepare_context(CONTEXT).unwrap().start();
    let packet = packet(&mut f, &mut future).await;
    f.stop.store(true, Ordering::Release);
    assert!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    f.peer.reply(&packet, json!({})).await;
    let result = future.await;
    // Assert
    assert!(matches!(result, Err(error) if error.kind() == FailureKind::TargetClosed));
    assert_eq!(f.authority.0.pending.load(Ordering::Acquire), 0);
    f.finish().await;
}

#[tokio::test]
async fn shared_intent_capacity_rejects_before_command_and_drop_reclaims_it() {
    // Arrange
    let f = Fixture::new(1).await;
    let before = f.bytes();
    let page = f.authority.bind_page(PAGE).unwrap();
    assert_eq!(f.bytes(), before + INTENT_BYTES);
    // Act
    let result = f.authority.clone().prepare_context(CONTEXT);
    // Assert
    assert!(matches!(result, Err(error) if error.kind() == FailureKind::ResourceExhausted));
    assert!(f.peer.no_commands());
    drop(page);
    assert_eq!(f.bytes(), before);
    drop(f.authority.prepare_context(CONTEXT).unwrap());
    assert_eq!(f.bytes(), before);
    f.finish().await;
}

#[tokio::test]
async fn prepared_context_revalidates_before_native_mutation() {
    // Arrange
    let f = Fixture::new(1).await;
    let prepared = f.authority.prepare_context(CONTEXT).unwrap();
    f.authority
        .0
        .topology
        .lock()
        .unwrap()
        .close_context(EPOCH, "context-a")
        .unwrap();
    // Act
    let result = prepared.start().await;
    // Assert
    assert!(result.is_err());
    assert!(f.peer.no_commands());
    assert_eq!(f.authority.0.pending.load(Ordering::Acquire), 0);
    f.finish().await;
}
