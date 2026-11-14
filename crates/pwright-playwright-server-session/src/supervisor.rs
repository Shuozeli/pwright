//! Optional, bounded Tokio owner for one validated remote Browser lease.
//!
//! A supervised peer's `receive` future must be cancellation-safe: selecting a
//! queued command cancels an in-flight `RemoteBrowserLease::receive_next`. The
//! opt-in live Tokio peer reads through `StreamExt::next`; injected peers must
//! uphold the same contract. The generic `WsPeer` trait alone does not promise
//! this property.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Debug, Formatter};
use std::io::{self, Write};
#[cfg(any(feature = "page-events", feature = "close-notices"))]
use std::sync::Mutex as StdMutex;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use pwright_playwright_server_client_core::{
    ClientError, CloseReason, CommandId, Ingress, MAX_GUID_BYTES, MAX_METHOD_BYTES, MAX_OBJECTS,
    MAX_OUTBOUND_BYTES, ProtocolViolation, ReplyOutcome,
};
use pwright_transport_ws::WsPeer;
use serde_json::Value;
#[cfg(any(feature = "page-events", feature = "close-notices"))]
use tokio::sync::Notify;
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore, mpsc, oneshot, watch};
use tokio::time::timeout;

use crate::{CloseOutcome, RemoteBrowserLease, SendError, SessionError, SessionFailure};

#[cfg(feature = "page-events")]
pub use pwright_playwright_server_page_events::Notice as PageNotice;
#[cfg(feature = "close-notices")]
pub use pwright_playwright_server_page_events::{CloseNotice, ObjectId};
#[cfg(any(feature = "page-events", feature = "close-notices"))]
use pwright_playwright_server_page_events::{
    PageEventProjection, ProjectionError as PageProjectionError,
};

pub const MAX_SUPERVISOR_COMMANDS: usize = 128;
pub const MAX_SUPERVISOR_EVENTS: usize = 128;
/// Approximate retained serialized event payload, not a total RSS bound.
/// Legitimate larger/streamed events require a separate future capability.
pub const MAX_SUPERVISOR_EVENT_BYTES: usize = 4 * 1024 * 1024;
/// Includes the synthetic Root and follows the protocol core's live-GUID cap.
pub const MAX_SUPERVISOR_OBJECTS: usize = MAX_OBJECTS;
/// Approximate retained serialized initializers plus GUID/type/parent envelopes.
/// Snapshot clones share initializer allocations; this is not a total RSS cap.
pub const MAX_SUPERVISOR_OBJECT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_SUPERVISOR_SEND_BUDGET_MS: u64 = 30_000;
const ROOT_PROJECTED_BYTES: usize = 128 + "Root".len() + 2;

/// Choose how validated, wire-ordered ingress is delivered. Every policy
/// maintains the bounded GUID graph; Page notices require an explicit feature.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EventPolicy {
    #[default]
    QueueBounded,
    DropUnsupportedEvents,
    /// Project only typed Page notices; unrelated ordinary events are not queued.
    #[cfg(feature = "page-events")]
    ProjectPageNotices,
    /// Project only Page, Context, Browser, and terminal close notices.
    #[cfg(feature = "close-notices")]
    ProjectCloseNotices,
}

/// A live remote protocol object. The initializer is shared across snapshots,
/// not reparsed by a language binding. It may contain secrets; Debug redacts it.
#[derive(Clone)]
pub struct RemoteObjectSnapshot {
    pub guid: String,
    pub parent: Option<String>,
    pub object_type: String,
    pub initializer: Arc<Value>,
    pub children: Vec<String>,
    pub created_sequence: u64,
    #[cfg(any(feature = "frame-title", feature = "close-notices"))]
    pub closed: bool,
}

impl Debug for RemoteObjectSnapshot {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RemoteObjectSnapshot")
            .field("guid", &self.guid)
            .field("parent", &self.parent)
            .field("object_type", &self.object_type)
            .field("initializer", &"<redacted>")
            .field("children", &self.children)
            .field("created_sequence", &self.created_sequence)
            .finish()
    }
}

/// An ordered point-in-time view. After terminal closure it retains the last
/// known graph for language-layer close notification; inspect `state()` before
/// treating any object as actionable.
#[derive(Clone)]
pub struct RemoteGraphSnapshot {
    pub objects: Vec<RemoteObjectSnapshot>,
    pub retained_bytes: usize,
    /// Monotonic even when the most recently created object was disposed.
    pub last_created_sequence: u64,
}

impl Debug for RemoteGraphSnapshot {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RemoteGraphSnapshot")
            .field("object_count", &self.objects.len())
            .field("retained_bytes", &self.retained_bytes)
            .field("last_created_sequence", &self.last_created_sequence)
            .finish()
    }
}

struct ProjectedObject {
    guid: String,
    parent: Option<String>,
    object_type: String,
    initializer: Arc<Value>,
    children: BTreeSet<String>,
    created_sequence: u64,
    #[cfg(feature = "frame-title")]
    adoption_epoch: u64,
    #[cfg(any(feature = "frame-title", feature = "close-notices"))]
    closed: bool,
    retained_bytes: usize,
}

impl ProjectedObject {
    fn snapshot(&self) -> RemoteObjectSnapshot {
        RemoteObjectSnapshot {
            guid: self.guid.clone(),
            parent: self.parent.clone(),
            object_type: self.object_type.clone(),
            initializer: Arc::clone(&self.initializer),
            children: self.children.iter().cloned().collect(),
            created_sequence: self.created_sequence,
            #[cfg(any(feature = "frame-title", feature = "close-notices"))]
            closed: self.closed,
        }
    }
}

