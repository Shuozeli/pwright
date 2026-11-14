//! Optional Page and BrowserContext init-script registration. Rust owns
//! bounded persistence, target-session selection, future child-session
//! installation, and rollback.

use std::future::Future;
use std::sync::{Arc, Mutex};

use pwright_cdp::CdpConnection;
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};
use pwright_plugin_chromium_session_initialization::{
    ChromiumSessionInitializationFuture, ChromiumSessionInitializationTarget,
    ChromiumSessionInitializer,
};
use serde_json::Value;

use crate::{HostError, HostSession};

const SCOPE: ResourceScope = ResourceScope::extension(138, 0);
pub const MAX_SOURCE_BYTES: usize = 64 * 1024;
const MAX_SCRIPTS: usize = 128;

#[derive(Clone, Copy)]
enum InstallationPhase {
    NewSession,
    ExistingSession,
}

async fn install_with_sender<SendCommand, Response>(
    source: &str,
    phase: InstallationPhase,
    send_command: SendCommand,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>>
where
    SendCommand: FnOnce(&'static str, Value) -> Response,
    Response: Future<Output = pwright_cdp::connection::Result<Value>>,
{
    let params = match phase {
        // The new target may already own its initial about:blank document by
        // the time its paused CDP session is initialized.
        InstallationPhase::NewSession => {
            serde_json::json!({ "source": source, "runImmediately": true })
        }
        // Playwright does not execute a dynamically added script in the
        // document that already exists; it takes effect on later documents.
        InstallationPhase::ExistingSession => serde_json::json!({ "source": source }),
    };
    let value = send_command("Page.addScriptToEvaluateOnNewDocument", params).await?;
    value
        .get("identifier")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            Box::new(HostError::Backend(
                "Page.addScriptToEvaluateOnNewDocument is missing identifier".into(),
            )) as Box<dyn std::error::Error + Send + Sync>
        })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RegistrationScope {
    Page(ObjectId),
    Context(ObjectId),
}

impl RegistrationScope {
    fn applies_to(self, page: ObjectId, context: ObjectId) -> bool {
        match self {
            Self::Page(expected) => expected == page,
            Self::Context(expected) => expected == context,
        }
    }
}

#[derive(Clone)]
struct Registration {
    id: u64,
    scope: RegistrationScope,
    source: Box<str>,
}

struct State {
    registrations: Vec<Registration>,
    next_id: u64,
}

fn add_registration_to_state(
    state: &mut State,
    scope: RegistrationScope,
    source: String,
) -> Result<u64, HostError> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(HostError::InvalidArgument(format!(
            "init script exceeds {MAX_SOURCE_BYTES} UTF-8 bytes"
        )));
    }
    if state.registrations.len() == MAX_SCRIPTS {
        return Err(HostError::ResourceExhausted(format!(
            "init-script capacity exceeded ({MAX_SCRIPTS})"
        )));
    }
    let id = state.next_id;
    state.next_id = state
        .next_id
        .checked_add(1)
        .ok_or_else(|| HostError::ResourceExhausted("init-script ID space exhausted".into()))?;
    state.registrations.push(Registration {
        id,
        scope,
        source: source.into_boxed_str(),
    });
    Ok(id)
}

fn remove_context_from_state(state: &mut State, context: ObjectId) {
    state
        .registrations
        .retain(|registration| registration.scope != RegistrationScope::Context(context));
}

pub(crate) struct Installation {
    connection: Arc<CdpConnection>,
    state: Mutex<State>,
    _storage: ResourceLease,
}

pub(crate) const fn requirements() -> ResourceRequirements {
    let bytes = (std::mem::size_of::<Installation>()
        + std::mem::size_of::<State>()
        + MAX_SCRIPTS * (std::mem::size_of::<Registration>() + MAX_SOURCE_BYTES))
        as u64;
    ResourceRequirements::none()
        .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
        .with_metadata(1, 1)
}

impl Installation {
    pub(crate) fn new(
        connection: Arc<CdpConnection>,
        resources: &ProductionResourceLedger,
    ) -> Result<Arc<Self>, HostError> {
        let plan = requirements();
        let storage = resources
            .claim(plan)
            .map_err(HostError::backend)?
            .reserve(
                SCOPE,
                ResourceClass::BackendStateBytes,
                plan.class(ResourceClass::BackendStateBytes).global,
            )
            .map_err(HostError::backend)?;
        let mut registrations = Vec::new();
        registrations
            .try_reserve_exact(MAX_SCRIPTS)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        Ok(Arc::new(Self {
            connection,
            state: Mutex::new(State {
                registrations,
                next_id: 1,
            }),
            _storage: storage,
        }))
    }

