use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use pwright_backend_cdp_network_events::{CdpNetworkEventBatch, CdpNetworkEventEffect};
use pwright_cap_network_events::{
    NetworkEventPayload, NetworkEventStream, NetworkRequest, NetworkRequestFailed,
    NetworkRequestFinished, NetworkResponse,
};
use pwright_core::{
    ControlMessage, Event, EventSpec, EventTag, ObjectId, ObjectKindTag, ObjectRecord, PushOutcome,
};
use pwright_runtime::{ControlledDispatcher, DirectDispatcher, RuntimeStateError};
use pwright_runtime_control::{
    ControlMessageSource, OrderedControlGateError, OrderedRuntimeControl,
    OrderedRuntimeControlError,
};
use pwright_server_events::{ServerEventEmitError, TypedEventSender};
use pwright_server_wire::{ConcurrentServerBackend, ServerSupervisorCloseReason};
use pwright_transport::{FrameTransport, WireCodec};

/// Runtime boundary required by normalized Chromium HTTP lifecycle delivery.
///
/// Implementations atomically admit lifecycle batches. The publisher keeps the
/// subsequent control creation fence and semantic event admission in one
/// caller-polled turn.
pub trait ChromiumNetworkLifecycleRuntime {
    fn apply_network_lifecycle(
        &self,
        messages: &[ControlMessage],
    ) -> Result<Vec<ObjectRecord<ObjectKindTag>>, RuntimeStateError>;
}

impl<Service> ChromiumNetworkLifecycleRuntime for DirectDispatcher<Service> {
    fn apply_network_lifecycle(
        &self,
        messages: &[ControlMessage],
    ) -> Result<Vec<ObjectRecord<ObjectKindTag>>, RuntimeStateError> {
        self.apply_lifecycle_batch(messages)
    }
}

impl<Service, Cancellation, Deadlines, Cleanup> ChromiumNetworkLifecycleRuntime
    for ControlledDispatcher<Service, Cancellation, Deadlines, Cleanup>
{
    fn apply_network_lifecycle(
        &self,
        messages: &[ControlMessage],
    ) -> Result<Vec<ObjectRecord<ObjectKindTag>>, RuntimeStateError> {
        self.apply_lifecycle_batch(messages)
    }
}

/// Remote delivery targets selected for one Page-scoped lifecycle event.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChromiumNetworkEventScopes {
    direct_page: bool,
    browser_context: Option<ObjectId>,
}

impl ChromiumNetworkEventScopes {
    pub const fn new(direct_page: bool, browser_context: Option<ObjectId>) -> Self {
        Self {
            direct_page,
            browser_context,
        }
    }

    pub const fn direct_page(self) -> bool {
        self.direct_page
    }

    pub const fn browser_context(self) -> Option<ObjectId> {
        self.browser_context
    }

    pub const fn is_empty(self) -> bool {
        !self.direct_page && self.browser_context.is_none()
    }
}

/// Injected dynamic subscription-selection port.
///
/// The delivery leaf deliberately knows nothing about topology ownership or
/// listener mutation. An aggregate may select the Page lane, its owning
/// BrowserContext lane, both, or neither for each exact event tag.
pub trait ChromiumNetworkEventScopeSelector: Send + Sync {
    fn remote_scopes(
        &self,
        page: ObjectId,
        event: EventTag,
    ) -> Result<ChromiumNetworkEventScopes, Box<dyn Error + Send + Sync>>;
}

/// Stateless selector useful when a deployment wants every event only on its
/// direct Page route and has no dynamic listener registry.
#[derive(Clone, Copy, Debug, Default)]
pub struct ChromiumDirectNetworkEventScopeSelector;

impl ChromiumNetworkEventScopeSelector for ChromiumDirectNetworkEventScopeSelector {
    fn remote_scopes(
        &self,
        _page: ObjectId,
        _event: EventTag,
    ) -> Result<ChromiumNetworkEventScopes, Box<dyn Error + Send + Sync>> {
        Ok(ChromiumNetworkEventScopes::new(true, None))
    }
}

/// Injected single-cursor normalized batch port.
///
/// The future borrows the driver, so this trait cannot detach work or own a
/// runtime task. A successful publication must acknowledge the exact Page
/// before another batch may be received.
pub type ChromiumNetworkBatchReceiveFuture<'a, DriverError> =
    Pin<Box<dyn Future<Output = Result<Option<CdpNetworkEventBatch>, DriverError>> + Send + 'a>>;

