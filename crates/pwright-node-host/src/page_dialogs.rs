//! Optional Page dialog events and exact-session dialog actions.
//!
//! The topology cursor observes Chromium dialog openings and publishes them on
//! the existing ordered Page-event fence. Each public dialog carries one
//! opaque Rust action owner for the exact CDP session that opened it. There is
//! no language-owned native ID table, polling loop, or unbounded side queue.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};

use pwright_backend_cdp_topology::CdpTopology;
use pwright_cdp::{CdpClient, CdpEvent, CdpSession, generated::page};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequest,
    ResourceRequirements, ResourceScope,
};
use pwright_plugin_chromium_frame_observer::{ChromiumFrameEventObserver, FrameObservationResult};

use crate::frame_trees::FrameTrees;
use crate::{HostError, PageDialog};

const MAX_LIVE_DIALOGS: u64 = 16;
const OWNER_BYTES: u64 = 1024;
const INSTALLATION_BYTES: u64 = 2048;
const INSTALLATION_SCOPE: ResourceScope = ResourceScope::extension(174, 1);
pub(crate) const MAX_DIALOG_MESSAGE_BYTES: usize = 16 * 1024;
pub(crate) const MAX_DIALOG_DEFAULT_BYTES: usize = 16 * 1024;

pub(crate) const fn requirements() -> ResourceRequirements {
    ResourceRequirements::none()
        .with_class(ResourceClass::LiveObject, MAX_LIVE_DIALOGS, 1)
        .with_class(
            ResourceClass::BackendStateBytes,
            MAX_LIVE_DIALOGS * OWNER_BYTES + INSTALLATION_BYTES,
            INSTALLATION_BYTES,
        )
        .with_metadata(MAX_LIVE_DIALOGS as usize + 1, MAX_LIVE_DIALOGS as usize + 1)
}

#[derive(Default)]
struct ActiveDialogs {
    pages: Mutex<BTreeMap<ObjectId, ActiveDialog>>,
}

struct ActiveDialog {
    id: u64,
    action: Weak<DialogAction>,
}

impl ActiveDialogs {
    fn clear(&self, page: ObjectId, id: u64) {
        if let Ok(mut pages) = self.pages.lock()
            && pages.get(&page).is_some_and(|dialog| dialog.id == id)
        {
            pages.remove(&page);
        }
    }
}

struct ActiveDialogLease {
    registry: Weak<ActiveDialogs>,
    page: ObjectId,
    id: u64,
}

impl Drop for ActiveDialogLease {
    fn drop(&mut self) {
        if let Some(registry) = self.registry.upgrade() {
            registry.clear(self.page, self.id);
        }
    }
}

struct ActionOwner {
    client: Arc<dyn CdpClient>,
    _owner: ResourceLease,
    _active: ActiveDialogLease,
}

enum DialogActionState {
    Ready(ActionOwner),
    Handled,
    Closed,
}

/// Opaque, single-use native owner retained by a foreign `Dialog` wrapper.
pub struct DialogAction {
    id: u64,
    state: Mutex<DialogActionState>,
}

