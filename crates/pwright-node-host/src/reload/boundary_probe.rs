//! Diagnostic intervention, not a parity pass or production close workaround.
//! Observes only the exact owned Page on the existing host CDP connection.
use super::*;
use pwright_cdp::{
    CdpSession,
    events::{CdpEvent, CdpEventRecvError},
};
use pwright_core::ObjectId;
use serde::Serialize;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::io::{AsyncBufReadExt, AsyncReadExt};
use tokio_util::sync::CancellationToken;

#[derive(Serialize)]
struct Record {
    elapsed_us: u128,
    phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    loader: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    request: Option<String>,
}
struct Trace {
    started: Instant,
    records: Vec<Record>,
    overflow: bool,
}
impl Trace {
    fn record(&mut self, phase: &str) {
        if phase.len() > 128 || self.records.len() == 256 {
            self.overflow = true;
            return;
        }
        self.records.push(Record {
            elapsed_us: self.started.elapsed().as_micros(),
            phase: phase.to_owned(),
            loader: None,
            request: None,
        });
    }
}
fn mark(trace: &Mutex<Trace>, phase: &str) {
    trace.lock().unwrap().record(phase);
}

fn record_event(trace: &Mutex<Trace>, event: &CdpEvent, phase: &str) {
    let loader = event
        .params
        .get("loaderId")
        .or_else(|| {
            event
                .params
                .get("frame")
                .and_then(|frame| frame.get("loaderId"))
        })
        .and_then(|value| value.as_str());
    let request = event
        .params
        .get("requestId")
        .and_then(|value| value.as_str());
    let mut trace = trace.lock().unwrap();
    if loader.is_some_and(|value| value.len() > 128)
        || request.is_some_and(|value| value.len() > 128)
    {
        trace.overflow = true;
        return;
    }
    trace.record(phase);
    if !trace.overflow {
        let record = trace.records.last_mut().unwrap();
        record.loader = loader.map(str::to_owned);
        record.request = request.map(str::to_owned);
    }
}
fn projected_event(event: &CdpEvent, target: &str, session: &str) -> Option<String> {
    if event.session_id.as_deref() != Some(session)
        && event.params.get("targetId").and_then(|v| v.as_str()) != Some(target)
        && event.params.get("sessionId").and_then(|v| v.as_str()) != Some(session)
    {
        return None;
    }
    match event.method.as_str() {
        "Page.frameNavigated"
        | "Page.frameStartedNavigating"
        | "Page.frameStartedLoading"
        | "Page.frameStoppedLoading"
        | "Network.requestWillBeSent"
        | "Network.responseReceived"
        | "Network.loadingFailed"
        | "Network.loadingFinished"
        | "Target.targetDestroyed"
        | "Target.detachedFromTarget"
        | "Inspector.detached" => Some(event.method.clone()),
        "Page.lifecycleEvent" => event
            .params
            .get("name")
            .and_then(|v| v.as_str())
            .filter(|v| v.len() <= 64)
            .map(|v| format!("Page.lifecycleEvent:{v}")),
        _ => None,
    }
}

#[derive(Serialize)]
struct Report {
    mode: &'static str,
    target: String,
    close: String,
    reload: String,
    intervention: Option<String>,
    present_after_close: bool,
    present_after_reload: bool,
    present_after_context_cleanup: bool,
    records: Vec<Record>,
    overflow: bool,
}

