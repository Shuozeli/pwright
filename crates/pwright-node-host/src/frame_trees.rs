//! Session-local ownership projection. Retained public trees/nodes never own
//! this index, the URL book, the host, or a browser transport.
use crate::frame_metadata::{FrameClosePermit, FrameMetadata};
use crate::frame_tree::{
    FrameBinding, FrameConversion, FrameNode, FrameTree, FrameTreeBinding, FrameTreeFactory,
    FrameTreeLimits, FrameTreeMetrics, ReservedFrameBinding, ReservedPageTree, ReservedTreeBinding,
    Storage,
};
use crate::{FrameUrlSnapshot, HostError};
use pwright_composition_chromium::{
    ChromiumFrameOwnership, ChromiumFrameOwnershipView, ChromiumFrameUrlObserver,
};
use pwright_core::ObjectId;
use std::{
    error::Error,
    sync::{Arc, Mutex},
};

const PAGES: usize = 16;
const FRAMES: usize = 128;
mod close_events;
mod collections;
mod external_close;
mod ready;
use collections::CollectionState;
struct PageEntry {
    id: ObjectId,
    tree: FrameTree,
    close: Option<FrameClosePermit>,
    ready: Option<crate::frame_metadata::CreationDelivery>,
}
struct NodeEntry {
    page: ObjectId,
    node: FrameNode,
}
struct State {
    pages: Vec<PageEntry>,
    nodes: Vec<NodeEntry>,
    closed: bool,
    collections: Option<CollectionState>,
}

pub(crate) struct FrameTrees {
    urls: Arc<FrameMetadata>,
    factory: FrameTreeFactory,
    state: Mutex<State>,
    _storage: Arc<Storage>,
}
pub(crate) struct ReservedPage {
    tree: ReservedPageTree,
    binding: ReservedFrameBinding,
    tree_binding: ReservedTreeBinding,
    ready: Option<crate::frame_metadata::CreationDelivery>,
    close: FrameClosePermit,
}

impl FrameTrees {
    pub fn new(urls: Arc<FrameMetadata>) -> Result<Self, HostError> {
        let factory = FrameTreeFactory::new(FrameTreeLimits {
            states: 256,
            frames_per_page: FRAMES,
            revisions: 256,
            bytes: 8 * 1024 * 1024,
        })?;
        // Fixed active vectors plus one mutex-serialized reconciliation's
        // borrowed-ID and desired-node vectors, reserved before allocation.
        let index_bytes = std::mem::size_of::<Self>()
            + 2 * std::mem::size_of::<usize>()
            + PAGES * std::mem::size_of::<PageEntry>()
            + FRAMES
                * (std::mem::size_of::<NodeEntry>()
                    + std::mem::size_of::<ChromiumFrameOwnership>()
                    + std::mem::size_of::<FrameNode>());
        let bytes = urls
            .publication_storage_bytes()?
            .checked_add(index_bytes)
            .ok_or_else(|| HostError::ResourceExhausted("Frame index storage overflow".into()))?;
        let storage = Arc::new(factory.reserve_index(bytes)?);
        urls.admit_page_closure(storage.clone())?;
        Ok(Self {
            urls,
            factory,
            state: Mutex::new(State {
                pages: Vec::with_capacity(PAGES),
                nodes: Vec::with_capacity(FRAMES),
                closed: false,
                collections: None,
            }),
            _storage: storage,
        })
    }

    pub fn get(&self, page: ObjectId) -> Result<FrameTree, HostError> {
        self.state
            .lock()
            .map_err(HostError::backend)?
            .pages
            .iter()
            .find(|entry| entry.id == page)
            .map(|entry| entry.tree.clone())
            .ok_or_else(|| HostError::Backend("Page Frame tree is not available".into()))
    }
    pub fn metrics(&self) -> Result<FrameTreeMetrics, HostError> {
        self.factory.metrics()
    }
    pub fn reserve_conversion(&self, bytes: usize) -> Result<FrameConversion, HostError> {
        self.factory.reserve_conversion(bytes)
    }

