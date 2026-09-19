//! Raw native diagnostic: no host, navigation service, runtime or Node addon.
//! Successful integrity assertions do not imply successful close or API parity.
use std::{
    error::Error,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use pwright_cdp::{CdpConnection, CdpEvent, CdpEventReceiver, CdpSession};
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt};
use tokio_util::sync::CancellationToken;

type TestResult<T> = Result<T, Box<dyn Error>>;

#[derive(Serialize)]
struct Record {
    elapsed_us: u128,
    phase: String,
    loader: Option<String>,
    request: Option<String>,
}

struct Trace {
    start: Instant,
    records: Vec<Record>,
    incomplete: bool,
}

impl Trace {
    fn new() -> Self {
        Self {
            start: Instant::now(),
            records: Vec::with_capacity(256),
            incomplete: false,
        }
    }

    fn record(&mut self, phase: &str, loader: Option<&str>, request: Option<&str>) {
        if self.records.len() >= 256
            || phase.len() > 128
            || loader.is_some_and(|v| v.len() > 128)
            || request.is_some_and(|v| v.len() > 128)
        {
            self.incomplete = true;
            return;
        }
        self.records.push(Record {
            elapsed_us: self.start.elapsed().as_micros(),
            phase: phase.to_owned(),
            loader: loader.map(str::to_owned),
            request: request.map(str::to_owned),
        });
    }
}

fn mark(trace: &Mutex<Trace>, phase: &str) {
    trace.lock().unwrap().record(phase, None, None);
}

fn observe(trace: &Mutex<Trace>, event: &CdpEvent, session: &str, target: &str) {
    if event.session_id.as_deref() != Some(session)
        && event.params["targetId"].as_str() != Some(target)
        && event.params["sessionId"].as_str() != Some(session)
    {
        return;
    }
    if !matches!(
        event.method.as_str(),
        "Page.frameNavigated"
            | "Page.frameStartedNavigating"
            | "Page.frameStoppedLoading"
            | "Network.requestWillBeSent"
            | "Network.responseReceived"
            | "Network.loadingFailed"
            | "Network.loadingFinished"
            | "Target.detachedFromTarget"
            | "Target.targetDestroyed"
            | "Inspector.detached"
    ) {
        return;
    }
    trace.lock().unwrap().record(
        &event.method,
        event.params["loaderId"]
            .as_str()
            .or_else(|| event.params["frame"]["loaderId"].as_str()),
        event.params["requestId"].as_str(),
    );
}

async fn receive(events: &mut CdpEventReceiver) -> TestResult<CdpEvent> {
    events
        .recv()
        .await
        .map_err(|error| format!("incomplete diagnostic event cursor: {error:?}").into())
}

async fn wait_initial_load(
    events: &mut CdpEventReceiver,
    session: &str,
    loader: &str,
) -> TestResult<()> {
    for _ in 0..256 {
        let event = receive(events).await?;
        if event.session_id.as_deref() == Some(session)
            && event.method == "Page.lifecycleEvent"
            && event.params["name"] == "load"
            && event.params["loaderId"] == loader
        {
            return Ok(());
        }
    }
    Err("initial document load exceeds diagnostic event bound".into())
}

async fn wait_document_response(
    events: &mut CdpEventReceiver,
    session: &str,
    old_loader: &str,
    trace: &Mutex<Trace>,
) -> TestResult<()> {
    let mut document = None;
    for _ in 0..256 {
        let event = receive(events).await?;
        if event.session_id.as_deref() != Some(session)
            || event.params["type"] != "Document"
            || event.params["loaderId"] == old_loader
        {
            continue;
        }
        let request = event.params["requestId"]
            .as_str()
            .ok_or("missing document request")?;
        if request.len() > 128 {
            return Err("document identity exceeds diagnostic bound".into());
        }
        if event.method == "Network.requestWillBeSent" {
            document = Some(request.to_owned());
        }
        if event.method == "Network.responseReceived" && document.as_deref() == Some(request) {
            trace.lock().unwrap().record(
                "boundary:native-document-response",
                event.params["loaderId"].as_str(),
                Some(request),
            );
            return Ok(());
        }
    }
    Err("document response exceeds diagnostic event bound".into())
}

