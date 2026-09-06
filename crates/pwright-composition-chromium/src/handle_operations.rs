//! Chromium composition for Playwright-shaped Handle-local operations.

use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use pwright_backend_cdp::{
    CdpEvaluatedHandle, CdpEvaluationService, CdpHandleEvaluationRequest,
    CdpHandleEvaluationService, CdpHandlePropertiesService, CdpHandlePropertyLimits,
    CdpHandleValueEvaluationService,
};
use pwright_cap_handle_evaluation::{
    GetPropertiesOptions, GetPropertyParams, HandleEvaluate, HandleEvaluateHandle,
    HandleEvaluateParams, HandleEvaluationErrorDetails, HandleEvaluationValue, HandleGetProperties,
    HandleGetProperty, HandleInitializer, HandleJsonValue, HandlePropertiesResult,
    HandlePropertyInitializer, HandleValueResult, JsonValueOptions,
};
use pwright_cap_handle_lifecycle::HandleDispose;
use pwright_cdp::CdpClient;
use pwright_core::{
    CallId, CallMetadata, Command, CommandResult, CommandSpec, ControlMessage, FailureKind,
    ObjectId, ObjectIdSource, OperationError, ResourceClass, ResourceLease, Service,
};
use pwright_runtime::{
    CleanupFailure, CleanupReservation, CleanupSubmitError, ControlledLifecycleHandle,
    ObjectCreationSpec, PreparedObjectCreationBatch, ReservedCleanupExecutor,
};
use pwright_runtime_control::{
    ControlMessageSource, OrderedRuntimeControl, OrderedRuntimeControlBarrier,
};
use pwright_transport::{FrameTransport, WireCodec};

use crate::{
    CdpPageServices, ChromiumHandleRouteEntry, ChromiumHandleRoutes,
    ResolvedChromiumHandleArguments, handle_capacity_error, handle_route_resource_error,
    handle_route_state_error, map_control_gate_failure, map_runtime_handle_evaluation_error,
    publish_handle_route, release_failed_handle, resolve_handle_arguments,
};

#[derive(Clone)]
pub struct ChromiumHandleOperationCommandService {
    page_services: CdpPageServices,
    runtime: ControlledLifecycleHandle,
    object_ids: Arc<dyn ObjectIdSource>,
    routes: ChromiumHandleRoutes,
    native_id_bytes: usize,
    preview_bytes: usize,
    property_limits: CdpHandlePropertyLimits,
}

/// Typed placeholder used only by compositions that intentionally negotiate
/// Handle evaluation below version 1.2. The 1.1 codec rejects these methods
/// before dispatch, while keeping the generated handler tree fully typed.
#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableHandleOperationCommandService;

/// Remote-only decorator that publishes every returned Handle `Create` on the
/// shared ordered lifecycle stream before exposing the typed command result.
pub struct OrderedHandleOperationCommandService<Inner, Dispose, Source, Codec, Transport> {
    service: Arc<Inner>,
    dispose: Arc<Dispose>,
    control: OrderedRuntimeControl<Source, Codec, Transport>,
}

impl<Inner, Dispose, Source, Codec, Transport>
    OrderedHandleOperationCommandService<Inner, Dispose, Source, Codec, Transport>
{
    pub const fn new(
        service: Arc<Inner>,
        dispose: Arc<Dispose>,
        control: OrderedRuntimeControl<Source, Codec, Transport>,
    ) -> Self {
        Self {
            service,
            dispose,
            control,
        }
    }
}

pub trait HandleCreationCommand: CommandSpec<ErrorDetails = HandleEvaluationErrorDetails> {
    fn created_handles(result: &Self::Result) -> Vec<HandleInitializer>;
    fn operation_name() -> &'static str;
}

/// Reserves bounded continuation ownership for Handle-producing local
/// operations.
pub struct ContinuableHandleOperationCommandService<Inner, Dispose, Continuation>
where
    Continuation: ReservedCleanupExecutor,
{
    service: Arc<Inner>,
    dispose: Arc<Dispose>,
    continuation: Continuation,
}

impl<Inner, Dispose, Continuation>
    ContinuableHandleOperationCommandService<Inner, Dispose, Continuation>
where
    Continuation: ReservedCleanupExecutor,
{
    pub const fn new(
        service: Arc<Inner>,
        dispose: Arc<Dispose>,
        continuation: Continuation,
    ) -> Self {
        Self {
            service,
            dispose,
            continuation,
        }
    }
}

type BoxHandleOperationFuture<Spec> =
    Pin<Box<dyn Future<Output = Result<CommandResult<Spec>, Infallible>> + Send + 'static>>;

