//! Per-command Frame selector / Page keyboard binding with exact cleanup ownership.
use crate::{CommandRuntime, HostError, HostSession};
use pwright_backend_cdp_locator::CdpLocatorService;
use pwright_cap_locator::LocatorErrorDetails;
use pwright_core::{
    Command, CommandCancellation, CommandResult, CommandSpec, OperationError, Service,
};
use std::convert::Infallible;

impl HostSession {
    pub(crate) async fn invoke_locator_keyboard<C>(
        &self,
        id: &str,
        params: C::Params,
        timeout_ms: u32,
        map_error: fn(OperationError<LocatorErrorDetails>, u32) -> HostError,
    ) -> Result<(), HostError>
    where
        C: CommandSpec<Result = (), ErrorDetails = LocatorErrorDetails>,
        C::Params: Send + 'static,
        CommandRuntime: Service<Command<C>, Response = CommandResult<C>, Error = Infallible>,
        CdpLocatorService: Service<C::Params, Response = (), Error = OperationError<LocatorErrorDetails>>
            + CommandCancellation<C>,
    {
        #[cfg(not(feature = "drag-runtime"))]
        {
            self.invoke_with_error::<C>(id, params, timeout_ms, map_error)
                .await
        }
        #[cfg(feature = "drag-runtime")]
        {
            selected::invoke::<C>(self, id, params, timeout_ms, map_error).await
        }
    }
}

#[cfg(feature = "drag-runtime")]
pub(crate) use selected::{EXECUTING, requirements};

#[cfg(feature = "drag-runtime")]
mod selected {
    use super::*;
    use crate::drag::host::{Installation, PageController};
    use pwright_composition_chromium::{CdpPageServices, ChromiumLocatorKeyboardRoute};
    use pwright_core::{
        CallMetadata, CancellationReason, FailureKind, ObjectId, OperationContext, ResourceClass,
        ResourceLease, ResourceRequirements, ResourceScope,
    };
    use std::{
        future::Future,
        pin::Pin,
        sync::{Arc, OnceLock, atomic::Ordering},
        time::Duration,
    };

