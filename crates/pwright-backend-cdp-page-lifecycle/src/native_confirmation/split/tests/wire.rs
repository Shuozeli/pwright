//! Bounded test-only protocol peer. This is not a browser or compatibility oracle.
use super::*;
use futures_util::{SinkExt, StreamExt};
use pwright_cdp::connection::{CdpConnectionProductionProfile, ConnectionConfig};
use pwright_core::ResourceLimits;
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;
use tokio::{
    net::TcpListener,
    sync::{Notify, mpsc},
    task::JoinHandle,
};
use tokio_tungstenite::tungstenite::Message;

#[derive(Clone, Default)]
pub(super) struct Stop {
    stopped: Arc<AtomicBool>,
    changed: Arc<Notify>,
}
impl Stop {
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
        self.changed.notify_waiters();
    }
}
impl NativeCloseInterruption for Stop {
    fn is_interrupted(&self) -> bool {
        self.stopped.load(Ordering::Acquire)
    }
    fn interrupted(&self) -> Result<CloseInterruptionFuture, Error> {
        let this = self.clone();
        Ok(Box::pin(async move {
            loop {
                let wake = this.changed.notified();
                if this.is_interrupted() {
                    return;
                }
                wake.await;
            }
        }))
    }
}

pub(super) struct Peer {
    commands: mpsc::Receiver<Value>,
    responses: mpsc::Sender<Value>,
    task: Option<JoinHandle<()>>,
}
impl Drop for Peer {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}
impl Peer {
    pub async fn command(&mut self) -> Value {
        tokio::time::timeout(Duration::from_secs(10), self.commands.recv())
            .await
            .unwrap()
            .unwrap()
    }
    pub fn no_commands(&self) -> bool {
        self.commands.is_empty()
    }
    pub async fn reply(&self, packet: &Value, result: Value) {
        self.responses
            .send(json!({"id": packet["id"], "result": result}))
            .await
            .unwrap();
    }
    pub async fn error(&self, packet: &Value, message: &str) {
        self.responses
            .send(json!({"id": packet["id"], "error": {"code": -32000, "message": message}}))
            .await
            .unwrap();
    }
    pub async fn destroy(&self, target: &str) {
        self.responses
            .send(json!({"method": "Target.targetDestroyed", "params": {"targetId": target}}))
            .await
            .unwrap();
    }
    pub async fn disconnect(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
            if let Err(error) = task.await {
                assert!(error.is_cancelled());
            }
        }
    }
    async fn finish(mut self) {
        self.disconnect().await;
    }
}

pub(super) struct Fixture {
    pub source: NativePageCloseConfirmation,
    pub stop: Stop,
    pub peer: Peer,
    resources: ProductionResourceLedger,
}
impl Fixture {
    pub async fn new(pending: u16) -> Self {
        // Test-harness environment only. Mac runs pass the verified Tailscale IP;
        // CI without Tailscale uses the prescribed local-dev wildcard fallback.
        let bind: IpAddr = std::env::var("TAILSCALE_IP")
            .unwrap_or_else(|_| "0.0.0.0".into())
            .parse()
            .unwrap();
        let listener = TcpListener::bind((bind, 0)).await.unwrap();
        let mut address = listener.local_addr().unwrap();
        if address.ip().is_unspecified() {
            address.set_ip(Ipv4Addr::LOCALHOST.into());
        }
        let (commands, receive) = mpsc::channel(8);
        let (responses, mut send) = mpsc::channel::<Value>(8);
        let task = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(socket).await.unwrap();
            loop {
                tokio::select! {
                    packet = socket.next() => {
                        let Some(Ok(Message::Text(text))) = packet else { break; };
                        assert!(text.len() <= 4096);
                        commands.send(serde_json::from_str(&text).unwrap()).await.unwrap();
                    }
                    value = send.recv() => {
                        let Some(value) = value else { break; };
                        socket.send(Message::Text(value.to_string().into())).await.unwrap();
                    }
                }
            }
        });
        let peer = Peer {
            commands: receive,
            responses,
            task: Some(task),
        };
        let config = ConnectionConfig {
            timeout: Duration::from_secs(10),
            write_channel_capacity: 4,
            event_channel_capacity: 8,
            pending_command_capacity: 4,
            max_command_bytes: 4096,
            max_response_bytes: 4096,
            max_event_bytes: 4096,
            max_websocket_bytes: 4096,
            max_protocol_error_bytes: 256,
        };
        let profile = CdpConnectionProductionProfile::new(ResourceScope::Connection, config)
            .with_reclaiming_event_fanout(usize::from(pending))
            .unwrap();
        let native =
            NativePageCloseProfile::new(ResourceScope::extension(134, 1), pending, 1024).unwrap();
        let requirements = profile
            .requirements()
            .combine_concurrent(native.requirements());
        let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let capacity = requirements.class(class);
            limits = limits.with_class(class, capacity.global, capacity.per_scope);
        }
        let resources = ProductionResourceLedger::new(limits, requirements).unwrap();
        let connection = CdpConnection::connect_with_production_resources(
            &format!("ws://{address}"),
            profile,
            &resources,
        )
        .await
        .unwrap();
        let stop = Stop::default();
        let source =
            NativePageCloseConfirmation::new(connection, &resources, native, stop.clone()).unwrap();
        Self {
            source,
            stop,
            peer,
            resources,
        }
    }
    pub fn bytes(&self) -> u64 {
        self.resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current
    }
    pub async fn finish(self) {
        let done = self.source.0.connection.io_finished();
        self.peer.finish().await;
        drop(self.source);
        tokio::time::timeout(Duration::from_secs(10), done)
            .await
            .unwrap();
        let metrics = self.resources.metrics().unwrap();
        assert_eq!(metrics.active_leases, 0);
        for class in ResourceClass::ALL {
            assert_eq!(metrics.class(class).current, 0);
        }
    }
}
