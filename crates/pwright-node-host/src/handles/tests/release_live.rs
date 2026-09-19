use super::*;
use pwright_cap_handle_evaluation::EvaluateHandleParams;

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_TEST_CDP"]
async fn retained_handle_reads_and_explicit_dispose_complete_before_return() {
    use crate::evaluation::EvaluationValue;
    use pwright_cap_handle_evaluation::HandleEvaluateParams;

    // Arrange
    let endpoint = std::env::var("PWRIGHT_TEST_CDP").unwrap();
    let host = HostSession::connect_over_cdp_with_lifecycle_events(&endpoint, 5000)
        .await
        .unwrap();
    let context = host.new_context().await.unwrap();
    let outcome: Result<(), HostError> = async {
        let page = host.new_page(&context.id).await?;
        let handle = host
            .frame_prepare_handle(
                &page.main_frame_id,
                EvaluateHandleParams::expression("({ answer: 42 })"),
            )
            .await?;

        // Act
        let value = host
            .handle_evaluate(
                &handle,
                HandleEvaluateParams::function_without_argument("value => value.answer"),
            )
            .await?;
        let json = host.handle_json_value(&handle).await?;
        let (first, second) = tokio::join!(handle.dispose(), handle.dispose());

        // Assert
        first?;
        second?;
        check(
            value
                == EvaluationValue::Number(
                    crate::evaluation::EvaluationNumber::new(42.0).map_err(HostError::backend)?,
                ),
            "retained evaluation read another value",
        )?;
        check(
            matches!(json, EvaluationValue::Object { .. }),
            "jsonValue did not return an object",
        )?;
        check(
            host.handle_metrics()?.current == 0,
            "dispose returned before native/local retirement",
        )?;
        check(
            matches!(
                host.handle_json_value(&handle).await,
                Err(HostError::InvalidArgument(_))
            ),
            "disposed Handle admitted a value read",
        )?;
        drop(handle);
        Ok(())
    }
    .await;
    let context_closed = host.close_context(&context.id).await;
    let closed = host.close().await;
    outcome.and(context_closed).and(closed).unwrap();
    assert_eq!(host.metrics().unwrap(), crate::SessionMetrics::default());
}

async fn empty(host: &HostSession) -> Result<(), HostError> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if host.handle_metrics()?.current == 0 && host.metrics()?.cleanup_in_flight == 0 {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .map_err(|_| HostError::backend("mixed lifecycle Handle cleanup did not drain"))?
}
async fn batch(host: &HostSession, frame: &str) -> Result<Vec<PendingHandle>, HostError> {
    let mut handles = Vec::new();
    for _ in 0..16 {
        handles.push(
            host.frame_prepare_handle(
                frame,
                EvaluateHandleParams::expression("document.createElement('input')"),
            )
            .await?,
        );
    }
    Ok(handles)
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_TEST_CDP"]
async fn pending_release_interleaves_page_close_navigation_and_context_close() {
    // Arrange: lifecycle-event mode exercises the selected native close driver
    // when enabled, and the ordinary serialized close service otherwise.
    let endpoint = std::env::var("PWRIGHT_TEST_CDP").unwrap();
    let host = HostSession::connect_over_cdp_with_lifecycle_events(&endpoint, 5000)
        .await
        .unwrap();
    let context = host.new_context().await.unwrap();
    let mut context_live = true;
    let outcome: Result<(), HostError> = async {
        let page = host.new_page(&context.id).await?;
        let other = host.new_page(&context.id).await?;
        host.page_goto(&page.id, "data:text/html,<title>NODE157-release</title>".into(), "load", None, 5000).await?;
        let mut held = batch(&host, &page.main_frame_id).await?;

        // Act / Assert: cleanup work and another Page's close share the driver.
        drop(held.split_off(8));
        host.close_page(&other.id).await?;
        host.page_goto(&page.id, "data:text/html,<title>NODE157-next-world</title>".into(), "load", None, 5000).await?;
        drop(held);
        empty(&host).await?;
        check(host.frame_title(&page.main_frame_id).await? == "NODE157-next-world", "surviving Page stopped working")?;

        // Parent retirement can supersede native release. Keep descriptors
        // across session close to prove they do not keep native work alive.
        let mut held = batch(&host, &page.main_frame_id).await?;
        drop(held.split_off(8));
        host.close_context(&context.id).await?;
        context_live = false;
        empty(&host).await?;
        tokio::time::timeout(Duration::from_secs(10), host.close()).await
            .map_err(|_| HostError::backend("close failed to drain parked Handle release"))??;
        check(host.metrics()? == crate::SessionMetrics {
            active_leases: 9, live_scopes: 1, ..crate::SessionMetrics::default()
        }, "only retained descriptor storage should survive close")?;
        drop(held);
        check(host.metrics()? == crate::SessionMetrics::default(), "descriptor storage did not return")?;
        eprintln!("NODE157 host release: unrelated Page close, world replacement, Context retirement and retained-descriptor shutdown pass");
        Ok(())
    }.await;
    let context_closed = if context_live {
        host.close_context(&context.id).await
    } else {
        Ok(())
    };
    let closed = host.close().await;
    outcome.and(context_closed).and(closed).unwrap();
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_TEST_CDP"]
async fn creation_survives_concurrent_unrelated_page_closes_without_repeating_scripts() {
    use crate::evaluation::{EvaluateParams, EvaluationValue};
    // Arrange: all interactions use safe HostSession APIs and an owned Context.
    let endpoint = std::env::var("PWRIGHT_TEST_CDP").unwrap();
    let host = HostSession::connect_over_cdp_with_lifecycle_events(&endpoint, 5000)
        .await
        .unwrap();
    let context = host.new_context().await.unwrap();
    let outcome: Result<(), HostError> = async {
        let page = host.new_page(&context.id).await?;
        host.frame_evaluate(&page.main_frame_id, EvaluateParams::expression("globalThis.__node159Count = 0")).await?;

        // Act: new Handle publication competes with an unrelated Page close.
        for _ in 0..8 {
            let other = host.new_page(&context.id).await?;
            let (handle, closed) = tokio::join!(
                host.frame_prepare_handle(&page.main_frame_id, EvaluateHandleParams::expression("(++globalThis.__node159Count, {answer: 42})")),
                host.close_page(&other.id),
            );
            let handle = handle?;
            closed?;
            drop(handle);
        }

        // Assert: no script replay, resource leak or damage to the surviving Page.
        empty(&host).await?;
        let count = host.frame_evaluate(&page.main_frame_id, EvaluateParams::expression("globalThis.__node159Count === 8")).await?;
        check(count == EvaluationValue::Bool(true), "concurrent close repeated or lost a creation script")?;
        eprintln!("NODE159 host: eight concurrent unrelated closes, eight creation scripts, all Handle cleanup drained");
        Ok(())
    }.await;
    let context_closed = host.close_context(&context.id).await;
    let closed = host.close().await;
    outcome.and(context_closed).and(closed).unwrap();
    assert_eq!(host.metrics().unwrap(), crate::SessionMetrics::default());
}
