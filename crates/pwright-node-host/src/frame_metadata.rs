//! Bounded native metadata, independent of browser/transport lifetime.

use std::collections::{BTreeMap, VecDeque};
use std::error::Error;
use std::sync::{Arc, Mutex};

use pwright_composition_chromium::ChromiumFrameUrlObserver;
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceLedger,
    ResourceLimits, ResourceRequirements, ResourceScope,
};

use crate::HostError;
use crate::frame_tree::{
    FrameClosePublication, FrameNode, FrameTree, FrameTreePublication, Storage,
};
use tokio::sync::Notify;

const SCOPE: ResourceScope = ResourceScope::extension(121, 1);

mod close;
mod collections;
mod drain;
mod lifecycle;
mod names;
pub(crate) use close::FrameClosePermit;
pub use collections::{
    CollectionBatch, CollectionContext, CollectionMember, CollectionMembers, CollectionPage,
    CollectionRebaseError, CollectionReservation, CollectionRevision, CollectionView,
    CollectionVisibilityError, VisibleMembers,
};
pub use drain::{FramePublicationDrain, PublicationDrainError, PublicationProgress};
pub(crate) use lifecycle::CreationDelivery;
pub(crate) use lifecycle::TerminalLifecycle;
pub use lifecycle::{
    LifecycleCause, LifecycleDelivery, LifecycleNotice, LifecycleNoticeView, LifecycleReservation,
};
pub(crate) use names::FrameNameLimits;
use names::{BudgetedName, NameResources};

#[derive(Clone, Copy)]
pub(crate) struct FrameMetadataLimits {
    pub entries: usize,
    pub url_bytes: usize,
    pub total_bytes: u64,
    pub publications: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameMetadataMetrics {
    pub entries: u32,
    pub url_bytes: u32,
    pub name_bytes: u32,
    pub active_frames: u32,
    pub pending_publications: u32,
}

struct BudgetedUrl {
    value: Box<str>,
    name: Option<Arc<BudgetedName>>,
    _bytes: Option<ResourceLease>,
}

struct FrameUrlState {
    url: Mutex<UrlViews>,
    names_enabled: bool,
    _entry: ResourceLease,
}

struct UrlViews {
    native: Arc<BudgetedUrl>,
    published: Arc<BudgetedUrl>,
}

/// A retained Frame owns URL views and metadata leases, not a session,
/// CDP route, runtime or browser. Page and main Frame share this exact object.
#[derive(Clone)]
pub struct FrameUrlSnapshot(Arc<FrameUrlState>);

impl FrameUrlSnapshot {
    pub fn url(&self) -> Result<String, HostError> {
        let state = self.0.url.lock().map_err(HostError::backend)?;
        Ok(state.native.value.to_string())
    }

    /// Only the foreign event-loop publication boundary advances this view.
    pub fn published_url(&self) -> Result<String, HostError> {
        let state = self.0.url.lock().map_err(HostError::backend)?;
        Ok(state.published.value.to_string())
    }

    pub fn with_published_url<T>(&self, read: impl FnOnce(&str) -> T) -> Result<T, HostError> {
        let value = self
            .0
            .url
            .lock()
            .map_err(HostError::backend)?
            .published
            .clone();
        Ok(read(&value.value))
    }

    /// Borrow the published name through conversion, without a second Rust
    /// string allocation or a callback under the metadata lock.
    pub fn with_published_name<T>(&self, read: impl FnOnce(&str) -> T) -> Result<T, HostError> {
        if !self.0.names_enabled {
            return Err(HostError::UnsupportedOperation(
                "Frame names were not selected".into(),
            ));
        }
        let value = self
            .0
            .url
            .lock()
            .map_err(HostError::backend)?
            .published
            .clone();
        Ok(read(names::value(&value.name)))
    }

