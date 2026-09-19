//! Caller-driven, bounded Frame metadata publication. No runtime, transport or
//! browser owner is retained here. Node integration must publish these tokens at
//! the same event-loop fence as URL metadata, not read the native view directly.
use crate::{FrameUrlSnapshot, HostError};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceLedger,
    ResourceLimits, ResourceRequirements, ResourceScope,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

const SCOPE: ResourceScope = ResourceScope::extension(121, 2);

#[derive(Clone, Copy)]
pub struct FrameTreeLimits {
    /// Combined retained Page-tree, Frame-node and FFI binding states.
    pub states: usize,
    pub frames_per_page: usize,
    /// Shared by immutable revisions and temporary, admitted snapshot reads.
    pub revisions: usize,
    pub bytes: u64,
}
#[derive(Debug, PartialEq, Eq)]
pub struct FrameTreeMetrics {
    pub states: u64,
    pub revisions: u64,
    pub bytes: u64,
}
#[derive(Clone)]
pub struct FrameTreeFactory {
    ledger: ResourceLedger,
    limits: FrameTreeLimits,
}
pub(crate) struct Storage {
    _count: ResourceLease,
    _bytes: ResourceLease,
}
fn invalid(message: &str) -> HostError {
    HostError::InvalidArgument(message.into())
}
fn exhausted(error: impl std::fmt::Display) -> HostError {
    HostError::ResourceExhausted(error.to_string())
}
impl FrameTreeFactory {
    pub(crate) fn reserve_tree_binding(&self) -> Result<ReservedTreeBinding, HostError> {
        Ok(ReservedTreeBinding(self.reserve(
            ResourceClass::LiveObject,
            std::mem::size_of::<FrameTreeBinding>(),
        )?))
    }
    pub fn reserve_conversion(&self, bytes: usize) -> Result<FrameConversion, HostError> {
        let bytes = bytes
            .checked_add(std::mem::size_of::<FrameConversion>())
            .ok_or_else(|| invalid("Frame conversion storage overflow"))?;
        Ok(FrameConversion(
            self.reserve(ResourceClass::EventItem, bytes)?,
        ))
    }
    pub(crate) fn reserve_binding(&self) -> Result<ReservedFrameBinding, HostError> {
        Ok(ReservedFrameBinding(self.reserve(
            ResourceClass::LiveObject,
            std::mem::size_of::<FrameBinding>(),
        )?))
    }
    pub(crate) fn reserve_index(&self, bytes: usize) -> Result<Storage, HostError> {
        self.reserve(ResourceClass::LiveObject, bytes)
    }

    pub(crate) fn reserve_collection_storage(&self, bytes: usize) -> Result<Storage, HostError> {
        self.reserve(ResourceClass::EventItem, bytes)
    }

