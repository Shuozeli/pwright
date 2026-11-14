//! Optional Context-scoped HTTP authentication challenge handling.
//!
//! Credentials stay in bounded Rust storage. Chromium request interception is
//! enabled only for selected Contexts and every authentication challenge is
//! answered on the exact CDP session that emitted it. Page routing can share
//! the same `Fetch` installation without either capability overwriting the
//! other's protocol configuration.

use std::collections::VecDeque;
use std::error::Error;
use std::sync::{Arc, Mutex};

use pwright_backend_cdp_topology::{CdpContextOwnership, CdpTopology};
use pwright_cdp::{CdpConnection, CdpEvent, connection::CdpError, generated::page};
#[cfg(not(feature = "page-routes"))]
use pwright_cdp::{CdpEventReceiver, CdpEventRecvError};
#[cfg(not(feature = "page-routes"))]
use pwright_composition_chromium::SharedChromiumTopology;
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};
use pwright_plugin_chromium_frame_observer::{ChromiumFrameEventObserver, FrameObservationResult};
#[cfg(not(feature = "page-routes"))]
use pwright_plugin_chromium_session_initialization::{
    ChromiumSessionInitializationFuture, ChromiumSessionInitializationTarget,
    ChromiumSessionInitializer,
};
use serde_json::{Value, json};

use crate::HostError;

pub(crate) const MAX_CONTEXTS: usize = 16;
const MAX_PENDING_ACTIONS: usize = 64;
const MAX_ATTEMPTED_AUTHENTICATIONS: usize = 64;
const MAX_REQUEST_MAPPINGS: usize = 64;
const MAX_INITIALIZING_SESSIONS: usize = 64;
const MAX_NATIVE_ID_BYTES: usize = 4 * 1024;
pub const MAX_HTTP_CREDENTIAL_FIELD_BYTES: usize = 64 * 1024;
const SCOPE: ResourceScope = ResourceScope::extension(196, 0);

#[derive(Clone, Eq, PartialEq)]
pub struct ContextHttpCredentials {
    username: Box<str>,
    password: Box<str>,
    origin: Option<Box<str>>,
    send_always: bool,
}

impl std::fmt::Debug for ContextHttpCredentials {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ContextHttpCredentials")
            .field("has_username", &!self.username.is_empty())
            .field("has_password", &!self.password.is_empty())
            .field("origin", &self.origin)
            .field("send_always", &self.send_always)
            .finish()
    }
}

impl ContextHttpCredentials {
    pub fn new(
        username: String,
        password: String,
        origin: Option<String>,
    ) -> Result<Self, HostError> {
        Self::new_with_send(username, password, origin, false)
    }

    pub fn new_with_send(
        username: String,
        password: String,
        origin: Option<String>,
        send_always: bool,
    ) -> Result<Self, HostError> {
        for (name, value) in [("username", &username), ("password", &password)] {
            if value.len() > MAX_HTTP_CREDENTIAL_FIELD_BYTES {
                return Err(HostError::ResourceExhausted(format!(
                    "HTTP credentials {name} exceeds {MAX_HTTP_CREDENTIAL_FIELD_BYTES} UTF-8 bytes"
                )));
            }
            if value.contains('\0') {
                return Err(HostError::InvalidArgument(format!(
                    "HTTP credentials {name} must not contain NUL"
                )));
            }
        }
        let origin = origin
            .map(|origin| {
                if origin.len() > MAX_HTTP_CREDENTIAL_FIELD_BYTES {
                    return Err(HostError::ResourceExhausted(format!(
                        "HTTP credentials origin exceeds {MAX_HTTP_CREDENTIAL_FIELD_BYTES} UTF-8 bytes"
                    )));
                }
                if origin.contains('\0') {
                    return Err(HostError::InvalidArgument(
                        "HTTP credentials origin must not contain NUL".into(),
                    ));
                }
                Ok(origin.to_ascii_lowercase().into_boxed_str())
            })
            .transpose()?;
        Ok(Self {
            username: username.into_boxed_str(),
            password: password.into_boxed_str(),
            origin,
            send_always,
        })
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }

    pub fn origin(&self) -> Option<&str> {
        self.origin.as_deref()
    }

    pub const fn send_always(&self) -> bool {
        self.send_always
    }