#[derive(Serialize)]
struct Report {
    mode: String,
    target: String,
    close: String,
    stop_loading: Option<String>,
    present_after_ack: bool,
    present_after_200_ms: bool,
    present_after_1000_ms: bool,
    present_after_cleanup: bool,
    incomplete: bool,
    records: Vec<Record>,
}

async fn present(browser: &CdpSession, target: &str) -> TestResult<bool> {
    Ok(browser
        .target_get_targets()
        .await?
        .iter()
        .any(|v| v.target_id == target))
}

struct PreparedPage {
    target: String,
    session: String,
    page: CdpSession,
    loader: String,
}

async fn prepare_page(
    connection: &Arc<CdpConnection>,
    browser: &CdpSession,
    context: &str,
    fixture: &str,
    mode: &str,
) -> TestResult<PreparedPage> {
    let target = browser
        .target_create_in_context("about:blank", context)
        .await?;
    let session = browser.target_attach(&target).await?;
    let page = CdpSession::new(connection.clone(), session.clone(), target.clone());
    page.page_enable().await?;
    page.page_set_lifecycle_events_enabled(true).await?;
    page.network_enable().await?;
    let mut events = connection.subscribe_composable_events()?;
    let navigation = page
        .page_navigate(&format!("{fixture}/document?mode={mode}"))
        .await?;
    if let Some(error) = navigation["errorText"].as_str() {
        return Err(format!("initial navigation failed: {error}").into());
    }
    let loader = navigation["loaderId"]
        .as_str()
        .ok_or("missing initial loader")?;
    if loader.len() > 128 || target.len() > 128 || session.len() > 128 {
        return Err("initial native identity exceeds diagnostic bound".into());
    }
    wait_initial_load(&mut events, &session, loader).await?;
    Ok(PreparedPage {
        target,
        session,
        page,
        loader: loader.to_owned(),
    })
}

async fn exercise(
    connection: &Arc<CdpConnection>,
    browser: &CdpSession,
    context: &str,
    fixture: &str,
    mode: &str,
) -> TestResult<Report> {
    // Only setup is subject to an outer deadline. Once the observer exists,
    // the bounded action settles before its observer is stopped and joined.
    let PreparedPage {
        target,
        session,
        page,
        loader,
    } = tokio::time::timeout(
        Duration::from_secs(10),
        prepare_page(connection, browser, context, fixture, mode),
    )
    .await??;
    let trace = Arc::new(Mutex::new(Trace::new()));
    let mut events = connection.subscribe_composable_events()?;
    let stopped = CancellationToken::new();
    let observer_trace = trace.clone();
    let observer_stop = stopped.clone();
    let observer_session = session.clone();
    let observer_target = target.clone();
    let observer = tokio::spawn(async move {
        loop {
            tokio::select! {
                () = observer_stop.cancelled() => break,
                event = events.recv() => match event {
                    Ok(event) => observe(&observer_trace, &event, &observer_session, &observer_target),
                    Err(_) => { observer_trace.lock().unwrap().incomplete = true; break; }
                }
            }
        }
    });
    let action = async {
        let mut response_events = if mode == "native-close-at-response" {
            Some(connection.subscribe_composable_events()?)
        } else {
            None
        };
        mark(&trace, "raw-reload:start");
        page.page_reload().await?;
        mark(&trace, "raw-reload:ack");
        let mut input = tokio::io::BufReader::new(tokio::io::stdin().take(128));
        let mut line = String::new();
        input.read_line(&mut line).await?;
        if line.trim_end() != mode {
            return Err("invalid causal fixture marker".into());
        }
        mark(&trace, "fixture:response-arrived");
        if let Some(events) = response_events.as_mut() {
            wait_document_response(events, &session, &loader, &trace).await?;
        }
        drop(response_events.take());
        let stop_loading = if mode == "stop-before-close" {
            mark(&trace, "stopLoading:start");
            let result = page.page_stop_loading().await;
            mark(
                &trace,
                if result.is_ok() {
                    "stopLoading:ack"
                } else {
                    "stopLoading:error"
                },
            );
            Some(
                result
                    .map(|_| "ack".to_owned())
                    .unwrap_or_else(|error| error.to_string()),
            )
        } else {
            None
        };
        mark(&trace, "close:start");
        let close = browser
            .target_close(&target)
            .await
            .map(|_| "native-command-ack".to_owned())
            .unwrap_or_else(|error| error.to_string());
        mark(&trace, "close:returned");
        let acknowledged = Instant::now();
        let present_after_ack = present(browser, &target).await?;
        // Observation windows only: there is no reload API timeout or cancellation.
        tokio::time::sleep_until((acknowledged + Duration::from_millis(200)).into()).await;
        let present_after_200_ms = present(browser, &target).await?;
        mark(&trace, "presence:200ms");
        tokio::time::sleep_until((acknowledged + Duration::from_millis(1000)).into()).await;
        let present_after_1000_ms = present(browser, &target).await?;
        mark(&trace, "presence:1000ms");
        Ok::<_, Box<dyn Error>>((
            close,
            stop_loading,
            present_after_ack,
            present_after_200_ms,
            present_after_1000_ms,
        ))
    };
    let outcome = tokio::time::timeout(Duration::from_secs(10), action).await;
    stopped.cancel();
    observer.await?;
    let (close, stop_loading, present_after_ack, present_after_200_ms, present_after_1000_ms) =
        outcome??;
    let mut trace = trace.lock().unwrap();
    Ok(Report {
        mode: mode.to_owned(),
        target,
        close,
        stop_loading,
        present_after_ack,
        present_after_200_ms,
        present_after_1000_ms,
        present_after_cleanup: true,
        incomplete: trace.incomplete,
        records: std::mem::take(&mut trace.records),
    })
}

