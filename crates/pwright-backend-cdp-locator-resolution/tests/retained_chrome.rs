#![cfg(feature = "retained-reference")]

//! Ignored native boundary check. Running requires explicit approval/preflight
//! for the dedicated Chrome profile; this test never starts or closes Chrome.

use std::{
    error::Error,
    future::Future,
    sync::{Arc, Mutex},
    time::Duration,
};

use pwright_backend_cdp_locator_resolution::{
    CdpLocatorResolutionService, NATIVE_ID_BYTES, ResolutionError, ResolutionProfile,
    RetainedElementRequest,
};
use pwright_cdp::{
    CdpClient, CdpEvent, CdpSession,
    connection::{CdpConnection, CdpError, ConnectionConfig},
};
use pwright_core::{
    AtomicObjectIdSource, ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope,
    Service,
};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type TestResult<T> = Result<T, Box<dyn Error + Send + Sync>>;
const STEP: Duration = Duration::from_secs(5);
const SOURCE_GROUP: &str = "pwright.retained.chrome.source";

#[derive(Default)]
struct OwnedNative {
    target: Option<String>,
    page: Option<Arc<CdpSession>>,
    source: Option<String>,
    source_group_started: bool,
}

// The existing Tokio dependency suffices for this fixed, bounded loopback HTTP
// discovery. No HTTP/provider dependency is added to the backend's closure.
async fn websocket_endpoint() -> TestResult<String> {
    let endpoint = std::env::var("PWRIGHT_TEST_CDP")?;
    if endpoint != "http://127.0.0.1:9222" {
        return Err("PWRIGHT_TEST_CDP must be exactly http://127.0.0.1:9222".into());
    }
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", 9222)).await?;
    stream
        .write_all(
            b"GET /json/version HTTP/1.1\r\nHost: 127.0.0.1:9222\r\nConnection: close\r\n\r\n",
        )
        .await?;
    let mut response = Vec::with_capacity(16 * 1024 + 1);
    stream
        .take(16 * 1024 + 1)
        .read_to_end(&mut response)
        .await?;
    if response.len() > 16 * 1024 {
        return Err("Chrome discovery response exceeds 16 KiB".into());
    }
    let boundary = response
        .windows(4)
        .position(|part| part == b"\r\n\r\n")
        .ok_or("Chrome discovery response has no HTTP headers")?;
    let headers = std::str::from_utf8(&response[..boundary])?;
    if headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        != Some("200")
    {
        return Err("Chrome discovery did not return HTTP 200".into());
    }
    let version: Value = serde_json::from_slice(&response[boundary + 4..])?;
    let websocket = version["webSocketDebuggerUrl"]
        .as_str()
        .ok_or("missing browser WebSocket")?;
    let identity = websocket
        .strip_prefix("ws://127.0.0.1:9222/devtools/browser/")
        .ok_or("Chrome discovery returned a non-dedicated WebSocket")?;
    if identity.is_empty()
        || identity.len() > 128
        || !identity
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err("invalid browser WebSocket identity".into());
    }
    Ok(websocket.to_owned())
}

