use std::convert::Infallible;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use pwright_cap_browser_events::BrowserDisconnected;
use pwright_cap_browser_lifecycle::{
    BROWSER_LIFECYCLE_CAPABILITY_ID, BrowserClose, BrowserLifecycleErrorDetails,
};
use pwright_core::{
    Command, CommandResult, FailureKind, ObjectId, OperationError, SequencedEvent, Service,
};
use pwright_provider_chromium::{
    ChromiumShutdownPolicy, ChromiumShutdownReport, TokioChromiumChild,
};
use pwright_runtime::{
    CleanupFailure, CleanupReservation, CleanupSubmitError, ControlledLifecycleHandle,
    ReservedCleanupExecutor, RuntimeCallFailure, RuntimeStateError,
};
use pwright_runtime_control::{ControlMessageSource, OrderedRuntimeControl, RuntimeControlDriver};
use pwright_server_browser_events::BrowserDisconnectedPublisher;
use pwright_server_browser_lifecycle::{
    BrowserLifecycleServerHandlers, BrowserLifecycleServerServices,
    browser_lifecycle_server_handlers,
};
use pwright_server_wire::AcceptedServerHandshake;
use pwright_transport::{FrameTransport, WireCapabilityId, WireCodec, WireProtocol};

use crate::{OwnedChromiumComposition, OwnedChromiumShutdownError};

/// A non-cloneable, session-authorized transfer of one owned process.
///
/// The only production constructor consumes an [`OwnedChromiumComposition`]
/// after confirming that this exact negotiated session selected
/// `pwright.browser_lifecycle`.
pub struct OwnedBrowserCloseGrant<Authority> {
    browser_object_id: ObjectId,
    authority: Authority,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnedBrowserCloseGrantError {
    CapabilityNotNegotiated,
    SessionResourcesUnavailable,
    SessionResourceBudgetMismatch,
}

impl Display for OwnedBrowserCloseGrantError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapabilityNotNegotiated => formatter
                .write_str("the session did not negotiate owned Browser lifecycle authority"),
            Self::SessionResourcesUnavailable => formatter.write_str(
                "the session was not established by a production resource-bound supervisor",
            ),
            Self::SessionResourceBudgetMismatch => formatter.write_str(
                "the session belongs to a different production connection resource budget",
            ),
        }
    }
}

impl Error for OwnedBrowserCloseGrantError {}

/// A failed grant conversion returns the untouched process owner so a
/// handshake mismatch can never orphan the launched browser.
pub struct OwnedBrowserCloseGrantRejection<Authority> {
    error: OwnedBrowserCloseGrantError,
    authority: Box<Authority>,
}

impl<Authority> OwnedBrowserCloseGrantRejection<Authority> {
    pub const fn error(&self) -> OwnedBrowserCloseGrantError {
        self.error
    }

    pub const fn authority(&self) -> &Authority {
        &self.authority
    }

    pub fn into_parts(self) -> (OwnedBrowserCloseGrantError, Authority) {
        (self.error, *self.authority)
    }
}

impl<Authority> fmt::Debug for OwnedBrowserCloseGrantRejection<Authority> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OwnedBrowserCloseGrantRejection")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl<Authority> Display for OwnedBrowserCloseGrantRejection<Authority> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.error, formatter)
    }
}

impl<Authority> Error for OwnedBrowserCloseGrantRejection<Authority> where Authority: 'static {}

impl OwnedChromiumComposition<TokioChromiumChild> {
    pub fn into_browser_close_grant<Authorization>(
        self,
        session: &AcceptedServerHandshake<Authorization>,
    ) -> Result<OwnedBrowserCloseGrant<Self>, OwnedBrowserCloseGrantRejection<Self>> {
        if self.browser_lifecycle.is_none() {
            return Err(OwnedBrowserCloseGrantRejection {
                error: OwnedBrowserCloseGrantError::CapabilityNotNegotiated,
                authority: Box::new(self),
            });
        }
        let capability = WireCapabilityId::new(
            BROWSER_LIFECYCLE_CAPABILITY_ID.as_str(),
            BROWSER_LIFECYCLE_CAPABILITY_ID.as_str().len(),
        )
        .expect("the static Browser lifecycle capability ID is valid");
        if session
            .hello()
            .enabled_capability_version(&capability)
            .is_none()
        {
            return Err(OwnedBrowserCloseGrantRejection {
                error: OwnedBrowserCloseGrantError::CapabilityNotNegotiated,
                authority: Box::new(self),
            });
        }
        if !session.shares_production_resource_budget_with(self.composition().resources()) {
            let error = if session.has_production_resource_budget() {
                OwnedBrowserCloseGrantError::SessionResourceBudgetMismatch
            } else {
                OwnedBrowserCloseGrantError::SessionResourcesUnavailable
            };
            return Err(OwnedBrowserCloseGrantRejection {
                error,
                authority: Box::new(self),
            });
        }
        let browser_object_id = self.composition().browser_object_id();
        Ok(OwnedBrowserCloseGrant {
            browser_object_id,
            authority: self,
        })
    }
}

/// Consuming termination seam used to test command/lifecycle behavior without
/// launching a process. Production implements it only for the owned launch
/// composition.
pub trait OwnedBrowserTermination: Send + 'static {
    type Report: Send + 'static;
    type Error: Error + Send + Sync + 'static;
    type Future: Future<Output = Result<Self::Report, Self::Error>> + Send + 'static;

    fn terminate(self, policy: ChromiumShutdownPolicy) -> Self::Future;
}

/// Optional composition-layer destination for provider-specific shutdown
/// diagnostics. Portable `Browser.close` still returns unit; only an assembly
/// root that selects a concrete owned provider sees this report type. Recording
/// happens after runtime disposal commits and is synchronous; sinks that need
/// asynchronous I/O should perform only bounded nonblocking admission here.
pub trait OwnedBrowserShutdownReportSink<Report>: Send + Sync + 'static {
    fn record(&self, report: Report);
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DiscardOwnedBrowserShutdownReport;

impl<Report> OwnedBrowserShutdownReportSink<Report> for DiscardOwnedBrowserShutdownReport {
    fn record(&self, _report: Report) {}
}

impl<Report, Sink> OwnedBrowserShutdownReportSink<Report> for Sink
where
    Sink: Fn(Report) + Send + Sync + 'static,
{
    fn record(&self, report: Report) {
        self(report);
    }
}

impl OwnedBrowserTermination for OwnedChromiumComposition<TokioChromiumChild> {
    type Report = ChromiumShutdownReport;
    type Error = OwnedChromiumShutdownError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Report, Self::Error>> + Send + 'static>>;

    fn terminate(mut self, policy: ChromiumShutdownPolicy) -> Self::Future {
        Box::pin(async move { self.shutdown(policy).await })
    }
}

/// The sole typed command service that can consume an owned-browser grant.
pub struct OwnedBrowserCloseCommandService<
    Authority,
    ReportSink = DiscardOwnedBrowserShutdownReport,
