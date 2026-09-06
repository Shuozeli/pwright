//! Optional remote delivery policy for retained Pwright Handles.
//!
//! This plugin composes three ownership fences without selecting a browser
//! backend, transport implementation, async runtime, or cleanup executor:
//!
//! 1. reserve bounded continuation capacity before invoking a Handle-producing
//!    service;
//! 2. publish every resulting `Create` on the ordered control stream before
//!    allowing the typed `Result` send to begin;
//! 3. retain that reservation until the `Result` send physically completes,
//!    and perform ordered disposal if execution or delivery is abandoned.

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use pwright_cap_handle_evaluation::{
    EvaluateHandle, HandleEvaluate, HandleEvaluateHandle, HandleEvaluationErrorDetails,
    HandleGetProperties, HandleGetProperty, HandleInitializer, HandleJsonValue,
};
use pwright_cap_handle_lifecycle::{HandleDispose, HandleLifecycleErrorDetails};
use pwright_codec_handle_evaluation::HandleEvaluationCodec;
use pwright_core::{
    CallId, CallMetadata, Command, CommandResult, CommandSpec, ControlMessage, FailureKind,
    ObjectId, OperationError, Service,
};
use pwright_runtime::{
    CleanupFailure, CleanupReservation, CleanupSubmitError, ReservedCleanupExecutor,
};
pub use pwright_runtime_control::{
    ControlMessageSource, OrderedControlGateError, OrderedRuntimeControl,
    OrderedRuntimeControlBarrier, RuntimeControlDriver,
};
use pwright_server_handle_evaluation::{
    HandleEvaluationServerHandlers, HandleEvaluationServerServices,
    handle_evaluation_server_handlers,
};
use pwright_server_wire::{ConcurrentServerFrameHandler, ServerFrameDisposition};
use pwright_transport::{FrameTransport, WireCodec, WireFrame, WireProtocol};

/// Fixed command family whose successful result owns one or more new Handles.
pub trait HandleCreationCommand: CommandSpec<ErrorDetails = HandleEvaluationErrorDetails> {
    fn created_handles(result: &Self::Result) -> Vec<HandleInitializer>;
    fn operation_name() -> &'static str;
}

impl HandleCreationCommand for EvaluateHandle {
    fn created_handles(result: &Self::Result) -> Vec<HandleInitializer> {
        vec![result.clone()]
    }

    fn operation_name() -> &'static str {
        "Frame.evaluateHandle"
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

/// Sends every successful Handle creation through an ordered lifecycle fence.
pub struct OrderedHandleCreationService<Inner, Dispose, Source, Codec, Transport> {
    service: Arc<Inner>,
    dispose: Arc<Dispose>,
    control: OrderedRuntimeControl<Source, Codec, Transport>,
}

impl<Inner, Dispose, Source, Codec, Transport>
    OrderedHandleCreationService<Inner, Dispose, Source, Codec, Transport>
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

    pub const fn service(&self) -> &Arc<Inner> {
        &self.service
    }

    pub const fn control(&self) -> &OrderedRuntimeControl<Source, Codec, Transport> {
        &self.control
    }
}

