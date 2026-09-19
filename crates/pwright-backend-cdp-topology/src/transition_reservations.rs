//! Opt-in capacity ownership across native waits, without a topology mutation gate.
//!
//! Reservations contain no topology snapshot and issue no native command. Only
//! `prepare` clones the latest state and acquires the existing exclusive gate.
//! Native identity/ancestor receipts, runtime disposal and publication admission
//! remain responsibilities of the selected composition, not this storage layer.
//!
//! ```
//! use std::num::NonZeroUsize;
//! use pwright_backend_cdp_topology::{
//!     CdpContextOwnership, CdpTopologyLimits, CdpTopologyProductionProfile,
//!     ConnectionEpoch, ProductionCdpTopology,
//!     transition_reservations::TransitionReservationProfile,
//! };
//! use pwright_core::{ObjectId, ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope};
//!
//! let base = CdpTopologyProductionProfile::new(
//!     CdpTopologyLimits::new(32, 4, 4, 8, 8, 32), ResourceScope::extension(1, 1),
//! )?;
//! let selected = TransitionReservationProfile::new(
//!     base, ResourceScope::extension(1, 2), NonZeroUsize::new(2).unwrap(),
//! )?;
//! let plan = base.requirements().combine_concurrent(selected.requirements());
//! let bytes = plan.class(ResourceClass::BackendStateBytes);
//! let resources = ProductionResourceLedger::new(
//!     ResourceLimits::deny_all(plan.scopes, plan.leases)
//!         .with_class(ResourceClass::BackendStateBytes, bytes.global, bytes.per_scope), plan,
//! )?;
//! let epoch = ConnectionEpoch::new(1);
//! let mut topology = ProductionCdpTopology::new(ObjectId::new(1), epoch, base, &resources)?;
//! topology.register_context(epoch, "owned", ObjectId::new(2), CdpContextOwnership::Owned)?;
//! let pool = selected.bind(&topology, &resources)?;
//! let reservation = pool.reserve(&topology)?;
//! // A sibling can arrive while an independently owned native receipt is pending.
//! topology.register_context(epoch, "sibling", ObjectId::new(3), CdpContextOwnership::Owned)?;
//! // In a real composition, validate the exact native receipt before this mutation.
//! let prepared = reservation.prepare(&mut topology, |next| next.close_context(epoch, "owned"))?;
//! prepared.commit(&mut topology)?;
//! assert_eq!(topology.context_object_id("sibling"), Some(ObjectId::new(3)));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::num::NonZeroUsize;
use std::sync::atomic::AtomicUsize;

use super::*;

/// Conservative retained pool/Arc/lease-metadata allowance; not a RAM estimate.
pub const POOL_STATE_BYTES: u64 = 512;
/// Reservation owner and lease metadata, additional to native string storage.
pub const RESERVATION_STATE_BYTES: u64 = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransitionReservationProfile {
    topology: CdpTopologyProductionProfile,
    scope: ResourceScope,
    limit: NonZeroUsize,
    bytes_per_slot: u64,
    requirements: ResourceRequirements,
}

impl TransitionReservationProfile {
    /// Declare a separate extension scope for N reservations plus one committed
    /// shadow. The latter may remain in the live topology until its next update.
    pub fn new(
        topology: CdpTopologyProductionProfile,
        scope: ResourceScope,
        limit: NonZeroUsize,
    ) -> Result<Self, TransitionReservationError> {
        if !matches!(scope, ResourceScope::Extension { .. }) || scope == topology.resource_scope() {
            return Err(TransitionReservationError::InvalidScope { scope });
        }
        let capacity =
            || TransitionReservationError::Production(CdpTopologyProductionError::CapacityOverflow);
        let slots = limit.get().checked_add(1).ok_or_else(capacity)?;
        let leases = slots.checked_add(1).ok_or_else(capacity)?;
        let bytes_per_slot = topology
            .maximum_retained_native_bytes()
            .checked_add(RESERVATION_STATE_BYTES)
            .ok_or_else(capacity)?;
        let bytes = bytes_per_slot
            .checked_mul(u64::try_from(slots).map_err(|_| capacity())?)
            .and_then(|value| value.checked_add(POOL_STATE_BYTES))
            .ok_or_else(capacity)?;
        Ok(Self {
            topology,
            scope,
            limit,
            bytes_per_slot,
            requirements: ResourceRequirements::none()
                .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
                .with_metadata(1, leases),
        })
    }

    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
    pub const fn limit(self) -> usize {
        self.limit.get()
    }