pub trait ChromiumNetworkEventBatchDriver: Send + 'static {
    type Error: Error + Send + Sync + 'static;

    fn receive_once(&mut self) -> ChromiumNetworkBatchReceiveFuture<'_, Self::Error>;

    fn acknowledge_batch_publication(&mut self, page: ObjectId) -> Result<(), Self::Error>;
}

/// Lifecycle publisher for one globally ordered remote network-event stream.
///
/// It owns neither a native cursor nor a task. The injected selector preserves
/// dynamic Page and BrowserContext listener semantics without introducing a
/// dependency on an aggregate topology registry.
#[derive(Clone)]
pub struct ChromiumNetworkLifecyclePublisher {
    ordered_remote: TypedEventSender<NetworkEventStream>,
    remote_scopes: Arc<dyn ChromiumNetworkEventScopeSelector>,
}

impl ChromiumNetworkLifecyclePublisher {
    pub fn new(
        ordered_remote: TypedEventSender<NetworkEventStream>,
        remote_scopes: Arc<dyn ChromiumNetworkEventScopeSelector>,
    ) -> Self {
        Self {
            ordered_remote,
            remote_scopes,
        }
    }

    pub fn direct(ordered_remote: TypedEventSender<NetworkEventStream>) -> Self {
        Self::new(
            ordered_remote,
            Arc::new(ChromiumDirectNetworkEventScopeSelector),
        )
    }

    /// Admits object creation, writes controls through the final Create,
    /// publishes every selected semantic snapshot, then retires terminal roots.
    ///
    /// Any queue loss is terminal for this connection. The caller must not
    /// acknowledge the batch after an error.
    pub async fn publish_after_creation_fence<Runtime, Source, Codec, Transport>(
        &self,
        batch: CdpNetworkEventBatch,
        runtime: &Runtime,
        control: &OrderedRuntimeControl<Source, Codec, Transport>,
    ) -> Result<
        ChromiumNetworkLifecyclePublication,
        ChromiumNetworkLifecyclePublishError<Codec::Error, Transport::Error>,
    >
    where
        Runtime: ChromiumNetworkLifecycleRuntime,
        Source: ControlMessageSource,
        Codec: WireCodec<ControlMessage>,
        Transport: FrameTransport,
    {
        let (lifecycle, events, retirement) = batch.into_parts();
        if !lifecycle.is_empty() {
            runtime
                .apply_network_lifecycle(&lifecycle)
                .map_err(ChromiumNetworkLifecyclePublishError::Runtime)?;
            if let Some((object_id, parent, kind)) = last_creation(&lifecycle) {
                let barrier = control
                    .begin_barrier()
                    .await
                    .map_err(ChromiumNetworkLifecyclePublishError::Gate)?;
                barrier
                    .send_through_creation(object_id, parent, kind)
                    .await
                    .map_err(ChromiumNetworkLifecyclePublishError::Control)?;
            }
        }

        let mut publication = ChromiumNetworkLifecyclePublication::default();
        for (effect_index, effect) in events.into_iter().enumerate() {
            match effect {
                CdpNetworkEventEffect::Request(event) => {
                    publication.requests += 1;
                    self.publish_selected(
                        event.object_id(),
                        NetworkRequest::EVENT,
                        NetworkEventPayload::Request(event.payload().clone()),
                        effect_index,
                        &mut publication,
                    )?;
                }
                CdpNetworkEventEffect::Response(event) => {
                    publication.responses += 1;
                    self.publish_selected(
                        event.object_id(),
                        NetworkResponse::EVENT,
                        NetworkEventPayload::Response(event.payload().clone()),
                        effect_index,
                        &mut publication,
                    )?;
                }
                CdpNetworkEventEffect::RequestFinished(event) => {
                    publication.finished += 1;
                    self.publish_selected(
                        event.object_id(),
                        NetworkRequestFinished::EVENT,
                        NetworkEventPayload::RequestFinished(event.payload().clone()),
                        effect_index,
                        &mut publication,
                    )?;
                }
                CdpNetworkEventEffect::RequestFailed(event) => {
                    publication.failed += 1;
                    self.publish_selected(
                        event.object_id(),
                        NetworkRequestFailed::EVENT,
                        NetworkEventPayload::RequestFailed(event.payload().clone()),
                        effect_index,
                        &mut publication,
                    )?;
                }
            }
        }

        if !retirement.is_empty() {
            publication.retired_roots = retirement.len();
            runtime
                .apply_network_lifecycle(&retirement)
                .map_err(ChromiumNetworkLifecyclePublishError::Runtime)?;
        }
        Ok(publication)
    }

