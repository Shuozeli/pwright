//! Optional one-command Page garbage-collection leaf.
//!
//! Browser/session identity and command lifetime stay in the Rust host. The
//! leaf retains no Page state, task, queue, timer, or browser process.

use pwright_cdp::CdpSession;
use serde::Serialize;

use crate::{HostError, HostSession};

const METHOD: &str = "HeapProfiler.collectGarbage";

#[derive(Serialize)]
struct CollectGarbageParams {}

impl HostSession {
    pub async fn page_request_gc(&self, page_id: &str) -> Result<(), HostError> {
        let (page, _call, _operation) = self.admit_operation(page_id)?;
        let session = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let route = topology
                .topology()
                .page_session_routes()
                .find(|route| route.object_id() == page)
                .ok_or(HostError::TargetClosed)?;
            CdpSession::new(
                live.composition.connection().clone(),
                route.session_id().to_owned(),
                route.target_id().to_owned(),
            )
        };
        let params = serde_json::to_value(CollectGarbageParams {}).map_err(HostError::backend)?;
        session
            .send(METHOD, params)
            .await
            .map(|_| ())
            .map_err(HostError::backend)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_command_has_the_exact_empty_chromium_payload() {
        // Arrange
        let params = CollectGarbageParams {};

        // Act
        let value = serde_json::to_value(params).unwrap();

        // Assert
        assert_eq!(METHOD, "HeapProfiler.collectGarbage");
        assert_eq!(value, serde_json::json!({}));
    }
}
