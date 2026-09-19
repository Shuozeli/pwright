use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[tokio::test]
async fn consuming_move_only_payload_borrows_valid_proof_under_original_admission() {
    // Arrange
    let mut f = wire::GenericFixture::<String>::new(1).await;
    let payload = String::from("caller-owned heap");
    let pointer = payload.as_ptr();
    assert!(f.scheduler.push_context(CONTEXT, payload).is_ok());
    let packet = command(&mut f).await;
    f.peer.reply(&packet, json!({})).await;
    let completion = next(&mut f).await;
    let before = f.bytes();
    // Act
    let returned = completion.consume(|payload, result| {
        assert_eq!(payload.as_ptr(), pointer);
        assert_eq!(f.scheduler.admission.pending.load(Ordering::Acquire), 1);
        assert_eq!(f.bytes(), before);
        let proof = result.as_ref().unwrap();
        assert_eq!(
            proof.validate().unwrap(),
            NativeCloseTarget::Context(CONTEXT)
        );
        proof
            .with_topology(
                &f.scheduler.authority.0.connection,
                &f.scheduler.authority.0.topology,
                |target, topology| {
                    assert_eq!(
                        target,
                        NativeCloseCommitTarget::Context {
                            object: CONTEXT,
                            browser_context_id: "context-a",
                        }
                    );
                    assert_eq!(topology.context_object_id("context-a"), Some(CONTEXT));
                    payload
                },
            )
            .unwrap()
    });
    // Assert
    assert_eq!(returned.as_ptr(), pointer);
    assert_eq!(f.scheduler.admission.pending.load(Ordering::Acquire), 0);
    assert!(f.bytes() < before);
    drop(returned);
    f.finish().await;
}

#[tokio::test]
async fn native_error_returns_payload_without_releasing_capacity_during_callback() {
    // Arrange
    let mut f = wire::GenericFixture::<String>::new(1).await;
    assert!(
        f.scheduler
            .push_context(CONTEXT, String::from("retry state"))
            .is_ok()
    );
    let packet = command(&mut f).await;
    f.peer.error(&packet, "native close rejected").await;
    let completion = next(&mut f).await;
    // Act
    let returned = completion.consume(|payload, result| {
        assert!(result.is_err());
        assert_eq!(f.scheduler.admission.pending.load(Ordering::Acquire), 1);
        payload
    });
    // Assert
    assert_eq!(returned, "retry state");
    assert_eq!(f.scheduler.admission.pending.load(Ordering::Acquire), 0);
    assert!(f.peer.no_commands());
    f.finish().await;
}

#[tokio::test]
async fn callback_panic_drops_payload_evidence_and_original_slot() {
    // Arrange
    let mut f = wire::GenericFixture::<Arc<()>>::new(1).await;
    let payload = Arc::new(());
    assert!(f.scheduler.push_context(CONTEXT, payload.clone()).is_ok());
    let packet = command(&mut f).await;
    f.peer.reply(&packet, json!({})).await;
    let completion = next(&mut f).await;
    // Act
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        completion.consume(|_payload, proof| {
            assert!(proof.is_ok());
            assert_eq!(f.scheduler.admission.pending.load(Ordering::Acquire), 1);
            panic!("caller callback failed");
        });
    }));
    // Assert
    assert!(outcome.is_err());
    assert_eq!(Arc::strong_count(&payload), 1);
    assert_eq!(f.scheduler.admission.pending.load(Ordering::Acquire), 0);
    f.finish().await;
}

#[tokio::test]
async fn consuming_parent_keeps_ancestor_admission_until_joined_child_finishes() {
    // Arrange
    let mut f = Fixture::new(2).await;
    push_page(&mut f, 10);
    acknowledge_page(&mut f).await;
    push_context(&mut f, CONTEXT, 20);
    let packet = command(&mut f).await;
    f.peer.reply(&packet, json!({})).await;
    let first = next(&mut f).await;
    let second = next(&mut f).await;
    let (page, context) = if *first.payload() == 10 {
        (first, second)
    } else {
        (second, first)
    };
    // Act
    context.consume(|payload, proof| {
        assert_eq!(payload, 20);
        assert_eq!(
            proof.as_ref().unwrap().validate().unwrap(),
            NativeCloseTarget::Context(CONTEXT)
        );
    });
    // Assert
    assert_eq!(f.scheduler.admission.pending.load(Ordering::Acquire), 2);
    page.consume(|payload, proof| {
        assert_eq!(payload, 10);
        assert_eq!(
            proof.as_ref().unwrap().validate().unwrap(),
            NativeCloseTarget::Page(PAGE)
        );
        assert_eq!(f.scheduler.admission.pending.load(Ordering::Acquire), 2);
    });
    assert_eq!(f.scheduler.admission.pending.load(Ordering::Acquire), 0);
    f.finish().await;
}

#[tokio::test]
async fn returned_callback_error_releases_admission_without_claiming_retirement() {
    // Arrange
    let mut f = Fixture::new(1).await;
    push_context(&mut f, CONTEXT, 20);
    let packet = command(&mut f).await;
    f.peer.reply(&packet, json!({})).await;
    let completion = next(&mut f).await;
    // Act
    let outcome: Result<(), u64> = completion.consume(|payload, proof| {
        assert!(proof.is_ok());
        Err(payload)
    });
    // Assert
    assert_eq!(outcome, Err(20));
    assert_eq!(f.scheduler.admission.pending.load(Ordering::Acquire), 0);
    assert_eq!(
        f.scheduler
            .authority
            .0
            .topology
            .lock()
            .unwrap()
            .context_object_id("context-a"),
        Some(CONTEXT)
    );
    f.finish().await;
}