> {
    grant: Mutex<Option<OwnedBrowserCloseGrant<Authority>>>,
    lifecycle: ControlledLifecycleHandle,
    policy: ChromiumShutdownPolicy,
    report_sink: ReportSink,
}

impl<Authority> OwnedBrowserCloseCommandService<Authority> {
    pub const fn new(
        grant: OwnedBrowserCloseGrant<Authority>,
        lifecycle: ControlledLifecycleHandle,
        policy: ChromiumShutdownPolicy,
    ) -> Self {
        Self {
            grant: Mutex::new(Some(grant)),
            lifecycle,
            policy,
            report_sink: DiscardOwnedBrowserShutdownReport,
        }
    }
}

impl<Authority, ReportSink> OwnedBrowserCloseCommandService<Authority, ReportSink> {
    pub const fn with_report_sink(
        grant: OwnedBrowserCloseGrant<Authority>,
        lifecycle: ControlledLifecycleHandle,
        policy: ChromiumShutdownPolicy,
        report_sink: ReportSink,
    ) -> Self {
        Self {
            grant: Mutex::new(Some(grant)),
            lifecycle,
            policy,
            report_sink,
        }
    }
}

impl<Authority, ReportSink> Service<Command<BrowserClose>>
    for OwnedBrowserCloseCommandService<Authority, ReportSink>
where
    Authority: OwnedBrowserTermination,
    ReportSink: OwnedBrowserShutdownReportSink<Authority::Report>,
{
    type Response = CommandResult<BrowserClose>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<BrowserClose>) -> Self::Future<'_> {
        Box::pin(async move {
            let (call_id, target, _metadata, options) = command.into_parts();
            let close_reason = options.into_reason().filter(|reason| !reason.is_empty());
            let expected = match self.grant.lock() {
                Ok(grant) => grant.as_ref().map(|grant| grant.browser_object_id),
                Err(_) => {
                    return Ok(browser_close_failure(
                        call_id,
                        FailureKind::Cleanup,
                        BrowserLifecycleErrorDetails::Cleanup(
                            "owned Browser close grant state is unavailable".to_string(),
                        ),
                    ));
                }
            };
            let Some(expected) = expected else {
                return Ok(browser_close_failure(
                    call_id,
                    FailureKind::ResourceExhausted,
                    BrowserLifecycleErrorDetails::NotAuthorized(
                        "owned Browser close authority is already claimed".to_string(),
                    ),
                ));
            };
            if target != expected {
                return Ok(browser_close_failure(
                    call_id,
                    FailureKind::UnsupportedCapability,
                    BrowserLifecycleErrorDetails::NotAuthorized(format!(
                        "owned Browser close grant targets `{expected}`, not `{target}`"
                    )),
                ));
            }

            let prepared = match self.lifecycle.prepare_object_disposal(target) {
                Ok(prepared) => prepared,
                Err(error) => return Ok(map_runtime_failure(call_id, error)),
            };
            let grant = match self.grant.lock() {
                Ok(mut grant) => grant.take(),
                Err(_) => None,
            };
            let Some(grant) = grant else {
                return Ok(browser_close_failure(
                    call_id,
                    FailureKind::ResourceExhausted,
                    BrowserLifecycleErrorDetails::NotAuthorized(
                        "owned Browser close authority became unavailable".to_string(),
                    ),
                ));
            };

            let shutdown = grant.authority.terminate(self.policy).await;
            let disposal = match close_reason {
                Some(reason) => prepared.commit_with_failure(RuntimeCallFailure::new(
                    FailureKind::TargetClosed,
                    reason,
                )),
                None => prepared.commit(),
            };
            match (shutdown, disposal) {
                (Ok(report), Ok(_)) => {
                    self.report_sink.record(report);
                    Ok(CommandResult::new(call_id, Ok(())))
                }
                (Err(shutdown), Ok(_)) => Ok(browser_close_failure(
                    call_id,
                    FailureKind::BrowserCrashed,
                    BrowserLifecycleErrorDetails::Shutdown(shutdown.to_string()),
                )),
                (Ok(report), Err(cleanup)) => {
                    self.report_sink.record(report);
                    Ok(map_runtime_failure(call_id, cleanup))
                }
                (Err(shutdown), Err(cleanup)) => Ok(browser_close_failure(
                    call_id,
                    FailureKind::Cleanup,
                    BrowserLifecycleErrorDetails::Cleanup(format!(
                        "owned Chromium shutdown failed ({shutdown}); Browser subtree disposal also failed: {cleanup}"
                    )),
                )),
            }
        })
    }
}

fn browser_close_failure(
    call_id: pwright_core::CallId,
    kind: FailureKind,
    details: BrowserLifecycleErrorDetails,
) -> CommandResult<BrowserClose> {
    CommandResult::new(
        call_id,
        Err(OperationError::new(
            kind,
            "owned Browser.close failed",
            Some(details),
        )),
    )
}

fn map_runtime_failure(
    call_id: pwright_core::CallId,
    error: RuntimeStateError,
) -> CommandResult<BrowserClose> {
    let kind = match error {
        RuntimeStateError::TransportClosed => FailureKind::TransportClosed,
        RuntimeStateError::LifecycleTransactionInProgress
        | RuntimeStateError::LifecycleTransactionIdExhausted
        | RuntimeStateError::ControlMessageCapacityExceeded { .. }
        | RuntimeStateError::Resource(_) => FailureKind::ResourceExhausted,
        _ => FailureKind::Cleanup,
    };
    browser_close_failure(
        call_id,
        kind,
        BrowserLifecycleErrorDetails::Cleanup(error.to_string()),
    )
}

type BrowserCloseFutureOutput = Result<CommandResult<BrowserClose>, Infallible>;
type BoxBrowserCloseFuture =
    Pin<Box<dyn Future<Output = BrowserCloseFutureOutput> + Send + 'static>>;

pub struct ContinuableBrowserCloseFuture<Reservation>
where
    Reservation: CleanupReservation,
{
    inner: Option<BoxBrowserCloseFuture>,
    reservation: Option<Reservation>,
    started: bool,
}

impl<Reservation> Unpin for ContinuableBrowserCloseFuture<Reservation> where
    Reservation: CleanupReservation
{
}

impl<Reservation> Future for ContinuableBrowserCloseFuture<Reservation>
where
    Reservation: CleanupReservation,
{
    type Output = BrowserCloseFutureOutput;

    fn poll(
        mut self: Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.started = true;
        let outcome = self
            .inner
            .as_mut()
            .expect("completed Browser close future must not be polled again")
            .as_mut()
            .poll(context);
        if outcome.is_ready() {
            self.inner = None;
            self.reservation = None;
        }
        outcome
    }
}

