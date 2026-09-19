//! Ignored assembled-host marker proof. Native marking uses only the host API;
//! setup and observations use host evaluation/navigation, never raw browser CDP.
//! This is not Node preparation promotion or public ElementHandle upload parity.

use crate::{
    HostError, HostSession,
    evaluation::{EvaluateParams, EvaluationValue},
    handles::{EvaluateHandleParams, PendingHandle},
};
use pwright_core::{ResourceClass, ResourceRequirements, ResourceScope};
use std::{
    future::Future,
    sync::{Arc, Mutex},
    time::Duration,
};

const STEP: Duration = Duration::from_secs(10);
const BODY: Duration = Duration::from_secs(30);
const FIRST_CALL: &str = "call@node183-first";
const NEXT_CALL: &str = "call@node183-next";
type TestResult<T> = Result<T, HostError>;

const SETUP: &str = r#"(() => {
    document.body.innerHTML = '<div id="A"></div><div id="B"></div>';
    globalThis.__node183Events = [];
    for (const type of ['__playwright_mark_target__', '__playwright_unmark_target__']) {
        document.addEventListener(type, event => {
            globalThis.__node183Events.push([
                event.type === '__playwright_mark_target__' ? 'mark' : 'unmark',
                event.target.id, event.detail,
                event.bubbles, event.cancelable, event.composed
            ]);
        });
    }
    return true;
})()"#;

fn check(condition: bool, message: &'static str) -> TestResult<()> {
    if condition {
        Ok(())
    } else {
        Err(HostError::backend(message))
    }
}

async fn bounded<T>(
    name: &'static str,
    work: impl Future<Output = TestResult<T>>,
) -> TestResult<T> {
    tokio::time::timeout(STEP, work)
        .await
        .map_err(|_| HostError::backend(format_args!("{name} exceeded {STEP:?}")))?
}

async fn evaluate(
    host: &HostSession,
    frame: &str,
    expression: &str,
) -> TestResult<EvaluationValue> {
    bounded(
        "host observation",
        host.frame_evaluate(frame, EvaluateParams::expression(expression)),
    )
    .await
}

async fn element(
    host: &HostSession,
    frame: &str,
    expression: &str,
) -> TestResult<Arc<PendingHandle>> {
    let handle = bounded(
        "retained ElementHandle creation",
        host.frame_prepare_handle(frame, EvaluateHandleParams::expression(expression)),
    )
    .await?;
    check(
        handle.is_element(),
        "fixture did not produce an ElementHandle",
    )?;
    Ok(Arc::new(handle))
}

async fn mark(host: &HostSession, handle: &Arc<PendingHandle>, call: &str) -> TestResult<()> {
    // The timeout is a test harness bound, not a new production operation deadline.
    bounded("host target marking", async {
        host.handle_mark_target(handle.clone(), call)
            .await
            .map_err(HostError::backend)
    })
    .await
}

async fn events(host: &HostSession, frame: &str, expected: &str) -> TestResult<()> {
    let actual = evaluate(host, frame, "JSON.stringify(globalThis.__node183Events)").await?;
    if actual == EvaluationValue::String(expected.into()) {
        Ok(())
    } else {
        Err(HostError::backend(format_args!(
            "marker events differ: expected {expected}, received {actual:?}"
        )))
    }
}

fn handle_owner_bytes(host: &HostSession) -> TestResult<u64> {
    Ok(host
        .resources
        .claim(ResourceRequirements::none())
        .map_err(HostError::backend)?
        .scope_metrics(ResourceScope::extension(154, 1))
        .map_err(HostError::backend)?
        .map_or(0, |scope| scope.units(ResourceClass::BackendStateBytes)))
}

