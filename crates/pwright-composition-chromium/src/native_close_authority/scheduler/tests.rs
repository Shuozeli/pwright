use super::*;
use serde_json::{Value, json};
use std::{future::poll_fn, task::Waker, time::Duration};
#[cfg(feature = "close-capacity")]
mod capacity;
mod commit_scope;
mod consume;
mod wire;
use wire::Fixture;
const EPOCH: ConnectionEpoch = ConnectionEpoch::new(1);
const CONTEXT: ObjectId = ObjectId::new(2);
const PAGE: ObjectId = ObjectId::new(3);
const OTHER_CONTEXT: ObjectId = ObjectId::new(5);

#[tokio::test]
async fn refilling_low_slots_cannot_starve_an_older_ready_completion() {
    // Arrange
    let mut f = Fixture::new(2).await;
    for payload in [10, 20] {
        let slot = f.scheduler.admission.reserve().unwrap();
        let stage = Stage::Complete(Err(fail(FailureKind::Backend, "completed native error")));
        assert!(f.scheduler.insert(payload, Ok((slot, stage))).is_ok());
    }
    drop(next(&mut f).await);
    let slot = f.scheduler.admission.reserve().unwrap();
    let stage = Stage::Complete(Err(fail(FailureKind::Backend, "completed replacement")));
    assert!(f.scheduler.insert(30, Ok((slot, stage))).is_ok());
    // Act
    let older = next(&mut f).await;
    // Assert
    assert_eq!(*older.payload(), 20);
    assert!(older.result().is_err());
    assert_eq!(older.into_payload(), 20);
    drop(next(&mut f).await);
    f.finish().await;
}

fn push_page(f: &mut Fixture, payload: u64) {
    assert!(f.scheduler.push_page(PAGE, payload).is_ok());
}
fn push_context(f: &mut Fixture, context: ObjectId, payload: u64) {
    assert!(f.scheduler.push_context(context, payload).is_ok());
}
fn poll_once<T>(f: &mut wire::GenericFixture<T>) -> Poll<Option<NativeCloseCompletion<T>>> {
    f.scheduler
        .poll_next(&mut Context::from_waker(Waker::noop()))
}
async fn command<T>(f: &mut wire::GenericFixture<T>) -> Value {
    tokio::select! {
        biased;
        result = poll_fn(|cx| f.scheduler.poll_next(cx)) => panic!("unexpected completion before command: {}", result.is_some()),
        packet = f.peer.command() => packet,
    }
}
async fn next<T>(f: &mut wire::GenericFixture<T>) -> NativeCloseCompletion<T> {
    tokio::time::timeout(
        Duration::from_secs(10),
        poll_fn(|cx| f.scheduler.poll_next(cx)),
    )
    .await
    .unwrap()
    .expect("native completion")
}
async fn page_command<T>(f: &mut wire::GenericFixture<T>) -> Value {
    let discovery = command(f).await;
    assert_eq!(discovery["method"], "Target.setDiscoverTargets");
    f.peer.reply(&discovery, json!({})).await;
    let page = command(f).await;
    assert_eq!(page["method"], "Target.closeTarget");
    page
}
async fn acknowledge_page<T>(f: &mut wire::GenericFixture<T>) {
    let page = page_command(f).await;
    f.peer.reply(&page, json!({"success": true})).await;
    // Drive the command response into the destruction phase without creating a
    // sleep-based scheduling assumption; a separate native command is the fence.
    let connection = f.scheduler.authority.0.connection.clone();
    let fence = connection.send_command("Browser.getVersion", json!({}));
    tokio::pin!(fence);
    let packet = tokio::select! {
        _ = &mut fence => panic!("fence replied before peer response"),
        packet = f.peer.command() => packet,
    };
    f.peer.reply(&packet, json!({"product": "fixture"})).await;
    fence.await.unwrap();
    assert!(poll_once(f).is_pending());
}