impl<Spec, Inner, Dispose, Source, Codec, Transport> Service<Command<Spec>>
    for OrderedHandleCreationService<Inner, Dispose, Source, Codec, Transport>
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
                    return Ok(creation_ordering_failure::<Spec>(
                        call_id,
                        map_gate_failure(error),
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
                    return Ok(creation_ordering_failure::<Spec>(
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

macro_rules! delegate_value_command {
    ($spec:ty) => {
        impl<Inner, Dispose, Source, Codec, Transport> Service<Command<$spec>>
            for OrderedHandleCreationService<Inner, Dispose, Source, Codec, Transport>
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

delegate_value_command!(HandleEvaluate);
delegate_value_command!(HandleJsonValue);

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
        if let Err(error) = result.into_outcome() {
            failures.push(format!(
                "{} native disposal failed: {error}",
                initializer.object_id
            ));
            continue;
        }
        if barrier
            .send_through_disposal(initializer.object_id)
            .await
            .is_err()
        {
            failures.push(format!(
                "{} lifecycle disposal fence failed",
                initializer.object_id
            ));
        }
    }
    if failures.is_empty() {
        "every created Handle was disposed and published".to_owned()
    } else {
        format!("Handle compensation failures: {}", failures.join("; "))
    }
}

fn creation_ordering_failure<Spec>(
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

/// Holds successful disposal behind the matching ordered `Dispose` message.
pub struct OrderedHandleDisposeService<Inner, Source, Codec, Transport> {
    service: Arc<Inner>,
    control: OrderedRuntimeControl<Source, Codec, Transport>,
}

impl<Inner, Source, Codec, Transport> OrderedHandleDisposeService<Inner, Source, Codec, Transport> {
    pub const fn new(
        service: Arc<Inner>,
        control: OrderedRuntimeControl<Source, Codec, Transport>,
    ) -> Self {
        Self { service, control }
    }

    pub const fn service(&self) -> &Arc<Inner> {
        &self.service
    }
}

impl<Inner, Source, Codec, Transport> Service<Command<HandleDispose>>
    for OrderedHandleDisposeService<Inner, Source, Codec, Transport>
where
    Inner: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Source: ControlMessageSource + Send + Sync + 'static,
    Codec: WireCodec<ControlMessage> + 'static,
    Transport: FrameTransport + 'static,
{
    type Response = CommandResult<HandleDispose>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<HandleDispose>) -> Self::Future<'_> {
        let call_id = command.call_id();
        let object_id = command.object_id();
        Box::pin(async move {
            let barrier = match self.control.begin_barrier().await {
                Ok(barrier) => barrier,
                Err(error) => {
                    return Ok(disposal_ordering_failure(
                        call_id,
                        map_gate_failure(error),
                        "Handle disposal was rejected before native work",
                    ));
                }
            };
            let result = match self.service.call(command).await {
                Ok(result) => result,
                Err(never) => match never {},
            };
            if result.outcome().is_err() {
                return Ok(result);
            }
            match barrier.send_through_disposal(object_id).await {
                Ok(_) => Ok(result),
                Err(_) => Ok(disposal_ordering_failure(
                    call_id,
                    FailureKind::Cleanup,
                    "native Handle disposal succeeded",
                )),
            }
        })
    }
}

fn disposal_ordering_failure(
    call_id: CallId,
    kind: FailureKind,
    primary: &'static str,
) -> CommandResult<HandleDispose> {
    CommandResult::new(
        call_id,
        Err(OperationError::new(
            kind,
            "Handle.dispose could not publish its lifecycle before completion",
            Some(HandleLifecycleErrorDetails::Cleanup {
                primary: primary.to_owned(),
                cleanup: "ordered remote lifecycle completion failed".to_owned(),
            }),
        )),
    )
}

fn map_gate_failure(error: OrderedControlGateError) -> FailureKind {
    match error {
        OrderedControlGateError::CapacityExceeded { .. } => FailureKind::ResourceExhausted,
        OrderedControlGateError::ZeroWaiterLimit
        | OrderedControlGateError::TicketExhausted
        | OrderedControlGateError::StateUnavailable => FailureKind::Cleanup,
    }
}

struct HandleDeliveryTicket<Dispose, Reservation> {
    dispose: Arc<Dispose>,
    reservation: Reservation,
    call_id: CallId,
    object_ids: Vec<ObjectId>,
}

type HandleDeliveryTracker<Dispose, Reservation> =
    Arc<Mutex<BTreeMap<CallId, HandleDeliveryTicket<Dispose, Reservation>>>>;
type BoxCreationFuture<Spec> =
    Pin<Box<dyn Future<Output = Result<CommandResult<Spec>, Infallible>> + Send + 'static>>;

/// Reserves one cleanup slot before starting a Handle-producing command.
pub struct ContinuableHandleCreationFuture<Spec, Dispose, Reservation>
where
    Spec: HandleCreationCommand,
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Reservation: CleanupReservation,
{
    inner: Option<BoxCreationFuture<Spec>>,
    dispose: Option<Arc<Dispose>>,
    reservation: Option<Reservation>,
    delivery: Option<HandleDeliveryTracker<Dispose, Reservation>>,
    call_id: CallId,
    started: bool,
}

impl<Spec, Dispose, Reservation> Unpin
    for ContinuableHandleCreationFuture<Spec, Dispose, Reservation>
where
    Spec: HandleCreationCommand,
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Reservation: CleanupReservation,
{
}

impl<Spec, Dispose, Reservation> Future
    for ContinuableHandleCreationFuture<Spec, Dispose, Reservation>
where
    Spec: HandleCreationCommand,
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Reservation: CleanupReservation,
{
    type Output = Result<CommandResult<Spec>, Infallible>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.started = true;
        let outcome = self
            .inner
            .as_mut()
            .expect("completed Handle creation future must not be polled again")
            .as_mut()
            .poll(context);
        if let Poll::Ready(Ok(result)) = &outcome
            && let Ok(value) = result.outcome()
        {
            let object_ids = Spec::created_handles(value)
                .into_iter()
                .map(|initializer| initializer.object_id)
                .collect::<Vec<_>>();
            if !object_ids.is_empty()
                && let Some(delivery) = self.delivery.take()
            {
                let ticket = HandleDeliveryTicket {
                    dispose: self
                        .dispose
                        .take()
                        .expect("tracked Handle creation owns its disposer"),
                    reservation: self
                        .reservation
                        .take()
                        .expect("tracked Handle creation owns continuation capacity"),
                    call_id: self.call_id,
                    object_ids,
                };
                let replaced = delivery
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .insert(self.call_id, ticket);
                if let Some(replaced) = replaced {
                    submit_delivery_cleanup(replaced);
                }
            }
        }
        if outcome.is_ready() {
            self.inner = None;
            self.dispose = None;
            self.reservation = None;
        }
        outcome
    }
}

impl<Spec, Dispose, Reservation> Drop
    for ContinuableHandleCreationFuture<Spec, Dispose, Reservation>
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
            let object_ids = match inner.await {
                Ok(result) => match result.into_outcome() {
                    Ok(value) => Spec::created_handles(&value)
                        .into_iter()
                        .map(|initializer| initializer.object_id)
                        .collect(),
                    Err(_) => return Ok(()),
                },
                Err(never) => match never {},
            };
            dispose_handles(dispose, call_id, object_ids).await
        }));
    }
}