fn resources() -> TestResult<(CdpLocatorResolutionService, ProductionResourceLedger)> {
    let profile =
        ResolutionProfile::new(ResourceScope::extension(168, 9), 2)?.with_retained_reference()?;
    let required = profile.requirements();
    let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
    for class in ResourceClass::ALL {
        let bound = required.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    let resources = ProductionResourceLedger::new(limits, required)?;
    let service = CdpLocatorResolutionService::new(
        profile,
        &resources,
        Arc::new(AtomicObjectIdSource::new(1)?),
    )?;
    Ok((service, resources))
}

async fn unique_world(
    events: &mut tokio::sync::broadcast::Receiver<CdpEvent>,
    numeric: i64,
) -> TestResult<Arc<str>> {
    tokio::time::timeout(STEP, async {
        loop {
            let event = events.recv().await?;
            if event.method == "Runtime.executionContextCreated"
                && event.params["context"]["id"].as_i64() == Some(numeric)
            {
                let unique = event.params["context"]["uniqueId"]
                    .as_str()
                    .filter(|id| !id.is_empty() && id.len() <= NATIVE_ID_BYTES)
                    .ok_or("missing bounded unique context identity")?;
                return Ok(Arc::from(unique));
            }
        }
    })
    .await?
}

fn assert_no_leases(resources: &ProductionResourceLedger) {
    let metrics = resources.metrics().unwrap();
    assert_eq!(metrics.active_leases, 0);
    for class in ResourceClass::ALL {
        assert_eq!(metrics.class(class).current, 0, "{class:?}");
    }
    assert_eq!(metrics.class(ResourceClass::SelectorBytes).peak, 0);
}

async fn assert_source_usable(page: &CdpSession, source: &str) -> TestResult<()> {
    let value = page.runtime_call_function_on(
        source,
        "function() { return { id: this.id, tag: this.tagName, connected: this.isConnected }; }",
        Vec::new(),
    ).await?;
    assert_eq!(
        value["result"]["value"],
        json!({
            "id":"retained-chrome-source", "tag":"INPUT", "connected":false
        })
    );
    Ok(())
}

async fn exercise(
    connection: Arc<CdpConnection>,
    context: String,
    owned: Arc<Mutex<OwnedNative>>,
    service: CdpLocatorResolutionService,
    resources: ProductionResourceLedger,
) -> TestResult<()> {
    // Arrange a detached exact node and two real utility worlds, no selector.
    let browser = CdpSession::browser(connection.clone());
    let target = browser
        .target_create_in_context("about:blank", &context)
        .await?;
    owned.lock().unwrap().target = Some(target.clone());
    let session = browser.target_attach(&target).await?;
    let page = Arc::new(CdpSession::new(connection, session, target));
    owned.lock().unwrap().page = Some(page.clone());
    let mut events = page.subscribe_events();
    page.runtime_enable().await?;
    let frame = page.page_get_frame_tree().await?.frame.id;
    let numeric_a = page
        .page_create_isolated_world(&frame, "__pwright_retained_a__", false)
        .await?;
    let unique_a = unique_world(&mut events, numeric_a).await?;
    let numeric_b = page
        .page_create_isolated_world(&frame, "__pwright_retained_b__", false)
        .await?;
    let unique_b = unique_world(&mut events, numeric_b).await?;
    assert_ne!(numeric_a, numeric_b);
    assert_ne!(unique_a, unique_b);
    owned.lock().unwrap().source_group_started = true;
    let source_response = page.send_to_settlement("Runtime.evaluate", json!({
        "expression":"(() => { const node = document.createElement('input'); node.id = 'retained-chrome-source'; document.body.append(node); node.remove(); return node; })()",
        "uniqueContextId":unique_a.as_ref(), "objectGroup":SOURCE_GROUP,
        "returnByValue":false, "awaitPromise":true
    })).await?;
    let source = source_response["result"]["objectId"]
        .as_str()
        .filter(|id| !id.is_empty() && id.len() <= NATIVE_ID_BYTES)
        .ok_or("missing bounded source object ID")?
        .to_owned();
    owned.lock().unwrap().source = Some(source.clone());
    drop(source_response);
    assert_source_usable(&page, &source).await?;
    let client: Arc<dyn CdpClient> = page.clone();
    let request = |unique| RetainedElementRequest {
        source_client: client.clone(),
        source_object_id: Arc::from(source.as_str()),
        client: client.clone(),
        utility_world: unique,
        utility_execution_context_id: numeric_a,
        destruction: None,
    };

    // Act: valid numeric+unique pairing adopts the exact node into its own group.
    let owner = service.call(request(unique_a)).await?;
    let adopted = owner.object_id().to_owned();
    assert_ne!(adopted, source);
    assert_source_usable(&page, &adopted).await?;
    owner.release().await?;

    // Assert actual native retirement separately from local lease reclamation.
    let released = page
        .runtime_call_function_on(&adopted, "function() { return true; }", Vec::new())
        .await;
    assert!(
        matches!(&released, Err(CdpError::Protocol { code: -32000, message })
        if message.contains("Could not find object with given id")),
        "temporary still usable: {released:?}"
    );
    assert_source_usable(&page, &source).await?;
    assert_no_leases(&resources);
    assert!(service.is_cleanup_healthy());

    // Act: same actual numeric world A but actual unique world B must reject.
    let mismatch = service.call(request(unique_b)).await;

    // Assert a primary CDP world error, not Cleanup: the retained entrypoint
    // returns this primary only AFTER its group-release ACK. Zero Rust leases
    // is an additional local check, not evidence of native release by itself.
    assert!(
        matches!(
            &mismatch,
            Err(ResolutionError::Backend(CdpError::Protocol { .. }))
        ),
        "mismatched real worlds were not rejected with acknowledged group cleanup: {mismatch:?}"
    );
    assert!(service.is_cleanup_healthy());
    assert_no_leases(&resources);
    assert_source_usable(&page, &source).await?;
    Ok(())
}

async fn cleanup_step(
    name: &'static str,
    failures: &mut Vec<String>,
    operation: impl Future<Output = Result<(), CdpError>>,
) {
    match tokio::time::timeout(STEP, operation).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => failures.push(format!("{name}: {error}")),
        Err(_) => failures.push(format!("{name}: timeout")),
    }
}

