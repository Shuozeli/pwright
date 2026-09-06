use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use pwright_core::{ObjectId, ObjectKindTag};

use crate::{CallIdSource, Client, RemoteInitializer};

pub const DEFAULT_LIVE_OBJECT_LIMIT: usize = 1_024;
pub const DEFAULT_TERMINAL_REASON_BYTES: usize = 1_024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClientLimits {
    live_objects: usize,
    terminal_reason_bytes: usize,
}

impl ClientLimits {
    pub const fn new(live_objects: usize) -> Self {
        Self {
            live_objects,
            terminal_reason_bytes: DEFAULT_TERMINAL_REASON_BYTES,
        }
    }

    pub const fn live_objects(self) -> usize {
        self.live_objects
    }

    pub const fn with_terminal_reason_bytes(mut self, terminal_reason_bytes: usize) -> Self {
        self.terminal_reason_bytes = terminal_reason_bytes;
        self
    }

    pub const fn terminal_reason_bytes(self) -> usize {
        self.terminal_reason_bytes
    }
}

impl Default for ClientLimits {
    fn default() -> Self {
        Self::new(DEFAULT_LIVE_OBJECT_LIMIT)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClientObjectMetrics {
    current: usize,
    peak: usize,
    limit: usize,
}

impl ClientObjectMetrics {
    pub const fn current(self) -> usize {
        self.current
    }

    pub const fn peak(self) -> usize {
        self.peak
    }

    pub const fn limit(self) -> usize {
        self.limit
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientStateError {
    DuplicateObject {
        object_id: ObjectId,
    },
    MissingAnnouncement {
        object_id: ObjectId,
    },
    InitializerMismatch {
        object_id: ObjectId,
        expected_parent: Option<ObjectId>,
        actual_parent: Option<ObjectId>,
        expected_kind: ObjectKindTag,
        actual_kind: ObjectKindTag,
    },
    AnnouncementDisposed {
        object_id: ObjectId,
    },
    CapacityExceeded {
        limit: usize,
    },
    StateUnavailable,
}

impl Display for ClientStateError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateObject { object_id } => {
                write!(formatter, "client object `{object_id}` already exists")
            }
            Self::MissingAnnouncement { object_id } => {
                write!(formatter, "client object `{object_id}` was not announced")
            }
            Self::InitializerMismatch {
                object_id,
                expected_parent,
                actual_parent,
                expected_kind,
                actual_kind,
            } => write!(
                formatter,
                "client object `{object_id}` initializer expected parent {expected_parent:?} and kind `{expected_kind}`, received parent {actual_parent:?} and kind `{actual_kind}`"
            ),
            Self::AnnouncementDisposed { object_id } => write!(
                formatter,
                "client object `{object_id}` was disposed before its initializer was claimed"
            ),
            Self::CapacityExceeded { limit } => {
                write!(formatter, "client live-object limit of {limit} was reached")
            }
            Self::StateUnavailable => write!(formatter, "client object state is unavailable"),
        }
    }
}

impl Error for ClientStateError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientTerminalReasonError {
    TooLong { actual: usize, limit: usize },
    StateUnavailable,
}

impl Display for ClientTerminalReasonError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong { actual, limit } => write!(
                formatter,
                "Browser terminal reason has {actual} bytes, exceeding client limit {limit}"
            ),
            Self::StateUnavailable => {
                formatter.write_str("client terminal reason state is unavailable")
            }
        }
    }
}

impl Error for ClientTerminalReasonError {}

#[derive(Debug)]
struct ClientTerminalState {
    reason_bytes: usize,
    reason: Mutex<Option<Arc<str>>>,
}

impl ClientTerminalState {
    fn new(reason_bytes: usize) -> Self {
        Self {
            reason_bytes,
            reason: Mutex::new(None),
        }
    }

    fn install(&self, reason: &str) -> Result<bool, ClientTerminalReasonError> {
        if reason.is_empty() {
            return Ok(false);
        }
        if reason.len() > self.reason_bytes {
            return Err(ClientTerminalReasonError::TooLong {
                actual: reason.len(),
                limit: self.reason_bytes,
            });
        }
        let mut terminal = self
            .reason
            .lock()
            .map_err(|_| ClientTerminalReasonError::StateUnavailable)?;
        if terminal.is_some() {
            return Ok(false);
        }
        *terminal = Some(Arc::from(reason));
        Ok(true)
    }