    fn publish_selected<Codec, Transport>(
        &self,
        page: ObjectId,
        event: EventTag,
        payload: NetworkEventPayload,
        effect_index: usize,
        publication: &mut ChromiumNetworkLifecyclePublication,
    ) -> Result<(), ChromiumNetworkLifecyclePublishError<Codec, Transport>> {
        let scopes = self
            .remote_scopes
            .remote_scopes(page, event)
            .map_err(ChromiumNetworkLifecyclePublishError::Subscriptions)?;
        if scopes.is_empty() {
            publication.skipped_unselected += 1;
            return Ok(());
        }
        if scopes.direct_page() {
            publish_ordered(&self.ordered_remote, page, payload.clone(), effect_index)?;
            publication.ordered_remote += 1;
        }
        if let Some(browser_context) = scopes.browser_context() {
            publish_ordered(&self.ordered_remote, browser_context, payload, effect_index)?;
            publication.ordered_remote += 1;
        }
        Ok(())
    }
}

fn last_creation(
    messages: &[ControlMessage],
) -> Option<(ObjectId, Option<ObjectId>, ObjectKindTag)> {
    messages.iter().rev().find_map(|message| match *message {
        ControlMessage::Create {
            object_id,
            parent,
            kind,
        } => Some((object_id, parent, kind)),
        _ => None,
    })
}

fn publish_ordered<Codec, Transport>(
    sender: &TypedEventSender<NetworkEventStream>,
    source: ObjectId,
    payload: NetworkEventPayload,
    effect_index: usize,
) -> Result<(), ChromiumNetworkLifecyclePublishError<Codec, Transport>> {
    let event = payload.event_tag();
    match sender.emit(Event::new(source, payload)) {
        Ok(PushOutcome::Enqueued) => Ok(()),
        Ok(PushOutcome::Rejected { .. }) => {
            Err(ChromiumNetworkLifecyclePublishError::QueueRejected {
                effect_index,
                event,
            })
        }
        Ok(PushOutcome::DroppedOldest { .. }) => {
            Err(ChromiumNetworkLifecyclePublishError::QueueEvicted {
                effect_index,
                event,
            })
        }
        Err(source) => Err(ChromiumNetworkLifecyclePublishError::Emit {
            effect_index,
            event,
            source,
        }),
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChromiumNetworkLifecyclePublication {
    pub requests: usize,
    pub responses: usize,
    pub finished: usize,
    pub failed: usize,
    pub ordered_remote: usize,
    pub skipped_unselected: usize,
    pub retired_roots: usize,
}

#[derive(Debug)]
pub enum ChromiumNetworkLifecyclePublishError<Codec, Transport> {
    Runtime(RuntimeStateError),
    Subscriptions(Box<dyn Error + Send + Sync>),
    Gate(OrderedControlGateError),
    Control(OrderedRuntimeControlError<Codec, Transport>),
    QueueRejected {
        effect_index: usize,
        event: EventTag,
    },
    QueueEvicted {
        effect_index: usize,
        event: EventTag,
    },
    Emit {
        effect_index: usize,
        event: EventTag,
        source: ServerEventEmitError,
    },
}

impl<Codec, Transport> Display for ChromiumNetworkLifecyclePublishError<Codec, Transport>
where
    Codec: Display,
    Transport: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runtime(error) => Display::fmt(error, formatter),
            Self::Subscriptions(error) => Display::fmt(error, formatter),
            Self::Gate(error) => Display::fmt(error, formatter),
            Self::Control(error) => Display::fmt(error, formatter),
            Self::QueueRejected {
                effect_index,
                event,
            } => write!(
                formatter,
                "network event {event:?} at batch index {effect_index} was rejected"
            ),
            Self::QueueEvicted {
                effect_index,
                event,
            } => write!(
                formatter,
                "network event {event:?} at batch index {effect_index} evicted older state"
            ),
            Self::Emit {
                effect_index,
                event,
                source,
            } => write!(
                formatter,
                "network event {event:?} at batch index {effect_index} could not be admitted: {source}"
            ),
        }
    }
}

