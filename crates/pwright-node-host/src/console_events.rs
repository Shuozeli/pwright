//! Optional Page console publication over the existing ordered topology
//! cursor and Node publication fence. This leaf installs no task, transport,
//! browser process, or JavaScript callback of its own.

use std::error::Error;
use std::fmt;
use std::io;
use std::sync::Arc;

#[cfg(any(feature = "console-history", feature = "console-argument-handles"))]
use std::collections::VecDeque;

use pwright_backend_cdp_topology::CdpTopology;
use pwright_cdp::{CdpConnection, CdpEvent, generated::page};
use pwright_core::ObjectId;
#[cfg(feature = "console-history")]
use pwright_core::{
    ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequest, ResourceRequirements,
    ResourceScope,
};
use pwright_plugin_chromium_frame_observer::{ChromiumFrameEventObserver, FrameObservationResult};
use pwright_plugin_chromium_session_initialization::{
    ChromiumSessionInitializationFuture, ChromiumSessionInitializationTarget,
    ChromiumSessionInitializer,
};
use pwright_runtime::{
    CleanupFailure, CleanupReservation, CleanupSubmitError, ReservedCleanupExecutor,
};
use pwright_runtime_tokio::TokioCleanupExecutor;
#[cfg(feature = "console-argument-handles")]
use pwright_runtime_tokio::TokioCleanupReservation;
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};
use serde_json::Value;

#[cfg(feature = "console-events")]
use crate::frame_trees::FrameTrees;
use crate::{ConsoleMessage, HostError};

pub(crate) const MAX_LOG_RELEASE_JOBS: usize = 512;
const MAX_LOG_ARGUMENTS: usize = 16;
const MAX_LOG_OBJECT_ID_BYTES: usize = 4096;

#[cfg(feature = "console-events")]
const MAX_SUBSCRIBED_PAGES: usize = 16;
#[cfg(feature = "console-events")]
const MAX_SUBSCRIBED_CONTEXTS: usize = 16;
#[cfg(feature = "console-events")]
const MAX_CONSOLE_WAITERS_PER_TARGET: usize = 32;

#[cfg(feature = "console-events")]
struct SubscriptionState {
    pages: Vec<SubscriptionEntry>,
    contexts: Vec<SubscriptionEntry>,
    next_waiter_token: u64,
}

#[cfg(feature = "console-events")]
struct SubscriptionEntry {
    id: ObjectId,
    listeners: u16,
    waiters: Vec<WaiterSubscription>,
}

#[cfg(feature = "console-events")]
struct WaiterSubscription {
    token: u64,
    active: bool,
}

#[cfg(feature = "console-events")]
pub(crate) struct Subscriptions {
    state: std::sync::Mutex<SubscriptionState>,
}

#[cfg(feature = "console-events")]
impl Subscriptions {
    pub(crate) fn new() -> Self {
        Self {
            state: std::sync::Mutex::new(SubscriptionState {
                pages: Vec::with_capacity(MAX_SUBSCRIBED_PAGES),
                contexts: Vec::with_capacity(MAX_SUBSCRIBED_CONTEXTS),
                next_waiter_token: 1,
            }),
        }
    }

    fn entry_index(
        entries: &mut Vec<SubscriptionEntry>,
        id: ObjectId,
        limit: usize,
        label: &str,
    ) -> Result<usize, HostError> {
        if let Some(index) = entries.iter().position(|entry| entry.id == id) {
            return Ok(index);
        }
        if entries.len() == limit {
            return Err(HostError::ResourceExhausted(format!(
                "console {label} subscriptions exceed {limit}"
            )));
        }
        entries.push(SubscriptionEntry {
            id,
            listeners: 0,
            waiters: Vec::new(),
        });
        Ok(entries.len() - 1)
    }

    fn update_listener(
        entries: &mut Vec<SubscriptionEntry>,
        id: ObjectId,
        enabled: bool,
        limit: usize,
        label: &str,
    ) -> Result<(), HostError> {
        if !enabled {
            let Some(index) = entries.iter().position(|entry| entry.id == id) else {
                return Ok(());
            };
            entries[index].listeners = entries[index].listeners.saturating_sub(1);
            if entries[index].listeners == 0 && entries[index].waiters.is_empty() {
                entries.swap_remove(index);
            }
            return Ok(());
        }
        let index = Self::entry_index(entries, id, limit, label)?;
        entries[index].listeners = entries[index].listeners.checked_add(1).ok_or_else(|| {
            HostError::ResourceExhausted(format!("{label} listener count exhausted"))
        })?;
        Ok(())
    }

    fn subscribe_waiter(
        entries: &mut Vec<SubscriptionEntry>,
        id: ObjectId,
        token: u64,
        limit: usize,
        label: &str,
    ) -> Result<(), HostError> {
        let index = Self::entry_index(entries, id, limit, label)?;
        if entries[index].waiters.len() == MAX_CONSOLE_WAITERS_PER_TARGET {
            return Err(HostError::ResourceExhausted(format!(
                "console {label} waiters exceed {MAX_CONSOLE_WAITERS_PER_TARGET}"
            )));
        }
        entries[index]
            .waiters
            .try_reserve_exact(1)
            .map_err(HostError::backend)?;
        entries[index].waiters.push(WaiterSubscription {
            token,
            active: false,
        });
        Ok(())
    }

    fn next_waiter_token(state: &mut SubscriptionState) -> Result<u64, HostError> {
        let token = state.next_waiter_token;
        state.next_waiter_token = state.next_waiter_token.checked_add(1).ok_or_else(|| {
            HostError::ResourceExhausted("console waiter token space exhausted".into())
        })?;
        Ok(token)
    }