    fn reason(&self) -> Result<Option<Arc<str>>, ClientTerminalReasonError> {
        self.reason
            .lock()
            .map(|reason| reason.clone())
            .map_err(|_| ClientTerminalReasonError::StateUnavailable)
    }
}

#[derive(Debug)]
pub(crate) struct ObjectState {
    live: AtomicBool,
    terminal: Arc<ClientTerminalState>,
}

impl ObjectState {
    fn new(terminal: Arc<ClientTerminalState>) -> Self {
        Self {
            live: AtomicBool::new(true),
            terminal,
        }
    }

    pub(crate) fn is_live(&self) -> bool {
        self.live.load(Ordering::Acquire)
    }

    fn invalidate(&self) {
        self.live.store(false, Ordering::Release);
    }

    pub(crate) fn install_terminal_reason(
        &self,
        reason: &str,
    ) -> Result<bool, ClientTerminalReasonError> {
        self.terminal.install(reason)
    }

    pub(crate) fn terminal_reason(&self) -> Result<Option<Arc<str>>, ClientTerminalReasonError> {
        self.terminal.reason()
    }
}

#[derive(Debug)]
struct ObjectTable {
    objects: BTreeMap<ObjectId, ClientObjectEntry>,
    peak: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ClientObjectPhase {
    Announced,
    Claimed,
    DisposedBeforeClaim,
}

#[derive(Debug)]
struct ClientObjectEntry {
    parent: Option<ObjectId>,
    kind: ObjectKindTag,
    phase: ClientObjectPhase,
    state: Arc<ObjectState>,
}

#[derive(Debug)]
pub(crate) struct ClientState {
    limit: usize,
    terminal_reason_bytes: usize,
    announcements_required: AtomicBool,
    table: Mutex<ObjectTable>,
}

impl ClientState {
    pub(crate) fn new(limits: ClientLimits) -> Self {
        Self {
            limit: limits.live_objects,
            terminal_reason_bytes: limits.terminal_reason_bytes,
            announcements_required: AtomicBool::new(false),
            table: Mutex::new(ObjectTable {
                objects: BTreeMap::new(),
                peak: 0,
            }),
        }
    }

    pub(crate) fn require_announced_initializers(&self) {
        self.announcements_required.store(true, Ordering::Release);
    }

    pub(crate) fn announcements_required(&self) -> bool {
        self.announcements_required.load(Ordering::Acquire)
    }

    pub(crate) fn object_parent(
        &self,
        object_id: ObjectId,
    ) -> Result<Option<ObjectId>, ClientStateError> {
        self.table
            .lock()
            .map_err(|_| ClientStateError::StateUnavailable)?
            .objects
            .get(&object_id)
            .map(|entry| entry.parent)
            .ok_or(ClientStateError::MissingAnnouncement { object_id })
    }

    fn insert(
        &self,
        object_id: ObjectId,
        parent: Option<ObjectId>,
        kind: ObjectKindTag,
        phase: ClientObjectPhase,
    ) -> Result<Arc<ObjectState>, ClientStateError> {
        let mut table = self
            .table
            .lock()
            .map_err(|_| ClientStateError::StateUnavailable)?;
        if table.objects.contains_key(&object_id) {
            return Err(ClientStateError::DuplicateObject { object_id });
        }
        if table.objects.len() >= self.limit {
            return Err(ClientStateError::CapacityExceeded { limit: self.limit });
        }
        let terminal = parent
            .and_then(|parent| table.objects.get(&parent))
            .map_or_else(
                || Arc::new(ClientTerminalState::new(self.terminal_reason_bytes)),
                |parent| Arc::clone(&parent.state.terminal),
            );
        let state = Arc::new(ObjectState::new(terminal));
        table.objects.insert(
            object_id,
            ClientObjectEntry {
                parent,
                kind,
                phase,
                state: Arc::clone(&state),
            },
        );
        table.peak = table.peak.max(table.objects.len());
        Ok(state)
    }

