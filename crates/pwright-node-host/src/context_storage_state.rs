//! Optional, bounded BrowserContext storage-state ownership.
//!
//! The language boundary supplies typed data; only this leaf visits origins,
//! reads browser-owned storage, and restores it before a user Page is created.

use std::error::Error;
use std::future::Future;
use std::sync::{Arc, Mutex, Weak};

use pwright_backend_cdp_topology::{CdpTopology, ConnectionEpoch};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};
use pwright_plugin_chromium_frame_observer::{
    ChromiumCommittedFrameOwnership, ChromiumFrameOwnershipView, ChromiumFrameUrlObserver,
    FrameObservationResult,
};
use pwright_runtime::{CleanupFailure, CleanupReservation, ReservedCleanupExecutor};
use pwright_runtime_tokio::TokioCleanupReservation;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use crate::context_cookies::{BrowserContextCookie, BrowserContextCookieInput};
use crate::evaluation::{EvaluateParams, EvaluationValue};
use crate::{HostError, HostSession};

const SCOPE: ResourceScope = ResourceScope::extension(198, 0);
const MAX_CONTEXTS: usize = 16;
const MAX_FRAMES: usize = 128;
pub const MAX_STORAGE_ORIGINS: usize = 128;
pub const MAX_STORAGE_ITEMS: usize = 4096;
pub const MAX_STORAGE_STATE_BYTES: usize = 8 * 1024 * 1024;
const MAX_ORIGIN_BYTES: usize = 4096;
const MAX_ORIGIN_BYTES_TOTAL: usize = 256 * 1024;
const MAX_ONE_ORIGIN_JSON_BYTES: usize = 256 * 1024;
const NAVIGATION_TIMEOUT_MS: u32 = 30_000;
const CHANGED_ORIGIN_ERROR: &str = "storage-state Page changed origin during collection";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextStorageItem {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextStorageOrigin {
    pub origin: String,
    #[serde(default)]
    pub local_storage: Vec<ContextStorageItem>,
    #[serde(rename = "indexedDB", default, skip_serializing_if = "Option::is_none")]
    pub indexed_db: Option<Vec<Value>>,
}

#[derive(Clone, Debug, Default)]
pub struct ContextStorageState {
    pub cookies: Vec<BrowserContextCookieInput>,
    pub origins: Vec<ContextStorageOrigin>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContextStorageStateSnapshot {
    pub cookies: Vec<BrowserContextCookie>,
    pub origins: Vec<ContextStorageOrigin>,
}

#[derive(Clone)]
struct ContextEntry {
    id: ObjectId,
    origins: Vec<String>,
    overflowed: bool,
}

#[derive(Clone)]
struct FrameEntry {
    id: ObjectId,
    page: ObjectId,
    context: ObjectId,
    main: bool,
    origin: Option<String>,
}

struct State {
    contexts: Vec<ContextEntry>,
    frames: Vec<FrameEntry>,
    pending: Vec<(ObjectId, String)>,
    overflowed_unknown: bool,
    closed: bool,
}

/// An exact, task-local capability for one storage-state scratch Page. It is
/// never returned by a HostSession API or accepted from a language binding.
#[derive(Clone)]
struct InternalPagePermit {
    owner: Weak<()>,
    context: ObjectId,
    page: ObjectId,
    main_frame: ObjectId,
    epoch: ConnectionEpoch,
}

tokio::task_local! {
    static INTERNAL_PAGE_PERMIT: InternalPagePermit;
}

impl InternalPagePermit {
    fn new(host: &Arc<HostSession>, page: &crate::PageDescriptor) -> Result<Self, HostError> {
        fn object_id(value: &str) -> Result<ObjectId, HostError> {
            value
                .parse::<u64>()
                .ok()
                .filter(|id| *id != 0)
                .map(ObjectId::new)
                .ok_or(HostError::TargetClosed)
        }

        let page_id = object_id(&page.id)?;
        let main_frame = object_id(&page.main_frame_id)?;
        let context = object_id(&page.context_id)?;
        if !host.is_connected() || !host.frame_trees.is_internal_page(page_id)? {
            return Err(HostError::TargetClosed);
        }
        let state = host.state.lock().map_err(HostError::backend)?;
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        let topology = live.composition.topology().map_err(HostError::backend)?;
        let topology = topology.topology();
        if topology.page_object_id_for_frame(main_frame) != Some(page_id)
            || !topology
                .materialized_page_memberships()
                .any(|(candidate, owner)| candidate == page_id && owner == context)
            || !topology.page_session_routes().any(|route| {
                route.object_id() == page_id && route.browser_context_object_id() == context
            })
        {
            return Err(HostError::TargetClosed);
        }
        Ok(Self {
            owner: Arc::downgrade(&host.context_storage_state.identity),
            context,
            page: page_id,
            main_frame,
            epoch: topology.epoch(),
        })
    }

    fn matches(
        &self,
        identity: &Arc<()>,
        topology: &CdpTopology,
        page: ObjectId,
        frame: Option<ObjectId>,
    ) -> bool {
        self.owner
            .upgrade()
            .is_some_and(|owner| Arc::ptr_eq(&owner, identity))
            && self.page == page
            && frame.is_none_or(|frame| frame == self.main_frame)
            && topology.epoch() == self.epoch
            && topology.page_object_id_for_frame(self.main_frame) == Some(self.page)
            && topology
                .materialized_page_memberships()
                .any(|(candidate, owner)| candidate == self.page && owner == self.context)
            && topology.page_session_routes().any(|route| {
                route.object_id() == self.page && route.browser_context_object_id() == self.context
            })
    }
}

async fn with_internal_page_permit<T>(
    permit: &InternalPagePermit,
    work: impl Future<Output = T>,
) -> T {
    INTERNAL_PAGE_PERMIT.scope(permit.clone(), work).await
}

pub(crate) fn permits_internal_page_target(
    host: &HostSession,
    topology: &CdpTopology,
    page: ObjectId,
    frame: Option<ObjectId>,
) -> Result<bool, HostError> {
    let matches = INTERNAL_PAGE_PERMIT
        .try_with(|permit| {
            permit.matches(&host.context_storage_state.identity, topology, page, frame)
        })
        .unwrap_or(false);
    if !matches || !host.is_connected() || !host.frame_trees.is_internal_page(page)? {
        return Ok(false);
    }
    Ok(!host
        .context_storage_state
        .state
        .lock()
        .map_err(HostError::backend)?
        .closed)
}

type OriginCandidate = (String, Option<(ObjectId, ObjectId)>);

/// An indexed Frame is a direct storage source only when its Page is public
/// in a fresh owned Browser. Hidden/internal candidates still leave their
/// origin in the book, so the caller visits it with its private scratch Page.
fn direct_collection_frame(
    current: Option<(ObjectId, ObjectId)>,
    owned_contexts_only: bool,
    is_published_public_page: impl FnOnce(ObjectId) -> Result<bool, HostError>,
) -> Result<Option<ObjectId>, HostError> {
    let Some((page, frame)) = current else {
        return Ok(None);
    };
    if owned_contexts_only && !is_published_public_page(page)? {
        return Ok(None);
    }
    Ok(Some(frame))
}

pub(crate) struct Installation {
    state: Mutex<State>,
    operation: tokio::sync::Mutex<()>,
    identity: Arc<()>,
    _storage: ResourceLease,
}

impl Installation {
    pub(crate) fn new(resources: &ProductionResourceLedger) -> Result<Arc<Self>, HostError> {
        let bytes = (std::mem::size_of::<Self>()
            + std::mem::size_of::<State>()
            + MAX_CONTEXTS * std::mem::size_of::<ContextEntry>()
            + MAX_FRAMES
                * (std::mem::size_of::<FrameEntry>() + std::mem::size_of::<(ObjectId, String)>())
            + MAX_CONTEXTS * MAX_STORAGE_ORIGINS * std::mem::size_of::<String>()
            + MAX_FRAMES * MAX_ORIGIN_BYTES
            + MAX_ORIGIN_BYTES_TOTAL
            + 2 * std::mem::size_of::<usize>()) as u64;
        let plan = ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, 1);
        let storage = resources
            .claim(plan)
            .map_err(HostError::backend)?
            .reserve(SCOPE, ResourceClass::BackendStateBytes, bytes)
            .map_err(HostError::backend)?;
        let mut contexts = Vec::new();
        let mut frames = Vec::new();
        let mut pending = Vec::new();
        contexts
            .try_reserve_exact(MAX_CONTEXTS)
            .map_err(HostError::backend)?;
        frames
            .try_reserve_exact(MAX_FRAMES)
            .map_err(HostError::backend)?;
        pending
            .try_reserve_exact(MAX_FRAMES)
            .map_err(HostError::backend)?;
        Ok(Arc::new(Self {
            state: Mutex::new(State {
                contexts,
                frames,
                pending,
                overflowed_unknown: false,
                closed: false,
            }),
            operation: tokio::sync::Mutex::new(()),
            identity: Arc::new(()),
            _storage: storage,
        }))
    }

    pub(crate) fn close(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.closed = true;
            state.contexts.clear();
            state.frames.clear();
            state.pending.clear();
        }
    }

    fn ordered_origins(&self, context: ObjectId) -> Result<Vec<OriginCandidate>, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        let entry = state
            .contexts
            .iter()
            .find(|entry| entry.id == context)
            .ok_or(HostError::TargetClosed)?;
        if entry.overflowed {
            return Err(HostError::ResourceExhausted(
                "storage-state visited-origin capacity was exceeded".into(),
            ));
        }
        let mut result = Vec::new();
        result
            .try_reserve_exact(entry.origins.len())
            .map_err(HostError::backend)?;
        for frame in state
            .frames
            .iter()
            .filter(|frame| frame.context == context && frame.main)
        {
            if let Some(origin) = frame.origin.as_ref()
                && entry.origins.contains(origin)
                && !result.iter().any(|(known, _)| known == origin)
            {
                result.push((origin.clone(), Some((frame.page, frame.id))));
            }
        }
        for origin in &entry.origins {
            if !result.iter().any(|(known, _)| known == origin) {
                result.push((origin.clone(), None));
            }
        }
        Ok(result)
    }

    fn replace_origins(
        &self,
        context: ObjectId,
        origins: &[ContextStorageOrigin],
    ) -> Result<(), HostError> {
        let origin_bytes = origins
            .iter()
            .try_fold(0usize, |total, origin| {
                total.checked_add(origin.origin.len())
            })
            .ok_or_else(|| {
                HostError::ResourceExhausted("storage-state origin byte count overflow".into())
            })?;
        let mut state = self.state.lock().map_err(HostError::backend)?;
        let retained_bytes = state
            .contexts
            .iter()
            .filter(|entry| entry.id != context)
            .flat_map(|entry| &entry.origins)
            .map(String::len)
            .sum::<usize>();
        if retained_bytes
            .checked_add(origin_bytes)
            .is_none_or(|total| total > MAX_ORIGIN_BYTES_TOTAL)
        {
            return Err(HostError::ResourceExhausted(format!(
                "storage-state origin book exceeds {MAX_ORIGIN_BYTES_TOTAL} bytes"
            )));
        }
        let entry = state
            .contexts
            .iter_mut()
            .find(|entry| entry.id == context)
            .ok_or(HostError::TargetClosed)?;
        entry.origins.clear();
        entry.overflowed = false;
        for origin in origins {
            if !entry.origins.contains(&origin.origin) {
                entry.origins.push(origin.origin.clone());
            }
        }
        Ok(())
    }

    fn record(&self, frame: ObjectId, url: &str) -> Result<(), HostError> {
        let Some(origin) = origin_from_url(url) else {
            return Ok(());
        };
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        if origin.len() > MAX_ORIGIN_BYTES {
            state.overflowed_unknown = true;
            for context in &mut state.contexts {
                context.overflowed = true;
            }
            return Ok(());
        }
        if let Some(entry) = state.frames.iter_mut().find(|entry| entry.id == frame) {
            entry.origin = Some(origin.clone());
            let context = entry.context;
            insert_origin(&mut state, context, origin);
        } else if let Some(pending) = state.pending.iter_mut().find(|pending| pending.0 == frame) {
            pending.1 = origin;
        } else if state.pending.len() < MAX_FRAMES {
            state.pending.push((frame, origin));
        } else {
            state.overflowed_unknown = true;
            for context in &mut state.contexts {
                context.overflowed = true;
            }
        }
        Ok(())
    }

    fn retain_live(&self, is_live: &dyn Fn(ObjectId) -> bool) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        state.frames.retain(|frame| is_live(frame.id));
        state.pending.retain(|(frame, _)| is_live(*frame));
        Ok(())
    }

    fn reconcile(&self, view: ChromiumFrameOwnershipView<'_>) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        let context_ids = view
            .collection_contexts()
            .map(|entry| entry.id)
            .collect::<Vec<_>>();
        state
            .contexts
            .retain(|entry| context_ids.contains(&entry.id));
        for context in context_ids {
            if !state.contexts.iter().any(|entry| entry.id == context) {
                if state.contexts.len() == MAX_CONTEXTS {
                    return Err(HostError::ResourceExhausted(
                        "storage-state Context capacity exceeded".into(),
                    ));
                }
                let overflowed = state.overflowed_unknown;
                state.contexts.push(ContextEntry {
                    id: context,
                    origins: Vec::new(),
                    overflowed,
                });
            }
        }
        let pages = view.collection_pages().collect::<Vec<_>>();
        let mut frames = Vec::new();
        frames
            .try_reserve_exact(MAX_FRAMES)
            .map_err(HostError::backend)?;
        for owner in view.frames() {
            if frames.len() == MAX_FRAMES {
                return Err(HostError::ResourceExhausted(
                    "storage-state Frame capacity exceeded".into(),
                ));
            }
            let Some(context) = pages
                .iter()
                .find(|page| page.id == owner.page)
                .map(|page| page.context)
            else {
                continue;
            };
            let origin = state
                .frames
                .iter()
                .find(|old| old.id == owner.frame)
                .and_then(|old| old.origin.clone())
                .or_else(|| {
                    state
                        .pending
                        .iter()
                        .find(|old| old.0 == owner.frame)
                        .map(|old| old.1.clone())
                });
            frames.push(FrameEntry {
                id: owner.frame,
                page: owner.page,
                context,
                main: owner.parent.is_none(),
                origin,
            });
        }
        state.frames = frames;
        let observed = state
            .frames
            .iter()
            .filter_map(|frame| {
                frame
                    .origin
                    .as_ref()
                    .map(|origin| (frame.context, origin.clone()))
            })
            .collect::<Vec<_>>();
        for (context, origin) in observed {
            insert_origin(&mut state, context, origin);
        }
        let live_frame_ids = state
            .frames
            .iter()
            .map(|frame| frame.id)
            .collect::<Vec<_>>();
        state.pending.retain(|(id, _)| !live_frame_ids.contains(id));
        Ok(())
    }
}