    pub(crate) fn update_page_listener(
        &self,
        page: ObjectId,
        enabled: bool,
    ) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        Self::update_listener(
            &mut state.pages,
            page,
            enabled,
            MAX_SUBSCRIBED_PAGES,
            "Page",
        )
    }

    pub(crate) fn update_context_listener(
        &self,
        context: ObjectId,
        enabled: bool,
    ) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        Self::update_listener(
            &mut state.contexts,
            context,
            enabled,
            MAX_SUBSCRIBED_CONTEXTS,
            "BrowserContext",
        )
    }

    pub(crate) fn subscribe_page_waiter(&self, page: ObjectId) -> Result<u64, HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        let token = Self::next_waiter_token(&mut state)?;
        Self::subscribe_waiter(&mut state.pages, page, token, MAX_SUBSCRIBED_PAGES, "Page")?;
        Ok(token)
    }

    pub(crate) fn subscribe_context_waiter(&self, context: ObjectId) -> Result<u64, HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        let token = Self::next_waiter_token(&mut state)?;
        Self::subscribe_waiter(
            &mut state.contexts,
            context,
            token,
            MAX_SUBSCRIBED_CONTEXTS,
            "BrowserContext",
        )?;
        Ok(token)
    }

    fn release_waiter_from(entries: &mut Vec<SubscriptionEntry>, token: u64) -> bool {
        let Some(entry_index) = entries
            .iter()
            .position(|entry| entry.waiters.iter().any(|waiter| waiter.token == token))
        else {
            return false;
        };
        let waiter_index = entries[entry_index]
            .waiters
            .iter()
            .position(|waiter| waiter.token == token)
            .expect("waiter token was located in this entry");
        entries[entry_index].waiters.swap_remove(waiter_index);
        if entries[entry_index].listeners == 0 && entries[entry_index].waiters.is_empty() {
            entries.swap_remove(entry_index);
        }
        true
    }

    pub(crate) fn release_waiter(&self, token: u64) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if !Self::release_waiter_from(&mut state.pages, token) {
            Self::release_waiter_from(&mut state.contexts, token);
        }
        Ok(())
    }

    pub(crate) fn activate_pending(&self) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        for entry in &mut state.pages {
            for waiter in &mut entry.waiters {
                waiter.active = true;
            }
        }
        for entry in &mut state.contexts {
            for waiter in &mut entry.waiters {
                waiter.active = true;
            }
        }
        Ok(())
    }

    fn activate_for_page(&self, topology: &CdpTopology, page: ObjectId) -> Result<(), HostError> {
        let context = topology
            .page_session_routes()
            .find(|route| route.object_id() == page)
            .map(|route| route.browser_context_object_id());
        let mut state = self.state.lock().map_err(HostError::backend)?;
        for entry in state.pages.iter_mut().filter(|entry| entry.id == page) {
            for waiter in &mut entry.waiters {
                waiter.active = true;
            }
        }
        if let Some(context) = context {
            for entry in state
                .contexts
                .iter_mut()
                .filter(|entry| entry.id == context)
            {
                for waiter in &mut entry.waiters {
                    waiter.active = true;
                }
            }
        }
        Ok(())
    }

    fn contains(entries: &[SubscriptionEntry], id: ObjectId) -> bool {
        entries.iter().any(|entry| {
            entry.id == id
                && (entry.listeners != 0 || entry.waiters.iter().any(|waiter| waiter.active))
        })
    }

    fn publishes(&self, topology: &CdpTopology, page: ObjectId) -> Result<bool, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if Self::contains(&state.pages, page) {
            return Ok(true);
        }
        Ok(topology
            .page_session_routes()
            .find(|route| route.object_id() == page)
            .is_some_and(|route| {
                Self::contains(&state.contexts, route.browser_context_object_id())
            }))
    }

    fn retain(&self, topology: &CdpTopology) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        state.pages.retain(|entry| {
            topology
                .page_session_routes()
                .any(|route| route.object_id() == entry.id)
        });
        state.contexts.retain(|entry| {
            topology
                .browser_context_routes()
                .any(|route| route.object_id() == entry.id)
        });
        Ok(())
    }

    fn clear(&self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.pages.clear();
        state.contexts.clear();
    }
}

#[cfg(feature = "console-history")]
const MAX_HISTORY_PAGES: usize = 16;
#[cfg(feature = "console-history")]
pub(crate) const MAX_CONSOLE_MESSAGES_PER_PAGE: usize = 200;
#[cfg(feature = "console-history")]
const MAX_CONSOLE_HISTORY_ITEMS: u64 = (MAX_HISTORY_PAGES * MAX_CONSOLE_MESSAGES_PER_PAGE) as u64;
#[cfg(feature = "console-history")]
const MAX_CONSOLE_HISTORY_BYTES: u64 = 16 * 1024 * 1024;
#[cfg(feature = "console-history")]
const HISTORY_SCOPE: ResourceScope = ResourceScope::extension(176, 1);

#[cfg(feature = "console-history")]
struct StoredMessage {
    sequence: u64,
    message: Arc<ConsoleMessage>,
    _lease: ResourceLease,
}

#[cfg(feature = "console-history")]
struct PageHistory {
    page: Option<ObjectId>,
    messages: VecDeque<StoredMessage>,
}

#[cfg(feature = "console-history")]
impl PageHistory {
    fn new() -> Result<Self, HostError> {
        let mut messages = VecDeque::new();
        messages
            .try_reserve_exact(MAX_CONSOLE_MESSAGES_PER_PAGE)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        Ok(Self {
            page: None,
            messages,
        })
    }

    fn clear(&mut self) {
        self.messages.clear();
        self.page = None;
    }
}

#[cfg(feature = "console-history")]
struct HistoryState {
    next_sequence: u64,
    pages: Vec<PageHistory>,
    closed: bool,
}

#[cfg(feature = "console-history")]
pub(crate) const fn requirements() -> ResourceRequirements {
    let storage_bytes = (std::mem::size_of::<Installation>()
        + std::mem::size_of::<HistoryState>()
        + MAX_HISTORY_PAGES
            * (std::mem::size_of::<PageHistory>()
                + MAX_CONSOLE_MESSAGES_PER_PAGE * std::mem::size_of::<StoredMessage>()))
        as u64;
    ResourceRequirements::none()
        .with_class(
            ResourceClass::EventItem,
            MAX_CONSOLE_HISTORY_ITEMS,
            MAX_CONSOLE_MESSAGES_PER_PAGE as u64,
        )
        .with_class(
            ResourceClass::EventBytes,
            MAX_CONSOLE_HISTORY_BYTES,
            MAX_CONSOLE_HISTORY_BYTES,
        )
        .with_class(
            ResourceClass::BackendStateBytes,
            storage_bytes,
            storage_bytes,
        )
        .with_metadata(
            MAX_HISTORY_PAGES + 1,
            MAX_CONSOLE_HISTORY_ITEMS as usize + 1,
        )
}

#[cfg(feature = "console-history")]
pub(crate) struct Installation {
    resources: pwright_core::ResourceLedger,
    state: std::sync::Mutex<HistoryState>,
    _storage: ResourceLease,
}

#[cfg(feature = "console-history")]
impl Installation {
    pub(crate) fn new(resources: &ProductionResourceLedger) -> Result<Arc<Self>, HostError> {
        let plan = requirements();
        let ledger = resources.claim(plan).map_err(HostError::backend)?;
        let storage = ledger
            .reserve(
                HISTORY_SCOPE,
                ResourceClass::BackendStateBytes,
                plan.class(ResourceClass::BackendStateBytes).global,
            )
            .map_err(HostError::backend)?;
        let mut pages = Vec::new();
        pages
            .try_reserve_exact(MAX_HISTORY_PAGES)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        for _ in 0..MAX_HISTORY_PAGES {
            pages.push(PageHistory::new()?);
        }
        Ok(Arc::new(Self {
            resources: ledger,
            state: std::sync::Mutex::new(HistoryState {
                next_sequence: 1,
                pages,
                closed: false,
            }),
            _storage: storage,
        }))
    }

    fn requests(storage_bytes: usize) -> Result<[ResourceRequest; 2], HostError> {
        let bytes = u64::try_from(storage_bytes)
            .map_err(|_| HostError::ResourceExhausted("console history size overflow".into()))?;
        if bytes > MAX_CONSOLE_HISTORY_BYTES {
            return Err(HostError::ResourceExhausted(
                "console message exceeds the history byte capacity".into(),
            ));
        }
        Ok([
            ResourceRequest::new(ResourceClass::EventItem, 1),
            ResourceRequest::new(ResourceClass::EventBytes, bytes),
        ])
    }

    fn oldest_page(state: &HistoryState) -> Option<usize> {
        state
            .pages
            .iter()
            .enumerate()
            .filter_map(|(index, page)| {
                page.messages
                    .front()
                    .map(|message| (index, message.sequence))
            })
            .min_by_key(|(_, sequence)| *sequence)
            .map(|(index, _)| index)
    }

