//! Opt-in native creation followed by caller-serialized local publication.
//!
//! The caller must retain the creation future through native settlement and
//! retain the returned owner until publication or explicit asynchronous release.
//! Dropping an owner does not execute browser I/O. No worker, queue, retry loop,
//! runtime or shutdown policy is installed by this leaf. A language host must
//! pre-admit continuation storage before starting the future, including recovery
//! when a publication receiver disappears. Route/native storage uses the existing
//! selected ledgers; retained future/inbox storage belongs to that caller.

use super::*;
use pwright_composition_chromium_handle_lifecycle::ChromiumHandleRouteReservation;

mod batch;
pub use batch::{
    FailedNativeHandleBatchCreation, NativeHandleBatchCreationCommit,
    PendingNativeHandleBatchCreation,
};

/// One native result which is not yet present in the portable object graph.
#[must_use = "publish at a lifecycle boundary or explicitly release the native result"]
pub struct PendingNativeHandleCreation {
    service: ChromiumHandleEvaluationCommandService,
    object_id: ObjectId,
    route: HandleCreationRoute,
    native: CdpEvaluatedHandle,
    reservation: Option<ChromiumHandleRouteReservation>,
}

#[must_use = "retain a deferred owner or compensate a rejected native result"]
pub enum NativeHandleCreationCommit {
    Published(HandleInitializer),
    Deferred(PendingNativeHandleCreation),
    Rejected(FailedNativeHandleCreation),
}

/// A failed local publication which still owns native compensation.
#[must_use = "release the unpublished native result and report the combined failure"]
pub struct FailedNativeHandleCreation {
    creation: PendingNativeHandleCreation,
    error: OperationError<HandleEvaluationErrorDetails>,
}

impl ChromiumHandleEvaluationCommandService {
    /// Stage one exact iframe/frame owner Element in its parent's canonical
    /// main world. The caller supplies an independently admitted parent-world
    /// snapshot: both the session and world identity must still match this
    /// service's current parent route before `DOM.getFrameOwner` is sent.
    ///
    /// The caller owns cancellation continuation, publication and disposal.
    /// This method never searches page DOM or invents a portable Handle ID.
    pub async fn stage_frame_owner_element(
        &self,
        parent_frame: ObjectId,
        child_native_frame_id: &str,
        parent_client: Arc<dyn CdpClient>,
        parent_main_execution_context_id: i64,
        parent_main_unique_id: &str,
    ) -> Result<PendingNativeHandleCreation, OperationError<HandleEvaluationErrorDetails>> {
        validate_live(&self.runtime, parent_frame, "Frame")?;
        let route = self.frame_route(parent_frame)?;
        if !Arc::ptr_eq(&route.service.client(), &parent_client)
            || route.service.execution_context_id() != Some(parent_main_execution_context_id)
            || route.service.unique_context_id() != Some(parent_main_unique_id)
        {
            return Err(OperationError::new(
                FailureKind::TargetClosed,
                "parent Frame main-world route changed before owner lookup",
                Some(HandleEvaluationErrorDetails::Backend(
                    "parent Frame main-world route changed before owner lookup".into(),
                )),
            ));
        }
        // Durable route capacity and portable identity precede native
        // acquisition. A malformed or cancelled DOM.resolveNode cannot leave
        // an unadmitted browser object behind.
        let object_id = next_id(&self.object_ids)?;
        let reservation = self.handle_routes.reserve().map_err(map_route_error)?;
        // The portable ID is unique for this installed service. Do not copy a
        // browser-supplied world name into the native object group or turn a
        // valid, bounded world ID into an overlong group.
        let group = format!("pwright.frame-owner.{}", object_id.get());
        let limits = CdpHandleEvaluationLimits::new(
            route.service.limits(),
            self.native_id_bytes,
            self.preview_bytes,
        );
        let native =
            pwright_backend_cdp_handle_evaluation::CdpFrameOwnerElementService::new(limits)
                .resolve(
                    parent_client,
                    child_native_frame_id,
                    parent_main_execution_context_id,
                    &group,
                )
                .await?;
        Ok(PendingNativeHandleCreation {
            service: self.clone(),
            object_id,
            route: HandleCreationRoute {
                owner_frame: parent_frame,
                owner_page: route.page_id,
                evaluation: route.service,
                current_handles: Vec::new(),
            },
            native,
            reservation: Some(reservation),
        })
    }