#[derive(Debug)]
enum ProjectionError {
    Limit,
    Invalid(&'static str),
}

impl ProjectionError {
    fn terminal(self) -> (SessionError, CloseReason) {
        match self {
            Self::Limit => (SessionError::ObjectGraphLimit, CloseReason::ResourceLimit),
            Self::Invalid(reason) => (
                SessionError::ObjectGraphInvalid(reason),
                CloseReason::Protocol(ProtocolViolation::InvalidParams),
            ),
        }
    }
}

struct ObjectProjection {
    objects: BTreeMap<String, ProjectedObject>,
    retained_bytes: usize,
    next_sequence: u64,
}

impl ObjectProjection {
    fn new() -> Self {
        let root = ProjectedObject {
            guid: String::new(),
            parent: None,
            object_type: "Root".to_owned(),
            initializer: Arc::new(Value::Object(Default::default())),
            children: BTreeSet::new(),
            created_sequence: 0,
            #[cfg(feature = "frame-title")]
            adoption_epoch: 0,
            #[cfg(any(feature = "frame-title", feature = "close-notices"))]
            closed: false,
            retained_bytes: ROOT_PROJECTED_BYTES,
        };
        Self {
            objects: BTreeMap::from([(String::new(), root)]),
            retained_bytes: ROOT_PROJECTED_BYTES,
            next_sequence: 1,
        }
    }

    fn apply(&mut self, ingress: &Ingress) -> Result<(), ProjectionError> {
        match ingress {
            Ingress::Created {
                parent,
                guid,
                object_type,
                initializer,
            } => self.create(parent, guid, object_type, initializer),
            Ingress::Adopted { parent, guid } => self.adopt(parent, guid),
            Ingress::Disposed { guids, .. } => self.dispose(guids),
            #[cfg(any(feature = "frame-title", feature = "close-notices"))]
            Ingress::Event { guid, method, .. } if method == "close" => {
                if let Some(object) = self.objects.get_mut(guid)
                    && matches!(object.object_type.as_str(), "Page" | "BrowserContext")
                {
                    object.closed = true;
                }
                Ok(())
            }
            Ingress::Reply { .. } | Ingress::Event { .. } => Ok(()),
        }
    }

    fn create(
        &mut self,
        parent: &str,
        guid: &str,
        object_type: &str,
        initializer: &Value,
    ) -> Result<(), ProjectionError> {
        if !self.objects.contains_key(parent) || self.objects.contains_key(guid) {
            return Err(ProjectionError::Invalid(
                "create parent or GUID disagrees with core",
            ));
        }
        if self.objects.len() >= MAX_SUPERVISOR_OBJECTS {
            return Err(ProjectionError::Limit);
        }
        let retained_bytes = object_retained_bytes(parent, guid, object_type, initializer)?;
        let total = self
            .retained_bytes
            .checked_add(retained_bytes)
            .filter(|total| *total <= MAX_SUPERVISOR_OBJECT_BYTES)
            .ok_or(ProjectionError::Limit)?;
        let created_sequence = self.next_sequence;
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(ProjectionError::Limit)?;
        self.objects.insert(
            guid.to_owned(),
            ProjectedObject {
                guid: guid.to_owned(),
                parent: Some(parent.to_owned()),
                object_type: object_type.to_owned(),
                initializer: Arc::new(initializer.clone()),
                children: BTreeSet::new(),
                created_sequence,
                #[cfg(feature = "frame-title")]
                adoption_epoch: 0,
                #[cfg(any(feature = "frame-title", feature = "close-notices"))]
                closed: object_type == "Page"
                    && initializer.get("isClosed").and_then(Value::as_bool) == Some(true),
                retained_bytes,
            },
        );
        self.objects
            .get_mut(parent)
            .expect("parent checked before insert")
            .children
            .insert(guid.to_owned());
        self.retained_bytes = total;
        Ok(())
    }

    fn adopt(&mut self, parent: &str, guid: &str) -> Result<(), ProjectionError> {
        if !self.objects.contains_key(parent) {
            return Err(ProjectionError::Invalid(
                "adoption parent disagrees with core",
            ));
        }
        let child = self
            .objects
            .get(guid)
            .ok_or(ProjectionError::Invalid("adopted GUID disagrees with core"))?;
        let old_parent = child
            .parent
            .as_ref()
            .ok_or(ProjectionError::Invalid("Root cannot be adopted"))?
            .clone();
        if !self.objects.contains_key(&old_parent) {
            return Err(ProjectionError::Invalid("old adoption parent is missing"));
        }
        let new_bytes = child
            .retained_bytes
            .checked_sub(old_parent.len())
            .and_then(|bytes| bytes.checked_add(parent.len()))
            .ok_or(ProjectionError::Limit)?;
        let total = self
            .retained_bytes
            .checked_sub(child.retained_bytes)
            .and_then(|bytes| bytes.checked_add(new_bytes))
            .filter(|total| *total <= MAX_SUPERVISOR_OBJECT_BYTES)
            .ok_or(ProjectionError::Limit)?;
        #[cfg(feature = "frame-title")]
        let new_adoption_epoch = child
            .adoption_epoch
            .checked_add(1)
            .ok_or(ProjectionError::Limit)?;
        self.objects
            .get_mut(&old_parent)
            .expect("old parent checked")
            .children
            .remove(guid);
        self.objects
            .get_mut(parent)
            .expect("new parent checked")
            .children
            .insert(guid.to_owned());
        let child = self.objects.get_mut(guid).expect("child checked");
        child.parent = Some(parent.to_owned());
        #[cfg(feature = "frame-title")]
        {
            child.adoption_epoch = new_adoption_epoch;
        }
        child.retained_bytes = new_bytes;
        self.retained_bytes = total;
        Ok(())
    }