/// Adds bounded abandonment cleanup to a Handle-producing service.
pub struct ContinuableHandleCreationService<Inner, Dispose, Continuation>
where
    Continuation: ReservedCleanupExecutor,
{
    service: Arc<Inner>,
    dispose: Arc<Dispose>,
    continuation: Continuation,
    delivery: Option<HandleDeliveryTracker<Dispose, Continuation::Reservation>>,
}

impl<Inner, Dispose, Continuation> ContinuableHandleCreationService<Inner, Dispose, Continuation>
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
            delivery: None,
        }
    }

    fn with_delivery_tracking(
        mut self,
        delivery: HandleDeliveryTracker<Dispose, Continuation::Reservation>,
    ) -> Self {
        self.delivery = Some(delivery);
        self
    }
}

impl<Spec, Inner, Dispose, Continuation> Service<Command<Spec>>
    for ContinuableHandleCreationService<Inner, Dispose, Continuation>
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
        = ContinuableHandleCreationFuture<Spec, Dispose, Continuation::Reservation>
    where
        Self: 'a;

    fn call(&self, command: Command<Spec>) -> Self::Future<'_> {
        let call_id = command.call_id();
        let reservation = match self.continuation.try_reserve() {
            Ok(reservation) => reservation,
            Err(error) => {
                return ContinuableHandleCreationFuture {
                    inner: Some(Box::pin(std::future::ready(Ok(
                        continuation_failure::<Spec>(call_id, error),
                    )))),
                    dispose: None,
                    reservation: None,
                    delivery: None,
                    call_id,
                    started: false,
                };
            }
        };
        let service = Arc::clone(&self.service);
        ContinuableHandleCreationFuture {
            inner: Some(Box::pin(async move { service.call(command).await })),
            dispose: Some(Arc::clone(&self.dispose)),
            reservation: Some(reservation),
            delivery: self.delivery.clone(),
            call_id,
            started: false,
        }
    }
}

