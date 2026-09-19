//! Host-selected Page controller inventory; native cleanup precedes transport teardown.
use super::{PageDragMoveSource, PageDragSourceProfile};
use crate::HostError;
use pwright_backend_cdp_drag::{
    DragCaptureFactory, DragDataLimits, DragDataProfile, DragDataStore, DragListenerFactory,
    DragPage,
    mouse::{MouseDragDispatcher, MouseDragRetirementDriver},
};
use pwright_cdp::CdpClient;
use pwright_composition_chromium::ChromiumFrameWorldAccess;
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};
use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

const PAGES: usize = 32;
pub(crate) const EVENT_SUBSCRIBERS: usize = PAGES;
const WORLDS: usize = 32;
const OPERATIONS: usize = crate::mouse::EXECUTING
    * pwright_input_model::INPUT_MOUSE_MAX_CLICKS as usize
    + crate::keyboard::EXECUTING
    + crate::locator::keyboard::EXECUTING;
const DISPATCHES: usize = crate::mouse::MAX_PENDING_COMMANDS
    + crate::keyboard::EXECUTING
    + crate::locator::keyboard::EXECUTING;
const INVENTORY: ResourceScope = ResourceScope::extension(122, 1);
const CONTROLLER: ResourceScope = ResourceScope::extension(122, 2);
// Separate from reusable wrapper payloads. Native child futures, script/wire
// argument trees and bounded errors remain host-owned provider allowances.
const DATA_BYTES: usize = 256 * 1024;
const PROVIDER_BYTES: u64 =
    (WORLDS * (16 * 1024 + 16 * 1024) + OPERATIONS * (DATA_BYTES + 64 * 1024)) as u64;
