//! Optional Chromium drag observation, capture, movement and Page event owners.
//! Mouse policy is opt-in via `mouse-dispatch`; no frame discovery or executor.
mod capture;
#[cfg(feature = "mouse-dispatch")]
pub mod mouse;
mod movement;
mod page;
pub use movement::{DragMove, DragWorld, DragWorldLifetime};
pub use page::{DragEventClient, DragEventKind, DragPage, DragPageOperation, DragPosition};
mod data;
pub use capture::{DragCapture, DragCaptureFactory, DragInterceptionClient, DragToggleFuture};
pub use data::{
    AdmittedDragData, DragDataAdmissionError, DragDataLimits, DragDataProfile, DragDataStore,
};
use pwright_cdp::{
    CdpClient,
    connection::{CdpError, Result},
};
use pwright_core::{
    ProductionResourceLedger, ResourceClass, ResourceLease, ResourceLedger, ResourceRequirements,
    ResourceScope,
};
use serde_json::{Value, json};
use std::{
    future::{Future, poll_fn},
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

const SETUP: &str = include_str!("setup.js");
const CLEANUP: &str = include_str!("cleanup.js");
const MAX_LISTENERS: usize = 32;
const MAX_NAMESPACE: usize = 64;
const MAX_CONTEXT: usize = 128;
type Native = Pin<Box<dyn Future<Output = Result<Value>> + Send>>;

/// Caller-supplied namespace must be unique among simultaneously live factories
/// using the same utility world. No global ID source or randomness is used.
#[derive(Clone)]
pub struct DragListenerFactory(Arc<Factory>);
struct Factory {
    namespace: String,
    scope: ResourceScope,
    capacity: usize,
    ledger: ResourceLedger,
    state: Mutex<FactoryState>,
    _storage: ResourceLease,
}
#[derive(Default)]
struct FactoryState {
    active: usize,
    next: u64,
}
impl DragListenerFactory {
    /// Partial payload accounting: fixed state/owners, token, wrapper future and
    /// outer argument-vector storage. Excludes shared client/world state,
    /// allocator overhead, JSON map-node layout, injected CDP future internals,
    /// replies and error trees. This is not a complete allocation-tree/RSS bound.
    pub fn requirements(capacity: usize) -> Result<ResourceRequirements> {
        if !(1..=MAX_LISTENERS).contains(&capacity) {
            return Err(error("drag listener capacity must be 1..=32"));
        }
        let bytes = state_bytes() + operation_bytes() * capacity;
        Ok(ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, bytes as u64, bytes as u64)
            .with_metadata(1, capacity + 1))
    }
    pub fn new(
        namespace: &str,
        scope: ResourceScope,
        capacity: usize,
        resources: &ProductionResourceLedger,
    ) -> Result<Self> {
        if namespace.is_empty()
            || namespace.len() > MAX_NAMESPACE
            || !namespace
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        {
            return Err(error(
                "drag namespace must be 1..=64 ASCII identifier bytes",
            ));
        }
        let ledger = resources
            .claim(Self::requirements(capacity)?)
            .map_err(|_| error("drag listener resource claim failed"))?;
        let storage = ledger
            .reserve(
                scope,
                ResourceClass::BackendStateBytes,
                state_bytes() as u64,
            )
            .map_err(|_| error("drag listener state admission failed"))?;
        Ok(Self(Arc::new(Factory {
            namespace: namespace.to_owned(),
            scope,
            capacity,
            ledger,
            state: Mutex::new(FactoryState::default()),
            _storage: storage,
        })))
    }
    pub fn active_listeners(&self) -> usize {
        self.0.state.lock().expect("drag factory poisoned").active
    }
    /// Reserves before cloning context/token or allocating the owner future.
    /// The exact unique context must be an already selected utility world.
    /// This method neither creates a world nor sends a native command.
    pub fn start(&self, client: Arc<dyn CdpClient>, unique_context: &str) -> Result<DragListener> {
        if unique_context.is_empty() || unique_context.len() > MAX_CONTEXT {
            return Err(error("drag unique context must be 1..=128 bytes"));
        }
        let mut state = self.0.state.lock().expect("drag factory poisoned");
        if state.active == self.0.capacity {
            return Err(error("drag listener capacity exceeded"));
        }
        let next = state
            .next
            .checked_add(1)
            .ok_or_else(|| error("drag listener identifiers exhausted"))?;
        let storage = self
            .0
            .ledger
            .reserve(
                self.0.scope,
                ResourceClass::BackendStateBytes,
                operation_bytes() as u64,
            )
            .map_err(|_| error("drag listener operation admission failed"))?;
        state.next = next;
        state.active += 1;
        drop(state);
        Ok(DragListener {
            client,
            context: Arc::from(unique_context),
            token: Arc::from(format!("{}:{next}", self.0.namespace)),
            stage: Stage::Unstarted,
            pending: None,
            cleanup_mode: false,
            retiring: false,
            _admission: Admission {
                factory: self.clone(),
                storage: Some(storage),
            },
        })
    }
}
fn state_bytes() -> usize {
    std::mem::size_of::<Factory>() + 2 * std::mem::size_of::<usize>() + MAX_NAMESPACE
}
fn operation_bytes() -> usize {
    // The async wrapper layout is independent of argument values. No provider
    // method is called while measuring it.
    std::mem::size_of::<DragListener>()
        + std::mem::size_of::<NativeArgs>()
        + 2 * (MAX_NAMESPACE + 21)
        + MAX_CONTEXT
        + 4 * std::mem::size_of::<usize>()
        + 2 * std::mem::size_of::<Value>()
        + std::mem::size_of_val(&native(None, false, false))
}
struct Admission {
    factory: DragListenerFactory,
    storage: Option<ResourceLease>,
}
impl Drop for Admission {
    fn drop(&mut self) {
        drop(self.storage.take());
        self.factory
            .0
            .state
            .lock()
            .expect("drag factory poisoned")
            .active -= 1;
    }
}
struct NativeArgs {
    client: Arc<dyn CdpClient>,
    context: Arc<str>,
    token: Arc<str>,
}
async fn native(args: Option<NativeArgs>, setup: bool, observe: bool) -> Result<Value> {
    let args = args.expect("native wrapper arguments");
    let arguments = if setup {
        vec![json!({"value": args.token.as_ref()})]
    } else {
        vec![
            json!({"value": args.token.as_ref()}),
            json!({"value": observe}),
        ]
    };
    args.client
        .runtime_call_function_by_value_in_unique_context(
            &args.context,
            if setup { SETUP } else { CLEANUP },
            arguments,
        )
        .await
}
fn error(message: &str) -> CdpError {
    CdpError::Other(message.into())
}
fn boolean(value: Value) -> Result<bool> {
    value
        .get("result")
        .and_then(|value| value.get("value"))
        .and_then(Value::as_bool)
        .ok_or_else(|| error("drag listener returned a non-boolean envelope"))
}
#[derive(Clone, Copy, Eq, PartialEq)]
enum Stage {
    Unstarted,
    Preparing,
    Prepared,
    SetupUncertain,
    Cleaning,
    CleanupUncertain,
    Retired,
}

