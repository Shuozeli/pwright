//! Safe-host production assembly checks, without a Node binding or CDP shortcut.
use super::*;
use crate::locator::HostLocator;
use crate::locator_query::{QueryBudget, QueryLimits};

fn query(host: &Arc<HostSession>, frame: &str, selector: &str) -> Result<HostLocator, HostError> {
    let queries = QueryBudget::new(QueryLimits {
        recipes: 32,
        bytes: 1024 * 1024,
    })?;
    let query = queries
        .prepare_leaf(selector.len())?
        .css(selector.to_owned())?;
    host.locator(frame, query)
}

async fn evaluate(
    host: &HostSession,
    frame: &str,
    source: &str,
) -> Result<EvaluationValue, HostError> {
    host.frame_evaluate(frame, EvaluateParams::expression(source))
        .await
}

fn handles(host: &HostSession) -> u64 {
    host.resources
        .metrics()
        .unwrap()
        .class(ResourceClass::RemoteHandle)
        .current
}

async fn fixture() -> Result<(Arc<HostSession>, String, crate::PageDescriptor), HostError> {
    let endpoint = std::env::var("PWRIGHT_TEST_CDP").expect("explicit dedicated Chrome endpoint");
    let host = Arc::new(HostSession::connect_over_cdp(&endpoint, 5000).await?);
    let context = host.new_context().await?;
    let setup = async {
        let page = host.new_page(&context.id).await?;
        evaluate(&host, &page.main_frame_id,
        "globalThis.mainMarker='main'; document.body.innerHTML='<input id=original value=kept><i></i><i></i>'; document.querySelectorAll=()=>{throw new Error('main-world selector forbidden')}"
        ).await?;
        Ok::<_, HostError>(page)
    }.await;
    match setup {
        Ok(page) => Ok((host, context.id, page)),
        Err(primary) => {
            let context_cleanup = host.close_context(&context.id).await;
            let session_cleanup = host.close().await;
            if context_cleanup.is_err() || session_cleanup.is_err() {
                return Err(HostError::backend(format_args!(
                    "{primary}; fixture cleanup: {context_cleanup:?} / {session_cleanup:?}"
                )));
            }
            Err(primary)
        }
    }
}

