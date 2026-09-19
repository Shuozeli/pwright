//! Private utility-world marker storage, independent of temporary target groups.
//! This primitive has no Handle registry, dispatcher, deadline or DOM preflight.
//! Use its cancellation-safe composition: dropping started raw work fails closed.

mod diagnostic;
#[cfg(test)]
mod tests;

use super::MAX_NATIVE_ID;
use diagnostic::{DIAGNOSTIC_BYTES, bounded_error, bounded_text};
use pwright_backend_cdp_locator_resolution::{ObjectGroupDestructionProof, ResolvedElement};
use pwright_cdp::{CdpClient, connection::CdpError};
use pwright_core::{
    ObjectIdSource, ProductionResourceError, ProductionResourceLedger, ResourceClass,
    ResourceError, ResourceLease, ResourceLedger, ResourceRequest, ResourceRequirements,
    ResourceScope,
};
use serde_json::{Value, json};
use std::{
    fmt,
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

const CREATE_SCRIPT: &str = include_str!("target_marker_create.js");
const MARK_SCRIPT: &str = include_str!("target_marker_mark.js");
const MAX_CALL_ID: usize = 128;
const NATIVE_UNITS: u64 = 2;
const ADMIN_BYTES: u64 = 512;
const CONTROL_BYTES: usize = 8192;
// Retained IDs plus overlapping typed/JSON parameters, bounded primary and
// cleanup diagnostics (including String -> Box overlap), and checked futures /
// fixed-shape container backing. Transport frames/reply envelopes are separately
// transport-owned; arbitrary DOM graphs kept by the native Set are not an RSS bound.
const STATE_BYTES: u64 =
    (7 * MAX_NATIVE_ID + 2 * CREATE_SCRIPT.len() + 4 * DIAGNOSTIC_BYTES + CONTROL_BYTES) as u64;
const CALL_BYTES: u64 = (8 * MAX_NATIVE_ID
    + 2 * MAX_CALL_ID
    + 2 * MARK_SCRIPT.len()
    + 4 * DIAGNOSTIC_BYTES
    + CONTROL_BYTES) as u64;

#[derive(Debug)]
pub enum TargetMarkingError {
    InvalidProfile,
    InvalidWorld,
    InvalidTarget,
    InvalidCallId,
    IdentifierExhausted,
    Capacity,
    AlreadyUsed,
    Unhealthy,
    Protocol(&'static str),
    Backend(CdpError),
    JavaScript(Box<str>),
    Cleanup {
        primary: Option<Box<TargetMarkingError>>,
        source: CdpError,
    },
    Budget(ResourceError),
    Plan(ProductionResourceError),
}
impl TargetMarkingError {
    pub fn is_cleanup_failure(&self) -> bool {
        matches!(self, Self::Cleanup { .. })
    }
}
impl fmt::Display for TargetMarkingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProfile => f.write_str("invalid retained target-marker profile"),
            Self::InvalidWorld => f.write_str("target marker requires a bounded unique utility world"),
            Self::InvalidTarget => f.write_str("target marker requires the exact session/world and a settlement-owned target group"),
            Self::InvalidCallId => f.write_str("target-marker call ID exceeds selected capacity"),
            Self::IdentifierExhausted => f.write_str("target-marker object-group IDs exhausted"),
            Self::Capacity => f.write_str("target-marker capacity exhausted"),
            Self::AlreadyUsed => f.write_str("target-marker admission was already used"),
            Self::Unhealthy => f.write_str("a prior target-marker cleanup failed or was abandoned"),
            Self::Protocol(message) => f.write_str(message),
            Self::Backend(error) => error.fmt(f),
            Self::JavaScript(message) => f.write_str(message),
            Self::Cleanup { primary, source } => {
                if let Some(primary) = primary { write!(f, "{primary}; ")?; }
                write!(f, "target-marker cleanup failed: {source}")
            }
            Self::Budget(error) => error.fmt(f),
            Self::Plan(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for TargetMarkingError {}

#[derive(Clone, Copy, Debug)]
pub struct TargetMarkingProfile {
    scope: ResourceScope,
    states: usize,
    calls: usize,
    bytes: u64,
}
impl TargetMarkingProfile {
    pub fn new(
        scope: ResourceScope,
        states: usize,
        calls: usize,
    ) -> Result<Self, TargetMarkingError> {
        let count = states
            .checked_add(calls)
            .ok_or(TargetMarkingError::InvalidProfile)?;
        let bytes = u64::try_from(states)
            .ok()
            .and_then(|n| n.checked_mul(STATE_BYTES))
            .and_then(|n| n.checked_add(u64::try_from(calls).ok()?.checked_mul(CALL_BYTES)?))
            .and_then(|n| n.checked_add(ADMIN_BYTES))
            .ok_or(TargetMarkingError::InvalidProfile)?;
        if states == 0
            || calls == 0
            || count.checked_add(1).is_none()
            || u64::try_from(count)
                .ok()
                .and_then(|n| n.checked_mul(NATIVE_UNITS))
                .is_none()
        {
            return Err(TargetMarkingError::InvalidProfile);
        }
        Ok(Self {
            scope,
            states,
            calls,
            bytes,
        })
    }
    pub fn requirements(self) -> ResourceRequirements {
        let count = self.states + self.calls;
        ResourceRequirements::none()
            .with_class(
                ResourceClass::RemoteHandle,
                count as u64 * NATIVE_UNITS,
                count as u64 * NATIVE_UNITS,
            )
            .with_class(ResourceClass::BackendStateBytes, self.bytes, self.bytes)
            .with_metadata(1, count + 1)
    }
}

/// Caller-owned exact-world receipt. The optional proof must cover this unique
/// world on THIS client/connection epoch, not route replacement or error text.
/// Its backing is admitted by its supplying composition, not this small descriptor.
pub struct MarkerWorld {
    pub client: Arc<dyn CdpClient>,
    pub utility_world: Arc<str>,
    pub destruction: Option<Arc<dyn ObjectGroupDestructionProof>>,
}

struct Control {
    healthy: AtomicBool,
    states: AtomicUsize,
    calls: AtomicUsize,
    _admin: ResourceLease,
}
struct Slot {
    control: Arc<Control>,
    state: bool,
}
impl Drop for Slot {
    fn drop(&mut self) {
        let counter = if self.state {
            &self.control.states
        } else {
            &self.control.calls
        };
        counter.fetch_sub(1, Ordering::AcqRel);
    }
}
struct CleanupGuard {
    control: Arc<Control>,
    armed: bool,
    proof: Option<Arc<dyn ObjectGroupDestructionProof>>,
}
impl Drop for CleanupGuard {
    fn drop(&mut self) {
        if self.armed
            && !self
                .proof
                .as_deref()
                .is_some_and(ObjectGroupDestructionProof::all_group_objects_retired)
        {
            self.control.healthy.store(false, Ordering::Release);
        }
    }
}

pub struct CdpTargetMarkingService {
    profile: TargetMarkingProfile,
    ledger: ResourceLedger,
    ids: Arc<dyn ObjectIdSource>,
    control: Arc<Control>,
}
impl CdpTargetMarkingService {
    /// All installations sharing a native session must share the same checked,
    /// never-reused ID source. Group names are scoped to that native session.
    pub fn new(
        profile: TargetMarkingProfile,
        resources: &ProductionResourceLedger,
        ids: Arc<dyn ObjectIdSource>,
    ) -> Result<Self, TargetMarkingError> {
        let ledger = resources
            .claim(profile.requirements())
            .map_err(TargetMarkingError::Plan)?;
        let admin = ledger
            .reserve(profile.scope, ResourceClass::BackendStateBytes, ADMIN_BYTES)
            .map_err(TargetMarkingError::Budget)?;
        const {
            assert!(
                std::mem::size_of::<Control>()
                    + 2 * std::mem::size_of::<usize>()
                    + std::mem::size_of::<CdpTargetMarkingService>()
                    <= ADMIN_BYTES as usize
            );
        }
        Ok(Self {
            profile,
            ledger,
            ids,
            control: Arc::new(Control {
                healthy: AtomicBool::new(true),
                states: AtomicUsize::new(0),
                calls: AtomicUsize::new(0),
                _admin: admin,
            }),
        })
    }
    pub fn is_cleanup_healthy(&self) -> bool {
        self.control.healthy.load(Ordering::Acquire)
    }

    fn reserve(&self, state: bool) -> Result<(ResourceLease, Slot), TargetMarkingError> {
        if !self.is_cleanup_healthy() {
            return Err(TargetMarkingError::Unhealthy);
        }
        let (counter, capacity, bytes) = if state {
            (&self.control.states, self.profile.states, STATE_BYTES)
        } else {
            (&self.control.calls, self.profile.calls, CALL_BYTES)
        };
        counter
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < capacity).then(|| n + 1)
            })
            .map_err(|_| TargetMarkingError::Capacity)?;
        let slot = Slot {
            control: self.control.clone(),
            state,
        };
        let lease = self
            .ledger
            .reserve_many(
                self.profile.scope,
                &[
                    ResourceRequest::new(ResourceClass::RemoteHandle, NATIVE_UNITS),
                    ResourceRequest::new(ResourceClass::BackendStateBytes, bytes),
                ],
            )
            .map_err(TargetMarkingError::Budget)?;
        Ok((lease, slot))
    }

    /// Admission happens synchronously before the future exists. Unpolled Drop
    /// creates no native owner; after dispatch a raw dropped future poisons this
    /// service. A composing cleanup executor must retain it through settlement.
    pub fn create(
        &self,
        world: MarkerWorld,
    ) -> Result<
        impl Future<Output = Result<MarkerState, TargetMarkingError>> + Send + 'static,
        TargetMarkingError,
    > {
        if world.utility_world.is_empty() || world.utility_world.len() > MAX_NATIVE_ID {
            return Err(TargetMarkingError::InvalidWorld);
        }
        let (lease, slot) = self.reserve(true)?;
        let id = self
            .ids
            .next_object_id()
            .map_err(|_| TargetMarkingError::IdentifierExhausted)?;
        let state = MarkerState {
            cleanup: CleanupGuard {
                control: self.control.clone(),
                armed: false,
                proof: world.destruction.clone(),
            },
            world,
            group: format!("pwright.target-marker.{}", id.get()),
            object_id: Box::default(),
            _lease: lease,
            _slot: slot,
        };
        let future = async move {
            let mut state = state;
            if !state.cleanup.control.healthy.load(Ordering::Acquire) {
                return Err(TargetMarkingError::Unhealthy);
            }
            state.cleanup.armed = true;
            let acquired = state
                .world
                .client
                .runtime_call_function_as_object_in_group_to_settlement(
                    &state.world.utility_world,
                    CREATE_SCRIPT,
                    Vec::new(),
                    &state.group,
                    true,
                )
                .await
                .map_err(|error| TargetMarkingError::Backend(bounded_error(error)))
                .and_then(|response| parse_state(&response));
            match acquired {
                Ok(id) => {
                    state.object_id = id;
                    Ok(state)
                }
                Err(primary) => match state.release_inner().await {
                    Ok(()) => Err(primary),
                    Err(source) => Err(TargetMarkingError::Cleanup {
                        primary: Some(Box::new(primary)),
                        source,
                    }),
                },
            }
        };
        check_storage(&future);
        Ok(future)
    }

    pub fn admit(&self) -> Result<MarkerAdmission, TargetMarkingError> {
        let (lease, slot) = self.reserve(false)?;
        Ok(MarkerAdmission {
            cleanup: CleanupGuard {
                control: self.control.clone(),
                armed: false,
                proof: None,
            },
            used: false,
            _lease: lease,
            _slot: slot,
        })
    }
}

