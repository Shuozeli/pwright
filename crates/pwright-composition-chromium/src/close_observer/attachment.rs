//! Attachment-level composition keeps the observer inside cancellation safety.
use crate::*;

pub type ObservedAttachedContextCloseService<Continuation> =
    ContinuableBrowserContextCloseCommandService<
        ObservedChromiumCloseCommandService<ChromiumBrowserContextCloseCommandService>,
        Continuation,
    >;
pub type ObservedAttachedPageCloseService<Continuation> = ContinuablePageCloseCommandService<
    ObservedChromiumCloseCommandService<ChromiumPageCloseCommandService>,
    Continuation,
>;

impl AttachedChromiumComposition {
    pub fn observed_browser_context_close_service_for_page_runtime<
        Deadlines,
        Cleanup,
        Continuation,
    >(
        &self,
        runtime: &ControlledDispatcher<CdpPageServices, CdpPageServices, Deadlines, Cleanup>,
        continuation: Continuation,
        observer: Arc<dyn ChromiumCloseObserver>,
    ) -> Option<ObservedAttachedContextCloseService<Continuation>>
    where
        Deadlines: DeadlineScheduler,
        Cleanup: CleanupExecutor,
        Continuation: ReservedCleanupExecutor,
    {
        self.prepared.profile.context_lifecycle.map(|_| {
            let browser_client: Arc<dyn CdpClient> =
                Arc::new(CdpSession::browser(Arc::clone(&self.connection)));
            let mut service = ChromiumBrowserContextCloseCommandService::new(
                browser_client,
                self.shared_topology(),
                runtime.lifecycle_handle(),
            )
            .with_page_services(runtime.service().clone());
            if let Some(routes) = runtime.service().handle_routes() {
                service = service.with_handle_routes(routes);
            } else if let Some(routes) = runtime.service().handle_leaf_routes() {
                service = service.with_handle_leaf_routes(routes);
            }
            ContinuableBrowserContextCloseCommandService::new(
                Arc::new(ObservedChromiumCloseCommandService::new(service, observer)),
                continuation,
            )
        })
    }

    pub fn observed_browser_context_close_service_for_page_runtime_with_network<
        Deadlines,
        Cleanup,
        Continuation,
    >(
        &self,
        runtime: &ControlledDispatcher<CdpPageServices, CdpPageServices, Deadlines, Cleanup>,
        continuation: Continuation,
        observer: Arc<dyn ChromiumCloseObserver>,
        network_registration: ChromiumNetworkPageRegistrationHandle,
    ) -> Option<ObservedAttachedContextCloseService<Continuation>>
    where
        Deadlines: DeadlineScheduler,
        Cleanup: CleanupExecutor,
        Continuation: ReservedCleanupExecutor,
    {
        self.prepared.profile.context_lifecycle.map(|_| {
            let browser_client: Arc<dyn CdpClient> =
                Arc::new(CdpSession::browser(Arc::clone(&self.connection)));
            let mut service = ChromiumBrowserContextCloseCommandService::new(
                browser_client,
                self.shared_topology(),
                runtime.lifecycle_handle(),
            )
            .with_network_registration(runtime.service().clone(), network_registration);
            if let Some(routes) = runtime.service().handle_routes() {
                service = service.with_handle_routes(routes);
            } else if let Some(routes) = runtime.service().handle_leaf_routes() {
                service = service.with_handle_leaf_routes(routes);
            }
            ContinuableBrowserContextCloseCommandService::new(
                Arc::new(ObservedChromiumCloseCommandService::new(service, observer)),
                continuation,
            )
        })
    }

    pub fn observed_page_lifecycle_services<Deadlines, Cleanup, Continuation>(
        &self,
        runtime: &ControlledDispatcher<CdpPageServices, CdpPageServices, Deadlines, Cleanup>,
        continuation: Continuation,
        poll: Arc<dyn CdpPageClosePoll>,
        observer: Arc<dyn ChromiumCloseObserver>,
    ) -> Option<(
        ObservedAttachedPageCloseService<Continuation>,
        ChromiumPageRunBeforeUnloadCommandService,
    )>
    where
        Deadlines: DeadlineScheduler,
        Cleanup: CleanupExecutor,
        Continuation: ReservedCleanupExecutor,
    {
        self.prepared.profile.page_lifecycle.map(|profile| {
            let browser_client: Arc<dyn CdpClient> =
                Arc::new(CdpSession::browser(Arc::clone(&self.connection)));
            let lifecycle = runtime.lifecycle_handle();
            let services = runtime.service().clone();
            let mut close = ChromiumPageCloseCommandService::new(
                browser_client,
                self.shared_topology(),
                services.clone(),
                lifecycle.clone(),
                profile.backend(),
                poll,
            );
            if let Some(routes) = services.handle_routes() {
                close = close.with_handle_routes(routes);
            } else if let Some(routes) = services.handle_leaf_routes() {
                close = close.with_handle_leaf_routes(routes);
            }
            (
                ContinuablePageCloseCommandService::new(
                    Arc::new(ObservedChromiumCloseCommandService::new(close, observer)),
                    continuation,
                ),
                ChromiumPageRunBeforeUnloadCommandService::new(services, lifecycle),
            )
        })
    }
}
