//! Explicitly selected, bounded public Chromium CDP sessions.
//!
//! A browser-root auxiliary attachment keeps these sessions separate from
//! Pwright's Page-discovery attachment. The topology driver recognizes only
//! that exact parent session ID; it never owns the public child sessions.

use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::sync::{Arc, Mutex};
#[cfg(feature = "browser-cdp-session")]
use std::time::Duration;

use pwright_cdp::{
    CdpConnection, CdpEvent, CdpEventReceiver, CdpEventRecvError, CdpEventTryRecvError, CdpSession,
};
use pwright_composition_chromium::ChromiumAuxiliarySessionParent;
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequest,
    ResourceRequirements, ResourceScope,
};
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::{HostError, HostSession, LiveSession, context_metadata::ContextMetadata};

pub const MAX_CDP_SESSIONS: usize = 8;
pub const MAX_CDP_SESSION_EVENTS: usize = 64;
pub const MAX_CDP_SESSION_EVENT_BYTES: usize = 1024 * 1024;
const MAX_NATIVE_ID_BYTES: usize = 256;
const MAX_METHOD_BYTES: usize = 256;
const MAX_COMMAND_PARAMS_BYTES: usize = 1024 * 1024;
const BARRIER_CAPACITY: usize = 8;
const INSTALLATION_SCOPE: ResourceScope = ResourceScope::extension(199, 1);
#[cfg(not(feature = "browser-cdp-session"))]
const BACKEND_STATE_BYTES: u64 = 16 * 1024;
#[cfg(feature = "browser-cdp-session")]
const BACKEND_STATE_BYTES: u64 = 96 * 1024;
#[cfg(feature = "browser-cdp-session")]
const MAX_RECENT_TARGETS: usize = 32;
#[cfg(feature = "browser-cdp-session")]
const MAX_TARGET_FILTER_ENTRIES: usize = 64;
#[cfg(feature = "browser-cdp-session")]
const TARGET_EVENT_FENCE_TIMEOUT_MS: u32 = 30_000;

pub(crate) const fn requirements() -> ResourceRequirements {
    ResourceRequirements::none()
        .with_class(
            ResourceClass::EventItem,
            (MAX_CDP_SESSIONS * MAX_CDP_SESSION_EVENTS) as u64,
            (MAX_CDP_SESSIONS * MAX_CDP_SESSION_EVENTS) as u64,
        )
        .with_class(
            ResourceClass::EventBytes,
            (MAX_CDP_SESSIONS * MAX_CDP_SESSION_EVENT_BYTES) as u64,
            (MAX_CDP_SESSIONS * MAX_CDP_SESSION_EVENT_BYTES) as u64,
        )
        .with_class(
            ResourceClass::BackendStateBytes,
            BACKEND_STATE_BYTES,
            BACKEND_STATE_BYTES,
        )
        .with_metadata(1, 1)
}

/// One event delivered by a public CDP session, without the transport envelope.
#[derive(Clone, Debug, PartialEq)]
pub struct CdpSessionEvent {
    pub method: String,
    pub params: Value,
}

struct BufferedEvent {
    event: CdpSessionEvent,
    bytes: usize,
}

struct EventClaim {
    generation: u64,
    events: VecDeque<BufferedEvent>,
    peeked: bool,
}

#[derive(Clone)]
enum SessionOwner {
    Page {
        context: ObjectId,
        page: ObjectId,
        target_id: String,
    },
    #[cfg(feature = "browser-cdp-session")]
    Browser,
}

#[cfg(feature = "browser-cdp-session")]
pub(crate) struct BrowserTargetFence(Vec<(u64, u64, u64)>);

/// CDP TargetFilter is first-match-wins. A missing filter uses Chromium's
/// default, which includes page targets; a specified filter with no page
/// match excludes them. Invalid or oversized filters conservatively retain
/// the bounded fence if Chromium nevertheless accepts the command.
#[cfg(feature = "browser-cdp-session")]
fn discovers_page_targets(params: &Value) -> bool {
    match params.get("discover").and_then(Value::as_bool) {
        Some(false) => return false,
        Some(true) => {}
        None => return true,
    }
    let Some(filter) = params.get("filter") else {
        return true;
    };
    let Some(entries) = filter.as_array() else {
        return true;
    };
    if entries.len() > MAX_TARGET_FILTER_ENTRIES {
        return true;
    }
    for entry in entries {
        let Some(entry) = entry.as_object() else {
            return true;
        };
        let exclude = match entry.get("exclude") {
            None => false,
            Some(Value::Bool(exclude)) => *exclude,
            Some(_) => return true,
        };
        let matches_page = match entry.get("type") {
            None => true,
            Some(Value::String(kind)) => kind == "page",
            Some(_) => return true,
        };
        if matches_page {
            return !exclude;
        }
    }
    false
}

struct Entry {
    owner: SessionOwner,
    session_id: Option<String>,
    detaching: bool,
    abandoned: bool,
    events: VecDeque<BufferedEvent>,
    claim: Option<EventClaim>,
    event_bytes: usize,
    overflowed: bool,
    #[cfg(feature = "browser-cdp-session")]
    discover_targets: bool,
    #[cfg(feature = "browser-cdp-session")]
    discovery_epoch: u64,
    #[cfg(feature = "browser-cdp-session")]
    target_sequence: u64,
    #[cfg(feature = "browser-cdp-session")]
    recent_targets: VecDeque<(u64, String)>,
}

impl Entry {
    fn event_count(&self) -> usize {
        self.events.len() + self.claim.as_ref().map_or(0, |claim| claim.events.len())
    }

    fn rollback_claim(&mut self) {
        if let Some(mut claim) = self.claim.take() {
            while let Some(event) = claim.events.pop_back() {
                self.events.push_front(event);
            }
        }
    }

    fn clear_event_payloads(&mut self) {
        self.events.clear();
        self.claim = None;
        self.event_bytes = 0;
        self.overflowed = false;
        #[cfg(feature = "browser-cdp-session")]
        self.recent_targets.clear();
    }
}

struct Registry {
    next_token: u64,
    next_claim_generation: u64,
    event_generation: u64,
    event_changes: watch::Sender<u64>,
    entries: BTreeMap<u64, Entry>,
}

impl Registry {
    fn new() -> Self {
        let (event_changes, _) = watch::channel(0);
        Self {
            next_token: 1,
            next_claim_generation: 1,
            event_generation: 0,
            event_changes,
            entries: BTreeMap::new(),
        }
    }

    fn signal_event_change(&mut self) {
        self.event_generation = self.event_generation.wrapping_add(1);
        self.event_changes.send_replace(self.event_generation);
    }

    fn subscribe_events(&self) -> watch::Receiver<u64> {
        self.event_changes.subscribe()
    }

    fn reserve(
        &mut self,
        context: ObjectId,
        page: ObjectId,
        target_id: String,
    ) -> Result<u64, HostError> {
        if target_id.is_empty() || target_id.len() > MAX_NATIVE_ID_BYTES {
            return Err(HostError::InvalidArgument(
                "CDP Page target ID exceeds native identity bounds".into(),
            ));
        }
        self.reserve_owner(SessionOwner::Page {
            context,
            page,
            target_id,
        })
    }

    #[cfg(feature = "browser-cdp-session")]
    fn reserve_browser(&mut self) -> Result<u64, HostError> {
        self.reserve_owner(SessionOwner::Browser)
    }

    fn reserve_owner(&mut self, owner: SessionOwner) -> Result<u64, HostError> {
        if self.entries.len() >= MAX_CDP_SESSIONS {
            return Err(HostError::ResourceExhausted(format!(
                "CDP session capacity exceeded ({MAX_CDP_SESSIONS})"
            )));
        }
        let token = self.next_token;
        self.next_token = token.checked_add(1).ok_or_else(|| {
            HostError::ResourceExhausted("CDP session token space exhausted".into())
        })?;
        self.entries.insert(
            token,
            Entry {
                owner,
                session_id: None,
                detaching: false,
                abandoned: false,
                events: VecDeque::new(),
                claim: None,
                event_bytes: 0,
                overflowed: false,
                #[cfg(feature = "browser-cdp-session")]
                discover_targets: false,
                #[cfg(feature = "browser-cdp-session")]
                discovery_epoch: 0,
                #[cfg(feature = "browser-cdp-session")]
                target_sequence: 0,
                #[cfg(feature = "browser-cdp-session")]
                recent_targets: VecDeque::new(),
            },
        );
        Ok(token)
    }

    fn commit(&mut self, token: u64, session_id: String) -> Result<(), HostError> {
        if session_id.is_empty() || session_id.len() > MAX_NATIVE_ID_BYTES {
            return Err(HostError::Backend(
                "CDP child session ID exceeds native identity bounds".into(),
            ));
        }
        let entry = self
            .entries
            .get_mut(&token)
            .ok_or(HostError::TargetClosed)?;
        if entry.session_id.is_some() {
            return Err(HostError::Backend(
                "CDP session was already committed".into(),
            ));
        }
        entry.session_id = Some(session_id);
        if entry.abandoned {
            return Err(HostError::TargetClosed);
        }
        Ok(())
    }

    fn active(&self, token: u64) -> Result<(SessionOwner, String), HostError> {
        let entry = self.entries.get(&token).ok_or(HostError::TargetClosed)?;
        if entry.detaching || entry.abandoned {
            return Err(HostError::TargetClosed);
        }
        let session_id = entry.session_id.as_ref().ok_or(HostError::TargetClosed)?;
        Ok((entry.owner.clone(), session_id.clone()))
    }

    #[cfg(feature = "browser-cdp-session")]
    fn set_discover_targets(&mut self, token: u64, enabled: bool) -> Result<(), HostError> {
        let entry = self
            .entries
            .get_mut(&token)
            .ok_or(HostError::TargetClosed)?;
        if entry.detaching || entry.abandoned || entry.session_id.is_none() {
            return Err(HostError::TargetClosed);
        }
        if matches!(entry.owner, SessionOwner::Browser) {
            entry.discovery_epoch = entry.discovery_epoch.checked_add(1).ok_or_else(|| {
                HostError::ResourceExhausted("browser CDP discovery epoch exhausted".into())
            })?;
            entry.discover_targets = enabled;
            self.signal_event_change();
        }
        Ok(())
    }

    #[cfg(feature = "browser-cdp-session")]
    fn target_fence(&self) -> BrowserTargetFence {
        BrowserTargetFence(
            self.entries
                .iter()
                .filter_map(|(&token, entry)| {
                    (matches!(entry.owner, SessionOwner::Browser)
                        && entry.discover_targets
                        && !entry.detaching
                        && !entry.abandoned
                        && entry.session_id.is_some())
                    .then_some((token, entry.target_sequence, entry.discovery_epoch))
                })
                .collect(),
        )
    }