    pub fn has_unpublished_url(&self) -> Result<bool, HostError> {
        let state = self.0.url.lock().map_err(HostError::backend)?;
        Ok(!Arc::ptr_eq(&state.native, &state.published))
    }
}

enum FrameChangeValue {
    Url {
        target: FrameUrlSnapshot,
        value: Arc<BudgetedUrl>,
    },
    Tree(FrameTreePublication),
    Close(FrameClosePublication),
    Collection(CollectionView),
    Lifecycle(Box<lifecycle::LifecycleGroup>),
}

struct FrameChange {
    sequence: u64,
    value: FrameChangeValue,
    _event: ResourceLease,
}

struct MetadataState {
    active: BTreeMap<ObjectId, FrameUrlSnapshot>,
    pending: VecDeque<FrameChange>,
    sequence: u64,
    discarded: bool,
    connection: ConnectionPublication,
    collections: Option<CollectionView>,
    collection_batch: Option<u64>,
    // The selected Page owner admits the complete queue before allocation.
    // Keep that lease through the final publication/discard endpoint lifetime.
    queue_storage: Option<Arc<Storage>>,
    drain: drain::DrainOwnership,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConnectionPublication {
    Open,
    DisconnectReady,
    Disconnected,
}

impl MetadataState {
    fn publish_one_through(&mut self, sequence: u64) -> Result<bool, HostError> {
        if !self
            .pending
            .front()
            .is_some_and(|change| change.sequence <= sequence)
        {
            return Ok(false);
        }
        if matches!(
            self.pending.front().map(|change| &change.value),
            Some(FrameChangeValue::Lifecycle(_))
        ) {
            return Err(HostError::UnsupportedOperation(
                "lifecycle publication requires notice conversion and acknowledgement".into(),
            ));
        }
        let change = self.pending.pop_front().expect("checked pending front");
        match change.value {
            FrameChangeValue::Url { target, value } => {
                target.0.url.lock().map_err(HostError::backend)?.published = value;
            }
            FrameChangeValue::Tree(publication) => publication.publish()?,
            FrameChangeValue::Close(publication) => publication.publish()?,
            FrameChangeValue::Collection(view) => {
                self.collections = Some(view);
            }
            FrameChangeValue::Lifecycle(_) => unreachable!("checked before consuming queue entry"),
        }
        Ok(true)
    }

    fn complete_publication(&mut self, sequence: u64) {
        if sequence == u64::MAX && !self.discarded && self.connection == ConnectionPublication::Open
        {
            self.collections = None;
            self.connection = ConnectionPublication::DisconnectReady;
        }
    }

    fn publishable_sequence(&self) -> u64 {
        self.collection_batch.unwrap_or(self.sequence)
    }

    fn next_sequence(&self) -> Result<u64, HostError> {
        // MAX is reserved for the single, infallible session-teardown batch.
        self.sequence
            .checked_add(1)
            .filter(|value| *value < u64::MAX)
            .ok_or_else(|| {
                HostError::ResourceExhausted("metadata publication sequence exhausted".into())
            })
    }
}

pub(crate) struct FrameMetadata {
    state: Mutex<MetadataState>,
    resources: ResourceLedger,
    limits: FrameMetadataLimits,
    names: Option<NameResources>,
    changed: Notify,
}

/// A foreign-runtime publication endpoint, with no transport or runtime owner.
#[derive(Clone)]
pub struct FramePublications(Arc<FrameMetadata>);

/// A fixed watermark, not a snapshot of whatever happens to be latest on delivery.
pub struct FramePublicationFence {
    book: FramePublications,
    sequence: u64,
}

impl FramePublicationFence {
    pub fn publish(self) -> Result<(), HostError> {
        let mut state = self.book.0.state.lock().map_err(HostError::backend)?;
        if state.drain.active.is_some() {
            return Err(HostError::UnsupportedOperation(
                "metadata publication has an exclusive drain owner".into(),
            ));
        }
        while state.publish_one_through(self.sequence)? {}
        state.complete_publication(self.sequence);
        Ok(())
    }
}

impl FramePublications {
    /// Public connection state changes at event dispatch, not when background
    /// shutdown starts or an unrelated completion publishes terminal Page data.
    pub fn connection_is_published_open(&self) -> Result<bool, HostError> {
        let state = self.0.state.lock().map_err(HostError::backend)?;
        Ok(state.connection != ConnectionPublication::Disconnected)
    }

