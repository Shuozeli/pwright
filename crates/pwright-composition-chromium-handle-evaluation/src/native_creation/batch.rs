//! Pre-admitted, all-or-none native property-batch publication.

use super::*;
use pwright_composition_chromium_handle_lifecycle::ChromiumHandleRouteBatchReservation;

/// A bounded set of siblings. The caller must retain this owner through either
/// local publication or asynchronous attempt-all compensation, just as for a
/// single staged result. Dropping it performs no native I/O.
#[must_use = "publish the whole batch or explicitly release every native result"]
pub struct PendingNativeHandleBatchCreation {
    service: ChromiumHandleEvaluationCommandService,
    object_ids: Vec<ObjectId>,
    route: HandleCreationRoute,
    native: Vec<(String, CdpEvaluatedHandle)>,
    reservation: Option<ChromiumHandleRouteBatchReservation>,
}

#[must_use = "retain a deferred batch or compensate all rejected results"]
pub enum NativeHandleBatchCreationCommit {
    Published(HandlePropertiesResult),
    Deferred(PendingNativeHandleBatchCreation),
    Rejected(FailedNativeHandleBatchCreation),
}

#[must_use = "release the unpublished native batch and report the combined failure"]
pub struct FailedNativeHandleBatchCreation {
    creation: PendingNativeHandleBatchCreation,
    error: OperationError<HandleEvaluationErrorDetails>,
}

impl ChromiumHandleEvaluationCommandService {
    /// Stage a bounded batch of `RemoteObject`s already emitted by the current
    /// main execution world. This performs no JavaScript evaluation or CDP I/O.
    ///
    /// The event producer owns the supplied native objects until this returns
    /// `Ok`. On `Err`, it must release every object-bearing RemoteObject because
    /// validation may have stopped partway through the batch.
    pub fn stage_remote_objects(
        &self,
        frame: ObjectId,
        remotes: &[serde_json::Value],
    ) -> Result<PendingNativeHandleBatchCreation, OperationError<HandleEvaluationErrorDetails>>
    {
        validate_live(&self.runtime, frame, "Frame")?;
        if remotes.len() > self.property_limits.max_properties() {
            return Err(batch_capacity_error());
        }
        let resolved = self.frame_route(frame)?;
        let route = HandleCreationRoute {
            owner_frame: frame,
            owner_page: resolved.page_id,
            evaluation: resolved.service,
            current_handles: Vec::new(),
        };
        if remotes.is_empty() {
            return Ok(PendingNativeHandleBatchCreation {
                service: self.clone(),
                object_ids: Vec::new(),
                route,
                native: Vec::new(),
                reservation: None,
            });
        }
        let reservation = self
            .handle_routes
            .reserve_batch(remotes.len())
            .map_err(map_route_error)?;
        let mut object_ids = Vec::with_capacity(remotes.len());
        for _ in remotes {
            object_ids.push(next_id(&self.object_ids)?);
        }
        let limits = CdpHandleEvaluationLimits::new(
            route.evaluation.limits(),
            self.native_id_bytes,
            self.preview_bytes,
        );
        let client = route.evaluation.client();
        let native = remotes
            .iter()
            .enumerate()
            .map(|(index, remote)| {
                CdpEvaluatedHandle::from_remote_object(client.clone(), remote, limits)
                    .map(|handle| (index.to_string(), handle))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PendingNativeHandleBatchCreation {
            service: self.clone(),
            object_ids,
            route,
            native,
            reservation: Some(reservation),
        })
    }

    /// Admit `maximum_results` portable slots before any browser I/O. The host
    /// separately admits sibling cleanup tickets plus one aggregate continuation.
    /// Native temporary peak is one carrier plus N+1 overflow-detection values;
    /// only N results may leave the backend and enter this publication owner.
    pub async fn stage_get_properties(
        &self,
        target: ObjectId,
        metadata: CallMetadata,
        maximum_results: usize,
    ) -> Result<PendingNativeHandleBatchCreation, OperationError<HandleEvaluationErrorDetails>>
    {
        validate_metadata(metadata, "Handle.getProperties")?;
        validate_live(&self.runtime, target, "Handle")?;
        if maximum_results > self.property_limits.max_properties() {
            return Err(batch_capacity_error());
        }
        let resolved = self.resolve_handle(target, &[]).await?;
        let route = HandleCreationRoute {
            owner_frame: resolved.owner_frame,
            owner_page: resolved.owner_page,
            evaluation: resolved.evaluation,
            current_handles: vec![target],
        };
        // Primitive getProperties is the existing no-I/O empty operation. It
        // needs no result slots, but still carries and validates source identity.
        if resolved.target.object_id().is_none() {
            return Ok(PendingNativeHandleBatchCreation {
                service: self.clone(),
                object_ids: Vec::new(),
                route,
                native: Vec::new(),
                reservation: None,
            });
        }
        if maximum_results == 0 {
            return Err(batch_capacity_error());
        }
        let mut reservation = self
            .handle_routes
            .reserve_batch(maximum_results)
            .map_err(map_route_error)?;
        // IDs are also acquired before browser work, not fabricated from one
        // CallId: every sibling gets its own globally allocated object identity.
        let mut object_ids = Vec::with_capacity(maximum_results);
        for _ in 0..maximum_results {
            object_ids.push(next_id(&self.object_ids)?);
        }
        let limits = CdpHandlePropertyLimits::new(
            maximum_results,
            self.property_limits.property_name_bytes(),
        )
        .map_err(|_| batch_capacity_error())?;
        let native = CdpHandlePropertiesService::from_evaluation(
            route.evaluation.clone(),
            self.native_id_bytes,
            self.preview_bytes,
            limits,
        )
        .call_on_handle(target, resolved.target)
        .await?;
        // Backend count validation precedes success. Trim while native ownership
        // and the original maximum reservation are both still in this future.
        let trimmed = reservation.trim_to(native.len());
        object_ids.truncate(native.len());
        let creation = PendingNativeHandleBatchCreation {
            service: self.clone(),
            object_ids,
            route,
            native,
            reservation: Some(reservation),
        };
        if let Err(error) = trimmed {
            return Err(FailedNativeHandleBatchCreation {
                creation,
                error: map_route_error(error),
            }
            .release()
            .await);
        }
        Ok(creation)
    }
}

impl PendingNativeHandleBatchCreation {
    pub fn len(&self) -> usize {
        self.native.len()
    }