pub struct ContinuableHandleOperationFuture<Spec, Dispose, Reservation>
where
    Spec: HandleCreationCommand,
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Reservation: CleanupReservation,
{
    inner: Option<BoxHandleOperationFuture<Spec>>,
    dispose: Option<Arc<Dispose>>,
    reservation: Option<Reservation>,
    call_id: CallId,
    started: bool,
}

impl<Spec, Dispose, Reservation> Unpin
    for ContinuableHandleOperationFuture<Spec, Dispose, Reservation>
where
    Spec: HandleCreationCommand,
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Reservation: CleanupReservation,
{
}

impl<Spec, Dispose, Reservation> Future
    for ContinuableHandleOperationFuture<Spec, Dispose, Reservation>
where
    Spec: HandleCreationCommand,
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Reservation: CleanupReservation,
{
    type Output = Result<CommandResult<Spec>, Infallible>;

    fn poll(
        mut self: Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.started = true;
        let outcome = self
            .inner
            .as_mut()
            .expect("completed Handle operation future must not be polled again")
            .as_mut()
            .poll(context);
        if outcome.is_ready() {
            self.inner = None;
            self.dispose = None;
            self.reservation = None;
        }
        outcome
    }
}

impl<Spec, Dispose, Reservation> Drop
    for ContinuableHandleOperationFuture<Spec, Dispose, Reservation>
where
    Spec: HandleCreationCommand,
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Reservation: CleanupReservation,
{
    fn drop(&mut self) {
        if !self.started {
            return;
        }
        let Some(inner) = self.inner.take() else {
            return;
        };
        let Some(dispose) = self.dispose.take() else {
            return;
        };
        let Some(reservation) = self.reservation.take() else {
            return;
        };
        let call_id = self.call_id;
        reservation.submit(Box::pin(async move {
            let handles = match inner.await {
                Ok(result) => match result.into_outcome() {
                    Ok(value) => Spec::created_handles(&value),
                    Err(_) => return Ok(()),
                },
                Err(never) => match never {},
            };
            dispose_abandoned_handles(dispose, call_id, handles).await
        }));
    }
}

impl HandleCreationCommand for HandleEvaluateHandle {
    fn created_handles(result: &Self::Result) -> Vec<HandleInitializer> {
        vec![result.clone()]
    }

    fn operation_name() -> &'static str {
        "Handle.evaluateHandle"
    }
}

impl HandleCreationCommand for HandleGetProperty {
    fn created_handles(result: &Self::Result) -> Vec<HandleInitializer> {
        vec![result.clone()]
    }

    fn operation_name() -> &'static str {
        "Handle.getProperty"
    }
}

impl HandleCreationCommand for HandleGetProperties {
    fn created_handles(result: &Self::Result) -> Vec<HandleInitializer> {
        result
            .properties
            .iter()
            .map(|property| property.handle.clone())
            .collect()
    }

    fn operation_name() -> &'static str {
        "Handle.getProperties"
    }
}

impl ChromiumHandleOperationCommandService {
    pub fn new(
        page_services: CdpPageServices,
        runtime: ControlledLifecycleHandle,
        object_ids: Arc<dyn ObjectIdSource>,
        routes: ChromiumHandleRoutes,
        native_id_bytes: usize,
        preview_bytes: usize,
        property_limits: CdpHandlePropertyLimits,
    ) -> Self {
        Self {
            page_services,
            runtime,
            object_ids,
            routes,
            native_id_bytes,
            preview_bytes,
            property_limits,
        }
    }

    async fn evaluate_value(
        &self,
        target_id: ObjectId,
        metadata: CallMetadata,
        params: HandleEvaluateParams,
    ) -> Result<HandleValueResult, OperationError<HandleEvaluationErrorDetails>> {
        validate_metadata(metadata, "Handle.evaluate")?;
        validate_live_handle(&self.runtime, target_id)?;
        let handles = params
            .referenced_handles(self.value_limits(target_id)?)
            .map_err(invalid_argument)?;
        let resolved = self.resolve(target_id, &handles)?;
        let backend = CdpHandleValueEvaluationService::from_evaluation(
            resolved.evaluation,
            self.native_id_bytes,
            self.preview_bytes,
        );
        backend
            .call(CdpHandleEvaluationRequest::new_on_handle(
                params.as_frame_params(),
                (target_id, resolved.target),
                resolved.arguments,
            ))
            .await
            .map(HandleValueResult::new)
    }

    async fn evaluate_handle(
        &self,
        target_id: ObjectId,
        metadata: CallMetadata,
        params: HandleEvaluateParams,
    ) -> Result<HandleInitializer, OperationError<HandleEvaluationErrorDetails>> {
        validate_metadata(metadata, "Handle.evaluateHandle")?;
        self.create_handle(target_id, params).await
    }

