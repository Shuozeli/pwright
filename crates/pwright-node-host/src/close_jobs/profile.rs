//! Explicit host selection; ordinary collection-only attachments stay cold.
use super::{PendingClose, Selected};
use crate::HostError;
use pwright_backend_cdp_page_lifecycle::native_confirmation::NativePageCloseProfile;
use pwright_backend_cdp_topology::{
    CdpTopologyProductionProfile, transition_reservations::TransitionReservationProfile,
};
use pwright_composition_chromium::native_close_authority::{
    NativeCloseAuthorityProfile, scheduler::NativeCloseSchedulerProfile,
};
use pwright_core::{ResourceClass, ResourceRequirements, ResourceScope};
use pwright_runtime::disposal_reservations::DisposalReservationProfile;
use std::num::{NonZeroU16, NonZeroUsize};

pub(super) const MAX_CLOSES: usize = 8;
pub(crate) const MAX_OBJECTS: usize = 256;
pub(crate) const MAX_CALLS: usize = 16;
const OWNER_SCOPE: ResourceScope = ResourceScope::extension(138, 6);

pub(crate) struct Profile {
    pub(super) scheduler: NativeCloseSchedulerProfile<PendingClose>,
    pub(super) authority: NativeCloseAuthorityProfile,
    pub(super) pages: NativePageCloseProfile,
    pub(super) topology: TransitionReservationProfile,
    pub(super) runtime: DisposalReservationProfile,
}
impl Profile {
    pub(crate) fn new(topology: CdpTopologyProductionProfile) -> Result<Self, HostError> {
        let pending = NonZeroUsize::new(MAX_CLOSES).expect("positive host close limit");
        let native_pending = NonZeroU16::new(8).expect("positive host close limit");
        Ok(Self {
            scheduler: NativeCloseSchedulerProfile::new(
                ResourceScope::extension(138, 1),
                native_pending,
            ),
            authority: NativeCloseAuthorityProfile::new(
                ResourceScope::extension(138, 2),
                native_pending,
                NonZeroU16::new(128).expect("positive native identity bound"),
            )
            .map_err(HostError::backend)?,
            pages: NativePageCloseProfile::new(
                ResourceScope::extension(138, 3),
                native_pending.get(),
                128,
            )
            .map_err(HostError::backend)?,
            topology: TransitionReservationProfile::new(
                topology,
                ResourceScope::extension(138, 4),
                pending,
            )
            .map_err(HostError::backend)?,
            runtime: DisposalReservationProfile::new_in_process(
                NonZeroUsize::new(MAX_OBJECTS).expect("positive graph bound"),
                pending,
                ResourceScope::extension(138, 5),
            )
            .and_then(|profile| {
                profile.with_deferred_commit(
                    NonZeroUsize::new(MAX_CALLS).expect("positive call bound"),
                )
            })
            .map_err(HostError::backend)?,
        })
    }
    pub(crate) fn event_subscribers(&self) -> usize {
        self.pages.event_subscribers()
    }
    pub(crate) fn requirements(&self) -> ResourceRequirements {
        self.scheduler
            .requirements()
            .combine_concurrent(self.authority.requirements())
            .combine_concurrent(self.pages.requirements())
            .combine_concurrent(self.topology.requirements())
            .combine_concurrent(self.runtime.requirements())
            .combine_concurrent(owner_requirements())
    }
}
fn owner_bytes() -> u64 {
    (std::mem::size_of::<Selected>()
        + std::mem::size_of::<tokio::sync::Notify>()
        + 4 * std::mem::size_of::<usize>()) as u64
}
fn owner_requirements() -> ResourceRequirements {
    ResourceRequirements::none()
        .with_class(
            ResourceClass::BackendStateBytes,
            owner_bytes(),
            owner_bytes(),
        )
        .with_metadata(1, 1)
}
pub(super) fn reserve_owner(
    resources: &pwright_core::ProductionResourceLedger,
) -> Result<pwright_core::ResourceLease, HostError> {
    resources
        .claim(owner_requirements())
        .map_err(HostError::backend)?
        .reserve(OWNER_SCOPE, ResourceClass::BackendStateBytes, owner_bytes())
        .map_err(HostError::backend)
}