    fn matches_url(&self, url: &str) -> bool {
        let Some(expected) = self.origin.as_deref() else {
            return true;
        };
        url::Url::parse(url)
            .map(|url| url.origin().ascii_serialization().to_ascii_lowercase() == expected)
            .unwrap_or(false)
    }
}

struct ContextEntry {
    context: ObjectId,
    credentials: ContextHttpCredentials,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PendingActionKind {
    ContinueRequest,
    ContinueWithAuth {
        response: &'static str,
        username: Option<Box<str>>,
        password: Option<Box<str>>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingAction {
    session_id: Box<str>,
    request_id: Box<str>,
    kind: PendingActionKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AttemptedAuthentication {
    session_id: Box<str>,
    request_id: Box<str>,
    network_id: Option<Box<str>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RequestMapping {
    session_id: Box<str>,
    request_id: Box<str>,
    network_id: Box<str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SessionScope {
    pub(crate) page: ObjectId,
    pub(crate) context: ObjectId,
    pub(crate) ownership: CdpContextOwnership,
}

struct InitializingSession {
    session_id: Box<str>,
    scope: SessionScope,
}

struct State {
    default_credentials: Option<ContextHttpCredentials>,
    entries: Vec<ContextEntry>,
    pending: VecDeque<PendingAction>,
    attempted: Vec<AttemptedAuthentication>,
    requests: Vec<RequestMapping>,
    initializing_sessions: Vec<InitializingSession>,
    closed: bool,
}

pub(crate) const fn requirements() -> ResourceRequirements {
    let credential_bytes = 3 * MAX_HTTP_CREDENTIAL_FIELD_BYTES;
    let pending_bytes =
        MAX_PENDING_ACTIONS * (std::mem::size_of::<PendingAction>() + 2 * MAX_NATIVE_ID_BYTES);
    let attempted_bytes = MAX_ATTEMPTED_AUTHENTICATIONS
        * (std::mem::size_of::<AttemptedAuthentication>() + 3 * MAX_NATIVE_ID_BYTES);
    let request_mapping_bytes =
        MAX_REQUEST_MAPPINGS * (std::mem::size_of::<RequestMapping>() + 3 * MAX_NATIVE_ID_BYTES);
    let initializing_session_bytes = MAX_INITIALIZING_SESSIONS
        * (std::mem::size_of::<InitializingSession>() + MAX_NATIVE_ID_BYTES);
    let storage_bytes = (std::mem::size_of::<Installation>()
        + std::mem::size_of::<State>()
        + MAX_CONTEXTS * (std::mem::size_of::<ContextEntry>() + credential_bytes)
        + pending_bytes
        + attempted_bytes
        + request_mapping_bytes
        + initializing_session_bytes) as u64;
    ResourceRequirements::none()
        .with_class(
            ResourceClass::BackendStateBytes,
            storage_bytes,
            storage_bytes,
        )
        .with_metadata(1, 1)
}

pub(crate) struct Installation {
    connection: Arc<CdpConnection>,
    state: Mutex<State>,
    _storage: ResourceLease,
}

impl Installation {
    pub(crate) fn new_with_default(
        resources: &ProductionResourceLedger,
        connection: Arc<CdpConnection>,
        default_credentials: Option<ContextHttpCredentials>,
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
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(MAX_CONTEXTS)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let mut pending = VecDeque::new();
        pending
            .try_reserve_exact(MAX_PENDING_ACTIONS)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let mut attempted = Vec::new();
        attempted
            .try_reserve_exact(MAX_ATTEMPTED_AUTHENTICATIONS)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let mut requests = Vec::new();
        requests
            .try_reserve_exact(MAX_REQUEST_MAPPINGS)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let mut initializing_sessions = Vec::new();
        initializing_sessions
            .try_reserve_exact(MAX_INITIALIZING_SESSIONS)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        Ok(Arc::new(Self {
            connection,
            state: Mutex::new(State {
                default_credentials,
                entries,
                pending,
                attempted,
                requests,
                initializing_sessions,
                closed: false,
            }),
            _storage: storage,
        }))
    }

    pub(crate) fn install(
        &self,
        context: ObjectId,
        credentials: ContextHttpCredentials,
    ) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        if state.entries.iter().any(|entry| entry.context == context) {
            return Err(HostError::InvalidArgument(
                "HTTP credentials are already installed for this Context".into(),
            ));
        }
        let retained = state
            .entries
            .len()
            .saturating_add(usize::from(state.default_credentials.is_some()));
        if retained >= MAX_CONTEXTS {
            return Err(HostError::ResourceExhausted(format!(
                "HTTP credentials exceed {MAX_CONTEXTS} live Contexts"
            )));
        }
        state.entries.push(ContextEntry {
            context,
            credentials,
        });
        Ok(())
    }

    pub(crate) fn remove(&self, context: ObjectId) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        state.entries.retain(|entry| entry.context != context);
        Ok(())
    }

    pub(crate) fn enabled(
        &self,
        context: ObjectId,
        ownership: CdpContextOwnership,
    ) -> Result<bool, HostError> {
        Ok(self.credentials(context, ownership)?.is_some())
    }

    pub(crate) fn credentials(
        &self,
        context: ObjectId,
        ownership: CdpContextOwnership,
    ) -> Result<Option<ContextHttpCredentials>, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        Ok(match ownership {
            CdpContextOwnership::Default => state.default_credentials.clone(),
            CdpContextOwnership::Owned => state
                .entries
                .iter()
                .find(|entry| entry.context == context)
                .map(|entry| entry.credentials.clone()),
            CdpContextOwnership::External => None,
        })
    }

    fn enqueue(&self, action: PendingAction) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        if state.pending.len() == MAX_PENDING_ACTIONS {
            return Err(HostError::ResourceExhausted(format!(
                "HTTP authentication action capacity exceeded ({MAX_PENDING_ACTIONS})"
            )));
        }
        state.pending.push_back(action);
        Ok(())
    }

    pub(crate) fn continue_request(
        &self,
        session_id: &str,
        request_id: &str,
        network_id: Option<&str>,
    ) -> Result<(), HostError> {
        validate_native_id("sessionId", session_id)?;
        validate_native_id("requestId", request_id)?;
        self.observe_request(session_id, request_id, network_id)?;
        self.enqueue(PendingAction {
            session_id: session_id.to_owned().into_boxed_str(),
            request_id: request_id.to_owned().into_boxed_str(),
            kind: PendingActionKind::ContinueRequest,
        })
    }

    fn observe_request(
        &self,
        session_id: &str,
        request_id: &str,
        network_id: Option<&str>,
    ) -> Result<(), HostError> {
        let Some(network_id) = network_id else {
            return Ok(());
        };
        validate_native_id("networkId", network_id)?;
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if let Some(mapping) = state.requests.iter_mut().find(|mapping| {
            mapping.session_id.as_ref() == session_id && mapping.request_id.as_ref() == request_id
        }) {
            mapping.network_id = network_id.to_owned().into_boxed_str();
            return Ok(());
        }
        if state.requests.len() == MAX_REQUEST_MAPPINGS {
            return Err(HostError::ResourceExhausted(format!(
                "HTTP authentication request mapping capacity exceeded ({MAX_REQUEST_MAPPINGS})"
            )));
        }
        state.requests.push(RequestMapping {
            session_id: session_id.to_owned().into_boxed_str(),
            request_id: request_id.to_owned().into_boxed_str(),
            network_id: network_id.to_owned().into_boxed_str(),
        });
        Ok(())
    }

    fn retire_request(&self, session_id: &str, network_id: &str) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        retire_request_state(&mut state, session_id, network_id);
        Ok(())
    }

    pub(crate) fn remember_initializing_session(
        &self,
        session_id: &str,
        scope: SessionScope,
    ) -> Result<(), HostError> {
        validate_native_id("sessionId", session_id)?;
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        if let Some(entry) = state
            .initializing_sessions
            .iter_mut()
            .find(|entry| entry.session_id.as_ref() == session_id)
        {
            entry.scope = scope;
            return Ok(());
        }
        if state.initializing_sessions.len() == MAX_INITIALIZING_SESSIONS {
            return Err(HostError::ResourceExhausted(format!(
                "HTTP authentication initializing session capacity exceeded ({MAX_INITIALIZING_SESSIONS})"
            )));
        }
        state.initializing_sessions.push(InitializingSession {
            session_id: session_id.to_owned().into_boxed_str(),
            scope,
        });
        Ok(())
    }

    pub(crate) fn initializing_session(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionScope>, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        Ok(state
            .initializing_sessions
            .iter()
            .find(|entry| entry.session_id.as_ref() == session_id)
            .map(|entry| entry.scope))
    }

    pub(crate) fn retire_initializing_session(&self, session_id: &str) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        state
            .initializing_sessions
            .retain(|entry| entry.session_id.as_ref() != session_id);
        Ok(())
    }