    fn record(
        &self,
        page: ObjectId,
        storage_bytes: usize,
        message: Arc<ConsoleMessage>,
    ) -> Result<(), HostError> {
        let requests = Self::requests(storage_bytes)?;
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        let page_index = state
            .pages
            .iter()
            .position(|history| history.page == Some(page))
            .or_else(|| {
                state
                    .pages
                    .iter()
                    .position(|history| history.page.is_none())
            })
            .ok_or_else(|| {
                HostError::ResourceExhausted(format!(
                    "console history exceeds {MAX_HISTORY_PAGES} live Pages"
                ))
            })?;
        let next_sequence = state
            .next_sequence
            .checked_add(1)
            .ok_or_else(|| HostError::ResourceExhausted("console sequence exhausted".into()))?;
        let sequence = std::mem::replace(&mut state.next_sequence, next_sequence);
        let scope = ResourceScope::Object(page);

        if state.pages[page_index].messages.len() == MAX_CONSOLE_MESSAGES_PER_PAGE {
            let mut oldest = state.pages[page_index]
                .messages
                .pop_front()
                .expect("a full console history has an oldest item");
            loop {
                match oldest._lease.try_replace(scope, &requests) {
                    Ok(()) => break,
                    Err(error) => {
                        let eviction = state.pages[page_index]
                            .messages
                            .front()
                            .map(|message| message.sequence)
                            .map(|sequence| (page_index, sequence))
                            .or_else(|| {
                                Self::oldest_page(&state).and_then(|index| {
                                    state.pages[index]
                                        .messages
                                        .front()
                                        .map(|message| (index, message.sequence))
                                })
                            });
                        let Some((eviction_page, _)) = eviction else {
                            state.pages[page_index].messages.push_front(oldest);
                            state.next_sequence = sequence;
                            return Err(HostError::ResourceExhausted(error.to_string()));
                        };
                        state.pages[eviction_page].messages.pop_front();
                    }
                }
            }
            state.pages[page_index].messages.push_back(StoredMessage {
                sequence,
                message,
                _lease: oldest._lease,
            });
            return Ok(());
        }

        let lease = loop {
            match self.resources.reserve_many(scope, &requests) {
                Ok(lease) => break lease,
                Err(error) => {
                    let Some(oldest_page) = Self::oldest_page(&state) else {
                        state.next_sequence = sequence;
                        return Err(HostError::ResourceExhausted(error.to_string()));
                    };
                    state.pages[oldest_page].messages.pop_front();
                }
            }
        };
        state.pages[page_index].page = Some(page);
        state.pages[page_index].messages.push_back(StoredMessage {
            sequence,
            message,
            _lease: lease,
        });
        Ok(())
    }

    fn snapshot(&self, page: ObjectId) -> Result<Vec<Arc<ConsoleMessage>>, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        Ok(state
            .pages
            .iter()
            .find(|history| history.page == Some(page))
            .map(|history| {
                history
                    .messages
                    .iter()
                    .map(|stored| Arc::clone(&stored.message))
                    .collect()
            })
            .unwrap_or_default())
    }

    fn retain(&self, topology: &CdpTopology) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        for history in &mut state.pages {
            if history.page.is_some_and(|page| {
                !topology
                    .page_session_routes()
                    .any(|route| route.object_id() == page)
            }) {
                history.clear();
            }
        }
        Ok(())
    }

    fn close(&self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.closed = true;
        for history in &mut state.pages {
            history.clear();
        }
    }
}

pub(crate) struct Initializer<Downstream> {
    downstream: Downstream,
    connection: Arc<CdpConnection>,
}

impl<Downstream> Initializer<Downstream> {
    pub(crate) fn new(downstream: Downstream, connection: Arc<CdpConnection>) -> Self {
        Self {
            downstream,
            connection,
        }
    }

    async fn enable(&self, session_id: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.connection
            .send_command_with_session(
                "Log.enable",
                Value::Object(serde_json::Map::new()),
                Some(session_id),
            )
            .await?;
        Ok(())
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
        Box::pin(async move {
            let downstream = self
                .downstream
                .initialize(session_id, target_id, ownership, is_page);
            let enable = self.enable(session_id);
            tokio::try_join!(downstream, enable).map(|_| ())
        })
    }

    fn initialize_scoped<'a>(
        &'a self,
        target: ChromiumSessionInitializationTarget<'a>,
    ) -> ChromiumSessionInitializationFuture<'a> {
        Box::pin(async move {
            let downstream = self.downstream.initialize_scoped(target);
            let enable = self.enable(target.session_id);
            tokio::try_join!(downstream, enable).map(|_| ())
        })
    }
}

pub(crate) struct Observer {
    #[cfg(feature = "console-events")]
    trees: Arc<FrameTrees>,
    #[cfg(feature = "console-events")]
    subscriptions: Arc<Subscriptions>,
    #[cfg(feature = "console-history")]
    history: Arc<Installation>,
    downstream: Arc<dyn ChromiumFrameEventObserver>,
    connection: Arc<CdpConnection>,
    cleanup: TokioCleanupExecutor,
    #[cfg(feature = "console-argument-handles")]
    handles: Arc<crate::handles::Installation>,
    #[cfg(feature = "console-argument-handles")]
    pending: std::sync::Mutex<VecDeque<PendingConsoleMessage>>,
}

#[cfg(feature = "console-argument-handles")]
struct PendingConsoleMessage {
    page: ObjectId,
    publish: bool,
    #[cfg(feature = "console-history")]
    history_storage_bytes: usize,
    event_storage_bytes: usize,
    message: ConsoleMessage,
    handles: Option<crate::handles::PendingEventHandles>,
}

impl Observer {
    pub(crate) fn new(
        #[cfg(feature = "console-events")] trees: Arc<FrameTrees>,
        #[cfg(feature = "console-events")] subscriptions: Arc<Subscriptions>,
        #[cfg(feature = "console-history")] history: Arc<Installation>,
        downstream: Arc<dyn ChromiumFrameEventObserver>,
        connection: Arc<CdpConnection>,
        cleanup: TokioCleanupExecutor,
        #[cfg(feature = "console-argument-handles")] handles: Arc<crate::handles::Installation>,
    ) -> Self {
        Self {
            #[cfg(feature = "console-events")]
            trees,
            #[cfg(feature = "console-events")]
            subscriptions,
            #[cfg(feature = "console-history")]
            history,
            downstream,
            connection,
            cleanup,
            #[cfg(feature = "console-argument-handles")]
            handles,
            #[cfg(feature = "console-argument-handles")]
            pending: std::sync::Mutex::new(VecDeque::with_capacity(crate::handles::MAX_CALLS)),
        }
    }
}

fn invalid(field: &str) -> Box<dyn Error + Send + Sync> {
    Box::new(HostError::Backend(format!(
        "Runtime console event is missing {field}"
    )))
}

fn write_argument(output: &mut impl fmt::Write, argument: &Value) -> fmt::Result {
    let argument = argument.as_object().ok_or(fmt::Error)?;
    if let Some(value) = argument.get("value") {
        return match value {
            Value::String(value) => output.write_str(value),
            Value::Null => output.write_str("null"),
            Value::Bool(value) => output.write_str(if *value { "true" } else { "false" }),
            Value::Number(value) => write!(output, "{value}"),
            Value::Array(_) | Value::Object(_) => write_preview(output, argument),
        };
    }
    if let Some(value) = argument.get("unserializableValue").and_then(Value::as_str) {
        return output.write_str(value);
    }
    if argument
        .get("description")
        .and_then(Value::as_str)
        .is_some()
    {
        return write_preview(output, argument);
    }
    match argument.get("type").and_then(Value::as_str) {
        Some("undefined") => output.write_str("undefined"),
        Some("object") if argument.get("subtype").and_then(Value::as_str) == Some("null") => {
            output.write_str("null")
        }
        Some(value) => output.write_str(value),
        None => Err(fmt::Error),
    }
}

fn write_preview_property(
    output: &mut impl fmt::Write,
    property: &serde_json::Map<String, Value>,
) -> fmt::Result {
    if let Some(value) = property.get("value").and_then(Value::as_str) {
        return output.write_str(value);
    }
    if let Some(preview) = property.get("valuePreview").and_then(Value::as_object) {
        return write_preview(output, preview);
    }
    output.write_str(
        property
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("undefined"),
    )
}