impl<Reservation> Drop for ContinuableBrowserCloseFuture<Reservation>
where
    Reservation: CleanupReservation,
{
    fn drop(&mut self) {
        if !self.started {
            return;
        }
        let (Some(inner), Some(reservation)) = (self.inner.take(), self.reservation.take()) else {
            return;
        };
        reservation.submit(Box::pin(async move {
            match inner.await {
                Ok(result) if result.outcome().is_ok() => Ok(()),
                Ok(_) => Err(CleanupFailure::new(
                    "abandoned owned Browser.close continuation failed",
                )),
                Err(never) => match never {},
            }
        }));
    }
}

pub struct ContinuableOwnedBrowserCloseCommandService<Inner, Continuation> {
    service: Arc<Inner>,
    continuation: Continuation,
}

impl<Inner, Continuation> ContinuableOwnedBrowserCloseCommandService<Inner, Continuation> {
    pub const fn new(service: Arc<Inner>, continuation: Continuation) -> Self {
        Self {
            service,
            continuation,
        }
    }
}

impl<Inner, Continuation> Service<Command<BrowserClose>>
    for ContinuableOwnedBrowserCloseCommandService<Inner, Continuation>
where
    Inner: Service<Command<BrowserClose>, Response = CommandResult<BrowserClose>, Error = Infallible>
        + 'static,
    Continuation: ReservedCleanupExecutor,
{
    type Response = CommandResult<BrowserClose>;
    type Error = Infallible;
    type Future<'a>
        = ContinuableBrowserCloseFuture<Continuation::Reservation>
    where
        Self: 'a;

    fn call(&self, command: Command<BrowserClose>) -> Self::Future<'_> {
        let call_id = command.call_id();
        let reservation = match self.continuation.try_reserve() {
            Ok(reservation) => reservation,
            Err(error) => {
                let kind = match error {
                    CleanupSubmitError::CapacityExceeded { .. } => FailureKind::ResourceExhausted,
                    CleanupSubmitError::Unavailable | CleanupSubmitError::StatePoisoned => {
                        FailureKind::Cleanup
                    }
                };
                return ContinuableBrowserCloseFuture {
                    inner: Some(Box::pin(std::future::ready(Ok(browser_close_failure(
                        call_id,
                        kind,
                        BrowserLifecycleErrorDetails::Cleanup(
                            "Browser.close continuation capacity is unavailable".to_string(),
                        ),
                    ))))),
                    reservation: None,
                    started: false,
                };
            }
        };
        let service = Arc::clone(&self.service);
        ContinuableBrowserCloseFuture {
            inner: Some(Box::pin(async move { service.call(command).await })),
            reservation: Some(reservation),
            started: false,
        }
    }
}

/// Sends every child-first Dispose control through the Browser fence before
/// the typed handler can write its correlated Result.
pub struct OrderedOwnedBrowserCloseCommandService<Inner, Source, Codec, Transport> {
    service: Arc<Inner>,
    control: OrderedRuntimeControl<Source, Codec, Transport>,
}

/// Eventful terminal wrapper which preserves Playwright's exact Browser close
/// order on one transport: descendants are disposed, disconnected is
/// published once, then Browser Dispose and the command Result may follow.
pub struct OrderedEventfulOwnedBrowserCloseCommandService<
    Inner,
    Source,
    ControlCodec,
    EventCodec,
    Transport,
> {
    service: Arc<Inner>,
    control: OrderedRuntimeControl<Source, ControlCodec, Arc<Transport>>,
    disconnected: BrowserDisconnectedPublisher<EventCodec, Arc<Transport>>,
}

impl<Inner, Source, ControlCodec, EventCodec, Transport>
    OrderedEventfulOwnedBrowserCloseCommandService<
        Inner,
        Source,
        ControlCodec,
        EventCodec,
        Transport,
    >
{
    pub const fn new(
        service: Arc<Inner>,
        control: OrderedRuntimeControl<Source, ControlCodec, Arc<Transport>>,
        disconnected: BrowserDisconnectedPublisher<EventCodec, Arc<Transport>>,
    ) -> Self {
        Self {
            service,
            control,
            disconnected,
        }
    }
}

impl<Inner, Source, ControlCodec, EventCodec, Transport> Service<Command<BrowserClose>>
    for OrderedEventfulOwnedBrowserCloseCommandService<
        Inner,
        Source,
        ControlCodec,
        EventCodec,
        Transport,
    >
where
    Inner: Service<Command<BrowserClose>, Response = CommandResult<BrowserClose>, Error = Infallible>
        + 'static,
    Source: ControlMessageSource + Send + Sync + 'static,
    ControlCodec: WireCodec<pwright_core::ControlMessage> + 'static,
    ControlCodec::Error: Display,
    EventCodec: WireCodec<SequencedEvent<BrowserDisconnected>> + 'static,
    EventCodec::Error: Display,
    Transport: FrameTransport + 'static,
    Transport::Error: Display,
{
    type Response = CommandResult<BrowserClose>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<BrowserClose>) -> Self::Future<'_> {
        let call_id = command.call_id();
        let object_id = command.object_id();
        Box::pin(async move {
            let barrier = match self.control.begin_barrier().await {
                Ok(barrier) => barrier,
                Err(error) => {
                    return Ok(browser_close_failure(
                        call_id,
                        FailureKind::ResourceExhausted,
                        BrowserLifecycleErrorDetails::Cleanup(error.to_string()),
                    ));
                }
            };
            let result = match self.service.call(command).await {
                Ok(result) => result,
                Err(never) => match never {},
            };
            if let Err(error) = barrier.send_before_disposal(object_id).await {
                return if result.outcome().is_err() {
                    Ok(result)
                } else {
                    Ok(browser_close_failure(
                        call_id,
                        FailureKind::Cleanup,
                        BrowserLifecycleErrorDetails::Cleanup(format!(
                            "Browser.close could not publish descendant lifecycle before disconnected: {error}"
                        )),
                    ))
                };
            }
            let disconnected_error = self
                .disconnected
                .publish()
                .await
                .err()
                .map(|error| error.to_string());
            let disposal = barrier.send_through_disposal(object_id).await;
            if let Err(error) = disposal {
                return Ok(browser_close_failure(
                    call_id,
                    FailureKind::Cleanup,
                    BrowserLifecycleErrorDetails::Cleanup(format!(
                        "Browser.close could not publish Browser disposal after disconnected: {error}"
                    )),
                ));
            }
            if let Some(error) = disconnected_error {
                return Ok(browser_close_failure(
                    call_id,
                    FailureKind::Cleanup,
                    BrowserLifecycleErrorDetails::Cleanup(format!(
                        "Browser.close could not publish disconnected: {error}"
                    )),
                ));
            }
            Ok(result)
        })
    }
}

impl<Inner, Source, Codec, Transport>
    OrderedOwnedBrowserCloseCommandService<Inner, Source, Codec, Transport>
{
    pub const fn new(
        service: Arc<Inner>,
        control: OrderedRuntimeControl<Source, Codec, Transport>,
    ) -> Self {
        Self { service, control }
    }
}

impl<Inner, Source, Codec, Transport> Service<Command<BrowserClose>>
    for OrderedOwnedBrowserCloseCommandService<Inner, Source, Codec, Transport>
