//! Explicit diagnostic lane, not a retry or an exception in a parity gate.
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pwright_node_host::{HostSession, SessionMetrics};
use serde::Serialize;
use tracing::{Event, Subscriber, field::Visit};
use tracing_subscriber::{Layer, layer::Context, prelude::*};

#[path = "attach_probe/lifecycle.rs"]
mod lifecycle;
use lifecycle::NativeLifecycle;

#[derive(Clone, Serialize)]
struct Command {
    id: u64,
    method: String,
    session_id: String,
    prepared_ms: u128,
    response_ms: Option<u128>,
}

struct TraceState {
    started: Instant,
    commands: u64,
    responses: u64,
    pending: BTreeMap<u64, Command>,
    completed: VecDeque<Command>,
    event_methods: BTreeMap<String, u64>,
    sessions: BTreeMap<String, String>,
    lifecycle: Vec<NativeLifecycle>,
    omitted_events: u64,
    overflow: bool,
    frozen: bool,
}

impl TraceState {
    fn new() -> Self {
        Self {
            started: Instant::now(),
            commands: 0,
            responses: 0,
            pending: BTreeMap::new(),
            completed: VecDeque::new(),
            event_methods: BTreeMap::new(),
            sessions: BTreeMap::new(),
            lifecycle: Vec::new(),
            omitted_events: 0,
            overflow: false,
            frozen: false,
        }
    }
}

#[derive(Default)]
struct Fields {
    id: Option<u64>,
    method: Option<String>,
    session_id: String,
    target_id: Option<String>,
    oversized: bool,
    protocol_error: bool,
    malformed: bool,
}

impl Visit for Fields {
    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        if field.name() == "malformed" {
            self.malformed = value;
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, _: i64) {
        // The connection emits the normal response receipt first, then a
        // second id/code diagnostic. The latter is not another response.
        if field.name() == "code" {
            self.protocol_error = true;
        }
    }
    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if field.name() == "id" {
            self.id = Some(value);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if matches!(field.name(), "method" | "session_id" | "target_id") {
            if value.len() > 128 {
                self.oversized = true;
            } else if field.name() == "session_id" {
                self.session_id = value.to_owned();
            } else if field.name() == "target_id" {
                self.target_id = Some(value.to_owned());
            } else {
                self.method = Some(value.to_owned());
            }
        }
    }

    fn record_debug(&mut self, _: &tracing::field::Field, _: &dyn std::fmt::Debug) {}
}

#[derive(Clone)]
struct CommandTrace(Arc<Mutex<TraceState>>);

impl<S: Subscriber> Layer<S> for CommandTrace {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        if !matches!(
            event.metadata().target(),
            "pwright_cdp::connection"
                | "pwright_cdp::event_admission"
                | "pwright_cdp::session"
                | "pwright_cdp::lifecycle"
        ) || *event.metadata().level() != tracing::Level::TRACE
        {
            return;
        }
        let mut fields = Fields::default();
        event.record(&mut fields);
        let mut state = self.0.lock().expect("diagnostic state poisoned");
        if state.frozen || fields.protocol_error {
            return;
        }
        state.overflow |= fields.oversized;
        if fields.oversized {
            return;
        }
        if event.metadata().target() == "pwright_cdp::lifecycle" {
            state.record_lifecycle(fields);
            return;
        }
        if event.metadata().target() == "pwright_cdp::session" {
            let Some(target) = fields.target_id.filter(|target| !target.is_empty()) else {
                state.overflow = true;
                return;
            };
            if let Some(existing) = state.sessions.get(&fields.session_id) {
                state.overflow |= *existing != target;
            } else if state.sessions.len() < 64 && !fields.session_id.is_empty() {
                state.sessions.insert(fields.session_id, target);
            } else {
                state.overflow = true;
            }
            return;
        }
        if event.metadata().target() == "pwright_cdp::event_admission" {
            state.omitted_events = state.omitted_events.saturating_add(1);
            return;
        }
        let Some(id) = fields.id else {
            if let Some(method) = fields.method {
                // Event method/count only: never retain browser event payloads.
                // Use a fixed distinct-method ceiling as well as bounded keys.
                if let Some(count) = state.event_methods.get_mut(&method) {
                    *count = count.saturating_add(1);
                } else if state.event_methods.len() < 64 {
                    state.event_methods.insert(method, 1);
                } else {
                    state.overflow = true;
                }
            }
            return;
        };
        let elapsed = state.started.elapsed().as_millis();
        match fields.method {
            Some(method) => {
                state.commands += 1;
                if state.pending.len() == 32 || state.pending.contains_key(&id) {
                    state.overflow = true;
                    return;
                }
                state.pending.insert(
                    id,
                    Command {
                        id,
                        method,
                        session_id: fields.session_id,
                        prepared_ms: elapsed,
                        response_ms: None,
                    },
                );
            }
            None => {
                state.responses += 1;
                if let Some(mut command) = state.pending.remove(&id) {
                    command.response_ms = Some(elapsed);
                    if state.completed.len() == 128 {
                        state.completed.pop_front();
                    }
                    state.completed.push_back(command);
                }
            }
        }
    }
}