    fn dispose(&mut self, guids: &[String]) -> Result<(), ProjectionError> {
        let mut removed = BTreeSet::new();
        let mut removed_bytes = 0usize;
        for guid in guids {
            if guid.is_empty() || !removed.insert(guid) {
                return Err(ProjectionError::Invalid(
                    "disposal contains Root or duplicate GUID",
                ));
            }
            let object = self.objects.get(guid).ok_or(ProjectionError::Invalid(
                "disposed GUID disagrees with core",
            ))?;
            removed_bytes = removed_bytes.checked_add(object.retained_bytes).ok_or(
                ProjectionError::Invalid("disposal byte accounting overflow"),
            )?;
        }
        let total =
            self.retained_bytes
                .checked_sub(removed_bytes)
                .ok_or(ProjectionError::Invalid(
                    "disposal byte accounting underflow",
                ))?;
        for guid in guids {
            let object = self.objects.remove(guid).expect("disposal checked");
            if let Some(parent) = object.parent
                && let Some(parent) = self.objects.get_mut(&parent)
            {
                parent.children.remove(guid);
            }
        }
        self.retained_bytes = total;
        Ok(())
    }

    fn object(&self, guid: &str) -> Option<RemoteObjectSnapshot> {
        self.objects.get(guid).map(ProjectedObject::snapshot)
    }

    fn snapshot(&self) -> RemoteGraphSnapshot {
        let mut objects = self
            .objects
            .values()
            .map(ProjectedObject::snapshot)
            .collect::<Vec<_>>();
        objects.sort_by_key(|object| object.created_sequence);
        RemoteGraphSnapshot {
            objects,
            retained_bytes: self.retained_bytes,
            last_created_sequence: self.next_sequence - 1,
        }
    }

    fn last_created_sequence(&self) -> u64 {
        self.next_sequence - 1
    }

    #[cfg(feature = "frame-title")]
    fn matches_frame_title(&self, fence: &FrameTitleFence) -> bool {
        let [frame, page, context, browser] = &fence.lineage;
        for (expected, object_type, parent) in [
            (frame, "Frame", Some(page.guid.as_str())),
            (page, "Page", Some(context.guid.as_str())),
            (context, "BrowserContext", Some(browser.guid.as_str())),
            (browser, "Browser", None),
        ] {
            let Some(object) = self.objects.get(&expected.guid) else {
                return false;
            };
            if object.object_type != object_type
                || object.created_sequence != expected.created_sequence
                || parent.is_some_and(|parent| object.parent.as_deref() != Some(parent))
                || object.closed
            {
                return false;
            }
        }
        self.objects
            .get(&frame.guid)
            .is_some_and(|frame| frame.adoption_epoch == fence.frame_adoption_epoch)
    }
}

fn object_retained_bytes(
    parent: &str,
    guid: &str,
    object_type: &str,
    initializer: &Value,
) -> Result<usize, ProjectionError> {
    let overhead = 128usize
        .checked_add(parent.len())
        .and_then(|bytes| bytes.checked_add(guid.len()))
        .and_then(|bytes| bytes.checked_add(object_type.len()))
        .ok_or(ProjectionError::Limit)?;
    let mut writer = BoundedCounter {
        bytes: overhead,
        limit: MAX_SUPERVISOR_OBJECT_BYTES,
    };
    serde_json::to_writer(&mut writer, initializer).map_err(|_| ProjectionError::Limit)?;
    Ok(writer.bytes)
}

#[derive(Clone, Debug, PartialEq)]
pub enum SupervisorState {
    Connected,
    Closed {
        failure: Box<SessionFailure>,
        close_outcome: Option<Box<CloseOutcome>>,
    },
    /// The owner task exited without a reported protocol or transport terminal.
    Stopped,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SupervisorError {
    Client(ClientError),
    Terminal(SessionFailure),
    ActorStopped,
    EventStreamDisabled,
    TargetGenerationChanged,
}

/// A terminal-state observer that does not retain the socket owner. Keeping
/// this watcher alive cannot prevent a dropped supervisor from closing.
pub struct TerminalWatcher {
    state: watch::Receiver<SupervisorState>,
}

impl TerminalWatcher {
    pub fn state(&self) -> SupervisorState {
        self.state.borrow().clone()
    }

    pub async fn wait_terminal(&mut self) -> SupervisorState {
        loop {
            let current = self.state.borrow_and_update().clone();
            if !matches!(current, SupervisorState::Connected) {
                return current;
            }
            if self.state.changed().await.is_err() {
                let last = self.state.borrow().clone();
                return if matches!(last, SupervisorState::Connected) {
                    SupervisorState::Stopped
                } else {
                    last
                };
            }
        }
    }
}

struct CommandRequest {
    guid: String,
    expected_created_sequence: Option<u64>,
    #[cfg(feature = "frame-title")]
    frame_title_fence: Option<FrameTitleFence>,
    method: String,
    params: Value,
    metadata: Value,
    reply: oneshot::Sender<Result<ReplyOutcome, SupervisorError>>,
    _permit: OwnedSemaphorePermit,
}

struct PendingReply {
    reply: oneshot::Sender<Result<ReplyOutcome, SupervisorError>>,
    #[cfg(feature = "frame-title")]
    frame_title_fence: Option<FrameTitleFence>,
    _permit: OwnedSemaphorePermit,
}

#[cfg(feature = "frame-title")]
struct ExpectedFrameTitleObject {
    guid: String,
    created_sequence: u64,
}

#[cfg(feature = "frame-title")]
struct FrameTitleFence {
    // Frame, Page, Context, Browser. Each parent is the next entry.
    lineage: [ExpectedFrameTitleObject; 4],
    frame_adoption_epoch: u64,
}

struct QueuedIngress {
    ingress: Ingress,
    _bytes: OwnedSemaphorePermit,
}

#[cfg(any(feature = "page-events", feature = "close-notices"))]
struct PageEventDelivery {
    projection: StdMutex<PageEventProjection>,
    notify: Notify,
    reader: Mutex<()>,
}

#[cfg(any(feature = "page-events", feature = "close-notices"))]
impl PageEventDelivery {
    fn new(close_only: bool) -> Self {
        Self {
            projection: StdMutex::new(if close_only {
                PageEventProjection::new_close_only()
            } else {
                PageEventProjection::new()
            }),
            notify: Notify::new(),
            reader: Mutex::new(()),
        }
    }