#[tokio::test]
async fn context_dispatches_while_page_destruction_is_withheld() {
    // Arrange
    let mut f = Fixture::new(2).await;
    push_page(&mut f, 10);
    acknowledge_page(&mut f).await;
    push_context(&mut f, CONTEXT, 20);
    // Act
    let context = command(&mut f).await;
    // Assert
    assert_eq!(context["method"], "Target.disposeBrowserContext");
    assert_eq!(context["params"]["browserContextId"], "context-a");
    assert!(
        f.scheduler
            .authority
            .0
            .topology
            .lock()
            .unwrap()
            .prepare_page_close(PAGE)
            .is_ok()
    );
    f.peer.reply(&context, json!({})).await;
    let a = next(&mut f).await;
    let b = next(&mut f).await;
    assert_eq!([a.payload, b.payload], [10, 20]);
    assert_eq!(
        a.result.as_ref().unwrap().validate().unwrap(),
        NativeCloseTarget::Page(PAGE)
    );
    assert_eq!(
        b.result.as_ref().unwrap().validate().unwrap(),
        NativeCloseTarget::Context(CONTEXT)
    );
    assert!(f.scheduler.is_empty());
    drop((a, b));
    f.finish().await;
}

#[tokio::test]
async fn ancestor_completion_retains_a_still_written_child_command_and_capacity() {
    // Arrange
    let mut f = Fixture::new(2).await;
    push_page(&mut f, 10);
    let page = page_command(&mut f).await;
    push_context(&mut f, CONTEXT, 20);
    let context = command(&mut f).await;
    f.peer.reply(&context, json!({})).await;
    let parent = next(&mut f).await;
    assert_eq!(parent.payload, 20);
    drop(parent);
    // Act
    let rejected = f.scheduler.push_context(OTHER_CONTEXT, 30);
    // Assert
    assert!(matches!(rejected, Err((30, error)) if error.kind() == FailureKind::ResourceExhausted));
    assert!(poll_once(&mut f).is_pending());
    assert!(!f.scheduler.is_empty());
    f.peer.reply(&page, json!({"success": true})).await;
    let child = next(&mut f).await;
    assert_eq!(child.payload, 10);
    assert!(child.result.as_ref().unwrap().validate().is_ok());
    drop(child);
    assert_eq!(f.scheduler.admission.pending.load(Ordering::Acquire), 0);
    f.finish().await;
}

#[tokio::test]
async fn different_context_cannot_settle_a_page_wait() {
    // Arrange
    let mut f = Fixture::new(2).await;
    push_page(&mut f, 10);
    acknowledge_page(&mut f).await;
    push_context(&mut f, OTHER_CONTEXT, 20);
    let context = command(&mut f).await;
    // Act
    f.peer.reply(&context, json!({})).await;
    let parent = next(&mut f).await;
    // Assert
    assert_eq!(parent.payload, 20);
    assert!(poll_once(&mut f).is_pending());
    f.peer.destroy("page-a").await;
    let child = next(&mut f).await;
    assert!(child.result.as_ref().unwrap().validate().is_ok());
    drop((parent, child));
    f.finish().await;
}

#[tokio::test]
async fn context_error_is_not_ancestor_death() {
    // Arrange
    let mut f = Fixture::new(2).await;
    push_page(&mut f, 10);
    acknowledge_page(&mut f).await;
    push_context(&mut f, CONTEXT, 20);
    let context = command(&mut f).await;
    // Act
    f.peer
        .error(&context, "Failed to find context with id context-a")
        .await;
    let parent = next(&mut f).await;
    // Assert
    assert_eq!(parent.payload, 20);
    assert!(parent.result.is_err());
    assert!(poll_once(&mut f).is_pending());
    f.peer.destroy("page-a").await;
    let child = next(&mut f).await;
    assert!(child.result.is_ok());
    drop((parent, child));
    f.finish().await;
}

#[tokio::test]
async fn early_exact_event_survives_until_command_acknowledgement() {
    // Arrange
    let mut f = Fixture::new(1).await;
    push_page(&mut f, 10);
    let page = page_command(&mut f).await;
    f.peer.destroy("page-a").await;
    // Act
    f.peer.reply(&page, json!({"success": true})).await;
    let child = next(&mut f).await;
    // Assert
    assert!(child.result.as_ref().unwrap().validate().is_ok());
    drop(child);
    f.finish().await;
}

