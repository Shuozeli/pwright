//! Bounded Rust ownership for Playwright's already-serialized call argument.
//! This is a transport staging owner only: it performs no browser mutation.

use crate::handles::HandleSourceIdentity;
use crate::{HostError, HostSession};
use pwright_core::{
    ProductionResourceLedger, ResourceClass, ResourceLease, ResourceLedger, ResourceRequest,
    ResourceRequirements, ResourceScope,
};
use serde_json::Value;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

const SCOPE: ResourceScope = ResourceScope::extension(190, 1);
const CONTROL_BYTES: u64 = 4096;
const OWNER_BYTES: u64 = 96 * 1024 * 1024;
const MAX_DEPTH: usize = 64;
const MAX_NODES: usize = 8192;
const MAX_ENTRIES: usize = 8192;
const MAX_STRING_BYTES: usize = 72 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SerializedPayloadMetrics {
    pub depth: usize,
    pub nodes: usize,
    pub entries: usize,
    pub string_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SerializedPayloadLimits {
    pub max_depth: usize,
    pub max_nodes: usize,
    pub max_entries: usize,
    pub max_string_bytes: usize,
}

pub const fn serialized_payload_limits() -> SerializedPayloadLimits {
    SerializedPayloadLimits {
        max_depth: MAX_DEPTH,
        max_nodes: MAX_NODES,
        max_entries: MAX_ENTRIES,
        max_string_bytes: MAX_STRING_BYTES,
    }
}

pub(crate) fn requirements() -> ResourceRequirements {
    ResourceRequirements::none()
        .with_class(ResourceClass::ActiveCommand, 1, 1)
        .with_class(
            ResourceClass::BackendStateBytes,
            CONTROL_BYTES + OWNER_BYTES,
            CONTROL_BYTES + OWNER_BYTES,
        )
        .with_metadata(1, 2)
}

pub(crate) struct Budget {
    ledger: ResourceLedger,
    active: Arc<AtomicUsize>,
    _control: ResourceLease,
}

impl Budget {
    pub(crate) fn new(resources: &ProductionResourceLedger) -> Result<Self, HostError> {
        let ledger = resources
            .claim(requirements())
            .map_err(HostError::backend)?;
        let control = ledger
            .reserve(SCOPE, ResourceClass::BackendStateBytes, CONTROL_BYTES)
            .map_err(HostError::backend)?;
        Ok(Self {
            ledger,
            active: Arc::new(AtomicUsize::new(0)),
            _control: control,
        })
    }

    pub(crate) fn admit(&self) -> Result<SerializedPayloadAdmission, HostError> {
        self.active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active == 0).then_some(1)
            })
            .map_err(|_| {
                HostError::ResourceExhausted(
                    "serialized file-input payload import capacity exceeded".into(),
                )
            })?;
        let mut slot = SerializedPayloadSlot {
            active: Arc::clone(&self.active),
            lease: None,
        };
        slot.lease = Some(
            self.ledger
                .reserve_many(
                    SCOPE,
                    &[
                        ResourceRequest::new(ResourceClass::ActiveCommand, 1),
                        ResourceRequest::new(ResourceClass::BackendStateBytes, OWNER_BYTES),
                    ],
                )
                .map_err(HostError::backend)?,
        );
        Ok(SerializedPayloadAdmission { slot: Some(slot) })
    }
}

struct SerializedPayloadSlot {
    active: Arc<AtomicUsize>,
    lease: Option<ResourceLease>,
}

impl Drop for SerializedPayloadSlot {
    fn drop(&mut self) {
        self.lease = None;
        self.active.fetch_sub(1, Ordering::AcqRel);
    }
}

#[must_use = "finish the serialized graph import or drop its admission"]
pub struct SerializedPayloadAdmission {
    slot: Option<SerializedPayloadSlot>,
}