    fn reconcile_initializing_sessions(&self, topology: &CdpTopology) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        state.initializing_sessions.retain(|entry| {
            topology
                .target_session_route(entry.session_id.as_ref())
                .is_none()
        });
        Ok(())
    }

    fn authenticate(
        &self,
        context: ObjectId,
        ownership: CdpContextOwnership,
        session_id: &str,
        request_id: &str,
        url: &str,
    ) -> Result<(), HostError> {
        validate_native_id("sessionId", session_id)?;
        validate_native_id("requestId", request_id)?;
        let credentials = self.credentials(context, ownership)?;
        let matching = credentials.filter(|credentials| credentials.matches_url(url));
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        if state.pending.len() == MAX_PENDING_ACTIONS {
            return Err(HostError::ResourceExhausted(format!(
                "HTTP authentication action capacity exceeded ({MAX_PENDING_ACTIONS})"
            )));
        }
        let kind = authentication_kind(&mut state, matching, session_id, request_id)?;
        state.pending.push_back(PendingAction {
            session_id: session_id.to_owned().into_boxed_str(),
            request_id: request_id.to_owned().into_boxed_str(),
            kind,
        });
        Ok(())
    }

    pub(crate) async fn finish_next(&self) -> Result<(), HostError> {
        let action = self
            .state
            .lock()
            .map_err(HostError::backend)?
            .pending
            .pop_front();
        let Some(action) = action else {
            return Ok(());
        };
        let result = match action.kind {
            PendingActionKind::ContinueRequest => {
                self.connection
                    .send_command_with_session(
                        "Fetch.continueRequest",
                        json!({ "requestId": action.request_id }),
                        Some(&action.session_id),
                    )
                    .await
            }
            PendingActionKind::ContinueWithAuth {
                response,
                username,
                password,
            } => {
                let auth_challenge_response =
                    auth_challenge_response(response, username.as_deref(), password.as_deref())?;
                self.connection
                    .send_command_with_session(
                        "Fetch.continueWithAuth",
                        json!({
                            "requestId": action.request_id,
                            "authChallengeResponse": auth_challenge_response
                        }),
                        Some(&action.session_id),
                    )
                    .await
            }
        };
        match result {
            Ok(_) => Ok(()),
            Err(error) if terminal_action_error(&error) => Ok(()),
            Err(error) => Err(HostError::backend(error)),
        }
    }

    pub(crate) fn close(&self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.closed = true;
        state.default_credentials = None;
        state.entries.clear();
        state.pending.clear();
        state.attempted.clear();
        state.requests.clear();
        state.initializing_sessions.clear();
    }

    pub(crate) fn observe_event(
        &self,
        topology: &CdpTopology,
        event: &CdpEvent,
    ) -> FrameObservationResult {
        if event.method == "Target.detachedFromTarget" {
            if let Some(session_id) = event
                .params
                .as_object()
                .and_then(|params| params.get("sessionId"))
                .and_then(Value::as_str)
            {
                self.retire_initializing_session(session_id)?;
            }
            return Ok(());
        }
        if !matches!(
            event.method.as_str(),
            "Fetch.authRequired"
                | "Fetch.requestPaused"
                | "Network.loadingFinished"
                | "Network.loadingFailed"
        ) {
            return Ok(());
        }
        let session_id = event
            .session_id
            .as_deref()
            .ok_or_else(|| invalid("sessionId"))?;
        let params = event.params.as_object().ok_or_else(|| invalid("params"))?;
        let request_id = params
            .get("requestId")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("requestId"))?;
        if matches!(
            event.method.as_str(),
            "Network.loadingFinished" | "Network.loadingFailed"
        ) {
            self.retire_request(session_id, request_id)?;
            return Ok(());
        }
        if event.method == "Fetch.requestPaused" {
            let network_id = params.get("networkId").and_then(Value::as_str);
            self.observe_request(session_id, request_id, network_id)?;
            #[cfg(not(feature = "page-routes"))]
            self.continue_request(session_id, request_id, network_id)?;
            return Ok(());
        }
        let scope = if let Some(target) = topology.target_session_route(session_id) {
            let page = topology
                .page_object_id(target.page_target_id())
                .ok_or_else(|| invalid("Page route"))?;
            let context = topology
                .page_session_routes()
                .find(|route| route.object_id() == page)
                .map(|route| route.browser_context_object_id())
                .ok_or_else(|| invalid("BrowserContext route"))?;
            let ownership = topology
                .browser_context_routes()
                .find(|route| route.object_id() == context)
                .map(|route| route.ownership())
                .ok_or_else(|| invalid("BrowserContext ownership"))?;
            SessionScope {
                page,
                context,
                ownership,
            }
        } else {
            self.initializing_session(session_id)?
                .ok_or_else(|| invalid("target route"))?
        };
        let url = params
            .get("request")
            .and_then(Value::as_object)
            .and_then(|request| request.get("url"))
            .and_then(Value::as_str)
            .ok_or_else(|| invalid("request.url"))?;
        self.authenticate(scope.context, scope.ownership, session_id, request_id, url)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }
}