fn write_preview(
    output: &mut impl fmt::Write,
    argument: &serde_json::Map<String, Value>,
) -> fmt::Result {
    if argument.get("type").and_then(Value::as_str) == Some("undefined") {
        return output.write_str("undefined");
    }
    if let Some(value) = argument.get("value") {
        return match value {
            Value::String(value) => output.write_str(value),
            _ => write!(output, "{value}"),
        };
    }
    if let Some(value) = argument.get("unserializableValue").and_then(Value::as_str) {
        return output.write_str(value);
    }
    let description = argument
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("Object");
    let preview = argument.get("preview").and_then(Value::as_object);
    let properties = preview
        .and_then(|preview| preview.get("properties"))
        .and_then(Value::as_array);
    if description == "Object"
        && let Some(properties) = properties
    {
        output.write_char('{')?;
        let mut first = true;
        for property in properties.iter().filter_map(Value::as_object) {
            if !first {
                output.write_str(", ")?;
            }
            first = false;
            output.write_str(property.get("name").and_then(Value::as_str).unwrap_or(""))?;
            output.write_str(": ")?;
            write_preview_property(output, property)?;
        }
        return output.write_char('}');
    }
    if argument.get("subtype").and_then(Value::as_str) == Some("array")
        && let Some(properties) = properties
    {
        let mut entries = properties
            .iter()
            .filter_map(Value::as_object)
            .filter_map(|property| {
                property
                    .get("name")
                    .and_then(Value::as_str)
                    .and_then(|value| value.parse::<usize>().ok())
                    .map(|index| (index, property))
            })
            .collect::<Vec<_>>();
        entries.sort_unstable_by_key(|(index, _)| *index);
        output.write_char('[')?;
        let mut previous = None;
        for (entry, (index, property)) in entries.into_iter().enumerate() {
            if entry != 0 {
                output.write_str(", ")?;
            }
            let empty = previous.map_or(index, |previous| index.saturating_sub(previous + 1));
            match empty {
                0 => {}
                1 => output.write_str("empty, ")?,
                count => write!(output, "empty x {count}, ")?,
            }
            write_preview_property(output, property)?;
            previous = Some(index);
        }
        return output.write_char(']');
    }
    output.write_str(description)
}

enum PropertyJson<'a> {
    String(&'a str),
    Number(serde_json::Number),
    Bool(bool),
    Null,
}

impl Serialize for PropertyJson<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::String(value) => serializer.serialize_str(value),
            Self::Number(value) => value.serialize(serializer),
            Self::Bool(value) => serializer.serialize_bool(*value),
            Self::Null => serializer.serialize_none(),
        }
    }
}

fn property_json(property: &serde_json::Map<String, Value>) -> Option<PropertyJson<'_>> {
    match property.get("type").and_then(Value::as_str) {
        Some("string") => property
            .get("value")
            .and_then(Value::as_str)
            .map(PropertyJson::String),
        Some("number") => property
            .get("value")
            .and_then(Value::as_str)
            .and_then(|value| value.parse::<serde_json::Number>().ok())
            .map(PropertyJson::Number),
        Some("boolean") => property
            .get("value")
            .and_then(Value::as_str)
            .and_then(|value| value.parse::<bool>().ok())
            .map(PropertyJson::Bool),
        Some("object") if property.get("subtype").and_then(Value::as_str) == Some("null") => {
            Some(PropertyJson::Null)
        }
        _ => None,
    }
}

struct ArgumentJson<'a>(&'a Value);

impl Serialize for ArgumentJson<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let Some(argument) = self.0.as_object() else {
            return serializer.serialize_none();
        };
        if let Some(value) = argument.get("value") {
            return value.serialize(serializer);
        }
        let properties = argument
            .get("preview")
            .and_then(Value::as_object)
            .and_then(|preview| preview.get("properties"))
            .and_then(Value::as_array);
        if argument.get("description").and_then(Value::as_str) == Some("Object")
            && let Some(properties) = properties
        {
            let mut map = serializer.serialize_map(None)?;
            for property in properties.iter().filter_map(Value::as_object) {
                let Some(name) = property.get("name").and_then(Value::as_str) else {
                    continue;
                };
                if let Some(value) = property_json(property) {
                    map.serialize_entry(name, &value)?;
                }
            }
            return map.end();
        }
        if argument.get("subtype").and_then(Value::as_str) == Some("array")
            && let Some(properties) = properties
        {
            let largest = properties
                .iter()
                .filter_map(Value::as_object)
                .filter_map(|property| property.get("name"))
                .filter_map(Value::as_str)
                .filter_map(|value| value.parse::<usize>().ok())
                .max();
            let mut sequence = serializer.serialize_seq(largest.map(|index| index + 1))?;
            if let Some(largest) = largest {
                for index in 0..=largest {
                    let value = properties
                        .iter()
                        .rev()
                        .filter_map(Value::as_object)
                        .find(|property| {
                            property
                                .get("name")
                                .and_then(Value::as_str)
                                .and_then(|value| value.parse::<usize>().ok())
                                == Some(index)
                        })
                        .and_then(property_json);
                    sequence.serialize_element(&value)?;
                }
            }
            return sequence.end();
        }
        serializer.serialize_none()
    }
}

struct ArgumentsJson<'a>(&'a [Value]);

impl Serialize for ArgumentsJson<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for argument in self.0 {
            sequence.serialize_element(&ArgumentJson(argument))?;
        }
        sequence.end()
    }
}

struct BoundedCounter {
    bytes: usize,
    limit: usize,
}

impl BoundedCounter {
    const fn new(limit: usize) -> Self {
        Self { bytes: 0, limit }
    }

    fn admit(&mut self, additional: usize) -> bool {
        let Some(next) = self.bytes.checked_add(additional) else {
            return false;
        };
        if next > self.limit {
            return false;
        }
        self.bytes = next;
        true
    }
}

impl fmt::Write for BoundedCounter {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        self.admit(value.len()).then_some(()).ok_or(fmt::Error)
    }
}