impl SerializedPayloadAdmission {
    pub fn finish(
        mut self,
        graph: Value,
        metrics: SerializedPayloadMetrics,
    ) -> Result<RetainedSerializedPayload, HostError> {
        let limits = serialized_payload_limits();
        if metrics.depth > limits.max_depth
            || metrics.nodes > limits.max_nodes
            || metrics.entries > limits.max_entries
            || metrics.string_bytes > limits.max_string_bytes
        {
            return Err(HostError::ResourceExhausted(
                "serialized file-input payload exceeds selected import limits".into(),
            ));
        }
        Ok(RetainedSerializedPayload {
            graph: Mutex::new(Some(graph)),
            metrics,
            _slot: self.slot.take().expect("live serialized payload admission"),
        })
    }
}

/// One move-once JSON-compatible graph. Its full 96 MiB owner allowance stays
/// reserved until terminal consumption or preparation revocation.
#[must_use = "consume in a retained native phase or drop for reclamation"]
pub struct RetainedSerializedPayload {
    graph: Mutex<Option<Value>>,
    metrics: SerializedPayloadMetrics,
    _slot: SerializedPayloadSlot,
}

impl RetainedSerializedPayload {
    pub fn metrics(&self) -> SerializedPayloadMetrics {
        self.metrics
    }

    pub fn take(&self) -> Result<Value, HostError> {
        self.graph
            .lock()
            .map_err(HostError::backend)?
            .take()
            .ok_or_else(|| {
                HostError::InvalidArgument(
                    "serialized file-input payload was already consumed".into(),
                )
            })
    }
}

impl HostSession {
    /// Reserve the entire Rust graph allowance before the Node boundary copies
    /// one byte. This validates only exact session/source provenance; browser
    /// liveness is checked again by the later native mutation phase.
    pub fn admit_retained_serialized_file_input(
        &self,
        source: &HandleSourceIdentity,
    ) -> Result<SerializedPayloadAdmission, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state
            .as_ref()
            .filter(|_| !self.stop.is_cancelled())
            .ok_or(HostError::TargetClosed)?;
        live.handles.owners.check_file_input_source(source)?;
        live.file_input.serialized.admit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_core::ResourceLimits;
    use serde_json::json;

    fn resources() -> ProductionResourceLedger {
        let required = requirements();
        let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
        for class in ResourceClass::ALL {
            let amount = required.class(class);
            limits = limits.with_class(class, amount.global, amount.per_scope);
        }
        ProductionResourceLedger::new(limits, required).unwrap()
    }

    #[test]
    fn one_live_admission_blocks_the_second_until_drop() {
        // Arrange
        let resources = resources();
        let budget = Budget::new(&resources).unwrap();
        let first = budget.admit().unwrap();

        // Act
        let blocked = budget.admit();
        drop(first);
        let reclaimed = budget.admit();

        // Assert
        assert!(matches!(blocked, Err(HostError::ResourceExhausted(_))));
        assert!(reclaimed.is_ok());
    }

    #[test]
    fn retained_graph_is_moved_once_and_keeps_its_metrics() {
        // Arrange
        let resources = resources();
        let budget = Budget::new(&resources).unwrap();
        let graph = json!({"a": [1, "two"]});
        let metrics = SerializedPayloadMetrics {
            depth: 2,
            nodes: 4,
            entries: 3,
            string_bytes: 4,
        };
        let retained = budget
            .admit()
            .unwrap()
            .finish(graph.clone(), metrics)
            .unwrap();

        // Act
        let moved = retained.take();
        let repeated = retained.take();

        // Assert
        assert_eq!(retained.metrics(), metrics);
        assert_eq!(moved.unwrap(), graph);
        assert!(matches!(repeated, Err(HostError::InvalidArgument(_))));
    }

    #[test]
    fn rejected_metrics_release_the_owner_slot() {
        // Arrange
        let resources = resources();
        let budget = Budget::new(&resources).unwrap();
        let limits = serialized_payload_limits();
        let oversized = SerializedPayloadMetrics {
            depth: limits.max_depth + 1,
            nodes: 1,
            entries: 1,
            string_bytes: 0,
        };

        // Act
        let rejected = budget.admit().unwrap().finish(Value::Null, oversized);
        let next = budget.admit();

        // Assert
        assert!(matches!(rejected, Err(HostError::ResourceExhausted(_))));
        assert!(next.is_ok());
    }
}