#[cfg(not(feature = "page-routes"))]
pub(crate) struct EventDriver {
    installation: Arc<Installation>,
    topology: SharedChromiumTopology,
    receiver: CdpEventReceiver,
}

#[cfg(not(feature = "page-routes"))]
impl EventDriver {
    pub(crate) fn new(
        installation: Arc<Installation>,
        topology: SharedChromiumTopology,
        connection: &CdpConnection,
    ) -> Result<Self, HostError> {
        Ok(Self {
            installation,
            topology,
            receiver: connection
                .subscribe_composable_events()
                .map_err(HostError::backend)?,
        })
    }

    pub(crate) async fn run(
        &mut self,
        stopped: &tokio_util::sync::CancellationToken,
    ) -> Result<(), HostError> {
        loop {
            let event = tokio::select! {
                biased;
                () = stopped.cancelled() => return Ok(()),
                event = self.receiver.recv() => event,
            };
            let event = match event {
                Ok(event) => event,
                Err(CdpEventRecvError::Closed) => return Err(HostError::TargetClosed),
                Err(CdpEventRecvError::Lagged(skipped)) => {
                    return Err(HostError::Backend(format!(
                        "HTTP authentication event lane lagged by {skipped} events"
                    )));
                }
            };
            {
                let topology = self.topology.lock().map_err(HostError::backend)?;
                self.installation
                    .observe_event(topology.topology(), &event)
                    .map_err(HostError::backend)?;
            }
            self.installation.finish_next().await?;
        }
    }
}

