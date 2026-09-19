//! Opt-in, caller-polled world readiness under the enclosing command lifetime.

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use pwright_backend_cdp::{CdpDomContentService, CdpLocatorService, CdpTopologyProductionProfile};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceLedger,
    ResourceRequirements, ResourceScope,
};
use tokio::sync::Notify;

use crate::{CdpPageRouteError, CdpPageServices, CdpResolvedEvaluationRoute, CdpServiceRoutes};

const FIXED_BYTES: u64 = 256;
const FRAME_PAIR_BYTES: u64 = 128;
const WAITER_BYTES: u64 = 512;

#[cfg(test)]
mod tests;

/// Bounded notification and world-independent Frame ownership, absent by default.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumExecutionWorldReadinessProfile {
    scope: ResourceScope,
    max_frames: usize,
    max_waiters: NonZeroUsize,
}

impl ChromiumExecutionWorldReadinessProfile {
    pub const fn from_topology(
        scope: ResourceScope,
        topology: CdpTopologyProductionProfile,
        max_waiters: NonZeroUsize,
    ) -> Self {
        Self {
            scope,
            max_frames: topology.limits().frames,
            max_waiters,
        }
    }

    pub const fn max_frames(self) -> usize {
        self.max_frames
    }
    pub const fn max_waiters(self) -> usize {
        self.max_waiters.get()
    }

    fn state_bytes(self) -> u64 {
        // Current and prepared bounded route arrays coexist until commit.
        FIXED_BYTES.saturating_add(
            u64::try_from(self.max_frames)
                .unwrap_or(u64::MAX)
                .saturating_mul(FRAME_PAIR_BYTES)
                .saturating_mul(2),
        )
    }

    pub fn requirements(self) -> ResourceRequirements {
        let bytes = self.state_bytes().saturating_add(
            u64::try_from(self.max_waiters())
                .unwrap_or(u64::MAX)
                .saturating_mul(WAITER_BYTES),
        );
        ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, self.max_waiters().saturating_add(1))
    }
}

pub(crate) struct ExecutionWorldReadiness {
    pub(crate) profile: ChromiumExecutionWorldReadinessProfile,
    ledger: ResourceLedger,
    waiters: AtomicUsize,
    changed: Notify,
    _state: ResourceLease,
}

impl ExecutionWorldReadiness {
    pub(crate) fn new(
        profile: ChromiumExecutionWorldReadinessProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, CdpPageRouteError> {
        let ledger = resources.claim(profile.requirements())?;
        let state = ledger.reserve(
            profile.scope,
            ResourceClass::BackendStateBytes,
            profile.state_bytes(),
        )?;
        Ok(Self {
            profile,
            ledger,
            waiters: AtomicUsize::new(0),
            changed: Notify::new(),
            _state: state,
        })
    }

    pub(crate) fn committed(&self) {
        self.changed.notify_waiters();
    }

    fn admit(&self) -> Result<WorldWaiter<'_>, CdpPageRouteError> {
        self.waiters
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < self.profile.max_waiters()).then(|| count + 1)
            })
            .map_err(|_| CdpPageRouteError::WorldReadinessCapacityExceeded {
                limit: self.profile.max_waiters(),
            })?;
        match self.ledger.reserve(
            self.profile.scope,
            ResourceClass::BackendStateBytes,
            WAITER_BYTES,
        ) {
            Ok(lease) => Ok(WorldWaiter {
                owner: self,
                _lease: lease,
            }),
            Err(error) => {
                self.waiters.fetch_sub(1, Ordering::AcqRel);
                Err(CdpPageRouteError::WorldReadinessResource(error))
            }
        }
    }

    pub(crate) async fn resolve<T>(
        &self,
        mut resolve: impl FnMut() -> Result<Option<T>, CdpPageRouteError>,
    ) -> Result<T, CdpPageRouteError> {
        if let Some(value) = resolve()? {
            return Ok(value);
        }
        let _waiter = self.admit()?;
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            // Register BEFORE inspecting the current world route. A commit
            // between inspection and await then cannot be lost or consumed by
            // another command. Notification alone never establishes readiness.
            changed.as_mut().enable();
            if let Some(value) = resolve()? {
                return Ok(value);
            }
            changed.await;
        }
    }
}

