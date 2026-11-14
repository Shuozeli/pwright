//! Optional Playwright-compatible document replacement over Rust-owned
//! evaluation, lifecycle, timeout, and resource admission.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

use pwright_core::{
    ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements, ResourceScope,
};
use pwright_plugin_chromium_frame_lifecycle::PageLifecycleHandle;

use crate::evaluation::{EvaluateParams, EvaluationValue};
use crate::{HostError, HostSession};

const MAX_OPERATIONS: u64 = 8;
const OPERATION_BYTES: u64 = 64 * 1024;
const OPERATION_SCOPE: ResourceScope = ResourceScope::extension(130, 1);
const SLOT_SCOPE: ResourceScope = ResourceScope::extension(130, 2);
const SET_CONTENT_SOURCE: &str =
    "html => { document.open(); document.write(html); document.close(); }";

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
                    "Page set_content capacity exceeded ({MAX_OPERATIONS})"
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
    pub async fn page_set_content(
        &self,
        page_id: &str,
        html: String,
        wait_until: &str,
        timeout_ms: u32,
    ) -> Result<String, HostError> {
        let wait = crate::commands::parse_wait_until(wait_until)?;
        let (page_id_value, _, _operation) = self.admit_operation(page_id)?;
        let _permit = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .set_content
                .slots
                .admit()?
        };
        let deadline = (timeout_ms != 0)
            .then(|| tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms)));
        let page = self.frame_lifecycle(page_id).await?;
        let tree = self.frame_trees.get(page_id_value)?;
        let main = tree.main_frame_snapshot()?;
        let frame = main.id();
        let metadata = main.metadata_snapshot();
        page.reset_document(frame)
            .map_err(|error| lifecycle_error(&page, error))?;
        let mut revision = page
            .revision()
            .map_err(|error| lifecycle_error(&page, error))?;

        let evaluation_timeout = remaining_timeout(deadline, timeout_ms)?;
        let evaluation = self
            .frame_evaluate_with_timeout(
                &frame.get().to_string(),
                EvaluateParams::function(SET_CONTENT_SOURCE, EvaluationValue::String(html)),
                evaluation_timeout,
            )
            .await;
        match evaluation {
            Err(HostError::Timeout(_)) => return Err(HostError::Timeout(timeout_ms)),
            Err(error) => return Err(error),
            Ok(_) => {}
        }

        loop {
            self.flush_topology().await?;
            if page.is_sealed() || !self.is_connected() {
                return Err(HostError::TargetClosed);
            }
            if deadline.is_some_and(|at| tokio::time::Instant::now() >= at) {
                return Err(HostError::Timeout(timeout_ms));
            }
            let snapshot = page
                .snapshot(frame)
                .map_err(|error| lifecycle_error(&page, error))?
                .ok_or(HostError::TargetClosed)?;
            if snapshot.reached(wait) {
                return metadata.url();
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
                () = self.stop.cancelled() => return Err(HostError::TargetClosed),
                () = self.disconnected.cancelled() => return Err(HostError::TargetClosed),
                () = page.closed() => return Err(HostError::TargetClosed),
                () = wait_deadline(deadline) => return Err(HostError::Timeout(timeout_ms)),
                changed = page.changed_after(revision) => {
                    revision = changed.map_err(|error| lifecycle_error(&page, error))?;
                }
                () = wait_duration(lifecycle_delay) => {}
            }
        }
    }
}

fn remaining_timeout(
    deadline: Option<tokio::time::Instant>,
    original_ms: u32,
) -> Result<u32, HostError> {
    let Some(deadline) = deadline else {
        return Ok(0);
    };
    let Some(remaining) = deadline.checked_duration_since(tokio::time::Instant::now()) else {
        return Err(HostError::Timeout(original_ms));
    };
    Ok(
        u32::try_from(remaining.as_millis().clamp(1, u128::from(u32::MAX)))
            .expect("clamped timeout fits u32"),
    )
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