#[tokio::test]
#[ignore = "raw diagnostic; dedicated Chrome/Pwright preflight and causal dynamic fixture required"]
async fn raw_reload_close_boundary() -> TestResult<()> {
    // Arrange
    let endpoint = std::env::var("PWRIGHT_CDP_WS_URL")?;
    assert!(endpoint.starts_with("ws://127.0.0.1:9222/devtools/browser/"));
    let fixture = std::env::var("PWRIGHT_RELOAD_PROBE_FIXTURE")?;
    let url = url::Url::parse(&fixture)?;
    assert_eq!(url.scheme(), "http");
    assert!(
        url.host_str()
            .ok_or("missing fixture host")?
            .ends_with(".ts.net")
    );
    for mode in [
        "native-close",
        "native-close-at-response",
        "stop-before-close",
    ] {
        let connection =
            CdpConnection::connect_with_timeout(&endpoint, Duration::from_secs(5)).await?;
        let browser = CdpSession::browser(connection.clone());
        let context = browser.target_create_browser_context(None, None).await?;
        println!("raw-owned-context {context}");
        // Act: no host or composition, no auto-attach and no reload policy.
        let result = exercise(&connection, &browser, &context, &fixture, mode).await;
        let cleanup = tokio::time::timeout(
            Duration::from_secs(5),
            browser.target_dispose_browser_context(&context),
        )
        .await;
        cleanup??;
        let mut report = result?;
        report.present_after_cleanup = present(&browser, &report.target).await?;
        // Assert integrity and exact owned cleanup, not close success or parity.
        assert!(!report.incomplete);
        assert!(!report.present_after_cleanup);
        let encoded = serde_json::to_string(&report)?;
        assert!(encoded.len() <= 64 * 1024);
        println!("raw-reload-close-probe {encoded}");
    }
    Ok(())
}

#[test]
fn trace_budget_rejects_excess_records() {
    // Arrange
    let mut trace = Trace::new();
    for _ in 0..256 {
        trace.record("phase", None, None);
    }
    // Act
    trace.record("phase", None, None);
    // Assert
    assert!(trace.incomplete);
    assert_eq!(trace.records.len(), 256);
}

#[test]
fn trace_budget_rejects_long_identity_before_retaining_it() {
    // Arrange
    let mut trace = Trace::new();
    // Act
    trace.record("phase", Some(&"x".repeat(129)), None);
    // Assert
    assert!(trace.incomplete);
    assert!(trace.records.is_empty());
}

#[test]
fn observer_ignores_unrelated_page_events() {
    // Arrange
    let trace = Mutex::new(Trace::new());
    let event = CdpEvent::new(
        "Page.frameNavigated".into(),
        serde_json::Value::Null,
        Some("unrelated".into()),
    );
    // Act
    observe(&trace, &event, "owned-session", "owned-target");
    // Assert
    assert!(trace.lock().unwrap().records.is_empty());
}