    /// Execute once without publishing a portable Handle. Arguments/adoption,
    /// native result bounds and route admission match the ordinary command.
    /// A busy unrelated lifecycle transaction does not reject the native result.
    /// This raw future requires caller-owned cancellation/abandonment recovery.
    pub async fn stage_evaluate_handle(
        &self,
        frame: ObjectId,
        metadata: CallMetadata,
        params: EvaluateHandleParams,
    ) -> Result<PendingNativeHandleCreation, OperationError<HandleEvaluationErrorDetails>> {
        self.stage_evaluate_program(frame, metadata, params.into())
            .await
    }

    /// Native-only program; shares the exact staged-result recovery contract.
    pub async fn stage_evaluate_program(
        &self,
        frame: ObjectId,
        metadata: CallMetadata,
        params: CdpHandleEvaluationProgram,
    ) -> Result<PendingNativeHandleCreation, OperationError<HandleEvaluationErrorDetails>> {
        let creation = self.resolve_frame_creation(frame, metadata, params).await?;
        self.stage_resolved_creation(creation).await
    }

    /// Evaluate a program in the Frame's canonical world and retain exactly
    /// one Element result. An exact JavaScript `null` means that the atomic
    /// query found no element and creates no portable or native Handle.
    pub async fn stage_evaluate_optional_element_program(
        &self,
        frame: ObjectId,
        metadata: CallMetadata,
        params: CdpHandleEvaluationProgram,
    ) -> Result<Option<PendingNativeHandleCreation>, OperationError<HandleEvaluationErrorDetails>>
    {
        let creation = self.resolve_frame_creation(frame, metadata, params).await?;
        let pending = self.stage_resolved_creation(creation).await?;
        classify_optional_element(pending).await
    }

    /// Evaluate an element-producing program in one caller-selected execution
    /// world, then publish the eventual portable ElementHandle in the Frame's
    /// canonical world. Utility-world nodes are adopted before this method
    /// returns; the temporary source object is always released.
    ///
    /// The source route must come from the same live Page/CDP session as the
    /// canonical Frame route. As with the other staged methods, the caller must
    /// retain this future through native settlement and retain or compensate
    /// the returned owner.
    pub async fn stage_evaluate_element_program_in_world(
        &self,
        frame: ObjectId,
        metadata: CallMetadata,
        source: ChromiumHandleEvaluationRoute,
        params: CdpHandleEvaluationProgram,
    ) -> Result<PendingNativeHandleCreation, OperationError<HandleEvaluationErrorDetails>> {
        self.stage_evaluate_element_program_in_world_inner(frame, metadata, source, params, false)
            .await?
            .ok_or_else(non_node_selector_result)
    }

    /// World-selected variant of [`Self::stage_evaluate_optional_element_program`].
    /// A utility-world Element is adopted into the canonical Frame world, but
    /// an exact `null` is consumed without adoption or Handle publication.
    pub async fn stage_evaluate_optional_element_program_in_world(
        &self,
        frame: ObjectId,
        metadata: CallMetadata,
        source: ChromiumHandleEvaluationRoute,
        params: CdpHandleEvaluationProgram,
    ) -> Result<Option<PendingNativeHandleCreation>, OperationError<HandleEvaluationErrorDetails>>
    {
        self.stage_evaluate_element_program_in_world_inner(frame, metadata, source, params, true)
            .await
    }

