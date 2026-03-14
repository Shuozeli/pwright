use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, oneshot};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
use tracing::{debug, error, trace, warn};

use crate::events::CdpEvent;

/// Errors from the CDP connection layer.
#[derive(Debug, thiserror::Error)]
pub enum CdpError {
    #[error("WebSocket error: {0}")]
    WebSocket(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("CDP error {code}: {message}")]
    Protocol { code: i64, message: String },
    #[error("Connection closed")]
    Closed,
    #[error("Response channel dropped")]
    ChannelDropped,
    #[error("Timeout waiting for response")]
    Timeout,
    #[error("Element not found for selector: {selector}")]
    ElementNotFound { selector: String },
    #[error("Navigation failed for {url}: {reason}")]
    NavigationFailed { url: String, reason: String },
    #[error("{0}")]
    Other(String),
}

impl From<tokio_tungstenite::tungstenite::Error> for CdpError {
    fn from(e: tokio_tungstenite::tungstenite::Error) -> Self {
        CdpError::WebSocket(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, CdpError>;

type PendingMap = Arc<DashMap<u64, oneshot::Sender<std::result::Result<Value, CdpError>>>>;

/// Raw CDP WebSocket connection -- one per browser.
///
/// Manages the WebSocket transport, message ID allocation,
/// and dispatches responses/events to the appropriate receivers.
///
/// When dropped, aborts the reader/writer tasks and closes the WebSocket.
pub struct CdpConnection {
    write_tx: tokio::sync::mpsc::Sender<Message>,
    next_id: AtomicU64,
    pending: PendingMap,
    event_tx: broadcast::Sender<CdpEvent>,
    default_timeout: Duration,
    shutdown: tokio_util::sync::CancellationToken,
}

impl Drop for CdpConnection {
    fn drop(&mut self) {
        self.shutdown.cancel();
    }
}

/// Default command timeout (30 seconds).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

impl CdpConnection {
    /// Connect to a Chrome DevTools Protocol endpoint via WebSocket.
    pub async fn connect(ws_url: &str) -> Result<Arc<Self>> {
        Self::connect_with_timeout(ws_url, DEFAULT_TIMEOUT).await
    }

    /// Connect with a custom command timeout.
    pub async fn connect_with_timeout(ws_url: &str, timeout: Duration) -> Result<Arc<Self>> {
        debug!(url = ws_url, "connecting to CDP WebSocket");

        let (ws_stream, _response) = connect_async(ws_url).await?;
        let (ws_write, ws_read) = ws_stream.split();

        let (write_tx, write_rx) = tokio::sync::mpsc::channel::<Message>(256);
        let pending: PendingMap = Arc::new(DashMap::new());
        let (event_tx, _) = broadcast::channel::<CdpEvent>(1024);
        let shutdown = tokio_util::sync::CancellationToken::new();

        // Writer task: forwards messages from the channel to the WebSocket
        let shutdown_w = shutdown.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = shutdown_w.cancelled() => {}
                _ = Self::writer_loop(ws_write, write_rx) => {}
            }
        });

        // Reader task: routes incoming messages to pending responses or event broadcast
        let pending_clone = pending.clone();
        let event_tx_clone = event_tx.clone();
        let shutdown_r = shutdown.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = shutdown_r.cancelled() => {}
                _ = Self::reader_loop(ws_read, pending_clone, event_tx_clone) => {}
            }
        });

        let conn = Arc::new(Self {
            write_tx,
            next_id: AtomicU64::new(1),
            pending,
            event_tx,
            default_timeout: timeout,
            shutdown,
        });

        debug!("CDP WebSocket connected");
        Ok(conn)
    }

    /// Send a CDP command (browser-level, no session).
    pub async fn send_command(&self, method: &str, params: Value) -> Result<Value> {
        self.send_command_with_session(method, params, None).await
    }

    /// Send a CDP command, optionally scoped to a session.
    pub async fn send_command_with_session(
        &self,
        method: &str,
        params: Value,
        session_id: Option<&str>,
    ) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let mut msg = serde_json::json!({
            "id": id,
            "method": method,
            "params": params,
        });

        if let Some(sid) = session_id {
            msg["sessionId"] = Value::String(sid.to_string());
        }

        let (tx, rx) = oneshot::channel();
        self.pending.insert(id, tx);

        let raw = serde_json::to_string(&msg)?;
        trace!(id, method, "sending CDP command");

        self.write_tx
            .send(Message::Text(raw.into()))
            .await
            .map_err(|_| CdpError::Closed)?;

        match tokio::time::timeout(self.default_timeout, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(CdpError::ChannelDropped),
            Err(_) => {
                self.pending.remove(&id);
                Err(CdpError::Timeout)
            }
        }
    }

    /// Subscribe to all CDP events.
    pub fn subscribe_events(&self) -> broadcast::Receiver<CdpEvent> {
        self.event_tx.subscribe()
    }

    // Writer loop: drain channel -> websocket
    async fn writer_loop(
        mut ws_write: futures_util::stream::SplitSink<
            WebSocketStream<MaybeTlsStream<TcpStream>>,
            Message,
        >,
        mut write_rx: tokio::sync::mpsc::Receiver<Message>,
    ) {
        while let Some(msg) = write_rx.recv().await {
            if let Err(e) = ws_write.send(msg).await {
                error!("WebSocket write error: {}", e);
                break;
            }
        }
        // Send close frame for clean shutdown
        let _ = ws_write.send(Message::Close(None)).await;
        debug!("CDP writer loop ended");
    }

    // Reader loop: websocket -> route to pending or event_tx
    async fn reader_loop(
        mut ws_read: futures_util::stream::SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        pending: PendingMap,
        event_tx: broadcast::Sender<CdpEvent>,
    ) {
        while let Some(msg_result) = ws_read.next().await {
            let msg = match msg_result {
                Ok(m) => m,
                Err(e) => {
                    error!("WebSocket read error: {}", e);
                    break;
                }
            };

            let text = match msg {
                Message::Text(t) => t,
                Message::Close(_) => {
                    debug!("WebSocket closed by remote");
                    break;
                }
                _ => continue,
            };

            let value: Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(e) => {
                    warn!("Failed to parse CDP message: {}", e);
                    continue;
                }
            };

            // Response to a command (has "id" field)
            if let Some(id) = value.get("id").and_then(|v| v.as_u64()) {
                trace!(id, "received CDP response");
                if let Some((_, tx)) = pending.remove(&id) {
                    let result = if let Some(err) = value.get("error") {
                        let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
                        let message = err
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("unknown error")
                            .to_string();
                        Err(CdpError::Protocol { code, message })
                    } else {
                        Ok(value.get("result").cloned().unwrap_or(Value::Null))
                    };
                    let _ = tx.send(result);
                }
                continue;
            }

            // Event (has "method" field, no "id")
            if let Some(method) = value.get("method").and_then(|m| m.as_str()) {
                let session_id = value
                    .get("sessionId")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string());
                let params = value.get("params").cloned().unwrap_or(Value::Null);

                let event = CdpEvent {
                    method: method.to_string(),
                    params,
                    session_id,
                };

                trace!(method = event.method, "CDP event received");
                let _ = event_tx.send(event);
            }
        }

        // Connection closed -- fail all pending requests
        let entries: Vec<_> = pending.iter().map(|e| *e.key()).collect();
        for id in entries {
            if let Some((_, tx)) = pending.remove(&id) {
                let _ = tx.send(Err(CdpError::Closed));
            }
        }

        debug!("CDP reader loop ended");
    }
}