    async fn json_value(
        &self,
        target_id: ObjectId,
        metadata: CallMetadata,
    ) -> Result<HandleValueResult, OperationError<HandleEvaluationErrorDetails>> {
        validate_metadata(metadata, "Handle.jsonValue")?;
        self.evaluate_value(
            target_id,
            CallMetadata::default(),
            HandleEvaluateParams::function_without_argument("target => target"),
        )
        .await
    }

    async fn get_property(
        &self,
        target_id: ObjectId,
        metadata: CallMetadata,
        params: GetPropertyParams,
    ) -> Result<HandleInitializer, OperationError<HandleEvaluationErrorDetails>> {
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
        self.create_handle(
            target_id,
            HandleEvaluateParams::function(
                "(target, name) => target[name]",
                HandleEvaluationValue::String(params.name().to_owned()),
            ),
        )
        .await
    }

    async fn get_properties(
        &self,
        target_id: ObjectId,
        metadata: CallMetadata,
    ) -> Result<HandlePropertiesResult, OperationError<HandleEvaluationErrorDetails>> {
        validate_metadata(metadata, "Handle.getProperties")?;
        validate_live_handle(&self.runtime, target_id)?;
        let resolved = self.resolve(target_id, &[])?;
        let owner_frame = resolved.owner_frame;
        let owner_page = resolved.owner_page;
        let client = resolved.evaluation.client();
        let backend = CdpHandlePropertiesService::from_evaluation(
            resolved.evaluation,
            self.native_id_bytes,
            self.preview_bytes,
            self.property_limits,
        );
        let native = backend.call_on_handle(target_id, resolved.target).await?;
        if native.is_empty() {
            return Ok(HandlePropertiesResult::default());
        }
        let result = self
            .publish_property_batch(target_id, owner_frame, owner_page, &client, native)
            .await?;
        Ok(HandlePropertiesResult::new(result))
    }

    fn value_limits(
        &self,
        target_id: ObjectId,
    ) -> Result<
        pwright_cap_evaluation::EvaluationValueLimits,
        OperationError<HandleEvaluationErrorDetails>,
    > {
        let owner_frame = self.owner_frame(target_id)?;
        self.page_services
            .handle_evaluation_route(owner_frame)
            .map_err(|error| handle_route_state_error(&error.to_string()))?
            .map(|route| route.service.limits().value)
            .ok_or_else(|| missing_evaluation_route(owner_frame))
    }

    fn owner_frame(
        &self,
        target_id: ObjectId,
    ) -> Result<ObjectId, OperationError<HandleEvaluationErrorDetails>> {
        self.routes
            .state
            .lock()
            .map_err(|_| handle_route_state_error("handle route table is poisoned"))?
            .routes
            .get(&target_id)
            .map(|route| route.owner_frame)
            .ok_or_else(|| missing_handle(target_id))
    }

    fn resolve(
        &self,
        target_id: ObjectId,
        argument_ids: &[ObjectId],
    ) -> Result<ResolvedHandleOperation, OperationError<HandleEvaluationErrorDetails>> {
        let owner_frame = self.owner_frame(target_id)?;
        let evaluation = self
            .page_services
            .handle_evaluation_route(owner_frame)
            .map_err(|error| handle_route_state_error(&error.to_string()))?
            .ok_or_else(|| missing_evaluation_route(owner_frame))?;
        let client = evaluation.service.client();
        let state = self
            .routes
            .state
            .lock()
            .map_err(|_| handle_route_state_error("handle route table is poisoned"))?;
        let target = state
            .routes
            .get(&target_id)
            .ok_or_else(|| missing_handle(target_id))?;
        validate_target_route(
            target_id,
            target,
            owner_frame,
            evaluation.page_object_id,
            &client,
        )?;
        let arguments = resolve_handle_arguments(
            &state,
            target.owner_frame,
            target.owner_page,
            &client,
            argument_ids,
        )?;
        Ok(ResolvedHandleOperation {
            owner_frame: target.owner_frame,
            owner_page: target.owner_page,
            target: target.native.clone(),
            arguments,
            evaluation: evaluation.service.as_ref().clone(),
        })
    }

