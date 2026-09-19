//! Optional synchronous observer of the production driver's ordered cursor.
use pwright_backend_cdp::CdpTopology;
use pwright_cdp::{CdpEvent, generated::page};
use pwright_core::ObjectId;
use std::error::Error;

pub type FrameObservationResult = Result<(), Box<dyn Error + Send + Sync>>;

/// Implementations admit their own storage. Callbacks run under the topology
/// lock and must not reenter topology, their own observer, or JavaScript.
/// No extra queue, subscriber, task or domain is selected by default.
pub trait ChromiumFrameEventObserver: Send + Sync {
    fn observes_page(&self, page: ObjectId) -> bool;
    /// Admit selected Page storage before enabling native domains. Newly
    /// discovered Page IDs may not have been published in topology yet.
    fn prepare_page(&self, _topology: &CdpTopology, _page: ObjectId) -> FrameObservationResult {
        Ok(())
    }
    /// Release an abandoned pre-publication reservation. A replacement attempt
    /// can reuse a live Page ID: implementations must preserve published owners.
    fn discard_prepared_page(
        &self,
        _topology: &CdpTopology,
        _page: ObjectId,
    ) -> FrameObservationResult {
        Ok(())
    }
    fn initialize_frame(
        &self,
        topology: &CdpTopology,
        session: &str,
        frame: &page::Frame,
    ) -> FrameObservationResult;
    fn reconcile(&self, topology: &CdpTopology) -> FrameObservationResult;
    fn event(&self, topology: &CdpTopology, event: &CdpEvent) -> FrameObservationResult;
    /// Terminal and non-blocking: retained handles must stop exposing readiness.
    fn seal(&self);
}

pub(crate) struct ObservationAttempt {
    observer: Option<std::sync::Arc<dyn ChromiumFrameEventObserver>>,
}
impl ObservationAttempt {
    pub(crate) fn new(observer: &Option<std::sync::Arc<dyn ChromiumFrameEventObserver>>) -> Self {
        Self {
            observer: observer.clone(),
        }
    }
    pub(crate) fn completed(&mut self) {
        self.observer = None;
    }
}
impl Drop for ObservationAttempt {
    fn drop(&mut self) {
        if let Some(observer) = &self.observer {
            observer.seal();
        }
    }
}