    /// Reserve all initial metadata before the caller creates a native Page.
    pub(crate) fn reserve_page(
        &self,
        url: FrameUrlSnapshot,
    ) -> Result<ReservedPageTree, HostError> {
        Ok(ReservedPageTree {
            factory: self.clone(),
            url,
            node: self.reserve(ResourceClass::LiveObject, arc_bytes::<NodeState>())?,
            tree: self.reserve(ResourceClass::LiveObject, arc_bytes::<TreeState>())?,
            revision: self.reserve(
                ResourceClass::EventItem,
                arc_bytes::<Revision>() + std::mem::size_of::<FrameNode>(),
            )?,
        })
    }
    pub fn new(limits: FrameTreeLimits) -> Result<Self, HostError> {
        if limits.states == 0 || limits.frames_per_page == 0 || limits.revisions == 0 {
            return Err(invalid("Frame tree capacities must be positive"));
        }
        let leases = limits
            .states
            .checked_add(limits.revisions)
            .and_then(|n| n.checked_mul(2))
            .ok_or_else(|| invalid("Frame tree lease capacity overflow"))?;
        let requirements = ResourceRequirements::none()
            .with_class(
                ResourceClass::LiveObject,
                limits.states as u64,
                limits.states as u64,
            )
            .with_class(
                ResourceClass::EventItem,
                limits.revisions as u64,
                limits.revisions as u64,
            )
            .with_class(ResourceClass::BackendStateBytes, limits.bytes, limits.bytes)
            .with_metadata(1, leases);
        let bounds = ResourceLimits::deny_all(1, leases)
            .with_class(
                ResourceClass::LiveObject,
                limits.states as u64,
                limits.states as u64,
            )
            .with_class(
                ResourceClass::EventItem,
                limits.revisions as u64,
                limits.revisions as u64,
            )
            .with_class(ResourceClass::BackendStateBytes, limits.bytes, limits.bytes);
        let ledger = ProductionResourceLedger::new(bounds, requirements)
            .and_then(|ledger| ledger.claim(requirements))
            .map_err(exhausted)?;
        Ok(Self { ledger, limits })
    }
    fn reserve(&self, class: ResourceClass, bytes: usize) -> Result<Storage, HostError> {
        let count = self.ledger.reserve(SCOPE, class, 1).map_err(exhausted)?;
        let bytes = self
            .ledger
            .reserve(SCOPE, ResourceClass::BackendStateBytes, bytes as u64)
            .map_err(exhausted)?;
        Ok(Storage {
            _count: count,
            _bytes: bytes,
        })
    }
    pub fn frame(
        &self,
        id: ObjectId,
        parent: Option<ObjectId>,
        url: FrameUrlSnapshot,
    ) -> Result<FrameNode, HostError> {
        if parent == Some(id) {
            return Err(invalid("Frame cannot parent itself"));
        }
        let storage = self.reserve(ResourceClass::LiveObject, arc_bytes::<NodeState>())?;
        Ok(FrameNode(Arc::new(NodeState {
            id,
            parent,
            url,
            budget: self.ledger.clone(),
            page_root_claimed: AtomicBool::new(false),
            detached: AtomicBool::new(false),
            _storage: storage,
        })))
    }
    pub fn page(&self, root: FrameNode) -> Result<FrameTree, HostError> {
        if root.parent_id().is_some() || root.is_detached() {
            return Err(invalid("Page tree requires a live main Frame"));
        }
        if !self.ledger.shares_budget_with(&root.0.budget) {
            return Err(invalid("Page root belongs to another Frame budget"));
        }
        let storage = self.reserve(ResourceClass::LiveObject, arc_bytes::<TreeState>())?;
        let revision = self.revision(0, std::slice::from_ref(&root), std::iter::empty())?;
        root.0
            .page_root_claimed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| invalid("Main Frame already belongs to a Page tree"))?;
        Ok(FrameTree(Arc::new(TreeState {
            factory: self.clone(),
            views: Mutex::new(Views {
                native: revision.clone(),
                published: revision,
                closed: false,
                published_closed: false,
            }),
            _storage: storage,
        })))
    }
    fn revision<'a>(
        &self,
        sequence: u64,
        frames: &[FrameNode],
        retired: impl Iterator<Item = &'a FrameNode> + Clone,
    ) -> Result<Arc<Revision>, HostError> {
        let retired_count = retired.clone().count();
        if retired_count > self.limits.states {
            return Err(exhausted("Frame retirement count exceeds admission"));
        }
        let bytes = frames
            .len()
            .checked_add(retired_count)
            .and_then(|n| n.checked_mul(std::mem::size_of::<FrameNode>()))
            .and_then(|n| n.checked_add(arc_bytes::<Revision>()))
            .ok_or_else(|| invalid("Frame revision storage overflow"))?;
        let storage = self.reserve(ResourceClass::EventItem, bytes)?;
        // Two passes over borrowed state admit the final allocation before copy.
        // No extra full-capacity scratch vector or scratch lease is necessary.
        let mut retired_nodes = Vec::with_capacity(retired_count);
        retired_nodes.extend(retired.cloned());
        Ok(Arc::new(Revision {
            sequence,
            frames: frames.into(),
            retired: retired_nodes.into_boxed_slice(),
            _storage: storage,
        }))
    }
    pub fn metrics(&self) -> Result<FrameTreeMetrics, HostError> {
        let metrics = self.ledger.metrics().map_err(exhausted)?;
        Ok(FrameTreeMetrics {
            states: metrics.class(ResourceClass::LiveObject).current,
            revisions: metrics.class(ResourceClass::EventItem).current,
            bytes: metrics.class(ResourceClass::BackendStateBytes).current,
        })
    }
}
const fn arc_bytes<T>() -> usize {
    std::mem::size_of::<T>() + 2 * std::mem::size_of::<usize>()
}

