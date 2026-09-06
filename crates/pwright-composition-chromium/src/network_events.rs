use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use pwright_backend_cdp::{CdpNetworkEventBatch, CdpNetworkEventEffect};
use pwright_cap_network_events::{
    NetworkEventPayload, NetworkEventStream, NetworkRequest, NetworkRequestFailed,
    NetworkRequestFinished, NetworkResponse,
};
use pwright_core::{ControlMessage, EventSpec, EventTag, ObjectKindTag, ObjectRecord, PushOutcome};
use pwright_runtime::{ControlledDispatcher, DirectDispatcher, RuntimeStateError};
use pwright_runtime_control::{
    ControlMessageSource, OrderedControlGateError, OrderedRuntimeControl,
    OrderedRuntimeControlError,
};
use pwright_server_events::{ServerEventEmitError, TypedEventSender};
use pwright_server_wire::{ConcurrentServerBackend, ServerSupervisorCloseReason};
use pwright_transport::{FrameTransport, WireCodec};

use crate::{
    ChromiumNetworkEventDriver, ChromiumNetworkEventDriverError,
    ChromiumNetworkSubscriptionRegistry, ChromiumNetworkSubscriptionStateError,
};

impl pwright_composition_chromium_network_events::ChromiumNetworkEventScopeSelector
    for ChromiumNetworkSubscriptionRegistry
{
    fn remote_scopes(
        &self,
        page: pwright_core::ObjectId,
        event: EventTag,
    ) -> Result<
        pwright_composition_chromium_network_events::ChromiumNetworkEventScopes,
        Box<dyn Error + Send + Sync>,
    > {
        ChromiumNetworkSubscriptionRegistry::remote_scopes(self, page, event)
            .map(|(direct, browser_context)| {
                pwright_composition_chromium_network_events::ChromiumNetworkEventScopes::new(
                    direct,
                    browser_context,
                )
            })
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }
}

impl pwright_composition_chromium_network_events::ChromiumNetworkEventBatchDriver
    for ChromiumNetworkEventDriver
{
    type Error = ChromiumNetworkEventDriverError;

    fn receive_once(
        &mut self,
    ) -> pwright_composition_chromium_network_events::ChromiumNetworkBatchReceiveFuture<
        '_,
        Self::Error,
    > {
        Box::pin(ChromiumNetworkEventDriver::receive_once(self))
    }

    fn acknowledge_batch_publication(
        &mut self,
        page: pwright_core::ObjectId,
    ) -> Result<(), Self::Error> {
        ChromiumNetworkEventDriver::acknowledge_batch_publication(self, page)
    }
}

/// Runtime boundary required by HTTP lifecycle publication.
///
/// Creations are admitted atomically before an event can expose a new Request
/// or Response identity. Terminal root disposals are admitted only after every
/// event in the same batch has entered its selected bounded queue.
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

/// Four independently optional typed event queues behind one ordered network
/// lifecycle publisher. This type owns no CDP cursor, task, transport, or
/// response body.
#[derive(Clone, Default)]
pub struct ChromiumNetworkEventPublisher {
    requests: Option<TypedEventSender<NetworkRequest>>,
    responses: Option<TypedEventSender<NetworkResponse>>,
    finished: Option<TypedEventSender<NetworkRequestFinished>>,
    failed: Option<TypedEventSender<NetworkRequestFailed>>,
    ordered_remote: Option<TypedEventSender<NetworkEventStream>>,
    remote_subscriptions: Option<ChromiumNetworkSubscriptionRegistry>,
}

struct SelectedRemoteScopes<'a> {
    sender: Option<&'a TypedEventSender<NetworkEventStream>>,
    direct: bool,
    browser_context: Option<pwright_core::ObjectId>,
}

impl ChromiumNetworkEventPublisher {
    pub const fn new(
        requests: Option<TypedEventSender<NetworkRequest>>,
        responses: Option<TypedEventSender<NetworkResponse>>,
        finished: Option<TypedEventSender<NetworkRequestFinished>>,
        failed: Option<TypedEventSender<NetworkRequestFailed>>,
    ) -> Self {
        Self {
            requests,
            responses,
            finished,
            failed,
            ordered_remote: None,
            remote_subscriptions: None,
        }
    }

    pub fn with_ordered_remote(mut self, sender: TypedEventSender<NetworkEventStream>) -> Self {
        self.ordered_remote = Some(sender);
        self
    }

    /// Filters only the dynamic remote stream through the committed Page/tag
    /// registry. Independently installed in-process typed queues are not
    /// affected by a remote peer's listener choices.
    pub fn with_remote_subscriptions(
        mut self,
        subscriptions: ChromiumNetworkSubscriptionRegistry,
    ) -> Self {
        self.remote_subscriptions = Some(subscriptions);
        self
    }

    pub const fn is_inert(&self) -> bool {
        self.requests.is_none()
            && self.responses.is_none()
            && self.finished.is_none()
            && self.failed.is_none()
            && self.ordered_remote.is_none()
    }