    /// The language event port calls this immediately before emitting its
    /// disconnected event. Rust validates readiness and suppresses duplicates.
    pub fn publish_disconnect(&self) -> Result<bool, HostError> {
        let mut state = self.0.state.lock().map_err(HostError::backend)?;
        if state.discarded {
            return Err(HostError::TargetClosed);
        }
        match state.connection {
            ConnectionPublication::Open => Err(HostError::Backend(
                "Browser disconnect publication precedes its terminal metadata fence".into(),
            )),
            ConnectionPublication::DisconnectReady => {
                state.connection = ConnectionPublication::Disconnected;
                Ok(true)
            }
            ConnectionPublication::Disconnected => Ok(false),
        }
    }

    /// Admit and stage a topology revision on the same watermark as URL changes.
    /// No topology is installed or polled by this endpoint: the lifecycle owner
    /// must explicitly supply authoritative, budgeted nodes. Queue admission
    /// precedes native-tree mutation, so overflow leaves both views unchanged.
    pub fn stage_tree(&self, tree: &FrameTree, frames: &[FrameNode]) -> Result<(), HostError> {
        let mut state = self.0.state.lock().map_err(HostError::backend)?;
        if state.discarded {
            return Err(HostError::TargetClosed);
        }
        let sequence = state.next_sequence()?;
        let event = self
            .0
            .resources
            .reserve(SCOPE, ResourceClass::EventItem, 1)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let publication = tree.stage(frames)?;
        state.pending.push_back(FrameChange {
            sequence,
            value: FrameChangeValue::Tree(publication),
            _event: event,
        });
        state.sequence = sequence;
        self.0.changed.notify_one();
        Ok(())
    }

    pub fn fence(&self) -> Result<FramePublicationFence, HostError> {
        let sequence = self
            .0
            .state
            .lock()
            .map_err(HostError::backend)?
            .publishable_sequence();
        Ok(FramePublicationFence {
            book: self.clone(),
            sequence,
        })
    }

    pub async fn next(&self) -> Result<FramePublicationFence, HostError> {
        loop {
            let changed = self.0.changed.notified();
            {
                let state = self.0.state.lock().map_err(HostError::backend)?;
                let sequence = state.publishable_sequence();
                if state
                    .pending
                    .front()
                    .is_some_and(|change| change.sequence <= sequence)
                    || state.discarded
                {
                    return Ok(FramePublicationFence {
                        book: self.clone(),
                        sequence,
                    });
                }
            }
            changed.await;
        }
    }

    pub fn is_drained(&self) -> Result<bool, HostError> {
        Ok(self
            .0
            .state
            .lock()
            .map_err(HostError::backend)?
            .pending
            .is_empty())
    }

    /// Environment teardown abandons publication without executing foreign code.
    pub fn discard(&self) {
        let mut state = self
            .0
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.discarded = true;
        state.drain.active = None;
        state.collection_batch = None;
        state.pending.clear();
        state.collections = None;
        self.0.changed.notify_one();
    }
}

impl FrameMetadata {
    pub fn new(limits: FrameMetadataLimits) -> Result<Self, HostError> {
        let entries = u64::try_from(limits.entries).map_err(HostError::backend)?;
        let publications = u64::try_from(limits.publications).map_err(HostError::backend)?;
        let leases = limits
            .entries
            .checked_mul(2)
            .and_then(|n| {
                limits
                    .publications
                    .checked_mul(2)
                    .and_then(|events| n.checked_add(events))
            })
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| {
                HostError::ResourceExhausted("metadata lease capacity overflow".into())
            })?;
        let requirements = ResourceRequirements::none()
            .with_class(ResourceClass::LiveObject, entries, entries)
            .with_class(ResourceClass::EventItem, publications, publications)
            .with_class(
                ResourceClass::BackendStateBytes,
                limits.total_bytes,
                limits.total_bytes,
            )
            .with_metadata(1, leases);
        let resource_limits = ResourceLimits::deny_all(1, leases)
            .with_class(ResourceClass::LiveObject, entries, entries)
            .with_class(ResourceClass::EventItem, publications, publications)
            .with_class(
                ResourceClass::BackendStateBytes,
                limits.total_bytes,
                limits.total_bytes,
            );
        let resources = ProductionResourceLedger::new(resource_limits, requirements)
            .and_then(|ledger| ledger.claim(requirements))
            .map_err(HostError::backend)?;
        Ok(Self {
            state: Mutex::new(MetadataState {
                active: BTreeMap::new(),
                pending: VecDeque::new(),
                sequence: 0,
                discarded: false,
                connection: ConnectionPublication::Open,
                collections: None,
                collection_batch: None,
                queue_storage: None,
                drain: drain::DrainOwnership::default(),
            }),
            resources,
            limits,
            names: None,
            changed: Notify::new(),
        })
    }