struct NodeState {
    id: ObjectId,
    parent: Option<ObjectId>,
    url: FrameUrlSnapshot,
    budget: ResourceLedger,
    page_root_claimed: AtomicBool,
    detached: AtomicBool,
    _storage: Storage,
}
#[derive(Clone)]
pub struct FrameNode(Arc<NodeState>);
impl FrameNode {
    pub(crate) fn metadata_snapshot(&self) -> FrameUrlSnapshot {
        self.0.url.clone()
    }

    pub fn with_url<T>(&self, read: impl FnOnce(&str) -> T) -> Result<T, HostError> {
        self.0.url.with_published_url(read)
    }

    pub fn with_name<T>(&self, read: impl FnOnce(&str) -> T) -> Result<T, HostError> {
        self.0.url.with_published_name(read)
    }

    pub fn id(&self) -> ObjectId {
        self.0.id
    }
    pub fn parent_id(&self) -> Option<ObjectId> {
        self.0.parent
    }
    pub fn is_detached(&self) -> bool {
        self.0.detached.load(Ordering::Acquire)
    }
    pub fn url(&self) -> Result<String, HostError> {
        self.0.url.published_url()
    }
    pub fn has_unpublished_url(&self) -> Result<bool, HostError> {
        self.0.url.has_unpublished_url()
    }
    pub fn same_identity(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
struct Revision {
    sequence: u64,
    frames: Box<[FrameNode]>,
    retired: Box<[FrameNode]>,
    _storage: Storage,
}
struct Views {
    native: Arc<Revision>,
    published: Arc<Revision>,
    closed: bool,
    published_closed: bool,
}
struct TreeState {
    factory: FrameTreeFactory,
    views: Mutex<Views>,
    _storage: Storage,
}
#[derive(Clone)]
pub struct FrameTree(Arc<TreeState>);
/// Metadata-only temporary conversion lease; never owns the FFI task registry.
pub struct FrameConversion(Storage);
impl FrameConversion {
    pub fn finish(self) {
        drop(self.0);
    }
}
/// The native Page-tree wrapper's own storage, retained until finalization.
pub struct FrameTreeBinding {
    tree: FrameTree,
    _storage: Storage,
}
impl FrameTreeBinding {
    pub fn tree(&self) -> &FrameTree {
        &self.tree
    }
}
pub(crate) struct ReservedTreeBinding(Storage);
impl ReservedTreeBinding {
    pub(crate) fn activate(self, tree: FrameTree) -> FrameTreeBinding {
        FrameTreeBinding {
            tree,
            _storage: self.0,
        }
    }
}
/// A borrowed immutable collection with conversion storage reserved before copy.
/// This owner contains no host, task registry, transport or execution context.
pub struct FrameTreeRead {
    revision: Arc<Revision>,
    _storage: Storage,
}
impl FrameTreeRead {
    pub fn frames(&self) -> &[FrameNode] {
        &self.revision.frames
    }

    /// Bind from this immutable read, even if a newer tree view retired it.
    pub fn bind(&self, id: ObjectId) -> Result<FrameBinding, HostError> {
        let node = self
            .frames()
            .iter()
            .find(|node| node.id() == id)
            .ok_or(HostError::TargetClosed)?;
        let count = node
            .0
            .budget
            .reserve(SCOPE, ResourceClass::LiveObject, 1)
            .map_err(exhausted)?;
        let bytes = node
            .0
            .budget
            .reserve(
                SCOPE,
                ResourceClass::BackendStateBytes,
                std::mem::size_of::<FrameBinding>() as u64,
            )
            .map_err(exhausted)?;
        Ok(FrameBinding {
            node: node.clone(),
            _storage: Storage {
                _count: count,
                _bytes: bytes,
            },
        })
    }
}
/// FFI wrapper storage remains charged until the native wrapper is finalized.
pub struct FrameBinding {
    node: FrameNode,
    _storage: Storage,
}
pub(crate) struct ReservedFrameBinding(Storage);
impl ReservedFrameBinding {
    pub(crate) fn activate(self, node: FrameNode) -> FrameBinding {
        FrameBinding {
            node,
            _storage: self.0,
        }
    }
}
impl FrameBinding {
    pub fn node(&self) -> &FrameNode {
        &self.node
    }
}
pub struct FrameTreePublication {
    tree: FrameTree,
    revision: Arc<Revision>,
}

/// One-shot metadata publication, retaining the tree's charged storage until
/// delivery/discard. This contains no host, browser, runtime or transport owner.
pub struct FrameClosePublication(FrameTree);

impl FrameClosePublication {
    pub(crate) fn is_for(&self, tree: &FrameTree) -> bool {
        Arc::ptr_eq(&self.0.0, &tree.0)
    }

    pub fn publish(self) -> Result<(), HostError> {
        self.0
            .0
            .views
            .lock()
            .map_err(HostError::backend)?
            .published_closed = true;
        Ok(())
    }
}

pub(crate) struct ReservedPageTree {
    factory: FrameTreeFactory,
    url: FrameUrlSnapshot,
    node: Storage,
    tree: Storage,
    revision: Storage,
}

impl ReservedPageTree {
    pub(crate) fn activate(self, id: ObjectId) -> (FrameNode, FrameTree) {
        let root = FrameNode(Arc::new(NodeState {
            id,
            parent: None,
            url: self.url,
            budget: self.factory.ledger.clone(),
            page_root_claimed: AtomicBool::new(true),
            detached: AtomicBool::new(false),
            _storage: self.node,
        }));
        let revision = Arc::new(Revision {
            sequence: 0,
            frames: vec![root.clone()].into_boxed_slice(),
            retired: Box::new([]),
            _storage: self.revision,
        });
        let tree = FrameTree(Arc::new(TreeState {
            factory: self.factory,
            views: Mutex::new(Views {
                native: revision.clone(),
                published: revision,
                closed: false,
                published_closed: false,
            }),
            _storage: self.tree,
        }));
        (root, tree)
    }
}

impl FrameTree {
    pub(crate) fn main_frame_snapshot(&self) -> Result<FrameNode, HostError> {
        let views = self.0.views.lock().map_err(HostError::backend)?;
        Ok(views.published.frames[0].clone())
    }

    /// Metadata-only admission shared by synchronous lookup/conversion helpers.
    /// Borrowing it does not retain a browser, runtime, or transport owner.
    pub fn lookup_resources(&self) -> &FrameTreeFactory {
        &self.0.factory
    }

    pub fn bind_tree(&self) -> Result<FrameTreeBinding, HostError> {
        Ok(self
            .0
            .factory
            .reserve_tree_binding()?
            .activate(self.clone()))
    }
    /// Hold this guard through actual Node value conversion, not just Vec creation.
    /// `bytes_per_frame` covers the caller's temporary descriptor and ID copies.
    pub fn read(&self, bytes_per_frame: usize) -> Result<FrameTreeRead, HostError> {
        let views = self.0.views.lock().map_err(HostError::backend)?;
        let bytes = views
            .published
            .frames
            .len()
            .checked_mul(bytes_per_frame)
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<FrameTreeRead>()))
            .ok_or_else(|| invalid("Frame read storage overflow"))?;
        let storage = self.0.factory.reserve(ResourceClass::EventItem, bytes)?;
        Ok(FrameTreeRead {
            revision: views.published.clone(),
            _storage: storage,
        })
    }