    async fn stage_evaluate_element_program_in_world_inner(
        &self,
        frame: ObjectId,
        metadata: CallMetadata,
        source: ChromiumHandleEvaluationRoute,
        params: CdpHandleEvaluationProgram,
        allow_null: bool,
    ) -> Result<Option<PendingNativeHandleCreation>, OperationError<HandleEvaluationErrorDetails>>
    {
        validate_metadata(metadata, "Frame.querySelector")?;
        validate_live(&self.runtime, frame, "Frame")?;
        let target = self.frame_route(frame)?;
        if source.page_id != target.page_id
            || !Arc::ptr_eq(&source.service.client(), &target.service.client())
        {
            return Err(OperationError::new(
                FailureKind::ProtocolViolation,
                "element source world does not belong to the target Frame Page",
                Some(HandleEvaluationErrorDetails::InvalidValue(
                    "element source world does not belong to the target Frame Page".to_owned(),
                )),
            ));
        }
        let referenced = params
            .referenced_handles(source.service.limits().value)
            .map_err(invalid_argument)?;
        if !referenced.is_empty() {
            return Err(OperationError::new(
                FailureKind::ProtocolViolation,
                "world-selected element programs cannot reference portable Handles",
                Some(HandleEvaluationErrorDetails::InvalidValue(
                    "world-selected element programs cannot reference portable Handles".to_owned(),
                )),
            ));
        }

        // Admit the durable route before the browser can emit either the
        // source node or its adopted main-world wrapper.
        let object_id = next_id(&self.object_ids)?;
        let reservation = self.handle_routes.reserve().map_err(map_route_error)?;
        let source_native = CdpHandleEvaluationService::from_evaluation(
            source.service.clone(),
            self.native_id_bytes,
            self.preview_bytes,
        )
        .call(CdpHandleEvaluationRequest::new_program(
            params,
            None,
            Vec::new(),
        ))
        .await?;

        if source_native.kind() != CdpHandleKind::Element {
            if allow_null && source_native.is_null_primitive()? {
                // Primitive null owns no native object. Dropping the route
                // reservation proves that an absent query result published
                // neither native nor portable Handle state.
                drop(source_native);
                drop(reservation);
                return Ok(None);
            }
            return release_failed(source_native, non_node_selector_result()).await;
        }

        let native = if same_execution_world(&source.service, &target.service) {
            source_native
        } else {
            let Some(adopter) = self.persistent_element_adopter.as_ref() else {
                return release_failed(
                    source_native,
                    OperationError::new(
                        FailureKind::UnsupportedCapability,
                        "cross-world ElementHandle adoption is not selected",
                        Some(HandleEvaluationErrorDetails::InvalidValue(
                            "cross-world ElementHandle adoption is not selected".to_owned(),
                        )),
                    ),
                )
                .await;
            };
            let request = CdpElementHandleAdoptionRequest::new(
                source_native.clone(),
                target.service.client(),
                target
                    .service
                    .execution_context_id()
                    .expect("canonical Handle route has a numeric execution-context ID"),
            );
            let adopted = match adopter
                .call(CdpElementHandleAdoptionBatchRequest::new(vec![request]))
                .await
            {
                Ok(adopted) => adopted,
                Err(error) => return release_failed(source_native, error).await,
            };
            if let Err(source_cleanup) = source_native.release().await {
                let primary = OperationError::new(
                    FailureKind::Cleanup,
                    format!("temporary selector ElementHandle cleanup failed: {source_cleanup}"),
                    Some(HandleEvaluationErrorDetails::Cleanup {
                        primary: "selector node was adopted into the canonical world".to_owned(),
                        cleanup: source_cleanup.to_string(),
                    }),
                );
                return match adopted.release().await {
                    Ok(()) => Err(primary),
                    Err(cleanup) => Err(combine_adoption_cleanup(primary, cleanup)),
                };
            }
            let mut adopted = adopted.into_handles();
            debug_assert_eq!(adopted.len(), 1);
            adopted
                .pop()
                .expect("one adoption request returns one adopted Handle")
        };

        Ok(Some(PendingNativeHandleCreation {
            service: self.clone(),
            object_id,
            route: HandleCreationRoute {
                owner_frame: frame,
                owner_page: target.page_id,
                evaluation: target.service,
                current_handles: Vec::new(),
            },
            native,
            reservation: Some(reservation),
        }))
    }

    /// Execute against an existing Handle without publishing the result yet.
    /// Publication revalidates both the exact source world and the target, even
    /// when the program is an expression which ignores its target argument.
    pub async fn stage_evaluate_program_for_handle(
        &self,
        target: ObjectId,
        metadata: CallMetadata,
        params: CdpHandleEvaluationProgram,
    ) -> Result<PendingNativeHandleCreation, OperationError<HandleEvaluationErrorDetails>> {
        validate_metadata(metadata, "Handle.evaluateHandle")?;
        validate_live(&self.runtime, target, "Handle")?;
        let ids = params
            .referenced_handles(self.value_limits(target)?)
            .map_err(invalid_argument)?;
        let resolved = self.resolve_handle(target, &ids).await?;
        let mut current_handles = Vec::with_capacity(ids.len().saturating_add(1));
        current_handles.push(target);
        current_handles.extend(ids);
        let ResolvedArguments { arguments, adopted } = resolved.arguments;
        self.stage_resolved_creation(ResolvedFrameCreation {
            route: HandleCreationRoute {
                owner_frame: resolved.owner_frame,
                owner_page: resolved.owner_page,
                evaluation: resolved.evaluation,
                current_handles,
            },
            request: CdpHandleEvaluationRequest::new_program(
                params,
                Some((target, resolved.target)),
                arguments,
            ),
            adopted,
        })
        .await
    }

