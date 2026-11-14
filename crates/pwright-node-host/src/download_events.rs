//! Optional bounded Chromium download publication.
//!
//! Download policy, browser-event correlation, completion state, temporary
//! paths and queue ownership stay in Rust. Language bindings receive one
//! retained owner and public metadata; they never subscribe to CDP directly.

use std::collections::{BTreeMap, VecDeque};
use std::error::Error;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};

use pwright_backend_cdp_topology::CdpTopology;
use pwright_cdp::{CdpEvent, CdpSession, generated::browser, generated::page};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequest,
    ResourceRequirements, ResourceScope,
};
use pwright_plugin_chromium_frame_observer::{ChromiumFrameEventObserver, FrameObservationResult};
use tokio::sync::Notify;

use crate::{HostError, HostSession};

pub const MAX_DOWNLOAD_CONTEXTS: usize = 16;
pub const MAX_DOWNLOAD_OWNERS: usize = 64;
pub const MAX_DOWNLOAD_NOTICES: usize = 64;
pub const MAX_DOWNLOAD_PATH_BYTES: usize = 4096;
const MAX_DOWNLOAD_GUID_BYTES: usize = 256;
const MAX_DOWNLOAD_URL_BYTES: usize = 1024 * 1024;
const MAX_DOWNLOAD_FILENAME_BYTES: usize = 4096;
const CONTEXT_BYTES: u64 = 1024;
const OWNER_BYTES: u64 = 4096;
const INSTALLATION_BYTES: u64 = 4096;
const INSTALLATION_SCOPE: ResourceScope = ResourceScope::extension(181, 1);
const CONTEXT_SCOPE: ResourceScope = ResourceScope::extension(181, 2);

static NEXT_INSTALLATION_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) const fn requirements() -> ResourceRequirements {
    ResourceRequirements::none()
        .with_class(
            ResourceClass::LiveObject,
            MAX_DOWNLOAD_OWNERS as u64,
            MAX_DOWNLOAD_OWNERS as u64,
        )
        .with_class(
            ResourceClass::BackendStateBytes,
            INSTALLATION_BYTES
                + MAX_DOWNLOAD_CONTEXTS as u64 * CONTEXT_BYTES
                + MAX_DOWNLOAD_OWNERS as u64 * OWNER_BYTES,
            MAX_DOWNLOAD_OWNERS as u64 * OWNER_BYTES,
        )
        .with_class(
            ResourceClass::EventItem,
            MAX_DOWNLOAD_NOTICES as u64,
            MAX_DOWNLOAD_NOTICES as u64,
        )
        .with_metadata(
            MAX_DOWNLOAD_CONTEXTS + MAX_DOWNLOAD_OWNERS + 1,
            MAX_DOWNLOAD_CONTEXTS + MAX_DOWNLOAD_NOTICES + MAX_DOWNLOAD_OWNERS,
        )
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum DownloadState {
    Pending,
    Complete(PathBuf),
    Failed(String),
    Closed,
}

pub struct DownloadOwner {
    page: ObjectId,
    guid: Box<str>,
    url: Box<str>,
    suggested_filename: Box<str>,
    expected_path: PathBuf,
    state: Mutex<DownloadState>,
    notify: Notify,
    _owner: ResourceLease,
}

impl std::fmt::Debug for DownloadOwner {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DownloadOwner")
            .field("page", &self.page)
            .field("guid", &self.guid)
            .field("url", &self.url)
            .field("suggested_filename", &self.suggested_filename)
            .finish_non_exhaustive()
    }
}