fn insert_origin(state: &mut State, context: ObjectId, origin: String) {
    let bytes = state
        .contexts
        .iter()
        .flat_map(|entry| &entry.origins)
        .map(String::len)
        .sum::<usize>();
    let Some(entry) = state.contexts.iter_mut().find(|entry| entry.id == context) else {
        return;
    };
    if entry.origins.contains(&origin) {
        return;
    }
    if entry.origins.len() == MAX_STORAGE_ORIGINS
        || bytes
            .checked_add(origin.len())
            .is_none_or(|total| total > MAX_ORIGIN_BYTES_TOTAL)
    {
        entry.overflowed = true;
        return;
    }
    entry.origins.push(origin);
}

fn origin_from_url(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    matches!(parsed.scheme(), "http" | "https").then(|| parsed.origin().ascii_serialization())
}

pub(crate) struct Observer {
    downstream: Arc<crate::frame_trees::FrameTrees>,
    installation: Arc<Installation>,
}

impl Observer {
    pub(crate) fn new(
        downstream: Arc<crate::frame_trees::FrameTrees>,
        installation: Arc<Installation>,
    ) -> Self {
        Self {
            downstream,
            installation,
        }
    }
}

impl ChromiumFrameUrlObserver for Observer {
    fn update(&self, frame: ObjectId, url: &str, fragment: Option<&str>) -> FrameObservationResult {
        self.downstream.update(frame, url, fragment)?;
        self.installation
            .record(frame, url)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }

    fn update_navigation(
        &self,
        frame: ObjectId,
        url: &str,
        fragment: Option<&str>,
        name: &str,
    ) -> FrameObservationResult {
        self.downstream
            .update_navigation(frame, url, fragment, name)?;
        self.installation
            .record(frame, url)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }

    fn retain_live(&self, is_live: &dyn Fn(ObjectId) -> bool) -> FrameObservationResult {
        self.downstream.retain_live(is_live)?;
        self.installation
            .retain_live(is_live)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }

    fn reconcile_ownership(&self, view: ChromiumFrameOwnershipView<'_>) -> FrameObservationResult {
        self.downstream.reconcile_ownership(view)?;
        self.installation
            .reconcile(view)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }

    fn reconcile_committed(
        &self,
        publication: ChromiumCommittedFrameOwnership<'_>,
    ) -> FrameObservationResult {
        self.downstream.reconcile_committed(publication)?;
        self.installation
            .reconcile(publication.ownership())
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }
}

fn validate_origin(origin: &ContextStorageOrigin) -> Result<usize, HostError> {
    if origin.origin.len() > MAX_ORIGIN_BYTES
        || origin_from_url(&origin.origin).as_deref() != Some(&origin.origin)
    {
        return Err(HostError::InvalidArgument(
            "storage-state origin must be a canonical HTTP(S) origin".into(),
        ));
    }
    if origin.local_storage.len() > MAX_STORAGE_ITEMS {
        return Err(HostError::ResourceExhausted(format!(
            "localStorage item count exceeds {MAX_STORAGE_ITEMS}"
        )));
    }
    if origin
        .indexed_db
        .as_ref()
        .is_some_and(|databases| databases.iter().any(contains_encoded_value))
    {
        return Err(HostError::UnsupportedOperation(
            "storage-state encoded IndexedDB keys and values are not selected".into(),
        ));
    }
    let bytes = serde_json::to_vec(origin)
        .map_err(HostError::backend)?
        .len();
    if bytes > MAX_ONE_ORIGIN_JSON_BYTES {
        return Err(HostError::ResourceExhausted(format!(
            "one origin storage state exceeds {MAX_ONE_ORIGIN_JSON_BYTES} bytes"
        )));
    }
    Ok(bytes)
}

