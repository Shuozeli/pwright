//! Minimal Chromium composition for Playwright-shaped Page events.
//!
//! This leaf owns three independently bounded portable event queues and one
//! selected CDP cursor for `page.on("load")`. Topology ownership and ingestion
//! remain injected: committed topology effects may be published as
//! `browser_context.on("page")` and `page.on("popup")` without making this
//! crate own a runtime, transport reader, task, browser process, or unrelated
//! capability.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

pub use pwright_backend_cdp_page_events::{
    CdpPageEventAdapter, CdpPageEventError, map_browser_context_page_effect, map_page_popup_effect,
};
use pwright_backend_cdp_topology::{CdpTopologyEffect, CdpTopologyUpdate, ProductionCdpTopology};
use pwright_cap_page_events::{BrowserContextPage, PageLoad, PagePopup};
use pwright_cdp::{CdpEventReceiver, CdpEventRecvError};
pub use pwright_codec_page_events::PageEventsCodec;
use pwright_core::{
    ControlMessage, Event, EventSpec, EventTag, ObjectId, ObjectKindTag, OverflowPolicy,
    ProductionResourceLedger, ProtocolVersion, PushOutcome, ResourceRequirements,
};
use pwright_runtime_control::{
    ControlMessageSource, OrderedControlGateError, OrderedRuntimeControl,
    OrderedRuntimeControlError,
};
use pwright_server_events::{
    ServerEventEmitError, ServerEventProductionError, ServerEventProductionProfile,
    TypedEventOutbound, TypedEventSender,
};
use pwright_server_wire::{ConcurrentServerBackend, ServerSupervisorCloseReason};
pub use pwright_transport::FrameLimits;
use pwright_transport::{FrameTransport, ProductionFrameTransport, WireCodec, WireProtocol};

const RELATED_PAGE_RETAINED_BYTES: usize = size_of::<ObjectId>();

pub type SharedChromiumPageEventTopology = Arc<Mutex<ProductionCdpTopology>>;

pub type ChromiumPageLoadOutbound<Transport> =
    TypedEventOutbound<PageLoad, PageEventsCodec, Arc<Transport>>;
pub type ChromiumBrowserContextPageOutbound<Transport> =
    TypedEventOutbound<BrowserContextPage, PageEventsCodec, Arc<Transport>>;
pub type ChromiumPagePopupOutbound<Transport> =
    TypedEventOutbound<PagePopup, PageEventsCodec, Arc<Transport>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumPageEventQueueProfile {
    capacity: usize,
    max_retained_bytes_per_event: usize,
}

impl ChromiumPageEventQueueProfile {
    pub const fn new(capacity: usize, max_retained_bytes_per_event: usize) -> Self {
        Self {
            capacity,
            max_retained_bytes_per_event,
        }
    }

    pub const fn capacity(self) -> usize {
        self.capacity
    }

    pub const fn max_retained_bytes_per_event(self) -> usize {
        self.max_retained_bytes_per_event
    }
}

/// Exact wire and queue bounds for the three events in
/// `pwright.page-events` 1.2.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumPageEventsProfile {
    wire_version: ProtocolVersion,
    send_limits: FrameLimits,
    receive_limits: FrameLimits,
    page_load: ChromiumPageEventQueueProfile,
    context_page: ChromiumPageEventQueueProfile,
    popup: ChromiumPageEventQueueProfile,
}

impl ChromiumPageEventsProfile {
    pub const fn new(
        wire_version: ProtocolVersion,
        send_limits: FrameLimits,
        receive_limits: FrameLimits,
        page_load: ChromiumPageEventQueueProfile,
        context_page: ChromiumPageEventQueueProfile,
        popup: ChromiumPageEventQueueProfile,
    ) -> Self {
        Self {
            wire_version,
            send_limits,
            receive_limits,
            page_load,
            context_page,
            popup,
        }
    }

    pub const fn wire_version(self) -> ProtocolVersion {
        self.wire_version
    }

    pub const fn send_limits(self) -> FrameLimits {
        self.send_limits
    }

    pub const fn receive_limits(self) -> FrameLimits {
        self.receive_limits
    }

    pub const fn page_load_capacity(self) -> usize {
        self.page_load.capacity()
    }

    pub const fn context_page_capacity(self) -> usize {
        self.context_page.capacity()
    }

    pub const fn popup_capacity(self) -> usize {
        self.popup.capacity()
    }

    pub const fn page_load_queue(self) -> ChromiumPageEventQueueProfile {
        self.page_load
    }

    pub const fn context_page_queue(self) -> ChromiumPageEventQueueProfile {
        self.context_page
    }

    pub const fn popup_queue(self) -> ChromiumPageEventQueueProfile {
        self.popup
    }