    fn register(&self, scope: RegistrationScope, source: String) -> Result<u64, HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        add_registration_to_state(&mut state, scope, source)
    }

    fn unregister(&self, id: u64) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if let Some(index) = state.registrations.iter().position(|entry| entry.id == id) {
            state.registrations.remove(index);
        }
        Ok(())
    }

    pub(crate) fn remove_context(&self, context: ObjectId) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        remove_context_from_state(&mut state, context);
        Ok(())
    }

    fn sources(&self, page: ObjectId, context: ObjectId) -> Result<Vec<String>, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        Ok(state
            .registrations
            .iter()
            .filter(|registration| registration.scope.applies_to(page, context))
            .map(|registration| registration.source.to_string())
            .collect())
    }

    async fn install(
        &self,
        session_id: &str,
        source: &str,
        phase: InstallationPhase,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        install_with_sender(source, phase, |method, params| {
            self.connection
                .send_command_with_session(method, params, Some(session_id))
        })
        .await
    }

    async fn uninstall(&self, session_id: &str, identifier: &str) {
        let _ = self
            .connection
            .send_command_with_session(
                "Page.removeScriptToEvaluateOnNewDocument",
                serde_json::json!({ "identifier": identifier }),
                Some(session_id),
            )
            .await;
    }
}

pub(crate) struct Initializer<Downstream> {
    downstream: Downstream,
    installation: Arc<Installation>,
}

impl<Downstream> Initializer<Downstream> {
    pub(crate) fn new(downstream: Downstream, installation: Arc<Installation>) -> Self {
        Self {
            downstream,
            installation,
        }
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
        self.downstream
            .initialize(session_id, target_id, ownership, is_page)
    }

    fn initialize_scoped<'a>(
        &'a self,
        target: ChromiumSessionInitializationTarget<'a>,
    ) -> ChromiumSessionInitializationFuture<'a> {
        Box::pin(async move {
            self.downstream.initialize_scoped(target).await?;
            for source in self.installation.sources(target.page, target.context)? {
                self.installation
                    .install(target.session_id, &source, InstallationPhase::NewSession)
                    .await?;
            }
            Ok(())
        })
    }
}