    pub fn is_empty(&self) -> bool {
        self.native.is_empty()
    }

    pub fn world_identity(&self) -> ChromiumHandleEvaluationRouteIdentity {
        ChromiumHandleEvaluationRouteIdentity {
            page_id: self.route.owner_page,
            execution_context_id: self
                .route
                .evaluation
                .execution_context_id()
                .expect("bound batch creation has a numeric execution context"),
            unique_context_id: self
                .route
                .evaluation
                .shared_unique_context_id()
                .expect("bound batch creation has a stable execution context"),
        }
    }

    /// No native I/O or partial publication. A busy lifecycle gate retains the
    /// same IDs, results and reservation for a later caller-owned boundary.
    pub fn commit_or_defer(mut self) -> NativeHandleBatchCreationCommit {
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
        if self.native.is_empty() {
            return NativeHandleBatchCreationCommit::Published(HandlePropertiesResult::default());
        }
        let specs = self
            .object_ids
            .iter()
            .zip(&self.native)
            .map(|(id, (_, native))| {
                ObjectCreationSpec::new(*id, Some(self.route.owner_frame), native.object_kind())
            })
            .collect::<Vec<_>>();
        let prepared = match self.service.runtime.prepare_object_creation_batch(&specs) {
            Ok(prepared) => prepared,
            Err(RuntimeStateError::LifecycleTransactionInProgress) => {
                return NativeHandleBatchCreationCommit::Deferred(self);
            }
            Err(error) => return self.reject(map_runtime_error(error)),
        };
        let identity = self.world_identity();
        let initializers = self
            .object_ids
            .iter()
            .zip(&self.native)
            .map(|(id, (name, native))| {
                HandlePropertyInitializer::new(
                    name.clone(),
                    HandleInitializer::new(
                        *id,
                        self.route.owner_frame,
                        native.object_kind(),
                        native.preview().to_owned(),
                    ),
                )
            })
            .collect();
        let entries = self
            .object_ids
            .iter()
            .zip(&self.native)
            .map(|(id, (_, native))| {
                let payload = Arc::new(ChromiumEvaluatedHandleRoute {
                    native: native.clone(),
                    source_execution_context_id: identity.execution_context_id,
                    source_unique_context_id: Arc::clone(&identity.unique_context_id),
                });
                let disposer = native
                    .object_id()
                    .map(|_| Arc::clone(&payload) as Arc<dyn ChromiumHandleDisposer>);
                (
                    *id,
                    ChromiumHandleRouteSpec::new(
                        self.route.owner_frame,
                        self.route.owner_page,
                        native.native_argument_bytes(),
                        native.preview().len(),
                        disposer,
                    )
                    .with_payload(payload),
                )
            })
            .collect();
        let reservation = self
            .reservation
            .take()
            .expect("nonempty batch owns route admission");
        match reservation.publish_with(entries, || prepared.commit()) {
            Ok(()) => NativeHandleBatchCreationCommit::Published(HandlePropertiesResult::new(
                initializers,
            )),
            Err(error) => self.reject(map_publication_error(error)),
        }
    }