    pub(crate) fn announce(
        &self,
        object_id: ObjectId,
        parent: Option<ObjectId>,
        kind: ObjectKindTag,
    ) -> Result<(), ClientStateError> {
        self.insert(object_id, parent, kind, ClientObjectPhase::Announced)
            .map(drop)
    }

    pub(crate) fn claim(
        &self,
        object_id: ObjectId,
        parent: Option<ObjectId>,
        kind: ObjectKindTag,
    ) -> Result<Arc<ObjectState>, ClientStateError> {
        self.claim_inner(object_id, parent, kind, false)
    }

    pub(crate) fn claim_announced(
        &self,
        object_id: ObjectId,
        parent: Option<ObjectId>,
        kind: ObjectKindTag,
    ) -> Result<Arc<ObjectState>, ClientStateError> {
        self.claim_inner(object_id, parent, kind, true)
    }

    /// Claims a related initializer set as one client-side transaction.
    ///
    /// Validation is completed for every entry before any announcement moves
    /// to `Claimed` and before any direct-mode object is inserted.
    pub(crate) fn claim_result_batch(
        &self,
        initializers: &[RemoteInitializer],
    ) -> Result<Vec<Arc<ObjectState>>, ClientStateError> {
        let announcement_required = self.announcements_required();
        let mut table = self
            .table
            .lock()
            .map_err(|_| ClientStateError::StateUnavailable)?;

        for (index, initializer) in initializers.iter().enumerate() {
            if initializers[..index]
                .iter()
                .any(|earlier| earlier.id() == initializer.id())
            {
                return Err(ClientStateError::DuplicateObject {
                    object_id: initializer.id(),
                });
            }
            match table.objects.get(&initializer.id()) {
                None if announcement_required => {
                    return Err(ClientStateError::MissingAnnouncement {
                        object_id: initializer.id(),
                    });
                }
                None => {}
                Some(entry)
                    if entry.parent != initializer.parent() || entry.kind != initializer.kind() =>
                {
                    return Err(ClientStateError::InitializerMismatch {
                        object_id: initializer.id(),
                        expected_parent: entry.parent,
                        actual_parent: initializer.parent(),
                        expected_kind: entry.kind,
                        actual_kind: initializer.kind(),
                    });
                }
                Some(entry) => match entry.phase {
                    ClientObjectPhase::Announced => {}
                    ClientObjectPhase::Claimed => {
                        return Err(ClientStateError::DuplicateObject {
                            object_id: initializer.id(),
                        });
                    }
                    ClientObjectPhase::DisposedBeforeClaim => {
                        return Err(ClientStateError::AnnouncementDisposed {
                            object_id: initializer.id(),
                        });
                    }
                },
            }
        }

        let absent = initializers
            .iter()
            .filter(|initializer| !table.objects.contains_key(&initializer.id()))
            .count();
        if table.objects.len().saturating_add(absent) > self.limit {
            return Err(ClientStateError::CapacityExceeded { limit: self.limit });
        }

        let mut states = Vec::with_capacity(initializers.len());
        for initializer in initializers {
            if let Some(entry) = table.objects.get_mut(&initializer.id()) {
                debug_assert_eq!(entry.phase, ClientObjectPhase::Announced);
                entry.phase = ClientObjectPhase::Claimed;
                states.push(Arc::clone(&entry.state));
            } else {
                let terminal = initializer
                    .parent()
                    .and_then(|parent| table.objects.get(&parent))
                    .map_or_else(
                        || Arc::new(ClientTerminalState::new(self.terminal_reason_bytes)),
                        |parent| Arc::clone(&parent.state.terminal),
                    );
                let state = Arc::new(ObjectState::new(terminal));
                table.objects.insert(
                    initializer.id(),
                    ClientObjectEntry {
                        parent: initializer.parent(),
                        kind: initializer.kind(),
                        phase: ClientObjectPhase::Claimed,
                        state: Arc::clone(&state),
                    },
                );
                states.push(state);
            }
        }
        table.peak = table.peak.max(table.objects.len());
        Ok(states)
    }