    /// Stage one property result with the same bounds and ownership contract as
    /// Handle evaluation. The property-access algorithm stays in the Rust leaf.
    pub async fn stage_get_property(
        &self,
        target: ObjectId,
        metadata: CallMetadata,
        params: GetPropertyParams,
    ) -> Result<PendingNativeHandleCreation, OperationError<HandleEvaluationErrorDetails>> {
        validate_metadata(metadata, "Handle.getProperty")?;
        if params.name().len() > self.property_limits.property_name_bytes() {
            return Err(OperationError::new(
                FailureKind::ResourceExhausted,
                format!(
                    "property name has {} bytes, limit is {}",
                    params.name().len(),
                    self.property_limits.property_name_bytes()
                ),
                Some(HandleEvaluationErrorDetails::InvalidValue(
                    "property name exceeds configured byte limit".to_owned(),
                )),
            ));
        }
        validate_live(&self.runtime, target, "Handle")?;
        let resolved = self.resolve_handle(target, &[]).await?;
        let object_id = next_id(&self.object_ids)?;
        // Admit the persistent result before either temporary snapshot creation
        // or property retrieval. The temporary never enters the portable graph.
        let reservation = self.handle_routes.reserve().map_err(map_route_error)?;
        let native = CdpHandlePropertiesService::from_evaluation(
            resolved.evaluation.clone(),
            self.native_id_bytes,
            self.preview_bytes,
            self.property_limits,
        )
        .call_property_on_handle(target, resolved.target, params.name())
        .await?;
        Ok(PendingNativeHandleCreation {
            service: self.clone(),
            object_id,
            route: HandleCreationRoute {
                owner_frame: resolved.owner_frame,
                owner_page: resolved.owner_page,
                evaluation: resolved.evaluation,
                current_handles: vec![target],
            },
            native,
            reservation: Some(reservation),
        })
    }

    async fn stage_resolved_creation(
        &self,
        creation: ResolvedFrameCreation,
    ) -> Result<PendingNativeHandleCreation, OperationError<HandleEvaluationErrorDetails>> {
        let object_id = next_id(&self.object_ids)?;
        let reservation = self.handle_routes.reserve().map_err(map_route_error)?;
        let outcome = CdpHandleEvaluationService::from_evaluation(
            creation.route.evaluation.clone(),
            self.native_id_bytes,
            self.preview_bytes,
        )
        .call(creation.request)
        .await;
        let native = finish_adopted_handle_creation(outcome, creation.adopted).await?;
        Ok(PendingNativeHandleCreation {
            service: self.clone(),
            object_id,
            route: creation.route,
            native,
            reservation: Some(reservation),
        })
    }
}

async fn classify_optional_element(
    pending: PendingNativeHandleCreation,
) -> Result<Option<PendingNativeHandleCreation>, OperationError<HandleEvaluationErrorDetails>> {
    if pending.native.kind() == CdpHandleKind::Element {
        return Ok(Some(pending));
    }
    if pending.native.is_null_primitive()? {
        // Primitive null has no Runtime objectId, so this is I/O-free while
        // still consuming the pending route reservation through ownership.
        pending.release().await?;
        return Ok(None);
    }
    let native = pending.native.clone();
    let result = release_failed(native, non_node_selector_result()).await;
    drop(pending);
    result
}

fn non_node_selector_result() -> OperationError<HandleEvaluationErrorDetails> {
    OperationError::new(
        FailureKind::ProtocolViolation,
        "selector engine returned a non-Node result",
        Some(HandleEvaluationErrorDetails::InvalidBackendResponse(
            "selector engine returned a non-Node result".to_owned(),
        )),
    )
}

impl PendingNativeHandleCreation {
    /// Typed native outcome for hosts with independent world-retirement proof.
    /// The caller retains its proof separately; a failed release is not proof.
    pub async fn release_native(self) -> Result<(), OperationError<HandleLifecycleErrorDetails>> {
        let result = self.native.release().await.map_err(|error| {
            OperationError::new(
                pwright_backend_cdp_common::classify_cdp_failure(&error),
                format!("unpublished native Handle release failed: {error}"),
                Some(HandleLifecycleErrorDetails::Backend(error.to_string())),
            )
        });
        drop(self);
        result
    }