impl fmt::Debug for DialogAction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DialogAction")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl DialogAction {
    pub async fn handle(&self, accept: bool, prompt_text: Option<&str>) -> Result<(), HostError> {
        if prompt_text.is_some_and(|value| value.len() > MAX_DIALOG_DEFAULT_BYTES) {
            return Err(HostError::InvalidArgument(format!(
                "dialog prompt text exceeds {MAX_DIALOG_DEFAULT_BYTES} UTF-8 bytes"
            )));
        }
        let owner = {
            let mut state = self.state.lock().map_err(HostError::backend)?;
            match std::mem::replace(&mut *state, DialogActionState::Handled) {
                DialogActionState::Ready(owner) => owner,
                DialogActionState::Handled => {
                    *state = DialogActionState::Handled;
                    return Err(HostError::InvalidArgument(
                        "JavaScript dialog has already been handled".into(),
                    ));
                }
                DialogActionState::Closed => {
                    *state = DialogActionState::Closed;
                    return Err(HostError::TargetClosed);
                }
            }
        };
        owner
            .client
            .page_handle_java_script_dialog(accept, prompt_text)
            .await
            .map_err(HostError::backend)
    }

    fn revoke(&self) -> Result<(), HostError> {
        let owner = {
            let mut state = self.state.lock().map_err(HostError::backend)?;
            match std::mem::replace(&mut *state, DialogActionState::Closed) {
                DialogActionState::Ready(owner) => Some(owner),
                previous => {
                    *state = previous;
                    None
                }
            }
        };
        drop(owner);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogKind {
    Alert,
    Confirm,
    Prompt,
    BeforeUnload,
}

impl DialogKind {
    const fn from_cdp(value: page::DialogType) -> Self {
        match value {
            page::DialogType::Alert => Self::Alert,
            page::DialogType::Confirm => Self::Confirm,
            page::DialogType::Prompt => Self::Prompt,
            page::DialogType::Beforeunload => Self::BeforeUnload,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Alert => "alert",
            Self::Confirm => "confirm",
            Self::Prompt => "prompt",
            Self::BeforeUnload => "beforeunload",
        }
    }
}

pub struct Installation {
    resources: ProductionResourceLedger,
    active: Arc<ActiveDialogs>,
    next_id: AtomicU64,
    _storage: ResourceLease,
}

impl Installation {
    pub(crate) fn new(resources: &ProductionResourceLedger) -> Result<Arc<Self>, HostError> {
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
            active: Arc::new(ActiveDialogs::default()),
            next_id: AtomicU64::new(1),
            _storage: storage,
        }))
    }

    fn create(
        &self,
        page: ObjectId,
        client: Arc<dyn CdpClient>,
        params: page::JavascriptDialogOpeningEvent,
    ) -> Result<Option<PageDialog>, HostError> {
        if params.message.len() > MAX_DIALOG_MESSAGE_BYTES {
            return Err(HostError::ResourceExhausted(format!(
                "dialog message exceeds {MAX_DIALOG_MESSAGE_BYTES} UTF-8 bytes"
            )));
        }
        let default_value = params.default_prompt.unwrap_or_default();
        if default_value.len() > MAX_DIALOG_DEFAULT_BYTES {
            return Err(HostError::ResourceExhausted(format!(
                "dialog default value exceeds {MAX_DIALOG_DEFAULT_BYTES} UTF-8 bytes"
            )));
        }
        let mut active = self.active.pages.lock().map_err(HostError::backend)?;
        if active.contains_key(&page) {
            return Ok(None);
        }
        let id = self
            .next_id
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .map_err(|_| HostError::ResourceExhausted("dialog ID space exhausted".into()))?;
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
                    "Page dialog owner capacity exceeded ({MAX_LIVE_DIALOGS}): {error}"
                ))
            })?;
        let action = Arc::new(DialogAction {
            id,
            state: Mutex::new(DialogActionState::Ready(ActionOwner {
                client,
                _owner: owner,
                _active: ActiveDialogLease {
                    registry: Arc::downgrade(&self.active),
                    page,
                    id,
                },
            })),
        });
        active.insert(
            page,
            ActiveDialog {
                id,
                action: Arc::downgrade(&action),
            },
        );
        drop(active);
        PageDialog::new(
            id,
            page,
            DialogKind::from_cdp(params.r#type),
            params.message,
            default_value,
            action,
        )
        .map(Some)
    }

    fn revoke_page(&self, page: ObjectId) -> Result<(), HostError> {
        let action = self
            .active
            .pages
            .lock()
            .map_err(HostError::backend)?
            .remove(&page)
            .and_then(|dialog| dialog.action.upgrade());
        if let Some(action) = action {
            action.revoke()?;
        }
        Ok(())
    }

    pub(crate) fn close(&self) -> Result<(), HostError> {
        let dialogs = {
            let mut pages = self.active.pages.lock().map_err(HostError::backend)?;
            std::mem::take(&mut *pages)
        };
        for dialog in dialogs.into_values() {
            if let Some(action) = dialog.action.upgrade() {
                action.revoke()?;
            }
        }
        Ok(())
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
        "Page.javascriptDialogOpening is missing {field}"
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
        if event.method == "Page.javascriptDialogClosed" {
            let session_id = event
                .session_id
                .as_deref()
                .ok_or_else(|| invalid("sessionId"))?;
            let page = topology
                .page_session_routes()
                .find(|route| route.session_id() == session_id)
                .map(|route| route.object_id())
                .or_else(|| {
                    topology
                        .target_session_route(session_id)
                        .and_then(|route| topology.page_object_id(route.page_target_id()))
                });
            if let Some(page) = page {
                self.installation
                    .revoke_page(page)
                    .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
            }
            return Ok(());
        }
        if event.method != "Page.javascriptDialogOpening" {
            return Ok(());
        }
        let session_id = event
            .session_id
            .as_deref()
            .ok_or_else(|| invalid("sessionId"))?;
        let params: page::JavascriptDialogOpeningEvent =
            serde_json::from_value(event.params.clone()).map_err(|_| invalid("valid params"))?;
        // Chromium can report the same Page through an auto-attached observer
        // session while explicit Page creation retains a separate command
        // session. The event session remains the only correct dialog-action
        // destination; frame ownership supplies its portable Page and target.
        let frame_page = topology
            .frame_object_id(&params.frame_id)
            .and_then(|frame| topology.page_object_id_for_frame(frame));
        let session_page_route = topology
            .page_session_routes()
            .find(|route| route.session_id() == session_id);
        let target_route = topology.target_session_route(session_id);
        let page = frame_page
            .or_else(|| session_page_route.map(|route| route.object_id()))
            .or_else(|| {
                target_route.and_then(|route| topology.page_object_id(route.page_target_id()))
            });
        let Some(page) = page else {
            return Ok(());
        };
        let page_route = session_page_route
            .filter(|route| route.object_id() == page)
            .or_else(|| {
                topology
                    .page_session_routes()
                    .find(|route| route.object_id() == page)
            });
        let target_id = page_route
            .map(|route| route.target_id())
            .or_else(|| target_route.map(|route| route.target_id()));
        let Some(target_id) = target_id else {
            return Ok(());
        };
        let client: Arc<dyn CdpClient> = Arc::new(CdpSession::new(
            Arc::clone(&self.connection),
            session_id.to_owned(),
            target_id.to_owned(),
        ));
        let Some(dialog) = self
            .installation
            .create(page, client, params)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?
        else {
            return Ok(());
        };
        let payload_bytes = dialog
            .storage_bytes()
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
        self.trees
            .publish_page_dialog(page, payload_bytes, || Ok(dialog))
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }

    fn seal(&self) {
        self.downstream.seal();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_core::{ProductionResourceLedger, ResourceClass, ResourceLimits};
    use pwright_fake::FakeCdpClient;

    use super::*;

    fn resources() -> ProductionResourceLedger {
        let required = requirements();
        let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
        for class in ResourceClass::ALL {
            let bound = required.class(class);
            limits = limits.with_class(class, bound.global, bound.per_scope);
        }
        ProductionResourceLedger::new(limits, required).unwrap()
    }

    fn params(kind: page::DialogType) -> page::JavascriptDialogOpeningEvent {
        page::JavascriptDialogOpeningEvent {
            url: "https://example.test/".into(),
            frame_id: "frame-1".into(),
            message: "question?".into(),
            r#type: kind,
            has_browser_handler: true,
            default_prompt: Some("yes.".into()),
        }
    }

    #[tokio::test]
    async fn action_sends_exact_prompt_once() {
        // Arrange
        let installation = Installation::new(&resources()).unwrap();
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let dialog = installation
            .create(
                ObjectId::new(7),
                fake.clone(),
                params(page::DialogType::Prompt),
            )
            .unwrap()
            .unwrap();
        let action = dialog.action();

        // Act
        action.handle(true, Some("answer!")).await.unwrap();
        let duplicate = action.handle(false, None).await.unwrap_err();

        // Assert
        assert_eq!(
            fake.calls(),
            [(
                "Page.handleJavaScriptDialog".into(),
                vec![serde_json::json!({
                    "accept": true,
                    "promptText": "answer!",
                })]
            )]
        );
        assert!(duplicate.to_string().contains("already been handled"));
    }

    #[tokio::test]
    async fn duplicate_session_opening_keeps_one_active_dialog_per_page() {
        // Arrange
        let installation = Installation::new(&resources()).unwrap();
        let fake: Arc<dyn CdpClient> = Arc::new(FakeCdpClient::from_html(""));
        let first = installation
            .create(
                ObjectId::new(7),
                fake.clone(),
                params(page::DialogType::Alert),
            )
            .unwrap()
            .unwrap();

        // Act
        let duplicate = installation
            .create(
                ObjectId::new(7),
                fake.clone(),
                params(page::DialogType::Alert),
            )
            .unwrap();
        first.action().handle(false, None).await.unwrap();
        let after_close = installation
            .create(ObjectId::new(7), fake, params(page::DialogType::Alert))
            .unwrap();

        // Assert
        assert!(duplicate.is_none());
        assert!(after_close.is_some());
    }

    #[tokio::test]
    async fn dialog_close_revokes_the_action_and_releases_its_client() {
        // Arrange
        let installation = Installation::new(&resources()).unwrap();
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let client: Arc<dyn CdpClient> = fake.clone();
        let dialog = installation
            .create(ObjectId::new(7), client, params(page::DialogType::Prompt))
            .unwrap()
            .unwrap();
        let action = dialog.action();
        assert_eq!(Arc::strong_count(&fake), 2);

        // Act
        installation.revoke_page(ObjectId::new(7)).unwrap();
        let error = action.handle(true, Some("answer!")).await.unwrap_err();

        // Assert
        assert!(matches!(error, HostError::TargetClosed));
        assert_eq!(Arc::strong_count(&fake), 1);
        assert!(fake.calls().is_empty());
    }

    #[tokio::test]
    async fn installation_close_revokes_every_retained_dialog_action() {
        // Arrange
        let installation = Installation::new(&resources()).unwrap();
        let fake = Arc::new(FakeCdpClient::from_html(""));
        let first = installation
            .create(
                ObjectId::new(7),
                fake.clone(),
                params(page::DialogType::Alert),
            )
            .unwrap()
            .unwrap()
            .action();
        let second = installation
            .create(
                ObjectId::new(8),
                fake.clone(),
                params(page::DialogType::Confirm),
            )
            .unwrap()
            .unwrap()
            .action();
        assert_eq!(Arc::strong_count(&fake), 3);

        // Act
        installation.close().unwrap();
        let first_error = first.handle(false, None).await.unwrap_err();
        let second_error = second.handle(true, None).await.unwrap_err();

        // Assert
        assert!(matches!(first_error, HostError::TargetClosed));
        assert!(matches!(second_error, HostError::TargetClosed));
        assert_eq!(Arc::strong_count(&fake), 1);
        assert!(fake.calls().is_empty());
    }

    #[test]
    fn public_descriptor_preserves_playwright_dialog_fields() {
        // Arrange
        let installation = Installation::new(&resources()).unwrap();
        let fake: Arc<dyn CdpClient> = Arc::new(FakeCdpClient::from_html(""));

        // Act
        let dialog = installation
            .create(
                ObjectId::new(7),
                fake,
                params(page::DialogType::Beforeunload),
            )
            .unwrap()
            .unwrap();

        // Assert
        assert_eq!(dialog.kind().as_str(), "beforeunload");
        assert_eq!(dialog.message(), "question?");
        assert_eq!(dialog.default_value(), "yes.");
    }

    #[test]
    fn owner_capacity_is_bounded_before_retaining_an_extra_session() {
        // Arrange
        let installation = Installation::new(&resources()).unwrap();
        let fake: Arc<dyn CdpClient> = Arc::new(FakeCdpClient::from_html(""));
        let mut retained = Vec::new();
        for page in 1..=MAX_LIVE_DIALOGS {
            retained.push(
                installation
                    .create(
                        ObjectId::new(page),
                        fake.clone(),
                        params(page::DialogType::Alert),
                    )
                    .unwrap()
                    .unwrap(),
            );
        }

        // Act
        let error = installation
            .create(
                ObjectId::new(MAX_LIVE_DIALOGS + 1),
                fake,
                params(page::DialogType::Alert),
            )
            .unwrap_err();

        // Assert
        assert!(error.to_string().contains("capacity exceeded"));
        assert_eq!(retained.len(), MAX_LIVE_DIALOGS as usize);
    }
}