async fn one_cached_world_without_active_call(host: &HostSession) -> TestResult<()> {
    bounded("single cached world and retired predecessor", async {
        loop {
            // Test-only accounting is observed under a short host lock, never
            // retained across sleep or a native operation. DOM events above
            // independently establish persistent same-world state reuse.
            let settled = {
                let state = host.state.lock().map_err(HostError::backend)?;
                let live = state.as_ref().ok_or(HostError::TargetClosed)?;
                let installation = &live.file_input.retained.markers;
                let cache = installation.cache.metrics();
                cache.current == 1
                    && cache.creating == 0
                    && cache.retiring == 0
                    && cache.waiters == 0
                    && installation.service.active_states() == 1
                    && installation.service.active_calls() == 0
            };
            if settled {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
}

fn no_retained_handle_work(host: &HostSession, owner_baseline: u64) -> TestResult<()> {
    check(
        handle_owner_bytes(host)? == owner_baseline,
        "persistent marker retained a Handle descriptor or per-call allowance",
    )?;
    let metrics = host.metrics()?;
    check(
        metrics.active_operations == 0 && metrics.pending_calls == 0,
        "completed marker call retained host operation work",
    )
}

async fn exercise(
    host: Arc<HostSession>,
    owned_context: Arc<Mutex<Option<String>>>,
) -> TestResult<()> {
    // Arrange two real persistent ElementHandles in one test-owned Context/Page.
    let context = bounded("owned Context creation", host.new_context()).await?;
    *owned_context.lock().map_err(HostError::backend)? = Some(context.id.clone());
    let page = bounded("owned Page creation", host.new_page(&context.id)).await?;
    check(
        evaluate(&host, &page.main_frame_id, SETUP).await? == EvaluationValue::Bool(true),
        "marker fixture setup failed",
    )?;
    let owner_baseline = handle_owner_bytes(&host)?;
    let a = element(&host, &page.main_frame_id, "document.getElementById('A')").await?;
    let b = element(&host, &page.main_frame_id, "document.getElementById('B')").await?;
    check(
        host.handle_metrics()?.current == 2,
        "fixture Handle ownership differs",
    )?;

    // Act: a single state must survive both completed temporary adoptions and
    // the same-source second call. Recreating state would emit a second mark A.
    mark(&host, &a, FIRST_CALL).await?;
    mark(&host, &a, FIRST_CALL).await?;
    mark(&host, &b, FIRST_CALL).await?;

    // Assert the same-ID policy and real composed/cancelable bubbling events.
    events(
        &host,
        &page.main_frame_id,
        r#"[["mark","A","call@node183-first",true,true,true],["unmark","A","call@node183-first",true,true,true],["mark","B","call@node183-first",true,true,true]]"#,
    )
    .await?;
    check(
        host.handle_metrics()?.current == 2,
        "marking retained an extra persistent Handle",
    )?;
    one_cached_world_without_active_call(&host).await?;

    // A different call ID replaces the remembered set without unmarking B.
    mark(&host, &a, NEXT_CALL).await?;
    events(
        &host,
        &page.main_frame_id,
        r#"[["mark","A","call@node183-first",true,true,true],["unmark","A","call@node183-first",true,true,true],["mark","B","call@node183-first",true,true,true],["mark","A","call@node183-next",true,true,true]]"#,
    )
    .await?;

    // Disposal releases the source wrapper, not the private marker's DOM set.
    // The next same-ID mark must still unmark A using that independent state.
    bounded("source A disposal", a.dispose()).await?;
    // Empty protocol metadata skips marking before native/owner validation.
    mark(&host, &a, "").await?;
    drop(a);
    check(
        host.handle_metrics()?.current == 1,
        "cached marker pinned disposed Handle A",
    )?;
    mark(&host, &b, NEXT_CALL).await?;
    events(
        &host,
        &page.main_frame_id,
        r#"[["mark","A","call@node183-first",true,true,true],["unmark","A","call@node183-first",true,true,true],["mark","B","call@node183-first",true,true,true],["mark","A","call@node183-next",true,true,true],["unmark","A","call@node183-next",true,true,true],["mark","B","call@node183-next",true,true,true]]"#,
    )
    .await?;
    bounded("source B disposal", b.dispose()).await?;
    drop(b);
    check(
        host.handle_metrics()?.current == 0,
        "cached marker pinned disposed Handle B",
    )?;
    one_cached_world_without_active_call(&host).await?;
    no_retained_handle_work(&host, owner_baseline)?;

    // A new document must receive a new exact utility-world marker. The same
    // call ID cannot reuse the previous world's native object or marked set.
    bounded(
        "fresh document navigation",
        host.page_goto(
            &page.id,
            "data:text/html,<title>NODE183-fresh-world</title><body>fresh</body>".into(),
            "load",
            None,
            5000,
        ),
    )
    .await?;
    check(
        evaluate(&host, &page.main_frame_id, SETUP).await? == EvaluationValue::Bool(true),
        "fresh-world fixture setup failed",
    )?;
    let fresh = element(&host, &page.main_frame_id, "document.getElementById('A')").await?;
    mark(&host, &fresh, NEXT_CALL).await?;
    events(
        &host,
        &page.main_frame_id,
        r#"[["mark","A","call@node183-next",true,true,true]]"#,
    )
    .await?;
    bounded("fresh source disposal", fresh.dispose()).await?;
    drop(fresh);
    check(
        host.handle_metrics()?.current == 0,
        "fresh-world marker pinned a disposed persistent Handle",
    )?;
    // The old world must finish native/proof-backed cleanup, not merely lose its
    // current cache key. The successor remains cached until session shutdown.
    one_cached_world_without_active_call(&host).await?;
    no_retained_handle_work(&host, owner_baseline)?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires approved dedicated Chrome preflight and PWRIGHT_TEST_CDP=http://127.0.0.1:9222"]
async fn assembled_marker_reuses_exact_world_state_and_retires_without_pinning_handles()
-> Result<(), String> {
    // Arrange: no discovery fallback or connection to a normal browser profile.
    let endpoint = std::env::var("PWRIGHT_TEST_CDP").map_err(|error| error.to_string())?;
    if endpoint != "http://127.0.0.1:9222" {
        return Err("PWRIGHT_TEST_CDP must be exactly http://127.0.0.1:9222".into());
    }
    let host = Arc::new(
        bounded(
            "host attachment",
            HostSession::connect_over_cdp(&endpoint, 5000),
        )
        .await
        .map_err(|error| error.to_string())?,
    );
    let owned_context = Arc::new(Mutex::new(None));
    let mut body = tokio::spawn(exercise(host.clone(), owned_context.clone()));

    // Act: ordinary errors, panics and total timeout all enter the same cleanup.
    let outcome = tokio::time::timeout(BODY, &mut body).await;
    let mut failures = Vec::new();
    match outcome {
        Ok(Ok(Ok(()))) => {}
        Ok(Ok(Err(error))) => failures.push(format!("exercise: {error}")),
        Ok(Err(error)) => failures.push(format!("exercise task: {error}")),
        Err(_) => {
            failures.push(format!("exercise exceeded {BODY:?}"));
            body.abort();
            if tokio::time::timeout(STEP, &mut body).await.is_err() {
                failures.push("aborted exercise did not finish within cleanup bound".into());
            }
        }
    }

    // Always attempt both owned Context and attachment cleanup. No Browser.close
    // or dedicated-browser restart is used, and the shared browser is preserved.
    let context = owned_context
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take();
    if let Some(context) = context
        && let Err(error) = bounded("owned Context cleanup", host.close_context(&context)).await
    {
        failures.push(format!("Context cleanup: {error}"));
    }
    match tokio::time::timeout(STEP, host.close()).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => failures.push(format!("host cleanup: {error}")),
        Err(_) => {
            failures.push("host cleanup exceeded 10s; possible retained native work owner".into());
            // This is failure containment, not proof of native release. Stop only
            // this attachment's transport and keep the original failure visible.
            let connection = host
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .as_ref()
                .map(|live| live.composition.connection().clone());
            if let Some(connection) = connection {
                connection.request_disconnect();
            }
            if let Err(error) = bounded("host cleanup after disconnect", host.close()).await {
                failures.push(error.to_string());
            }
        }
    }

    // Assert after all cleanup attempts. Ledger zero is local ownership evidence;
    // it is not browser memory measurement or a substitute for native ACKs/proofs.
    match host.metrics() {
        Ok(metrics) if metrics == crate::SessionMetrics::default() => {}
        result => failures.push(format!("terminal host metrics: {result:?}")),
    }
    match host.resources.metrics() {
        Ok(metrics) => {
            for class in ResourceClass::ALL {
                if metrics.class(class).current != 0 {
                    failures.push(format!(
                        "retained {class:?}: {}",
                        metrics.class(class).current
                    ));
                }
            }
            if metrics.active_leases != 0 {
                failures.push(format!("retained leases: {}", metrics.active_leases));
            }
        }
        Err(error) => failures.push(format!("terminal resource metrics: {error}")),
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}