    /// Atomically admits Request/Response objects, physically writes controls
    /// through the final Create, exposes semantic events, then retires every
    /// terminal Request root.
    ///
    /// Queue loss is terminal. Some effects from a redirect batch may already
    /// have been admitted when a later typed queue rejects; the caller must
    /// close or reconstruct the connection and must not continue this lane.
    pub async fn publish_after_creation_fence<Runtime, Source, Codec, Transport>(
        &self,
        batch: CdpNetworkEventBatch,
        runtime: &Runtime,
        control: &OrderedRuntimeControl<Source, Codec, Transport>,
    ) -> Result<
        ChromiumNetworkEventPublication,
        ChromiumNetworkEventPublishError<Codec::Error, Transport::Error>,
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
                .map_err(ChromiumNetworkEventPublishError::Runtime)?;
            if let Some((object_id, parent, kind)) = lifecycle.iter().rev().find_map(|message| {
                if let ControlMessage::Create {
                    object_id,
                    parent,
                    kind,
                } = *message
                {
                    Some((object_id, parent, kind))
                } else {
                    None
                }
            }) {
                let barrier = control
                    .begin_barrier()
                    .await
                    .map_err(ChromiumNetworkEventPublishError::Gate)?;
                barrier
                    .send_through_creation(object_id, parent, kind)
                    .await
                    .map_err(ChromiumNetworkEventPublishError::Control)?;
            }
        }

        let mut publication = ChromiumNetworkEventPublication::default();
        for (effect_index, effect) in events.into_iter().enumerate() {
            match effect {
                CdpNetworkEventEffect::Request(event) => {
                    let remote_scopes =
                        self.selected_remote_scopes(event.object_id(), NetworkRequest::EVENT)?;
                    publication.ordered_remote += publish_ordered_scopes(
                        remote_scopes,
                        event.object_id(),
                        NetworkEventPayload::Request(event.payload().clone()),
                        effect_index,
                    )?;
                    publish_event(self.requests.as_ref(), event, effect_index)?;
                    if self.requests.is_some() {
                        publication.requests += 1;
                    } else {
                        publication.skipped_uninstalled += 1;
                    }
                }
                CdpNetworkEventEffect::Response(event) => {
                    let remote_scopes =
                        self.selected_remote_scopes(event.object_id(), NetworkResponse::EVENT)?;
                    publication.ordered_remote += publish_ordered_scopes(
                        remote_scopes,
                        event.object_id(),
                        NetworkEventPayload::Response(event.payload().clone()),
                        effect_index,
                    )?;
                    publish_event(self.responses.as_ref(), event, effect_index)?;
                    if self.responses.is_some() {
                        publication.responses += 1;
                    } else {
                        publication.skipped_uninstalled += 1;
                    }
                }
                CdpNetworkEventEffect::RequestFinished(event) => {
                    let remote_scopes = self
                        .selected_remote_scopes(event.object_id(), NetworkRequestFinished::EVENT)?;
                    publication.ordered_remote += publish_ordered_scopes(
                        remote_scopes,
                        event.object_id(),
                        NetworkEventPayload::RequestFinished(event.payload().clone()),
                        effect_index,
                    )?;
                    publish_event(self.finished.as_ref(), event, effect_index)?;
                    if self.finished.is_some() {
                        publication.finished += 1;
                    } else {
                        publication.skipped_uninstalled += 1;
                    }
                }
                CdpNetworkEventEffect::RequestFailed(event) => {
                    let remote_scopes = self
                        .selected_remote_scopes(event.object_id(), NetworkRequestFailed::EVENT)?;
                    publication.ordered_remote += publish_ordered_scopes(
                        remote_scopes,
                        event.object_id(),
                        NetworkEventPayload::RequestFailed(event.payload().clone()),
                        effect_index,
                    )?;
                    publish_event(self.failed.as_ref(), event, effect_index)?;
                    if self.failed.is_some() {
                        publication.failed += 1;
                    } else {
                        publication.skipped_uninstalled += 1;
                    }
                }
            }
        }
        if !retirement.is_empty() {
            let retired_roots = retirement.len();
            runtime
                .apply_network_lifecycle(&retirement)
                .map_err(ChromiumNetworkEventPublishError::Runtime)?;
            publication.retired_roots = retired_roots;
        }
        Ok(publication)
    }

    fn selected_remote_scopes<Codec, Transport>(
        &self,
        page: pwright_core::ObjectId,
        event: EventTag,
    ) -> Result<SelectedRemoteScopes<'_>, ChromiumNetworkEventPublishError<Codec, Transport>> {
        let Some(sender) = self.ordered_remote.as_ref() else {
            return Ok(SelectedRemoteScopes {
                sender: None,
                direct: false,
                browser_context: None,
            });
        };
        let Some(subscriptions) = self.remote_subscriptions.as_ref() else {
            return Ok(SelectedRemoteScopes {
                sender: Some(sender),
                direct: true,
                browser_context: None,
            });
        };
        subscriptions
            .remote_scopes(page, event)
            .map(|(direct, browser_context)| SelectedRemoteScopes {
                sender: Some(sender),
                direct,
                browser_context,
            })
            .map_err(ChromiumNetworkEventPublishError::Subscriptions)
    }
}

impl ChromiumNetworkEventDriver {
    /// Owns one complete network turn: normalize one native event, publish its
    /// lifecycle and semantic effects, then acknowledge the exact Page. A
    /// publication failure intentionally leaves the driver barrier closed so
    /// topology replacement/retirement cannot overtake an ambiguous batch.
    pub async fn receive_and_publish_once<Runtime, Source, Codec, Transport>(
        &mut self,
        publisher: &ChromiumNetworkEventPublisher,
        runtime: &Runtime,
        control: &OrderedRuntimeControl<Source, Codec, Transport>,
    ) -> Result<
        Option<ChromiumNetworkEventPublication>,
        ChromiumNetworkEventPumpError<Codec::Error, Transport::Error>,
    >
    where
        Runtime: ChromiumNetworkLifecycleRuntime,
        Source: ControlMessageSource,
        Codec: WireCodec<ControlMessage>,
        Transport: FrameTransport,
    {
        let Some(batch) = self
            .receive_once()
            .await
            .map_err(ChromiumNetworkEventPumpError::Driver)?
        else {
            return Ok(None);
        };
        let page = batch
            .page()
            .expect("a non-empty normalized network batch has one Page source");
        let publication = publisher
            .publish_after_creation_fence(batch, runtime, control)
            .await
            .map_err(ChromiumNetworkEventPumpError::Publication)?;
        self.acknowledge_batch_publication(page)
            .map_err(ChromiumNetworkEventPumpError::Driver)?;
        Ok(Some(publication))
    }
}

