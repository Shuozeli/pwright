//! Read-only, opt-in receipts for exact retained ElementHandle composition.

use super::*;

/// A validated creation-route descriptor, NOT a native-use or route-lease pin.
///
/// This shares the existing bounded native/source payload without copying its
/// IDs. The caller must budget descriptor and shared-payload retention, retain
/// the actual native future and world owners, and validate again immediately
/// before adoption/mutation. Concurrent explicit disposal or native-world
/// destruction may still make acquisition fail after validation returns.
///
/// `owner_frame` and `owner_page` are creation ownership, not a query of the
/// node's current ownerDocument. This descriptor transfers no release authority
/// and exposes no disposer or owned `CdpEvaluatedHandle`.
#[derive(Clone)]
pub struct ChromiumRetainedElementSource {
    target: ObjectId,
    owner_frame: ObjectId,
    owner_page: ObjectId,
    payload: Arc<ChromiumEvaluatedHandleRoute>,
}

impl std::fmt::Debug for ChromiumRetainedElementSource {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ChromiumRetainedElementSource")
            .field("target", &self.target)
            .field("owner_frame", &self.owner_frame)
            .field("owner_page", &self.owner_page)
            .finish_non_exhaustive()
    }
}

impl ChromiumRetainedElementSource {
    pub const fn target(&self) -> ObjectId {
        self.target
    }

    pub fn client(&self) -> Arc<dyn CdpClient> {
        self.payload.native.client()
    }

    pub fn object_id(&self) -> &str {
        self.payload
            .native
            .object_id()
            .expect("validated Element source has a native object ID")
    }

    pub const fn owner_frame(&self) -> ObjectId {
        self.owner_frame
    }

    pub const fn owner_page(&self) -> ObjectId {
        self.owner_page
    }

    pub fn creation_execution_context_id(&self) -> i64 {
        self.payload.source_execution_context_id
    }

    pub fn creation_unique_context_id(&self) -> &str {
        &self.payload.source_unique_context_id
    }

    /// Shares the already-retained unique-world identity; performs no I/O.
    pub fn world_identity(&self) -> ChromiumHandleEvaluationRouteIdentity {
        ChromiumHandleEvaluationRouteIdentity {
            page_id: self.owner_page,
            execution_context_id: self.payload.source_execution_context_id,
            unique_context_id: Arc::clone(&self.payload.source_unique_context_id),
        }
    }
}

impl ChromiumHandleEvaluationCommandService {
    /// Synchronously borrow the exact live Element source. This neither issues
    /// browser calls nor adopts, publishes, releases or pins a native object.
    pub fn borrow_element_source(
        &self,
        target: ObjectId,
    ) -> Result<ChromiumRetainedElementSource, OperationError<HandleEvaluationErrorDetails>> {
        validate_live(&self.runtime, target, "ElementHandle")?;
        let route = self.handle_snapshot(target)?;
        validate_live(&self.runtime, route.owner_frame(), "Frame")?;
        validate_live(&self.runtime, route.owner_page(), "Page")?;
        let payload = route.payload();
        let frame = self.frame_route(route.owner_frame())?;
        if route.owner_page() != frame.page_id || !handle_belongs_to_world(&payload, &frame.service)
        {
            return Err(foreign_handle(target));
        }
        if payload.native.kind() != CdpHandleKind::Element
            || payload
                .native
                .object_id()
                .is_none_or(|id| id.is_empty() || id.len() > self.native_id_bytes)
        {
            return Err(OperationError::new(
                FailureKind::ProtocolViolation,
                "retained element source requires an ElementHandle with a bounded native ID",
                None,
            ));
        }
        Ok(ChromiumRetainedElementSource {
            target,
            owner_frame: route.owner_frame(),
            owner_page: route.owner_page(),
            payload,
        })
    }

    /// Rechecks current runtime, route and creation-world identity plus the
    /// exact source receipt. This is a synchronous check, not a native lock.
    pub fn validate_element_source(
        &self,
        target: ObjectId,
        source: &ChromiumRetainedElementSource,
    ) -> Result<(), OperationError<HandleEvaluationErrorDetails>> {
        let current = self.borrow_element_source(target)?;
        if source.target != target
            || source.owner_frame != current.owner_frame
            || source.owner_page != current.owner_page
            || !Arc::ptr_eq(&source.payload, &current.payload)
            || !source.payload.native.belongs_to_client(&current.client())
            || source.object_id() != current.object_id()
            || source.creation_execution_context_id() != current.creation_execution_context_id()
            || source.creation_unique_context_id() != current.creation_unique_context_id()
        {
            return Err(foreign_handle(target));
        }
        Ok(())
    }
}