    #[cfg(feature = "browser-cdp-session")]
    fn target_fence_satisfied(
        &self,
        fence: &BrowserTargetFence,
        target_id: &str,
    ) -> Result<bool, HostError> {
        for &(token, after, epoch) in &fence.0 {
            let Some(entry) = self.entries.get(&token) else {
                // A retired public session no longer has a listener to order.
                continue;
            };
            if entry.detaching
                || entry.abandoned
                || !entry.discover_targets
                || entry.discovery_epoch != epoch
            {
                continue;
            }
            if entry.overflowed {
                return Err(HostError::ResourceExhausted(
                    "browser CDP target-created fence overflowed".into(),
                ));
            }
            if entry
                .recent_targets
                .iter()
                .any(|(sequence, observed)| *sequence > after && observed == target_id)
            {
                continue;
            }
            if entry.target_sequence.saturating_sub(after) > MAX_RECENT_TARGETS as u64 {
                return Err(HostError::ResourceExhausted(
                    "browser CDP target-created fence history expired".into(),
                ));
            }
            return Ok(false);
        }
        Ok(true)
    }

    fn begin_detach(&mut self, token: u64) -> Result<String, HostError> {
        let entry = self
            .entries
            .get_mut(&token)
            .ok_or(HostError::TargetClosed)?;
        if entry.detaching || entry.abandoned {
            return Err(HostError::TargetClosed);
        }
        let session_id = entry.session_id.clone().ok_or(HostError::TargetClosed)?;
        entry.rollback_claim();
        entry.detaching = true;
        self.signal_event_change();
        Ok(session_id)
    }

    fn finish_detach(&mut self, token: u64, succeeded: bool) {
        if succeeded {
            if self.entries.remove(&token).is_some() {
                self.signal_event_change();
            }
        } else if let Some(entry) = self.entries.get_mut(&token) {
            entry.detaching = false;
            // Events arriving while native detach was pending were ignored.
            // A failed detach must not expose a silently incomplete queue.
            entry.overflowed = true;
            self.signal_event_change();
        }
    }

    fn record(&mut self, event: &CdpEvent) {
        if event.method == "Target.detachedFromTarget"
            && !event.session_id.as_deref().is_some_and(|session_id| {
                self.has_browser_session(session_id)
                    && event.params.get("sessionId").and_then(Value::as_str) != Some(session_id)
            })
        {
            if let Some(detached) = event.params.get("sessionId").and_then(Value::as_str) {
                let before = self.entries.len();
                self.entries
                    .retain(|_, entry| entry.session_id.as_deref() != Some(detached));
                if self.entries.len() != before {
                    self.signal_event_change();
                }
            }
            return;
        }
        if event.method == "Inspector.detached" {
            if let Some(detached) = event.session_id.as_deref() {
                let before = self.entries.len();
                self.entries
                    .retain(|_, entry| entry.session_id.as_deref() != Some(detached));
                if self.entries.len() != before {
                    self.signal_event_change();
                }
            }
            return;
        }
        let Some(event_session) = event.session_id.as_deref() else {
            return;
        };
        let Some(entry) = self.entries.values_mut().find(|entry| {
            !entry.detaching
                && !entry.abandoned
                && entry.session_id.as_deref() == Some(event_session)
        }) else {
            return;
        };
        if entry.overflowed {
            return;
        }
        let Some(bytes) = serde_json::to_vec(&event.params)
            .ok()
            .and_then(|encoded| encoded.len().checked_add(event.method.len()))
        else {
            entry.overflowed = true;
            self.signal_event_change();
            return;
        };
        if entry.event_count() >= MAX_CDP_SESSION_EVENTS
            || entry
                .event_bytes
                .checked_add(bytes)
                .is_none_or(|total| total > MAX_CDP_SESSION_EVENT_BYTES)
        {
            entry.overflowed = true;
            self.signal_event_change();
            return;
        }
        entry.event_bytes += bytes;
        entry.events.push_back(BufferedEvent {
            event: CdpSessionEvent {
                method: event.method.clone(),
                params: event.params.clone(),
            },
            bytes,
        });
        #[cfg(feature = "browser-cdp-session")]
        if matches!(entry.owner, SessionOwner::Browser)
            && entry.discover_targets
            && event.method == "Target.targetCreated"
        {
            let target_id = event
                .params
                .get("targetInfo")
                .and_then(|value| value.get("targetId"))
                .and_then(Value::as_str);
            if let Some(target_id) = target_id
                && !target_id.is_empty()
                && target_id.len() <= MAX_NATIVE_ID_BYTES
                && let Some(sequence) = entry.target_sequence.checked_add(1)
            {
                entry.target_sequence = sequence;
                if entry.recent_targets.len() == MAX_RECENT_TARGETS {
                    entry.recent_targets.pop_front();
                }
                entry
                    .recent_targets
                    .push_back((sequence, target_id.to_owned()));
            } else {
                entry.overflowed = true;
            }
        }
        self.signal_event_change();
    }

    fn has_browser_session(&self, session_id: &str) -> bool {
        #[cfg(feature = "browser-cdp-session")]
        {
            self.entries.values().any(|entry| {
                matches!(entry.owner, SessionOwner::Browser)
                    && entry.session_id.as_deref() == Some(session_id)
            })
        }
        #[cfg(not(feature = "browser-cdp-session"))]
        {
            let _ = session_id;
            false
        }
    }

    fn mark_lagged(&mut self) {
        for entry in self.entries.values_mut() {
            entry.overflowed = true;
        }
        if !self.entries.is_empty() {
            self.signal_event_change();
        }
    }

    fn drain(&mut self, token: u64, max_events: usize) -> Result<Vec<CdpSessionEvent>, HostError> {
        if max_events == 0 || max_events > MAX_CDP_SESSION_EVENTS {
            return Err(HostError::InvalidArgument(format!(
                "CDP event drain count must be between 1 and {MAX_CDP_SESSION_EVENTS}"
            )));
        }
        let entry = self
            .entries
            .get_mut(&token)
            .ok_or(HostError::TargetClosed)?;
        if entry.detaching || entry.abandoned || entry.session_id.is_none() {
            return Err(HostError::TargetClosed);
        }
        if entry.claim.is_some() {
            return Err(HostError::InvalidArgument(
                "CDP session already has an outstanding event claim".into(),
            ));
        }
        if entry.overflowed {
            return Err(HostError::ResourceExhausted(
                "CDP session event queue overflowed".into(),
            ));
        }
        let mut drained = Vec::with_capacity(max_events.min(entry.events.len()));
        for _ in 0..max_events {
            let Some(event) = entry.events.pop_front() else {
                break;
            };
            entry.event_bytes -= event.bytes;
            drained.push(event.event);
        }
        Ok(drained)
    }

    fn claim(&mut self, token: u64, max_events: usize) -> Result<u64, HostError> {
        if max_events == 0 || max_events > MAX_CDP_SESSION_EVENTS {
            return Err(HostError::InvalidArgument(format!(
                "CDP event claim count must be between 1 and {MAX_CDP_SESSION_EVENTS}"
            )));
        }
        let entry = self
            .entries
            .get_mut(&token)
            .ok_or(HostError::TargetClosed)?;
        if entry.detaching || entry.abandoned || entry.session_id.is_none() {
            return Err(HostError::TargetClosed);
        }
        if entry.claim.is_some() {
            return Err(HostError::InvalidArgument(
                "CDP session already has an outstanding event claim".into(),
            ));
        }
        if entry.overflowed {
            return Err(HostError::ResourceExhausted(
                "CDP session event queue overflowed".into(),
            ));
        }
        let generation = self.next_claim_generation;
        self.next_claim_generation = generation.checked_add(1).ok_or_else(|| {
            HostError::ResourceExhausted("CDP event claim generation space exhausted".into())
        })?;
        let mut events = VecDeque::with_capacity(max_events.min(entry.events.len()));
        for _ in 0..max_events {
            let Some(event) = entry.events.pop_front() else {
                break;
            };
            events.push_back(event);
        }
        entry.claim = Some(EventClaim {
            generation,
            events,
            peeked: false,
        });
        Ok(generation)
    }

    fn claimed(&self, token: u64, generation: u64) -> Result<&Entry, HostError> {
        let entry = self.entries.get(&token).ok_or(HostError::TargetClosed)?;
        if entry.detaching || entry.abandoned || entry.session_id.is_none() {
            return Err(HostError::TargetClosed);
        }
        if entry.claim.as_ref().map(|claim| claim.generation) != Some(generation) {
            return Err(HostError::TargetClosed);
        }
        Ok(entry)
    }

    fn peek_claim(
        &mut self,
        token: u64,
        generation: u64,
    ) -> Result<Option<CdpSessionEvent>, HostError> {
        self.claimed(token, generation)?;
        let entry = self
            .entries
            .get_mut(&token)
            .ok_or(HostError::TargetClosed)?;
        if entry.overflowed {
            return Err(HostError::ResourceExhausted(
                "CDP session event queue overflowed".into(),
            ));
        }
        let claim = entry.claim.as_mut().ok_or(HostError::TargetClosed)?;
        let next = claim.events.front().map(|event| event.event.clone());
        claim.peeked = next.is_some();
        Ok(next)
    }

    fn ack_claim(&mut self, token: u64, generation: u64) -> Result<(), HostError> {
        self.claimed(token, generation)?;
        let entry = self
            .entries
            .get_mut(&token)
            .ok_or(HostError::TargetClosed)?;
        let claim = entry.claim.as_mut().ok_or(HostError::TargetClosed)?;
        if !claim.peeked {
            return Err(HostError::InvalidArgument(
                "CDP event must be peeked before acknowledgement".into(),
            ));
        }
        let event = claim.events.front().ok_or_else(|| {
            HostError::InvalidArgument("CDP event claim has no next event to acknowledge".into())
        })?;
        if entry.event_bytes < event.bytes {
            return Err(HostError::Backend(
                "CDP event claim byte accounting underflow".into(),
            ));
        }
        entry.event_bytes -= event.bytes;
        claim.events.pop_front();
        claim.peeked = false;
        Ok(())
    }

    fn rollback_claim(&mut self, token: u64, generation: u64) -> Result<(), HostError> {
        // A finally/Drop rollback must never mask the original terminal
        // error or mutate a newer claim on the same session.
        if let Some(entry) = self.entries.get_mut(&token)
            && entry.claim.as_ref().map(|claim| claim.generation) == Some(generation)
        {
            if entry.detaching || entry.abandoned || entry.session_id.is_none() {
                entry.claim = None;
            } else {
                entry.rollback_claim();
            }
        }
        Ok(())
    }

    fn retain_pages(&mut self, pages: &[ObjectId]) {
        // The portable Page is gone, but native detach is not yet proved.
        // Keep the child or in-flight attach in the capacity budget until a
        // native detach event, explicit compensation, or host shutdown.
        let mut changed = false;
        for entry in self.entries.values_mut() {
            if let SessionOwner::Page { page, .. } = &entry.owner
                && !pages.contains(page)
            {
                changed |= !entry.abandoned;
                entry.abandoned = true;
                entry.clear_event_payloads();
            }
        }
        if changed {
            self.signal_event_change();
        }
    }