#[tokio::test]
async fn native_false_success_never_mints_page_death() {
    // Arrange
    let mut f = Fixture::new(1).await;
    push_page(&mut f, 10);
    let page = page_command(&mut f).await;
    // Act
    f.peer.reply(&page, json!({"success": false})).await;
    let child = next(&mut f).await;
    // Assert
    assert!(child.result.is_err());
    drop(child);
    f.finish().await;
}

#[tokio::test]
async fn returned_completion_holds_admission_until_dropped() {
    // Arrange
    let mut f = Fixture::new(1).await;
    push_context(&mut f, CONTEXT, 20);
    let context = command(&mut f).await;
    f.peer.reply(&context, json!({})).await;
    let parent = next(&mut f).await;
    // Act
    let rejected = f.scheduler.push_context(OTHER_CONTEXT, 30);
    // Assert
    assert!(matches!(rejected, Err((30, error)) if error.kind() == FailureKind::ResourceExhausted));
    drop(parent);
    push_context(&mut f, OTHER_CONTEXT, 30);
    let packet = command(&mut f).await;
    f.peer.reply(&packet, json!({})).await;
    drop(next(&mut f).await);
    f.finish().await;
}

#[tokio::test]
async fn interruption_keeps_written_commands_until_reply_without_success_evidence() {
    // Arrange
    let mut f = Fixture::new(2).await;
    push_page(&mut f, 10);
    let page = page_command(&mut f).await;
    push_context(&mut f, CONTEXT, 20);
    let context = command(&mut f).await;
    // Act
    f.stop.stop();
    // Assert
    assert!(poll_once(&mut f).is_pending());
    assert_eq!(f.scheduler.admission.pending.load(Ordering::Acquire), 2);
    f.peer.reply(&context, json!({})).await;
    let parent = next(&mut f).await;
    assert_eq!(parent.payload, 20);
    assert!(parent.result.is_err());
    f.peer.reply(&page, json!({"success": true})).await;
    let child = next(&mut f).await;
    assert!(child.result.is_err());
    drop((parent, child));
    f.finish().await;
}

#[tokio::test]
async fn peer_loss_drains_both_command_owners() {
    // Arrange
    let mut f = Fixture::new(2).await;
    push_page(&mut f, 10);
    page_command(&mut f).await;
    push_context(&mut f, CONTEXT, 20);
    command(&mut f).await;
    // Act
    f.peer.disconnect().await;
    let a = next(&mut f).await;
    let b = next(&mut f).await;
    // Assert
    assert!(a.result.is_err() && b.result.is_err());
    drop((a, b));
    assert_eq!(f.scheduler.admission.pending.load(Ordering::Acquire), 0);
    f.finish().await;
}

#[tokio::test]
async fn evidence_revalidates_epoch_after_native_completion() {
    // Arrange
    let mut f = Fixture::new(1).await;
    push_page(&mut f, 10);
    acknowledge_page(&mut f).await;
    f.peer.destroy("page-a").await;
    let child = next(&mut f).await;
    f.scheduler
        .authority
        .0
        .topology
        .lock()
        .unwrap()
        .begin_reconnect(ConnectionEpoch::new(2))
        .unwrap();
    // Act
    let validated = child.result.as_ref().unwrap().validate();
    // Assert
    assert!(validated.is_err());
    drop(child);
    f.finish().await;
}

#[tokio::test]
async fn abandoned_unpolled_intents_release_all_capacity_without_commands() {
    // Arrange
    let mut f = Fixture::new(2).await;
    let baseline = f.bytes();
    push_page(&mut f, 10);
    push_context(&mut f, CONTEXT, 20);
    // Act
    for entry in &mut f.scheduler.entries {
        *entry = None;
    }
    // Assert
    assert_eq!(f.bytes(), baseline);
    assert!(f.peer.no_commands());
    assert_eq!(f.scheduler.admission.pending.load(Ordering::Acquire), 0);
    f.finish().await;
}