fn publish_ordered_scopes<Codec, Transport>(
    scopes: SelectedRemoteScopes<'_>,
    page: pwright_core::ObjectId,
    payload: NetworkEventPayload,
    effect_index: usize,
) -> Result<usize, ChromiumNetworkEventPublishError<Codec, Transport>> {
    let Some(sender) = scopes.sender else {
        return Ok(0);
    };
    let mut published = 0;
    if scopes.direct {
        publish_ordered(Some(sender), page, payload.clone(), effect_index)?;
        published += 1;
    }
    if let Some(browser_context) = scopes.browser_context {
        publish_ordered(Some(sender), browser_context, payload, effect_index)?;
        published += 1;
    }
    Ok(published)
}

fn publish_ordered<Codec, Transport>(
    sender: Option<&TypedEventSender<NetworkEventStream>>,
    source: pwright_core::ObjectId,
    payload: NetworkEventPayload,
    effect_index: usize,
) -> Result<(), ChromiumNetworkEventPublishError<Codec, Transport>> {
    let event = payload.event_tag();
    let Some(sender) = sender else {
        return Ok(());
    };
    match sender.emit(pwright_core::Event::new(source, payload)) {
        Ok(PushOutcome::Enqueued) => Ok(()),
        Ok(PushOutcome::Rejected { .. }) => Err(ChromiumNetworkEventPublishError::QueueRejected {
            effect_index,
            event,
        }),
        Ok(PushOutcome::DroppedOldest { .. }) => {
            Err(ChromiumNetworkEventPublishError::QueueEvicted {
                effect_index,
                event,
            })
        }
        Err(source) => Err(ChromiumNetworkEventPublishError::Emit {
            effect_index,
            event,
            source,
        }),
    }
}

fn publish_event<S, Codec, Transport>(
    sender: Option<&TypedEventSender<S>>,
    event: pwright_core::Event<S>,
    effect_index: usize,
) -> Result<(), ChromiumNetworkEventPublishError<Codec, Transport>>
where
    S: EventSpec,
    S::Payload: Send,
{
    let Some(sender) = sender else {
        return Ok(());
    };
    match sender.emit(event) {
        Ok(PushOutcome::Enqueued) => Ok(()),
        Ok(PushOutcome::Rejected { .. }) => Err(ChromiumNetworkEventPublishError::QueueRejected {
            effect_index,
            event: S::EVENT,
        }),
        Ok(PushOutcome::DroppedOldest { .. }) => {
            Err(ChromiumNetworkEventPublishError::QueueEvicted {
                effect_index,
                event: S::EVENT,
            })
        }
        Err(source) => Err(ChromiumNetworkEventPublishError::Emit {
            effect_index,
            event: S::EVENT,
            source,
        }),
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChromiumNetworkEventPublication {
    pub requests: usize,
    pub responses: usize,
    pub finished: usize,
    pub failed: usize,
    pub ordered_remote: usize,
    pub skipped_uninstalled: usize,
    /// Terminal Request roots retired after their value snapshots were
    /// admitted. Response descendants are included by runtime cascade.
    pub retired_roots: usize,
}

#[derive(Debug)]
pub enum ChromiumNetworkEventPublishError<Codec, Transport> {
    Runtime(RuntimeStateError),
    Subscriptions(ChromiumNetworkSubscriptionStateError),
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

#[derive(Debug)]
pub enum ChromiumNetworkEventPumpError<Codec, Transport> {
    Driver(ChromiumNetworkEventDriverError),
    Publication(ChromiumNetworkEventPublishError<Codec, Transport>),
}

type BoxedChromiumNetworkEventDrive<CodecError, TransportError> = Pin<
    Box<
        dyn Future<
                Output = (
                    ChromiumNetworkEventDriver,
                    Result<
                        Option<ChromiumNetworkEventPublication>,
                        ChromiumNetworkEventPumpError<CodecError, TransportError>,
                    >,
                ),
            > + Send
            + 'static,
    >,
>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChromiumNetworkEventServerBackendOutcome {
    Ignored,
    Published(ChromiumNetworkEventPublication),
}

#[derive(Debug)]
pub enum ChromiumNetworkEventServerBackendError<Codec, Transport> {
    ConnectionTerminated { reason: ServerSupervisorCloseReason },
    Pump(ChromiumNetworkEventPumpError<Codec, Transport>),
}

impl<Codec, Transport> Display for ChromiumNetworkEventServerBackendError<Codec, Transport>
where
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

impl<Codec, Transport> Error for ChromiumNetworkEventServerBackendError<Codec, Transport>
where
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

/// Adapts the single-cursor network driver to the server supervisor's backend
/// lane. One retained future owns each complete receive/publication/ack turn,
/// so progress in another supervisor lane cannot cancel an admitted batch.
pub struct ChromiumNetworkEventServerBackend<Runtime, Source, Codec, Transport>
where
    Codec: WireCodec<ControlMessage>,
    Transport: FrameTransport,
{
    runtime: Arc<Runtime>,
    control: OrderedRuntimeControl<Source, Codec, Transport>,
    publisher: ChromiumNetworkEventPublisher,
    driver: Option<ChromiumNetworkEventDriver>,
    in_flight: Option<BoxedChromiumNetworkEventDrive<Codec::Error, Transport::Error>>,
    connection_termination: Option<ServerSupervisorCloseReason>,
}

impl<Runtime, Source, Codec, Transport>
    ChromiumNetworkEventServerBackend<Runtime, Source, Codec, Transport>
where
    Codec: WireCodec<ControlMessage>,
    Transport: FrameTransport,
{
    pub const fn new(
        driver: ChromiumNetworkEventDriver,
        publisher: ChromiumNetworkEventPublisher,
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

    pub const fn driver(&self) -> Option<&ChromiumNetworkEventDriver> {
        self.driver.as_ref()
    }

    pub const fn connection_termination(&self) -> Option<ServerSupervisorCloseReason> {
        self.connection_termination
    }
}

impl<Runtime, Source, Codec, Transport> ConcurrentServerBackend
    for ChromiumNetworkEventServerBackend<Runtime, Source, Codec, Transport>
where
    Runtime: ChromiumNetworkLifecycleRuntime + Send + Sync + 'static,
    Source: ControlMessageSource + Send + Sync + 'static,
    Codec: WireCodec<ControlMessage> + Send + Sync + 'static,
    Codec::Error: Send,
    Transport: FrameTransport + Send + Sync + 'static,
    Transport::Error: Send,
{
    type Event = ChromiumNetworkEventServerBackendOutcome;
    type Error = ChromiumNetworkEventServerBackendError<Codec::Error, Transport::Error>;

    fn poll_drive_once(
        &mut self,
        context: &mut Context<'_>,
    ) -> Poll<Result<Self::Event, Self::Error>> {
        if let Some(reason) = self.connection_termination {
            return Poll::Ready(Err(
                ChromiumNetworkEventServerBackendError::ConnectionTerminated { reason },
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
                let result = driver
                    .receive_and_publish_once(&publisher, runtime.as_ref(), &control)
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
                        ChromiumNetworkEventServerBackendOutcome::Published(publication)
                    }
                    None => ChromiumNetworkEventServerBackendOutcome::Ignored,
                })
                .map_err(ChromiumNetworkEventServerBackendError::Pump),
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

impl<Codec, Transport> Display for ChromiumNetworkEventPumpError<Codec, Transport>
where
    Codec: Display,
    Transport: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Driver(error) => Display::fmt(error, formatter),
            Self::Publication(error) => Display::fmt(error, formatter),
        }
    }
}

impl<Codec, Transport> Error for ChromiumNetworkEventPumpError<Codec, Transport>
where
    Codec: Error + 'static,
    Transport: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Driver(error) => Some(error),
            Self::Publication(error) => Some(error),
        }
    }
}