    pub fn with_names(mut self, limits: FrameNameLimits) -> Result<Self, HostError> {
        self.names = Some(NameResources::new(self.limits, limits)?);
        Ok(self)
    }

    pub fn get(&self, frame: ObjectId) -> Result<FrameUrlSnapshot, HostError> {
        self.state
            .lock()
            .map_err(HostError::backend)?
            .active
            .get(&frame)
            .cloned()
            .ok_or_else(|| HostError::Backend("Frame URL snapshot is not available".into()))
    }

    /// Attachment precedes navigation; upstream starts such a Frame at empty URL.
    pub(crate) fn ensure_attached(&self, frame: ObjectId) -> Result<FrameUrlSnapshot, HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if let Some(snapshot) = state.active.get(&frame) {
            return Ok(snapshot.clone());
        }
        let snapshot = self.allocate("", None)?;
        state.active.insert(frame, snapshot.clone());
        Ok(snapshot)
    }

    /// Admit the no-options new Page's initial metadata before native mutation.
    /// Dropping an unused reservation releases both its entry and URL bytes.
    pub fn reserve_new_page(&self) -> Result<FrameUrlSnapshot, HostError> {
        self.allocate("about:blank", None)
    }

    /// Called by the serial lifecycle driver before it resumes native events.
    pub fn activate_new_page(
        &self,
        frame: ObjectId,
        snapshot: FrameUrlSnapshot,
    ) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        match state.active.entry(frame) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(snapshot);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(_) => Err(HostError::Backend(
                "new Page main Frame metadata was already initialized".into(),
            )),
        }
    }

    fn allocate(&self, url: &str, fragment: Option<&str>) -> Result<FrameUrlSnapshot, HostError> {
        self.allocate_navigation(url, fragment, "")
    }

    fn allocate_navigation(
        &self,
        url: &str,
        fragment: Option<&str>,
        name: &str,
    ) -> Result<FrameUrlSnapshot, HostError> {
        let entry = self
            .resources
            .reserve(SCOPE, ResourceClass::LiveObject, 1)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let name = self
            .names
            .as_ref()
            .map(|names| names.allocate(name))
            .transpose()?
            .flatten();
        let value = Arc::new(self.budgeted_url(url, fragment, name)?);
        Ok(FrameUrlSnapshot(Arc::new(FrameUrlState {
            url: Mutex::new(UrlViews {
                native: value.clone(),
                published: value,
            }),
            names_enabled: self.names.is_some(),
            _entry: entry,
        })))
    }

    pub fn clear(&self) {
        // Cleanup only. Retained wrappers keep their last URL and leases.
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .active
            .clear();
    }

    pub fn publications(self: &Arc<Self>) -> FramePublications {
        FramePublications(self.clone())
    }

    pub fn metrics(&self) -> Result<FrameMetadataMetrics, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        let metrics = self.resources.metrics().map_err(HostError::backend)?;
        Ok(FrameMetadataMetrics {
            active_frames: u32::try_from(state.active.len()).map_err(HostError::backend)?,
            pending_publications: u32::try_from(state.pending.len()).map_err(HostError::backend)?,
            entries: u32::try_from(metrics.class(ResourceClass::LiveObject).current)
                .map_err(HostError::backend)?,
            url_bytes: u32::try_from(metrics.class(ResourceClass::BackendStateBytes).current)
                .map_err(HostError::backend)?,
            name_bytes: self.names.as_ref().map_or(Ok(0), NameResources::bytes)?,
        })
    }

    fn budgeted_url(
        &self,
        url: &str,
        fragment: Option<&str>,
        name: Option<Arc<BudgetedName>>,
    ) -> Result<BudgetedUrl, HostError> {
        let fragment = fragment.unwrap_or("");
        let bytes = url
            .len()
            .checked_add(fragment.len())
            .filter(|bytes| *bytes <= self.limits.url_bytes)
            .ok_or_else(|| {
                HostError::ResourceExhausted("Frame URL exceeds native metadata byte limit".into())
            })?;
        // Reserve the new allocation before copying. The previous value stays
        // charged until replacement succeeds, including transient overlap.
        let lease = if bytes == 0 {
            None
        } else {
            Some(
                self.resources
                    .reserve(
                        SCOPE,
                        ResourceClass::BackendStateBytes,
                        u64::try_from(bytes).map_err(HostError::backend)?,
                    )
                    .map_err(|error| HostError::ResourceExhausted(error.to_string()))?,
            )
        };
        let mut value = String::with_capacity(bytes);
        value.push_str(url);
        value.push_str(fragment);
        Ok(BudgetedUrl {
            value: value.into_boxed_str(),
            name,
            _bytes: lease,
        })
    }
    fn update_metadata(
        &self,
        frame: ObjectId,
        url: &str,
        fragment: Option<&str>,
        navigation_name: Option<&str>,
    ) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if let Some(snapshot) = state.active.get(&frame).cloned() {
            let mut current = snapshot.0.url.lock().map_err(HostError::backend)?;
            // Identical initialization/events need no temporary allocation.
            let name_changed = self.names.is_some()
                && navigation_name.is_some_and(|name| name != names::value(&current.native.name));
            if !name_changed
                && current.native.value.strip_prefix(url) == Some(fragment.unwrap_or(""))
            {
                return Ok(());
            }
            let make_value = || {
                let name = if name_changed {
                    self.names
                        .as_ref()
                        .expect("selected name resources")
                        .allocate(navigation_name.expect("navigation name"))?
                } else {
                    current.native.name.clone()
                };
                self.budgeted_url(url, fragment, name).map(Arc::new)
            };
            if self.limits.publications == 0 || state.discarded {
                let value = make_value()?;
                current.native = value.clone();
                if !state.discarded {
                    current.published = value;
                }
                return Ok(());
            }
            let sequence = state.next_sequence()?;
            let event = self
                .resources
                .reserve(SCOPE, ResourceClass::EventItem, 1)
                .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
            let value = make_value()?;
            state.pending.push_back(FrameChange {
                sequence,
                value: FrameChangeValue::Url {
                    target: snapshot.clone(),
                    value: value.clone(),
                },
                _event: event,
            });
            current.native = value;
            state.sequence = sequence;
            self.changed.notify_one();
        } else {
            state.active.insert(
                frame,
                self.allocate_navigation(url, fragment, navigation_name.unwrap_or(""))?,
            );
        }
        Ok(())
    }
}

