//! Preallocate the two long-lived control outbox buffers before native work.
use super::*;

/// Independently selected alongside disposal reservations. This pays for the
/// message and EventItem-lease backing arrays, not runtime graph nodes, pending
/// calls, commit scratch, arbitrary diagnostics or allocator bookkeeping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlOutboxStorageProfile {
    capacity: usize,
    scope: ResourceScope,
    bytes: u64,
    requirements: ResourceRequirements,
}

impl ControlOutboxStorageProfile {
    pub fn new(
        capacity: NonZeroUsize,
        scope: ResourceScope,
    ) -> Result<Self, DisposalReservationError> {
        if !matches!(scope, ResourceScope::Extension { .. }) {
            return Err(DisposalReservationError::Invalid(
                "an extension scope is required",
            ));
        }
        let overflow = || DisposalReservationError::Invalid("outbox storage capacity overflow");
        // Old grown storage and freshly reserved storage can coexist during
        // installation. Keep this peak allowance for the full store lifetime.
        let slots = capacity
            .get()
            .checked_mul(2)
            .map(|n| n.max(8))
            .ok_or_else(overflow)?;
        let bytes = slots
            .checked_mul(2)
            .and_then(|n| {
                n.checked_mul(
                    std::mem::size_of::<ControlMessage>() + std::mem::size_of::<ResourceLease>(),
                )
            })
            .and_then(|n| n.checked_add(std::mem::size_of::<ResourceLease>() + 128))
            .filter(|n| *n <= isize::MAX as usize)
            .and_then(|n| u64::try_from(n).ok())
            .ok_or_else(overflow)?;
        Ok(Self {
            capacity: capacity.get(),
            scope,
            bytes,
            requirements: ResourceRequirements::none()
                .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
                .with_metadata(1, 1),
        })
    }

    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }

    /// Install once on this exact runtime, before admitting close reservations.
    /// Existing queued messages, their leases and all queue metrics are kept.
    /// All fallible work precedes the final resource-buffer swap. The byte lease
    /// stays in RuntimeStore through queue clear, pool drop and transport close.
    pub fn install(
        self,
        handle: &ControlledLifecycleHandle,
        resources: &ProductionResourceLedger,
    ) -> Result<(), DisposalReservationError> {
        let mut state = handle
            .store
            .state
            .lock()
            .map_err(|_| RuntimeStateError::StatePoisoned)?;
        if state.transport_closed {
            return Err(RuntimeStateError::TransportClosed.into());
        }
        state.ensure_no_lifecycle_transaction()?;
        if state.control_outbox_storage.is_some() {
            return Err(DisposalReservationError::Invalid(
                "outbox storage already installed",
            ));
        }
        if state.resource_ledger.is_none() {
            return Err(DisposalReservationError::Invalid(
                "a budgeted runtime is required",
            ));
        }
        if state.control_message_claimed || state.reserved_control_slots != 0 {
            return Err(DisposalReservationError::Invalid(
                "outbox has outstanding claims or reservations",
            ));
        }
        let queue = state
            .control_messages
            .as_ref()
            .ok_or(DisposalReservationError::Invalid(
                "a controlled outbox is required",
            ))?;
        if queue.capacity() != self.capacity || queue.len() != state.control_resources.len() {
            return Err(DisposalReservationError::Invalid(
                "outbox storage profile or lease inventory mismatch",
            ));
        }
        let ledger = resources
            .claim(self.requirements)
            .map_err(RuntimeStateError::from)?;
        let lease = ledger
            .reserve(self.scope, ResourceClass::BackendStateBytes, self.bytes)
            .map_err(RuntimeStateError::from)?;
        // Allocate the replacement lease buffer first, leaving both installed
        // buffers unchanged if this allocation or the queue reservation fails.
        let mut control_resources = VecDeque::new();
        control_resources
            .try_reserve_exact(self.capacity)
            .map_err(|_| {
                DisposalReservationError::Invalid("outbox lease storage allocation failed")
            })?;
        state
            .control_messages
            .as_mut()
            .expect("validated outbox")
            .try_reserve_full_capacity()
            .map_err(|_| {
                DisposalReservationError::Invalid("outbox message storage allocation failed")
            })?;
        control_resources.extend(state.control_resources.drain(..));
        state.control_resources = control_resources;
        state.control_outbox_storage = Some(lease);
        Ok(())
    }
}