impl io::Write for BoundedCounter {
    fn write(&mut self, value: &[u8]) -> io::Result<usize> {
        if self.admit(value.len()) {
            Ok(value.len())
        } else {
            Err(io::Error::other(
                "bounded console serialization exceeded limit",
            ))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct StringWriter<'a>(&'a mut String);

impl io::Write for StringWriter<'_> {
    fn write(&mut self, value: &[u8]) -> io::Result<usize> {
        let value = std::str::from_utf8(value)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        self.0.push_str(value);
        Ok(value.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn write_console_text(output: &mut impl fmt::Write, args: &[Value]) -> fmt::Result {
    for (index, argument) in args.iter().enumerate() {
        if index != 0 {
            output.write_char(' ')?;
        }
        write_argument(output, argument)?;
    }
    Ok(())
}

fn write_arguments_json(output: impl io::Write, args: &[Value]) -> Result<(), serde_json::Error> {
    let mut serializer = serde_json::Serializer::new(output);
    ArgumentsJson(args).serialize(&mut serializer)
}

fn console_location(params: &Value) -> Result<(&str, u32, u32), HostError> {
    let Some(frame) = params
        .get("stackTrace")
        .and_then(|value| value.get("callFrames"))
        .and_then(Value::as_array)
        .and_then(|frames| frames.first())
        .and_then(Value::as_object)
    else {
        return Ok(("", 0, 0));
    };
    let url = frame.get("url").and_then(Value::as_str).unwrap_or("");
    let number = |field: &str| {
        frame
            .get(field)
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| HostError::Backend(format!("console location is missing {field}")))
    };
    Ok((url, number("lineNumber")?, number("columnNumber")?))
}

fn console_timestamp_micros(value: Option<&Value>) -> Result<u64, HostError> {
    let Some(timestamp_ms) = value.and_then(Value::as_f64) else {
        return Ok(0);
    };
    let timestamp_micros = timestamp_ms * 1_000.0;
    if !timestamp_micros.is_finite()
        || timestamp_micros.is_sign_negative()
        || timestamp_micros > u64::MAX as f64
    {
        return Err(HostError::Backend(
            "console event timestamp is outside the supported range".into(),
        ));
    }
    Ok(timestamp_micros as u64)
}

pub(crate) struct ConsoleMessagePlan<'a> {
    message_type: &'a str,
    args: &'a [Value],
    location_url: &'a str,
    line_number: u32,
    column_number: u32,
    timestamp_micros: u64,
    text_bytes: usize,
    argument_bytes: usize,
    storage_bytes: usize,
}

impl<'a> ConsoleMessagePlan<'a> {
    pub(crate) fn new(
        message_type: &'a str,
        args: &'a [Value],
        params: &'a Value,
    ) -> Result<Self, HostError> {
        if message_type.len() > crate::frame_metadata::MAX_CONSOLE_TYPE_BYTES {
            return Err(HostError::ResourceExhausted(
                "console message type exceeds byte limit".into(),
            ));
        }
        let (location_url, line_number, column_number) = console_location(params)?;
        let timestamp_micros = console_timestamp_micros(params.get("timestamp"))?;
        if location_url.len() > crate::frame_metadata::MAX_CONSOLE_LOCATION_URL_BYTES {
            return Err(HostError::ResourceExhausted(
                "console message location URL exceeds byte limit".into(),
            ));
        }
        let mut text = BoundedCounter::new(ConsoleMessage::maximum_text_bytes());
        write_console_text(&mut text, args).map_err(|_| {
            HostError::ResourceExhausted("console message text exceeds byte limit".into())
        })?;
        let mut arguments = BoundedCounter::new(crate::frame_metadata::MAX_CONSOLE_ARGUMENT_BYTES);
        write_arguments_json(&mut arguments, args).map_err(|_| {
            HostError::ResourceExhausted(
                "console message argument snapshot exceeds byte limit".into(),
            )
        })?;
        let storage_bytes = ConsoleMessage::storage_bytes(
            message_type.len(),
            text.bytes,
            arguments.bytes,
            location_url.len(),
        )?;
        Ok(Self {
            message_type,
            args,
            location_url,
            line_number,
            column_number,
            timestamp_micros,
            text_bytes: text.bytes,
            argument_bytes: arguments.bytes,
            storage_bytes,
        })
    }

    pub(crate) const fn storage_bytes(&self) -> usize {
        self.storage_bytes
    }

    pub(crate) fn build(self) -> Result<ConsoleMessage, HostError> {
        let mut text = String::with_capacity(self.text_bytes);
        write_console_text(&mut text, self.args).map_err(HostError::backend)?;
        let mut arguments_json = String::with_capacity(self.argument_bytes);
        write_arguments_json(StringWriter(&mut arguments_json), self.args)
            .map_err(HostError::backend)?;
        debug_assert_eq!(text.len(), self.text_bytes);
        debug_assert_eq!(arguments_json.len(), self.argument_bytes);
        ConsoleMessage::new(
            self.message_type.to_owned(),
            text,
            arguments_json,
            self.location_url.to_owned(),
            self.line_number,
            self.column_number,
            self.timestamp_micros,
        )
    }
}

struct LogMessagePlan<'a> {
    message_type: &'a str,
    text: &'a str,
    location_url: &'a str,
    line_number: u32,
    timestamp_micros: u64,
    storage_bytes: usize,
}

impl<'a> LogMessagePlan<'a> {
    fn new(entry: &'a serde_json::Map<String, Value>) -> Result<Self, HostError> {
        let message_type = entry
            .get("level")
            .and_then(Value::as_str)
            .ok_or_else(|| HostError::Backend("Log entry is missing level".into()))?;
        let text = entry
            .get("text")
            .and_then(Value::as_str)
            .ok_or_else(|| HostError::Backend("Log entry is missing text".into()))?;
        let location_url = entry.get("url").and_then(Value::as_str).unwrap_or("");
        let line_number = entry
            .get("lineNumber")
            .and_then(Value::as_u64)
            .map_or(Ok(0), |value| {
                u32::try_from(value).map_err(HostError::backend)
            })?;
        let timestamp_micros = console_timestamp_micros(entry.get("timestamp"))?;
        if message_type.len() > crate::frame_metadata::MAX_CONSOLE_TYPE_BYTES {
            return Err(HostError::ResourceExhausted(
                "console message type exceeds byte limit".into(),
            ));
        }
        if text.len() > ConsoleMessage::maximum_text_bytes() {
            return Err(HostError::ResourceExhausted(
                "console message text exceeds byte limit".into(),
            ));
        }
        if location_url.len() > crate::frame_metadata::MAX_CONSOLE_LOCATION_URL_BYTES {
            return Err(HostError::ResourceExhausted(
                "console message location URL exceeds byte limit".into(),
            ));
        }
        let storage_bytes =
            ConsoleMessage::storage_bytes(message_type.len(), text.len(), 2, location_url.len())?;
        Ok(Self {
            message_type,
            text,
            location_url,
            line_number,
            timestamp_micros,
            storage_bytes,
        })
    }

    const fn storage_bytes(&self) -> usize {
        self.storage_bytes
    }

    fn build(self) -> Result<ConsoleMessage, HostError> {
        ConsoleMessage::new(
            self.message_type.to_owned(),
            self.text.to_owned(),
            "[]".to_owned(),
            self.location_url.to_owned(),
            self.line_number,
            0,
            self.timestamp_micros,
        )
    }
}

fn cleanup_admission_error(error: CleanupSubmitError) -> HostError {
    match error {
        CleanupSubmitError::CapacityExceeded { .. } => {
            HostError::ResourceExhausted(error.to_string())
        }
        CleanupSubmitError::Unavailable => HostError::TargetClosed,
        CleanupSubmitError::StatePoisoned => HostError::backend(error),
    }
}

fn release_failure(error: impl std::fmt::Display) -> CleanupFailure {
    CleanupFailure::new(format!("console argument release failed: {error}"))
}

#[cfg(feature = "console-argument-handles")]
struct RemoteObjectRecovery {
    reservation: Option<TokioCleanupReservation>,
    connection: Arc<CdpConnection>,
    session_id: Box<str>,
    object_ids: Vec<Box<str>>,
}

#[cfg(feature = "console-argument-handles")]
impl RemoteObjectRecovery {
    fn new(observer: &Observer, session_id: &str, args: &[Value]) -> Result<Self, HostError> {
        if args.len() > crate::handles::MAX_EVENT_ARGUMENT_HANDLES {
            return Err(HostError::ResourceExhausted(format!(
                "console argument count exceeds {}",
                crate::handles::MAX_EVENT_ARGUMENT_HANDLES
            )));
        }
        let ids = args
            .iter()
            .filter_map(Value::as_object)
            .filter_map(|argument| argument.get("objectId"))
            .filter_map(Value::as_str);
        let bytes = ids
            .clone()
            .try_fold(0usize, |total, id| total.checked_add(id.len()))
            .ok_or_else(|| {
                HostError::ResourceExhausted("console object ID bytes overflow".into())
            })?;
        if bytes > MAX_LOG_OBJECT_ID_BYTES {
            return Err(HostError::ResourceExhausted(
                "console object ID bytes exceed their limit".into(),
            ));
        }
        let mut object_ids = Vec::with_capacity(args.len());
        object_ids.extend(ids.map(Box::<str>::from));
        let reservation = (!object_ids.is_empty())
            .then(|| observer.cleanup.try_reserve())
            .transpose()
            .map_err(cleanup_admission_error)?;
        Ok(Self {
            reservation,
            connection: observer.connection.clone(),
            session_id: session_id.into(),
            object_ids,
        })
    }