#[tokio::test]
async fn stale_page_binding_is_rejected_before_any_native_command() {
    // Arrange
    let mut f = Fixture::new(1).await;
    push_page(&mut f, 10);
    f.scheduler
        .authority
        .0
        .topology
        .lock()
        .unwrap()
        .begin_reconnect(ConnectionEpoch::new(2))
        .unwrap();
    // Act
    let completion = poll_once(&mut f);
    // Assert
    let Poll::Ready(Some(child)) = completion else {
        panic!("stale Page must fail before native polling")
    };
    assert!(child.result.is_err());
    assert!(f.peer.no_commands());
    drop(child);
    f.finish().await;
}

#[tokio::test]
async fn byte_failure_returns_payload_and_releases_partial_admission() {
    // Arrange
    let mut f = Fixture::new(1).await;
    let admission = f.scheduler.admission.clone();
    let free = admission
        .ledger
        .limits()
        .class(ResourceClass::BackendStateBytes)
        .global
        - f.bytes();
    let held = admission
        .ledger
        .reserve(admission.scope, ResourceClass::BackendStateBytes, free)
        .unwrap();
    let before = f.bytes();
    // Act
    let rejected = f.scheduler.push_page(PAGE, 10);
    // Assert
    assert!(matches!(rejected, Err((10, error)) if error.kind() == FailureKind::ResourceExhausted));
    assert_eq!(f.bytes(), before);
    assert_eq!(admission.pending.load(Ordering::Acquire), 0);
    assert!(f.peer.no_commands());
    drop((held, admission));
    f.finish().await;
}

#[tokio::test]
async fn terminated_owner_has_priority_over_full_admission() {
    // Arrange
    let mut f = Fixture::new(1).await;
    push_context(&mut f, CONTEXT, 20);
    let context = command(&mut f).await;
    f.stop.stop();
    // Act
    let rejected = f.scheduler.push_page(PAGE, 10);
    // Assert
    assert!(matches!(rejected, Err((10, error)) if error.kind() == FailureKind::TargetClosed));
    f.peer.reply(&context, json!({})).await;
    drop(next(&mut f).await);
    f.finish().await;
}

#[tokio::test]
async fn ancestor_proof_survives_parent_topology_retirement_before_child_reply() {
    // Arrange
    let mut f = Fixture::new(2).await;
    push_page(&mut f, 10);
    let page = page_command(&mut f).await;
    push_context(&mut f, CONTEXT, 20);
    let context = command(&mut f).await;
    f.peer.reply(&context, json!({})).await;
    let parent = next(&mut f).await;
    f.scheduler
        .authority
        .0
        .topology
        .lock()
        .unwrap()
        .close_context(EPOCH, "context-a")
        .unwrap();
    drop(parent);
    // Act
    f.peer.error(&page, "Target closed").await;
    let child = next(&mut f).await;
    // Assert
    assert_eq!(
        child.result.as_ref().unwrap().validate().unwrap(),
        NativeCloseTarget::Page(PAGE)
    );
    drop(child);
    f.finish().await;
}

#[tokio::test]
async fn page_rebinding_after_event_is_rejected_at_commit_validation() {
    // Arrange
    let mut f = Fixture::new(1).await;
    push_page(&mut f, 10);
    acknowledge_page(&mut f).await;
    f.peer.destroy("page-a").await;
    let child = next(&mut f).await;
    {
        let mut topology = f.scheduler.authority.0.topology.lock().unwrap();
        topology
            .detach_target(EPOCH, "session-a", Some("page-a"))
            .unwrap();
        topology
            .attach_page(
                EPOCH,
                pwright_backend_cdp::CdpPageTarget::new("page-a", "other-session", "context-b"),
                pwright_backend_cdp::CdpPageObjectIds::new(ObjectId::new(10), ObjectId::new(11)),
            )
            .unwrap();
    }
    // Act
    let checked = child.result.as_ref().unwrap().validate();
    // Assert
    assert!(checked.is_err());
    drop(child);
    f.finish().await;
}