async fn probe(
    mode: &'static str,
    endpoint: &str,
    fixture: &str,
) -> Result<Report, Box<dyn std::error::Error>> {
    let host = Arc::new(HostSession::connect_over_cdp(endpoint, 5000).await?);
    let context = host.new_context().await?;
    let outcome = async {
        let page = host.new_page(&context.id).await?;
        host.page_goto(&page.id, format!("{fixture}/document?mode={mode}"), "load", None, 5000).await?;
        let (connection, target, session) = {
            let state = host.state.lock().unwrap();
            let live = state.as_ref().unwrap();
            let topology = live.composition.topology()?;
            let object = ObjectId::new(page.id.parse()?);
            let route = topology.topology().page_session_routes().find(|route| route.object_id() == object).ok_or("missing exact Page route")?;
            (live.composition.connection().clone(), route.target_id().to_owned(), route.session_id().to_owned())
        };
        let trace = Arc::new(Mutex::new(Trace { started: Instant::now(), records: Vec::with_capacity(256), overflow: false }));
        let mut events = connection.subscribe_composable_events()?;
        let stopped = CancellationToken::new();
        let task_stopped = stopped.clone();
        let task_trace = trace.clone();
        let task_target = target.clone();
        let task_session = session.clone();
        let observer = tokio::spawn(async move {
            loop {
                tokio::select! {
                    () = task_stopped.cancelled() => break,
                    event = events.recv() => match event {
                        Ok(event) => if let Some(phase) = projected_event(&event, &task_target, &task_session) { record_event(&task_trace, &event, &phase); },
                        Err(CdpEventRecvError::Closed) => break,
                        Err(CdpEventRecvError::Lagged(_)) => { task_trace.lock().unwrap().overflow = true; break; },
                    }
                }
            }
        });
        let run = async {
            let browser = CdpSession::browser(connection.clone());
            let mut response_events = if mode == "native-close-at-response" {
                Some(connection.subscribe_composable_events()?)
            } else { None };
            mark(&trace, "reload:start");
            let request_host = host.clone();
            let page_id = page.id.clone();
            let reload = tokio::spawn(async move { request_host.page_reload(&page_id, "load", 1000).await });
            let action = async {
                let arrival = async {
                    let mut input = tokio::io::BufReader::new(tokio::io::stdin().take(128));
                    let mut line = String::new();
                    input.read_line(&mut line).await?;
                    if line.trim_end() != mode { return Err("invalid causal arrival marker".into()); }
                    Ok::<_, Box<dyn std::error::Error>>(())
                };
                tokio::time::timeout(Duration::from_secs(5), arrival).await??;
                mark(&trace, "fixture:response-arrived");
                if let Some(events) = response_events.as_mut() {
                    // A distinct control boundary, never a replacement for the
                    // server-write signal used by the original failing case.
                    let response = async {
                        let mut document = None;
                        for _ in 0..256 {
                            let event = events.recv().await.map_err(|error| match error {
                                CdpEventRecvError::Closed => "document response event cursor closed".to_owned(),
                                CdpEventRecvError::Lagged(count) => format!("document response event cursor lost {count} events"),
                            })?;
                            if event.session_id.as_deref() != Some(session.as_str())
                                || event.params.get("type").and_then(|v| v.as_str()) != Some("Document") { continue; }
                            let request = event.params.get("requestId").and_then(|v| v.as_str()).ok_or("missing document request identity")?;
                            if request.len() > 128 { return Err("document request identity exceeds diagnostic bound".into()); }
                            if event.method == "Network.requestWillBeSent" { document = Some(request.to_owned()); }
                            if event.method == "Network.responseReceived" && document.as_deref() == Some(request) {
                                record_event(&trace, &event, "boundary:native-document-response");
                                return Ok::<_, Box<dyn std::error::Error>>(());
                            }
                        }
                        Err("document response search exceeds diagnostic event bound".into())
                    };
                    tokio::time::timeout(Duration::from_secs(5), response).await??;
                }
                drop(response_events.take());
                let intervention = if mode == "stop-before-close" {
                    mark(&trace, "intervention:stopLoading:start");
                    let result = CdpSession::new(connection.clone(), session.clone(), target.clone()).page_stop_loading().await;
                    mark(&trace, if result.is_ok() { "intervention:stopLoading:ack" } else { "intervention:stopLoading:error" });
                    Some(result.map(|_| "ok".to_owned()).unwrap_or_else(|error| error.to_string()))
                } else { None };
                mark(&trace, "close:start");
                let close = if mode == "native-close" || mode == "native-close-at-response" {
                    // Control: bypass the host close/disposal path. A command
                    // acknowledgement is deliberately not called close success.
                    browser.target_close(&target).await.map(|_| "native-command-ack".to_owned()).unwrap_or_else(|error| error.to_string())
                } else {
                    host.close_page(&page.id).await.map(|_| "ok".to_owned()).unwrap_or_else(|error| error.to_string())
                };
                mark(&trace, "close:returned");
                let present = browser.target_get_targets().await?.iter().any(|v| v.target_id == target);
                Ok::<_, Box<dyn std::error::Error>>((close, present, intervention))
            }.await;
            // The original 1-second API timeout bounds this task. Never detach it.
            let reload = reload.await?.map(|_| "resolved".to_owned()).unwrap_or_else(|error| error.to_string());
            mark(&trace, "reload:returned");
            let (close, present_after_close, intervention) = action?;
            let present_after_reload = browser.target_get_targets().await?.iter().any(|v| v.target_id == target);
            mark(&trace, "context-cleanup:start");
            host.close_context(&context.id).await?;
            mark(&trace, "context-cleanup:returned");
            let present_after_context_cleanup = browser.target_get_targets().await?.iter().any(|v| v.target_id == target);
            Ok::<_, Box<dyn std::error::Error>>((close, reload, intervention, present_after_close, present_after_reload, present_after_context_cleanup))
        }.await;
        stopped.cancel();
        observer.await?;
        let (close, reload, intervention, present_after_close, present_after_reload, present_after_context_cleanup) = run?;
        let mut trace = trace.lock().unwrap();
        Ok::<_, Box<dyn std::error::Error>>(Report { mode, target, close, reload, intervention, present_after_close, present_after_reload, present_after_context_cleanup, records: std::mem::take(&mut trace.records), overflow: trace.overflow })
    }.await;
    // Cleanup runs even on diagnostic failure. Closing only our owned Context.
    let context_cleanup = host.close_context(&context.id).await;
    let host_cleanup = host.close().await;
    context_cleanup?;
    host_cleanup?;
    let metrics = host.metrics()?;
    assert_eq!(metrics.active_leases, 0);
    assert_eq!(metrics.active_operations, 0);
    assert_eq!(metrics.cleanup_in_flight, 0);
    outcome
}