    fn abandon(&mut self, token: u64) {
        if let Some(entry) = self.entries.get_mut(&token) {
            let changed = !entry.abandoned;
            entry.abandoned = true;
            entry.clear_event_payloads();
            if changed {
                self.signal_event_change();
            }
        }
    }

    fn begin_unclaimed_cleanup(&mut self, token: u64) -> Result<String, HostError> {
        let child_id = self
            .native_child(token)
            .ok_or(HostError::TargetClosed)?
            .to_owned();
        // Delivery-lease cleanup is independent of the portable Page's
        // lifetime. Preserve the exact native ID in the bounded registry
        // while making the token unusable to ordinary public operations.
        self.abandon(token);
        Ok(child_id)
    }

    fn native_child(&self, token: u64) -> Option<&str> {
        self.entries
            .get(&token)
            .and_then(|entry| entry.session_id.as_deref())
    }

    fn clear(&mut self) -> Vec<(SessionOwner, String)> {
        let sessions = self
            .entries
            .values()
            .filter_map(|entry| {
                entry
                    .session_id
                    .as_ref()
                    .map(|session_id| (entry.owner.clone(), session_id.clone()))
            })
            .collect();
        self.entries.clear();
        self.signal_event_change();
        sessions
    }
}

pub(crate) struct Installation {
    connection: Arc<CdpConnection>,
    parent: Arc<ChromiumAuxiliarySessionParent>,
    parent_native: tokio::sync::Mutex<Option<String>>,
    registry: Arc<Mutex<Registry>>,
    barrier: mpsc::Sender<oneshot::Sender<()>>,
    stop: CancellationToken,
    receiver_task: tokio::sync::Mutex<Option<JoinHandle<()>>>,
    _budget: ResourceLease,
}

impl Installation {
    pub(crate) fn new(
        connection: Arc<CdpConnection>,
        parent: Arc<ChromiumAuxiliarySessionParent>,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, HostError> {
        let budget = resources
            .claim(requirements())
            .map_err(HostError::backend)?
            .reserve_many(
                INSTALLATION_SCOPE,
                &[
                    ResourceRequest::new(
                        ResourceClass::EventItem,
                        (MAX_CDP_SESSIONS * MAX_CDP_SESSION_EVENTS) as u64,
                    ),
                    ResourceRequest::new(
                        ResourceClass::EventBytes,
                        (MAX_CDP_SESSIONS * MAX_CDP_SESSION_EVENT_BYTES) as u64,
                    ),
                    ResourceRequest::new(ResourceClass::BackendStateBytes, BACKEND_STATE_BYTES),
                ],
            )
            .map_err(HostError::backend)?;
        let receiver = connection
            .subscribe_composable_events()
            .map_err(HostError::backend)?;
        let registry = Arc::new(Mutex::new(Registry::new()));
        let stop = CancellationToken::new();
        let (barrier, barriers) = mpsc::channel(BARRIER_CAPACITY);
        let receiver_task = tokio::spawn(receive_events(
            receiver,
            Arc::clone(&registry),
            Arc::clone(&parent),
            barriers,
            stop.clone(),
        ));
        Ok(Self {
            connection,
            parent,
            parent_native: tokio::sync::Mutex::new(None),
            registry,
            barrier,
            stop,
            receiver_task: tokio::sync::Mutex::new(Some(receiver_task)),
            _budget: budget,
        })
    }

    async fn parent_session(&self, stop: &CancellationToken) -> Result<String, HostError> {
        let mut parent = self.parent_native.lock().await;
        if let Some(session_id) = parent.as_ref() {
            return Ok(session_id.clone());
        }
        let browser = CdpSession::browser(Arc::clone(&self.connection));
        let session_id =
            attach_to_settlement(&browser, "Target.attachToBrowserTarget", json!({}), stop).await?;
        if let Err(error) = self.parent.register(&session_id) {
            if let Err(detach_error) = browser.target_detach(&session_id).await {
                // An unregistered parent cannot be tracked by the topology.
                // Fail closed rather than leaving its native session alive.
                self.connection.request_disconnect();
                return Err(HostError::Backend(format!(
                    "CDP parent registration failed ({error}); native detach failed ({detach_error}); transport disconnected"
                )));
            }
            return Err(HostError::backend(error));
        }
        *parent = Some(session_id.clone());
        Ok(session_id)
    }

    async fn attached_parent_session(&self) -> Result<String, HostError> {
        self.parent_native
            .lock()
            .await
            .clone()
            .ok_or(HostError::TargetClosed)
    }

    async fn synchronize(&self) -> Result<(), HostError> {
        let (reply, completion) = oneshot::channel();
        self.barrier
            .send(reply)
            .await
            .map_err(|_| HostError::TargetClosed)?;
        completion.await.map_err(|_| HostError::TargetClosed)
    }

    async fn next_event_generation(&self, after: u64) -> Result<u64, HostError> {
        let changes = self.registry()?.subscribe_events();
        wait_for_event_generation(changes, after, &self.stop).await
    }

    #[cfg(feature = "browser-cdp-session")]
    async fn wait_for_target_created(
        &self,
        fence: &BrowserTargetFence,
        target_id: &str,
    ) -> Result<(), HostError> {
        let mut changes = self.registry()?.subscribe_events();
        let wait = async {
            loop {
                let _ = changes.borrow_and_update();
                if self.registry()?.target_fence_satisfied(fence, target_id)? {
                    return Ok(());
                }
                tokio::select! {
                    biased;
                    () = self.stop.cancelled() => return Err(HostError::TargetClosed),
                    changed = changes.changed() => changed.map_err(|_| HostError::TargetClosed)?,
                }
            }
        };
        tokio::time::timeout(
            Duration::from_millis(u64::from(TARGET_EVENT_FENCE_TIMEOUT_MS)),
            wait,
        )
        .await
        .map_err(|_| HostError::Timeout(TARGET_EVENT_FENCE_TIMEOUT_MS))?
    }

    fn registry(&self) -> Result<std::sync::MutexGuard<'_, Registry>, HostError> {
        self.registry.lock().map_err(HostError::backend)
    }

    fn reconcile_pages(&self, pages: &[ObjectId]) -> Result<(), HostError> {
        self.registry()?.retain_pages(pages);
        Ok(())
    }

    async fn compensate_child(
        &self,
        token: u64,
        parent: &CdpSession,
        child_id: &str,
    ) -> Result<(), HostError> {
        match parent.target_detach(child_id).await {
            Ok(()) => {
                let mut registry = self.registry()?;
                if registry.entries.remove(&token).is_some() {
                    registry.signal_event_change();
                }
                Ok(())
            }
            Err(error) => {
                let tracked = if let Ok(mut registry) = self.registry() {
                    if !registry.entries.contains_key(&token) {
                        // The exact native detach event already retired it.
                        return Ok(());
                    }
                    let matches = registry.native_child(token) == Some(child_id);
                    if matches {
                        registry.abandon(token);
                    }
                    matches
                } else {
                    false
                };
                if !tracked {
                    // A malformed/uncommitted native ID has no bounded
                    // registry slot to retry from. Terminate its transport.
                    self.connection.request_disconnect();
                }
                Err(HostError::Backend(format!(
                    "CDP auxiliary child {child_id} native detach failed: {error}"
                )))
            }
        }
    }

    #[cfg(feature = "browser-cdp-session")]
    async fn compensate_browser_session(
        &self,
        token: u64,
        session_id: &str,
    ) -> Result<(), HostError> {
        let detached = CdpSession::browser(Arc::clone(&self.connection))
            .target_detach(session_id)
            .await;
        match detached {
            Ok(()) => {
                let topology = self
                    .parent
                    .clear_public_browser_session(session_id)
                    .map_err(HostError::backend);
                let mut registry = self.registry()?;
                if registry.entries.remove(&token).is_some() {
                    registry.signal_event_change();
                }
                if topology.is_err() {
                    self.connection.request_disconnect();
                }
                topology.map(|_| ())
            }
            Err(error) => {
                let tracked = if let Ok(mut registry) = self.registry() {
                    if !registry.entries.contains_key(&token) {
                        return Ok(());
                    }
                    let matches = registry.native_child(token) == Some(session_id);
                    if matches {
                        registry.abandon(token);
                    }
                    matches
                } else {
                    false
                };
                if !tracked {
                    self.connection.request_disconnect();
                }
                Err(HostError::Backend(format!(
                    "CDP browser session {session_id} native detach failed: {error}"
                )))
            }
        }
    }

    pub(crate) async fn shutdown(&self, native_available: bool) -> Result<(), HostError> {
        self.stop.cancel();
        if let Some(task) = self.receiver_task.lock().await.take() {
            task.await.map_err(HostError::backend)?;
        }
        let sessions = self.registry()?.clear();
        let parent_id = self.parent_native.lock().await.take();
        let mut outcome = Ok(());
        if native_available {
            for (owner, session_id) in &sessions {
                let detached = match owner {
                    SessionOwner::Page { .. } => {
                        let Some(parent_id) = parent_id.as_ref() else {
                            self.connection.request_disconnect();
                            outcome = Err(HostError::Backend(
                                "CDP Page attachment parent is unavailable during shutdown".into(),
                            ));
                            break;
                        };
                        CdpSession::new(
                            Arc::clone(&self.connection),
                            parent_id.clone(),
                            String::new(),
                        )
                        .target_detach(session_id)
                        .await
                    }
                    #[cfg(feature = "browser-cdp-session")]
                    SessionOwner::Browser => {
                        CdpSession::browser(Arc::clone(&self.connection))
                            .target_detach(session_id)
                            .await
                    }
                };
                if let Err(error) = detached {
                    outcome = Err(HostError::backend(error));
                }
            }
            if let Some(parent_id) = parent_id.as_ref()
                && let Err(error) = CdpSession::browser(Arc::clone(&self.connection))
                    .target_detach(parent_id)
                    .await
            {
                outcome = Err(HostError::backend(error));
            }
        }
        for (owner, session_id) in sessions {
            #[cfg(feature = "browser-cdp-session")]
            if matches!(owner, SessionOwner::Browser) {
                self.parent
                    .clear_public_browser_session(&session_id)
                    .map_err(HostError::backend)?;
            }
            #[cfg(not(feature = "browser-cdp-session"))]
            let _ = (owner, session_id);
        }
        if let Some(parent_id) = parent_id {
            self.parent.clear(&parent_id).map_err(HostError::backend)?;
        }
        outcome
    }
}

