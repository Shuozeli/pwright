use std::sync::Arc;

use serde_json::Value;

use crate::connection::{CdpConnection, Result};
use crate::events::CdpEvent;

/// A per-target session for tab-scoped CDP commands.
///
/// Browser-level commands use session_id = None.
/// Tab-level commands use the session_id returned by Target.attachToTarget.
pub struct CdpSession {
    conn: Arc<CdpConnection>,
    session_id: Option<String>,
    target_id: String,
}

impl CdpSession {
    /// Create a browser-level session (no session ID).
    pub fn browser(conn: Arc<CdpConnection>) -> Self {
        Self {
            conn,
            session_id: None,
            target_id: String::new(),
        }
    }

    /// Create a tab-level session.
    pub fn new(conn: Arc<CdpConnection>, session_id: String, target_id: String) -> Self {
        Self {
            conn,
            session_id: Some(session_id),
            target_id,
        }
    }

    /// The CDP target ID this session is attached to.
    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    /// The CDP session ID (None for browser-level).
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// Send a CDP command on this session and await the response.
    pub async fn send(&self, method: &str, params: Value) -> Result<Value> {
        self.conn
            .send_command_with_session(method, params, self.session_id.as_deref())
            .await
    }

    /// Subscribe to CDP events. Caller should filter by session_id if desired.
    pub fn subscribe_events(&self) -> tokio::sync::broadcast::Receiver<CdpEvent> {
        self.conn.subscribe_events()
    }

    /// The underlying connection.
    pub fn connection(&self) -> &Arc<CdpConnection> {
        &self.conn
    }
}
