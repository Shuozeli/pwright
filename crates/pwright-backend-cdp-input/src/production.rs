//! Optional admission for retained Page input states, independent of Locator.
use crate::{CdpInputCoordinator, CdpInputPolicy};
use pwright_core::{
    ProductionResourceError, ProductionResourceLedger, ResourceClass, ResourceError, ResourceLease,
    ResourceLedger, ResourceRequirements, ResourceScope,
};
use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpInputProductionProfile {
    scope: ResourceScope,
    policy: CdpInputPolicy,
    states: usize,
}
impl CdpInputProductionProfile {
    /// Includes both current Pages and retained old states during route replacement.
    pub const fn new(scope: ResourceScope, policy: CdpInputPolicy, states: usize) -> Self {
        Self {
            scope,
            policy,
            states,
        }
    }
    pub const fn scope(self) -> ResourceScope {
        self.scope
    }
    pub const fn policy(self) -> CdpInputPolicy {
        self.policy
    }
    pub const fn states(self) -> usize {
        self.states
    }
    pub fn state_bytes(self) -> u64 {
        (CdpInputCoordinator::state_allocation_bytes(self.policy)
            + std::mem::size_of::<InputAdmission>()
            + 2 * std::mem::size_of::<usize>()) as u64
    }
    pub fn requirements(self) -> ResourceRequirements {
        if self.states == 0 {
            return ResourceRequirements::none();
        }
        let bytes = self
            .state_bytes()
            .saturating_mul(u64::try_from(self.states).unwrap_or(u64::MAX));
        ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, self.states)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CdpInputAdmissionError {
    Production(ProductionResourceError),
    Resource(ResourceError),
    Capacity { limit: usize },
}
impl fmt::Display for CdpInputAdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Production(error) => error.fmt(f),
            Self::Resource(error) => error.fmt(f),
            Self::Capacity { limit } => write!(f, "Page input state capacity exceeded ({limit})"),
        }
    }
}
impl std::error::Error for CdpInputAdmissionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Production(error) => Some(error),
            Self::Resource(error) => Some(error),
            Self::Capacity { .. } => None,
        }
    }
}
impl From<ProductionResourceError> for CdpInputAdmissionError {
    fn from(error: ProductionResourceError) -> Self {
        Self::Production(error)
    }
}

#[derive(Clone)]
pub struct CdpInputServiceFactory {
    profile: CdpInputProductionProfile,
    ledger: ResourceLedger,
    active: Arc<AtomicUsize>,
}
impl CdpInputServiceFactory {
    pub fn new_with_production_resources(
        profile: CdpInputProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, CdpInputAdmissionError> {
        Ok(Self {
            profile,
            ledger: resources.claim(profile.requirements())?,
            active: Arc::new(AtomicUsize::new(0)),
        })
    }
    pub fn active_states(&self) -> usize {
        self.active.load(Ordering::Acquire)
    }
    pub const fn profile(&self) -> CdpInputProductionProfile {
        self.profile
    }
    pub fn create(&self) -> Result<CdpInputCoordinator, CdpInputAdmissionError> {
        self.active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < self.profile.states).then(|| n + 1)
            })
            .map_err(|_| CdpInputAdmissionError::Capacity {
                limit: self.profile.states,
            })?;
        let lease = match self.ledger.reserve(
            self.profile.scope,
            ResourceClass::BackendStateBytes,
            self.profile.state_bytes(),
        ) {
            Ok(lease) => lease,
            Err(error) => {
                self.active.fetch_sub(1, Ordering::AcqRel);
                return Err(CdpInputAdmissionError::Resource(error));
            }
        };
        let admission = Arc::new(InputAdmission {
            lease: Some(lease),
            active: self.active.clone(),
        });
        Ok(CdpInputCoordinator::with_admission(
            self.profile.policy,
            admission,
        ))
    }
}

pub(crate) struct InputAdmission {
    lease: Option<ResourceLease>,
    active: Arc<AtomicUsize>,
}
impl Drop for InputAdmission {
    fn drop(&mut self) {
        drop(self.lease.take());
        self.active.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests;