    async fn create_handle(
        &self,
        target_id: ObjectId,
        params: HandleEvaluateParams,
    ) -> Result<HandleInitializer, OperationError<HandleEvaluationErrorDetails>> {
        validate_live_handle(&self.runtime, target_id)?;
        let handles = params
            .referenced_handles(self.value_limits(target_id)?)
            .map_err(invalid_argument)?;
        let object_id = next_handle_id(&self.object_ids)?;
        let owner_frame = self.owner_frame(target_id)?;
        let evaluation = self
            .page_services
            .handle_evaluation_route(owner_frame)
            .map_err(|error| handle_route_state_error(&error.to_string()))?
            .ok_or_else(|| missing_evaluation_route(owner_frame))?;
        let client = evaluation.service.client();
        let (target, arguments, owner_page) = {
            let state = self
                .routes
                .state
                .lock()
                .map_err(|_| handle_route_state_error("handle route table is poisoned"))?;
            if state.routes.len() >= self.routes.max_handles {
                return Err(handle_capacity_error(self.routes.max_handles));
            }
            let target = state
                .routes
                .get(&target_id)
                .ok_or_else(|| missing_handle(target_id))?;
            validate_target_route(
                target_id,
                target,
                owner_frame,
                evaluation.page_object_id,
                &client,
            )?;
            let arguments = resolve_handle_arguments(
                &state,
                target.owner_frame,
                target.owner_page,
                &client,
                &handles,
            )?;
            (target.native.clone(), arguments, target.owner_page)
        };
        let mut route_resource = self
            .routes
            .reserve_route(self.native_id_bytes, self.preview_bytes)?;
        let backend = CdpHandleEvaluationService::from_evaluation(
            evaluation.service.as_ref().clone(),
            self.native_id_bytes,
            self.preview_bytes,
        );
        let native = backend
            .call(CdpHandleEvaluationRequest::new_on_handle(
                params.as_frame_params(),
                (target_id, target),
                arguments,
            ))
            .await?;
        let retained_bytes = retained_bytes(&native);
        if let Some(resource) = &mut route_resource
            && resource.units(ResourceClass::BackendStateBytes).is_some()
            && let Err(error) =
                resource.try_resize(ResourceClass::BackendStateBytes, retained_bytes)
        {
            return release_failed_handle(native, handle_route_resource_error(error)).await;
        }
        let prepared = match self.runtime.prepare_object_creation(
            object_id,
            Some(owner_frame),
            native.object_kind(),
        ) {
            Ok(prepared) => prepared,
            Err(error) => {
                return release_failed_handle(native, map_runtime_handle_evaluation_error(error))
                    .await;
            }
        };
        let initializer = HandleInitializer::new(
            object_id,
            owner_frame,
            native.object_kind(),
            native.preview().to_owned(),
        );
        match publish_handle_route(
            &self.routes,
            prepared,
            owner_frame,
            owner_page,
            native,
            retained_bytes,
            route_resource,
        ) {
            Ok(()) => Ok(initializer),
            Err(failure) => {
                let (native, primary) = *failure;
                release_failed_handle(native, primary).await
            }
        }
    }

    async fn publish_property_batch(
        &self,
        target_id: ObjectId,
        owner_frame: ObjectId,
        owner_page: ObjectId,
        client: &Arc<dyn CdpClient>,
        native: Vec<(String, CdpEvaluatedHandle)>,
    ) -> Result<Vec<HandlePropertyInitializer>, OperationError<HandleEvaluationErrorDetails>> {
        let mut ids = Vec::with_capacity(native.len());
        for _ in &native {
            match next_handle_id(&self.object_ids) {
                Ok(id) => ids.push(id),
                Err(primary) => return release_failed_handles(native, primary).await,
            }
        }
        let mut resources = Vec::with_capacity(native.len());
        for (_, handle) in &native {
            let mut resource = match self
                .routes
                .reserve_route(self.native_id_bytes, self.preview_bytes)
            {
                Ok(resource) => resource,
                Err(primary) => return release_failed_handles(native, primary).await,
            };
            let retained = retained_bytes(handle);
            if let Some(lease) = &mut resource
                && lease.units(ResourceClass::BackendStateBytes).is_some()
                && let Err(error) = lease.try_resize(ResourceClass::BackendStateBytes, retained)
            {
                return release_failed_handles(native, handle_route_resource_error(error)).await;
            }
            resources.push((resource, retained));
        }
        let prepared = match (|| {
            let state = self
                .routes
                .state
                .lock()
                .map_err(|_| handle_route_state_error("handle route table is poisoned"))?;
            let target = state
                .routes
                .get(&target_id)
                .ok_or_else(|| missing_handle(target_id))?;
            validate_target_route(target_id, target, owner_frame, owner_page, client)?;
            if state.routes.len().saturating_add(native.len()) > self.routes.max_handles {
                return Err(handle_capacity_error(self.routes.max_handles));
            }
            let specs = ids
                .iter()
                .zip(&native)
                .map(|(id, (_, handle))| {
                    ObjectCreationSpec::new(*id, Some(owner_frame), handle.object_kind())
                })
                .collect::<Vec<_>>();
            let prepared = self
                .runtime
                .prepare_object_creation_batch(&specs)
                .map_err(map_runtime_handle_evaluation_error)?;
            Ok(prepared)
        })() {
            Ok(prepared) => prepared,
            Err(primary) => return release_failed_handles(native, primary).await,
        };
        let initializers = ids
            .iter()
            .zip(&native)
            .map(|(id, (name, handle))| {
                HandlePropertyInitializer::new(
                    name.clone(),
                    HandleInitializer::new(
                        *id,
                        owner_frame,
                        handle.object_kind(),
                        handle.preview().to_owned(),
                    ),
                )
            })
            .collect::<Vec<_>>();
        match publish_handle_route_batch(
            &self.routes,
            prepared,
            owner_frame,
            owner_page,
            ids,
            native,
            resources,
        ) {
            Ok(()) => Ok(initializers),
            Err(failure) => {
                let (native, primary) = *failure;
                release_failed_handles(native, primary).await
            }
        }
    }
}