where
    Inner: Service<Command<BrowserClose>, Response = CommandResult<BrowserClose>, Error = Infallible>
        + 'static,
    Source: ControlMessageSource + Send + Sync + 'static,
    Codec: WireCodec<pwright_core::ControlMessage> + 'static,
    Transport: FrameTransport + 'static,
{
    type Response = CommandResult<BrowserClose>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, command: Command<BrowserClose>) -> Self::Future<'_> {
        let call_id = command.call_id();
        let object_id = command.object_id();
        Box::pin(async move {
            let barrier = match self.control.begin_barrier().await {
                Ok(barrier) => barrier,
                Err(error) => {
                    return Ok(browser_close_failure(
                        call_id,
                        FailureKind::ResourceExhausted,
                        BrowserLifecycleErrorDetails::Cleanup(error.to_string()),
                    ));
                }
            };
            let result = match self.service.call(command).await {
                Ok(result) => result,
                Err(never) => match never {},
            };
            match barrier.send_through_disposal(object_id).await {
                Ok(_) => Ok(result),
                Err(_error) if result.outcome().is_err() => Ok(result),
                Err(_error) => Ok(browser_close_failure(
                    call_id,
                    FailureKind::Cleanup,
                    BrowserLifecycleErrorDetails::Cleanup(
                        "Browser.close could not publish its lifecycle before completion"
                            .to_string(),
                    ),
                )),
            }
        })
    }
}

/// Independently selected execution plugins for abandoned-close ownership and
/// provider-local diagnostic delivery.
pub struct OwnedBrowserCloseExecution<ReportSink, Continuation> {
    report_sink: ReportSink,
    continuation: Continuation,
}

impl<ReportSink, Continuation> OwnedBrowserCloseExecution<ReportSink, Continuation> {
    pub const fn new(report_sink: ReportSink, continuation: Continuation) -> Self {
        Self {
            report_sink,
            continuation,
        }
    }

    pub fn into_parts(self) -> (ReportSink, Continuation) {
        (self.report_sink, self.continuation)
    }
}

pub type OwnedBrowserLifecycleHandlersWithReportSink<
    Authority,
    ReportSink,
    Continuation,
    Runtime,
    Codec,
    Transport,
> = BrowserLifecycleServerHandlers<
    OrderedOwnedBrowserCloseCommandService<
        ContinuableOwnedBrowserCloseCommandService<
            OwnedBrowserCloseCommandService<Authority, ReportSink>,
            Continuation,
        >,
        Runtime,
        Codec,
        Arc<Transport>,
    >,
>;

pub type OwnedBrowserLifecycleHandlers<Authority, Continuation, Runtime, Codec, Transport> =
    OwnedBrowserLifecycleHandlersWithReportSink<
        Authority,
        DiscardOwnedBrowserShutdownReport,
        Continuation,
        Runtime,
        Codec,
        Transport,
    >;

pub type EventfulOwnedBrowserLifecycleHandlersWithReportSink<
    Authority,
    ReportSink,
    Continuation,
    Runtime,
    ControlCodec,
    EventCodec,
    Transport,
> = BrowserLifecycleServerHandlers<
    OrderedEventfulOwnedBrowserCloseCommandService<
        ContinuableOwnedBrowserCloseCommandService<
            OwnedBrowserCloseCommandService<Authority, ReportSink>,
            Continuation,
        >,
        Runtime,
        ControlCodec,
        EventCodec,
        Transport,
    >,
>;

pub type EventfulOwnedBrowserLifecycleHandlers<
    Authority,
    Continuation,
    Runtime,
    ControlCodec,
    EventCodec,
    Transport,
> = EventfulOwnedBrowserLifecycleHandlersWithReportSink<
    Authority,
    DiscardOwnedBrowserShutdownReport,
    Continuation,
    Runtime,
    ControlCodec,
    EventCodec,
    Transport,
>;

pub fn owned_browser_lifecycle_handlers<Authority, Continuation, Runtime, Codec, Transport>(
    protocol: Arc<WireProtocol<pwright_codec_browser_lifecycle::BrowserLifecycleCodec>>,
    grant: OwnedBrowserCloseGrant<Authority>,
    lifecycle: ControlledLifecycleHandle,
    policy: ChromiumShutdownPolicy,
    continuation: Continuation,
    control: OrderedRuntimeControl<Runtime, Codec, Arc<Transport>>,
) -> OwnedBrowserLifecycleHandlers<Authority, Continuation, Runtime, Codec, Transport>
where
    Authority: OwnedBrowserTermination,
    Continuation: ReservedCleanupExecutor,
    Runtime: ControlMessageSource + Send + Sync + 'static,
    Codec: WireCodec<pwright_core::ControlMessage> + 'static,
    Transport: FrameTransport + 'static,
{
    owned_browser_lifecycle_handlers_with_report_sink(
        protocol,
        grant,
        lifecycle,
        policy,
        OwnedBrowserCloseExecution::new(DiscardOwnedBrowserShutdownReport, continuation),
        control,
    )
}

pub fn owned_browser_lifecycle_handlers_with_report_sink<
    Authority,
    ReportSink,
    Continuation,
    Runtime,
    Codec,
    Transport,
>(
    protocol: Arc<WireProtocol<pwright_codec_browser_lifecycle::BrowserLifecycleCodec>>,
    grant: OwnedBrowserCloseGrant<Authority>,
    lifecycle: ControlledLifecycleHandle,
    policy: ChromiumShutdownPolicy,
    execution: OwnedBrowserCloseExecution<ReportSink, Continuation>,
    control: OrderedRuntimeControl<Runtime, Codec, Arc<Transport>>,
) -> OwnedBrowserLifecycleHandlersWithReportSink<
    Authority,
    ReportSink,
    Continuation,
    Runtime,
    Codec,
    Transport,
>
where
    Authority: OwnedBrowserTermination,
    ReportSink: OwnedBrowserShutdownReportSink<Authority::Report>,
    Continuation: ReservedCleanupExecutor,
    Runtime: ControlMessageSource + Send + Sync + 'static,
    Codec: WireCodec<pwright_core::ControlMessage> + 'static,
    Transport: FrameTransport + 'static,
{
    let (report_sink, continuation) = execution.into_parts();
    let close = Arc::new(OwnedBrowserCloseCommandService::with_report_sink(
        grant,
        lifecycle,
        policy,
        report_sink,
    ));
    let continuable = Arc::new(ContinuableOwnedBrowserCloseCommandService::new(
        close,
        continuation,
    ));
    let ordered = Arc::new(OrderedOwnedBrowserCloseCommandService::new(
        continuable,
        control,
    ));
    browser_lifecycle_server_handlers(
        protocol,
        BrowserLifecycleServerServices {
            browser_close: ordered,
        },
    )
}

pub fn eventful_owned_browser_lifecycle_handlers<
    Authority,
    Continuation,
    Runtime,
    ControlCodec,
    EventCodec,
    Transport,
