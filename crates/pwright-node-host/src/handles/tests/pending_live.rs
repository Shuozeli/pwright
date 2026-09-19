use super::*;
use crate::evaluation::{EvaluateParams, EvaluationValue};
use pwright_cap_handle_evaluation::EvaluateHandleParams;

async fn wait_for_cleanup(host: &HostSession) -> Result<(), HostError> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if host.handle_metrics()?.current == 0 && host.metrics()?.cleanup_in_flight == 0 {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .map_err(|_| HostError::backend("pending Handle cleanup did not drain"))?
}

async fn wait_for_started(observer: &HostSession, frame: &str) -> Result<(), HostError> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if observer
                .frame_evaluate(
                    frame,
                    EvaluateParams::expression("globalThis.__pendingStarted === true"),
                )
                .await?
                == EvaluationValue::Bool(true)
            {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .map_err(|_| HostError::backend("pending creation did not reach the browser"))?
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_TEST_CDP"]
async fn pending_results_survive_conversion_wait_but_not_session_shutdown() {
    // Arrange: the observer owns the Context; the subject host only borrows it.
    // Closing the subject cannot use Page/Context destruction as cleanup proof.
    let endpoint = std::env::var("PWRIGHT_TEST_CDP").unwrap();
    let observer = HostSession::connect_over_cdp(&endpoint, 5000)
        .await
        .unwrap();
    let context = observer.new_context().await.unwrap();
    let page = observer.new_page(&context.id).await.unwrap();
    observer
        .page_goto(
            &page.id,
            "data:text/html,<title>NODE154-owned-results</title>".into(),
            "load",
            None,
            5000,
        )
        .await
        .unwrap();
    let host = Arc::new(
        HostSession::connect_over_cdp(&endpoint, 5000)
            .await
            .unwrap(),
    );
    let mut frame = None;
    for context in host.contexts().unwrap() {
        for page in host.pages(&context.id).unwrap() {
            if page
                .main_frame_metadata
                .url()
                .unwrap()
                .contains("NODE154-owned-results")
            {
                frame = Some(page.main_frame_id);
            }
        }
    }
    let frame = frame.expect("subject discovers the borrowed Page");
    let outcome: Result<(), HostError> = async {
        // Act / Assert: an unpolled request performs no browser work.
        observer.frame_evaluate(&page.main_frame_id, EvaluateParams::expression("globalThis.__pendingCount = 0")).await?;
        let unpolled = host.frame_prepare_handle(&frame, EvaluateHandleParams::expression("(++globalThis.__pendingCount, {})"));
        drop(unpolled);
        let count = observer.frame_evaluate(&page.main_frame_id, EvaluateParams::expression("globalThis.__pendingCount")).await?;
        check(count == EvaluationValue::number(0.0).unwrap(), "unpolled creation changed the browser")?;

        // A Ready result remains live while the language conversion owner holds
        // it. Dropping that result releases the native object without GC.
        let pending = host.frame_prepare_handle(&frame, EvaluateHandleParams::expression("({answer: 42})")).await?;
        check(host.handle_metrics()?.current == 1, "pending result has no native route")?;
        check(pending.initializer().parent_id.get().to_string() == frame, "result lost Frame parent")?;
        drop(pending);
        wait_for_cleanup(&host).await?;

        // Drop after actual browser submission; an independent observer releases
        // the promise so the pre-admitted continuation can finish and compensate.
        let pending_host = host.clone();
        let pending_frame = frame.clone();
        let creating = tokio::spawn(async move {
            pending_host.frame_prepare_handle(&pending_frame, EvaluateHandleParams::expression(
                "(globalThis.__pendingStarted = true, new Promise(resolve => { globalThis.__releasePending = () => resolve({late: true}); }))",
            )).await
        });
        let started = wait_for_started(&observer, &page.main_frame_id).await;
        creating.abort();
        let aborted = creating.await;
        // Always unblock native evaluation before propagating probe failures.
        observer.frame_evaluate(&page.main_frame_id, EvaluateParams::expression("globalThis.__releasePending?.(); undefined")).await?;
        started?;
        check(aborted.is_err_and(|error| error.is_cancelled()), "creation was not abandoned")?;
        wait_for_cleanup(&host).await?;

        // Retain all 64 Ready results across close. A 65th request must fail
        // before changing the browser, independently of native route capacity.
        let mut pending = Vec::new();
        for _ in 0..MAX_HANDLES {
            pending.push(host.frame_prepare_handle(&frame, EvaluateHandleParams::expression(
                "(++globalThis.__pendingCount, document.createElement('input'))",
            )).await?);
        }
        check(host.handle_metrics()?.current == MAX_HANDLES, "native result capacity was not filled")?;
        let overflow = host.frame_prepare_handle(&frame, EvaluateHandleParams::expression("(++globalThis.__pendingCount, {})")).await;
        check(matches!(overflow, Err(HostError::ResourceExhausted(_))), "result overflow was not rejected")?;
        let count = observer.frame_evaluate(&page.main_frame_id, EvaluateParams::expression("globalThis.__pendingCount")).await?;
        check(count == EvaluationValue::number(MAX_HANDLES as f64).unwrap(), "overflow executed browser code")?;
        tokio::time::timeout(Duration::from_secs(10), host.close()).await
            .map_err(|_| HostError::backend("session close waited for retained result collection"))??;
        check(pending.len() == MAX_HANDLES, "test dropped result owners before close")?;
        let metrics = host.metrics()?;
        eprintln!("Closed host with 64 retained portable results: {metrics:?}");
        check(metrics == crate::SessionMetrics {
            active_leases: MAX_HANDLES as u32 + 1,
            live_scopes: 1,
            ..crate::SessionMetrics::default()
        }, "closed host must retain only the 64 descriptor leases and owner-table state")?;
        let title = observer.frame_title(&page.main_frame_id).await?;
        check(title == "NODE154-owned-results", "subject close destroyed the borrowed Page")?;
        drop(pending);
        check(host.metrics()? == crate::SessionMetrics::default(), "descriptor drop did not return its storage leases")?;
        eprintln!("Pending Handle host: unpolled/drop-started/drop-Ready, 64 retained results, pre-I/O overflow and shutdown without descriptor collection pass; borrowed Page survives");
        Ok(())
    }.await;
    // All terminal paths attempt only this test's connection/Context cleanup.
    let closed = host.close().await;
    let context_closed = observer.close_context(&context.id).await;
    let observer_closed = observer.close().await;
    outcome
        .and(closed)
        .and(context_closed)
        .and(observer_closed)
        .unwrap();
}