fn controller_bytes() -> u64 {
    (std::mem::size_of::<PageController>() + 4096) as u64 + PROVIDER_BYTES
}
fn inventory_bytes() -> u64 {
    (std::mem::size_of::<Installation>() + PAGES * std::mem::size_of::<Entry>() + 256) as u64
}
fn source_profile() -> Result<PageDragSourceProfile, HostError> {
    PageDragSourceProfile::new(
        ResourceScope::extension(122, 3),
        ResourceScope::extension(122, 4),
        WORLDS,
        1,
    )
    .map_err(HostError::backend)
}
fn data_profile() -> Result<DragDataProfile, HostError> {
    DragDataProfile::new(
        ResourceScope::extension(122, 5),
        DragDataLimits {
            items: 32,
            files: 32,
            text_bytes: DATA_BYTES,
        },
        NonZeroUsize::new(1).expect("one retained drag record"),
    )
    .map_err(HostError::backend)
}
pub(crate) fn requirements() -> Result<ResourceRequirements, HostError> {
    let bytes = inventory_bytes() + PAGES as u64 * controller_bytes();
    let mut required = ResourceRequirements::none()
        .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
        .with_metadata(2, PAGES + 1);
    let page = source_profile()?
        .requirements()
        .map_err(HostError::backend)?
        .combine_concurrent(data_profile()?.requirements())
        .combine_concurrent(DragListenerFactory::requirements(WORLDS).map_err(HostError::backend)?)
        .combine_concurrent(DragCaptureFactory::requirements())
        .combine_concurrent(DragPage::requirements(OPERATIONS).map_err(HostError::backend)?)
        .combine_concurrent(
            MouseDragDispatcher::requirements(DISPATCHES).map_err(HostError::backend)?,
        );
    for _ in 0..PAGES {
        required = required.combine_concurrent(page);
    }
    Ok(required)
}
struct Entry {
    id: ObjectId,
    client: Arc<dyn CdpClient>,
    controller: Arc<PageController>,
}
pub(crate) struct Installation {
    pub(crate) locator_calls: std::sync::atomic::AtomicUsize,
    access: ChromiumFrameWorldAccess,
    resources: ProductionResourceLedger,
    entries: Mutex<Vec<Entry>>,
    _storage: ResourceLease,
}
pub(crate) struct PageController {
    pub(crate) dispatcher: Arc<MouseDragDispatcher>,
    page: DragPage,
    driver: tokio::sync::Mutex<MouseDragRetirementDriver>,
    _storage: ResourceLease,
}
impl PageController {
    pub(crate) async fn drain(&self) -> Result<(), HostError> {
        self.driver
            .lock()
            .await
            .retire_quarantined()
            .await
            .map_err(HostError::backend)
    }
    async fn shutdown(&self) -> Result<(), HostError> {
        self.dispatcher.stop();
        let drained = self.drain().await;
        // Preserve native cancellation ownership through its reply. The host
        // awaits this task; no caller timeout drops an in-flight cancel command.
        let cancel = self.page.cancel().map_err(HostError::backend)?;
        let cancelled = cancel.await.map(|_| ()).map_err(HostError::backend);
        drained.and(cancelled)
    }
}
impl Installation {
    pub(crate) fn new(
        access: ChromiumFrameWorldAccess,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, HostError> {
        let ledger = resources
            .claim(requirements()?)
            .map_err(HostError::backend)?;
        let storage = ledger
            .reserve(
                INVENTORY,
                ResourceClass::BackendStateBytes,
                inventory_bytes(),
            )
            .map_err(HostError::backend)?;
        Ok(Self {
            locator_calls: std::sync::atomic::AtomicUsize::new(0),
            access,
            resources: resources.clone(),
            entries: Mutex::new(Vec::with_capacity(PAGES)),
            _storage: storage,
        })
    }
    pub(crate) fn bind(
        &self,
        id: ObjectId,
        client: &Arc<dyn CdpClient>,
    ) -> Result<Arc<PageController>, HostError> {
        let mut entries = self.entries.lock().map_err(HostError::backend)?;
        if let Some(entry) = entries
            .iter()
            .find(|entry| entry.id == id && Arc::ptr_eq(&entry.client, client))
        {
            return Ok(entry.controller.clone());
        }
        // Only discard a stale route after revoking its escaped dispatchers and
        // observing no native owner or active Page drag. Dirty state is retained.
        let mut index = 0;
        while index < entries.len() {
            let entry = &entries[index];
            let binding = match self.access.page_input_binding(entry.id) {
                Ok(binding) => binding,
                Err(pwright_composition_chromium::FrameWorldAccessError::Topology(
                    pwright_composition_chromium::CdpPageRouteError::PageCrashed { .. },
                )) => None,
                Err(error) => return Err(HostError::backend(error)),
            };
            let current =
                binding.is_some_and(|binding| Arc::ptr_eq(&binding.into_parts().0, &entry.client));
            if !current {
                entry.controller.dispatcher.stop();
            }
            if !current
                && entry.controller.dispatcher.is_drained()
                && !entry.controller.page.is_dragging()
            {
                entries.remove(index);
            } else {
                index += 1;
            }
        }
        if entries.len() >= PAGES {
            return Err(HostError::ResourceExhausted(
                "drag Page/session controller capacity exceeded".into(),
            ));
        }
        let ledger = self
            .resources
            .claim(requirements()?)
            .map_err(HostError::backend)?;
        let storage = ledger
            .reserve(
                CONTROLLER,
                ResourceClass::BackendStateBytes,
                controller_bytes(),
            )
            .map_err(HostError::backend)?;
        let native = client.clone().retained_native_session().ok_or_else(|| {
            HostError::Backend("drag runtime requires the retained native session".into())
        })?;
        let store =
            DragDataStore::new(data_profile()?, &self.resources).map_err(HostError::backend)?;
        let captures = DragCaptureFactory::new(
            native.clone(),
            store,
            ResourceScope::extension(122, 6),
            &self.resources,
        )
        .map_err(HostError::backend)?;
        let session = native
            .session_id()
            .ok_or_else(|| HostError::Backend("drag requires a Page session".into()))?;
        if session.len() > 48 {
            return Err(HostError::ResourceExhausted(
                "drag listener namespace session ID exceeds 48 bytes".into(),
            ));
        }
        let namespace = format!("node-host-drag-{session}");
        let listeners = DragListenerFactory::new(
            &namespace,
            ResourceScope::extension(122, 7),
            WORLDS,
            &self.resources,
        )
        .map_err(HostError::backend)?;
        let source = Arc::new(
            PageDragMoveSource::new(
                self.access.clone(),
                id,
                listeners,
                captures.clone(),
                source_profile()?,
                &self.resources,
            )
            .map_err(HostError::backend)?,
        );
        let page = DragPage::new(
            native,
            captures,
            ResourceScope::extension(122, 8),
            OPERATIONS,
            &self.resources,
        )
        .map_err(HostError::backend)?;
        let (dispatcher, driver) = MouseDragDispatcher::new(
            client.clone(),
            page.clone(),
            source,
            ResourceScope::extension(122, 9),
            DISPATCHES,
            &self.resources,
        )
        .map_err(HostError::backend)?;
        let controller = Arc::new(PageController {
            dispatcher: Arc::new(dispatcher),
            page,
            driver: tokio::sync::Mutex::new(driver),
            _storage: storage,
        });
        entries.push(Entry {
            id,
            client: client.clone(),
            controller: controller.clone(),
        });
        Ok(controller)
    }
    pub(crate) async fn shutdown(&self) -> Result<(), HostError> {
        let mut first = None;
        // Mouse admission is sealed and all invocation workers have finished, so
        // the inventory cannot change. Never hold its std Mutex across await.
        for index in 0..PAGES {
            let controller = self
                .entries
                .lock()
                .map_err(HostError::backend)?
                .get(index)
                .map(|entry| entry.controller.clone());
            let Some(controller) = controller else {
                break;
            };
            if let Err(error) = controller.shutdown().await {
                first.get_or_insert(error);
            }
        }
        first.map_or(Ok(()), Err)
    }
}
