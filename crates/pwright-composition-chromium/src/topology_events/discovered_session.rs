//! Native initialization before Page publication or network registration.
use super::*;

impl ChromiumTopologyEventDriver {
    pub(super) async fn initialize_discovered_session(
        &self,
        client: &dyn CdpClient,
        route: &SessionRoute,
        context: ObjectId,
        page: ObjectId,
    ) -> Result<Option<page::FrameTree>, ChromiumTopologyEventDriverError> {
        let initialization = async {
            client.page_enable().await?;
            self.enable_frame_events(client, page).await?;
            self.initialize_context_session(route, context, page)
                .await?;
            let world_name = self.utility_world.map(|profile| profile.world_name(page));
            let tree = self
                .initialize_session_worlds(client, world_name.as_deref(), false)
                .await?;
            if !self.profile.parallel_recursive_attachment() {
                client.target_set_auto_attach(true, true, true).await?;
            }
            if !self.resumes_with_utility_worlds() {
                client.runtime_run_if_waiting_for_debugger().await?;
            }
            Ok::<_, ChromiumTopologyEventDriverError>(tree)
        }
        .await;
        match initialization {
            Ok(tree) => Ok(Some(tree)),
            Err(error) if missing_native_session(&error) => {
                // Chrome's exact receipt or typed detach proves this session is gone. No Page
                // has been published by this attempt; queued detach events may
                // still retire an earlier route. Do not retry, detach a newer
                // session, or turn unrelated protocol/transport errors into OK.
                if let Some(observer) = &self.frame_events {
                    let topology = self
                        .topology
                        .lock()
                        .map_err(|_| ChromiumCompositionError::TopologyStateUnavailable)?;
                    self.ensure_current_epoch(topology.topology().epoch())?;
                    observer
                        .discard_prepared_page(topology.topology(), page)
                        .map_err(ChromiumTopologyEventDriverError::FrameEventObserver)?;
                }
                self.refresh_frame_ownership()?;
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }
}

pub(super) fn missing_native_session(error: &ChromiumTopologyEventDriverError) -> bool {
    let native = match error {
        ChromiumTopologyEventDriverError::Cdp(error) => Some(error),
        ChromiumTopologyEventDriverError::SessionInitialization(error) => {
            error.downcast_ref::<CdpError>()
        }
        _ => None,
    };
    match native {
        Some(CdpError::SessionDetached) => true,
        Some(CdpError::Protocol {
            code: -32001,
            message,
        }) => message == "Session with given id not found.",
        _ => false,
    }
}