    fn disarm(mut self) {
        self.reservation.take();
    }
}

#[cfg(feature = "console-argument-handles")]
impl Drop for RemoteObjectRecovery {
    fn drop(&mut self) {
        let Some(reservation) = self.reservation.take() else {
            return;
        };
        let connection = self.connection.clone();
        let session_id = self.session_id.clone();
        let object_ids = std::mem::take(&mut self.object_ids);
        reservation.submit(Box::pin(async move {
            for object_id in object_ids {
                let result = connection
                    .send_command_with_session(
                        "Runtime.releaseObject",
                        serde_json::json!({ "objectId": object_id }),
                        Some(&session_id),
                    )
                    .await;
                match result {
                    Ok(_) => {}
                    Err(pwright_cdp::connection::CdpError::Closed)
                    | Err(pwright_cdp::connection::CdpError::SessionDetached)
                    | Err(pwright_cdp::connection::CdpError::Protocol { .. }) => return Ok(()),
                    Err(error) => return Err(release_failure(error)),
                }
            }
            Ok(())
        }));
    }
}

impl Observer {
    #[cfg(feature = "console-argument-handles")]
    pub(crate) fn finish_ready(&self) -> Result<(), HostError> {
        loop {
            let (pending, ready) = {
                let mut queue = self.pending.lock().map_err(HostError::backend)?;
                let Some(front) = queue.front_mut() else {
                    return Ok(());
                };
                let ready = match front.handles.as_mut() {
                    Some(handles) => match settle_console_argument_handles(handles.try_finish())? {
                        ConsoleArgumentHandleSettlement::Pending => {
                            return Ok(());
                        }
                        ConsoleArgumentHandleSettlement::Ready(ready) => Some(ready),
                        ConsoleArgumentHandleSettlement::SourceClosed => None,
                    },
                    None => None,
                };
                (
                    queue
                        .pop_front()
                        .expect("ready console message is at the queue front"),
                    ready,
                )
            };
            if pending.publish {
                let event_message = match ready.as_ref() {
                    Some(ready) => {
                        let argument_handles = ready.handles().to_vec().into_boxed_slice();
                        Arc::new(
                            pending
                                .message
                                .clone()
                                .with_argument_handles(argument_handles)?,
                        )
                    }
                    None => Arc::new(pending.message.clone()),
                };
                self.trees.publish_page_console(
                    pending.page,
                    pending.event_storage_bytes,
                    || Ok(event_message),
                )?;
            }
            #[cfg(feature = "console-history")]
            self.history.record(
                pending.page,
                pending.history_storage_bytes,
                Arc::new(pending.message),
            )?;
            // `ready` retains the batch call allowance until the lifecycle
            // publication has acquired its own exact event-storage lease.
            drop(ready);
        }
    }

