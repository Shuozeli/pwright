//! Real native marker ownership and DOM events, not the public Node upload API.
use super::*;
use anyhow::{Context as _, Result, ensure};
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpClient, CdpConnection, CdpConnectionProductionProfile, CdpSession};
use pwright_composition_chromium_locator_resolution::{
    CdpLocatorResolutionService, ContinuableResolutionService, ResolutionProfile,
    RetainedElementRequest,
};
use pwright_core::{AtomicObjectIdSource, ResourceLimits};
use pwright_runtime_tokio::TokioCleanupExecutor;
use serde_json::{Value, json};
use std::time::Duration;

const STEP: Duration = Duration::from_secs(10);
const SOURCE_GROUP: &str = "pwright.node182.fixture";
const FIXTURE_SCOPE: ResourceScope = ResourceScope::extension(182, 7);

struct Profiles {
    marker: TargetMarkingProfile,
    state: UploadProfile,
    call: UploadProfile,
    resolution: ResolutionProfile,
}

fn production(needs: ResourceRequirements) -> Result<ProductionResourceLedger> {
    let mut limits = ResourceLimits::deny_all(needs.scopes, needs.leases);
    for class in ResourceClass::ALL {
        let bound = needs.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    Ok(ProductionResourceLedger::new(limits, needs)?)
}

async fn isolated_world(session: &CdpSession) -> Result<(i64, Arc<str>)> {
    let mut events = session.subscribe_events();
    session.runtime_enable().await?;
    let frame = session.page_get_main_frame_id().await?;
    let numeric = session
        .page_create_isolated_world(&frame, "pwright-node182-marker", false)
        .await?;
    loop {
        let event = events.recv().await?;
        if event.session_id.as_deref() == session.session_id()
            && event.method == "Runtime.executionContextCreated"
            && event.params.pointer("/context/id").and_then(Value::as_i64) == Some(numeric)
        {
            let unique = event
                .params
                .pointer("/context/uniqueId")
                .and_then(Value::as_str)
                .context("missing exact utility-world identity")?;
            return Ok((numeric, Arc::from(unique)));
        }
    }
}

const SETUP: &str = r#"(() => {
    document.body.innerHTML = '<div id="A"></div><div id="B"></div><label id="label">text<input id="input" type="file"></label><div id="host"></div>';
    const shadow = document.getElementById('host').attachShadow({mode:'open'});
    shadow.innerHTML = '<label id="shadowLabel">shadow<input type="file"></label>';
    globalThis.__node182Events = [];
    for (const type of ['__playwright_mark_target__', '__playwright_unmark_target__']) {
        document.addEventListener(type, event => {
            event.preventDefault();
            globalThis.__node182Events.push([
                event.type, event.target.id, event.composedPath()[0].id,
                event.detail, event.bubbles, event.cancelable, event.composed, event.defaultPrevented
            ]);
        });
    }
    return true;
})()"#;

