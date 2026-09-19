//! Opt-in retirement of pending calls from an actual native session event.
use serde::Deserialize;

use super::{CdpError, PendingMap, RetainedCommandResult, Value};

#[derive(Deserialize)]
struct DetachedSession<'a> {
    #[serde(rename = "sessionId")]
    session_id: &'a str,
}

pub(super) fn retire_from_event(pending: &PendingMap, method: &str, params: &Value, limit: usize) {
    if method != "Target.detachedFromTarget" {
        return;
    }
    let Ok(event) = DetachedSession::deserialize(params) else {
        return;
    };
    if event.session_id.is_empty() || event.session_id.len() > limit {
        return;
    }
    // Never hold a DashMap iterator guard while removing an entry. No copied
    // command list or second registry: iteration is bounded by pending capacity.
    for _ in 0..pending.len() {
        let id = pending.iter().find_map(|entry| {
            (entry.session_id.as_deref() == Some(event.session_id)).then_some(*entry.key())
        });
        let Some(id) = id else { break };
        if let Some((_, entry)) = pending.remove(&id) {
            let _ = entry.response.send(RetainedCommandResult {
                result: Err(CdpError::SessionDetached),
                _resource: entry.resource,
            });
        }
    }
}