impl ChromiumFrameUrlObserver for FrameMetadata {
    fn update(
        &self,
        frame: ObjectId,
        url: &str,
        fragment: Option<&str>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.update_metadata(frame, url, fragment, None)
            .map_err(Into::into)
    }

    fn update_navigation(
        &self,
        frame: ObjectId,
        url: &str,
        fragment: Option<&str>,
        name: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.update_metadata(frame, url, fragment, Some(name))
            .map_err(Into::into)
    }

    fn retain_live(
        &self,
        is_live: &dyn Fn(ObjectId) -> bool,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.state
            .lock()
            .map_err(HostError::backend)?
            .active
            .retain(|frame, _| is_live(*frame));
        Ok(())
    }
}

#[cfg(test)]
mod tree_tests;

#[cfg(test)]
mod connection_tests;

#[cfg(test)]
mod name_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn publication_book(publications: usize, total_bytes: u64) -> Arc<FrameMetadata> {
        Arc::new(
            FrameMetadata::new(FrameMetadataLimits {
                entries: 2,
                url_bytes: 32,
                total_bytes,
                publications,
            })
            .unwrap(),
        )
    }

    #[test]
    fn watermark_publishes_only_changes_observed_when_captured() {
        // Arrange
        let book = publication_book(4, 128);
        let id = ObjectId::new(1);
        book.update(id, "old", None).unwrap();
        let frame = book.get(id).unwrap();
        book.update(id, "first", None).unwrap();
        let fence = book.publications().fence().unwrap();
        book.update(id, "second", None).unwrap();
        assert_eq!(frame.published_url().unwrap(), "old");
        assert_eq!(frame.url().unwrap(), "second");
        assert!(frame.has_unpublished_url().unwrap());
        // Act
        fence.publish().unwrap();
        // Assert
        assert_eq!(frame.published_url().unwrap(), "first");
        assert_eq!(book.metrics().unwrap().pending_publications, 1);
        assert_eq!(book.metrics().unwrap().url_bytes, 11);
    }

    #[test]
    fn delayed_older_watermark_cannot_regress_a_newer_publication() {
        // Arrange
        let book = publication_book(4, 128);
        let id = ObjectId::new(1);
        book.update(id, "old", None).unwrap();
        let frame = book.get(id).unwrap();
        book.update(id, "first", None).unwrap();
        let old = book.publications().fence().unwrap();
        book.update(id, "second", None).unwrap();
        book.publications().fence().unwrap().publish().unwrap();
        // Act
        old.publish().unwrap();
        // Assert
        assert_eq!(frame.published_url().unwrap(), "second");
        assert!(!frame.has_unpublished_url().unwrap());
        assert_eq!(book.metrics().unwrap().pending_publications, 0);
        assert_eq!(book.metrics().unwrap().url_bytes, 6);
    }

    #[test]
    fn publication_overflow_rejects_before_native_or_visible_state_changes() {
        // Arrange
        let book = publication_book(1, 128);
        let id = ObjectId::new(1);
        book.update(id, "old", None).unwrap();
        let frame = book.get(id).unwrap();
        book.update(id, "first", None).unwrap();
        let before = book.metrics().unwrap();
        // Act
        let result = book.update(id, "second", None);
        // Assert
        assert!(result.is_err());
        assert_eq!(frame.url().unwrap(), "first");
        assert_eq!(frame.published_url().unwrap(), "old");
        assert_eq!(book.metrics().unwrap(), before);
    }

    #[test]
    fn byte_rejection_releases_publication_item_admission() {
        // Arrange
        let book = publication_book(1, 6);
        let id = ObjectId::new(1);
        book.update(id, "old", None).unwrap();
        // Act
        let result = book.update(id, "oversize", None);
        // Assert
        assert!(result.is_err());
        assert_eq!(book.metrics().unwrap().pending_publications, 0);
        book.update(id, "new", None).unwrap();
        assert_eq!(book.metrics().unwrap().pending_publications, 1);
    }

    #[test]
    fn retired_frame_publishes_without_a_live_route_and_releases_after_delivery() {
        // Arrange
        let book = publication_book(1, 32);
        let id = ObjectId::new(1);
        book.update(id, "old", None).unwrap();
        let frame = book.get(id).unwrap();
        book.update(id, "new", None).unwrap();
        let fence = book.publications().fence().unwrap();
        book.clear();
        // Act
        fence.publish().unwrap();
        // Assert
        assert_eq!(frame.published_url().unwrap(), "new");
        assert_eq!(book.metrics().unwrap().active_frames, 0);
        drop(frame);
        assert_eq!(book.metrics().unwrap().entries, 0);
        assert_eq!(book.metrics().unwrap().url_bytes, 0);
    }

    #[test]
    fn environment_discard_reclaims_queued_changes_without_publication() {
        // Arrange
        let book = publication_book(2, 32);
        let id = ObjectId::new(1);
        book.update(id, "old", None).unwrap();
        let frame = book.get(id).unwrap();
        book.update(id, "first", None).unwrap();
        book.update(id, "new", None).unwrap();
        let fence = book.publications().fence().unwrap();
        book.clear();
        // Act
        book.publications().discard();
        // Assert
        fence.publish().unwrap();
        assert_eq!(frame.published_url().unwrap(), "old");
        assert_eq!(book.metrics().unwrap().pending_publications, 0);
        assert_eq!(book.metrics().unwrap().url_bytes, 6);
        drop(frame);
        assert_eq!(book.metrics().unwrap().entries, 0);
        assert_eq!(book.metrics().unwrap().url_bytes, 0);
    }

    #[tokio::test]
    async fn publication_wait_observes_an_update_that_precedes_registration() {
        // Arrange
        let book = publication_book(1, 32);
        let id = ObjectId::new(1);
        book.update(id, "old", None).unwrap();
        book.update(id, "new", None).unwrap();
        // Act
        let fence = book.publications().next().await.unwrap();
        // Assert
        fence.publish().unwrap();
        assert_eq!(book.get(id).unwrap().published_url().unwrap(), "new");
        assert!(book.publications().is_drained().unwrap());
    }

    #[test]
    fn teardown_disables_publication_before_remaining_native_updates_drain() {
        // Arrange
        let book = publication_book(1, 32);
        let id = ObjectId::new(1);
        book.update(id, "old", None).unwrap();
        let frame = book.get(id).unwrap();
        book.update(id, "first", None).unwrap();
        book.publications().discard();
        // Act
        book.update(id, "late", None).unwrap();
        // Assert
        assert_eq!(frame.url().unwrap(), "late");
        assert_eq!(frame.published_url().unwrap(), "old");
        assert!(book.publications().is_drained().unwrap());
        book.clear();
        drop(frame);
        assert_eq!(book.metrics().unwrap().entries, 0);
        assert_eq!(book.metrics().unwrap().url_bytes, 0);
    }

    fn book(entries: usize, url_bytes: usize, total_bytes: u64) -> FrameMetadata {
        FrameMetadata::new(FrameMetadataLimits {
            entries,
            url_bytes,
            total_bytes,
            publications: 0,
        })
        .unwrap()
    }

    #[test]
    fn retained_snapshot_survives_close_without_an_active_route_and_releases_on_drop() {
        // Arrange
        let book = book(2, 32, 64);
        book.update(ObjectId::new(1), "https://a.test/", Some("#你好"))
            .unwrap();
        let snapshot = book.get(ObjectId::new(1)).unwrap();
        // Act
        book.clear();
        // Assert
        assert_eq!(snapshot.url().unwrap(), "https://a.test/#你好");
        assert_eq!(
            book.metrics().unwrap(),
            FrameMetadataMetrics {
                entries: 1,
                url_bytes: 22,
                name_bytes: 0,
                active_frames: 0,
                pending_publications: 0,
            }
        );
        drop(snapshot);
        assert_eq!(
            book.metrics().unwrap(),
            FrameMetadataMetrics {
                entries: 0,
                url_bytes: 0,
                name_bytes: 0,
                active_frames: 0,
                pending_publications: 0,
            }
        );
    }

    #[test]
    fn old_and_new_url_bytes_must_fit_before_replacement() {
        // Arrange
        let book = book(1, 16, 10);
        book.update(ObjectId::new(1), "123456", None).unwrap();
        let snapshot = book.get(ObjectId::new(1)).unwrap();
        // Act
        let result = book.update(ObjectId::new(1), "abcde", None);
        // Assert
        assert!(result.is_err());
        assert_eq!(snapshot.url().unwrap(), "123456");
        assert_eq!(book.metrics().unwrap().url_bytes, 6);
    }

    #[test]
    fn disposed_wrappers_still_consume_entry_admission() {
        // Arrange
        let book = book(1, 16, 32);
        book.update(ObjectId::new(1), "old", None).unwrap();
        let old = book.get(ObjectId::new(1)).unwrap();
        book.retain_live(&|_| false).unwrap();
        // Act
        let result = book.update(ObjectId::new(2), "new", None);
        // Assert
        assert!(result.is_err());
        assert_eq!(old.url().unwrap(), "old");
        drop(old);
        book.update(ObjectId::new(2), "new", None).unwrap();
        assert_eq!(book.metrics().unwrap().entries, 1);
    }

    #[test]
    fn new_page_reservation_rejects_when_a_retired_wrapper_holds_the_last_entry() {
        // Arrange
        let book = book(1, 16, 32);
        book.update(ObjectId::new(1), "old", None).unwrap();
        let retained = book.get(ObjectId::new(1)).unwrap();
        book.clear();
        // Act
        let result = book.reserve_new_page();
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(retained.url().unwrap(), "old");
        assert_eq!(book.metrics().unwrap().active_frames, 0);
        assert_eq!(book.metrics().unwrap().entries, 1);
    }

    #[test]
    fn new_page_reservation_releases_entry_when_url_bytes_are_unavailable() {
        // Arrange
        let book = book(1, 16, 10);
        // Act
        let result = book.reserve_new_page();
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(book.metrics().unwrap().entries, 0);
        assert_eq!(book.metrics().unwrap().url_bytes, 0);
    }

    #[test]
    fn abandoned_new_page_reservation_releases_without_publication() {
        // Arrange
        let book = book(1, 16, 32);
        let reservation = book.reserve_new_page().unwrap();
        assert_eq!(book.metrics().unwrap().entries, 1);
        assert_eq!(book.metrics().unwrap().active_frames, 0);
        // Act
        drop(reservation);
        // Assert
        assert_eq!(book.metrics().unwrap().entries, 0);
        assert_eq!(book.metrics().unwrap().url_bytes, 0);
        assert_eq!(book.metrics().unwrap().pending_publications, 0);
    }

    #[test]
    fn created_page_activates_the_reserved_snapshot_without_double_charging() {
        // Arrange
        let book = book(1, 16, 11);
        let reservation = book.reserve_new_page().unwrap();
        // Act
        book.activate_new_page(ObjectId::new(1), reservation.clone())
            .unwrap();
        // Assert
        book.update(ObjectId::new(1), "about:blank", None).unwrap();
        assert!(Arc::ptr_eq(
            &reservation.0,
            &book.get(ObjectId::new(1)).unwrap().0
        ));
        assert_eq!(reservation.published_url().unwrap(), "about:blank");
        assert_eq!(book.metrics().unwrap().entries, 1);
        assert_eq!(book.metrics().unwrap().url_bytes, 11);
        assert_eq!(book.metrics().unwrap().active_frames, 1);
    }

    #[test]
    fn empty_url_retains_only_its_metadata_entry() {
        // Arrange
        let book = book(1, 16, 32);
        // Act
        book.update(ObjectId::new(1), "", None).unwrap();
        // Assert
        assert_eq!(book.get(ObjectId::new(1)).unwrap().url().unwrap(), "");
        assert_eq!(book.metrics().unwrap().entries, 1);
        assert_eq!(book.metrics().unwrap().url_bytes, 0);
    }

    #[test]
    fn unicode_limit_rejects_before_allocating_a_new_entry_or_truncating() {
        // Arrange
        let book = book(1, 5, 32);
        // Act
        let result = book.update(ObjectId::new(1), "你好", None);
        // Assert
        assert!(result.is_err());
        assert_eq!(book.metrics().unwrap().entries, 0);
        assert_eq!(book.metrics().unwrap().url_bytes, 0);
    }
}
