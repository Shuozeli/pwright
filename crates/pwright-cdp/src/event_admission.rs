//! Optional, allocation-free selection before retaining an event payload.
use serde_json::Value;

/// A connection-wide, immutable admission policy selected by its composition.
/// The function borrows reader-owned JSON and must not allocate, retain input,
/// block, panic or access global mutable state. It may omit only events the
/// installed consumers explicitly do not need. Responses are never filtered.
/// No policy is installed by default; malformed/unknown input should normally
/// be retained so the existing consumer can report its typed failure.
#[derive(Clone, Copy, Debug)]
pub struct CdpEventAdmissionPolicy {
    name: &'static str,
    retain: fn(&str, &Value, Option<&str>) -> bool,
}

impl CdpEventAdmissionPolicy {
    pub const fn new(name: &'static str, retain: fn(&str, &Value, Option<&str>) -> bool) -> Self {
        Self { name, retain }
    }

    pub const fn name(self) -> &'static str {
        self.name
    }

    pub(crate) fn retains(self, method: &str, params: &Value, session: Option<&str>) -> bool {
        (self.retain)(method, params, session)
    }
}