impl DownloadOwner {
    pub fn page(&self) -> ObjectId {
        self.page
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn suggested_filename(&self) -> &str {
        &self.suggested_filename
    }

    fn settle(&self, state: DownloadState) -> Result<(), HostError> {
        let mut current = self.state.lock().map_err(HostError::backend)?;
        if !matches!(*current, DownloadState::Pending) {
            return Ok(());
        }
        *current = state;
        drop(current);
        self.notify.notify_waiters();
        Ok(())
    }

    pub async fn path(&self) -> Result<String, HostError> {
        loop {
            let notified = self.notify.notified();
            let state = self.state.lock().map_err(HostError::backend)?.clone();
            match state {
                DownloadState::Pending => notified.await,
                DownloadState::Complete(path) => {
                    return path.into_os_string().into_string().map_err(|_| {
                        HostError::Backend("download path is not valid UTF-8".into())
                    });
                }
                DownloadState::Failed(error) => return Err(HostError::Backend(error)),
                DownloadState::Closed => return Err(HostError::TargetClosed),
            }
        }
    }
}

#[derive(Clone)]
pub struct DownloadNotice {
    pub page: ObjectId,
    pub owner: Arc<DownloadOwner>,
}

struct ContextEntry {
    path: PathBuf,
    _storage: ResourceLease,
}

struct State {
    contexts: BTreeMap<ObjectId, ContextEntry>,
    owners: BTreeMap<Box<str>, Weak<DownloadOwner>>,
    notices: VecDeque<DownloadNotice>,
    closed: bool,
}

pub(crate) struct Installation {
    resources: ProductionResourceLedger,
    root: PathBuf,
    state: Mutex<State>,
    notify: Notify,
    _storage: ResourceLease,
}

impl Installation {
    pub(crate) fn new(resources: &ProductionResourceLedger) -> Result<Arc<Self>, HostError> {
        let id = NEXT_INSTALLATION_ID
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .map_err(|_| {
                HostError::ResourceExhausted("download installation IDs exhausted".into())
            })?;
        let root =
            std::env::temp_dir().join(format!("pwright-downloads-{}-{id}", std::process::id()));
        if root.as_os_str().as_encoded_bytes().len() > MAX_DOWNLOAD_PATH_BYTES {
            return Err(HostError::ResourceExhausted(format!(
                "download root exceeds {MAX_DOWNLOAD_PATH_BYTES} bytes"
            )));
        }
        let storage = resources
            .claim(requirements())
            .map_err(HostError::backend)?
            .reserve(
                INSTALLATION_SCOPE,
                ResourceClass::BackendStateBytes,
                INSTALLATION_BYTES,
            )
            .map_err(HostError::backend)?;
        Ok(Arc::new(Self {
            resources: resources.clone(),
            root,
            state: Mutex::new(State {
                contexts: BTreeMap::new(),
                owners: BTreeMap::new(),
                notices: VecDeque::new(),
                closed: false,
            }),
            notify: Notify::new(),
            _storage: storage,
        }))
    }

    async fn prepare_context(&self, context: ObjectId) -> Result<PathBuf, HostError> {
        let path = self.root.join(context.get().to_string());
        if path.as_os_str().as_encoded_bytes().len() > MAX_DOWNLOAD_PATH_BYTES {
            return Err(HostError::ResourceExhausted(format!(
                "download path exceeds {MAX_DOWNLOAD_PATH_BYTES} bytes"
            )));
        }
        let storage = self
            .resources
            .claim(requirements())
            .map_err(HostError::backend)?
            .reserve(
                CONTEXT_SCOPE,
                ResourceClass::BackendStateBytes,
                CONTEXT_BYTES,
            )
            .map_err(|error| {
                HostError::ResourceExhausted(format!(
                    "download Context capacity exceeded ({MAX_DOWNLOAD_CONTEXTS}): {error}"
                ))
            })?;
        {
            let state = self.state.lock().map_err(HostError::backend)?;
            if state.closed {
                return Err(HostError::TargetClosed);
            }
            if let Some(existing) = state.contexts.get(&context) {
                return Ok(existing.path.clone());
            }
            if state.contexts.len() >= MAX_DOWNLOAD_CONTEXTS {
                return Err(HostError::ResourceExhausted(format!(
                    "download Context capacity exceeded ({MAX_DOWNLOAD_CONTEXTS})"
                )));
            }
        }
        tokio::fs::create_dir_all(&path)
            .await
            .map_err(HostError::backend)?;
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        state.contexts.entry(context).or_insert(ContextEntry {
            path: path.clone(),
            _storage: storage,
        });
        Ok(path)
    }

    async fn disable_context(&self, context: ObjectId) -> Result<(), HostError> {
        let entry = self
            .state
            .lock()
            .map_err(HostError::backend)?
            .contexts
            .remove(&context);
        if let Some(entry) = entry {
            match tokio::fs::remove_dir_all(&entry.path).await {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(HostError::backend(error)),
            }
        }
        Ok(())
    }