    /// The caller's production plan must combine these requirements concurrently
    /// with the ordinary topology profile and all other selected components.
    pub fn bind(
        self,
        owner: &ProductionCdpTopology,
        resources: &ProductionResourceLedger,
    ) -> Result<TransitionReservations, TransitionReservationError> {
        if owner.profile() != self.topology {
            return Err(TransitionReservationError::ProfileMismatch);
        }
        let ledger = resources
            .claim(self.requirements)
            .map_err(CdpTopologyProductionError::from)?;
        let lease = ledger
            .reserve(
                self.scope,
                ResourceClass::BackendStateBytes,
                POOL_STATE_BYTES,
            )
            .map_err(CdpTopologyProductionError::from)?;
        Ok(TransitionReservations {
            state: Arc::new(ReservationState {
                owner: Arc::clone(&owner.transaction_gate),
                ledger,
                scope: self.scope,
                maximum: self.topology.maximum_retained_native_bytes(),
                bytes_per_slot: self.bytes_per_slot,
                limit: self.limit.get(),
                pending: AtomicUsize::new(0),
                _lease: lease,
            }),
        })
    }
}

struct ReservationState {
    owner: Arc<AtomicU64>,
    ledger: ResourceLedger,
    scope: ResourceScope,
    maximum: u64,
    bytes_per_slot: u64,
    limit: usize,
    pending: AtomicUsize,
    _lease: ResourceLease,
}

/// A clone shares the same pending limit; it does not create another pool.
#[derive(Clone)]
pub struct TransitionReservations {
    state: Arc<ReservationState>,
}

impl TransitionReservations {
    pub fn pending(&self) -> usize {
        self.state.pending.load(Ordering::Acquire)
    }

    pub fn reserve(
        &self,
        owner: &ProductionCdpTopology,
    ) -> Result<ReservedTopologyTransition, TransitionReservationError> {
        self.state.check_owner(owner)?;
        self.state
            .pending
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |pending| {
                (pending < self.state.limit).then(|| pending + 1)
            })
            .map_err(|_| TransitionReservationError::SlotsExhausted {
                limit: self.state.limit,
            })?;
        // Construct rollback ownership before the fallible byte/metadata lease.
        let permit = ReservationPermit(Arc::clone(&self.state));
        let lease = self
            .state
            .ledger
            .reserve(
                self.state.scope,
                ResourceClass::BackendStateBytes,
                self.state.bytes_per_slot,
            )
            .map_err(CdpTopologyProductionError::from)?;
        Ok(ReservedTopologyTransition {
            permit,
            lease,
            epoch: owner.epoch(),
        })
    }
}

impl ReservationState {
    fn check_owner(&self, owner: &ProductionCdpTopology) -> Result<(), TransitionReservationError> {
        if !Arc::ptr_eq(&self.owner, &owner.transaction_gate) {
            return Err(TransitionReservationError::Production(
                CdpTopologyProductionError::TransactionOwnerMismatch,
            ));
        }
        Ok(())
    }
}

struct ReservationPermit(Arc<ReservationState>);
impl Drop for ReservationPermit {
    fn drop(&mut self) {
        self.0.pending.fetch_sub(1, Ordering::AcqRel);
    }
}

#[must_use = "retain until native receipt or drop to release capacity"]
pub struct ReservedTopologyTransition {
    lease: ResourceLease,
    // Drop byte ownership before making the pending slot available again.
    permit: ReservationPermit,
    epoch: ConnectionEpoch,
}

