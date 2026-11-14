//! Optional event-driven Page URL wait over Rust-owned metadata and lifecycle.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;
use std::{future::Future, future::ready};

use crate::{HostError, HostSession};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};
use pwright_plugin_chromium_frame_lifecycle::PageLifecycleHandle;

const MAX_OPERATIONS: u64 = 8;
const OPERATION_BYTES: u64 = 64 * 1024;
const OPERATION_SCOPE: ResourceScope = ResourceScope::extension(129, 1);
const SLOT_SCOPE: ResourceScope = ResourceScope::extension(129, 2);

#[derive(Debug)]
pub enum UrlWaitError<E> {
    Host(HostError),
    Matcher(E),
}

impl<E> From<HostError> for UrlWaitError<E> {
    fn from(error: HostError) -> Self {
        Self::Host(error)
    }
}

struct Slots {
    active: AtomicU64,
    resources: ProductionResourceLedger,
    _lease: ResourceLease,
}

struct OperationPermit {
    owner: Arc<Slots>,
    _storage: ResourceLease,
}

impl Drop for OperationPermit {
    fn drop(&mut self) {
        self.owner.active.fetch_sub(1, Ordering::AcqRel);
    }
}

impl Slots {
    fn new(resources: &ProductionResourceLedger) -> Result<Arc<Self>, HostError> {
        let plan = slot_requirements();
        let lease = resources
            .claim(plan)
            .map_err(HostError::backend)?
            .reserve(
                SLOT_SCOPE,
                ResourceClass::BackendStateBytes,
                plan.class(ResourceClass::BackendStateBytes).global,
            )
            .map_err(HostError::backend)?;
        Ok(Arc::new(Self {
            active: AtomicU64::new(0),
            resources: resources.clone(),
            _lease: lease,
        }))
    }

    fn admit(self: &Arc<Self>) -> Result<OperationPermit, HostError> {
        self.active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < MAX_OPERATIONS).then_some(active + 1)
            })
            .map_err(|_| {
                HostError::ResourceExhausted(format!(
                    "Page URL wait capacity exceeded ({MAX_OPERATIONS})"
                ))
            })?;
        let storage = self
            .resources
            .claim(operation_requirements())
            .map_err(HostError::backend)
            .and_then(|ledger| {
                ledger
                    .reserve(
                        OPERATION_SCOPE,
                        ResourceClass::BackendStateBytes,
                        OPERATION_BYTES,
                    )
                    .map_err(HostError::backend)
            });
        match storage {
            Ok(storage) => Ok(OperationPermit {
                owner: self.clone(),
                _storage: storage,
            }),
            Err(error) => {
                self.active.fetch_sub(1, Ordering::AcqRel);
                Err(error)
            }
        }
    }
}

pub(crate) struct Installation {
    slots: Arc<Slots>,
}

const fn slot_requirements() -> ResourceRequirements {
    let bytes = (std::mem::size_of::<Slots>() + 2 * std::mem::size_of::<usize>() + 256) as u64;
    ResourceRequirements::none()
        .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
        .with_metadata(1, 1)
}

const fn operation_requirements() -> ResourceRequirements {
    ResourceRequirements::none()
        .with_class(
            ResourceClass::BackendStateBytes,
            OPERATION_BYTES * MAX_OPERATIONS,
            OPERATION_BYTES * MAX_OPERATIONS,
        )
        .with_metadata(1, MAX_OPERATIONS as usize)
}

pub(crate) fn requirements() -> ResourceRequirements {
    operation_requirements().combine_concurrent(slot_requirements())
}

pub(crate) fn install(resources: &ProductionResourceLedger) -> Result<Installation, HostError> {
    Ok(Installation {
        slots: Slots::new(resources)?,
    })
}

impl HostSession {
    /// Wait for a main-Frame URL and requested document lifecycle state. URL
    /// matching is an injected synchronous port so each language binding can
    /// preserve its own RegExp/predicate semantics without moving event or
    /// timeout ownership out of Rust.
    pub async fn page_wait_for_url<E, M>(
        &self,
        page_id: &str,
        wait_until: &str,
        timeout_ms: u32,
        mut matches: M,
    ) -> Result<String, UrlWaitError<E>>
    where
        E: Send,
        M: FnMut(&str) -> Result<bool, E> + Send,
    {
        self.wait_for_frame_url(page_id, None, wait_until, timeout_ms, move |candidate| {
            ready(matches(&candidate))
        })
        .await
    }

    /// Wait for any Frame URL while Rust retains ordering, lifecycle and
    /// timeout ownership. The asynchronous matcher is a language-semantic
    /// port: Node can preserve RegExp and URL-predicate behavior without
    /// moving the wait loop into JavaScript.
    pub async fn frame_wait_for_url<E, M, F>(
        &self,
        page_id: &str,
        frame_id: &str,
        wait_until: &str,
        timeout_ms: u32,
        matches: M,
    ) -> Result<String, UrlWaitError<E>>
    where
        E: Send,
        M: FnMut(String) -> F + Send,
        F: Future<Output = Result<bool, E>> + Send,
    {
        let frame = parse_frame(frame_id)?;
        self.wait_for_frame_url(page_id, Some(frame), wait_until, timeout_ms, matches)
            .await
    }