    fn claim_inner(
        &self,
        object_id: ObjectId,
        parent: Option<ObjectId>,
        kind: ObjectKindTag,
        announcement_required: bool,
    ) -> Result<Arc<ObjectState>, ClientStateError> {
        let mut table = self
            .table
            .lock()
            .map_err(|_| ClientStateError::StateUnavailable)?;
        let inherited_terminal = parent
            .and_then(|parent| table.objects.get(&parent))
            .map(|parent| Arc::clone(&parent.state.terminal));
        let Some(entry) = table.objects.get_mut(&object_id) else {
            if announcement_required {
                return Err(ClientStateError::MissingAnnouncement { object_id });
            }
            if table.objects.len() >= self.limit {
                return Err(ClientStateError::CapacityExceeded { limit: self.limit });
            }
            let terminal = inherited_terminal
                .unwrap_or_else(|| Arc::new(ClientTerminalState::new(self.terminal_reason_bytes)));
            let state = Arc::new(ObjectState::new(terminal));
            table.objects.insert(
                object_id,
                ClientObjectEntry {
                    parent,
                    kind,
                    phase: ClientObjectPhase::Claimed,
                    state: Arc::clone(&state),
                },
            );
            table.peak = table.peak.max(table.objects.len());
            return Ok(state);
        };
        if entry.parent != parent || entry.kind != kind {
            return Err(ClientStateError::InitializerMismatch {
                object_id,
                expected_parent: entry.parent,
                actual_parent: parent,
                expected_kind: entry.kind,
                actual_kind: kind,
            });
        }
        match entry.phase {
            ClientObjectPhase::Announced => {
                entry.phase = ClientObjectPhase::Claimed;
                Ok(Arc::clone(&entry.state))
            }
            ClientObjectPhase::Claimed => Err(ClientStateError::DuplicateObject { object_id }),
            ClientObjectPhase::DisposedBeforeClaim => {
                table.objects.remove(&object_id);
                Err(ClientStateError::AnnouncementDisposed { object_id })
            }
        }
    }

    fn invalidate(&self, object_id: ObjectId) -> Result<bool, ClientStateError> {
        let mut table = self
            .table
            .lock()
            .map_err(|_| ClientStateError::StateUnavailable)?;
        let Some(entry) = table.objects.get_mut(&object_id) else {
            return Ok(false);
        };
        match entry.phase {
            ClientObjectPhase::Announced => {
                entry.state.invalidate();
                entry.phase = ClientObjectPhase::DisposedBeforeClaim;
                Ok(true)
            }
            ClientObjectPhase::Claimed => {
                let entry = table
                    .objects
                    .remove(&object_id)
                    .expect("claimed entry was found while holding the table lock");
                entry.state.invalidate();
                Ok(true)
            }
            ClientObjectPhase::DisposedBeforeClaim => Ok(false),
        }
    }

    fn metrics(&self) -> Result<ClientObjectMetrics, ClientStateError> {
        let table = self
            .table
            .lock()
            .map_err(|_| ClientStateError::StateUnavailable)?;
        Ok(ClientObjectMetrics {
            current: table.objects.len(),
            peak: table.peak,
            limit: self.limit,
        })
    }
}

impl<Channel, Ids> Client<Channel, Ids>
where
    Ids: CallIdSource,
{
    /// Enables the remote lifecycle contract for future command-result objects.
    ///
    /// Once enabled, an object initializer returned by a command must match an
    /// earlier lifecycle `Create` announcement. This mode is monotonic for the
    /// connection and is normally enabled by `pwright-client-control`.
    pub fn require_announced_initializers(&self) {
        self.state.require_announced_initializers();
    }

    pub fn announce(&self, initializer: RemoteInitializer) -> Result<(), ClientStateError> {
        self.state
            .announce(initializer.id(), initializer.parent(), initializer.kind())
    }

    pub fn invalidate(&self, object_id: ObjectId) -> Result<bool, ClientStateError> {
        self.state.invalidate(object_id)
    }

    pub fn invalidate_all(
        &self,
        object_ids_child_first: &[ObjectId],
    ) -> Result<usize, ClientStateError> {
        let mut invalidated = 0;
        for object_id in object_ids_child_first {
            invalidated += usize::from(self.state.invalidate(*object_id)?);
        }
        Ok(invalidated)
    }

    pub fn object_metrics(&self) -> Result<ClientObjectMetrics, ClientStateError> {
        self.state.metrics()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_core::ObjectId;

    use crate::{
        AtomicCallIdSource, BrowserContextKind, BrowserKind, Client, ClientLimits,
        HandleCreationError, PageKind, RemoteInitializer, RemoteKind,
    };

    fn initializer(
        id: u64,
        parent: Option<u64>,
        kind: pwright_core::ObjectKindTag,
    ) -> RemoteInitializer {
        RemoteInitializer::new(ObjectId::new(id), parent.map(ObjectId::new), kind)
    }

    #[test]
    fn invalidation_marks_all_handle_clones_stale_and_is_idempotent() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let page = client.page(ObjectId::new(7));
        let clone = page.clone();

        // Act
        let first = client.invalidate(ObjectId::new(7));
        let second = client.invalidate(ObjectId::new(7));

        // Assert
        assert_eq!(first, Ok(true));
        assert_eq!(second, Ok(false));
        assert!(!page.is_live());
        assert!(!clone.is_live());
        assert!(page.is_connected());
    }

    #[test]
    fn ordered_invalidation_applies_runtime_child_first_results() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let browser = client
            .browser(initializer(1, None, BrowserKind::OBJECT_KIND))
            .expect("browser should initialize");
        let context = browser
            .context(initializer(2, Some(1), BrowserContextKind::OBJECT_KIND))
            .expect("context should initialize");
        let page = context
            .page(initializer(3, Some(2), PageKind::OBJECT_KIND))
            .expect("page should initialize");

        // Act
        let invalidated =
            client.invalidate_all(&[ObjectId::new(3), ObjectId::new(2), ObjectId::new(1)]);

        // Assert
        assert_eq!(invalidated, Ok(3));
        assert!(!page.is_live());
        assert!(!context.is_live());
        assert!(!browser.is_live());
        assert_eq!(
            client.object_metrics().map(|metrics| metrics.current()),
            Ok(0)
        );
    }