fn contains_encoded_value(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().any(contains_encoded_value),
        Value::Object(properties) => properties.iter().any(|(key, value)| {
            matches!(key.as_str(), "keyEncoded" | "valueEncoded") || contains_encoded_value(value)
        }),
        _ => false,
    }
}

fn validate_state(state: &ContextStorageState) -> Result<(), HostError> {
    if state.origins.len() > MAX_STORAGE_ORIGINS {
        return Err(HostError::ResourceExhausted(format!(
            "storage-state origin count exceeds {MAX_STORAGE_ORIGINS}"
        )));
    }
    let mut total = 0usize;
    let mut origin_bytes = 0usize;
    for origin in &state.origins {
        origin_bytes = origin_bytes
            .checked_add(origin.origin.len())
            .ok_or_else(|| {
                HostError::ResourceExhausted("storage-state origin byte count overflow".into())
            })?;
        if origin_bytes > MAX_ORIGIN_BYTES_TOTAL {
            return Err(HostError::ResourceExhausted(format!(
                "storage-state origin book exceeds {MAX_ORIGIN_BYTES_TOTAL} bytes"
            )));
        }
        total = total.checked_add(validate_origin(origin)?).ok_or_else(|| {
            HostError::ResourceExhausted("storage-state byte count overflow".into())
        })?;
        if total > MAX_STORAGE_STATE_BYTES {
            return Err(HostError::ResourceExhausted(format!(
                "storage state exceeds {MAX_STORAGE_STATE_BYTES} bytes"
            )));
        }
    }
    Ok(())
}

const COLLECT_SCRIPT: &str = r#"async includeIndexedDB => {
  const wellFormed = value => String(value).toWellFormed();
  const plainJSON = (value, seen = new WeakSet()) => {
    if (value === null || typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean')
      return true;
    if (typeof value !== 'object' || seen.has(value)) return false;
    if (!Array.isArray(value) && Object.getPrototypeOf(value) !== Object.prototype) return false;
    seen.add(value);
    const valid = Object.values(value).every(item => plainJSON(item, seen));
    seen.delete(value);
    return valid;
  };
  const request = value => new Promise((resolve, reject) => {
    value.addEventListener('success', () => resolve(value.result), { once: true });
    value.addEventListener('error', () => reject(value.error), { once: true });
  });
  const state = {
    origin: location.origin,
    localStorage: Object.keys(localStorage).map(name => ({
      name: wellFormed(name), value: wellFormed(localStorage.getItem(name))
    }))
  };
  if (includeIndexedDB) {
    state.indexedDB = [];
    for (const info of await indexedDB.databases()) {
      if (!info.name || !info.version) throw new Error('IndexedDB database metadata is incomplete');
      const db = await request(indexedDB.open(info.name));
      const stores = [];
      try {
        for (const storeName of db.objectStoreNames) {
          const transaction = db.transaction(storeName, 'readonly');
          const store = transaction.objectStore(storeName);
          const keys = await request(store.getAllKeys());
          const records = [];
          for (const key of keys) {
            const value = await request(store.get(key));
            if (!plainJSON(key) || !plainJSON(value))
              throw new Error('Encoded IndexedDB key or value is not selected');
            if (store.keyPath === null) records.push({ key, value });
            else records.push({ value });
          }
          const indexes = [];
          for (const name of store.indexNames) {
            const index = store.index(name);
            indexes.push({ name: index.name,
              ...(typeof index.keyPath === 'string' ? { keyPath: index.keyPath } : { keyPathArray: index.keyPath }),
              multiEntry: index.multiEntry, unique: index.unique });
          }
          stores.push({ name: storeName, records, indexes, autoIncrement: store.autoIncrement,
            ...(typeof store.keyPath === 'string' ? { keyPath: store.keyPath } :
              Array.isArray(store.keyPath) ? { keyPathArray: store.keyPath } : {}) });
        }
      } finally { db.close(); }
      state.indexedDB.push({ name: info.name, version: info.version, stores });
    }
  }
  return JSON.stringify(state);
}"#;

