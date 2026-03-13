/// A CDP event received from Chrome.
#[derive(Debug, Clone)]
pub struct CdpEvent {
    /// The CDP method name, e.g. "Page.loadEventFired"
    pub method: String,
    /// The event parameters
    pub params: serde_json::Value,
    /// Session ID if scoped to a target, None for browser-level events
    pub session_id: Option<String>,
}
