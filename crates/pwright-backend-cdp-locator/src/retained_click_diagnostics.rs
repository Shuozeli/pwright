//! Bounded observations for one opt-in retained ElementHandle click.
//!
//! A Page service is shared by concurrent actions, so diagnostics must never
//! use its ordinary selector-action progress slot. The host retains this value
//! until its cancellation cleanup has completed.

use std::sync::Mutex;

use super::ActionabilityProgress;

const MAX_OBSERVATIONS: usize = 8;
const MAX_TAG_BYTES: usize = 24;
const MAX_ID_BYTES: usize = 64;

#[derive(Debug)]
pub(super) struct BlockerPreview(String);

#[derive(Debug)]
struct Observation {
    progress: ActionabilityProgress,
    blocker: Option<BlockerPreview>,
}

#[derive(Debug)]
struct State {
    observations: [Option<Observation>; MAX_OBSERVATIONS],
    next: usize,
    len: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            observations: std::array::from_fn(|_| None),
            next: 0,
            len: 0,
        }
    }
}

/// Independent, fixed-capacity actionability history for one retained click.
#[derive(Debug, Default)]
pub struct RetainedClickDiagnostics {
    state: Mutex<State>,
}

impl RetainedClickDiagnostics {
    /// Accept only a simple tag and ID, never an element's outerHTML or text.
    /// Both inputs have already been sliced by the browser script, but this
    /// native validation is the authoritative bound and escaping boundary.
    pub(super) fn blocker_preview(tag: &str, id: &str) -> Option<BlockerPreview> {
        let tag_bytes = tag.as_bytes();
        if tag_bytes.is_empty()
            || tag_bytes.len() > MAX_TAG_BYTES
            || !tag_bytes[0].is_ascii_lowercase()
            || !tag_bytes[1..]
                .iter()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
        {
            return None;
        }
        if id.len() > MAX_ID_BYTES
            || !id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        {
            return None;
        }
        let markup = if id.is_empty() {
            format!("<{tag}></{tag}>")
        } else {
            format!("<{tag} id=\"{id}\"></{tag}>")
        };
        Some(BlockerPreview(markup))
    }

    pub(super) fn record(&self, progress: ActionabilityProgress, blocker: Option<BlockerPreview>) {
        if let Ok(mut state) = self.state.lock() {
            let next = state.next;
            state.observations[next] = Some(Observation { progress, blocker });
            state.next = (next + 1) % MAX_OBSERVATIONS;
            state.len = (state.len + 1).min(MAX_OBSERVATIONS);
        }
    }

    /// A previously ready target no longer has a waiting-actionability log.
    pub(super) fn clear(&self) {
        if let Ok(mut state) = self.state.lock() {
            *state = State::default();
        }
    }

    /// Returns only the final eight already-observed retries, or none if the
    /// invocation never entered an actionability retry. No browser I/O occurs.
    pub fn timeout_log(&self) -> Option<String> {
        let state = self.state.lock().ok()?;
        if state.len == 0 {
            return None;
        }
        let mut log = String::with_capacity(1024);
        let oldest = (state.next + MAX_OBSERVATIONS - state.len) % MAX_OBSERVATIONS;
        for offset in 0..state.len {
            let index = (oldest + offset) % MAX_OBSERVATIONS;
            let observation = state.observations[index].as_ref()?;
            log.push_str("  - ");
            match observation.progress {
                ActionabilityProgress::Missing | ActionabilityProgress::Detached => {
                    log.push_str("element is detached");
                }
                ActionabilityProgress::NotVisible => log.push_str("element is not visible"),
                ActionabilityProgress::Unstable => log.push_str("element is not stable"),
                ActionabilityProgress::Disabled => log.push_str("element is disabled"),
                ActionabilityProgress::Obscured => {
                    if let Some(blocker) = observation.blocker.as_ref() {
                        log.push_str(&blocker.0);
                        log.push_str(" intercepts pointer events");
                    } else {
                        log.push_str("element does not receive pointer events");
                    }
                }
            }
            log.push('\n');
        }
        log.push_str("  - retrying click action");
        Some(log)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocker_preview_accepts_only_bounded_tag_and_id() {
        // Arrange
        let valid = ("div", "blocker");
        let injected = ("div", "x\" onmouseover=\"evil");
        let oversized = "x".repeat(MAX_ID_BYTES + 1);

        // Act
        let accepted = RetainedClickDiagnostics::blocker_preview(valid.0, valid.1);
        let rejected_injected = RetainedClickDiagnostics::blocker_preview(injected.0, injected.1);
        let rejected_oversized = RetainedClickDiagnostics::blocker_preview("div", &oversized);

        // Assert
        assert_eq!(accepted.unwrap().0, "<div id=\"blocker\"></div>");
        assert!(rejected_injected.is_none());
        assert!(rejected_oversized.is_none());
    }

    #[test]
    fn progress_history_is_bounded_and_clears_after_ready() {
        // Arrange
        let diagnostics = RetainedClickDiagnostics::default();
        for _ in 0..MAX_OBSERVATIONS + 3 {
            diagnostics.record(ActionabilityProgress::Unstable, None);
        }

        // Act
        let before_ready = diagnostics.timeout_log().unwrap();
        diagnostics.clear();
        let after_ready = diagnostics.timeout_log();

        // Assert
        assert_eq!(
            before_ready.matches("element is not stable").count(),
            MAX_OBSERVATIONS
        );
        assert!(before_ready.len() < 1024);
        assert!(after_ready.is_none());
    }

    #[test]
    fn concurrent_invocations_do_not_share_blocker_or_stability_progress() {
        // Arrange
        let blocked = std::sync::Arc::new(RetainedClickDiagnostics::default());
        let unstable = std::sync::Arc::new(RetainedClickDiagnostics::default());
        let barrier = std::sync::Barrier::new(2);

        // Act
        std::thread::scope(|scope| {
            scope.spawn(|| {
                barrier.wait();
                for _ in 0..MAX_OBSERVATIONS + 3 {
                    let blocker = RetainedClickDiagnostics::blocker_preview("div", "blocker");
                    blocked.record(ActionabilityProgress::Obscured, blocker);
                }
            });
            scope.spawn(|| {
                barrier.wait();
                for _ in 0..MAX_OBSERVATIONS + 3 {
                    unstable.record(ActionabilityProgress::Unstable, None);
                }
            });
        });
        let blocked_log = blocked.timeout_log().unwrap();
        let unstable_log = unstable.timeout_log().unwrap();

        // Assert
        assert!(blocked_log.contains("<div id=\"blocker\"></div> intercepts pointer events"));
        assert!(!blocked_log.contains("element is not stable"));
        assert!(unstable_log.contains("element is not stable"));
        assert!(!unstable_log.contains("blocker"));
    }
}