    /// Frozen source identity is available even before a portable route exists.
    pub fn world_identity(&self) -> ChromiumHandleEvaluationRouteIdentity {
        ChromiumHandleEvaluationRouteIdentity {
            page_id: self.route.owner_page,
            execution_context_id: self
                .route
                .evaluation
                .execution_context_id()
                .expect("bound creation has a numeric execution context"),
            unique_context_id: self
                .route
                .evaluation
                .shared_unique_context_id()
                .expect("bound creation has a stable execution context"),
        }
    }

    /// Synchronous, native-I/O-free publication. A busy gate returns the exact
    /// owner and reservation; retry only at a later caller-owned boundary.
    /// Each attempt revalidates the source world and referenced Handles.
    pub fn commit_or_defer(mut self) -> NativeHandleCreationCommit {
        if let Err(error) = self
            .service
            .ensure_current_frame_route(
                self.route.owner_frame,
                self.route.owner_page,
                &self.route.evaluation,
            )
            .and_then(|()| {
                self.service
                    .ensure_current_handles(self.route.owner_page, &self.route.current_handles)
            })
        {
            return self.reject(error);
        }
        let prepared = match self.service.runtime.prepare_object_creation(
            self.object_id,
            Some(self.route.owner_frame),
            self.native.object_kind(),
        ) {
            Ok(prepared) => prepared,
            Err(RuntimeStateError::LifecycleTransactionInProgress) => {
                return NativeHandleCreationCommit::Deferred(self);
            }
            Err(error) => return self.reject(map_runtime_error(error)),
        };
        let initializer = HandleInitializer::new(
            self.object_id,
            self.route.owner_frame,
            self.native.object_kind(),
            self.native.preview().to_owned(),
        );
        let identity = self.world_identity();
        let payload = Arc::new(ChromiumEvaluatedHandleRoute {
            native: self.native.clone(),
            source_execution_context_id: identity.execution_context_id,
            source_unique_context_id: identity.unique_context_id,
        });
        let disposer = self
            .native
            .object_id()
            .map(|_| Arc::clone(&payload) as Arc<dyn ChromiumHandleDisposer>);
        let spec = ChromiumHandleRouteSpec::new(
            self.route.owner_frame,
            self.route.owner_page,
            self.native.native_argument_bytes(),
            self.native.preview().len(),
            disposer,
        )
        .with_payload(payload);
        let reservation = self
            .reservation
            .take()
            .expect("pending creation owns route admission");
        match reservation.publish_with(self.object_id, spec, || prepared.commit()) {
            Ok(()) => NativeHandleCreationCommit::Published(initializer),
            Err(error) => self.reject(map_publication_error(error)),
        }
    }

    fn reject(
        self,
        error: OperationError<HandleEvaluationErrorDetails>,
    ) -> NativeHandleCreationCommit {
        NativeHandleCreationCommit::Rejected(FailedNativeHandleCreation {
            creation: self,
            error,
        })
    }

    /// Abandon before publication, returning native failure instead of claiming
    /// success from a local drop. No portable object was registered or disposed.
    pub async fn release(self) -> Result<(), OperationError<HandleEvaluationErrorDetails>> {
        let result = self.native.release().await.map_err(|error| {
            OperationError::new(
                FailureKind::Cleanup,
                format!("unpublished Handle cleanup failed: {error}"),
                Some(HandleEvaluationErrorDetails::Cleanup {
                    primary: "Handle publication abandoned".to_owned(),
                    cleanup: error.to_string(),
                }),
            )
        });
        drop(self);
        result
    }
}

impl FailedNativeHandleCreation {
    /// Separate primary failure from typed native compensation for proof-based hosts.
    pub async fn release_native(
        self,
    ) -> (
        OperationError<HandleEvaluationErrorDetails>,
        Result<(), OperationError<HandleLifecycleErrorDetails>>,
    ) {
        let Self { creation, error } = self;
        (error, creation.release_native().await)
    }

    pub fn error(&self) -> &OperationError<HandleEvaluationErrorDetails> {
        &self.error
    }

    /// Preserve the original publication error and any native cleanup failure.
    pub async fn release(self) -> OperationError<HandleEvaluationErrorDetails> {
        let Self { creation, error } = self;
        let result = release_failed::<()>(creation.native.clone(), error).await;
        drop(creation);
        result.expect_err("failed publication always returns its primary error")
    }
}