    fn release_log_arguments(
        &self,
        session_id: &str,
        entry: &serde_json::Map<String, Value>,
    ) -> Result<(), HostError> {
        let Some(arguments) = entry.get("args").and_then(Value::as_array) else {
            return Ok(());
        };
        let ids = || {
            arguments
                .iter()
                .filter_map(Value::as_object)
                .filter_map(|argument| argument.get("objectId"))
                .filter_map(Value::as_str)
        };
        let count = ids().count();
        if count == 0 {
            return Ok(());
        }
        let bytes = ids()
            .try_fold(0usize, |total, id| total.checked_add(id.len()))
            .ok_or_else(|| HostError::ResourceExhausted("Log object ID bytes overflow".into()))?;
        if count > MAX_LOG_ARGUMENTS || bytes > MAX_LOG_OBJECT_ID_BYTES {
            return Err(HostError::ResourceExhausted(
                "Log argument release bounds exceeded".into(),
            ));
        }
        let reservation = self
            .cleanup
            .try_reserve()
            .map_err(cleanup_admission_error)?;
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(count)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        owned.extend(ids().map(str::to_owned));
        let session_id = session_id.to_owned();
        let connection = self.connection.clone();
        reservation.submit(Box::pin(async move {
            for object_id in owned {
                let result = connection
                    .send_command_with_session(
                        "Runtime.releaseObject",
                        serde_json::json!({ "objectId": object_id }),
                        Some(&session_id),
                    )
                    .await;
                match result {
                    Ok(_) => {}
                    Err(pwright_cdp::connection::CdpError::Closed)
                    | Err(pwright_cdp::connection::CdpError::SessionDetached)
                    | Err(pwright_cdp::connection::CdpError::Protocol { .. }) => return Ok(()),
                    Err(error) => return Err(release_failure(error)),
                }
            }
            Ok(())
        }));
        Ok(())
    }
}

#[cfg(feature = "console-argument-handles")]
enum ConsoleArgumentHandleSettlement {
    Pending,
    Ready(crate::handles::ReadyEventHandles),
    SourceClosed,
}

#[cfg(feature = "console-argument-handles")]
fn settle_console_argument_handles(
    result: Result<Option<crate::handles::ReadyEventHandles>, HostError>,
) -> Result<ConsoleArgumentHandleSettlement, HostError> {
    match result {
        Ok(Some(ready)) => Ok(ConsoleArgumentHandleSettlement::Ready(ready)),
        Ok(None) => Ok(ConsoleArgumentHandleSettlement::Pending),
        // Console text and its bounded JSON snapshot were captured before the
        // source Page retired. A short-lived popup can disappear before its
        // RemoteObjects become portable Handles; that invalidates only those
        // argument owners, not the already observed console event or Browser.
        Err(HostError::TargetClosed) => Ok(ConsoleArgumentHandleSettlement::SourceClosed),
        Err(error) => Err(error),
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
        #[cfg(feature = "console-events")]
        self.subscriptions
            .retain(topology)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
        #[cfg(feature = "console-history")]
        self.history
            .retain(topology)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
        Ok(())
    }

    fn event(&self, topology: &CdpTopology, event: &CdpEvent) -> FrameObservationResult {
        self.downstream.event(topology, event)?;
        #[cfg(feature = "console-events")]
        if event.method == "Page.frameNavigated" {
            let frame_id = event
                .params
                .get("frame")
                .and_then(|value| value.get("id"))
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("frame.id"))?;
            let parent = event
                .params
                .get("frame")
                .and_then(|value| value.get("parentId"))
                .and_then(Value::as_str);
            if parent.is_none()
                && let Some(route) = topology.page_session_routes().find(|route| {
                    topology.main_frame_native_id(route.object_id()) == Some(frame_id)
                })
            {
                self.subscriptions
                    .activate_for_page(topology, route.object_id())
                    .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
            }
        }
        match event.method.as_str() {
            "Runtime.consoleAPICalled" => {
                let session_id = event
                    .session_id
                    .as_deref()
                    .ok_or_else(|| invalid("sessionId"))?;
                let execution_context_id = event
                    .params
                    .get("executionContextId")
                    .and_then(Value::as_i64)
                    .ok_or_else(|| invalid("executionContextId"))?;
                // Chromium replays messages whose execution world is already gone as
                // context zero when Runtime is enabled. Playwright deliberately drops
                // those messages because no public Page argument ownership remains.
                if execution_context_id == 0 {
                    return Ok(());
                }
                let Some(frame) = topology.frame_execution_routes().find(|route| {
                    route.session_id() == session_id
                        && route.execution_context_id() == execution_context_id
                }) else {
                    return Ok(());
                };
                let Some(page) = topology.page_object_id_for_frame(frame.object_id()) else {
                    return Ok(());
                };
                #[cfg(feature = "console-events")]
                let publish = self
                    .subscriptions
                    .publishes(topology, page)
                    .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
                let message_type = event
                    .params
                    .get("type")
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid("type"))?;
                let args = event
                    .params
                    .get("args")
                    .and_then(Value::as_array)
                    .ok_or_else(|| invalid("args"))?;
                #[cfg(feature = "console-argument-handles")]
                if publish
                    && self
                        .pending
                        .lock()
                        .map_err(|error| {
                            Box::new(HostError::backend(error)) as Box<dyn Error + Send + Sync>
                        })?
                        .len()
                        >= crate::handles::MAX_CALLS
                {
                    return Err(Box::new(HostError::ResourceExhausted(
                        "pending console Handle publication capacity exceeded".into(),
                    )));
                }
                #[cfg(feature = "console-argument-handles")]
                let recovery = RemoteObjectRecovery::new(self, session_id, args)
                    .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
                let plan = ConsoleMessagePlan::new(message_type, args, &event.params)
                    .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
                let storage_bytes = plan.storage_bytes();
                let message = plan
                    .build()
                    .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
                #[cfg(feature = "console-argument-handles")]
                {
                    if !publish {
                        #[cfg(feature = "console-history")]
                        self.history
                            .record(page, storage_bytes, Arc::new(message))
                            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
                        return Ok(());
                    }
                    let event_storage_bytes = storage_bytes
                        .checked_add(
                            args.len()
                                .checked_mul(
                                    std::mem::size_of::<Arc<crate::handles::PendingHandle>>(),
                                )
                                .ok_or_else(|| {
                                    Box::new(HostError::ResourceExhausted(
                                        "console argument Handle storage overflow".into(),
                                    ))
                                        as Box<dyn Error + Send + Sync>
                                })?,
                        )
                        .ok_or_else(|| {
                            Box::new(HostError::ResourceExhausted(
                                "console event storage overflow".into(),
                            )) as Box<dyn Error + Send + Sync>
                        })?;
                    let handles = self
                        .handles
                        .stage_event_handles(frame.object_id(), args)
                        .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
                    recovery.disarm();
                    self.pending
                        .lock()
                        .map_err(|error| {
                            Box::new(HostError::backend(error)) as Box<dyn Error + Send + Sync>
                        })?
                        .push_back(PendingConsoleMessage {
                            page,
                            publish,
                            #[cfg(feature = "console-history")]
                            history_storage_bytes: storage_bytes,
                            event_storage_bytes,
                            message,
                            handles: Some(handles),
                        });
                    // This callback is the topology driver's ordered event
                    // boundary. Publish native-free local Handle ownership and
                    // the console lifecycle group before the driver's ensuing
                    // session synchronization can retire a short-lived popup.
                    self.handles
                        .finish_event_handles_ready()
                        .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
                    self.finish_ready()
                        .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
                }
                #[cfg(not(feature = "console-argument-handles"))]
                {
                    let message = Arc::new(message);
                    #[cfg(feature = "console-events")]
                    if publish {
                        self.trees
                            .publish_page_console(page, storage_bytes, || Ok(Arc::clone(&message)))
                            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
                    }
                    #[cfg(feature = "console-history")]
                    self.history
                        .record(page, storage_bytes, message)
                        .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
                    Ok(())
                }
            }
            "Log.entryAdded" => {
                let session_id = event
                    .session_id
                    .as_deref()
                    .ok_or_else(|| invalid("sessionId"))?;
                let entry = event
                    .params
                    .get("entry")
                    .and_then(Value::as_object)
                    .ok_or_else(|| invalid("entry"))?;
                self.release_log_arguments(session_id, entry)
                    .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
                if entry.get("source").and_then(Value::as_str) == Some("worker") {
                    return Ok(());
                }
                let Some(target) = topology.target_session_route(session_id) else {
                    return Ok(());
                };
                let Some(page) = topology.page_object_id(target.page_target_id()) else {
                    return Ok(());
                };
                #[cfg(feature = "console-events")]
                let publish = self
                    .subscriptions
                    .publishes(topology, page)
                    .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
                let plan = LogMessagePlan::new(entry)
                    .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
                let storage_bytes = plan.storage_bytes();
                let message = plan
                    .build()
                    .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
                #[cfg(feature = "console-argument-handles")]
                {
                    if !publish {
                        #[cfg(feature = "console-history")]
                        self.history
                            .record(page, storage_bytes, Arc::new(message))
                            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
                        return Ok(());
                    }
                    let mut pending = self.pending.lock().map_err(|error| {
                        Box::new(HostError::backend(error)) as Box<dyn Error + Send + Sync>
                    })?;
                    if pending.len() >= crate::handles::MAX_CALLS {
                        return Err(Box::new(HostError::ResourceExhausted(
                            "pending console publication capacity exceeded".into(),
                        )));
                    }
                    pending.push_back(PendingConsoleMessage {
                        page,
                        publish,
                        #[cfg(feature = "console-history")]
                        history_storage_bytes: storage_bytes,
                        event_storage_bytes: storage_bytes,
                        message,
                        handles: None,
                    });
                    drop(pending);
                    self.finish_ready()
                        .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
                }
                #[cfg(not(feature = "console-argument-handles"))]
                {
                    let message = Arc::new(message);
                    #[cfg(feature = "console-events")]
                    if publish {
                        self.trees
                            .publish_page_console(page, storage_bytes, || Ok(Arc::clone(&message)))
                            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
                    }
                    #[cfg(feature = "console-history")]
                    self.history
                        .record(page, storage_bytes, message)
                        .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }

    fn seal(&self) {
        #[cfg(feature = "console-events")]
        self.subscriptions.clear();
        #[cfg(feature = "console-argument-handles")]
        self.pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
        #[cfg(feature = "console-history")]
        self.history.close();
        self.downstream.seal();
    }
}

#[cfg(feature = "console-history")]
impl crate::HostSession {
    pub fn console_messages(&self, page_id: &str) -> Result<Vec<Arc<ConsoleMessage>>, HostError> {
        let page = page_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Page ID".into()))?;
        self.require_visible_page_id(page)?;
        let history = self
            .state
            .lock()
            .map_err(HostError::backend)?
            .as_ref()
            .ok_or(HostError::TargetClosed)?
            .console_history
            .clone();
        self.frame_trees.get(page)?;
        history.snapshot(page)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "console-history")]
    use std::sync::Arc;

    #[cfg(any(feature = "console-events", feature = "console-history"))]
    use pwright_core::ObjectId;
    #[cfg(feature = "console-history")]
    use pwright_core::{ProductionResourceLedger, ResourceClass, ResourceLimits};
    use serde_json::json;

    #[cfg(feature = "console-argument-handles")]
    use super::{ConsoleArgumentHandleSettlement, settle_console_argument_handles};
    use super::{ConsoleMessagePlan, LogMessagePlan};
    #[cfg(feature = "console-history")]
    use super::{Installation, MAX_CONSOLE_MESSAGES_PER_PAGE, requirements};
    #[cfg(feature = "console-history")]
    use crate::ConsoleMessage;

    #[cfg(feature = "console-events")]
    #[test]
    fn console_subscription_stays_pending_until_the_next_command_boundary() {
        // Arrange
        let subscriptions = super::Subscriptions::new();
        let page = ObjectId::new(7);

        // Act
        let token = subscriptions
            .subscribe_page_waiter(page)
            .expect("subscription should fit");
        let (pending_before, active_before) = {
            let state = subscriptions.state.lock().expect("subscription state");
            (
                !state.pages[0].waiters[0].active,
                state.pages[0].waiters[0].active,
            )
        };
        subscriptions
            .activate_pending()
            .expect("command boundary should activate");
        let (pending_after, active_after) = {
            let state = subscriptions.state.lock().expect("subscription state");
            (
                !state.pages[0].waiters[0].active,
                state.pages[0].waiters[0].active,
            )
        };
        subscriptions
            .release_waiter(token)
            .expect("subscription should release");

        // Assert
        assert_eq!((pending_before, active_before), (true, false));
        assert_eq!((pending_after, active_after), (false, true));
        assert!(
            subscriptions
                .state
                .lock()
                .expect("subscription state")
                .pages
                .is_empty()
        );
    }

    #[cfg(feature = "console-argument-handles")]
    #[test]
    fn closed_console_argument_source_preserves_the_observed_message() {
        // Arrange
        let result = Err(crate::HostError::TargetClosed);

        // Act
        let settlement = settle_console_argument_handles(result).expect("source retirement");

        // Assert
        assert!(matches!(
            settlement,
            ConsoleArgumentHandleSettlement::SourceClosed
        ));
    }

    #[cfg(feature = "console-events")]
    #[test]
    fn console_listener_release_does_not_release_a_pending_waiter() {
        // Arrange
        let subscriptions = super::Subscriptions::new();
        let page = ObjectId::new(7);
        subscriptions
            .update_page_listener(page, true)
            .expect("listener should fit");
        let token = subscriptions
            .subscribe_page_waiter(page)
            .expect("waiter should fit");

        // Act
        subscriptions
            .update_page_listener(page, false)
            .expect("listener should release");
        let (listeners, waiter_token, waiter_active) = {
            let state = subscriptions.state.lock().expect("subscription state");
            (
                state.pages[0].listeners,
                state.pages[0].waiters[0].token,
                state.pages[0].waiters[0].active,
            )
        };
        subscriptions
            .release_waiter(token)
            .expect("waiter should release");

        // Assert
        assert_eq!(listeners, 0);
        assert_eq!(waiter_token, token);
        assert!(!waiter_active);
        assert!(
            subscriptions
                .state
                .lock()
                .expect("subscription state")
                .pages
                .is_empty()
        );
    }

    #[cfg(feature = "console-history")]
    fn resources() -> ProductionResourceLedger {
        let required = requirements();
        let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
        for class in ResourceClass::ALL {
            let bound = required.class(class);
            limits = limits.with_class(class, bound.global, bound.per_scope);
        }
        ProductionResourceLedger::new(limits, required).expect("valid history resource plan")
    }

    #[cfg(feature = "console-history")]
    fn message(index: usize) -> Arc<ConsoleMessage> {
        Arc::new(
            ConsoleMessage::new(
                "log".to_owned(),
                format!("message{index}"),
                "[]".to_owned(),
                String::new(),
                0,
                0,
                0,
            )
            .expect("bounded console message"),
        )
    }

    #[cfg(feature = "console-history")]
    #[test]
    fn history_retains_the_last_two_hundred_messages_without_over_admission() {
        // Arrange
        let resources = resources();
        let history = Installation::new(&resources).expect("bounded history");
        let page = ObjectId::new(7);

        // Act
        for index in 0..=MAX_CONSOLE_MESSAGES_PER_PAGE {
            let message = message(index);
            let bytes = ConsoleMessage::storage_bytes(
                message.message_type().len(),
                message.text().len(),
                message.arguments_json().len(),
                message.location_url().len(),
            )
            .expect("bounded storage");
            history
                .record(page, bytes, message)
                .expect("drop-oldest admission");
        }

        // Assert
        let messages = history.snapshot(page).expect("history snapshot");
        assert_eq!(messages.len(), MAX_CONSOLE_MESSAGES_PER_PAGE);
        assert_eq!(messages.first().expect("first message").text(), "message1");
        assert_eq!(messages.last().expect("last message").text(), "message200");
        let metrics = resources.metrics().expect("resource metrics");
        assert_eq!(
            metrics.class(ResourceClass::EventItem).current,
            MAX_CONSOLE_MESSAGES_PER_PAGE as u64
        );
        assert_eq!(metrics.class(ResourceClass::EventItem).rejected, 0);
    }

    #[cfg(feature = "console-history")]
    #[test]
    fn close_releases_console_history_and_rejects_future_reads() {
        // Arrange
        let resources = resources();
        let history = Installation::new(&resources).expect("bounded history");
        let page = ObjectId::new(11);
        let message = message(1);
        let bytes = ConsoleMessage::storage_bytes(
            message.message_type().len(),
            message.text().len(),
            message.arguments_json().len(),
            message.location_url().len(),
        )
        .expect("bounded storage");
        history.record(page, bytes, message).expect("record");

        // Act
        history.close();

        // Assert
        assert!(matches!(
            history.snapshot(page),
            Err(crate::HostError::TargetClosed)
        ));
        let metrics = resources.metrics().expect("resource metrics");
        assert_eq!(metrics.class(ResourceClass::EventItem).current, 0);
        assert_eq!(metrics.class(ResourceClass::EventBytes).current, 0);
    }

    #[test]
    fn console_text_preserves_primitive_argument_order() {
        // Arrange
        let arguments = vec![
            json!({"type":"string", "value":"press"}),
            json!({"type":"number", "value":7}),
            json!({"type":"undefined"}),
        ];

        // Act
        let params = json!({});
        let message = ConsoleMessagePlan::new("log", &arguments, &params)
            .expect("bounded console message plan")
            .build()
            .expect("bounded console message");

        // Assert
        assert_eq!(message.message_type(), "log");
        assert_eq!(message.text(), "press 7 undefined");
        assert_eq!(message.arguments_json(), r#"["press",7,null]"#);
    }

    #[test]
    fn console_text_compacts_sparse_array_holes() {
        // Arrange
        let arguments = vec![json!({
            "type":"object",
            "subtype":"array",
            "description":"Array(101)",
            "preview": {"properties": [
                {"name":"100", "type":"number", "value":"3"},
                {"name":"1", "type":"number", "value":"1"},
                {"name":"10", "type":"number", "value":"2"}
            ]}
        })];

        // Act
        let message = ConsoleMessagePlan::new("log", &arguments, &json!({}))
            .expect("bounded console message plan")
            .build()
            .expect("bounded console message");

        // Assert
        assert_eq!(message.text(), "[empty, 1, empty x 8, 2, empty x 89, 3]");
    }

    #[test]
    fn log_entry_plan_preserves_level_text_and_location_without_arguments() {
        // Arrange
        let value = json!({
            "level": "error",
            "text": "Blocked by CORS",
            "url": "https://example.test/app.js",
            "lineNumber": 12,
        });
        let entry = value.as_object().unwrap();

        // Act
        let message = LogMessagePlan::new(entry)
            .expect("bounded Log message plan")
            .build()
            .expect("bounded Log message");

        // Assert
        assert_eq!(message.message_type(), "error");
        assert_eq!(message.text(), "Blocked by CORS");
        assert_eq!(message.arguments_json(), "[]");
        assert_eq!(message.location_url(), "https://example.test/app.js");
        assert_eq!(message.line_number(), 12);
        assert_eq!(message.column_number(), 0);
    }
}
