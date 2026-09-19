//! One strict, synchronous selector attempt in an exact utility world.
//!
//! No polling, user evaluation, portable Handle, runtime or browser ownership.
//! A caller must place started attempts and returned owners under pre-admitted
//! continuation cleanup; see the independent Chromium resolution composition.

#[cfg(feature = "retained-reference")]
mod retained;
#[cfg(feature = "retained-reference")]
pub use retained::RetainedElementRequest;

use pwright_backend_cdp_selector_runtime::{
    MAX_COMPILED_SELECTOR_BYTES, MAX_SELECTOR_NODES, MAX_SELECTOR_STRING_BYTES, SELECTOR_RUNTIME,
    SELECTOR_RUNTIME_SYMBOL, SELECTOR_RUNTIME_VERSION, compile_selector, measure_selector,
    selector_runtime_install_script,
};
use pwright_cdp::{CdpClient, connection::CdpError};
use pwright_core::{
    ObjectIdSource, OperationError, ProductionResourceError, ProductionResourceLedger,
    ResourceClass, ResourceError, ResourceLease, ResourceLedger, ResourceRequest,
    ResourceRequirements, ResourceScope, Service,
};
use pwright_selector_model::{LocatorErrorDetails, Selector};
use serde_json::Value;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub const NATIVE_ID_BYTES: usize = 1024;
pub const OWNER_BYTES: u64 = (2 * NATIVE_ID_BYTES + 64 + 256) as u64;
// Simultaneous selector tree, compiler/install strings, JSON escaping and CDP
// request copies. Response-envelope storage has the transport's separate bound.
pub const ATTEMPT_BYTES: u64 = (8 * (MAX_COMPILED_SELECTOR_BYTES + SELECTOR_RUNTIME.len() + 1024)
    + MAX_SELECTOR_NODES * std::mem::size_of::<Selector>()
    + 2 * MAX_SELECTOR_STRING_BYTES) as u64;