async fn wait_for_event_generation(
    mut changes: watch::Receiver<u64>,
    after: u64,
    stop: &CancellationToken,
) -> Result<u64, HostError> {
    loop {
        // borrow_and_update and changed form a race-free check/park pair.
        // An unacknowledged batch does not advance this generation, so a
        // failed foreign callback cannot cause a tight redelivery loop.
        let current = *changes.borrow_and_update();
        if current != after {
            return Ok(current);
        }
        tokio::select! {
            biased;
            () = stop.cancelled() => return Err(HostError::TargetClosed),
            changed = changes.changed() => changed.map_err(|_| HostError::TargetClosed)?,
        }
    }
}

impl Drop for Installation {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

trait ClaimRegistryOwner {
    fn claim_registry(&self) -> Result<std::sync::MutexGuard<'_, Registry>, HostError>;
}

impl ClaimRegistryOwner for Arc<Installation> {
    fn claim_registry(&self) -> Result<std::sync::MutexGuard<'_, Registry>, HostError> {
        self.registry()
    }
}

#[cfg(test)]
impl ClaimRegistryOwner for Arc<Mutex<Registry>> {
    fn claim_registry(&self) -> Result<std::sync::MutexGuard<'_, Registry>, HostError> {
        self.lock().map_err(HostError::backend)
    }
}

struct ClaimReceipt<Owner: ClaimRegistryOwner> {
    owner: Owner,
    token: u64,
    generation: u64,
}

impl<Owner: ClaimRegistryOwner> ClaimReceipt<Owner> {
    fn peek(&self) -> Result<Option<CdpSessionEvent>, HostError> {
        self.owner
            .claim_registry()?
            .peek_claim(self.token, self.generation)
    }

    fn ack_next(&self) -> Result<(), HostError> {
        self.owner
            .claim_registry()?
            .ack_claim(self.token, self.generation)
    }

    fn rollback(&self) -> Result<(), HostError> {
        self.owner
            .claim_registry()?
            .rollback_claim(self.token, self.generation)
    }
}

impl<Owner: ClaimRegistryOwner> Drop for ClaimReceipt<Owner> {
    fn drop(&mut self) {
        if let Err(error) = self.rollback() {
            tracing::error!(%error, "CDP event claim Drop rollback failed");
        }
    }
}

/// A bounded, cancellation-safe FIFO claim over one CDP session's events.
///
/// The installation lease stays alive until this batch is released. Callers
/// acknowledge only events they have delivered; dropping or rolling back the
/// batch restores its unacknowledged suffix before newer queued events.
pub struct CdpSessionEventBatch {
    claim: ClaimReceipt<Arc<Installation>>,
}

impl CdpSessionEventBatch {
    /// Clone the next unacknowledged event without removing its native owner.
    pub fn peek(&self) -> Result<Option<CdpSessionEvent>, HostError> {
        self.claim.peek()
    }

    /// Acknowledge exactly the event last observed by `peek`.
    pub fn ack_next(&self) -> Result<(), HostError> {
        self.claim.ack_next()
    }

    /// Restore all unacknowledged events. Repeated or terminal rollback is safe.
    pub fn rollback(&self) -> Result<(), HostError> {
        self.claim.rollback()
    }
}

#[cfg(feature = "browser-cdp-session")]
fn record_event(
    registry: &mut Registry,
    parent: &ChromiumAuxiliarySessionParent,
    event: &CdpEvent,
) {
    let detached = match event.method.as_str() {
        "Target.detachedFromTarget" => event
            .params
            .get("sessionId")
            .and_then(Value::as_str)
            .filter(|session_id| {
                event.session_id.is_none() || event.session_id.as_deref() == Some(*session_id)
            }),
        "Inspector.detached" => event.session_id.as_deref(),
        _ => None,
    };
    let retired_browser = detached.and_then(|session_id| {
        registry
            .entries
            .values()
            .any(|entry| {
                matches!(entry.owner, SessionOwner::Browser)
                    && entry.session_id.as_deref() == Some(session_id)
            })
            .then(|| session_id.to_owned())
    });
    registry.record(event);
    if let Some(session_id) = retired_browser
        && let Err(error) = parent.clear_public_browser_session(&session_id)
    {
        tracing::error!(%session_id, %error, "CDP browser session topology retirement failed");
    }
}

#[cfg(not(feature = "browser-cdp-session"))]
fn record_event(
    registry: &mut Registry,
    _parent: &ChromiumAuxiliarySessionParent,
    event: &CdpEvent,
) {
    registry.record(event);
}

async fn receive_events(
    mut receiver: CdpEventReceiver,
    registry: Arc<Mutex<Registry>>,
    parent: Arc<ChromiumAuxiliarySessionParent>,
    mut barriers: mpsc::Receiver<oneshot::Sender<()>>,
    stop: CancellationToken,
) {
    loop {
        tokio::select! {
            biased;
            () = stop.cancelled() => break,
            Some(reply) = barriers.recv() => {
                loop {
                    match receiver.try_recv() {
                        Ok(event) => {
                            if let Ok(mut registry) = registry.lock() {
                                record_event(&mut registry, &parent, &event);
                            }
                        }
                        Err(CdpEventTryRecvError::Empty) => break,
                        Err(CdpEventTryRecvError::Lagged(_)) => {
                            if let Ok(mut registry) = registry.lock() {
                                registry.mark_lagged();
                            }
                        }
                        Err(CdpEventTryRecvError::Closed) => return,
                    }
                }
                let _ = reply.send(());
            }
            event = receiver.recv() => {
                match event {
                    Ok(event) => {
                        if let Ok(mut registry) = registry.lock() {
                            record_event(&mut registry, &parent, &event);
                        }
                    }
                    Err(CdpEventRecvError::Lagged(_)) => {
                        if let Ok(mut registry) = registry.lock() {
                            registry.mark_lagged();
                        }
                    }
                    Err(CdpEventRecvError::Closed) => break,
                }
            }
        }
    }
}

fn portable_id(value: &str, kind: &str) -> Result<ObjectId, HostError> {
    value
        .parse::<u64>()
        .ok()
        .filter(|value| *value != 0)
        .map(ObjectId::new)
        .ok_or_else(|| HostError::InvalidArgument(format!("invalid portable {kind} ID")))
}

/// Unlike ordinary CDP sends, native attach must keep its command registration
/// until a response or transport loss. On host shutdown, disconnecting this
/// host's transport retires an otherwise silent peer and lets cleanup proceed;
/// it sacrifices graceful CDP cleanup only when an attach is still pending.
async fn await_attach_response<F, D>(
    response: F,
    stop: &CancellationToken,
    disconnect: D,
) -> Result<Value, HostError>
where
    F: Future<Output = pwright_cdp::connection::Result<Value>>,
    D: FnOnce(),
{
    tokio::pin!(response);
    tokio::select! {
        biased;
        result = &mut response => result.map_err(HostError::backend),
        () = stop.cancelled() => {
            disconnect();
            response.await.map_err(HostError::backend)
        }
    }
}

async fn attach_to_settlement(
    session: &CdpSession,
    method: &str,
    params: Value,
    stop: &CancellationToken,
) -> Result<String, HostError> {
    if stop.is_cancelled() {
        return Err(HostError::TargetClosed);
    }
    let value = await_attach_response(session.send_to_settlement(method, params), stop, || {
        session.connection().request_disconnect()
    })
    .await?;
    value
        .get("sessionId")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| HostError::Backend(format!("{method} returned no CDP session ID")))
}

/// A queued token is not published until the awaiting host future confirms
/// receipt. Dropping this receipt also drops its acknowledgement sender, so
/// the retained native task can compensate a child whose result was unread.
struct AttachmentReceipt {
    result: oneshot::Receiver<Result<u64, HostError>>,
    acknowledgement: oneshot::Sender<()>,
}

impl AttachmentReceipt {
    async fn receive(self) -> Result<u64, HostError> {
        let Self {
            result,
            acknowledgement,
        } = self;
        let token = result.await.map_err(|_| HostError::TargetClosed)??;
        acknowledgement
            .send(())
            .map_err(|_| HostError::TargetClosed)?;
        Ok(token)
    }
}

impl HostSession {
    fn cdp_installation(&self) -> Result<Arc<Installation>, HostError> {
        if !self.is_connected() {
            return Err(HostError::TargetClosed);
        }
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        Ok(Arc::clone(&live.cdp_sessions))
    }

    fn cdp_live_page(
        &self,
        page: ObjectId,
        context: ObjectId,
        target_id: &str,
    ) -> Result<(), HostError> {
        live_page(
            &self.state,
            &self.context_metadata,
            page,
            context,
            target_id,
        )
    }

    fn cdp_live_owner(&self, owner: &SessionOwner) -> Result<(), HostError> {
        match owner {
            SessionOwner::Page {
                context,
                page,
                target_id,
            } => self.cdp_live_page(*page, *context, target_id),
            #[cfg(feature = "browser-cdp-session")]
            SessionOwner::Browser => {
                if self.is_connected() {
                    Ok(())
                } else {
                    Err(HostError::TargetClosed)
                }
            }
        }
    }

    #[cfg(feature = "browser-cdp-session")]
    pub(crate) fn cdp_browser_target_fence(&self) -> Result<BrowserTargetFence, HostError> {
        Ok(self.cdp_installation()?.registry()?.target_fence())
    }