    fn reject(
        self,
        error: OperationError<HandleEvaluationErrorDetails>,
    ) -> NativeHandleBatchCreationCommit {
        NativeHandleBatchCreationCommit::Rejected(FailedNativeHandleBatchCreation {
            creation: self,
            error,
        })
    }

    /// Attempt every sibling release. A generic failure must not be hidden by
    /// an earlier TargetClosed result when a host applies world-retirement proof.
    pub async fn release_native(self) -> Result<(), OperationError<HandleLifecycleErrorDetails>> {
        let limit = self.route.evaluation.limits().error_bytes;
        let mut failure: Option<OperationError<HandleLifecycleErrorDetails>> = None;
        for (_, native) in &self.native {
            let Some(id) = native.object_id() else {
                continue;
            };
            if let Err(error) = native
                .client()
                .runtime_release_object_to_settlement(id)
                .await
            {
                let kind = pwright_backend_cdp_common::classify_cdp_failure(&error);
                let replace = failure.as_ref().is_none_or(|previous| {
                    previous.kind() == FailureKind::TargetClosed
                        && kind != FailureKind::TargetClosed
                });
                if replace {
                    failure = Some(OperationError::new(
                        kind,
                        diagnostic(
                            format_args!("unpublished native Handle batch release failed: {error}"),
                            limit,
                        ),
                        Some(HandleLifecycleErrorDetails::Backend(diagnostic(
                            error, limit,
                        ))),
                    ));
                }
            }
        }
        drop(self);
        failure.map_or(Ok(()), Err)
    }

    pub async fn release(self) -> Result<(), OperationError<HandleEvaluationErrorDetails>> {
        let limit = self.route.evaluation.limits().error_bytes;
        self.release_native().await.map_err(|error| {
            batch_cleanup_error("Handle batch publication abandoned", error.message(), limit)
        })
    }
}

impl FailedNativeHandleBatchCreation {
    pub fn error(&self) -> &OperationError<HandleEvaluationErrorDetails> {
        &self.error
    }

    pub async fn release_native(
        self,
    ) -> (
        OperationError<HandleEvaluationErrorDetails>,
        Result<(), OperationError<HandleLifecycleErrorDetails>>,
    ) {
        let Self { creation, error } = self;
        (error, creation.release_native().await)
    }

    pub async fn release(self) -> OperationError<HandleEvaluationErrorDetails> {
        let limit = self.creation.route.evaluation.limits().error_bytes;
        let (primary, cleanup) = self.release_native().await;
        match cleanup {
            Ok(()) => primary,
            Err(cleanup) => batch_cleanup_error(primary.message(), cleanup.message(), limit),
        }
    }
}

fn batch_capacity_error() -> OperationError<HandleEvaluationErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        "Handle property batch exceeds caller-admitted result capacity",
        None,
    )
}

fn batch_cleanup_error(
    primary: &str,
    cleanup: &str,
    limit: usize,
) -> OperationError<HandleEvaluationErrorDetails> {
    OperationError::new(
        FailureKind::Cleanup,
        diagnostic(format_args!("{primary}; cleanup failed: {cleanup}"), limit),
        Some(HandleEvaluationErrorDetails::Cleanup {
            primary: diagnostic(primary, limit),
            cleanup: diagnostic(cleanup, limit),
        }),
    )
}

fn diagnostic(value: impl Display, limit: usize) -> String {
    use std::fmt::Write as _;
    struct Bounded {
        text: String,
        limit: usize,
    }
    impl std::fmt::Write for Bounded {
        fn write_str(&mut self, value: &str) -> std::fmt::Result {
            let mut end = value.len().min(self.limit.saturating_sub(self.text.len()));
            while !value.is_char_boundary(end) {
                end -= 1;
            }
            self.text.push_str(&value[..end]);
            if end < value.len() {
                Err(std::fmt::Error)
            } else {
                Ok(())
            }
        }
    }
    let mut output = Bounded {
        text: String::new(),
        limit,
    };
    let _ = write!(output, "{value}");
    output.text
}
