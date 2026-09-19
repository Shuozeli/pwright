//! One caller-owned interception interval on a fixed native session.
use crate::{AdmittedDragData, DragDataStore, error};
use pwright_cdp::{
    CdpEvent, CdpEventReceiver, CdpEventRecvError, CdpSession, connection::Result,
    generated::input::SetInterceptDragsParams,
};
use pwright_core::{
    ProductionResourceLedger, ResourceClass, ResourceLease, ResourceLedger, ResourceRequirements,
    ResourceScope,
};
use std::{
    future::{Future, poll_fn},
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

pub type DragToggleFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;
/// A fixed connection/session binding. Implementations must never reconnect or
/// change the session behind this object. Subscription and toggles must use that
/// same binding. The toggle future must be lazy and retain its native receipt.
pub trait DragInterceptionClient: Send + Sync {
    fn session_id(&self) -> Option<&str>;
    fn subscribe(&self) -> Result<CdpEventReceiver>;
    fn set_interception(self: Arc<Self>, enabled: bool) -> DragToggleFuture;
}
impl DragInterceptionClient for CdpSession {
    fn session_id(&self) -> Option<&str> {
        self.session_id()
    }
    fn subscribe(&self) -> Result<CdpEventReceiver> {
        self.subscribe_composable_events()
            .map_err(|_| error("drag event subscription admission failed"))
    }
    fn set_interception(self: Arc<Self>, enabled: bool) -> DragToggleFuture {
        Box::pin(async move {
            let params = serde_json::to_value(SetInterceptDragsParams { enabled })
                .map_err(|_| error("drag interception parameter encoding failed"))?;
            self.send("Input.setInterceptDrags", params).await?;
            Ok(())
        })
    }
}
type ReadResult = (
    CdpEventReceiver,
    std::result::Result<CdpEvent, CdpEventRecvError>,
);
type EventRead = Pin<Box<dyn Future<Output = ReadResult> + Send>>;
async fn read(receiver: Option<CdpEventReceiver>) -> ReadResult {
    let mut receiver = receiver.expect("subscribed capture receiver");
    let result = receiver.recv().await;
    (receiver, result)
}

#[derive(Default)]
struct State {
    next: u64,
    busy: bool,
    poisoned: bool,
}
struct Factory {
    client: Arc<dyn DragInterceptionClient>,
    store: DragDataStore,
    scope: ResourceScope,
    ledger: ResourceLedger,
    state: Mutex<State>,
    _storage: ResourceLease,
}
/// Install exactly one shared factory per Page/session. Clones share exclusivity;
/// constructing independent factories is not a global native interception lock.
#[derive(Clone)]
pub struct DragCaptureFactory(Arc<Factory>);
fn factory_bytes() -> usize {
    std::mem::size_of::<Factory>() + 2 * std::mem::size_of::<usize>()
}
fn capture_bytes() -> usize {
    // Source queue/client/store and provider toggle future allocation trees have
    // separate budgets. Includes the concrete wrapper-read future layout.
    std::mem::size_of::<DragCapture>() + 128 + read_bytes()
}
fn read_bytes() -> usize {
    // Measuring an unpolled future creates no receiver or subscription.
    std::mem::size_of_val(&read(None))
}
impl DragCaptureFactory {
    /// Checks another trait view of this same native client allocation. Equal
    /// session-ID strings are insufficient; the caller must reuse the client.
    pub fn uses_cdp_client(&self, client: &Arc<dyn pwright_cdp::CdpClient>) -> bool {
        std::ptr::addr_eq(Arc::as_ptr(&self.0.client), Arc::as_ptr(client))
    }
    #[cfg(feature = "mouse-dispatch")]
    pub(crate) fn shares_factory(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    pub(crate) fn uses_client(&self, client: &Arc<dyn DragInterceptionClient>) -> bool {
        Arc::ptr_eq(&self.0.client, client)
    }
    pub(crate) fn owns_data(&self, capture: &DragCapture, data: &AdmittedDragData) -> bool {
        Arc::ptr_eq(&self.0, &capture.factory.0)
            && capture
                .data
                .as_ref()
                .is_some_and(|owned| owned.shares_payload_with(data))
    }
    pub fn requirements() -> ResourceRequirements {
        let bytes = (factory_bytes() + capture_bytes()) as u64;
        ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, 2)
    }
    pub fn new(
        client: Arc<dyn DragInterceptionClient>,
        store: DragDataStore,
        scope: ResourceScope,
        resources: &ProductionResourceLedger,
    ) -> Result<Self> {
        if !client
            .session_id()
            .is_some_and(|id| (1..=128).contains(&id.len()))
        {
            return Err(error(
                "drag capture requires a fixed session of 1..=128 bytes",
            ));
        }
        let ledger = resources
            .claim(Self::requirements())
            .map_err(|_| error("drag capture resource claim failed"))?;
        let storage = ledger
            .reserve(
                scope,
                ResourceClass::BackendStateBytes,
                factory_bytes() as u64,
            )
            .map_err(|_| error("drag capture factory admission failed"))?;
        Ok(Self(Arc::new(Factory {
            client,
            store,
            scope,
            ledger,
            state: Mutex::new(State::default()),
            _storage: storage,
        })))
    }
    /// Admits a single interval; no subscription or native command until enable
    /// is polled. Epochs are factory-local, checked, and never reused.
    pub fn start(&self) -> Result<DragCapture> {
        let mut state = self.0.state.lock().expect("drag capture factory poisoned");
        if state.busy || state.poisoned {
            return Err(error(
                "drag capture session busy or abandoned without cleanup",
            ));
        }
        let epoch = state
            .next
            .checked_add(1)
            .ok_or_else(|| error("drag capture epochs exhausted"))?;
        let storage = self
            .0
            .ledger
            .reserve(
                self.0.scope,
                ResourceClass::BackendStateBytes,
                capture_bytes() as u64,
            )
            .map_err(|_| error("drag capture admission failed"))?;
        state.next = epoch;
        state.busy = true;
        Ok(DragCapture {
            session: self
                .0
                .client
                .session_id()
                .expect("validated fixed session")
                .into(),
            epoch,
            factory: self.clone(),
            stage: Stage::Unstarted,
            pending: None,
            reader: None,
            data: None,
            read_failed: false,
            retiring: false,
            _storage: storage,
        })
    }
}
#[derive(Clone, Copy, Eq, PartialEq)]
enum Stage {
    Unstarted,
    Enabling,
    Enabled,
    Uncertain,
    Disabling,
    Retired,
}
/// Keep this owner through retirement, even when enable/receive/retire futures
/// are dropped. Drop is not cleanup: abandoning a dirty owner poisons its factory.
#[must_use = "retain capture until interception is disabled or exact session destruction is confirmed"]
pub struct DragCapture {
    session: String,
    epoch: u64,
    factory: DragCaptureFactory,
    stage: Stage,
    pending: Option<DragToggleFuture>,
    reader: Option<EventRead>,
    data: Option<AdmittedDragData>,
    read_failed: bool,
    retiring: bool,
    _storage: ResourceLease,
}
impl DragCapture {
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn is_retired(&self) -> bool {
        self.stage == Stage::Retired
    }
    fn begin(&mut self, enabled: bool) {
        self.pending = Some(self.factory.0.client.clone().set_interception(enabled));
        self.stage = if enabled {
            Stage::Enabling
        } else {
            Stage::Disabling
        };
    }
    fn poll_native(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        let Poll::Ready(result) = self
            .pending
            .as_mut()
            .expect("retained drag toggle")
            .as_mut()
            .poll(cx)
        else {
            return Poll::Pending;
        };
        self.pending = None;
        self.stage = match (self.stage, result.is_ok()) {
            (Stage::Enabling, true) => Stage::Enabled,
            (Stage::Disabling, true) => Stage::Retired,
            (Stage::Enabling | Stage::Disabling, false) => Stage::Uncertain,
            _ => unreachable!("toggle stage"),
        };
        Poll::Ready(result)
    }
    pub async fn enable(&mut self) -> Result<()> {
        poll_fn(|cx| {
            if self.retiring {
                return Poll::Ready(Err(error("drag capture already retiring")));
            }
            match self.stage {
                Stage::Unstarted => {
                    // Fresh subscription discards prior intervals' queued history.
                    // No wire epoch exists: the caller must not overlap factories
                    // or reconnect this binding, nor treat late native events as
                    // authenticated solely by this local epoch number.
                    let receiver = match self.factory.0.client.subscribe() {
                        Ok(receiver) => receiver,
                        Err(error) => {
                            self.stage = Stage::Retired;
                            return Poll::Ready(Err(error));
                        }
                    };
                    let future = read(Some(receiver));
                    self.reader = Some(Box::pin(future));
                    self.begin(true);
                }
                Stage::Enabling => {}
                _ => return Poll::Ready(Err(error("drag enable already observed or failed"))),
            }
            self.poll_native(cx)
        })
        .await
    }
    /// First matching event wins, like upstream's resolved Promise. Await only
    /// when the listener observation expects drag. No timeout/executor is hidden
    /// here; the enclosing operation supplies cancellation/deadline supervision.
    /// Cancellation of this borrowed future leaves the subscription/read owned.
    pub async fn receive(&mut self) -> Result<AdmittedDragData> {
        poll_fn(|cx| {
            if self.retiring || self.stage != Stage::Enabled || self.read_failed {
                return Poll::Ready(Err(error("drag capture cannot receive in this state")));
            }
            if let Some(data) = &self.data {
                return Poll::Ready(Ok(data.clone()));
            }
            // Bounded work per poll even under unrelated-event traffic.
            for _ in 0..32 {
                let Poll::Ready((receiver, event)) = self
                    .reader
                    .as_mut()
                    .expect("capture reader")
                    .as_mut()
                    .poll(cx)
                else {
                    return Poll::Pending;
                };
                self.reader = None;
                let event = match event {
                    Ok(event) => event,
                    Err(failure) => {
                        self.read_failed = true;
                        return Poll::Ready(Err(error(match failure {
                            CdpEventRecvError::Closed => "drag event stream closed",
                            CdpEventRecvError::Lagged(_) => {
                                "drag event stream lagged; capture is incomplete"
                            }
                        })));
                    }
                };
                match self.factory.0.store.admit(&event, &self.session) {
                    Ok(Some(data)) => {
                        self.data = Some(data.clone());
                        // Drop this interval's cursor after first match; later
                        // events do not replace the Promise's captured value.
                        return Poll::Ready(Ok(data));
                    }
                    Ok(None) => self.reader = Some(Box::pin(read(Some(receiver)))),
                    Err(_) => {
                        self.read_failed = true;
                        return Poll::Ready(Err(error(
                            "drag intercepted data rejected by admission",
                        )));
                    }
                }
            }
            cx.waker().wake_by_ref();
            Poll::Pending
        })
        .await
    }
    /// Removes the subscription, drains any started enable, and then disables.
    /// Failed enable is ambiguous and still requires disable. Failed disable
    /// remains dirty and can be explicitly retried with this same owner.
    pub async fn retire(&mut self) -> Result<()> {
        poll_fn(|cx| {
            self.retiring = true;
            self.reader = None;
            loop {
                match self.stage {
                    Stage::Unstarted => {
                        self.stage = Stage::Retired;
                        return Poll::Ready(Ok(()));
                    }
                    Stage::Retired => return Poll::Ready(Ok(())),
                    Stage::Enabling => match self.poll_native(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(_) => {}
                    },
                    Stage::Enabled | Stage::Uncertain => self.begin(false),
                    Stage::Disabling => return self.poll_native(cx),
                }
            }
        })
        .await
    }
    /// Caller must first independently confirm destruction of this exact bound
    /// session, not transport loss or a matching ID on another connection. Arc
    /// identity prevents an acknowledgement for a replacement client binding.
    pub fn acknowledge_destroyed_session(
        &mut self,
        client: &Arc<dyn DragInterceptionClient>,
    ) -> Result<()> {
        if !Arc::ptr_eq(client, &self.factory.0.client) || self.pending.is_some() {
            return Err(error(
                "exact destroyed drag binding required with native calls drained",
            ));
        }
        self.reader = None;
        self.retiring = true;
        self.stage = Stage::Retired;
        Ok(())
    }
}
impl Drop for DragCapture {
    fn drop(&mut self) {
        let mut state = self
            .factory
            .0
            .state
            .lock()
            .expect("drag capture factory poisoned");
        if !matches!(self.stage, Stage::Unstarted | Stage::Retired) {
            state.poisoned = true;
        }
        state.busy = false;
    }
}

#[cfg(test)]
mod tests;