    pub(crate) const EXECUTING: usize = 8;
    const RETAINED: usize = 16;
    // Retained selector/text params, service/snapshot/cancellation future storage.
    // Native provider futures and Frame route leases remain separate owners.
    const CALL_BYTES: u64 = 1024 * 1024 + 64 * 1024 + 16 * 1024;
    const SCOPE: ResourceScope = ResourceScope::extension(122, 10);
    pub(crate) fn requirements() -> ResourceRequirements {
        ResourceRequirements::none()
            .with_class(
                ResourceClass::BackendStateBytes,
                CALL_BYTES * RETAINED as u64,
                CALL_BYTES * RETAINED as u64,
            )
            .with_metadata(1, RETAINED)
    }
    struct Lease {
        _storage: ResourceLease,
        manager: Arc<Installation>,
    }
    impl Drop for Lease {
        fn drop(&mut self) {
            self.manager.locator_calls.fetch_sub(1, Ordering::AcqRel);
        }
    }
    struct Binding {
        route: ChromiumLocatorKeyboardRoute,
        controller: Arc<PageController>,
    }
    struct Invocation {
        runtime: std::sync::Weak<CommandRuntime>,
        call: pwright_core::CallId,
        frame: ObjectId,
        services: CdpPageServices,
        binding: OnceLock<Binding>,
        lease: Lease,
    }
    type ResultFuture<'a> =
        Pin<Box<dyn Future<Output = Result<(), OperationError<LocatorErrorDetails>>> + Send + 'a>>;
    struct Handoff(pwright_runtime::CallLifetimeHandoff);
    impl pwright_backend_cdp_locator::LocatorInputHandoff for Handoff {
        fn handoff(&self) -> Result<(), OperationError<LocatorErrorDetails>> {
            self.0.commit().map_err(|error| error.operation_error())
        }
    }
    fn backend(error: impl std::fmt::Display) -> OperationError<LocatorErrorDetails> {
        OperationError::new(
            FailureKind::Backend,
            HostError::backend(error).to_string(),
            None,
        )
    }
    impl<P: Send + 'static> Service<P> for Invocation
    where
        CdpLocatorService: Service<P, Response = (), Error = OperationError<LocatorErrorDetails>>,
    {
        type Response = ();
        type Error = OperationError<LocatorErrorDetails>;
        type Future<'a> = ResultFuture<'a>;
        fn call(&self, params: P) -> Self::Future<'_> {
            Box::pin(async move {
                if self.binding.get().is_some() {
                    return Err(backend(
                        "Locator keyboard invocation identity/reuse mismatch",
                    ));
                }
                let route = self
                    .services
                    .locator_keyboard_route(self.frame)
                    .await
                    .map_err(backend)?
                    .ok_or_else(|| {
                        OperationError::new(
                            FailureKind::TargetClosed,
                            "Locator keyboard route unavailable",
                            None,
                        )
                    })?;
                let controller = self
                    .lease
                    .manager
                    .bind(route.page_id(), route.keyboard_client())
                    .map_err(backend)?;
                let handoff = if self.frame == route.page_id() {
                    None
                } else {
                    Some(
                        self.runtime
                            .upgrade()
                            .ok_or_else(|| backend("Locator runtime is gone"))?
                            .prepare_lifetime_handoff(self.call, self.frame, route.page_id())
                            .map_err(|error| error.operation_error())?,
                    )
                };
                let route = route
                    .select(controller.dispatcher.clone())
                    .map_err(backend)?;
                let route = match handoff {
                    Some(handoff) => route
                        .with_handoff(Arc::new(Handoff(handoff)))
                        .map_err(backend)?,
                    None => route,
                };
                self.binding
                    .set(Binding { route, controller })
                    .map_err(|_| backend("Locator keyboard binding already selected"))?;
                self.binding
                    .get()
                    .expect("published exact binding")
                    .route
                    .service()
                    .call(params)
                    .await
            })
        }
    }
    impl<C> CommandCancellation<C> for Invocation
    where
        C: CommandSpec<ErrorDetails = LocatorErrorDetails>,
        CdpLocatorService: CommandCancellation<C>,
    {
        type Future<'a> = ResultFuture<'a>;
        fn cancel(
            &self,
            context: OperationContext,
            reason: CancellationReason,
        ) -> Self::Future<'_> {
            Box::pin(async move {
                if context.object_id() != self.frame {
                    return Err(backend("Locator keyboard cleanup identity mismatch"));
                }
                let Some(binding) = self.binding.get() else {
                    return Ok(());
                };
                let cleaned = <CdpLocatorService as CommandCancellation<C>>::cancel(
                    binding.route.service(),
                    context,
                    reason,
                )
                .await;
                let retired = binding
                    .controller
                    .drain()
                    .await
                    .map_err(|error| backend(error).reclassify(FailureKind::Cleanup));
                cleaned.and(retired)
            })
        }
    }
    pub(super) async fn invoke<C>(
        host: &HostSession,
        id: &str,
        params: C::Params,
        timeout_ms: u32,
        map_error: fn(OperationError<LocatorErrorDetails>, u32) -> HostError,
    ) -> Result<(), HostError>
    where
        C: CommandSpec<Result = (), ErrorDetails = LocatorErrorDetails>,
        C::Params: Send + 'static,
        CdpLocatorService: Service<C::Params, Response = (), Error = OperationError<LocatorErrorDetails>>
            + CommandCancellation<C>,
    {
        let (frame, call, _operation) = host.admit_operation(id)?;
        let (runtime, manager, resources) = {
            let state = host.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            (
                live.runtime.clone(),
                live.mouse.drag_installation().cloned().ok_or_else(|| {
                    HostError::Backend("Locator drag installation missing".into())
                })?,
                live.composition.resources().clone(),
            )
        };
        manager
            .locator_calls
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < RETAINED).then_some(count + 1)
            })
            .map_err(|_| {
                HostError::ResourceExhausted(
                    "Locator keyboard retained invocation capacity exceeded".into(),
                )
            })?;
        let storage = match resources.claim(requirements()).and_then(|ledger| {
            ledger
                .reserve(SCOPE, ResourceClass::BackendStateBytes, CALL_BYTES)
                .map_err(Into::into)
        }) {
            Ok(storage) => storage,
            Err(error) => {
                manager.locator_calls.fetch_sub(1, Ordering::AcqRel);
                return Err(HostError::backend(error));
            }
        };
        let invocation = Arc::new(Invocation {
            runtime: Arc::downgrade(&runtime),
            call,
            frame,
            services: runtime.service().clone(),
            binding: OnceLock::new(),
            lease: Lease {
                _storage: storage,
                manager,
            },
        });
        let timeout = (timeout_ms != 0).then(|| Duration::from_millis(u64::from(timeout_ms)));
        let command = Command::<C>::new(call, frame, CallMetadata::new(timeout, None), params);
        let outcome = tokio::select! {
            biased;
            () = host.stop.cancelled() => return Err(HostError::TargetClosed),
            result = runtime.dispatch_with(command, invocation.as_ref(), invocation.clone()) => result,
        };
        let result = match outcome {
            Ok(result) => result,
            Err(never) => match never {},
        };
        result
            .into_outcome()
            .map_err(|error| map_error(error, timeout_ms))
    }
}