    pub fn bind(&self, id: u64) -> Result<FrameBinding, HostError> {
        let views = self.0.views.lock().map_err(HostError::backend)?;
        let node = views
            .published
            .frames
            .iter()
            .find(|node| node.id() == ObjectId::new(id))
            .ok_or(HostError::TargetClosed)?;
        Ok(self.0.factory.reserve_binding()?.activate(node.clone()))
    }

    pub(crate) fn matches_native(&self, frames: &[FrameNode]) -> Result<bool, HostError> {
        let views = self.0.views.lock().map_err(HostError::backend)?;
        Ok(views.native.frames.len() == frames.len()
            && views
                .native
                .frames
                .iter()
                .zip(frames)
                .all(|(left, right)| left.same_identity(right)))
    }

    pub(crate) fn close_for_teardown(&self) -> FrameClosePublication {
        self.0
            .views
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .closed = true;
        FrameClosePublication(self.clone())
    }
    /// Caller owns the output vector allocation; integration must admit its FFI
    /// conversion separately. Node storage/URL leases follow every returned Arc.
    pub fn frames(&self) -> Result<Vec<FrameNode>, HostError> {
        Ok(self
            .0
            .views
            .lock()
            .map_err(HostError::backend)?
            .published
            .frames
            .to_vec())
    }
    /// Stage an authoritative topology snapshot. Identity must come from the
    /// runtime's non-reused portable IDs, not native renderer IDs or DOM indexes.
    pub fn stage(&self, frames: &[FrameNode]) -> Result<FrameTreePublication, HostError> {
        let mut views = self.0.views.lock().map_err(HostError::backend)?;
        if views.closed {
            return Err(HostError::TargetClosed);
        }
        if frames.is_empty() || frames.len() > self.0.factory.limits.frames_per_page {
            return Err(exhausted("Frame tree snapshot count exceeds admission"));
        }
        let root = &views.native.frames[0];
        if !frames[0].same_identity(root) {
            return Err(invalid("Page main Frame identity changed"));
        }
        for (index, frame) in frames.iter().enumerate() {
            if !self.0.factory.ledger.shares_budget_with(&frame.0.budget) {
                return Err(invalid("Frame belongs to another tree budget"));
            }
            if frame.is_detached() || frames[..index].iter().any(|other| other.id() == frame.id()) {
                return Err(invalid("detached or duplicate Frame in snapshot"));
            }
            if views
                .native
                .retired
                .iter()
                .any(|retired| retired.id() == frame.id())
            {
                return Err(invalid("retired Frame identity cannot be reused"));
            }
            if views
                .native
                .frames
                .iter()
                .chain(views.published.frames.iter())
                .any(|prior| prior.id() == frame.id() && !prior.same_identity(frame))
            {
                return Err(invalid("Frame ID was rebound to a different snapshot"));
            }
            let mut cursor = frame;
            for depth in 0..=frames.len() {
                if depth == frames.len() {
                    return Err(invalid("Frame parent cycle"));
                }
                match cursor.parent_id() {
                    Some(parent) => {
                        cursor = frames
                            .iter()
                            .find(|node| node.id() == parent)
                            .ok_or_else(|| invalid("Frame parent missing from snapshot"))?
                    }
                    None if cursor.same_identity(root) => break,
                    None => return Err(invalid("multiple main Frames in snapshot")),
                }
            }
        }
        let retired = views
            .native
            .retired
            .iter()
            .filter(|node| !node.is_detached())
            .chain(
                views
                    .native
                    .frames
                    .iter()
                    .filter(|node| !frames.iter().any(|next| next.same_identity(node))),
            );
        let sequence = views
            .native
            .sequence
            .checked_add(1)
            .ok_or_else(|| exhausted("Frame revision sequence exhausted"))?;
        let revision = self.0.factory.revision(sequence, frames, retired)?;
        views.native = revision.clone();
        Ok(FrameTreePublication {
            tree: self.clone(),
            revision,
        })
    }
    /// Page/transport closure is not a Frame-detached notification. Keep the
    /// published collection and retained parent/URL views, as upstream does.
    pub fn close(&self) -> Result<FrameClosePublication, HostError> {
        self.0.views.lock().map_err(HostError::backend)?.closed = true;
        Ok(FrameClosePublication(self.clone()))
    }

    /// Synchronous language-facing state. Native retirement alone must not
    /// change this value while the language event loop is blocked.
    pub fn is_closed(&self) -> Result<bool, HostError> {
        Ok(self
            .0
            .views
            .lock()
            .map_err(HostError::backend)?
            .published_closed)
    }
}

#[cfg(test)]
mod tests;
impl FrameTreePublication {
    pub fn publish(self) -> Result<(), HostError> {
        let mut views = self.tree.0.views.lock().map_err(HostError::backend)?;
        if self.revision.sequence <= views.published.sequence {
            return Ok(());
        }
        for node in &self.revision.retired {
            node.0.detached.store(true, Ordering::Release);
        }
        views.published = self.revision;
        Ok(())
    }
}