    fn accept(&self, ingress: &Ingress) -> Result<(), PageProjectionError> {
        let mut projection = self
            .projection
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let before = projection.queued_notice_count();
        projection.accept(ingress)?;
        let published = projection.queued_notice_count() > before;
        drop(projection);
        if published {
            self.notify.notify_one();
        }
        Ok(())
    }

    fn finish(&self) -> Result<(), PageProjectionError> {
        let mut projection = self
            .projection
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match projection.finish() {
            Ok(()) => {
                drop(projection);
                self.notify.notify_one();
                Ok(())
            }
            // A prior projection violation already drives a terminal failure;
            // its queued prefix remains readable, but no new notice can be sent.
            Err(PageProjectionError::Failed) => Ok(()),
            Err(error) => Err(error),
        }
    }

    #[cfg(feature = "page-events")]
    fn next_notice(&self) -> Option<PageNotice> {
        self.projection
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .next_notice()
    }

    #[cfg(feature = "close-notices")]
    fn next_close_notice(&self) -> Option<CloseNotice> {
        self.projection
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .next_close_notice()
            .expect("close-notice policy owns a close-only projection")
    }
}

enum DeliveryFailure {
    QueueFull,
    #[cfg(any(feature = "page-events", feature = "close-notices"))]
    Projection(PageProjectionError),
}

impl DeliveryFailure {
    fn terminal(self) -> (SessionError, CloseReason) {
        match self {
            Self::QueueFull => (SessionError::EventQueueFull, CloseReason::ResourceLimit),
            #[cfg(any(feature = "page-events", feature = "close-notices"))]
            Self::Projection(error) => {
                let reason = match error {
                    PageProjectionError::Invalid(_)
                    | PageProjectionError::Finished
                    | PageProjectionError::Failed => {
                        CloseReason::Protocol(ProtocolViolation::InvalidParams)
                    }
                    PageProjectionError::ObjectLimit
                    | PageProjectionError::MetadataLimit
                    | PageProjectionError::NoticeLimit => CloseReason::ResourceLimit,
                };
                (SessionError::PageEventProjection(error), reason)
            }
        }
    }
}

struct EventDelivery {
    sender: mpsc::Sender<QueuedIngress>,
    bytes: Arc<Semaphore>,
    policy: EventPolicy,
    #[cfg(any(feature = "page-events", feature = "close-notices"))]
    page_events: Option<Arc<PageEventDelivery>>,
}

impl EventDelivery {
    fn publish(&self, ingress: Ingress) -> Result<(), DeliveryFailure> {
        match self.policy {
            EventPolicy::QueueBounded => queue_event(&self.sender, &self.bytes, ingress)
                .map_err(|()| DeliveryFailure::QueueFull),
            EventPolicy::DropUnsupportedEvents => Ok(()),
            #[cfg(feature = "page-events")]
            EventPolicy::ProjectPageNotices => self
                .page_events
                .as_ref()
                .expect("Page notice delivery is initialized for its policy")
                .accept(&ingress)
                .map_err(DeliveryFailure::Projection),
            #[cfg(feature = "close-notices")]
            EventPolicy::ProjectCloseNotices => self
                .page_events
                .as_ref()
                .expect("Close notice delivery is initialized for its policy")
                .accept(&ingress)
                .map_err(DeliveryFailure::Projection),
        }
    }

    fn finish(&self) -> Result<(), DeliveryFailure> {
        #[cfg(any(feature = "page-events", feature = "close-notices"))]
        if let Some(page_events) = &self.page_events {
            return page_events.finish().map_err(DeliveryFailure::Projection);
        }
        Ok(())
    }
}

/// A single Tokio task owns the socket and GUID graph; callers share only
/// bounded command admission and one ordered, single-consumer event stream.
/// No Node/Python callback or browser process is created here.
pub struct RemoteBrowserSupervisor {
    commands: mpsc::Sender<CommandRequest>,
    closes: mpsc::Sender<oneshot::Sender<CloseOutcome>>,
    events: Mutex<mpsc::Receiver<QueuedIngress>>,
    state: watch::Receiver<SupervisorState>,
    objects: Arc<RwLock<ObjectProjection>>,
    admissions: Arc<Semaphore>,
    guid: String,
    name: String,
    version: String,
    bootstrap_event_count: usize,
    event_policy: EventPolicy,
    #[cfg(any(feature = "page-events", feature = "close-notices"))]
    page_events: Option<Arc<PageEventDelivery>>,
    #[cfg(test)]
    actor_abort: tokio::task::AbortHandle,
}

impl RemoteBrowserSupervisor {
    pub async fn start<P: WsPeer + 'static>(
        lease: RemoteBrowserLease<P>,
    ) -> Result<Self, SessionFailure> {
        Self::start_with_event_policy(lease, EventPolicy::QueueBounded).await
    }

