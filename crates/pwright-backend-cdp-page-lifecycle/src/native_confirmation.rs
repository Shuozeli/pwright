//! Selected exact-target destruction confirmation, with bounded live ownership.
//! Discovery is connection-local and enabled before the first close mutation.
use crate::{CdpPageCloseConfirmation, CdpPreparedPageClose, PageCloseFuture};
use pwright_cap_page_lifecycle::PageLifecycleErrorDetails;
use pwright_cdp::{CdpClient, CdpConnection, CdpEventReceiver, CdpEventRecvError, CdpSession};
use pwright_core::{
    FailureKind, OperationError, ProductionResourceLedger, ResourceClass, ResourceLease,
    ResourceLedger, ResourceRequirements, ResourceScope,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::{
    future::{Future, poll_fn},
    pin::{Pin, pin},
    task::Poll,
};

pub type CloseInterruptionFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
pub const INTERRUPTION_FUTURE_BYTES: usize = 1024;

/// Executor-neutral owner termination. The returned future's box must fit the
/// declared allowance, already included in each confirmation admission.
pub trait NativeCloseInterruption: Send + Sync {
    fn is_interrupted(&self) -> bool;
    fn interrupted(&self) -> Result<CloseInterruptionFuture, Error>;
}

type Error = OperationError<PageLifecycleErrorDetails>;
const STATE_BYTES: u64 = 4096;
const CALL_BYTES: u64 = 16 * 1024;
const ERROR_BYTES: usize = 4096;
mod diagnostic;
use diagnostic::backend_error;
mod split;
pub use split::{
    AcknowledgedNativePageClose, NativePageCloseCommandFuture, PreparedNativePageCloseCommand,
};

#[derive(Clone, Copy)]
pub struct NativePageCloseProfile {
    scope: ResourceScope,
    max_pending: u16,
    native_id_bytes: u16,
    owner_bytes: u32,
}

impl NativePageCloseProfile {
    pub fn new(
        scope: ResourceScope,
        max_pending: u16,
        native_id_bytes: u16,
    ) -> Result<Self, Error> {
        if max_pending == 0 || native_id_bytes == 0 || native_id_bytes > 1024 {
            return Err(failure(
                FailureKind::ProtocolViolation,
                "invalid native close confirmation limits",
            ));
        }
        Ok(Self {
            scope,
            max_pending,
            native_id_bytes,
            owner_bytes: 0,
        })
    }
    pub fn requirements(self) -> ResourceRequirements {
        let bytes =
            STATE_BYTES + u64::from(self.owner_bytes) + CALL_BYTES * u64::from(self.max_pending);
        ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, usize::from(self.max_pending) + 1)
    }
    /// Include the selecting composition's measured wrapper/Arc allocation.
    pub fn with_installation_owner_bytes(mut self, bytes: usize) -> Result<Self, Error> {
        self.owner_bytes = u32::try_from(bytes).map_err(|_| {
            failure(
                FailureKind::ProtocolViolation,
                "native close installation size exceeds u32",
            )
        })?;
        Ok(self)
    }
    pub const fn event_subscribers(self) -> usize {
        self.max_pending as usize
    }
}

struct State {
    connection: Arc<CdpConnection>,
    ledger: ResourceLedger,
    profile: NativePageCloseProfile,
    discovery_enabled: AtomicBool,
    pending: AtomicUsize,
    interruption: Arc<dyn NativeCloseInterruption>,
    _storage: ResourceLease,
}

#[derive(Clone)]
pub struct NativePageCloseConfirmation(Arc<State>);

impl NativePageCloseConfirmation {
    pub fn new<Interruption: NativeCloseInterruption + 'static>(
        connection: Arc<CdpConnection>,
        resources: &ProductionResourceLedger,
        profile: NativePageCloseProfile,
        interruption: Interruption,
    ) -> Result<Self, Error> {
        if std::mem::size_of::<State>()
            + std::mem::size_of::<Interruption>()
            + 4 * std::mem::size_of::<usize>()
            > STATE_BYTES as usize
        {
            return Err(failure(
                FailureKind::ResourceExhausted,
                "native close source exceeds state budget",
            ));
        }
        let ledger = resources
            .claim(profile.requirements())
            .map_err(resource_error)?;
        let storage = ledger
            .reserve(
                profile.scope,
                ResourceClass::BackendStateBytes,
                STATE_BYTES + u64::from(profile.owner_bytes),
            )
            .map_err(resource_error)?;
        Ok(Self(Arc::new(State {
            connection,
            ledger,
            profile,
            discovery_enabled: AtomicBool::new(false),
            pending: AtomicUsize::new(0),
            interruption: Arc::new(interruption),
            _storage: storage,
        })))
    }
}

impl NativePageCloseConfirmation {
    fn prepare_owner(&self, target: &str) -> Result<Box<Prepared>, Error> {
        if target.is_empty() || target.len() > usize::from(self.0.profile.native_id_bytes) {
            return Err(failure(
                FailureKind::ProtocolViolation,
                "invalid native close target identity",
            ));
        }
        if std::mem::size_of::<Prepared>() + target.len() + ERROR_BYTES > CALL_BYTES as usize {
            return Err(failure(
                FailureKind::ResourceExhausted,
                "native close owner exceeds admission",
            ));
        }
        admit_pending(&self.0.pending, usize::from(self.0.profile.max_pending))?;
        let slot = PendingSlot(self.0.clone());
        let storage = self
            .0
            .ledger
            .reserve(
                self.0.profile.scope,
                ResourceClass::BackendStateBytes,
                CALL_BYTES,
            )
            .map_err(resource_error)?;
        let events =
            self.0
                .connection
                .subscribe_composable_events()
                .map_err(|error| match error {
                    pwright_cdp::CdpEventSubscriptionError::ReclaimingFanout(
                        pwright_cdp::event_fanout::ReclaimingFanoutSubscribeError::Closed,
                    ) => failure(
                        FailureKind::TransportClosed,
                        "native close source connection is closed",
                    ),
                    other => resource_error(other),
                })?;
        // Both allocations follow admission; receiver cursor capacity belongs to
        // the selected connection fanout and is reserved separately in its plan.
        Ok(Box::new(Prepared {
            slot,
            target: target.into(),
            events,
            _storage: storage,
        }))
    }
}