/// Unique persistent group owner. Its state intentionally outlives individual
/// marker calls and adopted targets; no per-call release clears the native Set.
#[must_use = "release the marker group or use the cancellation-safe composition owner"]
pub struct MarkerState {
    // Poison before returning the owner's resources on every abandoned path.
    cleanup: CleanupGuard,
    world: MarkerWorld,
    group: String,
    object_id: Box<str>,
    _lease: ResourceLease,
    // Keep shared admission administration alive until after the owner lease.
    _slot: Slot,
}
impl MarkerState {
    pub fn object_id(&self) -> &str {
        &self.object_id
    }
    pub fn utility_world(&self) -> &str {
        &self.world.utility_world
    }
    pub fn object_group(&self) -> &str {
        &self.group
    }
    pub fn client(&self) -> Arc<dyn CdpClient> {
        self.world.client.clone()
    }
    async fn release_inner(&mut self) -> Result<(), CdpError> {
        let proof = self.world.destruction.as_deref();
        if let Some(proof) = proof {
            std::future::poll_fn(|cx| proof.poll_release_ready(cx)).await;
        }
        if proof.is_some_and(ObjectGroupDestructionProof::all_group_objects_retired) {
            self.cleanup.armed = false;
            return Ok(());
        }
        let result = self
            .world
            .client
            .runtime_release_object_group_to_settlement(&self.group)
            .await
            .map_err(bounded_error);
        if let Some(proof) = proof {
            std::future::poll_fn(|cx| proof.poll_release_ready(cx)).await;
        }
        if result.is_ok()
            || proof.is_some_and(ObjectGroupDestructionProof::all_group_objects_retired)
        {
            self.cleanup.armed = false;
            Ok(())
        } else {
            result
        }
    }
    pub fn release(
        mut self,
    ) -> impl Future<Output = Result<(), TargetMarkingError>> + Send + 'static {
        let future = async move {
            self.release_inner()
                .await
                .map_err(|source| TargetMarkingError::Cleanup {
                    primary: None,
                    source,
                })
        };
        check_storage(&future);
        future
    }
}