impl<Codec, Transport> Display for ChromiumNetworkEventPublishError<Codec, Transport>
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

impl<Codec, Transport> Error for ChromiumNetworkEventPublishError<Codec, Transport>
where
    Codec: Error + 'static,
    Transport: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Runtime(error) => Some(error),
            Self::Subscriptions(error) => Some(error),
            Self::Gate(error) => Some(error),
            Self::Control(error) => Some(error),
            Self::Emit { source, .. } => Some(source),
            Self::QueueRejected { .. } | Self::QueueEvicted { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::convert::Infallible;
    use std::sync::{Arc, Mutex};

    use pwright_backend_cdp::{
        CdpNetworkEventAdapter, CdpNetworkEventEffect, CdpNetworkEventError,
        CdpNetworkEventProductionProfile,
    };
    use pwright_browser_model::{
        BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND, BROWSER_OBJECT_KIND_CATALOG,
        PAGE_OBJECT_KIND,
    };
    use pwright_cap_network_events::{
        NetworkEventPayload, NetworkEventStream, NetworkRequest, NetworkRequestFailed,
        NetworkRequestFinished, NetworkResponse, network_stream_event_tag,
    };
    use pwright_cdp::{CdpClient, CdpEvent};
    use pwright_codec_control::ControlCodec;
    use pwright_codec_network_events::{NetworkEventsCodec, NetworkEventsCodecLimits};
    use pwright_core::{
        AtomicObjectIdSource, ControlMessage, Event, EventSpec, ObjectId, OverflowPolicy,
        ProductionResourceLedger, ProtocolVersion, ResourceLimits, ResourceRequirements,
        ResourceScope,
    };
    use pwright_fake::FakeCdpClient;
    use pwright_network_model::NetworkLifecycleLimits;
    use pwright_runtime::{DirectDispatcher, ObjectTopologyMode, RuntimeLimits};
    use pwright_runtime_control::{OrderedRuntimeControl, RuntimeControlDriver};
    use pwright_server_events::{TypedEventOutbound, TypedEventSender};
    use pwright_server_wire::{
        ConcurrentServerBackend, ConcurrentServerOutbound, ServerSupervisorCloseReason,
    };
    use pwright_transport::{FrameLimits, FrameRoute, FrameTransport, VersionPolicy, WireProtocol};
    use pwright_transport_memory::MemoryFrameTransport;
    use serde_json::json;

    use super::{
        ChromiumNetworkEventPublishError, ChromiumNetworkEventPublisher,
        ChromiumNetworkEventServerBackend, ChromiumNetworkEventServerBackendOutcome,
        publish_ordered_scopes,
    };
    use crate::{
        ChromiumNetworkEventDriver, ChromiumNetworkEventDriverShared,
        ChromiumNetworkEventDriverState, ChromiumNetworkPageAdapter,
        ChromiumNetworkSubscriptionRegistry,
    };

    type NetworkSenders = (
        TypedEventSender<NetworkRequest>,
        TypedEventSender<NetworkResponse>,
        TypedEventSender<NetworkRequestFinished>,
        TypedEventSender<NetworkRequestFailed>,
    );

    fn network_limits() -> NetworkLifecycleLimits {
        NetworkLifecycleLimits::new(8, 64, 256, 16, 32, 8, 256, 64, 64, 128)
    }

    fn adapter() -> CdpNetworkEventAdapter {
        CdpNetworkEventAdapter::new(
            ObjectId::new(3),
            "session-1".to_owned(),
            Arc::new(AtomicObjectIdSource::new(10).expect("object IDs should validate")),
            network_limits(),
        )
        .expect("adapter should build")
    }

    fn driver(
        source: &Arc<FakeCdpClient>,
        subscriptions: ChromiumNetworkSubscriptionRegistry,
    ) -> ChromiumNetworkEventDriver {
        let object_ids: Arc<dyn pwright_core::ObjectIdSource> =
            Arc::new(AtomicObjectIdSource::new(10).expect("object IDs should validate"));
        let mut adapters = BTreeMap::new();
        adapters.insert(
            ObjectId::new(3),
            ChromiumNetworkPageAdapter {
                browser_context_object_id: ObjectId::new(2),
                adapter: CdpNetworkEventAdapter::new(
                    ObjectId::new(3),
                    "session-1".to_owned(),
                    Arc::clone(&object_ids),
                    network_limits(),
                )
                .expect("network adapter should build"),
            },
        );
        let requirements = ResourceRequirements::none();
        let resources = ProductionResourceLedger::new(ResourceLimits::deny_all(0, 0), requirements)
            .expect("empty driver ledger should build");
        let adapter_profile = CdpNetworkEventProductionProfile::new(
            network_limits(),
            ResourceScope::extension(77, 1),
        )
        .expect("adapter profile should validate");
        let shared = Arc::new(ChromiumNetworkEventDriverShared {
            state: Mutex::new(ChromiumNetworkEventDriverState {
                adapters,
                pending_page_registration: None,
                pending_page_replacement: None,
                unpublished_batch: None,
            }),
            subscriptions,
            object_ids,
            adapter_profile,
            resources,
            max_page_lanes: 1,
        });
        ChromiumNetworkEventDriver {
            receiver: source.subscribe_events().into(),
            shared,
            held_event: None,
        }
    }

    fn cdp_event(method: &str, params: serde_json::Value) -> CdpEvent {
        CdpEvent::new(method.to_owned(), params, Some("session-1".to_owned()))
    }

    fn request(request_id: &str, url: &str) -> CdpEvent {
        cdp_event(
            "Network.requestWillBeSent",
            json!({
                "requestId": request_id,
                "type": "Fetch",
                "request": {"url": url, "method": "GET", "headers": {}}
            }),
        )
    }

    fn response(request_id: &str, url: &str) -> CdpEvent {
        cdp_event(
            "Network.responseReceived",
            json!({
                "requestId": request_id,
                "response": {
                    "url": url,
                    "status": 500,
                    "statusText": "Server Error",
                    "headers": {}
                }
            }),
        )
    }

    fn finished(request_id: &str) -> CdpEvent {
        cdp_event(
            "Network.loadingFinished",
            json!({"requestId": request_id, "encodedDataLength": 17}),
        )
    }

    fn failed(request_id: &str) -> CdpEvent {
        cdp_event(
            "Network.loadingFailed",
            json!({"requestId": request_id, "errorText": "net::ERR_ABORTED"}),
        )
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
            FrameLimits::new(64, 0, 0, 64),
        )
        .expect("control protocol should build")
    }

    fn event_protocol() -> WireProtocol<()> {
        WireProtocol::new_exact_directional(
            (),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(512, 0, 0, 512),
            FrameLimits::new(512, 0, 0, 512),
        )
    }

    fn network_event_protocol() -> WireProtocol<NetworkEventsCodec> {
        WireProtocol::new_exact_directional(
            NetworkEventsCodec::new(NetworkEventsCodecLimits::from_lifecycle(network_limits())),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(2048, 0, 0, 2048),
            FrameLimits::new(2048, 0, 0, 2048),
        )
    }

    fn publisher(capacity: usize) -> (ChromiumNetworkEventPublisher, NetworkSenders) {
        let (requests, _request_outbound) = TypedEventOutbound::<NetworkRequest, _, _>::new(
            event_protocol(),
            (),
            capacity,
            OverflowPolicy::RejectNewest,
        );
        let (responses, _response_outbound) = TypedEventOutbound::<NetworkResponse, _, _>::new(
            event_protocol(),
            (),
            capacity,
            OverflowPolicy::RejectNewest,
        );
        let (finished, _finished_outbound) =
            TypedEventOutbound::<NetworkRequestFinished, _, _>::new(
                event_protocol(),
                (),
                capacity,
                OverflowPolicy::RejectNewest,
            );
        let (failed, _failed_outbound) = TypedEventOutbound::<NetworkRequestFailed, _, _>::new(
            event_protocol(),
            (),
            capacity,
            OverflowPolicy::RejectNewest,
        );
        (
            ChromiumNetworkEventPublisher::new(
                Some(requests.clone()),
                Some(responses.clone()),
                Some(finished.clone()),
                Some(failed.clone()),
            ),
            (requests, responses, finished, failed),
        )
    }

    #[tokio::test]
    async fn owned_network_turn_publishes_then_acknowledges_the_exact_page() {
        // Arrange
        let source = Arc::new(FakeCdpClient::from_html(""));
        let subscriptions = ChromiumNetworkSubscriptionRegistry::new_unbudgeted(1);
        subscriptions
            .register_page_lane(ObjectId::new(3), ObjectId::new(2), "session-1".to_owned())
            .expect("Page lane should register");
        let object_ids: Arc<dyn pwright_core::ObjectIdSource> =
            Arc::new(AtomicObjectIdSource::new(10).expect("object IDs should validate"));
        let mut adapters = BTreeMap::new();
        adapters.insert(
            ObjectId::new(3),
            ChromiumNetworkPageAdapter {
                browser_context_object_id: ObjectId::new(2),
                adapter: CdpNetworkEventAdapter::new(
                    ObjectId::new(3),
                    "session-1".to_owned(),
                    Arc::clone(&object_ids),
                    network_limits(),
                )
                .expect("network adapter should build"),
            },
        );
        let requirements = ResourceRequirements::none();
        let resources = ProductionResourceLedger::new(ResourceLimits::deny_all(0, 0), requirements)
            .expect("empty driver ledger should build");
        let adapter_profile = CdpNetworkEventProductionProfile::new(
            network_limits(),
            ResourceScope::extension(77, 1),
        )
        .expect("adapter profile should validate");
        let shared = Arc::new(ChromiumNetworkEventDriverShared {
            state: Mutex::new(ChromiumNetworkEventDriverState {
                adapters,
                pending_page_registration: None,
                pending_page_replacement: None,
                unpublished_batch: None,
            }),
            subscriptions,
            object_ids,
            adapter_profile,
            resources,
            max_page_lanes: 1,
        });
        let mut driver = ChromiumNetworkEventDriver {
            receiver: source.subscribe_events().into(),
            shared,
            held_event: None,
        };
        let runtime = runtime();
        let (server_transport, peer_transport) = MemoryFrameTransport::pair(1);
        let control = OrderedRuntimeControl::new(
            Arc::clone(&runtime),
            RuntimeControlDriver::new(control_protocol(), server_transport),
            1,
        )
        .expect("ordered control should build");
        let (publisher, _senders) = publisher(1);
        source.send_event(request("native-owned", "https://example.test/owned"));

        // Act
        let publication = driver
            .receive_and_publish_once(&publisher, runtime.as_ref(), &control)
            .await
            .expect("owned network turn should publish")
            .expect("request should produce a publication");
        let create = peer_transport
            .receive()
            .await
            .expect("control receive should succeed")
            .expect("Request Create should be written");

        // Assert
        assert_eq!(publication.requests, 1);
        assert_eq!(driver.unpublished_batch(), None);
        assert_eq!(runtime.is_object_live(ObjectId::new(10)), Ok(true));
        assert_eq!(create.route(), FrameRoute::Control);
    }

    #[tokio::test]
    async fn server_backend_retains_and_completes_one_owned_network_turn() {
        // Arrange
        let source = Arc::new(FakeCdpClient::from_html(""));
        let subscriptions = ChromiumNetworkSubscriptionRegistry::new_unbudgeted(1);
        subscriptions
            .register_page_lane(ObjectId::new(3), ObjectId::new(2), "session-1".to_owned())
            .expect("Page lane should register");
        let driver = driver(&source, subscriptions);
        let runtime = runtime();
        let (server_transport, _peer_transport) = MemoryFrameTransport::pair(1);
        let control = OrderedRuntimeControl::new(
            Arc::clone(&runtime),
            RuntimeControlDriver::new(control_protocol(), server_transport),
            1,
        )
        .expect("ordered control should build");
        let (publisher, _senders) = publisher(1);
        let mut backend = ChromiumNetworkEventServerBackend::new(
            driver,
            publisher,
            Arc::clone(&runtime),
            control,
        );

        // Act
        let initially_pending =
            std::future::poll_fn(|context| match backend.poll_drive_once(context) {
                std::task::Poll::Pending => std::task::Poll::Ready(true),
                std::task::Poll::Ready(_) => std::task::Poll::Ready(false),
            })
            .await;
        source.send_event(request("native-backend", "https://example.test/backend"));
        let outcome = std::future::poll_fn(|context| backend.poll_drive_once(context))
            .await
            .expect("network backend should complete its retained turn");

        // Assert
        assert!(initially_pending);
        assert!(matches!(
            outcome,
            ChromiumNetworkEventServerBackendOutcome::Published(publication)
                if publication.requests == 1
        ));
        assert!(!backend.is_driving());
        assert_eq!(
            backend
                .driver()
                .expect("completed backend should retain its driver")
                .unpublished_batch(),
            None
        );
    }

    #[test]
    fn server_backend_connection_teardown_releases_network_registrations() {
        // Arrange
        let source = Arc::new(FakeCdpClient::from_html(""));
        let subscriptions = ChromiumNetworkSubscriptionRegistry::new_unbudgeted(1);
        subscriptions
            .register_page_lane(ObjectId::new(3), ObjectId::new(2), "session-1".to_owned())
            .expect("Page lane should register");
        let driver = driver(&source, subscriptions.clone());
        let runtime = runtime();
        let (server_transport, _peer_transport) = MemoryFrameTransport::pair(1);
        let control = OrderedRuntimeControl::new(
            Arc::clone(&runtime),
            RuntimeControlDriver::new(control_protocol(), server_transport),
            1,
        )
        .expect("ordered control should build");
        let mut backend = ChromiumNetworkEventServerBackend::new(
            driver,
            ChromiumNetworkEventPublisher::default(),
            runtime,
            control,
        );

        // Act
        backend.handle_connection_terminated(ServerSupervisorCloseReason::EndOfStream);
        let metrics = subscriptions
            .metrics()
            .expect("network registration metrics should remain readable");

        // Assert
        assert_eq!(metrics.registered_page_lanes(), 0);
        assert!(backend.driver().is_none());
        assert_eq!(
            backend.connection_termination(),
            Some(ServerSupervisorCloseReason::EndOfStream)
        );
    }

    #[test]
    fn ordered_remote_selection_uses_the_committed_page_and_event_tag_mask() {
        // Arrange
        let subscriptions = ChromiumNetworkSubscriptionRegistry::new_unbudgeted(1);
        subscriptions
            .register_page_lane(ObjectId::new(3), ObjectId::new(2), "session-1".to_owned())
            .expect("Page lane should register");
        subscriptions
            .begin_transition(
                ObjectId::new(3),
                "session-1",
                NetworkRequest::EVENT,
                true,
                false,
            )
            .expect("request interest should prepare")
            .commit()
            .expect("request interest should commit");
        let (remote, _outbound) = TypedEventOutbound::<NetworkEventStream, _, _>::new_routed(
            event_protocol(),
            (),
            1,
            OverflowPolicy::RejectNewest,
            network_stream_event_tag,
        );
        let publisher = ChromiumNetworkEventPublisher::default()
            .with_ordered_remote(remote)
            .with_remote_subscriptions(subscriptions);

        // Act
        let request = publisher
            .selected_remote_scopes::<Infallible, Infallible>(
                ObjectId::new(3),
                NetworkRequest::EVENT,
            )
            .expect("request selection should be readable");
        let response = publisher
            .selected_remote_scopes::<Infallible, Infallible>(
                ObjectId::new(3),
                NetworkResponse::EVENT,
            )
            .expect("response selection should be readable");
        let other_page = publisher
            .selected_remote_scopes::<Infallible, Infallible>(
                ObjectId::new(4),
                NetworkRequest::EVENT,
            )
            .expect("other Page selection should be readable");

        // Assert
        assert!(request.sender.is_some());
        assert!(request.direct);
        assert_eq!(request.browser_context, None);
        assert!(response.sender.is_some());
        assert!(!response.direct);
        assert_eq!(response.browser_context, None);
        assert!(other_page.sender.is_some());
        assert!(!other_page.direct);
        assert_eq!(other_page.browser_context, None);
    }

    #[tokio::test]
    async fn page_and_context_selection_publish_page_first_with_shared_request_identity() {
        // Arrange
        let subscriptions = ChromiumNetworkSubscriptionRegistry::new_unbudgeted(1);
        subscriptions
            .register_page_lane(ObjectId::new(3), ObjectId::new(2), "session-1".to_owned())
            .expect("Page lane should register");
        subscriptions
            .begin_transition(
                ObjectId::new(3),
                "session-1",
                NetworkRequest::EVENT,
                true,
                false,
            )
            .expect("Page request interest should prepare")
            .commit()
            .expect("Page request interest should commit");
        subscriptions
            .set_browser_context_interest_for_test(ObjectId::new(2), NetworkRequest::EVENT, true)
            .expect("Context request interest should commit");
        let (server_transport, peer_transport) = MemoryFrameTransport::pair(2);
        let (sender, outbound) = TypedEventOutbound::<NetworkEventStream, _, _>::new_routed(
            network_event_protocol(),
            server_transport,
            2,
            OverflowPolicy::RejectNewest,
            network_stream_event_tag,
        );
        let publisher = ChromiumNetworkEventPublisher::default()
            .with_ordered_remote(sender.clone())
            .with_remote_subscriptions(subscriptions);
        let scopes = publisher
            .selected_remote_scopes::<Infallible, Infallible>(
                ObjectId::new(3),
                NetworkRequest::EVENT,
            )
            .expect("both subscription scopes should be readable");
        let mut normalizer = adapter();
        let payload = normalizer
            .map_event(&request("native-shared", "https://example.test/shared"))
            .expect("request should normalize")
            .into_parts()
            .1
            .into_iter()
            .find_map(|effect| match effect {
                CdpNetworkEventEffect::Request(event) => {
                    Some(NetworkEventPayload::Request(event.payload().clone()))
                }
                _ => None,
            })
            .expect("request payload should exist");

        // Act
        let published =
            publish_ordered_scopes::<Infallible, Infallible>(scopes, ObjectId::new(3), payload, 0)
                .expect("both scopes should fit the ordered queue");
        outbound
            .send_once()
            .await
            .expect("Page-scoped frame should send");
        outbound
            .send_once()
            .await
            .expect("Context-scoped frame should send");
        let page_frame = peer_transport
            .receive()
            .await
            .expect("Page frame receive should succeed")
            .expect("Page frame should exist");
        let context_frame = peer_transport
            .receive()
            .await
            .expect("Context frame receive should succeed")
            .expect("Context frame should exist");

        // Assert
        assert_eq!(published, 2);
        assert_eq!(
            page_frame.route(),
            FrameRoute::Event {
                source: ObjectId::new(3),
                event: NetworkRequest::EVENT,
            }
        );
        assert_eq!(
            context_frame.route(),
            FrameRoute::Event {
                source: ObjectId::new(2),
                event: NetworkRequest::EVENT,
            }
        );
    }

    #[tokio::test]
    async fn terminal_snapshots_retire_request_roots_before_page_teardown() {
        // Arrange
        let runtime = runtime();
        let (server_transport, peer_transport) = MemoryFrameTransport::pair(16);
        let control = OrderedRuntimeControl::new(
            Arc::clone(&runtime),
            RuntimeControlDriver::new(control_protocol(), server_transport),
            1,
        )
        .expect("ordered control should build");
        let (publisher, senders) = publisher(4);
        let mut adapter = adapter();
        let normal_url = "https://example.test/server-error";
        let failed_url = "https://example.test/aborted";

        // Act
        let request_publication = publisher
            .publish_after_creation_fence(
                adapter
                    .map_event(&request("native-1", normal_url))
                    .expect("request should normalize"),
                runtime.as_ref(),
                &control,
            )
            .await
            .expect("request should publish after its Create");
        let response_publication = publisher
            .publish_after_creation_fence(
                adapter
                    .map_event(&response("native-1", normal_url))
                    .expect("response should normalize"),
                runtime.as_ref(),
                &control,
            )
            .await
            .expect("HTTP 500 is still a response");
        let finished_publication = publisher
            .publish_after_creation_fence(
                adapter
                    .map_event(&finished("native-1"))
                    .expect("finish should normalize"),
                runtime.as_ref(),
                &control,
            )
            .await
            .expect("HTTP 500 should finish, not fail");
        publisher
            .publish_after_creation_fence(
                adapter
                    .map_event(&request("native-2", failed_url))
                    .expect("second request should normalize"),
                runtime.as_ref(),
                &control,
            )
            .await
            .expect("second request should publish");
        let failed_publication = publisher
            .publish_after_creation_fence(
                adapter
                    .map_event(&failed("native-2"))
                    .expect("failure should normalize"),
                runtime.as_ref(),
                &control,
            )
            .await
            .expect("transport failure should publish");
        let terminal_barrier = control
            .begin_barrier()
            .await
            .expect("terminal control barrier should fit");
        terminal_barrier
            .send_through_disposal(ObjectId::new(12))
            .await
            .expect("terminal Request disposal should reach the peer");
        let live_after_terminals = runtime
            .live_object_count()
            .expect("terminal retirement should leave a readable runtime");
        let removed = runtime
            .apply_lifecycle_batch(&[ControlMessage::Dispose {
                object_id: ObjectId::new(3),
            }])
            .expect("Page teardown should retire descendants");

        // Assert
        assert_eq!(request_publication.requests, 1);
        assert_eq!(response_publication.responses, 1);
        assert_eq!(finished_publication.finished, 1);
        assert_eq!(finished_publication.retired_roots, 1);
        assert_eq!(failed_publication.failed, 1);
        assert_eq!(failed_publication.retired_roots, 1);
        assert_eq!(live_after_terminals, 3);
        assert_eq!(
            senders
                .0
                .metrics()
                .expect("request metrics")
                .queue()
                .current_len,
            2
        );
        assert_eq!(
            senders
                .1
                .metrics()
                .expect("response metrics")
                .queue()
                .current_len,
            1
        );
        assert_eq!(
            senders
                .2
                .metrics()
                .expect("finished metrics")
                .queue()
                .current_len,
            1
        );
        assert_eq!(
            senders
                .3
                .metrics()
                .expect("failed metrics")
                .queue()
                .current_len,
            1
        );
        assert_eq!(
            peer_transport
                .metrics()
                .expect("transport metrics")
                .incoming_depth(),
            6
        );
        assert_eq!(adapter.active_request_count(), 0);
        assert!(
            removed
                .iter()
                .all(|record| record.id() != ObjectId::new(10))
        );
        assert!(
            removed
                .iter()
                .all(|record| record.id() != ObjectId::new(11))
        );
        assert!(
            removed
                .iter()
                .all(|record| record.id() != ObjectId::new(12))
        );
        assert_eq!(runtime.is_object_live(ObjectId::new(10)), Ok(false));
        assert_eq!(runtime.is_object_live(ObjectId::new(11)), Ok(false));
        assert_eq!(runtime.is_object_live(ObjectId::new(12)), Ok(false));
    }

    #[tokio::test]
    async fn a_full_typed_lane_fails_closed_after_the_second_create_fence() {
        // Arrange
        let runtime = runtime();
        let (server_transport, peer_transport) = MemoryFrameTransport::pair(8);
        let control = OrderedRuntimeControl::new(
            Arc::clone(&runtime),
            RuntimeControlDriver::new(control_protocol(), server_transport),
            1,
        )
        .expect("ordered control should build");
        let (publisher, senders) = publisher(1);
        let mut adapter = adapter();
        publisher
            .publish_after_creation_fence(
                adapter
                    .map_event(&request("native-1", "https://example.test/one"))
                    .expect("first request should normalize"),
                runtime.as_ref(),
                &control,
            )
            .await
            .expect("first request should fit");

        // Act
        let overflow = publisher
            .publish_after_creation_fence(
                adapter
                    .map_event(&request("native-2", "https://example.test/two"))
                    .expect("second request should normalize"),
                runtime.as_ref(),
                &control,
            )
            .await
            .expect_err("a full lifecycle lane must not drop and continue");

        // Assert
        assert!(matches!(
            overflow,
            ChromiumNetworkEventPublishError::QueueRejected {
                effect_index: 0,
                event: pwright_cap_network_events::NETWORK_REQUEST_EVENT_TAG,
            }
        ));
        assert_eq!(
            senders
                .0
                .metrics()
                .expect("request metrics")
                .queue()
                .current_len,
            1
        );
        assert_eq!(
            peer_transport
                .metrics()
                .expect("transport metrics")
                .incoming_depth(),
            2
        );
        assert_eq!(runtime.is_object_live(ObjectId::new(11)), Ok(true));
    }

    #[tokio::test]
    async fn rejected_terminal_snapshot_does_not_retire_an_unreported_request() {
        // Arrange
        let runtime = runtime();
        let (server_transport, _peer_transport) = MemoryFrameTransport::pair(8);
        let control = OrderedRuntimeControl::new(
            Arc::clone(&runtime),
            RuntimeControlDriver::new(control_protocol(), server_transport),
            1,
        )
        .expect("ordered control should build");
        let (publisher, senders) = publisher(1);
        let mut adapter = adapter();
        let url = "https://example.test/terminal-overflow";
        publisher
            .publish_after_creation_fence(
                adapter
                    .map_event(&request("native-1", url))
                    .expect("request should normalize"),
                runtime.as_ref(),
                &control,
            )
            .await
            .expect("request should publish");
        publisher
            .publish_after_creation_fence(
                adapter
                    .map_event(&response("native-1", url))
                    .expect("response should normalize"),
                runtime.as_ref(),
                &control,
            )
            .await
            .expect("response should publish");
        let terminal = adapter
            .map_event(&finished("native-1"))
            .expect("terminal event should normalize");
        let CdpNetworkEventEffect::RequestFinished(terminal_event) = &terminal.events()[0] else {
            panic!("terminal batch should contain requestfinished");
        };
        senders
            .2
            .emit(Event::new(
                terminal_event.object_id(),
                terminal_event.payload().clone(),
            ))
            .expect("the deliberate predecessor should fill the terminal queue");

        // Act
        let error = publisher
            .publish_after_creation_fence(terminal, runtime.as_ref(), &control)
            .await
            .expect_err("terminal event loss must fail before retirement");

        // Assert
        assert!(matches!(
            error,
            ChromiumNetworkEventPublishError::QueueRejected {
                effect_index: 0,
                event: pwright_cap_network_events::NETWORK_REQUEST_FINISHED_EVENT_TAG,
            }
        ));
        assert_eq!(runtime.is_object_live(ObjectId::new(10)), Ok(true));
        assert_eq!(runtime.is_object_live(ObjectId::new(11)), Ok(true));
        assert_eq!(runtime.live_object_count(), Ok(5));
    }

    #[test]
    fn adapter_errors_remain_distinct_from_publication_errors() {
        // Arrange
        let mut adapter = adapter();
        let invalid = cdp_event(
            "Network.responseReceived",
            json!({"requestId": "missing", "response": {}}),
        );

        // Act
        let error = match adapter.map_event(&invalid) {
            Ok(_) => panic!("invalid CDP data should fail in the adapter"),
            Err(error) => error,
        };

        // Assert
        assert!(matches!(error, CdpNetworkEventError::InvalidEvent { .. }));
    }
}