async fn exercise(
    connection: Arc<CdpConnection>,
    browser: &CdpSession,
    context: &str,
    resources: &ProductionResourceLedger,
    cleanup: TokioCleanupExecutor,
    profiles: Profiles,
) -> Result<()> {
    let target = browser
        .target_create_in_context("about:blank", context)
        .await?;
    let session_id = browser.target_attach(&target).await?;
    let session = Arc::new(CdpSession::new(connection, session_id, target));
    let (numeric, world) = isolated_world(&session).await?;
    session.runtime_evaluate(SETUP).await?;
    let client: Arc<dyn CdpClient> = session.clone();
    let backend = Arc::new(CdpTargetMarkingService::new(
        profiles.marker,
        resources,
        Arc::new(AtomicObjectIdSource::new(1)?),
    )?);
    let service = TargetMarkingOperationService::new(
        profiles.state,
        profiles.call,
        resources,
        backend,
        cleanup.clone(),
    )?;
    let resolver = Arc::new(CdpLocatorResolutionService::new(
        profiles.resolution,
        resources,
        Arc::new(AtomicObjectIdSource::new(1)?),
    )?);
    let attempts = ContinuableResolutionService::new(resolver.clone(), cleanup);
    let marker = service
        .create(
            MarkerWorld {
                client: client.clone(),
                utility_world: world.clone(),
                destruction: None,
            },
            None,
        )?
        .await?;

    // Arrange/Act: each invocation resolves its exact source then releases its
    // own temporary group. The separately owned marker survives all of them.
    for (node, call_id) in [
        ("A", "call@1"),
        ("A", "call@1"),
        ("B", "call@1"),
        ("A", "call@2"),
        ("label", "call@3"),
        ("text", "call@3"),
        ("B", "call@3"),
        ("shadowLabel", "call@4"),
        ("A", ""),
    ] {
        let response = session.runtime_call_function_as_object_in_group_to_settlement(
            &world,
            "function(id) { if (id === 'text') return document.getElementById('label').firstChild; if (id === 'shadowLabel') return document.getElementById('host').shadowRoot.getElementById(id); return document.getElementById(id); }",
            vec![json!({"value": node})], SOURCE_GROUP, false,
        ).await?;
        let object = response
            .pointer("/result/objectId")
            .and_then(Value::as_str)
            .context("fixture source did not produce a DOM node")?;
        let element = attempts
            .call_retained_retaining(
                RetainedElementRequest {
                    source_client: client.clone(),
                    source_object_id: Arc::from(object),
                    client: client.clone(),
                    utility_world: world.clone(),
                    utility_execution_context_id: numeric,
                    destruction: None,
                },
                None,
            )
            .await?;
        let marked = service.mark(&marker, element, Arc::from(call_id))?.await?;
        marked.release().await?;
        ensure!(
            service.active_states() == 1,
            "operation cleanup released persistent marker state"
        );
        ensure!(
            service.active_calls() == 0,
            "completed temporary cleanup retained a call"
        );
        ensure!(
            service.is_cleanup_healthy() && resolver.is_cleanup_healthy(),
            "cleanup became unhealthy"
        );
    }

    // Assert real DOM event target/detail/flags and cross-call state. A repeated
    // ID unmarks only removed nodes; a different ID never unmarks its old set.
    // Text/empty-ID calls do not alter state. Label marking precedes any future
    // input retargeting, and shadow events preserve composed propagation.
    let events = session
        .runtime_evaluate("globalThis.__node182Events")
        .await?;
    let event = |kind: &str, target: &str, original: &str, call: &str| {
        json!([
            format!("__playwright_{kind}_target__"),
            target,
            original,
            call,
            true,
            true,
            true,
            true
        ])
    };
    ensure!(
        events.pointer("/result/value")
            == Some(&json!([
                event("mark", "A", "A", "call@1"),
                event("unmark", "A", "A", "call@1"),
                event("mark", "B", "B", "call@1"),
                event("mark", "A", "A", "call@2"),
                event("mark", "label", "label", "call@3"),
                event("unmark", "label", "label", "call@3"),
                event("mark", "B", "B", "call@3"),
                event("mark", "host", "shadowLabel", "call@4"),
            ])),
        "unexpected native event sequence: {events}"
    );
    let inputs = session
        .runtime_evaluate("document.getElementById('input').files.length")
        .await?;
    ensure!(
        inputs.pointer("/result/value") == Some(&json!(0)),
        "marker mutated file input"
    );
    marker
        .release()
        .ok()
        .context("persistent marker was unexpectedly busy")?
        .await?;
    ensure!(
        service.active_states() == 0 && service.active_calls() == 0,
        "marker owners survived acknowledged release"
    );
    session
        .runtime_release_object_group_to_settlement(SOURCE_GROUP)
        .await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_TEST_CDP_WS=ws://127.0.0.1:9222/devtools/browser/..."]
async fn real_chrome_marker_persists_across_calls_and_releases_owned_groups() -> Result<()> {
    // Arrange: no browser launch/default-profile access. This connection owns
    // only its fresh dispose-on-detach Context; other targets are untouched.
    let endpoint = std::env::var("PWRIGHT_TEST_CDP_WS")?;
    ensure!(
        endpoint.starts_with("ws://127.0.0.1:9222/devtools/browser/")
            && !endpoint.contains(['?', '#', '@']),
        "dedicated loopback endpoint required"
    );
    let transport = CdpConnectionProductionProfile::new(
        ResourceScope::extension(182, 1),
        ConnectionConfig {
            timeout: Duration::from_secs(5),
            pending_command_capacity: 16,
            event_channel_capacity: 64,
            max_command_bytes: 64 * 1024,
            max_response_bytes: 256 * 1024,
            max_event_bytes: 64 * 1024,
            max_websocket_bytes: 256 * 1024,
            ..ConnectionConfig::default()
        },
    );
    let marker = TargetMarkingProfile::new(ResourceScope::extension(182, 2), 1, 1)?;
    let state = UploadProfile::new(ResourceScope::extension(182, 3), 1, 16 * 1024)?;
    let call = UploadProfile::new(ResourceScope::extension(182, 4), 1, 16 * 1024)?;
    let resolution =
        ResolutionProfile::new(ResourceScope::extension(182, 5), 2)?.with_retained_reference()?;
    let fixture = ResourceRequirements::none()
        .with_class(ResourceClass::RemoteHandle, 16, 16)
        .with_class(ResourceClass::BackendStateBytes, 64 * 1024, 64 * 1024)
        .with_metadata(1, 1);
    let resources = production(
        transport
            .requirements()
            .combine_concurrent(marker.requirements())
            .combine_concurrent(state.requirements())
            .combine_concurrent(call.requirements())
            .combine_concurrent(resolution.requirements())
            .combine_concurrent(fixture),
    )?;
    let connection = tokio::time::timeout(
        STEP,
        CdpConnection::connect_with_production_resources(&endpoint, transport, &resources),
    )
    .await??;
    let browser = CdpSession::browser(connection.clone());
    let cleanup = TokioCleanupExecutor::new(8);
    let owned_context = Arc::new(std::sync::Mutex::new(None));
    // Keep fixture-source storage outside the fallible exercise so a failure
    // cannot return its native allowance before owned Context cleanup settles.
    let fixture_ledger = resources.claim(fixture)?;
    let fixture_lease = fixture_ledger.reserve_many(
        FIXTURE_SCOPE,
        &[
            ResourceRequest::new(ResourceClass::RemoteHandle, 16),
            ResourceRequest::new(ResourceClass::BackendStateBytes, 64 * 1024),
        ],
    )?;

    // Act under a finite observation bound. Timed-out work drops into the real
    // pre-admitted cleanup continuations; timeout itself is never success.
    let task_connection = connection.clone();
    let task_resources = resources.clone();
    let task_cleanup = cleanup.clone();
    let task_context = owned_context.clone();
    let mut body = tokio::spawn(async move {
        let task_browser = CdpSession::browser(task_connection.clone());
        let context = task_browser
            .target_create_browser_context(None, None)
            .await?;
        *task_context
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(context.clone());
        exercise(
            task_connection,
            &task_browser,
            &context,
            &task_resources,
            task_cleanup,
            Profiles {
                marker,
                state,
                call,
                resolution,
            },
        )
        .await
    });
    let outcome = tokio::time::timeout(Duration::from_secs(40), &mut body).await;
    let mut failures = Vec::new();
    match outcome {
        Ok(Ok(Ok(()))) => {}
        Ok(Ok(Err(error))) => failures.push(format!("exercise: {error:#}")),
        Ok(Err(error)) => failures.push(format!("exercise task: {error}")),
        Err(error) => {
            failures.push(format!("exercise timeout: {error}"));
            body.abort();
            if tokio::time::timeout(STEP, &mut body).await.is_err() {
                failures.push("aborted exercise did not retire within cleanup bound".into());
            }
        }
    }
    match tokio::time::timeout(STEP, cleanup.shutdown()).await {
        Ok(report) if report.metrics.failed == 0 && report.metrics.timed_out == 0 => {}
        result => failures.push(format!("cleanup drain: {result:?}")),
    }
    let context = owned_context
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take();
    if let Some(context) = context {
        match tokio::time::timeout(STEP, browser.target_dispose_browser_context(&context)).await {
            Ok(Ok(())) => {}
            result => failures.push(format!("owned Context cleanup: {result:?}")),
        }
    }
    drop(fixture_lease);
    let finished = connection.io_finished();
    connection.request_disconnect();
    drop(browser);
    drop(connection);
    if let Err(error) = tokio::time::timeout(STEP, finished).await {
        failures.push(format!("owned transport cleanup: {error}"));
    }

    // Assert after every cleanup attempt. These counters cover the declared
    // fixture/transport/marker plans, not browser RAM or public upload parity.
    let metrics = resources.metrics()?;
    for class in ResourceClass::ALL {
        if metrics.class(class).current != 0 {
            failures.push(format!(
                "retained {class:?}: {}",
                metrics.class(class).current
            ));
        }
    }
    ensure!(failures.is_empty(), "{}", failures.join("; "));
    Ok(())
}
