//! Optional Page lifecycle publication over the existing ordered topology
//! cursor and Node publication fence. No task, transport, or browser process is
//! installed by this leaf.

use std::error::Error;
use std::sync::{Arc, Mutex};

use pwright_backend_cdp_topology::CdpTopology;
use pwright_cdp::{CdpEvent, generated::page};
use pwright_core::ObjectId;
use pwright_plugin_chromium_frame_observer::{ChromiumFrameEventObserver, FrameObservationResult};

use crate::HostError;
use crate::frame_trees::FrameTrees;

pub(crate) struct Observer {
    trees: Arc<FrameTrees>,
    downstream: Arc<dyn ChromiumFrameEventObserver>,
    documents: Mutex<DocumentEvents>,
}

const PAGE_CAPACITY: usize = 16;
const NATIVE_ID_BYTES: usize = 128;

struct DocumentEvent {
    page: ObjectId,
    loader: NativeId,
    dom_content_loaded: bool,
    load: bool,
}

struct NativeId {
    bytes: [u8; NATIVE_ID_BYTES],
    len: usize,
}

impl NativeId {
    fn new(value: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        if value.len() > NATIVE_ID_BYTES {
            return Err(invalid("bounded loaderId"));
        }
        let mut bytes = [0; NATIVE_ID_BYTES];
        bytes[..value.len()].copy_from_slice(value.as_bytes());
        Ok(Self {
            bytes,
            len: value.len(),
        })
    }

    fn equals(&self, value: &str) -> bool {
        value.as_bytes() == &self.bytes[..self.len]
    }
}

struct DocumentEvents {
    entries: [Option<DocumentEvent>; PAGE_CAPACITY],
}

impl DocumentEvents {
    fn new() -> Self {
        Self {
            entries: std::array::from_fn(|_| None),
        }
    }

    fn seed(&mut self, page: ObjectId, loader: &str) -> FrameObservationResult {
        let loader = NativeId::new(loader)?;
        if let Some(entry) = self
            .entries
            .iter_mut()
            .flatten()
            .find(|entry| entry.page == page)
        {
            entry.loader = loader;
            entry.dom_content_loaded = true;
            entry.load = true;
            return Ok(());
        }
        let slot = self
            .entries
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or_else(|| invalid("Page lifecycle document capacity"))?;
        *slot = Some(DocumentEvent {
            page,
            loader,
            // The initial document is ready before its public Page wrapper is
            // delivered. Late native seed events must not look like a user
            // navigation after the caller installs a listener.
            dom_content_loaded: true,
            load: true,
        });
        Ok(())
    }

    fn select(
        &mut self,
        page: ObjectId,
        loader: &str,
        dom_content_loaded: bool,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.as_ref().is_some_and(|entry| entry.page == page));
        let index = if let Some(index) = index {
            index
        } else {
            self.entries
                .iter()
                .position(Option::is_none)
                .ok_or_else(|| invalid("Page lifecycle document capacity"))?
        };
        let replace = self.entries[index]
            .as_ref()
            .is_none_or(|entry| !entry.loader.equals(loader));
        if replace {
            self.entries[index] = Some(DocumentEvent {
                page,
                loader: NativeId::new(loader)?,
                dom_content_loaded: false,
                load: false,
            });
        }
        let entry = self.entries[index]
            .as_mut()
            .expect("selected Page document slot");
        let selected = if dom_content_loaded {
            !std::mem::replace(&mut entry.dom_content_loaded, true)
        } else {
            !std::mem::replace(&mut entry.load, true)
        };
        Ok(selected)
    }

    fn retain(&mut self, topology: &CdpTopology) {
        for entry in &mut self.entries {
            if entry
                .as_ref()
                .is_some_and(|entry| topology.main_frame_native_id(entry.page).is_none())
            {
                *entry = None;
            }
        }
    }

    fn clear(&mut self) {
        self.entries.fill_with(|| None);
    }
}

impl Observer {
    pub(crate) fn new(
        trees: Arc<FrameTrees>,
        downstream: Arc<dyn ChromiumFrameEventObserver>,
    ) -> Self {
        Self {
            trees,
            downstream,
            documents: Mutex::new(DocumentEvents::new()),
        }
    }
}

fn selected_lifecycle(name: &str) -> Option<bool> {
    match name {
        "DOMContentLoaded" => Some(true),
        "load" => Some(false),
        _ => None,
    }
}