#[tokio::test]
async fn missing_page_before_first_poll_does_not_send_close_to_an_old_target() {
    // Arrange
    let mut f = Fixture::new(1).await;
    push_page(&mut f, 10);
    f.scheduler
        .authority
        .0
        .topology
        .lock()
        .unwrap()
        .detach_target(EPOCH, "session-a", Some("page-a"))
        .unwrap();
    // Act
    let result = poll_once(&mut f);
    // Assert
    let Poll::Ready(Some(child)) = result else {
        panic!("missing Page must reject before native work")
    };
    assert!(child.result.is_err());
    assert!(f.peer.no_commands());
    drop(child);
    f.finish().await;
}

#[tokio::test]
async fn leaf_cursor_failure_returns_scheduler_slot_and_page_binding() {
    // Arrange
    let mut f = Fixture::new(1).await;
    let cursor = f
        .scheduler
        .authority
        .0
        .connection
        .subscribe_composable_events()
        .unwrap();
    let before = f.bytes();
    // Act
    let rejected = f.scheduler.push_page(PAGE, 10);
    // Assert
    assert!(matches!(rejected, Err((10, error)) if error.kind() == FailureKind::ResourceExhausted));
    assert_eq!(f.scheduler.admission.pending.load(Ordering::Acquire), 0);
    assert_eq!(f.scheduler.authority.0.pending.load(Ordering::Acquire), 0);
    assert_eq!(f.bytes(), before);
    drop(cursor);
    f.finish().await;
}

#[tokio::test]
async fn default_context_is_rejected_before_native_work() {
    // Arrange
    let mut f = Fixture::new(1).await;
    f.scheduler
        .authority
        .0
        .topology
        .lock()
        .unwrap()
        .register_context(
            EPOCH,
            "default",
            ObjectId::new(10),
            CdpContextOwnership::Default,
        )
        .unwrap();
    // Act
    let rejected = f.scheduler.push_context(ObjectId::new(10), 10);
    // Assert
    assert!(
        matches!(rejected, Err((10, error)) if error.kind() == FailureKind::UnsupportedCapability)
    );
    assert!(f.peer.no_commands());
    f.finish().await;
}

#[tokio::test]
async fn epoch_change_during_discovery_prevents_target_close_initiation() {
    // Arrange
    let mut f = Fixture::new(1).await;
    push_page(&mut f, 10);
    let discovery = command(&mut f).await;
    assert_eq!(discovery["method"], "Target.setDiscoverTargets");
    f.scheduler
        .authority
        .0
        .topology
        .lock()
        .unwrap()
        .begin_reconnect(ConnectionEpoch::new(2))
        .unwrap();
    // Act
    f.peer.reply(&discovery, json!({})).await;
    let child = next(&mut f).await;
    // Assert
    assert!(child.result.is_err());
    assert!(f.peer.no_commands());
    drop(child);
    f.finish().await;
}

#[tokio::test]
async fn maximum_identity_binding_fits_scheduler_and_leaf_command_envelopes() {
    // Arrange
    let mut f = Fixture::with_id_bytes(1, 1024).await;
    let target = "p".repeat(1024);
    let page_id = ObjectId::new(11);
    {
        let mut topology = f.scheduler.authority.0.topology.lock().unwrap();
        let context = "c".repeat(1024);
        topology
            .register_context(
                EPOCH,
                &context,
                ObjectId::new(10),
                CdpContextOwnership::Owned,
            )
            .unwrap();
        topology
            .attach_page(
                EPOCH,
                pwright_backend_cdp::CdpPageTarget::new(&target, "s".repeat(1024), &context),
                pwright_backend_cdp::CdpPageObjectIds::new(page_id, ObjectId::new(12)),
            )
            .unwrap();
    }
    assert!(f.scheduler.push_page(page_id, 10).is_ok());
    let page = page_command(&mut f).await;
    assert_eq!(page["params"]["targetId"], target);
    f.peer.destroy(&target).await;
    // Act
    f.peer.reply(&page, json!({"success": true})).await;
    let child = next(&mut f).await;
    // Assert
    assert_eq!(
        child.result.as_ref().unwrap().validate().unwrap(),
        NativeCloseTarget::Page(page_id)
    );
    drop(child);
    f.finish().await;
}