    /// Project ordered pre-reply lifecycle messages before exposing the handle.
    /// The queued policy fails closed on event backpressure, including during
    /// bootstrap replay. The drop policy retains no public event stream. The
    /// optional Page-notice policy projects inline and drops unrelated events.
    pub async fn start_with_event_policy<P: WsPeer + 'static>(
        mut lease: RemoteBrowserLease<P>,
        event_policy: EventPolicy,
    ) -> Result<Self, SessionFailure> {
        let (event_sender, event_receiver) = mpsc::channel(MAX_SUPERVISOR_EVENTS);
        let event_bytes = Arc::new(Semaphore::new(MAX_SUPERVISOR_EVENT_BYTES));
        #[cfg(any(feature = "page-events", feature = "close-notices"))]
        let page_events = match event_policy {
            #[cfg(feature = "page-events")]
            EventPolicy::ProjectPageNotices => Some(Arc::new(PageEventDelivery::new(false))),
            #[cfg(feature = "close-notices")]
            EventPolicy::ProjectCloseNotices => Some(Arc::new(PageEventDelivery::new(true))),
            EventPolicy::QueueBounded | EventPolicy::DropUnsupportedEvents => None,
        };
        let event_delivery = EventDelivery {
            sender: event_sender,
            bytes: event_bytes,
            policy: event_policy,
            #[cfg(any(feature = "page-events", feature = "close-notices"))]
            page_events: page_events.clone(),
        };
        let objects = Arc::new(RwLock::new(ObjectProjection::new()));
        let mut bootstrap_event_count = 0;
        while lease.bootstrap_pending_count() > 0 {
            let ingress = lease.receive_next().await?;
            if matches!(ingress, Ingress::Reply { .. }) {
                return Err(lease
                    .fail(
                        SessionError::Protocol(ProtocolViolation::InvalidReply),
                        CloseReason::Protocol(ProtocolViolation::InvalidReply),
                    )
                    .await);
            }
            let projection = {
                objects
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .apply(&ingress)
            };
            if let Err(error) = projection {
                let (error, reason) = error.terminal();
                return Err(lease.fail(error, reason).await);
            }
            if let Err(error) = event_delivery.publish(ingress) {
                let (error, reason) = error.terminal();
                return Err(lease.fail(error, reason).await);
            }
            if event_policy == EventPolicy::QueueBounded {
                bootstrap_event_count += 1;
            }
        }

        let (command_sender, command_receiver) = mpsc::channel(MAX_SUPERVISOR_COMMANDS);
        let (close_sender, close_receiver) = mpsc::channel(1);
        let (state_sender, state_receiver) = watch::channel(SupervisorState::Connected);
        let guid = lease.guid().to_owned();
        let name = lease.name().to_owned();
        let version = lease.version().to_owned();
        let actor = tokio::spawn(run_actor(
            lease,
            command_receiver,
            close_receiver,
            event_delivery,
            state_sender,
            Arc::clone(&objects),
        ));
        #[cfg(test)]
        let actor_abort = actor.abort_handle();
        drop(actor);
        Ok(Self {
            commands: command_sender,
            closes: close_sender,
            events: Mutex::new(event_receiver),
            state: state_receiver,
            objects,
            admissions: Arc::new(Semaphore::new(MAX_SUPERVISOR_COMMANDS)),
            guid,
            name,
            version,
            bootstrap_event_count,
            event_policy,
            #[cfg(any(feature = "page-events", feature = "close-notices"))]
            page_events,
            #[cfg(test)]
            actor_abort,
        })
    }

    pub fn guid(&self) -> &str {
        &self.guid
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    /// Query the last projected state without consuming ordered events. An
    /// object created before a command reply is visible when `call` returns.
    pub fn object(&self, guid: &str) -> Option<RemoteObjectSnapshot> {
        self.objects
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .object(guid)
    }

    /// Read `(snapshot, adoption_epoch, closed)` at one projected point in
    /// actor order without changing the existing public snapshot type.
    #[cfg(feature = "frame-title")]
    pub fn object_with_frame_title_state(
        &self,
        guid: &str,
    ) -> Option<(RemoteObjectSnapshot, u64, bool)> {
        self.objects
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .objects
            .get(guid)
            .map(|object| (object.snapshot(), object.adoption_epoch, object.closed))
    }

    /// Read the close tombstone with the same projected object generation.
    #[cfg(feature = "close-notices")]
    pub fn object_with_close_state(&self, guid: &str) -> Option<(RemoteObjectSnapshot, bool)> {
        self.objects
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .objects
            .get(guid)
            .map(|object| (object.snapshot(), object.closed))
    }

    /// Clone only bounded metadata and shared initializer references. Results
    /// are ordered by first creation, not lexicographic GUID order.
    pub fn snapshot(&self) -> RemoteGraphSnapshot {
        self.objects
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .snapshot()
    }

    /// Cheap creation fence for typed commands that require a new GUID.
    /// The actor still validates all reply references through the protocol
    /// core; this count alone is not a cryptographic causality proof.
    pub fn last_created_sequence(&self) -> u64 {
        self.objects
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .last_created_sequence()
    }

    /// Number of pre-`Root.initialize` lifecycle messages in the raw
    /// QueueBounded stream. Zero under the drop and Page-notice policies.
    /// Language bindings using QueueBounded can drain exactly this prefix.
    pub fn bootstrap_event_count(&self) -> usize {
        self.bootstrap_event_count
    }

    pub fn state(&self) -> SupervisorState {
        self.state.borrow().clone()
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.state(), SupervisorState::Connected)
    }

    pub fn terminal_watcher(&self) -> TerminalWatcher {
        TerminalWatcher {
            state: self.state.clone(),
        }
    }

    /// Wait for a connection terminal without consuming ordered ingress.
    /// Each caller owns an independent watch cursor, so concurrent language
    /// waiters cannot steal a state transition from one another.
    pub async fn wait_terminal(&self) -> SupervisorState {
        self.terminal_watcher().wait_terminal().await
    }

