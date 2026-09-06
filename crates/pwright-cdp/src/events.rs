use std::ops::Deref;
use std::sync::Arc;

use pwright_core::ResourceLease;

use crate::event_fanout::{
    ReclaimingEventReceiver, ReclaimingFanoutRecvError, ReclaimingFanoutTryRecvError,
};

/// Immutable data carried by one CDP event.
#[derive(Debug)]
pub struct CdpEventData {
    /// The CDP method name, e.g. "Page.loadEventFired"
    pub method: String,
    /// The event parameters
    pub params: serde_json::Value,
    /// Session ID if scoped to a target, None for browser-level events
    pub session_id: Option<String>,
    pub(crate) _resource: Option<ResourceLease>,
}

/// A CDP event received from Chrome.
///
/// Clones share one immutable payload. This is important for the broadcast
/// path: adding subscribers must not deep-copy an arbitrary JSON tree for each
/// receiver.
#[derive(Clone, Debug)]
pub struct CdpEvent(Arc<CdpEventData>);

impl CdpEvent {
    pub fn new(method: String, params: serde_json::Value, session_id: Option<String>) -> Self {
        Self::from_parts(method, params, session_id, None)
    }

    pub(crate) fn new_budgeted(
        method: String,
        params: serde_json::Value,
        session_id: Option<String>,
        resource: ResourceLease,
    ) -> Self {
        Self::from_parts(method, params, session_id, Some(resource))
    }

    fn from_parts(
        method: String,
        params: serde_json::Value,
        session_id: Option<String>,
        resource: Option<ResourceLease>,
    ) -> Self {
        Self(Arc::new(CdpEventData {
            method,
            params,
            session_id,
            _resource: resource,
        }))
    }

    /// Returns whether two values retain the same immutable event allocation.
    pub fn shares_payload_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Deref for CdpEvent {
    type Target = CdpEventData;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Event cursor used by composable v2 drivers.
///
/// Legacy connections retain Tokio broadcast behavior. Production profiles may
/// instead select reclaim-after-all-read fanout without changing each driver's
/// receive contract.
#[derive(Debug)]
pub enum CdpEventReceiver {
    Legacy(tokio::sync::broadcast::Receiver<CdpEvent>),
    Reclaiming(ReclaimingEventReceiver<CdpEvent>),
}

impl CdpEventReceiver {
    pub fn resubscribe(&mut self) {
        match self {
            Self::Legacy(receiver) => *receiver = receiver.resubscribe(),
            Self::Reclaiming(receiver) => receiver.resubscribe(),
        }
    }

    pub async fn recv(&mut self) -> Result<CdpEvent, CdpEventRecvError> {
        match self {
            Self::Legacy(receiver) => receiver.recv().await.map_err(|error| match error {
                tokio::sync::broadcast::error::RecvError::Closed => CdpEventRecvError::Closed,
                tokio::sync::broadcast::error::RecvError::Lagged(skipped) => {
                    CdpEventRecvError::Lagged(skipped)
                }
            }),
            Self::Reclaiming(receiver) => receiver.recv().await.map_err(|error| match error {
                ReclaimingFanoutRecvError::Closed => CdpEventRecvError::Closed,
                ReclaimingFanoutRecvError::Lagged(skipped) => CdpEventRecvError::Lagged(skipped),
            }),
        }
    }

    pub fn try_recv(&mut self) -> Result<CdpEvent, CdpEventTryRecvError> {
        match self {
            Self::Legacy(receiver) => receiver.try_recv().map_err(|error| match error {
                tokio::sync::broadcast::error::TryRecvError::Empty => CdpEventTryRecvError::Empty,
                tokio::sync::broadcast::error::TryRecvError::Closed => CdpEventTryRecvError::Closed,
                tokio::sync::broadcast::error::TryRecvError::Lagged(skipped) => {
                    CdpEventTryRecvError::Lagged(skipped)
                }
            }),
            Self::Reclaiming(receiver) => receiver.try_recv().map_err(|error| match error {
                ReclaimingFanoutTryRecvError::Empty => CdpEventTryRecvError::Empty,
                ReclaimingFanoutTryRecvError::Closed => CdpEventTryRecvError::Closed,
                ReclaimingFanoutTryRecvError::Lagged(skipped) => {
                    CdpEventTryRecvError::Lagged(skipped)
                }
            }),
        }
    }

    pub const fn is_reclaiming(&self) -> bool {
        matches!(self, Self::Reclaiming(_))
    }
}

impl From<tokio::sync::broadcast::Receiver<CdpEvent>> for CdpEventReceiver {
    fn from(receiver: tokio::sync::broadcast::Receiver<CdpEvent>) -> Self {
        Self::Legacy(receiver)
    }
}

impl From<ReclaimingEventReceiver<CdpEvent>> for CdpEventReceiver {
    fn from(receiver: ReclaimingEventReceiver<CdpEvent>) -> Self {
        Self::Reclaiming(receiver)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CdpEventRecvError {
    Closed,
    Lagged(u64),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CdpEventTryRecvError {
    Empty,
    Closed,
    Lagged(u64),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_shares_the_event_payload() {
        // Arrange
        let event = CdpEvent::new(
            "Runtime.consoleAPICalled".to_string(),
            serde_json::json!({"message": "large payload"}),
            Some("session-1".to_string()),
        );

        // Act
        let cloned = event.clone();

        // Assert
        assert!(event.shares_payload_with(&cloned));
        assert_eq!(cloned.method, "Runtime.consoleAPICalled");
    }
}