struct WorldWaiter<'a> {
    owner: &'a ExecutionWorldReadiness,
    _lease: ResourceLease,
}

impl Drop for WorldWaiter<'_> {
    fn drop(&mut self) {
        self.owner.waiters.fetch_sub(1, Ordering::AcqRel);
    }
}

impl CdpPageServices {
    async fn await_world_route<T>(
        &self,
        object_id: ObjectId,
        resolve: impl Fn(&CdpServiceRoutes, ObjectId) -> Option<T>,
    ) -> Result<T, CdpPageRouteError> {
        let readiness = self
            .world_readiness
            .as_ref()
            .expect("caller selected world readiness");
        readiness
            .resolve(|| {
                let routes = self
                    .routes
                    .lock()
                    .map_err(|_| CdpPageRouteError::StatePoisoned)?;
                let (frame_id, page_id) = match routes.pages.get(&object_id) {
                    Some(page) => (page.main_frame_object_id, object_id),
                    None => {
                        let index = routes
                            .readiness_frames
                            .binary_search_by_key(&object_id, |(frame, _)| *frame)
                            .map_err(|_| CdpPageRouteError::WorldTargetClosed { object_id })?;
                        (object_id, routes.readiness_frames[index].1)
                    }
                };
                let page = routes
                    .pages
                    .get(&page_id)
                    .ok_or(CdpPageRouteError::WorldTargetClosed { object_id })?;
                if page.crashed {
                    return Err(CdpPageRouteError::PageCrashed { object_id });
                }
                Ok(resolve(&routes, frame_id))
            })
            .await
    }

    pub(crate) fn ready_dom_content_route(
        &self,
        object_id: ObjectId,
    ) -> impl Future<Output = Result<Option<Arc<CdpDomContentService>>, CdpPageRouteError>> + Send + '_
    {
        // Default consumers retain their call-construction route snapshot.
        let immediate = self
            .world_readiness
            .is_none()
            .then(|| self.dom_content_route(object_id));
        async move {
            if let Some(result) = immediate {
                return result;
            }
            if self.dom_content.is_none() {
                return Ok(None);
            }
            self.await_world_route(object_id, |routes, frame| {
                routes
                    .frame_contexts
                    .get(&frame)
                    .and_then(|entry| entry.dom_content.clone())
            })
            .await
            .map(Some)
        }
    }

    pub(crate) fn ready_locator_route(
        &self,
        object_id: ObjectId,
    ) -> impl Future<Output = Result<Option<Arc<CdpLocatorService>>, CdpPageRouteError>> + Send + '_
    {
        let immediate = (self.world_readiness.is_none() || !self.locator_uses_utility_world)
            .then(|| self.locator_route(object_id));
        async move {
            if let Some(result) = immediate {
                return result;
            }
            if self.locator.is_none() {
                return Ok(None);
            }
            self.await_world_route(object_id, |routes, frame| {
                routes
                    .frame_contexts
                    .get(&frame)
                    .and_then(|entry| entry.locator.clone())
            })
            .await
            .map(Some)
        }
    }

    pub(crate) fn ready_evaluation_route(
        &self,
        object_id: ObjectId,
    ) -> impl Future<Output = Result<Option<CdpResolvedEvaluationRoute>, CdpPageRouteError>> + Send + '_
    {
        let immediate = self
            .world_readiness
            .is_none()
            .then(|| self.evaluation_route(object_id));
        async move {
            if let Some(result) = immediate {
                return result;
            }
            if self.evaluation.is_none() {
                return Ok(None);
            }
            self.await_world_route(object_id, |routes, frame| {
                routes
                    .frame_evaluations
                    .get(&frame)
                    .map(|entry| CdpResolvedEvaluationRoute {
                        page_object_id: entry.page_object_id,
                        service: Arc::clone(&entry.service),
                    })
            })
            .await
            .map(Some)
        }
    }
}