#[tokio::test]
#[ignore = "causal diagnostic; needs dedicated Chrome/Pwright preflight and held-response server"]
async fn trace_short_response_close_boundary() {
    // Arrange
    let endpoint = std::env::var("PWRIGHT_TEST_CDP").unwrap();
    assert_eq!(endpoint, "http://127.0.0.1:9222");
    let fixture = std::env::var("PWRIGHT_RELOAD_PROBE_FIXTURE").unwrap();
    let url = url::Url::parse(&fixture).unwrap();
    assert_eq!(url.scheme(), "http");
    assert!(url.host_str().unwrap().ends_with(".ts.net"));
    for mode in [
        "baseline",
        "native-close",
        "native-close-at-response",
        "stop-before-close",
    ] {
        // Act
        let report = probe(mode, &endpoint, &fixture).await.unwrap();
        // Assert diagnostic integrity only, never claim parity from intervention.
        assert!(!report.overflow);
        assert!(!report.present_after_context_cleanup);
        let encoded = serde_json::to_string(&report).unwrap();
        assert!(encoded.len() <= 64 * 1024);
        println!("reload-boundary-probe {encoded}");
    }
}

#[test]
fn unrelated_native_event_is_not_retained() {
    // Arrange
    let event = CdpEvent::new(
        "Page.frameNavigated".into(),
        serde_json::Value::Null,
        Some("unrelated".into()),
    );
    // Act
    let projected = projected_event(&event, "owned-target", "owned-session");
    // Assert
    assert!(projected.is_none());
}

#[test]
fn diagnostic_trace_exhaustion_is_explicit_and_does_not_grow_storage() {
    // Arrange
    let mut trace = Trace {
        started: Instant::now(),
        records: Vec::with_capacity(256),
        overflow: false,
    };
    for _ in 0..256 {
        trace.record("owned-event");
    }
    // Act
    trace.record("excess-event");
    // Assert
    assert!(trace.overflow);
    assert_eq!(trace.records.len(), 256);
}

#[test]
fn committed_document_loader_is_preserved_for_event_correlation() {
    // Arrange
    let event = CdpEvent::new(
        "Page.frameNavigated".into(),
        serde_json::from_str(r#"{"frame":{"loaderId":"new-document"}}"#).unwrap(),
        Some("owned-session".into()),
    );
    let trace = Mutex::new(Trace {
        started: Instant::now(),
        records: Vec::with_capacity(256),
        overflow: false,
    });
    // Act
    record_event(&trace, &event, "Page.frameNavigated");
    // Assert
    let trace = trace.lock().unwrap();
    assert_eq!(trace.records.len(), 1);
    assert_eq!(trace.records[0].loader.as_deref(), Some("new-document"));
    assert!(trace.records[0].request.is_none());
    assert!(!trace.overflow);
}

#[test]
fn oversized_native_identity_marks_the_trace_incomplete_without_retaining_it() {
    // Arrange
    let mut params = serde_json::Map::new();
    params.insert("loaderId".into(), "x".repeat(129).into());
    let event = CdpEvent::new(
        "Network.requestWillBeSent".into(),
        params.into(),
        Some("owned-session".into()),
    );
    let trace = Mutex::new(Trace {
        started: Instant::now(),
        records: Vec::with_capacity(256),
        overflow: false,
    });
    // Act
    record_event(&trace, &event, "Network.requestWillBeSent");
    // Assert
    let trace = trace.lock().unwrap();
    assert!(trace.overflow);
    assert!(trace.records.is_empty());
}