struct ResolvedHandleOperation {
    owner_frame: ObjectId,
    owner_page: ObjectId,
    target: CdpEvaluatedHandle,
    arguments: ResolvedChromiumHandleArguments,
    evaluation: CdpEvaluationService,
}

fn validate_target_route(
    target_id: ObjectId,
    target: &ChromiumHandleRouteEntry,
    expected_frame: ObjectId,
    expected_page: ObjectId,
    client: &Arc<dyn CdpClient>,
) -> Result<(), OperationError<HandleEvaluationErrorDetails>> {
    if target.owner_frame != expected_frame
        || target.owner_page != expected_page
        || !target.native.belongs_to_client(client)
    {
        return Err(OperationError::new(
            FailureKind::TargetClosed,
            format!("Handle `{target_id}` no longer belongs to the current execution world"),
            Some(HandleEvaluationErrorDetails::InvalidValue(
                "Handle target and current Frame route do not share one CDP session".to_owned(),
            )),
        ));
    }
    Ok(())
}

fn validate_live_handle(
    runtime: &ControlledLifecycleHandle,
    object_id: ObjectId,
) -> Result<(), OperationError<HandleEvaluationErrorDetails>> {
    match runtime.is_object_live(object_id) {
        Ok(true) => Ok(()),
        Ok(false) => Err(missing_handle(object_id)),
        Err(error) => Err(map_runtime_handle_evaluation_error(error)),
    }
}

fn validate_metadata(
    metadata: CallMetadata,
    operation: &str,
) -> Result<(), OperationError<HandleEvaluationErrorDetails>> {
    if metadata.timeout.is_none() {
        Ok(())
    } else {
        Err(OperationError::new(
            FailureKind::ProtocolViolation,
            format!("{operation} does not accept timeout metadata"),
            None,
        ))
    }
}

fn invalid_argument(
    error: pwright_cap_handle_evaluation::HandleEvaluationValueError,
) -> OperationError<HandleEvaluationErrorDetails> {
    OperationError::new(
        FailureKind::ProtocolViolation,
        format!("invalid Handle-local evaluation argument: {error}"),
        Some(HandleEvaluationErrorDetails::InvalidValue(
            error.to_string(),
        )),
    )
}

fn missing_handle(object_id: ObjectId) -> OperationError<HandleEvaluationErrorDetails> {
    OperationError::new(
        FailureKind::TargetClosed,
        format!("Handle `{object_id}` is disposed or unknown"),
        Some(HandleEvaluationErrorDetails::InvalidValue(
            "portable Handle has no live Chromium route".to_owned(),
        )),
    )
}

fn missing_evaluation_route(frame: ObjectId) -> OperationError<HandleEvaluationErrorDetails> {
    OperationError::new(
        FailureKind::TargetClosed,
        format!("Frame `{frame}` has no active Handle-evaluation route"),
        Some(HandleEvaluationErrorDetails::InvalidBackendResponse(
            "portable Frame is absent from Chromium execution-world routes".to_owned(),
        )),
    )
}

fn next_handle_id(
    source: &Arc<dyn ObjectIdSource>,
) -> Result<ObjectId, OperationError<HandleEvaluationErrorDetails>> {
    source.next_object_id().map_err(|error| {
        OperationError::new(
            FailureKind::ResourceExhausted,
            error.to_string(),
            Some(HandleEvaluationErrorDetails::InvalidBackendResponse(
                error.to_string(),
            )),
        )
    })
}

fn retained_bytes(handle: &CdpEvaluatedHandle) -> u64 {
    u64::try_from(
        handle
            .native_argument_bytes()
            .saturating_add(handle.preview().len()),
    )
    .unwrap_or(u64::MAX)
}

type NativeHandleBatch = Vec<(String, CdpEvaluatedHandle)>;
type HandleRouteBatchFailure = (
    NativeHandleBatch,
    OperationError<HandleEvaluationErrorDetails>,
);