impl CdpPageCloseConfirmation for NativePageCloseConfirmation {
    fn prepare(&self, target: &str) -> Result<Box<dyn CdpPreparedPageClose>, Error> {
        Ok(self.prepare_owner(target)?)
    }
}

struct Prepared {
    slot: PendingSlot,
    target: Box<str>,
    events: CdpEventReceiver,
    _storage: ResourceLease,
}

fn admit_pending(pending: &AtomicUsize, limit: usize) -> Result<(), Error> {
    pending
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
            if current < limit {
                Some(current + 1)
            } else {
                None
            }
        })
        .map(|_| ())
        .map_err(|_| {
            failure(
                FailureKind::ResourceExhausted,
                "native close confirmation capacity exhausted",
            )
        })
}
struct PendingSlot(Arc<State>);
impl Drop for PendingSlot {
    fn drop(&mut self) {
        self.0.pending.fetch_sub(1, Ordering::AcqRel);
    }
}

impl CdpPreparedPageClose for Prepared {
    fn start(self: Box<Self>) -> Result<PageCloseFuture, Error> {
        let retained = std::mem::size_of::<Self>()
            + self.target.len()
            + ERROR_BYTES
            + INTERRUPTION_FUTURE_BYTES;
        let interruption = self.slot.0.interruption.interrupted()?;
        let future = async move {
            if self.slot.0.interruption.is_interrupted() {
                return Err(failure(
                    FailureKind::Cancelled,
                    "native close owner retired before execution",
                ));
            }
            if !self.slot.0.discovery_enabled.load(Ordering::Acquire) {
                // Idempotent under concurrent first preparations. A failed
                // enable never marks the source ready or sends Target.closeTarget.
                self.slot.0.connection.send_command("Target.setDiscoverTargets", serde_json::json!({
                    "discover": true, "filter": [{"type": "page", "exclude": false}, {"exclude": true}]
                })).await.map_err(|error| backend_error("Target.setDiscoverTargets", error))?;
                self.slot.0.discovery_enabled.store(true, Ordering::Release);
            }
            let browser = CdpSession::browser(self.slot.0.connection.clone());
            let mut owner = self;
            close_with_receipt_and_interruption(
                &browser,
                &owner.target,
                &mut owner.events,
                Some(interruption),
            )
            .await
        };
        if retained + std::mem::size_of_val(&future) > CALL_BYTES as usize {
            return Err(failure(
                FailureKind::ResourceExhausted,
                "native close future exceeds admission",
            ));
        }
        Ok(Box::pin(future))
    }
}

#[cfg(test)]
async fn close_with_receipt(
    browser: &dyn CdpClient,
    target: &str,
    events: &mut CdpEventReceiver,
) -> Result<(), Error> {
    close_with_receipt_and_interruption(browser, target, events, None).await
}

async fn close_with_receipt_and_interruption(
    browser: &dyn CdpClient,
    target: &str,
    events: &mut CdpEventReceiver,
    interruption: Option<CloseInterruptionFuture>,
) -> Result<(), Error> {
    // Preserve target-closed errors rather than fabricating a native success:
    // the host may suppress them after authoritative topology reconciliation.
    browser
        .target_close(target)
        .await
        .map_err(|error| backend_error("Target.closeTarget", error))?;
    wait_for_receipt(target, events, interruption).await
}

async fn wait_for_receipt(
    target: &str,
    events: &mut CdpEventReceiver,
    mut interruption: Option<CloseInterruptionFuture>,
) -> Result<(), Error> {
    loop {
        let mut receive = pin!(events.recv());
        let received = poll_fn(|context| {
            if interruption
                .as_mut()
                .is_some_and(|stop| stop.as_mut().poll(context).is_ready())
            {
                return Poll::Ready(Err(failure(
                    FailureKind::Cancelled,
                    "native close owner retired while awaiting confirmation",
                )));
            }
            receive.as_mut().poll(context).map(Ok)
        })
        .await?;
        let event = received.map_err(|error| match error {
            CdpEventRecvError::Closed => failure(
                FailureKind::TargetClosed,
                "native close receipt stream ended",
            ),
            CdpEventRecvError::Lagged(_) => failure(
                FailureKind::Backend,
                "native close receipt stream lost events",
            ),
        })?;
        if event.method != "Target.targetDestroyed" || event.session_id.is_some() {
            continue;
        }
        let id = event
            .params
            .get("targetId")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                failure(
                    FailureKind::Backend,
                    "malformed native target destruction receipt",
                )
            })?;
        if id == target {
            return Ok(());
        }
    }
}

fn failure(kind: FailureKind, message: &str) -> Error {
    OperationError::new(
        kind,
        message,
        Some(PageLifecycleErrorDetails::Backend(message.into())),
    )
}
fn resource_error(error: impl std::fmt::Display) -> Error {
    failure(FailureKind::ResourceExhausted, &diagnostic::bounded(error))
}

#[cfg(test)]
mod tests;