    /// Admit at most 128 queued-or-in-flight commands. Cancellation of a
    /// caller does not release its slot until the wire reply or connection
    /// terminal state has retired the command ID.
    pub async fn call(
        &self,
        guid: &str,
        method: &str,
        params: Value,
        metadata: Value,
    ) -> Result<ReplyOutcome, SupervisorError> {
        self.call_with_expected_generation(
            guid,
            None,
            method,
            params,
            metadata,
            #[cfg(feature = "frame-title")]
            None,
        )
        .await
    }

    /// Send only while the actor's projected target is the same generation
    /// observed by the caller. A rejected command allocates no command ID and
    /// writes no frame. This protects against GUID reuse already observed by
    /// the actor; it cannot establish the remote peer's unseen current state.
    pub async fn call_for_generation(
        &self,
        guid: &str,
        expected_created_sequence: u64,
        method: &str,
        params: Value,
        metadata: Value,
    ) -> Result<ReplyOutcome, SupervisorError> {
        self.call_with_expected_generation(
            guid,
            Some(expected_created_sequence),
            method,
            params,
            metadata,
            #[cfg(feature = "frame-title")]
            None,
        )
        .await
    }

    /// Fence `Frame.title` against the exact ordered Frame, Page, Context,
    /// and Browser lineage. The actor validates before writing and at the
    /// correlated reply; later lifecycle ingress cannot invalidate a reply.
    #[cfg(feature = "frame-title")]
    pub async fn call_for_frame_title(
        &self,
        lineage: [(&str, u64); 4],
        frame_adoption_epoch: u64,
        metadata: Value,
    ) -> Result<ReplyOutcome, SupervisorError> {
        if lineage.iter().any(|(guid, _)| {
            guid.is_empty() || guid.len() > MAX_GUID_BYTES || guid.chars().any(char::is_control)
        }) {
            return Err(SupervisorError::Client(ClientError::InvalidGuid));
        }
        let fence = FrameTitleFence {
            lineage: lineage.map(|(guid, created_sequence)| ExpectedFrameTitleObject {
                guid: guid.to_owned(),
                created_sequence,
            }),
            frame_adoption_epoch,
        };
        self.call_with_expected_generation(
            lineage[0].0,
            None,
            "title",
            Value::Object(Default::default()),
            metadata,
            Some(fence),
        )
        .await
    }

    async fn call_with_expected_generation(
        &self,
        guid: &str,
        expected_created_sequence: Option<u64>,
        method: &str,
        params: Value,
        metadata: Value,
        #[cfg(feature = "frame-title")] frame_title_fence: Option<FrameTitleFence>,
    ) -> Result<ReplyOutcome, SupervisorError> {
        if !self.is_connected() {
            return Err(self.terminal_error());
        }
        validate_command(guid, method, &params, &metadata).map_err(SupervisorError::Client)?;
        let permit = Arc::clone(&self.admissions)
            .try_acquire_owned()
            .map_err(|_| SupervisorError::Client(ClientError::PendingLimit))?;
        let (reply, result) = oneshot::channel();
        let command = CommandRequest {
            guid: guid.to_owned(),
            expected_created_sequence,
            #[cfg(feature = "frame-title")]
            frame_title_fence,
            method: method.to_owned(),
            params,
            metadata,
            reply,
            _permit: permit,
        };
        self.commands
            .try_send(command)
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => {
                    SupervisorError::Client(ClientError::PendingLimit)
                }
                mpsc::error::TrySendError::Closed(_) => self.terminal_error(),
            })?;
        result.await.unwrap_or_else(|_| Err(self.terminal_error()))
    }

    /// The raw QueueBounded stream preserves wire order among non-reply
    /// messages. Count and approximate serialized-byte permits are released
    /// when an event is taken. Other policies return EventStreamDisabled.
    pub async fn next_event(&self) -> Result<Ingress, SupervisorError> {
        if self.event_policy != EventPolicy::QueueBounded {
            return Err(SupervisorError::EventStreamDisabled);
        }
        self.events
            .lock()
            .await
            .recv()
            .await
            .map(|queued| queued.ingress)
            .ok_or_else(|| self.terminal_error())
    }

    /// Await one typed Page notice. Only available under the opt-in
    /// `ProjectPageNotices` policy; concurrent readers are serialized. Notices
    /// remain in wire order, and terminal child closes precede `Disconnected`.
    #[cfg(feature = "page-events")]
    pub async fn next_page_notice(&self) -> Result<PageNotice, SupervisorError> {
        if self.event_policy != EventPolicy::ProjectPageNotices {
            return Err(SupervisorError::EventStreamDisabled);
        }
        let page_events = self
            .page_events
            .as_ref()
            .ok_or(SupervisorError::EventStreamDisabled)?;
        let _reader = page_events.reader.lock().await;
        let mut state = self.state.clone();
        loop {
            let notified = page_events.notify.notified();
            if let Some(notice) = page_events.next_notice() {
                return Ok(notice);
            }
            if !matches!(*state.borrow_and_update(), SupervisorState::Connected) {
                return Err(self.terminal_error());
            }
            tokio::select! {
                () = notified => {}
                result = state.changed() => {
                    if result.is_err() {
                        return Err(self.terminal_error());
                    }
                }
            }
        }
    }

    /// Await one ordered Page, Context, Browser, or terminal close notice.
    /// The queued prefix remains readable after the connection terminates.
    #[cfg(feature = "close-notices")]
    pub async fn next_close_notice(&self) -> Result<CloseNotice, SupervisorError> {
        if self.event_policy != EventPolicy::ProjectCloseNotices {
            return Err(SupervisorError::EventStreamDisabled);
        }
        let page_events = self
            .page_events
            .as_ref()
            .expect("Close notice delivery is initialized for its policy");
        let _reader = page_events.reader.lock().await;
        let mut state = self.state.clone();
        loop {
            let notified = page_events.notify.notified();
            if let Some(notice) = page_events.next_close_notice() {
                return Ok(notice);
            }
            if !matches!(*state.borrow_and_update(), SupervisorState::Connected) {
                return Err(self.terminal_error());
            }
            tokio::select! {
                () = notified => {}
                result = state.changed() => {
                    if result.is_err() {
                        return Err(self.terminal_error());
                    }
                }
            }
        }
    }

    /// Closing disconnects this client WebSocket, not the server-owned browser.
    /// Repeated calls, including after a remote close, are harmless.
    pub async fn close(&self) -> Result<CloseOutcome, SupervisorError> {
        if let Some(outcome) = self.closed_outcome() {
            return Ok(outcome);
        }
        let (reply, result) = oneshot::channel();
        if self.closes.send(reply).await.is_err() {
            return self.closed_outcome().ok_or(SupervisorError::ActorStopped);
        }
        result
            .await
            .or_else(|_| self.closed_outcome().ok_or(SupervisorError::ActorStopped))
    }

    fn terminal_error(&self) -> SupervisorError {
        match self.state() {
            SupervisorState::Connected => SupervisorError::ActorStopped,
            SupervisorState::Closed { failure, .. } => SupervisorError::Terminal(*failure),
            SupervisorState::Stopped => SupervisorError::ActorStopped,
        }
    }

    fn closed_outcome(&self) -> Option<CloseOutcome> {
        match self.state() {
            SupervisorState::Connected => None,
            SupervisorState::Closed {
                failure,
                close_outcome,
            } => Some(close_outcome.map_or_else(
                || CloseOutcome {
                    report: *failure.report,
                    transport_error: None,
                },
                |outcome| *outcome,
            )),
            SupervisorState::Stopped => None,
        }
    }
}