fn publish_handle_route_batch(
    routes: &ChromiumHandleRoutes,
    prepared: PreparedObjectCreationBatch,
    owner_frame: ObjectId,
    owner_page: ObjectId,
    ids: Vec<ObjectId>,
    native: NativeHandleBatch,
    resources: Vec<(Option<ResourceLease>, u64)>,
) -> Result<(), Box<HandleRouteBatchFailure>> {
    let created = u64::try_from(native.len()).unwrap_or(u64::MAX);
    let mut state = match routes.state.lock() {
        Ok(state) => state,
        Err(_) => {
            return Err(Box::new((
                native,
                handle_route_state_error("handle route table is poisoned"),
            )));
        }
    };
    if state.routes.len().saturating_add(native.len()) > routes.max_handles {
        return Err(Box::new((
            native,
            handle_capacity_error(routes.max_handles),
        )));
    }
    if let Err(error) = prepared.commit() {
        return Err(Box::new((
            native,
            map_runtime_handle_evaluation_error(error),
        )));
    }
    let mut retained_total = 0_u64;
    for (((object_id, (_, handle)), (resource, retained)), index) in
        ids.into_iter().zip(native).zip(resources).zip(0_u64..)
    {
        retained_total = retained_total.saturating_add(retained);
        let replaced = state.routes.insert(
            object_id,
            ChromiumHandleRouteEntry {
                owner_frame,
                owner_page,
                native: handle,
                retained_bytes: retained,
                _resource: resource,
            },
        );
        debug_assert!(replaced.is_none(), "batch route {index} replaced an object");
    }
    state.metrics.current = state.routes.len();
    state.metrics.peak = state.metrics.peak.max(state.metrics.current);
    state.metrics.retained_bytes = state.metrics.retained_bytes.saturating_add(retained_total);
    state.metrics.peak_retained_bytes = state
        .metrics
        .peak_retained_bytes
        .max(state.metrics.retained_bytes);
    state.metrics.created = state.metrics.created.saturating_add(created);
    Ok(())
}

async fn release_failed_handles<T>(
    handles: Vec<(String, CdpEvaluatedHandle)>,
    primary: OperationError<HandleEvaluationErrorDetails>,
) -> Result<T, OperationError<HandleEvaluationErrorDetails>> {
    let mut cleanup = None;
    for (_, handle) in handles {
        if let Err(error) = handle.release().await
            && cleanup.is_none()
        {
            cleanup = Some(error.to_string());
        }
    }
    match cleanup {
        None => Err(primary),
        Some(cleanup) => Err(OperationError::new(
            FailureKind::Cleanup,
            format!("{}; cleanup failed: {cleanup}", primary.message()),
            Some(HandleEvaluationErrorDetails::Cleanup {
                primary: primary.message().to_owned(),
                cleanup,
            }),
        )),
    }
}

macro_rules! impl_command_service {
    ($spec:ty, $method:ident, $params:pat => $args:expr) => {
        impl Service<Command<$spec>> for ChromiumHandleOperationCommandService {
            type Response = CommandResult<$spec>;
            type Error = Infallible;
            type Future<'a> =
                Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

            fn call(&self, command: Command<$spec>) -> Self::Future<'_> {
                let (call_id, object_id, metadata, params) = command.into_parts();
                let $params = params;
                Box::pin(async move {
                    Ok(CommandResult::new(
                        call_id,
                        self.$method(object_id, metadata, $args).await,
                    ))
                })
            }
        }
    };
}

impl_command_service!(HandleEvaluate, evaluate_value, params => params);
impl_command_service!(HandleEvaluateHandle, evaluate_handle, params => params);
impl_command_service!(HandleGetProperty, get_property, params => params);

impl Service<Command<HandleJsonValue>> for ChromiumHandleOperationCommandService {
    type Response = CommandResult<HandleJsonValue>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<HandleJsonValue>) -> Self::Future<'_> {
        let (call_id, object_id, metadata, JsonValueOptions) = command.into_parts();
        Box::pin(async move {
            Ok(CommandResult::new(
                call_id,
                self.json_value(object_id, metadata).await,
            ))
        })
    }
}

impl Service<Command<HandleGetProperties>> for ChromiumHandleOperationCommandService {
    type Response = CommandResult<HandleGetProperties>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<HandleGetProperties>) -> Self::Future<'_> {
        let (call_id, object_id, metadata, GetPropertiesOptions) = command.into_parts();
        Box::pin(async move {
            Ok(CommandResult::new(
                call_id,
                self.get_properties(object_id, metadata).await,
            ))
        })
    }
}

impl<Spec, Inner, Dispose, Source, Codec, Transport> Service<Command<Spec>>
    for OrderedHandleOperationCommandService<Inner, Dispose, Source, Codec, Transport>