    fn begin(
        &self,
        page: ObjectId,
        context: ObjectId,
        params: browser::DownloadWillBeginEvent,
    ) -> Result<(), HostError> {
        bounded(&params.guid, MAX_DOWNLOAD_GUID_BYTES, "download GUID")?;
        bounded(&params.url, MAX_DOWNLOAD_URL_BYTES, "download URL")?;
        bounded(
            &params.suggested_filename,
            MAX_DOWNLOAD_FILENAME_BYTES,
            "download suggested filename",
        )?;
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed || !state.contexts.contains_key(&context) {
            return Ok(());
        }
        if state.notices.len() >= MAX_DOWNLOAD_NOTICES {
            return Err(HostError::ResourceExhausted(format!(
                "download notice capacity exceeded ({MAX_DOWNLOAD_NOTICES})"
            )));
        }
        state.owners.retain(|_, owner| owner.strong_count() != 0);
        if state.owners.len() >= MAX_DOWNLOAD_OWNERS {
            return Err(HostError::ResourceExhausted(format!(
                "download owner capacity exceeded ({MAX_DOWNLOAD_OWNERS})"
            )));
        }
        if state.owners.contains_key(params.guid.as_str()) {
            return Ok(());
        }
        let context_path = state
            .contexts
            .get(&context)
            .expect("checked download Context")
            .path
            .clone();
        let expected_path = context_path.join(&params.guid);
        if expected_path.as_os_str().as_encoded_bytes().len() > MAX_DOWNLOAD_PATH_BYTES {
            return Err(HostError::ResourceExhausted(format!(
                "download result path exceeds {MAX_DOWNLOAD_PATH_BYTES} bytes"
            )));
        }
        let owner_lease = self
            .resources
            .claim(requirements())
            .map_err(HostError::backend)?
            .reserve_many(
                ResourceScope::Object(page),
                &[
                    ResourceRequest::new(ResourceClass::LiveObject, 1),
                    ResourceRequest::new(ResourceClass::BackendStateBytes, OWNER_BYTES),
                    ResourceRequest::new(ResourceClass::EventItem, 1),
                ],
            )
            .map_err(|error| {
                HostError::ResourceExhausted(format!(
                    "download owner capacity exceeded ({MAX_DOWNLOAD_OWNERS}): {error}"
                ))
            })?;
        let guid: Box<str> = params.guid.into_boxed_str();
        let owner = Arc::new(DownloadOwner {
            page,
            guid: guid.clone(),
            url: params.url.into_boxed_str(),
            suggested_filename: params.suggested_filename.into_boxed_str(),
            expected_path,
            state: Mutex::new(DownloadState::Pending),
            notify: Notify::new(),
            _owner: owner_lease,
        });
        state.owners.insert(guid, Arc::downgrade(&owner));
        state.notices.push_back(DownloadNotice {
            page,
            owner: Arc::clone(&owner),
        });
        drop(state);
        self.notify.notify_one();
        Ok(())
    }

    fn progress(&self, params: browser::DownloadProgressEvent) -> Result<(), HostError> {
        bounded(&params.guid, MAX_DOWNLOAD_GUID_BYTES, "download GUID")?;
        let owner = self
            .state
            .lock()
            .map_err(HostError::backend)?
            .owners
            .get(params.guid.as_str())
            .and_then(Weak::upgrade);
        let Some(owner) = owner else {
            return Ok(());
        };
        match params.state.as_str() {
            "completed" => {
                let path = params
                    .file_path
                    .map_or_else(|| owner.expected_path.clone(), PathBuf::from);
                if path.as_os_str().as_encoded_bytes().len() > MAX_DOWNLOAD_PATH_BYTES {
                    return Err(HostError::ResourceExhausted(format!(
                        "download result path exceeds {MAX_DOWNLOAD_PATH_BYTES} bytes"
                    )));
                }
                owner.settle(DownloadState::Complete(path))?;
            }
            "canceled" => owner.settle(DownloadState::Failed("Download canceled".into()))?,
            "inProgress" => {}
            other => {
                return Err(HostError::Backend(format!(
                    "unsupported Chromium download state {other:?}"
                )));
            }
        }
        Ok(())
    }

    pub(crate) async fn next_notice(&self) -> Result<DownloadNotice, HostError> {
        loop {
            let notified = self.notify.notified();
            {
                let mut state = self.state.lock().map_err(HostError::backend)?;
                if let Some(notice) = state.notices.pop_front() {
                    return Ok(notice);
                }
                if state.closed {
                    return Err(HostError::TargetClosed);
                }
            }
            notified.await;
        }
    }