macro_rules! delegate_continuable_value_command {
    ($spec:ty) => {
        impl<Inner, Dispose, Continuation> Service<Command<$spec>>
            for ContinuableHandleCreationService<Inner, Dispose, Continuation>
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

delegate_continuable_value_command!(HandleEvaluate);
delegate_continuable_value_command!(HandleJsonValue);

fn continuation_failure<Spec>(call_id: CallId, error: CleanupSubmitError) -> CommandResult<Spec>
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
                primary: format!("{} was rejected before browser I/O", Spec::operation_name()),
                cleanup: error.to_string(),
            }),
        )),
    )
}

async fn dispose_handles<Dispose>(
    dispose: Arc<Dispose>,
    call_id: CallId,
    object_ids: Vec<ObjectId>,
) -> Result<(), CleanupFailure>
where
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>,
{
    let mut first_error = None;
    for object_id in object_ids {
        let result = match dispose
            .call(Command::<HandleDispose>::new(
                call_id,
                object_id,
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
                "undelivered Handle `{object_id}` cleanup failed: {error}"
            ));
        }
    }
    first_error.map_or(Ok(()), |error| Err(CleanupFailure::new(error)))
}

fn submit_delivery_cleanup<Dispose, Reservation>(ticket: HandleDeliveryTicket<Dispose, Reservation>)
where
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Reservation: CleanupReservation,
{
    ticket.reservation.submit(Box::pin(async move {
        dispose_handles(ticket.dispose, ticket.call_id, ticket.object_ids).await
    }));
}

type BoxDeliveryFuture<Event, HandlerError> = Pin<
    Box<dyn Future<Output = Result<ServerFrameDisposition<Event>, HandlerError>> + Send + 'static>,
>;

/// Owns the interval between typed service completion and physical Result send.
pub struct HandleResultDeliveryFuture<Event, HandlerError, Dispose, Reservation>
where
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Reservation: CleanupReservation,
{
    inner: Option<BoxDeliveryFuture<Event, HandlerError>>,
    delivery: HandleDeliveryTracker<Dispose, Reservation>,
    call_id: Option<CallId>,
    started: bool,
}

impl<Event, HandlerError, Dispose, Reservation> Unpin
    for HandleResultDeliveryFuture<Event, HandlerError, Dispose, Reservation>
where
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Reservation: CleanupReservation,
{
}

impl<Event, HandlerError, Dispose, Reservation> Future
    for HandleResultDeliveryFuture<Event, HandlerError, Dispose, Reservation>
where
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Reservation: CleanupReservation,
{
    type Output = Result<ServerFrameDisposition<Event>, HandlerError>;

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        self.started = true;
        let outcome = self
            .inner
            .as_mut()
            .expect("completed Handle delivery future must not be polled again")
            .as_mut()
            .poll(context);
        let Poll::Ready(outcome) = outcome else {
            return Poll::Pending;
        };
        self.inner = None;
        if let Some(call_id) = self.call_id
            && let Some(ticket) = take_delivery_ticket(&self.delivery, call_id)
            && !matches!(&outcome, Ok(ServerFrameDisposition::Handled(_)))
        {
            submit_delivery_cleanup(ticket);
        }
        Poll::Ready(outcome)
    }
}