/// Two possible wrappers from one raw call belong to the adopted target group.
/// This admission must outlive that group's acknowledged/authoritative cleanup.
pub struct MarkerAdmission {
    cleanup: CleanupGuard,
    used: bool,
    _lease: ResourceLease,
    _slot: Slot,
}
impl MarkerAdmission {
    /// Only call after successful release/authoritative retirement of the ENTIRE
    /// adopted group. This does not release any browser object by itself.
    pub fn complete_cleanup(&mut self) {
        self.cleanup.armed = false;
    }

    pub fn mark<'a>(
        &'a mut self,
        state: &'a MarkerState,
        target: &'a ResolvedElement,
        call_id: &'a str,
    ) -> impl Future<Output = Result<(), TargetMarkingError>> + Send + 'a {
        let used = std::mem::replace(&mut self.used, true);
        let future = async move {
            if used {
                return Err(TargetMarkingError::AlreadyUsed);
            }
            if !self.cleanup.control.healthy.load(Ordering::Acquire) {
                return Err(TargetMarkingError::Unhealthy);
            }
            if !Arc::ptr_eq(&self.cleanup.control, &state.cleanup.control)
                || !Arc::ptr_eq(&state.world.client, &target.client())
                || state.utility_world() != target.utility_world()
                || !target.uses_settlement_release()
                || target.object_group() == state.object_group()
                || target.object_id().is_empty()
                || target.object_id().len() > MAX_NATIVE_ID
                || target.object_group().is_empty()
                || target.object_group().len() > MAX_NATIVE_ID
            {
                return Err(TargetMarkingError::InvalidTarget);
            }
            if call_id.len() > MAX_CALL_ID {
                return Err(TargetMarkingError::InvalidCallId);
            }
            if call_id.is_empty() {
                return Ok(());
            }
            self.cleanup.armed = true;
            let response = state
                .world
                .client
                .runtime_call_function_raw_in_group_to_settlement(
                    state.utility_world(),
                    MARK_SCRIPT,
                    vec![
                        json!({"objectId":state.object_id()}),
                        json!({"objectId":target.object_id()}),
                        json!({"value":call_id}),
                    ],
                    target.object_group(),
                    true,
                )
                .await
                .map_err(|error| TargetMarkingError::Backend(bounded_error(error)))?;
            check_exception(&response)?;
            let result = response
                .get("result")
                .ok_or(TargetMarkingError::Protocol("missing target-marker result"))?;
            if result.get("type").and_then(Value::as_str) != Some("undefined")
                || result.get("objectId").is_some()
                || result.get("value").is_some()
                || result.get("unserializableValue").is_some()
            {
                return Err(TargetMarkingError::Protocol(
                    "target marker must return undefined",
                ));
            }
            Ok(())
        };
        check_storage(&future);
        future
    }
}