fn invalid(field: &str) -> Box<dyn Error + Send + Sync> {
    Box::new(HostError::Backend(format!(
        "Page lifecycle event is missing {field}"
    )))
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
        self.downstream.initialize_frame(topology, session, frame)?;
        let Some(route) = topology
            .page_session_routes()
            .find(|route| route.session_id() == session)
        else {
            return Ok(());
        };
        if topology.main_frame_native_id(route.object_id()) != Some(frame.id.as_str()) {
            return Ok(());
        }
        self.documents
            .lock()
            .map_err(|_| invalid("document event state"))?
            .seed(route.object_id(), &frame.loader_id)
    }

    fn reconcile(&self, topology: &CdpTopology) -> FrameObservationResult {
        self.downstream.reconcile(topology)?;
        self.documents
            .lock()
            .map_err(|_| invalid("document event state"))?
            .retain(topology);
        Ok(())
    }

    fn event(&self, topology: &CdpTopology, event: &CdpEvent) -> FrameObservationResult {
        self.downstream.event(topology, event)?;
        if event.method == "Page.frameNavigated" {
            let frame_id = event
                .params
                .get("frame")
                .and_then(|value| value.get("id"))
                .and_then(|value| value.as_str())
                .ok_or_else(|| invalid("frame.id"))?;
            let parent = event
                .params
                .get("frame")
                .and_then(|value| value.get("parentId"))
                .and_then(|value| value.as_str());
            if parent.is_none()
                && let Some(route) = topology.page_session_routes().find(|route| {
                    topology.main_frame_native_id(route.object_id()) == Some(frame_id)
                })
            {
                self.trees
                    .publish_main_frame_navigated(route.object_id())
                    .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
            }
            #[cfg(feature = "frame-events")]
            self.publish_frame_navigation(topology, event, frame_id, false)?;
            return Ok(());
        }
        #[cfg(feature = "frame-events")]
        if event.method == "Page.navigatedWithinDocument" {
            let frame_id = event
                .params
                .get("frameId")
                .and_then(|value| value.as_str())
                .ok_or_else(|| invalid("frameId"))?;
            self.publish_frame_navigation(topology, event, frame_id, true)?;
            return Ok(());
        }
        if event.method != "Page.lifecycleEvent" {
            return Ok(());
        }
        let name = event
            .params
            .get("name")
            .and_then(|value| value.as_str())
            .ok_or_else(|| invalid("name"))?;
        let Some(dom_content_loaded) = selected_lifecycle(name) else {
            return Ok(());
        };
        let frame_id = event
            .params
            .get("frameId")
            .and_then(|value| value.as_str())
            .ok_or_else(|| invalid("frameId"))?;
        let loader_id = event
            .params
            .get("loaderId")
            .and_then(|value| value.as_str())
            .ok_or_else(|| invalid("loaderId"))?;
        let session_id = event
            .session_id
            .as_deref()
            .ok_or_else(|| invalid("sessionId"))?;
        let Some(route) = topology
            .page_session_routes()
            .find(|route| route.session_id() == session_id)
        else {
            return Ok(());
        };
        if topology.main_frame_native_id(route.object_id()) != Some(frame_id) {
            return Ok(());
        }
        if !self
            .documents
            .lock()
            .map_err(|_| invalid("document event state"))?
            .select(route.object_id(), loader_id, dom_content_loaded)?
        {
            return Ok(());
        }
        self.trees
            .publish_page_lifecycle(route.object_id(), dom_content_loaded)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }

    fn seal(&self) {
        if let Ok(mut documents) = self.documents.lock() {
            documents.clear();
        }
        self.downstream.seal();
    }
}

#[cfg(feature = "frame-events")]
impl Observer {
    fn publish_frame_navigation(
        &self,
        topology: &CdpTopology,
        event: &CdpEvent,
        native_frame_id: &str,
        include_main: bool,
    ) -> FrameObservationResult {
        let Some(session_id) = event.session_id.as_deref() else {
            return Ok(());
        };
        let Some(route) = topology
            .page_session_routes()
            .find(|route| route.session_id() == session_id)
        else {
            return Ok(());
        };
        let Some(frame) = topology.frame_object_id(native_frame_id) else {
            return Ok(());
        };
        if !include_main
            && topology.main_frame_native_id(route.object_id()) == Some(native_frame_id)
        {
            return Ok(());
        }
        if topology.page_object_id_for_frame(frame) != Some(route.object_id())
            || topology.frame_session_id(frame) != Some(session_id)
        {
            return Ok(());
        }
        self.trees
            .publish_frame_navigated(route.object_id(), frame)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }
}

#[cfg(test)]
mod tests {
    use super::{DocumentEvents, NATIVE_ID_BYTES, selected_lifecycle};
    use pwright_core::ObjectId;

    #[test]
    fn selects_only_public_page_lifecycle_events() {
        // Arrange
        let inputs = ["DOMContentLoaded", "load", "networkIdle", "init"];
        // Act
        let selected = inputs.map(selected_lifecycle);
        // Assert
        assert_eq!(selected, [Some(true), Some(false), None, None]);
    }

    #[test]
    fn initial_document_is_suppressed_and_each_replacement_phase_publishes_once() {
        // Arrange
        let page = ObjectId::new(7);
        let mut documents = DocumentEvents::new();
        documents.seed(page, "initial").expect("seed document");

        // Act
        let outcomes = [
            documents.select(page, "initial", true),
            documents.select(page, "initial", false),
            documents.select(page, "replacement", true),
            documents.select(page, "replacement", true),
            documents.select(page, "replacement", false),
            documents.select(page, "replacement", false),
        ];

        // Assert
        assert_eq!(
            outcomes.map(|outcome| outcome.expect("select event")),
            [false, false, true, false, true, false]
        );
    }

    #[test]
    fn loader_identity_is_bounded_before_retention() {
        // Arrange
        let page = ObjectId::new(9);
        let mut documents = DocumentEvents::new();

        // Act
        let result = documents.select(page, &"x".repeat(NATIVE_ID_BYTES + 1), false);

        // Assert
        assert!(result.is_err());
    }
}