impl<Codec, Transport> Error for ChromiumNetworkLifecyclePublishError<Codec, Transport>
where
    Codec: Error + 'static,
    Transport: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Runtime(error) => Some(error),
            Self::Subscriptions(error) => Some(error.as_ref()),
            Self::Gate(error) => Some(error),
            Self::Control(error) => Some(error),
            Self::Emit { source, .. } => Some(source),
            Self::QueueRejected { .. } | Self::QueueEvicted { .. } => None,
        }
    }
}

#[derive(Debug)]
pub enum ChromiumNetworkLifecyclePumpError<Driver, Codec, Transport> {
    Driver(Driver),
    BatchWithoutPage,
    Publication(ChromiumNetworkLifecyclePublishError<Codec, Transport>),
}

impl<Driver, Codec, Transport> Display
    for ChromiumNetworkLifecyclePumpError<Driver, Codec, Transport>
where
    Driver: Display,
    Codec: Display,
    Transport: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Driver(error) => Display::fmt(error, formatter),
            Self::BatchWithoutPage => formatter.write_str(
                "the network batch driver returned a non-ignored batch without a Page source",
            ),
            Self::Publication(error) => Display::fmt(error, formatter),
        }
    }
}

impl<Driver, Codec, Transport> Error for ChromiumNetworkLifecyclePumpError<Driver, Codec, Transport>
where
    Driver: Error + 'static,
    Codec: Error + 'static,
    Transport: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Driver(error) => Some(error),
            Self::Publication(error) => Some(error),
            Self::BatchWithoutPage => None,
        }
    }
}

pub async fn drive_chromium_network_lifecycle_once<Driver, Runtime, Source, Codec, Transport>(
    driver: &mut Driver,
    publisher: &ChromiumNetworkLifecyclePublisher,
    runtime: &Runtime,
    control: &OrderedRuntimeControl<Source, Codec, Transport>,
) -> Result<
    Option<ChromiumNetworkLifecyclePublication>,
    ChromiumNetworkLifecyclePumpError<Driver::Error, Codec::Error, Transport::Error>,
>
where
    Driver: ChromiumNetworkEventBatchDriver,
    Runtime: ChromiumNetworkLifecycleRuntime,
    Source: ControlMessageSource,
    Codec: WireCodec<ControlMessage>,
    Transport: FrameTransport,
{
    let Some(batch) = driver
        .receive_once()
        .await
        .map_err(ChromiumNetworkLifecyclePumpError::Driver)?
    else {
        return Ok(None);
    };
    let page = batch
        .page()
        .ok_or(ChromiumNetworkLifecyclePumpError::BatchWithoutPage)?;
    let publication = publisher
        .publish_after_creation_fence(batch, runtime, control)
        .await
        .map_err(ChromiumNetworkLifecyclePumpError::Publication)?;
    driver
        .acknowledge_batch_publication(page)
        .map_err(ChromiumNetworkLifecyclePumpError::Driver)?;
    Ok(Some(publication))
}

type BoxedChromiumNetworkLifecycleDrive<Driver, DriverError, CodecError, TransportError> = Pin<
    Box<
        dyn Future<
                Output = (
                    Driver,
                    Result<
                        Option<ChromiumNetworkLifecyclePublication>,
                        ChromiumNetworkLifecyclePumpError<DriverError, CodecError, TransportError>,
                    >,
                ),
            > + Send
            + 'static,
    >,
>;

type ChromiumNetworkLifecycleDriveFor<Driver, Codec, Transport> =
    BoxedChromiumNetworkLifecycleDrive<
        Driver,
        <Driver as ChromiumNetworkEventBatchDriver>::Error,
        <Codec as WireCodec<ControlMessage>>::Error,
        <Transport as FrameTransport>::Error,
    >;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChromiumNetworkLifecycleBackendOutcome {
    Ignored,
    Published(ChromiumNetworkLifecyclePublication),
}