async fn cleanup(host: Arc<HostSession>, context: String, outcome: Result<(), HostError>) {
    let context_close = host.close_context(&context).await;
    let session_close = host.close().await;
    // Keep assertion after cleanup: failures must not leave fixture Contexts.
    outcome.expect("host case should pass");
    context_close.expect("owned Context cleanup should pass");
    session_close.expect("session cleanup should pass");
    assert_eq!(host.metrics().unwrap(), crate::SessionMetrics::default());
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_TEST_CDP"]
async fn host_locator_evaluation_owns_prepared_identity_through_delayed_conversion() {
    // Arrange: actual host profile/driver/runtime; no native handle escapes.
    let (host, context, page) = fixture().await.unwrap();
    let outcome = tokio::time::timeout(Duration::from_secs(20), async {
        let locator = query(&host, &page.main_frame_id, "#original")?;
        let baseline = handles(&host);
        let callback_host = host.clone();
        let frame = page.main_frame_id.clone();
        // Act: conversion runs only after adoption. Detach and replace the
        // node, then delay longer than the resolution budget before evaluating.
        let value = locator.evaluate_with(500, move || async move {
            let pending = callback_host.metrics()?.pending_calls;
            if pending != 1 { return Err(HostError::backend("prepared command lost its runtime call")); }
            evaluate(&callback_host, &frame,
                "(()=>{let n=document.getElementById('original'); n.remove();document.body.insertAdjacentHTML('beforeend','<input id=original value=replacement>'); return new Promise(resolve=>setTimeout(resolve,650));})()"
            ).await?;
            Ok(EvaluateParams::function_without_argument("element=>[element.value,element.isConnected,globalThis.mainMarker]"))
        }).await?;
        // Assert: original identity, main world, and both temporary wrappers
        // reclaimed before the successful value leaves the command.
        assert!(matches!(value, EvaluationValue::Array { items, .. }
            if items == vec![EvaluationValue::String("kept".into()),
                EvaluationValue::Bool(false), EvaluationValue::String("main".into())]));
        assert_eq!(handles(&host), baseline);
        assert_eq!(host.metrics()?.pending_calls, 0);
        Ok(())
    }).await.expect("bounded host test");
    cleanup(host, context, outcome).await;
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_TEST_CDP"]
async fn host_locator_failures_skip_conversion_or_release_before_returning_its_error() {
    // Arrange
    let (host, context, page) = fixture().await.unwrap();
    let outcome = tokio::time::timeout(Duration::from_secs(20), async {
        let baseline = handles(&host);
        let started = Arc::new(AtomicUsize::new(0));
        let count = started.clone();
        let missing = query(&host, &page.main_frame_id, "#missing")?;
        // Act / Assert: strict wait's original deadline never calls conversion.
        let missing = missing.evaluate_with(50, move || async move {
            count.fetch_add(1, Ordering::Relaxed);
            Ok(EvaluateParams::expression("1"))
        }).await;
        assert!(matches!(missing, Err(HostError::Timeout(50))));
        assert_eq!(started.load(Ordering::Relaxed), 0);

        // Arrange / Act / Assert: strict failure also precedes conversion.
        let strict = query(&host, &page.main_frame_id, "i")?;
        let count = started.clone();
        let strict = strict.evaluate_with(2000, move || async move {
            count.fetch_add(1, Ordering::Relaxed);
            Ok(EvaluateParams::expression("1"))
        }).await;
        assert!(strict.is_err_and(|error| error.to_string().contains("strict mode violation")));
        assert_eq!(started.load(Ordering::Relaxed), 0);

        // Arrange / Act / Assert: conversion failure is returned only after
        // explicit temporary group cleanup, not left to JS finalization.
        let original = query(&host, &page.main_frame_id, "#original")?;
        let conversion = original.evaluate_with(2000, || async {
            Err(HostError::InvalidArgument("original conversion exception".into()))
        }).await;
        assert!(matches!(conversion, Err(HostError::InvalidArgument(message)) if message == "original conversion exception"));
        assert_eq!(handles(&host), baseline);

        // Arrange / Act / Assert: structured browser exception is preserved.
        let error = original.evaluate_with(2000, || async {
            Ok(EvaluateParams::function_without_argument("element=>{throw new TypeError(element.value)}"))
        }).await;
        assert!(matches!(error, Err(HostError::Javascript { name, message, .. }) if name == "TypeError" && message == "kept"));
        assert_eq!(handles(&host), baseline);
        assert_eq!(host.metrics()?.pending_calls, 0);
        Ok(())
    }).await.expect("bounded host test");
    cleanup(host, context, outcome).await;
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_TEST_CDP"]
async fn host_locator_queue_timeout_shares_existing_runtime_admission() {
    // Arrange: eight real prepared commands hold the host's eight Frame lanes.
    let (host, context, page) = fixture().await.unwrap();
    let outcome = tokio::time::timeout(Duration::from_secs(20), async {
        let baseline = handles(&host);
        let (ready, mut prepared) = tokio::sync::mpsc::channel(EXECUTING);
        let mut completions = Vec::new();
        let mut calls = tokio::task::JoinSet::new();
        for _ in 0..EXECUTING {
            let (complete, resume) = tokio::sync::oneshot::channel();
            completions.push(complete);
            let locator = query(&host, &page.main_frame_id, "#original")?;
            let ready = ready.clone();
            calls.spawn(async move {
                locator
                    .evaluate_with(3000, move || async move {
                        ready.send(()).await.map_err(HostError::backend)?;
                        resume.await.map_err(HostError::backend)?;
                        Ok(EvaluateParams::function_without_argument(
                            "element=>element.value",
                        ))
                    })
                    .await
            });
        }
        for _ in 0..EXECUTING {
            prepared.recv().await.expect("prepared command");
        }
        assert_eq!(host.metrics()?.pending_calls, EXECUTING as u32);
        let started = Arc::new(AtomicBool::new(false));
        let converted = started.clone();
        let queued = query(&host, &page.main_frame_id, "#original")?;
        // Act: ninth command cannot enter the backend before its own deadline.
        let result = queued
            .evaluate_with(30, move || async move {
                converted.store(true, Ordering::Release);
                Ok(EvaluateParams::expression("1"))
            })
            .await;
        // Assert
        assert!(matches!(result, Err(HostError::Timeout(30))));
        assert!(!started.load(Ordering::Acquire));
        assert_eq!(host.metrics()?.pending_calls, EXECUTING as u32);
        for complete in completions {
            complete.send(()).unwrap();
        }
        while let Some(result) = calls.join_next().await {
            assert_eq!(result.unwrap()?, EvaluationValue::String("kept".into()));
        }
        assert_eq!(handles(&host), baseline);
        Ok(())
    })
    .await
    .expect("bounded host queue test");
    cleanup(host, context, outcome).await;
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_TEST_CDP"]
async fn host_caller_cancellation_releases_prepared_element_without_conversion_completion() {
    // Arrange: an explicit owned Context, and a prepared value whose converter
    // remains pending. Use a signal, not a guessed delay before cancellation.
    let (host, context, page) = fixture().await.unwrap();
    let locator = query(&host, &page.main_frame_id, "#original").unwrap();
    let (ready, prepared) = tokio::sync::oneshot::channel();
    let call = tokio::spawn(async move {
        locator
            .evaluate_with(2000, move || async move {
                ready.send(()).unwrap();
                pending::<Result<EvaluateParams, HostError>>().await
            })
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), prepared)
        .await
        .unwrap()
        .unwrap();
    // The session may disconnect without destroying its owned browser Context;
    // close that exact Context explicitly first after cancelling the caller.
    // Act: drop the caller while its exact native group remains reachable.
    call.abort();
    let cancelled = call.await;
    // Assert: no conversion completion is required to drain native cleanup.
    assert!(cancelled.is_err_and(|error| error.is_cancelled()));
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if host.cleanup.metrics().reserved == 0 && host.metrics().unwrap().pending_calls == 0 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("cancelled command cleanup drains");
    cleanup(host, context, Ok(())).await;
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_TEST_CDP"]
async fn host_context_close_cancels_prepared_conversion_and_drains_native_ownership() {
    // Arrange: context closure must still reach a command after preparation.
    let (host, context, page) = fixture().await.unwrap();
    let locator = query(&host, &page.main_frame_id, "#original").unwrap();
    let (ready, prepared) = tokio::sync::oneshot::channel();
    let call = tokio::spawn(async move {
        locator
            .evaluate_with(2000, move || async move {
                ready.send(()).unwrap();
                pending::<Result<EvaluateParams, HostError>>().await
            })
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), prepared)
        .await
        .unwrap()
        .unwrap();
    // Act
    let close = host.close_context(&context).await;
    let result = tokio::time::timeout(Duration::from_secs(5), call).await;
    let shutdown = host.close().await;
    // Assert: target closure is not proof of native cleanup; both are checked.
    close.expect("owned Context close");
    assert!(matches!(result, Ok(Ok(Err(HostError::TargetClosed)))));
    shutdown.expect("authoritative group cleanup after Context close");
    assert_eq!(host.metrics().unwrap(), crate::SessionMetrics::default());
}
