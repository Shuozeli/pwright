//! Joined admission for a Page or Context intent, with no native mutation or
//! lifecycle gate while waiting. Suitable as a native-close scheduler payload.
use crate::{ChromiumCloseCapacity, ChromiumCloseObserver, ChromiumCloseTarget};
use pwright_backend_cdp_topology::{
    ProductionCdpTopology,
    transition_reservations::{
        ReservedTopologyTransition, TransitionReservationError, TransitionReservations,
    },
};
use pwright_core::OperationError;
use pwright_runtime::disposal_reservations::{
    DisposalReservationError, DisposalReservations, ReservedDisposalCapacity,
};
use std::fmt::{self, Display, Formatter};

/// Owns all three capacity reservations for one intent. It is not native-death
/// authority, a frozen topology/publication view or a prepared transaction.
/// Retain it through written native work; dropping a caller reply must not drop
/// this payload out of a running native-close scheduler.
///
/// No new allocation or pool belongs to the bundle itself. Each reservation
/// retains its existing admission; the selecting scheduler/caller admits this
/// inline payload and any other payload-owned heaps. Complete commit scratch
/// and persistent runtime/outbox storage remain independently selected.
#[must_use = "retain through native completion, then prepare latest-state retirement"]
pub struct ChromiumCloseCapacityBundle {
    // Abandon publication capacity before runtime queue slots and topology
    // capacity. No lifecycle mutation gate is owned in this waiting phase.
    publication: Box<dyn ChromiumCloseCapacity>,
    runtime: ReservedDisposalCapacity,
    topology: ReservedTopologyTransition,
    target: ChromiumCloseTarget,
}

/// Capacity owners for a SHORT commit under separately validated native proof.
/// The target is the originally requested portable identity, not native proof.
/// Validate it against the sealed receipt before preparing any local retirement.
pub struct ChromiumCloseCapacityParts {
    pub target: ChromiumCloseTarget,
    pub publication: Box<dyn ChromiumCloseCapacity>,
    pub runtime: ReservedDisposalCapacity,
    pub topology: ReservedTopologyTransition,
}

impl ChromiumCloseCapacityBundle {
    /// Reserve topology, runtime/outbox and close-publication capacity before a
    /// native command is admitted. Each subsequent failure drops earlier owners.
    /// `topology` must be the exact owner used to bind `topology_pool`; the runtime
    /// pool is already bound to its own exact runtime. Cross-component/native
    /// domain matching remains the selecting coordinator's responsibility.
    ///
    /// The caller holds topology only during admission and releases its mutex
    /// before polling native work. Observer capacity admission must not reenter
    /// lifecycle mutation or invoke user callbacks. No snapshot is prepared.
    pub fn reserve(
        target: ChromiumCloseTarget,
        topology: &ProductionCdpTopology,
        topology_pool: &TransitionReservations,
        runtime_pool: &DisposalReservations,
        observer: &dyn ChromiumCloseObserver,
    ) -> Result<Self, ChromiumCloseCapacityBundleError> {
        let topology = topology_pool
            .reserve(topology)
            .map_err(ChromiumCloseCapacityBundleError::Topology)?;
        let runtime = runtime_pool
            .reserve()
            .map_err(ChromiumCloseCapacityBundleError::Runtime)?;
        let publication = observer
            .reserve_capacity(target)
            .map_err(ChromiumCloseCapacityBundleError::Publication)?;
        Ok(Self {
            publication,
            runtime,
            topology,
            target,
        })
    }

    pub const fn target(&self) -> ChromiumCloseTarget {
        self.target
    }

    /// Move, rather than reacquire, all reservations into the native-validated
    /// latest-state commit. This method itself publishes nothing and sends no
    /// command; do not treat extraction as successful native retirement.
    pub fn into_parts(self) -> ChromiumCloseCapacityParts {
        ChromiumCloseCapacityParts {
            target: self.target,
            publication: self.publication,
            runtime: self.runtime,
            topology: self.topology,
        }
    }
}

#[derive(Debug)]
pub enum ChromiumCloseCapacityBundleError {
    Topology(TransitionReservationError),
    Runtime(DisposalReservationError),
    Publication(OperationError<()>),
}
impl Display for ChromiumCloseCapacityBundleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Topology(error) => Display::fmt(error, f),
            Self::Runtime(error) => Display::fmt(error, f),
            Self::Publication(error) => Display::fmt(error, f),
        }
    }
}
impl std::error::Error for ChromiumCloseCapacityBundleError {}

#[cfg(test)]
pub(crate) mod tests;