fn auth_challenge_response(
    response: &'static str,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<Value, HostError> {
    match (username, password) {
        (Some(username), Some(password)) => Ok(json!({
            "response": response,
            "username": username,
            "password": password
        })),
        (None, None) => Ok(json!({ "response": response })),
        _ => Err(HostError::Backend(
            "HTTP authentication action has incomplete credentials".into(),
        )),
    }
}

fn authentication_kind(
    state: &mut State,
    matching: Option<ContextHttpCredentials>,
    session_id: &str,
    request_id: &str,
) -> Result<PendingActionKind, HostError> {
    let attempted = state.attempted.iter().position(|attempt| {
        attempt.session_id.as_ref() == session_id && attempt.request_id.as_ref() == request_id
    });
    if let Some(index) = attempted {
        state.attempted.swap_remove(index);
        return Ok(PendingActionKind::ContinueWithAuth {
            response: "CancelAuth",
            username: None,
            password: None,
        });
    }
    let Some(credentials) = matching else {
        return Ok(PendingActionKind::ContinueWithAuth {
            response: "Default",
            username: None,
            password: None,
        });
    };
    if state.attempted.len() == MAX_ATTEMPTED_AUTHENTICATIONS {
        return Err(HostError::ResourceExhausted(format!(
            "HTTP authentication attempt capacity exceeded ({MAX_ATTEMPTED_AUTHENTICATIONS})"
        )));
    }
    state.attempted.push(AttemptedAuthentication {
        session_id: session_id.to_owned().into_boxed_str(),
        request_id: request_id.to_owned().into_boxed_str(),
        network_id: state
            .requests
            .iter()
            .find(|mapping| {
                mapping.session_id.as_ref() == session_id
                    && mapping.request_id.as_ref() == request_id
            })
            .map(|mapping| mapping.network_id.clone()),
    });
    Ok(PendingActionKind::ContinueWithAuth {
        response: "ProvideCredentials",
        username: Some(credentials.username),
        password: Some(credentials.password),
    })
}