    pub fn requirements(self) -> ResourceRequirements {
        self.page_load_production()
            .requirements()
            .combine_concurrent(self.context_page_production().requirements())
            .combine_concurrent(self.popup_production().requirements())
    }

    fn validate(self) -> Result<Self, ChromiumPageEventsSetupError> {
        if self.page_load.capacity() == 0 {
            return Err(ChromiumPageEventsSetupError::ZeroCapacity {
                lane: ChromiumPageEventLane::PageLoad,
            });
        }
        if self.context_page.capacity() == 0 {
            return Err(ChromiumPageEventsSetupError::ZeroCapacity {
                lane: ChromiumPageEventLane::BrowserContextPage,
            });
        }
        if self.popup.capacity() == 0 {
            return Err(ChromiumPageEventsSetupError::ZeroCapacity {
                lane: ChromiumPageEventLane::PagePopup,
            });
        }
        for (lane, queue, required) in [
            (ChromiumPageEventLane::PageLoad, self.page_load, 0),
            (
                ChromiumPageEventLane::BrowserContextPage,
                self.context_page,
                RELATED_PAGE_RETAINED_BYTES,
            ),
            (
                ChromiumPageEventLane::PagePopup,
                self.popup,
                RELATED_PAGE_RETAINED_BYTES,
            ),
        ] {
            if queue.max_retained_bytes_per_event() < required {
                return Err(ChromiumPageEventsSetupError::RetainedBytesTooSmall {
                    lane,
                    required,
                    actual: queue.max_retained_bytes_per_event(),
                });
            }
        }
        Ok(self)
    }

    fn protocol(self) -> WireProtocol<PageEventsCodec> {
        WireProtocol::new_exact_directional(
            PageEventsCodec,
            self.wire_version,
            self.send_limits,
            self.receive_limits,
        )
    }

    fn page_load_production(self) -> ServerEventProductionProfile {
        ServerEventProductionProfile::new(
            &self.protocol(),
            self.page_load.capacity(),
            self.page_load.max_retained_bytes_per_event(),
        )
    }

    fn context_page_production(self) -> ServerEventProductionProfile {
        ServerEventProductionProfile::new(
            &self.protocol(),
            self.context_page.capacity(),
            self.context_page.max_retained_bytes_per_event(),
        )
    }

    fn popup_production(self) -> ServerEventProductionProfile {
        ServerEventProductionProfile::new(
            &self.protocol(),
            self.popup.capacity(),
            self.popup.max_retained_bytes_per_event(),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChromiumPageEventLane {
    PageLoad,
    BrowserContextPage,
    PagePopup,
}

impl Display for ChromiumPageEventLane {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::PageLoad => formatter.write_str("page.load"),
            Self::BrowserContextPage => formatter.write_str("browser_context.page"),
            Self::PagePopup => formatter.write_str("page.popup"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChromiumPageEventsSetupError {
    ZeroCapacity {
        lane: ChromiumPageEventLane,
    },
    RetainedBytesTooSmall {
        lane: ChromiumPageEventLane,
        required: usize,
        actual: usize,
    },
}

impl Display for ChromiumPageEventsSetupError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroCapacity { lane } => {
                write!(
                    formatter,
                    "selected Page event lane `{lane}` requires nonzero capacity"
                )
            }
            Self::RetainedBytesTooSmall {
                lane,
                required,
                actual,
            } => write!(
                formatter,
                "Page event lane `{lane}` retains at most {actual} bytes per event, but requires {required}"
            ),
        }
    }
}

impl Error for ChromiumPageEventsSetupError {}

#[derive(Debug)]
pub enum ChromiumPageEventsInstallError {
    Setup(ChromiumPageEventsSetupError),
    Production(ServerEventProductionError),
}

impl Display for ChromiumPageEventsInstallError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Setup(error) => Display::fmt(error, formatter),
            Self::Production(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for ChromiumPageEventsInstallError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Setup(error) => Some(error),
            Self::Production(error) => Some(error),
        }
    }
}

impl From<ChromiumPageEventsSetupError> for ChromiumPageEventsInstallError {
    fn from(error: ChromiumPageEventsSetupError) -> Self {
        Self::Setup(error)
    }
}

impl From<ServerEventProductionError> for ChromiumPageEventsInstallError {
    fn from(error: ServerEventProductionError) -> Self {
        Self::Production(error)
    }
}

/// Explicit, task-free bridge from committed Chromium topology effects to the
/// two portable Page discovery event lanes.
#[derive(Clone, Default)]
pub struct ChromiumTopologyPageEventPublisher {
    context_pages: Option<TypedEventSender<BrowserContextPage>>,
    popups: Option<TypedEventSender<PagePopup>>,
}