#[test]
fn command_probe_bounds_unanswered_commands() {
    // Arrange
    let trace = CommandTrace(Arc::new(Mutex::new(TraceState::new())));
    let subscriber = tracing_subscriber::registry().with(trace.clone());
    // Act
    tracing::subscriber::with_default(subscriber, || {
        for id in 1..=33_u64 {
            tracing::trace!(target: "pwright_cdp::connection", id, method = "Page.enable", session_id = "fixture");
        }
    });
    // Assert
    let state = trace.0.lock().unwrap();
    assert_eq!(state.pending.len(), 32);
    assert!(state.overflow);
}

#[test]
fn command_probe_keeps_only_the_latest_bounded_completed_history() {
    // Arrange
    let trace = CommandTrace(Arc::new(Mutex::new(TraceState::new())));
    let subscriber = tracing_subscriber::registry().with(trace.clone());
    // Act
    tracing::subscriber::with_default(subscriber, || {
        for id in 1..=200_u64 {
            tracing::trace!(target: "pwright_cdp::connection", id, method = "Page.enable", session_id = "fixture");
            tracing::trace!(target: "pwright_cdp::connection", id, "received CDP response");
        }
    });
    // Assert
    let state = trace.0.lock().unwrap();
    assert_eq!(state.completed.len(), 128);
    assert_eq!(state.completed.front().unwrap().id, 73);
    assert_eq!(state.completed.back().unwrap().session_id, "fixture");
    assert!(state.pending.is_empty());
    assert!(!state.overflow);
}

#[test]
fn command_probe_rejects_oversized_identity_before_retention() {
    // Arrange
    let trace = CommandTrace(Arc::new(Mutex::new(TraceState::new())));
    let subscriber = tracing_subscriber::registry().with(trace.clone());
    let session = "s".repeat(129);
    // Act
    tracing::subscriber::with_default(subscriber, || {
        tracing::trace!(target: "pwright_cdp::connection", id = 1_u64, method = "Page.enable", session_id = session.as_str());
    });
    // Assert
    let state = trace.0.lock().unwrap();
    assert!(state.pending.is_empty());
    assert!(state.overflow);
}

#[test]
fn command_probe_counts_event_methods_without_retaining_payloads() {
    // Arrange
    let trace = CommandTrace(Arc::new(Mutex::new(TraceState::new())));
    let subscriber = tracing_subscriber::registry().with(trace.clone());
    // Act
    tracing::subscriber::with_default(subscriber, || {
        for _ in 0..2000 {
            tracing::trace!(target: "pwright_cdp::connection", method = "Runtime.consoleAPICalled", "CDP event received");
        }
    });
    // Assert
    let state = trace.0.lock().unwrap();
    assert_eq!(state.event_methods.len(), 1);
    assert_eq!(state.event_methods["Runtime.consoleAPICalled"], 2000);
    assert_eq!(state.commands, 0);
    assert_eq!(state.responses, 0);
    assert!(!state.overflow);
}