fn retire_request_state(state: &mut State, session_id: &str, network_id: &str) {
    state.attempted.retain(|attempt| {
        attempt.session_id.as_ref() != session_id
            || attempt.network_id.as_deref() != Some(network_id)
    });
    state.requests.retain(|mapping| {
        mapping.session_id.as_ref() != session_id || mapping.network_id.as_ref() != network_id
    });
}

fn validate_native_id(name: &str, value: &str) -> Result<(), HostError> {
    if value.is_empty() || value.len() > MAX_NATIVE_ID_BYTES {
        return Err(HostError::ResourceExhausted(format!(
            "HTTP authentication {name} must contain 1..={MAX_NATIVE_ID_BYTES} UTF-8 bytes"
        )));
    }
    Ok(())
}

fn terminal_action_error(error: &CdpError) -> bool {
    match error {
        CdpError::Closed | CdpError::SessionDetached | CdpError::PageClosed => true,
        CdpError::Protocol { message, .. } => {
            message.contains("Invalid InterceptionId")
                || message.contains("Target closed")
                || message.contains("Session closed")
                || message.contains("Session with given id not found")
        }
        _ => false,
    }
}

#[cfg(not(feature = "page-routes"))]
pub(crate) struct Initializer<Downstream> {
    downstream: Downstream,
    installation: Arc<Installation>,
}

#[cfg(not(feature = "page-routes"))]
impl<Downstream> Initializer<Downstream> {
    pub(crate) fn new(downstream: Downstream, installation: Arc<Installation>) -> Self {
        Self {
            downstream,
            installation,
        }
    }
}

#[cfg(not(feature = "page-routes"))]
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
        self.downstream
            .initialize(session_id, target_id, ownership, is_page)
    }

    fn initialize_scoped<'a>(
        &'a self,
        target: ChromiumSessionInitializationTarget<'a>,
    ) -> ChromiumSessionInitializationFuture<'a> {
        Box::pin(async move {
            if !self
                .installation
                .enabled(target.context, target.ownership)?
            {
                return self.downstream.initialize_scoped(target).await;
            }
            self.installation.remember_initializing_session(
                target.session_id,
                SessionScope {
                    page: target.page,
                    context: target.context,
                    ownership: target.ownership,
                },
            )?;
            let disable_cache = self.installation.connection.send_command_with_session(
                "Network.setCacheDisabled",
                json!({ "cacheDisabled": true }),
                Some(target.session_id),
            );
            let enable_fetch = self.installation.connection.send_command_with_session(
                "Fetch.enable",
                json!({
                    "handleAuthRequests": true,
                    "patterns": [{ "urlPattern": "*", "requestStage": "Request" }]
                }),
                Some(target.session_id),
            );
            let downstream = self.downstream.initialize_scoped(target);
            let protocol = async {
                tokio::try_join!(disable_cache, enable_fetch)?;
                Ok::<_, Box<dyn Error + Send + Sync>>(())
            };
            tokio::try_join!(downstream, protocol)?;
            Ok(())
        })
    }
}

pub(crate) struct Observer {
    installation: Arc<Installation>,
    downstream: Arc<dyn ChromiumFrameEventObserver>,
}

impl Observer {
    pub(crate) fn new(
        installation: Arc<Installation>,
        downstream: Arc<dyn ChromiumFrameEventObserver>,
    ) -> Self {
        Self {
            installation,
            downstream,
        }
    }
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
        self.downstream.reconcile(topology)?;
        self.installation
            .reconcile_initializing_sessions(topology)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }

    fn event(&self, topology: &CdpTopology, event: &CdpEvent) -> FrameObservationResult {
        self.downstream.event(topology, event)
    }

    fn seal(&self) {
        self.installation.close();
        self.downstream.seal();
    }
}