impl ChromiumTopologyPageEventPublisher {
    pub const fn new(
        context_pages: Option<TypedEventSender<BrowserContextPage>>,
        popups: Option<TypedEventSender<PagePopup>>,
    ) -> Self {
        Self {
            context_pages,
            popups,
        }
    }

    pub const fn is_inert(&self) -> bool {
        self.context_pages.is_none() && self.popups.is_none()
    }

    pub fn publish(
        &self,
        update: &CdpTopologyUpdate,
    ) -> Result<ChromiumTopologyPageEventPublication, ChromiumTopologyPageEventPublishError> {
        self.publish_effects(update.effects())
    }

    pub fn publish_effects(
        &self,
        effects: &[CdpTopologyEffect],
    ) -> Result<ChromiumTopologyPageEventPublication, ChromiumTopologyPageEventPublishError> {
        let mut publication = ChromiumTopologyPageEventPublication::default();
        for (effect_index, effect) in effects.iter().enumerate() {
            if let Some(event) = map_browser_context_page_effect(effect) {
                let Some(sender) = self.context_pages.as_ref() else {
                    publication.skipped_uninstalled =
                        publication.skipped_uninstalled.saturating_add(1);
                    continue;
                };
                emit_topology_page_event(sender, event, effect_index)?;
                publication.context_pages = publication.context_pages.saturating_add(1);
                continue;
            }
            if let Some(event) = map_page_popup_effect(effect) {
                let Some(sender) = self.popups.as_ref() else {
                    publication.skipped_uninstalled =
                        publication.skipped_uninstalled.saturating_add(1);
                    continue;
                };
                emit_topology_page_event(sender, event, effect_index)?;
                publication.popups = publication.popups.saturating_add(1);
            }
        }
        Ok(publication)
    }

    /// Sends lifecycle controls through the final created object before any
    /// discovery event referencing that object enters an outbound queue.
    pub async fn publish_after_creation_fence<Source, Codec, Transport>(
        &self,
        update: &CdpTopologyUpdate,
        control: &OrderedRuntimeControl<Source, Codec, Transport>,
    ) -> Result<
        ChromiumTopologyPageEventPublication,
        ChromiumTopologyPageEventFenceError<Codec::Error, Transport::Error>,
    >
    where
        Source: ControlMessageSource,
        Codec: WireCodec<ControlMessage>,
        Transport: FrameTransport,
    {
        if let Some((object_id, parent, kind)) = required_topology_creation_fence(update.effects())
        {
            let barrier = control
                .begin_barrier()
                .await
                .map_err(ChromiumTopologyPageEventFenceError::Gate)?;
            barrier
                .send_through_creation(object_id, parent, kind)
                .await
                .map_err(ChromiumTopologyPageEventFenceError::Control)?;
        }
        self.publish(update)
            .map_err(ChromiumTopologyPageEventFenceError::Publish)
    }
}

fn required_topology_creation_fence(
    effects: &[CdpTopologyEffect],
) -> Option<(ObjectId, Option<ObjectId>, ObjectKindTag)> {
    effects.iter().rev().find_map(|effect| match effect {
        CdpTopologyEffect::Lifecycle(ControlMessage::Create {
            object_id,
            parent,
            kind,
        }) => Some((*object_id, *parent, *kind)),
        _ => None,
    })
}

fn emit_topology_page_event<S>(
    sender: &TypedEventSender<S>,
    event: Event<S>,
    effect_index: usize,
) -> Result<(), ChromiumTopologyPageEventPublishError>
where
    S: EventSpec,
    S::Payload: Send,
{
    match sender.emit(event) {
        Ok(PushOutcome::Enqueued) => Ok(()),
        Ok(PushOutcome::Rejected { .. }) => {
            Err(ChromiumTopologyPageEventPublishError::QueueRejected {
                effect_index,
                event: S::EVENT,
            })
        }
        Ok(PushOutcome::DroppedOldest { .. }) => {
            Err(ChromiumTopologyPageEventPublishError::QueueEvicted {
                effect_index,
                event: S::EVENT,
            })
        }
        Err(source) => Err(ChromiumTopologyPageEventPublishError::Emit {
            effect_index,
            event: S::EVENT,
            source,
        }),
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChromiumTopologyPageEventPublication {
    pub context_pages: usize,
    pub popups: usize,
    pub skipped_uninstalled: usize,
}

#[derive(Debug)]
pub enum ChromiumTopologyPageEventPublishError {
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

impl ChromiumTopologyPageEventPublishError {
    pub const fn effect_index(&self) -> usize {
        match self {
            Self::QueueRejected { effect_index, .. }
            | Self::QueueEvicted { effect_index, .. }
            | Self::Emit { effect_index, .. } => *effect_index,
        }
    }

    pub const fn event(&self) -> EventTag {
        match self {
            Self::QueueRejected { event, .. }
            | Self::QueueEvicted { event, .. }
            | Self::Emit { event, .. } => *event,
        }
    }
}

impl Display for ChromiumTopologyPageEventPublishError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueueRejected {
                effect_index,
                event,
            } => write!(
                formatter,
                "topology effect {effect_index} event `{event}` was rejected; event resynchronization is required"
            ),
            Self::QueueEvicted {
                effect_index,
                event,
            } => write!(
                formatter,
                "topology effect {effect_index} event `{event}` evicted an older event; event resynchronization is required"
            ),
            Self::Emit {
                effect_index,
                event,
                source,
            } => write!(
                formatter,
                "topology effect {effect_index} event `{event}` could not be retained: {source}"
            ),
        }
    }
}

