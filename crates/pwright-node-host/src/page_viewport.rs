//! Optional Page-scoped viewport overrides.
//!
//! The leaf retains at most one fixed-size record per live Page and replays it
//! through the session-initialization fence after Chromium replaces a Page
//! session. Device scale and mobile semantics are composed from the separately
//! selected Context device-emulation owner; language bindings only validate
//! the public width/height shape.

use std::error::Error;
use std::sync::{Arc, Mutex};

use pwright_backend_cdp_topology::CdpContextOwnership;
use pwright_cdp::{CdpSession, generated::browser};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};
use pwright_plugin_chromium_session_initialization::{
    ChromiumSessionInitializationFuture, ChromiumSessionInitializationTarget,
    ChromiumSessionInitializer, ChromiumSessionTargetKind,
};

use crate::{HostError, HostSession, device_emulation};

pub mod profile {
    pub const PAGES: usize = 64;
    pub const BYTES: u64 = 8 * 1024;
}

const MAX_PAGES: usize = profile::PAGES;
const STORAGE_BYTES: u64 = profile::BYTES;
const SCOPE: ResourceScope = ResourceScope::extension(178, 1);

pub(crate) fn requirements() -> ResourceRequirements {
    ResourceRequirements::none()
        .with_class(
            ResourceClass::BackendStateBytes,
            STORAGE_BYTES,
            STORAGE_BYTES,
        )
        .with_metadata(1, 1)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PageState {
    page: ObjectId,
    viewport: device_emulation::DeviceViewport,
}

fn prune_closed_pages_and_has_capacity(
    pages: &mut [Option<PageState>; MAX_PAGES],
    active: &[Option<ObjectId>; MAX_PAGES],
    page: ObjectId,
) -> bool {
    for entry in pages.iter_mut() {
        if entry.is_some_and(|entry| !active.contains(&Some(entry.page))) {
            *entry = None;
        }
    }
    pages
        .iter()
        .any(|entry| entry.is_none_or(|entry| entry.page == page))
}

pub(crate) struct Installation {
    connection: Arc<pwright_cdp::CdpConnection>,
    devices: Arc<device_emulation::Installation>,
    gate: tokio::sync::Mutex<()>,
    pages: Mutex<[Option<PageState>; MAX_PAGES]>,
    headful: bool,
    host_platform: &'static str,
    _storage: ResourceLease,
}

impl Installation {
    pub(crate) fn new(
        connection: Arc<pwright_cdp::CdpConnection>,
        devices: Arc<device_emulation::Installation>,
        resources: &ProductionResourceLedger,
        headful: bool,
        host_platform: &'static str,
    ) -> Result<Arc<Self>, HostError> {
        let ledger = resources
            .claim(requirements())
            .map_err(HostError::backend)?;
        let storage = ledger
            .reserve(SCOPE, ResourceClass::BackendStateBytes, STORAGE_BYTES)
            .map_err(HostError::backend)?;
        Ok(Arc::new(Self {
            connection,
            devices,
            gate: tokio::sync::Mutex::new(()),
            pages: Mutex::new([None; MAX_PAGES]),
            headful,
            host_platform,
            _storage: storage,
        }))
    }

    fn prepare(
        &self,
        page: ObjectId,
        active: &[Option<ObjectId>; MAX_PAGES],
        viewport: device_emulation::DeviceViewport,
    ) -> Result<PageState, HostError> {
        let mut pages = self.pages.lock().map_err(HostError::backend)?;
        if !prune_closed_pages_and_has_capacity(&mut pages, active, page) {
            return Err(HostError::ResourceExhausted(
                "Page viewport capacity exceeds 64 live Pages".into(),
            ));
        }
        Ok(PageState { page, viewport })
    }

    fn commit(&self, state: PageState) -> Result<(), HostError> {
        let mut pages = self.pages.lock().map_err(HostError::backend)?;
        let slot = pages
            .iter()
            .position(|entry| entry.is_some_and(|entry| entry.page == state.page))
            .or_else(|| pages.iter().position(Option::is_none))
            .ok_or_else(|| {
                HostError::ResourceExhausted("Page viewport capacity exceeds 64 live Pages".into())
            })?;
        pages[slot] = Some(state);
        Ok(())
    }

    fn viewport(
        &self,
        page: ObjectId,
    ) -> Result<Option<device_emulation::DeviceViewport>, HostError> {
        Ok(self
            .pages
            .lock()
            .map_err(HostError::backend)?
            .iter()
            .flatten()
            .find(|entry| entry.page == page)
            .map(|entry| entry.viewport))
    }

    async fn apply(
        &self,
        session_id: &str,
        target_id: &str,
        context: ObjectId,
        ownership: CdpContextOwnership,
        viewport: device_emulation::DeviceViewport,
    ) -> Result<(), HostError> {
        let session = CdpSession::new(
            Arc::clone(&self.connection),
            session_id.to_owned(),
            target_id.to_owned(),
        );
        let window = session
            .send(
                "Browser.getWindowForTarget",
                serde_json::to_value(browser::GetWindowForTargetParams::default())
                    .map_err(HostError::backend)?,
            )
            .await
            .map_err(HostError::backend)?;
        let window: browser::GetWindowForTargetReturns =
            serde_json::from_value(window).map_err(HostError::backend)?;
        let (inset_width, inset_height) =
            device_emulation::window_insets(self.headful, self.host_platform);
        session
            .send(
                "Browser.setWindowBounds",
                serde_json::to_value(browser::SetWindowBoundsParams {
                    window_id: window.window_id,
                    bounds: browser::Bounds {
                        width: Some(i64::from(viewport.width() + inset_width)),
                        height: Some(i64::from(viewport.height() + inset_height)),
                        ..Default::default()
                    },
                })
                .map_err(HostError::backend)?,
            )
            .await
            .map_err(HostError::backend)?;
        let metrics = self.devices.page_metrics(context, ownership, viewport)?;
        session
            .send(
                "Emulation.setDeviceMetricsOverride",
                serde_json::to_value(metrics).map_err(HostError::backend)?,
            )
            .await
            .map(|_| ())
            .map_err(HostError::backend)
    }

    async fn initialize_target(
        &self,
        target: ChromiumSessionInitializationTarget<'_>,
    ) -> Result<(), HostError> {
        if target.kind != ChromiumSessionTargetKind::Page {
            return Ok(());
        }
        let Some(viewport) = self.viewport(target.page)? else {
            return Ok(());
        };
        self.apply(
            target.session_id,
            target.target_id,
            target.context,
            target.ownership,
            viewport,
        )
        .await
    }
}

const _: () = assert!(
    std::mem::size_of::<Installation>() + std::mem::size_of::<[Option<PageState>; MAX_PAGES]>()
        < STORAGE_BYTES as usize
);

pub(crate) struct Initializer<Downstream> {
    downstream: Downstream,
    installation: Arc<Installation>,
}

impl<Downstream> Initializer<Downstream> {
    pub(crate) fn new(downstream: Downstream, installation: Arc<Installation>) -> Self {
        Self {
            downstream,
            installation,
        }
    }
}

impl<Downstream> ChromiumSessionInitializer for Initializer<Downstream>
where
    Downstream: ChromiumSessionInitializer,
{
    fn initialize<'a>(
        &'a self,
        session_id: &'a str,
        target_id: &'a str,
        ownership: pwright_backend_cdp_topology::CdpContextOwnership,
        is_page: bool,
    ) -> ChromiumSessionInitializationFuture<'a> {
        self.downstream
            .initialize(session_id, target_id, ownership, is_page)
    }

    fn initialize_scoped<'a>(
        &'a self,
        target: ChromiumSessionInitializationTarget<'a>,
    ) -> ChromiumSessionInitializationFuture<'a> {
        Box::pin(async move {
            self.downstream.initialize_scoped(target).await?;
            self.installation
                .initialize_target(target)
                .await
                .map_err(|error| -> Box<dyn Error + Send + Sync> { Box::new(error) })
        })
    }
}