>(
    protocol: Arc<WireProtocol<pwright_codec_browser_lifecycle::BrowserLifecycleCodec>>,
    grant: OwnedBrowserCloseGrant<Authority>,
    lifecycle: ControlledLifecycleHandle,
    policy: ChromiumShutdownPolicy,
    continuation: Continuation,
    control: OrderedRuntimeControl<Runtime, ControlCodec, Arc<Transport>>,
    disconnected: BrowserDisconnectedPublisher<EventCodec, Arc<Transport>>,
) -> EventfulOwnedBrowserLifecycleHandlers<
    Authority,
    Continuation,
    Runtime,
    ControlCodec,
    EventCodec,
    Transport,
>
where
    Authority: OwnedBrowserTermination,
    Continuation: ReservedCleanupExecutor,
    Runtime: ControlMessageSource + Send + Sync + 'static,
    ControlCodec: WireCodec<pwright_core::ControlMessage> + 'static,
    ControlCodec::Error: Display,
    EventCodec: WireCodec<SequencedEvent<BrowserDisconnected>> + 'static,
    EventCodec::Error: Display,
    Transport: FrameTransport + 'static,
    Transport::Error: Display,
{
    eventful_owned_browser_lifecycle_handlers_with_report_sink(
        protocol,
        grant,
        lifecycle,
        policy,
        OwnedBrowserCloseExecution::new(DiscardOwnedBrowserShutdownReport, continuation),
        control,
        disconnected,
    )
}

pub fn eventful_owned_browser_lifecycle_handlers_with_report_sink<
    Authority,
    ReportSink,
    Continuation,
    Runtime,
    ControlCodec,
    EventCodec,
    Transport,
>(
    protocol: Arc<WireProtocol<pwright_codec_browser_lifecycle::BrowserLifecycleCodec>>,
    grant: OwnedBrowserCloseGrant<Authority>,
    lifecycle: ControlledLifecycleHandle,
    policy: ChromiumShutdownPolicy,
    execution: OwnedBrowserCloseExecution<ReportSink, Continuation>,
    control: OrderedRuntimeControl<Runtime, ControlCodec, Arc<Transport>>,
    disconnected: BrowserDisconnectedPublisher<EventCodec, Arc<Transport>>,
) -> EventfulOwnedBrowserLifecycleHandlersWithReportSink<
    Authority,
    ReportSink,
    Continuation,
    Runtime,
    ControlCodec,
    EventCodec,
    Transport,
>
where
    Authority: OwnedBrowserTermination,
    ReportSink: OwnedBrowserShutdownReportSink<Authority::Report>,
    Continuation: ReservedCleanupExecutor,
    Runtime: ControlMessageSource + Send + Sync + 'static,
    ControlCodec: WireCodec<pwright_core::ControlMessage> + 'static,
    ControlCodec::Error: Display,
    EventCodec: WireCodec<SequencedEvent<BrowserDisconnected>> + 'static,
    EventCodec::Error: Display,
    Transport: FrameTransport + 'static,
    Transport::Error: Display,
{
    let (report_sink, continuation) = execution.into_parts();
    let close = Arc::new(OwnedBrowserCloseCommandService::with_report_sink(
        grant,
        lifecycle,
        policy,
        report_sink,
    ));
    let continuable = Arc::new(ContinuableOwnedBrowserCloseCommandService::new(
        close,
        continuation,
    ));
    let ordered = Arc::new(OrderedEventfulOwnedBrowserCloseCommandService::new(
        continuable,
        control,
        disconnected,
    ));
    browser_lifecycle_server_handlers(
        protocol,
        BrowserLifecycleServerServices {
            browser_close: ordered,
        },
    )
}

pub fn browser_lifecycle_control<Runtime, Codec, Transport>(
    runtime: Runtime,
    driver: RuntimeControlDriver<Codec, Arc<Transport>>,
    waiter_limit: usize,
) -> Result<
    OrderedRuntimeControl<Runtime, Codec, Arc<Transport>>,
    pwright_runtime_control::OrderedControlGateError,
> {
    OrderedRuntimeControl::new(runtime, driver, waiter_limit)
}

#[cfg(test)]
mod tests {
    use std::future::{Future, Ready, ready};
    use std::pin::Pin;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::task::{Context, Poll, Waker};
    use std::time::Duration;

    use pwright_cap_browser_lifecycle::BrowserCloseOptions;
    use pwright_client::{
        AtomicCallIdSource, BrowserContextKind, BrowserKind, Client, PageKind, RemoteInitializer,
        RemoteKind,
    };
    use pwright_client_browser_events::browser_disconnected_handler;
    use pwright_client_browser_lifecycle::BrowserCloseExt;
    use pwright_client_control::{ClientLifecycleEvent, ClientLifecycleHandler};
    use pwright_client_wire::{Either, EncodedClientChannel, FrameHandlerExt, RoutedDriverEvent};
    use pwright_codec_browser_events::BrowserEventsCodec;
    use pwright_codec_browser_lifecycle::{BrowserLifecycleCodec, BrowserLifecycleCodecLimits};
    use pwright_codec_control::ControlCodec;
    use pwright_core::{
        CallId, CallMetadata, Command, ControlMessage, ObjectKindTag, ProtocolVersion, PushOutcome,
        Service,
    };
    use pwright_runtime::{CleanupQueue, ControlledDispatcher, NoDeadlineScheduler, RuntimeLimits};
    use pwright_runtime_tokio::TokioCleanupExecutor;
    use pwright_server_wire::{ConcurrentServerFrameHandler, ServerFrameDisposition};
    use pwright_transport::{FrameLimits, VersionPolicy, WireProtocol};
    use pwright_transport_memory::MemoryFrameTransport;

    use super::*;

    struct UnusedService;

    impl Service<()> for UnusedService {
        type Response = ();
        type Error = Infallible;
        type Future<'a> = Ready<Result<(), Infallible>>;