impl Error for ChromiumTopologyPageEventPublishError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Emit { source, .. } => Some(source),
            Self::QueueRejected { .. } | Self::QueueEvicted { .. } => None,
        }
    }
}

#[derive(Debug)]
pub enum ChromiumTopologyPageEventFenceError<CodecError, TransportError> {
    Gate(OrderedControlGateError),
    Control(OrderedRuntimeControlError<CodecError, TransportError>),
    Publish(ChromiumTopologyPageEventPublishError),
}

impl<CodecError, TransportError> Display
    for ChromiumTopologyPageEventFenceError<CodecError, TransportError>
where
    CodecError: Display,
    TransportError: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gate(error) => Display::fmt(error, formatter),
            Self::Control(error) => Display::fmt(error, formatter),
            Self::Publish(error) => Display::fmt(error, formatter),
        }
    }
}

impl<CodecError, TransportError> Error
    for ChromiumTopologyPageEventFenceError<CodecError, TransportError>
where
    CodecError: Error + 'static,
    TransportError: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Gate(error) => Some(error),
            Self::Control(error) => Some(error),
            Self::Publish(error) => Some(error),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChromiumPageLoadServerBackendOutcome {
    Ignored,
    Published { page: ObjectId },
}

#[derive(Debug)]
pub enum ChromiumPageLoadServerBackendError {
    ConnectionTerminated {
        reason: ServerSupervisorCloseReason,
    },
    EventStreamClosed,
    EventLossRequiresReconnect {
        skipped: u64,
    },
    TopologyStateUnavailable,
    Adapter(CdpPageEventError),
    QueueRejected {
        page: ObjectId,
    },
    QueueEvicted {
        page: ObjectId,
    },
    Emit {
        page: ObjectId,
        source: ServerEventEmitError,
    },
}

impl Display for ChromiumPageLoadServerBackendError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionTerminated { reason } => {
                write!(
                    formatter,
                    "PageLoad backend connection terminated: {reason:?}"
                )
            }
            Self::EventStreamClosed => formatter.write_str("CDP PageLoad event stream closed"),
            Self::EventLossRequiresReconnect { skipped } => write!(
                formatter,
                "CDP PageLoad stream lost {skipped} event(s); a new remote connection is required"
            ),
            Self::TopologyStateUnavailable => {
                formatter.write_str("Chromium topology state is unavailable for PageLoad")
            }
            Self::Adapter(error) => Display::fmt(error, formatter),
            Self::QueueRejected { page } => write!(
                formatter,
                "PageLoad for Page `{page}` was rejected; event resynchronization is required"
            ),
            Self::QueueEvicted { page } => write!(
                formatter,
                "PageLoad for Page `{page}` evicted an older event; event resynchronization is required"
            ),
            Self::Emit { page, source } => write!(
                formatter,
                "PageLoad for Page `{page}` could not be retained: {source}"
            ),
        }
    }
}

impl Error for ChromiumPageLoadServerBackendError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Adapter(error) => Some(error),
            Self::Emit { source, .. } => Some(source),
            Self::ConnectionTerminated { .. }
            | Self::EventStreamClosed
            | Self::EventLossRequiresReconnect { .. }
            | Self::TopologyStateUnavailable
            | Self::QueueRejected { .. }
            | Self::QueueEvicted { .. } => None,
        }
    }
}

type BoxedChromiumPageLoadDrive = Pin<
    Box<
        dyn Future<
                Output = (
                    CdpEventReceiver,
                    Result<
                        ChromiumPageLoadServerBackendOutcome,
                        ChromiumPageLoadServerBackendError,
                    >,
                ),
            > + Send
            + 'static,
    >,
>;