#[test]
fn command_probe_caps_distinct_event_methods() {
    // Arrange
    let trace = CommandTrace(Arc::new(Mutex::new(TraceState::new())));
    let subscriber = tracing_subscriber::registry().with(trace.clone());
    // Act
    tracing::subscriber::with_default(subscriber, || {
        for id in 0..65 {
            let method = format!("Test.event{id}");
            tracing::trace!(target: "pwright_cdp::connection", method = method.as_str(), "CDP event received");
        }
    });
    // Assert
    let state = trace.0.lock().unwrap();
    assert_eq!(state.event_methods.len(), 64);
    assert!(state.overflow);
}

#[test]
fn command_probe_separates_pre_admission_omissions_from_delivered_events() {
    // Arrange
    let trace = CommandTrace(Arc::new(Mutex::new(TraceState::new())));
    let subscriber = tracing_subscriber::registry().with(trace.clone());
    // Act
    tracing::subscriber::with_default(subscriber, || {
        tracing::trace!(target: "pwright_cdp::event_admission", method = "Runtime.executionContextCreated", "omitted");
        tracing::trace!(target: "pwright_cdp::connection", method = "Runtime.executionContextCreated", "received");
    });
    // Assert
    let state = trace.0.lock().unwrap();
    assert_eq!(state.omitted_events, 1);
    assert_eq!(state.event_methods["Runtime.executionContextCreated"], 1);
    assert_eq!(state.commands, 0);
    assert!(!state.overflow);
}

#[test]
fn command_probe_deduplicates_session_bindings_without_counting_commands() {
    // Arrange
    let trace = CommandTrace(Arc::new(Mutex::new(TraceState::new())));
    let subscriber = tracing_subscriber::registry().with(trace.clone());
    // Act
    tracing::subscriber::with_default(subscriber, || {
        for _ in 0..128 {
            tracing::trace!(target: "pwright_cdp::session", session_id = "session", target_id = "page");
        }
    });
    // Assert
    let state = trace.0.lock().unwrap();
    assert_eq!(state.sessions.len(), 1);
    assert_eq!(state.sessions["session"], "page");
    assert_eq!(state.commands, 0);
    assert!(state.event_methods.is_empty());
    assert!(!state.overflow);
}

#[test]
fn command_probe_caps_retained_session_bindings() {
    // Arrange
    let trace = CommandTrace(Arc::new(Mutex::new(TraceState::new())));
    let subscriber = tracing_subscriber::registry().with(trace.clone());
    // Act
    tracing::subscriber::with_default(subscriber, || {
        for id in 0..65 {
            let session = format!("session-{id}");
            tracing::trace!(target: "pwright_cdp::session", session_id = session.as_str(), target_id = "page");
        }
    });
    // Assert
    let state = trace.0.lock().unwrap();
    assert_eq!(state.sessions.len(), 64);
    assert!(state.overflow);
}

#[test]
fn command_probe_rejects_oversized_target_before_retention() {
    // Arrange
    let trace = CommandTrace(Arc::new(Mutex::new(TraceState::new())));
    let subscriber = tracing_subscriber::registry().with(trace.clone());
    let target = "x".repeat(129);
    // Act
    tracing::subscriber::with_default(subscriber, || {
        tracing::trace!(target: "pwright_cdp::session", session_id = "session", target_id = target.as_str());
    });
    // Assert
    let state = trace.0.lock().unwrap();
    assert!(state.sessions.is_empty());
    assert!(state.overflow);
}

#[test]
fn command_probe_marks_conflicting_session_identity_without_replacing_it() {
    // Arrange
    let trace = CommandTrace(Arc::new(Mutex::new(TraceState::new())));
    let subscriber = tracing_subscriber::registry().with(trace.clone());
    // Act
    tracing::subscriber::with_default(subscriber, || {
        tracing::trace!(target: "pwright_cdp::session", session_id = "session", target_id = "first");
        tracing::trace!(target: "pwright_cdp::session", session_id = "session", target_id = "different");
    });
    // Assert
    let state = trace.0.lock().unwrap();
    assert_eq!(state.sessions["session"], "first");
    assert!(state.overflow);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Report<'a> {
    schema: &'static str,
    timeout_ms: u32,
    elapsed_ms: u128,
    error: Option<String>,
    commands: u64,
    responses: u64,
    pending: &'a BTreeMap<u64, Command>,
    recent_completed: &'a VecDeque<Command>,
    event_methods: &'a BTreeMap<String, u64>,
    sessions: &'a BTreeMap<String, String>,
    lifecycle: &'a [NativeLifecycle],
    omitted_events: u64,
    overflow: bool,
}