    fn reconcile(&self, topology: &CdpTopology) -> Result<(), HostError> {
        let live_contexts = topology
            .browser_context_routes()
            .map(|route| route.object_id())
            .collect::<Vec<_>>();
        let live_pages = topology
            .materialized_page_memberships()
            .map(|(page, _)| page)
            .collect::<Vec<_>>();
        let mut state = self.state.lock().map_err(HostError::backend)?;
        state
            .contexts
            .retain(|context, _| live_contexts.contains(context));
        state
            .notices
            .retain(|notice| live_pages.contains(&notice.page));
        state.owners.retain(|_, owner| {
            owner
                .upgrade()
                .is_some_and(|owner| live_pages.contains(&owner.page))
        });
        Ok(())
    }

    pub(crate) fn close(&self) {
        let owners = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if state.closed {
                return;
            }
            state.closed = true;
            state.notices.clear();
            state.contexts.clear();
            std::mem::take(&mut state.owners)
        };
        for owner in owners.into_values().filter_map(|owner| owner.upgrade()) {
            let _ = owner.settle(DownloadState::Closed);
        }
        self.notify.notify_waiters();
    }
}

impl Drop for Installation {
    fn drop(&mut self) {
        self.close();
        match std::fs::remove_dir_all(&self.root) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {}
        }
    }
}

pub(crate) struct Observer {
    installation: Arc<Installation>,
    downstream: Arc<dyn ChromiumFrameEventObserver>,
}

impl Observer {
    pub(crate) fn new(
        installation: Arc<Installation>,
        downstream: Arc<dyn ChromiumFrameEventObserver>,
    ) -> Self {
        Self {
            installation,
            downstream,
        }
    }
}

impl ChromiumFrameEventObserver for Observer {
    fn observes_page(&self, page: ObjectId) -> bool {
        self.downstream.observes_page(page)
    }

    fn prepare_page(&self, topology: &CdpTopology, page: ObjectId) -> FrameObservationResult {
        self.downstream.prepare_page(topology, page)
    }

    fn discard_prepared_page(
        &self,
        topology: &CdpTopology,
        page: ObjectId,
    ) -> FrameObservationResult {
        self.downstream.discard_prepared_page(topology, page)
    }

    fn initialize_frame(
        &self,
        topology: &CdpTopology,
        session: &str,
        frame: &page::Frame,
    ) -> FrameObservationResult {
        self.downstream.initialize_frame(topology, session, frame)
    }

    fn reconcile(&self, topology: &CdpTopology) -> FrameObservationResult {
        self.downstream.reconcile(topology)?;
        self.installation
            .reconcile(topology)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }

    fn event(&self, topology: &CdpTopology, event: &CdpEvent) -> FrameObservationResult {
        self.downstream.event(topology, event)?;
        match event.method.as_str() {
            "Browser.downloadWillBegin" => {
                let params: browser::DownloadWillBeginEvent =
                    serde_json::from_value(event.params.clone())?;
                let Some(frame) = topology.frame_object_id(&params.frame_id) else {
                    return Ok(());
                };
                let Some(page) = topology.page_object_id_for_frame(frame) else {
                    return Ok(());
                };
                let Some(context) = topology
                    .materialized_page_memberships()
                    .find_map(|(candidate, context)| (candidate == page).then_some(context))
                else {
                    return Ok(());
                };
                self.installation
                    .begin(page, context, params)
                    .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
            }
            "Browser.downloadProgress" => {
                let params: browser::DownloadProgressEvent =
                    serde_json::from_value(event.params.clone())?;
                self.installation
                    .progress(params)
                    .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
            }
            _ => Ok(()),
        }
    }

    fn seal(&self) {
        self.installation.close();
        self.downstream.seal();
    }
}

impl HostSession {
    pub async fn enable_download_context(&self, context_id: &str) -> Result<(), HostError> {
        let context = parse_object_id(context_id, "Context")?;
        self.require_visible_context_id(context)?;
        let (installation, connection, native_context) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let route = topology
                .topology()
                .browser_context_route(context)
                .ok_or(HostError::TargetClosed)?;
            (
                Arc::clone(&live.download_events),
                Arc::clone(live.composition.connection()),
                route.browser_context_id().to_owned(),
            )
        };
        let path = installation.prepare_context(context).await?;
        let path_string = path
            .to_str()
            .ok_or_else(|| HostError::InvalidArgument("download path is not valid UTF-8".into()))?;
        let mut params = serde_json::json!({
            "behavior": "allowAndName",
            "downloadPath": path_string,
            "eventsEnabled": true,
        });
        if !native_context.is_empty() {
            params["browserContextId"] = serde_json::Value::String(native_context);
        }
        let result = CdpSession::browser(connection)
            .send("Browser.setDownloadBehavior", params)
            .await
            .map(|_| ())
            .map_err(HostError::backend);
        if let Err(primary) = result {
            if let Err(cleanup) = installation.disable_context(context).await {
                return Err(HostError::Backend(format!(
                    "download policy installation failed: {primary}; cleanup failed: {cleanup}"
                )));
            }
            return Err(primary);
        }
        Ok(())
    }

    pub(crate) async fn disable_download_context(&self, context_id: &str) -> Result<(), HostError> {
        let context = parse_object_id(context_id, "Context")?;
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            Arc::clone(&live.download_events)
        };
        installation.disable_context(context).await
    }

    pub async fn next_download_notice(&self) -> Result<DownloadNotice, HostError> {
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            Arc::clone(&live.download_events)
        };
        tokio::select! {
            result = installation.next_notice() => result,
            () = self.stop.cancelled() => Err(HostError::TargetClosed),
            () = self.disconnected.cancelled() => Err(HostError::TargetClosed),
        }
    }
}