/// Independent caller-polled `PageLoad` lane for one Chromium connection.
pub struct ChromiumPageLoadServerBackend {
    receiver: Option<CdpEventReceiver>,
    adapter: CdpPageEventAdapter,
    topology: SharedChromiumPageEventTopology,
    sender: TypedEventSender<PageLoad>,
    in_flight: Option<BoxedChromiumPageLoadDrive>,
    connection_termination: Option<ServerSupervisorCloseReason>,
}

impl ChromiumPageLoadServerBackend {
    pub fn new(
        receiver: impl Into<CdpEventReceiver>,
        adapter: CdpPageEventAdapter,
        topology: SharedChromiumPageEventTopology,
        sender: TypedEventSender<PageLoad>,
    ) -> Self {
        Self {
            receiver: Some(receiver.into()),
            adapter,
            topology,
            sender,
            in_flight: None,
            connection_termination: None,
        }
    }

    pub const fn is_driving(&self) -> bool {
        self.in_flight.is_some()
    }

    pub const fn connection_termination(&self) -> Option<ServerSupervisorCloseReason> {
        self.connection_termination
    }
}

impl ConcurrentServerBackend for ChromiumPageLoadServerBackend {
    type Event = ChromiumPageLoadServerBackendOutcome;
    type Error = ChromiumPageLoadServerBackendError;

    fn poll_drive_once(
        &mut self,
        context: &mut Context<'_>,
    ) -> Poll<Result<Self::Event, Self::Error>> {
        if let Some(reason) = self.connection_termination {
            return Poll::Ready(Err(
                ChromiumPageLoadServerBackendError::ConnectionTerminated { reason },
            ));
        }
        if self.in_flight.is_none() {
            let mut receiver = self
                .receiver
                .take()
                .expect("the PageLoad receiver is absent only while its future is retained");
            let adapter = self.adapter.clone();
            let topology = Arc::clone(&self.topology);
            let sender = self.sender.clone();
            self.in_flight = Some(Box::pin(async move {
                let result = match receiver.recv().await {
                    Ok(event) => {
                        let mapped = topology
                            .lock()
                            .map_err(|_| {
                                ChromiumPageLoadServerBackendError::TopologyStateUnavailable
                            })
                            .and_then(|topology| {
                                adapter
                                    .map_page_load_event(topology.topology(), &event)
                                    .map_err(ChromiumPageLoadServerBackendError::Adapter)
                            });
                        match mapped {
                            Ok(None) => Ok(ChromiumPageLoadServerBackendOutcome::Ignored),
                            Ok(Some(event)) => {
                                let page = event.object_id();
                                match sender.emit(event) {
                                    Ok(PushOutcome::Enqueued) => {
                                        Ok(ChromiumPageLoadServerBackendOutcome::Published { page })
                                    }
                                    Ok(PushOutcome::Rejected { .. }) => {
                                        Err(ChromiumPageLoadServerBackendError::QueueRejected {
                                            page,
                                        })
                                    }
                                    Ok(PushOutcome::DroppedOldest { .. }) => {
                                        Err(ChromiumPageLoadServerBackendError::QueueEvicted {
                                            page,
                                        })
                                    }
                                    Err(source) => Err(ChromiumPageLoadServerBackendError::Emit {
                                        page,
                                        source,
                                    }),
                                }
                            }
                            Err(error) => Err(error),
                        }
                    }
                    Err(CdpEventRecvError::Closed) => {
                        Err(ChromiumPageLoadServerBackendError::EventStreamClosed)
                    }
                    Err(CdpEventRecvError::Lagged(skipped)) => Err(
                        ChromiumPageLoadServerBackendError::EventLossRequiresReconnect { skipped },
                    ),
                };
                (receiver, result)
            }));
        }

        let output = {
            let drive = self
                .in_flight
                .as_mut()
                .expect("a PageLoad drive must exist after initialization");
            match drive.as_mut().poll(context) {
                Poll::Ready(output) => output,
                Poll::Pending => return Poll::Pending,
            }
        };
        self.in_flight = None;
        self.receiver = Some(output.0);
        Poll::Ready(output.1)
    }

    fn handle_connection_terminated(&mut self, reason: ServerSupervisorCloseReason) {
        if self.connection_termination.is_none() {
            self.in_flight = None;
            self.receiver = None;
            self.connection_termination = Some(reason);
        }
    }
}

/// Fully bound queue/cursor installation. It remains inert until the caller
/// polls the returned backend and outbound lanes.
pub struct ChromiumPageEventsInstallation<Transport: ?Sized> {
    adapter: CdpPageEventAdapter,
    publisher: ChromiumTopologyPageEventPublisher,
    load_backend: ChromiumPageLoadServerBackend,
    load_outbound: ChromiumPageLoadOutbound<Transport>,
    context_page_outbound: ChromiumBrowserContextPageOutbound<Transport>,
    popup_outbound: ChromiumPagePopupOutbound<Transport>,
}