/// Caller-owned native/listener lifetime, outside cancellable result futures.
/// `prepare` installs one listener set; `observe` reads drag-start status while
/// removing it. `retire` stops observation and removes any potentially installed
/// listeners. Failed cleanup leaves `is_retired == false`; retain this owner for
/// explicit retry or independently confirmed destruction of its exact world.
/// Dropping the owner is NOT proof of native cleanup.
#[must_use = "retain drag listeners through confirmed cleanup or exact world destruction"]
pub struct DragListener {
    client: Arc<dyn CdpClient>,
    context: Arc<str>,
    token: Arc<str>,
    stage: Stage,
    pending: Option<Native>,
    cleanup_mode: bool,
    retiring: bool,
    _admission: Admission,
}
impl DragListener {
    pub fn is_retired(&self) -> bool {
        self.stage == Stage::Retired
    }
    pub fn unique_context(&self) -> &str {
        &self.context
    }
    fn begin(&mut self, setup: bool, observe: bool) {
        self.pending = Some(Box::pin(native(
            Some(NativeArgs {
                client: self.client.clone(),
                context: self.context.clone(),
                token: self.token.clone(),
            }),
            setup,
            observe,
        )));
        self.cleanup_mode = observe;
        self.stage = if setup {
            Stage::Preparing
        } else {
            Stage::Cleaning
        };
    }
    fn poll_native(&mut self, cx: &mut Context<'_>) -> Poll<Result<bool>> {
        let result = self
            .pending
            .as_mut()
            .expect("retained native call")
            .as_mut()
            .poll(cx);
        let Poll::Ready(result) = result else {
            return Poll::Pending;
        };
        self.pending = None;
        let result = result.and_then(boolean);
        self.stage = match (self.stage, &result) {
            (Stage::Preparing, Ok(true)) => Stage::Prepared,
            // The script's explicit false is a no-install collision/capacity
            // receipt. Cleaning this token could delete another owner's entry.
            (Stage::Preparing, Ok(false)) => Stage::Retired,
            (Stage::Preparing, _) => Stage::SetupUncertain,
            (Stage::Cleaning, Ok(_)) => Stage::Retired,
            (Stage::Cleaning, Err(_)) => Stage::CleanupUncertain,
            _ => unreachable!("native operation stage"),
        };
        Poll::Ready(result)
    }
    pub async fn prepare(&mut self) -> Result<()> {
        poll_fn(|cx| {
            if self.retiring {
                return Poll::Ready(Err(error("drag listener retirement already requested")));
            }
            match self.stage {
                Stage::Unstarted => self.begin(true, false),
                Stage::Preparing => {}
                _ => return Poll::Ready(Err(error("drag prepare already observed or cancelled"))),
            }
            self.poll_native(cx).map(|result| {
                result.and_then(|installed| {
                    if installed {
                        Ok(())
                    } else {
                        Err(error("drag listener setup did not confirm installation"))
                    }
                })
            })
        })
        .await
    }
    pub async fn observe(&mut self) -> Result<bool> {
        poll_fn(|cx| {
            if self.retiring {
                return Poll::Ready(Err(error("drag listener retirement already requested")));
            }
            match self.stage {
                Stage::Prepared => self.begin(false, true),
                Stage::Cleaning if self.cleanup_mode => {}
                _ => {
                    return Poll::Ready(Err(error("drag observation requires prepared listeners")));
                }
            }
            self.poll_native(cx)
        })
        .await
    }
    pub async fn retire(&mut self) -> Result<()> {
        poll_fn(|cx| {
            self.retiring = true;
            loop {
                match self.stage {
                    Stage::Unstarted => {
                        self.stage = Stage::Retired;
                        return Poll::Ready(Ok(()));
                    }
                    Stage::Retired => return Poll::Ready(Ok(())),
                    Stage::Preparing => match self.poll_native(cx) {
                        Poll::Pending => return Poll::Pending,
                        // A failed setup response cannot prove no listener was
                        // installed. Always issue cleanup for that exact token.
                        Poll::Ready(_) => {}
                    },
                    Stage::Prepared | Stage::SetupUncertain | Stage::CleanupUncertain => {
                        self.begin(false, false)
                    }
                    Stage::Cleaning => {
                        return self.poll_native(cx).map(|result| result.map(|_| ()));
                    }
                }
            }
        })
        .await
    }
    /// The caller must obtain authoritative destruction evidence for this exact
    /// unique world, not merely a reused numeric context or lost transport.
    /// Pending native calls must still be drained before this acknowledgement.
    pub fn acknowledge_destroyed_world(&mut self, unique_context: &str) -> Result<()> {
        if unique_context != self.context.as_ref() || self.pending.is_some() {
            return Err(error(
                "exact destroyed world required with no pending native call",
            ));
        }
        self.stage = Stage::Retired;
        self.retiring = true;
        Ok(())
    }

    // The enclosing movement has matched independent destruction evidence to
    // this exact client/unique-world binding. Still drain an already-started
    // native receipt; never start new cleanup in the destroyed world.
    fn poll_retire_destroyed(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        self.retiring = true;
        if self.pending.is_some() && self.poll_native(cx).is_pending() {
            return Poll::Pending;
        }
        self.stage = Stage::Retired;
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests;