fn bounded(value: &str, limit: usize, kind: &str) -> Result<(), HostError> {
    if value.len() > limit {
        return Err(HostError::ResourceExhausted(format!(
            "{kind} exceeds {limit} UTF-8 bytes"
        )));
    }
    Ok(())
}

fn parse_object_id(value: &str, kind: &str) -> Result<ObjectId, HostError> {
    value
        .parse::<u64>()
        .ok()
        .filter(|value| *value != 0)
        .map(ObjectId::new)
        .ok_or_else(|| HostError::InvalidArgument(format!("invalid portable {kind} ID")))
}

#[cfg(test)]
mod tests {
    use pwright_core::{ResourceLimits, ResourceScope};

    use super::*;

    fn resources() -> ProductionResourceLedger {
        let required = requirements();
        let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
        for class in ResourceClass::ALL {
            let bound = required.class(class);
            limits = limits.with_class(class, bound.global, bound.per_scope);
        }
        ProductionResourceLedger::new(limits, required).unwrap()
    }

    async fn installation() -> Arc<Installation> {
        let installation = Installation::new(&resources()).unwrap();
        installation
            .prepare_context(ObjectId::new(3))
            .await
            .unwrap();
        installation
    }

    fn beginning(guid: &str) -> browser::DownloadWillBeginEvent {
        browser::DownloadWillBeginEvent {
            frame_id: "frame-7".into(),
            guid: guid.into(),
            url: "https://example.test/file".into(),
            suggested_filename: "file.txt".into(),
        }
    }

    #[tokio::test]
    async fn notice_is_fifo_and_completion_settles_the_exact_owner() {
        // Arrange
        let installation = installation().await;
        installation
            .begin(ObjectId::new(7), ObjectId::new(3), beginning("first"))
            .unwrap();
        installation
            .begin(ObjectId::new(7), ObjectId::new(3), beginning("second"))
            .unwrap();

        // Act
        let first = installation.next_notice().await.unwrap();
        let second = installation.next_notice().await.unwrap();
        installation
            .progress(browser::DownloadProgressEvent {
                guid: "first".into(),
                total_bytes: 5.0,
                received_bytes: 5.0,
                state: "completed".into(),
                file_path: Some("/tmp/first".into()),
            })
            .unwrap();

        // Assert
        assert_eq!(first.owner.guid.as_ref(), "first");
        assert_eq!(second.owner.guid.as_ref(), "second");
        assert_eq!(first.owner.path().await.unwrap(), "/tmp/first");
        installation.close();
        assert!(second.owner.path().await.is_err());
    }

    #[tokio::test]
    async fn unselected_context_does_not_allocate_an_owner() {
        // Arrange
        let installation = Installation::new(&resources()).unwrap();

        // Act
        installation
            .begin(ObjectId::new(7), ObjectId::new(3), beginning("ignored"))
            .unwrap();
        let state = installation.state.lock().unwrap();

        // Assert
        assert!(state.owners.is_empty());
        assert!(state.notices.is_empty());
    }

    #[test]
    fn requirements_keep_download_notices_and_owners_finite() {
        // Arrange
        let required = requirements();

        // Act
        let items = required.class(ResourceClass::EventItem);
        let owners = required.class(ResourceClass::LiveObject);

        // Assert
        assert_eq!(items.global, MAX_DOWNLOAD_NOTICES as u64);
        assert_eq!(owners.global, MAX_DOWNLOAD_OWNERS as u64);
        assert_ne!(ResourceScope::Object(ObjectId::new(7)), INSTALLATION_SCOPE);
    }
}