fn check_storage<F>(_: &F) {
    const {
        assert!(
            std::mem::size_of::<F>()
                + std::mem::size_of::<MarkerState>()
                + std::mem::size_of::<MarkerAdmission>()
                + 2 * std::mem::size_of::<TargetMarkingError>()
                + 4096
                < CONTROL_BYTES
        );
    }
}
fn check_exception(response: &Value) -> Result<(), TargetMarkingError> {
    if let Some(exception) = response.get("exceptionDetails") {
        let message = exception
            .pointer("/exception/description")
            .and_then(Value::as_str)
            .or_else(|| exception.get("text").and_then(Value::as_str))
            .unwrap_or("target-marker JavaScript evaluation threw");
        return Err(TargetMarkingError::JavaScript(bounded_text(message)));
    }
    Ok(())
}
fn parse_state(response: &Value) -> Result<Box<str>, TargetMarkingError> {
    check_exception(response)?;
    let result = response
        .get("result")
        .ok_or(TargetMarkingError::Protocol("missing target-marker state"))?;
    if result.get("type").and_then(Value::as_str) != Some("object")
        || result.get("subtype").is_some()
    {
        return Err(TargetMarkingError::Protocol(
            "target-marker state is not a plain object",
        ));
    }
    result
        .get("objectId")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty() && id.len() <= MAX_NATIVE_ID)
        .map(Into::into)
        .ok_or(TargetMarkingError::Protocol(
            "missing or oversized target-marker state ID",
        ))
}