where
    Spec: HandleCreationCommand + Send + Sync + 'static,
    Spec::Params: Send,
    Spec::Result: Send,
    Inner: Service<Command<Spec>, Response = CommandResult<Spec>, Error = Infallible> + 'static,
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Source: ControlMessageSource + Send + Sync + 'static,
    Codec: WireCodec<ControlMessage> + 'static,
    Transport: FrameTransport + 'static,
{
    type Response = CommandResult<Spec>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<Spec>) -> Self::Future<'_> {
        let call_id = command.call_id();
        Box::pin(async move {
            let barrier = match self.control.begin_barrier().await {
                Ok(barrier) => barrier,
                Err(error) => {
                    return Ok(handle_operation_ordering_failure::<Spec>(
                        call_id,
                        map_control_gate_failure(error),
                        "ordered lifecycle capacity was unavailable".to_owned(),
                    ));
                }
            };
            let result = match self.service.call(command).await {
                Ok(result) => result,
                Err(never) => match never {},
            };
            let handles = match result.outcome() {
                Ok(value) => Spec::created_handles(value),
                Err(_) => return Ok(result),
            };
            for initializer in &handles {
                if barrier
                    .send_through_creation(
                        initializer.object_id,
                        Some(initializer.parent_id),
                        initializer.object_kind,
                    )
                    .await
                    .is_err()
                {
                    let cleanup =
                        compensate_created_handles(&barrier, &self.dispose, call_id, handles).await;
                    return Ok(handle_operation_ordering_failure::<Spec>(
                        call_id,
                        FailureKind::Cleanup,
                        format!("creation fence failed; {cleanup}"),
                    ));
                }
            }
            Ok(result)
        })
    }
}

async fn compensate_created_handles<Dispose, Source, Codec, Transport>(
    barrier: &OrderedRuntimeControlBarrier<'_, Source, Codec, Transport>,
    dispose: &Arc<Dispose>,
    call_id: CallId,
    handles: Vec<HandleInitializer>,
) -> String
where
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>,
    Source: ControlMessageSource,
    Codec: WireCodec<ControlMessage>,
    Transport: FrameTransport,
{
    let mut failures = Vec::new();
    for initializer in handles {
        let disposal = match dispose
            .call(Command::<HandleDispose>::new(
                call_id,
                initializer.object_id,
                CallMetadata::default(),
                (),
            ))
            .await
        {
            Ok(disposal) => disposal,
            Err(never) => match never {},
        };
        if let Err(error) = disposal.into_outcome() {
            failures.push(format!("{}: {error}", initializer.object_id));
            continue;
        }
        if barrier
            .send_through_disposal(initializer.object_id)
            .await
            .is_err()
        {
            failures.push(format!(
                "{} lifecycle publication failed",
                initializer.object_id
            ));
        }
    }
    if failures.is_empty() {
        "every created Handle was disposed".to_owned()
    } else {
        format!("Handle compensation failures: {}", failures.join("; "))
    }
}

fn handle_operation_ordering_failure<Spec>(
    call_id: CallId,
    kind: FailureKind,
    cleanup: String,
) -> CommandResult<Spec>
where
    Spec: HandleCreationCommand,
{
    CommandResult::new(
        call_id,
        Err(OperationError::new(
            kind,
            format!(
                "{} could not publish every initializer before completion",
                Spec::operation_name()
            ),
            Some(HandleEvaluationErrorDetails::Cleanup {
                primary: format!(
                    "{} created browser-side Handle state",
                    Spec::operation_name()
                ),
                cleanup,
            }),
        )),
    )
}

macro_rules! impl_ordered_value_service {
    ($spec:ty) => {
        impl<Inner, Dispose, Source, Codec, Transport> Service<Command<$spec>>
            for OrderedHandleOperationCommandService<Inner, Dispose, Source, Codec, Transport>
        where
            Inner: Service<Command<$spec>, Response = CommandResult<$spec>, Error = Infallible>
                + 'static,
            Dispose: Send + Sync + 'static,
            Source: ControlMessageSource + Send + Sync + 'static,
            Codec: WireCodec<ControlMessage> + Send + Sync + 'static,
            Transport: FrameTransport + Send + Sync + 'static,
        {
            type Response = CommandResult<$spec>;
            type Error = Infallible;
            type Future<'a> =
                Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

            fn call(&self, command: Command<$spec>) -> Self::Future<'_> {
                Box::pin(async move { self.service.call(command).await })
            }
        }
    };
}

impl_ordered_value_service!(HandleEvaluate);
impl_ordered_value_service!(HandleJsonValue);

impl<Spec, Inner, Dispose, Continuation> Service<Command<Spec>>
    for ContinuableHandleOperationCommandService<Inner, Dispose, Continuation>