fn invalid(field: &str) -> Box<dyn Error + Send + Sync> {
    Box::new(HostError::Backend(format!(
        "HTTP authentication event is missing {field}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credentials(origin: Option<&str>) -> ContextHttpCredentials {
        ContextHttpCredentials::new(
            "user".to_owned(),
            "pass".to_owned(),
            origin.map(str::to_owned),
        )
        .expect("valid credentials")
    }

    fn state() -> State {
        State {
            default_credentials: None,
            entries: Vec::new(),
            pending: VecDeque::new(),
            attempted: Vec::new(),
            requests: Vec::new(),
            initializing_sessions: Vec::new(),
            closed: false,
        }
    }

    #[test]
    fn origin_matching_is_exactly_scoped() {
        // Arrange
        let credentials = credentials(Some("http://example.test"));

        // Act
        let matching = credentials.matches_url("http://example.test/private");
        let nonmatching = credentials.matches_url("http://other.test/private");

        // Assert
        assert!(matching);
        assert!(!nonmatching);
    }

    #[test]
    fn request_send_policy_remains_explicit() {
        // Arrange
        let credentials = ContextHttpCredentials::new_with_send(
            "user".to_owned(),
            "pass".to_owned(),
            Some("https://example.test".to_owned()),
            true,
        )
        .expect("valid credentials");

        // Act
        let observed = (
            credentials.username(),
            credentials.password(),
            credentials.origin(),
            credentials.send_always(),
        );

        // Assert
        assert_eq!(
            observed,
            ("user", "pass", Some("https://example.test"), true,)
        );
    }

    #[test]
    fn debug_output_redacts_credential_values() {
        // Arrange
        let credentials = ContextHttpCredentials::new_with_send(
            "private-user".to_owned(),
            "private-password".to_owned(),
            Some("https://example.test".to_owned()),
            true,
        )
        .expect("valid credentials");

        // Act
        let output = format!("{credentials:?}");

        // Assert
        assert!(!output.contains("private-user"));
        assert!(!output.contains("private-password"));
        assert!(output.contains("has_username: true"));
        assert!(output.contains("has_password: true"));
        assert!(output.contains("https://example.test"));
    }

    #[test]
    fn first_matching_challenge_provides_credentials() {
        // Arrange
        let mut state = state();

        // Act
        let kind = authentication_kind(&mut state, Some(credentials(None)), "session", "request")
            .expect("authenticate challenge");

        // Assert
        assert_eq!(
            kind,
            PendingActionKind::ContinueWithAuth {
                response: "ProvideCredentials",
                username: Some("user".into()),
                password: Some("pass".into()),
            }
        );
    }

    #[test]
    fn repeated_challenge_cancels_authentication() {
        // Arrange
        let mut state = state();
        authentication_kind(&mut state, Some(credentials(None)), "session", "request")
            .expect("first challenge");

        // Act
        let kind = authentication_kind(&mut state, Some(credentials(None)), "session", "request")
            .expect("repeated challenge");

        // Assert
        assert_eq!(
            kind,
            PendingActionKind::ContinueWithAuth {
                response: "CancelAuth",
                username: None,
                password: None,
            }
        );
    }

    #[test]
    fn cancel_auth_payload_omits_optional_credential_fields() {
        // Arrange
        let response = "CancelAuth";

        // Act
        let payload = auth_challenge_response(response, None, None)
            .expect("credential-free response should serialize");

        // Assert
        assert_eq!(payload, json!({ "response": "CancelAuth" }));
    }

    #[test]
    fn origin_mismatch_defers_to_chromium() {
        // Arrange
        let mut state = state();

        // Act
        let kind = authentication_kind(&mut state, None, "session", "request")
            .expect("mismatched challenge");

        // Assert
        assert_eq!(
            kind,
            PendingActionKind::ContinueWithAuth {
                response: "Default",
                username: None,
                password: None,
            }
        );
    }

    #[test]
    fn completed_network_request_releases_authentication_attempt() {
        // Arrange
        let mut state = state();
        state.requests.push(RequestMapping {
            session_id: "session".into(),
            request_id: "fetch-request".into(),
            network_id: "network-request".into(),
        });
        authentication_kind(
            &mut state,
            Some(credentials(None)),
            "session",
            "fetch-request",
        )
        .expect("first challenge");

        // Act
        retire_request_state(&mut state, "session", "network-request");

        // Assert
        assert!(state.attempted.is_empty());
        assert!(state.requests.is_empty());
    }
}