struct BoundedCounter {
    bytes: usize,
    limit: usize,
}

impl Write for BoundedCounter {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        if input.len() > self.limit.saturating_sub(self.bytes) {
            return Err(io::Error::other("remote command exceeds outbound limit"));
        }
        self.bytes += input.len();
        Ok(input.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn validate_command(
    guid: &str,
    method: &str,
    params: &Value,
    metadata: &Value,
) -> Result<(), ClientError> {
    if guid.len() > MAX_GUID_BYTES || guid.chars().any(char::is_control) {
        return Err(ClientError::InvalidGuid);
    }
    if method.is_empty() || method.len() > MAX_METHOD_BYTES || method.chars().any(char::is_control)
    {
        return Err(ClientError::InvalidMethod);
    }
    if !params.is_object() {
        return Err(ClientError::InvalidParams);
    }
    if !metadata.is_object() {
        return Err(ClientError::InvalidMetadata);
    }
    let mut writer = BoundedCounter {
        bytes: guid.len() + method.len() + 64,
        limit: MAX_OUTBOUND_BYTES,
    };
    serde_json::to_writer(&mut writer, params).map_err(|_| ClientError::OutboundTooLarge)?;
    serde_json::to_writer(&mut writer, metadata).map_err(|_| ClientError::OutboundTooLarge)?;
    Ok(())
}

fn event_retained_bytes(ingress: &Ingress) -> Result<u32, ()> {
    // Count the retained field content plus conservative envelope overhead.
    // This avoids cloning a possibly large JSON payload merely to meter it;
    // serde_json's in-memory representation may still exceed this estimate.
    let mut writer = BoundedCounter {
        bytes: 128,
        limit: MAX_SUPERVISOR_EVENT_BYTES,
    };
    let result = match ingress {
        Ingress::Created {
            parent,
            guid,
            object_type,
            initializer,
        } => serde_json::to_writer(&mut writer, &(parent, guid, object_type, initializer)),
        Ingress::Adopted { parent, guid } => serde_json::to_writer(&mut writer, &(parent, guid)),
        Ingress::Disposed { guids, reason } => serde_json::to_writer(&mut writer, &(guids, reason)),
        Ingress::Event {
            guid,
            method,
            params,
        } => serde_json::to_writer(&mut writer, &(guid, method, params)),
        Ingress::Reply { .. } => return Err(()),
    };
    result.map_err(|_| ())?;
    u32::try_from(writer.bytes).map_err(|_| ())
}

fn queue_event(
    events: &mpsc::Sender<QueuedIngress>,
    event_bytes: &Arc<Semaphore>,
    ingress: Ingress,
) -> Result<(), ()> {
    let bytes = event_retained_bytes(&ingress)?;
    let permit = Arc::clone(event_bytes)
        .try_acquire_many_owned(bytes)
        .map_err(|_| ())?;
    events
        .try_send(QueuedIngress {
            ingress,
            _bytes: permit,
        })
        .map_err(|_| ())
}

fn publish_terminal(
    pending: &mut BTreeMap<CommandId, PendingReply>,
    state: &watch::Sender<SupervisorState>,
    event_delivery: &EventDelivery,
    mut failure: SessionFailure,
    close_outcome: Option<CloseOutcome>,
) -> SessionFailure {
    if let Err(error) = event_delivery.finish() {
        let (projection_error, reason) = error.terminal();
        failure.error = projection_error;
        failure.report.reason = reason;
    }
    // A command's terminal error must never become observable before the
    // Browser's terminal state. Bindings can then project disconnect events
    // synchronously while handling that error, without racing the watcher.
    state.send_replace(SupervisorState::Closed {
        failure: Box::new(failure.clone()),
        close_outcome: close_outcome.map(Box::new),
    });
    for (_, waiter) in std::mem::take(pending) {
        let _ = waiter
            .reply
            .send(Err(SupervisorError::Terminal(failure.clone())));
    }
    failure
}

/// Cancellation and panic cannot run an async Close handshake. Still, they
/// must never leave a surviving handle claiming that its owner is connected.
struct ActorExitGuard {
    state: watch::Sender<SupervisorState>,
    #[cfg(any(feature = "page-events", feature = "close-notices"))]
    page_events: Option<Arc<PageEventDelivery>>,
}

impl Drop for ActorExitGuard {
    fn drop(&mut self) {
        let still_connected = matches!(*self.state.borrow(), SupervisorState::Connected);
        if still_connected {
            #[cfg(any(feature = "page-events", feature = "close-notices"))]
            if let Some(page_events) = &self.page_events {
                let _ = page_events.finish();
            }
            self.state.send_replace(SupervisorState::Stopped);
        }
    }
}

async fn run_actor<P: WsPeer>(
    mut lease: RemoteBrowserLease<P>,
    mut commands: mpsc::Receiver<CommandRequest>,
    mut closes: mpsc::Receiver<oneshot::Sender<CloseOutcome>>,
    event_delivery: EventDelivery,
    state: watch::Sender<SupervisorState>,
    objects: Arc<RwLock<ObjectProjection>>,
) {
    let mut pending = BTreeMap::<CommandId, PendingReply>::new();
    let _exit_guard = ActorExitGuard {
        state: state.clone(),
        #[cfg(any(feature = "page-events", feature = "close-notices"))]
        page_events: event_delivery.page_events.clone(),
    };
    loop {
        tokio::select! {
            close_request = closes.recv() => {
                let outcome = lease.close().await;
                let failure = SessionFailure {
                    error: SessionError::BrowserClosed,
                    report: Box::new(outcome.report.clone()),
                };
                publish_terminal(&mut pending, &state, &event_delivery, failure, Some(outcome.clone()));
                if let Some(reply) = close_request {
                    let _ = reply.send(outcome);
                }
                return;
            }
            command = commands.recv() => {
                let Some(command) = command else {
                    let outcome = lease.close().await;
                    let failure = SessionFailure {
                        error: SessionError::BrowserClosed,
                        report: Box::new(outcome.report.clone()),
                    };
                    publish_terminal(&mut pending, &state, &event_delivery, failure, Some(outcome));
                    return;
                };
                if command.reply.is_closed() {
                    continue;
                }
                let matches = {
                    let projection = objects
                        .read()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    let generation_matches = command.expected_created_sequence.is_none_or(|expected| {
                        projection.objects.get(&command.guid)
                            .is_some_and(|object| object.created_sequence == expected)
                    });
                    #[cfg(feature = "frame-title")]
                    let generation_matches = generation_matches
                        && command.frame_title_fence.as_ref()
                            .is_none_or(|fence| projection.matches_frame_title(fence));
                    generation_matches
                };
                if !matches {
                    let _ = command.reply.send(Err(SupervisorError::TargetGenerationChanged));
                    continue;
                }
                let sent = timeout(
                    Duration::from_millis(MAX_SUPERVISOR_SEND_BUDGET_MS),
                    lease.send_command(
                        &command.guid,
                        &command.method,
                        &command.params,
                        &command.metadata,
                    ),
                )
                .await;
                match sent {
                    Ok(Ok(id)) => {
                        pending.insert(id, PendingReply {
                            reply: command.reply,
                            #[cfg(feature = "frame-title")]
                            frame_title_fence: command.frame_title_fence,
                            _permit: command._permit,
                        });
                    }
                    Ok(Err(SendError::Client(error))) => {
                        let _ = command.reply.send(Err(SupervisorError::Client(error)));
                    }
                    Ok(Err(SendError::Terminal(failure))) => {
                        let failure = publish_terminal(
                            &mut pending, &state, &event_delivery, failure, None,
                        );
                        let _ = command.reply.send(Err(SupervisorError::Terminal(failure)));
                        return;
                    }
                    Err(_) => {
                        let failure = lease
                            .fail(SessionError::SendDeadlineExceeded, CloseReason::Transport)
                            .await;
                        let failure = publish_terminal(
                            &mut pending, &state, &event_delivery, failure, None,
                        );
                        let _ = command.reply.send(Err(SupervisorError::Terminal(failure)));
                        return;
                    }
                }
            }
            incoming = lease.receive_next() => {
                match incoming {
                    Ok(Ingress::Reply { pending: replied, outcome }) => {
                        let Some(waiter) = pending.remove(&replied.id) else {
                            let violation = ProtocolViolation::UnknownResponse(replied.id);
                            let failure = lease
                                .fail(SessionError::Protocol(violation.clone()), CloseReason::Protocol(violation))
                                .await;
                            publish_terminal(&mut pending, &state, &event_delivery, failure, None);
                            return;
                        };
                        #[cfg(feature = "frame-title")]
                        if waiter.frame_title_fence.as_ref().is_some_and(|fence| {
                            !objects
                                .read()
                                .unwrap_or_else(|poisoned| poisoned.into_inner())
                                .matches_frame_title(fence)
                        }) {
                            let _ = waiter.reply.send(Err(SupervisorError::TargetGenerationChanged));
                            continue;
                        }
                        let _ = waiter.reply.send(Ok(outcome));
                    }
                    Ok(event) => {
                        let projection = {
                            objects
                                .write()
                                .unwrap_or_else(|poisoned| poisoned.into_inner())
                                .apply(&event)
                        };
                        if let Err(error) = projection {
                            let (error, reason) = error.terminal();
                            let failure = lease.fail(error, reason).await;
                            publish_terminal(&mut pending, &state, &event_delivery, failure, None);
                            return;
                        }
                        if let Err(error) = event_delivery.publish(event) {
                            let (error, reason) = error.terminal();
                            let failure = lease.fail(error, reason).await;
                            publish_terminal(&mut pending, &state, &event_delivery, failure, None);
                            return;
                        }
                    }
                    Err(failure) => {
                        publish_terminal(&mut pending, &state, &event_delivery, failure, None);
                        return;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