    pub fn reserve_page(&self, url: FrameUrlSnapshot) -> Result<ReservedPage, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        if state.pages.len() == PAGES || state.nodes.len() == FRAMES {
            return Err(HostError::ResourceExhausted(
                "Frame index capacity exhausted".into(),
            ));
        }
        // Only the serial lifecycle driver creates Pages; topology observations
        // remain paused through activation or compensation.
        Ok(ReservedPage {
            close: self.urls.publications().reserve_close()?,
            tree: self.factory.reserve_page(url)?,
            binding: self.factory.reserve_binding()?,
            tree_binding: self.factory.reserve_tree_binding()?,
            ready: self.reserve_ready_delivery(&state)?,
        })
    }

    pub fn activate_page(
        &self,
        reservation: ReservedPage,
        page: ObjectId,
        main: ObjectId,
    ) -> Result<(FrameTree, FrameBinding, FrameTreeBinding), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        if state.pages.len() == PAGES || state.nodes.len() == FRAMES {
            return Err(HostError::ResourceExhausted(
                "Frame index capacity exhausted".into(),
            ));
        }
        if state.pages.iter().any(|entry| entry.id == page)
            || state.nodes.iter().any(|entry| entry.node.id() == main)
        {
            return Err(HostError::Backend(
                "Created Page Frame identity is already indexed".into(),
            ));
        }
        let (node, tree) = reservation.tree.activate(main);
        let binding = reservation.binding.activate(node.clone());
        let tree_binding = reservation.tree_binding.activate(tree.clone());
        state.nodes.push(NodeEntry { page, node });
        state.pages.push(PageEntry {
            id: page,
            tree: tree.clone(),
            close: Some(reservation.close),
            ready: reservation.ready,
        });
        Ok((tree, binding, tree_binding))
    }

    pub fn clear(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.closed = true;
        let terminal = Self::terminal_publication(&mut state);
        let pages = state
            .pages
            .iter_mut()
            .map(|page| (&page.tree, &mut page.close));
        if let Some((terminal, view)) = terminal {
            (*terminal).stage(&self.urls.publications(), view, pages);
        } else {
            self.urls.publications().close_for_teardown(pages);
        }
        state.pages.clear();
        state.nodes.clear();
        state.collections = None;
    }

    pub(crate) fn reconcile(
        &self,
        rows: impl Iterator<Item = ChromiumFrameOwnership>,
        contains_page: impl Fn(ObjectId) -> bool,
    ) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        let mut snapshot = Vec::with_capacity(FRAMES);
        for row in rows {
            if snapshot.len() == FRAMES {
                return Err(HostError::ResourceExhausted(
                    "Frame projection exceeds index capacity".into(),
                ));
            }
            snapshot.push(row);
        }
        snapshot.sort_unstable_by_key(|row| (row.page, row.parent.is_some(), row.frame));
        if snapshot.iter().filter(|row| row.parent.is_none()).count() > PAGES {
            return Err(HostError::ResourceExhausted(
                "Page projection exceeds index capacity".into(),
            ));
        }
        for page in &mut state.pages {
            if !contains_page(page.id) && page.close.is_some() {
                self.urls
                    .publications()
                    .stage_close(&page.tree, &mut page.close)?;
            }
        }
        state.pages.retain(|entry| contains_page(entry.id));
        // Retirement tokens preserve removed nodes independently of this index.
        state
            .nodes
            .retain(|entry| snapshot.iter().any(|row| row.frame == entry.node.id()));
        for row in &snapshot {
            if let Some(prior) = state
                .nodes
                .iter()
                .find(|entry| entry.node.id() == row.frame)
            {
                if prior.page != row.page || prior.node.parent_id() != row.parent {
                    return Err(HostError::Backend(
                        "Retained Frame ownership changed identity".into(),
                    ));
                }
            } else {
                let url = self.urls.ensure_attached(row.frame)?;
                let node = self.factory.frame(row.frame, row.parent, url)?;
                state.nodes.push(NodeEntry {
                    page: row.page,
                    node,
                });
            }
        }
        for row in snapshot.iter().filter(|row| row.parent.is_none()) {
            if !state.pages.iter().any(|entry| entry.id == row.page) {
                if state.pages.len() == PAGES {
                    return Err(HostError::ResourceExhausted("Page index is full".into()));
                }
                let root = state
                    .nodes
                    .iter()
                    .find(|entry| entry.node.id() == row.frame)
                    .ok_or_else(|| {
                        HostError::Backend("Projected main Frame was not indexed".into())
                    })?
                    .node
                    .clone();
                let close = self.urls.publications().reserve_close()?;
                let tree = self.factory.page(root)?;
                let ready = self.reserve_ready_delivery(&state)?;
                state.pages.push(PageEntry {
                    id: row.page,
                    tree,
                    close: Some(close),
                    ready,
                });
            }
        }
        let mut desired = Vec::with_capacity(FRAMES);
        for page in &state.pages {
            desired.clear();
            for row in snapshot.iter().filter(|row| row.page == page.id) {
                let node = state
                    .nodes
                    .iter()
                    .find(|entry| entry.node.id() == row.frame)
                    .ok_or_else(|| HostError::Backend("Projected Frame was not indexed".into()))?;
                desired.push(node.node.clone());
            }
            if !page.tree.matches_native(&desired)? {
                self.urls.publications().stage_tree(&page.tree, &desired)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod live;
#[cfg(test)]
mod terminal_live;
#[cfg(test)]
mod tests;

impl ChromiumFrameUrlObserver for FrameTrees {
    fn reconcile_committed(
        &self,
        publication: pwright_composition_chromium::ChromiumCommittedFrameOwnership<'_>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.external_disposals(publication.effects())?;
        self.reconcile_collections(publication.ownership())?;
        self.publish_ready_causes(publication.effects())?;
        Ok(())
    }

    fn update_navigation(
        &self,
        frame: ObjectId,
        url: &str,
        fragment: Option<&str>,
        name: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.urls.update_navigation(frame, url, fragment, name)
    }

    fn update(
        &self,
        frame: ObjectId,
        url: &str,
        fragment: Option<&str>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.urls.update(frame, url, fragment)
    }
    fn retain_live(
        &self,
        is_live: &dyn Fn(ObjectId) -> bool,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.urls.retain_live(is_live)
    }
    fn reconcile_ownership(
        &self,
        view: ChromiumFrameOwnershipView<'_>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.reconcile_collections(view)?;
        Ok(())
    }
}
