//! Atomic Frame selector / owning Page keyboard selection, with retained route leases.
use super::*;
use pwright_backend_cdp_input::CdpKeyDownInterceptor;

pub struct ChromiumLocatorKeyboardRoute {
    page: ObjectId,
    client: Arc<dyn pwright_cdp::CdpClient>,
    input: CdpInputCoordinator,
    locator: CdpLocatorService,
    _page_routes: Option<Arc<ResourceLease>>,
    _frame_routes: Option<Arc<ResourceLease>>,
}
impl ChromiumLocatorKeyboardRoute {
    pub fn page_id(&self) -> ObjectId {
        self.page
    }
    pub fn keyboard_client(&self) -> &Arc<dyn pwright_cdp::CdpClient> {
        &self.client
    }
    pub fn select(mut self, interceptor: Arc<dyn CdpKeyDownInterceptor>) -> Result<Self, CdpError> {
        self.locator =
            self.locator
                .with_keyboard_binding(self.client.clone(), &self.input, interceptor)?;
        Ok(self)
    }
    /// Select a caller-owned phase transition without changing route identity.
    pub fn with_handoff(
        mut self,
        handoff: Arc<dyn pwright_backend_cdp::LocatorInputHandoff>,
    ) -> Result<Self, CdpError> {
        self.locator = self.locator.with_keyboard_handoff(handoff)?;
        Ok(self)
    }

    /// Borrow the selected service while keeping both original route leases.
    pub fn service(&self) -> &CdpLocatorService {
        &self.locator
    }
}
impl CdpPageServices {
    /// Wait through the existing bounded utility-world readiness mechanism, then
    /// atomically choose the Frame selector and its owning Page's native input.
    /// This creates no session/world, task, cloned native-ID string or provider.
    /// The caller admits storage for the returned snapshot before calling this.
    pub async fn locator_keyboard_route(
        &self,
        object_id: ObjectId,
    ) -> Result<Option<ChromiumLocatorKeyboardRoute>, CdpPageRouteError> {
        loop {
            if self.ready_locator_route(object_id).await?.is_none() {
                return Ok(None);
            }
            let selected = {
                let routes = self
                    .routes
                    .lock()
                    .map_err(|_| CdpPageRouteError::StatePoisoned)?;
                let page_id = if routes.pages.contains_key(&object_id) {
                    object_id
                } else {
                    let Some(locator) = routes
                        .frame_contexts
                        .get(&object_id)
                        .and_then(|entry| entry.locator.as_ref())
                    else {
                        // Recheck readiness if the Frame world vanished
                        // after the first readiness check. The route lock
                        // is dropped before the next await.
                        continue;
                    };
                    // Input identity is retained by every Locator Frame route.
                    // The optional readiness inventory need not be installed
                    // when a caller selects only already-prepared utility worlds.
                    routes
                        .pages
                        .iter()
                        .find(|(_, page)| {
                            page.page_input
                                .as_ref()
                                .is_some_and(|input| locator.uses_input_coordinator(input))
                        })
                        .map(|(page, _)| *page)
                        .ok_or(CdpPageRouteError::FramePageUnavailable {
                            frame_object_id: object_id,
                        })?
                };
                let page = routes
                    .pages
                    .get(&page_id)
                    .ok_or(CdpPageRouteError::WorldTargetClosed { object_id: page_id })?;
                if page.crashed {
                    return Err(CdpPageRouteError::PageCrashed { object_id: page_id });
                }
                let locator = if object_id == page_id {
                    routes
                        .frame_contexts
                        .get(&page.main_frame_object_id)
                        .and_then(|entry| entry.locator.as_ref())
                        .or(page.locator.as_ref())
                } else {
                    routes
                        .frame_contexts
                        .get(&object_id)
                        .and_then(|entry| entry.locator.as_ref())
                };
                match locator {
                    Some(locator) => Some(ChromiumLocatorKeyboardRoute {
                        page: page_id,
                        client: page.client.clone(),
                        input: page.page_input.clone().ok_or(
                            CdpPageRouteError::LocatorInputOwnerUnavailable {
                                frame_object_id: object_id,
                            },
                        )?,
                        locator: locator.as_ref().clone(),
                        _page_routes: self._route_resource.clone(),
                        _frame_routes: self._frame_context_route_resource.clone(),
                    }),
                    None => None,
                }
            };
            if selected.is_some() {
                return Ok(selected);
            }
            // The world disappeared between readiness and selection. Re-enter
            // readiness, not a spin on the old service or a different Page.
        }
    }
}
