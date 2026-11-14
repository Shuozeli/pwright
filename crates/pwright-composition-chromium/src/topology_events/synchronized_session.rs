//! Initialization and exact native-session retirement for already routed targets.
use super::*;

impl ChromiumTopologyEventDriver {
    pub(super) fn session_route_is_current(
        &self,
        route: &SessionRoute,
    ) -> Result<bool, ChromiumTopologyEventDriverError> {
        let topology = self
            .topology
            .lock()
            .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
        self.ensure_current_epoch(topology.topology().epoch())?;
        Ok(topology
            .topology()
            .target_session_route(&route.session_id)
            .is_some_and(|current| {
                current.target_id() == route.target_id
                    && current.page_target_id() == route.page_target_id
            }))
    }
    pub(super) async fn initialize_synchronized_session(
        &self,
        route: &SessionRoute,
    ) -> Result<page::FrameTree, ChromiumTopologyEventDriverError> {
        let client = self
            .session_factory
            .create_session(route.session_id.clone(), route.target_id.clone());
        let page_session = route.target_id == route.page_target_id;
        client.page_enable().await?;
        if self.frame_events.is_some() {
            let page = self
                .topology
                .lock()
                .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?
                .topology()
                .page_object_id(&route.page_target_id)
                .ok_or(ChromiumTopologyEventDriverError::SessionContextUnavailable)?;
            self.enable_frame_events(client.as_ref(), page).await?;
        }
        if self.session_initializer.is_some() {
            let (context, page) = {
                let topology = self
                    .topology
                    .lock()
                    .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
                topology
                    .topology()
                    .page_session_routes()
                    .find(|page| page.target_id() == route.page_target_id)
                    .map(|page| (page.browser_context_object_id(), page.object_id()))
                    .ok_or(ChromiumTopologyEventDriverError::SessionContextUnavailable)?
            };
            self.initialize_context_session(route, context, page, None)
                .await?;
        }
        let world_name = self.utility_world_name_for_route(route)?;
        let tree = self
            .initialize_session_worlds(client.as_ref(), world_name.as_deref(), !page_session, false)
            .await?;
        if !self.profile.parallel_recursive_attachment() {
            client
                .target_set_auto_attach_types(
                    true,
                    self.page_discovery.is_some(),
                    true,
                    self.auto_attach_target_types(),
                )
                .await?;
        }
        if self.page_discovery.is_some() && !self.resumes_with_utility_worlds() {
            client.runtime_run_if_waiting_for_debugger().await?;
        }
        Ok(tree)
    }

    /// One retirement transaction shared by native detach events and the exact
    /// missing-session receipt. No browser command or synthetic target closure.
    pub(super) fn retire_session<Runtime: ChromiumTopologyRuntime>(
        &self,
        runtime: &Runtime,
        session: &str,
        target: Option<&str>,
    ) -> Result<ChromiumTopologyEventDriverOutcome, ChromiumTopologyEventDriverError> {
        let retiring_target = self
            .route_for_session(session)?
            .filter(|route| route.target_id == route.page_target_id)
            .map(|route| route.page_target_id);
        let retiring_page = match retiring_target {
            Some(target_id) => self
                .topology
                .lock()
                .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?
                .topology()
                .page_object_id(&target_id),
            None => None,
        };
        let publish = || {
            self.transact(runtime, |topology| {
                topology.detach_target(self.epoch, session, target)
            })
        };
        let outcome = match (self.network_registration.as_ref(), retiring_page) {
            (Some(registration), Some(page)) => registration
                .commit_page_retirement(page, publish)
                .map_err(ChromiumTopologyEventDriverError::Network)?
                .map(|(outcome, _cleanup)| outcome),
            _ => publish(),
        }?;
        if let (Some(observer), Some(page)) = (&self.frame_events, retiring_page) {
            let topology = self
                .topology
                .lock()
                .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
            observer
                .discard_prepared_page(topology.topology(), page)
                .map_err(ChromiumTopologyEventDriverError::FrameEventObserver)?;
        }
        Ok(outcome)
    }
}
