//! Optional real Chromium file-chooser events and exact-node mutation owners.
//!
//! Interception is installed on each selected target session. The topology
//! cursor publishes `Page.fileChooserOpened` through the existing bounded Page
//! event fence; foreign wrappers retain only an opaque Rust owner for the exact
//! CDP session, utility world, and backend node reported by Chromium.

use std::error::Error;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};

use pwright_backend_cdp_topology::{CdpContextOwnership, CdpTopology};
use pwright_cdp::{CdpClient, CdpEvent, CdpSession, generated::page};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequest,
    ResourceRequirements, ResourceScope,
};
use pwright_plugin_chromium_frame_observer::{ChromiumFrameEventObserver, FrameObservationResult};
use pwright_plugin_chromium_session_initialization::{
    ChromiumSessionInitializationFuture, ChromiumSessionInitializationTarget,
    ChromiumSessionInitializer,
};

use crate::file_input::{Installation as FileInputInstallation, MemoryFile, PreparedFileInput};
use crate::frame_trees::FrameTrees;
use crate::{HostError, PageFileChooser};

const MAX_LIVE_FILE_CHOOSERS: u64 = 16;
const OWNER_BYTES: u64 = 2048;
const INSTALLATION_BYTES: u64 = 1024;
const INSTALLATION_SCOPE: ResourceScope = ResourceScope::extension(175, 1);

pub(crate) const fn requirements() -> ResourceRequirements {
    ResourceRequirements::none()
        .with_class(ResourceClass::LiveObject, MAX_LIVE_FILE_CHOOSERS, 1)
        .with_class(
            ResourceClass::BackendStateBytes,
            MAX_LIVE_FILE_CHOOSERS * OWNER_BYTES + INSTALLATION_BYTES,
            OWNER_BYTES,
        )
        .with_metadata(
            MAX_LIVE_FILE_CHOOSERS as usize + 1,
            MAX_LIVE_FILE_CHOOSERS as usize + 1,
        )
}

pub struct FileChooserAction {
    connection: Weak<pwright_cdp::CdpConnection>,
    session_id: Arc<str>,
    target_id: Arc<str>,
    utility_world: Arc<str>,
    execution_context_id: i64,
    backend_node_id: i64,
    multiple: bool,
    files: Weak<FileInputInstallation>,
    _owner: ResourceLease,
}

impl std::fmt::Debug for FileChooserAction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FileChooserAction")
            .field("execution_context_id", &self.execution_context_id)
            .field("backend_node_id", &self.backend_node_id)
            .finish_non_exhaustive()
    }
}

impl FileChooserAction {
    fn validate_item_count(&self, count: usize) -> Result<(), HostError> {
        if count > 1 && !self.multiple {
            return Err(HostError::backend(
                pwright_file_input_model::FileInputError::MultipleFiles,
            ));
        }
        Ok(())
    }

    fn files(&self) -> Result<Arc<FileInputInstallation>, HostError> {
        self.files.upgrade().ok_or(HostError::TargetClosed)
    }

    fn client(&self) -> Result<Arc<dyn CdpClient>, HostError> {
        let connection = self.connection.upgrade().ok_or(HostError::TargetClosed)?;
        Ok(Arc::new(CdpSession::new(
            connection,
            self.session_id.to_string(),
            self.target_id.to_string(),
        )))
    }

