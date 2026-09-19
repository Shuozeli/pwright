//! Optional native release followed by a short, exact-incarnation local commit.
//! No executor, retry loop or topology scheduler is installed here. The caller
//! owns native future/continuation storage and must continue written I/O when a
//! language waiter is abandoned. Runtime admission is retained through deferral.

use super::*;
use pwright_runtime::disposal_reservations::{
    DeferredDisposalCommit, DisposalPreparation, DisposalReservationError, ReservedDisposalCapacity,
};

/// Logical retained Arc allocation payload/header, charged per selected route.
/// Allocator overhead and the caller's native future storage remain separate.
pub const ROUTE_OWNER_BYTES: u64 =
    (std::mem::size_of::<ResourceLease>() + 2 * std::mem::size_of::<usize>()) as u64;

#[must_use = "drive native release with an admitted continuation or drop before native I/O"]
pub struct NativeHandleRelease {
    receipt: NativeHandleReleaseReceipt,
    disposer: Option<Arc<dyn ChromiumHandleDisposer>>,
}

/// Constructible only after the injected native disposer acknowledges success
/// (or for a primitive route that has no native object). Retains the exact
/// route lease allocation so address reuse cannot match a replacement route.
#[must_use = "commit or retain acknowledged native release for local retirement"]
pub struct NativeHandleReleaseReceipt {
    runtime: ControlledLifecycleHandle,
    routes: ChromiumHandleRoutes,
    object_id: ObjectId,
    identity: Arc<ResourceLease>,
    capacity: ReservedDisposalCapacity,
}

#[must_use = "notify committed retirement after outer locks or retain deferred receipt"]
pub enum NativeHandleReleaseCommit {
    Committed(DeferredDisposalCommit),
    Deferred(NativeHandleReleaseReceipt),
    /// Exact old route no longer exists. No current runtime object is touched.
    Superseded,
}

impl ChromiumHandleDisposeCommandService {
    /// Admit the local commit and capture the current route before native I/O.
    /// None means runtime retirement was already observed, not native success.
    pub fn prepare_native_release(
        &self,
        object_id: ObjectId,
        capacity: ReservedDisposalCapacity,
    ) -> Result<Option<NativeHandleRelease>, OperationError<HandleLifecycleErrorDetails>> {
        capacity
            .validate_deferred_commit(&self.runtime)
            .map_err(map_capacity)?;
        if !self
            .runtime
            .is_object_live(object_id)
            .map_err(map_runtime_error)?
        {
            return Ok(None);
        }
        let state = self
            .routes
            .state
            .lock()
            .map_err(|_| map_route_error(ChromiumHandleRouteError::StatePoisoned))?;
        let route = state
            .routes
            .get(&object_id)
            .ok_or_else(|| map_route_error(ChromiumHandleRouteError::Missing { object_id }))?;
        Ok(Some(NativeHandleRelease {
            receipt: NativeHandleReleaseReceipt {
                runtime: self.runtime.clone(),
                routes: self.routes.clone(),
                object_id,
                identity: route._resource.clone(),
                capacity,
            },
            disposer: route.disposer.clone(),
        }))
    }
}

impl NativeHandleRelease {
    /// Performs only native release. Neither a runtime transaction nor a route
    /// lock is held across this await. Failure preserves the current local route.
    pub async fn release(
        self,
    ) -> Result<NativeHandleReleaseReceipt, OperationError<HandleLifecycleErrorDetails>> {
        if let Some(disposer) = self.disposer {
            disposer.dispose().await?;
        }
        Ok(self.receipt)
    }
}

impl NativeHandleReleaseReceipt {
    pub const fn object_id(&self) -> ObjectId {
        self.object_id
    }

    /// Call at a coordinator-owned serialized local boundary. Lock order is
    /// Handle routes then runtime mutex; no native wait or user notification.
    /// A busy gate returns the same receipt and capacity; retry ONLY this local
    /// phase. Errors after native success require caller-controlled recovery.
    pub fn commit_or_defer(
        self,
    ) -> Result<NativeHandleReleaseCommit, OperationError<HandleLifecycleErrorDetails>> {
        let mut state = self
            .routes
            .state
            .lock()
            .map_err(|_| map_route_error(ChromiumHandleRouteError::StatePoisoned))?;
        if state
            .routes
            .get(&self.object_id)
            .is_none_or(|route| !Arc::ptr_eq(&route._resource, &self.identity))
        {
            return Ok(NativeHandleReleaseCommit::Superseded);
        }
        let prepared = match self
            .capacity
            .prepare_or_defer(&self.runtime, self.object_id)
            .map_err(map_capacity)?
        {
            DisposalPreparation::Prepared(prepared) => prepared,
            DisposalPreparation::Deferred(capacity) => {
                drop(state);
                return Ok(NativeHandleReleaseCommit::Deferred(Self {
                    capacity,
                    ..self
                }));
            }
        };
        // JSHandle/ElementHandle are leaves owned by a Frame, not parents of
        // other handles. Never silently retire a non-Handle runtime subtree.
        if prepared.disposal_order() != [self.object_id] {
            return Err(OperationError::new(
                FailureKind::ProtocolViolation,
                "native Handle retirement requires one live leaf object",
                None,
            ));
        }
        let (_, committed) = prepared
            .commit_deferred_with_publication(None, || {
                Ok::<_, Infallible>(retire_locked(
                    &mut state,
                    &[self.object_id],
                    ChromiumHandleRetirementReason::Explicit,
                ))
            })
            .map_err(map_capacity)?
            .unwrap_or_else(|never| match never {});
        drop(state);
        Ok(NativeHandleReleaseCommit::Committed(committed))
    }
}

fn map_capacity(error: DisposalReservationError) -> OperationError<HandleLifecycleErrorDetails> {
    match error {
        DisposalReservationError::Runtime(error) => map_runtime_error(error),
        DisposalReservationError::Invalid(message) => {
            OperationError::new(FailureKind::ProtocolViolation, message, None)
        }
        DisposalReservationError::SlotsExhausted { limit } => OperationError::new(
            FailureKind::ResourceExhausted,
            format!("native Handle release capacity exceeded ({limit})"),
            None,
        ),
    }
}