#[derive(Debug)]
pub enum ChromiumNetworkLifecycleBackendError<Driver, Codec, Transport> {
    ConnectionTerminated { reason: ServerSupervisorCloseReason },
    Pump(ChromiumNetworkLifecyclePumpError<Driver, Codec, Transport>),
}

impl<Driver, Codec, Transport> Display
    for ChromiumNetworkLifecycleBackendError<Driver, Codec, Transport>
where
    Driver: Display,
    Codec: Display,
    Transport: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionTerminated { reason } => write!(
                formatter,
                "the server connection terminated as {reason:?} before another network turn"
            ),
            Self::Pump(error) => Display::fmt(error, formatter),
        }
    }
}

impl<Driver, Codec, Transport> Error
    for ChromiumNetworkLifecycleBackendError<Driver, Codec, Transport>
where
    Driver: Error + 'static,
    Codec: Error + 'static,
    Transport: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ConnectionTerminated { .. } => None,
            Self::Pump(error) => Some(error),
        }
    }
}

/// Caller-polled adapter that retains each receive/publish/ack turn across
/// supervisor polls. It owns no task and drops both the in-flight future and
/// driver when the connection reaches a terminal state.
pub struct ChromiumNetworkLifecycleBackend<Driver, Runtime, Source, Codec, Transport>
where
    Driver: ChromiumNetworkEventBatchDriver,
    Codec: WireCodec<ControlMessage>,
    Transport: FrameTransport,
{
    runtime: Arc<Runtime>,
    control: OrderedRuntimeControl<Source, Codec, Transport>,
    publisher: ChromiumNetworkLifecyclePublisher,
    driver: Option<Driver>,
    in_flight: Option<ChromiumNetworkLifecycleDriveFor<Driver, Codec, Transport>>,
    connection_termination: Option<ServerSupervisorCloseReason>,
}

impl<Driver, Runtime, Source, Codec, Transport>
    ChromiumNetworkLifecycleBackend<Driver, Runtime, Source, Codec, Transport>
where
    Driver: ChromiumNetworkEventBatchDriver,
    Codec: WireCodec<ControlMessage>,
    Transport: FrameTransport,
{
    pub const fn new(
        driver: Driver,
        publisher: ChromiumNetworkLifecyclePublisher,
        runtime: Arc<Runtime>,
        control: OrderedRuntimeControl<Source, Codec, Transport>,
    ) -> Self {
        Self {
            runtime,
            control,
            publisher,
            driver: Some(driver),
            in_flight: None,
            connection_termination: None,
        }
    }

    pub const fn is_driving(&self) -> bool {
        self.in_flight.is_some()
    }

    pub const fn driver(&self) -> Option<&Driver> {
        self.driver.as_ref()
    }

    pub const fn connection_termination(&self) -> Option<ServerSupervisorCloseReason> {
        self.connection_termination
    }
}

impl<Driver, Runtime, Source, Codec, Transport> ConcurrentServerBackend
    for ChromiumNetworkLifecycleBackend<Driver, Runtime, Source, Codec, Transport>
