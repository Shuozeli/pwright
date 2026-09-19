use super::*;

async fn context_receipt(f: &mut Fixture) -> NativeCloseCompletion<u64> {
    push_context(f, CONTEXT, 20);
    let packet = command(f).await;
    f.peer.reply(&packet, json!({})).await;
    next(f).await
}
async fn page_receipt(f: &mut Fixture) -> NativeCloseCompletion<u64> {
    push_page(f, 10);
    acknowledge_page(f).await;
    f.peer.destroy("page-a").await;
    next(f).await
}

#[tokio::test]
async fn context_proof_commits_latest_topology_under_one_exact_lock() {
    // Arrange
    let mut f = Fixture::new(1).await;
    let completion = context_receipt(&mut f).await;
    let state = &f.scheduler.authority.0;
    state
        .topology
        .lock()
        .unwrap()
        .register_context(
            EPOCH,
            "late-sibling",
            ObjectId::new(9),
            CdpContextOwnership::Owned,
        )
        .unwrap();
    let evidence = completion.result().as_ref().unwrap();
    // Act
    let result = evidence.with_topology(&state.connection, &state.topology, |target, topology| {
        assert!(state.topology.try_lock().is_err());
        assert_eq!(
            topology.topology().context_object_id("late-sibling"),
            Some(ObjectId::new(9))
        );
        let NativeCloseCommitTarget::Context {
            object,
            browser_context_id,
        } = target
        else {
            panic!("Context proof");
        };
        assert_eq!(object, CONTEXT);
        assert_eq!(browser_context_id, "context-a");
        topology.close_context(EPOCH, browser_context_id)
    });
    // Assert
    result.unwrap().unwrap();
    assert!(state.topology.try_lock().is_ok());
    assert_eq!(
        state
            .topology
            .lock()
            .unwrap()
            .topology()
            .context_object_id("context-a"),
        None
    );
    assert_eq!(
        state
            .topology
            .lock()
            .unwrap()
            .topology()
            .context_object_id("late-sibling"),
        Some(ObjectId::new(9))
    );
    assert!(f.peer.no_commands());
    drop(completion);
    f.finish().await;
}

#[tokio::test]
async fn page_destruction_proof_drives_the_existing_prepared_topology_commit() {
    // Arrange
    let mut f = Fixture::new(1).await;
    let completion = page_receipt(&mut f).await;
    let state = &f.scheduler.authority.0;
    // Act
    let result = completion.result().as_ref().unwrap().with_topology(
        &state.connection,
        &state.topology,
        |target, topology| {
            let NativeCloseCommitTarget::Page {
                object, target_id, ..
            } = target
            else {
                panic!("Page proof");
            };
            assert_eq!(target_id, "page-a");
            let prepared = topology.prepare_page_close(object).unwrap();
            prepared.commit(topology)
        },
    );
    // Assert
    result.unwrap().unwrap();
    assert_eq!(
        state
            .topology
            .lock()
            .unwrap()
            .topology()
            .page_object_id("page-a"),
        None
    );
    assert_eq!(
        state
            .topology
            .lock()
            .unwrap()
            .topology()
            .context_object_id("context-a"),
        Some(CONTEXT)
    );
    assert!(f.peer.no_commands());
    drop(completion);
    f.finish().await;
}

#[tokio::test]
async fn foreign_connection_with_identical_ids_cannot_enter_commit() {
    // Arrange
    let mut f = Fixture::new(1).await;
    let other = Fixture::new(1).await;
    let completion = context_receipt(&mut f).await;
    let state = &f.scheduler.authority.0;
    // Act
    let result = completion.result().as_ref().unwrap().with_topology(
        &other.scheduler.authority.0.connection,
        &state.topology,
        |_, _| panic!("foreign callback"),
    );
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::ProtocolViolation);
    drop(completion);
    f.finish().await;
    other.finish().await;
}

#[tokio::test]
async fn foreign_topology_with_identical_ids_cannot_enter_commit() {
    // Arrange
    let mut f = Fixture::new(1).await;
    let other = Fixture::new(1).await;
    let completion = context_receipt(&mut f).await;
    let state = &f.scheduler.authority.0;
    // Act
    let result = completion.result().as_ref().unwrap().with_topology(
        &state.connection,
        &other.scheduler.authority.0.topology,
        |_, _| panic!("foreign callback"),
    );
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::ProtocolViolation);
    drop(completion);
    f.finish().await;
    other.finish().await;
}

#[tokio::test]
async fn changed_epoch_is_rejected_before_callback() {
    // Arrange
    let mut f = Fixture::new(1).await;
    let completion = context_receipt(&mut f).await;
    let state = &f.scheduler.authority.0;
    state
        .topology
        .lock()
        .unwrap()
        .begin_reconnect(ConnectionEpoch::new(2))
        .unwrap();
    // Act
    let result = completion.result().as_ref().unwrap().with_topology(
        &state.connection,
        &state.topology,
        |_, _| panic!("stale callback"),
    );
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::TargetClosed);
    drop(completion);
    f.finish().await;
}

#[tokio::test]
async fn context_native_id_rebinding_is_rejected_before_callback() {
    // Arrange
    let mut f = Fixture::new(1).await;
    let completion = context_receipt(&mut f).await;
    let state = &f.scheduler.authority.0;
    {
        let mut topology = state.topology.lock().unwrap();
        topology.close_context(EPOCH, "context-a").unwrap();
        topology
            .register_context(
                EPOCH,
                "context-a",
                ObjectId::new(9),
                CdpContextOwnership::Owned,
            )
            .unwrap();
    }
    // Act
    let result = completion.result().as_ref().unwrap().with_topology(
        &state.connection,
        &state.topology,
        |_, _| panic!("rebound callback"),
    );
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::TargetClosed);
    drop(completion);
    f.finish().await;
}

