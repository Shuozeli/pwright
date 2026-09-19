//! Diagnostic receipts only: do not synthesize a response or mutate native state.
use super::{Fields, TraceState};
use serde::Serialize;

const MAX_RECEIPTS: usize = 64;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct NativeLifecycle {
    pub(super) method: String,
    pub(super) session_id: String,
    pub(super) target_id: String,
    pub(super) elapsed_ms: u128,
    // These had no observed CDP response at this receipt. This does not claim
    // their native futures remain pending after SessionDetached is delivered.
    pub(super) unanswered_command_ids: Vec<u64>,
}

impl TraceState {
    pub(super) fn record_lifecycle(&mut self, fields: Fields) {
        let target = fields.target_id.unwrap_or_default();
        let valid = match fields.method.as_deref() {
            Some("Target.detachedFromTarget") => !fields.session_id.is_empty() && target.is_empty(),
            Some("Target.targetDestroyed") => fields.session_id.is_empty() && !target.is_empty(),
            _ => false,
        };
        if fields.malformed || !valid || self.lifecycle.len() == MAX_RECEIPTS {
            self.overflow = true;
            return;
        }
        let unanswered_command_ids = self
            .pending
            .iter()
            .filter_map(|(id, command)| {
                let matches = if target.is_empty() {
                    command.session_id == fields.session_id
                } else {
                    self.sessions
                        .get(&command.session_id)
                        .is_some_and(|value| *value == target)
                };
                matches.then_some(*id)
            })
            .collect();
        self.lifecycle.push(NativeLifecycle {
            method: fields.method.expect("validated method"),
            session_id: fields.session_id,
            target_id: target,
            elapsed_ms: self.started.elapsed().as_millis(),
            unanswered_command_ids,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::super::{CommandTrace, TraceState};
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::prelude::*;

    #[test]
    fn detach_receipt_matches_only_its_session_without_fabricating_responses() {
        // Arrange
        let trace = CommandTrace(Arc::new(Mutex::new(TraceState::new())));
        let subscriber = tracing_subscriber::registry().with(trace.clone());
        // Act
        tracing::subscriber::with_default(subscriber, || {
            tracing::trace!(target: "pwright_cdp::connection", id = 1_u64, method = "Page.enable", session_id = "child");
            tracing::trace!(target: "pwright_cdp::connection", id = 2_u64, method = "Page.enable", session_id = "parent");
            tracing::trace!(target: "pwright_cdp::lifecycle", method = "Target.detachedFromTarget", session_id = "child", target_id = "", malformed = false);
        });
        // Assert
        let state = trace.0.lock().unwrap();
        assert_eq!(state.lifecycle.len(), 1);
        assert_eq!(state.lifecycle[0].unanswered_command_ids, [1]);
        assert_eq!(state.pending.len(), 2);
        assert_eq!(state.responses, 0);
        assert!(state.event_methods.is_empty());
        assert!(!state.overflow);
    }

    #[test]
    fn target_receipt_correlates_only_exact_known_session_bindings() {
        // Arrange
        let trace = CommandTrace(Arc::new(Mutex::new(TraceState::new())));
        let subscriber = tracing_subscriber::registry().with(trace.clone());
        // Act
        tracing::subscriber::with_default(subscriber, || {
            tracing::trace!(target: "pwright_cdp::session", session_id = "child", target_id = "page");
            tracing::trace!(target: "pwright_cdp::connection", id = 1_u64, method = "Page.enable", session_id = "child");
            tracing::trace!(target: "pwright_cdp::connection", id = 2_u64, method = "Page.enable", session_id = "page");
            tracing::trace!(target: "pwright_cdp::lifecycle", method = "Target.targetDestroyed", session_id = "", target_id = "page", malformed = false);
        });
        // Assert
        let state = trace.0.lock().unwrap();
        assert_eq!(state.lifecycle[0].unanswered_command_ids, [1]);
        assert!(!state.overflow);
    }

    #[test]
    fn earlier_native_response_is_not_counted_as_unanswered_at_detach() {
        // Arrange
        let trace = CommandTrace(Arc::new(Mutex::new(TraceState::new())));
        let subscriber = tracing_subscriber::registry().with(trace.clone());
        // Act
        tracing::subscriber::with_default(subscriber, || {
            tracing::trace!(target: "pwright_cdp::connection", id = 1_u64, method = "Page.enable", session_id = "child");
            tracing::trace!(target: "pwright_cdp::connection", id = 1_u64, "response");
            tracing::trace!(target: "pwright_cdp::lifecycle", method = "Target.detachedFromTarget", session_id = "child", target_id = "", malformed = false);
        });
        // Assert
        let state = trace.0.lock().unwrap();
        assert!(state.lifecycle[0].unanswered_command_ids.is_empty());
        assert_eq!(state.responses, 1);
    }

    #[test]
    fn receipt_capacity_fails_recording_without_evicting_earlier_evidence() {
        // Arrange
        let trace = CommandTrace(Arc::new(Mutex::new(TraceState::new())));
        let subscriber = tracing_subscriber::registry().with(trace.clone());
        // Act
        tracing::subscriber::with_default(subscriber, || {
            for _ in 0..65 {
                tracing::trace!(target: "pwright_cdp::lifecycle", method = "Target.detachedFromTarget", session_id = "child", target_id = "", malformed = false);
            }
        });
        // Assert
        let state = trace.0.lock().unwrap();
        assert_eq!(state.lifecycle.len(), 64);
        assert!(state.overflow);
    }

    #[test]
    fn frozen_capture_does_not_record_cleanup_as_initialization() {
        // Arrange
        let trace = CommandTrace(Arc::new(Mutex::new(TraceState::new())));
        trace.0.lock().unwrap().frozen = true;
        let subscriber = tracing_subscriber::registry().with(trace.clone());
        // Act
        tracing::subscriber::with_default(subscriber, || {
            tracing::trace!(target: "pwright_cdp::lifecycle", method = "Target.detachedFromTarget", session_id = "child", target_id = "", malformed = false);
        });
        // Assert
        let state = trace.0.lock().unwrap();
        assert!(state.lifecycle.is_empty());
        assert!(!state.overflow);
    }

    #[test]
    fn invalid_receipts_are_not_silently_accepted() {
        for (method, session, target, malformed) in [
            ("Target.detachedFromTarget", "", "", true),
            ("Target.detachedFromTarget", "child", "page", false),
            ("Target.targetDestroyed", "child", "page", false),
            ("Target.targetCrashed", "", "page", false),
        ] {
            // Arrange
            let trace = CommandTrace(Arc::new(Mutex::new(TraceState::new())));
            let subscriber = tracing_subscriber::registry().with(trace.clone());
            // Act
            tracing::subscriber::with_default(subscriber, || {
                tracing::trace!(target: "pwright_cdp::lifecycle", method, session_id = session, target_id = target, malformed);
            });
            // Assert
            let state = trace.0.lock().unwrap();
            assert!(state.lifecycle.is_empty());
            assert!(state.overflow);
        }
    }

    #[test]
    fn oversized_receipt_identity_is_rejected_before_retention() {
        // Arrange
        let trace = CommandTrace(Arc::new(Mutex::new(TraceState::new())));
        let subscriber = tracing_subscriber::registry().with(trace.clone());
        let session = "s".repeat(129);
        // Act
        tracing::subscriber::with_default(subscriber, || {
            tracing::trace!(target: "pwright_cdp::lifecycle", method = "Target.detachedFromTarget", session_id = session.as_str(), target_id = "", malformed = false);
        });
        // Assert
        let state = trace.0.lock().unwrap();
        assert!(state.lifecycle.is_empty());
        assert!(state.overflow);
    }
}