#[tokio::test]
#[ignore = "requires approved dedicated Chrome preflight and PWRIGHT_TEST_CDP=http://127.0.0.1:9222"]
async fn retained_reference_uses_exact_world_and_acknowledged_group_cleanup() -> TestResult<()> {
    // Arrange resources before opening native test-owned objects.
    let (service, resources) = resources()?;
    let websocket = tokio::time::timeout(STEP, websocket_endpoint()).await??;
    let connection = tokio::time::timeout(
        STEP,
        CdpConnection::connect_with_config(
            &websocket,
            ConnectionConfig {
                timeout: STEP,
                write_channel_capacity: 8,
                event_channel_capacity: 32,
                pending_command_capacity: 8,
                max_command_bytes: 64 * 1024,
                max_response_bytes: 256 * 1024,
                max_event_bytes: 64 * 1024,
                max_websocket_bytes: 256 * 1024,
                max_protocol_error_bytes: 2048,
            },
        ),
    )
    .await??;
    let browser = CdpSession::browser(connection.clone());
    let context_result =
        tokio::time::timeout(STEP, browser.target_create_browser_context(None, None)).await;
    let context = match context_result {
        Ok(Ok(context)) => context,
        outcome => {
            // createBrowserContext uses disposeOnDetach, including a late
            // reply whose identity this test never received.
            let io_finished = connection.io_finished();
            connection.request_disconnect();
            let retirement = tokio::time::timeout(STEP, io_finished).await;
            let primary = format!("could not create test-owned Context: {outcome:?}");
            return Err(match retirement {
                Ok(()) => primary.into(),
                Err(_) => format!(
                    "{primary}; test-owned CDP connection transport retirement timed out after {STEP:?}"
                )
                .into(),
            });
        }
    };
    let owned = Arc::new(Mutex::new(OwnedNative::default()));
    let mut work = tokio::spawn(exercise(
        connection.clone(),
        context.clone(),
        owned.clone(),
        service,
        resources.clone(),
    ));

    // Act with a total exercise bound. Panics are captured by JoinHandle; an
    // expired raw operation is aborted and joined before outer native cleanup.
    let outcome = tokio::time::timeout(Duration::from_secs(30), &mut work).await;
    if outcome.is_err() {
        work.abort();
        let _ = work.await;
    }

    // Always clean only this test's source/group/Page/Context. Unknown or late
    // wrappers on failure are finally covered by disposing the owned Context.
    let owned = std::mem::take(&mut *owned.lock().unwrap());
    let mut failures = Vec::with_capacity(7);
    if let Some(page) = &owned.page {
        if let Some(source) = &owned.source {
            cleanup_step(
                "source release",
                &mut failures,
                page.runtime_release_object_to_settlement(source),
            )
            .await;
        }
        if owned.source_group_started {
            cleanup_step(
                "source group release",
                &mut failures,
                page.runtime_release_object_group_to_settlement(SOURCE_GROUP),
            )
            .await;
        }
    }
    if let Some(target) = &owned.target {
        cleanup_step(
            "owned Page close",
            &mut failures,
            browser.target_close(target),
        )
        .await;
    }
    cleanup_step(
        "owned Context dispose",
        &mut failures,
        browser.target_dispose_browser_context(&context),
    )
    .await;
    cleanup_step("shared Chrome liveness", &mut failures, async {
        connection
            .send_command("Browser.getVersion", json!({}))
            .await
            .map(|_| ())
    })
    .await;
    let io_finished = connection.io_finished();
    connection.request_disconnect();
    if tokio::time::timeout(STEP, io_finished).await.is_err() {
        failures.push("test-owned CDP connection did not stop".into());
    }

    // Assert after all cleanup attempts, even when the spawned exercise failed.
    assert_no_leases(&resources);
    assert!(
        failures.is_empty(),
        "owned cleanup failed: {failures:?}; exercise: {outcome:?}"
    );
    assert!(
        matches!(outcome, Ok(Ok(Ok(())))),
        "retained native exercise failed or timed out: {outcome:?}"
    );
    Ok(())
}