    #[cfg(feature = "browser-cdp-session")]
    pub(crate) async fn cdp_wait_browser_target_fence(
        &self,
        page_id: &str,
        fence: &BrowserTargetFence,
    ) -> Result<(), HostError> {
        if fence.0.is_empty() {
            return Ok(());
        }
        let page = portable_id(page_id, "Page")?;
        self.flush_topology().await?;
        let target_id = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            topology
                .topology()
                .page_session_routes()
                .find(|route| route.object_id() == page)
                .map(|route| route.target_id().to_owned())
                .ok_or(HostError::TargetClosed)?
        };
        self.cdp_installation()?
            .wait_for_target_created(fence, &target_id)
            .await
    }

    /// Attach a public browser-root session independently of the private
    /// parent used for Page CDP children.
    #[cfg(feature = "browser-cdp-session")]
    pub async fn cdp_browser_session_create(&self) -> Result<u64, HostError> {
        let _operation = self.operations.token();
        let installation = self.cdp_installation()?;
        let token = installation.registry()?.reserve_browser()?;
        let (reply, completion) = oneshot::channel();
        let (acknowledgement, acknowledged) = oneshot::channel();
        let stop = self.stop.clone();
        let operation = self.operations.token();
        tokio::spawn(async move {
            let _operation = operation;
            let attached = async {
                let browser = CdpSession::browser(Arc::clone(&installation.connection));
                let session_id = attach_to_settlement(
                    &browser,
                    "Target.attachToBrowserTarget",
                    json!({}),
                    &stop,
                )
                .await?;
                let registered = installation
                    .parent
                    .register_public_browser_session(&session_id);
                if let Err(pwright_composition_chromium::ChromiumAuxiliarySessionParentError::PublicSessionAlreadyRegistered) = registered {
                    installation.connection.request_disconnect();
                    return Err(HostError::Backend(
                        "CDP reused an active browser session ID; connection retired".into(),
                    ));
                }
                let committed = registered
                    .map_err(HostError::backend)
                    .and_then(|()| installation.registry()?.commit(token, session_id.clone()));
                if let Err(error) = committed {
                    return match installation
                        .compensate_browser_session(token, &session_id)
                        .await
                    {
                        Ok(()) => Err(error),
                        Err(rollback) => Err(HostError::Backend(format!(
                            "CDP browser attachment rejected ({error}); {rollback}"
                        ))),
                    };
                }
                if stop.is_cancelled() {
                    installation
                        .compensate_browser_session(token, &session_id)
                        .await?;
                    return Err(HostError::TargetClosed);
                }
                Ok(token)
            }
            .await;
            if attached.is_err()
                && let Ok(mut registry) = installation.registry()
                && !registry
                    .entries
                    .get(&token)
                    .is_some_and(|entry| entry.abandoned)
            {
                registry.entries.remove(&token);
            }
            let created = attached.is_ok();
            let delivered = reply.send(attached).is_ok();
            let accepted = if created && delivered {
                tokio::select! {
                    biased;
                    () = stop.cancelled() => false,
                    result = acknowledged => result.is_ok(),
                }
            } else {
                false
            };
            if created && !accepted {
                let native_id = installation
                    .registry()
                    .ok()
                    .and_then(|registry| registry.native_child(token).map(str::to_owned));
                if let Some(session_id) = native_id {
                    if let Err(error) = installation
                        .compensate_browser_session(token, &session_id)
                        .await
                    {
                        tracing::error!(token, %error, "unclaimed CDP browser session cleanup remains pending");
                    }
                } else {
                    installation.connection.request_disconnect();
                }
            }
        });
        AttachmentReceipt {
            result: completion,
            acknowledgement,
        }
        .receive()
        .await
    }

    /// Attach an independent child session for a Page or its exact main Frame.
    pub async fn cdp_session_create(
        &self,
        context_id: &str,
        page_id: &str,
        frame_id: Option<&str>,
    ) -> Result<u64, HostError> {
        let _operation = self.operations.token();
        let context = portable_id(context_id, "Context")?;
        let page = portable_id(page_id, "Page")?;
        self.flush_topology().await?;
        let target_id = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let context_route = topology
                .topology()
                .browser_context_route(context)
                .ok_or(HostError::TargetClosed)?;
            if self
                .context_metadata
                .visible(
                    context,
                    context_route.ownership()
                        == pwright_backend_cdp_topology::CdpContextOwnership::Owned,
                )?
                .is_none()
            {
                return Err(HostError::TargetClosed);
            }
            let route = topology
                .topology()
                .page_session_routes()
                .find(|route| {
                    route.object_id() == page && route.browser_context_object_id() == context
                })
                .ok_or(HostError::TargetClosed)?;
            if let Some(frame_id) = frame_id {
                let frame = portable_id(frame_id, "Frame")?;
                if self.frame_trees.get(page)?.main_frame_snapshot()?.id() != frame {
                    return Err(HostError::UnsupportedOperation(
                        "CDP session requires the Page main Frame".into(),
                    ));
                }
            }
            route.target_id().to_owned()
        };
        let installation = self.cdp_installation()?;
        let token = installation
            .registry()?
            .reserve(context, page, target_id.clone())?;
        // The native attach must settle even if an FFI caller drops its
        // awaiter. Sending the token alone is insufficient: the result may be
        // queued but unread when cancellation drops the receiving future.
        let (reply, completion) = oneshot::channel();
        let (acknowledgement, acknowledged) = oneshot::channel();
        let state = Arc::clone(&self.state);
        let metadata = self.context_metadata.clone();
        let stop = self.stop.clone();
        let operation = self.operations.token();
        tokio::spawn(async move {
            let _operation = operation;
            let attached = async {
                if stop.is_cancelled() {
                    return Err(HostError::TargetClosed);
                }
                let parent_id = installation.parent_session(&stop).await?;
                if stop.is_cancelled() {
                    return Err(HostError::TargetClosed);
                }
                let parent_session = CdpSession::new(
                    Arc::clone(&installation.connection),
                    parent_id,
                    String::new(),
                );
                let child_id = attach_to_settlement(
                    &parent_session,
                    "Target.attachToTarget",
                    json!({"targetId": target_id, "flatten": true}),
                    &stop,
                )
                .await?;
                let committed = {
                    let mut registry = installation.registry()?;
                    registry.commit(token, child_id.clone())
                };
                let live = if stop.is_cancelled() {
                    Err(HostError::TargetClosed)
                } else {
                    live_page(&state, &metadata, page, context, &target_id)
                };
                if let Err(error) = committed.and(live) {
                    return match installation
                        .compensate_child(token, &parent_session, &child_id)
                        .await
                    {
                        Ok(()) => Err(error),
                        Err(rollback) => Err(HostError::Backend(format!(
                            "CDP auxiliary attach rejected ({error}); {rollback}"
                        ))),
                    };
                }
                Ok(token)
            }
            .await;
            if attached.is_err()
                && let Ok(mut registry) = installation.registry()
                && !registry
                    .entries
                    .get(&token)
                    .is_some_and(|entry| entry.abandoned)
            {
                registry.entries.remove(&token);
            }
            let created_child = attached.is_ok();
            let delivered = reply.send(attached).is_ok();
            let accepted = if created_child && delivered {
                tokio::select! {
                    biased;
                    () = stop.cancelled() => false,
                    result = acknowledged => result.is_ok(),
                }
            } else {
                false
            };
            if created_child && !accepted {
                let child_state = installation.registry().ok().map(|registry| {
                    (
                        registry.entries.contains_key(&token),
                        registry.native_child(token).map(str::to_owned),
                    )
                });
                let parent_id = installation.parent_native.lock().await.clone();
                if let (Some((_, Some(child_id))), Some(parent_id)) = (&child_state, parent_id) {
                    let parent_session = CdpSession::new(
                        Arc::clone(&installation.connection),
                        parent_id,
                        String::new(),
                    );
                    if let Err(error) = installation
                        .compensate_child(token, &parent_session, child_id)
                        .await
                    {
                        tracing::error!(token, %error, "unclaimed CDP child cleanup remains pending");
                    }
                } else if !matches!(child_state, Some((false, None))) {
                    if let Ok(mut registry) = installation.registry() {
                        registry.abandon(token);
                    }
                    installation.connection.request_disconnect();
                }
            }
        });
        AttachmentReceipt {
            result: completion,
            acknowledgement,
        }
        .receive()
        .await
    }

    /// Send a bounded command on the exact auxiliary session, then publish
    /// earlier CDP events into that session's bounded drain queue.
    pub async fn cdp_session_send(
        &self,
        token: u64,
        method: &str,
        params: Value,
    ) -> Result<Value, HostError> {
        let _operation = self.operations.token();
        let valid_method = method.split_once('.').is_some_and(|(domain, command)| {
            !domain.is_empty()
                && !command.is_empty()
                && domain.bytes().all(|byte| byte.is_ascii_alphanumeric())
                && command
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        });
        if method.len() > MAX_METHOD_BYTES || !valid_method {
            return Err(HostError::InvalidArgument("invalid CDP method name".into()));
        }
        if !params.is_object()
            || serde_json::to_vec(&params)
                .map_err(HostError::backend)?
                .len()
                > MAX_COMMAND_PARAMS_BYTES
        {
            return Err(HostError::InvalidArgument(
                "CDP command parameters must be a bounded object".into(),
            ));
        }
        let installation = self.cdp_installation()?;
        let (owner, session_id) = installation.registry()?.active(token)?;
        self.cdp_live_owner(&owner)?;
        #[cfg(feature = "browser-cdp-session")]
        let discovery = (method == "Target.setDiscoverTargets"
            && matches!(owner, SessionOwner::Browser))
        .then(|| discovers_page_targets(&params));
        let target_id = match owner {
            SessionOwner::Page { target_id, .. } => target_id,
            #[cfg(feature = "browser-cdp-session")]
            SessionOwner::Browser => String::new(),
        };
        let result = CdpSession::new(Arc::clone(&installation.connection), session_id, target_id)
            .send(method, params)
            .await
            .map_err(HostError::backend)?;
        installation.synchronize().await?;
        #[cfg(feature = "browser-cdp-session")]
        if let Some(enabled) = discovery {
            installation
                .registry()?
                .set_discover_targets(token, enabled)?;
        }
        Ok(result)
    }

    /// Drain at most `max_events` after all already-received CDP events have
    /// passed the receiver barrier.
    pub async fn cdp_session_drain(
        &self,
        token: u64,
        max_events: usize,
    ) -> Result<Vec<CdpSessionEvent>, HostError> {
        let _operation = self.operations.token();
        let installation = self.cdp_installation()?;
        installation.synchronize().await?;
        let (owner, _) = installation.registry()?.active(token)?;
        self.cdp_live_owner(&owner)?;
        installation.registry()?.drain(token, max_events)
    }

    /// Claim a bounded FIFO event batch for per-event delivery acknowledgement.
    /// Dropping the result restores its unacknowledged suffix automatically.
    pub async fn cdp_session_claim(
        &self,
        token: u64,
        max_events: usize,
    ) -> Result<CdpSessionEventBatch, HostError> {
        let _operation = self.operations.token();
        let installation = self.cdp_installation()?;
        installation.synchronize().await?;
        let (owner, _) = installation.registry()?.active(token)?;
        self.cdp_live_owner(&owner)?;
        let generation = installation.registry()?.claim(token, max_events)?;
        Ok(CdpSessionEventBatch {
            claim: ClaimReceipt {
                owner: installation,
                token,
                generation,
            },
        })
    }

    /// Park until a CDP session event, overflow, or native retirement changes
    /// the bounded registry. The caller drains with `cdp_session_claim` and
    /// acknowledges events independently; this watch never consumes payloads.
    pub async fn next_cdp_session_event_generation(&self, after: u64) -> Result<u64, HostError> {
        let _operation = self.operations.token();
        let installation = self.cdp_installation()?;
        installation.synchronize().await?;
        installation.next_event_generation(after).await
    }

    /// Explicitly release one auxiliary child without changing Page ownership.
    pub async fn cdp_session_detach(&self, token: u64) -> Result<(), HostError> {
        let _operation = self.operations.token();
        let installation = self.cdp_installation()?;
        let (owner, _) = installation.registry()?.active(token)?;
        self.cdp_live_owner(&owner)?;
        let session_id = installation.registry()?.begin_detach(token)?;
        let (reply, completion) = oneshot::channel();
        let operation = self.operations.token();
        tokio::spawn(async move {
            let _operation = operation;
            let detached = async {
                let result = match &owner {
                    SessionOwner::Page { .. } => {
                        let parent_id = installation.attached_parent_session().await?;
                        CdpSession::new(
                            Arc::clone(&installation.connection),
                            parent_id,
                            String::new(),
                        )
                        .target_detach(&session_id)
                        .await
                    }
                    #[cfg(feature = "browser-cdp-session")]
                    SessionOwner::Browser => {
                        CdpSession::browser(Arc::clone(&installation.connection))
                            .target_detach(&session_id)
                            .await
                    }
                };
                result.map_err(HostError::backend)
            }
            .await;
            let topology: Result<(), HostError> = if detached.is_ok() {
                #[cfg(feature = "browser-cdp-session")]
                {
                    if matches!(owner, SessionOwner::Browser) {
                        installation
                            .parent
                            .clear_public_browser_session(&session_id)
                            .map(|_| ())
                            .map_err(HostError::backend)
                    } else {
                        Ok(())
                    }
                }
                #[cfg(not(feature = "browser-cdp-session"))]
                Ok(())
            } else {
                Ok(())
            };
            if let Ok(mut registry) = installation.registry() {
                registry.finish_detach(token, detached.is_ok());
            }
            if topology.is_err() {
                installation.connection.request_disconnect();
            }
            let _ = reply.send(detached.and(topology));
        });
        completion.await.map_err(|_| HostError::TargetClosed)?
    }

    /// Compensate a child whose foreign delivery lease was never claimed.
    /// This intentionally skips portable Page liveness: Page retirement must
    /// not prevent native cleanup of a child already attached to Chromium.
    pub async fn cdp_session_cleanup_unclaimed(&self, token: u64) -> Result<(), HostError> {
        let _operation = self.operations.token();
        let installation = self.cdp_installation()?;
        let owner = installation
            .registry()?
            .entries
            .get(&token)
            .map(|entry| entry.owner.clone());
        let child_id = installation.registry()?.begin_unclaimed_cleanup(token)?;
        match owner.ok_or(HostError::TargetClosed)? {
            SessionOwner::Page { .. } => {
                let parent_id = installation.attached_parent_session().await?;
                let parent = CdpSession::new(
                    Arc::clone(&installation.connection),
                    parent_id,
                    String::new(),
                );
                installation
                    .compensate_child(token, &parent, &child_id)
                    .await
            }
            #[cfg(feature = "browser-cdp-session")]
            SessionOwner::Browser => {
                installation
                    .compensate_browser_session(token, &child_id)
                    .await
            }
        }
    }
}