    #[test]
    fn one_bounded_terminal_reason_is_shared_by_the_whole_object_family() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let browser = client
            .browser(initializer(1, None, BrowserKind::OBJECT_KIND))
            .unwrap();
        let context = browser
            .context(initializer(2, Some(1), BrowserContextKind::OBJECT_KIND))
            .unwrap();
        let page = context
            .page(initializer(3, Some(2), PageKind::OBJECT_KIND))
            .unwrap();

        // Act
        let installed = browser
            .as_remote()
            .install_terminal_reason("planned restart")
            .unwrap();
        let browser_reason = browser.as_remote().terminal_reason().unwrap().unwrap();
        let page_reason = page.as_remote().terminal_reason().unwrap().unwrap();

        // Assert
        assert!(installed);
        assert_eq!(browser_reason.as_ref(), "planned restart");
        assert!(Arc::ptr_eq(&browser_reason, &page_reason));
    }

    #[test]
    fn live_object_table_rejects_duplicates_and_enforces_capacity() {
        // Arrange
        let client = Client::with_limits((), AtomicCallIdSource::new(1), ClientLimits::new(1));
        let browser_initializer = initializer(1, None, BrowserKind::OBJECT_KIND);
        let browser = client
            .browser(browser_initializer)
            .expect("first object should fit");

        // Act
        let duplicate = client
            .browser(browser_initializer)
            .expect_err("duplicate live object should fail");
        let capacity = browser
            .context(initializer(2, Some(1), BrowserContextKind::OBJECT_KIND))
            .expect_err("second live object should exceed capacity");
        let metrics = client
            .object_metrics()
            .expect("metrics should remain available");

        // Assert
        assert_eq!(
            duplicate,
            HandleCreationError::DuplicateObject {
                object_id: ObjectId::new(1)
            }
        );
        assert_eq!(capacity, HandleCreationError::CapacityExceeded { limit: 1 });
        assert_eq!(metrics.current(), 1);
        assert_eq!(metrics.peak(), 1);
        assert_eq!(metrics.limit(), 1);
    }

    #[test]
    fn zero_capacity_rejects_the_first_remote_object() {
        // Arrange
        let client = Client::with_limits((), AtomicCallIdSource::new(1), ClientLimits::new(0));

        // Act
        let error = client
            .browser(initializer(1, None, BrowserKind::OBJECT_KIND))
            .expect_err("zero-capacity client must reject initialization");

        // Assert
        assert_eq!(error, HandleCreationError::CapacityExceeded { limit: 0 });
    }
}
