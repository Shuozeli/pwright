//! Optional synchronous observer of the production driver's ordered cursor.
pub use pwright_plugin_chromium_frame_observer::{
    ChromiumFrameEventObserver, FrameObservationResult,
};

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