impl<Transport> ChromiumPageEventsInstallation<Transport>
where
    Transport: ProductionFrameTransport + ?Sized,
{
    pub fn install(
        receiver: impl Into<CdpEventReceiver>,
        adapter: CdpPageEventAdapter,
        topology: SharedChromiumPageEventTopology,
        transport: Arc<Transport>,
        profile: ChromiumPageEventsProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ChromiumPageEventsInstallError> {
        let profile = profile.validate()?;
        let (load_sender, load_outbound) =
            TypedEventOutbound::<PageLoad, _, _>::new_with_production_resources(
                profile.protocol(),
                Arc::clone(&transport),
                profile.page_load_production(),
                OverflowPolicy::RejectNewest,
                resources,
                no_retained_event_bytes,
            )?;
        let (context_page_sender, context_page_outbound) =
            TypedEventOutbound::<BrowserContextPage, _, _>::new_with_production_resources(
                profile.protocol(),
                Arc::clone(&transport),
                profile.context_page_production(),
                OverflowPolicy::RejectNewest,
                resources,
                related_page_retained_bytes,
            )?;
        let (popup_sender, popup_outbound) =
            TypedEventOutbound::<PagePopup, _, _>::new_with_production_resources(
                profile.protocol(),
                transport,
                profile.popup_production(),
                OverflowPolicy::RejectNewest,
                resources,
                related_popup_retained_bytes,
            )?;
        let publisher =
            ChromiumTopologyPageEventPublisher::new(Some(context_page_sender), Some(popup_sender));
        let load_backend =
            ChromiumPageLoadServerBackend::new(receiver, adapter.clone(), topology, load_sender);
        Ok(Self {
            adapter,
            publisher,
            load_backend,
            load_outbound,
            context_page_outbound,
            popup_outbound,
        })
    }

    pub fn adapter(&self) -> &CdpPageEventAdapter {
        &self.adapter
    }

    pub fn publisher(&self) -> ChromiumTopologyPageEventPublisher {
        self.publisher.clone()
    }

    pub fn into_parts(
        self,
    ) -> (
        ChromiumTopologyPageEventPublisher,
        ChromiumPageLoadServerBackend,
        ChromiumPageLoadOutbound<Transport>,
        ChromiumBrowserContextPageOutbound<Transport>,
        ChromiumPagePopupOutbound<Transport>,
    ) {
        (
            self.publisher,
            self.load_backend,
            self.load_outbound,
            self.context_page_outbound,
            self.popup_outbound,
        )
    }
}

fn no_retained_event_bytes(_event: &Event<PageLoad>) -> usize {
    0
}

fn related_page_retained_bytes(_event: &Event<BrowserContextPage>) -> usize {
    RELATED_PAGE_RETAINED_BYTES
}

fn related_popup_retained_bytes(_event: &Event<PagePopup>) -> usize {
    RELATED_PAGE_RETAINED_BYTES
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fmt::{self, Display, Formatter};
    use std::future::{Ready, ready};
    use std::sync::{Arc, Mutex};

    use pwright_backend_cdp_topology::{
        CdpContextOwnership, CdpFrameAttachment, CdpPageObjectIds, CdpPageTarget,
        CdpTopologyLimits, CdpTopologyProductionProfile, ConnectionEpoch, ProductionCdpTopology,
    };
    use pwright_cap_page_events::{BrowserContextPage, PageLoad, PagePopup};
    use pwright_cdp::{CdpClient, CdpEvent, SessionFactory};
    use pwright_core::{
        ObjectId, ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceRequirements,
    };
    use pwright_fake::FakeCdpClient;
    use pwright_server_wire::{ConcurrentServerBackend, ConcurrentServerOutbound};
    use pwright_transport::{
        FrameTransport, ProductionFrameTransport, ProductionTransportResources, WireFrame,
    };

    use super::{
        ChromiumPageEventLane, ChromiumPageEventQueueProfile, ChromiumPageEventsInstallation,
        ChromiumPageEventsProfile, ChromiumPageEventsSetupError,
        ChromiumPageLoadServerBackendOutcome,
    };

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct TestTransportError;

    impl Display for TestTransportError {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            formatter.write_str("test transport failed")
        }
    }

    impl Error for TestTransportError {}

    struct TestTransport {
        sent: Arc<Mutex<Vec<WireFrame>>>,
        ledger: pwright_core::ResourceLedger,
    }

    impl FrameTransport for TestTransport {
        type Error = TestTransportError;
        type Send<'a>
            = Ready<Result<(), Self::Error>>
        where
            Self: 'a;
        type Receive<'a>
            = Ready<Result<Option<WireFrame>, Self::Error>>
        where
            Self: 'a;

        fn send(&self, frame: WireFrame) -> Self::Send<'_> {
            self.sent
                .lock()
                .expect("sent-frame state should be available")
                .push(frame);
            ready(Ok(()))
        }

        fn receive(&self) -> Self::Receive<'_> {
            ready(Ok(None))
        }
    }

    impl ProductionFrameTransport for TestTransport {
        fn production_resources(&self) -> ProductionTransportResources<'_> {
            ProductionTransportResources::new(&self.ledger, ResourceRequirements::none())
        }
    }

    struct SharedFakeFactory {
        client: Arc<FakeCdpClient>,
    }

    impl SessionFactory for SharedFakeFactory {
        fn create_session(&self, _session_id: String, _target_id: String) -> Arc<dyn CdpClient> {
            self.client.clone()
        }
    }

    fn profile() -> ChromiumPageEventsProfile {
        ChromiumPageEventsProfile::new(
            pwright_core::ProtocolVersion::new(1, 0),
            super::FrameLimits::new(64, 0, 0, 64),
            super::FrameLimits::new(64, 0, 0, 64),
            ChromiumPageEventQueueProfile::new(2, 0),
            ChromiumPageEventQueueProfile::new(2, super::RELATED_PAGE_RETAINED_BYTES),
            ChromiumPageEventQueueProfile::new(2, super::RELATED_PAGE_RETAINED_BYTES),
        )
    }

    fn resources(profile: ChromiumPageEventsProfile) -> ProductionResourceLedger {
        let requirements = profile.requirements();
        let events = requirements.class(ResourceClass::EventItem);
        let event_bytes = requirements.class(ResourceClass::EventBytes);
        let wire = requirements.class(ResourceClass::WireBytes);
        let limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases)
            .with_class(ResourceClass::EventItem, events.global, events.per_scope)
            .with_class(
                ResourceClass::EventBytes,
                event_bytes.global,
                event_bytes.per_scope,
            )
            .with_class(ResourceClass::WireBytes, wire.global, wire.per_scope);
        ProductionResourceLedger::new(limits, requirements)
            .expect("exact Page-event resources should build")
    }

    fn topology() -> Arc<Mutex<ProductionCdpTopology>> {
        let profile = CdpTopologyProductionProfile::new(
            CdpTopologyLimits::new(16, 4, 4, 8, 8, 64),
            pwright_core::ResourceScope::extension(71, 1),
        )
        .expect("topology profile should validate");
        let requirements = profile.requirements();
        let live_objects = requirements.class(ResourceClass::LiveObject);
        let backend = requirements.class(ResourceClass::BackendStateBytes);
        let limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases)
            .with_class(
                ResourceClass::LiveObject,
                live_objects.global,
                live_objects.per_scope,
            )
            .with_class(
                ResourceClass::BackendStateBytes,
                backend.global,
                backend.per_scope,
            );
        let resources = ProductionResourceLedger::new(limits, requirements)
            .expect("exact topology resources should build");
        let mut topology = ProductionCdpTopology::new(
            ObjectId::new(1),
            ConnectionEpoch::new(1),
            profile,
            &resources,
        )
        .expect("topology should build");
        topology
            .register_context(
                ConnectionEpoch::new(1),
                "context-1",
                ObjectId::new(2),
                CdpContextOwnership::Owned,
            )
            .expect("context should register");
        topology
            .attach_page(
                ConnectionEpoch::new(1),
                CdpPageTarget::new("target-1", "session-1", "context-1"),
                CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
            )
            .expect("page should attach");
        topology
            .attach_frame(
                ConnectionEpoch::new(1),
                CdpFrameAttachment::main("target-1", "frame-1"),
                ObjectId::new(4),
            )
            .expect("main frame should bind");
        Arc::new(Mutex::new(topology))
    }

    #[test]
    fn every_selected_event_lane_requires_bounded_capacity() {
        // Arrange
        let profile = ChromiumPageEventsProfile::new(
            pwright_core::ProtocolVersion::new(1, 0),
            super::FrameLimits::new(64, 0, 0, 64),
            super::FrameLimits::new(64, 0, 0, 64),
            ChromiumPageEventQueueProfile::new(1, 0),
            ChromiumPageEventQueueProfile::new(0, super::RELATED_PAGE_RETAINED_BYTES),
            ChromiumPageEventQueueProfile::new(1, super::RELATED_PAGE_RETAINED_BYTES),
        );

        // Act
        let error = profile
            .validate()
            .expect_err("a selected zero-capacity lane should fail");

        // Assert
        assert_eq!(
            error,
            ChromiumPageEventsSetupError::ZeroCapacity {
                lane: ChromiumPageEventLane::BrowserContextPage,
            }
        );
    }

    #[test]
    fn related_page_events_require_their_exact_retained_byte_bound() {
        // Arrange
        let actual = super::RELATED_PAGE_RETAINED_BYTES - 1;
        let profile = ChromiumPageEventsProfile::new(
            pwright_core::ProtocolVersion::new(1, 0),
            super::FrameLimits::new(64, 0, 0, 64),
            super::FrameLimits::new(64, 0, 0, 64),
            ChromiumPageEventQueueProfile::new(1, 0),
            ChromiumPageEventQueueProfile::new(1, actual),
            ChromiumPageEventQueueProfile::new(1, super::RELATED_PAGE_RETAINED_BYTES),
        );

        // Act
        let error = profile
            .validate()
            .expect_err("an under-budgeted related Page event should fail");

        // Assert
        assert_eq!(
            error,
            ChromiumPageEventsSetupError::RetainedBytesTooSmall {
                lane: ChromiumPageEventLane::BrowserContextPage,
                required: super::RELATED_PAGE_RETAINED_BYTES,
                actual,
            }
        );
    }

    #[tokio::test]
    async fn installation_publishes_all_three_events_through_independent_lanes() {
        // Arrange
        let selected_profile = profile();
        let resources = resources(selected_profile);
        let sent = Arc::new(Mutex::new(Vec::new()));
        let transport = Arc::new(TestTransport {
            sent: Arc::clone(&sent),
            ledger: resources
                .claim(ResourceRequirements::none())
                .expect("test transport should share the Page-event ledger"),
        });
        let topology = topology();
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let factory: Arc<dyn SessionFactory> = Arc::new(SharedFakeFactory { client: fake });
        let adapter = super::CdpPageEventAdapter::new(factory);
        let (native_sender, native_receiver) = tokio::sync::broadcast::channel(4);
        let installation = ChromiumPageEventsInstallation::install(
            native_receiver,
            adapter,
            Arc::clone(&topology),
            transport,
            selected_profile,
            &resources,
        )
        .expect("Page-event leaf should install");
        let (publisher, mut load_backend, load_outbound, context_outbound, popup_outbound) =
            installation.into_parts();
        native_sender
            .send(CdpEvent::new(
                "Page.lifecycleEvent".to_owned(),
                serde_json::json!({"name":"load","frameId":"frame-1"}),
                Some("session-1".to_owned()),
            ))
            .expect("load event should enqueue");
        let effects = [
            pwright_backend_cdp_topology::CdpTopologyEffect::PageReady {
                page: ObjectId::new(3),
                context: ObjectId::new(2),
            },
            pwright_backend_cdp_topology::CdpTopologyEffect::PopupLinked {
                opener: ObjectId::new(3),
                popup: ObjectId::new(5),
            },
        ];

        // Act
        let load = std::future::poll_fn(|context| load_backend.poll_drive_once(context))
            .await
            .expect("PageLoad should publish");
        let discovery = publisher
            .publish_effects(&effects)
            .expect("discovery events should publish");
        load_outbound
            .send_once()
            .await
            .expect("load should send")
            .expect("load should be queued");
        context_outbound
            .send_once()
            .await
            .expect("context.page should send")
            .expect("context.page should be queued");
        popup_outbound
            .send_once()
            .await
            .expect("popup should send")
            .expect("popup should be queued");

        // Assert
        assert_eq!(
            load,
            ChromiumPageLoadServerBackendOutcome::Published {
                page: ObjectId::new(3),
            }
        );
        assert_eq!(discovery.context_pages, 1);
        assert_eq!(discovery.popups, 1);
        let frames = sent.lock().expect("sent frames should be available");
        assert_eq!(frames.len(), 3);
        let load_event = selected_profile
            .protocol()
            .decode::<pwright_core::SequencedEvent<PageLoad>>(frames[0].clone())
            .expect("load frame should decode");
        let context_event = selected_profile
            .protocol()
            .decode::<pwright_core::SequencedEvent<BrowserContextPage>>(frames[1].clone())
            .expect("context.page frame should decode");
        let popup_event = selected_profile
            .protocol()
            .decode::<pwright_core::SequencedEvent<PagePopup>>(frames[2].clone())
            .expect("popup frame should decode");
        assert_eq!(load_event.object_id(), ObjectId::new(3));
        assert_eq!(context_event.payload().page, ObjectId::new(3));
        assert_eq!(popup_event.payload().page, ObjectId::new(5));
    }
}
