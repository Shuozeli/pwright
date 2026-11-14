//! Explicit native confirmation stays inside the existing continuation owner.
use crate::*;
use pwright_backend_cdp_page_lifecycle::CdpPageCloseConfirmation;

pub struct ConfirmedChromiumPageCloseCommandService {
    service: ChromiumPageCloseCommandService,
    observer: Arc<dyn ChromiumCloseObserver>,
    confirmation: Arc<dyn CdpPageCloseConfirmation>,
}

impl ConfirmedChromiumPageCloseCommandService {
    pub fn new(
        service: ChromiumPageCloseCommandService,
        observer: Arc<dyn ChromiumCloseObserver>,
        confirmation: Arc<dyn CdpPageCloseConfirmation>,
    ) -> Self {
        Self {
            service,
            observer,
            confirmation,
        }
    }
}

impl Service<Command<PageClose>> for ConfirmedChromiumPageCloseCommandService {
    type Response = CommandResult<PageClose>;
    type Error = Infallible;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;
    fn call(&self, command: Command<PageClose>) -> Self::Future<'_> {
        let (call_id, object_id, metadata, ()) = command.into_parts();
        Box::pin(async move {
            let outcome = self
                .service
                .close(
                    object_id,
                    metadata,
                    Some(self.observer.as_ref()),
                    Some(self.confirmation.as_ref()),
                )
                .await;
            Ok(CommandResult::new(call_id, outcome))
        })
    }
}

/// Terminal owner loss retires the failed operation's resources without
/// claiming a successful native close. Ordinary services stay strict.
pub struct RetirePageCloseOnOwnerLoss;
impl PageCloseRetirement for RetirePageCloseOnOwnerLoss {
    fn retired(outcome: &Result<(), OperationError<PageLifecycleErrorDetails>>) -> bool {
        match outcome {
            Ok(()) => true,
            Err(error) => matches!(
                error.kind(),
                FailureKind::Cancelled | FailureKind::TargetClosed | FailureKind::TransportClosed
            ),
        }
    }
}

pub type ConfirmedAttachedPageCloseService<Continuation> = ContinuablePageCloseCommandService<
    ConfirmedChromiumPageCloseCommandService,
    Continuation,
    RetirePageCloseOnOwnerLoss,
>;

impl AttachedChromiumComposition {
    /// The selected confirmation must be bound to this exact native connection.
    /// Its installation admission also covers this wrapper's retained body.
    pub fn confirmed_page_close_service<Deadlines, Cleanup, Continuation>(
        &self,
        runtime: &ControlledDispatcher<CdpPageServices, CdpPageServices, Deadlines, Cleanup>,
        continuation: Continuation,
        poll: Arc<dyn CdpPageClosePoll>,
        observer: Arc<dyn ChromiumCloseObserver>,
        confirmation: Arc<dyn CdpPageCloseConfirmation>,
    ) -> Option<ConfirmedAttachedPageCloseService<Continuation>>
    where
        Deadlines: DeadlineScheduler,
        Cleanup: CleanupExecutor,
        Continuation: ReservedCleanupExecutor,
    {
        self.prepared.profile.page_lifecycle.map(|profile| {
            let browser_client: Arc<dyn CdpClient> =
                Arc::new(CdpSession::browser(self.connection.clone()));
            let services = runtime.service().clone();
            let mut service = ChromiumPageCloseCommandService::new(
                browser_client,
                self.shared_topology(),
                services.clone(),
                runtime.lifecycle_handle(),
                profile.backend(),
                poll,
            );
            if let Some(routes) = services.handle_routes() {
                service = service.with_handle_routes(routes);
            } else if let Some(routes) = services.handle_leaf_routes() {
                service = service.with_handle_leaf_routes(routes);
            }
            ContinuablePageCloseCommandService::with_retirement_policy::<RetirePageCloseOnOwnerLoss>(
                Arc::new(ConfirmedChromiumPageCloseCommandService {
                    service,
                    observer,
                    confirmation,
                }),
                continuation,
            )
        })
    }

    /// Combines exact native confirmation and lifecycle observation with the
    /// selected network adapter retirement in one committed close transaction.
    pub fn confirmed_page_close_service_with_network<Deadlines, Cleanup, Continuation>(
        &self,
        runtime: &ControlledDispatcher<CdpPageServices, CdpPageServices, Deadlines, Cleanup>,
        continuation: Continuation,
        poll: Arc<dyn CdpPageClosePoll>,
        observer: Arc<dyn ChromiumCloseObserver>,
        confirmation: Arc<dyn CdpPageCloseConfirmation>,
        network_registration: ChromiumNetworkPageRegistrationHandle,
    ) -> Option<ConfirmedAttachedPageCloseService<Continuation>>
    where
        Deadlines: DeadlineScheduler,
        Cleanup: CleanupExecutor,
        Continuation: ReservedCleanupExecutor,
    {
        self.prepared.profile.page_lifecycle.map(|profile| {
            let browser_client: Arc<dyn CdpClient> =
                Arc::new(CdpSession::browser(self.connection.clone()));
            let services = runtime.service().clone();
            let mut service = ChromiumPageCloseCommandService::new(
                browser_client,
                self.shared_topology(),
                services.clone(),
                runtime.lifecycle_handle(),
                profile.backend(),
                poll,
            )
            .with_network_registration(network_registration);
            if let Some(routes) = services.handle_routes() {
                service = service.with_handle_routes(routes);
            } else if let Some(routes) = services.handle_leaf_routes() {
                service = service.with_handle_leaf_routes(routes);
            }
            ContinuablePageCloseCommandService::with_retirement_policy::<RetirePageCloseOnOwnerLoss>(
                Arc::new(ConfirmedChromiumPageCloseCommandService {
                    service,
                    observer,
                    confirmation,
                }),
                continuation,
            )
        })
    }
}
