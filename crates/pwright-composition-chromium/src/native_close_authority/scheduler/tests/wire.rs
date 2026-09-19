//! A bounded test peer, not a browser or Playwright oracle.
use super::*;
use futures_util::{SinkExt, StreamExt};
use pwright_backend_cdp::{
    CdpPageObjectIds, CdpPageTarget, CdpTopologyLimits, CdpTopologyProductionProfile,
    ProductionCdpTopology,
};
use pwright_cdp::connection::{CdpConnectionProductionProfile, ConnectionConfig};
use pwright_core::ResourceLimits;
use std::{
    net::{IpAddr, Ipv4Addr},
    sync::{Mutex, atomic::AtomicBool},
    time::Duration,
};
use tokio::{net::TcpListener, sync::mpsc, task::JoinHandle};
use tokio_tungstenite::tungstenite::Message;

#[derive(Clone, Default)]
pub(super) struct Stop {
    stopped: Arc<AtomicBool>,
    changed: Arc<tokio::sync::Notify>,
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
    fn interrupted(
        &self,
    ) -> Result<
        pwright_backend_cdp_page_lifecycle::native_confirmation::CloseInterruptionFuture,
        OperationError<pwright_cap_page_lifecycle::PageLifecycleErrorDetails>,
    > {
        let stop = self.clone();
        Ok(Box::pin(async move {
            loop {
                let wake = stop.changed.notified();
                if stop.is_interrupted() {
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
    pub async fn reply(&self, packet: &Value, value: Value) {
        self.responses
            .send(json!({"id": packet["id"], "result": value}))
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
pub(super) type Fixture = GenericFixture<u64>;
pub(super) struct GenericFixture<T> {
    pub scheduler: NativeCloseScheduler<T>,
    pub stop: Stop,
    pub peer: Peer,
    resources: ProductionResourceLedger,
}
impl<T> GenericFixture<T> {
    pub async fn new(pending: u16) -> Self {
        Self::with_id_bytes(pending, 128).await
    }
    pub async fn with_id_bytes(pending: u16, id_bytes: u16) -> Self {
        Self::build(pending, id_bytes, None).await
    }
    #[cfg(feature = "close-capacity")]
    pub async fn with_topology(pending: u16, topology: ProductionCdpTopology) -> Self {
        Self::build(pending, 128, Some(topology)).await
    }
    async fn build(pending: u16, id_bytes: u16, external: Option<ProductionCdpTopology>) -> Self {
        let bind: IpAddr = std::env::var("TAILSCALE_IP")
            .unwrap_or_else(|_| "0.0.0.0".into())
            .parse()
            .unwrap();
        let listener = TcpListener::bind((bind, 0)).await.unwrap();
        let mut address = listener.local_addr().unwrap();
        if address.ip().is_unspecified() {
            address.set_ip(Ipv4Addr::LOCALHOST.into());
        }
        let (commands, receive) = mpsc::channel(4);
        let (responses, mut send) = mpsc::channel::<Value>(4);
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
            event_channel_capacity: 4,
            pending_command_capacity: 4,
            max_command_bytes: 4096,
            max_response_bytes: 4096,
            max_event_bytes: 4096,
            max_websocket_bytes: 4096,
            max_protocol_error_bytes: 256,
        };
        let connection = CdpConnectionProductionProfile::new(ResourceScope::Connection, config)
            .with_reclaiming_event_fanout(usize::from(pending))
            .unwrap();
        let topology = CdpTopologyProductionProfile::new(
            CdpTopologyLimits::new(32, 4, 8, 16, 16, 1024),
            ResourceScope::extension(128, 1),
        )
        .unwrap();
        let authority = NativeCloseAuthorityProfile::new(
            ResourceScope::extension(128, 2),
            NonZeroU16::new(pending).unwrap(),
            NonZeroU16::new(id_bytes).unwrap(),
        )
        .unwrap();
        let scheduler = NativeCloseSchedulerProfile::<T>::new(
            ResourceScope::extension(129, 1),
            NonZeroU16::new(pending).unwrap(),
        );
        let pages =
            NativePageCloseProfile::new(ResourceScope::extension(129, 2), pending, id_bytes)
                .unwrap();
        let plan = connection
            .requirements()
            .combine_concurrent(topology.requirements())
            .combine_concurrent(authority.requirements())
            .combine_concurrent(scheduler.requirements())
            .combine_concurrent(pages.requirements());
        let mut limits = ResourceLimits::deny_all(plan.scopes, plan.leases);
        for class in ResourceClass::ALL {
            let capacity = plan.class(class);
            limits = limits.with_class(class, capacity.global, capacity.per_scope);
        }
        let resources = ProductionResourceLedger::new(limits, plan).unwrap();
        let mut native =
            ProductionCdpTopology::new(ObjectId::new(1), EPOCH, topology, &resources).unwrap();
        for (context, page, frame, name, target, session) in [
            (2, 3, 4, "context-a", "page-a", "session-a"),
            (5, 6, 7, "context-b", "page-b", "session-b"),
        ] {
            native
                .register_context(
                    EPOCH,
                    name,
                    ObjectId::new(context),
                    CdpContextOwnership::Owned,
                )
                .unwrap();
            native
                .attach_page(
                    EPOCH,
                    CdpPageTarget::new(target, session, name),
                    CdpPageObjectIds::new(ObjectId::new(page), ObjectId::new(frame)),
                )
                .unwrap();
        }
        // An externally admitted topology keeps its original owner identity and
        // leases. Its ledger is checked separately by the composition test.
        let native = external.unwrap_or(native);
        let connection = CdpConnection::connect_with_production_resources(
            &format!("ws://{address}"),
            connection,
            &resources,
        )
        .await
        .unwrap();
        let stop = Stop::default();
        let scheduler = NativeCloseScheduler::bind(
            connection,
            Arc::new(Mutex::new(native)),
            &resources,
            scheduler,
            authority,
            pages,
            stop.clone(),
        )
        .unwrap();
        Self {
            scheduler,
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
        let done = self.scheduler.authority.0.connection.io_finished();
        self.peer.finish().await;
        drop(self.scheduler);
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