    pub fn prepare_payloads(
        &self,
        files: &[MemoryFile<'_>],
    ) -> Result<PreparedFileInput, HostError> {
        self.validate_item_count(files.len())?;
        self.files()?.prepare(files)
    }

    pub async fn set_files(&self, input: PreparedFileInput) -> Result<(), HostError> {
        let files = self.files()?;
        files
            .prepare_direct_upload(
                self.client()?,
                self.utility_world.clone(),
                self.execution_context_id,
                self.backend_node_id,
                input,
            )?
            .await
    }

    pub async fn set_paths(&self, base: &Path, paths: &[&str]) -> Result<(), HostError> {
        self.validate_item_count(paths.len())?;
        let files = self.files()?;
        #[cfg(feature = "file-chooser-native-paths")]
        {
            return files
                .set_colocated_paths(
                    self.client()?,
                    self.utility_world.clone(),
                    self.execution_context_id,
                    self.backend_node_id,
                    base,
                    paths,
                )
                .await;
        }
        #[cfg(not(feature = "file-chooser-native-paths"))]
        let input = files.prepare_paths(base, paths).await?;
        #[cfg(not(feature = "file-chooser-native-paths"))]
        files
            .prepare_direct_upload(
                self.client()?,
                self.utility_world.clone(),
                self.execution_context_id,
                self.backend_node_id,
                input,
            )?
            .await
    }
}

pub(crate) struct Installation {
    resources: ProductionResourceLedger,
    files: Weak<FileInputInstallation>,
    next_id: AtomicU64,
    _storage: ResourceLease,
}

struct ChooserRoute {
    connection: Weak<pwright_cdp::CdpConnection>,
    session_id: Arc<str>,
    target_id: Arc<str>,
    utility_world: Arc<str>,
    execution_context_id: i64,
    backend_node_id: i64,
}

impl Installation {
    pub(crate) fn new(
        resources: &ProductionResourceLedger,
        files: &Arc<FileInputInstallation>,
    ) -> Result<Arc<Self>, HostError> {
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
            files: Arc::downgrade(files),
            next_id: AtomicU64::new(1),
            _storage: storage,
        }))
    }

    fn create(
        &self,
        page: ObjectId,
        multiple: bool,
        route: ChooserRoute,
    ) -> Result<PageFileChooser, HostError> {
        let id = self
            .next_id
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .map_err(|_| HostError::ResourceExhausted("file chooser ID space exhausted".into()))?;
        let owner = self
            .resources
            .claim(requirements())
            .map_err(HostError::backend)?
            .reserve_many(
                ResourceScope::Object(page),
                &[
                    ResourceRequest::new(ResourceClass::LiveObject, 1),
                    ResourceRequest::new(ResourceClass::BackendStateBytes, OWNER_BYTES),
                ],
            )
            .map_err(|error| {
                HostError::ResourceExhausted(format!(
                    "Page file chooser owner capacity exceeded ({MAX_LIVE_FILE_CHOOSERS}): {error}"
                ))
            })?;
        PageFileChooser::new(
            id,
            page,
            multiple,
            Arc::new(FileChooserAction {
                connection: route.connection,
                session_id: route.session_id,
                target_id: route.target_id,
                utility_world: route.utility_world,
                execution_context_id: route.execution_context_id,
                backend_node_id: route.backend_node_id,
                multiple,
                files: self.files.clone(),
                _owner: owner,
            }),
        )
    }
}

pub(crate) struct Initializer<Downstream> {
    downstream: Downstream,
    connection: Arc<pwright_cdp::CdpConnection>,
}

impl<Downstream> Initializer<Downstream> {
    pub(crate) fn new(downstream: Downstream, connection: Arc<pwright_cdp::CdpConnection>) -> Self {
        Self {
            downstream,
            connection,
        }
    }

    async fn enable(
        &self,
        session_id: &str,
        target_id: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Playwright only intercepts while the language client has a live
        // filechooser listener or waiter. Starting disabled also prevents a
        // client that never subscribes from swallowing the platform chooser.
        CdpSession::new(
            Arc::clone(&self.connection),
            session_id.to_owned(),
            target_id.to_owned(),
        )
        .page_set_intercept_file_chooser_dialog(false)
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
        ownership: CdpContextOwnership,
        is_page: bool,
    ) -> ChromiumSessionInitializationFuture<'a> {
        Box::pin(async move {
            let downstream = self
                .downstream
                .initialize(session_id, target_id, ownership, is_page);
            let enable = self.enable(session_id, target_id);
            tokio::try_join!(downstream, enable).map(|_| ())
        })
    }

    fn initialize_scoped<'a>(
        &'a self,
        target: ChromiumSessionInitializationTarget<'a>,
    ) -> ChromiumSessionInitializationFuture<'a> {
        Box::pin(async move {
            let downstream = self.downstream.initialize_scoped(target);
            let enable = self.enable(target.session_id, target.target_id);
            tokio::try_join!(downstream, enable).map(|_| ())
        })
    }
}

pub(crate) struct Observer {
    trees: Arc<FrameTrees>,
    installation: Arc<Installation>,
    downstream: Arc<dyn ChromiumFrameEventObserver>,
    connection: Arc<pwright_cdp::CdpConnection>,
}

impl Observer {
    pub(crate) fn new(
        trees: Arc<FrameTrees>,
        installation: Arc<Installation>,
        downstream: Arc<dyn ChromiumFrameEventObserver>,
        connection: Arc<pwright_cdp::CdpConnection>,
    ) -> Self {
        Self {
            trees,
            installation,
            downstream,
            connection,
        }
    }
}