const RESTORE_SCRIPT: &str = r#"async stateJson => {
  const state = JSON.parse(stateJson);
  if (location.origin !== state.origin)
    throw new Error('storage-state origin does not match temporary Page');
  const request = value => new Promise((resolve, reject) => {
    value.addEventListener('success', () => resolve(value.result), { once: true });
    value.addEventListener('error', () => reject(value.error), { once: true });
  });
  if (navigator.serviceWorker) {
    const registrations = await navigator.serviceWorker.getRegistrations();
    await Promise.all(registrations.map(registration => registration.unregister()));
  }
  for (const info of await indexedDB.databases()) {
    if (info.name) await request(indexedDB.deleteDatabase(info.name));
  }
  for (const info of state.indexedDB || []) {
    const open = indexedDB.open(info.name, info.version);
    open.addEventListener('upgradeneeded', () => {
      const db = open.result;
      for (const item of info.stores) {
        const store = db.createObjectStore(item.name, {
          autoIncrement: item.autoIncrement,
          keyPath: item.keyPathArray ?? item.keyPath
        });
        for (const index of item.indexes)
          store.createIndex(index.name, index.keyPathArray ?? index.keyPath,
            { unique: index.unique, multiEntry: index.multiEntry });
      }
    }, { once: true });
    const db = await request(open);
    try {
      if (db.objectStoreNames.length) {
        const transaction = db.transaction(db.objectStoreNames, 'readwrite');
        for (const item of info.stores) {
          const store = transaction.objectStore(item.name);
          for (const record of item.records)
            await request(Object.hasOwn(record, 'key')
              ? store.add(record.value, record.key) : store.add(record.value));
        }
        await new Promise((resolve, reject) => {
          transaction.addEventListener('complete', resolve, { once: true });
          transaction.addEventListener('abort', () => reject(transaction.error), { once: true });
          transaction.addEventListener('error', () => reject(transaction.error), { once: true });
        });
      }
    } finally { db.close(); }
  }
  sessionStorage.clear();
  localStorage.clear();
  for (const item of state.localStorage || []) localStorage.setItem(item.name, item.value);
  return true;
}"#;

fn evaluation_string(value: EvaluationValue) -> Result<String, HostError> {
    match value {
        EvaluationValue::String(value) => Ok(value),
        _ => Err(HostError::Backend(
            "storage-state collector returned a non-string value".into(),
        )),
    }
}

fn combine_primary_and_cleanup(
    primary: Result<(), HostError>,
    cleanup: Result<(), HostError>,
) -> Result<(), HostError> {
    match (primary, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(()), Err(cleanup)) => Err(cleanup),
        (Err(primary), Err(cleanup)) => Err(HostError::Backend(format!(
            "storage-state operation failed: {primary}; temporary Page cleanup failed: {cleanup}"
        ))),
    }
}

/// Owns cleanup even if the foreign caller cancels while navigation or
/// evaluation is suspended. The retry is scheduled on the captured runtime,
/// never on a detached, untracked browser transport.
struct TemporaryPage {
    host: Arc<HostSession>,
    page: crate::PageDescriptor,
    permit: InternalPagePermit,
    registration: Option<u64>,
    cleanup: Option<TokioCleanupReservation>,
    settled: bool,
}

impl TemporaryPage {
    fn new(
        host: Arc<HostSession>,
        page: crate::PageDescriptor,
        permit: InternalPagePermit,
        cleanup: TokioCleanupReservation,
    ) -> Self {
        Self {
            host,
            page,
            permit,
            registration: None,
            cleanup: Some(cleanup),
            settled: false,
        }
    }

    async fn close(&mut self) -> Result<(), HostError> {
        let result = with_internal_page_permit(&self.permit, async {
            if let Some(registration) = self.registration {
                self.host
                    .close_temporary_page(&self.page.id, registration)
                    .await
            } else {
                self.host.close_page(&self.page.id).await
            }
        })
        .await;
        if result.is_ok() {
            self.settled = true;
            self.cleanup.take();
        }
        result
    }
}

impl Drop for TemporaryPage {
    fn drop(&mut self) {
        if self.settled {
            return;
        }
        let host = self.host.clone();
        let page_id = self.page.id.clone();
        let permit = self.permit.clone();
        let registration = self.registration;
        let Some(reservation) = self.cleanup.take() else {
            return;
        };
        reservation.submit(Box::pin(async move {
            let cleanup = with_internal_page_permit(&permit, async {
                if let Some(registration) = registration {
                    host.close_temporary_page(&page_id, registration).await
                } else {
                    host.close_page(&page_id).await
                }
            })
            .await;
            if let Err(error) = cleanup {
                tracing::error!(page_id, error = %error, "temporary storage-state Page cleanup failed");
                return Err(CleanupFailure::new(error.to_string()));
            }
            Ok(())
        }));
    }
}

async fn bounded_detached_result<T, F>(work: F) -> Result<T, HostError>
where
    T: Send + 'static,
    F: Future<Output = Result<T, HostError>> + Send + 'static,
{
    let (reply, completion) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let _ = reply.send(work.await);
    });
    completion.await.map_err(|_| {
        HostError::Backend("temporary storage-state Page creation task ended".into())
    })?
}

impl HostSession {
    async fn collect_origin(
        &self,
        frame_id: &str,
        expected_origin: &str,
        indexed_db: bool,
        permit: Option<&InternalPagePermit>,
    ) -> Result<ContextStorageOrigin, HostError> {
        let evaluate = self.frame_evaluate(
            frame_id,
            EvaluateParams::function(COLLECT_SCRIPT, EvaluationValue::Bool(indexed_db)),
        );
        let value = match permit {
            Some(permit) => with_internal_page_permit(permit, evaluate).await?,
            None => evaluate.await?,
        };
        let value = evaluation_string(value)?;
        if value.len() > MAX_ONE_ORIGIN_JSON_BYTES {
            return Err(HostError::ResourceExhausted(format!(
                "one origin storage state exceeds {MAX_ONE_ORIGIN_JSON_BYTES} bytes"
            )));
        }
        let origin: ContextStorageOrigin =
            serde_json::from_str(&value).map_err(HostError::backend)?;
        if origin.origin != expected_origin {
            return Err(HostError::Backend(CHANGED_ORIGIN_ERROR.into()));
        }
        validate_origin(&origin)?;
        Ok(origin)
    }