impl HostSession {
    pub async fn page_set_viewport_size(
        &self,
        page_id: &str,
        width: u32,
        height: u32,
    ) -> Result<(), HostError> {
        let viewport = device_emulation::DeviceViewport::new(width, height)?;
        let (page, _call, _operation) = self.admit_operation(page_id)?;
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .page_viewport
                .clone()
        };
        let _gate = tokio::select! {
            biased;
            () = self.stop.cancelled() => return Err(HostError::TargetClosed),
            gate = installation.gate.lock() => gate,
        };
        let (session_id, target_id, context, active) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let topology = topology.topology();
            let route = topology
                .page_session_routes()
                .find(|route| route.object_id() == page)
                .ok_or(HostError::TargetClosed)?;
            let mut active = [None; MAX_PAGES];
            for (slot, route) in active.iter_mut().zip(topology.page_session_routes()) {
                *slot = Some(route.object_id());
            }
            (
                route.session_id().to_owned(),
                route.target_id().to_owned(),
                route.browser_context_object_id(),
                active,
            )
        };
        let next = installation.prepare(page, &active, viewport)?;
        installation
            .apply(
                &session_id,
                &target_id,
                context,
                CdpContextOwnership::Owned,
                viewport,
            )
            .await?;
        installation.commit(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_registry_prunes_closed_pages_before_enforcing_capacity() {
        // Arrange
        let page = ObjectId::new(70);
        let viewport = device_emulation::DeviceViewport::new(500, 500).unwrap();
        let mut stored = [None; MAX_PAGES];
        for (index, entry) in stored.iter_mut().enumerate() {
            *entry = Some(PageState {
                page: ObjectId::new(u64::try_from(index + 1).unwrap()),
                viewport,
            });
        }
        let active = [None; MAX_PAGES];

        // Act
        let admitted = prune_closed_pages_and_has_capacity(&mut stored, &active, page);

        // Assert
        assert!(admitted);
        assert!(stored.iter().all(Option::is_none));
    }
}