#[tokio::test]
async fn page_native_id_rebinding_is_rejected_before_callback() {
    // Arrange
    let mut f = Fixture::new(1).await;
    let completion = page_receipt(&mut f).await;
    let state = &f.scheduler.authority.0;
    {
        let mut topology = state.topology.lock().unwrap();
        topology
            .detach_target(EPOCH, "session-a", Some("page-a"))
            .unwrap();
        topology
            .attach_page(
                EPOCH,
                pwright_backend_cdp::CdpPageTarget::new("page-a", "rebound-session", "context-b"),
                pwright_backend_cdp::CdpPageObjectIds::new(ObjectId::new(10), ObjectId::new(11)),
            )
            .unwrap();
    }
    // Act
    let result = completion.result().as_ref().unwrap().with_topology(
        &state.connection,
        &state.topology,
        |_, _| panic!("rebound callback"),
    );
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::TargetClosed);
    drop(completion);
    f.finish().await;
}

#[tokio::test]
async fn parent_first_commit_retains_valid_child_native_identity_for_local_join() {
    // Arrange
    let mut f = Fixture::new(2).await;
    push_page(&mut f, 10);
    let page = page_command(&mut f).await;
    let parent = context_receipt(&mut f).await;
    {
        let state = &f.scheduler.authority.0;
        parent
            .result()
            .as_ref()
            .unwrap()
            .with_topology(&state.connection, &state.topology, |target, topology| {
                let NativeCloseCommitTarget::Context {
                    browser_context_id, ..
                } = target
                else {
                    panic!("Context proof");
                };
                topology.close_context(EPOCH, browser_context_id).unwrap();
            })
            .unwrap();
    }
    drop(parent);
    f.peer.error(&page, "Target closed").await;
    let child = next(&mut f).await;
    let state = &f.scheduler.authority.0;
    // Act
    let result = child.result().as_ref().unwrap().with_topology(
        &state.connection,
        &state.topology,
        |target, topology| {
            assert_eq!(
                target,
                NativeCloseCommitTarget::Page {
                    object: PAGE,
                    target_id: "page-a",
                    session_id: "session-a",
                }
            );
            assert_eq!(topology.topology().page_object_id("page-a"), None);
            assert_eq!(topology.topology().context_object_id("context-a"), None);
        },
    );
    // Assert
    result.unwrap();
    assert!(state.topology.try_lock().is_ok());
    drop(child);
    f.finish().await;
}

#[tokio::test]
async fn stop_while_waiting_for_topology_lock_rejects_commit() {
    // Arrange
    let mut f = Fixture::new(1).await;
    let completion = context_receipt(&mut f).await;
    let state = &f.scheduler.authority.0;
    let barrier = std::sync::Barrier::new(2);
    let guard = state.topology.lock().unwrap();
    let evidence = completion.result().as_ref().unwrap();
    // Act
    let result = std::thread::scope(|scope| {
        let waiting = scope.spawn(|| {
            barrier.wait();
            evidence.with_topology(&state.connection, &state.topology, |_, _| {
                panic!("stopped callback")
            })
        });
        barrier.wait();
        f.stop.stop();
        drop(guard);
        waiting.join().unwrap()
    });
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::TargetClosed);
    drop(completion);
    f.finish().await;
}

#[tokio::test]
async fn callback_result_is_returned_without_releasing_completion_admission() {
    // Arrange
    let mut f = Fixture::new(1).await;
    let completion = context_receipt(&mut f).await;
    let state = &f.scheduler.authority.0;
    let bytes = f.bytes();
    // Act
    let result = completion.result().as_ref().unwrap().with_topology(
        &state.connection,
        &state.topology,
        |_, _| Err::<(), _>("local preflight failed"),
    );
    // Assert
    assert_eq!(result.unwrap(), Err("local preflight failed"));
    assert_eq!(f.bytes(), bytes);
    assert!(state.topology.try_lock().is_ok());
    assert_eq!(
        state
            .topology
            .lock()
            .unwrap()
            .topology()
            .context_object_id("context-a"),
        Some(CONTEXT)
    );
    assert!(matches!(
        f.scheduler.push_context(OTHER_CONTEXT, 99),
        Err((99, _))
    ));
    drop(completion);
    f.finish().await;
}

#[tokio::test]
async fn terminated_native_connection_rejects_before_callback() {
    // Arrange
    let mut f = Fixture::new(1).await;
    let completion = context_receipt(&mut f).await;
    f.peer.disconnect().await;
    let state = &f.scheduler.authority.0;
    state.connection.io_finished().await;
    // Act
    let result = completion.result().as_ref().unwrap().with_topology(
        &state.connection,
        &state.topology,
        |_, _| panic!("disconnected callback"),
    );
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::TargetClosed);
    drop(completion);
    f.finish().await;
}

#[tokio::test]
async fn poisoned_topology_rejects_before_callback() {
    // Arrange
    let mut f = Fixture::new(1).await;
    let completion = context_receipt(&mut f).await;
    let state = &f.scheduler.authority.0;
    let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = state.topology.lock().unwrap();
        panic!("test topology poison");
    }));
    assert!(poisoned.is_err());
    // Act
    let result = completion.result().as_ref().unwrap().with_topology(
        &state.connection,
        &state.topology,
        |_, _| panic!("poisoned callback"),
    );
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::Backend);
    drop(completion);
    f.finish().await;
}