    async fn restore_origin(
        &self,
        frame_id: &str,
        origin: &ContextStorageOrigin,
        permit: &InternalPagePermit,
    ) -> Result<(), HostError> {
        let value = serde_json::to_string(origin).map_err(HostError::backend)?;
        if value.len() > MAX_ONE_ORIGIN_JSON_BYTES {
            return Err(HostError::ResourceExhausted(format!(
                "one origin storage state exceeds {MAX_ONE_ORIGIN_JSON_BYTES} bytes"
            )));
        }
        let result = with_internal_page_permit(
            permit,
            self.frame_evaluate(
                frame_id,
                EvaluateParams::function(RESTORE_SCRIPT, EvaluationValue::String(value)),
            ),
        )
        .await?;
        if !matches!(result, EvaluationValue::Bool(true)) {
            return Err(HostError::Backend(
                "storage-state restorer did not confirm completion".into(),
            ));
        }
        Ok(())
    }

    async fn temporary_page(
        self: &Arc<Self>,
        context_id: &str,
    ) -> Result<TemporaryPage, HostError> {
        if context_id.len() > 20 {
            return Err(HostError::InvalidArgument(
                "invalid portable Context ID".into(),
            ));
        }
        // A foreign future can be dropped at any await, including after the
        // driver creates the native Page but before the reply is polled. Own
        // creation in a bounded task and send the cleanup guard itself; an
        // abandoned receiver drops that guard and schedules its cleanup.
        let cleanup = self.cleanup.try_reserve().map_err(HostError::backend)?;
        let host = self.clone();
        let context_id = context_id.to_owned();
        bounded_detached_result(async move {
                let page = host.new_internal_page_inner(&context_id).await?;
                let permit = match InternalPagePermit::new(&host, &page) {
                    Ok(permit) => permit,
                    Err(error) => {
                        // The native Page exists but its exact ownership could
                        // not be proven; close the whole owned session rather
                        // than expose or abandon an untracked scratch Page.
                        host.request_shutdown();
                        return Err(error);
                    }
                };
                let mut guard = TemporaryPage::new(host.clone(), page, permit, cleanup);
                match with_internal_page_permit(
                    &guard.permit,
                    host.install_storage_state_route(&guard.page.id),
                ).await {
                    Ok(registration) => {
                        guard.registration = Some(registration);
                        Ok(guard)
                    }
                    Err(primary) => {
                        let cleanup = guard.close().await;
                        match cleanup {
                            Ok(()) => Err(primary),
                            Err(cleanup) => Err(HostError::Backend(format!(
                                "storage-state route setup failed: {primary}; temporary Page cleanup failed: {cleanup}"
                            ))),
                        }
                    }
                }
            }).await
    }