where
    Spec: HandleCreationCommand + Send + Sync + 'static,
    Spec::Params: Send,
    Spec::Result: Send,
    Inner: Service<Command<Spec>, Response = CommandResult<Spec>, Error = Infallible> + 'static,
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Continuation: ReservedCleanupExecutor,
{
    type Response = CommandResult<Spec>;
    type Error = Infallible;
    type Future<'a>
        = ContinuableHandleOperationFuture<Spec, Dispose, Continuation::Reservation>
    where
        Self: 'a;

    fn call(&self, command: Command<Spec>) -> Self::Future<'_> {
        let call_id = command.call_id();
        let reservation = match self.continuation.try_reserve() {
            Ok(reservation) => reservation,
            Err(error) => {
                return ContinuableHandleOperationFuture {
                    inner: Some(Box::pin(std::future::ready(Ok(
                        handle_operation_continuation_failure::<Spec>(call_id, error),
                    )))),
                    dispose: None,
                    reservation: None,
                    call_id,
                    started: false,
                };
            }
        };
        let service = Arc::clone(&self.service);
        ContinuableHandleOperationFuture {
            inner: Some(Box::pin(async move { service.call(command).await })),
            dispose: Some(Arc::clone(&self.dispose)),
            reservation: Some(reservation),
            call_id,
            started: false,
        }
    }
}

macro_rules! impl_continuable_value_service {
    ($spec:ty) => {
        impl<Inner, Dispose, Continuation> Service<Command<$spec>>
            for ContinuableHandleOperationCommandService<Inner, Dispose, Continuation>
        where
            Inner: Service<Command<$spec>, Response = CommandResult<$spec>, Error = Infallible>
                + 'static,
            Dispose: Send + Sync + 'static,
            Continuation: ReservedCleanupExecutor,
        {
            type Response = CommandResult<$spec>;
            type Error = Infallible;
            type Future<'a> =
                Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

            fn call(&self, command: Command<$spec>) -> Self::Future<'_> {
                Box::pin(async move { self.service.call(command).await })
            }
        }
    };
}

impl_continuable_value_service!(HandleEvaluate);
impl_continuable_value_service!(HandleJsonValue);

fn handle_operation_continuation_failure<Spec>(
    call_id: CallId,
    error: CleanupSubmitError,
) -> CommandResult<Spec>
where
    Spec: HandleCreationCommand,
{
    let kind = match error {
        CleanupSubmitError::CapacityExceeded { .. } => FailureKind::ResourceExhausted,
        CleanupSubmitError::Unavailable | CleanupSubmitError::StatePoisoned => FailureKind::Cleanup,
    };
    CommandResult::new(
        call_id,
        Err(OperationError::new(
            kind,
            format!(
                "{} continuation capacity is unavailable",
                Spec::operation_name()
            ),
            Some(HandleEvaluationErrorDetails::Cleanup {
                primary: "Handle operation was rejected before browser interaction".to_owned(),
                cleanup: error.to_string(),
            }),
        )),
    )
}

async fn dispose_abandoned_handles<Dispose>(
    dispose: Arc<Dispose>,
    call_id: CallId,
    handles: Vec<HandleInitializer>,
) -> Result<(), CleanupFailure>
where
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
{
    let mut first_error = None;
    for initializer in handles {
        let result = match dispose
            .call(Command::<HandleDispose>::new(
                call_id,
                initializer.object_id,
                CallMetadata::default(),
                (),
            ))
            .await
        {
            Ok(result) => result,
            Err(never) => match never {},
        };
        if let Err(error) = result.into_outcome()
            && first_error.is_none()
        {
            first_error = Some(format!(
                "abandoned Handle `{}` cleanup failed: {error}",
                initializer.object_id
            ));
        }
    }
    first_error.map_or(Ok(()), |error| Err(CleanupFailure::new(error)))
}

macro_rules! impl_unavailable_command_service {
    ($spec:ty) => {
        impl Service<Command<$spec>> for UnavailableHandleOperationCommandService {
            type Response = CommandResult<$spec>;
            type Error = Infallible;
            type Future<'a> = std::future::Ready<Result<Self::Response, Self::Error>>;

            fn call(&self, command: Command<$spec>) -> Self::Future<'_> {
                std::future::ready(Ok(CommandResult::new(
                    command.call_id(),
                    Err(OperationError::new(
                        FailureKind::ProtocolViolation,
                        "Handle-local operations require pwright.handle_evaluation 1.2",
                        None,
                    )),
                )))
            }
        }
    };
}

impl_unavailable_command_service!(HandleEvaluate);
impl_unavailable_command_service!(HandleEvaluateHandle);
impl_unavailable_command_service!(HandleGetProperties);
impl_unavailable_command_service!(HandleGetProperty);
impl_unavailable_command_service!(HandleJsonValue);