#[derive(Debug, thiserror::Error)]
pub enum ResolutionError {
    #[error("invalid Locator resolution setup: {0}")]
    Setup(&'static str),
    #[error("Locator resolution admission rejected: {0}")]
    Admission(&'static str),
    #[error(transparent)]
    Production(#[from] ProductionResourceError),
    #[error(transparent)]
    Resource(#[from] ResourceError),
    #[error("selector failed: {0}")]
    Selector(OperationError<LocatorErrorDetails>),
    #[error("strict mode violation: Locator resolved to {matches} elements")]
    StrictViolation { matches: u64 },
    #[error("invalid Locator resolution response: {0}")]
    Protocol(&'static str),
    #[error("Locator resolution CDP failure: {0}")]
    Backend(#[from] CdpError),
    #[error("Locator resolution cleanup failed: {source}; primary: {primary:?}")]
    Cleanup {
        primary: Option<Box<Self>>,
        source: CdpError,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct ResolutionProfile {
    scope: ResourceScope,
    requirements: ResourceRequirements,
    #[cfg(feature = "retained-reference")]
    retained_reference: bool,
}

impl ResolutionProfile {
    pub fn new(scope: ResourceScope, max_handles: usize) -> Result<Self, ResolutionError> {
        let count = u64::try_from(max_handles)
            .map_err(|_| ResolutionError::Setup("handle capacity overflow"))?;
        if count == 0 {
            return Err(ResolutionError::Setup("handle capacity must be positive"));
        }
        let peak = |bytes: u64| {
            bytes
                .checked_mul(count)
                .ok_or(ResolutionError::Setup("resource ceiling overflow"))
        };
        let bytes = peak(OWNER_BYTES)?;
        let selectors = peak(ATTEMPT_BYTES)?;
        Ok(Self {
            scope,
            #[cfg(feature = "retained-reference")]
            retained_reference: false,
            requirements: ResourceRequirements::none()
                .with_class(ResourceClass::RemoteHandle, count, count)
                .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
                .with_class(ResourceClass::SelectorBytes, selectors, selectors)
                .with_metadata(
                    1,
                    max_handles
                        .checked_mul(2)
                        .ok_or(ResolutionError::Setup("lease capacity overflow"))?,
                ),
        })
    }
    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }

    /// Explicit extra transient storage for exact retained-reference adoption.
    /// The default selector-only plan is unchanged even when the feature is
    /// compiled. An adoption consumes two of the same native-owner units.
    #[cfg(feature = "retained-reference")]
    pub fn with_retained_reference(mut self) -> Result<Self, ResolutionError> {
        if !self.retained_reference {
            let attempts = self.requirements.class(ResourceClass::RemoteHandle).global / 2;
            if attempts == 0 {
                return Err(ResolutionError::Setup(
                    "retained-reference requires at least two native-owner units",
                ));
            }
            let bytes = attempts
                .checked_mul(retained::ATTEMPT_BYTES)
                .and_then(|extra| {
                    self.requirements
                        .class(ResourceClass::BackendStateBytes)
                        .global
                        .checked_add(extra)
                })
                .ok_or(ResolutionError::Setup("retained attempt ceiling overflow"))?;
            self.requirements =
                self.requirements
                    .with_class(ResourceClass::BackendStateBytes, bytes, bytes);
            self.retained_reference = true;
        }
        Ok(self)
    }
}

pub struct ResolutionRequest {
    pub client: Arc<dyn CdpClient>,
    pub utility_world: Arc<str>,
    pub selector: Selector,
    pub destruction: Option<Arc<dyn ObjectGroupDestructionProof>>,
}

/// Optional caller-owned, monotonic proof that ALL native worlds which may
/// contain this group's objects have been destroyed on this exact connection.
/// Include derived/adopted wrappers, not just the query world. Route replacement,
/// transport loss, session detach and error text alone must return false.
/// The composing caller retains and budgets this proof through native cleanup.
pub trait ObjectGroupDestructionProof: Send + Sync {
    fn all_group_worlds_destroyed(&self) -> bool;

    /// Broader, optional retirement proof without claiming the worlds died.
    /// An implementation may also prove acknowledged retirement of the exact
    /// owned inspector session on this connection/epoch, after fencing native
    /// dispatch and retiring outstanding callbacks and late-result ownership.
    /// Mere session-detach events or failed commands remain insufficient.
    /// Default implementations preserve the world-destruction-only contract.
    fn all_group_objects_retired(&self) -> bool {
        self.all_group_worlds_destroyed()
    }
    /// Optional in-flight retirement fence. Pending must register the waker;
    /// both success AND failure of retirement must eventually wake it. Ready
    /// is only permission to check proof/attempt release, not retirement proof.
    fn poll_release_ready(&self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
        std::task::Poll::Ready(())
    }
}

async fn release_group(
    client: &dyn CdpClient,
    group: &str,
    proof: Option<&dyn ObjectGroupDestructionProof>,
    mode: GroupReleaseMode,
) -> Result<(), CdpError> {
    if let Some(proof) = proof {
        std::future::poll_fn(|cx| proof.poll_release_ready(cx)).await;
    }
    if proof.is_some_and(ObjectGroupDestructionProof::all_group_objects_retired) {
        return Ok(());
    }
    let result = match mode {
        GroupReleaseMode::Ordinary => client.runtime_release_object_group(group).await,
        #[cfg(feature = "retained-reference")]
        GroupReleaseMode::Settlement => client
            .runtime_release_object_group_to_settlement(group)
            .await
            .map_err(retained::bounded_error),
    };
    if let Some(proof) = proof {
        std::future::poll_fn(|cx| proof.poll_release_ready(cx)).await;
    }
    // Destruction can be observed while release is in flight. Only the same
    // authoritative proof can discharge a failed release, never its text.
    if result.is_err() && proof.is_some_and(ObjectGroupDestructionProof::all_group_objects_retired)
    {
        Ok(())
    } else {
        result
    }
}

/// Unique ownership of one group, including exception/unknown remote objects.
/// No clone and no portable identity: adoption/evaluation is composed separately.
#[must_use = "release the group, or use the cancellation-safe composition owner"]
pub struct ResolvedElement {
    client: Arc<dyn CdpClient>,
    utility_world: Arc<str>,
    object_id: Box<str>,
    group: String,
    _owner: ResourceLease,
    cleanup: GroupCleanupGuard,
    release_mode: GroupReleaseMode,
}

#[derive(Clone, Copy)]
enum GroupReleaseMode {
    Ordinary,
    #[cfg(feature = "retained-reference")]
    Settlement,
}

// If the executor itself abandons cleanup, or a raw caller drops a started
// attempt/owner, fail closed unless group release or destruction is confirmed.
struct GroupCleanupGuard {
    healthy: Arc<AtomicBool>,
    armed: bool,
    destruction: Option<Arc<dyn ObjectGroupDestructionProof>>,
}
impl Drop for GroupCleanupGuard {
    fn drop(&mut self) {
        if self.armed
            && !self
                .destruction
                .as_deref()
                .is_some_and(ObjectGroupDestructionProof::all_group_objects_retired)
        {
            self.healthy.store(false, Ordering::Release);
        }
    }
}

impl std::fmt::Debug for ResolvedElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResolvedElement")
            .field("group", &self.group)
            .finish_non_exhaustive()
    }
}

impl ResolvedElement {
    pub fn object_id(&self) -> &str {
        &self.object_id
    }
    pub fn utility_world(&self) -> &str {
        &self.utility_world
    }
    pub fn client(&self) -> Arc<dyn CdpClient> {
        Arc::clone(&self.client)
    }
    /// Derived temporary objects may join this group only while this owner is
    /// retained. Their additional native objects require separate admission.
    pub fn object_group(&self) -> &str {
        &self.group
    }
    /// Whether group cleanup retains its native command through settlement,
    /// rather than relying on the ordinary command response deadline. Derived
    /// retained-result capabilities must check this before creating wrappers;
    /// inspecting the mode does not upgrade it or grant another native lease.
    #[cfg(feature = "retained-reference")]
    pub fn uses_settlement_release(&self) -> bool {
        matches!(self.release_mode, GroupReleaseMode::Settlement)
    }
    pub async fn release(mut self) -> Result<(), ResolutionError> {
        release_group(
            self.client.as_ref(),
            &self.group,
            self.cleanup.destruction.as_deref(),
            self.release_mode,
        )
        .await
        .map_err(|source| ResolutionError::Cleanup {
            primary: None,
            source,
        })?;
        self.cleanup.armed = false;
        Ok(())
    }
}

#[derive(Clone)]
pub struct CdpLocatorResolutionService {
    profile: ResolutionProfile,
    ledger: ResourceLedger,
    ids: Arc<dyn ObjectIdSource>,
    healthy: Arc<AtomicBool>,
}

impl CdpLocatorResolutionService {
    /// All resolution installations sharing a CDP session must share the same
    /// never-reused ID source. Object groups are scoped to that native session.
    pub fn new(
        profile: ResolutionProfile,
        resources: &ProductionResourceLedger,
        ids: Arc<dyn ObjectIdSource>,
    ) -> Result<Self, ResolutionError> {
        Ok(Self {
            profile,
            ledger: resources.claim(profile.requirements())?,
            ids,
            healthy: Arc::new(AtomicBool::new(true)),
        })
    }

    fn prepare(
        &self,
        request: &ResolutionRequest,
    ) -> Result<(String, String, ResourceLease, ResourceLease), ResolutionError> {
        if !self.is_cleanup_healthy() {
            return Err(ResolutionError::Admission(
                "a prior native group cleanup failed",
            ));
        }
        if request.utility_world.is_empty() || request.utility_world.len() > NATIVE_ID_BYTES {
            return Err(ResolutionError::Setup(
                "exact utility world is missing or oversized",
            ));
        }
        measure_selector(&request.selector).map_err(ResolutionError::Selector)?;
        let owner = self.ledger.reserve_many(
            self.profile.scope,
            &[
                ResourceRequest::new(ResourceClass::RemoteHandle, 1),
                ResourceRequest::new(ResourceClass::BackendStateBytes, OWNER_BYTES),
            ],
        )?;
        let scratch = self.ledger.reserve(
            self.profile.scope,
            ResourceClass::SelectorBytes,
            ATTEMPT_BYTES,
        )?;
        let id = self
            .ids
            .next_object_id()
            .map_err(|_| ResolutionError::Setup("object-group IDs exhausted"))?;
        let group = format!("pwright.locator.{}", id.get());
        let selector =
            compile_selector(&request.selector, "document").map_err(ResolutionError::Selector)?;
        let install = selector_runtime_install_script();
        let script = format!(
            r#"function() {{
  'use strict';
  {install};
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) throw new Error('selector runtime missing');
  const matches = {selector};
  return matches.length === 1 ? matches[0] : matches.length;
}}"#
        );
        Ok((group, script, owner, scratch))
    }

    /// False is terminal for this installation; abandon its native session
    /// before installing a fresh resolver. Local lease reclamation alone does
    /// not establish that a failed native release destroyed the remote group.
    pub fn is_cleanup_healthy(&self) -> bool {
        self.healthy.load(Ordering::Acquire)
    }
}

pub type ResolutionFuture =
    Pin<Box<dyn Future<Output = Result<Option<ResolvedElement>, ResolutionError>> + Send>>;

impl Service<ResolutionRequest> for CdpLocatorResolutionService {
    type Response = Option<ResolvedElement>;
    type Error = ResolutionError;
    type Future<'a> = ResolutionFuture;

    fn call(&self, request: ResolutionRequest) -> ResolutionFuture {
        let prepared = self.prepare(&request);
        let healthy = Arc::clone(&self.healthy);
        Box::pin(async move {
            let (group, script, owner, _scratch) = prepared?;
            if !healthy.load(Ordering::Acquire) {
                return Err(ResolutionError::Admission(
                    "a prior native group cleanup failed",
                ));
            }
            let mut cleanup = GroupCleanupGuard {
                healthy,
                armed: true,
                destruction: request.destruction,
            };
            let response = request
                .client
                .runtime_query_object_in_group(&request.utility_world, &script, &group)
                .await;
            let object = response.map_err(ResolutionError::Backend).and_then(parse);
            match object {
                Ok(Some(object_id)) => Ok(Some(ResolvedElement {
                    client: request.client,
                    utility_world: request.utility_world,
                    object_id,
                    group,
                    _owner: owner,
                    cleanup,
                    release_mode: GroupReleaseMode::Ordinary,
                })),
                outcome => {
                    // Cleanup by group even if CDP failed or omitted/truncated the
                    // native ID. Do not rely on decoding an exception object first.
                    match release_group(
                        request.client.as_ref(),
                        &group,
                        cleanup.destruction.as_deref(),
                        GroupReleaseMode::Ordinary,
                    )
                    .await
                    {
                        Ok(()) => {
                            cleanup.armed = false;
                            outcome.map(|_| None)
                        }
                        Err(source) => Err(ResolutionError::Cleanup {
                            primary: outcome.err().map(Box::new),
                            source,
                        }),
                    }
                }
            }
        })
    }
}

fn parse(response: Value) -> Result<Option<Box<str>>, ResolutionError> {
    if response.get("exceptionDetails").is_some() {
        return Err(ResolutionError::Protocol("selector script threw"));
    }
    let result = response
        .get("result")
        .ok_or(ResolutionError::Protocol("missing RemoteObject"))?;
    if result.get("type").and_then(Value::as_str) == Some("number")
        && result.get("objectId").is_none()
    {
        return match result.get("value").and_then(Value::as_u64) {
            Some(0) => Ok(None),
            Some(matches @ 2..) => Err(ResolutionError::StrictViolation { matches }),
            _ => Err(ResolutionError::Protocol("invalid strict-match count")),
        };
    }
    if result.get("type").and_then(Value::as_str) != Some("object")
        || result.get("subtype").and_then(Value::as_str) != Some("node")
    {
        return Err(ResolutionError::Protocol("expected one DOM node"));
    }
    let id = result
        .get("objectId")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty() && id.len() <= NATIVE_ID_BYTES)
        .ok_or(ResolutionError::Protocol("missing or oversized native ID"))?;
    Ok(Some(id.into()))
}

#[cfg(test)]
mod tests;
