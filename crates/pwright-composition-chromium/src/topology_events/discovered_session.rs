//! Native initialization before Page publication or network registration.
use super::*;

impl ChromiumTopologyEventDriver {
    pub(super) async fn initialize_discovered_session(
        &self,
        client: &dyn CdpClient,
        route: &SessionRoute,
        context: ObjectId,
        page: ObjectId,
    ) -> Result<Option<(page::FrameTree, bool, bool)>, ChromiumTopologyEventDriverError> {
        let mut initializing_events = self
            .resumes_with_utility_worlds()
            .then(|| client.subscribe_composable_events())
            .transpose()?;
        let initialization = async {
            let world_name = self.utility_world.map(|profile| profile.world_name(page));
            let tree = if self.resumes_with_utility_worlds() {
                let observes_frame_events = self
                    .frame_events
                    .as_ref()
                    .is_some_and(|observer| observer.observes_page(page));

                // Poll every event-producing domain exactly once before doing
                // any work that can wait on the popup's document. An inline
                // prompt can start while its navigation is still settling and
                // hold the Network/lifecycle acknowledgements; Page.enable must
                // already be on the wire so the dialog can be observed and
                // accepted rather than deadlocking target initialization.
                let mut page_enable = Box::pin(async {
                    client
                        .page_enable()
                        .await
                        .map_err(ChromiumTopologyEventDriverError::from)
                });
                let immediate_page_enable = std::future::poll_fn(|context| {
                    Poll::Ready(match page_enable.as_mut().poll(context) {
                        Poll::Ready(result) => Some(result),
                        Poll::Pending => None,
                    })
                })
                .await;
                let immediate_page_enable = match immediate_page_enable {
                    Some(Err(error)) => return Err(error),
                    result => result,
                };
                let mut frame_events = Box::pin(self.enable_frame_events(client, page));
                let immediate_frame_events = std::future::poll_fn(|context| {
                    Poll::Ready(match frame_events.as_mut().poll(context) {
                        Poll::Ready(result) => Some(result),
                        Poll::Pending => None,
                    })
                })
                .await;
                let immediate_frame_events = match immediate_frame_events {
                    Some(Err(error)) => return Err(error),
                    result => result,
                };
                let activate_network =
                    self.network_registration.is_some() && !observes_frame_events;
                let mut network_enable = Box::pin(async {
                    if activate_network {
                        client
                            .network_enable()
                            .await
                            .map_err(ChromiumTopologyEventDriverError::from)?;
                    }
                    Ok::<(), ChromiumTopologyEventDriverError>(())
                });
                let immediate_network_enable = std::future::poll_fn(|context| {
                    Poll::Ready(match network_enable.as_mut().poll(context) {
                        Poll::Ready(result) => Some(result),
                        Poll::Pending => None,
                    })
                })
                .await;
                let immediate_network_enable = match immediate_network_enable {
                    Some(Err(error)) => return Err(error),
                    result => result,
                };

                // Queue every independent prerequisite before resume, but do
                // not require its acknowledgement first. Runtime.enable only
                // needs to precede resume on the wire. A popup that navigates
                // directly to an inline prompt can withhold the frame-tree and
                // Runtime replies until that prompt has been accepted.
                let mut prerequisites = Box::pin(async {
                    let ((), tree) = tokio::try_join!(
                        async {
                            client
                                .target_set_auto_attach_types(
                                    true,
                                    true,
                                    true,
                                    self.auto_attach_target_types(),
                                )
                                .await
                                .map_err(ChromiumTopologyEventDriverError::from)
                        },
                        self.initialize_session_prerequisites(client, world_name.as_deref(), true,),
                    )?;
                    Ok::<page::FrameTree, ChromiumTopologyEventDriverError>(tree)
                });
                let immediate_prerequisites = std::future::poll_fn(|context| {
                    Poll::Ready(match prerequisites.as_mut().poll(context) {
                        Poll::Ready(result) => Some(result),
                        Poll::Pending => None,
                    })
                })
                .await;
                let immediate_prerequisites = match immediate_prerequisites {
                    Some(Err(error)) => return Err(error),
                    result => result,
                };

                // Poll settings before resume so a pure configuration failure
                // still prevents native execution. A pending initializer is a
                // readiness fence, not permission to deadlock the popup.
                let resume_fence = ChromiumSessionResumeFence::default();
                let mut settings = Box::pin(self.initialize_context_session(
                    route,
                    context,
                    page,
                    Some(&resume_fence),
                ));
                let immediate_settings = std::future::poll_fn(|context| {
                    Poll::Ready(match settings.as_mut().poll(context) {
                        Poll::Ready(result) => Some(result),
                        Poll::Pending => None,
                    })
                })
                .await;
                let immediate_settings = match immediate_settings {
                    Some(Err(error)) => return Err(error),
                    Some(result) => Some(result),
                    None if resume_fence.is_waiting() => {
                        // Only a selected initializer can activate this fence.
                        // Keep driving its pending command until it is admitted
                        // to the ordered write queue, not until Chrome replies.
                        // A completed error still prevents debugger resume.
                        let result =
                            std::future::poll_fn(|context| match settings.as_mut().poll(context) {
                                Poll::Ready(result) => Poll::Ready(Some(result)),
                                Poll::Pending if resume_fence.is_waiting() => Poll::Pending,
                                Poll::Pending => Poll::Ready(None),
                            })
                            .await;
                        match result {
                            Some(Err(error)) => return Err(error),
                            result => result,
                        }
                    }
                    None => None,
                };
                let prerequisites = async {
                    match immediate_prerequisites {
                        Some(result) => result,
                        None => prerequisites.await,
                    }
                };
                let mut worlds = Box::pin(async {
                    let tree = prerequisites.await?;
                    let fallback = tree.clone();
                    match self
                        .finish_session_worlds(client, tree, world_name.as_deref(), false)
                        .await
                    {
                        Ok(tree) => Ok::<_, ChromiumTopologyEventDriverError>((tree, None)),
                        Err(error) => Ok((fallback, Some(error))),
                    }
                });
                let immediate_worlds = std::future::poll_fn(|context| {
                    Poll::Ready(match worlds.as_mut().poll(context) {
                        Poll::Ready(result) => Some(result),
                        Poll::Pending => None,
                    })
                })
                .await;
                let immediate_worlds = match immediate_worlds {
                    Some(Err(error)) => return Err(error),
                    result => result,
                };
                let settings = async {
                    match immediate_settings {
                        Some(result) => result,
                        None => settings.await,
                    }
                };
                let worlds = async {
                    match immediate_worlds {
                        Some(result) => result,
                        None => worlds.await,
                    }
                };
                let frame_events = async {
                    match immediate_frame_events {
                        Some(result) => result,
                        None => frame_events.await,
                    }
                };
                let page_enable = async {
                    match immediate_page_enable {
                        Some(result) => result,
                        None => page_enable.await,
                    }
                };
                let network_enable = async {
                    match immediate_network_enable {
                        Some(result) => result,
                        None => network_enable.await,
                    }
                };
                let resume = async {
                    client
                        .runtime_run_if_waiting_for_debugger()
                        .await
                        .map_err(ChromiumTopologyEventDriverError::from)
                };
                let (settings, worlds, frame_events, page_enable, network_enable, resume) = tokio::join!(
                    settings,
                    worlds,
                    frame_events,
                    page_enable,
                    network_enable,
                    resume
                );
                match worlds {
                    Err(error) => return Err(error),
                    Ok((tree, world_error)) => {
                        let completion = settings
                            .and_then(|()| world_error.map_or(Ok(()), Err))
                            .and(frame_events)
                            .and(page_enable)
                            .and(network_enable)
                            .and(resume);
                        match completion {
                            Ok(()) => tree,
                            Err(error) if missing_native_session(&error) => tree,
                            Err(error) => return Err(error),
                        }
                    }
                }
            } else {
                self.initialize_context_session(route, context, page, None)
                    .await?;
                self.enable_frame_events(client, page).await?;
                client.page_enable().await?;
                let tree = self
                    .initialize_session_worlds(client, world_name.as_deref(), false, false)
                    .await?;
                if self.network_registration.is_some()
                    && !self
                        .frame_events
                        .as_ref()
                        .is_some_and(|observer| observer.observes_page(page))
                {
                    client.network_enable().await?;
                }
                tree
            };
            if !self.profile.parallel_recursive_attachment() {
                client
                    .target_set_auto_attach_types(true, true, true, self.auto_attach_target_types())
                    .await?;
            }
            if !self.resumes_with_utility_worlds() {
                client.runtime_run_if_waiting_for_debugger().await?;
            }
            Ok::<_, ChromiumTopologyEventDriverError>(tree)
        };
        tokio::pin!(initialization);
        let interrupted_dialog = async {
            let Some(events) = initializing_events.as_mut() else {
                return std::future::pending().await;
            };
            loop {
                let event = events.recv().await.map_err(|error| match error {
                    CdpEventRecvError::Closed => {
                        ChromiumTopologyEventDriverError::EventStreamClosed
                    }
                    CdpEventRecvError::Lagged(skipped) => {
                        ChromiumTopologyEventDriverError::ResyncRequired { skipped }
                    }
                })?;
                if event.session_id.as_deref() != Some(route.session_id.as_str())
                    || event.method != "Page.javascriptDialogOpening"
                {
                    continue;
                }
                let params: page::JavascriptDialogOpeningEvent =
                    serde_json::from_value(event.params.clone()).map_err(|error| {
                        ChromiumTopologyEventDriverError::Decode {
                            method: "Page.javascriptDialogOpening",
                            error,
                        }
                    })?;
                return Ok::<_, ChromiumTopologyEventDriverError>(params);
            }
        };
        tokio::pin!(interrupted_dialog);
        let initialization = tokio::select! {
            biased;
            result = &mut initialization => result.map(|tree| (tree, false)),
            result = &mut interrupted_dialog => result.map(|dialog| {
                (
                    initializing_dialog_frame_tree(
                        &route.target_id,
                        &route.target_id,
                        &dialog.url,
                    ),
                    true,
                )
            }),
        };
        match initialization {
            Ok((tree, resume_after_dialog)) => Ok(Some((tree, false, resume_after_dialog))),
            Err(error) if self.resumes_with_utility_worlds() && missing_native_session(&error) => {
                // Playwright constructs an internal Page before initialization
                // and reports that Page even when a short-lived popup closes
                // during initialization. The caller has already installed the
                // initializing Page/session route for this profile, so retain a
                // synthetic main-frame fact until the ordered detach event
                // closes it. This preserves buffered console/dialog ownership
                // without guessing that an unrelated transport error is close.
                Ok(Some((
                    missing_session_frame_tree(&route.target_id),
                    true,
                    false,
                )))
            }
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

fn missing_session_frame_tree(target_id: &str) -> page::FrameTree {
    initializing_dialog_frame_tree(target_id, "", "")
}

fn initializing_dialog_frame_tree(frame_id: &str, loader_id: &str, url: &str) -> page::FrameTree {
    page::FrameTree {
        frame: page::Frame {
            id: frame_id.to_owned(),
            parent_id: None,
            loader_id: loader_id.to_owned(),
            name: None,
            url: url.to_owned(),
            url_fragment: None,
            domain_and_registry: String::new(),
            security_origin: "://".to_owned(),
            security_origin_details: None,
            mime_type: "text/html".to_owned(),
            unreachable_url: None,
            ad_frame_status: None,
            secure_context_type: page::SecureContextType::InsecureScheme,
            cross_origin_isolated_context_type: page::CrossOriginIsolatedContextType::NotIsolated,
            gated_api_features: Vec::new(),
        },
        child_frames: None,
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
    native.is_some_and(cdp_error_is_missing_native_session)
}

fn cdp_error_is_missing_native_session(error: &CdpError) -> bool {
    match error {
        CdpError::SessionDetached => true,
        CdpError::Protocol {
            code: -32001,
            message,
        } => message == "Session with given id not found.",
        CdpError::Compound { source, system } => {
            cdp_error_is_missing_native_session(source)
                && cdp_error_is_missing_native_session(system)
        }
        _ => false,
    }
}