fn invalid(field: &str) -> Box<dyn Error + Send + Sync> {
    Box::new(HostError::Backend(format!(
        "Page.fileChooserOpened is missing {field}"
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
        self.downstream.initialize_frame(topology, session, frame)
    }

    fn reconcile(&self, topology: &CdpTopology) -> FrameObservationResult {
        self.downstream.reconcile(topology)
    }

    fn event(&self, topology: &CdpTopology, event: &CdpEvent) -> FrameObservationResult {
        self.downstream.event(topology, event)?;
        if event.method != "Page.fileChooserOpened" {
            return Ok(());
        }
        let session_id = event
            .session_id
            .as_deref()
            .ok_or_else(|| invalid("sessionId"))?;
        let params: page::FileChooserOpenedEvent =
            serde_json::from_value(event.params.clone()).map_err(|_| invalid("valid params"))?;
        let backend_node_id = params
            .backend_node_id
            .filter(|value| *value > 0)
            .ok_or_else(|| invalid("backendNodeId"))?;
        let multiple = match params.mode.as_str() {
            "selectSingle" => false,
            "selectMultiple" => true,
            _ => return Err(invalid("supported mode")),
        };
        let Some(frame) = topology.frame_object_id(&params.frame_id) else {
            return Ok(());
        };
        let Some(page) = topology.page_object_id_for_frame(frame) else {
            return Ok(());
        };
        let Some(route) = topology
            .frame_utility_execution_routes()
            .find(|route| route.object_id() == frame && route.session_id() == session_id)
        else {
            return Ok(());
        };
        let chooser = self
            .installation
            .create(
                page,
                multiple,
                ChooserRoute {
                    connection: Arc::downgrade(&self.connection),
                    session_id: Arc::from(route.session_id()),
                    target_id: Arc::from(route.target_id()),
                    utility_world: Arc::from(route.unique_context_id()),
                    execution_context_id: route.execution_context_id(),
                    backend_node_id,
                },
            )
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
        let payload_bytes = chooser.storage_bytes();
        self.trees
            .publish_page_file_chooser(page, payload_bytes, || Ok(chooser))
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }

    fn seal(&self) {
        self.downstream.seal();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_core::{ProductionResourceLedger, ResourceClass, ResourceLimits};

    fn resources() -> ProductionResourceLedger {
        let required = requirements();
        let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
        for class in ResourceClass::ALL {
            let bound = required.class(class);
            limits = limits.with_class(class, bound.global, bound.per_scope);
        }
        ProductionResourceLedger::new(limits, required).unwrap()
    }

    fn installation(resources: &ProductionResourceLedger) -> Arc<Installation> {
        let storage = resources
            .claim(requirements())
            .unwrap()
            .reserve(
                INSTALLATION_SCOPE,
                ResourceClass::BackendStateBytes,
                INSTALLATION_BYTES,
            )
            .unwrap();
        Arc::new(Installation {
            resources: resources.clone(),
            files: Weak::new(),
            next_id: AtomicU64::new(1),
            _storage: storage,
        })
    }

    fn route() -> ChooserRoute {
        ChooserRoute {
            connection: Weak::new(),
            session_id: Arc::from("session"),
            target_id: Arc::from("target"),
            utility_world: Arc::from("utility-world"),
            execution_context_id: 7,
            backend_node_id: 9,
        }
    }

    #[test]
    fn chooser_owners_are_bounded_and_reclaimed() {
        // Arrange
        let resources = resources();
        let installation = installation(&resources);
        let mut owners = Vec::new();
        for id in 1..=MAX_LIVE_FILE_CHOOSERS {
            owners.push(
                installation
                    .create(ObjectId::new(id), id % 2 == 0, route())
                    .unwrap(),
            );
        }

        // Act
        let exhausted = installation
            .create(ObjectId::new(MAX_LIVE_FILE_CHOOSERS + 1), false, route())
            .unwrap_err();
        owners.pop();
        let replacement = installation
            .create(ObjectId::new(MAX_LIVE_FILE_CHOOSERS + 1), true, route())
            .unwrap();

        // Assert
        assert!(matches!(exhausted, HostError::ResourceExhausted(_)));
        assert!(replacement.is_multiple());
    }

    #[test]
    fn single_file_chooser_rejects_multiple_items_before_source_work() {
        // Arrange
        let resources = resources();
        let installation = installation(&resources);
        let chooser = installation
            .create(ObjectId::new(1), false, route())
            .unwrap();

        // Act
        let error = chooser.action().validate_item_count(2).unwrap_err();

        // Assert
        assert_eq!(
            error.to_string(),
            "Non-multiple file input can only accept single file"
        );
    }
}