    async fn wait_for_frame_url<E, M, F>(
        &self,
        page_id: &str,
        frame_id: Option<ObjectId>,
        wait_until: &str,
        timeout_ms: u32,
        mut matches: M,
    ) -> Result<String, UrlWaitError<E>>
    where
        E: Send,
        M: FnMut(String) -> F + Send,
        F: Future<Output = Result<bool, E>> + Send,
    {
        let wait = crate::commands::parse_wait_until(wait_until)?;
        let (page_id_value, _, _operation) = self.admit_operation(page_id)?;
        let _permit = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .url_wait
                .slots
                .admit()?
        };
        let deadline = (timeout_ms != 0)
            .then(|| tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms)));
        let page = self.frame_lifecycle(page_id).await?;
        let tree = self.frame_trees.get(page_id_value)?;
        let frame = match frame_id {
            Some(id) => tree.frame_snapshot(id)?,
            None => tree.main_frame_snapshot()?,
        };
        let frame_id = frame.id();
        let metadata = frame.metadata_snapshot();
        let mut revision = page
            .revision()
            .map_err(|error| lifecycle_error(&page, error))?;

        loop {
            self.flush_topology().await?;
            if page.is_sealed() || !self.is_connected() {
                return Err(HostError::TargetClosed.into());
            }
            if deadline.is_some_and(|at| tokio::time::Instant::now() >= at) {
                return Err(HostError::Timeout(timeout_ms).into());
            }
            let url = metadata.url()?;
            let matched = matches(url.clone()).await.map_err(UrlWaitError::Matcher)?;
            let snapshot = page
                .snapshot(frame_id)
                .map_err(|error| lifecycle_error(&page, error))?
                .ok_or(HostError::TargetClosed)?;
            if matched && snapshot.reached(wait) {
                return Ok(url);
            }

            let next = page
                .next_deadline()
                .map_err(|error| lifecycle_error(&page, error))?;
            let lifecycle_delay = next
                .map(|at| {
                    page.now()
                        .map(|now| at.saturating_sub(now))
                        .map_err(|error| lifecycle_error(&page, error))
                })
                .transpose()?;
            tokio::select! {
                biased;
                () = self.stop.cancelled() => return Err(HostError::TargetClosed.into()),
                () = self.disconnected.cancelled() => return Err(HostError::TargetClosed.into()),
                () = page.closed() => return Err(HostError::TargetClosed.into()),
                () = wait_deadline(deadline) => return Err(HostError::Timeout(timeout_ms).into()),
                changed = page.changed_after(revision) => {
                    revision = changed.map_err(|error| lifecycle_error(&page, error))?;
                }
                () = wait_duration(lifecycle_delay) => {}
            }
        }
    }
}

fn parse_frame(value: &str) -> Result<ObjectId, HostError> {
    value
        .parse::<u64>()
        .ok()
        .filter(|value| *value != 0)
        .map(ObjectId::new)
        .ok_or_else(|| HostError::InvalidArgument("invalid portable Frame ID".into()))
}

fn lifecycle_error(page: &PageLifecycleHandle, error: impl std::fmt::Display) -> HostError {
    if page.is_sealed() {
        HostError::TargetClosed
    } else {
        HostError::backend(error)
    }
}

async fn wait_deadline(deadline: Option<tokio::time::Instant>) {
    match deadline {
        Some(at) => tokio::time::sleep_until(at).await,
        None => std::future::pending().await,
    }
}

async fn wait_duration(delay: Option<Duration>) {
    match delay {
        Some(delay) => tokio::time::sleep(delay).await,
        None => std::future::pending().await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_core::ResourceLimits;

    fn resources() -> ProductionResourceLedger {
        let plan = requirements();
        let mut limits = ResourceLimits::deny_all(plan.scopes, plan.leases);
        for class in ResourceClass::ALL {
            let bound = plan.class(class);
            limits = limits.with_class(class, bound.global, bound.per_scope);
        }
        ProductionResourceLedger::new(limits, plan).unwrap()
    }

    #[test]
    fn operation_capacity_is_fixed_and_reclaimed_on_drop() {
        // Arrange
        let resources = resources();
        let slots = Slots::new(&resources).unwrap();
        let mut permits: Vec<_> = (0..MAX_OPERATIONS)
            .map(|_| slots.admit().unwrap())
            .collect();

        // Act
        let excess = slots.admit();
        drop(permits.pop());
        let reclaimed = slots.admit();

        // Assert
        assert!(matches!(excess, Err(HostError::ResourceExhausted(_))));
        assert!(reclaimed.is_ok());
    }
}