where
    Driver: ChromiumNetworkEventBatchDriver,
    Runtime: ChromiumNetworkLifecycleRuntime + Send + Sync + 'static,
    Source: ControlMessageSource + Send + Sync + 'static,
    Codec: WireCodec<ControlMessage> + Send + Sync + 'static,
    Codec::Error: Send,
    Transport: FrameTransport + Send + Sync + 'static,
    Transport::Error: Send,
{
    type Event = ChromiumNetworkLifecycleBackendOutcome;
    type Error =
        ChromiumNetworkLifecycleBackendError<Driver::Error, Codec::Error, Transport::Error>;

    fn poll_drive_once(
        &mut self,
        context: &mut Context<'_>,
    ) -> Poll<Result<Self::Event, Self::Error>> {
        if let Some(reason) = self.connection_termination {
            return Poll::Ready(Err(
                ChromiumNetworkLifecycleBackendError::ConnectionTerminated { reason },
            ));
        }
        if self.in_flight.is_none() {
            let mut driver = self
                .driver
                .take()
                .expect("a network driver is absent only while its drive future is retained");
            let publisher = self.publisher.clone();
            let runtime = Arc::clone(&self.runtime);
            let control = self.control.clone();
            self.in_flight = Some(Box::pin(async move {
                let result = drive_chromium_network_lifecycle_once(
                    &mut driver,
                    &publisher,
                    runtime.as_ref(),
                    &control,
                )
                .await;
                (driver, result)
            }));
        }

        let output = {
            let drive = self
                .in_flight
                .as_mut()
                .expect("a network drive must exist after initialization");
            match drive.as_mut().poll(context) {
                Poll::Ready(output) => output,
                Poll::Pending => return Poll::Pending,
            }
        };
        self.in_flight = None;
        self.driver = Some(output.0);
        Poll::Ready(
            output
                .1
                .map(|publication| match publication {
                    Some(publication) => {
                        ChromiumNetworkLifecycleBackendOutcome::Published(publication)
                    }
                    None => ChromiumNetworkLifecycleBackendOutcome::Ignored,
                })
                .map_err(ChromiumNetworkLifecycleBackendError::Pump),
        )
    }

    fn handle_connection_terminated(&mut self, reason: ServerSupervisorCloseReason) {
        if self.connection_termination.is_none() {
            self.in_flight = None;
            self.driver = None;
            self.connection_termination = Some(reason);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::error::Error;
    use std::fmt::{self, Display, Formatter};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::task::{Context, Poll};

    use pwright_backend_cdp_network_events::{CdpNetworkEventAdapter, CdpNetworkEventBatch};
    use pwright_browser_model::{
        BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND, BROWSER_OBJECT_KIND_CATALOG,
        PAGE_OBJECT_KIND,
    };
    use pwright_cap_network_events::{NetworkEventPayload, NetworkEventStream};
    use pwright_cdp::CdpEvent;
    use pwright_codec_control::ControlCodec;
    use pwright_codec_network_events::{NetworkEventsCodec, NetworkEventsCodecLimits};
    use pwright_core::{
        AtomicObjectIdSource, ObjectId, OverflowPolicy, ProtocolVersion, SequencedEvent,
    };
    use pwright_network_model::NetworkLifecycleLimits;
    use pwright_runtime::{DirectDispatcher, ObjectTopologyMode, RuntimeLimits};
    use pwright_runtime_control::{OrderedRuntimeControl, RuntimeControlDriver};
    use pwright_server_events::TypedEventOutbound;
    use pwright_server_wire::{
        ConcurrentServerBackend, ConcurrentServerOutbound, ServerSupervisorCloseReason,
    };
    use pwright_transport::{FrameLimits, FrameTransport, VersionPolicy, WireProtocol};
    use pwright_transport_memory::MemoryFrameTransport;
    use serde_json::json;

    use super::{
        ChromiumNetworkBatchReceiveFuture, ChromiumNetworkEventBatchDriver,
        ChromiumNetworkEventScopeSelector, ChromiumNetworkEventScopes,
        ChromiumNetworkLifecycleBackend, ChromiumNetworkLifecyclePublisher,
        drive_chromium_network_lifecycle_once,
    };

    fn network_limits() -> NetworkLifecycleLimits {
        NetworkLifecycleLimits::new(8, 64, 256, 16, 32, 8, 256, 64, 64, 128)
    }

    fn batch(request_id: &str) -> CdpNetworkEventBatch {
        let mut adapter = CdpNetworkEventAdapter::new(
            ObjectId::new(3),
            "session-1".to_owned(),
            Arc::new(AtomicObjectIdSource::new(10).expect("object IDs should validate")),
            network_limits(),
        )
        .expect("adapter should build");
        adapter
            .map_event(&CdpEvent::new(
                "Network.requestWillBeSent".to_owned(),
                json!({
                    "requestId": request_id,
                    "type": "Fetch",
                    "request": {
                        "url": "https://example.test/",
                        "method": "GET",
                        "headers": {}
                    }
                }),
                Some("session-1".to_owned()),
            ))
            .expect("request should normalize")
    }

    fn runtime() -> Arc<DirectDispatcher<()>> {
        let runtime = Arc::new(
            DirectDispatcher::with_object_kinds_and_topology(
                (),
                RuntimeLimits::new(16, 1).with_control_messages(32),
                BROWSER_OBJECT_KIND_CATALOG,
                ObjectTopologyMode::Complete,
            )
            .expect("runtime object catalog should validate"),
        );
        runtime
            .register_object(ObjectId::new(1), None, BROWSER_OBJECT_KIND.tag)
            .expect("browser should register");
        runtime
            .register_object(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                BROWSER_CONTEXT_OBJECT_KIND.tag,
            )
            .expect("context should register");
        runtime
            .register_object(
                ObjectId::new(3),
                Some(ObjectId::new(2)),
                PAGE_OBJECT_KIND.tag,
            )
            .expect("page should register");
        while runtime
            .pop_control_message()
            .expect("setup lifecycle should drain")
            .is_some()
        {}
        runtime
    }

    fn control_protocol() -> WireProtocol<ControlCodec> {
        WireProtocol::new(
            ControlCodec,
            VersionPolicy::exact(ProtocolVersion::new(1, 0)),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(1024, 0, 0, 1024),
        )
        .expect("control protocol should build")
    }

    fn event_protocol() -> WireProtocol<NetworkEventsCodec> {
        WireProtocol::new_exact_directional(
            NetworkEventsCodec::new(NetworkEventsCodecLimits::from_lifecycle(network_limits())),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(2048, 0, 0, 2048),
            FrameLimits::new(2048, 0, 0, 2048),
        )
    }

    fn event_delivery() -> (
        ChromiumNetworkLifecyclePublisher,
        TypedEventOutbound<NetworkEventStream, NetworkEventsCodec, MemoryFrameTransport>,
        MemoryFrameTransport,
    ) {
        let (server, peer) = MemoryFrameTransport::pair(4);
        let (sender, outbound) = TypedEventOutbound::new_routed(
            event_protocol(),
            server,
            4,
            OverflowPolicy::RejectNewest,
            pwright_cap_network_events::network_stream_event_tag,
        );
        (
            ChromiumNetworkLifecyclePublisher::direct(sender),
            outbound,
            peer,
        )
    }

    fn control(
        runtime: &Arc<DirectDispatcher<()>>,
    ) -> (
        OrderedRuntimeControl<Arc<DirectDispatcher<()>>, ControlCodec, MemoryFrameTransport>,
        MemoryFrameTransport,
    ) {
        let (server, peer) = MemoryFrameTransport::pair(4);
        let control = OrderedRuntimeControl::new(
            Arc::clone(runtime),
            RuntimeControlDriver::new(control_protocol(), server),
            1,
        )
        .expect("ordered control should build");
        (control, peer)
    }

    #[derive(Clone, Copy)]
    struct PageAndContextSelector;

    impl ChromiumNetworkEventScopeSelector for PageAndContextSelector {
        fn remote_scopes(
            &self,
            _page: ObjectId,
            _event: pwright_core::EventTag,
        ) -> Result<ChromiumNetworkEventScopes, Box<dyn Error + Send + Sync>> {
            Ok(ChromiumNetworkEventScopes::new(
                true,
                Some(ObjectId::new(2)),
            ))
        }
    }

    #[tokio::test]
    async fn lifecycle_publisher_fences_creation_then_orders_page_before_context() {
        // Arrange
        let runtime = runtime();
        let (control, control_peer) = control(&runtime);
        let (server, event_peer) = MemoryFrameTransport::pair(4);
        let (sender, outbound) = TypedEventOutbound::new_routed(
            event_protocol(),
            server,
            4,
            OverflowPolicy::RejectNewest,
            pwright_cap_network_events::network_stream_event_tag,
        );
        let publisher =
            ChromiumNetworkLifecyclePublisher::new(sender, Arc::new(PageAndContextSelector));

        // Act
        let publication = publisher
            .publish_after_creation_fence(batch("page-context"), runtime.as_ref(), &control)
            .await
            .expect("normalized request should publish");
        let creation = control_peer
            .receive()
            .await
            .expect("control receive should succeed")
            .expect("Request creation should be fenced");
        outbound
            .send_once()
            .await
            .expect("Page event send should succeed")
            .expect("Page event should be queued");
        outbound
            .send_once()
            .await
            .expect("Context event send should succeed")
            .expect("Context event should be queued");
        let page_frame = event_peer
            .receive()
            .await
            .expect("Page frame receive should succeed")
            .expect("Page frame should exist");
        let context_frame = event_peer
            .receive()
            .await
            .expect("Context frame receive should succeed")
            .expect("Context frame should exist");
        let page_event = event_protocol()
            .decode::<SequencedEvent<NetworkEventStream>>(page_frame)
            .expect("Page event should decode");
        let context_event = event_protocol()
            .decode::<SequencedEvent<NetworkEventStream>>(context_frame)
            .expect("Context event should decode");

        // Assert
        assert_eq!(creation.route(), pwright_transport::FrameRoute::Control);
        assert_eq!(publication.requests, 1);
        assert_eq!(publication.ordered_remote, 2);
        assert_eq!(page_event.object_id(), ObjectId::new(3));
        assert_eq!(context_event.object_id(), ObjectId::new(2));
        assert!(matches!(
            (page_event.payload(), context_event.payload()),
            (NetworkEventPayload::Request(page), NetworkEventPayload::Request(context))
                if page.request == context.request
        ));
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct TestDriverError;

    impl Display for TestDriverError {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            formatter.write_str("test driver failed")
        }
    }

    impl Error for TestDriverError {}

    struct OneBatchDriver {
        batch: Option<CdpNetworkEventBatch>,
        acknowledged: Option<ObjectId>,
    }

    impl ChromiumNetworkEventBatchDriver for OneBatchDriver {
        type Error = TestDriverError;

        fn receive_once(&mut self) -> ChromiumNetworkBatchReceiveFuture<'_, Self::Error> {
            Box::pin(async move { Ok(self.batch.take()) })
        }

        fn acknowledge_batch_publication(&mut self, page: ObjectId) -> Result<(), Self::Error> {
            self.acknowledged = Some(page);
            Ok(())
        }
    }

    #[tokio::test]
    async fn drive_turn_acknowledges_only_after_publication() {
        // Arrange
        let runtime = runtime();
        let (control, _control_peer) = control(&runtime);
        let (publisher, _outbound, _event_peer) = event_delivery();
        let mut driver = OneBatchDriver {
            batch: Some(batch("acknowledge")),
            acknowledged: None,
        };

        // Act
        let publication = drive_chromium_network_lifecycle_once(
            &mut driver,
            &publisher,
            runtime.as_ref(),
            &control,
        )
        .await
        .expect("drive turn should succeed")
        .expect("request should produce a publication");

        // Assert
        assert_eq!(publication.requests, 1);
        assert_eq!(driver.acknowledged, Some(ObjectId::new(3)));
    }

    struct PendingDriver {
        dropped: Arc<AtomicBool>,
    }

    impl Drop for PendingDriver {
        fn drop(&mut self) {
            self.dropped.store(true, Ordering::SeqCst);
        }
    }

    impl ChromiumNetworkEventBatchDriver for PendingDriver {
        type Error = Infallible;

        fn receive_once(&mut self) -> ChromiumNetworkBatchReceiveFuture<'_, Self::Error> {
            Box::pin(std::future::pending())
        }

        fn acknowledge_batch_publication(&mut self, _page: ObjectId) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn terminal_connection_drops_the_retained_driver_future() {
        // Arrange
        let runtime = runtime();
        let (control, _control_peer) = control(&runtime);
        let (publisher, _outbound, _event_peer) = event_delivery();
        let dropped = Arc::new(AtomicBool::new(false));
        let mut backend = ChromiumNetworkLifecycleBackend::new(
            PendingDriver {
                dropped: Arc::clone(&dropped),
            },
            publisher,
            runtime,
            control,
        );

        // Act
        let initially_pending = std::future::poll_fn(|context| {
            let is_pending = backend.poll_drive_once(context).is_pending();
            std::task::Poll::Ready(is_pending)
        })
        .await;
        backend.handle_connection_terminated(ServerSupervisorCloseReason::EndOfStream);

        // Assert
        assert!(initially_pending);
        assert!(dropped.load(Ordering::SeqCst));
        assert!(backend.driver().is_none());
        assert!(!backend.is_driving());
        assert_eq!(
            backend.connection_termination(),
            Some(ServerSupervisorCloseReason::EndOfStream)
        );
        assert!(matches!(
            backend.poll_drive_once(&mut Context::from_waker(std::task::Waker::noop())),
            Poll::Ready(Err(
                super::ChromiumNetworkLifecycleBackendError::ConnectionTerminated {
                    reason: ServerSupervisorCloseReason::EndOfStream
                }
            ))
        ));
    }
}