impl<Event, HandlerError, Dispose, Reservation> Drop
    for HandleResultDeliveryFuture<Event, HandlerError, Dispose, Reservation>
where
    Dispose: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + 'static,
    Reservation: CleanupReservation,
{
    fn drop(&mut self) {
        if !self.started {
            return;
        }
        drop(self.inner.take());
        let Some(call_id) = self.call_id else {
            return;
        };
        if let Some(ticket) = take_delivery_ticket(&self.delivery, call_id) {
            submit_delivery_cleanup(ticket);
        }
    }
}

fn take_delivery_ticket<Dispose, Reservation>(
    delivery: &HandleDeliveryTracker<Dispose, Reservation>,
    call_id: CallId,
) -> Option<HandleDeliveryTicket<Dispose, Reservation>> {
    delivery
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&call_id)
}

pub type OrderedCreation<Inner, Dispose, Source, Codec, Transport> =
    OrderedHandleCreationService<Inner, Dispose, Source, Codec, Transport>;
pub type OrderedDisposal<Dispose, Source, Codec, Transport> =
    OrderedHandleDisposeService<Dispose, Source, Codec, Transport>;
pub type ContinuableOrderedCreation<Inner, Dispose, Source, Codec, Transport, Continuation> =
    ContinuableHandleCreationService<
        OrderedCreation<Inner, Dispose, Source, Codec, Transport>,
        OrderedDisposal<Dispose, Source, Codec, Transport>,
        Continuation,
    >;
pub type OrderedHandleEvaluationHandlers<
    Create,
    Operations,
    Dispose,
    Source,
    Codec,
    Transport,
    Continuation,
> = HandleEvaluationServerHandlers<
    ContinuableOrderedCreation<Create, Dispose, Source, Codec, Transport, Continuation>,
    ContinuableOrderedCreation<Operations, Dispose, Source, Codec, Transport, Continuation>,
    ContinuableOrderedCreation<Operations, Dispose, Source, Codec, Transport, Continuation>,
    ContinuableOrderedCreation<Operations, Dispose, Source, Codec, Transport, Continuation>,
    ContinuableOrderedCreation<Operations, Dispose, Source, Codec, Transport, Continuation>,
    ContinuableOrderedCreation<Operations, Dispose, Source, Codec, Transport, Continuation>,
>;

/// Fully composed remote handler for the six Handle-evaluation commands.
pub struct OrderedHandleEvaluationHandler<
    Create,
    Operations,
    Dispose,
    Source,
    Codec,
    Transport,
    Continuation,
> where
    Continuation: ReservedCleanupExecutor + Clone,
{
    inner: OrderedHandleEvaluationHandlers<
        Create,
        Operations,
        Dispose,
        Source,
        Codec,
        Transport,
        Continuation,
    >,
    delivery: HandleDeliveryTracker<
        OrderedDisposal<Dispose, Source, Codec, Transport>,
        Continuation::Reservation,
    >,
}

impl<Create, Operations, Dispose, Source, Codec, Transport, Continuation>
    OrderedHandleEvaluationHandler<
        Create,
        Operations,
        Dispose,
        Source,
        Codec,
        Transport,
        Continuation,
    >