        fn call(&self, (): ()) -> Self::Future<'_> {
            ready(Ok(()))
        }
    }

    struct FakeAuthority {
        calls: Arc<AtomicUsize>,
        fail: bool,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct ProviderDiagnostic {
        shutdown_method: &'static str,
    }

    struct DiagnosticAuthority;

    #[derive(Debug)]
    struct FakeShutdownError;

    impl Display for FakeShutdownError {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            formatter.write_str("fake shutdown failed")
        }
    }

    impl Error for FakeShutdownError {}

    impl OwnedBrowserTermination for FakeAuthority {
        type Report = ();
        type Error = FakeShutdownError;
        type Future = Ready<Result<(), FakeShutdownError>>;

        fn terminate(self, _policy: ChromiumShutdownPolicy) -> Self::Future {
            self.calls.fetch_add(1, Ordering::SeqCst);
            ready(if self.fail {
                Err(FakeShutdownError)
            } else {
                Ok(())
            })
        }
    }

    impl OwnedBrowserTermination for DiagnosticAuthority {
        type Report = ProviderDiagnostic;
        type Error = FakeShutdownError;
        type Future = Ready<Result<ProviderDiagnostic, FakeShutdownError>>;

        fn terminate(self, _policy: ChromiumShutdownPolicy) -> Self::Future {
            ready(Ok(ProviderDiagnostic {
                shutdown_method: "provider-forced",
            }))
        }
    }

    struct PausedAuthority {
        started: Arc<std::sync::atomic::AtomicBool>,
        gate: Arc<tokio::sync::Notify>,
    }

    impl OwnedBrowserTermination for PausedAuthority {
        type Report = ();
        type Error = FakeShutdownError;
        type Future = Pin<Box<dyn Future<Output = Result<(), FakeShutdownError>> + Send + 'static>>;

        fn terminate(self, _policy: ChromiumShutdownPolicy) -> Self::Future {
            Box::pin(async move {
                self.started.store(true, Ordering::SeqCst);
                self.gate.notified().await;
                Ok(())
            })
        }
    }

    struct ShutdownDropSignal(Arc<AtomicBool>);

    impl Drop for ShutdownDropSignal {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    struct StalledAuthority {
        started: Arc<AtomicBool>,
        dropped: Arc<AtomicBool>,
    }

    impl OwnedBrowserTermination for StalledAuthority {
        type Report = ();
        type Error = FakeShutdownError;
        type Future = Pin<Box<dyn Future<Output = Result<(), FakeShutdownError>> + Send + 'static>>;

        fn terminate(self, _policy: ChromiumShutdownPolicy) -> Self::Future {
            Box::pin(async move {
                self.started.store(true, Ordering::SeqCst);
                let _drop_signal = ShutdownDropSignal(self.dropped);
                std::future::pending::<Result<(), FakeShutdownError>>().await
            })
        }
    }

    fn policy() -> ChromiumShutdownPolicy {
        ChromiumShutdownPolicy::new(
            std::time::Duration::from_secs(1),
            std::time::Duration::from_secs(1),
            std::time::Duration::from_secs(1),
        )
        .unwrap()
    }

    fn runtime() -> ControlledDispatcher<UnusedService, (), NoDeadlineScheduler> {
        let runtime = ControlledDispatcher::new(
            UnusedService,
            (),
            NoDeadlineScheduler,
            RuntimeLimits::new(8, 2).with_control_messages(8),
        );
        runtime
            .register_object(ObjectId::new(1), None, ObjectKindTag::new(1))
            .unwrap();
        runtime
            .register_object(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                ObjectKindTag::new(2),
            )
            .unwrap();
        runtime
            .register_object(
                ObjectId::new(3),
                Some(ObjectId::new(2)),
                ObjectKindTag::new(3),
            )
            .unwrap();
        for _ in 0..3 {
            runtime.pop_control_message().unwrap().unwrap();
        }
        runtime
    }

    fn browser_protocol() -> WireProtocol<BrowserLifecycleCodec> {
        WireProtocol::new(
            BrowserLifecycleCodec::new(BrowserLifecycleCodecLimits::new(256)),
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0)).unwrap(),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(1024, 0, 0, 1024),
        )
        .unwrap()
    }

    fn control_protocol() -> WireProtocol<ControlCodec> {
        WireProtocol::new(
            ControlCodec,
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0)).unwrap(),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(1024, 0, 0, 1024),
        )
        .unwrap()
    }

    fn browser_event_protocol() -> WireProtocol<BrowserEventsCodec> {
        WireProtocol::new_exact_directional(
            BrowserEventsCodec,
            ProtocolVersion::new(1, 0),
            FrameLimits::new(17, 0, 0, 17),
            FrameLimits::new(17, 0, 0, 17),
        )
    }

    fn poll_now<F>(future: Pin<&mut F>) -> Poll<F::Output>
    where
        F: Future + ?Sized,
    {
        let mut context = Context::from_waker(Waker::noop());
        future.poll(&mut context)
    }

    #[tokio::test]
    async fn authorized_close_consumes_authority_and_disposes_the_whole_browser_tree() {
        // Arrange
        let runtime = runtime();
        let calls = Arc::new(AtomicUsize::new(0));
        let grant = OwnedBrowserCloseGrant {
            browser_object_id: ObjectId::new(1),
            authority: FakeAuthority {
                calls: Arc::clone(&calls),
                fail: false,
            },
        };
        let service =
            OwnedBrowserCloseCommandService::new(grant, runtime.lifecycle_handle(), policy());
        let command = Command::<BrowserClose>::new(
            CallId::new(1),
            ObjectId::new(1),
            CallMetadata::default(),
            BrowserCloseOptions::default(),
        );

        // Act
        let result = service.call(command).await.unwrap();

        // Assert
        assert!(result.outcome().is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(runtime.live_object_count(), Ok(0));
        assert_eq!(
            runtime.pop_control_message().unwrap(),
            Some(ControlMessage::Dispose {
                object_id: ObjectId::new(3)
            })
        );
        assert_eq!(
            runtime.pop_control_message().unwrap(),
            Some(ControlMessage::Dispose {
                object_id: ObjectId::new(2)
            })
        );
        assert_eq!(
            runtime.pop_control_message().unwrap(),
            Some(ControlMessage::Dispose {
                object_id: ObjectId::new(1)
            })
        );
    }

    #[tokio::test]
    async fn shutdown_failure_still_invalidates_the_owned_browser_tree() {
        // Arrange
        let runtime = runtime();
        let grant = OwnedBrowserCloseGrant {
            browser_object_id: ObjectId::new(1),
            authority: FakeAuthority {
                calls: Arc::new(AtomicUsize::new(0)),
                fail: true,
            },
        };
        let service =
            OwnedBrowserCloseCommandService::new(grant, runtime.lifecycle_handle(), policy());
        let command = Command::<BrowserClose>::new(
            CallId::new(2),
            ObjectId::new(1),
            CallMetadata::default(),
            BrowserCloseOptions::default(),
        );

        // Act
        let result = service.call(command).await.unwrap();

        // Assert
        let error = result.outcome().as_ref().unwrap_err();
        assert_eq!(error.kind(), FailureKind::BrowserCrashed);
        assert_eq!(runtime.live_object_count(), Ok(0));
    }

    #[tokio::test]
    async fn wrong_browser_id_never_consumes_process_authority() {
        // Arrange
        let runtime = runtime();
        let calls = Arc::new(AtomicUsize::new(0));
        let grant = OwnedBrowserCloseGrant {
            browser_object_id: ObjectId::new(1),
            authority: FakeAuthority {
                calls: Arc::clone(&calls),
                fail: false,
            },
        };
        let service =
            OwnedBrowserCloseCommandService::new(grant, runtime.lifecycle_handle(), policy());
        let command = Command::<BrowserClose>::new(
            CallId::new(3),
            ObjectId::new(9),
            CallMetadata::default(),
            BrowserCloseOptions::default(),
        );

        // Act
        let result = service.call(command).await.unwrap();

        // Assert
        assert_eq!(
            result.outcome().as_ref().unwrap_err().kind(),
            FailureKind::UnsupportedCapability
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(runtime.live_object_count(), Ok(3));
    }

    #[tokio::test]
    async fn unpolled_close_does_not_claim_the_session_grant() {
        // Arrange
        let runtime = runtime();
        let calls = Arc::new(AtomicUsize::new(0));
        let grant = OwnedBrowserCloseGrant {
            browser_object_id: ObjectId::new(1),
            authority: FakeAuthority {
                calls: Arc::clone(&calls),
                fail: false,
            },
        };
        let queue = CleanupQueue::new(1);
        let service = ContinuableOwnedBrowserCloseCommandService::new(
            Arc::new(OwnedBrowserCloseCommandService::new(
                grant,
                runtime.lifecycle_handle(),
                policy(),
            )),
            queue.clone(),
        );
        let first = service.call(Command::<BrowserClose>::new(
            CallId::new(4),
            ObjectId::new(1),
            CallMetadata::default(),
            BrowserCloseOptions::default(),
        ));

        // Act
        drop(first);
        let second = service
            .call(Command::<BrowserClose>::new(
                CallId::new(5),
                ObjectId::new(1),
                CallMetadata::default(),
                BrowserCloseOptions::default(),
            ))
            .await
            .unwrap();

        // Assert
        assert!(second.outcome().is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(queue.metrics().unwrap().queued, 0);
    }

    #[tokio::test]
    async fn dropped_started_close_transfers_the_exact_shutdown_to_reserved_continuation() {
        // Arrange
        let runtime = runtime();
        let started = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let gate = Arc::new(tokio::sync::Notify::new());
        let grant = OwnedBrowserCloseGrant {
            browser_object_id: ObjectId::new(1),
            authority: PausedAuthority {
                started: Arc::clone(&started),
                gate: Arc::clone(&gate),
            },
        };
        let queue = CleanupQueue::new(1);
        let service = ContinuableOwnedBrowserCloseCommandService::new(
            Arc::new(OwnedBrowserCloseCommandService::new(
                grant,
                runtime.lifecycle_handle(),
                policy(),
            )),
            queue.clone(),
        );
        let mut close = Box::pin(service.call(Command::<BrowserClose>::new(
            CallId::new(6),
            ObjectId::new(1),
            CallMetadata::default(),
            BrowserCloseOptions::default(),
        )));
        assert!(poll_now(close.as_mut()).is_pending());
        assert!(started.load(Ordering::SeqCst));

        // Act
        drop(close);
        gate.notify_one();
        let job = queue.try_take().unwrap().unwrap();
        let continuation = job.await;

        // Assert
        assert!(continuation.is_ok());
        assert_eq!(runtime.live_object_count(), Ok(0));
        let metrics = queue.metrics().unwrap();
        assert_eq!(metrics.queued, 0);
        assert_eq!(metrics.completed, 1);
    }

    #[tokio::test]
    async fn application_shutdown_drains_the_exact_dropped_browser_close_continuation() {
        // Arrange
        let runtime = runtime();
        let started = Arc::new(AtomicBool::new(false));
        let gate = Arc::new(tokio::sync::Notify::new());
        let executor = TokioCleanupExecutor::new(1);
        let service = ContinuableOwnedBrowserCloseCommandService::new(
            Arc::new(OwnedBrowserCloseCommandService::new(
                OwnedBrowserCloseGrant {
                    browser_object_id: ObjectId::new(1),
                    authority: PausedAuthority {
                        started: Arc::clone(&started),
                        gate: Arc::clone(&gate),
                    },
                },
                runtime.lifecycle_handle(),
                policy(),
            )),
            executor.clone(),
        );
        let mut close = Box::pin(service.call(Command::<BrowserClose>::new(
            CallId::new(7),
            ObjectId::new(1),
            CallMetadata::default(),
            BrowserCloseOptions::default(),
        )));
        assert!(poll_now(close.as_mut()).is_pending());
        assert!(started.load(Ordering::SeqCst));

        // Act
        drop(close);
        assert!(executor.begin_shutdown());
        gate.notify_one();
        let report = executor.shutdown().await;

        // Assert
        assert!(report.already_started);
        assert_eq!(report.metrics.accepted, 1);
        assert_eq!(report.metrics.attempts, 1);
        assert_eq!(report.metrics.completed, 1);
        assert_eq!(report.metrics.in_flight, 0);
        assert_eq!(runtime.live_object_count(), Ok(0));
    }

    #[tokio::test]
    async fn provider_report_sink_is_typed_while_encoded_close_success_remains_unit() {
        // Arrange
        let runtime = runtime();
        let reports = Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink_reports = Arc::clone(&reports);
        let sink = move |report: ProviderDiagnostic| {
            sink_reports
                .lock()
                .expect("diagnostic reports should lock")
                .push(report);
        };
        let service = OwnedBrowserCloseCommandService::with_report_sink(
            OwnedBrowserCloseGrant {
                browser_object_id: ObjectId::new(1),
                authority: DiagnosticAuthority,
            },
            runtime.lifecycle_handle(),
            policy(),
            sink,
        );
        let protocol = browser_protocol();
        let command = Command::<BrowserClose>::new(
            CallId::new(7),
            ObjectId::new(1),
            CallMetadata::default(),
            BrowserCloseOptions::default(),
        );

        // Act
        let result = service.call(command).await.unwrap();
        let encoded = protocol.encode(&result).unwrap();
        let decoded: CommandResult<BrowserClose> = protocol.decode(encoded).unwrap();

        // Assert
        assert_eq!(decoded.outcome(), &Ok(()));
        assert_eq!(runtime.live_object_count(), Ok(0));
        assert_eq!(
            *reports.lock().expect("diagnostic reports should lock"),
            vec![ProviderDiagnostic {
                shutdown_method: "provider-forced",
            }]
        );
    }

    #[tokio::test]
    async fn dropped_started_close_runs_on_reserved_tokio_executor_without_manual_drain() {
        // Arrange
        let runtime = runtime();
        let started = Arc::new(AtomicBool::new(false));
        let gate = Arc::new(tokio::sync::Notify::new());
        let grant = OwnedBrowserCloseGrant {
            browser_object_id: ObjectId::new(1),
            authority: PausedAuthority {
                started: Arc::clone(&started),
                gate: Arc::clone(&gate),
            },
        };
        let executor = TokioCleanupExecutor::new(1);
        let service = ContinuableOwnedBrowserCloseCommandService::new(
            Arc::new(OwnedBrowserCloseCommandService::new(
                grant,
                runtime.lifecycle_handle(),
                policy(),
            )),
            executor.clone(),
        );
        let mut close = Box::pin(service.call(Command::<BrowserClose>::new(
            CallId::new(7),
            ObjectId::new(1),
            CallMetadata::default(),
            BrowserCloseOptions::default(),
        )));
        assert!(poll_now(close.as_mut()).is_pending());
        assert!(started.load(Ordering::SeqCst));

        // Act
        drop(close);
        gate.notify_one();
        for _ in 0..10 {
            if executor.metrics().in_flight == 0 {
                break;
            }
            tokio::task::yield_now().await;
        }

        // Assert
        assert_eq!(runtime.live_object_count(), Ok(0));
        let metrics = executor.metrics();
        assert_eq!(metrics.reserved, 0);
        assert_eq!(metrics.completed, 1);
        assert_eq!(metrics.timed_out, 0);
    }

    #[tokio::test(start_paused = true)]
    async fn tokio_continuation_deadline_drops_stalled_owned_shutdown_without_wedging_lifecycle() {
        // Arrange
        let runtime = runtime();
        let started = Arc::new(AtomicBool::new(false));
        let dropped = Arc::new(AtomicBool::new(false));
        let grant = OwnedBrowserCloseGrant {
            browser_object_id: ObjectId::new(1),
            authority: StalledAuthority {
                started: Arc::clone(&started),
                dropped: Arc::clone(&dropped),
            },
        };
        let executor = TokioCleanupExecutor::new_with_job_timeout(1, Duration::from_secs(5))
            .expect("nonzero continuation timeout should validate");
        let service = ContinuableOwnedBrowserCloseCommandService::new(
            Arc::new(OwnedBrowserCloseCommandService::new(
                grant,
                runtime.lifecycle_handle(),
                policy(),
            )),
            executor.clone(),
        );
        let mut close = Box::pin(service.call(Command::<BrowserClose>::new(
            CallId::new(8),
            ObjectId::new(1),
            CallMetadata::default(),
            BrowserCloseOptions::default(),
        )));
        assert!(poll_now(close.as_mut()).is_pending());
        assert!(started.load(Ordering::SeqCst));
        drop(close);
        tokio::task::yield_now().await;

        // Act
        tokio::time::advance(Duration::from_secs(5)).await;
        for _ in 0..10 {
            if executor.metrics().in_flight == 0 {
                break;
            }
            tokio::task::yield_now().await;
        }

        // Assert
        assert!(dropped.load(Ordering::SeqCst));
        let metrics = executor.metrics();
        assert_eq!(metrics.timed_out, 1);
        assert_eq!(metrics.cancelled, 0);
        assert_eq!(runtime.live_object_count(), Ok(3));
        assert!(
            runtime
                .lifecycle_handle()
                .prepare_object_disposal(ObjectId::new(1))
                .is_ok()
        );
    }

    #[tokio::test]
    async fn remote_close_orders_descendants_disconnected_browser_dispose_and_result() {
        // Arrange the real typed server handler around a fake process owner.
        let runtime = Arc::new(runtime());
        let grant = OwnedBrowserCloseGrant {
            browser_object_id: ObjectId::new(1),
            authority: FakeAuthority {
                calls: Arc::new(AtomicUsize::new(0)),
                fail: false,
            },
        };
        let (server_transport, client_transport) = MemoryFrameTransport::pair(8);
        let server_transport = Arc::new(server_transport);
        let control = OrderedRuntimeControl::new(
            Arc::clone(&runtime),
            RuntimeControlDriver::new(control_protocol(), Arc::clone(&server_transport)),
            2,
        )
        .unwrap();
        let disconnected = BrowserDisconnectedPublisher::new(
            ObjectId::new(1),
            browser_event_protocol(),
            Arc::clone(&server_transport),
        );
        let handler = eventful_owned_browser_lifecycle_handlers(
            Arc::new(browser_protocol()),
            grant,
            runtime.lifecycle_handle(),
            policy(),
            CleanupQueue::new(1),
            control,
            disconnected,
        );
        let command = Command::<BrowserClose>::new(
            CallId::new(40),
            ObjectId::new(1),
            CallMetadata::default(),
            BrowserCloseOptions::default(),
        );
        let command_frame = browser_protocol().encode(&command).unwrap();

        // Act: server completion writes the complete lifecycle fence first.
        let disposition = handler
            .handle_concurrently(command_frame, Arc::clone(&server_transport))
            .await
            .unwrap();

        // Assert the server classified the command and queued five wire frames.
        assert!(matches!(disposition, ServerFrameDisposition::Handled(_)));
        assert_eq!(client_transport.metrics().unwrap().incoming_depth(), 5);

        // Arrange the real client reader over those exact queued frames.
        let (channel, mut driver) =
            EncodedClientChannel::new(browser_protocol(), client_transport, 1);
        let client = Client::new(channel, AtomicCallIdSource::new(40));
        let browser = client
            .browser(RemoteInitializer::new(
                ObjectId::new(1),
                None,
                BrowserKind::OBJECT_KIND,
            ))
            .unwrap();
        let context = browser
            .context(RemoteInitializer::new(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                BrowserContextKind::OBJECT_KIND,
            ))
            .unwrap();
        let page = context
            .page(RemoteInitializer::new(
                ObjectId::new(3),
                Some(ObjectId::new(2)),
                PageKind::OBJECT_KIND,
            ))
            .unwrap();
        let lifecycle = ClientLifecycleHandler::new(client, control_protocol());
        let disconnected = browser_disconnected_handler(browser_event_protocol(), 1);
        let mut events = lifecycle.or_else(disconnected);
        let mut close = browser.close();
        assert!(poll_now(close.as_mut()).is_pending());

        // Act: Page and Context become stale before Browser disconnected.
        for expected in [ObjectId::new(3), ObjectId::new(2)] {
            let event = driver.receive_once_with(&mut events).await.unwrap();
            assert!(matches!(
                event,
                RoutedDriverEvent::Handled(Either::Left(ClientLifecycleEvent::Disposed { object_id, .. }))
                    if object_id == expected
            ));
            assert!(poll_now(close.as_mut()).is_pending());
        }
        let event = driver.receive_once_with(&mut events).await.unwrap();
        assert!(matches!(
            event,
            RoutedDriverEvent::Handled(Either::Right(PushOutcome::Enqueued))
        ));
        assert!(!page.is_live());
        assert!(!context.is_live());
        assert!(browser.is_live());
        assert!(poll_now(close.as_mut()).is_pending());

        // Browser disposal follows the event, and Result remains last.
        let browser_dispose = driver.receive_once_with(&mut events).await.unwrap();
        assert!(matches!(
            browser_dispose,
            RoutedDriverEvent::Handled(Either::Left(ClientLifecycleEvent::Disposed { object_id, .. }))
                if object_id == ObjectId::new(1)
        ));
        let result = driver.receive_once_with(&mut events).await.unwrap();
        let completion = poll_now(close.as_mut());

        // Assert Result is last and all public descendants are stale first.
        assert!(matches!(result, RoutedDriverEvent::Delivered(_)));
        assert!(!page.is_live());
        assert!(!context.is_live());
        assert!(!browser.is_live());
        assert!(matches!(completion, Poll::Ready(Ok(()))));
        assert_eq!(
            server_transport.metrics().unwrap().outgoing_depth(),
            0,
            "the client consumed every server frame"
        );
    }
}