fn live_page(
    state: &Arc<Mutex<Option<LiveSession>>>,
    metadata: &ContextMetadata,
    page: ObjectId,
    context: ObjectId,
    target_id: &str,
) -> Result<(), HostError> {
    let state = state.lock().map_err(HostError::backend)?;
    let live = state.as_ref().ok_or(HostError::TargetClosed)?;
    let topology = live.composition.topology().map_err(HostError::backend)?;
    let visible_context = topology
        .topology()
        .browser_context_route(context)
        .map(|route| {
            metadata.visible(
                context,
                route.ownership() == pwright_backend_cdp_topology::CdpContextOwnership::Owned,
            )
        })
        .transpose()?
        .flatten()
        .is_some();
    let live = topology.topology().page_session_routes().any(|route| {
        route.object_id() == page
            && route.browser_context_object_id() == context
            && route.target_id() == target_id
    });
    if visible_context && live {
        Ok(())
    } else {
        Err(HostError::TargetClosed)
    }
}

pub(crate) fn reconcile_state(
    state: &Arc<Mutex<Option<LiveSession>>>,
    installation: &Installation,
) -> Result<(), HostError> {
    let state = state.lock().map_err(HostError::backend)?;
    let Some(live) = state.as_ref() else {
        return Ok(());
    };
    let topology = live.composition.topology().map_err(HostError::backend)?;
    let pages: Vec<_> = topology
        .topology()
        .materialized_page_memberships()
        .map(|(page, _)| page)
        .collect();
    installation.reconcile_pages(&pages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[tokio::test]
    async fn event_watch_wakes_for_exact_child_without_consuming_payload() {
        // Arrange
        let (owner, token) = claimed_registry();
        let changes = owner.lock().unwrap().subscribe_events();
        let stop = CancellationToken::new();
        owner.lock().unwrap().record(&CdpEvent::new(
            "Runtime.event".into(),
            json!({"value": 0}),
            Some("other".into()),
        ));

        // Act
        owner.lock().unwrap().record(&event(1));
        let generation = wait_for_event_generation(changes, 0, &stop).await.unwrap();
        let payload = owner.lock().unwrap().drain(token, 1).unwrap();

        // Assert
        assert_eq!(generation, 1);
        assert_eq!(payload[0].params["value"], 1);
    }

    #[tokio::test]
    async fn rolled_back_batch_does_not_spin_event_watch() {
        // Arrange
        let (owner, token) = claimed_registry();
        owner.lock().unwrap().record(&event(1));
        let changes = owner.lock().unwrap().subscribe_events();
        let stop = CancellationToken::new();
        let batch = claim(&owner, token, 1);
        batch.rollback().unwrap();
        let stopped = stop.clone();
        let wait =
            tokio::spawn(async move { wait_for_event_generation(changes, 1, &stopped).await });
        tokio::task::yield_now().await;

        // Act
        let parked = !wait.is_finished();
        owner.lock().unwrap().record(&event(2));
        let generation = wait.await.unwrap().unwrap();

        // Assert
        assert!(parked);
        assert_eq!(generation, 2);
    }

    #[tokio::test]
    async fn cancelled_event_watch_releases_waiter_without_new_event() {
        // Arrange
        let (owner, _) = claimed_registry();
        let changes = owner.lock().unwrap().subscribe_events();
        let stop = CancellationToken::new();
        let cancelled = stop.clone();
        let wait =
            tokio::spawn(async move { wait_for_event_generation(changes, 0, &cancelled).await });
        tokio::task::yield_now().await;

        // Act
        stop.cancel();
        let outcome = wait.await.unwrap();

        // Assert
        assert!(matches!(outcome, Err(HostError::TargetClosed)));
    }

    #[test]
    fn unclaimed_cleanup_keeps_native_child_after_page_retirement() {
        // Arrange
        let (owner, token) = claimed_registry();
        owner.lock().unwrap().retain_pages(&[]);

        // Act
        let child = owner
            .lock()
            .unwrap()
            .begin_unclaimed_cleanup(token)
            .unwrap();

        // Assert
        assert_eq!(child, "child");
        let registry = owner.lock().unwrap();
        assert_eq!(registry.native_child(token), Some("child"));
        assert!(matches!(
            registry.active(token),
            Err(HostError::TargetClosed)
        ));
    }

    fn claimed_registry() -> (Arc<Mutex<Registry>>, u64) {
        let mut registry = Registry::new();
        let token = registry
            .reserve(ObjectId::new(1), ObjectId::new(2), "page".into())
            .unwrap();
        registry.commit(token, "child".into()).unwrap();
        (Arc::new(Mutex::new(registry)), token)
    }

    fn event(value: u64) -> CdpEvent {
        CdpEvent::new(
            "Runtime.event".into(),
            json!({"value": value}),
            Some("child".into()),
        )
    }

    fn claim(
        owner: &Arc<Mutex<Registry>>,
        token: u64,
        max_events: usize,
    ) -> ClaimReceipt<Arc<Mutex<Registry>>> {
        let generation = owner.lock().unwrap().claim(token, max_events).unwrap();
        ClaimReceipt {
            owner: Arc::clone(owner),
            token,
            generation,
        }
    }

    #[test]
    fn partial_ack_drop_prepends_unacked_suffix_before_newer_events() {
        // Arrange
        let (owner, token) = claimed_registry();
        {
            let mut registry = owner.lock().unwrap();
            for value in 1..=3 {
                registry.record(&event(value));
            }
        }
        let batch = claim(&owner, token, 3);
        assert_eq!(batch.peek().unwrap().unwrap().params["value"], 1);
        batch.ack_next().unwrap();
        owner.lock().unwrap().record(&event(4));

        // Act
        drop(batch);
        let drained = owner.lock().unwrap().drain(token, 4).unwrap();

        // Assert
        let values: Vec<_> = drained
            .iter()
            .map(|event| event.params["value"].as_u64().unwrap())
            .collect();
        assert_eq!(values, vec![2, 3, 4]);
        assert_eq!(owner.lock().unwrap().entries[&token].event_bytes, 0);
    }

    #[test]
    fn claim_requires_peek_before_each_ack_and_exhaustion_peeks_none() {
        // Arrange
        let (owner, token) = claimed_registry();
        owner.lock().unwrap().record(&event(1));
        let batch = claim(&owner, token, 1);

        // Act
        let premature = batch.ack_next();
        let first = batch.peek().unwrap();
        batch.ack_next().unwrap();
        let exhausted = batch.peek().unwrap();
        let repeated = batch.ack_next();

        // Assert
        assert!(matches!(premature, Err(HostError::InvalidArgument(_))));
        assert_eq!(first.unwrap().params["value"], 1);
        assert!(exhausted.is_none());
        assert!(matches!(repeated, Err(HostError::InvalidArgument(_))));
        assert_eq!(owner.lock().unwrap().entries[&token].event_bytes, 0);
    }

    #[test]
    fn queued_and_claimed_events_share_the_item_cap_and_block_legacy_drain() {
        // Arrange
        let (owner, token) = claimed_registry();
        {
            let mut registry = owner.lock().unwrap();
            for value in 0..MAX_CDP_SESSION_EVENTS {
                registry.record(&event(value as u64));
            }
        }
        let batch = claim(&owner, token, 32);

        // Act
        let legacy = owner.lock().unwrap().drain(token, 1);
        owner.lock().unwrap().record(&event(65));

        // Assert
        let registry = owner.lock().unwrap();
        assert!(matches!(legacy, Err(HostError::InvalidArgument(_))));
        assert_eq!(
            registry.entries[&token].event_count(),
            MAX_CDP_SESSION_EVENTS
        );
        assert_eq!(registry.entries[&token].events.len(), 32);
        assert_eq!(
            registry.entries[&token]
                .claim
                .as_ref()
                .unwrap()
                .events
                .len(),
            32
        );
        assert!(registry.entries[&token].overflowed);
        drop(registry);
        assert!(matches!(batch.peek(), Err(HostError::ResourceExhausted(_))));
    }

    #[test]
    fn claimed_bytes_remain_counted_against_the_one_megabyte_cap() {
        // Arrange
        let (owner, token) = claimed_registry();
        let method = "Runtime.large";
        let empty = json!({"payload": ""});
        let overhead = serde_json::to_vec(&empty).unwrap().len() + method.len();
        let payload = "x".repeat(MAX_CDP_SESSION_EVENT_BYTES - overhead);
        owner.lock().unwrap().record(&CdpEvent::new(
            method.into(),
            json!({"payload": payload}),
            Some("child".into()),
        ));
        let batch = claim(&owner, token, 1);

        // Act
        owner.lock().unwrap().record(&event(2));

        // Assert
        let registry = owner.lock().unwrap();
        assert_eq!(
            registry.entries[&token].event_bytes,
            MAX_CDP_SESSION_EVENT_BYTES
        );
        assert_eq!(registry.entries[&token].event_count(), 1);
        assert!(registry.entries[&token].overflowed);
        drop(registry);
        assert!(matches!(batch.peek(), Err(HostError::ResourceExhausted(_))));
    }

    #[test]
    fn duplicate_claim_is_rejected_and_stale_receipt_cannot_touch_new_generation() {
        // Arrange
        let (owner, token) = claimed_registry();
        owner.lock().unwrap().record(&event(1));
        let stale = claim(&owner, token, 1);
        let duplicate = owner.lock().unwrap().claim(token, 1);
        stale.rollback().unwrap();
        let current = claim(&owner, token, 1);

        // Act
        let old_peek = stale.peek();
        let old_ack = stale.ack_next();
        stale.rollback().unwrap();
        drop(stale);

        // Assert
        assert!(matches!(duplicate, Err(HostError::InvalidArgument(_))));
        assert!(matches!(old_peek, Err(HostError::TargetClosed)));
        assert!(matches!(old_ack, Err(HostError::TargetClosed)));
        assert_eq!(current.peek().unwrap().unwrap().params["value"], 1);
    }

    #[test]
    fn page_retirement_invalidates_claim_and_drops_unusable_event_payloads() {
        // Arrange
        let (owner, token) = claimed_registry();
        owner.lock().unwrap().record(&event(1));
        let batch = claim(&owner, token, 1);

        // Act
        owner.lock().unwrap().retain_pages(&[]);
        owner.lock().unwrap().record(&event(2));

        // Assert
        assert!(matches!(batch.peek(), Err(HostError::TargetClosed)));
        assert!(matches!(batch.ack_next(), Err(HostError::TargetClosed)));
        assert!(batch.rollback().is_ok());
        let registry = owner.lock().unwrap();
        assert_eq!(registry.native_child(token), Some("child"));
        assert_eq!(registry.entries[&token].event_count(), 0);
        assert_eq!(registry.entries[&token].event_bytes, 0);
    }

    #[test]
    fn explicit_detach_invalidates_claim_without_restoring_it_on_failure() {
        // Arrange
        let (owner, token) = claimed_registry();
        owner.lock().unwrap().record(&event(1));
        let batch = claim(&owner, token, 1);

        // Act
        owner.lock().unwrap().begin_detach(token).unwrap();
        owner.lock().unwrap().finish_detach(token, false);

        // Assert
        assert!(matches!(batch.peek(), Err(HostError::TargetClosed)));
        assert!(batch.rollback().is_ok());
        let registry = owner.lock().unwrap();
        assert_eq!(registry.entries[&token].event_count(), 1);
        assert!(registry.entries[&token].overflowed);
    }

    #[test]
    fn native_detach_and_shutdown_invalidate_outstanding_claims() {
        // Arrange
        let (first_owner, first_token) = claimed_registry();
        first_owner.lock().unwrap().record(&event(1));
        let first = claim(&first_owner, first_token, 1);
        let (second_owner, second_token) = claimed_registry();
        second_owner.lock().unwrap().record(&event(2));
        let second = claim(&second_owner, second_token, 1);

        // Act
        first_owner.lock().unwrap().record(&CdpEvent::new(
            "Target.detachedFromTarget".into(),
            json!({"sessionId": "child"}),
            Some("parent".into()),
        ));
        second_owner.lock().unwrap().clear();

        // Assert
        assert!(matches!(first.peek(), Err(HostError::TargetClosed)));
        assert!(matches!(second.peek(), Err(HostError::TargetClosed)));
        assert!(first.rollback().is_ok());
        assert!(second.rollback().is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn retained_attach_accepts_native_response_after_ordinary_command_deadline() {
        // Arrange
        let (peer, response) = oneshot::channel();
        let stop = CancellationToken::new();
        let disconnected = Arc::new(AtomicBool::new(false));
        let observed = Arc::clone(&disconnected);
        let wait = tokio::spawn(async move {
            await_attach_response(
                async {
                    response
                        .await
                        .map_err(|_| pwright_cdp::connection::CdpError::Closed)
                },
                &stop,
                || observed.store(true, Ordering::SeqCst),
            )
            .await
        });

        // Act
        tokio::time::advance(std::time::Duration::from_secs(31)).await;
        assert!(!wait.is_finished());
        peer.send(json!({"sessionId": "late-child"})).unwrap();
        let outcome = wait.await.unwrap();

        // Assert
        assert_eq!(outcome.unwrap()["sessionId"], "late-child");
        assert!(!disconnected.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn shutdown_disconnects_a_silent_attach_peer_and_retires_waiter() {
        // Arrange
        let (peer, response) = oneshot::channel::<Value>();
        let peer = Arc::new(Mutex::new(Some(peer)));
        let stop = CancellationToken::new();
        let cancelled = stop.clone();
        let disconnected = Arc::new(AtomicBool::new(false));
        let observed = Arc::clone(&disconnected);
        let pending_peer = Arc::clone(&peer);
        let wait = tokio::spawn(async move {
            await_attach_response(
                async {
                    response
                        .await
                        .map_err(|_| pwright_cdp::connection::CdpError::Closed)
                },
                &cancelled,
                || {
                    observed.store(true, Ordering::SeqCst);
                    pending_peer.lock().unwrap().take();
                },
            )
            .await
        });
        tokio::task::yield_now().await;

        // Act
        stop.cancel();
        let result = tokio::time::timeout(std::time::Duration::from_secs(1), wait)
            .await
            .expect("shutdown must not wait on an unresponsive CDP peer")
            .unwrap();

        // Assert
        assert!(matches!(result, Err(HostError::Backend(_))));
        assert!(disconnected.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn settled_attach_does_not_disconnect_when_stop_is_also_ready() {
        // Arrange
        let (peer, response) = oneshot::channel();
        let stop = CancellationToken::new();
        let disconnected = AtomicBool::new(false);
        peer.send(json!({"sessionId": "child"})).unwrap();
        stop.cancel();

        // Act
        let result = await_attach_response(
            async {
                response
                    .await
                    .map_err(|_| pwright_cdp::connection::CdpError::Closed)
            },
            &stop,
            || disconnected.store(true, Ordering::SeqCst),
        )
        .await;

        // Assert
        assert_eq!(result.unwrap()["sessionId"], "child");
        assert!(!disconnected.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn queued_but_unread_attachment_cannot_be_acknowledged() {
        // Arrange
        let (reply, result) = oneshot::channel();
        let (acknowledgement, acknowledged) = oneshot::channel();
        let receipt = AttachmentReceipt {
            result,
            acknowledgement,
        };
        reply.send(Ok(17)).unwrap();

        // Act
        drop(receipt);
        let accepted = acknowledged.await;

        // Assert
        assert!(accepted.is_err());
    }

    #[tokio::test]
    async fn consumed_attachment_acknowledges_before_returning_token() {
        // Arrange
        let (reply, result) = oneshot::channel();
        let (acknowledgement, acknowledged) = oneshot::channel();
        let receipt = AttachmentReceipt {
            result,
            acknowledgement,
        };
        reply.send(Ok(17)).unwrap();

        // Act
        let received = receipt.receive().await;
        let accepted = acknowledged.await;

        // Assert
        assert_eq!(received.unwrap(), 17);
        assert!(accepted.is_ok());
    }

    #[test]
    fn selected_profile_accounts_for_every_session_queue_and_parent() {
        // Arrange
        let declared = requirements();

        // Act
        let items = declared.class(ResourceClass::EventItem);
        let event_bytes = declared.class(ResourceClass::EventBytes);
        let owner_bytes = declared.class(ResourceClass::BackendStateBytes);

        // Assert
        assert_eq!(
            items.global,
            (MAX_CDP_SESSIONS * MAX_CDP_SESSION_EVENTS) as u64
        );
        assert_eq!(
            event_bytes.global,
            (MAX_CDP_SESSIONS * MAX_CDP_SESSION_EVENT_BYTES) as u64
        );
        assert_eq!(owner_bytes.global, BACKEND_STATE_BYTES);
        assert_eq!(declared.scopes, 1);
        assert_eq!(declared.leases, 1);
    }

    #[test]
    fn registry_rejects_ninth_session_without_reusing_tokens() {
        // Arrange
        let mut registry = Registry::new();
        for index in 0..MAX_CDP_SESSIONS {
            registry
                .reserve(ObjectId::new(1), ObjectId::new(2), format!("page-{index}"))
                .unwrap();
        }

        // Act
        let result = registry.reserve(ObjectId::new(1), ObjectId::new(2), "extra".into());

        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        registry.entries.remove(&1);
        assert_eq!(
            registry
                .reserve(ObjectId::new(1), ObjectId::new(2), "new".into())
                .unwrap(),
            9
        );
    }

    #[cfg(feature = "browser-cdp-session")]
    #[test]
    fn browser_and_page_sessions_share_capacity_but_page_retirement_keeps_browser_live() {
        // Arrange
        let mut registry = Registry::new();
        let page = registry
            .reserve(ObjectId::new(1), ObjectId::new(2), "page".into())
            .unwrap();
        registry.commit(page, "page-session".into()).unwrap();
        let browser = registry.reserve_browser().unwrap();
        registry.commit(browser, "browser-session".into()).unwrap();
        for index in 0..(MAX_CDP_SESSIONS - 2) {
            registry
                .reserve(ObjectId::new(1), ObjectId::new(3), format!("page-{index}"))
                .unwrap();
        }

        // Act
        registry.retain_pages(&[]);
        let additional = registry.reserve_browser();

        // Assert
        assert!(matches!(
            registry.active(page),
            Err(HostError::TargetClosed)
        ));
        assert!(matches!(
            registry.active(browser),
            Ok((SessionOwner::Browser, _))
        ));
        assert!(matches!(additional, Err(HostError::ResourceExhausted(_))));
    }

    #[cfg(feature = "browser-cdp-session")]
    #[test]
    fn nested_browser_detach_is_delivered_but_root_detach_retires_exact_session() {
        // Arrange
        let mut registry = Registry::new();
        let browser = registry.reserve_browser().unwrap();
        registry.commit(browser, "browser-session".into()).unwrap();
        let parent = ChromiumAuxiliarySessionParent::new(MAX_NATIVE_ID_BYTES).unwrap();
        parent
            .register_public_browser_session("browser-session")
            .unwrap();
        let nested = CdpEvent::new(
            "Target.detachedFromTarget".into(),
            json!({"sessionId": "nested-session"}),
            Some("browser-session".into()),
        );
        let root = CdpEvent::new(
            "Target.detachedFromTarget".into(),
            json!({"sessionId": "browser-session"}),
            None,
        );

        // Act
        record_event(&mut registry, &parent, &nested);
        let delivered = registry.drain(browser, 1).unwrap();
        record_event(&mut registry, &parent, &root);

        // Assert
        assert_eq!(delivered[0].method, "Target.detachedFromTarget");
        assert_eq!(delivered[0].params["sessionId"], "nested-session");
        assert!(matches!(
            registry.active(browser),
            Err(HostError::TargetClosed)
        ));
        assert_eq!(
            parent.clear_public_browser_session("browser-session"),
            Ok(false)
        );
    }

    #[cfg(feature = "browser-cdp-session")]
    #[test]
    fn page_including_discovery_filters_activate_the_browser_target_fence() {
        // Arrange
        let mut registry = Registry::new();
        let browser = registry.reserve_browser().unwrap();
        registry.commit(browser, "browser-session".into()).unwrap();
        let filters = [
            json!({"discover": true}),
            json!({"discover": true, "filter": [{"type": "page"}]}),
            json!({"discover": true, "filter": [{"type": "browser", "exclude": true}, {"type": "page"}]}),
            json!({"discover": true, "filter": [{"type": "page", "exclude": false}, {"exclude": true}]}),
            json!({"discover": true, "filter": [{}, {"type": "page", "exclude": true}]}),
        ];

        // Act and assert each independent filter.
        for params in filters {
            registry
                .set_discover_targets(browser, discovers_page_targets(&params))
                .unwrap();
            assert_eq!(registry.target_fence().0.len(), 1, "{params}");
        }
    }

    #[cfg(feature = "browser-cdp-session")]
    #[test]
    fn first_matching_exclusion_or_no_page_match_skips_the_target_fence() {
        // Arrange
        let mut registry = Registry::new();
        let browser = registry.reserve_browser().unwrap();
        registry.commit(browser, "browser-session".into()).unwrap();
        let filters = [
            json!({"discover": false}),
            json!({"discover": false, "filter": []}),
            json!({"discover": true, "filter": []}),
            json!({"discover": true, "filter": [{"type": "browser"}]}),
            json!({"discover": true, "filter": [{"type": "page", "exclude": true}, {}]}),
            json!({"discover": true, "filter": [{"exclude": true}, {"type": "page"}]}),
        ];

        // Act and assert each independent filter.
        for params in filters {
            registry
                .set_discover_targets(browser, discovers_page_targets(&params))
                .unwrap();
            assert!(registry.target_fence().0.is_empty(), "{params}");
        }
    }

    #[cfg(feature = "browser-cdp-session")]
    #[test]
    fn malformed_or_oversized_discovery_filters_keep_the_bounded_fence() {
        // Arrange
        let oversized = vec![json!({"type": "other"}); MAX_TARGET_FILTER_ENTRIES + 1];
        let filters = [
            json!({"discover": true, "filter": null}),
            json!({"discover": true, "filter": {"type": "page"}}),
            json!({"discover": true, "filter": [null]}),
            json!({"discover": true, "filter": [{"type": null}]}),
            json!({"discover": true, "filter": [{"type": "page", "exclude": null}]}),
            json!({"discover": null}),
            json!({"discover": true, "filter": oversized}),
        ];

        // Act and assert each independent filter.
        for params in filters {
            assert!(discovers_page_targets(&params), "{params}");
        }
    }

    #[cfg(feature = "browser-cdp-session")]
    #[test]
    fn target_fence_requires_exact_queued_event_even_after_drain() {
        // Arrange
        let mut registry = Registry::new();
        let browser = registry.reserve_browser().unwrap();
        registry.commit(browser, "browser-session".into()).unwrap();
        registry.set_discover_targets(browser, true).unwrap();
        let fence = registry.target_fence();

        // Act
        registry.record(&CdpEvent::new(
            "Target.targetCreated".into(),
            json!({"targetInfo": {"targetId": "other"}}),
            Some("browser-session".into()),
        ));
        let other_does_not_satisfy = registry.target_fence_satisfied(&fence, "new-page").unwrap();
        registry.record(&CdpEvent::new(
            "Target.targetCreated".into(),
            json!({"targetInfo": {"targetId": "new-page"}}),
            Some("browser-session".into()),
        ));
        registry.drain(browser, 2).unwrap();

        // Assert
        assert!(!other_does_not_satisfy);
        assert!(registry.target_fence_satisfied(&fence, "new-page").unwrap());
        registry.set_discover_targets(browser, false).unwrap();
        assert!(
            registry
                .target_fence_satisfied(&fence, "never-arrived")
                .unwrap()
        );
        assert!(registry.target_fence().0.is_empty());
        registry.set_discover_targets(browser, true).unwrap();
        assert!(
            registry
                .target_fence_satisfied(&fence, "never-arrived")
                .unwrap()
        );
    }

    #[cfg(feature = "browser-cdp-session")]
    #[test]
    fn target_fence_fails_closed_if_event_queue_overflows() {
        // Arrange
        let mut registry = Registry::new();
        let browser = registry.reserve_browser().unwrap();
        registry.commit(browser, "browser-session".into()).unwrap();
        registry.set_discover_targets(browser, true).unwrap();
        let fence = registry.target_fence();
        for index in 0..MAX_CDP_SESSION_EVENTS {
            registry.record(&CdpEvent::new(
                "Runtime.event".into(),
                json!({"value": index}),
                Some("browser-session".into()),
            ));
        }

        // Act
        registry.record(&CdpEvent::new(
            "Target.targetCreated".into(),
            json!({"targetInfo": {"targetId": "new-page"}}),
            Some("browser-session".into()),
        ));
        let result = registry.target_fence_satisfied(&fence, "new-page");

        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    }

    #[test]
    fn exact_child_event_is_queued_and_drained_once() {
        // Arrange
        let mut registry = Registry::new();
        let token = registry
            .reserve(ObjectId::new(1), ObjectId::new(2), "page".into())
            .unwrap();
        registry.commit(token, "child".into()).unwrap();
        let other = CdpEvent::new(
            "Runtime.consoleAPICalled".into(),
            json!({"value": 1}),
            Some("other".into()),
        );
        let owned = CdpEvent::new(
            "Runtime.consoleAPICalled".into(),
            json!({"value": 2}),
            Some("child".into()),
        );

        // Act
        registry.record(&other);
        registry.record(&owned);
        let events = registry.drain(token, 8).unwrap();

        // Assert
        assert_eq!(
            events,
            vec![CdpSessionEvent {
                method: "Runtime.consoleAPICalled".into(),
                params: json!({"value": 2})
            }]
        );
        assert!(registry.drain(token, 8).unwrap().is_empty());
    }

    #[test]
    fn browser_root_event_cannot_fill_pending_child() {
        // Arrange
        let mut registry = Registry::new();
        let token = registry
            .reserve(ObjectId::new(1), ObjectId::new(2), "page".into())
            .unwrap();

        // Act
        registry.record(&CdpEvent::new(
            "Target.targetCreated".into(),
            json!({"targetInfo": {"type": "page"}}),
            None,
        ));
        registry.record(&CdpEvent::new("Inspector.detached".into(), json!({}), None));

        // Assert
        assert!(registry.entries[&token].events.is_empty());
        assert!(registry.entries.contains_key(&token));
    }

    #[test]
    fn native_detach_invalidates_only_exact_child() {
        // Arrange
        let mut registry = Registry::new();
        let first = registry
            .reserve(ObjectId::new(1), ObjectId::new(2), "page".into())
            .unwrap();
        registry.commit(first, "first".into()).unwrap();
        let second = registry
            .reserve(ObjectId::new(1), ObjectId::new(3), "other".into())
            .unwrap();
        registry.commit(second, "second".into()).unwrap();

        // Act
        registry.record(&CdpEvent::new(
            "Target.detachedFromTarget".into(),
            json!({"sessionId": "first"}),
            Some("parent".into()),
        ));

        // Assert
        assert!(matches!(
            registry.active(first),
            Err(HostError::TargetClosed)
        ));
        assert!(registry.active(second).is_ok());
    }

    #[test]
    fn closed_page_invalidates_session_but_retains_native_cleanup_ownership() {
        // Arrange
        let mut registry = Registry::new();
        let first = registry
            .reserve(ObjectId::new(1), ObjectId::new(2), "page".into())
            .unwrap();
        let second = registry
            .reserve(ObjectId::new(1), ObjectId::new(3), "other".into())
            .unwrap();

        // Act
        registry.retain_pages(&[ObjectId::new(3)]);

        // Assert
        assert!(matches!(
            registry.active(first),
            Err(HostError::TargetClosed)
        ));
        assert!(registry.entries.contains_key(&first));
        assert!(registry.entries[&first].abandoned);
        assert!(registry.entries.contains_key(&second));
        assert!(!registry.entries[&second].abandoned);
    }

    #[test]
    fn late_child_on_closed_page_keeps_identity_and_capacity_until_native_detach() {
        // Arrange
        let mut registry = Registry::new();
        let first = registry
            .reserve(ObjectId::new(1), ObjectId::new(2), "page".into())
            .unwrap();
        registry.retain_pages(&[]);

        // Act
        let result = registry.commit(first, "late-child".into());

        // Assert
        assert!(matches!(result, Err(HostError::TargetClosed)));
        assert_eq!(registry.native_child(first), Some("late-child"));
        for index in 1..MAX_CDP_SESSIONS {
            registry
                .reserve(ObjectId::new(1), ObjectId::new(3), format!("other-{index}"))
                .unwrap();
        }
        assert!(matches!(
            registry.reserve(ObjectId::new(1), ObjectId::new(3), "ninth".into()),
            Err(HostError::ResourceExhausted(_))
        ));
        registry.record(&CdpEvent::new(
            "Target.detachedFromTarget".into(),
            json!({"sessionId": "late-child"}),
            Some("parent".into()),
        ));
        assert!(!registry.entries.contains_key(&first));
        assert!(
            registry
                .reserve(ObjectId::new(1), ObjectId::new(3), "replacement".into())
                .is_ok()
        );
    }

    #[test]
    fn event_capacity_fails_closed_without_growing_queue() {
        // Arrange
        let mut registry = Registry::new();
        let token = registry
            .reserve(ObjectId::new(1), ObjectId::new(2), "page".into())
            .unwrap();
        registry.commit(token, "child".into()).unwrap();
        let event = CdpEvent::new("Runtime.event".into(), json!({}), Some("child".into()));

        // Act
        for _ in 0..=MAX_CDP_SESSION_EVENTS {
            registry.record(&event);
        }

        // Assert
        assert_eq!(
            registry.entries[&token].events.len(),
            MAX_CDP_SESSION_EVENTS
        );
        assert!(matches!(
            registry.drain(token, 8),
            Err(HostError::ResourceExhausted(_))
        ));
    }
}