    async fn close_temporary_page(
        &self,
        page_id: &str,
        registration: u64,
    ) -> Result<(), HostError> {
        let route = match self.page_unroute(page_id, &[registration]).await {
            // The route table's removal is idempotent. If the Page disappeared
            // first, its Fetch session and all page-scoped routes are retired.
            Err(HostError::TargetClosed) => Ok(()),
            result => result,
        };
        let page = self.close_page(page_id).await;
        match (route, page) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
            (Err(route), Err(page)) => Err(HostError::Backend(format!(
                "temporary storage-state route cleanup failed: {route}; Page cleanup failed: {page}"
            ))),
        }
    }

    pub async fn context_storage_state(
        self: &Arc<Self>,
        context_id: &str,
        indexed_db: bool,
    ) -> Result<ContextStorageStateSnapshot, HostError> {
        let (context, _call, _operation) = self.admit_operation(context_id)?;
        let _exclusive = self
            .context_storage_state
            .operation
            .try_lock()
            .map_err(|_| {
                HostError::ResourceExhausted("storage-state operation already in progress".into())
            })?;
        let cookies = self.context_cookies(context_id, Vec::new()).await?;
        let origins = self.context_storage_state.ordered_origins(context)?;
        let mut saved = Vec::new();
        saved
            .try_reserve_exact(origins.len())
            .map_err(HostError::backend)?;
        let mut temporary: Option<TemporaryPage> = None;
        let mut total = 0usize;
        let primary = async {
            for (origin, current) in origins {
                let direct_frame = direct_collection_frame(
                    current,
                    self.context_metadata.owned_contexts_only(),
                    |page| self.frame_trees.is_published_public_page(page),
                )?;
                let current_result = if let Some(frame) = direct_frame {
                    match self
                        .collect_origin(&frame.get().to_string(), &origin, indexed_db, None)
                        .await
                    {
                        Ok(value) => Some(value),
                        Err(HostError::FrameDetached) => None,
                        Err(HostError::Backend(message)) if message == CHANGED_ORIGIN_ERROR => None,
                        Err(error) => return Err(error),
                    }
                } else {
                    None
                };
                let result = if let Some(result) = current_result {
                    result
                } else {
                    if temporary.is_none() {
                        temporary = Some(self.temporary_page(context_id).await?);
                    }
                    let scratch = temporary.as_ref().expect("temporary Page was installed");
                    with_internal_page_permit(
                        &scratch.permit,
                        self.page_goto(
                            &scratch.page.id,
                            origin.clone(),
                            "load",
                            None,
                            NAVIGATION_TIMEOUT_MS,
                        ),
                    )
                    .await?;
                    self.collect_origin(
                        &scratch.page.main_frame_id,
                        &origin,
                        indexed_db,
                        Some(&scratch.permit),
                    )
                    .await?
                };
                if !result.local_storage.is_empty()
                    || result
                        .indexed_db
                        .as_ref()
                        .is_some_and(|dbs| !dbs.is_empty())
                {
                    total = total
                        .checked_add(validate_origin(&result)?)
                        .ok_or_else(|| {
                            HostError::ResourceExhausted("storage-state byte count overflow".into())
                        })?;
                    if total > MAX_STORAGE_STATE_BYTES {
                        return Err(HostError::ResourceExhausted(format!(
                            "storage state exceeds {MAX_STORAGE_STATE_BYTES} bytes"
                        )));
                    }
                    saved.push(result);
                }
            }
            Ok(())
        }
        .await;
        let cleanup = if let Some(mut page) = temporary {
            page.close().await
        } else {
            Ok(())
        };
        combine_primary_and_cleanup(primary, cleanup)?;
        Ok(ContextStorageStateSnapshot {
            cookies,
            origins: saved,
        })
    }

    pub async fn context_set_storage_state(
        self: &Arc<Self>,
        context_id: &str,
        state: ContextStorageState,
    ) -> Result<(), HostError> {
        let (context, _call, _operation) = self.admit_operation(context_id)?;
        let _exclusive = self
            .context_storage_state
            .operation
            .try_lock()
            .map_err(|_| {
                HostError::ResourceExhausted("storage-state operation already in progress".into())
            })?;
        validate_state(&state)?;
        if !state.cookies.is_empty() {
            self.context_add_cookies(context_id, state.cookies.clone())
                .await?;
        }
        let mut temporary = None;
        let primary = async {
            if !state.origins.is_empty() {
                temporary = Some(self.temporary_page(context_id).await?);
            }
            for origin in &state.origins {
                let scratch = temporary.as_ref().expect("temporary Page was installed");
                with_internal_page_permit(
                    &scratch.permit,
                    self.page_goto(
                        &scratch.page.id,
                        origin.origin.clone(),
                        "load",
                        None,
                        NAVIGATION_TIMEOUT_MS,
                    ),
                )
                .await?;
                self.restore_origin(&scratch.page.main_frame_id, origin, &scratch.permit)
                    .await?;
            }
            Ok(())
        }
        .await;
        let cleanup = if let Some(mut page) = temporary {
            page.close().await
        } else {
            Ok(())
        };
        combine_primary_and_cleanup(primary, cleanup)?;
        self.context_storage_state
            .replace_origins(context, &state.origins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_backend_cdp_topology::{
        CdpContextOwnership, CdpPageObjectIds, CdpPageTarget, CdpTopologyLimits,
    };

    const TEST_EPOCH: ConnectionEpoch = ConnectionEpoch::new(1);

    fn scratch_topology() -> CdpTopology {
        let mut topology = CdpTopology::new(
            ObjectId::new(1),
            TEST_EPOCH,
            CdpTopologyLimits::new(16, 4, 4, 8, 4, 64),
        )
        .unwrap();
        topology
            .register_default_context(TEST_EPOCH, ObjectId::new(2))
            .unwrap();
        topology
            .register_context(
                TEST_EPOCH,
                "owned-context",
                ObjectId::new(5),
                CdpContextOwnership::Owned,
            )
            .unwrap();
        topology
            .attach_page(
                TEST_EPOCH,
                CdpPageTarget::new("scratch-page", "scratch-session", "owned-context"),
                CdpPageObjectIds::new(ObjectId::new(9), ObjectId::new(10)),
            )
            .unwrap();
        topology
    }

    fn scratch_permit(identity: &Arc<()>) -> InternalPagePermit {
        InternalPagePermit {
            owner: Arc::downgrade(identity),
            context: ObjectId::new(5),
            page: ObjectId::new(9),
            main_frame: ObjectId::new(10),
            epoch: TEST_EPOCH,
        }
    }

    #[test]
    fn hidden_indexed_origin_uses_scratch_page_fallback() {
        // Arrange: the origin book indexed a main Frame on an unpublished Page.
        let current = Some((ObjectId::new(9), ObjectId::new(10)));

        // Act
        let direct = direct_collection_frame(current, true, |_| Ok(false)).unwrap();

        // Assert: None selects the existing scratch navigation/collection path.
        assert_eq!(direct, None);
    }

    #[test]
    fn attached_current_origin_preserves_direct_collection() {
        // Arrange
        let current = Some((ObjectId::new(9), ObjectId::new(10)));

        // Act
        let direct = direct_collection_frame(current, false, |_| {
            panic!("attached mode must not require owned Page publication")
        })
        .unwrap();

        // Assert
        assert_eq!(direct, Some(ObjectId::new(10)));
    }

    #[test]
    fn current_origin_visibility_error_is_not_silently_fallback() {
        // Arrange
        let current = Some((ObjectId::new(9), ObjectId::new(10)));

        // Act
        let direct = direct_collection_frame(current, true, |_| Err(HostError::TargetClosed));

        // Assert
        assert!(matches!(direct, Err(HostError::TargetClosed)));
    }

    #[tokio::test]
    async fn scoped_scratch_permit_admits_only_exact_page_and_main_frame() {
        // Arrange
        let identity = Arc::new(());
        let topology = scratch_topology();
        let permit = scratch_permit(&identity);

        // Act
        let admitted = with_internal_page_permit(&permit, async {
            INTERNAL_PAGE_PERMIT
                .try_with(|active| {
                    [
                        active.matches(&identity, &topology, ObjectId::new(9), None),
                        active.matches(
                            &identity,
                            &topology,
                            ObjectId::new(9),
                            Some(ObjectId::new(10)),
                        ),
                        active.matches(
                            &identity,
                            &topology,
                            ObjectId::new(9),
                            Some(ObjectId::new(11)),
                        ),
                        active.matches(&identity, &topology, ObjectId::new(12), None),
                    ]
                })
                .unwrap()
        })
        .await;

        // Assert
        assert_eq!(admitted, [true, true, false, false]);
    }

    #[tokio::test]
    async fn concurrent_public_task_cannot_inherit_scratch_permit() {
        // Arrange
        let identity = Arc::new(());
        let permit = scratch_permit(&identity);
        let topology = scratch_topology();

        // Act
        let (inside, concurrent_outside) = with_internal_page_permit(&permit, async {
            let inside = INTERNAL_PAGE_PERMIT
                .try_with(|active| active.matches(&identity, &topology, ObjectId::new(9), None))
                .unwrap();
            let outside = tokio::spawn(async { INTERNAL_PAGE_PERMIT.try_with(|_| ()).is_err() })
                .await
                .unwrap();
            (inside, outside)
        })
        .await;

        // Assert
        assert!(inside);
        assert!(concurrent_outside);
        assert!(INTERNAL_PAGE_PERMIT.try_with(|_| ()).is_err());
    }

    #[test]
    fn installation_replacement_or_reconnect_revokes_scratch_permit() {
        // Arrange
        let identity = Arc::new(());
        let permit = scratch_permit(&identity);
        let mut topology = scratch_topology();

        // Act
        let retired_identity = Arc::new(());
        let after_retirement = permit.matches(&retired_identity, &topology, ObjectId::new(9), None);
        topology.begin_reconnect(ConnectionEpoch::new(2)).unwrap();
        let after_reconnect = permit.matches(&identity, &topology, ObjectId::new(9), None);

        // Assert
        assert!(!after_reconnect);
        assert!(!after_retirement);
    }

    #[test]
    fn retired_page_route_revokes_scratch_permit() {
        // Arrange
        let identity = Arc::new(());
        let permit = scratch_permit(&identity);
        let mut topology = scratch_topology();

        // Act
        topology
            .detach_target(TEST_EPOCH, "scratch-session", Some("scratch-page"))
            .unwrap();
        let after_retirement = permit.matches(&identity, &topology, ObjectId::new(9), None);

        // Assert
        assert!(!after_retirement);
    }

    #[tokio::test]
    async fn cancelled_owner_can_scope_its_detached_cleanup_without_public_leak() {
        struct CleanupProbe {
            permit: InternalPagePermit,
            report: Option<tokio::sync::oneshot::Sender<bool>>,
        }

        impl Drop for CleanupProbe {
            fn drop(&mut self) {
                let permit = self.permit.clone();
                let report = self.report.take().unwrap();
                tokio::spawn(async move {
                    let scoped = with_internal_page_permit(&permit, async {
                        INTERNAL_PAGE_PERMIT.try_with(|_| ()).is_ok()
                    })
                    .await;
                    let _ = report.send(scoped);
                });
            }
        }

        // Arrange
        let identity = Arc::new(());
        let permit = scratch_permit(&identity);
        let (report, completion) = tokio::sync::oneshot::channel();
        let (started, ready) = tokio::sync::oneshot::channel();
        let owner = tokio::spawn(async move {
            let _probe = CleanupProbe {
                permit,
                report: Some(report),
            };
            let _ = started.send(());
            std::future::pending::<()>().await;
        });
        ready.await.unwrap();

        // Act
        owner.abort();
        let _ = owner.await;
        let scoped_cleanup = tokio::time::timeout(std::time::Duration::from_secs(1), completion)
            .await
            .unwrap()
            .unwrap();

        // Assert
        assert!(scoped_cleanup);
        assert!(INTERNAL_PAGE_PERMIT.try_with(|_| ()).is_err());
    }

    struct DropSignal(Option<tokio::sync::oneshot::Sender<()>>);

    impl Drop for DropSignal {
        fn drop(&mut self) {
            if let Some(signal) = self.0.take() {
                let _ = signal.send(());
            }
        }
    }

    #[tokio::test]
    async fn abandoned_creation_receiver_still_drops_completed_guard() {
        // Arrange: the caller goes away while native creation is suspended.
        let (started, begun) = tokio::sync::oneshot::channel();
        let (release, resume) = tokio::sync::oneshot::channel();
        let (dropped, cleaned) = tokio::sync::oneshot::channel();
        let caller = tokio::spawn(bounded_detached_result(async move {
            let _ = started.send(());
            let _ = resume.await;
            Ok(DropSignal(Some(dropped)))
        }));
        begun.await.unwrap();

        // Act
        caller.abort();
        let _ = caller.await;
        release.send(()).unwrap();

        // Assert: the detached bounded work finishes, and the unreceived
        // guard is dropped even though the original caller did not resume.
        tokio::time::timeout(std::time::Duration::from_secs(1), cleaned)
            .await
            .unwrap()
            .unwrap();
    }

    #[test]
    fn origin_book_puts_current_page_before_previous_origin() {
        // Arrange
        let context = ObjectId::new(1);
        let mut state = State {
            contexts: vec![ContextEntry {
                id: context,
                origins: vec![
                    "https://www.example.com".into(),
                    "https://www.domain.com".into(),
                ],
                overflowed: false,
            }],
            frames: vec![FrameEntry {
                id: ObjectId::new(3),
                page: ObjectId::new(2),
                context,
                main: true,
                origin: Some("https://www.domain.com".into()),
            }],
            pending: Vec::new(),
            overflowed_unknown: false,
            closed: false,
        };
        // Act
        insert_origin(&mut state, context, "https://www.domain.com".into());
        // Assert
        assert_eq!(state.contexts[0].origins.len(), 2);
    }

    #[test]
    fn non_network_urls_do_not_enter_origin_book() {
        // Arrange
        let url = "about:blank";
        // Act
        let origin = origin_from_url(url);
        // Assert
        assert!(origin.is_none());
    }

    #[test]
    fn state_rejects_non_origin_url() {
        // Arrange
        let value = ContextStorageOrigin {
            origin: "https://example.com/path".into(),
            local_storage: Vec::new(),
            indexed_db: None,
        };
        // Act
        let error = validate_origin(&value).unwrap_err();
        // Assert
        assert!(error.to_string().contains("canonical HTTP(S) origin"));
    }
}