#[tokio::test]
#[ignore = "diagnostic only; requires dedicated Chrome and an explicit Pwright preflight"]
async fn report_unanswered_attach_commands_without_changing_the_deadline()
-> Result<(), Box<dyn std::error::Error>> {
    // Arrange: the current-thread runtime keeps this subscriber local to the probe.
    let endpoint = std::env::var("PWRIGHT_TEST_CDP").expect("explicit dedicated endpoint required");
    assert_eq!(endpoint, "http://127.0.0.1:9222");
    let trace = CommandTrace(Arc::new(Mutex::new(TraceState::new())));
    let subscriber = tracing_subscriber::registry().with(trace.clone());
    let _guard = tracing::subscriber::set_default(subscriber);
    // Act: one attempt, using the same production host and five-second budget.
    let result = HostSession::connect_over_cdp(&endpoint, 5000).await;
    // Assert: print bounded evidence even on failure; never count a timeout as a pass.
    {
        let mut state = trace.0.lock().expect("diagnostic state poisoned");
        state.frozen = true;
        let report = Report {
            schema: "pwright.attach-command-probe.v1",
            timeout_ms: 5000,
            elapsed_ms: state.started.elapsed().as_millis(),
            error: result.as_ref().err().map(ToString::to_string),
            commands: state.commands,
            responses: state.responses,
            pending: &state.pending,
            recent_completed: &state.completed,
            event_methods: &state.event_methods,
            sessions: &state.sessions,
            lifecycle: &state.lifecycle,
            omitted_events: state.omitted_events,
            overflow: state.overflow,
        };
        println!(
            "{}",
            serde_json::to_string(&report).expect("bounded report serializes")
        );
    }
    let session = result?;
    let cleanup = tokio::time::timeout(Duration::from_secs(10), session.close()).await;
    cleanup??;
    assert_eq!(
        session.metrics().expect("read host metrics"),
        SessionMetrics::default()
    );
    let state = trace.0.lock().expect("diagnostic state poisoned");
    assert!(!state.overflow, "diagnostic recording capacity exceeded");
    assert!(state.commands > 0, "CDP command tracing was not active");
    Ok(())
}

#[test]
fn command_probe_counts_a_protocol_failure_as_one_response() {
    // Arrange
    let trace = CommandTrace(Arc::new(Mutex::new(TraceState::new())));
    let subscriber = tracing_subscriber::registry().with(trace.clone());
    // Act
    tracing::subscriber::with_default(subscriber, || {
        tracing::trace!(target: "pwright_cdp::connection", id = 1_u64, method = "Page.enable", session_id = "fixture");
        tracing::trace!(target: "pwright_cdp::connection", id = 1_u64, "received CDP response");
        tracing::trace!(target: "pwright_cdp::connection", id = 1_u64, code = -32000_i64, "CDP protocol command failed");
    });
    // Assert
    let state = trace.0.lock().unwrap();
    assert_eq!(state.commands, 1);
    assert_eq!(state.responses, 1);
    assert_eq!(state.completed.len(), 1);
    assert!(state.pending.is_empty());
}

#[test]
fn command_probe_freezes_the_connect_snapshot_before_cleanup() {
    // Arrange
    let trace = CommandTrace(Arc::new(Mutex::new(TraceState::new())));
    trace.0.lock().unwrap().frozen = true;
    let subscriber = tracing_subscriber::registry().with(trace.clone());
    // Act
    tracing::subscriber::with_default(subscriber, || {
        tracing::trace!(target: "pwright_cdp::connection", id = 1_u64, method = "Target.detachFromTarget", session_id = "fixture");
        tracing::trace!(target: "pwright_cdp::connection", id = 1_u64, "received CDP response");
    });
    // Assert
    let state = trace.0.lock().unwrap();
    assert_eq!(state.commands, 0);
    assert_eq!(state.responses, 0);
    assert!(state.pending.is_empty());
    assert!(state.completed.is_empty());
    assert!(!state.overflow);
}