where
    Continuation: ReservedCleanupExecutor + Clone,
{
    pub fn new(
        protocol: Arc<WireProtocol<HandleEvaluationCodec>>,
        create: Arc<Create>,
        operations: Arc<Operations>,
        dispose: Arc<Dispose>,
        continuation: Continuation,
        control: OrderedRuntimeControl<Source, Codec, Transport>,
    ) -> Self {
        let ordered_dispose = Arc::new(OrderedHandleDisposeService::new(
            Arc::clone(&dispose),
            control.clone(),
        ));
        let delivery = Arc::new(Mutex::new(BTreeMap::new()));
        let create = ContinuableHandleCreationService::new(
            Arc::new(OrderedHandleCreationService::new(
                create,
                Arc::clone(&dispose),
                control.clone(),
            )),
            Arc::clone(&ordered_dispose),
            continuation.clone(),
        )
        .with_delivery_tracking(Arc::clone(&delivery));
        let operations = ContinuableHandleCreationService::new(
            Arc::new(OrderedHandleCreationService::new(
                operations, dispose, control,
            )),
            ordered_dispose,
            continuation,
        )
        .with_delivery_tracking(Arc::clone(&delivery));
        let create = Arc::new(create);
        let operations = Arc::new(operations);
        Self {
            inner: handle_evaluation_server_handlers(
                protocol,
                HandleEvaluationServerServices {
                    evaluate_handle: create,
                    handle_evaluate: Arc::clone(&operations),
                    handle_evaluate_handle: Arc::clone(&operations),
                    handle_get_properties: Arc::clone(&operations),
                    handle_get_property: Arc::clone(&operations),
                    handle_json_value: operations,
                },
            ),
            delivery,
        }
    }
}

impl<Create, Operations, Dispose, Source, Codec, Transport, Continuation, WireTransport>
    ConcurrentServerFrameHandler<WireTransport>
    for OrderedHandleEvaluationHandler<
        Create,
        Operations,
        Dispose,
        Source,
        Codec,
        Transport,
        Continuation,
    >
where
    Continuation: ReservedCleanupExecutor + Clone,
    OrderedDisposal<Dispose, Source, Codec, Transport>: Service<Command<HandleDispose>, Response = CommandResult<HandleDispose>, Error = Infallible>
        + Send
        + Sync
        + 'static,
    OrderedHandleEvaluationHandlers<
        Create,
        Operations,
        Dispose,
        Source,
        Codec,
        Transport,
        Continuation,
    >: ConcurrentServerFrameHandler<WireTransport>,
    <OrderedHandleEvaluationHandlers<
        Create,
        Operations,
        Dispose,
        Source,
        Codec,
        Transport,
        Continuation,
    > as ConcurrentServerFrameHandler<WireTransport>>::Event: Send + 'static,
    <OrderedHandleEvaluationHandlers<
        Create,
        Operations,
        Dispose,
        Source,
        Codec,
        Transport,
        Continuation,
    > as ConcurrentServerFrameHandler<WireTransport>>::Error: Send + 'static,
{
    type Event = <OrderedHandleEvaluationHandlers<
        Create,
        Operations,
        Dispose,
        Source,
        Codec,
        Transport,
        Continuation,
    > as ConcurrentServerFrameHandler<WireTransport>>::Event;
    type Error = <OrderedHandleEvaluationHandlers<
        Create,
        Operations,
        Dispose,
        Source,
        Codec,
        Transport,
        Continuation,
    > as ConcurrentServerFrameHandler<WireTransport>>::Error;
    type Future = HandleResultDeliveryFuture<
        Self::Event,
        Self::Error,
        OrderedDisposal<Dispose, Source, Codec, Transport>,
        Continuation::Reservation,
    >;

    fn handle_concurrently(&self, frame: WireFrame, transport: Arc<WireTransport>) -> Self::Future {
        let call_id = match frame.route() {
            pwright_transport::FrameRoute::Command {
                call_id, method, ..
            } if method == EvaluateHandle::METHOD
                || method == HandleEvaluateHandle::METHOD
                || method == HandleGetProperty::METHOD
                || method == HandleGetProperties::METHOD =>
            {
                Some(call_id)
            }
            _ => None,
        };
        HandleResultDeliveryFuture {
            inner: Some(Box::pin(self.inner.handle_concurrently(frame, transport))),
            delivery: Arc::clone(&self.delivery),
            call_id,
            started: false,
        }
    }
}