impl ReservedTopologyTransition {
    /// Prepare from current state without another byte reservation. Failure
    /// drops this reservation; it never publishes a partial mutation. This is
    /// not proof of native retirement: the caller must validate exact native
    /// identities/receipts in `apply` before removing anything. As with ordinary
    /// topology transitions, arbitrary callback/output storage is not covered
    /// by the native-string shadow budget and needs caller-owned admission.
    pub fn prepare<Output>(
        self,
        owner: &mut ProductionCdpTopology,
        apply: impl FnOnce(&mut CdpTopology) -> Result<Output, CdpTopologyError>,
    ) -> Result<ReservedPreparedTopologyTransition<Output>, TransitionReservationError> {
        self.permit.0.check_owner(owner)?;
        if owner.epoch() != self.epoch {
            return Err(TransitionReservationError::EpochChanged {
                reserved: self.epoch,
                current: owner.epoch(),
            });
        }
        owner.ensure_no_transaction()?;
        let mut next = owner.topology.clone();
        let output = apply(&mut next).map_err(CdpTopologyProductionError::from)?;
        // A reservation for retirement cannot itself change connection epoch.
        if next.epoch() != self.epoch {
            return Err(TransitionReservationError::EpochChanged {
                reserved: self.epoch,
                current: next.epoch(),
            });
        }
        let maximum = self.permit.0.maximum;
        let actual = u64::try_from(next.retained_native_bytes())
            .map_err(|_| CdpTopologyProductionError::CapacityOverflow)?;
        if actual > maximum {
            return Err(
                CdpTopologyProductionError::RetainedBytesInvariant { actual, maximum }.into(),
            );
        }
        let transaction_id = owner.next_transaction_id;
        owner.next_transaction_id = transaction_id
            .checked_add(1)
            .ok_or(CdpTopologyProductionError::TransactionIdExhausted)?;
        owner
            .transaction_gate
            .compare_exchange(0, transaction_id, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| CdpTopologyProductionError::TransactionInProgress)?;
        Ok(ReservedPreparedTopologyTransition {
            transaction: PreparedOwnedCdpTopologyTransition {
                transaction_gate: Arc::clone(&owner.transaction_gate),
                transaction_id,
                next: Some(next),
                next_lease: Some(self.lease),
                output: Some(output),
                finished: false,
            },
            _permit: self.permit,
        })
    }
}

/// The short exclusive commit phase. The slot stays owned until commit/drop;
/// the native-state lease transfers to the live topology on successful commit.
#[must_use = "commit the prepared state or drop to roll back"]
pub struct ReservedPreparedTopologyTransition<Output> {
    transaction: PreparedOwnedCdpTopologyTransition<Output>,
    _permit: ReservationPermit,
}

impl<Output> ReservedPreparedTopologyTransition<Output> {
    pub fn topology(&self) -> &CdpTopology {
        self.transaction.topology()
    }
    pub fn output(&self) -> &Output {
        self.transaction.output()
    }
    pub fn commit(
        self,
        owner: &mut ProductionCdpTopology,
    ) -> Result<Output, TransitionReservationError> {
        self.transaction.commit(owner).map_err(Into::into)
    }
}

#[derive(Debug)]
pub enum TransitionReservationError {
    InvalidScope {
        scope: ResourceScope,
    },
    ProfileMismatch,
    SlotsExhausted {
        limit: usize,
    },
    EpochChanged {
        reserved: ConnectionEpoch,
        current: ConnectionEpoch,
    },
    Production(CdpTopologyProductionError),
}

impl From<CdpTopologyProductionError> for TransitionReservationError {
    fn from(error: CdpTopologyProductionError) -> Self {
        Self::Production(error)
    }
}

impl Display for TransitionReservationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidScope { scope } => write!(
                f,
                "transition reservations require an independent extension scope, got {scope}"
            ),
            Self::ProfileMismatch => {
                f.write_str("transition reservation profile does not match its topology owner")
            }
            Self::SlotsExhausted { limit } => {
                write!(f, "transition reservation capacity exceeded ({limit})")
            }
            Self::EpochChanged { reserved, current } => write!(
                f,
                "transition reservation epoch {} does not match current epoch {}",
                reserved.get(),
                current.get()
            ),
            Self::Production(error) => Display::fmt(error, f),
        }
    }
}

impl Error for TransitionReservationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Production(error) => Some(error),
            Self::InvalidScope { .. }
            | Self::ProfileMismatch
            | Self::SlotsExhausted { .. }
            | Self::EpochChanged { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests;