impl HostSession {
    async fn add_init_script(
        &self,
        scope: RegistrationScope,
        source: String,
        sessions: Vec<String>,
    ) -> Result<(), HostError> {
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            live.init_scripts.clone()
        };
        let registration = installation.register(scope, source.clone())?;
        let mut installed = Vec::new();
        for session in &sessions {
            match installation
                .install(session, &source, InstallationPhase::ExistingSession)
                .await
            {
                Ok(identifier) => installed.push((session.clone(), identifier)),
                Err(error) => {
                    installation.unregister(registration)?;
                    for (session, identifier) in installed {
                        installation.uninstall(&session, &identifier).await;
                    }
                    return Err(HostError::backend(error));
                }
            }
        }
        Ok(())
    }

    pub async fn page_add_init_script(
        &self,
        page_id: &str,
        source: String,
    ) -> Result<(), HostError> {
        let page = page_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Page ID".into()))?;
        self.require_visible_page_id(page)?;
        let sessions = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let topology = topology.topology();
            let target = topology
                .page_session_routes()
                .find(|route| route.object_id() == page)
                .map(|route| route.target_id().to_owned())
                .ok_or(HostError::TargetClosed)?;
            topology
                .target_session_routes()
                .filter(|route| route.page_target_id() == target)
                .map(|route| route.session_id().to_owned())
                .collect::<Vec<_>>()
        };
        self.add_init_script(RegistrationScope::Page(page), source, sessions)
            .await
    }

    pub async fn context_add_init_script(
        &self,
        context_id: &str,
        source: String,
    ) -> Result<(), HostError> {
        let context = context_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Context ID".into()))?;
        self.require_visible_context_id(context)?;
        let sessions = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let topology = topology.topology();
            if !topology
                .browser_context_routes()
                .any(|route| route.object_id() == context)
            {
                return Err(HostError::TargetClosed);
            }
            let page_targets = topology
                .page_session_routes()
                .filter(|route| route.browser_context_object_id() == context)
                .map(|route| route.target_id().to_owned())
                .collect::<Vec<_>>();
            topology
                .target_session_routes()
                .filter(|route| page_targets.iter().any(|id| id == route.page_target_id()))
                .map(|route| route.session_id().to_owned())
                .collect::<Vec<_>>()
        };
        self.add_init_script(RegistrationScope::Context(context), source, sessions)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeCdpCommandSink {
        calls: Mutex<Vec<(&'static str, Value)>>,
    }

    impl FakeCdpCommandSink {
        async fn send(
            &self,
            method: &'static str,
            params: Value,
        ) -> pwright_cdp::connection::Result<Value> {
            self.calls.lock().unwrap().push((method, params));
            Ok(serde_json::json!({ "identifier": "script-1" }))
        }
    }

    #[tokio::test]
    async fn existing_context_script_runs_in_initial_document_of_new_page() {
        // Arrange
        let fake = FakeCdpCommandSink::default();

        // Act
        let identifier = install_with_sender(
            "globalThis.ready = true",
            InstallationPhase::NewSession,
            |method, params| fake.send(method, params),
        )
        .await
        .unwrap();

        // Assert
        assert_eq!(identifier, "script-1");
        assert_eq!(
            *fake.calls.lock().unwrap(),
            vec![(
                "Page.addScriptToEvaluateOnNewDocument",
                serde_json::json!({
                    "source": "globalThis.ready = true",
                    "runImmediately": true,
                }),
            )]
        );
    }

    #[tokio::test]
    async fn dynamically_added_script_does_not_run_in_existing_document() {
        // Arrange
        let fake = FakeCdpCommandSink::default();

        // Act
        let identifier = install_with_sender(
            "globalThis.future = true",
            InstallationPhase::ExistingSession,
            |method, params| fake.send(method, params),
        )
        .await
        .unwrap();

        // Assert
        assert_eq!(identifier, "script-1");
        assert_eq!(
            *fake.calls.lock().unwrap(),
            vec![(
                "Page.addScriptToEvaluateOnNewDocument",
                serde_json::json!({ "source": "globalThis.future = true" }),
            )]
        );
    }

    #[test]
    fn registration_is_bounded_and_page_scoped() {
        // Arrange
        let mut state = State {
            registrations: Vec::new(),
            next_id: 1,
        };
        let page = ObjectId::new(7);

        // Act
        add_registration_to_state(
            &mut state,
            RegistrationScope::Page(page),
            "globalThis.a = 1".into(),
        )
        .unwrap();
        add_registration_to_state(
            &mut state,
            RegistrationScope::Page(ObjectId::new(8)),
            "globalThis.b = 2".into(),
        )
        .unwrap();
        let sources: Vec<_> = state
            .registrations
            .iter()
            .filter(|registration| registration.scope.applies_to(page, ObjectId::new(3)))
            .map(|registration| registration.source.as_ref())
            .collect();

        // Assert
        assert_eq!(sources, ["globalThis.a = 1"]);
    }

    #[test]
    fn oversized_source_is_rejected_before_storage() {
        // Arrange
        let mut state = State {
            registrations: Vec::new(),
            next_id: 1,
        };

        // Act
        let error = add_registration_to_state(
            &mut state,
            RegistrationScope::Page(ObjectId::new(7)),
            "x".repeat(MAX_SOURCE_BYTES + 1),
        )
        .unwrap_err();

        // Assert
        assert!(matches!(error, HostError::InvalidArgument(_)));
        assert!(state.registrations.is_empty());
    }

    #[test]
    fn context_registration_applies_to_each_page_in_only_that_context() {
        // Arrange
        let context = ObjectId::new(3);
        let other_context = ObjectId::new(4);
        let scope = RegistrationScope::Context(context);

        // Act
        let first = scope.applies_to(ObjectId::new(7), context);
        let second = scope.applies_to(ObjectId::new(8), context);
        let other = scope.applies_to(ObjectId::new(9), other_context);

        // Assert
        assert!(first);
        assert!(second);
        assert!(!other);
    }

    #[test]
    fn removing_context_reclaims_only_that_contexts_registrations() {
        // Arrange
        let context = ObjectId::new(3);
        let other_context = ObjectId::new(4);
        let mut state = State {
            registrations: vec![
                Registration {
                    id: 1,
                    scope: RegistrationScope::Context(context),
                    source: "first".into(),
                },
                Registration {
                    id: 2,
                    scope: RegistrationScope::Context(other_context),
                    source: "second".into(),
                },
                Registration {
                    id: 3,
                    scope: RegistrationScope::Page(ObjectId::new(7)),
                    source: "page".into(),
                },
            ],
            next_id: 4,
        };

        // Act
        remove_context_from_state(&mut state, context);

        // Assert
        assert_eq!(state.registrations.len(), 2);
        assert_eq!(state.registrations[0].source.as_ref(), "second");
        assert_eq!(state.registrations[1].source.as_ref(), "page");
    }
}
